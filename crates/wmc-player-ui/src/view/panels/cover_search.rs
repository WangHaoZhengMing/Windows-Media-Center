use super::{PlayerView, resources};
use crate::backdrop::Backdrop;
use gpui_kit::*;
use wmc_enrichment::covers::{CoverCache, CoverCandidate, CoversClient, DownloadedCover};

impl PlayerView {
    pub(in crate::view) fn close_cover_search(&mut self) {
        self.cover_search.open = false;
        self.cover_search.busy = false;
        self.cover_search.generation += 1;
        self.cover_search.task = None;
    }
    pub(in crate::view) fn search_online_cover(&mut self, cx: &mut Context<Self>) {
        self.show_more = false;
        self.settings.info_open = false;
        self.cover_search.generation += 1;
        let generation = self.cover_search.generation;
        let query = self.online_query.clone();
        self.cover_search.open = true;
        self.lyrics_search.open = false;
        self.cover_search.busy = true;
        self.cover_search.candidates.clear();
        self.cover_search.message = "正在匹配专辑封面…".into();
        self.cover_search.task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { CoversClient::new()?.search(&query) })
                .await;
            let candidates = match result {
                Ok(result) => {
                    let candidates = result.candidates;
                    let visible = candidates.clone();
                    let errors = result.errors;
                    if this
                        .update(cx, |this, cx| {
                            if this.cover_search.generation != generation {
                                return false;
                            }
                            this.cover_search.busy = false;
                            this.cover_search.message = if visible.is_empty() {
                                "未找到封面，可以稍后重试".into()
                            } else {
                                format!("{} 个候选 · 点击应用 · 缩略图加载中", visible.len())
                            };
                            if !errors.is_empty() {
                                this.cover_search
                                    .message
                                    .push_str(&format!("\n{}", errors.join("\n")));
                            }
                            this.cover_search.candidates = visible
                                .into_iter()
                                .map(|candidate| (candidate, None))
                                .collect();
                            cx.notify();
                            true
                        })
                        .ok()
                        != Some(true)
                    {
                        return;
                    }
                    candidates
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        if this.cover_search.generation == generation {
                            this.cover_search.busy = false;
                            this.cover_search.message = format!("搜索失败：{error:#}");
                            cx.notify();
                        }
                    });
                    return;
                }
            };
            // Load previews in batches; all requests share a three-slot transport.
            for (batch, group) in candidates.chunks(3).enumerate() {
                let group = group.to_vec();
                let images = cx
                    .background_executor()
                    .spawn(async move {
                        std::thread::scope(|scope| {
                            let jobs: Vec<_> = group
                                .into_iter()
                                .map(|candidate| {
                                    scope.spawn(move || {
                                        CoversClient::new()
                                            .and_then(|client| client.fetch(&candidate))
                                            .and_then(|cover| cover.image())
                                            .ok()
                                            .map(|image| {
                                                resources::render_image(image::imageops::thumbnail(
                                                    &image, 96, 96,
                                                ))
                                            })
                                    })
                                })
                                .collect();
                            jobs.into_iter()
                                .map(|job| job.join().ok().flatten())
                                .collect::<Vec<_>>()
                        })
                    })
                    .await;
                if this
                    .update(cx, |this, cx| {
                        if this.cover_search.generation != generation {
                            return false;
                        }
                        for (offset, image) in images.into_iter().enumerate() {
                            if let Some(candidate) =
                                this.cover_search.candidates.get_mut(batch * 3 + offset)
                            {
                                candidate.1 = image;
                            }
                        }
                        cx.notify();
                        true
                    })
                    .ok()
                    != Some(true)
                {
                    return;
                }
            }
        }));
        cx.notify();
    }
    pub(in crate::view) fn select_online_cover(
        &mut self,
        candidate: CoverCandidate,
        cx: &mut Context<Self>,
    ) {
        self.cover_search.generation += 1;
        let generation = self.cover_search.generation;
        self.cover_search.busy = true;
        self.cover_search.message = format!("正在加载 {} · {}…", candidate.album, candidate.artist);
        let query = self.online_query.clone();
        let identity = self.media_identity.clone();
        self.cover_search.task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let cover = CoversClient::new()?.fetch(&candidate)?;
                    let image = cover.image()?;
                    Ok::<_, anyhow::Error>((cover, image))
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.cover_search.generation != generation {
                    return;
                }
                this.cover_search.busy = false;
                match result {
                    Ok((cover, image)) => {
                        let saved = cover.clone();
                        this.apply_online_cover(cover, image, cx);
                        this.cover_search.open = false;
                        this.cover_search.task = Some(cx.spawn(async move |this, cx| {
                            let result = cx
                                .background_executor()
                                .spawn(async move {
                                    let identity = identity.map(Ok).unwrap_or_else(|| {
                                        wmc_enrichment::cache::MediaIdentity::read(&query, None)
                                    })?;
                                    wmc_enrichment::cache::MediaCache::default_location()?
                                        .save_cover(&identity, &saved)
                                })
                                .await;
                            let _ = this.update(cx, |this, cx| {
                                if this.cover_search.generation == generation {
                                    if let Err(error) = result {
                                        this.status =
                                            Some(format!("封面已应用，缓存失败：{error:#}"));
                                    }
                                    cx.notify();
                                }
                            });
                        }));
                    }
                    Err(error) => {
                        this.cover_search.message = format!("加载失败：{error:#}，可以选择其他候选")
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }
    pub(in crate::view) fn load_cached_cover(
        &mut self,
        network_needed: bool,
        cx: &mut Context<Self>,
    ) {
        let query = self.online_query.clone();
        let identity = self.media_identity.clone();
        let generation = self.cover_search.generation;
        self.cover_search.task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let identity = identity.map(Ok).unwrap_or_else(|| {
                        wmc_enrichment::cache::MediaIdentity::read(&query, None)
                    })?;
                    let indexed = wmc_enrichment::cache::MediaCache::default_location()
                        .and_then(|cache| cache.load_cover(&identity))
                        .unwrap_or_else(|error| {
                            eprintln!("封面索引读取失败：{error:#}");
                            None
                        });
                    let cached = indexed.or_else(|| {
                        CoverCache::default_location()
                            .and_then(|cache| cache.load(&query))
                            .ok()
                            .flatten()
                    });
                    if let Some(cover) = cached
                        && let Ok(image) = cover.image()
                    {
                        return Ok(Some((cover, image)));
                    }
                    if !network_needed {
                        return Ok(None);
                    }
                    let client = CoversClient::new()?;
                    let results = client.search(&query)?;
                    let mut errors = results.errors;
                    for candidate in results.candidates.iter().take(5) {
                        match client.fetch(candidate).and_then(|cover| {
                            let image = cover.image()?;
                            Ok((cover, image))
                        }) {
                            Ok(pair) => return Ok(Some(pair)),
                            Err(error) => errors.push(format!("{}：{error:#}", candidate.source)),
                        }
                    }
                    anyhow::bail!("没有找到可用的封面：{}", errors.join("；"))
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.cover_search.generation != generation {
                    return;
                }
                match result {
                    Ok(Some((cover, image))) => {
                        let saved = cover.clone();
                        let identity = this.media_identity.clone();
                        this.apply_online_cover(cover, image, cx);
                        this.cover_search.task = Some(cx.spawn(async move |this, cx| {
                            let result = cx
                                .background_executor()
                                .spawn(async move {
                                    if let Some(identity) = identity {
                                        wmc_enrichment::cache::MediaCache::default_location()?
                                            .save_cover(&identity, &saved)?;
                                    }
                                    Ok::<_, anyhow::Error>(())
                                })
                                .await;
                            let _ = this.update(cx, |this, cx| {
                                if this.cover_search.generation == generation {
                                    if let Err(error) = result {
                                        this.status = Some(format!("封面缓存保存失败：{error:#}"));
                                    }
                                    cx.notify();
                                }
                            });
                        }));
                    }
                    Err(error) => eprintln!("封面缓存不可用：{error:#}"),
                    _ => {}
                }
                cx.notify();
            });
        }));
    }
    fn apply_online_cover(
        &mut self,
        cover: DownloadedCover,
        image: image::RgbaImage,
        cx: &mut Context<Self>,
    ) {
        self.cover_search.art_generation += 1;
        self.cover_search.source = Some(format!(
            "封面：{} · {} · {}",
            cover.candidate.source, cover.candidate.album, cover.candidate.artist
        ));
        self.status = self.cover_search.source.clone();
        self.app
            .update_cover(&self.title, &self.artist, &self.online_query.album, &image);
        let backdrop = Backdrop::from_cover(&image);
        let old = std::mem::replace(&mut self.cover, resources::render_image(image));
        self.background.retire(old);
        self.background.replace_backdrop(backdrop);
        cx.notify();
    }
}

#[derive(Default)]
pub(in crate::view) struct SearchState {
    pub candidates: Vec<(CoverCandidate, Option<std::sync::Arc<RenderImage>>)>,
    pub open: bool,
    pub busy: bool,
    pub generation: u64,
    pub art_generation: u64,
    pub message: String,
    pub source: Option<String>,
    pub task: Option<Task<()>>,
}

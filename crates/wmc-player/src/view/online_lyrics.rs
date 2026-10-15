use super::{PlayerView, lyrics};
use crate::{
    online_lyrics::{Candidate, DownloadedLyrics, LyricCache, LyricsClient},
    presentation::LyricsMotion,
};
use gpui_kit::*;

impl PlayerView {
    pub(super) fn search_online_lyrics(&mut self, cx: &mut Context<Self>) {
        self.show_more = false;
        self.settings.info_open = false;
        if self.cover_open {
            self.close_cover_search();
        }
        self.online_generation += 1;
        let generation = self.online_generation;
        let query = self.online_query.clone();
        self.online_open = true;
        self.online_busy = true;
        self.online_candidates.clear();
        self.online_message = format!("正在搜索 {}…", query.text());
        self._online_task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { LyricsClient::new()?.search(&query) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.online_generation != generation {
                    return;
                }
                this.online_busy = false;
                match result {
                    Ok(results) => {
                        this.online_message = if results.candidates.is_empty() {
                            "没有找到匹配歌曲".into()
                        } else {
                            format!("{} 个结果 · 点击选择歌词", results.candidates.len())
                        };
                        if !results.errors.is_empty() {
                            this.online_message
                                .push_str(&format!("\n{}", results.errors.join("\n")));
                        }
                        this.online_candidates = results.candidates;
                    }
                    Err(error) => this.online_message = format!("搜索失败：{error:#}"),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    pub(super) fn select_online_lyrics(&mut self, candidate: Candidate, cx: &mut Context<Self>) {
        self.online_generation += 1;
        let generation = self.online_generation;
        let query = self.online_query.clone();
        let identity = self.media_identity.clone();
        let duration = self.audio.duration();
        self.online_busy = true;
        self.online_message = format!(
            "正在获取 {} · {}…",
            candidate.provider.name(),
            candidate.title
        );
        self._online_task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let downloaded = LyricsClient::new()?.fetch(&candidate)?;
                    let parsed = downloaded.parse(duration)?;
                    Ok::<_, anyhow::Error>((downloaded, parsed))
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.online_generation != generation {
                    return;
                }
                this.online_busy = false;
                match result {
                    Ok((downloaded, parsed)) => {
                        // Persist only after this response has passed the generation
                        // check. An obsolete download must not change the saved choice.
                        let saved = downloaded.clone();
                        this._online_task = Some(cx.spawn(async move |this, cx| {
                            let result = cx
                                .background_executor()
                                .spawn(async move {
                                    let identity = identity.map(Ok).unwrap_or_else(|| {
                                        crate::media_cache::MediaIdentity::read(&query, None)
                                    })?;
                                    crate::media_cache::MediaCache::default_location()?
                                        .save_lyrics(&identity, &saved)
                                })
                                .await;
                            let _ = this.update(cx, |this, cx| {
                                if this.online_generation == generation {
                                    if let Err(error) = result {
                                        this.status = Some(format!("缓存保存失败：{error:#}"));
                                    }
                                    cx.notify();
                                }
                            });
                        }));
                        this.apply_online_lyrics(downloaded, parsed, cx);
                        this.status = this.lyric_source.clone();
                        this.online_open = false;
                    }
                    Err(error) => {
                        this.online_message = format!("获取失败：{error:#}\n可以选择其他来源或版本")
                    }
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    pub(super) fn load_cached_lyrics(&mut self, network_needed: bool, cx: &mut Context<Self>) {
        let query = self.online_query.clone();
        let identity = self.media_identity.clone();
        let duration = self.audio.duration();
        let generation = self.online_generation;
        self._online_task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let identity = identity
                        .map(Ok)
                        .unwrap_or_else(|| crate::media_cache::MediaIdentity::read(&query, None))?;
                    let indexed = crate::media_cache::MediaCache::default_location()
                        .and_then(|cache| cache.load_lyrics(&identity))
                        .unwrap_or_else(|error| {
                            eprintln!("歌词索引读取失败：{error:#}");
                            None
                        });
                    let cached = indexed.or_else(|| {
                        LyricCache::default_location()
                            .and_then(|cache| cache.load(&query))
                            .ok()
                            .flatten()
                    });
                    if let Some(downloaded) = cached
                        && let Ok(parsed) = downloaded.parse(duration)
                    {
                        return Ok(Some((downloaded, parsed)));
                    }
                    if !network_needed {
                        return Ok(None);
                    }
                    let client = LyricsClient::new()?;
                    let results = client.search(&query)?;
                    let mut errors = results.errors;
                    let mut line_fallback = None;
                    for candidate in results.candidates.iter().filter(|candidate| candidate.matches_recording(&query)).take(5) {
                        match client.fetch(candidate).and_then(|downloaded| {
                            let parsed = downloaded.parse(duration)?;
                            Ok((downloaded, parsed))
                        }) {
                            Ok(pair) => {
                                if pair.1.lines.iter().any(|line| line.word_synced) { return Ok(Some(pair)); }
                                if line_fallback.is_none() { line_fallback = Some(pair); }
                            }
                            Err(error) => {
                                errors.push(format!("{}：{error:#}", candidate.provider.name()))
                            }
                        }
                    }
                    if line_fallback.is_some() { return Ok(line_fallback); }
                    anyhow::bail!("没有找到可用的歌词：{}", errors.join("；"))
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.online_generation != generation {
                    return;
                }
                match result {
                    Ok(Some((downloaded, parsed))) => {
                        let saved = downloaded.clone();
                        let identity = this.media_identity.clone();
                        let query = this.online_query.clone();
                        this.apply_online_lyrics(downloaded, parsed, cx);
                        this._online_task = Some(cx.spawn(async move |this, cx| {
                            let result = cx
                                .background_executor()
                                .spawn(async move {
                                    if let Some(identity) = identity {
                                        let cache = crate::media_cache::MediaCache::default_location()?;
                                        cache.save_lyrics(&identity, &saved)?;
                                        if saved.candidate.provider != crate::online_lyrics::Provider::Local
                                            && !saved.parse(duration)?.lines.iter().any(|line| line.word_synced)
                                            && cache.word_lyrics_upgrade_due(&identity)? {
                                            match LyricsClient::new()?.upgrade(&query, &saved, duration) {
                                                Ok(upgraded) => {
                                                    cache.mark_word_lyrics_checked(&identity)?;
                                                    if let Some(upgraded) = upgraded {
                                                        let parsed = upgraded.parse(duration)?;
                                                        return Ok(Some((upgraded, parsed, identity)));
                                                    }
                                                }
                                                Err(error) => eprintln!("逐字歌词升级失败，保留缓存：{error:#}"),
                                            }
                                        }
                                    }
                                    Ok::<_, anyhow::Error>(None)
                                })
                                .await;
                            let _ = this.update(cx, |this, cx| {
                                if this.online_generation == generation {
                                    match result {
                                        Ok(Some((upgraded, parsed, identity))) => {
                                            this.apply_online_lyrics(upgraded.clone(), parsed, cx);
                                            this._online_task = Some(cx.spawn(async move |this, cx| {
                                                let result = cx.background_executor().spawn(async move {
                                                    crate::media_cache::MediaCache::default_location()?.save_lyrics(&identity, &upgraded)
                                                }).await;
                                                let _ = this.update(cx, |this, cx| {
                                                    if this.online_generation == generation {
                                                        if let Err(error) = result { this.status = Some(format!("逐字歌词缓存保存失败：{error:#}")); }
                                                        cx.notify();
                                                    }
                                                });
                                            }));
                                        }
                                        Err(error) => this.status = Some(format!("歌词缓存保存失败：{error:#}")),
                                        _ => {},
                                    }
                                    cx.notify();
                                }
                            });
                        }));
                    }
                    Err(error) => this.status = Some(format!("歌词自动加载失败：{error:#}")),
                    _ => {}
                }
                cx.notify();
            });
        }));
    }

    pub(super) fn apply_online_lyrics(
        &mut self,
        downloaded: DownloadedLyrics,
        parsed: crate::lyrics::Lyrics,
        cx: &mut Context<Self>,
    ) {
        self.settings.set_document(&downloaded,cx);
        self.translation.invalidate();
        self.lyric_source = Some(format!(
            "歌词：{} · {} · {} · ID {}",
            downloaded.candidate.provider.name(),
            downloaded.candidate.artists.join(" / "),
            downloaded.format,
            downloaded.candidate.id
        ));
        let text = parsed
            .lines
            .iter()
            .map(|line| line.text())
            .collect::<String>();
        self.motion = LyricsMotion::new(parsed.lines.len());
        self.lyric_freeze = None;
        self.layouts = lyrics::layout::LayoutCache::default();
        self.retired_lyrics.push(std::mem::replace(&mut self.lyric_images,
            lyrics::cache::LineImageCache::new(None, parsed.lines.len())));
        self.lyrics = parsed;
        self.restore_translation_cache(cx);
        self._font_loader = cx.spawn(async move |this, cx| {
            let font = cx
                .background_executor()
                .spawn(async move { lyrics::font::SongFont::load(&text) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.lyric_images.set_font(font);
                cx.notify();
            });
        });
    }

}

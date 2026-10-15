use super::HomeView;
use gpui_kit::{prelude::*, *};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use wmc_core::LibraryKind;
use wmc_library::{LibraryConfig, MediaFile, MediaKind};

pub(super) const PAGE_SIZE: usize = 24;
pub(super) struct LibraryItem {
    pub file: MediaFile,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub cover: Option<Arc<RenderImage>>,
    pub photo_time: Option<i64>,
}
pub(super) struct LibraryState {
    pub config: LibraryConfig,
    pub draft: LibraryConfig,
    pub items: Vec<LibraryItem>,
    pub loading: bool,
    pub error: Option<String>,
    pub skipped: usize,
    pub generation: u64,
    pub selection: [usize; 3],
    pub pages: [usize; 3],
    pub hovered: Option<usize>,
    pub pointer_navigation: bool,
    pub loading_track: bool,
    pub play_generation: u64,
    pub cancelled_scan: Arc<AtomicBool>,
}
impl Drop for LibraryState {
    fn drop(&mut self) {
        self.cancelled_scan.store(true, Ordering::Relaxed);
    }
}
impl LibraryState {
    pub fn new() -> Self {
        let result = LibraryConfig::location().and_then(|path| LibraryConfig::load(&path));
        let (config, error) = match result {
            Ok(config) => (config, None),
            Err(error) => (LibraryConfig::default(), Some(error.to_string())),
        };
        Self {
            draft: config.clone(),
            config,
            items: Vec::new(),
            loading: false,
            error,
            skipped: 0,
            generation: 0,
            selection: [0; 3],
            pages: [0; 3],
            hovered: None,
            pointer_navigation: false,
            loading_track: false,
            play_generation: 0,
            cancelled_scan: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn indices(&self, kind: LibraryKind, tab: usize) -> Vec<usize> {
        let kind = media_kind(kind);
        let mut indices: Vec<_> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.file.kind == kind)
            .map(|(index, _)| index)
            .collect();
        if kind == MediaKind::Photos {
            // Chronological grid: oldest at top, newest at bottom-right.
            indices.sort_by_cached_key(|&index| {
                (
                    self.items[index].photo_time,
                    self.items[index].file.path.clone(),
                )
            });
        } else if kind == MediaKind::Music {
            indices.sort_by_cached_key(|&index| {
                let item = &self.items[index];
                let group = match tab {
                    0 => item.album.as_deref(),
                    1 | 7 => item.artist.as_deref(),
                    _ => None,
                };
                (
                    group.unwrap_or_default().to_lowercase(),
                    item.title.to_lowercase(),
                )
            });
        }
        indices
    }
}
pub(super) fn media_kind(kind: LibraryKind) -> MediaKind {
    match kind {
        LibraryKind::Photos => MediaKind::Photos,
        LibraryKind::Music => MediaKind::Music,
        LibraryKind::Videos => MediaKind::Videos,
    }
}

#[cfg(test)]
mod tests {
    use super::{LibraryItem, LibraryState};
    use wmc_core::LibraryKind;
    use wmc_library::{MediaFile, MediaKind};
    #[test]
    fn photo_grid_orders_by_time_with_newest_last_and_stable_ties() {
        let mut library = LibraryState::new();
        library.items = [
            (Some(2000), "a"),
            (Some(1000), "z"),
            (None, "b"),
            (Some(1000), "c"),
        ]
        .into_iter()
        .map(|(photo_time, name)| LibraryItem {
            file: MediaFile {
                path: name.into(),
                name: name.into(),
                kind: MediaKind::Photos,
            },
            title: name.into(),
            artist: None,
            album: None,
            cover: None,
            photo_time,
        })
        .collect();
        assert_eq!(library.indices(LibraryKind::Photos, 0), vec![2, 3, 1, 0]);
    }
}

impl HomeView {
    pub(super) fn rescan_library(&mut self, cx: &mut Context<Self>) {
        self.context_menu = None;
        self.media_bounds.clear();
        self.library.hovered = None;
        self.library.pointer_navigation = false;
        self.library.cancelled_scan.store(true, Ordering::Relaxed);
        self.library.cancelled_scan = Arc::new(AtomicBool::new(false));
        let cancelled = self.library.cancelled_scan.clone();
        self.library.generation += 1;
        let generation = self.library.generation;
        let config = self.library.config.clone();
        self.library.loading = true;
        self.library.items.clear();
        self.library.pages = [0; 3];
        self.library.selection = [0; 3];
        cx.spawn(async move |this, cx| {
            let (items, skipped, warning) = cx
                .background_executor()
                .spawn(async move {
                    let scan = wmc_library::scan_cancellable(&config, &cancelled);
                    let items = scan
                        .files
                        .into_iter()
                        .take_while(|_| !cancelled.load(Ordering::Relaxed))
                        .map(|file| {
                            let tags = if file.kind == MediaKind::Music {
                                wmc_player::metadata::TrackMetadata::read(&file.path).ok()
                            } else {
                                None
                            };
                            let title = tags
                                .as_ref()
                                .and_then(|tags| tags.title.clone())
                                .unwrap_or_else(|| file.name.clone());
                            let artist = tags.as_ref().and_then(|tags| tags.artist.clone());
                            let album = tags.as_ref().and_then(|tags| tags.album.clone());
                            let cover = tags
                                .and_then(|tags| tags.cover)
                                .and_then(|data| image::load_from_memory(&data).ok())
                                .map(|image| {
                                    let mut image = image.thumbnail(160, 160).to_rgba8();
                                    for pixel in image.pixels_mut() {
                                        pixel.0.swap(0, 2);
                                    }
                                    Arc::new(RenderImage::new(vec![image::Frame::new(image)]))
                                });
                            LibraryItem {
                                photo_time: if file.kind == MediaKind::Photos {
                                    super::photos::photo_time(&file.path)
                                } else {
                                    None
                                },
                                file,
                                title,
                                artist,
                                album,
                                cover,
                            }
                        })
                        .collect();
                    (items, scan.skipped, scan.warnings.into_iter().next())
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.library.generation != generation {
                    return;
                }
                this.library.items = items;
                this.library.loading = false;
                this.library.skipped = skipped;
                if warning.is_some() {
                    this.library.error = warning;
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn add_media_folder(&mut self, kind: MediaKind, cx: &mut Context<Self>) {
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: true,
            prompt: Some(format!("添加{}文件夹", kind.label()).into()),
        });
        cx.spawn(async move |this, cx| {
            let result = prompt.await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(Ok(Some(paths))) => {
                        for path in paths {
                            if let Err(error) = this.library.draft.add(kind, path) {
                                this.library.error = Some(error.to_string());
                            }
                        }
                    }
                    Ok(Ok(None)) => {}
                    Ok(Err(error)) => this.library.error = Some(error.to_string()),
                    Err(error) => this.library.error = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }
    pub(super) fn save_media_folders(&mut self, cx: &mut Context<Self>) {
        match LibraryConfig::location().and_then(|path| self.library.draft.save(&path)) {
            Ok(()) => {
                self.library.config = self.library.draft.clone();
                self.library.error = None;
                self.rescan_library(cx);
                self.back(cx);
            }
            Err(error) => {
                self.library.error = Some(format!("保存失败：{error:#}"));
                cx.notify();
            }
        }
    }
    pub(super) fn open_media(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.library.items.get(index) else {
            return;
        };
        let path = item.file.path.clone();
        match item.file.kind {
            MediaKind::Photos => {
                self.photo = Some(path);
                cx.notify();
            }
            MediaKind::Videos => match url::Url::from_file_path(&path) {
                Ok(url) => cx.open_url(url.as_str()),
                Err(()) => {
                    self.library.error = Some("无法打开视频路径".into());
                    cx.notify();
                }
            },
            MediaKind::Music => self.load_song(path, window, cx),
        }
    }
    pub(super) fn load_song(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let index = self
            .song_queue
            .iter()
            .position(|(queued, _)| queued == &path)
            .unwrap_or_else(|| {
                let title = self
                    .library
                    .items
                    .iter()
                    .find(|item| item.file.path == path)
                    .map(|item| item.title.clone())
                    .unwrap_or_else(|| {
                        path.file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned()
                    });
                self.song_queue.push((path, title));
                self.song_queue.len() - 1
            });
        self.load_queue_song(index, true, window, cx);
    }

    pub(super) fn load_queue_song(
        &mut self,
        index: usize,
        foreground: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((path, _)) = self.song_queue.get(index) else {
            return;
        };
        let path = path.clone();
        let open_player = foreground && !self.player_visible;
        self.library.play_generation += 1;
        let generation = self.library.play_generation;
        self.library.loading_track = true;
        let started = std::time::Instant::now();
        self.song_transition = foreground.then_some(started);
        self.library_return = None;
        let reduced = cx.reduce_motion();
        self.library.error = None;
        cx.spawn_in(window, async move |this, cx| {
            let prepared = cx
                .background_executor()
                .spawn(async move {
                    wmc_player::PlayerConfig {
                        audio: Some(path),
                        ..Default::default()
                    }
                    .prepare()
                })
                .await;
            if foreground && !reduced && prepared.is_ok() {
                let duration = std::time::Duration::from_secs_f32(
                    wmc_motion::player_entry::LIBRARY_EXIT_SECONDS,
                );
                if let Some(remaining) = duration.checked_sub(started.elapsed()) {
                    cx.background_executor().timer(remaining).await;
                }
            }
            let _ = this.update_in(cx, |this, window, cx| {
                if this.library.play_generation != generation {
                    return;
                }
                this.library.loading_track = false;
                this.song_transition = None;
                match prepared {
                    Ok(track) => {
                        if let Some(old) = &this.player {
                            old.update(cx, |player, _| player.deactivate());
                        }
                        let player = cx.new(|cx| {
                            wmc_player::PlayerView::from_prepared(track, false, window, cx)
                        });
                        player.update(cx, |player, _| player.play());
                        this.player_subscription = Some(cx.subscribe_in(
                            &player,
                            window,
                            |this, _, event: &wmc_player::PlayerEvent, window, cx| match event {
                                wmc_player::PlayerEvent::Back => this.return_from_player(cx),
                                wmc_player::PlayerEvent::Queue => this.show_queue(cx),
                                wmc_player::PlayerEvent::Previous => {
                                    this.step_queue(false, window, cx)
                                }
                                wmc_player::PlayerEvent::Next => this.step_queue(true, window, cx),
                            },
                        ));
                        this.player = Some(player);
                        this.player_visible = open_player || this.player_visible;
                        if this.player_visible {
                            this.player_exit = None;
                        }
                        this.queue_index = Some(index);
                        if !this.queue_played.contains(&index) {
                            this.queue_played.push(index);
                        }
                        this.playback_finished = false;
                        if !this.player_visible {
                            window.focus(&this.focus, cx);
                        }
                    }
                    Err(error) => this.library.error = Some(format!("歌曲加载失败：{error:#}")),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}

mod view;
use crate::navigation::LibraryKind;
use crate::shell::AppShell;
use gpui_kit::{prelude::*, *};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use wmc_library::{LibraryConfig, MediaKind};

pub(crate) const PAGE_SIZE: usize = 24;
pub(crate) struct MusicFolder {
    pub key: wmc_library::music::GroupKey,
    pub selection: usize,
    pub page: usize,
    pub offset: Point<Pixels>,
}
pub(crate) struct LibraryItem {
    pub entry: wmc_library::LibraryEntry,
    pub cover: Option<Arc<RenderImage>>,
}
enum ScanUpdate {
    Entries(Vec<LibraryItem>),
    Finished {
        skipped: usize,
        warning: Option<String>,
    },
}
impl std::ops::Deref for LibraryItem {
    type Target = wmc_library::LibraryEntry;
    fn deref(&self) -> &Self::Target {
        &self.entry
    }
}
pub(crate) struct LibraryState {
    pub list: bool,
    pub music_folder: Option<MusicFolder>,
    pub bounds: std::collections::HashMap<usize, Bounds<Pixels>>,
    pub config: LibraryConfig,
    pub draft: LibraryConfig,
    pub items: Vec<LibraryItem>,
    pub loading: bool,
    pub error: Option<String>,
    pub skipped: usize,
    pub generation: u64,
    pub selection: [usize; 3],
    pub pages: [usize; 3],
    pub scroll: [ScrollHandle; 3],
    pub hovered: Option<usize>,
    pub pointer_navigation: bool,
    pub cancelled_scan: Arc<AtomicBool>,
    metadata_cache: Arc<wmc_library::MetadataCache>,
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
            list: false,
            music_folder: None,
            bounds: std::collections::HashMap::new(),
            draft: config.clone(),
            config,
            items: Vec::new(),
            loading: false,
            error,
            skipped: 0,
            generation: 0,
            selection: [0; 3],
            pages: [0; 3],
            scroll: std::array::from_fn(|_| ScrollHandle::new()),
            hovered: None,
            pointer_navigation: false,
            cancelled_scan: Arc::new(AtomicBool::new(false)),
            metadata_cache: Arc::new(wmc_library::MetadataCache::default()),
        }
    }
    pub fn indices(&self, kind: LibraryKind, tab: usize) -> Vec<usize> {
        if kind == LibraryKind::Music {
            let items: Vec<_> = self.items.iter().map(|item| &item.entry).collect();
            let category = wmc_library::music::Category::from_tab(tab);
            if self.music_is_grouped(tab) {
                return wmc_library::music::groups(&items, category)
                    .iter()
                    .map(|group| group.tracks[0])
                    .collect();
            }
            return wmc_library::music::tracks(&items, category, self.music_group(tab));
        }
        wmc_library::query::indices(self.items.iter().map(|item| &item.entry), kind, tab)
    }
    pub fn music_group(&self, tab: usize) -> Option<&wmc_library::music::GroupKey> {
        self.music_folder
            .as_ref()
            .filter(|folder| folder.key.category == wmc_library::music::Category::from_tab(tab))
            .map(|folder| &folder.key)
    }
    pub fn music_is_grouped(&self, tab: usize) -> bool {
        wmc_library::music::Category::from_tab(tab) != wmc_library::music::Category::Songs
            && self.music_group(tab).is_none()
    }
}
pub(crate) fn media_kind(kind: LibraryKind) -> MediaKind {
    match kind {
        LibraryKind::Photos => MediaKind::Photos,
        LibraryKind::Music => MediaKind::Music,
        LibraryKind::Videos => MediaKind::Videos,
    }
}

impl AppShell {
    pub(crate) fn open_library_item(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.destination_state.kind
            == crate::navigation::DestinationKind::Library(LibraryKind::Music)
            && self.library.music_is_grouped(self.destination_state.tab)
        {
            let items: Vec<_> = self.library.items.iter().map(|item| &item.entry).collect();
            let category = wmc_library::music::Category::from_tab(self.destination_state.tab);
            if let Some(group) = wmc_library::music::groups(&items, category)
                .into_iter()
                .find(|group| group.tracks[0] == index)
            {
                self.library.music_folder = Some(MusicFolder {
                    key: group.key,
                    selection: self.library.selection[1],
                    page: self.library.pages[1],
                    offset: self.library.scroll[1].offset(),
                });
                self.library.selection[1] = 0;
                self.library.pages[1] = 0;
                self.library.scroll[1].set_offset(point(px(0.), px(0.)));
                self.library.hovered = None;
                self.library.bounds.clear();
                self.destination_state.focus = crate::navigation::DestinationFocus::Content;
                cx.notify();
                return;
            }
        }
        self.open_media(index, window, cx);
    }

    pub(crate) fn leave_music_group(&mut self, cx: &mut Context<Self>) -> bool {
        if self.destination_state.kind
            != crate::navigation::DestinationKind::Library(LibraryKind::Music)
        {
            return false;
        }
        let Some(folder) = self.library.music_folder.take() else {
            return false;
        };
        self.library.selection[1] = folder.selection;
        self.library.pages[1] = folder.page;
        self.library.scroll[1].set_offset(folder.offset);
        self.library.hovered = None;
        self.library.bounds.clear();
        cx.notify();
        true
    }

    pub(crate) fn rescan_library(&mut self, cx: &mut Context<Self>) {
        self.library.music_folder = None;
        self.context_menu = None;
        self.library.bounds.clear();
        self.library.hovered = None;
        self.library.pointer_navigation = false;
        self.library.cancelled_scan.store(true, Ordering::Relaxed);
        self.library.cancelled_scan = Arc::new(AtomicBool::new(false));
        let cancelled = self.library.cancelled_scan.clone();
        self.library.generation += 1;
        let generation = self.library.generation;
        let config = self.library.config.clone();
        let metadata_cache = self.library.metadata_cache.clone();
        self.library.loading = true;
        let retired: Vec<_> = self
            .library
            .items
            .drain(..)
            .filter_map(|item| item.cover)
            .collect();
        cx.defer(move |cx| {
            for image in retired {
                cx.drop_image(image, None);
            }
        });
        self.library.pages = [0; 3];
        for scroll in &self.library.scroll {
            scroll.set_offset(point(px(0.), px(0.)));
        }
        self.library.selection = [0; 3];
        let (sender, receiver) = async_channel::bounded(2);
        cx.background_executor()
            .spawn(async move {
                let scan =
                    wmc_library::inspect_stream(&config, &cancelled, &metadata_cache, |entries| {
                        let items = entries
                            .into_iter()
                            .map(|mut entry| {
                                let cover =
                                    entry.thumbnail.take().map(wmc_design::images::render_image);
                                LibraryItem { entry, cover }
                            })
                            .collect();
                        sender.send_blocking(ScanUpdate::Entries(items)).is_ok()
                    });
                let _ = sender.send_blocking(ScanUpdate::Finished {
                    skipped: scan.skipped,
                    warning: scan.warning,
                });
            })
            .detach();
        cx.spawn(async move |this, cx| {
            while let Ok(update) = receiver.recv().await {
                let keep_receiving = this.update(cx, |this, cx| {
                    if this.library.generation != generation {
                        return false;
                    }
                    match update {
                        ScanUpdate::Entries(items) => {
                            let at_bottom =
                                this.photo_page.scroll.is_scrolled_to_end().unwrap_or(true);
                            this.library.items.extend(items);
                            this.library.bounds.clear();
                            this.library.hovered = None;
                            if at_bottom {
                                this.photo_page.scroll.scroll_to_bottom();
                            }
                        }
                        ScanUpdate::Finished { skipped, warning } => {
                            this.library.loading = false;
                            this.library.skipped = skipped;
                            if warning.is_some() {
                                this.library.error = warning;
                            }
                        }
                    }
                    cx.notify();
                    true
                });
                if !matches!(keep_receiving, Ok(true)) {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
    }

    pub(crate) fn add_media_folder(&mut self, kind: MediaKind, cx: &mut Context<Self>) {
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
    pub(crate) fn save_media_folders(&mut self, cx: &mut Context<Self>) {
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
    pub(crate) fn open_media(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.library.items.get(index) else {
            return;
        };
        let path = item.file.path.clone();
        match item.file.kind {
            MediaKind::Photos => {
                self.photo_page.path = Some(path);
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
}

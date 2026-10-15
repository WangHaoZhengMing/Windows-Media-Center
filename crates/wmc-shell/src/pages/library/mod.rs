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
pub(crate) struct LibraryItem {
    pub entry: wmc_library::LibraryEntry,
    pub cover: Option<Arc<RenderImage>>,
}
impl std::ops::Deref for LibraryItem {
    type Target = wmc_library::LibraryEntry;
    fn deref(&self) -> &Self::Target {
        &self.entry
    }
}
pub(crate) struct LibraryState {
    pub list: bool,
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
    pub hovered: Option<usize>,
    pub pointer_navigation: bool,
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
            list: false,
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
            hovered: None,
            pointer_navigation: false,
            cancelled_scan: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn indices(&self, kind: LibraryKind, tab: usize) -> Vec<usize> {
        wmc_library::query::indices(self.items.iter().map(|item| &item.entry), kind, tab)
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
    pub(crate) fn rescan_library(&mut self, cx: &mut Context<Self>) {
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
        self.library.selection = [0; 3];
        cx.spawn(async move |this, cx| {
            let (items, skipped, warning) = cx
                .background_executor()
                .spawn(async move {
                    let scan = wmc_library::inspect(&config, &cancelled);
                    let items = scan
                        .entries
                        .into_iter()
                        .map(|mut entry| {
                            let cover =
                                entry.thumbnail.take().map(wmc_design::images::render_image);
                            LibraryItem { entry, cover }
                        })
                        .collect();
                    (items, scan.skipped, scan.warning)
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

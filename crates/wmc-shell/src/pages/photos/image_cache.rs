//! Page-scoped photo pixels. Never pass original photo paths to GPUI's global cache.
use crate::shell::AppShell;
use gpui_kit::*;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub(crate) const THUMB: SizeKind = SizeKind::Square;
pub(crate) const DETAIL: SizeKind = SizeKind::DetailSquare;

pub use wmc_media::photos::decode::SizeKind;
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Key {
    path: PathBuf,
    size: SizeKind,
    generation: u64,
}

pub(crate) struct PhotoImages {
    wanted: Vec<Key>,
    images: HashMap<Key, Result<Arc<RenderImage>, String>>,
    inflight: HashMap<Key, Arc<AtomicBool>>,
}

impl Drop for PhotoImages {
    fn drop(&mut self) {
        for cancelled in self.inflight.values() {
            cancelled.store(true, Ordering::Relaxed);
        }
        // CPU pixel buffers are owned by images and drop with this object.
        // The GPUI release observer below explicitly releases renderer textures.
    }
}

impl PhotoImages {
    pub fn new(cx: &mut App) -> Entity<Self> {
        let entity = cx.new(|_| Self {
            wanted: Vec::new(),
            images: HashMap::new(),
            inflight: HashMap::new(),
        });
        cx.observe_release(&entity, |cache, cx| {
            for cancelled in cache.inflight.values() {
                cancelled.store(true, Ordering::Relaxed);
            }
            for (_, image) in cache.images.drain() {
                if let Ok(image) = image {
                    cx.drop_image(image, None);
                }
            }
        })
        .detach();
        entity
    }

    fn sync(&mut self, wanted: Vec<Key>, window: &mut Window, cx: &mut Context<Self>) {
        let active: HashSet<_> = wanted.iter().cloned().collect();
        self.evict_except(&active, |image| cx.drop_image(image, Some(window)));
        for (key, cancelled) in &self.inflight {
            if !active.contains(key) {
                cancelled.store(true, Ordering::Relaxed);
            }
        }
        self.wanted = wanted;
        self.pump(cx);
    }

    fn evict_except(&mut self, active: &HashSet<Key>, mut release: impl FnMut(Arc<RenderImage>)) {
        self.images.retain(|key, image| {
            if active.contains(key) {
                return true;
            }
            if let Ok(image) = image {
                release(image.clone());
            }
            false
        });
    }

    fn pump(&mut self, cx: &mut Context<Self>) {
        // Keep cancelled jobs in the count until their pixel buffers are released.
        while self.inflight.len() < 2 {
            let Some(key) = self
                .wanted
                .iter()
                .find(|key| !self.images.contains_key(*key) && !self.inflight.contains_key(*key))
                .cloned()
            else {
                return;
            };
            let cancelled = Arc::new(AtomicBool::new(false));
            self.inflight.insert(key.clone(), cancelled.clone());
            cx.spawn(async move |this, cx| {
                let task_key = key.clone();
                let task_cancelled = cancelled.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        wmc_media::photos::decode::decode(
                            &task_key.path,
                            task_key.size,
                            &task_cancelled,
                        )
                    })
                    .await;
                let _ = this.update(cx, |cache, cx| {
                    cache.inflight.remove(&key);
                    if !cancelled.load(Ordering::Relaxed) && cache.wanted.contains(&key) {
                        // Only small, still-needed pixels become renderer resources.
                        let image = result.map(wmc_design::images::render_image);
                        cache.images.insert(key, image);
                        cx.notify();
                    }
                    // A completed or cancelled decode releases one bounded worker slot.
                    cache.pump(cx);
                });
            })
            .detach();
        }
    }
}

impl AppShell {
    pub(crate) fn photo_size(window: &Window) -> SizeKind {
        let viewport = window.viewport_size();
        let physical =
            |n: Pixels| ((f32::from(n) * window.scale_factor()).ceil() as u32).clamp(1, 2048);
        SizeKind::Fit(physical(viewport.width), physical(viewport.height))
    }

    pub(crate) fn sync_photos(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let generation = self.library.generation;
        let mut wanted = Vec::new();
        let mut add = |path: &PathBuf, size| {
            wanted.push(Key {
                path: path.clone(),
                size,
                generation,
            })
        };
        if let Some(path) = &self.photo_page.path {
            add(path, Self::photo_size(window));
        } else if !self.player_page.visible
            && self.home_page.navigation.opened().is_some()
            && let crate::navigation::DestinationKind::Library(
                crate::navigation::LibraryKind::Photos,
            ) = self.destination_state.kind
        {
            let indices = self.library.indices(
                crate::navigation::LibraryKind::Photos,
                self.destination_state.tab,
            );
            if !indices.is_empty() && self.photo_page.grid_generation != Some(generation) {
                self.photo_page.grid_generation = Some(generation);
                self.library.selection[0] = indices.len() - 1;
                self.photo_page.scroll.scroll_to_bottom();
                self.photo_page.visible = indices.len().saturating_sub(25)..indices.len();
            }
            for &index in indices
                .iter()
                .skip(self.photo_page.visible.start)
                .take(self.photo_page.visible.len())
            {
                add(&self.library.items[index].file.path, THUMB);
            }
        }
        if let Some(details) = &self.details
            && details.kind == wmc_library::MediaKind::Photos
        {
            add(&details.path, DETAIL);
        }
        if let Some(item) = self
            .context_menu
            .as_ref()
            .and_then(|menu| menu.target)
            .and_then(|index| self.library.items.get(index))
            && item.file.kind == wmc_library::MediaKind::Photos
        {
            add(&item.file.path, THUMB);
        }
        self.photo_page
            .images
            .update(cx, |cache, cx| cache.sync(wanted, window, cx));
    }

    pub(crate) fn photo_image(
        &self,
        path: &Path,
        size: SizeKind,
        cx: &App,
    ) -> Option<Arc<RenderImage>> {
        self.photo_page
            .images
            .read(cx)
            .images
            .get(&Key {
                path: path.to_path_buf(),
                size,
                generation: self.library.generation,
            })
            .and_then(|result| result.as_ref().ok())
            .cloned()
    }

    pub(crate) fn photo_error(&self, path: &Path, size: SizeKind, cx: &App) -> Option<String> {
        self.photo_page
            .images
            .read(cx)
            .images
            .get(&Key {
                path: path.to_path_buf(),
                size,
                generation: self.library.generation,
            })
            .and_then(|result| result.as_ref().err())
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;
    #[test]
    fn eviction_releases_pixels_and_requests_texture_drop() {
        let key = Key {
            path: PathBuf::from("previous-page"),
            size: THUMB,
            generation: 0,
        };
        let image = Arc::new(RenderImage::new(vec![image::Frame::new(
            image::RgbaImage::new(256, 256),
        )]));
        let weak = Arc::downgrade(&image);
        let mut cache = PhotoImages {
            wanted: Vec::new(),
            images: HashMap::from([(key.clone(), Ok(image))]),
            inflight: HashMap::new(),
        };
        let mut released = 0;
        cache.evict_except(&HashSet::from([key]), |_| released += 1);
        assert_eq!(released, 0);
        assert!(weak.upgrade().is_some());
        cache.evict_except(&HashSet::new(), |_| released += 1);
        assert_eq!(released, 1);
        assert!(weak.upgrade().is_none());
        assert!(cache.images.is_empty());
    }

    #[test]
    #[ignore = "Reads WMC_PHOTO_PROBE_ROOT and decodes its first page sequentially"]
    fn real_photo_page_uses_six_mib_and_drops_pixels() {
        let root =
            std::env::var_os("WMC_PHOTO_PROBE_ROOT").expect("explicit photo probe directory");
        let config = wmc_library::LibraryConfig {
            version: 1,
            photos: vec![root.into()],
            music: Vec::new(),
            videos: Vec::new(),
        };
        let files = wmc_library::scan(&config).files;
        let mut original_bytes = 0_u64;
        let mut thumbnail_bytes = 0;
        let mut images = Vec::new();
        for file in files.iter().take(crate::pages::library::PAGE_SIZE) {
            let (w, h) = image::image_dimensions(&file.path).unwrap();
            original_bytes += u64::from(w) * u64::from(h) * 4;
            let key = Key {
                path: file.path.clone(),
                size: THUMB,
                generation: 0,
            };
            let pixels =
                wmc_media::photos::decode::decode(&key.path, key.size, &AtomicBool::new(false))
                    .expect("decode photo thumbnail");
            assert_eq!(pixels.dimensions(), (256, 256));
            thumbnail_bytes += pixels.len();
            images.push(Arc::new(RenderImage::new(vec![image::Frame::new(pixels)])));
        }
        let weak: Vec<_> = images.iter().map(Arc::downgrade).collect();
        println!(
            "{} photos: original pixels {:.2} MiB, retained thumbnails {:.2} MiB",
            images.len(),
            original_bytes as f64 / 1048576.,
            thumbnail_bytes as f64 / 1048576.
        );
        assert!(thumbnail_bytes <= 6 * 1024 * 1024);
        drop(images);
        assert!(weak.iter().all(|image| image.upgrade().is_none()));
    }
    #[test]
    fn cache_drop_cancels_pending_decode() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let second = Arc::new(AtomicBool::new(false));
        let key = Key {
            path: PathBuf::from("unused"),
            size: THUMB,
            generation: 0,
        };
        let cache = PhotoImages {
            wanted: vec![key.clone()],
            images: HashMap::new(),
            inflight: HashMap::from([
                (key.clone(), cancelled.clone()),
                (
                    Key {
                        path: "second".into(),
                        ..key
                    },
                    second.clone(),
                ),
            ]),
        };
        drop(cache);
        assert!(cancelled.load(Ordering::Relaxed));
        assert!(second.load(Ordering::Relaxed));
    }
}

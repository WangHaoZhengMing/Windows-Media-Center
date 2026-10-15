//! Page-scoped photo pixels. Never pass original photo paths to GPUI's global cache.
use super::HomeView;
use gpui_kit::*;
use image::{DynamicImage, ImageDecoder, imageops::FilterType};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub(super) const THUMB: SizeKind = SizeKind::Square;
pub(super) const DETAIL: SizeKind = SizeKind::DetailSquare;

pub(super) fn photo_time(path: &Path) -> Option<i64> {
    use chrono::{Local, NaiveDateTime, TimeZone};
    let taken = (|| {
        let file = std::fs::File::open(path).ok()?;
        let exif = exif::Reader::new()
            .read_from_container(&mut std::io::BufReader::new(file))
            .ok()?;
        let field = exif
            .get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)
            .or_else(|| exif.get_field(exif::Tag::DateTimeDigitized, exif::In::PRIMARY))
            .or_else(|| exif.get_field(exif::Tag::DateTime, exif::In::PRIMARY))?;
        let exif::Value::Ascii(values) = &field.value else {
            return None;
        };
        let text = std::str::from_utf8(values.first()?)
            .ok()?
            .trim_end_matches('\0');
        let date = NaiveDateTime::parse_from_str(text, "%Y:%m:%d %H:%M:%S").ok()?;
        if let Some(offset) = exif.get_field(exif::Tag::OffsetTimeOriginal, exif::In::PRIMARY)
            && let exif::Value::Ascii(values) = &offset.value
            && let Some(value) = values.first()
            && let Ok(offset) = std::str::from_utf8(value)
            && let Ok(date) = chrono::DateTime::parse_from_str(
                &format!("{text}{}", offset.trim_end_matches('\0')),
                "%Y:%m:%d %H:%M:%S%:z",
            )
        {
            return Some(date.timestamp());
        }
        Local
            .from_local_datetime(&date)
            .earliest()
            .map(|date| date.timestamp())
    })();
    taken.or_else(|| {
        std::fs::metadata(path)
            .ok()?
            .modified()
            .ok()
            .map(|time| chrono::DateTime::<chrono::Utc>::from(time).timestamp())
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum SizeKind {
    Square,
    DetailSquare,
    Fit(u32, u32),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Key {
    path: PathBuf,
    size: SizeKind,
    generation: u64,
}

pub(super) struct PhotoImages {
    wanted: Vec<Key>,
    images: HashMap<Key, Result<Arc<RenderImage>, String>>,
    inflight: Option<(Key, Arc<AtomicBool>)>,
}

impl Drop for PhotoImages {
    fn drop(&mut self) {
        if let Some((_, cancelled)) = &self.inflight {
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
            inflight: None,
        });
        cx.observe_release(&entity, |cache, cx| {
            if let Some((_, cancelled)) = &cache.inflight {
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
        if let Some((key, cancelled)) = &self.inflight
            && !active.contains(key)
        {
            cancelled.store(true, Ordering::Relaxed);
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
        if self.inflight.is_some() {
            return;
        }
        let Some(key) = self
            .wanted
            .iter()
            .find(|key| !self.images.contains_key(*key))
            .cloned()
        else {
            return;
        };
        let cancelled = Arc::new(AtomicBool::new(false));
        self.inflight = Some((key.clone(), cancelled.clone()));
        cx.spawn(async move |this, cx| {
            let task_key = key.clone();
            let task_cancelled = cancelled.clone();
            let result = cx
                .background_executor()
                .spawn(async move { decode(&task_key, &task_cancelled) })
                .await;
            let _ = this.update(cx, |cache, cx| {
                cache.inflight = None;
                if !cancelled.load(Ordering::Relaxed) && cache.wanted.contains(&key) {
                    // Only small, still-needed pixels become renderer resources.
                    let image = result
                        .map(|pixels| Arc::new(RenderImage::new(vec![image::Frame::new(pixels)])));
                    cache.images.insert(key, image);
                    cx.notify();
                }
                // Even a cancelled decode must finish before the next begins.
                cache.pump(cx);
            });
        })
        .detach();
    }
}

fn decode(key: &Key, cancelled: &AtomicBool) -> Result<image::RgbaImage, String> {
    let run = || -> anyhow::Result<image::RgbaImage> {
        anyhow::ensure!(!cancelled.load(Ordering::Relaxed), "cancelled");
        let mut reader = image::ImageReader::open(&key.path)?.with_guessed_format()?;
        let mut limits = image::Limits::default();
        limits.max_alloc = Some(256 * 1024 * 1024);
        limits.max_image_width = Some(32768);
        limits.max_image_height = Some(32768);
        reader.limits(limits);
        let mut decoder = reader.into_decoder()?;
        let (w, h) = decoder.dimensions();
        anyhow::ensure!(
            u64::from(w) * u64::from(h) <= 32_000_000,
            "照片超过 3200 万像素解码上限"
        );
        let orientation = decoder.orientation()?;
        let mut image = DynamicImage::from_decoder(decoder)?;
        image.apply_orientation(orientation);
        anyhow::ensure!(!cancelled.load(Ordering::Relaxed), "cancelled");
        let mut pixels = resize(&image, key.size).to_rgba8();
        // Full-resolution decode and temporary resize buffers end here.
        for pixel in pixels.pixels_mut() {
            pixel.0.swap(0, 2);
        }
        anyhow::ensure!(!cancelled.load(Ordering::Relaxed), "cancelled");
        Ok(pixels)
    };
    run().map_err(|error| error.to_string())
}

fn resize(image: &DynamicImage, size: SizeKind) -> DynamicImage {
    match size {
        // Square grid / aspect-fill: equal scale on both axes, crop around center.
        SizeKind::Square => image.resize_to_fill(256, 256, FilterType::Triangle),
        SizeKind::DetailSquare => image.resize_to_fill(512, 512, FilterType::Triangle),
        SizeKind::Fit(w, h) => image.thumbnail(w, h),
    }
}

impl HomeView {
    pub(super) fn photo_size(window: &Window) -> SizeKind {
        let viewport = window.viewport_size();
        let physical =
            |n: Pixels| ((f32::from(n) * window.scale_factor()).ceil() as u32).clamp(1, 2048);
        SizeKind::Fit(physical(viewport.width), physical(viewport.height))
    }

    pub(super) fn sync_photos(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let generation = self.library.generation;
        let mut wanted = Vec::new();
        let mut add = |path: &PathBuf, size| {
            wanted.push(Key {
                path: path.clone(),
                size,
                generation,
            })
        };
        if let Some(path) = &self.photo {
            add(path, Self::photo_size(window));
        } else if !self.player_visible
            && self.state.opened().is_some()
            && let wmc_core::DestinationKind::Library(wmc_core::LibraryKind::Photos) =
                self.destination_state.kind
        {
            let indices = self
                .library
                .indices(wmc_core::LibraryKind::Photos, self.destination_state.tab);
            if !indices.is_empty() && self.photo_grid_generation != Some(generation) {
                self.photo_grid_generation = Some(generation);
                self.library.selection[0] = indices.len() - 1;
                self.photo_scroll.scroll_to_bottom();
                self.photo_visible = indices.len().saturating_sub(25)..indices.len();
            }
            for &index in indices
                .iter()
                .skip(self.photo_visible.start)
                .take(self.photo_visible.len())
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
        self.photos
            .update(cx, |cache, cx| cache.sync(wanted, window, cx));
    }

    pub(super) fn photo_image(
        &self,
        path: &Path,
        size: SizeKind,
        cx: &App,
    ) -> Option<Arc<RenderImage>> {
        self.photos
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

    pub(super) fn photo_error(&self, path: &Path, size: SizeKind, cx: &App) -> Option<String> {
        self.photos
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
    fn square_crop_keeps_center_and_does_not_stretch() {
        let input = DynamicImage::ImageRgba8(image::RgbaImage::from_fn(768, 256, |x, _| {
            if (256..512).contains(&x) {
                image::Rgba([0, 255, 0, 255])
            } else {
                image::Rgba([255, 0, 0, 255])
            }
        }));
        let square = resize(&input, THUMB).to_rgba8();
        assert_eq!(square.dimensions(), (256, 256));
        assert_eq!(square.get_pixel(128, 128).0, [0, 255, 0, 255]);
        assert_eq!(resize(&input, DETAIL).width(), 512);
        assert_eq!(resize(&input, DETAIL).height(), 512);
        assert!(resize(&input, SizeKind::Fit(512, 512)).height() < 512);
        let portrait = input.rotate90();
        let square = resize(&portrait, THUMB).to_rgba8();
        assert_eq!(square.dimensions(), (256, 256));
        assert_eq!(square.get_pixel(128, 128).0, [0, 255, 0, 255]);
    }
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
            inflight: None,
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
        for file in files.iter().take(super::super::media_library::PAGE_SIZE) {
            let (w, h) = image::image_dimensions(&file.path).unwrap();
            original_bytes += u64::from(w) * u64::from(h) * 4;
            let key = Key {
                path: file.path.clone(),
                size: THUMB,
                generation: 0,
            };
            let pixels = decode(&key, &AtomicBool::new(false)).expect("decode photo thumbnail");
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
        let key = Key {
            path: PathBuf::from("unused"),
            size: THUMB,
            generation: 0,
        };
        let cache = PhotoImages {
            wanted: vec![key.clone()],
            images: HashMap::new(),
            inflight: Some((key, cancelled.clone())),
        };
        drop(cache);
        assert!(cancelled.load(Ordering::Relaxed));
    }
}

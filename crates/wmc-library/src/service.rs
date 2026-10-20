use crate::MetadataCache;
use crate::{LibraryConfig, MediaFile, MediaKind};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
#[derive(Clone, Debug)]
pub struct LibraryEntry {
    pub file: MediaFile,
    pub title: String,
    pub title_known: bool,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub thumbnail: Option<image::RgbaImage>,
    pub photo_time: Option<i64>,
}
pub struct LibraryScan {
    pub entries: Vec<LibraryEntry>,
    pub skipped: usize,
    pub warning: Option<String>,
}
pub fn inspect(config: &LibraryConfig, cancelled: &AtomicBool) -> LibraryScan {
    let mut entries = Vec::new();
    let mut scan = inspect_stream(config, cancelled, &MetadataCache::default(), |batch| {
        entries.extend(batch);
        true
    });
    entries.sort_by_cached_key(|entry| entry.file.name.to_lowercase());
    scan.entries = entries;
    scan
}

/// Bounded workers overlap directory I/O with metadata reading and cover decoding.
/// Returning false from `emit` cancels the producer and all workers.
pub fn inspect_stream(
    config: &LibraryConfig,
    cancelled: &AtomicBool,
    cache: &MetadataCache,
    mut emit: impl FnMut(Vec<LibraryEntry>) -> bool,
) -> LibraryScan {
    std::thread::scope(|scope| {
        let (files_tx, files_rx) = std::sync::mpsc::sync_channel(64);
        let files_rx = std::sync::Arc::new(Mutex::new(files_rx));
        let (entries_tx, entries_rx) = std::sync::mpsc::sync_channel(32);
        let producer = scope.spawn(move || {
            crate::scan::visit_cancellable(config, cancelled, |file| {
                if files_tx.send(file).is_err() {
                    cancelled.store(true, Ordering::Relaxed);
                }
            })
        });
        let workers = std::thread::available_parallelism()
            .map_or(2, usize::from)
            .clamp(1, 4);
        for _ in 0..workers {
            let files_rx = files_rx.clone();
            let entries_tx = entries_tx.clone();
            scope.spawn(move || {
                while !cancelled.load(Ordering::Relaxed) {
                    let file = files_rx.lock().unwrap_or_else(|e| e.into_inner()).recv();
                    let Ok(file) = file else { break };
                    if cancelled.load(Ordering::Relaxed) {
                        break;
                    }
                    let entry = cache.read(file);
                    if entries_tx.send(entry).is_err() {
                        break;
                    }
                }
            });
        }
        drop(files_rx);
        drop(entries_tx);
        let mut batch = Vec::new();
        let mut last = std::time::Instant::now();
        loop {
            match entries_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(entry) if !cancelled.load(Ordering::Relaxed) => batch.push(entry),
                Ok(_) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
            if batch.is_empty() {
                continue;
            }
            if batch.len() >= 32 || last.elapsed() >= std::time::Duration::from_millis(100) {
                if !emit(std::mem::take(&mut batch)) {
                    cancelled.store(true, Ordering::Relaxed);
                }
                last = std::time::Instant::now();
            }
        }
        if !batch.is_empty() && !cancelled.load(Ordering::Relaxed) && !emit(batch) {
            cancelled.store(true, Ordering::Relaxed);
        }
        let scan = producer.join().expect("library enumeration panicked");
        LibraryScan {
            entries: Vec::new(),
            skipped: scan.skipped,
            warning: scan.warnings.into_iter().next(),
        }
    })
}

pub(crate) fn read_entry(file: MediaFile) -> LibraryEntry {
    let tags = if file.kind == MediaKind::Music {
        wmc_media::audio::metadata::TrackMetadata::read(&file.path).ok()
    } else {
        None
    };
    let title = tags
        .as_ref()
        .and_then(|t| t.title.clone())
        .unwrap_or_else(|| file.name.clone());
    let artist = tags.as_ref().and_then(|t| t.artist.clone());
    let album = tags.as_ref().and_then(|t| t.album.clone());
    let title_known = tags
        .as_ref()
        .and_then(|t| t.title.as_deref())
        .and_then(crate::music::known)
        .is_some();
    let album_artist = tags.as_ref().and_then(|t| t.album_artist.clone());
    let track_number = tags.as_ref().and_then(|t| t.track_number);
    let disc_number = tags.as_ref().and_then(|t| t.disc_number);
    let thumbnail = tags.and_then(|t| t.cover).and_then(cover_thumbnail);
    let photo_time = if file.kind == MediaKind::Photos {
        wmc_media::photos::exif::photo_time(&file.path)
    } else {
        None
    };
    LibraryEntry {
        file,
        title,
        title_known,
        artist,
        album,
        album_artist,
        track_number,
        disc_number,
        thumbnail,
        photo_time,
    }
}

fn cover_thumbnail(bytes: Vec<u8>) -> Option<image::RgbaImage> {
    use image::ImageDecoder;
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let decoder = reader.into_decoder().ok()?;
    let (width, height) = decoder.dimensions();
    if u64::from(width) * u64::from(height) > 32_000_000 {
        return None;
    }
    Some(
        image::DynamicImage::from_decoder(decoder)
            .ok()?
            .thumbnail(160, 160)
            .to_rgba8(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(count: usize) -> (tempfile::TempDir, LibraryConfig) {
        let dir = tempfile::tempdir().unwrap();
        for n in 0..count {
            std::fs::write(dir.path().join(format!("{n:04}.mp4")), []).unwrap();
        }
        let root = dir.path().to_owned();
        (
            dir,
            LibraryConfig {
                version: 1,
                photos: vec![],
                music: vec![],
                videos: vec![root.clone(), root],
            },
        )
    }
    #[test]
    fn streamed_batches_preserve_all_files_and_deduplicate_overlapping_roots() {
        let (_dir, config) = fixture(145);
        let expected = crate::scan(&config).files;
        let mut entries = Vec::new();
        let mut batches = 0;
        let scan = inspect_stream(
            &config,
            &AtomicBool::new(false),
            &MetadataCache::default(),
            |batch| {
                assert!(!batch.is_empty() && batch.len() <= 32);
                batches += 1;
                entries.extend(batch);
                true
            },
        );
        assert!(batches >= 5);
        assert_eq!(scan.skipped, 0);
        let indices = crate::query::indices(&entries, MediaKind::Videos, 0);
        let actual: Vec<_> = indices
            .iter()
            .map(|&n| entries[n].file.path.clone())
            .collect();
        assert_eq!(
            actual,
            expected
                .into_iter()
                .map(|file| file.path)
                .collect::<Vec<_>>()
        );
    }
    #[test]
    fn rejecting_a_batch_cancels_and_joins_workers_with_full_queues() {
        let (_dir, config) = fixture(500);
        let cancelled = AtomicBool::new(false);
        let mut batches = 0;
        inspect_stream(&config, &cancelled, &MetadataCache::default(), |_| {
            batches += 1;
            false
        });
        assert_eq!(batches, 1);
        assert!(cancelled.load(Ordering::Relaxed));
        inspect_stream(&config, &cancelled, &MetadataCache::default(), |_| {
            panic!("cancelled scan emitted a batch")
        });
    }

    #[test]
    #[ignore = "Read-only timing probe of the configured media library"]
    fn benchmark_configured_library() {
        let config = LibraryConfig::load(&LibraryConfig::location().unwrap()).unwrap();
        let cancelled = AtomicBool::new(false);
        let start = std::time::Instant::now();
        let files = crate::scan(&config).files;
        let count = files.len();
        for file in files {
            drop(read_entry(file));
        }
        println!("serial: {count} files in {:?}", start.elapsed());
        let cache = MetadataCache::default();
        for name in ["parallel", "cached refresh"] {
            let start = std::time::Instant::now();
            let mut first = None;
            let mut count = 0;
            let scan = inspect_stream(&config, &cancelled, &cache, |batch| {
                first.get_or_insert_with(|| start.elapsed());
                count += batch.len();
                true
            });
            println!(
                "{name}: {count} files in {:?}, first batch {first:?}, skipped {}",
                start.elapsed(),
                scan.skipped
            );
        }
    }
}

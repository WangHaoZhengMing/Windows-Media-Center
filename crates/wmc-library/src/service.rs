use crate::{LibraryConfig, MediaFile, MediaKind};
use std::sync::atomic::{AtomicBool, Ordering};
#[derive(Debug)]
pub struct LibraryEntry {
    pub file: MediaFile,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub thumbnail: Option<image::RgbaImage>,
    pub photo_time: Option<i64>,
}
pub struct LibraryScan {
    pub entries: Vec<LibraryEntry>,
    pub skipped: usize,
    pub warning: Option<String>,
}
pub fn inspect(config: &LibraryConfig, cancelled: &AtomicBool) -> LibraryScan {
    let scan = crate::scan_cancellable(config, cancelled);
    let entries = scan
        .files
        .into_iter()
        .take_while(|_| !cancelled.load(Ordering::Relaxed))
        .map(|file| {
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
            let thumbnail = tags
                .and_then(|t| t.cover)
                .and_then(|bytes| image::load_from_memory(&bytes).ok())
                .map(|i| i.thumbnail(160, 160).to_rgba8());
            let photo_time = if file.kind == MediaKind::Photos {
                wmc_media::photos::exif::photo_time(&file.path)
            } else {
                None
            };
            LibraryEntry {
                file,
                title,
                artist,
                album,
                thumbnail,
                photo_time,
            }
        })
        .collect();
    LibraryScan {
        entries,
        skipped: scan.skipped,
        warning: scan.warnings.into_iter().next(),
    }
}

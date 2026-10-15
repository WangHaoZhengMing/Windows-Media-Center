use crate::{LibraryConfig, MediaKind};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Debug)]
pub struct MediaFile {
    pub path: PathBuf,
    pub name: String,
    pub kind: MediaKind,
}

#[derive(Default)]
pub struct ScanResult {
    pub files: Vec<MediaFile>,
    pub warnings: Vec<String>,
    pub skipped: usize,
}

pub fn scan(config: &LibraryConfig) -> ScanResult {
    scan_cancellable(config, &AtomicBool::new(false))
}

pub fn scan_cancellable(config: &LibraryConfig, cancelled: &AtomicBool) -> ScanResult {
    let mut result = ScanResult::default();
    let mut seen = HashSet::new();
    for kind in MediaKind::ALL {
        for root in config.roots(kind) {
            if cancelled.load(Ordering::Relaxed) {
                return result;
            }
            if !root.is_dir() {
                warn(&mut result, format!("目录不可访问：{}", root.display()));
                continue;
            }
            for entry in walkdir::WalkDir::new(root)
                .follow_links(false)
                .into_iter()
                .filter_entry(|entry| entry.depth() == 0 || !is_link(entry))
            {
                if cancelled.load(Ordering::Relaxed) {
                    return result;
                }
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        warn(&mut result, error.to_string());
                        continue;
                    }
                };
                if !entry.file_type().is_file() || !supported(entry.path(), kind) {
                    continue;
                }
                let canonical = match std::fs::canonicalize(entry.path()) {
                    Ok(path) => path,
                    Err(error) => {
                        warn(&mut result, error.to_string());
                        continue;
                    }
                };
                if !seen.insert((kind.index(), canonical)) {
                    continue;
                }
                result.files.push(MediaFile {
                    path: entry.path().to_owned(),
                    kind,
                    name: entry
                        .path()
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                });
            }
        }
    }
    result
        .files
        .sort_by_cached_key(|file| file.name.to_lowercase());
    result
}

fn warn(result: &mut ScanResult, message: String) {
    result.skipped += 1;
    if result.warnings.len() < 20 {
        result.warnings.push(message);
    }
}
fn is_link(entry: &walkdir::DirEntry) -> bool {
    if entry.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        std::fs::symlink_metadata(entry.path())
            .is_ok_and(|meta| meta.file_attributes() & 0x400 != 0)
    }
    #[cfg(not(windows))]
    {
        false
    }
}
fn supported(path: &std::path::Path, kind: MediaKind) -> bool {
    let ext = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    let extensions: &[&str] = match kind {
        MediaKind::Photos => &["jpg", "jpeg", "png", "webp", "gif", "bmp", "tif", "tiff"],
        MediaKind::Music => &["mp3", "flac", "wav", "ogg", "m4a", "aac"],
        MediaKind::Videos => &[
            "mp4", "mkv", "avi", "mov", "wmv", "webm", "m4v", "mpeg", "mpg",
        ],
    };
    extensions.contains(&ext.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_cancelled_scan_does_not_enter_any_roots() {
        let config = LibraryConfig {
            version: 1,
            photos: vec![PathBuf::from("missing")],
            music: vec![],
            videos: vec![],
        };
        let result = scan_cancellable(&config, &AtomicBool::new(true));
        assert!(result.files.is_empty());
        assert_eq!(result.skipped, 0);
    }
    #[test]
    fn scan_is_recursive_case_insensitive_deduplicated_and_scoped() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("nested");
        std::fs::create_dir(&nested).unwrap();
        for name in ["a.JPG", "b.MP3", "c.mp4", "ignored.txt"] {
            std::fs::write(nested.join(name), []).unwrap();
        }
        let config = LibraryConfig {
            version: 1,
            photos: vec![dir.path().to_owned(), nested.clone()],
            music: vec![dir.path().to_owned()],
            videos: vec![nested, dir.path().join("missing")],
        };
        let found = scan(&config);
        assert_eq!(found.files.len(), 3);
        assert_eq!(found.skipped, 1);
        for kind in MediaKind::ALL {
            assert_eq!(
                found.files.iter().filter(|file| file.kind == kind).count(),
                1
            );
        }
    }
}

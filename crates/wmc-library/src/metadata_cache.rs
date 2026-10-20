//! App-owned refresh cache. Never retains original photo pixels or embedded cover bytes.
use crate::{LibraryEntry, MediaFile};
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::Mutex,
    time::SystemTime,
};

const MAX_BYTES: usize = 32 * 1024 * 1024;
const MAX_ENTRIES: usize = 20_000;
type Key = (usize, PathBuf);

#[derive(Clone, Copy, PartialEq, Eq)]
struct Stamp {
    len: u64,
    modified: SystemTime,
}
impl Stamp {
    fn read(file: &MediaFile) -> Option<Self> {
        let metadata = std::fs::metadata(&file.path).ok()?;
        Some(Self {
            len: metadata.len(),
            modified: metadata.modified().ok()?,
        })
    }
}
struct Cached {
    stamp: Stamp,
    entry: LibraryEntry,
    bytes: usize,
}
#[derive(Default)]
struct State {
    entries: HashMap<Key, Cached>,
    order: VecDeque<Key>,
    bytes: usize,
}
#[derive(Default)]
pub struct MetadataCache {
    state: Mutex<State>,
}
impl MetadataCache {
    pub(crate) fn read(&self, file: MediaFile) -> LibraryEntry {
        let key = (file.kind.index(), file.path.clone());
        let stamp = Stamp::read(&file);
        if let Some(stamp) = stamp
            && let Some(entry) = self
                .state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .entries
                .get(&key)
                .filter(|cached| cached.stamp == stamp)
        {
            return entry.entry.clone();
        }
        let entry = crate::service::read_entry(file);
        if let Some(stamp) = stamp
            && Stamp::read(&entry.file) == Some(stamp)
        {
            self.insert(key, stamp, entry.clone());
        }
        entry
    }
    fn insert(&self, key: Key, stamp: Stamp, entry: LibraryEntry) {
        let bytes = 1024
            + entry.thumbnail.as_ref().map_or(0, |image| image.len())
            + entry.title.len()
            + entry.file.path.as_os_str().len() * 2
            + [&entry.artist, &entry.album, &entry.album_artist]
                .into_iter()
                .map(|text| text.as_ref().map_or(0, String::len))
                .sum::<usize>();
        if bytes > MAX_BYTES {
            return;
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(old) = state.entries.remove(&key) {
            state.bytes -= old.bytes;
            state.order.retain(|existing| existing != &key);
        }
        while state.bytes + bytes > MAX_BYTES || state.entries.len() >= MAX_ENTRIES {
            let Some(old) = state.order.pop_front() else {
                break;
            };
            if let Some(old) = state.entries.remove(&old) {
                state.bytes -= old.bytes;
            }
        }
        state.bytes += bytes;
        state.order.push_back(key.clone());
        state.entries.insert(
            key,
            Cached {
                stamp,
                entry,
                bytes,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unchanged_files_reuse_metadata_and_changed_files_are_re_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("photo.png");
        std::fs::write(&path, b"first").unwrap();
        let file = MediaFile {
            path,
            name: "photo".into(),
            kind: crate::MediaKind::Photos,
        };
        let cache = MetadataCache::default();
        let mut entry = cache.read(file.clone());
        entry.title = "cached marker".into();
        cache.insert(
            (file.kind.index(), file.path.clone()),
            Stamp::read(&file).unwrap(),
            entry,
        );
        assert_eq!(cache.read(file.clone()).title, "cached marker");
        std::fs::write(&file.path, b"different length").unwrap();
        assert_eq!(cache.read(file.clone()).title, "photo");
        std::fs::remove_file(&file.path).unwrap();
        assert_eq!(cache.read(file).photo_time, None);
    }

    #[test]
    fn cover_cache_evicts_old_entries_instead_of_growing_with_the_library() {
        let cache = MetadataCache::default();
        let stamp = Stamp {
            len: 0,
            modified: SystemTime::UNIX_EPOCH,
        };
        for n in 0..350 {
            let file = MediaFile {
                path: format!("{n}.mp4").into(),
                name: n.to_string(),
                kind: crate::MediaKind::Videos,
            };
            let mut entry = crate::service::read_entry(file);
            entry.thumbnail = Some(image::RgbaImage::new(160, 160));
            cache.insert((2, entry.file.path.clone()), stamp, entry);
        }
        let state = cache.state.lock().unwrap();
        assert!(state.bytes <= MAX_BYTES);
        assert!(!state.entries.contains_key(&(2, "0.mp4".into())));
        assert!(state.entries.contains_key(&(2, "349.mp4".into())));
        assert_eq!(state.order.len(), state.entries.len());
    }
}

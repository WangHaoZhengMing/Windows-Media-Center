//! Typed, UI-independent music grouping. Missing metadata always sorts last.
use crate::{LibraryEntry, MediaKind};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Albums,
    Artists,
    Songs,
}
impl Category {
    pub const LABELS: &'static [&'static str] = &["专辑", "艺术家", "歌曲"];
    pub fn from_tab(tab: usize) -> Self {
        match tab {
            0 => Self::Albums,
            1 => Self::Artists,
            _ => Self::Songs,
        }
    }
}

pub(crate) fn known(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty()
        || matches!(
            value.to_lowercase().as_str(),
            "unknown"
                | "unknown artist"
                | "unknown album"
                | "unknown title"
                | "未知"
                | "未知歌手"
                | "未知艺术家"
                | "未知专辑"
                | "未知歌曲"
                | "未知标题"
        )
    {
        None
    } else {
        Some(value)
    }
}
fn clean(value: Option<&str>) -> Option<String> {
    value.and_then(known).map(str::to_owned)
}
fn order(value: Option<&str>) -> (bool, String) {
    let value = value.and_then(known);
    (value.is_none(), value.unwrap_or_default().to_lowercase())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupKey {
    pub category: Category,
    pub name: Option<String>,
    pub artist: Option<String>,
}
impl GroupKey {
    pub fn label(&self) -> &str {
        self.name.as_deref().unwrap_or(match self.category {
            Category::Albums => "未知专辑",
            _ => "未知艺术家",
        })
    }
    fn for_track(track: &LibraryEntry, category: Category) -> Self {
        let name = clean(match category {
            Category::Albums => track.album.as_deref(),
            _ => track.artist.as_deref(),
        });
        let artist = if category == Category::Albums && name.is_some() {
            clean(track.album_artist.as_deref()).or_else(|| clean(track.artist.as_deref()))
        } else {
            None
        };
        Self {
            category,
            name,
            artist,
        }
    }
    fn order(&self) -> ((bool, String), (bool, String)) {
        (order(self.name.as_deref()), order(self.artist.as_deref()))
    }
}

#[derive(Clone, Debug)]
pub struct Group {
    pub key: GroupKey,
    pub tracks: Vec<usize>,
}

pub fn tracks(items: &[&LibraryEntry], category: Category, group: Option<&GroupKey>) -> Vec<usize> {
    let mut result: Vec<_> = items
        .iter()
        .enumerate()
        .filter(|(_, track)| track.file.kind == MediaKind::Music)
        .filter(|(_, track)| {
            group.is_none_or(|group| {
                GroupKey::for_track(track, group.category).order() == group.order()
            })
        })
        .map(|(index, _)| index)
        .collect();
    result.sort_by_cached_key(|&index| {
        let track = items[index];
        let category_key = match category {
            Category::Albums => (
                order(track.album.as_deref()),
                order(track.album_artist.as_deref().or(track.artist.as_deref())),
            ),
            Category::Artists => (
                order(track.artist.as_deref()),
                order(track.album.as_deref()),
            ),
            Category::Songs => ((false, String::new()), (false, String::new())),
        };
        let numbered = group.is_some_and(|group| group.category == Category::Albums);
        (
            category_key,
            if numbered {
                (
                    track.disc_number.unwrap_or(1),
                    track.track_number.is_none(),
                    track.track_number.unwrap_or(0),
                )
            } else {
                (0, false, 0)
            },
            !track.title_known,
            track.title.to_lowercase(),
            track.file.path.clone(),
        )
    });
    result
}

pub fn groups(items: &[&LibraryEntry], category: Category) -> Vec<Group> {
    if category == Category::Songs {
        return Vec::new();
    }
    let mut groups = BTreeMap::new();
    for index in tracks(items, category, None) {
        let key = GroupKey::for_track(items[index], category);
        groups
            .entry(key.order())
            .or_insert_with(|| Group {
                key,
                tracks: Vec::new(),
            })
            .tracks
            .push(index);
    }
    groups.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn track(title: &str, album: Option<&str>, artist: Option<&str>) -> LibraryEntry {
        LibraryEntry {
            file: crate::MediaFile {
                path: title.into(),
                name: title.into(),
                kind: MediaKind::Music,
            },
            title: title.into(),
            title_known: true,
            artist: artist.map(str::to_owned),
            album: album.map(str::to_owned),
            album_artist: None,
            track_number: None,
            disc_number: None,
            thumbnail: None,
            photo_time: None,
        }
    }
    #[test]
    fn albums_and_artists_group_tracks_and_put_missing_metadata_last() {
        let entries = [
            track("One", Some("Album"), Some("Singer")),
            track("Two", Some(" album "), Some("singer")),
            track("Missing", None, None),
            track("Other", Some("Album"), Some("Other singer")),
        ];
        let items: Vec<_> = entries.iter().collect();
        let albums = groups(&items, Category::Albums);
        assert_eq!(
            albums.len(),
            3,
            "same title by different album artists remains separate"
        );
        assert_eq!(albums.last().unwrap().key.label(), "未知专辑");
        assert!(albums.iter().any(|group| group.tracks.len() == 2));
        let artists = groups(&items, Category::Artists);
        assert_eq!(artists.len(), 3);
        assert_eq!(artists.last().unwrap().key.label(), "未知艺术家");
    }
    #[test]
    fn album_tracks_follow_disc_and_track_numbers_and_unknown_titles_sort_last_in_songs() {
        let mut entries = [
            track("Z", Some("Album"), Some("Singer")),
            track("A", Some("Album"), Some("Singer")),
            track("000-file", None, None),
            track("B", Some("Album"), Some("Singer")),
        ];
        entries[0].track_number = Some(1);
        entries[1].track_number = Some(2);
        entries[2].title_known = false;
        entries[3].disc_number = Some(2);
        entries[3].track_number = Some(1);
        let items: Vec<_> = entries.iter().collect();
        let album = groups(&items, Category::Albums).remove(0);
        assert_eq!(
            tracks(&items, Category::Albums, Some(&album.key)),
            vec![0, 1, 3]
        );
        assert_eq!(tracks(&items, Category::Songs, None), vec![1, 3, 0, 2]);
    }

    #[test]
    fn album_artist_keeps_duet_tracks_in_the_same_album() {
        let mut entries = [
            track("One", Some("Album"), Some("Singer A")),
            track("Two", Some("Album"), Some("Singer B")),
        ];
        for entry in &mut entries {
            entry.album_artist = Some("Band".into());
        }
        let items: Vec<_> = entries.iter().collect();
        let albums = groups(&items, Category::Albums);
        assert_eq!(albums.len(), 1);
        assert_eq!(albums[0].tracks.len(), 2);
        assert_eq!(albums[0].key.artist.as_deref(), Some("Band"));
        assert_eq!(groups(&items, Category::Artists).len(), 2);
    }
}

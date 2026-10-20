use crate::{LibraryEntry, MediaKind};
pub fn indices<'a>(
    items: impl IntoIterator<Item = &'a LibraryEntry>,
    kind: MediaKind,
    tab: usize,
) -> Vec<usize> {
    let items: Vec<_> = items.into_iter().collect();
    let mut indices: Vec<_> = items
        .iter()
        .enumerate()
        .filter(|(_, i)| i.file.kind == kind)
        .map(|(n, _)| n)
        .collect();
    if kind == MediaKind::Photos {
        indices.sort_by_cached_key(|&n| (items[n].photo_time, items[n].file.path.clone()));
    } else if kind == MediaKind::Music {
        return crate::music::tracks(&items, crate::music::Category::from_tab(tab), None);
    } else {
        indices.sort_by_cached_key(|&n| {
            (
                items[n].file.name.to_lowercase(),
                items[n].file.path.clone(),
            )
        });
    }
    indices
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::MediaFile;
    #[test]
    fn photos_order_by_time_and_stable_path() {
        let entries: Vec<_> = [
            (Some(2000), "a"),
            (Some(1000), "z"),
            (None, "b"),
            (Some(1000), "c"),
        ]
        .into_iter()
        .map(|(photo_time, name)| LibraryEntry {
            file: MediaFile {
                path: name.into(),
                name: name.into(),
                kind: MediaKind::Photos,
            },
            title: name.into(),
            title_known: false,
            artist: None,
            album: None,
            album_artist: None,
            track_number: None,
            disc_number: None,
            thumbnail: None,
            photo_time,
        })
        .collect();
        assert_eq!(indices(&entries, MediaKind::Photos, 0), vec![2, 3, 1, 0]);
    }
}

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
        indices.sort_by_cached_key(|&n| {
            let i = items[n];
            let group = match tab {
                0 => i.album.as_deref(),
                1 | 7 => i.artist.as_deref(),
                _ => None,
            };
            (
                group.unwrap_or_default().to_lowercase(),
                i.title.to_lowercase(),
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
            artist: None,
            album: None,
            thumbnail: None,
            photo_time,
        })
        .collect();
        assert_eq!(indices(&entries, MediaKind::Photos, 0), vec![2, 3, 1, 0]);
    }
}

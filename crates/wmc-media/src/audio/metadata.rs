//! Read-only, format-independent audio tags. No network or file modification.
pub use super::properties;
use anyhow::{Context, Result};
use lofty::{
    config::ParseOptions,
    file::TaggedFileExt,
    picture::PictureType,
    probe::Probe,
    tag::{ItemKey, Tag},
};
use std::path::Path;

#[derive(Default, Debug)]
pub struct TrackMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub lyrics: Option<String>,
    /// Original embedded picture bytes, not a temporary extracted file.
    pub cover: Option<Vec<u8>>,
}

impl TrackMetadata {
    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
        let file = Probe::open(path.as_ref())?
            .guess_file_type()?
            .options(ParseOptions::new().read_properties(false))
            .read()
            .context("无法读取音频元数据")?;
        let primary = file.primary_tag();
        let tags: Vec<&Tag> = primary
            .into_iter()
            .chain(
                file.tags()
                    .iter()
                    .filter(|tag| primary.is_none_or(|first| !std::ptr::eq(first, *tag))),
            )
            .collect();
        Ok(Self::from_tags(&tags))
    }

    fn from_tags(tags: &[&Tag]) -> Self {
        let text = |key| {
            tags.iter().find_map(|tag| {
                tag.get_string(key)
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                    .map(str::to_owned)
            })
        };
        let pictures = || tags.iter().flat_map(|tag| tag.pictures());
        let cover = pictures()
            .find(|picture| picture.pic_type() == PictureType::CoverFront)
            .or_else(|| pictures().next())
            .map(|picture| picture.data().to_vec());
        Self {
            title: text(ItemKey::TrackTitle),
            artist: text(ItemKey::TrackArtist).or_else(|| text(ItemKey::AlbumArtist)),
            album: text(ItemKey::AlbumTitle),
            lyrics: text(ItemKey::Lyrics).or_else(|| text(ItemKey::UnsyncLyrics)),
            cover,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lofty::tag::{Accessor, TagType};

    #[test]
    fn missing_primary_fields_fall_back_to_other_tags_and_album_artist() {
        let mut primary = Tag::new(TagType::Id3v2);
        primary.set_title("  ".into());
        primary.insert_text(ItemKey::AlbumArtist, "专辑歌手".into());
        primary.insert_text(ItemKey::UnsyncLyrics, "没有时间戳的歌词".into());
        let mut secondary = Tag::new(TagType::Id3v1);
        secondary.set_title("歌曲标题".into());
        let result = TrackMetadata::from_tags(&[&primary, &secondary]);
        assert_eq!(result.title.as_deref(), Some("歌曲标题"));
        assert_eq!(result.artist.as_deref(), Some("专辑歌手"));
        assert_eq!(result.lyrics.as_deref(), Some("没有时间戳的歌词"));
    }
}

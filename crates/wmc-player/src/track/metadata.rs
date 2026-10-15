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

    #[test]
    fn tagged_file_loads_embedded_art_and_lyrics_and_manual_title_wins() {
        use lofty::{
            config::WriteOptions,
            picture::{MimeType, Picture},
            tag::TagExt,
        };
        use std::io::Cursor;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("song.wav");
        let mut wav = b"RIFF".to_vec();
        wav.extend(1636_u32.to_le_bytes());
        wav.extend(b"WAVEfmt ");
        wav.extend(16_u32.to_le_bytes());
        wav.extend(1_u16.to_le_bytes());
        wav.extend(1_u16.to_le_bytes());
        wav.extend(8000_u32.to_le_bytes());
        wav.extend(16000_u32.to_le_bytes());
        wav.extend(2_u16.to_le_bytes());
        wav.extend(16_u16.to_le_bytes());
        wav.extend(b"data");
        wav.extend(1600_u32.to_le_bytes());
        wav.extend([0; 1600]);
        std::fs::write(&path, wav).unwrap();
        let mut png = Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(2, 3, image::Rgba([30, 80, 130, 255]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let png = png.into_inner();
        let mut tag = Tag::new(TagType::Id3v2);
        tag.set_title("标签标题".into());
        tag.set_artist("标签歌手".into());
        tag.set_album("标签专辑".into());
        tag.insert_text(
            ItemKey::UnsyncLyrics,
            "[00:00.000]<00:00.000>测试<00:00.050>".into(),
        );
        tag.push_picture(
            Picture::unchecked(vec![1, 2, 3])
                .pic_type(PictureType::CoverBack)
                .mime_type(MimeType::Png)
                .build(),
        );
        tag.push_picture(
            Picture::unchecked(png.clone())
                .pic_type(PictureType::CoverFront)
                .mime_type(MimeType::Png)
                .build(),
        );
        tag.save_to_path(&path, WriteOptions::default()).unwrap();
        let before = std::fs::read(&path).unwrap();
        let metadata = TrackMetadata::read(&path).unwrap();
        assert_eq!(metadata.title.as_deref(), Some("标签标题"));
        assert_eq!(metadata.album.as_deref(), Some("标签专辑"));
        assert_eq!(metadata.cover.as_deref(), Some(png.as_slice()));
        let track = crate::PlayerConfig {
            audio: Some(path.clone()),
            title: Some("手动标题".into()),
            ..Default::default()
        }
        .prepare()
        .unwrap();
        assert_eq!(track.title, "手动标题");
        assert_eq!(track.artist, "标签歌手");
        assert_eq!(track.lyrics.lines[0].text(), "测试");
        assert_eq!(track.cover.width(), 2);
        assert_eq!(track.cover.height(), 3);
        assert_eq!(std::fs::read(path).unwrap(), before);
    }
}

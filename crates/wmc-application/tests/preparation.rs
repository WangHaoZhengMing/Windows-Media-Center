use lofty::{
    picture::PictureType,
    tag::{Accessor, ItemKey, Tag, TagType},
};
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
    let metadata = wmc_media::audio::metadata::TrackMetadata::read(&path).unwrap();
    assert_eq!(metadata.title.as_deref(), Some("标签标题"));
    assert_eq!(metadata.album.as_deref(), Some("标签专辑"));
    assert_eq!(metadata.cover.as_deref(), Some(png.as_slice()));
    let track = wmc_application::PlayerConfig {
        audio: Some(path.clone()),
        title: Some("手动标题".into()),
        ..Default::default()
    }
    .prepare()
    .unwrap();
    assert_eq!(track.title, "手动标题");
    assert_eq!(track.artist, "标签歌手");
    assert_eq!(track.lyrics.lines[0].text(), "测试");
    assert_eq!(track.cover.as_ref().unwrap().width(), 2);
    assert_eq!(track.cover.as_ref().unwrap().height(), 3);
    assert_eq!(std::fs::read(path).unwrap(), before);
}

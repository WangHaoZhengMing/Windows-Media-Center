//! Write the selected assets on a sibling copy, then atomically replace the song.
use anyhow::{Context, Result, ensure};
use lofty::{
    config::{ParseOptions, WriteOptions},
    file::{FileType, TaggedFileExt},
    picture::{MimeType, Picture, PictureType},
    probe::Probe,
    tag::{ItemKey, Tag, TagExt, TagType},
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub(crate) fn write_assets(path: &Path, lyrics: &str, cover: &[u8]) -> Result<PathBuf> {
    ensure!(path.is_file(), "找不到源音乐文件");
    let original_metadata = fs::metadata(path)?;
    ensure!(!lyrics.trim().is_empty(), "当前没有可写入的歌词");
    ensure!(!cover.is_empty(), "当前没有可写入的封面");
    image::load_from_memory(cover).context("封面图片无效")?;
    let mut file = Probe::open(path)?
        .guess_file_type()?
        .options(ParseOptions::new().read_properties(false))
        .read()?;
    let tag_type = if file.file_type() == FileType::Wav {
        TagType::Id3v2
    } else {
        file.primary_tag_type()
    };
    ensure!(
        file.tag_support(tag_type).is_writable(),
        "此音乐格式不支持内嵌歌词和封面"
    );
    if file.tag(tag_type).is_none() {
        file.insert_tag(Tag::new(tag_type));
    }
    let tag = file.tag_mut(tag_type).context("无法创建音乐标签")?;
    tag.remove_key(ItemKey::Lyrics);
    tag.remove_key(ItemKey::UnsyncLyrics);
    ensure!(
        tag.insert_text(ItemKey::UnsyncLyrics, lyrics.to_owned()),
        "此标签格式不支持歌词"
    );
    tag.remove_picture_type(PictureType::CoverFront);
    tag.push_picture(
        Picture::unchecked(cover.to_vec())
            .pic_type(PictureType::CoverFront)
            .mime_type(MimeType::Png)
            .build(),
    );
    let parent = path.parent().context("源文件目录无效")?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".wmc-assets-")
        .tempfile_in(parent)?;
    fs::copy(path, temporary.path()).context("无法复制源文件")?;
    tag.save_to_path(temporary.path(), WriteOptions::default())
        .context("标签写入失败，源文件未修改")?;
    // Read the copy back before committing: unsupported artwork fields must not
    // silently report success, and malformed writes never replace the original.
    let checked = super::metadata::TrackMetadata::read(temporary.path())?;
    ensure!(
        checked.lyrics.as_deref() == Some(lyrics),
        "歌词写入校验失败，源文件未修改"
    );
    ensure!(
        checked.cover.as_deref() == Some(cover),
        "封面写入校验失败，源文件未修改"
    );
    let mut backup_name = path.as_os_str().to_os_string();
    backup_name.push(".wmc-backup");
    let backup = PathBuf::from(backup_name);
    // Keep the first backup rather than overwriting it on a second save.
    let current_metadata = fs::metadata(path)?;
    ensure!(
        current_metadata.len() == original_metadata.len()
            && current_metadata.modified()? == original_metadata.modified()?,
        "源文件已被其他程序修改，请重新写入"
    );
    if !backup.exists() {
        let mut backup_copy = tempfile::Builder::new()
            .prefix(".wmc-backup-")
            .tempfile_in(parent)?;
        let mut source = fs::File::open(path)?;
        std::io::copy(&mut source, backup_copy.as_file_mut())?;
        backup_copy.flush()?;
        backup_copy.as_file_mut().sync_all()?;
        match backup_copy.persist_noclobber(&backup) {
            Ok(_) => {}
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.error).context("无法保留源文件备份"),
        }
    }
    fs::set_permissions(temporary.path(), fs::metadata(path)?.permissions())?;
    temporary.as_file_mut().sync_all()?;
    // The stopped audio callback may take one buffer to release its old reader.
    for attempt in 0..10 {
        match temporary.persist(path) {
            Ok(_) => return Ok(backup),
            Err(error)
                if error.error.kind() == std::io::ErrorKind::PermissionDenied && attempt < 9 =>
            {
                temporary = error.file;
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            Err(error) => {
                return Err(error.error).context("源音乐文件正被独占或不可写，原文件未修改");
            }
        }
    }
    unreachable!("bounded commit always returns")
}

#[cfg(test)]
mod tests {
    use super::*;
    use lofty::tag::Accessor;
    #[test]
    fn embedding_preserves_audio_other_tags_and_the_first_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.wav");
        let samples: Vec<i16> = (0..800).map(|i| (i * 20 - 8000) as i16).collect();
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
        for sample in samples {
            wav.extend(sample.to_le_bytes());
        }
        fs::write(&path, wav).unwrap();
        let mut tag = Tag::new(TagType::Id3v2);
        tag.set_title("保留标题".into());
        tag.insert_text(ItemKey::Comment, "保留注释".into());
        tag.save_to_path(&path, WriteOptions::default()).unwrap();
        let original = fs::read(&path).unwrap();
        let decode = || {
            rodio::Decoder::try_from(fs::File::open(&path).unwrap())
                .unwrap()
                .collect::<Vec<_>>()
        };
        let audio = decode();
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(3, 4, image::Rgba([20, 80, 120, 255]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let png = png.into_inner();
        let lyrics = "[00:00.000]<00:00.000>测试<00:00.050>";
        let backup = write_assets(&path, lyrics, &png).unwrap();
        assert_eq!(decode(), audio);
        assert_eq!(fs::read(&backup).unwrap(), original);
        let read = Probe::open(&path).unwrap().read().unwrap();
        let tag = read.tag(TagType::Id3v2).unwrap();
        assert_eq!(tag.title().as_deref(), Some("保留标题"));
        assert_eq!(tag.get_string(ItemKey::Comment), Some("保留注释"));
        write_assets(&path, "[00:00]第二次保存", &png).unwrap();
        assert_eq!(fs::read(backup).unwrap(), original);
        assert_eq!(decode(), audio);
    }
    #[test]
    fn rejected_cover_leaves_the_original_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.mp3");
        fs::write(&path, b"original").unwrap();
        assert!(write_assets(&path, "lyrics", b"invalid image").is_err());
        assert_eq!(fs::read(path).unwrap(), b"original");
    }

    #[test]
    #[ignore = "Requires the locally installed ffmpeg for isolated audio fixtures"]
    fn mp3_flac_and_m4a_round_trip_without_changing_decoded_audio() {
        let dir=tempfile::tempdir().unwrap();
        let mut png=std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(4,4,image::Rgba([10,50,90,255]))
            .write_to(&mut png,image::ImageFormat::Png).unwrap();
        for (extension,codec) in [("mp3","libmp3lame"),("flac","flac"),("m4a","aac")] {
            let path=dir.path().join(format!("fixture.{extension}"));
            let result=std::process::Command::new("ffmpeg")
                .args(["-v","error","-f","lavfi","-i","sine=frequency=440:sample_rate=44100:duration=0.3","-c:a",codec])
                .arg(&path).output().unwrap();
            assert!(result.status.success(),"{}",String::from_utf8_lossy(&result.stderr));
            let decode=||rodio::Decoder::try_from(fs::File::open(&path).unwrap()).unwrap().collect::<Vec<_>>();
            let before=decode();
            write_assets(&path,"[00:00.00]嵌入歌词测试",png.get_ref()).unwrap();
            assert_eq!(decode(),before,"audio changed in {extension}");
        }
    }
}

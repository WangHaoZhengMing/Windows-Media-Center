//! Resolve local files, metadata, cover art and lyric sources.
use super::{PlayerConfig, PreparedTrack, metadata::TrackMetadata};
use crate::{audio::AudioData, lyrics::Lyrics};
use anyhow::{Context, Result, ensure};
use std::time::Instant;

impl PlayerConfig {
    pub fn prepare(self) -> Result<PreparedTrack> {
        self.prepare_with_timings().map(|(track, _)| track)
    }

    pub fn prepare_with_timings(self) -> Result<(PreparedTrack, crate::PreparationTimings)> {
        let started = Instant::now();
        let is_demo = self.audio.is_none();
        let cover_cache_allowed = self.cover.is_none() && !is_demo;
        let lyrics_cache_allowed = self.lyrics.is_none() && !is_demo;
        let media_path = self.audio.clone();
        let mut local_sources = crate::media_cache::LocalSources {
            lyrics_file: self.lyrics.clone(),
            cover_file: self.cover.clone(),
            ..Default::default()
        };
        let metadata = self
            .audio
            .as_ref()
            .map(|path| {
                TrackMetadata::read(path).unwrap_or_else(|error| {
                    eprintln!("元数据读取失败，继续加载音频：{error:#}");
                    TrackMetadata::default()
                })
            })
            .unwrap_or_default();
        let metadata_time = started.elapsed();
        local_sources.embedded_lyrics = metadata.lyrics.is_some();
        local_sources.embedded_cover = metadata.cover.is_some();
        let fallback_title = self
            .audio
            .as_ref()
            .and_then(|path| path.file_stem())
            .map(|name| name.to_string_lossy().into_owned());
        let sidecar = self.audio.as_ref().and_then(|path| {
            ["ttml", "yrc", "qrc", "lrc"]
                .into_iter()
                .map(|extension| path.with_extension(extension))
                .find(|path| path.is_file())
        });
        let stage = Instant::now();
        let audio = match &self.audio {
            Some(path) => AudioData::load(path)?,
            None => AudioData::demo(),
        };
        local_sources.lyrics_file = local_sources.lyrics_file.or(sidecar.clone());
        ensure!(audio.duration() > 0., "音频为空");
        let audio_time = stage.elapsed();
        let stage = Instant::now();
        let lyric_content = match self.lyrics {
            Some(path) => std::fs::read_to_string(path)?,
            None if is_demo => include_str!("../../assets/lyrics/blue-hour.lrc").to_owned(),
            None => {
                let embedded = metadata.lyrics.as_deref().filter(|source|
                    Lyrics::parse_text(source, audio.duration()).is_ok());
                match embedded {
                    Some(source) => source.to_owned(),
                    None => match sidecar { Some(path) => std::fs::read_to_string(path)?, None => String::new() },
                }
            }
        };
        let lyrics = Lyrics::parse_text(&lyric_content, audio.duration())?;
        let lyrics_time = stage.elapsed();
        let stage = Instant::now();
        let embedded_cover = metadata
            .cover
            .as_deref()
            .and_then(|bytes| image::load_from_memory(bytes).ok());
        let sidecar_cover = self.audio.as_ref().and_then(|audio| {
            let parent = audio.parent()?;
            [
                audio.with_extension("jpg"),
                audio.with_extension("png"),
                audio.with_extension("webp"),
                parent.join("cover.jpg"),
                parent.join("cover.png"),
                parent.join("folder.jpg"),
            ]
            .into_iter()
            .find_map(|path| {
                if path.is_file() {
                    image::open(&path).ok().map(|image| (path, image))
                } else {
                    None
                }
            })
        });
        let cover_online_needed =
            cover_cache_allowed && embedded_cover.is_none() && sidecar_cover.is_none();
        local_sources.cover_file = local_sources.cover_file.or_else(|| sidecar_cover.as_ref().map(|(path, _)| path.clone()));
        let cover = match self.cover {
            Some(path) => image::open(path)?.to_rgba8(),
            None if embedded_cover.is_some() => {
                embedded_cover.expect("checked embedded cover").to_rgba8()
            }
            None if sidecar_cover.is_some() => {
                sidecar_cover.expect("checked sidecar cover").1.to_rgba8()
            }
            None => {
                let bytes = wmc_assets::load(wmc_assets::Shortcut::MusicFavorites.images().focused)
                    .context("缺少原版音乐封面资源")?;
                let art = image::load_from_memory(&bytes)?.to_rgba8();
                // Select the album area while rendering; source PNG bytes remain untouched.
                image::imageops::crop_imm(&art, 48, 22, 82, 91).to_image()
            }
        };
        let cover_time = stage.elapsed();
        let stage = Instant::now();
        let mut prepared = PreparedTrack::new(
            audio,
            lyrics,
            cover,
            self.title
                .or(metadata.title)
                .or(fallback_title)
                .unwrap_or_else(|| {
                    if is_demo {
                        "蓝色时刻".into()
                    } else {
                        "本地音乐".into()
                    }
                }),
            self.artist.or(metadata.artist).unwrap_or_else(|| {
                if is_demo {
                    "WMC Demo · 合成伴奏".into()
                } else {
                    "未知歌手".into()
                }
            }),
            metadata.album.unwrap_or_default(),
        )?;
        prepared.cover_cache_allowed = cover_cache_allowed;
        prepared.cover_online_needed = cover_online_needed;
        prepared.lyrics_cache_allowed = lyrics_cache_allowed;
        prepared.media_path = media_path;
        prepared.lyric_content = lyric_content;
        prepared.local_sources = local_sources;
        Ok((
            prepared,
            crate::PreparationTimings {
                metadata: metadata_time,
                audio: audio_time,
                lyrics: lyrics_time,
                cover: cover_time,
                background: stage.elapsed(),
                total: started.elapsed(),
            },
        ))
    }
}

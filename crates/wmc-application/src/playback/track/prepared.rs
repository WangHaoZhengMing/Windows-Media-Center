//! CPU song preparation results; GPU image conversion belongs to the view.
use image::RgbaImage;
use wmc_lyrics::Lyrics;
use wmc_playback::AudioData;

pub struct PreparedTrack {
    pub(crate) audio: Option<AudioData>,
    pub duration: f64,
    pub lyrics: Lyrics,
    pub lyric_content: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub cover: Option<RgbaImage>,
    pub cover_cache_allowed: bool,
    pub cover_online_needed: bool,
    pub lyrics_cache_allowed: bool,
    pub media_path: Option<std::path::PathBuf>,
    pub local_sources: wmc_enrichment::cache::LocalSources,
}

impl PreparedTrack {
    pub fn new(
        audio: AudioData,
        lyrics: Lyrics,
        cover: Option<RgbaImage>,
        title: String,
        artist: String,
        album: String,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            duration: audio.duration(),
            audio: Some(audio),
            lyrics,
            lyric_content: String::new(),
            title,
            artist,
            album,
            cover,
            cover_cache_allowed: true,
            cover_online_needed: false,
            lyrics_cache_allowed: false,
            media_path: None,
            local_sources: wmc_enrichment::cache::LocalSources::default(),
        })
    }
}

use std::time::Duration;

/// CPU preparation timings, excluding window creation and audio-device opening.
#[derive(Debug)]
pub struct PreparationTimings {
    pub metadata: Duration,
    pub audio: Duration,
    pub lyrics: Duration,
    pub cover: Duration,
    pub background: Duration,
    pub total: Duration,
}

#[derive(Clone)]
pub struct TrackPresentation {
    pub duration: f64,
    pub lyrics: Lyrics,
    pub lyric_content: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub cover: Option<RgbaImage>,
    pub cover_cache_allowed: bool,
    pub cover_online_needed: bool,
    pub lyrics_cache_allowed: bool,
    pub media_path: Option<std::path::PathBuf>,
    pub local_sources: wmc_enrichment::cache::LocalSources,
}
impl PreparedTrack {
    pub fn presentation(&self) -> TrackPresentation {
        TrackPresentation {
            duration: self.duration,
            lyrics: self.lyrics.clone(),
            lyric_content: self.lyric_content.clone(),
            title: self.title.clone(),
            artist: self.artist.clone(),
            album: self.album.clone(),
            cover: self.cover.clone(),
            cover_cache_allowed: self.cover_cache_allowed,
            cover_online_needed: self.cover_online_needed,
            lyrics_cache_allowed: self.lyrics_cache_allowed,
            media_path: self.media_path.clone(),
            local_sources: self.local_sources.clone(),
        }
    }
}

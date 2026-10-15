//! CPU song preparation results; GPU image conversion belongs to the view.
use crate::{audio::AudioData, background::Backdrop, lyrics::Lyrics};
use image::RgbaImage;

pub struct PreparedTrack {
    pub(crate) audio: AudioData,
    pub(crate) lyrics: Lyrics,
    pub(crate) lyric_content: String,
    pub(crate) title: String,
    pub(crate) artist: String,
    pub(crate) album: String,
    pub(crate) cover: RgbaImage,
    pub(crate) background: RgbaImage,
    pub(crate) backdrop: Backdrop,
    pub(crate) cover_cache_allowed: bool,
    pub(crate) cover_online_needed: bool,
    pub(crate) lyrics_cache_allowed: bool,
    pub(crate) media_path: Option<std::path::PathBuf>,
    pub(crate) local_sources: crate::media_cache::LocalSources,
}

impl PreparedTrack {
    pub(crate) fn new(
        audio: AudioData,
        lyrics: Lyrics,
        cover: RgbaImage,
        title: String,
        artist: String,
        album: String,
    ) -> anyhow::Result<Self> {
        let backdrop = Backdrop::from_cover(&cover);
        // One frame is enough to enter the player. Complete the animation cache
        // after playback starts, rather than blocking song preparation on it.
        let background = backdrop.render(0., 256, 144);
        Ok(Self {
            audio,
            lyrics,
            lyric_content: String::new(),
            title,
            artist,
            album,
            cover,
            background,
            backdrop,
            cover_cache_allowed: true,
            cover_online_needed: false,
            lyrics_cache_allowed: false,
            media_path: None,
            local_sources: crate::media_cache::LocalSources::default(),
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

//! Song configuration and CPU preparation, independent from player windows.
mod prepare;
mod prepared;
pub use prepared::{PreparationTimings, PreparedTrack, TrackPresentation};
use std::path::PathBuf;

/// File paths are optional; absent paths use original WMC artwork and an
/// independently written timed-lyric / synthesized instrumental sample.
#[derive(Default)]
pub struct PlayerConfig {
    pub audio: Option<PathBuf>,
    pub lyrics: Option<PathBuf>,
    pub cover: Option<PathBuf>,
    pub title: Option<String>,
    pub artist: Option<String>,
}

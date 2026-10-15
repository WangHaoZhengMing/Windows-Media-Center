//! Application use cases; no GPUI objects or page lifetime dependencies.
pub mod playback;
pub use playback::track::{PlayerConfig, PreparationTimings, PreparedTrack, TrackPresentation};
pub use playback::{Application, TrackLoad};
pub mod settings;

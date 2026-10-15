//! A reusable now-playing page: timed lyrics, motion, artwork and real audio.
//! Audio, song preparation and lyric timing are independent from the GPUI view.
mod audio;
pub mod background;
mod cursor;
pub use cursor::application;
mod diagnostics;
pub mod focus;
pub mod lyrics;
pub mod translation;
mod media;
mod network;
mod cache_paths;
pub mod motion;
pub mod online_lyrics;
pub mod online_covers;
pub mod media_cache;
pub mod specs;
mod system_volume;
pub mod track;
mod view;

// Preserve existing module paths for hosts and diagnostic examples.
pub use audio::clock;
pub use lyrics::{presentation, progression};
pub use track::metadata;
pub use track::{PlayerConfig, PreparationTimings, PreparedTrack};
pub use view::initialize_app_identity;
pub use view::{PlayerEvent, PlayerView, bind_keys};

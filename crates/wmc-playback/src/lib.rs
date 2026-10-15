//! Audio and queue lifetime independent from any view.
pub mod audio;
pub mod queue;
pub use audio::{AudioData, AudioPlayer, PlaybackSnapshot};
pub mod session;
pub use session::PlaybackSession;

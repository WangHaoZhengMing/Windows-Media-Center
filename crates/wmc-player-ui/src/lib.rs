//! GPUI player presentation; playback is owned by the application.
mod backdrop;
mod diagnostics;
mod view;
pub use view::{PlayerEvent, PlayerView, bind_keys};

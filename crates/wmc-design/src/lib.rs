//! Shared WMC visual language on GPUI.
mod controls;
pub use controls::{button, image_button, init};
mod assets;
pub mod components;
pub mod images;
pub mod theme;
pub use assets::MediaAssets;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortcutArtwork {
    Landscape,
    Slideshow,
    Calendar,
    Heart,
    Cinema,
    History,
    Folder,
    Record,
    Albums,
    Queue,
    Display,
    About,
}

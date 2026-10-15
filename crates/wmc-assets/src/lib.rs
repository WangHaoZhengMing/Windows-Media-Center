//! Original WMC PNG resources, embedded without modifying their bytes.
mod artwork;
mod browse;
#[path = "generated/catalog.rs"]
mod catalog;
mod icons;
mod launch;
mod source;

pub use artwork::{
    AMBIENT_GLOW, BLUE_BACKGROUND, GLASS_FRAME, HORIZONTAL_GLEAM, Shortcut, ShortcutImages,
    VERTICAL_GLEAM,
};
pub use browse::*;
pub use icons::{Icon, IconState, TRANSPORT_BACKGROUND, TRANSPORT_DIVIDER};
pub use launch::LAUNCH_IMAGES;
pub use source::{all, load};

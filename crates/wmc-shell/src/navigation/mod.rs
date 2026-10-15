//! Presentation-independent home catalog and navigation.
mod destination;
mod home;

pub use crate::pages::home::catalog::{CATEGORIES, Category, Entry, EntryId};
pub use destination::{
    DestinationEffect, DestinationFocus, DestinationKind, DestinationState, LibraryKind,
    LibraryKindExt,
};
pub use home::{Direction, HomeState};
pub use wmc_domain::MediaCommand;

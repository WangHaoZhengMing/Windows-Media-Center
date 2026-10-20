//! Local media indexing and persisted folder scope, independent of the UI.
mod config;
mod scan;
pub use config::LibraryConfig;
pub use scan::{ScanResult, scan, scan_cancellable};

pub use wmc_domain::{MediaFile, MediaKind};
mod metadata_cache;
pub mod music;
pub mod query;
mod service;
pub use metadata_cache::MetadataCache;
pub use service::{LibraryEntry, LibraryScan, inspect, inspect_stream};

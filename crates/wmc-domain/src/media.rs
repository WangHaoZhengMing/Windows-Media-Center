use std::path::PathBuf;
#[derive(Clone, Debug)]
pub struct MediaFile {
    pub path: PathBuf,
    pub name: String,
    pub kind: MediaKind,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaKind {
    Photos,
    Music,
    Videos,
}

impl MediaKind {
    pub const ALL: [Self; 3] = [Self::Photos, Self::Music, Self::Videos];
    pub const fn index(self) -> usize {
        match self {
            Self::Photos => 0,
            Self::Music => 1,
            Self::Videos => 2,
        }
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::Photos => "照片",
            Self::Music => "音乐",
            Self::Videos => "视频",
        }
    }
}

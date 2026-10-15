//! The home menu describes user destinations, never GPUI elements or asset paths.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryId {
    PhotoLibrary,
    Slideshow,
    PhotoDates,
    Favorites,
    VideoLibrary,
    RecentVideos,
    VideoFolders,
    MusicLibrary,
    Albums,
    PlayQueue,
    MediaLocations,
    Appearance,
    About,
}

pub use wmc_design::ShortcutArtwork as Artwork;

#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub id: EntryId,
    pub title: &'static str,
    pub artwork: Artwork,
}

#[derive(Clone, Copy, Debug)]
pub struct Category {
    pub title: &'static str,
    pub entries: &'static [Entry],
}

pub const CATEGORIES: [Category; 4] = [
    Category {
        title: "照片",
        entries: &[
            Entry {
                id: EntryId::PhotoLibrary,
                title: "照片库",
                artwork: Artwork::Landscape,
            },
            Entry {
                id: EntryId::Slideshow,
                title: "幻灯片",
                artwork: Artwork::Slideshow,
            },
            Entry {
                id: EntryId::PhotoDates,
                title: "拍摄日期",
                artwork: Artwork::Calendar,
            },
            Entry {
                id: EntryId::Favorites,
                title: "收藏",
                artwork: Artwork::Heart,
            },
        ],
    },
    Category {
        title: "视频",
        entries: &[
            Entry {
                id: EntryId::VideoLibrary,
                title: "视频库",
                artwork: Artwork::Cinema,
            },
            Entry {
                id: EntryId::RecentVideos,
                title: "最近播放",
                artwork: Artwork::History,
            },
            Entry {
                id: EntryId::VideoFolders,
                title: "文件夹",
                artwork: Artwork::Folder,
            },
        ],
    },
    Category {
        title: "音乐",
        entries: &[
            Entry {
                id: EntryId::MusicLibrary,
                title: "音乐库",
                artwork: Artwork::Record,
            },
            Entry {
                id: EntryId::Albums,
                title: "专辑",
                artwork: Artwork::Albums,
            },
            Entry {
                id: EntryId::PlayQueue,
                title: "播放队列",
                artwork: Artwork::Queue,
            },
        ],
    },
    Category {
        title: "设置",
        entries: &[
            Entry {
                id: EntryId::MediaLocations,
                title: "媒体位置",
                artwork: Artwork::Folder,
            },
            Entry {
                id: EntryId::Appearance,
                title: "显示与动效",
                artwork: Artwork::Display,
            },
            Entry {
                id: EntryId::About,
                title: "关于",
                artwork: Artwork::About,
            },
        ],
    },
];

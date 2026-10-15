//! The home menu describes user destinations, never GPUI elements or asset paths.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CategoryId {
    Photos,
    Videos,
    Music,
    Settings,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Artwork {
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

#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub id: EntryId,
    pub title: &'static str,
    pub description: &'static str,
    pub eyebrow: &'static str,
    pub artwork: Artwork,
}

#[derive(Clone, Copy, Debug)]
pub struct Category {
    pub id: CategoryId,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub headline: &'static str,
    pub entries: &'static [Entry],
}

pub const CATEGORIES: [Category; 4] = [
    Category {
        id: CategoryId::Photos,
        title: "照片",
        subtitle: "PICTURES",
        headline: "留住每一个瞬间",
        entries: &[
            Entry {
                id: EntryId::PhotoLibrary,
                title: "照片库",
                description: "让珍藏的风景，在大屏幕上重新绽放。",
                eyebrow: "YOUR MOMENTS",
                artwork: Artwork::Landscape,
            },
            Entry {
                id: EntryId::Slideshow,
                title: "幻灯片",
                description: "放慢脚步，让回忆一张张流过。",
                eyebrow: "SIT BACK & ENJOY",
                artwork: Artwork::Slideshow,
            },
            Entry {
                id: EntryId::PhotoDates,
                title: "拍摄日期",
                description: "沿着时间，找回那个值得记住的日子。",
                eyebrow: "A JOURNEY IN TIME",
                artwork: Artwork::Calendar,
            },
            Entry {
                id: EntryId::Favorites,
                title: "收藏",
                description: "把最喜欢的瞬间，留在触手可及的地方。",
                eyebrow: "CLOSE TO YOUR HEART",
                artwork: Artwork::Heart,
            },
        ],
    },
    Category {
        id: CategoryId::Videos,
        title: "视频",
        subtitle: "VIDEOS",
        headline: "今晚，进入另一个世界",
        entries: &[
            Entry {
                id: EntryId::VideoLibrary,
                title: "视频库",
                description: "为一个好故事，留出一段属于自己的时间。",
                eyebrow: "THE BIG SCREEN",
                artwork: Artwork::Cinema,
            },
            Entry {
                id: EntryId::RecentVideos,
                title: "最近播放",
                description: "接着上次的故事，继续看下去。",
                eyebrow: "PICK UP WHERE YOU LEFT",
                artwork: Artwork::History,
            },
            Entry {
                id: EntryId::VideoFolders,
                title: "文件夹",
                description: "按你熟悉的方式，浏览本地视频。",
                eyebrow: "YOUR COLLECTION",
                artwork: Artwork::Folder,
            },
        ],
    },
    Category {
        id: CategoryId::Music,
        title: "音乐",
        subtitle: "MUSIC",
        headline: "让房间充满旋律",
        entries: &[
            Entry {
                id: EntryId::MusicLibrary,
                title: "音乐库",
                description: "一首熟悉的歌，就能改变此刻的心情。",
                eyebrow: "SOUNDTRACK OF YOUR DAY",
                artwork: Artwork::Record,
            },
            Entry {
                id: EntryId::Albums,
                title: "专辑",
                description: "从第一首到最后一首，完整听一张专辑。",
                eyebrow: "ONE ALBUM AT A TIME",
                artwork: Artwork::Albums,
            },
            Entry {
                id: EntryId::PlayQueue,
                title: "播放队列",
                description: "为接下来的时间，选好你的背景音乐。",
                eyebrow: "UP NEXT",
                artwork: Artwork::Queue,
            },
        ],
    },
    Category {
        id: CategoryId::Settings,
        title: "设置",
        subtitle: "SETTINGS",
        headline: "让这里，更像你的客厅",
        entries: &[
            Entry {
                id: EntryId::MediaLocations,
                title: "媒体位置",
                description: "为照片、视频和音乐安排一个家。",
                eyebrow: "MAKE YOURSELF AT HOME",
                artwork: Artwork::Folder,
            },
            Entry {
                id: EntryId::Appearance,
                title: "显示与动效",
                description: "选择流动的背景，或一份安静的蓝色。",
                eyebrow: "YOUR COMFORT",
                artwork: Artwork::Display,
            },
            Entry {
                id: EntryId::About,
                title: "关于",
                description: "熟悉的客厅体验，用 Rust 重新构建。",
                eyebrow: "A NEW BEGINNING",
                artwork: Artwork::About,
            },
        ],
    },
];

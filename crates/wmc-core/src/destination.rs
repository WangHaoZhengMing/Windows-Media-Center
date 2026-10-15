use crate::{Direction, EntryId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryKind {
    Photos,
    Videos,
    Music,
}

impl LibraryKind {
    pub const fn title(self) -> &'static str {
        match self {
            Self::Photos => "照片库",
            Self::Videos => "视频库",
            Self::Music => "音乐库",
        }
    }

    pub const fn tabs(self) -> &'static [&'static str] {
        match self {
            Self::Photos => &["文件夹", "拍摄日期", "评分", "标签"],
            Self::Videos => &["文件夹", "拍摄日期", "标题", "最近播放"],
            Self::Music => &[
                "专辑",
                "艺术家",
                "流派",
                "歌曲",
                "播放列表",
                "作曲家",
                "年份",
                "专辑艺术家",
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DestinationKind {
    Library(LibraryKind),
    Queue,
    MediaLocations,
    Appearance,
    About,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DestinationFocus {
    Commands,
    Pivots,
    Content,
}

pub enum DestinationEffect {
    None,
    Close,
    ApplyAppearance(bool),
}

/// View state for native browse/command-content layouts, independent of GPUI.
pub struct DestinationState {
    pub kind: DestinationKind,
    pub tab: usize,
    pub row: usize,
    pub focus: DestinationFocus,
    pub draft_ambient: bool,
    pub shuffle: bool,
    pub repeat: bool,
}

impl Default for DestinationState {
    fn default() -> Self {
        Self::new(EntryId::PhotoLibrary, true)
    }
}

impl DestinationState {
    pub fn new(entry: EntryId, ambient: bool) -> Self {
        use EntryId::*;
        let (kind, tab) = match entry {
            PhotoLibrary | Slideshow => (DestinationKind::Library(LibraryKind::Photos), 0),
            PhotoDates => (DestinationKind::Library(LibraryKind::Photos), 1),
            Favorites => (DestinationKind::Library(LibraryKind::Photos), 2),
            VideoLibrary | VideoFolders => (DestinationKind::Library(LibraryKind::Videos), 0),
            RecentVideos => (DestinationKind::Library(LibraryKind::Videos), 3),
            MusicLibrary | Albums => (DestinationKind::Library(LibraryKind::Music), 0),
            PlayQueue => (DestinationKind::Queue, 0),
            MediaLocations => (DestinationKind::MediaLocations, 0),
            Appearance => (DestinationKind::Appearance, 0),
            About => (DestinationKind::About, 0),
        };
        Self {
            kind,
            tab,
            row: if kind == DestinationKind::Appearance {
                1
            } else {
                0
            },
            focus: if matches!(kind, DestinationKind::Library(_)) {
                DestinationFocus::Pivots
            } else {
                DestinationFocus::Commands
            },
            draft_ambient: ambient,
            shuffle: false,
            repeat: false,
        }
    }

    pub fn reset(&mut self, entry: EntryId, ambient: bool) {
        let (shuffle, repeat) = (self.shuffle, self.repeat);
        *self = Self::new(entry, ambient);
        self.shuffle = shuffle;
        self.repeat = repeat;
    }

    pub fn choose_tab(&mut self, tab: usize) {
        if let DestinationKind::Library(kind) = self.kind
            && tab < kind.tabs().len()
        {
            self.tab = tab;
            self.focus = DestinationFocus::Pivots;
        }
    }

    pub fn navigate(&mut self, direction: Direction) {
        use Direction::*;
        match self.kind {
            DestinationKind::Library(kind) => match direction {
                Left => self.tab = self.tab.saturating_sub(1),
                Right => self.tab = (self.tab + 1).min(kind.tabs().len() - 1),
                Up => {
                    self.focus = match self.focus {
                        DestinationFocus::Content => DestinationFocus::Pivots,
                        _ => DestinationFocus::Commands,
                    }
                }
                Down => {
                    self.focus = match self.focus {
                        DestinationFocus::Commands => DestinationFocus::Pivots,
                        _ => DestinationFocus::Content,
                    }
                }
            },
            DestinationKind::Appearance => match direction {
                Right => self.row = 0,
                Left => self.row = 1,
                Up if self.row > 0 => self.row = 1,
                Down if self.row > 0 => self.row = 2,
                _ => {}
            },
            kind => {
                let max = match kind {
                    DestinationKind::Queue => 4,
                    DestinationKind::MediaLocations => 2,
                    _ => 0,
                };
                match direction {
                    Up => self.row = self.row.saturating_sub(1),
                    Down => self.row = (self.row + 1).min(max),
                    _ => {}
                }
            }
        }
    }

    pub fn activate(&mut self) -> DestinationEffect {
        match self.kind {
            DestinationKind::Library(_) => {
                if self.focus == DestinationFocus::Pivots {
                    self.focus = DestinationFocus::Content;
                }
                DestinationEffect::None
            }
            DestinationKind::Appearance => match self.row {
                0 => {
                    self.draft_ambient = !self.draft_ambient;
                    DestinationEffect::None
                }
                1 => DestinationEffect::ApplyAppearance(self.draft_ambient),
                _ => DestinationEffect::Close,
            },
            DestinationKind::About => DestinationEffect::Close,
            DestinationKind::Queue => {
                match self.row {
                    3 => self.shuffle = !self.shuffle,
                    4 => self.repeat = !self.repeat,
                    _ => {}
                }
                DestinationEffect::None
            }
            DestinationKind::MediaLocations => DestinationEffect::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_routes_to_a_valid_native_page() {
        for category in crate::CATEGORIES {
            for entry in category.entries {
                let state = DestinationState::new(entry.id, true);
                if let DestinationKind::Library(kind) = state.kind {
                    assert!(state.tab < kind.tabs().len());
                }
            }
        }
        assert_eq!(DestinationState::new(EntryId::PhotoDates, true).tab, 1);
        assert_eq!(DestinationState::new(EntryId::RecentVideos, true).tab, 3);
    }

    #[test]
    fn browse_navigation_changes_pivots_without_touching_home_selection() {
        let mut page = DestinationState::new(EntryId::MusicLibrary, true);
        for _ in 0..30 {
            page.navigate(Direction::Right);
        }
        assert_eq!(page.tab, LibraryKind::Music.tabs().len() - 1);
        page.navigate(Direction::Down);
        assert_eq!(page.focus, DestinationFocus::Content);
        page.navigate(Direction::Up);
        assert_eq!(page.focus, DestinationFocus::Pivots);
        page.choose_tab(usize::MAX);
        assert_eq!(page.tab, LibraryKind::Music.tabs().len() - 1);
    }

    #[test]
    fn appearance_edits_are_staged_until_save() {
        let mut page = DestinationState::new(EntryId::Appearance, true);
        page.navigate(Direction::Right);
        page.activate();
        assert!(!page.draft_ambient);
        page.navigate(Direction::Left);
        assert!(matches!(
            page.activate(),
            DestinationEffect::ApplyAppearance(false)
        ));
        page.navigate(Direction::Down);
        assert!(matches!(page.activate(), DestinationEffect::Close));
    }
}

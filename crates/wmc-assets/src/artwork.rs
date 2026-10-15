pub const BLUE_BACKGROUND: &str = "wmc/backgrounds/COMMON.ANIMATED.BACKGROUND.PNG";
pub const AMBIENT_GLOW: &str = "wmc/backgrounds/COMMON.BACKGROUND.LAYER3A.PNG";
pub const GLASS_FRAME: &str = "wmc/glass/STARTMENU.QUICKLINK.BACKGROUND.PNG";
pub const HORIZONTAL_GLEAM: &str = "wmc/glass/SELECTOR.FOCUSLOOP.HOTSPOT.HORIZONTAL.PNG";
pub const VERTICAL_GLEAM: &str = "wmc/glass/SELECTOR.FOCUSLOOP.HOTSPOT.VERTICAL.PNG";

/// Original 258 × 145 focus art and 162 × 91 dormant art form a matched pair.
#[derive(Clone, Copy, Debug)]
pub struct ShortcutImages {
    pub focused: &'static str,
    pub idle: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub enum Shortcut {
    Photos,
    PhotoFavorites,
    Videos,
    Music,
    MusicFavorites,
    Settings,
    LearnMore,
}

impl Shortcut {
    pub const fn images(self) -> ShortcutImages {
        macro_rules! pair {
            ($name:literal) => {
                ShortcutImages {
                    focused: concat!("wmc/shortcuts/STARTMENU.QUICKLINK.", $name, ".FOCUS.PNG"),
                    idle: concat!("wmc/shortcuts/STARTMENU.QUICKLINK.", $name, ".NOFOCUS.PNG"),
                }
            };
        }
        match self {
            Self::Photos => pair!("PICTURES"),
            Self::PhotoFavorites => pair!("PICTURES.PLAYFAVS"),
            Self::Videos => pair!("VIDEOS"),
            Self::Music => pair!("MUSIC"),
            Self::MusicFavorites => pair!("MUSIC.PLAYFAVS"),
            Self::Settings => pair!("SETTINGS"),
            Self::LearnMore => pair!("LEARNMORE"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Icon {
    Home,
    Play,
    Pause,
    Calendar,
    Folder,
    Queue,
    Display,
    Back,
    Previous,
    Next,
    Rewind,
    FastForward,
    Stop,
    Mute,
    Muted,
    VolumeDown,
    VolumeUp,
    Playlist,
    Minimize,
    Maximize,
    Restore,
    Close,
}

#[derive(Clone, Copy)]
pub enum IconState {
    Normal,
    Focused,
    Disabled,
}

impl Icon {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Home => "主页",
            Self::Play => "播放",
            Self::Pause => "暂停",
            Self::Calendar => "日历",
            Self::Folder => "文件夹",
            Self::Queue | Self::Playlist => "播放队列",
            Self::Display => "显示设置",
            Self::Back => "返回",
            Self::Previous => "上一首",
            Self::Next => "下一首",
            Self::Rewind => "快退",
            Self::FastForward => "快进",
            Self::Stop => "停止",
            Self::Mute => "静音",
            Self::Muted => "取消静音",
            Self::VolumeDown => "降低音量",
            Self::VolumeUp => "提高音量",
            Self::Minimize => "最小化",
            Self::Maximize => "最大化",
            Self::Restore => "还原窗口",
            Self::Close => "关闭",
        }
    }

    pub const fn path(self) -> &'static str {
        self.image(IconState::Normal)
    }

    pub const fn image(self, state: IconState) -> &'static str {
        macro_rules! control {
            ($name:literal) => {
                match state {
                    IconState::Normal => {
                        concat!("wmc/controls/TWOFOOT.CONTROL.", $name, ".NOFOCUS.PNG")
                    }
                    IconState::Focused => {
                        concat!("wmc/controls/TWOFOOT.CONTROL.", $name, ".FOCUS.PNG")
                    }
                    IconState::Disabled => {
                        concat!("wmc/controls/TWOFOOT.CONTROL.", $name, ".DISABLED.PNG")
                    }
                }
            };
        }
        match self {
            Self::Home => control!("MENU"),
            Self::Back => control!("GOBACK"),
            Self::Play => control!("PLAY"),
            Self::Pause => control!("PAUSE"),
            Self::Previous => control!("PREVIOUS"),
            Self::Next => control!("NEXT"),
            Self::Rewind => control!("BACKWARD"),
            Self::FastForward => control!("FORWARD"),
            Self::Stop => control!("STOP"),
            Self::Mute => control!("MUTE"),
            Self::Muted => control!("MUTEON"),
            Self::VolumeDown => control!("MINUS"),
            Self::VolumeUp => control!("PLUS"),
            Self::Playlist => control!("GUIDE"),
            Self::Minimize => control!("MINIMIZE"),
            Self::Maximize => control!("MAXIMIZE"),
            Self::Restore => control!("RESTORE"),
            Self::Close => control!("EXIT"),
            Self::Calendar => "wmc/icons/BROWSE.CMD.SCHEDULED.FOCUS.PNG",
            Self::Folder => "wmc/icons/COMMON.ICON.SUBFOLDER.LARGE.PNG",
            Self::Queue => "wmc/icons/AUDIO.ADDTOQUEUE.ICON.PNG",
            Self::Display => "wmc/icons/OPTIONS.CALIBRATION.ICON.DISPLAY.FLAT.PNG",
        }
    }
}

pub const TRANSPORT_BACKGROUND: &str = "wmc/controls/TWOFOOT.FULLSCREEN.BOTTOMBAR.RIGHT.PNG";
pub const TRANSPORT_DIVIDER: &str = "wmc/controls/TWOFOOT.CONTROL.DIVIDER.PNG";

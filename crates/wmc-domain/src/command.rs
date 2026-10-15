/// Media controls are independent from homepage navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaCommand {
    PlayPause,
    Stop,
    Previous,
    Next,
    Rewind,
    FastForward,
    ToggleMute,
    VolumeDown,
    VolumeUp,
}

use gpui_kit::{App, KeyBinding};
gpui_kit::actions!(
    media_center,
    [
        MoveUp,
        MoveDown,
        MoveLeft,
        MoveRight,
        Open,
        Back,
        Fullscreen,
        ContextMenu,
        DetailsNext,
        DetailsPrevious
    ]
);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", MoveUp, Some("MediaCenter")),
        KeyBinding::new("down", MoveDown, Some("MediaCenter")),
        KeyBinding::new("left", MoveLeft, Some("MediaCenter")),
        KeyBinding::new("right", MoveRight, Some("MediaCenter")),
        KeyBinding::new("enter", Open, Some("MediaCenter")),
        KeyBinding::new("escape", Back, Some("MediaCenter")),
        KeyBinding::new("backspace", Back, Some("MediaCenter")),
        KeyBinding::new("f11", Fullscreen, Some("MediaCenter")),
        KeyBinding::new("shift-f10", ContextMenu, Some("MediaCenter")),
        KeyBinding::new("tab", DetailsNext, Some("MediaCenter")),
        KeyBinding::new("shift-tab", DetailsPrevious, Some("MediaCenter")),
    ]);
}

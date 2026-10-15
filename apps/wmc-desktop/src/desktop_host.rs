use gpui_kit::{Application, Platform};
use std::rc::Rc;
pub fn application() -> (Application, Rc<dyn Fn()>) {
    let platform: Rc<dyn Platform> = gpui_kit::platform::current_platform(false);
    let cursor_platform = platform.clone();
    let hide_cursor = Rc::new(move || cursor_platform.hide_cursor_until_mouse_moves());
    (Application::with_platform(platform), hide_cursor)
}

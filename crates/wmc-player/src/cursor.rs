//! Retain the application's real platform handle for safe cursor auto-hiding.
use gpui_kit::{Application, Platform};
use std::{cell::RefCell, rc::Rc};
thread_local! { static PLATFORM: RefCell<Option<Rc<dyn Platform>>> = RefCell::new(None); }

/// Desktop application constructor with playback cursor auto-hide support.
/// Hosts embedding PlayerView should use this in place of gpui_kit::application.
pub fn application() -> Application {
    let platform = gpui_kit::platform::current_platform(false);
    PLATFORM.with(|slot| *slot.borrow_mut() = Some(platform.clone()));
    Application::with_platform(platform)
}
pub(crate) fn hide_until_mouse_moves() {
    PLATFORM.with(|slot| {
        if let Some(platform) = slot.borrow().as_ref() {
            platform.hide_cursor_until_mouse_moves();
        }
    });
}

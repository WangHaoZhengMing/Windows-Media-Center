use gpui_kit::{Hsla, Window, rgb};

pub(crate) const WHITE: u32 = 0xf1faff;
pub(crate) const MUTED: u32 = 0x6f9fc8;
pub(crate) const INK: u32 = 0x060e2d;
pub(crate) const SHORTCUT_WIDTH: f32 = 238.;
pub(crate) const SHORTCUT_HEIGHT: f32 = SHORTCUT_WIDTH * 145. / 258.;

pub(crate) fn color(hex: u32) -> Hsla {
    rgb(hex).into()
}

/// One design coordinate system, scaled to the available client area.
#[derive(Clone, Copy)]
pub(crate) struct Layout {
    pub scale: f32,
    pub height: f32,
}

impl Layout {
    pub fn for_window(window: &Window) -> Self {
        let viewport = window.viewport_size();
        let width = f32::from(viewport.width);
        let height = f32::from(viewport.height);
        Self {
            scale: (width / 1280.).min(height / 720.),
            height,
        }
    }
    pub fn unit(self, value: f32) -> gpui_kit::Pixels {
        gpui_kit::px(value * self.scale)
    }
    pub fn content_top(self) -> f32 {
        self.height * 0.50
    }
}

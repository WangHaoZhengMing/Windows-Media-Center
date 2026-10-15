//! White native text; the lyric rasterizer uses the same bold system font.
use gpui_kit::{prelude::*, *};

pub(super) fn shape(
    text: &str,
    size: f32,
    weight: FontWeight,
    color: Hsla,
    window: &Window,
) -> ShapedLine {
    crate::diagnostics::shape();
    let mut face = font("Microsoft YaHei UI");
    face.weight = weight;
    window.text_system().shape_line(
        text.to_owned().into(),
        px(size),
        &[TextRun {
            len: text.len(),
            font: face,
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        }],
        None,
    )
}

pub(super) struct LitText<'a> {
    pub text: &'a str,
    pub size: f32,
    pub weight: FontWeight,
    pub alpha: f32,
}

impl LitText<'_> {
    pub fn paint(&self, origin: Point<Pixels>, height: Pixels, window: &mut Window, cx: &mut App) {
        let white = shape(self.text, self.size, self.weight,
            rgb(0xffffff).opacity(self.alpha).into(), window);
        let _ = white.paint(origin, height, TextAlign::Left, None, window, cx);
    }
}

pub(super) fn caption(text: String, size: f32, weight: FontWeight, scale: f32) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, cx| {
            LitText {
                text: &text,
                size: size * scale,
                weight,
                alpha: 0.97,
            }
            .paint(bounds.origin, bounds.size.height, window, cx);
        },
    )
    .size_full()
}

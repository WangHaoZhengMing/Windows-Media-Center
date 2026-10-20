//! Native text with the same per-grapheme fallback policy as lyric rasterization.
use gpui_kit::{prelude::*, *};

pub(super) fn font_pixels(size: f32) -> Pixels {
    px((size * 2.).round().max(1.) / 2.)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[::core::prelude::v1::test]
    fn continuous_resizing_reuses_a_bounded_set_of_native_label_sizes() {
        let mut sizes = HashSet::new();
        for step in 0..10_000 {
            let requested = 12. + 12. * step as f32 / 9_999.;
            let actual = f32::from(font_pixels(requested));
            sizes.insert(actual.to_bits());
            assert!((actual - requested).abs() <= 0.251);
        }
        assert!(sizes.len() <= 25);
    }
}

pub(super) fn shape(
    text: &str,
    size: f32,
    weight: FontWeight,
    color: Hsla,
    window: &Window,
) -> ShapedLine {
    crate::diagnostics::shape();
    let spans = super::lyrics::font_catalog::select(text, weight.0 as u16);
    let mut runs: Vec<_> = spans
        .iter()
        .map(|span| {
            let mut face = font(span.family.clone());
            face.weight = weight;
            let native = window.text_system().resolve_font(&face);
            super::lyrics::font_catalog::register(native, span.id);
            TextRun {
                len: span.len,
                font: face,
                color,
                background_color: None,
                underline: None,
                strikethrough: None,
            }
        })
        .collect();
    if runs.is_empty() {
        let mut face = font("SF Pro Display");
        face.weight = weight;
        face.fallbacks = Some(FontFallbacks::from_fonts(vec![
            "Microsoft YaHei UI".into(),
            "Noto Sans".into(),
            "DejaVu Sans".into(),
        ]));
        runs.push(TextRun {
            len: text.len(),
            font: face,
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        });
    }
    window
        .text_system()
        .shape_line(text.to_owned().into(), px(size), &runs, None)
}

pub(super) struct LitText<'a> {
    pub text: &'a str,
    pub size: f32,
    pub weight: FontWeight,
    pub alpha: f32,
}

impl LitText<'_> {
    pub fn paint(&self, origin: Point<Pixels>, height: Pixels, window: &mut Window, cx: &mut App) {
        let white = shape(
            self.text,
            f32::from(font_pixels(self.size)),
            self.weight,
            rgb(0xffffff).opacity(self.alpha).into(),
            window,
        );
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

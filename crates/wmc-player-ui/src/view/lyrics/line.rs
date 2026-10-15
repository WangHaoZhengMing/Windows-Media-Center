//! Playback changes GPU image clipping, not font shaping or glyph rasterization.
use super::super::{resources::guarded_image_element, text::LitText};
use super::{layout::LineLayout, raster::RasterizedLine};
use gpui_kit::{prelude::*, *};
use std::sync::Arc;
use wmc_motion::lyrics::{highlight as progression, profile as specs};

fn slice(
    image: Arc<RenderImage>,
    x: f32,
    width: f32,
    image_width: f32,
    image_height: f32,
    alpha: f32,
) -> Div {
    div()
        .absolute()
        .left(px(x))
        .top_0()
        .w(px(width.max(0.)))
        .h(px(image_height))
        .overflow_hidden()
        .opacity(alpha)
        .child(
            guarded_image_element(image)
                .absolute()
                .left(px(-x))
                .top_0()
                .w(px(image_width))
                .h(px(image_height)),
        )
}

pub(in crate::view) fn render(
    layout: Arc<LineLayout>,
    cached: Option<Arc<RasterizedLine>>,
    seconds: f64,
    highlight: f32,
    scale: f32,
) -> AnyElement {
    let Some(cached) = cached else {
        return fallback(layout, seconds, highlight, scale).into_any_element();
    };
    let cursor = layout.cursor(seconds, highlight);
    let feather = specs::PROGRESSION_FEATHER;
    let mut offset = 0.;
    let mut row = div().relative().size_full();
    for (word, images) in layout.words.iter().zip(&cached.words) {
        // Cumulative advance does not reset at a word or visual row boundary.
        // Texture padding belongs to the halo, not the timed text distance.
        let coverage = |x: f32| {
            progression::coverage_at(
                cursor,
                offset + (x / scale - images.padding).clamp(0., word.width),
                feather,
            )
        };
        let width = images.width * scale;
        let height = images.height * scale;
        let mut tile = div()
            .absolute()
            .left(px((word.x - images.padding) * scale))
            .top(px((word.y - images.padding) * scale))
            .w(px(width))
            .h(px(height))
            .child(
                guarded_image_element(images.normal.clone())
                    .absolute()
                    .size_full()
                    .opacity(0.8 - 0.46 * highlight),
            );
        if highlight > 0. && coverage(0.) > 0. {
            if coverage(width) >= 1. {
                tile = tile.child(
                    guarded_image_element(images.lit.clone())
                        .absolute()
                        .size_full()
                        .opacity(highlight),
                );
            } else {
                // Fixed sample positions avoid moving, pixel-snapped stripe boundaries.
                // All words sample the same feather, including its cross-word tail.
                let bands = width.ceil().clamp(1., 160.) as usize;
                let band_width = width / bands as f32;
                let mut solid = 0.;
                for band in 0..bands {
                    let x = band as f32 * band_width;
                    let alpha = coverage(x + band_width / 2.);
                    if alpha >= 1. {
                        solid = x + band_width;
                    } else if alpha > 0. {
                        tile = tile.child(slice(
                            images.lit.clone(),
                            x,
                            band_width,
                            width,
                            height,
                            highlight * alpha,
                        ));
                    }
                }
                if solid > 0. {
                    tile = tile.child(slice(
                        images.lit.clone(),
                        0.,
                        solid,
                        width,
                        height,
                        highlight,
                    ));
                }
            }
        }
        offset += word.width;
        row = row.child(tile);
    }
    row.into_any_element()
}

// Immediate native text while the bounded background queue prepares textures,
// also retaining a useful fallback if an installed raster font is unavailable.
fn fallback(layout: Arc<LineLayout>, seconds: f64, highlight: f32, scale: f32) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, cx| {
            let _profile = crate::diagnostics::Timer::paint();
            let cursor = layout.cursor(seconds, highlight);
            let mut offset = 0.;
            for word in &layout.words {
                let origin = bounds.origin + point(px(word.x * scale), px(word.y * scale));
                let height = px(layout.line_height * scale);
                let _ = word
                    .shaped
                    .paint(origin, height, TextAlign::Left, None, window, cx);
                let feather = specs::PROGRESSION_FEATHER;
                if highlight <= 0. || progression::coverage_at(cursor, offset, feather) <= 0. {
                    offset += word.width;
                    continue;
                }
                // Bounded native fallback while textures load; fixed slices share
                // the same line cursor, so cache arrival does not reset a word.
                let bands = if progression::coverage_at(cursor, offset + word.width, feather) >= 1.
                {
                    1
                } else {
                    16
                };
                let band_width = (word.width + 2.) / bands as f32;
                for band in 0..bands {
                    let x = -1. + band as f32 * band_width;
                    let alpha = progression::coverage_at(
                        cursor,
                        offset + (x + band_width / 2.).clamp(0., word.width),
                        feather,
                    );
                    if alpha <= 0. {
                        continue;
                    }
                    let mask = ContentMask {
                        bounds: Bounds::new(
                            origin + point(px(x * scale), px(-4. * scale)),
                            size(px(band_width * scale), height + px(8. * scale)),
                        ),
                    };
                    window.with_content_mask(Some(mask), |window| {
                        LitText {
                            text: &word.text,
                            size: layout.font_size * scale,
                            weight: FontWeight::BOLD,
                            alpha: highlight * alpha,
                        }
                        .paint(origin, height, window, cx);
                    });
                }
                offset += word.width;
            }
        },
    )
    .size_full()
}

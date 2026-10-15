//! Cached word lighting and Gaussian masks generated off the UI thread.
use super::super::resources::{guarded_image as render_image, guarded_image_element};
use super::{font::SongFont, layout::LineLayout};
use gpui_kit::{Div, RenderImage, Window, div, prelude::*, px};
use image::{GrayImage, Rgba, RgbaImage, imageops};
use std::sync::Arc;
const LEVELS: [f32; 7] = [0., 1., 2., 3.5, 5., 7., 9.];
const PADDING: f32 = 30.;
const WORD_PADDING: f32 = 4.;

pub(in crate::view) struct WordImages {
    pub normal: Arc<RenderImage>,
    pub lit: Arc<RenderImage>,
    pub width: f32,
    pub height: f32,
    pub padding: f32,
}

pub(in crate::view) struct RasterizedLine {
    pub(super) layers: Vec<Arc<RenderImage>>,
    pub words: Vec<WordImages>,
    pub(super) width: f32,
    pub(super) height: f32,
}

impl RasterizedLine {
    pub fn blurred(&self, sigma: f32, scale: f32) -> Div {
        let upper = LEVELS
            .partition_point(|level| *level < sigma)
            .min(LEVELS.len() - 1);
        let lower = upper.saturating_sub(1);
        let mix = if upper == lower {
            0.
        } else {
            (sigma - LEVELS[lower]) / (LEVELS[upper] - LEVELS[lower])
        };
        div()
            .absolute()
            .left(px(-PADDING * scale))
            .top(px(-PADDING * scale))
            .w(px(self.width * scale))
            .h(px(self.height * scale))
            .opacity(1. - (1. - sigma / 0.9).clamp(0., 1.))
            .child(
                guarded_image_element(self.layers[lower].clone())
                    .absolute()
                    .size_full()
                    .opacity(1. - mix.clamp(0., 1.)),
            )
            .child(
                guarded_image_element(self.layers[upper].clone())
                    .absolute()
                    .size_full()
                    .opacity(mix.clamp(0., 1.)),
            )
    }

    pub(super) fn drop_images(&self, window: &mut Window) {
        for image in &self.layers {
            let _ = window.drop_image(image.clone());
        }
        for word in &self.words {
            let _ = window.drop_image(word.normal.clone());
            let _ = window.drop_image(word.lit.clone());
        }
    }
}

pub(in crate::view) struct RasterJob {
    pub(super) font: Arc<SongFont>,
    pub(super) layout: Arc<LineLayout>,
    pub(super) scale: f32,
    pub generation: u64,
}

impl RasterJob {
    pub fn run(self) -> RasterizedLine {
        let _profile = crate::diagnostics::Timer::blur();
        let scale = self.scale;
        let padding = (PADDING * scale).ceil() as u32;
        let wp = (WORD_PADDING * scale).ceil() as u32;
        let mut mask = GrayImage::new(
            (self.layout.width * scale).ceil() as u32 + padding * 2,
            (self.layout.height * scale).ceil() as u32 + padding * 2,
        );
        let mut words = Vec::with_capacity(self.layout.words.len());
        for word in &self.layout.words {
            let shaped = &word.shaped;
            let mut word_mask = GrayImage::new(
                (word.width * scale).ceil() as u32 + wp * 2,
                (self.layout.line_height * scale).ceil() as u32 + wp * 2,
            );
            let baseline = f32::from(
                (px(self.layout.line_height * scale) - shaped.ascent - shaped.descent) / 2.
                    + shaped.ascent,
            );
            for glyph in shaped.runs.iter().flat_map(|run| &run.glyphs) {
                let Some(character) = word.text[glyph.index..].chars().next() else {
                    continue;
                };
                let Some(metrics) = self
                    .font
                    .rasterize(character, self.layout.font_size * scale)
                else {
                    continue;
                };
                let gx = wp as i32 + f32::from(glyph.position.x).round() as i32 + metrics.xmin;
                let gy = wp as i32 + (baseline + f32::from(glyph.position.y)).round() as i32
                    - metrics.ymin
                    - metrics.height as i32;
                for row in 0..metrics.height {
                    for col in 0..metrics.width {
                        let x = gx + col as i32;
                        let y = gy + row as i32;
                        if x >= 0
                            && y >= 0
                            && x < word_mask.width() as i32
                            && y < word_mask.height() as i32
                        {
                            let pixel = word_mask.get_pixel_mut(x as u32, y as u32);
                            pixel.0[0] = pixel.0[0].max(metrics.bitmap[row * metrics.width + col]);
                        }
                    }
                }
            }
            // Merge this same mask into the line; sharp text and blur share geometry.
            for (x, y, alpha) in word_mask.enumerate_pixels() {
                let mx = padding as i32 + (word.x * scale).round() as i32 + x as i32 - wp as i32;
                let my = padding as i32 + (word.y * scale).round() as i32 + y as i32 - wp as i32;
                if mx >= 0 && my >= 0 && mx < mask.width() as i32 && my < mask.height() as i32 {
                    let pixel = mask.get_pixel_mut(mx as u32, my as u32);
                    pixel.0[0] = pixel.0[0].max(alpha.0[0]);
                }
            }
            let normal = tint(&word_mask, [255, 255, 255], 1.);
            let lit = lighting(&word_mask, wp, scale, self.layout.line_height);
            words.push(WordImages {
                width: word_mask.width() as f32 / scale,
                padding: wp as f32 / scale,
                height: word_mask.height() as f32 / scale,
                normal: render_image(normal),
                lit: render_image(lit),
            });
        }
        let layers = LEVELS
            .iter()
            .map(|sigma| {
                let blurred = if *sigma == 0. {
                    mask.clone()
                } else {
                    imageops::blur(&mask, *sigma * scale)
                };
                render_image(tint(&blurred, [255, 255, 255], 0.8))
            })
            .collect();
        RasterizedLine {
            layers,
            words,
            width: mask.width() as f32 / scale,
            height: mask.height() as f32 / scale,
        }
    }
}

fn tint(mask: &GrayImage, color: [u8; 3], opacity: f32) -> RgbaImage {
    RgbaImage::from_fn(mask.width(), mask.height(), |x, y| {
        Rgba([
            color[0],
            color[1],
            color[2],
            (f32::from(mask.get_pixel(x, y).0[0]) * opacity).round() as u8,
        ])
    })
}

fn lighting(mask: &GrayImage, _padding: u32, _scale: f32, _line_height: f32) -> RgbaImage {
    tint(mask, [255, 255, 255], 1.)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::layout::LYRIC_LINE_HEIGHT;
    #[test]
    fn cached_lyrics_are_white_with_transparent_surroundings() {
        let mut mask = GrayImage::new(24, 74);
        mask.put_pixel(12, 20, image::Luma([255]));
        let lit = lighting(&mask, 4, 1., LYRIC_LINE_HEIGHT);
        assert_eq!(lit.get_pixel(0, 0)[3], 0);
        assert_eq!(lit.get_pixel(12, 20).0, [255, 255, 255, 255]);
        assert_eq!(lit.get_pixel(13, 20)[3], 0);
    }
}

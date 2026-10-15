//! CPU translation of the recovered three-artwork-layer SKSL equations.
//! Render on a small surface, blur there, then let GPUI enlarge the result.
use image::{Rgba, RgbaImage, imageops};
use std::{
    f32::consts::{PI, TAU},
    sync::Arc,
};

#[derive(Clone)]
pub struct Backdrop {
    texture: Arc<RgbaImage>,
    black_scrim: f32,
}
#[derive(Clone, Copy)]
pub(crate) struct Parameters {
    pub time: f32,
    pub width: f32,
    pub height: f32,
    pub spectrum: [f32; 3],
    pub has_lyrics: bool,
    pub blur: f32,
}
impl Backdrop {
    pub fn from_cover(cover: &RgbaImage) -> Self {
        let texture = if cover.width() == 0 || cover.height() == 0 {
            RgbaImage::from_pixel(128, 128, Rgba([50, 50, 50, 255]))
        } else {
            let mut opaque = cover.clone();
            for pixel in opaque.pixels_mut() {
                for i in 0..3 {
                    pixel[i] = (u16::from(pixel[i]) * u16::from(pixel[3]) / 255) as u8;
                }
                pixel[3] = 255;
            }
            imageops::resize(&opaque, 128, 128, imageops::FilterType::Triangle)
        };
        
        let mut sum = 0.;
        let mut squared = 0.;
        let mut bright = 0.;
        let mut neutral = 0.;
        let mut dark = 0.;
        let mut chroma = 0.;
        for pixel in texture.pixels() {
            let rgb = [pixel[0], pixel[1], pixel[2]].map(|v| v as f32 / 255.);
            let luma = rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722;
            let range = rgb.into_iter().fold(f32::NEG_INFINITY, f32::max)
                - rgb.into_iter().fold(f32::INFINITY, f32::min);
            sum += luma;
            squared += luma * luma;
            chroma += range;
            if luma > 0.6 {
                bright += 1.;
            }
            if luma < 0.15 {
                dark += 1.;
            }
            if luma > 0.5 && range < 0.12 {
                neutral += 1.;
            }
        }
        let mean = sum / 16384.;
        let bright = bright / 16384.;
        let neutral = neutral / 16384.;
        let dark = dark / 16384.;
        let chroma = chroma / 16384.;
        let deviation = (squared / 16384. - mean * mean).max(0.).sqrt();
        let light = chroma > 0.72
            || (bright > 0.55 && mean > 0.5 && neutral < 0.35 && chroma > 0.15)
            || (mean > 0.6 && bright > 0.55)
            || bright > 0.7
            || (dark > 0.3 && bright > 0.2 && deviation > 0.3);
        Self {
            texture: Arc::new(texture),
            black_scrim: if light { 0.5 } else { 0.35 },
        }
    }
    /// Compatibility entry point for static previews. `time` is shader time,
    /// deliberately not a periodic 24-frame phase.
    pub fn render(&self, time: f32, width: u32, height: u32) -> RgbaImage {
        self.render_frame(
            Parameters {
                time,
                width: width as f32,
                height: height as f32,
                spectrum: [0.; 3],
                has_lyrics: true,
                blur: 60.,
            },
            width,
            height,
        )
    }
    pub(crate) fn surface_size(width: f32, height: f32, wide: bool) -> (u32, u32) {
        let (rw, rh) = if wide { (1366., 1024.) } else { (645., 1398.) };
        let q = ((width / rw).min(height / rh).max(1.) / 0.05).round() * 0.05;
        let down = 0.07 / q;
        (
            (width * down).round().clamp(24., 480.) as u32,
            (height * down).round().clamp(24., 480.) as u32,
        )
    }
    pub(crate) fn render_frame(&self, p: Parameters, width: u32, height: u32) -> RgbaImage {
        self.render_transition(p, width, height, None)
    }
    pub(crate) fn render_transition(
        &self,
        p: Parameters,
        width: u32,
        height: u32,
        previous: Option<(&Self, f32)>,
    ) -> RgbaImage {
        if width == 0 || height == 0 {
            return RgbaImage::new(width, height);
        }
        let [low, mid, high] = p.spectrum.map(|x| x.clamp(0., 1.));
        let has = if p.has_lyrics { 1. } else { 0. };
        let breathing = 1. + 0.33 * (0.9 * low + 0.1 * mid).powi(2);
        let zoom = breathing * (1. + 0.25 * (1. - has));
        let contrast = 1. + 0.076 * low;
        let vertical = p.width / p.height.max(1.) < 1.3 || p.width < 1024.;
        let saturation = if vertical { 1.8 } else { 2.4 } + 0.166 * high + 0.5 * (1. - has);
        let black_scrim = previous.map_or(self.black_scrim, |(old, mix)| {
            old.black_scrim * (1. - mix) + self.black_scrim * mix
        });
        let a = [
            p.time * TAU / 120.,
            p.time * TAU / 70.,
            p.time * TAU / 90. + PI,
        ];
        let centers = [
            [0., 0.],
            [0.1 + (-a[1] * 0.5).cos(), (-a[1] * 0.5).sin()],
            [-0.25, 0.15],
        ];
        let sizes = [2.8, 1.4, 1.4];
        let rotations = a.map(|a| a.sin_cos());
        let aspect = if p.width >= p.height {
            [1., p.height / p.width.max(1.)]
        } else {
            [p.width / p.height.max(1.), 1.]
        };
        let field = RgbaImage::from_fn(width, height, |x, y| {
            let uv = [
                ((x as f32 + 0.5) / width as f32 * 2. - 1.) * aspect[0] / zoom,
                ((y as f32 + 0.5) / height as f32 * 2. - 1.) * aspect[1] / zoom,
            ];
            let mut color = [0.; 3];
            for i in [0, 2, 1] {
                let pos = [uv[0] - centers[i][0], uv[1] - centers[i][1]];
                let (sin, cos) = rotations[i];
                let r = [pos[0] * cos - pos[1] * sin, pos[0] * sin + pos[1] * cos];
                if r[0].abs() > sizes[i] * 0.5 || r[1].abs() > sizes[i] * 0.5 {
                    continue;
                }
                let tex = [r[0] / sizes[i] - 0.5, r[1] / sizes[i] - 0.5];
                let mut sampled = self.sample(tex);
                if let Some((old, mix)) = previous {
                    let old = old.sample(tex);
                    for i in 0..3 {
                        sampled[i] = old[i] * (1. - mix) + sampled[i] * mix;
                    }
                }
                let shaded = sampled
                    .map(|c| (c * (1. - black_scrim - i as f32 * 0.0076) - 0.5) * contrast + 0.5);
                let luma = shaded[0] * 0.2126 + shaded[1] * 0.7152 + shaded[2] * 0.0722;
                color = shaded.map(|c| luma + (c - luma) * saturation);
            }
            let c = color.map(|c| ((c * 0.905 + 0.095).clamp(0., 1.) * 255.).round() as u8);
            Rgba([c[0], c[1], c[2], 255])
        });
        imageops::blur(&field, 0.14 * p.blur)
    }
    fn sample(&self, uv: [f32; 2]) -> [f32; 3] {
        // The native image-shader call passes tile mode 2 (mirror).
        let mirror = |v: f32| {
            let v = v.rem_euclid(2.);
            if v > 1. { 2. - v } else { v }
        };
        let [x, y] = uv.map(|v| (mirror(v) * 128. - 0.5).clamp(0., 127.));
        let ix = x.floor() as u32;
        let iy = y.floor() as u32;
        let fx = x - ix as f32;
        let fy = y - iy as f32;
        let mut out = [0.; 3];
        for (px, py, w) in [
            (ix, iy, (1. - fx) * (1. - fy)),
            ((ix + 1).min(127), iy, fx * (1. - fy)),
            (ix, (iy + 1).min(127), (1. - fx) * fy),
            ((ix + 1).min(127), (iy + 1).min(127), fx * fy),
        ] {
            let c = self.texture.get_pixel(px, py);
            for i in 0..3 {
                out[i] += c[i] as f32 / 255. * c[3] as f32 / 255. * w;
            }
        }
        out
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn artwork_and_pcm_modulation_both_change_the_background() {
        let cover =
            RgbaImage::from_fn(32, 32, |x, y| Rgba([(x * 8) as u8, (y * 8) as u8, 60, 255]));
        let b = Backdrop::from_cover(&cover);
        let p = Parameters {
            time: 5.,
            width: 1376.,
            height: 1030.,
            spectrum: [0.; 3],
            has_lyrics: true,
            blur: 60.,
        };
        let quiet = b.render_frame(p, 72, 54);
        let loud = b.render_frame(
            Parameters {
                spectrum: [1.; 3],
                ..p
            },
            72,
            54,
        );
        let later = b.render_frame(Parameters { time: 25., ..p }, 72, 54);
        assert_ne!(quiet, loud);
        assert_ne!(quiet, later);
        assert!(quiet.pixels().all(|p| p[3] == 255));
        let gray = Backdrop::from_cover(&RgbaImage::from_pixel(8, 8, Rgba([110, 110, 110, 255])))
            .render(0., 24, 14);
        assert!(gray.pixels().all(|p| p[0] == p[1] && p[1] == p[2]));
    }
}

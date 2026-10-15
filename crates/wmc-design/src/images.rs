//! Convert prepared pixels to GPUI images. Backgrounds need an atlas gutter:
//! GPUI's DirectX atlas allocates unpadded tiles and samples their edges linearly.
use gpui_kit::{prelude::*, *};
use image::{Frame, RgbaImage};
use std::sync::Arc;

pub const BACKGROUND_GUTTER: u32 = 2;

pub fn background_image(image: RgbaImage) -> Arc<RenderImage> {
    render_image(pad_background(&image))
}

pub fn guarded_image(image: RgbaImage) -> Arc<RenderImage> {
    background_image(image)
}

pub fn guarded_image_element(image: Arc<RenderImage>) -> Div {
    div().child(
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                paint_guarded_image(window, bounds, image.clone());
            },
        )
        .size_full(),
    )
}

pub fn paint_guarded_image(window: &mut Window, bounds: Bounds<Pixels>, image: Arc<RenderImage>) {
    let texture = image.size(0);
    let padding = BACKGROUND_GUTTER as f32;
    let core_width = (texture.width.0 as f32 - padding * 2.).max(1.);
    let core_height = (texture.height.0 as f32 - padding * 2.).max(1.);
    let dx = bounds.size.width * (padding / core_width);
    let dy = bounds.size.height * (padding / core_height);
    let image_bounds = Bounds::new(
        bounds.origin - point(dx, dy),
        size(bounds.size.width + dx * 2., bounds.size.height + dy * 2.),
    );
    let _ = window.paint_image(
        bounds,
        image_bounds,
        Corners::default(),
        image.clone(),
        0,
        false,
    );
}

fn pad_background(image: &RgbaImage) -> RgbaImage {
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return image.clone();
    }
    let padding = BACKGROUND_GUTTER;
    RgbaImage::from_fn(width + padding * 2, height + padding * 2, |x, y| {
        *image.get_pixel(
            x.saturating_sub(padding).min(width - 1),
            y.saturating_sub(padding).min(height - 1),
        )
    })
}

pub fn render_image(mut image: RgbaImage) -> Arc<RenderImage> {
    // GPUI's RenderImage accepts BGRA pixels in image::Frame storage.
    for pixel in image.pixels_mut() {
        pixel.0.swap(0, 2);
    }
    Arc::new(RenderImage::new(vec![Frame::new(image)]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    // Simulate the DirectX atlas sampler, including unrelated neighboring tiles.
    fn linear_sample(image: &RgbaImage, x: f32, y: f32) -> [f32; 4] {
        let ix = x.floor() as u32;
        let iy = y.floor() as u32;
        let fx = x - ix as f32;
        let fy = y - iy as f32;
        let mut color = [0.; 4];
        for (x, y, weight) in [
            (ix, iy, (1. - fx) * (1. - fy)),
            (ix + 1, iy, fx * (1. - fy)),
            (ix, iy + 1, (1. - fx) * fy),
            (ix + 1, iy + 1, fx * fy),
        ] {
            for (result, channel) in color.iter_mut().zip(image.get_pixel(x, y).0) {
                *result += f32::from(channel) * weight;
            }
        }
        color
    }

    #[::core::prelude::v1::test]
    fn enlarged_background_edges_never_sample_recycled_atlas_neighbors() {
        let source = RgbaImage::from_fn(3, 2, |x, y| {
            Rgba([(30 + x * 70) as u8, (60 + y * 80) as u8, 20, 255])
        });
        let padded = pad_background(&source);
        let mut atlas = RgbaImage::from_pixel(16, 12, Rgba([128, 128, 128, 255]));
        image::imageops::replace(&mut atlas, &padded, 4, 3);
        let origin = [4. + BACKGROUND_GUTTER as f32, 3. + BACKGROUND_GUTTER as f32];
        // The first/last fragments of an enlarged tile fall half a texel outside
        // the source texel centers, at each edge and all four corners.
        for y in [-0.49_f32, 0., 0.5, 1., 1.49] {
            for x in [-0.49_f32, 0., 0.5, 1., 2., 2.49] {
                let expected = linear_sample(
                    &padded,
                    x.clamp(0., 2.) + BACKGROUND_GUTTER as f32,
                    y.clamp(0., 1.) + BACKGROUND_GUTTER as f32,
                );
                let actual = linear_sample(&atlas, origin[0] + x, origin[1] + y);
                for (actual, expected) in actual.into_iter().zip(expected) {
                    assert!((actual - expected).abs() < 0.001);
                }
            }
        }
        // Without the gutter this exact corner blends with the gray atlas tile.
        image::imageops::replace(&mut atlas, &source, 4, 3);
        assert!(linear_sample(&atlas, 3.51, 2.51)[0] > 50.);
    }

    #[::core::prelude::v1::test]
    fn transparent_lyric_tiles_do_not_pick_up_a_white_rectangular_outline() {
        let mut lyric = RgbaImage::new(8, 8);
        lyric.put_pixel(4, 4, Rgba([255, 255, 255, 255]));
        let padded = pad_background(&lyric);
        let mut atlas = RgbaImage::from_pixel(24, 24, Rgba([255, 255, 255, 255]));
        image::imageops::replace(&mut atlas, &padded, 4, 4);
        for position in [-0.49, 0., 3., 7., 7.49] {
            for (x, y) in [
                (position, -0.49),
                (position, 7.49),
                (-0.49, position),
                (7.49, position),
            ] {
                assert_eq!(linear_sample(&atlas, 6. + x, 6. + y)[3], 0.);
            }
        }
    }
}

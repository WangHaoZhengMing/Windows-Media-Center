use image::{DynamicImage, ImageDecoder, imageops::FilterType};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SizeKind {
    Square,
    DetailSquare,
    Fit(u32, u32),
}

pub fn decode(
    path: &Path,
    size: SizeKind,
    cancelled: &AtomicBool,
) -> Result<image::RgbaImage, String> {
    let run = || -> anyhow::Result<image::RgbaImage> {
        anyhow::ensure!(!cancelled.load(Ordering::Relaxed), "cancelled");
        let mut reader = image::ImageReader::open(path)?.with_guessed_format()?;
        let mut limits = image::Limits::default();
        limits.max_alloc = Some(256 * 1024 * 1024);
        limits.max_image_width = Some(32768);
        limits.max_image_height = Some(32768);
        reader.limits(limits);
        let mut decoder = reader.into_decoder()?;
        let (w, h) = decoder.dimensions();
        anyhow::ensure!(
            u64::from(w) * u64::from(h) <= 32_000_000,
            "照片超过 3200 万像素解码上限"
        );
        let orientation = decoder.orientation()?;
        let mut image = DynamicImage::from_decoder(decoder)?;
        image.apply_orientation(orientation);
        anyhow::ensure!(!cancelled.load(Ordering::Relaxed), "cancelled");
        let pixels = resize(&image, size).to_rgba8();
        anyhow::ensure!(!cancelled.load(Ordering::Relaxed), "cancelled");
        Ok(pixels)
    };
    run().map_err(|error| error.to_string())
}

fn resize(image: &DynamicImage, size: SizeKind) -> DynamicImage {
    match size {
        // Square grid / aspect-fill: equal scale on both axes, crop around center.
        SizeKind::Square => image.resize_to_fill(256, 256, FilterType::Triangle),
        SizeKind::DetailSquare => image.resize_to_fill(512, 512, FilterType::Triangle),
        SizeKind::Fit(w, h) => image.thumbnail(w, h),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const THUMB: SizeKind = SizeKind::Square;
    const DETAIL: SizeKind = SizeKind::DetailSquare;
    #[test]
    fn square_crop_keeps_center_and_does_not_stretch() {
        let input = DynamicImage::ImageRgba8(image::RgbaImage::from_fn(768, 256, |x, _| {
            if (256..512).contains(&x) {
                image::Rgba([0, 255, 0, 255])
            } else {
                image::Rgba([255, 0, 0, 255])
            }
        }));
        let square = resize(&input, THUMB).to_rgba8();
        assert_eq!(square.dimensions(), (256, 256));
        assert_eq!(square.get_pixel(128, 128).0, [0, 255, 0, 255]);
        assert_eq!(resize(&input, DETAIL).width(), 512);
        assert_eq!(resize(&input, DETAIL).height(), 512);
        assert!(resize(&input, SizeKind::Fit(512, 512)).height() < 512);
        let portrait = input.rotate90();
        let square = resize(&portrait, THUMB).to_rgba8();
        assert_eq!(square.dimensions(), (256, 256));
        assert_eq!(square.get_pixel(128, 128).0, [0, 255, 0, 255]);
    }
}

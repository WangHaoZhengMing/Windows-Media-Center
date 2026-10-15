//! Render cover-driven background previews without a window or audio device.
use anyhow::{Context, Result, ensure};
use image::{Rgba, RgbaImage, imageops};
use std::{f32::consts::TAU, path::PathBuf, time::Instant};
use wmc_player::background::Backdrop;

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let output = PathBuf::from(args.next().context("output PNG path required")?);
    let mut cover = if let Some(path) = args.next() {
        image::open(path)?.to_rgba8()
    } else {
        let bytes = wmc_assets::load(wmc_assets::Shortcut::MusicFavorites.images().focused)
            .context("default album art unavailable")?;
        let art = image::load_from_memory(&bytes)?.to_rgba8();
        imageops::crop_imm(&art, 48, 22, 82, 91).to_image()
    };
    // Optional x,y,width,height isolates album art in a supplied reference screenshot.
    if let Some(region) = args.next() {
        let region = region.to_string_lossy();
        let values: Vec<u32> = region
            .split(',')
            .map(str::parse)
            .collect::<Result<_, _>>()?;
        ensure!(values.len() == 4, "crop needs x,y,width,height");
        let [x, y, w, h] = [values[0], values[1], values[2], values[3]];
        ensure!(
            w > 0
                && h > 0
                && u64::from(x) + u64::from(w) <= u64::from(cover.width())
                && u64::from(y) + u64::from(h) <= u64::from(cover.height()),
            "crop outside image"
        );
        cover = imageops::crop_imm(&cover, x, y, w, h).to_image();
    }
    let started = Instant::now();
    let backdrop = Backdrop::from_cover(&cover);
    let mut sheet = RgbaImage::from_pixel(768, 144, Rgba([0, 0, 0, 255]));
    let thumb = imageops::thumbnail(&cover, 82, 100);
    for (i, phase) in [0., TAU / 3., TAU * 2. / 3.].into_iter().enumerate() {
        let frame = backdrop.render(phase, 256, 144);
        imageops::overlay(&mut sheet, &frame, i as i64 * 256, 0);
        imageops::overlay(&mut sheet, &thumb, i as i64 * 256 + 18, 22);
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    sheet.save(&output)?;
    println!(
        "{}; palette + 3 frames: {:.2} ms",
        output.display(),
        started.elapsed().as_secs_f64() * 1000.
    );
    Ok(())
}

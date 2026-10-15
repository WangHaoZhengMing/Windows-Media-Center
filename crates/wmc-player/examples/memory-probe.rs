//! Staged memory attribution without opening a window or an audio device.
use fontdue::{Font, FontSettings};
use std::{
    io::{self, Write},
    path::PathBuf,
    time::Instant,
};

fn stage(name: &str) -> anyhow::Result<()> {
    println!("stage={name} pid={}", std::process::id());
    io::stdout().flush()?;
    let mut next = String::new();
    io::stdin().read_line(&mut next)?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    stage("baseline")?;
    let path = PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into()))
        .join("Fonts/msyh.ttc");
    let bytes = std::fs::read(path)?;
    println!("font_file_bytes={}", bytes.len());
    stage("font_bytes")?;
    let start = Instant::now();
    let font = Font::from_bytes(
        bytes,
        FontSettings {
            collection_index: 1,
            scale: 44.,
            ..Default::default()
        },
    )
    .map_err(|error| anyhow::anyhow!(error))?;
    println!(
        "glyph_count={} parse_ms={:.2}",
        font.glyph_count(),
        start.elapsed().as_secs_f64() * 1000.
    );
    stage("font_parsed")?;
    let cover = image::open(
        std::env::args_os()
            .nth(1)
            .ok_or_else(|| anyhow::anyhow!("请传入封面路径"))?,
    )?
    .to_rgba8();
    stage("cover_decoded")?;
    let backdrop = wmc_player::background::Backdrop::from_cover(&cover);
    let backgrounds: Vec<_> = (0..24)
        .map(|index| backdrop.render(index as f32 / 24. * std::f32::consts::TAU, 256, 144))
        .collect();
    println!(
        "background_pixel_bytes={}",
        backgrounds
            .iter()
            .map(|image| image.as_raw().len())
            .sum::<usize>()
    );
    stage("backgrounds")?;
    drop(backgrounds);
    drop(cover);
    drop(font);
    stage("dropped")?;
    Ok(())
}

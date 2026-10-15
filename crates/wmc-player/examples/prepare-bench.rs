use std::path::PathBuf;
use wmc_player::PlayerConfig;

fn main() -> anyhow::Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("请传入音频文件路径"))?;
    let (_, time) = PlayerConfig {
        audio: Some(PathBuf::from(path)),
        ..Default::default()
    }
    .prepare_with_timings()?;
    println!(
        "metadata_ms={:.2} audio_ms={:.2} lyrics_ms={:.2} cover_ms={:.2} background_ms={:.2} total_ms={:.2}",
        time.metadata.as_secs_f64() * 1000.,
        time.audio.as_secs_f64() * 1000.,
        time.lyrics.as_secs_f64() * 1000.,
        time.cover.as_secs_f64() * 1000.,
        time.background.as_secs_f64() * 1000.,
        time.total.as_secs_f64() * 1000.
    );
    Ok(())
}

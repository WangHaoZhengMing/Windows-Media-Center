//! Local-file integration demo; audio and converted lyrics stay outside Git.
use std::path::PathBuf;
use wmc_player::{PlayerConfig, run_demo_with_config};

fn main() -> anyhow::Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config = PlayerConfig {
        audio: Some(PathBuf::from(r"C:\Users\15049\OneDrive\Desktop\英雄泪.mp3")),
        lyrics: Some(root.join("target/local-media/英雄泪.lrc")),
        cover: Some(PathBuf::from(
            r"C:\Users\15049\OneDrive\Pictures\cefc1e178a82b901d744aaf8798da9773812ef14.webp",
        )),
        title: None,
        artist: None,
    };
    if std::env::args().any(|arg| arg == "--check") {
        use rodio::Source;
        let metadata = wmc_player::metadata::TrackMetadata::read(config.audio.as_ref().unwrap())?;
        println!(
            "Metadata: title={:?}, artist={:?}, album={:?}, embedded cover={}, embedded lyrics={}",
            metadata.title,
            metadata.artist,
            metadata.album,
            metadata.cover.is_some(),
            metadata.lyrics.is_some()
        );
        let decoder =
            rodio::Decoder::try_from(std::fs::File::open(config.audio.as_ref().unwrap())?)?;
        let channels = decoder.channels();
        let rate = decoder.sample_rate();
        let duration = decoder.count() as f64 / f64::from(channels) / f64::from(rate);
        let source = std::fs::read_to_string(config.lyrics.as_ref().unwrap())?;
        let lyrics = wmc_player::lyrics::Lyrics::parse_lrc(&source, duration)?;
        let words: usize = lyrics
            .lines
            .iter()
            .filter(|line| line.word_synced)
            .map(|line| line.words.len())
            .sum();
        println!(
            "Audio: {duration:.3}s; lyrics: {} lines, {words} timed words",
            lyrics.lines.len()
        );
        if let Some(path) = &config.cover {
            let cover = image::open(path)?;
            println!("Cover: {} × {}", cover.width(), cover.height());
        }
    } else {
        run_demo_with_config(config);
    }
    Ok(())
}

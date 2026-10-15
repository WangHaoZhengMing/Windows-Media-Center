//! Chapter Seven TTML preview; supply --audio to play the matching recording.
use anyhow::{Context, Result, bail};
use std::{io::Write, path::PathBuf};
use wmc_player::{PlayerConfig, lyrics::Lyrics, run_demo_with_config};

// The supplied TTML declares body dur="3:48.640".
const DURATION: f64 = 228.640;

fn silent_preview() -> Result<tempfile::NamedTempFile> {
    let mut file = tempfile::Builder::new().suffix(".wav").tempfile()?;
    let rate = 8_000_u32;
    let bytes = (DURATION * f64::from(rate)).ceil() as u32 * 2;
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + bytes).to_le_bytes())?;
    file.write_all(b"WAVEfmt ")?;
    file.write_all(&16_u32.to_le_bytes())?;
    file.write_all(&1_u16.to_le_bytes())?;
    file.write_all(&1_u16.to_le_bytes())?;
    file.write_all(&rate.to_le_bytes())?;
    file.write_all(&(rate * 2).to_le_bytes())?;
    file.write_all(&2_u16.to_le_bytes())?;
    file.write_all(&16_u16.to_le_bytes())?;
    file.write_all(b"data")?;
    file.write_all(&bytes.to_le_bytes())?;
    let silence = [0_u8; 8192];
    let mut remaining = bytes as usize;
    while remaining > 0 {
        let count = remaining.min(silence.len());
        file.write_all(&silence[..count])?;
        remaining -= count;
    }
    file.flush()?;
    Ok(file)
}

fn main() -> Result<()> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets/lyrics/Jay Chou - Chapter Seven - Still Fantasy.ttml");
    let mut audio = None;
    let mut check = false;
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--check") => check = true,
            Some("--audio") => {
                audio = Some(PathBuf::from(
                    args.next().context("--audio 后需要歌曲路径")?,
                ))
            }
            _ => bail!("支持的参数：--check、--audio <音频文件>"),
        }
    }
    let source = std::fs::read_to_string(&path)?;
    let lyrics = Lyrics::parse_text(&source, DURATION)?;
    if check {
        println!(
            "Lyrics: {}; duration: {DURATION:.3}s; {} rows",
            path.display(),
            lyrics.lines.len()
        );
        let overlaps: Vec<_> = lyrics
            .lines
            .iter()
            .flat_map(|line| [line.vocal_start(), line.vocal_end()])
            .filter(|&seconds| {
                lyrics
                    .selected_lines(seconds, 2, 0.)
                    .iter()
                    .filter(|&&index| !lyrics.lines[index].background)
                    .count()
                    >= 2
            })
            .collect();
        println!("Overlapping vocal boundaries: {}", overlaps.len());
        println!(
            "First vocal: {:.3}s; last vocal: {:.3}s",
            lyrics.lines.first().map_or(0., |line| line.vocal_start()),
            lyrics
                .lines
                .iter()
                .map(|line| line.vocal_end())
                .fold(0., f64::max)
        );
        return Ok(());
    }
    // Keep the temporary file alive for decoding and subsequent seeks.
    let preview = if audio.is_none() {
        Some(silent_preview()?)
    } else {
        None
    };
    let silent = preview.is_some();
    let audio = audio.or_else(|| preview.as_ref().map(|file| file.path().to_owned()));
    run_demo_with_config(PlayerConfig {
        audio,
        lyrics: Some(path),
        title: Some("夜的第七章".into()),
        artist: Some(
            if silent {
                "Jay Chou · 无声歌词预览"
            } else {
                "Jay Chou"
            }
            .into(),
        ),
        ..Default::default()
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodio::Source;

    #[test]
    fn silent_preview_is_decodable_and_covers_the_whole_lyric_timeline() {
        let file = silent_preview().unwrap();
        let decoder = rodio::Decoder::try_from(std::fs::File::open(file.path()).unwrap()).unwrap();
        assert_eq!(decoder.channels(), 1);
        assert_eq!(decoder.sample_rate(), 8_000);
        let duration = decoder.count() as f64 / 8_000.;
        assert!((duration - DURATION).abs() < 0.001);
    }
}

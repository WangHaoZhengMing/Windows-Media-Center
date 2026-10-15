//! Lightweight audio properties for file details; no decoding or audio device.
use anyhow::Result;
use lofty::{config::ParseOptions, file::AudioFile, probe::Probe};
use std::{path::Path, time::Duration};

#[derive(Debug)]
pub struct AudioProperties {
    pub duration: Duration,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
    pub bit_depth: Option<u8>,
}

impl AudioProperties {
    pub fn read(path: &Path) -> Result<Self> {
        let file = Probe::open(path)?
            .guess_file_type()?
            .options(
                ParseOptions::new()
                    .read_tags(false)
                    .read_properties(true)
                    .read_cover_art(false),
            )
            .read()?;
        let properties = file.properties();
        Ok(Self {
            duration: properties.duration(),
            bitrate: properties.audio_bitrate().or(properties.overall_bitrate()),
            sample_rate: properties.sample_rate(),
            channels: properties.channels(),
            bit_depth: properties.bit_depth(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_properties_are_read_without_opening_a_player() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("properties.wav");
        let mut wav = b"RIFF".to_vec();
        wav.extend(1636_u32.to_le_bytes());
        wav.extend(b"WAVEfmt ");
        wav.extend(16_u32.to_le_bytes());
        wav.extend(1_u16.to_le_bytes());
        wav.extend(1_u16.to_le_bytes());
        wav.extend(8000_u32.to_le_bytes());
        wav.extend(16000_u32.to_le_bytes());
        wav.extend(2_u16.to_le_bytes());
        wav.extend(16_u16.to_le_bytes());
        wav.extend(b"data");
        wav.extend(1600_u32.to_le_bytes());
        wav.extend([0; 1600]);
        std::fs::write(&path, &wav).unwrap();
        let result = AudioProperties::read(&path).unwrap();
        assert_eq!(result.sample_rate, Some(8000));
        assert_eq!(result.channels, Some(1));
        assert_eq!(result.bit_depth, Some(16));
        assert_eq!(result.duration, Duration::from_millis(100));
        assert_eq!(std::fs::read(path).unwrap(), wav);
    }
}

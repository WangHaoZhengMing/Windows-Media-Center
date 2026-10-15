//! File decoding and in-memory audio sources, independent from output devices.
use anyhow::{Context, Result};
use rodio::{Decoder, Source, buffer::SamplesBuffer};
use std::{
    f32::consts::TAU,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

pub(crate) struct AudioData {
    pub(super) source: AudioSource,
}

pub(super) enum AudioSource {
    Memory {
        samples: Vec<f32>,
        channels: u16,
        rate: u32,
    },
    File {
        path: PathBuf,
        duration: f64,
        decoder: Option<Decoder<BufReader<File>>>,
    },
}

impl AudioData {
    pub fn demo() -> Self {
        let rate = 16_000;
        let chords = [
            [164.81, 196., 246.94],
            [130.81, 164.81, 196.],
            [196., 246.94, 293.66],
            [146.83, 185., 220.],
        ];
        let melody = [329.63, 392., 493.88, 392., 293.66, 329.63, 392., 293.66];
        let samples = (0..48 * rate)
            .map(|sample| {
                let t = sample as f32 / rate as f32;
                let chord = chords[(t / 4.) as usize % chords.len()];
                let beat = t % 0.5;
                let envelope = (beat * 35.).min(1.) * (-beat * 5.).exp();
                let note = melody[(t * 2.) as usize % melody.len()];
                let lead =
                    ((TAU * note * t).sin() + 0.25 * (TAU * note * 2. * t).sin()) * envelope * 0.09;
                let pad = chord.iter().map(|f| (TAU * f * t).sin()).sum::<f32>() * 0.035;
                let fade = (t / 0.4).min(1.) * ((48. - t) / 1.5).min(1.);
                (pad + lead) * fade
            })
            .collect();
        Self {
            source: AudioSource::Memory {
                samples,
                channels: 1,
                rate,
            },
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let decoder = Decoder::try_from(File::open(path)?).context("无法解码音频")?;
        let duration = match decoder.total_duration() {
            Some(duration) if !duration.is_zero() => duration.as_secs_f64(),
            _ => {
                use lofty::{config::ParseOptions, file::AudioFile, probe::Probe};
                Probe::open(path)?
                    .guess_file_type()?
                    .options(ParseOptions::new().read_tags(false).read_cover_art(false))
                    .read()?
                    .properties()
                    .duration()
                    .as_secs_f64()
            }
        };
        anyhow::ensure!(
            duration.is_finite() && duration > 0.,
            "无法获取有效音频时长"
        );
        Ok(Self {
            source: AudioSource::File {
                path: path.to_owned(),
                duration,
                decoder: Some(decoder),
            },
        })
    }

    pub fn duration(&self) -> f64 {
        match &self.source {
            AudioSource::Memory {
                samples,
                channels,
                rate,
            } => samples.len() as f64 / f64::from(*channels) / f64::from(*rate),
            AudioSource::File { duration, .. } => *duration,
        }
    }

    pub(super) fn source(&mut self) -> Result<Box<dyn Source<Item = f32> + Send>> {
        match &mut self.source {
            AudioSource::Memory {
                samples,
                channels,
                rate,
            } => Ok(Box::new(SamplesBuffer::new(
                *channels,
                *rate,
                samples.clone(),
            ))),
            AudioSource::File { path, decoder, .. } => {
                let source = match decoder.take() {
                    Some(source) => source,
                    None => Decoder::try_from(File::open(path)?).context("无法重新打开音频")?,
                };
                Ok(Box::new(source))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[test]
    fn local_audio_is_streamed_and_can_seek_and_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("stream.wav");
        let mut bytes = b"RIFF".to_vec();
        bytes.extend(16036_u32.to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16_u32.to_le_bytes());
        bytes.extend(1_u16.to_le_bytes());
        bytes.extend(1_u16.to_le_bytes());
        bytes.extend(8000_u32.to_le_bytes());
        bytes.extend(16000_u32.to_le_bytes());
        bytes.extend(2_u16.to_le_bytes());
        bytes.extend(16_u16.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend(16000_u32.to_le_bytes());
        for value in 0..8000_i16 {
            bytes.extend(value.to_le_bytes());
        }
        std::fs::write(&path, bytes).unwrap();
        let mut audio = AudioData::load(&path).unwrap();
        assert!(matches!(
            &audio.source,
            AudioSource::File {
                decoder: Some(_),
                ..
            }
        ));
        assert!((audio.duration() - 1.).abs() < 0.001);
        let mut source = audio.source().unwrap();
        let first = source.next().unwrap();
        source.try_seek(Duration::from_millis(500)).unwrap();
        assert!(source.next().unwrap() > first);
        drop(source);
        assert_eq!(audio.source().unwrap().next().unwrap(), first);
        std::fs::remove_file(path).unwrap();
        assert!(audio.source().is_err());
    }
}

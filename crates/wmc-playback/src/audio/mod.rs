//! Output-device lifecycle and playback controls.
pub mod clock;
mod source;
mod spectrum;
use anyhow::Result;
use rodio::{OutputStream, OutputStreamBuilder, Sink};
pub use source::AudioData;
use std::{
    cell::RefCell,
    sync::Arc,
    time::{Duration, Instant},
};

pub struct AudioPlayer {
    data: AudioData,
    stream: Option<OutputStream>,
    sink: Option<Sink>,
    pub error: Option<String>,
    muted: bool,
    fallback_position: f64,
    fallback_started: Option<Instant>,
    clock: RefCell<clock::PlaybackClock>,
    clock_epoch: Instant,
    stopped: bool,
    spectrum: Arc<spectrum::Spectrum>,
}

/// A single clock sample shared by all visual consumers of a frame.
pub struct PlaybackSnapshot {
    pub seconds: f64,
    pub duration: f64,
    pub playing: bool,
    pub stopped: bool,
}

impl AudioPlayer {
    pub fn is_finished(&self) -> bool {
        !self.stopped && !self.playing() && self.raw_position() >= self.duration()
    }

    pub fn snapshot(&self) -> PlaybackSnapshot {
        let seconds = self.position();
        let duration = self.duration();
        let playing = self.sink.as_ref().map_or(
            self.fallback_started.is_some() && seconds < duration,
            |sink| !sink.is_paused() && !sink.empty(),
        );
        PlaybackSnapshot {
            seconds,
            duration,
            playing,
            stopped: self.stopped || seconds >= duration,
        }
    }

    pub fn new(data: AudioData) -> Self {
        let mut player = Self::without_output(data);
        match OutputStreamBuilder::open_default_stream() {
            Ok(mut stream) => {
                stream.log_on_drop(false);
                let sink = Sink::connect_new(stream.mixer());
                sink.pause();
                sink.set_volume(1.);
                match player.data.source() {
                    Ok(source) => {
                        sink.append(spectrum::Tap::new(source, player.spectrum.clone()));
                        player.stream = Some(stream);
                        player.sink = Some(sink);
                    }
                    Err(error) => player.error = Some(error.to_string()),
                }
            }
            Err(error) => player.error = Some(error.to_string()),
        }
        player
    }

    fn without_output(data: AudioData) -> Self {
        Self {
            data,
            stream: None,
            sink: None,
            error: None,
            muted: false,
            fallback_position: 0.,
            fallback_started: None,
            clock: RefCell::new(clock::PlaybackClock::default()),
            clock_epoch: Instant::now(),
            stopped: true,
            spectrum: Arc::new(spectrum::Spectrum::default()),
        }
    }
    #[cfg(any(test, feature = "test-support"))]
    pub fn preview(data: AudioData) -> Self {
        let mut player = Self::without_output(data);
        player.error = Some("Headless audio preview".into());
        player
    }

    pub fn spectrum(&self) -> [f32; 3] {
        self.spectrum.read()
    }

    pub fn suspend_tag_write(&mut self) -> PlaybackSnapshot {
        let snapshot = self.snapshot();
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        if let source::AudioSource::File { decoder, .. } = &mut self.data.source {
            *decoder = None;
        }
        self.fallback_position = snapshot.seconds;
        self.fallback_started = None;
        self.clock.borrow_mut().reset(
            snapshot.seconds,
            self.clock_epoch.elapsed().as_secs_f64(),
            false,
        );
        snapshot
    }

    pub fn resume_tag_write(&mut self, snapshot: PlaybackSnapshot) -> Result<()> {
        self.reload()?;
        self.seek(snapshot.seconds)?;
        if snapshot.playing {
            self.play();
        }
        self.stopped = snapshot.stopped;
        Ok(())
    }

    pub fn duration(&self) -> f64 {
        self.data.duration()
    }
    pub fn position(&self) -> f64 {
        let raw = self.raw_position();
        let playing = self.sink.as_ref().map_or(
            self.fallback_started.is_some() && raw < self.duration(),
            |sink| !sink.is_paused() && !sink.empty(),
        );
        self.clock.borrow_mut().sample(
            raw,
            self.clock_epoch.elapsed().as_secs_f64(),
            playing,
            self.duration(),
        )
    }

    pub fn raw_position(&self) -> f64 {
        if let Some(sink) = &self.sink {
            if sink.empty() {
                self.duration()
            } else {
                sink.get_pos().as_secs_f64().min(self.duration())
            }
        } else {
            (self.fallback_position
                + self
                    .fallback_started
                    .map_or(0., |time| time.elapsed().as_secs_f64()))
            .min(self.duration())
        }
    }
    pub fn playing(&self) -> bool {
        self.sink.as_ref().map_or(
            self.fallback_started.is_some() && self.position() < self.duration(),
            |sink| !sink.is_paused() && !sink.empty(),
        )
    }
    pub fn toggle(&mut self) {
        let mut position = self.position();
        if position >= self.duration() - 0.01 {
            if let Err(error) = self.reload() {
                self.error = Some(error.to_string());
                return;
            }
            position = 0.;
        }
        if let Some(sink) = &self.sink {
            if sink.is_paused() {
                sink.play();
            } else {
                sink.pause();
            }
        } else if let Some(start) = self.fallback_started.take() {
            self.fallback_position =
                (self.fallback_position + start.elapsed().as_secs_f64()).min(self.duration());
        } else {
            self.fallback_started = Some(Instant::now());
        }
        let playing = self.playing();
        self.stopped = false;
        self.clock
            .borrow_mut()
            .reset(position, self.clock_epoch.elapsed().as_secs_f64(), playing);
    }
    pub fn pause(&mut self) {
        if self.playing() {
            self.toggle();
        }
    }
    pub fn play(&mut self) {
        if !self.playing() {
            self.toggle();
        }
    }
    fn reload(&mut self) -> Result<()> {
        if let Some(stream) = &self.stream {
            let sink = Sink::connect_new(stream.mixer());
            sink.pause();
            sink.set_volume(if self.muted { 0. } else { 1. });
            sink.append(spectrum::Tap::new(
                self.data.source()?,
                self.spectrum.clone(),
            ));
            self.sink = Some(sink);
        }
        self.fallback_position = 0.;
        self.fallback_started = None;
        self.clock
            .borrow_mut()
            .reset(0., self.clock_epoch.elapsed().as_secs_f64(), false);
        Ok(())
    }
    pub fn seek(&mut self, seconds: f64) -> Result<()> {
        let target = seconds.clamp(0., (self.duration() - 0.001).max(0.));
        let playing = self.playing();
        if self.sink.as_ref().is_some_and(Sink::empty) {
            self.reload()?;
        }
        if let Some(sink) = &self.sink {
            sink.try_seek(Duration::from_secs_f64(target))
                .map_err(|error| anyhow::anyhow!(error.to_string()))?;
            if playing {
                sink.play();
            }
        } else {
            self.fallback_position = target;
            self.fallback_started = playing.then(Instant::now);
        }
        self.clock
            .borrow_mut()
            .reset(target, self.clock_epoch.elapsed().as_secs_f64(), playing);
        self.stopped = false;
        Ok(())
    }
    pub fn stop(&mut self) {
        self.pause();
        if let Err(error) = self.reload() {
            self.error = Some(error.to_string());
        }
        self.stopped = true;
    }
    // Diagnostic captures stay silent without muting the whole operating system.
    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        self.update_volume();
    }
    fn update_volume(&self) {
        if let Some(sink) = &self.sink {
            sink.set_volume(if self.muted { 0. } else { 1. });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use source::AudioSource;
    fn preview_audio() -> AudioPlayer {
        AudioPlayer {
            data: AudioData {
                source: AudioSource::Memory {
                    samples: vec![0.; 16000],
                    channels: 1,
                    rate: 16000,
                },
            },
            stream: None,
            sink: None,
            error: None,
            muted: false,
            fallback_position: 0.,
            fallback_started: None,
            clock: RefCell::new(clock::PlaybackClock::default()),
            clock_epoch: Instant::now(),
            stopped: true,
            spectrum: Arc::new(spectrum::Spectrum::default()),
        }
    }

    #[test]
    fn natural_completion_is_distinct_from_pause_and_stop() {
        let mut audio = preview_audio();
        assert!(!audio.is_finished());
        audio.play();
        audio.pause();
        assert!(!audio.is_finished());
        audio.fallback_position = audio.duration();
        assert!(audio.is_finished());
        audio.stop();
        assert!(!audio.is_finished());
    }

    #[test]
    fn diagnostic_mute_restores_unity_gain_without_system_volume_changes() {
        let mut audio = preview_audio();
        let (sink, _output) = Sink::new();
        audio.sink = Some(sink);
        audio.toggle_mute();
        assert_eq!(audio.sink.as_ref().unwrap().volume(), 0.);
        audio.toggle_mute();
        assert_eq!(audio.sink.as_ref().unwrap().volume(), 1.);
    }

    #[test]
    fn preview_without_a_device_can_pause_seek_and_resume_without_clock_reborrows() {
        let mut audio = preview_audio();
        audio.play();
        assert!(audio.playing());
        audio.play();
        assert!(audio.playing(), "repeated system Play must not pause");
        audio.pause();
        audio.pause();
        audio.seek(0.5).unwrap();
        assert!(!audio.playing());
        assert_eq!(audio.position(), 0.5);
        let paused = audio.snapshot();
        assert_eq!(paused.seconds, 0.5);
        assert_eq!(paused.duration, 1.);
        assert!(!paused.playing);
        audio.toggle();
        assert!(audio.playing());
        assert!(audio.position() >= 0.5);
        audio.stop();
        assert_eq!(audio.position(), 0.);
        assert!(audio.snapshot().stopped);
        audio.play();
        assert!(audio.playing());
        assert!(!audio.snapshot().stopped);
    }
}

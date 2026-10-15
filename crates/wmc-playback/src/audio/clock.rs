//! Audio-anchored presentation clock: smooth buffer-sized position updates.
//! Wall time fills gaps; bounded phase correction prevents independent drift.
#[derive(Default)]
pub struct PlaybackClock {
    position: f64,
    last_raw: f64,
    last_tick: f64,
    rate: f64,
    playing: bool,
    initialized: bool,
}

impl PlaybackClock {
    pub fn reset(&mut self, position: f64, now: f64, playing: bool) {
        self.position = position;
        self.last_raw = position;
        self.last_tick = now;
        self.rate = 1.;
        self.playing = playing;
        self.initialized = true;
    }

    pub fn sample(&mut self, raw: f64, now: f64, playing: bool, duration: f64) -> f64 {
        if !self.initialized {
            self.reset(raw, now, playing);
        }
        let dt = (now - self.last_tick).max(0.);
        self.last_tick = now;
        if raw >= duration {
            self.reset(duration, now, false);
            return duration;
        }
        if self.playing && playing {
            let predicted = self.position + dt * self.rate;
            let error = raw - predicted;
            // An unexpected external discontinuity is a seek, not buffer jitter.
            if (raw - self.last_raw).abs() > 0.25 || error.abs() > 0.25 {
                self.position = raw;
                self.rate = 1.;
            } else {
                self.position = predicted.min(raw + 0.12);
                self.rate = (1. + error * 2.).clamp(0.9, 1.1);
            }
        }
        // Pausing holds the displayed sample; resuming preserves that anchor.
        self.playing = playing;
        self.last_raw = raw;
        self.position = self.position.clamp(0., duration);
        self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forty_millisecond_audio_batches_advance_smoothly_without_drift() {
        let mut clock = PlaybackClock::default();
        let mut previous = 0.;
        for frame in 0..60 * 120 {
            let now = frame as f64 / 120.;
            let raw = (now / 0.04).floor() * 0.04;
            let displayed = clock.sample(raw, now, true, 100.);
            if frame > 0 {
                let delta = displayed - previous;
                assert!((0.007..0.010).contains(&delta), "frame {frame}: {delta}");
                assert!((displayed - raw).abs() < 0.06);
            }
            previous = displayed;
        }
    }
    #[test]
    fn pause_seek_resume_and_end_do_not_extrapolate_old_audio() {
        let mut clock = PlaybackClock::default();
        clock.reset(3., 1., true);
        let frozen = clock.sample(3., 1.01, false, 20.);
        assert_eq!(clock.sample(3.03, 9., false, 20.), frozen);
        clock.reset(12., 9., false);
        assert_eq!(clock.sample(12., 10., false, 20.), 12.);
        assert_eq!(clock.sample(12., 11., true, 20.), 12.);
        assert!(clock.sample(12., 11.01, true, 20.) > 12.);
        assert_eq!(clock.sample(20., 15., false, 20.), 20.);
    }
    #[test]
    fn a_stalled_backend_cannot_keep_animating_indefinitely() {
        let mut clock = PlaybackClock::default();
        clock.reset(5., 0., true);
        for frame in 1..240 {
            clock.sample(5., frame as f64 / 120., true, 20.);
        }
        assert!(clock.position <= 5.12);
    }
}

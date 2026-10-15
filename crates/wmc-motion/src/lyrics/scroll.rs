//! Per-line transition state. No GPUI or audio device is needed for this model.
use crate::{lyrics::profile as specs, spring::Spring};

pub struct AnimatedLine {
    pub position: Spring,
    pub clarity: Spring,
    pub highlight: f32,
    target: f32,
    delay: f32,
    selected: bool,
    finish_remaining: f32,
}

pub struct LyricsMotion {
    pub lines: Vec<AnimatedLine>,
    anchor: Option<usize>,
    initialized: bool,
    immediate: bool,
}

impl LyricsMotion {
    pub fn new(count: usize) -> Self {
        Self {
            lines: (0..count)
                .map(|_| AnimatedLine {
                    position: Spring::new(0.),
                    clarity: Spring::new(0.),
                    highlight: 0.,
                    target: 0.,
                    delay: 0.,
                    selected: false,
                    finish_remaining: 0.,
                })
                .collect(),
            anchor: None,
            initialized: false,
            immediate: false,
        }
    }

    /// A user seek keeps spring velocity but cancels the automatic ripple delay.
    pub fn seek(&mut self) {
        self.immediate = true;
    }

    pub fn update(
        &mut self,
        targets: &[f32],
        anchor: Option<usize>,
        selected: &[usize],
        dt: f32,
        reduced: bool,
    ) {
        assert_eq!(targets.len(), self.lines.len());
        let dt = dt.max(0.);
        let changed = self.anchor != anchor;
        for (index, (line, target)) in self.lines.iter_mut().zip(targets).enumerate() {
            let focused = selected.contains(&index);
            if line.selected && !focused {
                line.finish_remaining = specs::FINISH_DURATION;
            }
            line.selected = focused;
            line.finish_remaining = (line.finish_remaining - dt).max(0.);
            line.highlight = if focused {
                1.
            } else {
                line.finish_remaining / specs::FINISH_DURATION
            };
            let clarity = if focused { 1. } else { 0. };
            if reduced || !self.initialized {
                line.position = Spring::new(*target);
                line.clarity = Spring::new(clarity);
                line.target = *target;
                line.delay = 0.;
                line.highlight = clarity;
                line.finish_remaining = 0.;
                continue;
            }
            if self.immediate {
                line.delay = 0.;
            } else if changed {
                // Mapping distance to delay is our adaptation of Specs.lineDelay.
                let distance = index
                    .abs_diff(anchor.unwrap_or(0))
                    .min(specs::MAX_STAGGER_ROWS);
                line.delay = distance as f32 * specs::LINE_DELAY;
            }
            let waiting = line.delay.min(dt);
            if waiting > 0. {
                line.position
                    .step_physical(line.target, waiting, specs::LINE_SPRING);
            }
            line.delay = (line.delay - dt).max(0.);
            if line.delay == 0. {
                line.target = *target;
                line.position
                    .step_physical(*target, dt - waiting, specs::LINE_SPRING);
            }
            line.clarity.step(clarity, dt, specs::CLARITY_OMEGA);
        }
        self.anchor = anchor;
        self.initialized = true;
        self.immediate = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nearby_lines_follow_in_a_ripple_and_seeking_cancels_waits() {
        let mut motion = LyricsMotion::new(3);
        motion.update(&[0., 91., 182.], Some(0), &[0], 0., false);
        motion.update(&[-91., 0., 91.], Some(1), &[1], 0.02, false);
        assert_eq!(motion.lines[0].position.position, 0.);
        assert!(motion.lines[1].position.position < 91.);
        let velocity = motion.lines[1].position.velocity;
        motion.seek();
        motion.update(&[0., 91., 182.], Some(0), &[0], 0., false);
        assert_eq!(motion.lines[1].position.velocity, velocity);
        motion.update(&[0., 91., 182.], Some(0), &[0], 0.02, false);
        assert!(motion.lines[0].position.position.abs() < 0.001);
        assert_eq!(motion.lines[2].delay, 0.);
    }
    #[test]
    fn reduced_motion_snaps_without_residual_highlight_or_velocity() {
        let mut motion = LyricsMotion::new(2);
        motion.update(&[0., 91.], Some(0), &[0], 0., false);
        motion.update(&[-91., 0.], Some(1), &[1], 0.1, true);
        assert_eq!(motion.lines[0].position.position, -91.);
        assert_eq!(motion.lines[0].highlight, 0.);
        assert_eq!(motion.lines[1].clarity.position, 1.);
        assert_eq!(motion.lines[1].position.velocity, 0.);
    }

    #[test]
    fn a_delayed_transition_is_consistent_across_frame_rates() {
        fn sample(frames: usize) -> LyricsMotion {
            let mut motion = LyricsMotion::new(3);
            motion.update(&[0., 91., 182.], Some(0), &[0], 0., false);
            for _ in 0..frames {
                motion.update(&[-91., 0., 91.], Some(1), &[1], 1. / frames as f32, false);
            }
            motion
        }
        let coarse = sample(30);
        let fine = sample(144);
        for (a, b) in coarse.lines.iter().zip(fine.lines) {
            assert!((a.position.position - b.position.position).abs() < 0.003);
            assert!((a.position.velocity - b.position.velocity).abs() < 0.003);
        }
    }
}

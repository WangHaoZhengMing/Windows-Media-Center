use crate::profile::{self, *};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CategoryPose {
    pub row_y: f32,
    pub title_x: f32,
    pub title_y: f32,
    pub highlight: f32,
    pub strip_opacity: f32,
}

impl CategoryPose {
    fn settled(index: usize, selected: usize) -> Self {
        let row_y = if index < selected {
            -((selected - index) as f32) * ROW_STEP
        } else if index > selected {
            EXPANDED_STEP + (index - selected - 1) as f32 * ROW_STEP
        } else {
            0.
        };
        let active = index == selected;
        Self {
            row_y,
            title_x: if active { 0. } else { TITLE_LEFT_SHIFT },
            title_y: if active { TITLE_TOP_INSET } else { 0. },
            highlight: if active { 1. } else { 0. },
            strip_opacity: if active { 1. } else { 0. },
        }
    }
}

/// One retained timeline coordinates all rows. Retargeting uses the current
/// sample, matching MCML's RelativeTo=Current without jumping back to a frame.
pub struct CategoryMotion<const N: usize> {
    selected: usize,
    from: [CategoryPose; N],
    to: [CategoryPose; N],
    started_at: Duration,
}

pub struct MenuSample<const N: usize> {
    pub rows: [CategoryPose; N],
    pub moving: bool,
}

impl<const N: usize> CategoryMotion<N> {
    pub fn new(selected: usize) -> Self {
        assert!(selected < N);
        let poses = std::array::from_fn(|index| CategoryPose::settled(index, selected));
        Self {
            selected,
            from: poses,
            to: poses,
            started_at: Duration::ZERO,
        }
    }

    pub fn retarget(&mut self, selected: usize, now: Duration) {
        assert!(selected < N);
        if selected == self.selected {
            return;
        }
        self.from = self.sample(now, false).rows;
        self.to = std::array::from_fn(|index| CategoryPose::settled(index, selected));
        self.selected = selected;
        self.started_at = now;
    }

    pub fn sample(&self, now: Duration, reduce_motion: bool) -> MenuSample<N> {
        let elapsed = now.saturating_sub(self.started_at).as_secs_f32() * 1000.;
        if reduce_motion || elapsed >= CATEGORY_MOVE_MS as f32 || self.from == self.to {
            return MenuSample {
                rows: self.to,
                moving: false,
            };
        }
        let movement = s_curve(profile::phase(elapsed, 0, CATEGORY_MOVE_MS));
        let rows = std::array::from_fn(|index| {
            let from = self.from[index];
            let to = self.to[index];
            let (margin, alpha) = if index == self.selected {
                let progress = profile::phase(elapsed, GAIN_DELAY_MS, CATEGORY_MOVE_MS);
                (s_curve(progress), profile::log_curve(progress))
            } else {
                let progress = profile::phase(elapsed, 0, LOSE_FOCUS_MS);
                (s_curve(progress), profile::exp_curve(progress))
            };
            CategoryPose {
                row_y: profile::lerp(from.row_y, to.row_y, movement),
                title_x: profile::lerp(from.title_x, to.title_x, margin),
                title_y: profile::lerp(from.title_y, to.title_y, margin),
                highlight: profile::lerp(from.highlight, to.highlight, alpha),
                // Moving both strips through the fixed viewport is a Rust-side
                // composition approximation; MCML also has panning-driven alpha.
                strip_opacity: profile::lerp(from.strip_opacity, to.strip_opacity, alpha),
            }
        });
        MenuSample { rows, moving: true }
    }
}

//! Playback presentation parameters, separate from the WMC page geometry.
//! See docs/APPLE_MUSIC_REFERENCE.md for supplied evidence and WMC adaptations.
use crate::motion::SpringParameters;

pub const LINE_SPRING: SpringParameters = SpringParameters {
    mass: 1.,
    stiffness: 100.,
    damping: 18.,
};
pub const LINE_DELAY: f32 = 0.05;
pub const ANIMATION_HEADSTART: f64 = 0.1;
pub const MAX_SELECTED_LINES: usize = 2;
pub const MAX_END_TIME_OFFSET: f64 = 0.5;
pub const DESELECTED_SCALE: f32 = 0.98;
// The builder's explicit +376 store is 40; the earlier declaration says 30.
pub const PROGRESSION_FEATHER: f32 = 40.;
pub const SYLLABLE_LIFT: f32 = 2.;
pub const TAP_PROGRESS_FREEZE: f64 = 0.1;
pub const FINISH_DURATION: f32 = 0.25;
// WMC-specific safety/tuning; the supplied files do not recover this mapping.
pub const MAX_STAGGER_ROWS: usize = 6;
pub const CLARITY_OMEGA: f32 = 16.;

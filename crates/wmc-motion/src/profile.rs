//! Timings and layout declarations extracted from STARTMENU*.MCML.
//! Curve formulas below are approximations of the native MCML interpolation.

pub const CATEGORY_MOVE_MS: u64 = 267;
pub const GAIN_DELAY_MS: u64 = 133;
pub const LOSE_FOCUS_MS: u64 = 100;
pub const THUMB_ALPHA_MS: u64 = 50;
pub const QUICK_LINK_MOVE_MS: u64 = 400;
pub const QUICK_LINK_DELAY_MS: u64 = 67;

const ART_SCALE: f32 = 238. / 258.;
pub const ROW_STEP: f32 = (74. - 4.) * ART_SCALE;
pub const EXPANDED_STEP: f32 = (319. - 4.) * ART_SCALE;
pub const TITLE_TOP_INSET: f32 = 24. * ART_SCALE;
pub const TITLE_LEFT_SHIFT: f32 = (14. - -6.) * ART_SCALE;

/// A monotonic S curve of strength 2; native SCurve internals are not recovered.
pub fn s_curve(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    let a = t * t;
    let b = (1. - t) * (1. - t);
    a / (a + b)
}

pub(crate) fn log_curve(t: f32) -> f32 {
    1. - (1. - t.clamp(0., 1.)).powi(3)
}

pub(crate) fn exp_curve(t: f32) -> f32 {
    t.clamp(0., 1.).powi(3)
}

pub(crate) fn phase(elapsed_ms: f32, start: u64, end: u64) -> f32 {
    ((elapsed_ms - start as f32) / (end - start) as f32).clamp(0., 1.)
}

pub(crate) fn lerp(from: f32, to: f32, progress: f32) -> f32 {
    from + (to - from) * progress
}

//! Spatial defocus and viewport fading, separate from timed word highlighting.
//! These curve constants are tuned for WMC, not asserted binary presets.
pub const MAX_SIGMA: f32 = 9.;

pub fn blur_sigma(distance_in_rows: f32, clarity: f32) -> f32 {
    (distance_in_rows.abs() * 2.).min(MAX_SIGMA) * (1. - clarity.clamp(0., 1.))
}

pub fn edge_opacity(center_y: f32, viewport_height: f32) -> f32 {
    let distance = center_y.min(viewport_height - center_y).max(0.);
    let t = (distance / 72.).clamp(0., 1.);
    t * t * (3. - 2. * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn focused_line_stays_sharp_and_defocus_increases_with_distance() {
        assert_eq!(blur_sigma(4., 1.), 0.);
        assert!(blur_sigma(1., 0.) < blur_sigma(3., 0.));
        assert_eq!(blur_sigma(100., 0.), MAX_SIGMA);
        assert_eq!(blur_sigma(-2., 0.3), blur_sigma(2., 0.3));
    }
    #[test]
    fn viewport_fade_is_symmetric_without_an_inner_clip_edge() {
        assert_eq!(edge_opacity(0., 720.), 0.);
        assert_eq!(edge_opacity(360., 720.), 1.);
        assert_eq!(edge_opacity(24., 720.), edge_opacity(696., 720.));
        assert!(edge_opacity(48., 720.) > edge_opacity(24., 720.));
    }
}

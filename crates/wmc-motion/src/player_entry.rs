//! Recovered AUDIONOWPLAYING.XML show tracks; see player-entry-motion.json.
//! SCurve/EaseIn use smoothstep/quadratic approximations; Cosine is exact.
pub const LIBRARY_EXIT_SECONDS: f32 = 0.25;

pub fn library_exit(time: f32, reduced: bool) -> f32 {
    if reduced {
        return 1.;
    }
    1. - smooth((time / LIBRARY_EXIT_SECONDS).clamp(0., 1.))
}

#[derive(Clone, Copy)]
pub struct PlayerEntry {
    pub cover_alpha: f32,
    pub details_alpha: f32,
    pub details_x: f32,
    pub background_alpha: f32,
}

fn smooth(t: f32) -> f32 {
    t * t * (3. - 2. * t)
}

impl PlayerEntry {
    pub fn sample(time: f32, reduced: bool) -> Self {
        let time = if reduced { 2.6 } else { time.max(0.) };
        let details_x = if time >= 2.6 {
            0.
        } else if time < 1. {
            -200. + 190. * smooth(time)
        } else {
            let t = ((time - 1.) / 1.6).clamp(0., 1.);
            -10. + 10. * t * t
        };
        let fade = ((time - 0.3) / 0.7).clamp(0., 1.);
        Self {
            cover_alpha: (time / 0.7).clamp(0., 1.),
            details_alpha: (1. - (fade * std::f32::consts::PI).cos()) * 0.5,
            details_x,
            background_alpha: smooth((time / 0.5).clamp(0., 1.)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_delays_endpoints_and_reduced_motion() {
        assert_eq!(PlayerEntry::sample(0.3, false).details_alpha, 0.);
        let cover = PlayerEntry::sample(0.7, false);
        assert_eq!(cover.cover_alpha, 1.);
        assert_eq!(PlayerEntry::sample(1., false).details_alpha, 1.);
        let end = PlayerEntry::sample(2.6, false);
        assert_eq!(end.details_x, 0.);
        assert_eq!(library_exit(0., false), 1.);
        assert_eq!(library_exit(0.25, false), 0.);
        assert_eq!(library_exit(0., true), 1.);
    }
}

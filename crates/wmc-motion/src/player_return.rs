//! Browse NavigateToBackward tracks, with independent layer clocks.
//! Log/Exp are approximated by cubic ease-out/ease-in; timing and values are recovered.
pub const RETURN_DURATION: f32 = 0.533;

pub struct PlayerReturn {
    pub outgoing_alpha: f32,
    pub title_alpha: f32,
    pub title_x: f32,
    pub title_scale: f32,
    pub commands_alpha: f32,
    pub pivots_scale: f32,
    pub item_alpha: f32,
    pub item_scale: f32,
    pub selected_scale: f32,
    pub counter_alpha: f32,
}

fn phase(time: f32, start: f32, end: f32) -> f32 {
    ((time - start) / (end - start)).clamp(0., 1.)
}
fn log(t: f32) -> f32 {
    1. - (1. - t).powi(3)
}
fn exp(t: f32) -> f32 {
    t.powi(3)
}

impl PlayerReturn {
    pub fn sample(time: f32, reduced: bool) -> Self {
        let t = if reduced {
            RETURN_DURATION
        } else {
            time.max(0.)
        };
        let title = phase(t, 0.2, 0.4);
        let pivots = phase(t, 0.333, 0.5);
        let items = phase(t, 0.3, 0.5);
        Self {
            // Host adaptation: outgoing audio page has no proven matching hide track.
            outgoing_alpha: 1. - phase(t, 0., 0.2),
            title_alpha: exp(title),
            title_x: 50. * (1. - log(title)),
            title_scale: 1. + 0.2 * (1. - log(title)),
            commands_alpha: exp(pivots),
            pivots_scale: 1. + 0.1 * (1. - log(pivots)),
            item_alpha: items,
            item_scale: 1. + 0.2 * (1. - log(items)),
            selected_scale: 1. + 0.5 * (1. - log(items)),
            counter_alpha: phase(t, 0.333, 0.533),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_return_layers_keep_the_original_delays_and_selection_scale() {
        let held = PlayerReturn::sample(0.2, false);
        assert_eq!(held.title_alpha, 0.);
        assert_eq!(held.item_alpha, 0.);
        assert_eq!(held.selected_scale, 1.5);
        let mid = PlayerReturn::sample(0.35, false);
        assert!(mid.title_alpha > mid.commands_alpha);
        assert!(mid.selected_scale > mid.item_scale);
        assert_eq!(mid.outgoing_alpha, 0.);
        for reduced in [false, true] {
            let end = PlayerReturn::sample(if reduced { 0. } else { RETURN_DURATION }, reduced);
            assert_eq!(end.title_alpha, 1.);
            assert_eq!(end.item_scale, 1.);
            assert_eq!(end.selected_scale, 1.);
            assert_eq!(end.counter_alpha, 1.);
        }
    }
}

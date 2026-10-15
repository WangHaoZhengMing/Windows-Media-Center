//! Playback chrome fades on pointer inactivity; no background timer is needed.
use std::time::{Duration, Instant};

const LEAVE_DELAY: Duration = Duration::from_secs(2);
const IDLE_DELAY: Duration = Duration::from_secs(5);
const FADE: f32 = 0.25;

pub(super) struct AutoHide {
    inside: bool,
    position: Option<(f32, f32)>,
    last_motion: Instant,
    left_at: Instant,
    from: f32,
    visible: bool,
    transition: Instant,
}
impl AutoHide {
    pub fn new(now: Instant) -> Self {
        Self {
            inside: false,
            position: None,
            last_motion: now,
            left_at: now,
            from: 1.,
            visible: true,
            transition: now,
        }
    }
    fn alpha_at(&self, now: Instant) -> f32 {
        let t = (now.saturating_duration_since(self.transition).as_secs_f32() / FADE).clamp(0., 1.);
        let eased = t * t * (3. - 2. * t);
        self.from + (if self.visible { 1. } else { 0. } - self.from) * eased
    }
    pub fn update(
        &mut self,
        now: Instant,
        inside: bool,
        position: (f32, f32),
        interacting: bool,
    ) -> f32 {
        if inside && (!self.inside || self.position != Some(position) || interacting) {
            self.last_motion = now;
        }
        if !inside && self.inside {
            self.left_at = now;
        }
        self.inside = inside;
        self.position = Some(position);
        let deadline = if inside {
            self.last_motion + IDLE_DELAY
        } else {
            self.left_at + LEAVE_DELAY
        };
        let visible = interacting || (now < deadline && (inside || self.visible));
        if visible != self.visible {
            self.from = self.alpha_at(now);
            self.visible = visible;
            self.transition = if visible { now } else { deadline };
        }
        self.alpha_at(now)
    }
    pub fn hide_cursor(&self) -> bool {
        self.inside && !self.visible
    }
}

impl super::PlayerView {
    /// Hosts use the same fade for their window header when embedding this player.
    pub fn controls_opacity(&mut self, window: &gpui_kit::Window, cx: &gpui_kit::App) -> f32 {
        self.update_chrome_visibility(window, cx)
    }
    pub(super) fn update_chrome_visibility(
        &mut self,
        window: &gpui_kit::Window,
        cx: &gpui_kit::App,
    ) -> f32 {
        let position = window.mouse_position();
        // An already-hidden pointer (e.g. keyboard input) uses the two-second
        // absence delay too. Our own hide must not reveal the chrome again.
        let inside = window.is_window_hovered() && cx.is_cursor_visible();
        let alpha = self.auto_hide.update(
            Instant::now(),
            inside,
            (f32::from(position.x), f32::from(position.y)),
            self.dragging,
        );
        if self.auto_hide.hide_cursor() {
            (self.hide_cursor)();
        }
        alpha
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn idle_pointer_hides_at_five_seconds_and_movement_fades_back_in() {
        let start = Instant::now();
        let mut state = AutoHide::new(start);
        assert_eq!(state.update(start, true, (10., 10.), false), 1.);
        assert_eq!(
            state.update(start + Duration::from_millis(4999), true, (10., 10.), false),
            1.
        );
        let alpha = state.update(start + Duration::from_millis(5125), true, (10., 10.), false);
        assert!((alpha - 0.5).abs() < 0.001);
        assert!(state.hide_cursor());
        assert_eq!(
            state.update(start + Duration::from_millis(5250), true, (10., 10.), false),
            0.
        );
        assert_eq!(
            state.update(start + Duration::from_secs(6), true, (11., 10.), false),
            0.
        );
        assert!(!state.hide_cursor());
        assert_eq!(
            state.update(start + Duration::from_millis(6250), true, (11., 10.), false),
            1.
        );
    }
    #[test]
    fn leaving_has_a_separate_two_second_delay_and_dragging_stays_visible() {
        let start = Instant::now();
        let mut state = AutoHide::new(start);
        state.update(start, true, (0., 0.), false);
        state.update(start + Duration::from_secs(1), false, (0., 0.), false);
        assert_eq!(
            state.update(start + Duration::from_millis(2999), false, (0., 0.), false),
            1.
        );
        assert_eq!(
            state.update(start + Duration::from_millis(3250), false, (0., 0.), false),
            0.
        );
        assert!(!state.hide_cursor());
        state.update(start + Duration::from_secs(4), true, (0., 0.), true);
        assert_eq!(
            state.update(start + Duration::from_secs(20), true, (0., 0.), true),
            1.
        );
        assert_eq!(
            state.update(start + Duration::from_secs(24), true, (0., 0.), false),
            1.
        );
    }
    #[test]
    fn leaving_after_idle_does_not_reveal_hidden_controls() {
        let start = Instant::now();
        let mut state = AutoHide::new(start);
        state.update(start, true, (0., 0.), false);
        assert_eq!(
            state.update(start + Duration::from_secs(6), true, (0., 0.), false),
            0.
        );
        assert_eq!(
            state.update(start + Duration::from_secs(7), false, (0., 0.), false),
            0.
        );
        assert_eq!(
            state.update(start + Duration::from_secs(8), false, (0., 0.), false),
            0.
        );
        state.update(start + Duration::from_secs(9), true, (0., 0.), false);
        assert_eq!(
            state.update(start + Duration::from_millis(9250), true, (0., 0.), false),
            1.
        );
    }
}

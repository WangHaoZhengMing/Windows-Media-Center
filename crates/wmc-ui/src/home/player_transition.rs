//! Concurrent outgoing player and per-layer Browse return tracks.
use super::HomeView;
use gpui_kit::{Context, Window};
use std::time::Instant;
use wmc_motion::player_return::{PlayerReturn, RETURN_DURATION};

impl HomeView {
    pub(super) fn return_from_player(&mut self, cx: &mut Context<Self>) {
        if self.player_exit.is_some() || !self.player_visible {
            return;
        }
        self.player_visible = false;
        if let Some(player) = &self.player {
            player.update(cx, |player, _| player.leave_page());
        }
        let started = Instant::now();
        self.player_exit = Some(started);
        self.library_return = Some(started);
        cx.notify();
    }

    pub(super) fn advance_player_exit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(started) = self.player_exit else {
            return;
        };
        if cx.reduce_motion() || started.elapsed().as_secs_f32() >= RETURN_DURATION {
            self.player_exit = None;
            window.focus(&self.focus, cx);
        } else {
            window.request_animation_frame();
        }
    }

    pub(super) fn player_exit_alpha(&self, reduced: bool) -> f32 {
        self.player_exit.map_or(1., |started| {
            PlayerReturn::sample(started.elapsed().as_secs_f32(), reduced).outgoing_alpha
        })
    }
}

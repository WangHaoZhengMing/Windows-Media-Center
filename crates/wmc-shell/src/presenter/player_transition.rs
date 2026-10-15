//! Concurrent outgoing player and per-layer Browse return tracks.
use crate::shell::AppShell;
use gpui_kit::{Context, Window};
use std::time::Instant;
use wmc_motion::player_return::{PlayerReturn, RETURN_DURATION};

impl AppShell {
    pub(crate) fn return_from_player(&mut self, cx: &mut Context<Self>) {
        if self.player_page.exiting.is_some() || !self.player_page.visible {
            return;
        }
        self.player_page.visible = false;
        if let Some(player) = &self.player_page.view {
            player.update(cx, |player, _| player.leave_page());
        }
        let started = Instant::now();
        self.player_page.exiting = Some(started);
        self.player_page.library_return = Some(started);
        cx.notify();
    }

    pub(crate) fn advance_player_exit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(started) = self.player_page.exiting else {
            return;
        };
        if cx.reduce_motion() || started.elapsed().as_secs_f32() >= RETURN_DURATION {
            self.player_page.exiting = None;
            if !self.player_page.visible {
                if let Some(player) = self.player_page.view.take() {
                    player.update(cx, |player, _| player.release(window));
                }
                self.player_page.subscription = None;
                if let Some(track) = self.app.current_track()
                    && let Some(cover) = track.cover
                {
                    if let Some(old) = self.player_page.cover.take() {
                        let _ = window.drop_image(old);
                    }
                    self.player_page.cover = Some(wmc_design::images::render_image(
                        image::imageops::thumbnail(&cover, 64, 64),
                    ));
                }
            }
            window.focus(&self.focus, cx);
        } else {
            window.request_animation_frame();
        }
    }

    pub(crate) fn player_exit_alpha(&self, reduced: bool) -> f32 {
        self.player_page.exiting.map_or(1., |started| {
            PlayerReturn::sample(started.elapsed().as_secs_f32(), reduced).outgoing_alpha
        })
    }
}

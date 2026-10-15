//! The home page owns the player even when its full-screen view is hidden.
use super::HomeView;
use crate::theme::{Layout, WHITE, color};
use gpui_kit::{prelude::*, *};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use wmc_core::{CATEGORIES, EntryId, MediaCommand};

fn queue_target(
    length: usize,
    current: Option<usize>,
    forward: bool,
    repeat: bool,
    shuffle: bool,
    played: &[usize],
    random: usize,
) -> Option<usize> {
    if length == 0 {
        return None;
    }
    let Some(current) = current.filter(|&index| index < length) else {
        return Some(0);
    };
    if shuffle && forward {
        let mut candidates: Vec<_> = (0..length)
            .filter(|&index| index != current && !played.contains(&index))
            .collect();
        if candidates.is_empty() && repeat {
            candidates = (0..length)
                .filter(|&index| length == 1 || index != current)
                .collect();
        }
        return (!candidates.is_empty()).then(|| candidates[random % candidates.len()]);
    }
    if forward {
        if current + 1 < length {
            Some(current + 1)
        } else if repeat {
            Some(0)
        } else {
            None
        }
    } else if current > 0 {
        Some(current - 1)
    } else if repeat {
        Some(length - 1)
    } else {
        None
    }
}

impl HomeView {
    pub(super) fn start_playback_monitor(&self, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(250))
                    .await;
                if this
                    .update_in(cx, |this, window, cx| {
                        let finished = this
                            .player
                            .as_ref()
                            .is_some_and(|player| player.read(cx).is_finished());
                        if finished && !this.playback_finished && !this.library.loading_track {
                            this.playback_finished = true;
                            this.step_queue(true, window, cx);
                        } else if !finished {
                            this.playback_finished = false;
                        }
                        if this.player.is_some() {
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    pub(super) fn step_queue(
        &mut self,
        forward: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let random = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as usize;
        if let Some(index) = queue_target(
            self.song_queue.len(),
            self.queue_index,
            forward,
            self.destination_state.repeat,
            self.destination_state.shuffle,
            &self.queue_played,
            random,
        ) {
            if self.destination_state.shuffle && self.queue_played.len() >= self.song_queue.len() {
                self.queue_played.clear();
            }
            self.load_queue_song(index, self.player_visible, window, cx);
        }
    }

    pub(super) fn clear_queue(&mut self, cx: &mut Context<Self>) {
        self.song_queue.clear();
        self.queue_index = None;
        self.queue_played.clear();
        self.library.play_generation += 1;
        self.library.loading_track = false;
        self.song_transition = None;
        cx.notify();
    }

    pub(super) fn show_queue(&mut self, cx: &mut Context<Self>) {
        if self.player_visible {
            self.return_from_player(cx);
        }
        self.preview = CATEGORIES
            .iter()
            .flat_map(|category| category.entries)
            .find(|entry| entry.id == EntryId::PlayQueue);
        self.queue_visible = true;
        self.destination_state
            .reset(EntryId::PlayQueue, self.ambient_motion);
        cx.notify();
    }

    pub(super) fn media_command(
        &mut self,
        command: MediaCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if command == MediaCommand::Stop {
            self.library.play_generation += 1;
            self.library.loading_track = false;
            self.song_transition = None;
        }
        match command {
            MediaCommand::Next => self.step_queue(true, window, cx),
            MediaCommand::Previous => self.step_queue(false, window, cx),
            MediaCommand::PlayPause if self.player.is_none() => {
                if !self.song_queue.is_empty() {
                    self.load_queue_song(0, false, window, cx);
                }
            }
            _ => {
                if let Some(player) = &self.player {
                    player.update(cx, |player, cx| player.dispatch(command, cx));
                } else {
                    self.transport.dispatch(command);
                }
            }
        }
        cx.notify();
    }

    pub(super) fn now_playing(&self, layout: Layout, cx: &mut Context<Self>) -> Div {
        let Some(player) = &self.player else {
            return div();
        };
        let player = player.read(cx);
        div()
            .absolute()
            .left(layout.unit(24.))
            .bottom(layout.unit(16.))
            .child(
                wmc_controls::button("home-now-playing", "打开正在播放")
                    .flex()
                    .items_center()
                    .justify_start()
                    .gap(layout.unit(12.))
                    .max_w(layout.unit(410.))
                    .cursor_pointer()
                    .text_color(color(WHITE))
                    .child(
                        img(player.cover())
                            .size(layout.unit(64.))
                            .rounded(layout.unit(64. * 0.03))
                            .object_fit(ObjectFit::Cover),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(
                                div()
                                    .text_size(layout.unit(19.))
                                    .text_ellipsis()
                                    .child(player.title().to_owned()),
                            )
                            .child(
                                div()
                                    .text_size(layout.unit(15.))
                                    .text_color(color(WHITE).opacity(0.7))
                                    .text_ellipsis()
                                    .child(player.artist().to_owned()),
                            ),
                    )
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.player_exit = None;
                        this.player_visible = true;
                        if let Some(player) = &this.player {
                            window.focus(&player.focus_handle(cx), cx);
                        }
                        cx.notify();
                    })),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::queue_target;
    #[test]
    fn queue_navigation_handles_empty_end_repeat_and_cleared_cursor() {
        assert_eq!(queue_target(0, None, true, true, true, &[], 0), None);
        assert_eq!(
            queue_target(3, Some(0), true, false, false, &[], 0),
            Some(1)
        );
        assert_eq!(queue_target(3, Some(2), true, false, false, &[], 0), None);
        assert_eq!(queue_target(3, Some(2), true, true, false, &[], 0), Some(0));
        assert_eq!(
            queue_target(3, Some(0), false, true, false, &[], 0),
            Some(2)
        );
        assert_eq!(queue_target(3, None, true, false, false, &[], 0), Some(0));
    }
    #[test]
    fn shuffle_chooses_another_valid_track() {
        for random in 0..20 {
            let next = queue_target(4, Some(2), true, false, true, &[2], random).unwrap();
            assert!(next < 4);
            assert_ne!(next, 2);
        }
        assert_eq!(queue_target(1, Some(0), true, false, true, &[0], 0), None);
        assert_eq!(
            queue_target(3, Some(1), true, false, true, &[0, 1], 0),
            Some(2)
        );
        assert_eq!(
            queue_target(3, Some(2), true, false, true, &[0, 1, 2], 0),
            None
        );
        assert_eq!(
            queue_target(3, Some(2), true, true, true, &[0, 1, 2], 0),
            Some(0)
        );
        assert_eq!(queue_target(1, Some(0), true, true, true, &[0], 0), Some(0));
    }
}

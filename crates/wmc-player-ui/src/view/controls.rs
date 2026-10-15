use super::{PlayerView, button::button};
use gpui_kit::{prelude::*, *};
use wmc_assets::{Icon, TRANSPORT_BACKGROUND, TRANSPORT_DIVIDER};

impl PlayerView {
    pub(super) fn progress(
        &mut self,
        frame: &super::frame::FrameState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let (scale, seconds, duration) = (frame.scale, frame.seconds, frame.duration);
        let (left, top, width) = (
            frame.columns.info_left,
            frame.columns.info_top + 50.,
            frame.columns.info_width,
        );
        let playback_progress = if duration > 0.0 {
            (seconds / duration).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };
        // Playback updates must not overwrite the user's position during a drag.
        if !self.dragging
            && (self.progress_slider.read(cx).value().end() - playback_progress).abs()
                > f32::EPSILON
        {
            self.progress_slider.update(cx, |state, cx| {
                state.set_value(playback_progress, window, cx)
            });
        }
        let progress = self.progress_slider.read(cx).percentage().end;
        let preview_seconds = if self.dragging {
            f64::from(progress) * duration
        } else {
            seconds
        };
        let disabled = self.settings.embedding || duration <= 0.0;

        let track = gpui_kit::base::SliderTrack::new(&self.progress_slider)
            .disabled(disabled)
            .relative()
            .size_full()
            .when(!disabled, |track| track.cursor_pointer())
            .child(
                div()
                    .absolute()
                    .top(px(10. * scale))
                    .left_0()
                    .w_full()
                    .h(px(6. * scale))
                    .rounded(px(3. * scale))
                    .bg(rgb(0xffffff).opacity(0.22)),
            )
            .child(
                gpui_kit::base::SliderIndicator::new(&self.progress_slider)
                    .absolute()
                    .inset_0()
                    .child(
                        div()
                            .absolute()
                            .top(px(10. * scale))
                            .left_0()
                            .w(relative(progress))
                            .h(px(6. * scale))
                            .rounded(px(3. * scale))
                            .bg(rgb(0xffffff).opacity(0.78)),
                    ),
            )
            .when(self.dragging, |track| {
                track.child(
                    gpui_kit::base::SliderThumb::new(&self.progress_slider)
                        .disabled(disabled)
                        .absolute()
                        .top(px(8. * scale))
                        .left(relative(progress))
                        .ml(px(-5. * scale))
                        .size(px(10. * scale))
                        .rounded_full()
                        .bg(rgb(0xffffff)),
                )
            });
        let slider = gpui_kit::base::Slider::new(&self.progress_slider)
            .disabled(disabled)
            .h(px(26. * scale))
            .w_full()
            .child(track);

        div()
            .absolute()
            .left(px(left * scale))
            .top(px(top * scale))
            .w(px(width * scale))
            .child(
                div()
                    .id("player-progress")
                    .track_focus(&self.progress_focus)
                    .child(slider),
            )
            .child(
                div()
                    .flex()
                    .justify_between()
                    .text_size(super::text::font_pixels(12. * scale))
                    .text_color(rgb(0xffffff).opacity(0.45))
                    .child(format_time(preview_seconds))
                    .child(format!(
                        "−{}",
                        format_time((duration - preview_seconds).max(0.0))
                    )),
            )
    }
    pub(super) fn transport(&self, scale: f32, playing: bool, cx: &mut Context<Self>) -> Div {
        let mut row = div()
            .relative()
            .size_full()
            .flex()
            .items_center()
            .justify_end()
            .pr(px(10. * scale))
            .gap(px(2. * scale));
        for (index, (id, icon)) in [
            ("player-queue", Icon::Playlist),
            ("player-stop", Icon::Stop),
            ("player-rewind", Icon::Rewind),
            ("player-restart", Icon::Previous),
            (
                "player-play",
                if playing { Icon::Pause } else { Icon::Play },
            ),
            ("player-next", Icon::Next),
            ("player-forward", Icon::FastForward),
            (
                "player-mute",
                if self
                    .volume_state
                    .is_some_and(|state| state.muted || state.percent == 0)
                {
                    Icon::Muted
                } else {
                    Icon::Mute
                },
            ),
            ("player-volume-down", Icon::VolumeDown),
            ("player-volume-up", Icon::VolumeUp),
        ]
        .into_iter()
        .enumerate()
        {
            if index == 1 || index == 2 || index == 7 {
                row = row.child(
                    img(TRANSPORT_DIVIDER)
                        .w(px(2. * scale))
                        .h(px(20. * scale))
                        .mx(px(2. * scale)),
                );
            }
            let (w, h) = if index == 4 { (42., 43.) } else { (34., 31.) };

            row = row.child(button(id, icon, w, h, scale).on_click(cx.listener(
                move |this, _, _, cx| {
                    match index {
                        0 => cx.emit(super::PlayerEvent::Queue),
                        1 => this.stop(cx),
                        2 => this.seek(this.app.position() - 5., cx),
                        3 => cx.emit(super::PlayerEvent::Previous),
                        4 => this.toggle_playback(cx),
                        5 => cx.emit(super::PlayerEvent::Next),
                        6 => this.seek(this.app.position() + 5., cx),
                        7 => this.toggle_mute(cx),
                        8 => this.change_volume(-5, cx),
                        9 => this.change_volume(5, cx),
                        _ => {}
                    }
                    cx.notify();
                },
            )));
        }
        div()
            .absolute()
            .right_0()
            .bottom_0()
            .occlude()
            .w(px(436. * scale))
            .h(px(48. * scale))
            .child(
                img(TRANSPORT_BACKGROUND)
                    .absolute()
                    .size_full()
                    .object_fit(ObjectFit::Fill),
            )
            .child(row)
            .when(
                self.volume_changed
                    .is_some_and(|time| time.elapsed().as_secs_f32() < 2.),
                |bar| {
                    let state = self.volume_state;
                    let volume = state.map_or(0., |state| {
                        if state.muted {
                            0.
                        } else {
                            f32::from(state.percent) / 100.
                        }
                    });
                    bar.child(
                        div()
                            .absolute()
                            .right(px(12. * scale))
                            .bottom(px(56. * scale))
                            .w(px(150. * scale))
                            .p(px(12. * scale))
                            .rounded(px(5. * scale))
                            .bg(rgb(0x071c35).opacity(0.95))
                            .border_1()
                            .border_color(rgb(0x8dbfdc).opacity(0.4))
                            .text_size(super::text::font_pixels(14. * scale))
                            .child(match (state, &self.volume_error) {
                                (_, Some(_)) => "系统音量不可用".to_owned(),
                                (Some(state), _) if state.muted => "系统已静音".to_owned(),
                                (Some(state), _) => format!("系统音量 {}%", state.percent),
                                _ => "正在读取系统音量".to_owned(),
                            })
                            .child(
                                div()
                                    .mt(px(8. * scale))
                                    .h(px(4. * scale))
                                    .w_full()
                                    .bg(rgb(0x36546b))
                                    .child(div().h_full().w(relative(volume)).bg(rgb(0x72ccf5))),
                            ),
                    )
                },
            )
    }
}

fn format_time(seconds: f64) -> String {
    let seconds = seconds.max(0.).floor() as u64;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

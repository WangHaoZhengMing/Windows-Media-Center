use super::HomeView;
use crate::{components::resource_button, theme::Layout};
use gpui_kit::{prelude::*, *};
use wmc_assets::{Icon, TRANSPORT_BACKGROUND, TRANSPORT_DIVIDER};
use wmc_core::MediaCommand;

impl HomeView {
    pub(super) fn transport_bar(&self, layout: Layout, cx: &mut Context<Self>) -> Div {
        div()
            .absolute()
            .right_0()
            .bottom_0()
            .w(layout.unit(436.))
            .h(layout.unit(48.))
            .child(
                img(TRANSPORT_BACKGROUND)
                    .absolute()
                    .size_full()
                    .object_fit(ObjectFit::Fill),
            )
            .child(
                div()
                    .relative()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_end()
                    .pr(layout.unit(10.))
                    .gap(layout.unit(2.))
                    .child(
                        resource_button(
                            "media-queue",
                            Icon::Playlist,
                            34.,
                            31.,
                            true,
                            layout.scale,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.show_queue(cx);
                        })),
                    )
                    .child(divider(layout))
                    .child(self.media_button(
                        "media-stop",
                        Icon::Stop,
                        MediaCommand::Stop,
                        layout,
                        cx,
                    ))
                    .child(divider(layout))
                    .child(self.media_button(
                        "media-rewind",
                        Icon::Rewind,
                        MediaCommand::Rewind,
                        layout,
                        cx,
                    ))
                    .child(self.media_button(
                        "media-previous",
                        Icon::Previous,
                        MediaCommand::Previous,
                        layout,
                        cx,
                    ))
                    .child(
                        self.media_button(
                            "media-play",
                            if self
                                .player
                                .as_ref()
                                .is_some_and(|player| player.read(cx).is_playing())
                            {
                                Icon::Pause
                            } else {
                                Icon::Play
                            },
                            MediaCommand::PlayPause,
                            layout,
                            cx,
                        ),
                    )
                    .child(self.media_button(
                        "media-next",
                        Icon::Next,
                        MediaCommand::Next,
                        layout,
                        cx,
                    ))
                    .child(self.media_button(
                        "media-forward",
                        Icon::FastForward,
                        MediaCommand::FastForward,
                        layout,
                        cx,
                    ))
                    .child(divider(layout))
                    .child(
                        self.media_button(
                            "media-mute",
                            if self
                                .player
                                .as_ref()
                                .map_or(self.transport.muted(), |player| player.read(cx).is_muted())
                            {
                                Icon::Muted
                            } else {
                                Icon::Mute
                            },
                            MediaCommand::ToggleMute,
                            layout,
                            cx,
                        ),
                    )
                    .child(self.media_button(
                        "media-volume-down",
                        Icon::VolumeDown,
                        MediaCommand::VolumeDown,
                        layout,
                        cx,
                    ))
                    .child(self.media_button(
                        "media-volume-up",
                        Icon::VolumeUp,
                        MediaCommand::VolumeUp,
                        layout,
                        cx,
                    )),
            )
    }

    fn media_button(
        &self,
        id: &'static str,
        kind: Icon,
        command: MediaCommand,
        layout: Layout,
        cx: &mut Context<Self>,
    ) -> gpui_kit::base::Button {
        let available = self.player.is_some()
            || (command == MediaCommand::PlayPause && !self.song_queue.is_empty())
            || self.transport.available(command);
        let (width, height) = if command == MediaCommand::PlayPause {
            (42., 43.)
        } else {
            (34., 31.)
        };
        resource_button(id, kind, width, height, available, layout.scale).on_click(cx.listener(
            move |this, _, window, cx| {
                if available {
                    this.media_command(command, window, cx);
                }
            },
        ))
    }
}

fn divider(layout: Layout) -> Div {
    div()
        .mx(layout.unit(2.))
        .w(layout.unit(2.))
        .h(layout.unit(20.))
        .child(
            img(TRANSPORT_DIVIDER)
                .size_full()
                .object_fit(ObjectFit::Fill),
        )
}

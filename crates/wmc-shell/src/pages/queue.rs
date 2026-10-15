use crate::{
    pages::browse::{command_button, commands, content_column, page_title},
    shell::AppShell,
};
use gpui_kit::{prelude::*, *};
use wmc_assets::{CHECKED, UNCHECKED};
use wmc_design::theme::{Layout, MUTED, WHITE, color};

impl AppShell {
    pub(crate) fn queue_page(&self, layout: Layout, cx: &mut Context<Self>) -> Div {
        let mut left = commands(layout);
        for (index, title) in [
            "编辑播放列表",
            "清除全部",
            "保存为播放列表",
            "随机播放",
            "重复播放",
        ]
        .into_iter()
        .enumerate()
        {
            let available = index >= 3 || (index == 1 && !self.app.queue().entries().is_empty());
            let checked = if index == 3 {
                self.app.queue().shuffle
            } else {
                self.app.queue().repeat
            };
            left = left.child(
                command_button(
                    [
                        "queue-edit",
                        "queue-clear",
                        "queue-save",
                        "queue-shuffle",
                        "queue-repeat",
                    ][index],
                    title,
                    self.destination_state.row == index,
                    available,
                    layout,
                )
                .when(index >= 3, |button| {
                    button.child(
                        img(if checked { CHECKED } else { UNCHECKED })
                            .absolute()
                            .right(layout.unit(10.))
                            .size(layout.unit(24.)),
                    )
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    if index == 1 && available {
                        this.clear_queue(cx);
                    } else if available {
                        this.destination_state.row = index;
                        this.activate_destination(cx);
                    }
                })),
            );
        }
        let songs = if self.app.queue().entries().is_empty() {
            div()
                .mt(layout.unit(38.))
                .text_size(layout.unit(22.))
                .text_color(color(WHITE).opacity(0.75))
                .child("播放队列为空。")
                .into_any_element()
        } else {
            uniform_list(
                "queued-songs",
                self.app.queue().entries().len(),
                cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                    range
                        .map(|index| {
                            let title = this.app.queue().entries()[index].1.clone();
                            div()
                                .h(layout.unit(52.))
                                .w_full()
                                .pb(layout.unit(8.))
                                .child(
                                    wmc_design::button(("queued-song", index), title.clone())
                                        .size_full()
                                        .justify_start()
                                        .px(layout.unit(10.))
                                        .cursor_pointer()
                                        .text_size(layout.unit(20.))
                                        .when(this.app.queue().current() == Some(index), |row| {
                                            row.bg(color(0x317aa6).opacity(0.4))
                                        })
                                        .hover(|row| row.bg(color(0x317aa6).opacity(0.5)))
                                        .focus(|row| row.bg(color(0x317aa6).opacity(0.5)))
                                        .child(div().text_ellipsis().child(title))
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.load_queue_song(index, true, window, cx);
                                        })),
                                )
                        })
                        .collect()
                }),
            )
            .track_scroll(&self.queue_scroll)
            .h(layout.unit(350.))
            .w_full()
            .into_any_element()
        };
        div()
            .absolute()
            .size_full()
            .child(page_title("正在播放", layout))
            .child(
                div()
                    .absolute()
                    .left(layout.unit(170.))
                    .top(layout.unit(157.))
                    .text_size(layout.unit(29.))
                    .font_weight(FontWeight::LIGHT)
                    .text_color(color(MUTED))
                    .child("播放队列"),
            )
            .child(left)
            .child(
                content_column(layout)
                    .child(
                        div()
                            .flex()
                            .gap(layout.unit(105.))
                            .text_size(layout.unit(17.))
                            .text_color(color(MUTED))
                            .child("歌曲")
                            .child("艺术家")
                            .child("时长"),
                    )
                    .child(
                        div()
                            .h(layout.unit(1.))
                            .w_full()
                            .bg(color(MUTED).opacity(0.2)),
                    )
                    .child(songs),
            )
    }
}

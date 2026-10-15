use super::{PlayerView, frame::FrameState, text};
use gpui_kit::{prelude::*, *};

impl PlayerView {
    pub(super) fn metadata(&self, frame: &FrameState, cx: &mut Context<Self>) -> Div {
        let scale = frame.scale;
        let columns = &frame.columns;
        let unit = |v| px(v * scale);
        div()
            .absolute()
            .left(unit(columns.info_left))
            .top(unit(columns.info_top))
            .w(unit(columns.info_width))
            .child(
                div()
                    .w(unit((columns.info_width - 44.).max(40.)))
                    .h(unit(24.))
                    .overflow_hidden()
                    .child(text::caption(
                        self.title.clone(),
                        20.,
                        FontWeight::BOLD,
                        scale,
                    )),
            )
            .child(
                div()
                    .w(unit((columns.info_width - 44.).max(40.)))
                    .h(unit(23.))
                    .overflow_hidden()
                    .text_size(unit(18.))
                    .line_height(unit(23.))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(rgb(0xffffff).opacity(0.58))
                    .truncate()
                    .child(self.artist.clone()),
            )
            .child(
                div()
                    .id("player-song-more")
                    .absolute()
                    .right_0()
                    .top(unit(8.))
                    .size(unit(32.))
                    .rounded_full()
                    .border_1()
                    .border_color(rgb(0xffffff).opacity(0.2))
                    .bg(rgb(0xffffff).opacity(0.06))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(unit(20.))
                    .text_color(rgb(0xffffff))
                    .cursor_pointer()
                    .child("⋯")
                    .hover(|s| s.bg(rgb(0xffffff).opacity(0.16)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_more = !this.show_more;
                        cx.notify();
                    })),
            )
    }
}

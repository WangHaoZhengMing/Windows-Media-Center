//! Original WMC contextual menu materials around song actions.
use super::{
    PlayerView,
    frame::FrameState,
    wmc_skin::{self, Skin},
};
use gpui_kit::{prelude::*, *};
fn row(id: &'static str, text: &'static str, shortcut: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .group(id)
        .relative()
        .h(px(51.))
        .px(px(18.))
        .flex()
        .items_center()
        .justify_between()
        .cursor_pointer()
        .child(
            wmc_skin::surface(Skin::Focus)
                .opacity(0.)
                .group_hover(id, |s| s.opacity(1.)),
        )
        .child(div().relative().text_size(px(20.)).child(text))
        .child(
            div()
                .relative()
                .text_size(px(12.))
                .text_color(rgb(0x6f9fc8))
                .child(shortcut),
        )
}
impl PlayerView {
    pub(super) fn more_menu(&self, frame: &FrameState, cx: &mut Context<Self>) -> Div {
        let s = frame.scale;
        let c = &frame.columns;
        let x = ((c.info_left + c.info_width) * s - 287.).max(12.);
        let bottom = (c.lyric_top + c.lyric_height) * s;
        let y = ((c.info_top + 38.) * s).min(bottom - 181.).max(52.);
        div()
            .absolute()
            .size_full()
            .child(
                div()
                    .id("dismiss-player-more")
                    .absolute()
                    .size_full()
                    .occlude()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.show_more = false;
                            cx.notify();
                        }),
                    ),
            )
            .child(
                div()
                    .id("player-more-menu")
                    .absolute()
                    .left(px(x))
                    .top(px(y))
                    .w(px(287.))
                    .p(px(14.))
                    .occlude()
                    .bg(rgb(0x060e2d))
                    .text_color(rgb(0xf1faff))
                    .child(wmc_skin::surface(Skin::Background))
                    .child(wmc_skin::surface(Skin::Outline))
                    .child(row("more-translate-lyrics", "歌词翻译", "Ctrl+T").on_click(cx.listener(|this, _, window, cx| this.open_translation(window, cx))))
                    .child(row("more-lyric-info", "歌词信息", "").on_click(
                        cx.listener(|this, _, window, cx| this.open_lyric_info(window, cx)),
                    ))
                    .child(
                        row("more-search-lyrics", "搜索歌词", "Ctrl+L")
                            .on_click(cx.listener(|this, _, _, cx| this.search_online_lyrics(cx))),
                    )
                    .child(
                        row("more-search-cover", "匹配封面", "Ctrl+Shift+C")
                            .on_click(cx.listener(|this, _, _, cx| this.search_online_cover(cx))),
                    ),
            )
    }
}

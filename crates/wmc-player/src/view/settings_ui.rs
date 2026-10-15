//! Shared visual language for the player's settings dialogs.
use gpui_kit::{prelude::*, *};

pub(super) fn card() -> Div {
    div()
        .p(px(12.))
        .rounded(px(8.))
        .bg(rgb(0x102c41))
        .border_1()
        .border_color(rgb(0x25475e))
}
pub(super) fn label(text: &'static str) -> Div {
    div()
        .mt(px(12.))
        .mb(px(8.))
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0xaec9db))
        .child(text)
}
pub(super) fn button(id: &'static str, title: impl Into<SharedString>) -> gpui_kit::base::Button {
    let title = title.into();
    wmc_controls::button(id, title.clone())
        .h(px(36.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .px(px(14.))
        .rounded(px(6.))
        .border_1()
        .border_color(rgb(0x3a607b))
        .bg(rgb(0x1c405a))
        .text_size(px(13.))
        .text_color(rgb(0xe7f4fc))
        .cursor_pointer()
        .hover(|s| s.bg(rgb(0x2a5675)).border_color(rgb(0x75b9dd)))
        .focus_visible(|s| s.bg(rgb(0x2a5675)).border_color(rgb(0x75b9dd)))
        .child(title)
}
pub(super) fn panel(
    id: &'static str,
    title: &'static str,
    window: &Window,
    close: gpui_kit::base::Button,
    content: Div,
    scroll: bool,
) -> Stateful<Div> {
    let viewport = window.viewport_size();
    let (width, height) = (f32::from(viewport.width), f32::from(viewport.height));
    let panel_width = (width - 32.).clamp(240., 720.);
    let panel_height = (height - 32.).clamp(180., 760.);
    div()
        .id(id)
        .absolute()
        .inset_0()
        .occlude()
        .bg(rgb(0x020a12).opacity(0.66))
        .font_family("Microsoft YaHei UI")
        .text_size(px(14.))
        .text_color(rgb(0xe7f2fa))
        .child(
            div()
                .absolute()
                .left(px((width - panel_width) / 2.))
                .top(px((height - panel_height) / 2.))
                .w(px(panel_width))
                .h(px(panel_height))
                .flex()
                .flex_col()
                .overflow_hidden()
                .rounded(px(12.))
                .bg(rgb(0x091f31))
                .border_1()
                .border_color(rgb(0x3a607a))
                .shadow(vec![
                    BoxShadow::new(px(0.), px(16.), rgb(0x000000).opacity(0.5).into())
                        .blur_radius(px(36.)),
                ])
                .child(
                    div()
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px(px(22.))
                        .py(px(16.))
                        .border_b_1()
                        .border_color(rgb(0x25475e))
                        .child(
                            div()
                                .text_size(px(22.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(title),
                        )
                        .child(close),
                )
                .child(
                    div()
                        .id("song-settings-body")
                        .flex_1()
                        .min_h_0()
                        .p(px(16.))
                        .when(scroll, |body| body.overflow_y_scroll())
                        .child(content),
                ),
        )
}

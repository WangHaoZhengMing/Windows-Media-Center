use gpui_kit::{prelude::*, *};
use wmc_assets::{COMMAND_BUTTON, COMMAND_BUTTON_IDLE};
use wmc_design::theme::{Layout, MUTED, WHITE, color};

/// BrowsePage.MCML: a separate, right-aligned, light-font page title.
pub(crate) fn page_title(title: &'static str, layout: Layout) -> Div {
    div()
        .absolute()
        .right(layout.unit(53.))
        .top(layout.unit(46.))
        .text_size(layout.unit(48.))
        .font_weight(FontWeight::LIGHT)
        .text_color(color(0x1c6599).opacity(0.55))
        .child(title)
}

/// Options XML uses CommandContentLayout: area label, commands, right content.
pub(crate) fn settings_shell(title: &'static str, subtitle: &'static str, layout: Layout) -> Div {
    div()
        .absolute()
        .size_full()
        .child(page_title(title, layout))
        .child(
            div()
                .absolute()
                .left(layout.unit(170.))
                .top(layout.unit(157.))
                .text_size(layout.unit(29.))
                .font_weight(FontWeight::LIGHT)
                .text_color(color(MUTED).opacity(0.75))
                .child("设置"),
        )
        .child(
            div()
                .absolute()
                .left(layout.unit(510.))
                .top(layout.unit(157.))
                .text_size(layout.unit(24.))
                .text_color(color(WHITE))
                .child(subtitle),
        )
}

pub(crate) fn command_button(
    id: &'static str,
    title: &'static str,
    chosen: bool,
    available: bool,
    layout: Layout,
) -> gpui_kit::base::Button {
    wmc_design::button(id, title)
        .disabled(!available)
        .relative()
        .w(layout.unit(235.))
        .h(layout.unit(43.))
        .flex()
        .items_center()
        .justify_start()
        .px(layout.unit(18.))
        .when(available, |button| {
            button.cursor_pointer().hover(|style| style.opacity(1.))
        })
        .opacity(if available { 0.9 } else { 0.4 })
        .focus_visible(|style| style.opacity(1.))
        .child(
            img(if chosen {
                COMMAND_BUTTON
            } else {
                COMMAND_BUTTON_IDLE
            })
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .object_fit(ObjectFit::Fill),
        )
        .child(
            div()
                .relative()
                .text_size(layout.unit(19.))
                .text_color(color(if chosen { WHITE } else { MUTED }))
                .child(title),
        )
}

pub(crate) fn commands(layout: Layout) -> Div {
    div()
        .absolute()
        .left(layout.unit(166.))
        .top(layout.unit(224.))
        .w(layout.unit(245.))
        .flex()
        .flex_col()
        .gap(layout.unit(8.))
}

pub(crate) fn content_column(layout: Layout) -> Div {
    div()
        .absolute()
        .left(layout.unit(510.))
        .right(layout.unit(105.))
        .top(layout.unit(228.))
        .bottom(layout.unit(105.))
        .flex()
        .flex_col()
        .gap(layout.unit(18.))
}

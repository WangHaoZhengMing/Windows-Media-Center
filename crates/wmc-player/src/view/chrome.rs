//! Standalone window buttons and navigation chrome.
use super::{PlayerView, button::button};
use gpui_kit::{prelude::*, *};
use wmc_assets::Icon;

impl PlayerView {
    pub(super) fn chrome(&self, scale: f32, window: &Window, cx: &mut Context<Self>) -> Div {
        let zoom = if window.is_maximized() || window.is_fullscreen() {
            Icon::Restore
        } else {
            Icon::Maximize
        };
        div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .h(px(48. * scale))
            .flex()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10. * scale))
                    .ml(px(8. * scale))
                    .mt(px(7. * scale))
                    .child(
                        button("player-back", Icon::Back, 42., 42., scale)
                            .on_click(cx.listener(|this, _, _, cx| this.back(cx))),
                    )
                    .child(button("player-home", Icon::Home, 40., 40., scale).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.back(cx);
                        }),
                    )),
            )
            .child(
                div()
                    .id("player-title-drag")
                    .flex_1()
                    .h_full()
                    .window_control_area(WindowControlArea::Drag),
            )
            .child(
                div()
                    .flex()
                    .mt(px(4. * scale))
                    .mr(px(4. * scale))
                    .child(
                        button("player-minimize", Icon::Minimize, 35., 28., scale)
                            .window_control_area(WindowControlArea::Min)
                            .on_click(|_, window, _| window.minimize_window()),
                    )
                    .child(
                        button("player-maximize", zoom, 35., 28., scale)
                            .window_control_area(WindowControlArea::Max)
                            .on_click(|_, window, _| {
                                if window.is_fullscreen() {
                                    window.toggle_fullscreen()
                                } else {
                                    window.zoom_window()
                                }
                            }),
                    )
                    .child(
                        button("player-close", Icon::Close, 35., 28., scale)
                            .window_control_area(WindowControlArea::Close)
                            .on_click(|_, _, cx| cx.quit()),
                    ),
            )
    }
}

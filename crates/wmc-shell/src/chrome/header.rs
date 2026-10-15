use crate::shell::AppShell;
use gpui_kit::{prelude::*, *};
use wmc_assets::Icon;
use wmc_design::{components::resource_button, theme::Layout};

impl AppShell {
    pub(crate) fn header(&self, layout: Layout, window: &Window, cx: &mut Context<Self>) -> Div {
        let can_back = self.home_page.navigation.opened().is_some()
            || self.queue_visible
            || self.player_page.visible
            || self.photo_page.path.is_some();
        let maximize = if window.is_maximized() || window.is_fullscreen() {
            Icon::Restore
        } else {
            Icon::Maximize
        };
        div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .h(layout.unit(48.))
            .flex()
            .items_start()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(layout.unit(10.))
                    .ml(layout.unit(8.))
                    .mt(layout.unit(7.))
                    .child(
                        resource_button(
                            "back-header",
                            Icon::Back,
                            42.,
                            42.,
                            can_back,
                            layout.scale,
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.back(cx);
                            window.focus(&this.focus, cx);
                        })),
                    )
                    .child(
                        resource_button("home-orb", Icon::Home, 40., 40., true, layout.scale)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.back(cx);
                                window.focus(&this.focus, cx);
                            })),
                    ),
            )
            // Only the empty strip is draggable; buttons keep their own hit targets.
            .child(
                div()
                    .id("title-drag-region")
                    .flex_1()
                    .h_full()
                    .window_control_area(WindowControlArea::Drag),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .mt(layout.unit(4.))
                    .mr(layout.unit(4.))
                    .child(
                        resource_button(
                            "window-minimize",
                            Icon::Minimize,
                            35.,
                            28.,
                            true,
                            layout.scale,
                        )
                        .window_control_area(WindowControlArea::Min)
                        .on_click(|_, window, _| window.minimize_window()),
                    )
                    .child(
                        resource_button("window-maximize", maximize, 35., 28., true, layout.scale)
                            .window_control_area(WindowControlArea::Max)
                            .on_click(|_, window, _| {
                                if window.is_fullscreen() {
                                    window.toggle_fullscreen();
                                } else {
                                    window.zoom_window();
                                }
                            }),
                    )
                    .child(
                        resource_button("window-close", Icon::Close, 35., 28., true, layout.scale)
                            .window_control_area(WindowControlArea::Close)
                            .on_click(|_, _, cx| cx.quit()),
                    ),
            )
    }
}

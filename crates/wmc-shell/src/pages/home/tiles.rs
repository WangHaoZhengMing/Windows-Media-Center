use crate::navigation::CATEGORIES;
use crate::shell::AppShell;
use gpui_kit::{
    base::motion::{Transition, transition},
    prelude::*,
    *,
};
use std::time::Duration;
use wmc_motion::{
    HomeArrival, MenuSample,
    profile::{QUICK_LINK_DELAY_MS, QUICK_LINK_MOVE_MS, THUMB_ALPHA_MS, s_curve},
};
use {
    wmc_design::components::{focus_frame, shortcut},
    wmc_design::theme::{Layout, MUTED, SHORTCUT_HEIGHT, SHORTCUT_WIDTH, WHITE, color},
};

impl AppShell {
    pub(crate) fn tiles(
        &self,
        layout: Layout,
        motion: &MenuSample<{ CATEGORIES.len() }>,
        arrival: &HomeArrival,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let link_layout = Layout {
            scale: layout.scale * arrival.links.scale[0],
            ..layout
        };
        let mut viewport = div()
            .absolute()
            .left_0()
            .top(px(layout.content_top() + 66. * layout.scale))
            .w_full()
            .h(layout.unit(220.))
            .overflow_hidden();
        for (category_index, category) in CATEGORIES.iter().enumerate() {
            let pose = motion.rows[category_index];
            if pose.strip_opacity < 0.001 {
                continue;
            }
            let selected = self
                .home_page
                .navigation
                .selection_for_category(category_index)
                .unwrap_or(0);
            let position = transition(
                SharedString::from(format!("shortcut-position-{category_index}")),
                selected as f32,
                Transition::new(Duration::from_millis(
                    QUICK_LINK_MOVE_MS - QUICK_LINK_DELAY_MS,
                ))
                .delay(Duration::from_millis(QUICK_LINK_DELAY_MS))
                .ease(s_curve),
                window,
                cx,
            );
            let mut row = div()
                .id(("shortcut-strip", category_index))
                .absolute()
                .left_0()
                .top(layout.unit(pose.row_y + 2.))
                .w_full()
                .h(layout.unit(190.))
                .opacity(pose.strip_opacity * arrival.links.opacity);
            for (index, entry) in category.entries.iter().enumerate() {
                let focus = transition(
                    SharedString::from(format!("tile-{category_index}-{index}")),
                    if index == selected { 1.0_f32 } else { 0.0 },
                    Transition::new(Duration::from_millis(THUMB_ALPHA_MS)),
                    window,
                    cx,
                );
                row = row.child(
                    div()
                        .id(("tile", index))
                        .absolute()
                        .left(
                            layout.unit(210.) + link_layout.unit((index as f32 - position) * 235.),
                        )
                        .top_0()
                        .w(link_layout.unit(SHORTCUT_WIDTH))
                        .h(link_layout.unit(190.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .cursor_pointer()
                        .opacity(0.56 + focus * 0.44)
                        .hover(|style| style.opacity(1.))
                        .active(|style| style.opacity(0.75))
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                                this.home_page.navigation.select_category(category_index);
                                this.home_page.navigation.select_entry(index);
                                this.show_context_menu(None, event.position, window, cx);
                            }),
                        )
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if this.home_page.navigation.category_index() != category_index {
                                return;
                            }
                            this.home_page.navigation.select_entry(index);
                            this.open(window, cx);
                            window.focus(&this.focus, cx);
                        }))
                        .child(shortcut(entry.artwork, focus, link_layout))
                        .child(
                            div()
                                .mt(link_layout.unit(10.))
                                .text_size(link_layout.unit(18. + focus * 7.))
                                .font_weight(FontWeight::LIGHT)
                                .text_color(color(if index == selected { WHITE } else { MUTED }))
                                .child(entry.title),
                        ),
                );
            }
            viewport = viewport.child(row);
        }
        viewport.child(
            div()
                .absolute()
                .left(layout.unit(210. + SHORTCUT_WIDTH * (1. - arrival.selector.scale[0]) / 2.))
                .top(layout.unit(2. + SHORTCUT_HEIGHT * (1. - arrival.selector.scale[1]) / 2.))
                .opacity(arrival.selector.opacity)
                .child(focus_frame(Layout {
                    scale: layout.scale * arrival.selector.scale[0],
                    ..layout
                })),
        )
    }
}

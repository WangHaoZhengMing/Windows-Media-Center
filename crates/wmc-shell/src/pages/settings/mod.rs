mod about;
mod media_locations;
use crate::navigation::DestinationKind;
use crate::{
    pages::browse::{command_button, commands, content_column, settings_shell},
    shell::AppShell,
};
use gpui_kit::{prelude::*, *};
use wmc_assets::{CHECKED, UNCHECKED};
use wmc_design::theme::{Layout, MUTED, WHITE, color};

impl AppShell {
    pub(crate) fn settings_page(&self, layout: Layout, cx: &mut Context<Self>) -> Div {
        match self.destination_state.kind {
            DestinationKind::Appearance => self.appearance_page(layout, cx),
            DestinationKind::MediaLocations => self.media_locations_page(layout, cx),
            _ => self.about_page(layout, cx),
        }
    }

    fn appearance_page(&self, layout: Layout, cx: &mut Context<Self>) -> Div {
        let row = self.destination_state.row;
        settings_shell("常规", "显示与动效", layout)
            .child(
                commands(layout)
                    .child(
                        command_button("appearance-save", "保存", row == 1, true, layout).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.destination_state.row = 1;
                                this.activate_destination(cx);
                            }),
                        ),
                    )
                    .child(
                        command_button("appearance-cancel", "取消", row == 2, true, layout)
                            .on_click(cx.listener(|this, _, _, cx| this.back(cx))),
                    ),
            )
            .child(
                content_column(layout)
                    .child(
                        div()
                            .id("ambient-toggle")
                            .flex()
                            .items_center()
                            .gap(layout.unit(16.))
                            .cursor_pointer()
                            .text_size(layout.unit(21.))
                            .text_color(color(WHITE))
                            .hover(|style| style.text_color(color(0x9cdef8)))
                            .child(
                                img(if self.destination_state.draft_ambient {
                                    CHECKED
                                } else {
                                    UNCHECKED
                                })
                                .size(layout.unit(32.))
                                .object_fit(ObjectFit::Contain),
                            )
                            .child("启用背景流光")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.destination_state.row = 0;
                                this.activate_destination(cx);
                            })),
                    )
                    .child(
                        div()
                            .text_size(layout.unit(16.))
                            .text_color(color(MUTED))
                            .child("关闭后使用静态蓝色背景。"),
                    )
                    .child(
                        div()
                            .mt(layout.unit(24.))
                            .h(layout.unit(1.))
                            .w_full()
                            .bg(color(MUTED).opacity(0.22)),
                    )
                    .child(
                        div()
                            .text_size(layout.unit(19.))
                            .text_color(color(WHITE))
                            .child("界面动画"),
                    )
                    .child(
                        div()
                            .text_size(layout.unit(16.))
                            .text_color(color(MUTED))
                            .child("跟随系统的减少动画设置。"),
                    ),
            )
    }
}

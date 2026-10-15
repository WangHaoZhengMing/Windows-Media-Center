use crate::{
    pages::browse::{command_button, commands, content_column, settings_shell},
    shell::AppShell,
};
use gpui_kit::{prelude::*, *};
use wmc_design::theme::{Layout, MUTED, WHITE, color};
use wmc_library::{LibraryConfig, MediaKind};

impl AppShell {
    pub(crate) fn media_locations_page(&self, layout: Layout, cx: &mut Context<Self>) -> Div {
        let kind = MediaKind::ALL[self.destination_state.row.min(2)];
        let mut tabs = div().flex().gap(layout.unit(24.));
        for (index, category) in MediaKind::ALL.into_iter().enumerate() {
            tabs = tabs.child(
                div()
                    .id(("location-kind", index))
                    .text_size(layout.unit(22.))
                    .text_color(color(if category == kind { WHITE } else { MUTED }))
                    .cursor_pointer()
                    .child(category.label())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.destination_state.row = index;
                        cx.notify();
                    })),
            );
        }
        let mut roots = div()
            .id("configured-folders")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(layout.unit(16.));
        for (index, path) in self.library.draft.roots(kind).iter().enumerate() {
            roots = roots.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(layout.unit(8.))
                    .p(layout.unit(12.))
                    .border_1()
                    .border_color(color(MUTED).opacity(0.25))
                    .child(
                        div()
                            .text_size(layout.unit(16.))
                            .text_color(color(WHITE))
                            .child(path.display().to_string()),
                    )
                    .child(
                        div()
                            .id(("remove-folder", index))
                            .text_size(layout.unit(15.))
                            .text_color(color(0x9cdef8))
                            .cursor_pointer()
                            .child("移出媒体库")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if index < this.library.draft.roots(kind).len() {
                                    this.library.draft.roots_mut(kind).remove(index);
                                }
                                cx.notify();
                            })),
                    ),
            );
        }
        if self.library.draft.roots(kind).is_empty() {
            roots = roots.child(
                div()
                    .text_color(color(MUTED))
                    .text_size(layout.unit(17.))
                    .child("未配置文件夹，此媒体库不会扫描任何目录。"),
            );
        }
        let buttons = commands(layout)
            .child(
                command_button("folders-save", "保存", true, true, layout)
                    .on_click(cx.listener(|this, _, _, cx| this.save_media_folders(cx))),
            )
            .child(
                command_button("folders-cancel", "取消", false, true, layout)
                    .on_click(cx.listener(|this, _, _, cx| this.back(cx))),
            )
            .child(
                command_button("folders-add", "添加文件夹", false, true, layout)
                    .on_click(cx.listener(move |this, _, _, cx| this.add_media_folder(kind, cx))),
            )
            .child(
                command_button("folders-default", "恢复系统默认", false, true, layout).on_click(
                    cx.listener(move |this, _, _, cx| {
                        *this.library.draft.roots_mut(kind) =
                            LibraryConfig::default().roots(kind).to_vec();
                        cx.notify();
                    }),
                ),
            );
        settings_shell("媒体库", "媒体位置", layout)
            .child(buttons)
            .child(
                content_column(layout)
                    .child(tabs)
                    .child(
                        div()
                            .text_size(layout.unit(14.))
                            .text_color(color(MUTED))
                            .child(
                                "包含所选目录的子目录。移出媒体库不会删除文件；保存后重新扫描。",
                            ),
                    )
                    .child(roots)
                    .when(self.library.error.is_some(), |content| {
                        content.child(
                            div()
                                .text_size(layout.unit(14.))
                                .text_color(color(0xffcf9e))
                                .child(self.library.error.clone().unwrap_or_default()),
                        )
                    }),
            )
    }
}

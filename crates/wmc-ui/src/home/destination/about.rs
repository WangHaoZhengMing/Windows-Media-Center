use super::{HomeView, shell::command_button};
use crate::theme::{Layout, MUTED, WHITE, color};
use gpui_kit::{prelude::*, *};
use wmc_assets::Icon;

impl HomeView {
    /// OPTIONS.GLOBAL.ABOUT.XML: Done at left; logo, version paragraphs,
    /// project information and runtime details in a compact right column.
    pub(super) fn about_page(&self, layout: Layout, cx: &mut Context<Self>) -> Div {
        let platform = match std::env::consts::OS {
            "windows" => "Windows",
            "macos" => "macOS",
            "linux" => "Linux",
            other => other,
        };
        let processors = std::thread::available_parallelism()
            .map(|count| count.to_string())
            .unwrap_or_else(|_| "未知".into());
        let version = format!("版本：{}", env!("CARGO_PKG_VERSION"));
        div()
            .absolute()
            .size_full()
            .child(
                div()
                    .absolute()
                    .left(layout.unit(170.))
                    .top(layout.unit(157.))
                    .text_size(layout.unit(29.))
                    .font_weight(FontWeight::LIGHT)
                    .text_color(color(MUTED).opacity(0.65))
                    .child("设置"),
            )
            .child(
                div()
                    .absolute()
                    .left(layout.unit(510.))
                    .top(layout.unit(157.))
                    .text_size(layout.unit(24.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(color(WHITE).opacity(0.9))
                    .child("软件版本"),
            )
            .child(
                div()
                    .absolute()
                    .left(layout.unit(192.))
                    .top(layout.unit(224.))
                    .child(
                        command_button("about-done", "完成", true, true, layout)
                            .w(layout.unit(260.))
                            .on_click(cx.listener(|this, _, _, cx| this.back(cx))),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .left(layout.unit(510.))
                    .right(layout.unit(85.))
                    .top(layout.unit(211.))
                    .bottom(layout.unit(95.))
                    .flex()
                    .flex_col()
                    .gap(layout.unit(22.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(layout.unit(10.))
                            .child(
                                img(Icon::Home.path())
                                    .size(layout.unit(60.))
                                    .object_fit(ObjectFit::Contain),
                            )
                            .child(
                                div()
                                    .font_family("Segoe UI")
                                    .font_weight(FontWeight::LIGHT)
                                    .text_size(layout.unit(38.))
                                    .text_color(color(WHITE))
                                    .child("WMC Media Center"),
                            ),
                    )
                    .child(info_lines(
                        layout,
                        [
                            platform.to_owned(),
                            version,
                            "Rust 重写版 · GPUI Kit 0.7.1".into(),
                            "界面资源：Windows Media Center".into(),
                        ],
                    ))
                    .child(info_lines(
                        layout,
                        [
                            "用 Rust 重建熟悉的客厅媒体体验。".to_owned(),
                            "照片、视频与音乐。".to_owned(),
                        ],
                    ))
                    .child(info_lines(
                        layout,
                        [
                            format!("平台：{platform} / {}", std::env::consts::ARCH),
                            format!("可用逻辑处理器：{processors}"),
                        ],
                    )),
            )
    }
}

fn info_lines<const N: usize>(layout: Layout, lines: [String; N]) -> Div {
    let mut paragraph = div()
        .flex()
        .flex_col()
        .gap(layout.unit(1.))
        .text_size(layout.unit(19.))
        .line_height(layout.unit(23.))
        .text_color(color(0xd4e4f0).opacity(0.85));
    for line in lines {
        paragraph = paragraph.child(line);
    }
    paragraph
}

use super::HomeView;
use crate::{
    components::{artwork, background},
    theme::{Layout, MUTED, WHITE, color},
};
use chrono::{DateTime, FixedOffset, Utc};
use gpui_kit::{prelude::*, *};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use wmc_core::{Direction, LibraryKind};
use wmc_library::MediaKind;
use wmc_player::metadata::properties::AudioProperties;

pub(super) struct DetailsState {
    pub(super) path: PathBuf,
    title: String,
    artist: Option<String>,
    album: Option<String>,
    pub(super) kind: MediaKind,
    cover: Option<Arc<RenderImage>>,
    opened: Instant,
    closing: Option<(Instant, f32)>,
    facts: Option<FileFacts>,
    copied: bool,
    focused: usize,
}

impl DetailsState {
    fn alpha(&self, reduced: bool) -> f32 {
        if let Some((epoch, from)) = self.closing {
            if reduced {
                0.
            } else {
                from * (1. - (epoch.elapsed().as_secs_f32() / 0.18).clamp(0., 1.))
            }
        } else if reduced {
            1.
        } else {
            1. - (1. - (self.opened.elapsed().as_secs_f32() / 0.22).clamp(0., 1.)).powi(3)
        }
    }
}

#[derive(Default)]
struct FileFacts {
    size: Option<u64>,
    modified: Option<String>,
    dimensions: Option<(u32, u32)>,
    audio: Option<AudioProperties>,
    available: bool,
    warning: Option<String>,
}

impl FileFacts {
    fn read(path: &Path, kind: MediaKind) -> Self {
        let metadata = match std::fs::metadata(path) {
            Ok(metadata) if metadata.is_file() => metadata,
            Ok(_) => {
                return Self {
                    warning: Some("此路径已不是媒体文件。".into()),
                    ..Default::default()
                };
            }
            Err(error) => {
                return Self {
                    warning: Some(format!("无法访问此文件：{error}")),
                    ..Default::default()
                };
            }
        };
        let mut facts = Self {
            size: Some(metadata.len()),
            modified: metadata.modified().ok().map(|time| {
                let date: DateTime<Utc> = time.into();
                date.with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap())
                    .format("%Y-%m-%d  %H:%M")
                    .to_string()
            }),
            available: true,
            ..Default::default()
        };
        match kind {
            MediaKind::Music => match AudioProperties::read(path) {
                Ok(audio) => facts.audio = Some(audio),
                Err(_) => facts.warning = Some("音频属性暂时无法读取，文件信息仍可查看。".into()),
            },
            MediaKind::Photos => match image::image_dimensions(path) {
                Ok(dimensions) => facts.dimensions = Some(dimensions),
                Err(_) => facts.warning = Some("图片尺寸暂时无法读取，文件信息仍可查看。".into()),
            },
            MediaKind::Videos => {}
        }
        facts
    }
}

impl HomeView {
    pub(super) fn show_details(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(item) = self.library.items.get(index) else {
            return;
        };
        let path = item.file.path.clone();
        let kind = item.file.kind;
        let opened = Instant::now();
        self.details = Some(DetailsState {
            path: path.clone(),
            title: item.title.clone(),
            artist: item.artist.clone(),
            album: item.album.clone(),
            kind,
            cover: item.cover.clone(),
            opened,
            closing: None,
            facts: None,
            copied: false,
            focused: 0,
        });
        cx.spawn(async move |this, cx| {
            let facts = cx
                .background_executor()
                .spawn(async move { FileFacts::read(&path, kind) })
                .await;
            let _ = this.update(cx, |this, cx| {
                // A dismissed/reopened panel must never accept a previous file's result.
                if let Some(details) = &mut this.details
                    && details.opened == opened
                {
                    details.facts = Some(facts);
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn close_details(&mut self, cx: &mut Context<Self>) {
        if let Some(details) = &mut self.details
            && details.closing.is_none()
        {
            details.closing = Some((Instant::now(), details.alpha(cx.reduce_motion())));
            cx.notify();
        }
    }

    pub(super) fn advance_details(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(details) = &self.details else {
            return;
        };
        if let Some((epoch, _)) = details.closing {
            if cx.reduce_motion() || epoch.elapsed() >= Duration::from_millis(180) {
                self.details = None;
            } else {
                window.request_animation_frame();
            }
        } else if !cx.reduce_motion() && details.opened.elapsed() < Duration::from_millis(220) {
            window.request_animation_frame();
        }
    }

    pub(super) fn navigate_details(
        &mut self,
        direction: Direction,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(details) = &mut self.details else {
            return false;
        };
        if details.closing.is_some() {
            return true;
        }
        details.focused = match direction {
            Direction::Right | Direction::Down => (details.focused + 1) % 3,
            Direction::Left | Direction::Up => (details.focused + 2) % 3,
        };
        cx.notify();
        true
    }

    pub(super) fn activate_details(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(details) = &self.details else {
            return;
        };
        self.details_action(details.focused, window, cx);
    }

    fn details_action(&mut self, action: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(details) = &mut self.details else {
            return;
        };
        if details.closing.is_some() {
            return;
        }
        match action {
            0 => {
                if !details.facts.as_ref().is_some_and(|facts| facts.available) {
                    return;
                }
                let path = details.path.clone();
                if let Some(index) = self
                    .library
                    .items
                    .iter()
                    .position(|item| item.file.path == path)
                {
                    self.details = None;
                    self.open_media(index, window, cx);
                } else {
                    if let Some(facts) = &mut details.facts {
                        facts.available = false;
                        facts.warning = Some("此文件已不在媒体库中，请重新扫描。".into());
                    }
                    cx.notify();
                }
            }
            1 => {
                cx.write_to_clipboard(ClipboardItem::new_string(
                    details.path.to_string_lossy().into_owned(),
                ));
                details.copied = true;
                details.focused = 1;
                cx.notify();
            }
            _ => self.close_details(cx),
        }
    }

    pub(super) fn details_overlay(
        &self,
        layout: Layout,
        _window: &Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let details = self.details.as_ref().unwrap();
        let alpha = details.alpha(cx.reduce_motion());
        let facts = details.facts.as_ref();
        let format = details
            .path
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_uppercase)
            .unwrap_or_else(|| "未知格式".into());
        let size = facts
            .and_then(|facts| facts.size)
            .map(format_size)
            .unwrap_or_else(|| {
                if facts.is_none() {
                    "读取中…"
                } else {
                    "未提供"
                }
                .into()
            });
        let (_measure_label, measure) = match details.kind {
            MediaKind::Music => (
                "时长",
                facts
                    .and_then(|facts| facts.audio.as_ref())
                    .map(|audio| format_duration(audio.duration))
                    .unwrap_or_else(|| {
                        if facts.is_none() {
                            "读取中…"
                        } else {
                            "未提供"
                        }
                        .into()
                    }),
            ),
            MediaKind::Photos => (
                "尺寸",
                facts
                    .and_then(|facts| facts.dimensions)
                    .map(|(w, h)| format!("{w} × {h}"))
                    .unwrap_or_else(|| {
                        if facts.is_none() {
                            "读取中…"
                        } else {
                            "未提供"
                        }
                        .into()
                    }),
            ),
            MediaKind::Videos => ("媒体类型", "视频".into()),
        };
        let cover = if details.kind == MediaKind::Photos {
            if let Some(image) = self.photo_image(&details.path, super::photos::DETAIL, cx) {
                img(image)
                    .size_full()
                    .object_fit(ObjectFit::Cover)
                    .into_any_element()
            } else {
                artwork(LibraryKind::Photos, layout.unit(256.)).into_any_element()
            }
        } else if let Some(cover) = &details.cover {
            img(cover.clone())
                .size_full()
                .object_fit(ObjectFit::Cover)
                .into_any_element()
        } else {
            artwork(
                if details.kind == MediaKind::Music {
                    LibraryKind::Music
                } else {
                    LibraryKind::Videos
                },
                layout.unit(256.),
            )
            .into_any_element()
        };
        let file_name = details
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| details.title.clone());
        let mut attributes = div()
            .id("details-attributes")
            .absolute()
            .left(layout.unit(510.))
            .right(layout.unit(100.))
            .top(layout.unit(342.))
            .bottom(layout.unit(162.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(layout.unit(18.));
        if details.kind == MediaKind::Music {
            attributes = attributes
                .child(detail_row(
                    "艺术家",
                    details.artist.as_deref().unwrap_or("未提供"),
                    layout,
                ))
                .child(detail_row(
                    "专辑",
                    details.album.as_deref().unwrap_or("未提供"),
                    layout,
                ));
            if let Some(audio) = facts.and_then(|facts| facts.audio.as_ref()) {
                if let Some(rate) = audio.sample_rate {
                    attributes = attributes.child(detail_row(
                        "采样率",
                        format!("{} kHz", f64::from(rate) / 1000.),
                        layout,
                    ));
                }
                if let Some(channels) = audio.channels {
                    attributes = attributes.child(detail_row(
                        "声道",
                        match channels {
                            1 => "单声道".into(),
                            2 => "立体声".into(),
                            count => format!("{count} 声道"),
                        },
                        layout,
                    ));
                }
                if let Some(bitrate) = audio.bitrate {
                    attributes =
                        attributes.child(detail_row("码率", format!("{bitrate} kbps"), layout));
                }
                if let Some(bits) = audio.bit_depth {
                    attributes =
                        attributes.child(detail_row("位深", format!("{bits} bit"), layout));
                }
            }
        }
        attributes = attributes
            .child(detail_row("文件名", wrap_name(&file_name), layout))
            .child(detail_row(
                "修改时间",
                facts
                    .and_then(|facts| facts.modified.as_deref())
                    .unwrap_or(if facts.is_none() {
                        "读取中…"
                    } else {
                        "未提供"
                    }),
                layout,
            ));
        if let Some(warning) = facts.and_then(|facts| facts.warning.as_ref()) {
            attributes = attributes.child(
                div()
                    .text_size(layout.unit(13.))
                    .text_color(color(0xffce92))
                    .child(warning.clone()),
            );
        }
        let mut commands = div()
            .absolute()
            .left(layout.unit(170.))
            .top(layout.unit(498.))
            .w(layout.unit(276.))
            .flex()
            .flex_col()
            .gap(layout.unit(8.));
        for (action, label) in [
            (
                0,
                if details.kind == MediaKind::Photos {
                    "查看照片"
                } else {
                    "播放"
                },
            ),
            (
                1,
                if details.copied {
                    "路径已复制"
                } else {
                    "复制文件路径"
                },
            ),
            (2, "返回媒体库"),
        ] {
            let enabled = action != 0 || facts.is_some_and(|facts| facts.available);
            commands = commands.child(
                detail_button(action, label, details.focused == action, enabled, layout).on_click(
                    cx.listener(move |this, _, window, cx| this.details_action(action, window, cx)),
                ),
            );
        }
        let summary = format!("{format}  ·  {measure}  ·  {size}");
        let panel = div()
            .id("media-details-panel")
            .absolute()
            .size_full()
            .occlude()
            .opacity(alpha)
            .font_family("Microsoft YaHei UI")
            .text_color(color(WHITE))
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .child(background(layout, false))
            .child(
                div()
                    .absolute()
                    .size_full()
                    .top(layout.unit(14. * (1. - alpha)))
                    .child(
                        div()
                            .absolute()
                            .right(layout.unit(53.))
                            .top(layout.unit(46.))
                            .font_weight(FontWeight::LIGHT)
                            .text_size(layout.unit(48.))
                            .text_color(color(0x1c6599).opacity(0.55))
                            .child("详细信息"),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(layout.unit(170.))
                            .top(layout.unit(151.))
                            .text_size(layout.unit(22.))
                            .text_color(color(0x6fafd2))
                            .child(format!("{}  /  详细信息", details.kind.label())),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(layout.unit(184.))
                            .top(layout.unit(221.))
                            .size(layout.unit(256.))
                            .overflow_hidden()
                            .border_1()
                            .border_color(color(WHITE).opacity(0.22))
                            .bg(color(0x091d3c))
                            .child(cover),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(layout.unit(510.))
                            .right(layout.unit(100.))
                            .top(layout.unit(204.))
                            .flex()
                            .flex_col()
                            .gap(layout.unit(14.))
                            .child(
                                div()
                                    .id("details-title")
                                    .max_h(layout.unit(88.))
                                    .overflow_y_scroll()
                                    .text_size(layout.unit(if details.title.chars().count() > 32 {
                                        24.
                                    } else {
                                        42.
                                    }))
                                    .whitespace_normal()
                                    .font_weight(FontWeight::LIGHT)
                                    .child(wrap_name(&details.title)),
                            )
                            .child(
                                div()
                                    .text_size(layout.unit(19.))
                                    .text_color(color(0x96ccea))
                                    .child(summary),
                            ),
                    )
                    .child(attributes)
                    .child(
                        div()
                            .absolute()
                            .left(layout.unit(510.))
                            .right(layout.unit(100.))
                            .bottom(layout.unit(65.))
                            .flex()
                            .flex_col()
                            .gap(layout.unit(9.))
                            .child(
                                div()
                                    .text_size(layout.unit(16.))
                                    .text_color(color(MUTED))
                                    .child("文件位置"),
                            )
                            .child(
                                div()
                                    .max_h(layout.unit(65.))
                                    .overflow_hidden()
                                    .text_size(layout.unit(17.))
                                    .text_color(color(0xa3c3df))
                                    .child(
                                        details
                                            .path
                                            .to_string_lossy()
                                            .replace('\\', "\\\u{200b}")
                                            .replace('/', "/\u{200b}"),
                                    ),
                            ),
                    )
                    .child(commands)
                    .child(
                        div()
                            .absolute()
                            .left(layout.unit(184.))
                            .bottom(layout.unit(26.))
                            .text_size(layout.unit(15.))
                            .text_color(color(MUTED).opacity(0.8))
                            .child("Esc 返回"),
                    ),
            );
        div().absolute().size_full().child(panel)
    }
}

fn wrap_name(value: &str) -> String {
    // Permit breaks inside long hash/file-name runs without changing copied paths.
    value
        .chars()
        .enumerate()
        .flat_map(|(index, ch)| {
            if index > 0 && index % 12 == 0 {
                vec!['\u{200b}', ch]
            } else {
                vec![ch]
            }
        })
        .collect()
}

fn detail_row(label: &str, value: impl Into<SharedString>, layout: Layout) -> Div {
    div()
        .flex()
        .gap(layout.unit(22.))
        .items_start()
        .child(
            div()
                .w(layout.unit(146.))
                .flex_shrink_0()
                .text_size(layout.unit(17.))
                .text_color(color(MUTED))
                .child(label.to_owned()),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .whitespace_normal()
                .text_size(layout.unit(20.))
                .text_color(color(WHITE).opacity(0.92))
                .child(value.into()),
        )
}

fn detail_button(
    action: usize,
    label: &str,
    focused: bool,
    enabled: bool,
    layout: Layout,
) -> Stateful<Div> {
    div()
        .id(("details-action", action))
        .relative()
        .w(layout.unit(276.))
        .h(layout.unit(45.))
        .flex()
        .items_center()
        .px(layout.unit(22.))
        .opacity(if enabled { 1. } else { 0.35 })
        .when(enabled, |button| {
            button
                .cursor_pointer()
                .hover(|style| style.text_color(color(WHITE)))
        })
        .child(
            img(wmc_assets::COMMAND_BUTTON)
                .absolute()
                .size_full()
                .object_fit(ObjectFit::Fill)
                .opacity(if focused { 1. } else { 0. }),
        )
        .child(
            div()
                .relative()
                .text_size(layout.unit(21.))
                .text_color(color(if focused { WHITE } else { 0x85c5e8 }))
                .child(label.to_owned()),
        )
}

fn format_size(bytes: u64) -> String {
    if bytes < 1000 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    for unit in ["KB", "MB", "GB", "TB"] {
        value /= 1000.;
        if value < 1000. || unit == "TB" {
            return format!("{value:.1} {unit}");
        }
    }
    unreachable!()
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    if duration.is_zero() {
        return "未提供".into();
    }
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::{FileFacts, format_duration, format_size, wrap_name};
    use std::time::Duration;
    use wmc_library::MediaKind;

    #[test]
    fn file_sizes_and_long_durations_use_consistent_units() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(999), "999 B");
        assert_eq!(format_size(1000), "1.0 KB");
        assert_eq!(format_size(2_500_000), "2.5 MB");
        assert_eq!(format_size(1_000_000_000), "1.0 GB");
        assert_eq!(format_duration(Duration::ZERO), "未提供");
        assert_eq!(format_duration(Duration::from_secs(65)), "1:05");
        assert_eq!(format_duration(Duration::from_secs(3723)), "1:02:03");
    }

    #[test]
    fn long_names_wrap_without_losing_any_characters() {
        let name = "01013767427fe8757f0ff492280c3314e9ca0e8e44.jpg";
        let wrapped = wrap_name(name);
        assert!(wrapped.contains('\u{200b}'));
        assert_eq!(wrapped.replace('\u{200b}', ""), name);
    }

    #[test]
    fn missing_files_disable_playback_and_report_a_warning() {
        let path = std::env::temp_dir().join(format!(
            "wmc-details-missing-{}.mp3",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let facts = FileFacts::read(&path, MediaKind::Music);
        assert!(!facts.available);
        assert!(facts.size.is_none());
        assert!(facts.warning.is_some());
    }
}

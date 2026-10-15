use super::{
    HomeView,
    shell::{command_button, page_title},
};
use crate::{
    components::artwork,
    home::media_library::{PAGE_SIZE, media_kind},
    theme::{Layout, MUTED, WHITE, color},
};
use gpui_kit::{
    base::motion::{Transition, transition},
    prelude::*,
    *,
};
use std::time::Duration;
use wmc_assets::{PIVOT_LEFT, PIVOT_RIGHT, REFRESH_COMMAND};
use wmc_core::{DestinationFocus, Entry, LibraryKind};
use wmc_motion::context_menu::{mcml_exp, mcml_log};

impl HomeView {
    pub(super) fn library_page(
        &self,
        _: &'static Entry,
        kind: LibraryKind,
        layout: Layout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let returning = self.library_return.map(|started| {
            wmc_motion::player_return::PlayerReturn::sample(
                started.elapsed().as_secs_f32(),
                cx.reduce_motion(),
            )
        });
        let indices = self.library.indices(kind, self.destination_state.tab);
        let category = media_kind(kind).index();
        let page = self.library.pages[category].min(indices.len().saturating_sub(1) / PAGE_SIZE);
        let selected = self.library.selection[category];
        let mut pivots = div()
            .absolute()
            .left(layout.unit(188.))
            .right(layout.unit(244.))
            .top(layout.unit(139.))
            .h(layout.unit(43.))
            .opacity(returning.as_ref().map_or(1., |pose| pose.commands_alpha))
            .flex()
            .items_center()
            .gap(layout.unit(28. * returning.as_ref().map_or(1., |pose| pose.pivots_scale)))
            .overflow_hidden();
        for (index, &title) in kind.tabs().iter().enumerate() {
            let active = index == self.destination_state.tab;
            pivots = pivots.child(
                div()
                    .id(("library-pivot", index))
                    .flex()
                    .items_center()
                    .gap(layout.unit(8.))
                    .cursor_pointer()
                    .text_size(layout.unit(
                        (if active { 22. } else { 18. })
                            * returning.as_ref().map_or(1., |pose| pose.pivots_scale),
                    ))
                    .text_color(color(0x00b6df).opacity(if active { 1. } else { 0.8 }))
                    .when(active, |tab| {
                        tab.child(img(PIVOT_LEFT).w(layout.unit(9.)).h(layout.unit(16.)))
                    })
                    .child(title)
                    .when(active, |tab| {
                        tab.child(img(PIVOT_RIGHT).w(layout.unit(9.)).h(layout.unit(16.)))
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.destination_state.choose_tab(index);
                        this.library.selection[category] = 0;
                        this.library.pages[category] = 0;
                        this.library.hovered = None;
                        this.library.pointer_navigation = false;
                        window.focus(&this.focus, cx);
                        cx.notify();
                    })),
            );
        }
        if kind == LibraryKind::Photos {
            return self.photo_library_page(
                page_title("照片", layout),
                pivots,
                indices,
                layout,
                cx,
            );
        }
        let mut gallery = div()
            .id("media-gallery")
            .absolute()
            .left(layout.unit(202.))
            .top(layout.unit(203.))
            .bottom(layout.unit(108.))
            .w(layout.unit(844.))
            .p(layout.unit(22.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(layout.unit(20.));
        let shown: Vec<_> = indices
            .iter()
            .enumerate()
            .skip(page * PAGE_SIZE)
            .take(PAGE_SIZE)
            .collect();
        let columns = if self.library_list { 1 } else { 5 };
        let thumb_size = if self.library_list { 48. } else { 136. };
        for row in shown.chunks(columns) {
            let mut cards = div().flex().gap(layout.unit(24.));
            for &(position, &index) in row {
                let item = &self.library.items[index];
                let weak = cx.entity().downgrade();
                let keyboard_focused = self.destination_state.focus == DestinationFocus::Content
                    && selected == position;
                let focused = if self.library.pointer_navigation {
                    self.library.hovered == Some(index)
                } else {
                    keyboard_focused
                };
                let easing: fn(f32) -> f32 = if focused { hover_gain } else { hover_loss };
                let focus = transition(
                    SharedString::from(format!(
                        "library-focus-{}-{category}-{index}",
                        self.library.generation
                    )),
                    if focused { 1.0_f32 } else { 0.0 },
                    Transition::new(Duration::from_millis(100)).ease(easing),
                    window,
                    cx,
                );
                let item_scale = returning.as_ref().map_or(1., |pose| {
                    if keyboard_focused {
                        pose.selected_scale
                    } else {
                        pose.item_scale
                    }
                }) * (1. + 0.26 * focus);
                let thumb_layout = crate::theme::Layout {
                    scale: layout.scale * item_scale,
                    ..layout
                };
                let thumb = if kind == LibraryKind::Photos {
                    if let Some(image) =
                        self.photo_image(&item.file.path, super::super::photos::THUMB, cx)
                    {
                        img(image)
                            .size(thumb_layout.unit(thumb_size))
                            .object_fit(ObjectFit::Cover)
                    } else {
                        artwork(kind, thumb_layout.unit(thumb_size))
                    }
                } else if let Some(cover) = &item.cover {
                    img(cover.clone())
                        .size(thumb_layout.unit(thumb_size))
                        .object_fit(ObjectFit::Cover)
                } else {
                    artwork(kind, thumb_layout.unit(thumb_size))
                };
                cards = cards.child(
                    div()
                        .id(("media-file", index))
                        .relative()
                        .w(layout.unit(if self.library_list { 760. } else { 136. }))
                        .flex()
                        .when(!self.library_list, |card| card.flex_col())
                        .when(self.library_list, |card| card.items_center())
                        .gap(layout.unit(4.))
                        .cursor_pointer()
                        .opacity(returning.as_ref().map_or(1., |pose| pose.item_alpha))
                        .child(
                            div()
                                .size(layout.unit(thumb_size))
                                .relative()
                                .flex_shrink_0()
                                .child(
                                    div()
                                        .absolute()
                                        .left(layout.unit(thumb_size * (1. - item_scale) / 2.))
                                        .top(layout.unit(thumb_size * (1. - item_scale) / 2.))
                                        .size(thumb_layout.unit(thumb_size))
                                        .overflow_hidden()
                                        .border_1()
                                        .border_color(color(WHITE).opacity(0.2 + 0.6 * focus))
                                        .when(focus > 0.001, |thumb| thumb.shadow_md())
                                        .opacity(
                                            self.context_menu
                                                .as_ref()
                                                .filter(|menu| {
                                                    menu.target == Some(index)
                                                        && menu.anchor.is_some()
                                                })
                                                .map_or(1., |menu| {
                                                    1. - menu
                                                        .pose(cx.reduce_motion())
                                                        .contextual_alpha
                                                }),
                                        )
                                        .child(thumb.opacity(0.85 + 0.15 * focus)),
                                ),
                        )
                        .child(
                            div()
                                .h(layout.unit(if self.library_list { 40. } else { 60. }))
                                .pt(layout.unit(if self.library_list {
                                    0.
                                } else {
                                    thumb_size * 0.13 * focus
                                }))
                                .overflow_hidden()
                                .text_size(layout.unit(16. + 2. * focus))
                                .text_color(color(WHITE))
                                .child(item.title.clone()),
                        )
                        .when(item.artist.is_some(), |card| {
                            card.child(
                                div()
                                    .h(layout.unit(20.))
                                    .overflow_hidden()
                                    .text_size(layout.unit(12.))
                                    .text_color(color(MUTED))
                                    .child(item.artist.clone().unwrap_or_default()),
                            )
                        })
                        .child(
                            canvas(
                                move |bounds, _, cx| {
                                    let _ = weak.update(cx, |this, _| {
                                        this.media_bounds.insert(index, bounds);
                                    });
                                },
                                |_, _, _, _| {},
                            )
                            .absolute()
                            .top_0()
                            .left_0()
                            .size(layout.unit(thumb_size)),
                        )
                        .on_hover(cx.listener(move |this, hovered: &bool, window, cx| {
                            if *hovered {
                                if this.context_menu.is_some() || this.details.is_some() {
                                    return;
                                }
                                this.library.pointer_navigation = true;
                                this.library.hovered = Some(index);
                                this.library.selection[category] = position;
                                this.destination_state.focus = DestinationFocus::Content;
                                window.focus(&this.focus, cx);
                            } else if this.library.hovered == Some(index) {
                                this.library.hovered = None;
                            } else {
                                return;
                            }
                            cx.notify();
                        }))
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                                this.library.selection[category] = position;
                                this.destination_state.focus = DestinationFocus::Content;
                                this.show_context_menu(Some(index), event.position, window, cx);
                            }),
                        )
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.library.selection[category] = position;
                            this.destination_state.focus = DestinationFocus::Content;
                            this.open_media(index, window, cx);
                        })),
                );
            }
            gallery = gallery.child(cards);
        }
        if indices.is_empty() {
            gallery = gallery.child(
                div()
                    .text_size(layout.unit(20.))
                    .text_color(color(MUTED))
                    .child(if self.library.loading {
                        "正在扫描本地文件…"
                    } else {
                        "没有找到媒体。请在设置 → 媒体位置添加文件夹。"
                    }),
            );
        }
        let status = if self.library.loading_track {
            "正在加载歌曲…".into()
        } else if let Some(error) = &self.library.error {
            error.clone()
        } else {
            format!(
                "{} 个文件 · {} 个扫描目录{}",
                indices.len(),
                self.library.config.roots(media_kind(kind)).len(),
                if self.library.skipped > 0 {
                    format!(" · {} 项无法访问", self.library.skipped)
                } else {
                    String::new()
                }
            )
        };
        let mut pages = div()
            .absolute()
            .left(layout.unit(224.))
            .bottom(layout.unit(74.))
            .flex()
            .items_center()
            .gap(layout.unit(14.));
        for (id, label, delta) in [
            ("library-prev", "上一页", -1_isize),
            ("library-next", "下一页", 1),
        ] {
            let enabled = if delta < 0 {
                page > 0
            } else {
                (page + 1) * PAGE_SIZE < indices.len()
            };
            pages = pages.child(command_button(id, label, false, enabled, layout).on_click(
                cx.listener(move |this, _, _, cx| {
                    if !enabled {
                        return;
                    }
                    this.library.pages[category] = page.saturating_add_signed(delta);
                    this.library.selection[category] = this.library.pages[category] * PAGE_SIZE;
                    cx.notify();
                }),
            ));
        }
        pages = pages.child(
            div()
                .text_size(layout.unit(16.))
                .text_color(color(MUTED))
                .child(format!(
                    "{} / {}",
                    page + 1,
                    indices.len().div_ceil(PAGE_SIZE).max(1)
                )),
        );
        div()
            .absolute()
            .size_full()
            .child(
                page_title(kind.title(), layout)
                    .right(layout.unit(53. - returning.as_ref().map_or(0., |pose| pose.title_x)))
                    .text_size(
                        layout.unit(48. * returning.as_ref().map_or(1., |pose| pose.title_scale)),
                    )
                    .opacity(returning.as_ref().map_or(1., |pose| pose.title_alpha)),
            )
            .child(pivots)
            .child(gallery)
            .child(pages.opacity(returning.as_ref().map_or(1., |pose| pose.counter_alpha)))
            .child(
                div()
                    .id("library-refresh")
                    .absolute()
                    .right(layout.unit(190.))
                    .top(layout.unit(139.))
                    .size(layout.unit(44.))
                    .overflow_hidden()
                    .opacity(
                        returning.as_ref().map_or(1., |pose| pose.commands_alpha)
                            * if self.library.loading { 0.35 } else { 0.8 },
                    )
                    .when(!self.library.loading, |button| {
                        button.cursor_pointer().hover(|style| style.opacity(1.))
                    })
                    // Original shortcut art includes transparent padding. Position
                    // the full image so its 76px orb fills this 44px hit target.
                    .child(
                        img(REFRESH_COMMAND)
                            .absolute()
                            .left(layout.unit(-46.))
                            .top(layout.unit(-16.22))
                            .w(layout.unit(136.))
                            .h(layout.unit(76.44)),
                    )
                    .tooltip({
                        let scanning = self.library.loading;
                        move |_, cx| cx.new(|_| RefreshTooltip { scanning }).into()
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        if !this.library.loading {
                            this.rescan_library(cx);
                        }
                    })),
            )
            .child(
                div()
                    .absolute()
                    .left(layout.unit(224.))
                    .right(layout.unit(70.))
                    .bottom(layout.unit(35.))
                    .h(layout.unit(32.))
                    .overflow_hidden()
                    .text_size(layout.unit(14.))
                    .text_color(color(MUTED))
                    .child(status),
            )
    }
}

struct RefreshTooltip {
    scanning: bool,
}

impl Render for RefreshTooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(px(10.))
            .py(px(6.))
            .rounded(px(4.))
            .bg(color(0x071c35))
            .text_color(color(WHITE))
            .text_size(px(14.))
            .child(if self.scanning {
                "正在扫描…"
            } else {
                "重新扫描"
            })
    }
}

// BROWSEANIMATIONS.MCML: Thumb.Scale.Mouse uses Log over 100ms;
// Thumb.Scale.LoseFocus uses Exp over 100ms, both from the current value.
fn hover_gain(t: f32) -> f32 {
    mcml_log(t, 1.)
}
fn hover_loss(t: f32) -> f32 {
    mcml_exp(t, 1.)
}

//! A continuous, virtualized chronological grid. Only visible rows are laid out.
use super::HomeView;
use crate::{
    components::artwork,
    theme::{Layout, MUTED, WHITE, color},
};
use gpui_kit::{prelude::*, *};
use wmc_core::{DestinationFocus, LibraryKind};

const GAP: f32 = 2.;
const WIDTH: f32 = 800.;
const DEFAULT_SIZE: f32 = (WIDTH - GAP * 4.) / 5.;

pub(super) struct Grid {
    pub columns: usize,
    pub cell: f32,
    pub width: f32,
}

impl Grid {
    pub fn new(zoom: f32) -> Self {
        let cell = (DEFAULT_SIZE * zoom).clamp(64., 264.);
        let columns = (((WIDTH + GAP) / (cell + GAP) + 0.0001).floor() as usize).clamp(3, 12);
        Self {
            columns,
            cell,
            width: cell * columns as f32 + GAP * (columns - 1) as f32,
        }
    }
}

fn leading_slots(count: usize, columns: usize) -> usize {
    (columns - count % columns) % columns
}

pub(super) fn photo_row(position: usize, count: usize, columns: usize) -> usize {
    (position + leading_slots(count, columns)) / columns
}

fn row_photos(row: usize, count: usize, columns: usize) -> std::ops::Range<usize> {
    let padding = leading_slots(count, columns);
    (row * columns).saturating_sub(padding).min(count)
        ..((row + 1) * columns).saturating_sub(padding).min(count)
}

pub(super) fn photo_band(
    rows: std::ops::Range<usize>,
    count: usize,
    columns: usize,
) -> std::ops::Range<usize> {
    let padding = leading_slots(count, columns);
    (rows.start.saturating_sub(1) * columns)
        .saturating_sub(padding)
        .min(count)
        ..((rows.end + 1) * columns)
            .saturating_sub(padding)
            .min(count)
}

impl HomeView {
    fn zoom_photos(
        &mut self,
        delta: f32,
        position: Point<Pixels>,
        layout: Layout,
        cx: &mut Context<Self>,
    ) {
        if !delta.is_finite()
            || delta == 0.
            || self.context_menu.is_some()
            || self.details.is_some()
        {
            return;
        }
        let old = Grid::new(self.photo_zoom);
        let zoom = (self.photo_zoom * (1. + delta.clamp(-0.5, 0.5)))
            .clamp(64. / DEFAULT_SIZE, 264. / DEFAULT_SIZE);
        if zoom == self.photo_zoom {
            return;
        }
        let new = Grid::new(zoom);
        let indices = self
            .library
            .indices(LibraryKind::Photos, self.destination_state.tab);
        let count = indices.len();
        let bottom = self.photo_scroll.is_scrolled_to_end().unwrap_or(true);
        let handle = self.photo_scroll.0.borrow().base_handle.clone();
        let offset = handle.offset();
        let y = (f32::from(position.y) - 203. * layout.scale).max(0.);
        let old_height = (old.cell + GAP) * layout.scale;
        let content_y = (y - f32::from(offset.y)).max(0.);
        let row = (content_y / old_height).floor() as usize;
        let range = row_photos(row, count, old.columns);
        let anchor = self
            .media_bounds
            .iter()
            .find(|(_, bounds)| bounds.contains(&position))
            .and_then(|(&index, _)| indices.iter().position(|&item| item == index))
            .unwrap_or(range.start + range.len() / 2)
            .min(count.saturating_sub(1));
        self.photo_zoom = zoom;
        if bottom {
            self.photo_scroll.scroll_to_bottom();
        } else if count > 0 {
            let fraction = (content_y / old_height).fract();
            let new_y = (photo_row(anchor, count, new.columns) as f32 + fraction)
                * (new.cell + GAP)
                * layout.scale;
            handle.set_offset(point(offset.x, px((y - new_y).min(0.))));
        }
        cx.notify();
    }

    pub(super) fn photo_library_page(
        &self,
        title: Div,
        pivots: Div,
        indices: Vec<usize>,
        layout: Layout,
        cx: &mut Context<Self>,
    ) -> Div {
        let count = indices.len();
        let grid = Grid::new(self.photo_zoom);
        let columns = grid.columns;
        let cell = grid.cell;
        let rows = count.div_ceil(columns);
        let gallery = uniform_list(
            "photo-grid",
            rows,
            cx.processor(move |this, range: std::ops::Range<usize>, _window, cx| {
                let band = photo_band(range.clone(), count, columns);
                let first = range.start;
                this.media_bounds.clear();
                range
                    .map(|row| {
                        let mut cards = div()
                            .id(("photo-row", row))
                            .relative()
                            .w(layout.unit(grid.width))
                            .h(layout.unit(cell + GAP))
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(layout.unit(GAP));
                        if row == first {
                            let weak = cx.entity().downgrade();
                            let band = band.clone();
                            // uniform_list also calls this processor to measure a
                            // dummy row. Load only when real rows reach prepaint.
                            cards = cards.child(
                                canvas(
                                    move |_, window, cx| {
                                        let _ = weak.update(cx, |this, cx| {
                                            this.photo_visible = band.clone();
                                            this.sync_photos(window, cx);
                                        });
                                    },
                                    |_, _, _, _| {},
                                )
                                .absolute()
                                .top_0()
                                .left_0()
                                .size_full(),
                            );
                        }
                        let positions = row_photos(row, count, columns);
                        for (position, &index) in indices
                            .iter()
                            .enumerate()
                            .take(positions.end)
                            .skip(positions.start)
                        {
                            cards = cards.child(this.photo_card(index, position, cell, layout, cx));
                        }
                        div()
                            .id(("photo-row-host", row))
                            .w_full()
                            .h(layout.unit(cell + GAP))
                            .flex()
                            .justify_center()
                            .child(cards)
                    })
                    .collect::<Vec<_>>()
            }),
        )
        .track_scroll(&self.photo_scroll)
        .size_full()
        .px(layout.unit(22.));
        div()
            .absolute()
            .size_full()
            .child(title)
            .child(pivots)
            .child(
                div()
                    .absolute()
                    .left(layout.unit(202.))
                    .top(layout.unit(203.))
                    .bottom(layout.unit(88.))
                    .w(layout.unit(844.))
                    .on_pinch(cx.listener(move |this, event: &PinchEvent, _, cx| {
                        this.zoom_photos(event.delta, event.position, layout, cx);
                        cx.stop_propagation();
                    }))
                    .when(count == 0, |host| {
                        host.child(
                            div()
                                .p(layout.unit(22.))
                                .text_size(layout.unit(20.))
                                .text_color(color(MUTED))
                                .child(if self.library.loading {
                                    "正在扫描照片…"
                                } else {
                                    "没有找到照片。请在媒体位置添加文件夹。"
                                }),
                        )
                    })
                    .when(count > 0, |host| host.child(gallery)),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .bottom(layout.unit(58.))
                    .text_size(layout.unit(18.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(color(WHITE).opacity(0.8))
                    .child(format!("{count} 张照片")),
            )
            .child(
                div()
                    .id("photo-refresh")
                    .absolute()
                    .right(layout.unit(190.))
                    .top(layout.unit(139.))
                    .size(layout.unit(44.))
                    .cursor_pointer()
                    .overflow_hidden()
                    .child(
                        img(wmc_assets::REFRESH_COMMAND)
                            .absolute()
                            .left(layout.unit(-46.))
                            .top(layout.unit(-16.22))
                            .w(layout.unit(136.))
                            .h(layout.unit(76.44)),
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.rescan_library(cx))),
            )
    }

    fn photo_card(
        &self,
        index: usize,
        position: usize,
        cell: f32,
        layout: Layout,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let item = &self.library.items[index];
        let focused = if self.library.pointer_navigation {
            self.library.hovered == Some(index)
        } else {
            self.destination_state.focus == DestinationFocus::Content
                && self.library.selection[0] == position
        };
        let image = if let Some(image) = self.photo_image(&item.file.path, super::photos::THUMB, cx)
        {
            img(image).size_full().object_fit(ObjectFit::Cover)
        } else {
            artwork(LibraryKind::Photos, layout.unit(cell))
        };
        let weak = cx.entity().downgrade();
        div()
            .id(("photo-file", index))
            .relative()
            .size(layout.unit(cell))
            .flex_shrink_0()
            .overflow_hidden()
            .cursor_pointer()
            .border_1()
            .border_color(color(WHITE).opacity(if focused { 0.9 } else { 0.15 }))
            .child(image)
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
                .size_full(),
            )
            .on_hover(cx.listener(move |this, hovered: &bool, window, cx| {
                if this.context_menu.is_some() || this.details.is_some() {
                    return;
                }
                if *hovered {
                    this.library.pointer_navigation = true;
                    this.library.hovered = Some(index);
                    this.library.selection[0] = position;
                    this.destination_state.focus = DestinationFocus::Content;
                    window.focus(&this.focus, cx);
                } else if this.library.hovered == Some(index) {
                    this.library.hovered = None;
                }
                cx.notify();
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.library.selection[0] = position;
                    this.destination_state.focus = DestinationFocus::Content;
                    this.show_context_menu(Some(index), event.position, window, cx);
                }),
            )
            .on_click(cx.listener(move |this, _, window, cx| {
                this.library.selection[0] = position;
                this.destination_state.focus = DestinationFocus::Content;
                this.open_media(index, window, cx);
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::{Grid, WIDTH, photo_band, photo_row, row_photos};
    #[test]
    fn scrolling_only_requests_nearby_rows() {
        assert_eq!(photo_band(195..201, 1003, 5), 968..1003);
        assert_eq!(photo_band(0..3, 1003, 5), 0..18);
    }

    #[test]
    fn only_the_first_row_can_be_incomplete() {
        for columns in 3..=12 {
            for count in 1_usize..1010 {
                let rows = count.div_ceil(columns);
                let positions: Vec<_> = (0..rows)
                    .flat_map(|row| row_photos(row, count, columns))
                    .collect();
                assert_eq!(positions, (0..count).collect::<Vec<_>>());
                for row in 1..rows {
                    assert_eq!(row_photos(row, count, columns).len(), columns);
                }
                assert_eq!(photo_row(count - 1, count, columns), rows - 1);
                for position in 0..count {
                    assert!(
                        row_photos(photo_row(position, count, columns), count, columns)
                            .contains(&position)
                    );
                }
            }
        }
        assert_eq!(row_photos(0, 1003, 5), 0..3);
        assert_eq!(row_photos(200, 1003, 5), 998..1003);
    }

    #[test]
    fn zoom_is_continuous_and_stays_inside_the_original_gallery() {
        assert_eq!(Grid::new(1.).columns, 5);
        for tick in 1..300 {
            let grid = Grid::new(tick as f32 / 100.);
            assert!(grid.width <= WIDTH + 0.01);
            assert!((3..=12).contains(&grid.columns));
            assert!((64. ..=264.).contains(&grid.cell));
        }
        assert!((Grid::new(1.001).cell - Grid::new(1.).cell).abs() < 0.2);
    }
}

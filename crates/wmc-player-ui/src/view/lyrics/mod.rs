//! Lyric viewport rendering and preparation of its visible image neighborhood.
pub(super) mod cache;
pub(super) mod font;
pub(super) mod font_catalog;
pub(super) mod layout;
pub(super) mod line;
mod raster;
use super::{PlayerView, frame::FrameState, layout::*};
use gpui_kit::{prelude::*, *};
use wmc_motion::lyrics::profile as specs;

/// Include actual painted bounds plus one spatial neighbor above and below.
/// Wrapped text, translations and overlapping voices need no fixed row count.
fn viewport_lines(bounds: &[(f32, f32)], height: f32, active: Option<usize>) -> Vec<usize> {
    let mut visible = Vec::new();
    let mut above: Option<(usize, f32)> = None;
    let mut below: Option<(usize, f32)> = None;
    for (index, &(top, line_height)) in bounds.iter().enumerate() {
        let bottom = top + line_height;
        if bottom + 30. >= 0. && top - 30. <= height {
            visible.push(index);
        } else if bottom < 0. && above.is_none_or(|(_, edge)| bottom > edge) {
            above = Some((index, bottom));
        } else if top > height && below.is_none_or(|(_, edge)| top < edge) {
            below = Some((index, top));
        }
    }
    visible.sort_by_key(|index| index.abs_diff(active.unwrap_or(0)));
    // Prefetch must not take raster slots before the visible text.
    visible.extend(above.map(|(index, _)| index));
    visible.extend(below.map(|(index, _)| index));
    visible
}

pub(super) fn sharp_opacity(sigma: f32) -> f32 {
    (1. - sigma / 0.9).clamp(0., 1.)
}

#[cfg(test)]
mod viewport_tests {
    use super::{sharp_opacity, viewport_lines};
    #[test]
    fn all_visible_rows_and_only_one_neighbor_per_side_are_prepared() {
        let bounds: Vec<_> = (0..20)
            .map(|index| (index as f32 * 120. - 200., 90.))
            .collect();
        let wanted = viewport_lines(&bounds, 720., Some(3));
        assert_eq!(wanted[0], 3);
        for index in 1..=7 {
            assert!(wanted.contains(&index));
        }
        assert!(wanted.contains(&0));
        assert!(wanted.contains(&8));
        assert!(!wanted.contains(&9));
        assert_eq!(wanted.len(), 9);
        assert!(viewport_lines(&bounds, 1200., Some(3)).len() > wanted.len());
    }
    #[test]
    fn wrapped_rows_and_translations_use_their_full_height() {
        let bounds = [
            (-500., 100.),
            (-180., 240.),
            (140., 420.),
            (620., 90.),
            (850., 80.),
            (1000., 80.),
        ];
        let wanted = viewport_lines(&bounds, 720., Some(1));
        assert!(
            wanted.contains(&1),
            "partially visible tall stanza must be cached"
        );
        assert!(wanted.contains(&2));
        assert!(wanted.contains(&3));
        assert!(wanted.contains(&0));
        assert!(wanted.contains(&4));
        assert!(!wanted.contains(&5));
    }
    #[test]
    fn pending_images_never_force_distant_lines_sharp() {
        assert_eq!(sharp_opacity(0.), 1.);
        assert!((sharp_opacity(0.45) - 0.5).abs() < 0.001);
        assert_eq!(sharp_opacity(0.9), 0.);
        assert_eq!(sharp_opacity(7.), 0.);
    }
}

impl PlayerView {
    pub(super) fn prepare_lyric_images(
        &mut self,
        frame: &FrameState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for mut cache in self.retired_lyrics.drain(..) {
            cache.release(window);
        }
        let FrameState {
            scale,
            logical_height,
            columns,
            layouts,
            active,
            heights,
            ..
        } = frame;
        let scale = *scale;
        let logical_height = *logical_height;
        let lyric_width = columns.lyric_width;
        self.lyric_images.prepare_layout(scale, lyric_width, window);
        self.translation
            .images
            .prepare_layout(scale, lyric_width, window);
        let bounds: Vec<_> = self
            .motion
            .lines
            .iter()
            .enumerate()
            .map(|(index, line)| (line.position.position, heights[index]))
            .collect();
        let visible = viewport_lines(&bounds, logical_height, *active);
        self.lyric_images.select_lines(&visible, window);
        self.translation.images.select_lines(&visible, window);
        for index in visible {
            if let Some(layout) = self.translation.raster_layout(index)
                && let Some(job) = self.translation.images.request(index, layout, scale)
            {
                let generation = job.generation;
                cx.spawn(async move |this, cx| {
                    let result = cx
                        .background_executor()
                        .spawn(async move { job.run() })
                        .await;
                    let _ = this.update(cx, |this, cx| {
                        this.translation.images.accept(generation, index, result);
                        cx.notify();
                    });
                })
                .detach();
            }
            if let Some(job) = self
                .lyric_images
                .request(index, layouts.lines[index].clone(), scale)
            {
                let generation = job.generation;
                cx.spawn(async move |this, cx| {
                    let result = cx
                        .background_executor()
                        .spawn(async move { job.run() })
                        .await;
                    let _ = this.update(cx, |this, cx| {
                        this.lyric_images.accept(generation, index, result);
                        cx.notify();
                    });
                })
                .detach();
            }
        }
    }

    pub(super) fn lyric_view(&self, frame: &FrameState, cx: &mut Context<Self>) -> Div {
        let FrameState {
            scale,
            logical_height,
            columns,
            layouts,
            word_seconds,
            selected,
            heights,
            anchor,
            arrival,
            ..
        } = frame;
        let scale = *scale;
        let word_seconds = *word_seconds;
        let anchor = *anchor;
        let lyric_width = columns.lyric_width;
        let lyric_height = *logical_height;
        let unit = |v| px(v * scale);
        let mut lyrics = div()
            .absolute()
            .left(unit(columns.lyric_left - 30. + arrival.details_x))
            .opacity(arrival.details_alpha)
            .top(unit(columns.lyric_top))
            .w(unit(lyric_width + 60.))
            .h(unit(lyric_height))
            .overflow_hidden();
        if let Some(mut dots) = frame.interlude {
            // Springs may still be moving through the reserved slot. Reveal
            // dots only as actual text bounds clear it, rather than overlaying.
            let clearance = self
                .motion
                .lines
                .iter()
                .enumerate()
                .map(|(index, line)| {
                    let top = line.position.position - 8.;
                    let bottom = line.position.position
                        + layouts.lines[index].height
                        + self.translation.height(index)
                        + 8.;
                    (top - (anchor + 48.)).max(anchor - bottom)
                })
                .fold(12_f32, f32::min);
            let t = (clearance / 12.).clamp(0., 1.);
            dots.alpha *= t * t * (3. - 2. * t);
            lyrics = lyrics.child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        for (index, pulse) in dots.pulses.iter().enumerate() {
                            let center = bounds.origin
                                + point(px((8. + index as f32 * 24.) * scale), px(12. * scale));
                            let radius = (6. + 1.2 * pulse) * scale;
                            let blue = dots.blue[index];
                            let channel =
                                |gray: f32, lit: f32| (gray + (lit - gray) * blue).round() as u32;
                            let white = channel(180., 255.);
                            let color = (white << 16) | (white << 8) | white;
                            // Filled paths retain fractional coordinates; quads
                            // snap their edges and make this small radius jump.
                            let mut circle = PathBuilder::fill();
                            let at =
                                |x: f32, y: f32| center + point(px(x * radius), px(y * radius));
                            let k = 0.552_284_8;
                            circle.move_to(at(1., 0.));
                            circle.cubic_bezier_to(at(0., 1.), at(1., k), at(k, 1.));
                            circle.cubic_bezier_to(at(-1., 0.), at(-k, 1.), at(-1., k));
                            circle.cubic_bezier_to(at(0., -1.), at(-1., -k), at(-k, -1.));
                            circle.cubic_bezier_to(at(1., 0.), at(k, -1.), at(1., -k));
                            circle.close();
                            if let Ok(path) = circle.build() {
                                window.paint_path(path, rgb(color).opacity(dots.alpha * 0.85));
                            }
                        }
                    },
                )
                .absolute()
                .left(unit(30.))
                .top(unit(anchor + 12.))
                .w(unit(64.))
                .h(unit(24.)),
            );
        }
        for (index, line) in self.lyrics.lines.iter().enumerate() {
            let animation = &self.motion.lines[index];
            let clarity = animation.clarity.position.clamp(0., 1.);
            let visual_scale = specs::DESELECTED_SCALE + (1. - specs::DESELECTED_SCALE) * clarity;
            let highlight = animation.highlight;
            let focused = selected.contains(&index);
            let top = animation.position.position;
            if top + heights[index] + 30. < 0. || top - 30. > lyric_height {
                continue;
            }
            let distance = ((top - anchor) / (LYRIC_LINE_HEIGHT + LYRIC_ROW_GAP)).abs();
            let sigma = wmc_motion::player::focus::blur_sigma(distance, clarity);
            let sharp_mix = sharp_opacity(sigma);
            let cached = self.lyric_images.get(index);
            // A pending blur texture is not permission to display a sharp line.
            let show_sharp = sharp_mix > 0.001;
            let opacity_sharp = sharp_mix;
            let opacity = if focused {
                1.
            } else {
                (0.62 - distance * 0.055).max(0.25)
            };
            let start = line.start;
            lyrics = lyrics.child(
                div()
                    .id(("lyric-line", index))
                    .absolute()
                    .left(unit(
                        30. + if line.right_aligned {
                            (lyric_width - layouts.lines[index].width).max(0.)
                        } else {
                            0.
                        },
                    ))
                    .top(unit(top))
                    .w(unit(lyric_width))
                    .h(unit(heights[index]))
                    .cursor_pointer()
                    .opacity(
                        opacity
                            * wmc_motion::player::focus::edge_opacity(
                                top + LYRIC_LINE_HEIGHT / 2.,
                                lyric_height,
                            ),
                    )
                    .hover(|style| style.opacity(1.))
                    .child(
                        div()
                            .absolute()
                            .top(unit(layouts.lines[index].height * (1. - visual_scale) / 2.))
                            .w(unit(lyric_width * visual_scale))
                            .h(unit(layouts.lines[index].height * visual_scale))
                            .when(show_sharp, |row| {
                                row.child(
                                    div().relative().size_full().opacity(opacity_sharp).child(
                                        line::render(
                                            layouts.lines[index].clone(),
                                            cached.clone(),
                                            word_seconds,
                                            highlight,
                                            scale * visual_scale,
                                        ),
                                    ),
                                )
                            })
                            .when_some(cached, |row, cached| {
                                row.when(sigma > 0., |row| {
                                    row.child(cached.blurred(sigma, scale * visual_scale))
                                })
                            }),
                    )
                    .when(self.translation.height(index) > 0., |row| {
                        row.child(
                            div()
                                .absolute()
                                .left(unit(if line.right_aligned {
                                    -(lyric_width - layouts.lines[index].width).max(0.)
                                        * visual_scale
                                } else {
                                    0.
                                }))
                                .top(unit(
                                    layouts.lines[index].height * (1. + visual_scale) / 2.
                                        + super::translation::LYRIC_GAP * visual_scale,
                                ))
                                .w(unit(lyric_width * visual_scale))
                                .when(line.right_aligned, |row| row.text_align(TextAlign::Right))
                                .child(self.translation.line(index, scale * visual_scale, sigma)),
                        )
                    })
                    .on_click(cx.listener(move |this, _, _, cx| this.seek_lyric(start, cx))),
            );
        }
        if self.lyrics.lines.is_empty() {
            if let Some(text) = &self.lyrics.plain_text {
                lyrics = lyrics.child(
                    div()
                        .id("unsynchronized-lyrics")
                        .size_full()
                        .overflow_y_scroll()
                        .px(unit(30.))
                        .pt(unit(100.))
                        .pb(unit(80.))
                        .text_size(super::text::font_pixels(29. * scale))
                        .text_color(rgb(0xd9e8f0).opacity(0.8))
                        .child(
                            div()
                                .mb(unit(20.))
                                .text_size(super::text::font_pixels(14. * scale))
                                .child("文本歌词 · 无同步时间"),
                        )
                        .children(text.lines().enumerate().map(|(index, line)| {
                            div()
                                .mb(unit(10.))
                                .child(line.to_owned())
                                .when(self.translation.enabled, |row| {
                                    row.child(self.translation.line(index, scale, 0.))
                                })
                        })),
                );
            } else {
                lyrics = lyrics.child(
                    div()
                        .mt(unit(170.))
                        .text_size(super::text::font_pixels(29. * scale))
                        .text_color(rgb(0xd9e8f0).opacity(0.6))
                        .child("此歌曲暂无同步歌词"),
                );
            }
        }
        lyrics
    }
}

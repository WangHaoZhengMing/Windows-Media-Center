use super::HomeView;
use crate::theme::{Layout, MUTED, WHITE, color};
use gpui_kit::{prelude::*, *};
use wmc_core::CATEGORIES;
use wmc_motion::{HomeArrival, MenuSample, profile::TITLE_TOP_INSET};

impl HomeView {
    pub(super) fn rail(
        &self,
        layout: Layout,
        motion: &MenuSample<{ CATEGORIES.len() }>,
        arrival: &HomeArrival,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut rail = div().absolute().size_full();
        for (index, category) in CATEGORIES.iter().enumerate() {
            let pose = motion.rows[index];
            let entrance = arrival.category(index, self.state.category_index());
            let title_width = category.title.chars().count() as f32 * 44.;
            rail =
                rail.child(
                    div()
                        .id(("category", index))
                        .absolute()
                        .left(layout.unit(
                            210. + pose.title_x - title_width * (entrance.scale[0] - 1.) / 2.,
                        ))
                        .top(px(layout.content_top()
                            + (pose.row_y + pose.title_y
                                - TITLE_TOP_INSET
                                - 65. * (entrance.scale[1] - 1.) / 2.)
                                * layout.scale))
                        .w(layout.unit(550. * entrance.scale[0]))
                        .h(layout.unit(65. * entrance.scale[1]))
                        .cursor_pointer()
                        .text_size(layout.unit(44. * entrance.scale[1]))
                        .opacity(entrance.opacity)
                        .font_family("Microsoft YaHei UI")
                        .font_weight(FontWeight::LIGHT)
                        .text_color(category_color(pose.highlight))
                        .hover(|style| style.text_color(color(WHITE).opacity(0.8)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if this.queue_visible {
                                return;
                            }
                            this.state.select_category(index);
                            this.launch_dismissed = true;
                            window.focus(&this.focus, cx);
                            cx.notify();
                        }))
                        .child(category.title),
                );
        }
        rail
    }
}

fn category_color(highlight: f32) -> Hsla {
    let highlight = highlight.clamp(0., 1.);
    let channel = |shift: u32| {
        let from = ((MUTED >> shift) & 255) as f32;
        let to = ((WHITE >> shift) & 255) as f32;
        (from + (to - from) * highlight).round() as u32
    };
    color((channel(16) << 16) | (channel(8) << 8) | channel(0)).opacity(0.5 + highlight * 0.5)
}

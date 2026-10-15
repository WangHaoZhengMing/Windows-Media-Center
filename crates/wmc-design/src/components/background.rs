use crate::theme::Layout;
use gpui_kit::{prelude::*, *};
use std::{f32::consts::TAU, time::Duration};
use wmc_assets::{AMBIENT_GLOW, BLUE_BACKGROUND};

pub fn background(layout: Layout, animate: bool) -> Div {
    let glow = img(AMBIENT_GLOW)
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .object_fit(ObjectFit::Fill);
    let glow = if animate {
        glow.with_animation(
            "ambient-light",
            Animation::new(Duration::from_secs(28))
                .repeat()
                .with_max_fps(30.),
            move |image, progress| {
                let wave = (progress * TAU).sin();
                image
                    .left(layout.unit(wave * 16.))
                    .top(layout.unit(wave * 9.))
                    .opacity(0.12 + wave * 0.06)
            },
        )
        .into_any_element()
    } else {
        glow.opacity(0.12).into_any_element()
    };
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .overflow_hidden()
        .child(
            img(BLUE_BACKGROUND)
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .object_fit(ObjectFit::Fill),
        )
        .child(glow)
}

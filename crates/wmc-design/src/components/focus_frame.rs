use crate::theme::{Layout, SHORTCUT_HEIGHT, SHORTCUT_WIDTH};
use gpui_kit::{prelude::*, *};
use std::{f32::consts::TAU, time::Duration};
use wmc_assets::{GLASS_FRAME, HORIZONTAL_GLEAM, VERTICAL_GLEAM};

/// STARTMENU.MCML draws a separate frame and animated hotspot PNGs.
/// This selector belongs to the viewport, not to an individual moving shortcut.
pub fn focus_frame(layout: Layout) -> Div {
    let width = SHORTCUT_WIDTH;
    div()
        .relative()
        .w(layout.unit(width))
        .h(layout.unit(SHORTCUT_HEIGHT))
        .child(
            img(GLASS_FRAME)
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .object_fit(ObjectFit::Fill),
        )
        .child(
            img(HORIZONTAL_GLEAM)
                .absolute()
                .top(layout.unit(1.))
                .w(layout.unit(66.))
                .h(layout.unit(3.7))
                .object_fit(ObjectFit::Fill)
                .with_animation(
                    "glass-edge-gleam",
                    Animation::new(Duration::from_secs(9))
                        .repeat()
                        .with_max_fps(20.),
                    move |image, progress| {
                        let sweep = ((progress * TAU).sin() + 1.) * 0.5;
                        image
                            .left(layout.unit(2. + sweep * (width - 70.)))
                            .opacity(0.35)
                    },
                ),
        )
        .child(
            img(VERTICAL_GLEAM)
                .absolute()
                .right(layout.unit(1.))
                .top(layout.unit(22.))
                .w(layout.unit(3.7))
                .h(layout.unit(44.))
                .opacity(0.2)
                .object_fit(ObjectFit::Fill),
        )
}

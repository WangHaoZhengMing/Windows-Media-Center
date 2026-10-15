//! The same original NineGrid resources/splits used by the WMC library menus.
use super::resources::{guarded_image, paint_guarded_image};
use gpui_kit::{prelude::*, *};
use std::sync::{Arc, OnceLock};
#[derive(Clone, Copy)]
pub(super) enum Skin {
    Outline,
    Background,
    Focus,
}
struct Slices {
    parts: Vec<Arc<RenderImage>>,
    x: [u32; 4],
    y: [u32; 4],
}
fn skins() -> &'static [Slices] {
    static CACHE: OnceLock<Vec<Slices>> = OnceLock::new();
    CACHE.get_or_init(|| {
        [
            ("OVERLAYS.OUTLINE.PNG", [4, 283], [4, 82]),
            ("OVERLAYS.BACKGROUND.TOP.DARK.PNG", [10, 261], [10, 216]),
            ("COMMON.BUTTON.LEFT.FOCUS.PNG", [20, 231], [12, 41]),
        ]
        .into_iter()
        .map(|(name, sx, sy)| {
            let bytes = wmc_assets::load(&format!("wmc/context-menu/{name}"))
                .expect("original WMC menu skin");
            let image = image::load_from_memory(&bytes)
                .expect("valid WMC skin")
                .to_rgba8();
            let x = [0, sx[0], sx[1], image.width()];
            let y = [0, sy[0], sy[1], image.height()];
            let mut parts = Vec::with_capacity(9);
            for row in 0..3 {
                for col in 0..3 {
                    parts.push(guarded_image(
                        image::imageops::crop_imm(
                            &image,
                            x[col],
                            y[row],
                            x[col + 1] - x[col],
                            y[row + 1] - y[row],
                        )
                        .to_image(),
                    ));
                }
            }
            Slices { parts, x, y }
        })
        .collect()
    })
}
pub(super) fn surface(kind: Skin) -> Div {
    div().absolute().top_0().left_0().size_full().child(
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let skin = &skins()[kind as usize];
                let width = f32::from(bounds.size.width);
                let height = f32::from(bounds.size.height);
                let x = [
                    0.,
                    skin.x[1] as f32,
                    width - (skin.x[3] - skin.x[2]) as f32,
                    width,
                ];
                let y = [
                    0.,
                    skin.y[1] as f32,
                    height - (skin.y[3] - skin.y[2]) as f32,
                    height,
                ];
                for row in 0..3 {
                    for col in 0..3 {
                        let tile = Bounds::new(
                            bounds.origin + point(px(x[col]), px(y[row])),
                            size(
                                px((x[col + 1] - x[col]).max(0.)),
                                px((y[row + 1] - y[row]).max(0.)),
                            ),
                        );
                        paint_guarded_image(window, tile, skin.parts[row * 3 + col].clone());
                    }
                }
            },
        )
        .size_full(),
    )
}

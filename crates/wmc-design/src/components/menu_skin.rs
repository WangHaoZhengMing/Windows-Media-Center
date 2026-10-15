//! MCML NineGrid/MetaImage rendering using unmodified original resource pixels.
use gpui_kit::{prelude::*, *};
use std::sync::{Arc, OnceLock};

#[derive(Clone, Copy)]
pub enum MenuSkin {
    Outline,
    Shadow,
    TopLight,
    TopDark,
    BottomLight,
    BottomDark,
    Focus,
    FocusOverlay,
}

struct Slices {
    parts: Vec<Arc<RenderImage>>,
    x: [u32; 4],
    y: [u32; 4],
}

const SOURCES: [(&str, [u32; 2], [u32; 2]); 8] = [
    ("OVERLAYS.OUTLINE.PNG", [4, 283], [4, 82]),
    ("OVERLAYS.SHADOW.PNG", [40, 280], [40, 408]),
    ("OVERLAYS.BACKGROUND.TOP.LIGHT.PNG", [10, 261], [10, 216]),
    ("OVERLAYS.BACKGROUND.TOP.DARK.PNG", [10, 261], [10, 216]),
    ("OVERLAYS.BACKGROUND.BOTTOM.LIGHT.PNG", [10, 261], [10, 162]),
    ("OVERLAYS.BACKGROUND.BOTTOM.DARK.PNG", [10, 261], [10, 162]),
    ("COMMON.BUTTON.LEFT.FOCUS.PNG", [20, 231], [12, 41]),
    ("COMMON.BUTTON.LEFT.FOCUSOVERLAY.PNG", [20, 231], [12, 41]),
];

fn slices() -> &'static [Slices] {
    static CACHE: OnceLock<Vec<Slices>> = OnceLock::new();
    CACHE.get_or_init(|| {
        SOURCES
            .iter()
            .map(|(name, split_x, split_y)| {
                let path = format!("wmc/context-menu/{name}");
                let bytes = wmc_assets::load(&path).expect("embedded original menu resource");
                let image = image::load_from_memory(&bytes)
                    .expect("validated original PNG")
                    .to_rgba8();
                let x = [0, split_x[0], split_x[1], image.width()];
                let y = [0, split_y[0], split_y[1], image.height()];
                let mut parts = Vec::with_capacity(9);
                for row in 0..3 {
                    for col in 0..3 {
                        let mut part = image::imageops::crop_imm(
                            &image,
                            x[col],
                            y[row],
                            x[col + 1] - x[col],
                            y[row + 1] - y[row],
                        )
                        .to_image();
                        for pixel in part.pixels_mut() {
                            pixel.0.swap(0, 2);
                        }
                        parts.push(Arc::new(RenderImage::new(vec![image::Frame::new(part)])));
                    }
                }
                Slices { parts, x, y }
            })
            .collect()
    })
}

/// Border pixels retain their original size; only the middle strips stretch.
pub fn menu_skin(skin: MenuSkin, width: f32, height: f32, scale: [f32; 2]) -> Div {
    let skin = &slices()[skin as usize];
    let x = [
        0.,
        skin.x[1] as f32 * scale[0],
        width - (skin.x[3] - skin.x[2]) as f32 * scale[0],
        width,
    ];
    let y = [
        0.,
        skin.y[1] as f32 * scale[1],
        height - (skin.y[3] - skin.y[2]) as f32 * scale[1],
        height,
    ];
    let mut root = div().absolute().w(px(width)).h(px(height));
    for row in 0..3 {
        for col in 0..3 {
            root = root.child(
                img(skin.parts[row * 3 + col].clone())
                    .absolute()
                    .left(px(x[col]))
                    .top(px(y[row]))
                    .w(px((x[col + 1] - x[col]).max(0.)))
                    .h(px((y[row + 1] - y[row]).max(0.)))
                    .object_fit(ObjectFit::Fill),
            );
        }
    }
    root
}

#[cfg(test)]
mod tests {
    use super::SOURCES;

    #[test]
    fn original_splits_are_inside_every_embedded_image() {
        for (name, x, y) in SOURCES {
            let data = wmc_assets::load(&format!("wmc/context-menu/{name}")).unwrap();
            let image = image::load_from_memory(&data).unwrap();
            assert!(x[0] > 0 && x[0] < x[1] && x[1] < image.width());
            assert!(y[0] > 0 && y[0] < y[1] && y[1] < image.height());
        }
    }
}

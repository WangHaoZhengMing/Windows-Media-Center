//! Song-owned, lazy glyph masks. Font tables stay encoded until a glyph is requested.
use ab_glyph::{Font, FontVec, point};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub(crate) struct GlyphMask {
    pub xmin: i32,
    pub ymin: i32,
    pub width: usize,
    pub height: usize,
    pub bitmap: Vec<u8>,
}

struct Masks {
    size: u32,
    glyphs: HashMap<u16, Arc<GlyphMask>>,
}

pub(crate) struct SongFont {
    font: FontVec,
    characters: HashMap<char, u16>,
    masks: Mutex<Masks>,
}

impl SongFont {
    pub fn load(text: &str) -> Option<Arc<Self>> {
        let (path, index) = if cfg!(target_os = "windows") {
            (
                PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into()))
                    .join("Fonts/msyhbd.ttc"),
                1,
            )
        } else if cfg!(target_os = "macos") {
            (PathBuf::from("/System/Library/Fonts/PingFang.ttc"), 0)
        } else {
            (
                PathBuf::from("/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc"),
                0,
            )
        };
        let font = FontVec::try_from_vec_and_index(std::fs::read(path).ok()?, index).ok()?;
        let characters = text.chars().map(|c| (c, font.glyph_id(c).0)).collect();
        Some(Arc::new(Self {
            font,
            characters,
            masks: Mutex::new(Masks {
                size: 0,
                glyphs: HashMap::new(),
            }),
        }))
    }

    pub fn rasterize(&self, character: char, size: f32) -> Option<Arc<GlyphMask>> {
        let id = *self.characters.get(&character)?;
        let mut masks = self.masks.lock().unwrap_or_else(|error| error.into_inner());
        // A resize replaces the size cache rather than retaining every past scale.
        if masks.size != size.to_bits() {
            masks.glyphs.clear();
            masks.size = size.to_bits();
        }
        Some(
            masks
                .glyphs
                .entry(id)
                .or_insert_with(|| {
                    let scale = size * self.font.height_unscaled()
                        / self.font.units_per_em().unwrap_or(2048.);
                    let outline = self.font.outline_glyph(
                        ab_glyph::GlyphId(id).with_scale_and_position(scale, point(0., 0.)),
                    );
                    let mut mask = GlyphMask {
                        xmin: 0,
                        ymin: 0,
                        width: 0,
                        height: 0,
                        bitmap: Vec::new(),
                    };
                    if let Some(outline) = outline {
                        let bounds = outline.px_bounds();
                        mask.xmin = bounds.min.x as i32;
                        mask.ymin = -bounds.max.y as i32;
                        mask.width = bounds.width() as usize;
                        mask.height = bounds.height() as usize;
                        mask.bitmap.resize(mask.width * mask.height, 0);
                        outline.draw(|x, y, coverage| {
                            mask.bitmap[y as usize * mask.width + x as usize] =
                                (coverage.clamp(0., 1.) * 255.).round() as u8;
                        });
                    }
                    Arc::new(mask)
                })
                .clone(),
        )
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn rasterization_preserves_em_size_baseline_and_stroke_coverage() {
        let font = SongFont::load("英雄泪Ag").unwrap();
        let old = fontdue::Font::from_bytes(
            font.font.as_slice(),
            fontdue::FontSettings {
                collection_index: 1,
                scale: 44.,
                ..Default::default()
            },
        )
        .unwrap();
        for size in [22., 44., 66.] {
            for c in "英雄泪Ag".chars() {
                let mask = font.rasterize(c, size).unwrap();
                let (metrics, bitmap) = old.rasterize(c, size);
                assert!((mask.xmin - metrics.xmin).abs() <= 1, "{c} at {size}");
                assert!((mask.ymin - metrics.ymin).abs() <= 1, "{c} at {size}");
                assert!((mask.width as i32 - metrics.width as i32).abs() <= 1);
                assert!((mask.height as i32 - metrics.height as i32).abs() <= 1);
                let sum = |pixels: &[u8]| pixels.iter().map(|a| u64::from(*a)).sum::<u64>() as f64;
                assert!(
                    (sum(&mask.bitmap) / sum(&bitmap) - 1.).abs() < 0.05,
                    "{c} stroke coverage at {size}"
                );
            }
        }
    }

    #[test]
    fn only_requested_song_glyphs_are_cached_and_resize_replaces_them() {
        let font = SongFont::load("英雄泪 A ").expect("system font");
        assert!(font.masks.lock().unwrap().glyphs.is_empty());
        let first = font.rasterize('英', 44.).unwrap();
        assert!(first.bitmap.iter().any(|a| *a > 0));
        assert!(Arc::ptr_eq(&first, &font.rasterize('英', 44.).unwrap()));
        assert!(font.rasterize('外', 44.).is_none());
        font.rasterize('雄', 44.).unwrap();
        assert_eq!(font.masks.lock().unwrap().glyphs.len(), 2);
        font.rasterize('泪', 66.).unwrap();
        assert_eq!(font.masks.lock().unwrap().glyphs.len(), 1);
        assert_eq!(font.rasterize(' ', 66.).unwrap().width, 0);
    }
}

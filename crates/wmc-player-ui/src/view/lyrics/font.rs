//! Song-owned font faces and masks. Glyph identities include their actual face.
use super::font_catalog::{self, Face};
use ab_glyph::{Font, point};
use gpui_kit::{FontId, GlyphId};
use std::{
    collections::HashMap,
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
    glyphs: HashMap<(fontdb::ID, u16), Arc<GlyphMask>>,
}
pub(crate) struct SongFont {
    faces: Mutex<HashMap<fontdb::ID, Arc<Face>>>,
    masks: Mutex<Masks>,
}
impl SongFont {
    pub fn load(text: &str) -> Option<Arc<Self>> {
        let faces: HashMap<_, _> = font_catalog::resolve(text, 700)
            .into_iter()
            .map(|span| (span.face.id, span.face))
            .collect();
        if faces.is_empty() {
            return None;
        }
        Some(Arc::new(Self {
            faces: Mutex::new(faces),
            masks: Mutex::new(Masks {
                size: 0,
                glyphs: HashMap::new(),
            }),
        }))
    }
    pub fn rasterize_shaped(
        &self,
        native: FontId,
        glyph: GlyphId,
        character: char,
        size: f32,
    ) -> Option<Arc<GlyphMask>> {
        let (face, glyph) = if let Some(face) = font_catalog::native_face(native) {
            (face, u16::try_from(glyph.0).ok()?)
        } else {
            let face = font_catalog::resolve(&character.to_string(), 700)
                .into_iter()
                .next()?
                .face;
            let glyph = face.font.glyph_id(character).0;
            (face, glyph)
        };
        self.mask(face, glyph, size)
    }
    fn mask(&self, face: Arc<Face>, id: u16, size: f32) -> Option<Arc<GlyphMask>> {
        self.faces
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .entry(face.id)
            .or_insert_with(|| face.clone());
        let mut masks = self.masks.lock().unwrap_or_else(|error| error.into_inner());
        if masks.size != size.to_bits() {
            masks.glyphs.clear();
            masks.size = size.to_bits();
        }
        Some(
            masks
                .glyphs
                .entry((face.id, id))
                .or_insert_with(|| {
                    let font = &face.font;
                    let scale =
                        size * font.height_unscaled() / font.units_per_em().unwrap_or(2048.);
                    let outline = font.outline_glyph(
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
    #[cfg(test)]
    fn rasterize(&self, character: char, size: f32) -> Option<Arc<GlyphMask>> {
        let face = font_catalog::resolve(&character.to_string(), 700)
            .into_iter()
            .next()?
            .face;
        let glyph = face.font.glyph_id(character).0;
        self.mask(face, glyph, size)
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    #[test]
    fn korean_chinese_and_latin_use_supported_faces_and_real_glyph_masks() {
        let font = SongFont::load("汉字 한글아파서 ABC").expect("system fonts");
        for character in "汉字한글아파서ABC".chars() {
            let face = font_catalog::resolve(&character.to_string(), 700)
                .remove(0)
                .face;
            assert_ne!(
                face.font.glyph_id(character).0,
                0,
                "missing {character} in {:?}",
                face.id
            );
            let mask = font.rasterize(character, 44.).unwrap();
            assert!(
                mask.width > 0 && mask.bitmap.iter().any(|alpha| *alpha > 0),
                "{character}"
            );
        }
        assert!(font.faces.lock().unwrap().len() >= 2);
    }
    #[test]
    fn glyph_masks_reuse_the_selected_face_and_resize_replaces_them() {
        let font = SongFont::load("英雄泪 A ").unwrap();
        assert!(font.masks.lock().unwrap().glyphs.is_empty());
        let first = font.rasterize('英', 44.).unwrap();
        assert!(Arc::ptr_eq(&first, &font.rasterize('英', 44.).unwrap()));
        font.rasterize('雄', 44.).unwrap();
        assert_eq!(font.masks.lock().unwrap().glyphs.len(), 2);
        font.rasterize('泪', 66.).unwrap();
        assert_eq!(font.masks.lock().unwrap().glyphs.len(), 1);
        assert_eq!(font.rasterize(' ', 66.).unwrap().width, 0);
    }
    #[test]
    fn shaped_font_and_glyph_id_are_used_instead_of_re_mapping_a_character() {
        let font = SongFont::load("한A").unwrap();
        let face = font_catalog::resolve("한", 700).remove(0).face;
        let id = face.font.glyph_id('한');
        let native = FontId(usize::MAX);
        font_catalog::register(native, face.id);
        let shaped = font
            .rasterize_shaped(native, GlyphId(u32::from(id.0)), 'A', 44.)
            .unwrap();
        let korean = font.rasterize('한', 44.).unwrap();
        assert!(Arc::ptr_eq(&shaped, &korean));
    }
}

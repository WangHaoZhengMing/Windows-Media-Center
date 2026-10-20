//! One fallback policy for native shaping and CPU rasterization.
use ab_glyph::FontVec;
use fontdb::{Database, Family, ID, Query, Weight};
use gpui_kit::FontId;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, OnceLock, Weak},
};
use unicode_segmentation::UnicodeSegmentation;

const FAMILIES: &[&str] = &[
    "SF Pro Display",
    "Microsoft YaHei UI",
    "Microsoft YaHei",
    "PingFang SC",
    "Malgun Gothic",
    "Noto Sans CJK SC",
    "Noto Sans CJK KR",
    "Noto Sans CJK JP",
    "Noto Sans",
    "Segoe UI",
    "Nirmala UI",
    "Yu Gothic",
    "Hiragino Sans",
    "Noto Sans Arabic",
    "Noto Sans Devanagari",
    "DejaVu Sans",
    "Liberation Sans",
    "Arial",
];

pub(in crate::view) struct Face {
    pub id: ID,
    pub font: FontVec,
}
pub(in crate::view) struct Span {
    pub len: usize,
    pub face: Arc<Face>,
}
pub(in crate::view) struct SelectedSpan {
    pub len: usize,
    pub id: ID,
    pub family: String,
}
struct Catalog {
    database: Database,
    choices: HashMap<(u16, String), Option<ID>>,
    faces: HashMap<ID, Weak<Face>>,
    native: HashMap<FontId, ID>,
    validated: HashSet<ID>,
}
fn catalog() -> &'static Mutex<Catalog> {
    static CATALOG: OnceLock<Mutex<Catalog>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let mut database = Database::new();
        database.load_system_fonts();
        Mutex::new(Catalog::new(database))
    })
}
impl Catalog {
    fn new(database: Database) -> Self {
        Self {
            database,
            choices: HashMap::new(),
            faces: HashMap::new(),
            native: HashMap::new(),
            validated: HashSet::new(),
        }
    }
    fn usable_face(&mut self, grapheme: &str, weight: u16) -> Option<ID> {
        loop {
            let id = self.choose(grapheme, weight)?;
            if self.validated.contains(&id) || self.load(id).is_some() {
                self.validated.insert(id);
                return Some(id);
            }
            // Catalog-readable fonts can still be unsupported by the rasterizer.
            self.database.remove_face(id);
            self.choices.clear();
        }
    }
    fn choose(&mut self, grapheme: &str, weight: u16) -> Option<ID> {
        let key = (weight, grapheme.to_owned());
        if let Some(id) = self.choices.get(&key) {
            return *id;
        }
        let mut primary = None;
        let mut selected = None;
        for family in FAMILIES
            .iter()
            .map(|name| Family::Name(name))
            .chain([Family::SansSerif])
        {
            let Some(id) = self.database.query(&Query {
                families: &[family],
                weight: Weight(weight),
                ..Default::default()
            }) else {
                continue;
            };
            primary.get_or_insert(id);
            let covered = self
                .database
                .with_face_data(id, |bytes, index| {
                    ttf_parser::Face::parse(bytes, index).is_ok_and(|face| {
                        grapheme
                            .chars()
                            .filter(|c| {
                                !c.is_control()
                                    && !matches!(
                                        c,
                                        '\u{200c}' | '\u{200d}' | '\u{fe0e}' | '\u{fe0f}'
                                    )
                            })
                            .all(|character| face.glyph_index(character).is_some())
                    })
                })
                .unwrap_or(false);
            if covered {
                selected = Some(id);
                break;
            }
        }
        // An unsupported character keeps an honest missing-glyph marker.
        let selected = selected.or(primary);
        if self.choices.len() >= 4096 {
            self.choices.clear();
        }
        self.choices.insert(key, selected);
        selected
    }
    fn load(&mut self, id: ID) -> Option<Arc<Face>> {
        if let Some(face) = self.faces.get(&id).and_then(Weak::upgrade) {
            return Some(face);
        }
        let font = self.database.with_face_data(id, |bytes, index| {
            FontVec::try_from_vec_and_index(bytes.to_vec(), index).ok()
        })??;
        let face = Arc::new(Face { id, font });
        self.faces.insert(id, Arc::downgrade(&face));
        Some(face)
    }
}

// Native shaping needs only the family name; it must not reload font tables each frame.
pub(in crate::view) fn select(text: &str, weight: u16) -> Vec<SelectedSpan> {
    let mut catalog = catalog().lock().unwrap_or_else(|error| error.into_inner());
    let mut spans: Vec<SelectedSpan> = Vec::new();
    for grapheme in text.graphemes(true) {
        let Some(id) = catalog.usable_face(grapheme, weight) else {
            return Vec::new();
        };
        if let Some(previous) = spans.last_mut().filter(|span| span.id == id) {
            previous.len += grapheme.len();
        } else {
            let family = catalog.database.face(id).unwrap().families[0].0.clone();
            spans.push(SelectedSpan {
                len: grapheme.len(),
                id,
                family,
            });
        }
    }
    spans
}

pub(in crate::view) fn resolve(text: &str, weight: u16) -> Vec<Span> {
    let mut catalog = catalog().lock().unwrap_or_else(|error| error.into_inner());
    let mut spans: Vec<Span> = Vec::new();
    for grapheme in text.graphemes(true) {
        let Some(id) = catalog.usable_face(grapheme, weight) else {
            return Vec::new();
        };
        if let Some(previous) = spans.last_mut().filter(|span| span.face.id == id) {
            previous.len += grapheme.len();
        } else {
            let Some(face) = catalog.load(id) else {
                return Vec::new();
            };
            spans.push(Span {
                len: grapheme.len(),
                face,
            });
        }
    }
    spans
}

pub(in crate::view) fn register(native: FontId, id: ID) {
    catalog()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .native
        .insert(native, id);
}
pub(super) fn native_face(native: FontId) -> Option<Arc<Face>> {
    let mut catalog = catalog().lock().unwrap_or_else(|error| error.into_inner());
    let id = *catalog.native.get(&native)?;
    catalog.load(id)
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn priority_skips_absent_families_and_faces_without_the_required_glyph() {
        let mut database = Database::new();
        database
            .load_font_file("C:/Windows/Fonts/msyhbd.ttc")
            .unwrap();
        database
            .load_font_file("C:/Windows/Fonts/malgunbd.ttf")
            .unwrap();
        let yahei = database
            .query(&Query {
                families: &[Family::Name("Microsoft YaHei UI")],
                weight: Weight(700),
                ..Default::default()
            })
            .unwrap();
        let mut sf = database.face(yahei).unwrap().clone();
        sf.families[0].0 = "SF Pro Display".into();
        let sf = database.push_face_info(sf);
        let mut catalog = Catalog::new(database);
        assert_eq!(catalog.choose("A", 700), Some(sf));
        let korean = catalog.choose("한", 700).unwrap();
        assert_eq!(
            catalog.database.face(korean).unwrap().families[0].0,
            "Malgun Gothic"
        );
        catalog.database.remove_face(sf);
        catalog.choices.clear();
        assert_eq!(catalog.choose("A", 700), Some(yahei));
    }

    #[test]
    fn mixed_text_preserves_all_bytes_and_catalog_does_not_own_font_tables() {
        let text = "汉字 한글아파서 ABC e\u{301}";
        assert_eq!(
            select(text, 700).iter().map(|span| span.len).sum::<usize>(),
            text.len()
        );
        assert_eq!(
            resolve(text, 700)
                .iter()
                .map(|span| span.len)
                .sum::<usize>(),
            text.len()
        );
        let mut database = Database::new();
        database
            .load_font_file("C:/Windows/Fonts/malgunbd.ttf")
            .unwrap();
        let mut catalog = Catalog::new(database);
        let id = catalog.choose("한", 700).unwrap();
        let face = catalog.load(id).unwrap();
        let weak = Arc::downgrade(&face);
        drop(face);
        assert!(weak.upgrade().is_none());
        assert!(catalog.load(id).is_some());
    }
}

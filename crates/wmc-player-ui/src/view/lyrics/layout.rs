//! Stable native word geometry, rebuilt only when the viewport layout changes.
use super::super::{layout::*, text};
use gpui_kit::*;
use std::ops::Range;
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;
use wmc_lyrics::Lyrics;
use wmc_motion::lyrics::{highlight as progression, profile as specs};

fn balanced_width(total: f32, available: f32, font_size: f32) -> f32 {
    let rows = (total / available).ceil().max(1.);
    let minimum = if rows > 1. { 0. } else { available * 0.85 };
    (total / rows + font_size * 0.5).max(minimum).min(available)
}

// Split only oversized timing spans. Break at spaces when possible and never
// inside a Unicode grapheme (combining marks or joined emoji).
fn split_ranges(text: &str, limit: f32, x: impl Fn(usize) -> f32) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let mut end = start;
        let mut space = None;
        for (relative, grapheme) in text[start..].grapheme_indices(true) {
            let next = start + relative + grapheme.len();
            if x(next) - x(start) > limit && end > start {
                end = space.unwrap_or(end);
                break;
            }
            end = next;
            if grapheme.chars().all(char::is_whitespace) {
                space = Some(end);
            }
        }
        ranges.push(start..end);
        start = end;
    }
    ranges
}

pub(in crate::view) struct PlacedWord {
    pub shaped: ShapedLine,
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub start: f64,
    pub end: f64,
}

pub(in crate::view) struct LineLayout {
    pub words: Vec<PlacedWord>,
    pub width: f32,
    pub height: f32,
    pub word_synced: bool,
    pub font_size: f32,
    pub line_height: f32,
}

impl LineLayout {
    pub fn cursor(&self, seconds: f64, highlight: f32) -> f32 {
        if !self.word_synced {
            return if highlight > 0. { f32::INFINITY } else { 0. };
        }
        progression::line_cursor(
            seconds,
            self.words
                .iter()
                .map(|word| (word.start, word.end, word.width)),
            specs::PROGRESSION_FEATHER,
        )
    }
}

pub(in crate::view) struct Layouts {
    pub scale: f32,
    available_width: f32,
    disable_syllables: bool,
    pub lines: Vec<Arc<LineLayout>>,
    pub order: Vec<usize>,
    pub interludes: Vec<wmc_lyrics::interlude::Interlude>,
}

#[derive(Default)]
pub(in crate::view) struct LayoutCache(Option<Arc<Layouts>>);

impl LayoutCache {
    pub fn get(
        &mut self,
        lyrics: &Lyrics,
        scale: f32,
        width: f32,
        disable_syllables: bool,
        window: &Window,
    ) -> Arc<Layouts> {
        if let Some(layouts) = &self.0
            && (layouts.scale - scale).abs() < 0.001
            && (layouts.available_width - width).abs() < 0.1
            && layouts.disable_syllables == disable_syllables
        {
            return layouts.clone();
        }
        let lines = lyrics
            .lines
            .iter()
            .map(|line| {
                let font_size = if line.background {
                    24.
                } else {
                    LYRIC_FONT_SIZE
                };
                let line_height = if line.background {
                    34.
                } else {
                    LYRIC_LINE_HEIGHT
                };
                let mut x = 0.;
                let mut y = 0.;
                let mut line_width: f32 = 0.;
                let measured: Vec<_> = line
                    .words
                    .iter()
                    .map(|word| {
                        let shaped = text::shape(
                            &word.text,
                            font_size * scale,
                            FontWeight::BOLD,
                            rgb(0xffffff).into(),
                            window,
                        );
                        (word, shaped)
                    })
                    .collect();
                let total = measured
                    .iter()
                    .map(|(_, shaped)| f32::from(shaped.width()) / scale)
                    .sum();
                let wrap_width = balanced_width(total, width, font_size);
                let mut words = Vec::new();
                for (word, shaped) in measured {
                    let advance = f32::from(shaped.width()) / scale;
                    let ranges = if advance > wrap_width {
                        split_ranges(&word.text, wrap_width, |index| {
                            f32::from(shaped.x_for_index(index)) / scale
                        })
                    } else {
                        std::iter::once(0..word.text.len()).collect()
                    };
                    let mut fragments = Vec::new();
                    if ranges.len() == 1 {
                        fragments.push((word.text.clone(), shaped, advance));
                    } else {
                        for range in ranges {
                            let fragment = word.text[range].to_owned();
                            let shaped_fragment = text::shape(
                                &fragment,
                                font_size * scale,
                                FontWeight::BOLD,
                                rgb(0xffffff).into(),
                                window,
                            );
                            let advance = f32::from(shaped_fragment.width()) / scale;
                            fragments.push((fragment, shaped_fragment, advance));
                        }
                    }
                    let fragment_total: f32 =
                        fragments.iter().map(|(_, _, advance)| *advance).sum();
                    let mut consumed = 0.;
                    for (fragment, shaped, advance) in fragments {
                        if x > 0. && x + advance > wrap_width {
                            x = 0.;
                            y += line_height;
                        }
                        let placed = PlacedWord {
                            shaped,
                            text: fragment,
                            x,
                            y,
                            width: advance,
                            // Partition the original visual sweep, not the source
                            // lyric timestamps: wrapping cannot restart a word.
                            start: word.start
                                + (word.end - word.start)
                                    * f64::from(consumed / fragment_total.max(0.001)),
                            end: word.start
                                + (word.end - word.start)
                                    * f64::from((consumed + advance) / fragment_total.max(0.001)),
                        };
                        x += advance;
                        line_width = line_width.max(x);
                        consumed += advance;
                        words.push(placed);
                    }
                }
                Arc::new(LineLayout {
                    words,
                    width: line_width,
                    height: y + line_height,
                    word_synced: line.word_synced && !disable_syllables,
                    font_size,
                    line_height,
                })
            })
            .collect();
        let layouts = Arc::new(Layouts {
            scale,
            available_width: width,
            disable_syllables,
            lines,
            order: lyrics.display_order(),
            interludes: lyrics.interludes(3.),
        });
        self.0 = Some(layouts.clone());
        layouts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn long_chinese_spans_wrap_into_balanced_rows() {
        let text = "甲乙丙丁戊己庚辛壬癸子丑寅卯";
        let limit = balanced_width(14. * 44., 44. * 12., 44.);
        let ranges = split_ranges(text, limit, |index| {
            text[..index].chars().count() as f32 * 44.
        });
        assert_eq!(ranges.len(), 2);
        assert_eq!(
            ranges
                .iter()
                .map(|range| &text[range.clone()])
                .collect::<String>(),
            text
        );
        assert!(
            ranges
                .iter()
                .all(|range| (4..=12).contains(&text[range.clone()].chars().count()))
        );
    }

    #[::core::prelude::v1::test]
    fn wrapping_prefers_spaces_and_preserves_combining_and_joined_characters() {
        let text = "hello world again";
        let ranges = split_ranges(text, 8., |index| index as f32);
        assert_eq!(&text[ranges[0].clone()], "hello ");
        assert_eq!(
            ranges
                .iter()
                .map(|range| &text[range.clone()])
                .collect::<String>(),
            text
        );
        let text = "a\u{301}👩‍💻b";
        let ranges = split_ranges(text, 1., |index| {
            text[..index].graphemes(true).count() as f32
        });
        assert_eq!(
            ranges
                .iter()
                .map(|range| &text[range.clone()])
                .collect::<Vec<_>>(),
            vec!["a\u{301}", "👩‍💻", "b"]
        );
    }

    #[::core::prelude::v1::test]
    fn reading_width_remains_bounded_on_wide_windows() {
        let c = Columns::new(2560., 1440., ui_scale(2560., 1440.));
        assert!(c.lyric_width > 48. * 12.);
        assert!(c.cover_left + c.cover_size < c.lyric_left);
        assert!(balanced_width(300., 528., 44.) >= 300.);
    }
}

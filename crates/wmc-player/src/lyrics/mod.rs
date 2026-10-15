//! Format-independent lyric model and text format dispatch.
use anyhow::{Result, ensure};

pub mod interlude;
mod lrc;
mod word_timed;
pub mod presentation;
pub mod progression;
mod timeline;
mod ttml;
use lrc::timestamp;
pub(crate) use word_timed::qrc_text;

#[derive(Clone, Debug)]
pub struct Word {
    pub text: String,
    pub start: f64,
    pub end: f64,
}

impl Word {
    pub fn progress(&self, seconds: f64) -> f32 {
        ((seconds - self.start) / (self.end - self.start).max(0.001)).clamp(0., 1.) as f32
    }
}

#[derive(Clone, Debug)]
pub struct Line {
    pub start: f64,
    pub end: f64,
    pub words: Vec<Word>,
    /// Plain LRC has line timing only; do not pretend it has real word timing.
    pub word_synced: bool,
    pub agents: Vec<String>,
    pub background: bool,
    /// Stable source group; background rows share their parent's group.
    pub group: Option<usize>,
    pub right_aligned: bool,
}

impl Line {
    /// Keep the line interval separate from the actual sung word interval.
    /// An ELRC closing marker can precede the next line by a long silence.
    pub fn vocal_start(&self) -> f64 {
        self.words.first().map_or(self.start, |word| word.start)
    }

    pub fn vocal_end(&self) -> f64 {
        self.words.last().map_or(self.end, |word| word.end)
    }
    pub fn text(&self) -> String {
        self.words.iter().map(|word| word.text.as_str()).collect()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Lyrics {
    pub lines: Vec<Line>,
    /// Unsynchronized text stays readable without invented line/word timing.
    pub plain_text: Option<String>,
}

impl Lyrics {
    pub fn word_format(source: &str) -> Option<&'static str> {
        word_timed::format(source)
    }
    pub fn parse_text(source: &str, duration: f64) -> Result<Self> {
        if word_timed::recognizes(source) {
            return word_timed::parse(source, duration);
        }
        if source
            .trim()
            .trim_start_matches('\u{feff}')
            .starts_with('<')
        {
            return ttml::parse(source, duration);
        }
        let timed = source.lines().any(|line| {
            line.trim()
                .trim_start_matches('\u{feff}')
                .strip_prefix('[')
                .and_then(|body| body.split_once(']'))
                .is_some_and(|(stamp, _)| timestamp(stamp).is_ok())
        });
        if timed {
            Self::parse_lrc(source, duration)
        } else {
            ensure!(duration.is_finite() && duration > 0., "无效的歌曲时长");
            Ok(Self {
                lines: Vec::new(),
                plain_text: Some(source.trim().trim_start_matches('\u{feff}').to_owned())
                    .filter(|text| !text.is_empty()),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn different_start_times_preserve_overlapping_elrc_vocals() {
        let lyrics =
            Lyrics::parse_lrc("[00:10]<00:10>甲<00:14>\n[00:12]<00:12>乙<00:16>", 20.).unwrap();
        assert_eq!(lyrics.lines[0].vocal_end(), 14.);
        assert_eq!(lyrics.selected_lines(13., 2, 0.5), vec![0, 1]);
        assert_eq!(lyrics.active_line(13.), Some(0));
        assert_eq!(lyrics.selected_lines(14.5, 2, 0.5), vec![1]);
        assert_eq!(lyrics.active_line(15.), Some(1));
        assert_eq!(lyrics.selected_lines(11., 2, 0.5), vec![0]);
    }
    #[test]
    fn plain_embedded_lyrics_do_not_receive_invented_timestamps() {
        let lyrics = Lyrics::parse_text("第一行\n第二行", 60.).unwrap();
        assert!(lyrics.lines.is_empty());
        assert_eq!(lyrics.plain_text.as_deref(), Some("第一行\n第二行"));
        assert!(
            Lyrics::parse_text("[00:01.000]同步歌词", 60.)
                .unwrap()
                .plain_text
                .is_none()
        );
    }
    #[test]
    fn vocal_end_markers_preserve_gaps_and_same_time_lines_can_overlap() {
        let lyrics =
            Lyrics::parse_lrc("[00:01]<00:02>甲<00:03>\n[00:06]乙\n[00:06]丙", 10.).unwrap();
        assert_eq!(lyrics.lines[0].vocal_start(), 2.);
        assert_eq!(lyrics.lines[0].vocal_end(), 3.);
        assert_eq!(lyrics.lines[0].end, 6.);
        assert!(lyrics.selected_lines(1.5, 2, 0.5).is_empty());
        assert_eq!(lyrics.selected_lines(3.4, 2, 0.5), vec![0]);
        assert!(lyrics.selected_lines(4., 2, 0.5).is_empty());
        assert_eq!(lyrics.selected_lines(6., 2, 0.5), vec![1, 2]);
        assert_eq!(lyrics.selected_lines(6., 1, 0.5), vec![1]);
    }

    #[test]
    fn enhanced_lrc_word_boundaries_and_seeking() {
        let lyrics = Lyrics::parse_lrc(
            "[00:01.00]<00:01.00>你好<00:02.00>世界<00:03.00>\n[00:04.00]下一行",
            8.,
        )
        .unwrap();
        assert_eq!(lyrics.active_line(0.), None);
        assert_eq!(lyrics.active_line(4.), Some(1));
        assert_eq!(lyrics.active_line(1.5), Some(0));
        assert_eq!(lyrics.lines[0].words[0].progress(1.5), 0.5);
        assert_eq!(lyrics.lines[0].words[1].progress(3.), 1.);
        assert!(!lyrics.lines[1].word_synced);
    }

    #[test]
    fn repeated_timestamps_and_negative_offset() {
        let lyrics = Lyrics::parse_lrc("[offset:-500]\n[00:01][00:05]重唱", 9.).unwrap();
        assert_eq!(lyrics.lines.len(), 2);
        assert_eq!(lyrics.lines[0].start, 0.5);
        assert_eq!(lyrics.lines[0].end, 4.5);
        assert_eq!(lyrics.lines[1].end, 8.5);
    }

    #[test]
    fn enhanced_lrc_prefix_inherits_line_time_and_marker_padding_is_not_a_word() {
        let lyrics = Lyrics::parse_text("[00:01]甲<00:02>乙<00:03>", 8.).unwrap();
        let line = &lyrics.lines[0];
        assert!(line.word_synced);
        assert_eq!(line.text(), "甲乙");
        assert_eq!((line.words[0].start, line.words[0].end), (1., 2.));
        assert_eq!((line.words[1].start, line.words[1].end), (2., 3.));
        let padded = Lyrics::parse_text("[00:01]  <00:02>甲<00:03>", 8.).unwrap();
        assert!(padded.lines[0].word_synced);
        assert_eq!(padded.lines[0].words.len(), 1);
        assert_eq!(padded.lines[0].vocal_start(), 2.);
    }

    #[test]
    fn repeated_enhanced_lrc_shifts_word_times_gaps_and_closing_marker() {
        let lyrics = Lyrics::parse_text(
            "[offset:-500]\n[00:01][00:10]甲<00:02><00:03>乙<00:04>",
            20.,
        )
        .unwrap();
        assert_eq!(lyrics.lines.len(), 2);
        for (line, start) in lyrics.lines.iter().zip([0.5, 9.5]) {
            assert!(line.word_synced);
            assert_eq!(line.text(), "甲乙");
            assert_eq!(
                (line.words[0].start, line.words[0].end),
                (start, start + 1.)
            );
            assert_eq!(
                (line.words[1].start, line.words[1].end),
                (start + 2., start + 3.)
            );
            assert_eq!(line.vocal_end(), start + 3.);
            assert_eq!(line.words[0].progress(start + 0.5), 0.5);
        }
    }

    #[test]
    fn repeated_enhanced_lrc_uses_first_written_stamp_before_sorting() {
        let lyrics = Lyrics::parse_text("[00:10][00:01][00:10]<00:10.5>甲<00:11.5>", 20.).unwrap();
        assert_eq!(
            lyrics
                .lines
                .iter()
                .map(|line| line.vocal_start())
                .collect::<Vec<_>>(),
            vec![1.5, 10.5, 10.5]
        );
        assert_eq!(lyrics.selected_lines(11., 2, 0.), vec![1, 2]);
        let fractional = Lyrics::parse_text("[00:10.1][00:01.1]<00:10.1>甲<00:11.1>", 20.).unwrap();
        for line in &fractional.lines {
            assert_eq!(line.vocal_start(), line.start);
        }
    }

    #[test]
    fn literal_angle_brackets_stay_text_and_bad_word_times_are_rejected() {
        let plain = Lyrics::parse_text("[00:01]甲<乙> <3", 8.).unwrap();
        assert!(!plain.lines[0].word_synced);
        assert_eq!(plain.lines[0].text(), "甲<乙> <3");
        for source in [
            "[00:01]甲<00:00>乙",
            "[00:01]甲<00:02>乙<00:01>",
            "[00:01]甲<00:99>乙",
            "[00:01]甲<00:02",
        ] {
            assert!(Lyrics::parse_text(source, 8.).is_err(), "{source}");
        }
    }

    #[test]
    fn invalid_word_order_and_nonfinite_times_are_rejected() {
        assert!(Lyrics::parse_lrc("[00:01]<00:02>甲<00:01>乙", 5.).is_err());
        assert!(Lyrics::parse_lrc("[00:NaN]错误", 5.).is_err());
    }
}

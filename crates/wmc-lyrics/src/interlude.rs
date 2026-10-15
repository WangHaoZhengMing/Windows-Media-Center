//! Instrumental gaps derived from the union of all audible lyric voices.
use super::Lyrics;
const BREATH_PERIOD: f64 = 3.2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interlude {
    pub start: f64,
    pub end: f64,
    pub next_line: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct Dots {
    pub alpha: f32,
    pub pulses: [f32; 3],
    pub blue: [f32; 3],
}

impl Interlude {
    pub fn sample(self, seconds: f64, reduced: bool) -> Option<Dots> {
        if seconds < self.start || seconds >= self.end {
            return None;
        }
        let smooth = |value: f64| {
            let value = value.clamp(0., 1.) as f32;
            value * value * (3. - 2. * value)
        };
        let fade_in = if self.start == 0. {
            1.
        } else {
            smooth((seconds - self.start) / 0.5)
        };
        let alpha = fade_in * smooth((self.end - seconds) / 0.65);
        let pulse = if reduced {
            0.5
        } else {
            let phase = (seconds - self.start) / BREATH_PERIOD * std::f64::consts::TAU;
            ((1. + phase.cos()) * 0.5) as f32
        };
        let pulses = [pulse; 3];
        // Complete the left-to-right color progression before the final fade.
        let progress =
            ((seconds - self.start) / (self.end - self.start - 0.65).max(0.001)).clamp(0., 1.);
        let blue = std::array::from_fn(|index| smooth(progress * 3. - index as f64));
        Some(Dots {
            alpha,
            pulses,
            blue,
        })
    }
}

impl Lyrics {
    /// Includes intro and internal gaps, but not an outro with no upcoming lyric.
    /// Build once with the layout; never sort the entire song per frame.
    pub fn interludes(&self, minimum: f64) -> Vec<Interlude> {
        let mut vocals: Vec<_> = self
            .lines
            .iter()
            .enumerate()
            .flat_map(|(index, line)| {
                line.words
                    .iter()
                    .filter(|word| !word.text.trim().is_empty() && word.end > word.start)
                    .map(move |word| (word.start, word.end, index))
            })
            .collect();
        vocals.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut covered_until = 0.;
        let mut gaps = Vec::new();
        for (start, end, index) in vocals {
            if start - covered_until >= minimum.max(0.) && start > covered_until {
                let line = &self.lines[index];
                let next_line = if line.background {
                    self.lines
                        .iter()
                        .position(|parent| {
                            !parent.background && line.group.is_some() && parent.group == line.group
                        })
                        .unwrap_or(index)
                } else {
                    index
                };
                gaps.push(Interlude {
                    start: covered_until,
                    end: start,
                    next_line,
                });
            }
            covered_until = covered_until.max(end);
        }
        gaps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn intro_and_middle_gaps_exclude_overlapping_voices_and_outro() {
        let lyrics = Lyrics::parse_lrc(
            "[00:05]<00:05>甲<00:10>\n[00:08]<00:08>乙<00:14>\n[00:20]<00:20>丙<00:24>",
            30.,
        )
        .unwrap();
        assert_eq!(
            lyrics.interludes(3.),
            vec![
                Interlude {
                    start: 0.,
                    end: 5.,
                    next_line: 0
                },
                Interlude {
                    start: 14.,
                    end: 20.,
                    next_line: 2
                }
            ]
        );
        assert!(Lyrics::default().interludes(3.).is_empty());
        assert!(
            Lyrics::parse_text("无同步歌词", 30.)
                .unwrap()
                .interludes(3.)
                .is_empty()
        );
    }

    #[test]
    fn background_vocals_extend_the_sung_interval() {
        let mut lyrics = Lyrics::parse_lrc(
            "[00:05]<00:05>甲<00:10>\n[00:08]<00:08>乙<00:14>\n[00:20]<00:20>丙<00:24>",
            30.,
        )
        .unwrap();
        lyrics.lines[1].background = true;
        lyrics.lines[0].group = Some(0);
        lyrics.lines[1].group = Some(0);
        assert_eq!(lyrics.interludes(3.)[1].start, 14.);
    }

    #[test]
    fn seeking_and_pausing_are_stateless_and_fades_meet_vocal_boundaries() {
        let gap = Interlude {
            start: 10.,
            end: 20.,
            next_line: 0,
        };
        assert!(gap.sample(9., false).is_none());
        assert!(gap.sample(20., false).is_none());
        assert_eq!(gap.sample(10., false).unwrap().alpha, 0.);
        assert!(gap.sample(19.999, false).unwrap().alpha < 0.001);
        let before = gap.sample(12., false).unwrap();
        let _ = gap.sample(19., false);
        let paused_or_sought = gap.sample(12., false).unwrap();
        assert_eq!(before.pulses, paused_or_sought.pulses);
        assert_eq!(before.pulses[0], before.pulses[1]);
        assert_eq!(before.pulses[1], before.pulses[2]);
        assert_eq!(gap.sample(12., true).unwrap().pulses, [0.5; 3]);
    }

    #[test]
    fn dots_start_gray_and_turn_blue_in_order_without_resetting_on_seek() {
        let gap = Interlude {
            start: 10.,
            end: 20.,
            next_line: 0,
        };
        assert_eq!(gap.sample(10., false).unwrap().blue, [0.; 3]);
        let early = gap.sample(12., false).unwrap();
        assert!(early.blue[0] > 0.);
        assert_eq!(early.blue[1..], [0.; 2]);
        let middle = gap.sample(15., false).unwrap();
        assert_eq!(middle.blue[0], 1.);
        assert!(middle.blue[1] > 0. && middle.blue[1] < 1.);
        assert_eq!(middle.blue[2], 0.);
        assert_eq!(gap.sample(19.5, false).unwrap().blue, [1.; 3]);
        assert_eq!(gap.sample(12., false).unwrap().blue, early.blue);
        assert_eq!(gap.sample(15., true).unwrap().blue, middle.blue);
    }
}

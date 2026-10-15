//! Plain and enhanced LRC parser.
use super::{Line, Lyrics, Word};
use anyhow::{Context, Result, ensure};

impl Lyrics {
    pub fn parse_lrc(source: &str, duration: f64) -> Result<Self> {
        ensure!(duration.is_finite() && duration > 0., "无效的歌曲时长");
        let offset = source
            .lines()
            .find_map(|line| {
                line.trim()
                    .strip_prefix("[offset:")?
                    .strip_suffix(']')?
                    .parse::<f64>()
                    .ok()
            })
            .unwrap_or(0.)
            / 1000.;
        ensure!(offset.is_finite(), "无效的歌词偏移");
        let mut entries = Vec::new();
        for (number, raw) in source.lines().enumerate() {
            let mut rest = raw.trim().trim_start_matches('\u{feff}');
            let mut starts = Vec::new();
            while rest.starts_with('[') {
                let Some(end) = rest.find(']') else { break };
                let stamp = &rest[1..end];
                if !stamp.as_bytes().first().is_some_and(u8::is_ascii_digit) {
                    break;
                }
                starts.push(timestamp(stamp).with_context(|| format!("歌词第 {} 行", number + 1))?);
                rest = &rest[end + 1..];
            }
            if let Some(anchor) = starts.first().copied() {
                let marks = timed_marks(rest, anchor)
                    .with_context(|| format!("歌词第 {} 行", number + 1))?;
                for start in starts {
                    // Inline times belong to the first occurrence. Repeated line
                    // stamps reuse its relative timing, including silence/end marks.
                    let shifted = marks
                        .iter()
                        .map(|(time, text)| (start + (time - anchor), text.clone()))
                        .collect::<Vec<_>>();
                    entries.push((start, rest.to_owned(), shifted));
                }
            }
        }
        ensure!(!entries.is_empty(), "没有找到带时间戳的 LRC 歌词");
        entries.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut lines = Vec::new();
        for (index, (start, text, marks)) in entries.iter().enumerate() {
            let mut end = entries[index + 1..]
                .iter()
                .find(|(next, _, _)| next > start)
                .map_or(duration, |(next, _, _)| *next)
                .max(*start);
            if !marks.is_empty() {
                ensure!(
                    marks.windows(2).all(|w| w[0].0 <= w[1].0),
                    "逐词时间戳必须递增"
                );
                // Explicit ELRC closing markers can overlap another voice.
                if let Some((closing, text)) = marks.last()
                    && text.is_empty()
                {
                    end = end.max(*closing);
                }
                ensure!(
                    marks
                        .iter()
                        .all(|(time, _)| *time >= *start && *time <= end),
                    "逐词时间超出歌词行"
                );
            }
            let word_synced = !marks.is_empty();
            let words = if word_synced {
                marks
                    .iter()
                    .enumerate()
                    .filter(|(_, (_, text))| !text.is_empty())
                    .map(|(i, (time, text))| Word {
                        text: text.clone(),
                        start: (*time + offset).max(0.),
                        end: (marks.get(i + 1).map_or(end, |mark| mark.0) + offset).max(0.),
                    })
                    .collect()
            } else {
                vec![Word {
                    text: text.clone(),
                    start: (*start + offset).max(0.),
                    end: (end + offset).max(0.),
                }]
            };
            lines.push(Line {
                start: (*start + offset).max(0.),
                end: (end + offset).max(0.),
                words,
                word_synced,
                agents: Vec::new(),
                background: false,
                group: None,
                right_aligned: false,
            });
        }
        Ok(Self {
            lines,
            plain_text: None,
        })
    }
}

// Find timestamp-shaped tags, leaving literal angle brackets in ordinary text.
fn mark_start(text: &str) -> Option<usize> {
    text.match_indices('<').find_map(|(index, _)| {
        let (minutes, _) = text[index + 1..].split_once(':')?;
        (!minutes.is_empty() && minutes.bytes().all(|byte| byte.is_ascii_digit())).then_some(index)
    })
}

fn timed_marks(text: &str, start: f64) -> Result<Vec<(f64, String)>> {
    let Some(first) = mark_start(text) else {
        return Ok(Vec::new());
    };
    let mut marks = Vec::new();
    let prefix = &text[..first];
    if !prefix.trim().is_empty() {
        // A missing opening word marker inherits the line's timestamp.
        marks.push((start, prefix.to_owned()));
    }
    let mut cursor = &text[first..];
    loop {
        let body = &cursor[1..];
        let close = body.find('>').context("逐词时间戳缺少 >")?;
        let time = timestamp(&body[..close])?;
        let body = &body[close + 1..];
        let next = mark_start(body).unwrap_or(body.len());
        marks.push((time, body[..next].to_owned()));
        if next == body.len() {
            break;
        }
        cursor = &body[next..];
    }
    Ok(marks)
}

pub(super) fn timestamp(text: &str) -> Result<f64> {
    let (minutes, seconds) = text.split_once(':').context("时间戳需要 mm:ss.xx")?;
    let minutes = minutes.parse::<u32>()?;
    let seconds = seconds.parse::<f64>()?;
    ensure!(
        seconds.is_finite() && (0. ..60.).contains(&seconds),
        "无效的秒数"
    );
    Ok(f64::from(minutes) * 60. + seconds)
}

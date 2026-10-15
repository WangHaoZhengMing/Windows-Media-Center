//! NetEase YRC and decrypted QQ QRC. Times are absolute milliseconds.
use super::{Line, Lyrics, Word};
use anyhow::{Context, Result, ensure};

fn header(text: &str) -> Option<(&str, &str)> {
    let (stamp, body) = text
        .trim()
        .trim_start_matches('\u{feff}')
        .strip_prefix('[')?
        .split_once(']')?;
    let (start, length) = stamp.split_once(',')?;
    (start.parse::<u64>().is_ok() && length.parse::<u64>().is_ok()).then_some((stamp, body))
}
pub(super) fn recognizes(source: &str) -> bool {
    source.contains("<QrcInfos") || source.lines().any(|line| header(line).is_some())
}
pub(super) fn format(source: &str) -> Option<&'static str> {
    if source.contains("<QrcInfos") {
        return Some("QRC");
    }
    source
        .lines()
        .find_map(header)
        .map(|(_, body)| if body.starts_with('(') { "YRC" } else { "QRC" })
}
fn numbers(text: &str, count: usize) -> Result<Vec<f64>> {
    let values = text
        .split(',')
        .map(str::parse::<u64>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(values.len() == count, "逐字时间字段数量错误");
    Ok(values.into_iter().map(|n| n as f64 / 1000.).collect())
}
pub fn qrc_text(source: &str) -> Result<String> {
    if !source.contains("<QrcInfos") {
        return Ok(source.to_owned());
    }
    // QQ puts literal newlines inside LyricContent. XML attribute normalization
    // would erase row boundaries, so preserve quoted newlines as character refs.
    let mut quote = None;
    let mut output = String::with_capacity(source.len());
    for c in source.chars() {
        if matches!(c, '\'' | '"') {
            if quote == Some(c) {
                quote = None
            } else if quote.is_none() {
                quote = Some(c)
            }
        }
        if quote.is_some() && c == '\n' {
            output.push_str("&#10;");
        } else if quote.is_some() && c == '\r' {
            output.push_str("&#13;");
        } else {
            output.push(c);
        }
    }
    let xml = roxmltree::Document::parse(output.trim().trim_start_matches('\u{feff}'))?;
    Ok(xml
        .descendants()
        .find_map(|node| node.attribute("LyricContent"))
        .context("QRC 缺少 LyricContent")?
        .to_owned())
}
pub(super) fn parse(source: &str, duration: f64) -> Result<Lyrics> {
    ensure!(duration.is_finite() && duration > 0., "无效的歌曲时长");
    let content = qrc_text(source)?;
    let source = content.as_str();
    if !source.lines().any(|row| header(row).is_some()) {
        return Lyrics::parse_lrc(source, duration);
    }
    let mut lines = Vec::new();
    let mut offset = 0.;
    for row in source.lines() {
        if let Some(value) = row
            .trim()
            .trim_start_matches('\u{feff}')
            .strip_prefix("[offset:")
            .and_then(|v| v.strip_suffix(']'))
        {
            offset = value.parse::<f64>()? / 1000.;
            ensure!(offset.is_finite(), "无效的歌词偏移");
        }
    }
    for row in source.lines() {
        let Some((stamp, mut body)) = header(row) else {
            continue;
        };
        let interval = numbers(stamp, 2)?;
        let mut words = Vec::new();
        let yrc = body.starts_with('(');
        while !body.trim().is_empty() {
            let (text, time, rest) = if yrc {
                let (time, remaining) = body
                    .strip_prefix('(')
                    .context("YRC 缺少词时间")?
                    .split_once(')')
                    .context("YRC 时间未闭合")?;
                // A literal parenthesis is text unless it starts a numeric time tuple.
                let next = remaining
                    .match_indices('(')
                    .find(|(i, _)| {
                        remaining[i + 1..]
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_ascii_digit())
                            && remaining[i + 1..]
                                .split_once(')')
                                .is_some_and(|(t, _)| numbers(t, 3).is_ok())
                    })
                    .map_or(remaining.len(), |(i, _)| i);
                (&remaining[..next], time, &remaining[next..])
            } else {
                let marker = body
                    .match_indices('(')
                    .find(|(i, _)| {
                        body[i + 1..]
                            .split_once(')')
                            .is_some_and(|(t, _)| numbers(t, 2).is_ok())
                    })
                    .map(|(i, _)| i)
                    .context("QRC 缺少词时间")?;
                let (time, rest) = body[marker + 1..]
                    .split_once(')')
                    .context("QRC 时间未闭合")?;
                (&body[..marker], time, rest)
            };
            let time = numbers(time, if yrc { 3 } else { 2 })?;
            let start = (time[0] - offset).max(0.);
            let end = (time[0] + time[1] - offset).max(0.);
            if let Some(word) = words
                .last_mut()
                .filter(|word: &&mut Word| word.start == start && word.end == end)
            {
                word.text.push_str(text);
                body = rest;
                continue;
            }
            ensure!(
                words
                    .last()
                    .is_none_or(|word: &Word| start >= word.start && start + 0.002 >= word.end),
                "逐字时间倒退或重叠"
            );
            if !text.is_empty() {
                words.push(Word {
                    text: text.to_owned(),
                    start,
                    end,
                });
            }
            body = rest;
        }
        if words.is_empty() {
            continue;
        }
        let start = (interval[0] - offset).max(0.).min(words[0].start);
        let end = (interval[0] + interval[1] - offset).max(words.last().unwrap().end);
        ensure!(start <= duration + 1., "歌词超出歌曲时长");
        lines.push(Line {
            start,
            end,
            words,
            word_synced: true,
            agents: vec![],
            background: false,
            group: None,
            right_aligned: false,
        });
    }
    ensure!(!lines.is_empty(), "没有可用的逐字歌词");
    lines.sort_by(|a, b| a.start.total_cmp(&b.start));
    Ok(Lyrics {
        lines,
        plain_text: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn yrc_and_qrc_preserve_word_duration_gaps_and_overlapping_rows() {
        for text in [
            "[1000,3000](1000,400,0)甲(2000,500,0)乙\n[2000,2000](2000,500,0)丙",
            "[1000,3000]甲(1000,400)乙(2000,500)\n[2000,2000]丙(2000,500)",
        ] {
            let parsed = Lyrics::parse_text(text, 10.).unwrap();
            assert_eq!(parsed.lines.len(), 2);
            assert_eq!(parsed.lines[0].text(), "甲乙");
            assert_eq!(parsed.lines[0].words[0].end, 1.4);
            assert_eq!(parsed.lines[0].words[1].start, 2.);
            assert!(parsed.lines.iter().all(|line| line.word_synced));
        }
    }
    #[test]
    fn xml_offset_unicode_and_invalid_times() {
        let plain = "<QrcInfos><LyricInfo LyricContent=\"[00:01]甲\n[00:02]乙\"/></QrcInfos>";
        assert_eq!(qrc_text(plain).unwrap(), "[00:01]甲\n[00:02]乙");
        let lines = Lyrics::parse_text(plain, 10.).unwrap();
        assert_eq!(lines.lines.len(), 2);
        assert!(!lines.lines[0].word_synced);
        let parsed = Lyrics::parse_text("<QrcInfos><LyricInfo LyricContent=\"[1000,2000]你&amp;(1000,500)好(1600,500)\"/></QrcInfos>", 10.).unwrap();
        assert_eq!(parsed.lines[0].text(), "你&好");
        let parsed =
            Lyrics::parse_text("[offset:100]\n[1000,2000](1000,500,0)🎵(1600,500,0)好", 10.)
                .unwrap();
        assert_eq!(parsed.lines[0].words[0].start, 0.9);
        assert!(Lyrics::parse_text("[1000,2000](2000,500,0)甲(1000,500,0)乙", 10.).is_err());
        assert!(Lyrics::parse_text("[1000,2000]甲(1000,no)", 10.).is_err());
    }
}

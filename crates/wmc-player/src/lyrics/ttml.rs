//! Deliberate TTML subset: media clock/offset times and parallel text containers.
use crate::lyrics::{Line, Lyrics, Word};
use anyhow::{Context, Result, bail, ensure};
use roxmltree::Node;

const TT: &str = "http://www.w3.org/ns/ttml";
const META: &str = "http://www.w3.org/ns/ttml#metadata";
const PARAM: &str = "http://www.w3.org/ns/ttml#parameter";
const XML: &str = "http://www.w3.org/XML/1998/namespace";
const ITUNES: &str = "http://music.apple.com/lyric-ttml-internal";
const AMLL: &str = "http://www.example.com/ns/amll";

fn amll_document(node: Node<'_, '_>) -> bool {
    node.document()
        .root_element()
        .namespaces()
        .any(|namespace| namespace.uri() == AMLL)
}

fn text_element(node: Node<'_, '_>, name: &str) -> bool {
    node.tag_name().name() == name
        && (node.tag_name().namespace() == Some(TT)
            || (node.tag_name().namespace().is_none_or(str::is_empty) && amll_document(node)))
}

fn time(value: &str) -> Result<f64> {
    let value = value.trim();
    let result = if value.contains(':') {
        let parts: Vec<_> = value.split(':').collect();
        ensure!(
            (2..=3).contains(&parts.len()),
            "不支持的 TTML 时钟时间：{value}"
        );
        let mut result = 0.;
        for (index, part) in parts.iter().enumerate() {
            let number: f64 = part.parse().context("无效的 TTML 时钟时间")?;
            ensure!(
                number >= 0. && (index == 0 || number < 60.),
                "无效的 TTML 时钟字段"
            );
            if index + 1 < parts.len() {
                ensure!(number.fract() == 0., "TTML 小数只能用于秒");
            }
            result = result * 60. + number;
        }
        result
    } else if let Ok(seconds) = value.parse::<f64>() {
        seconds
    } else {
        let units = [("ms", 0.001), ("s", 1.), ("m", 60.), ("h", 3600.)];
        let (number, multiplier) = units
            .iter()
            .find_map(|(unit, multiplier)| {
                value.strip_suffix(unit).map(|number| (number, *multiplier))
            })
            .context("仅支持 TTML 秒/毫秒/分/小时和时钟时间，不支持帧或滴答时间")?;
        number.parse::<f64>()? * multiplier
    };
    ensure!(result.is_finite() && result >= 0., "无效的 TTML 时间");
    Ok(result)
}

fn interval(node: Node<'_, '_>, duration: f64) -> Result<(f64, f64)> {
    // Apple lyric documents explicitly identify their absolute media timeline.
    // Standard TTML continues to use parent-relative begin/end values.
    let apple = amll_document(node)
        || node
            .document()
            .root_element()
            .attribute((ITUNES, "timing"))
            .is_some();
    let mut start = 0.;
    let mut end = duration;
    let mut chain: Vec<_> = node.ancestors().filter(Node::is_element).collect();
    chain.reverse();
    for element in chain {
        let base = if apple { 0. } else { start };
        let begin = element
            .attribute("begin")
            .map(time)
            .transpose()?
            .unwrap_or(start - base);
        start = base + begin;
        if let Some(value) = element.attribute("end") {
            end = end.min(base + time(value)?);
        }
        if let Some(value) = element.attribute("dur") {
            end = end.min(start + time(value)?);
        }
        ensure!(
            start.is_finite() && end.is_finite() && start <= end,
            "TTML 时间区间超出父容器或歌曲"
        );
    }
    Ok((start, end))
}

fn background(node: Node<'_, '_>) -> bool {
    node.attribute((META, "role")).is_some_and(|roles| {
        roles
            .split_whitespace()
            .any(|role| matches!(role, "x-bg" | "x-background"))
    })
}

// Auxiliary text must not be concatenated into the sung words. The original
// downloaded TTML remains in the cache for future translation/romanization UI.
fn auxiliary(node: Node<'_, '_>) -> bool {
    node.attribute((META, "role")).is_some_and(|roles| {
        roles
            .split_whitespace()
            .any(|role| matches!(role, "x-translation" | "x-roman" | "x-romanization"))
    })
}

fn agents(node: Node<'_, '_>) -> Vec<String> {
    node.ancestors()
        .find_map(|parent| parent.attribute((META, "agent")))
        .map(|ids| ids.split_whitespace().map(str::to_owned).collect())
        .unwrap_or_default()
}

fn words(node: Node<'_, '_>, duration: f64, output: &mut Vec<Word>) -> Result<()> {
    let (start, end) = interval(node, duration)?;
    let preserve = node
        .ancestors()
        .find_map(|parent| parent.attribute((XML, "space")))
        == Some("preserve");
    for child in node.children() {
        if child.is_text() {
            let text = child.text().unwrap_or_default();
            // Ignore XML indentation; preserve meaningful spaces between words.
            if !preserve && text.contains(['\n', '\r']) && text.trim().is_empty() {
                continue;
            }
            let text = if preserve {
                text.to_owned()
            } else {
                let mut normalized = String::new();
                let mut space = false;
                for character in text.chars() {
                    if character.is_whitespace() {
                        if !space {
                            normalized.push(' ');
                        }
                        space = true;
                    } else {
                        normalized.push(character);
                        space = false;
                    }
                }
                normalized
            };
            let text = if text.is_empty() && !child.text().unwrap_or_default().is_empty() {
                " ".to_owned()
            } else {
                text
            };
            if text.is_empty() {
                continue;
            }
            if text.trim().is_empty() {
                if let Some(last) = output.last_mut() {
                    last.text.push_str(&text);
                }
                continue;
            }
            if let Some(last) = output.last_mut()
                && last.start == start
                && last.end == end
            {
                last.text.push_str(&text);
            } else {
                output.push(Word { text, start, end });
            }
        } else if child.is_element() {
            if background(child) || auxiliary(child) {
                continue;
            }
            if text_element(child, "span") {
                words(child, duration, output)?;
            } else if text_element(child, "br") {
                bail!("首版 TTML 不支持显式 br 换行，请拆为独立 p 行");
            } else {
                bail!("不支持的 TTML 文本元素：{}", child.tag_name().name());
            }
        }
    }
    Ok(())
}

pub(crate) fn parse(source: &str, duration: f64) -> Result<Lyrics> {
    ensure!(duration.is_finite() && duration > 0., "无效的歌曲时长");
    let doc = roxmltree::Document::parse(source.trim().trim_start_matches('\u{feff}'))?;
    let root = doc.root_element();
    ensure!(
        root.tag_name().namespace() == Some(TT) && root.tag_name().name() == "tt",
        "不是 TTML 文档"
    );
    ensure!(
        root.attribute((ITUNES, "timing"))
            .is_none_or(|timing| matches!(timing, "Word" | "Line")),
        "不支持的 Apple TTML timing 类型"
    );
    ensure!(
        root.attribute((PARAM, "timeBase"))
            .is_none_or(|value| value == "media"),
        "首版 TTML 仅支持 media 时间基准"
    );
    for node in root.descendants().filter(Node::is_element) {
        ensure!(
            node.attribute("timeContainer")
                .is_none_or(|value| value == "par"),
            "首版 TTML 不支持 seq 时间容器"
        );
        if let Some(roles) = node.attribute((META, "role")) {
            ensure!(
                !roles.split_whitespace().any(|role| role.starts_with("x-")
                    && !matches!(
                        role,
                        "x-bg" | "x-background" | "x-translation" | "x-roman" | "x-romanization"
                    )),
                "不支持的 TTML 扩展角色：{roles}"
            );
        }
    }
    let declared: Vec<_> = root
        .descendants()
        .filter(|node| {
            node.tag_name().namespace() == Some(META) && node.tag_name().name() == "agent"
        })
        .filter_map(|node| node.attribute((XML, "id")))
        .collect();
    let mut lanes = Vec::<String>::new();
    let mut lines = Vec::new();
    for (group, paragraph) in root
        .descendants()
        .filter(|node| text_element(*node, "p"))
        .filter(|node| !node.ancestors().any(auxiliary))
        .enumerate()
    {
        let mut rows = vec![paragraph];
        rows.extend(paragraph.descendants().filter(|node| {
            *node != paragraph
                && background(*node)
                && !node.ancestors().any(auxiliary)
                && !node
                    .ancestors()
                    .skip(1)
                    .take_while(|parent| *parent != paragraph)
                    .any(background)
        }));
        for row in rows {
            let (start, end) = interval(row, duration)?;
            let row_agents = agents(row);
            ensure!(
                row_agents.iter().all(|id| declared.contains(&id.as_str())),
                "TTML 引用了未声明的演唱者"
            );
            for id in row_agents.iter().filter(|_| !background(row)) {
                if !lanes.contains(id) {
                    lanes.push(id.clone());
                }
            }
            let right_aligned = row_agents.len() == 1 && lanes.get(1) == row_agents.first();
            let mut row_words = Vec::new();
            words(row, duration, &mut row_words)?;
            if row_words.iter().all(|word| word.text.trim().is_empty()) {
                continue;
            }
            let word_synced = row
                .descendants()
                .filter(|node| {
                    node.is_element()
                        && *node != row
                        && !node
                            .ancestors()
                            .take_while(|parent| *parent != row)
                            .any(|node| background(node) || auxiliary(node))
                })
                .any(|node| {
                    ["begin", "end", "dur"]
                        .iter()
                        .any(|name| node.attribute(*name).is_some())
                });
            ensure!(
                row_words
                    .windows(2)
                    .all(|pair| pair[0].start <= pair[1].start && pair[0].end <= pair[1].start),
                "同一声部中的词不能倒序或重叠，请用伴唱子行表达另一声部"
            );
            lines.push(Line {
                start,
                end,
                words: row_words,
                word_synced,
                agents: row_agents,
                background: background(row),
                group: Some(group),
                right_aligned,
            });
        }
    }
    ensure!(!lines.is_empty(), "TTML 没有可显示的歌词");
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
    fn apple_absolute_media_times_do_not_accumulate_parent_begins() {
        let source = r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:i="http://music.apple.com/lyric-ttml-internal" i:timing="Word"><body dur="3:48.640"><div begin="42.950" end="1:05.373"><p begin="42.950" end="45.563"><span begin="42.950" end="43.336">甲</span><span begin="43.336" end="45.563">乙</span></p></div></body></tt>"#;
        let lyrics = parse(source, 228.640).unwrap();
        assert_eq!(lyrics.lines[0].start, 42.950);
        assert_eq!(lyrics.lines[0].words[0].start, 42.950);
        assert_eq!(lyrics.lines[0].words[1].start, 43.336);
        assert_eq!(lyrics.lines[0].vocal_end(), 45.563);
        assert!(lyrics.selected_lines(42., 2, 0.).is_empty());
        assert_eq!(lyrics.selected_lines(43., 2, 0.), vec![0]);
    }
    const SAMPLE: &str = include_str!("../../tests/fixtures/duet.ttml");

    #[test]
    fn two_leads_and_background_have_independent_timing_and_stable_layout() {
        let lyrics = Lyrics::parse_text(SAMPLE, 48.).unwrap();
        let selected = lyrics.selected_lines(2.5, 2, 0.5);
        assert_eq!(selected.len(), 3);
        assert_eq!(
            selected
                .iter()
                .filter(|&&index| !lyrics.lines[index].background)
                .count(),
            2
        );
        assert_eq!(lyrics.active_line(0.5), lyrics.active_line(2.5));
        let bg = lyrics
            .lines
            .iter()
            .position(|line| line.background)
            .unwrap();
        assert_eq!(lyrics.lines[bg].vocal_start(), 1.);
        assert_eq!(lyrics.lines[bg].vocal_end(), 7.);
        assert_eq!(lyrics.lines[bg].text(), "轻轻唱着回响");
        assert!(lyrics.selected_lines(6., 2, 0.5).contains(&bg));
        let order = lyrics.display_order();
        assert_eq!(order[0], 0);
        assert_eq!(order[1], bg);
        assert_eq!(order.len(), lyrics.lines.len());
        for line in &lyrics.lines {
            if line.agents == ["voice-b"] {
                assert!(line.right_aligned);
            }
            if line.agents.len() > 1 {
                assert!(!line.right_aligned);
            }
        }
        assert_eq!(lyrics.selected_lines(2.5, 2, 0.5), selected);
        assert!(lyrics.selected_lines(8.5, 2, 0.5).is_empty());
    }

    #[test]
    fn inherited_relative_times_and_namespace_aliases_are_resolved() {
        let source = r#"<t:tt xmlns:t="http://www.w3.org/ns/ttml"><t:body begin="5s"><t:div begin="1s"><t:p begin="2s" dur="4s"><t:span begin="500ms" dur="1s">Hello </t:span><t:span begin="1.5s" dur="1s">world</t:span></t:p></t:div></t:body></t:tt>"#;
        let lyrics = parse(source, 20.).unwrap();
        assert_eq!(lyrics.lines[0].start, 8.);
        assert_eq!(lyrics.lines[0].end, 12.);
        assert_eq!(lyrics.lines[0].words[0].start, 8.5);
        assert_eq!(lyrics.lines[0].words[0].end, 9.5);
        assert_eq!(lyrics.lines[0].text(), "Hello world");
        assert!(lyrics.lines[0].word_synced);
    }

    #[test]
    fn plain_line_whitespace_and_clock_times_are_preserved() {
        let lyrics = parse(r#"<tt xmlns="http://www.w3.org/ns/ttml"><body><p begin="00:00:01.250" end="00:00:03.5">Hello   world</p></body></tt>"#, 10.).unwrap();
        assert_eq!(lyrics.lines[0].text(), "Hello world");
        assert_eq!(lyrics.lines[0].start, 1.25);
        assert!(!lyrics.lines[0].word_synced);
        assert_eq!(time("1250ms").unwrap(), 1.25);
    }

    #[test]
    fn community_ttml_uses_absolute_times_and_mixed_namespaces() {
        let source = r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:amll="http://www.example.com/ns/amll"><body dur="20s"><div xmlns="" begin="5s" end="20s"><p begin="5s" end="8s"><span begin="5s" end="6s">甲</span><span begin="6s" end="8s">乙</span></p></div></body></tt>"#;
        let lyrics = parse(source, 20.).unwrap();
        assert_eq!(lyrics.lines[0].start, 5.);
        assert_eq!(lyrics.lines[0].words[0].start, 5.);
        assert_eq!(lyrics.lines[0].words[1].end, 8.);
        assert_eq!(lyrics.lines[0].text(), "甲乙");
        assert!(lyrics.lines[0].word_synced);
    }

    #[test]
    fn auxiliary_roles_do_not_duplicate_sung_words() {
        let source = r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:ttm="http://www.w3.org/ns/ttml#metadata"><body><p begin="1s" end="3s"><span begin="0s" end="1s">你好</span><span ttm:role="x-translation">Hello</span><span ttm:role="x-roman">ni hao</span></p><p begin="1s" end="3s" ttm:role="x-translation">Hello</p></body></tt>"#;
        let lyrics = parse(source, 10.).unwrap();
        assert_eq!(lyrics.lines.len(), 1);
        assert_eq!(lyrics.lines[0].text(), "你好");
    }

    #[test]
    fn unsupported_or_invalid_timing_is_reported_instead_of_guessed() {
        for value in ["10f", "10t", "NaNs", "-1s", "00:61:00", "0:0:0:12"] {
            assert!(time(value).is_err(), "{value}");
        }
        for body in [
            r#"<p begin="4s" end="3s">bad</p>"#,
            r#"<div timeContainer="seq"><p>bad</p></div>"#,
            r#"<p ttm:agent="unknown">bad</p>"#,
            r#"<p ttm:role="x-unknown">bad</p>"#,
            r#"<p><span begin="1s" end="4s">a</span><span begin="2s" end="5s">b</span></p>"#,
        ] {
            let source = format!(
                r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:ttm="http://www.w3.org/ns/ttml#metadata"><body>{body}</body></tt>"#
            );
            assert!(parse(&source, 10.).is_err(), "{body}");
        }
    }
}

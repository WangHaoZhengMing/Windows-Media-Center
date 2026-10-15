use super::*;
use anyhow::bail;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};

const QQ: &str = "https://u.y.qq.com/cgi-bin/musicu.fcg";
const AMLL: &str = "https://amlldb.bikonoo.com";
const MIRROR: &str =
    "https://raw.githubusercontent.com/amll-dev/amll-ttml-db/refs/heads/main/raw-lyrics";

fn string(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .or_else(|| value.as_u64().map(|id| id.to_string()))
        .unwrap_or_default()
}
fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| items.iter().map(string).filter(|s| !s.is_empty()).collect())
        .unwrap_or_else(|| {
            let text = string(value);
            if text.is_empty() { vec![] } else { vec![text] }
        })
}
fn names(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| string(&item["name"]))
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}
fn list<'a>(value: &'a Value, pointer: &str) -> Result<&'a Vec<Value>> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .context("来源响应缺少歌曲列表")
}
fn check_code(value: &Value, expected: i64) -> Result<()> {
    ensure!(
        value["code"].as_i64() == Some(expected),
        "来源业务错误：code={}",
        value["code"]
    );
    Ok(())
}

fn ascii_json(value: &Value) -> String {
    // Keep the legacy gateway's wire payload ASCII, without changing the query.
    // Supplementary characters need UTF-16 surrogate pairs.
    let mut encoded = String::new();
    for character in serde_json::to_string_pretty(value)
        .expect("JSON value serializes")
        .chars()
    {
        if character.is_ascii() {
            encoded.push(character);
        } else {
            for unit in character.encode_utf16(&mut [0; 2]) {
                encoded.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    encoded
}

pub(super) fn search(
    client: &LyricsClient,
    provider: Provider,
    query: &TrackQuery,
) -> Result<Vec<Candidate>> {
    let text = query.text();
    let mut items = match provider {
        Provider::Local => return Ok(Vec::new()),
        Provider::Netease => {
            let value = client.json(
                client
                    .http
                    .get("https://music.163.com/api/search/get/")
                    .header("Referer", "https://music.163.com/")
                    .query(&[("s", text.as_str()), ("type", "1"), ("limit", "20")]),
            )?;
            check_code(&value, 200)?;
            if value.pointer("/result/songCount").and_then(Value::as_u64) == Some(0) {
                return Ok(vec![]);
            }
            list(&value, "/result/songs")?
                .iter()
                .map(|song| Candidate {
                    provider,
                    id: string(&song["id"]),
                    mid: None,
                    title: string(&song["name"]),
                    artists: names(&song["artists"]),
                    album: string(&song["album"]["name"]),
                    duration_ms: song["duration"].as_u64(),
                    title_aliases: vec![],
                    score: 0,
                })
                .collect::<Vec<_>>()
        }
        Provider::Qq => {
            let value = client.json(client.http.post(QQ).header("Referer", "https://y.qq.com/")
                .header("User-Agent", "QQ/8.9.80")
                .header("Accept-Encoding", "identity")
                .header("Connection", "close")
                .header("Content-Type", "application/json")
                .body(ascii_json(&json!({"comm":{"ct":11,"cv":0,"uin":0,"format":"json"},"req_1":{"module":"music.search.SearchCgiService", "method":"DoSearchForQQMusicDesktop",
                    "param":{"query":text,"search_type":0,"page_num":1,"num_per_page":20}}}))))?;
            check_code(&value, 0)?;
            check_code(&value["req_1"], 0)?;
            list(&value, "/req_1/data/body/song/list")?
                .iter()
                .map(|song| Candidate {
                    provider,
                    id: string(&song["id"]),
                    mid: song["mid"].as_str().map(str::to_owned),
                    title: string(&song["title"]),
                    artists: names(&song["singer"]),
                    album: string(&song["album"]["title"]),
                    duration_ms: song["interval"].as_u64().and_then(|s| s.checked_mul(1000)),
                    title_aliases: vec![],
                    score: 0,
                })
                .collect()
        }
        Provider::Amll => {
            let value = client.json(
                client
                    .http
                    .post(format!("{AMLL}/api/search-lyrics"))
                    // AMLL matches a phrase; concatenating artist + title can
                    // hide otherwise exact matches. Rank artists locally.
                    .json(&json!({"query":query.title.trim()})),
            )?;
            value
                .as_array()
                .context("AMLL 响应不是列表")?
                .iter()
                .take(40)
                .map(|song| Candidate {
                    provider,
                    id: string(&song["file"]),
                    mid: None,
                    title: string(&song["title"]),
                    title_aliases: strings(&song["titles"]),
                    artists: strings(&song["artists"]),
                    album: strings(&song["albums"]).join(" / "),
                    duration_ms: song["durationMs"].as_u64(),
                    score: 0,
                })
                .collect()
        }
    };
    items.retain(|song| !song.id.is_empty() && !song.title.is_empty());
    Ok(items)
}

pub(super) fn fetch(client: &LyricsClient, candidate: &Candidate) -> Result<DownloadedLyrics> {
    if matches!(candidate.provider, Provider::Netease | Provider::Qq) {
        match super::word_online::fetch(client, candidate) {
            Ok(Some(downloaded)) => return Ok(downloaded),
            Ok(None) => {}
            Err(error) => eprintln!(
                "{} 逐字歌词不可用，回退 LRC：{error:#}",
                candidate.provider.name()
            ),
        }
    }
    fetch_line(client, candidate)
}

pub(super) fn fetch_line(client: &LyricsClient, candidate: &Candidate) -> Result<DownloadedLyrics> {
    let (content, translation, source_url, format) = match candidate.provider {
        Provider::Local => bail!("本地歌词无需在线下载"),
        Provider::Netease => {
            ensure!(candidate.id.parse::<u64>().is_ok(), "网易云 ID 无效");
            let url = "https://music.163.com/api/song/lyric";
            let value = client.json(
                client
                    .http
                    .get(url)
                    .header("Referer", "https://music.163.com/")
                    .query(&[
                        ("os", "pc"),
                        ("id", candidate.id.as_str()),
                        ("lv", "-1"),
                        ("kv", "-1"),
                        ("tv", "-1"),
                    ]),
            )?;
            check_code(&value, 200)?;
            (
                string(&value["lrc"]["lyric"]),
                value["tlyric"]["lyric"].as_str().map(str::to_owned),
                format!("{url}?id={}", candidate.id),
                "lrc",
            )
        }
        Provider::Qq => {
            let mid = candidate.mid.as_deref().context("QQ 候选缺少 MID")?;
            ensure!(
                !mid.is_empty() && mid.chars().all(|c| c.is_ascii_alphanumeric()),
                "QQ MID 无效"
            );
            let url = "https://c.y.qq.com/lyric/fcgi-bin/fcg_query_lyric_new.fcg";
            let value = client.json(
                client
                    .http
                    .get(url)
                    .header("Referer", "https://y.qq.com/")
                    .query(&[
                        ("songmid", mid),
                        ("g_tk", "5381"),
                        ("format", "json"),
                        ("inCharset", "utf8"),
                        ("outCharset", "utf8"),
                        ("notice", "0"),
                        ("platform", "yqq"),
                        ("needNewCode", "0"),
                        ("nobase64", "1"),
                    ]),
            )?;
            check_code(&value, 0)?;
            let decode = |text: &str| -> Result<String> {
                // Servers may ignore nobase64; only decode when it is valid UTF-8.
                let decoded = if text.starts_with('[') || text.contains('\n') {
                    text.to_owned()
                } else {
                    STANDARD
                        .decode(text)
                        .ok()
                        .and_then(|bytes| String::from_utf8(bytes).ok())
                        .unwrap_or_else(|| text.to_owned())
                };
                Ok(decode_entities(&decoded))
            };
            let content = decode(value["lyric"].as_str().unwrap_or_default())?;
            let translation = value["trans"].as_str().map(decode).transpose()?;
            (content, translation, format!("{url}?songmid={mid}"), "lrc")
        }
        Provider::Amll => {
            // Treat the filename as an opaque identifier, never as a relative path.
            ensure!(
                candidate.id.ends_with(".ttml")
                    && candidate.id.len() < 240
                    && candidate
                        .id
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')),
                "AMLL 文件标识无效"
            );
            let primary = format!("{AMLL}/raw-lyrics/{}", candidate.id);
            let mirror = format!("{MIRROR}/{}", candidate.id);
            let (content, url) = match client.text(client.http.get(&primary)) {
                Ok(text) if text.trim_start().starts_with('<') && !text.contains("<html") => {
                    (text, primary)
                }
                _ => (client.text(client.http.get(&mirror))?, mirror),
            };
            (content, None, url, "ttml")
        }
    };
    ensure!(
        !content.trim().is_empty(),
        "{} 没有这首歌的歌词",
        candidate.provider.name()
    );
    // Catch error pages before they get accepted as unsynchronized lyric text.
    if content.trim_start().starts_with("<!DOCTYPE html") || content.contains("<html") {
        bail!("来源返回了网页而非歌词");
    }
    Ok(DownloadedLyrics {
        candidate: candidate.clone(),
        content,
        translation,
        source_url,
        format: format.into(),
        fetched_at_unix: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs(),
    })
}

fn decode_entities(source: &str) -> String {
    let mut result = String::new();
    let mut rest = source;
    while let Some(position) = rest.find('&') {
        result.push_str(&rest[..position]);
        rest = &rest[position..];
        let Some(end) = rest.find(';').filter(|end| *end < 16) else {
            result.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..end];
        let character = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => entity
                .strip_prefix("#x")
                .and_then(|n| u32::from_str_radix(n, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|n| n.parse().ok()))
                .and_then(char::from_u32),
        };
        if let Some(character) = character {
            result.push(character);
        } else {
            result.push_str(&rest[..=end]);
        }
        rest = &rest[end + 1..];
    }
    result.push_str(rest);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn qq_wire_encoding_preserves_chinese_surrogates_and_escapes() {
        let value = json!({"query":"晴天 🎵 \\uABCD \""});
        let body = ascii_json(&value);
        assert!(body.is_ascii());
        assert_eq!(serde_json::from_str::<Value>(&body).unwrap(), value);
    }
    #[test]
    fn qq_entities_preserve_unknown_and_decode_only_once() {
        assert_eq!(
            decode_entities("&#91;00:01&#93;你&amp;我&#xA;&amp;lt;&other;"),
            "[00:01]你&我\n&lt;&other;"
        );
    }
}

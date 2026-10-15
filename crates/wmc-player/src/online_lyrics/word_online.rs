//! Word-synchronized paths are optional: providers fall back to ordinary LRC.
use super::{Candidate, DownloadedLyrics, LyricsClient, Provider, word_crypto};
use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::json;

// Word endpoints can transiently refuse a connection even while ordinary LRC works.
// Retry one idempotent lyric request before falling back; never downgrade cached words.
fn json_with_retry(
    client: &LyricsClient,
    request: crate::network::Request,
) -> Result<serde_json::Value> {
    client
        .json(request.clone())
        .or_else(|_| client.json(request))
}

pub(super) fn fetch(
    client: &LyricsClient,
    candidate: &Candidate,
) -> Result<Option<DownloadedLyrics>> {
    let (content, translation, source_url, format) =
        match candidate.provider {
            Provider::Netease => {
                let id = candidate.id.parse::<u64>()?;
                let url = "https://interface.music.163.com/eapi/song/lyric/v1";
                let body = word_crypto::eapi(
                    "/api/song/lyric/v1",
                    &json!({
                        "id": id, "lv": -1, "tv": -1, "rv": -1, "yv": -1,
                        "e_r": false, "header": {"os":"pc", "appver":"3.1.3.203419"}
                    }),
                );
                let value = json_with_retry(
                    client,
                    client
                        .http
                        .post(url)
                        .header("Content-Type", "application/x-www-form-urlencoded")
                        .header("Referer", "https://music.163.com/")
                        .body(body),
                )?;
                ensure!(value["code"].as_i64() == Some(200), "网易云逐字接口失败");
                let Some(content) = value["yrc"]["lyric"]
                    .as_str()
                    .filter(|s| !s.trim().is_empty())
                else {
                    return Ok(None);
                };
                let content = content.to_owned();
                let translation = value["ytlrc"]["lyric"]
                    .as_str()
                    .or(value["tlyric"]["lyric"].as_str())
                    .filter(|text| !text.is_empty())
                    .map(str::to_owned);
                (content, translation, format!("{url}?id={id}"), "yrc")
            }
            Provider::Qq => {
                let id = candidate.id.parse::<u64>()?;
                let url = "https://u.y.qq.com/cgi-bin/musicu.fcg";
                let value = json_with_retry(client, client.http.post(url)
                .header("Referer", "https://y.qq.com/")
                .json(&json!({"comm":{"ct":19,"cv":2111,"uin":0},"request":{
                    "module":"music.musichallSong.PlayLyricInfo", "method":"GetPlayLyricInfo",
                    "param": {"songID": id, "songName":STANDARD.encode(&candidate.title),
                        "singerName":STANDARD.encode(candidate.artists.join("/")),
                        "albumName":STANDARD.encode(&candidate.album),
                        "interval":candidate.duration_ms.unwrap_or(0)/1000,
                        "crypt":1,"ct":19,"cv":2111,"qrc":1,"qrc_t":0,
                        "lrc_t":0,"trans":1,"trans_t":0,"roma":0,"roma_t":0,"type":0}
                }})))?;
                ensure!(
                    value["code"].as_i64() == Some(0)
                        && value["request"]["code"].as_i64() == Some(0),
                    "QQ 逐字接口失败"
                );
                let data = &value["request"]["data"];
                if data["qrc_t"].as_u64() == Some(0) || data["qrc_t"].as_str() == Some("0") {
                    return Ok(None);
                }
                let content =
                    word_crypto::decrypt_qrc(data["lyric"].as_str().context("这首歌没有 QRC")?)?;
                let translation = data["trans"]
                    .as_str()
                    .filter(|text| !text.is_empty())
                    .and_then(|text| word_crypto::decrypt_qrc(text).ok())
                    .and_then(|text| crate::lyrics::qrc_text(&text).ok());
                (
                    content,
                    translation,
                    format!("{url}?songid={id}&module=music.musichallSong.PlayLyricInfo"),
                    "qrc",
                )
            }
            _ => anyhow::bail!("该来源没有 YRC/QRC 接口"),
        };
    let downloaded = DownloadedLyrics {
        candidate: candidate.clone(),
        content,
        translation,
        source_url,
        format: format.into(),
        fetched_at_unix: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs(),
    };
    // A successful response is not proof that word timing exists.
    ensure!(
        downloaded
            .parse(
                candidate
                    .duration_ms
                    .map_or(24. * 3600., |n| n as f64 / 1000.)
            )?
            .lines
            .iter()
            .any(|line| line.word_synced),
        "接口没有返回有效逐字时间"
    );
    Ok(Some(downloaded))
}

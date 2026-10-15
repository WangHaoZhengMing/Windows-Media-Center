//! Anonymous album artwork search, bounded decoding and confirmed-choice cache.
use crate::transport::Client;
use anyhow::{Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, io::Cursor, path::PathBuf};
use wmc_domain::TrackQuery;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoverCandidate {
    pub source: String,
    pub id: String,
    pub album: String,
    pub artist: String,
    pub artwork_url: String,
    pub release_date: String,
    pub score: i32,
}
#[derive(Default)]
pub struct CoverResults {
    pub candidates: Vec<CoverCandidate>,
    pub errors: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct DownloadedCover {
    pub candidate: CoverCandidate,
    pub fetched_at_unix: u64,
    data: String,
}
impl DownloadedCover {
    pub fn encoded_bytes(&self) -> Result<Vec<u8>> {
        Ok(STANDARD.decode(&self.data)?)
    }
    pub fn from_bytes(candidate: CoverCandidate, bytes: &[u8], fetched_at_unix: u64) -> Self {
        Self {
            candidate,
            data: STANDARD.encode(bytes),
            fetched_at_unix,
        }
    }
    pub fn image(&self) -> Result<RgbaImage> {
        decode(&STANDARD.decode(&self.data)?)
    }
}

#[derive(Clone)]
pub struct CoversClient {
    http: Client,
}
impl CoversClient {
    pub fn new() -> Result<Self> {
        Ok(Self {
            http: Client::shared(),
        })
    }
    pub fn search(&self, query: &TrackQuery) -> Result<CoverResults> {
        ensure!(
            !query.album.trim().is_empty() || !query.title.trim().is_empty(),
            "歌曲和专辑名为空"
        );
        let mut results = CoverResults::default();
        std::thread::scope(|scope| {
            let jobs: Vec<_> = ["网易云", "QQ 音乐", "iTunes"]
                .into_iter()
                .map(|source| {
                    (
                        source,
                        scope.spawn(move || self.search_source(source, query)),
                    )
                })
                .collect();
            for (source, job) in jobs {
                match job.join() {
                    Ok(Ok(mut candidates)) => results.candidates.append(&mut candidates),
                    Ok(Err(error)) => results.errors.push(format!("{source}：{error:#}")),
                    Err(_) => results.errors.push(format!("{source}：搜索任务失败")),
                }
            }
        });
        rank(query, &mut results.candidates);
        let mut seen = HashSet::new();
        results.candidates.retain(|candidate| {
            !candidate.artwork_url.is_empty()
                && seen.insert((candidate.source.clone(), candidate.id.clone()))
        });
        results.candidates.truncate(20);
        Ok(results)
    }
    fn search_source(&self, source: &str, query: &TrackQuery) -> Result<Vec<CoverCandidate>> {
        let has_album = !query.album.trim().is_empty();
        let term = format!(
            "{} {}",
            if has_album {
                &query.album
            } else {
                &query.title
            },
            query.artist
        )
        .trim()
        .to_owned();
        let value = match source {
            "网易云" => {
                let value = self.http.json(
                    self.http
                        .get("https://music.163.com/api/search/get/")
                        .header("Referer", "https://music.163.com/")
                        .query(&[
                            ("s", term.as_str()),
                            ("type", if has_album { "10" } else { "1" }),
                            ("limit", "12"),
                        ]),
                )?;
                ensure!(
                    value["code"].as_i64() == Some(200),
                    "专辑搜索失败：{}",
                    value["code"]
                );
                value
            }
            "QQ 音乐" => {
                let value = self.http.json(self.http.post("https://u.y.qq.com/cgi-bin/musicu.fcg")
                    .header("Referer", "https://y.qq.com/").header("User-Agent", "QQ/8.9.80")
                    .json(&json!({"comm":{"ct":11,"cv":0,"uin":0,"format":"json"},"req_1":{"module":"music.search.SearchCgiService","method":"DoSearchForQQMusicDesktop","param":{"query":query.text(),"search_type":0,"num_per_page":12,"page_num":1}}})))?;
                ensure!(
                    value["code"].as_i64() == Some(0) && value["req_1"]["code"].as_i64() == Some(0),
                    "封面搜索失败：{}",
                    value["req_1"]["code"]
                );
                value
            }
            _ => self
                .http
                .json(self.http.get("https://itunes.apple.com/search").query(&[
                    ("term", term.as_str()),
                    ("media", "music"),
                    ("entity", if has_album { "album" } else { "song" }),
                    ("limit", "12"),
                    ("country", "TW"),
                ]))?,
        };
        parse_candidates(source, has_album, &value)
    }
    pub fn fetch(&self, candidate: &CoverCandidate) -> Result<DownloadedCover> {
        let url = url::Url::parse(&candidate.artwork_url)?;
        ensure!(url.scheme() == "https", "封面地址必须使用 HTTPS");
        let bytes = self.http.bytes(
            self.http.get(url).header("Referer", "https://y.qq.com/"),
            8 * 1024 * 1024,
        )?;
        decode(&bytes)?;
        Ok(DownloadedCover {
            candidate: candidate.clone(),
            data: STANDARD.encode(bytes),
            fetched_at_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        })
    }
}
fn text(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .or_else(|| value.as_u64().map(|n| n.to_string()))
        .unwrap_or_default()
}
fn parse_candidates(source: &str, has_album: bool, value: &Value) -> Result<Vec<CoverCandidate>> {
    let pointer = match source {
        "网易云" if has_album => "/result/albums",
        "网易云" => "/result/songs",
        "QQ 音乐" => "/req_1/data/body/song/list",
        _ => "/results",
    };
    let Some(items) = value.pointer(pointer).and_then(Value::as_array) else {
        if value.pointer("/result/albumCount").and_then(Value::as_u64) == Some(0)
            || value.pointer("/result/songCount").and_then(Value::as_u64) == Some(0)
            || value["resultCount"].as_u64() == Some(0)
        {
            return Ok(vec![]);
        }
        anyhow::bail!("来源缺少封面候选列表");
    };
    let candidates = items
        .iter()
        .filter_map(|item| {
            let (id, album, artist, artwork_url, release_date) = match source {
                "网易云" => {
                    let album = if has_album { item } else { &item["album"] };
                    let raw = text(&album["picUrl"]);
                    (
                        text(&album["id"]),
                        text(&album["name"]),
                        text(&album["artist"]["name"]),
                        raw.replacen("http://", "https://", 1),
                        String::new(),
                    )
                }
                "QQ 音乐" => {
                    let id = text(&item["album"]["mid"]);
                    let artist = item["singer"]
                        .as_array()
                        .map(|s| {
                            s.iter()
                                .map(|s| text(&s["name"]))
                                .collect::<Vec<_>>()
                                .join(" / ")
                        })
                        .unwrap_or_default();
                    let url = if id.is_empty() {
                        String::new()
                    } else {
                        format!("https://y.gtimg.cn/music/photo_new/T002R500x500M000{id}.jpg")
                    };
                    (
                        id,
                        text(&item["album"]["title"]),
                        artist,
                        url,
                        String::new(),
                    )
                }
                _ => (
                    text(&item["collectionId"]),
                    text(&item["collectionName"]),
                    text(&item["artistName"]),
                    text(&item["artworkUrl100"])
                        .replace("100x100bb", "600x600bb")
                        .replacen("http://", "https://", 1),
                    text(&item["releaseDate"]).chars().take(10).collect(),
                ),
            };
            if id.is_empty() || artwork_url.is_empty() {
                None
            } else {
                Some(CoverCandidate {
                    source: source.into(),
                    id,
                    album,
                    artist,
                    artwork_url,
                    release_date,
                    score: 0,
                })
            }
        })
        .collect();
    Ok(candidates)
}
fn normalized(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
fn rank(query: &TrackQuery, candidates: &mut [CoverCandidate]) {
    for candidate in candidates.iter_mut() {
        let album = normalized(&query.album);
        let artist = normalized(&query.artist);
        let ca = normalized(&candidate.album);
        let cr = normalized(&candidate.artist);
        candidate.score = if !album.is_empty() && album == ca {
            100
        } else if !album.is_empty() && (ca.contains(&album) || album.contains(&ca)) {
            30
        } else {
            0
        };
        candidate.score += if !artist.is_empty() && artist == cr {
            60
        } else if !artist.is_empty() && (cr.contains(&artist) || artist.contains(&cr)) {
            20
        } else {
            0
        };
    }
    candidates.sort_by_key(|c| std::cmp::Reverse(c.score));
}
fn decode(bytes: &[u8]) -> Result<RgbaImage> {
    let mut reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode()?.to_rgba8();
    Ok(if image.width() > 1200 || image.height() > 1200 {
        image::imageops::thumbnail(&image, 1200, 1200)
    } else {
        image
    })
}

pub struct CoverCache {
    root: PathBuf,
}
impl CoverCache {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    pub fn default_location() -> Result<Self> {
        Ok(Self::new(
            wmc_platform::paths::app_cache_root()?.join("covers"),
        ))
    }
    fn path(&self, query: &TrackQuery) -> PathBuf {
        let name = if query.album.trim().is_empty() {
            &query.title
        } else {
            &query.album
        };
        let hash =
            Sha256::digest(serde_json::to_vec(&(name, &query.artist)).expect("strings serialize"));
        self.root.join(format!("{hash:x}.json"))
    }
    pub fn load(&self, query: &TrackQuery) -> Result<Option<DownloadedCover>> {
        let path = self.path(query);
        if !path.is_file() {
            return Ok(None);
        }
        ensure!(
            std::fs::metadata(&path)?.len() <= 12 * 1024 * 1024,
            "封面缓存过大"
        );
        Ok(Some(serde_json::from_slice(&std::fs::read(path)?)?))
    }
    pub fn save(&self, query: &TrackQuery, cover: &DownloadedCover) -> Result<()> {
        std::fs::create_dir_all(&self.root)?;
        let path = self.path(query);
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        std::fs::write(&temporary, serde_json::to_vec(cover)?)?;
        std::fs::rename(temporary, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn album_art_candidates_use_album_ids_and_secure_urls() {
        let value = json!({"result":{"albums":[{"id":12,"name":"专辑","artist":{"name":"歌手"},"picUrl":"http://p1.music.126.net/art.jpg"}]}});
        let c = parse_candidates("网易云", true, &value).unwrap();
        assert_eq!(c[0].id, "12");
        assert_eq!(c[0].artwork_url, "https://p1.music.126.net/art.jpg");
        assert_eq!(c[0].artist, "歌手");
        let apple = json!({"results":[{"collectionId":13,"collectionName":"专辑","artistName":"歌手","artworkUrl100":"https://is1-ssl.mzstatic.com/a/100x100bb.jpg"}]});
        assert!(
            parse_candidates("iTunes", true, &apple).unwrap()[0]
                .artwork_url
                .ends_with("600x600bb.jpg")
        );
    }
    #[test]
    fn confirmed_cover_cache_is_shared_by_album_and_artist_not_other_versions() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CoverCache::new(dir.path().into());
        let query = TrackQuery {
            title: "歌曲甲".into(),
            artist: "歌手".into(),
            album: "专辑".into(),
            duration_ms: None,
        };
        let mut same = query.clone();
        same.title = "歌曲乙".into();
        assert_eq!(cache.path(&query), cache.path(&same));
        same.album = "专辑 Live".into();
        assert_ne!(cache.path(&query), cache.path(&same));
        let mut png = Cursor::new(vec![]);
        RgbaImage::new(4, 4)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let cover = DownloadedCover {
            candidate: CoverCandidate {
                source: "test".into(),
                id: "1".into(),
                album: "专辑".into(),
                artist: "歌手".into(),
                artwork_url: "https://example.com/a.png".into(),
                release_date: "".into(),
                score: 0,
            },
            data: STANDARD.encode(png.into_inner()),
            fetched_at_unix: 1,
        };
        cache.save(&query, &cover).unwrap();
        cache.save(&query, &cover).unwrap();
        assert_eq!(
            cache
                .load(&query)
                .unwrap()
                .unwrap()
                .image()
                .unwrap()
                .width(),
            4
        );
    }
}

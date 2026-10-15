//! Shared JSON catalog: audio MD5 first, then title and artist, with asset files.
use crate::{
    online_covers::DownloadedCover,
    online_lyrics::{DownloadedLyrics, SearchQuery},
};
use anyhow::{Context, Result, ensure};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaIdentity {
    pub md5: Option<String>,
    pub audio_path: Option<PathBuf>,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: Option<u64>,
}
impl MediaIdentity {
    /// Hash the audio file bytes in chunks, without decoding or loading it whole.
    pub fn read(query: &SearchQuery, path: Option<&Path>) -> Result<Self> {
        let md5 = path
            .map(|path| -> Result<String> {
                let mut file = File::open(path)?;
                let mut hash = Md5::new();
                let mut buffer = [0; 64 * 1024];
                loop {
                    let count = file.read(&mut buffer)?;
                    if count == 0 {
                        break;
                    }
                    hash.update(&buffer[..count]);
                }
                Ok(format!("{:x}", hash.finalize()))
            })
            .transpose()?;
        Ok(Self {
            md5,
            audio_path: path.map(Path::to_path_buf),
            title: query.title.clone(),
            artist: query.artist.clone(),
            album: query.album.clone(),
            duration_ms: query.duration_ms,
        })
    }
    fn key(&self) -> String {
        self.md5
            .as_ref()
            .filter(|md5| md5.len() == 32 && md5.chars().all(|c| c.is_ascii_hexdigit()))
            .map(|md5| format!("md5-{md5}"))
            .unwrap_or_else(|| {
                let hash = Md5::digest(
                    format!("{}\0{}", normalized(&self.title), normalized(&self.artist)).as_bytes(),
                );
                format!("name-{hash:x}")
            })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetRecord {
    pub file: String,
    pub source: String,
    pub source_id: String,
    pub source_url: String,
    pub format: String,
    pub fetched_at_unix: u64,
    #[serde(default)]
    pub matched_title: String,
    #[serde(default)]
    pub matched_artists: Vec<String>,
    #[serde(default)]
    pub matched_album: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_candidate: Option<crate::online_covers::CoverCandidate>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrackRecord {
    pub identity: MediaIdentity,
    pub lyrics: Option<AssetRecord>,
    pub cover: Option<AssetRecord>,
    pub updated_at_unix: u64,
    #[serde(default)]
    pub local_sources: LocalSources,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub translations: BTreeMap<String, String>,
    /// Last word-lyric capability check; old catalogs default to an immediate upgrade.
    #[serde(default)]
    pub word_lyrics_checked_at: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LocalSources {
    pub lyrics_file: Option<PathBuf>,
    pub cover_file: Option<PathBuf>,
    pub embedded_lyrics: bool,
    pub embedded_cover: bool,
}
#[derive(Serialize, Deserialize)]
struct Catalog {
    version: u32,
    tracks: BTreeMap<String, TrackRecord>,
}
impl Default for Catalog {
    fn default() -> Self {
        Self {
            version: 1,
            tracks: BTreeMap::new(),
        }
    }
}
pub struct MediaCache {
    root: PathBuf,
}
impl MediaCache {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    pub fn default_location() -> Result<Self> {
        Ok(Self::new(
            crate::cache_paths::app_cache_root()?.join("media-cache"),
        ))
    }
    pub fn index_path(&self) -> PathBuf {
        self.root.join("index.json")
    }
    fn catalog(&self) -> Result<Catalog> {
        let path = self.index_path();
        if !path.is_file() {
            return Ok(Catalog::default());
        }
        ensure!(
            std::fs::metadata(&path)?.len() <= 32 * 1024 * 1024,
            "缓存总索引过大"
        );
        let catalog: Catalog =
            serde_json::from_slice(&std::fs::read(&path)?).context("缓存总索引损坏")?;
        ensure!(catalog.version == 1, "不支持的缓存索引版本");
        Ok(catalog)
    }
    fn lock(&self) -> Result<File> {
        std::fs::create_dir_all(&self.root)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.root.join("index.lock"))?;
        lock.lock()?;
        Ok(lock)
    }
    fn write_json(&self, path: &Path, value: &impl Serialize) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // The catalog lock serializes writers across threads and player processes.
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        std::fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
        std::fs::rename(&temporary, path)?;
        Ok(())
    }
    fn update(&self, identity: &MediaIdentity, asset: Option<(bool, AssetRecord)>) -> Result<()> {
        let mut catalog = self.catalog()?;
        let key = identity.key();
        let record = catalog.tracks.entry(key).or_insert_with(|| TrackRecord {
            identity: identity.clone(),
            lyrics: None,
            cover: None,
            updated_at_unix: 0,
            local_sources: LocalSources::default(),
            translations: BTreeMap::new(),
            word_lyrics_checked_at: 0,
        });
        record.identity = identity.clone();
        record.updated_at_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        if let Some((is_cover, asset)) = asset {
            if is_cover {
                record.cover = Some(asset);
            } else {
                record.lyrics = Some(asset);
            }
        }
        self.write_json(&self.index_path(), &catalog)
    }
    pub fn record_track(&self, identity: &MediaIdentity) -> Result<()> {
        let _lock = self.lock()?;
        self.update(identity, None)
    }
    pub fn record_translation(&self, identity: &MediaIdentity, cache_file: &Path) -> Result<()> {
        let relative = cache_file
            .strip_prefix(&self.root)
            .context("译文缓存不在媒体缓存目录中")?;
        ensure!(
            relative
                .components()
                .all(|part| matches!(part, std::path::Component::Normal(_))),
            "译文缓存路径无效"
        );
        let key = cache_file
            .file_stem()
            .and_then(|name| name.to_str())
            .context("译文缓存文件名无效")?;
        let _lock = self.lock()?;
        self.update(identity, None)?;
        let mut catalog = self.catalog()?;
        if let Some(record) = catalog.tracks.get_mut(&identity.key()) {
            record
                .translations
                .insert(key.into(), relative.to_string_lossy().replace('\\', "/"));
        }
        self.write_json(&self.index_path(), &catalog)
    }
    pub fn word_lyrics_upgrade_due(&self, identity: &MediaIdentity) -> Result<bool> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        Ok(self.records(identity)?.first().is_none_or(|record| {
            now.saturating_sub(record.word_lyrics_checked_at) >= 7 * 24 * 3600
        }))
    }
    pub fn mark_word_lyrics_checked(&self, identity: &MediaIdentity) -> Result<()> {
        let _lock = self.lock()?;
        self.update(identity, None)?;
        let mut catalog = self.catalog()?;
        if let Some(record) = catalog.tracks.get_mut(&identity.key()) {
            record.word_lyrics_checked_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
        }
        self.write_json(&self.index_path(), &catalog)
    }

    pub fn record_track_with_local(
        &self,
        identity: &MediaIdentity,
        local: LocalSources,
    ) -> Result<()> {
        let _lock = self.lock()?;
        self.update(identity, None)?;
        let mut catalog = self.catalog()?;
        if let Some(record) = catalog.tracks.get_mut(&identity.key()) {
            record.local_sources = local;
        }
        self.write_json(&self.index_path(), &catalog)
    }
    pub fn save_lyrics(&self, identity: &MediaIdentity, lyrics: &DownloadedLyrics) -> Result<()> {
        let _lock = self.lock()?;
        let file = format!("lyrics/{}.json", identity.key());
        self.write_json(&self.root.join(&file), lyrics)?;
        self.update(
            identity,
            Some((
                false,
                AssetRecord {
                    file,
                    source: lyrics.candidate.provider.name().into(),
                    source_id: lyrics.candidate.id.clone(),
                    source_url: lyrics.source_url.clone(),
                    format: lyrics.format.clone(),
                    fetched_at_unix: lyrics.fetched_at_unix,
                    matched_title: lyrics.candidate.title.clone(),
                    matched_artists: lyrics.candidate.artists.clone(),
                    matched_album: lyrics.candidate.album.clone(),
                    cover_candidate: None,
                },
            )),
        )
    }
    pub fn save_cover(&self, identity: &MediaIdentity, cover: &DownloadedCover) -> Result<()> {
        let _lock = self.lock()?;
        let bytes = cover.encoded_bytes()?;
        let (extension, format) = match image::guess_format(&bytes)? {
            image::ImageFormat::Jpeg => ("jpg", "image/jpeg"),
            image::ImageFormat::Png => ("png", "image/png"),
            image::ImageFormat::WebP => ("webp", "image/webp"),
            _ => anyhow::bail!("不支持的封面文件格式"),
        };
        let file = format!("covers/{}.{extension}", identity.key());
        let path = self.root.join(&file);
        std::fs::create_dir_all(path.parent().context("缺少封面目录")?)?;
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        std::fs::write(&temporary, bytes)?;
        std::fs::rename(temporary, path)?;
        self.update(
            identity,
            Some((
                true,
                AssetRecord {
                    file,
                    source: cover.candidate.source.clone(),
                    source_id: cover.candidate.id.clone(),
                    source_url: cover.candidate.artwork_url.clone(),
                    format: format.into(),
                    fetched_at_unix: cover.fetched_at_unix,
                    matched_title: cover.candidate.album.clone(),
                    matched_artists: vec![cover.candidate.artist.clone()],
                    matched_album: cover.candidate.album.clone(),
                    cover_candidate: Some(cover.candidate.clone()),
                },
            )),
        )
    }
    fn records(&self, identity: &MediaIdentity) -> Result<Vec<TrackRecord>> {
        let catalog = self.catalog()?;
        let title = normalized(&identity.title);
        let artist = normalized(&identity.artist);
        let mut records: Vec<_> = catalog
            .tracks
            .into_values()
            .filter(|record| {
                (identity.md5.is_some() && identity.md5 == record.identity.md5)
                    || (!title.is_empty()
                        && normalized(&record.identity.title) == title
                        && normalized(&record.identity.artist) == artist)
            })
            .collect();
        records.sort_by_key(|record| {
            (
                std::cmp::Reverse(identity.md5.is_some() && identity.md5 == record.identity.md5),
                std::cmp::Reverse(
                    normalized(&identity.album) == normalized(&record.identity.album),
                ),
                identity
                    .duration_ms
                    .zip(record.identity.duration_ms)
                    .map(|(a, b)| a.abs_diff(b))
                    .unwrap_or(u64::MAX),
                std::cmp::Reverse(record.updated_at_unix),
            )
        });
        Ok(records)
    }
    fn read_asset_bytes(&self, asset: &AssetRecord) -> Result<Vec<u8>> {
        let relative = Path::new(&asset.file);
        ensure!(
            relative
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_))),
            "缓存资产路径无效"
        );
        let path = self.root.join(relative);
        ensure!(
            std::fs::metadata(&path)?.len() <= 16 * 1024 * 1024,
            "缓存资产过大"
        );
        Ok(std::fs::read(path)?)
    }
    fn read_asset<T: serde::de::DeserializeOwned>(&self, asset: &AssetRecord) -> Result<T> {
        Ok(serde_json::from_slice(&self.read_asset_bytes(asset)?)?)
    }
    fn read_cover(&self, asset: &AssetRecord) -> Result<DownloadedCover> {
        if asset.format == "image" {
            return self.read_asset(asset);
        }
        let bytes = self.read_asset_bytes(asset)?;
        ensure!(bytes.len() <= 8 * 1024 * 1024, "封面文件过大");
        let candidate = asset
            .cover_candidate
            .clone()
            .context("封面索引缺少候选信息")?;
        let cover = DownloadedCover::from_bytes(candidate, &bytes, asset.fetched_at_unix);
        cover.image()?;
        Ok(cover)
    }
    pub fn load_lyrics(&self, identity: &MediaIdentity) -> Result<Option<DownloadedLyrics>> {
        for record in self.records(identity)? {
            if let Some(asset) = record.lyrics {
                match self.read_asset(&asset) {
                    Ok(lyrics) => return Ok(Some(lyrics)),
                    Err(error) => eprintln!("跳过失效歌词缓存：{error:#}"),
                }
            }
        }
        Ok(None)
    }
    pub fn load_cover(&self, identity: &MediaIdentity) -> Result<Option<DownloadedCover>> {
        for record in self.records(identity)? {
            if let Some(asset) = record.cover {
                match self.read_cover(&asset) {
                    Ok(cover) => return Ok(Some(cover)),
                    Err(error) => eprintln!("跳过失效封面缓存：{error:#}"),
                }
            }
        }
        Ok(None)
    }
}
fn normalized(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::online_lyrics::{Candidate, Provider};
    fn lyrics(id: &str) -> DownloadedLyrics {
        DownloadedLyrics {
            candidate: Candidate {
                provider: Provider::Netease,
                id: id.into(),
                mid: None,
                title: "歌".into(),
                artists: vec!["人".into()],
                album: "专辑".into(),
                duration_ms: Some(10000),
                title_aliases: vec![],
                score: 0,
            },
            format: "lrc".into(),
            content: "[00:01]词".into(),
            translation: None,
            source_url: "https://example.com/lyrics".into(),
            fetched_at_unix: 1,
        }
    }
    #[test]
    fn md5_wins_after_rename_and_name_artist_fallback_handles_new_files() {
        let dir = tempfile::tempdir().unwrap();
        let cache = MediaCache::new(dir.path().join("cache"));
        let path = dir.path().join("audio.wav");
        std::fs::write(&path, b"abc").unwrap();
        let query = SearchQuery {
            title: "歌".into(),
            artist: "人".into(),
            album: "专辑".into(),
            duration_ms: Some(10000),
        };
        let first = MediaIdentity::read(&query, Some(&path)).unwrap();
        assert_eq!(
            first.md5.as_deref(),
            Some("900150983cd24fb0d6963f7d28e17f72")
        );
        cache.save_lyrics(&first, &lyrics("first")).unwrap();
        let mut other = first.clone();
        other.md5 = Some("11111111111111111111111111111111".into());
        cache.save_lyrics(&other, &lyrics("other")).unwrap();
        assert_eq!(
            cache.load_lyrics(&first).unwrap().unwrap().candidate.id,
            "first"
        );
        let mut renamed = first.clone();
        renamed.title = "改名".into();
        renamed.artist = "改名".into();
        assert_eq!(
            cache.load_lyrics(&renamed).unwrap().unwrap().candidate.id,
            "first"
        );
        let mut new_file = first.clone();
        new_file.md5 = Some("22222222222222222222222222222222".into());
        assert!(cache.load_lyrics(&new_file).unwrap().is_some());
        new_file.artist = "其他人".into();
        assert!(cache.load_lyrics(&new_file).unwrap().is_none());
        let index: ValueForTest =
            serde_json::from_slice(&std::fs::read(cache.index_path()).unwrap()).unwrap();
        assert_eq!(index["version"], 1);
        assert_eq!(index["tracks"].as_object().unwrap().len(), 2);
    }
    type ValueForTest = serde_json::Value;

    #[test]
    fn old_catalogs_are_upgradeable_and_check_time_survives_asset_updates() {
        let dir = tempfile::tempdir().unwrap();
        let cache = MediaCache::new(dir.path().into());
        let identity = MediaIdentity::read(
            &SearchQuery {
                title: "歌".into(),
                artist: "人".into(),
                album: String::new(),
                duration_ms: Some(10000),
            },
            None,
        )
        .unwrap();
        cache.save_lyrics(&identity, &lyrics("first")).unwrap();
        let mut old: serde_json::Value =
            serde_json::from_slice(&std::fs::read(cache.index_path()).unwrap()).unwrap();
        old["tracks"][identity.key()]
            .as_object_mut()
            .unwrap()
            .remove("word_lyrics_checked_at");
        std::fs::write(cache.index_path(), serde_json::to_vec(&old).unwrap()).unwrap();
        assert!(cache.word_lyrics_upgrade_due(&identity).unwrap());
        cache.mark_word_lyrics_checked(&identity).unwrap();
        assert!(!cache.word_lyrics_upgrade_due(&identity).unwrap());
        let mut upgraded = lyrics("next");
        upgraded.format = "yrc".into();
        upgraded.content = "[1000,2000](1000,1000,0)甲(2000,1000,0)乙".into();
        cache.save_lyrics(&identity, &upgraded).unwrap();
        assert!(!cache.word_lyrics_upgrade_due(&identity).unwrap());
        let loaded = cache.load_lyrics(&identity).unwrap().unwrap();
        assert_eq!(loaded.format, "yrc");
        assert_eq!(loaded.parse(10.).unwrap().lines[0].words.len(), 2);
    }

    #[test]
    fn concurrent_lyrics_and_cover_updates_preserve_both_assets_and_tracks() {
        let dir = tempfile::tempdir().unwrap();
        let cache = MediaCache::new(dir.path().into());
        let query = SearchQuery {
            title: "歌".into(),
            artist: "人".into(),
            album: "专辑".into(),
            duration_ms: Some(10000),
        };
        let first = MediaIdentity::read(&query, None).unwrap();
        let mut second = first.clone();
        second.title = "另一首".into();
        let mut cover: DownloadedCover = serde_json::from_value(serde_json::json!({
            "candidate":{"source":"test","id":"cover-id","album":"专辑","artist":"人","artwork_url":"https://example.com/a.png","release_date":"","score":0},
            "fetched_at_unix":1,"data":""
        })).unwrap();
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbaImage::new(4, 4)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        cover = DownloadedCover::from_bytes(cover.candidate, png.get_ref(), 1);
        std::thread::scope(|scope| {
            scope.spawn(|| cache.save_lyrics(&first, &lyrics("song-id")).unwrap());
            scope.spawn(|| cache.save_cover(&first, &cover).unwrap());
            scope.spawn(|| cache.record_track(&second).unwrap());
        });
        assert_eq!(
            cache.load_lyrics(&first).unwrap().unwrap().candidate.id,
            "song-id"
        );
        assert_eq!(
            cache.load_cover(&first).unwrap().unwrap().candidate.id,
            "cover-id"
        );
        let index = cache.catalog().unwrap();
        assert_eq!(index.tracks.len(), 2);
        let record = &index.tracks[&first.key()];
        assert!(record.lyrics.is_some() && record.cover.is_some());
        cache
            .record_track_with_local(
                &first,
                LocalSources {
                    embedded_cover: true,
                    ..Default::default()
                },
            )
            .unwrap();
        let updated = cache.catalog().unwrap();
        let record = &updated.tracks[&first.key()];
        assert!(record.local_sources.embedded_cover);
        assert!(record.lyrics.is_some() && record.cover.is_some());
        let cover_file = &record.cover.as_ref().unwrap().file;
        assert!(cover_file.ends_with(".png"));
        assert_eq!(
            std::fs::read(cache.root.join(cover_file)).unwrap(),
            png.into_inner()
        );
    }
}

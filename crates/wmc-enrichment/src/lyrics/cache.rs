use super::{DownloadedLyrics, TrackQuery};
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub struct LyricCache {
    root: PathBuf,
}
impl LyricCache {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    pub fn default_location() -> Result<Self> {
        Ok(Self::new(
            wmc_platform::paths::app_cache_root()?.join("lyrics"),
        ))
    }
    fn path(&self, query: &TrackQuery) -> Result<PathBuf> {
        // Include album and duration so different recordings do not share a choice.
        let digest = Sha256::digest(serde_json::to_vec(query)?);
        Ok(self.root.join(format!("{digest:x}.json")))
    }
    pub fn load(&self, query: &TrackQuery) -> Result<Option<DownloadedLyrics>> {
        let path = self.path(query)?;
        if !path.is_file() {
            return Ok(None);
        }
        ensure!(
            std::fs::metadata(&path)?.len() <= 8 * 1024 * 1024,
            "歌词缓存过大"
        );
        Ok(Some(serde_json::from_slice(&std::fs::read(path)?)?))
    }
    /// One file contains original lyrics, translation and complete provenance.
    pub fn save(&self, query: &TrackQuery, lyrics: &DownloadedLyrics) -> Result<()> {
        std::fs::create_dir_all(&self.root)?;
        let path = self.path(query)?;
        // Never overwrite the existing choice with a partially written document.
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        std::fs::write(&temporary, serde_json::to_vec_pretty(lyrics)?)?;
        std::fs::rename(&temporary, &path)
            .with_context(|| format!("保存歌词缓存：{}", path.display()))?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lyrics::{Candidate, Provider};
    #[test]
    fn cache_preserves_provenance_and_separates_recordings() {
        let dir = tempfile::tempdir().unwrap();
        let cache = LyricCache::new(dir.path().into());
        let mut query = TrackQuery {
            title: "歌".into(),
            artist: "人".into(),
            album: "专辑".into(),
            duration_ms: Some(2000),
        };
        let lyrics = DownloadedLyrics {
            candidate: Candidate {
                provider: Provider::Netease,
                id: "123".into(),
                mid: None,
                title: "歌".into(),
                artists: vec!["人".into()],
                album: "专辑".into(),
                duration_ms: Some(2000),
                title_aliases: vec![],
                score: 0,
            },
            content: "[00:01]词".into(),
            translation: None,
            source_url: "https://music.163.com/api/song/lyric?id=123".into(),
            format: "lrc".into(),
            fetched_at_unix: 1,
        };
        cache.save(&query, &lyrics).unwrap();
        assert_eq!(cache.load(&query).unwrap().unwrap().candidate.id, "123");
        cache.save(&query, &lyrics).unwrap();
        query.duration_ms = Some(3000);
        assert!(cache.load(&query).unwrap().is_none());
    }
}

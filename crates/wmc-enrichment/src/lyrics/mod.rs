//! Anonymous lyric providers. Blocking API: call on a worker, never the UI thread.
mod cache;
mod providers;
mod qrc_tables;
mod ranking;
mod word_crypto;
mod word_online;

use crate::transport::{Client, Request};
use anyhow::{Context, Result, ensure};
pub use cache::LyricCache;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    Netease,
    Qq,
    Amll,
    Local,
}

#[cfg(test)]
mod upgrade_tests {
    use super::*;
    #[test]
    fn local_edits_and_existing_word_lyrics_never_request_an_upgrade() {
        let client = LyricsClient::new().unwrap();
        let query = TrackQuery {
            title: "歌".into(),
            artist: "人".into(),
            album: String::new(),
            duration_ms: Some(10000),
        };
        let mut current = DownloadedLyrics {
            candidate: Candidate {
                provider: Provider::Local,
                id: "edited".into(),
                mid: None,
                title: query.title.clone(),
                artists: vec![query.artist.clone()],
                album: String::new(),
                duration_ms: query.duration_ms,
                title_aliases: vec![],
                score: 0,
            },
            format: "lrc".into(),
            content: "[00:01]歌词".into(),
            translation: None,
            source_url: String::new(),
            fetched_at_unix: 0,
        };
        assert!(client.upgrade(&query, &current, 10.).unwrap().is_none());
        current.candidate.provider = Provider::Netease;
        current.content = "[1000,2000](1000,1000,0)甲(2000,1000,0)乙".into();
        assert!(client.upgrade(&query, &current, 10.).unwrap().is_none());
    }
}
impl Provider {
    pub fn name(self) -> &'static str {
        match self {
            Self::Netease => "网易云",
            Self::Qq => "QQ 音乐",
            Self::Amll => "AMLL TTML",
            Self::Local => "本地编辑",
        }
    }
}

pub use wmc_domain::TrackQuery;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub provider: Provider,
    /// Numeric song ID for NetEase/QQ; raw filename for AMLL.
    pub id: String,
    pub mid: Option<String>,
    pub title: String,
    pub artists: Vec<String>,
    pub album: String,
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub title_aliases: Vec<String>,
    pub score: i32,
}
impl Candidate {
    pub fn matches_recording(&self, query: &TrackQuery) -> bool {
        ranking::recording_matches(query, self)
    }
    /// Download preference, not a claim that this candidate has word timing.
    pub fn extension(&self) -> &'static str {
        match self.provider {
            Provider::Amll => ".ttml",
            Provider::Netease => "YRC 优先",
            Provider::Qq => "QRC 优先",
            Provider::Local => "本地",
        }
    }
}

#[derive(Debug, Default)]
pub struct SearchResults {
    pub candidates: Vec<Candidate>,
    /// Partial failures do not discard results from other providers.
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DownloadedLyrics {
    pub candidate: Candidate,
    pub format: String,
    pub source_url: String,
    pub fetched_at_unix: u64,
    pub content: String,
    pub translation: Option<String>,
}
impl DownloadedLyrics {
    pub fn parse(&self, duration: f64) -> Result<wmc_lyrics::Lyrics> {
        let lyrics = wmc_lyrics::Lyrics::parse_text(&self.content, duration)?;
        ensure!(
            !lyrics.lines.is_empty() || lyrics.plain_text.is_some(),
            "来源返回了空歌词"
        );
        Ok(lyrics)
    }
}

#[derive(Clone)]
pub struct LyricsClient {
    pub(crate) http: Client,
}
impl LyricsClient {
    pub fn new() -> Result<Self> {
        Ok(Self {
            http: Client::shared(),
        })
    }

    /// Three concurrent requests with independent errors and a bounded result list.
    pub fn search(&self, query: &TrackQuery) -> Result<SearchResults> {
        ensure!(!query.title.trim().is_empty(), "请输入歌名");
        let mut results = SearchResults::default();
        std::thread::scope(|scope| {
            let jobs: Vec<_> = [Provider::Amll, Provider::Netease, Provider::Qq]
                .into_iter()
                .map(|provider| {
                    (
                        provider,
                        scope.spawn(move || self.search_provider(provider, query)),
                    )
                })
                .collect();
            for (provider, job) in jobs {
                match job.join() {
                    Ok(Ok(mut candidates)) => results.candidates.append(&mut candidates),
                    Ok(Err(error)) => results
                        .errors
                        .push(format!("{}：{error:#}", provider.name())),
                    Err(_) => results
                        .errors
                        .push(format!("{}：搜索任务失败", provider.name())),
                }
            }
        });
        ranking::rank(query, &mut results.candidates);
        results.candidates.truncate(60);
        Ok(results)
    }

    pub fn search_provider(
        &self,
        provider: Provider,
        query: &TrackQuery,
    ) -> Result<Vec<Candidate>> {
        providers::search(self, provider, query)
    }

    pub fn fetch(&self, candidate: &Candidate) -> Result<DownloadedLyrics> {
        providers::fetch(self, candidate)
    }

    /// Upgrade only online line lyrics; local edits and existing word timing are protected.
    pub fn upgrade(
        &self,
        query: &TrackQuery,
        current: &DownloadedLyrics,
        duration: f64,
    ) -> Result<Option<DownloadedLyrics>> {
        if current.candidate.provider == Provider::Local
            || current
                .parse(duration)?
                .lines
                .iter()
                .any(|line| line.word_synced)
        {
            return Ok(None);
        }
        let matches = ranking::recording_matches(query, &current.candidate);
        let mut errors = Vec::new();
        if matches {
            match self.fetch_word(&current.candidate, duration) {
                Ok(Some(downloaded)) => return Ok(Some(downloaded)),
                Err(error) => errors.push(error.to_string()),
                _ => {}
            }
        }
        let results = self.search(query)?;
        errors.extend(results.errors);
        let mut corrected = None;
        for candidate in results
            .candidates
            .iter()
            .filter(|candidate| {
                ranking::recording_matches(query, candidate)
                    && !(candidate.provider == current.candidate.provider
                        && candidate.id == current.candidate.id)
            })
            .take(4)
        {
            match self.fetch_word(candidate, duration) {
                Ok(Some(downloaded)) => return Ok(Some(downloaded)),
                Err(error) => errors.push(error.to_string()),
                Ok(None) if !matches && corrected.is_none() => {
                    if let Ok(downloaded) = providers::fetch_line(self, candidate)
                        && downloaded.parse(duration).is_ok()
                    {
                        corrected = Some(downloaded);
                    }
                }
                _ => {}
            }
        }
        // A transient failure must not suppress upgrades for the next seven days.
        ensure!(errors.is_empty(), "逐字查询未完成：{}", errors.join("；"));
        Ok(corrected)
    }

    fn fetch_word(&self, candidate: &Candidate, duration: f64) -> Result<Option<DownloadedLyrics>> {
        let downloaded = match candidate.provider {
            Provider::Netease | Provider::Qq => word_online::fetch(self, candidate)?,
            Provider::Amll => Some(self.fetch(candidate)?),
            Provider::Local => None,
        };
        if let Some(downloaded) = downloaded
            && downloaded
                .parse(duration)?
                .lines
                .iter()
                .any(|line| line.word_synced)
        {
            return Ok(Some(downloaded));
        }
        Ok(None)
    }

    pub(crate) fn bytes(&self, request: Request, limit: usize) -> Result<Vec<u8>> {
        self.http.bytes(request, limit)
    }
    fn text(&self, request: Request) -> Result<String> {
        String::from_utf8(self.bytes(request, 4 * 1024 * 1024)?).context("响应不是 UTF-8")
    }
    pub(crate) fn json(&self, request: Request) -> Result<serde_json::Value> {
        self.http.json(request)
    }
}

//! Verify first-load downloads and subsequent MD5 cache hits without an audio device.
use anyhow::{Context, Result};
use std::path::PathBuf;
use wmc_player::{
    media_cache::{MediaCache, MediaIdentity},
    metadata::{TrackMetadata, properties::AudioProperties},
    online_covers::CoversClient,
    online_lyrics::{LyricsClient, SearchQuery},
};
fn main() -> Result<()> {
    let path = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .context("用法：asset-cache-probe <音频路径>")?,
    );
    let metadata = TrackMetadata::read(&path)?;
    let duration = AudioProperties::read(&path)?.duration;
    let query = SearchQuery {
        title: metadata.title.unwrap_or_else(|| {
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        }),
        artist: metadata.artist.unwrap_or_default(),
        album: metadata.album.unwrap_or_default(),
        duration_ms: Some(duration.as_millis() as u64),
    };
    let identity = MediaIdentity::read(&query, Some(&path))?;
    let cache = MediaCache::default_location()?;
    cache.record_track_with_local(
        &identity,
        wmc_player::media_cache::LocalSources {
            embedded_lyrics: metadata.lyrics.is_some(),
            embedded_cover: metadata.cover.is_some(),
            ..Default::default()
        },
    )?;
    println!(
        "MD5={} 总索引={}",
        identity.md5.as_deref().unwrap_or(""),
        cache.index_path().display()
    );
    if let Some(lyrics) = cache.load_lyrics(&identity)? {
        let parsed = lyrics.parse(duration.as_secs_f64())?;
        cache.save_lyrics(&identity, &lyrics)?;
        println!(
            "歌词本地缓存命中 · {} · {} 行 · 未联网搜索",
            lyrics.candidate.provider.name(),
            parsed.lines.len()
        );
    } else if metadata.lyrics.is_some() {
        println!("歌曲已有内嵌歌词 · 未联网搜索");
    } else {
        let client = LyricsClient::new()?;
        let results = client.search(&query)?;
        let mut saved = false;
        for candidate in results.candidates.iter().take(5) {
            match client.fetch(candidate).and_then(|lyrics| {
                let parsed = lyrics.parse(duration.as_secs_f64())?;
                Ok((lyrics, parsed))
            }) {
                Ok((lyrics, parsed)) => {
                    cache.save_lyrics(&identity, &lyrics)?;
                    println!(
                        "歌词已联网获取并缓存 · {} · {} 行 · ID={}",
                        candidate.provider.name(),
                        parsed.lines.len(),
                        candidate.id
                    );
                    saved = true;
                    break;
                }
                Err(error) => eprintln!("歌词候选失败：{error:#}"),
            }
        }
        if !saved {
            eprintln!("没有获取到歌词；{}", results.errors.join("；"));
        }
    }
    if let Some(cover) = cache.load_cover(&identity)? {
        let image = cover.image()?;
        println!(
            "封面本地缓存命中 · {} · {}×{} · 未联网搜索",
            cover.candidate.source,
            image.width(),
            image.height()
        );
    } else if metadata.cover.is_some() {
        println!("歌曲已有内嵌封面 · 未联网搜索");
    } else {
        let client = CoversClient::new()?;
        let results = client.search(&query)?;
        for candidate in results.candidates.iter().take(5) {
            if let Ok(cover) = client.fetch(candidate) {
                cache.save_cover(&identity, &cover)?;
                println!("封面已联网获取并缓存 · {}", candidate.source);
                break;
            }
        }
    }
    Ok(())
}

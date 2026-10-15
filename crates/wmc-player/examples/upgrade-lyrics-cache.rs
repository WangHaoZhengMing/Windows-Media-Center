//! Upgrade previously downloaded line lyrics. Audio files and local edits are untouched.
use anyhow::{Context, Result};
use wmc_player::{
    media_cache::{MediaCache, TrackRecord},
    online_lyrics::{LyricsClient, Provider, SearchQuery},
};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let apply = args.iter().any(|arg| arg == "--apply");
    let filter = args
        .iter()
        .find(|arg| !arg.starts_with("--"))
        .map(String::as_str)
        .unwrap_or("");
    let cache = MediaCache::default_location()?;
    let index = cache.index_path();
    let catalog: serde_json::Value = serde_json::from_slice(&std::fs::read(&index)?)?;
    let tracks = catalog["tracks"]
        .as_object()
        .context("缓存索引缺少 tracks")?;
    let client = LyricsClient::new()?;
    let mut upgraded = 0;
    for value in tracks.values() {
        let record: TrackRecord = serde_json::from_value(value.clone())?;
        if !record.identity.title.contains(filter) {
            continue;
        }
        let Some(path) = &record.identity.audio_path else {
            continue;
        };
        if !path.is_file()
            || path
                .components()
                .any(|part| part.as_os_str().to_string_lossy().contains("backup"))
        {
            continue;
        }
        // Old hashes can point at the same path after metadata was edited.
        let query = SearchQuery {
            title: record.identity.title.clone(),
            artist: record.identity.artist.clone(),
            album: record.identity.album.clone(),
            duration_ms: record.identity.duration_ms,
        };
        if wmc_player::media_cache::MediaIdentity::read(&query, Some(path))?.md5
            != record.identity.md5
        {
            continue;
        }
        let Some(current) = cache.load_lyrics(&record.identity)? else {
            continue;
        };
        let duration = record.identity.duration_ms.unwrap_or(3600000) as f64 / 1000.;
        if current.candidate.provider == Provider::Local
            || current
                .parse(duration)?
                .lines
                .iter()
                .any(|line| line.word_synced)
        {
            continue;
        }
        println!("检查 {} · {}", query.title, query.artist);
        match client.upgrade(&query, &current, duration) {
            Ok(Some(next)) => {
                let parsed = next.parse(duration)?;
                println!(
                    "  {} · {} · {} 行 / {} 个词 {}",
                    next.candidate.provider.name(),
                    next.format,
                    parsed.lines.len(),
                    parsed
                        .lines
                        .iter()
                        .map(|line| line.words.len())
                        .sum::<usize>(),
                    if apply { "写入缓存" } else { "预览" }
                );
                if apply {
                    let backup = index.with_extension("before-word-upgrade.json");
                    if !backup.exists() {
                        std::fs::copy(&index, backup)?;
                    }
                    if let Some(asset) = &record.lyrics {
                        let source = index.parent().unwrap().join(&asset.file);
                        let backup = source.with_extension("before-word-upgrade.json");
                        if !backup.exists() {
                            std::fs::copy(&source, backup)?;
                        }
                    }
                    cache.save_lyrics(&record.identity, &next)?;
                    cache.mark_word_lyrics_checked(&record.identity)?;
                }
                upgraded += 1;
            }
            Ok(None) => {
                println!("  没有可用的逐字升级，保留现有歌词");
                if apply {
                    cache.mark_word_lyrics_checked(&record.identity)?;
                }
            }
            Err(error) => eprintln!("  保留现有缓存：{error:#}"),
        }
    }
    println!("可升级：{upgraded}；写入：{apply}");
    Ok(())
}

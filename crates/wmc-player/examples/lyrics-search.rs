//! Real network smoke test / diagnostic client, without opening the player.
use anyhow::{Context, Result};
use wmc_player::online_lyrics::{LyricsClient, Provider, SearchQuery, SearchResults};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let title = args
        .next()
        .context("用法：lyrics-search <歌名> <歌手> [下载的结果序号，从1开始]")?;
    let artist = args.next().unwrap_or_default();
    let selection = args
        .next()
        .map(|value| value.parse::<usize>())
        .transpose()?;
    let provider = args
        .next()
        .map(|name| match name.as_str() {
            "qq" => Ok(Provider::Qq),
            "netease" => Ok(Provider::Netease),
            "amll" => Ok(Provider::Amll),
            _ => anyhow::bail!("来源应为 qq、netease 或 amll"),
        })
        .transpose()?;
    let client = LyricsClient::new()?;
    let query = SearchQuery {
        title,
        artist,
        album: String::new(),
        duration_ms: None,
    };
    let result = match provider {
        Some(provider) => SearchResults {
            candidates: client.search_provider(provider, &query)?,
            errors: vec![],
        },
        None => client.search(&query)?,
    };
    for (index, candidate) in result.candidates.iter().enumerate() {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{:?}\tID={} MID={}",
            index + 1,
            candidate.provider.name(),
            candidate.title,
            candidate.artists.join(" / "),
            candidate.album,
            candidate.duration_ms,
            candidate.id,
            candidate.mid.as_deref().unwrap_or("")
        );
    }
    for error in &result.errors {
        eprintln!("{error}");
    }
    if let Some(index) = selection {
        let candidate = index
            .checked_sub(1)
            .and_then(|i| result.candidates.get(i))
            .context("结果序号超出范围")?;
        let downloaded = client.fetch(candidate)?;
        let duration = candidate
            .duration_ms
            .map(|ms| ms as f64 / 1000.)
            .unwrap_or(3600.);
        let lyrics = downloaded.parse(duration)?;
        // Avoid printing entire copyrighted lyric contents during diagnostics.
        println!(
            "下载并解析成功：{}，{} 行，{} 字节，来源 {}",
            downloaded.format,
            lyrics.lines.len(),
            downloaded.content.len(),
            downloaded.source_url
        );
    }
    Ok(())
}

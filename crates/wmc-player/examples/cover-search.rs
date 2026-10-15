use anyhow::{Context, Result};
use wmc_player::{online_covers::CoversClient, online_lyrics::SearchQuery};
fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let album = args
        .next()
        .context("用法：cover-search <专辑> <歌手> [下载的结果序号]")?;
    let artist = args.next().unwrap_or_default();
    let selected = args.next().map(|s| s.parse::<usize>()).transpose()?;
    let client = CoversClient::new()?;
    let query = SearchQuery {
        title: String::new(),
        artist,
        album,
        duration_ms: None,
    };
    let results = client.search(&query)?;
    for (index, candidate) in results.candidates.iter().enumerate() {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            index + 1,
            candidate.source,
            candidate.album,
            candidate.artist,
            candidate.artwork_url
        );
    }
    for error in results.errors {
        eprintln!("{error}");
    }
    if let Some(index) = selected {
        let candidate = index
            .checked_sub(1)
            .and_then(|index| results.candidates.get(index))
            .context("结果序号超出范围")?;
        let image = client.fetch(candidate)?.image()?;
        println!("封面下载并解码成功：{}×{}", image.width(), image.height());
    }
    Ok(())
}

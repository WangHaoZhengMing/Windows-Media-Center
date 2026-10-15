use super::{Candidate, SearchQuery};
fn normalized(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
pub(super) fn recording_matches(query: &SearchQuery, candidate: &Candidate) -> bool {
    let title_matches = std::iter::once(&candidate.title)
        .chain(candidate.title_aliases.iter())
        .any(|title| similarity(&query.title, title, 100, 35) > 0);
    let artist_matches = query.artist.trim().is_empty()
        || candidate
            .artists
            .iter()
            .any(|artist| similarity(&query.artist, artist, 60, 20) > 0);
    let duration_matches = query
        .duration_ms
        .zip(candidate.duration_ms)
        .is_none_or(|(a, b)| a.abs_diff(b) <= 5000);
    let query_title = normalized(&query.title);
    let candidate_title = normalized(&candidate.title);
    let version_matches = [
        "live", "cover", "翻唱", "伴奏", "inst", "remix", "dj", "消音",
    ]
    .iter()
    .all(|version| !candidate_title.contains(version) || query_title.contains(version));
    title_matches && artist_matches && duration_matches && version_matches
}
fn similarity(a: &str, b: &str, exact: i32, partial: i32) -> i32 {
    let a = normalized(a);
    let b = normalized(b);
    if a.is_empty() || b.is_empty() {
        0
    } else if a == b {
        exact
    } else if a.contains(&b) || b.contains(&a) {
        partial
    } else {
        0
    }
}
pub(super) fn rank(query: &SearchQuery, candidates: &mut [Candidate]) {
    for candidate in candidates.iter_mut() {
        let title_score = std::iter::once(&candidate.title)
            .chain(candidate.title_aliases.iter())
            .map(|title| similarity(&query.title, title, 100, 35))
            .max()
            .unwrap_or(0);
        let artist_score = candidate
            .artists
            .iter()
            .map(|artist| similarity(&query.artist, artist, 60, 20))
            .max()
            .unwrap_or(0);
        let album_score = similarity(&query.album, &candidate.album, 20, 8);
        let duration_score = match (query.duration_ms, candidate.duration_ms) {
            (Some(a), Some(b)) => match a.abs_diff(b) {
                0..=2000 => 20,
                2001..=5000 => 10,
                5001..=15000 => -20,
                _ => -70,
            },
            _ => 0,
        };
        // Retain version words. A live version must not become identical to the studio title.
        candidate.score = title_score + artist_score + album_score + duration_score;
    }
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.score));
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::online_lyrics::Provider;
    #[test]
    fn duration_and_version_prevent_wrong_recording_from_winning() {
        let query = SearchQuery {
            title: "晴天".into(),
            artist: "周杰伦".into(),
            album: "".into(),
            duration_ms: Some(269000),
        };
        let base = Candidate {
            provider: Provider::Qq,
            id: "1".into(),
            mid: None,
            title: "晴天".into(),
            artists: vec!["周杰伦".into()],
            album: "".into(),
            duration_ms: Some(269000),
            title_aliases: vec![],
            score: 0,
        };
        let mut live = base.clone();
        live.title = "晴天 (Live)".into();
        live.duration_ms = Some(330000);
        let mut other = base.clone();
        other.artists = vec!["其他歌手".into()];
        let mut candidates = vec![live, other, base];
        rank(&query, &mut candidates);
        assert_eq!(candidates[0].score, 180);
        assert_eq!(candidates[0].title, "晴天");
        assert_eq!(candidates[0].artists, vec!["周杰伦"]);
        assert!(recording_matches(&query, &candidates[0]));
        assert!(
            candidates[1..]
                .iter()
                .all(|candidate| !recording_matches(&query, candidate))
        );
    }
}

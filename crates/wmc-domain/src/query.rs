use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrackQuery {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: Option<u64>,
}
impl TrackQuery {
    pub fn text(&self) -> String {
        format!("{} {}", self.title.trim(), self.artist.trim())
            .trim()
            .to_owned()
    }
}

//! Provider-independent lyric translation, with bounded batch workers and caching.
mod providers;
mod service;
pub use crate::network::Cancellation;
use anyhow::Result;
pub use providers::{GoogleTranslator, MicrosoftTranslator};
pub use service::{Document, Progress, TranslationJob};

/// Implement this for HTTP services or a platform bridge (e.g. Apple Translation).
/// Results must correspond to the input order. The service validates their count.
pub trait Translator: Send + Sync {
    fn cache_id(&self) -> String;
    fn translate_batch(
        &self,
        texts: &[String],
        source: &str,
        target: &str,
        cancel: &Cancellation,
    ) -> Result<Vec<String>>;
}

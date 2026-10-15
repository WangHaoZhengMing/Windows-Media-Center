use super::{Cancellation, Translator};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};
use wmc_lyrics::Lyrics;

#[derive(Clone)]
pub struct Document {
    pub hash: String,
    pub row_count: usize,
    segments: Vec<(usize, String)>,
    aliases: BTreeMap<usize, Vec<usize>>,
}
impl Document {
    pub fn from_lyrics(lyrics: &Lyrics) -> Result<Self> {
        let rows: Vec<_> = if lyrics.lines.is_empty() {
            lyrics
                .plain_text
                .as_deref()
                .unwrap_or_default()
                .lines()
                .map(str::to_owned)
                .collect()
        } else {
            lyrics.lines.iter().map(|line| line.text()).collect()
        };
        ensure!(
            !rows.is_empty() && rows.len() <= 2000,
            "歌词为空或超过 2000 行"
        );
        let timeline: Vec<_> = lyrics
            .lines
            .iter()
            .map(|line| (line.start, line.end, &line.agents, line.background))
            .collect();
        let hash = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&(&rows, timeline))?)
        );
        let mut seen = HashMap::<String, usize>::new();
        let mut segments = vec![];
        let mut aliases = BTreeMap::<usize, Vec<usize>>::new();
        for (id, text) in rows.iter().enumerate() {
            let text = text.trim();
            if text.is_empty() {
                continue;
            }
            ensure!(text.chars().count() <= 3000, "单行歌词过长，请先拆分");
            let canonical = *seen.entry(text.to_owned()).or_insert_with(|| {
                segments.push((id, text.to_owned()));
                id
            });
            aliases.entry(canonical).or_default().push(id);
        }
        Ok(Self {
            hash,
            row_count: rows.len(),
            segments,
            aliases,
        })
    }
    fn expand(&self, rows: &BTreeMap<usize, String>) -> Vec<(usize, String)> {
        rows.iter()
            .flat_map(|(id, text)| {
                self.aliases
                    .get(id)
                    .into_iter()
                    .flatten()
                    .map(move |row| (*row, text.clone()))
            })
            .collect()
    }
}
pub enum Progress {
    Rows(Vec<(usize, String)>),
    Error(String),
    Finished,
}
pub struct TranslationJob {
    pub events: async_channel::Receiver<Progress>,
    cancel: Cancellation,
    pub cache_file: PathBuf,
}
impl TranslationJob {
    pub fn cached_rows(
        document: &Document,
        provider: &str,
        source: &str,
        target: &str,
        cache_root: &std::path::Path,
    ) -> Result<Vec<(usize, String)>> {
        let key = cache_key(&document.hash, provider, source, target)?;
        let expected = Cached {
            version: 1,
            source_hash: document.hash.clone(),
            provider: provider.into(),
            source_language: source.into(),
            target_language: target.into(),
            rows: BTreeMap::new(),
        };
        let cached = Cache {
            file: cache_root.join(format!("{key}.json")),
        }
        .load(&expected)?;
        Ok(cached.map_or_else(Vec::new, |cached| document.expand(&cached.rows)))
    }
    pub fn token(&self) -> Cancellation {
        self.cancel.clone()
    }
    pub fn start(
        document: Document,
        provider: Arc<dyn Translator>,
        source: String,
        target: String,
        cache_root: PathBuf,
        priority_line: usize,
    ) -> Result<Self> {
        ensure!(!target.trim().is_empty(), "请输入目标语言");
        let provider_id = provider.cache_id();
        let key = cache_key(&document.hash, &provider_id, &source, &target)?;
        let cache = Cache {
            file: cache_root.join(format!("{key}.json")),
        };
        let expected = Cached {
            version: 1,
            source_hash: document.hash.clone(),
            provider: provider_id,
            source_language: source.clone(),
            target_language: target.clone(),
            rows: BTreeMap::new(),
        };
        let existing = cache.load(&expected)?.unwrap_or_else(|| expected.clone());
        cache.merge(&expected, &BTreeMap::new())?;
        let (sender, events) = async_channel::unbounded();
        if !existing.rows.is_empty() {
            let _ = sender.try_send(Progress::Rows(document.expand(&existing.rows)));
        }
        let mut batches: Vec<Vec<(usize, String)>> = vec![];
        let mut batch = vec![];
        let mut characters = 0;
        for segment in &document.segments {
            if existing.rows.contains_key(&segment.0) {
                continue;
            }
            let length = segment.1.chars().count();
            if !batch.is_empty() && (batch.len() >= 6 || characters + length > 3000) {
                batches.push(std::mem::take(&mut batch));
                characters = 0;
            }
            batch.push(segment.clone());
            characters += length;
        }
        if !batch.is_empty() {
            batches.push(batch);
        }
        batches.sort_by_key(|batch| {
            batch
                .iter()
                .flat_map(|(id, _)| document.aliases[id].iter())
                .map(|id| id.abs_diff(priority_line))
                .min()
                .unwrap_or(usize::MAX)
        });
        let cancel = Cancellation::default();
        let cache_file = cache.file.clone();
        if batches.is_empty() {
            let _ = sender.try_send(Progress::Finished);
            return Ok(Self {
                events,
                cancel,
                cache_file,
            });
        }
        let state = Arc::new(JobState {
            document,
            provider,
            source,
            target,
            expected,
            cache,
            cancel: cancel.clone(),
            sender,
            remaining: AtomicUsize::new(batches.len()),
        });
        let queue = queue();
        let mut pending = queue.jobs.lock().unwrap_or_else(|e| e.into_inner());
        let mut retained = VecDeque::new();
        while let Some(work) = pending.pop_front() {
            if work.state.cancel.is_cancelled() {
                work.complete();
            } else {
                retained.push_back(work);
            }
        }
        *pending = retained;
        ensure!(
            pending.len() + batches.len() <= 1024,
            "翻译任务过多，请稍后重试"
        );
        pending.extend(batches.into_iter().map(|segments| Work {
            state: state.clone(),
            segments,
        }));
        drop(pending);
        queue.available.notify_all();
        Ok(Self {
            events,
            cancel,
            cache_file,
        })
    }
}
fn cache_key(hash: &str, provider: &str, source: &str, target: &str) -> Result<String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(1, hash, provider, source, target))?)
    ))
}
impl Drop for TranslationJob {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
struct JobState {
    document: Document,
    provider: Arc<dyn Translator>,
    source: String,
    target: String,
    expected: Cached,
    cache: Cache,
    cancel: Cancellation,
    sender: async_channel::Sender<Progress>,
    remaining: AtomicUsize,
}
struct Work {
    state: Arc<JobState>,
    segments: Vec<(usize, String)>,
}
impl Work {
    fn complete(&self) {
        if self.state.remaining.fetch_sub(1, Ordering::AcqRel) == 1 {
            let _ = self.state.sender.try_send(Progress::Finished);
        }
    }
    fn run(self) {
        let result = (|| -> Result<()> {
            self.state.cancel.check()?;
            let texts = self
                .segments
                .iter()
                .map(|(_, text)| text.clone())
                .collect::<Vec<_>>();
            let translated = self.state.provider.translate_batch(
                &texts,
                &self.state.source,
                &self.state.target,
                &self.state.cancel,
            )?;
            self.state.cancel.check()?;
            ensure!(translated.len() == texts.len(), "翻译服务返回行数不一致");
            ensure!(
                translated
                    .iter()
                    .all(|text| !text.trim().is_empty() && text.len() <= 16000),
                "翻译服务返回了无效译文"
            );
            let rows: BTreeMap<_, _> = self
                .segments
                .iter()
                .map(|(id, _)| *id)
                .zip(translated)
                .collect();
            // Persist each successful batch so retrying only translates missing rows.
            self.state.cache.merge(&self.state.expected, &rows)?;
            self.state.cancel.check()?;
            let _ = self
                .state
                .sender
                .try_send(Progress::Rows(self.state.document.expand(&rows)));
            Ok(())
        })();
        if let Err(error) = result
            && !self.state.cancel.is_cancelled()
        {
            let _ = self
                .state
                .sender
                .try_send(Progress::Error(format!("翻译失败：{error:#}")));
            self.state.cancel.cancel();
        }
        self.complete();
    }
}
struct Queue {
    jobs: Mutex<VecDeque<Work>>,
    available: Condvar,
}
fn queue() -> &'static Arc<Queue> {
    static QUEUE: OnceLock<Arc<Queue>> = OnceLock::new();
    QUEUE.get_or_init(|| {
        let queue = Arc::new(Queue {
            jobs: Mutex::new(VecDeque::<Work>::new()),
            available: Condvar::new(),
        });
        for _ in 0..2 {
            let worker = queue.clone();
            std::thread::spawn(move || {
                loop {
                    let work = {
                        let mut jobs = worker.jobs.lock().unwrap_or_else(|e| e.into_inner());
                        while jobs.is_empty() {
                            jobs = worker
                                .available
                                .wait(jobs)
                                .unwrap_or_else(|e| e.into_inner());
                        }
                        jobs.pop_front().expect("queue checked")
                    };
                    work.run();
                }
            });
        }
        queue
    })
}
#[derive(Clone, Serialize, Deserialize)]
struct Cached {
    version: u32,
    source_hash: String,
    provider: String,
    source_language: String,
    target_language: String,
    rows: BTreeMap<usize, String>,
}
struct Cache {
    file: PathBuf,
}
impl Cache {
    fn load(&self, expected: &Cached) -> Result<Option<Cached>> {
        if !self.file.is_file() {
            return Ok(None);
        }
        ensure!(
            std::fs::metadata(&self.file)?.len() <= 4 * 1024 * 1024,
            "翻译缓存过大"
        );
        let stored: Cached =
            serde_json::from_slice(&std::fs::read(&self.file)?).context("翻译缓存损坏")?;
        ensure!(
            stored.version == 1
                && stored.source_hash == expected.source_hash
                && stored.provider == expected.provider
                && stored.source_language == expected.source_language
                && stored.target_language == expected.target_language,
            "翻译缓存信息不匹配"
        );
        Ok(Some(stored))
    }
    fn merge(&self, expected: &Cached, rows: &BTreeMap<usize, String>) -> Result<()> {
        std::fs::create_dir_all(self.file.parent().context("缺少翻译缓存目录")?)?;
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.file.with_extension("lock"))?;
        lock.lock()?;
        let mut cached = self.load(expected)?.unwrap_or_else(|| expected.clone());
        cached.rows.extend(rows.clone());
        let temporary = self
            .file
            .with_extension(format!("{}.tmp", std::process::id()));
        std::fs::write(&temporary, serde_json::to_vec_pretty(&cached)?)?;
        std::fs::rename(temporary, &self.file)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fake {
        calls: AtomicUsize,
    }
    impl Translator for Fake {
        fn cache_id(&self) -> String {
            "fake-v1".into()
        }
        fn translate_batch(
            &self,
            texts: &[String],
            _: &str,
            _: &str,
            cancel: &Cancellation,
        ) -> Result<Vec<String>> {
            cancel.check()?;
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(texts.iter().map(|text| format!("译：{text}")).collect())
        }
    }
    #[test]
    fn repeated_lines_map_to_original_rows_and_cache_avoids_second_call() {
        let lyrics = Lyrics::parse_lrc("[00:01]Hello\n[00:02]World\n[00:03]Hello", 10.).unwrap();
        let doc = Document::from_lyrics(&lyrics).unwrap();
        assert_eq!(doc.segments.len(), 2);
        let dir = tempfile::tempdir().unwrap();
        let fake = Arc::new(Fake {
            calls: AtomicUsize::new(0),
        });
        let job = TranslationJob::start(
            doc.clone(),
            fake.clone(),
            "auto".into(),
            "zh-CN".into(),
            dir.path().into(),
            0,
        )
        .unwrap();
        let mut rows = BTreeMap::new();
        loop {
            match job.events.recv_blocking().unwrap() {
                Progress::Rows(batch) => rows.extend(batch),
                Progress::Finished => break,
                Progress::Error(error) => panic!("{error}"),
            }
        }
        assert_eq!(rows[&0], rows[&2]);
        assert_eq!(rows.len(), 3);
        assert_eq!(fake.calls.load(Ordering::Relaxed), 1);
        let second = TranslationJob::start(
            doc,
            fake.clone(),
            "auto".into(),
            "zh-CN".into(),
            dir.path().into(),
            0,
        )
        .unwrap();
        while !matches!(second.events.recv_blocking().unwrap(), Progress::Finished) {}
        assert_eq!(fake.calls.load(Ordering::Relaxed), 1);
        assert!(
            TranslationJob::cached_rows(
                &Document::from_lyrics(&lyrics).unwrap(),
                "fake-v1",
                "auto",
                "en",
                dir.path()
            )
            .unwrap()
            .is_empty()
        );
    }
    #[test]
    fn twenty_rows_use_batches_and_at_most_two_workers() {
        struct Measured {
            calls: AtomicUsize,
            active: AtomicUsize,
            maximum: AtomicUsize,
        }
        impl Translator for Measured {
            fn cache_id(&self) -> String {
                "measured".into()
            }
            fn translate_batch(
                &self,
                texts: &[String],
                _: &str,
                _: &str,
                cancel: &Cancellation,
            ) -> Result<Vec<String>> {
                assert!(texts.len() <= 6);
                self.calls.fetch_add(1, Ordering::Relaxed);
                let active = self.active.fetch_add(1, Ordering::AcqRel) + 1;
                self.maximum.fetch_max(active, Ordering::Relaxed);
                let result = cancel.wait(std::time::Duration::from_millis(5));
                self.active.fetch_sub(1, Ordering::AcqRel);
                result?;
                Ok(texts.to_vec())
            }
        }
        let lyrics = Lyrics {
            lines: vec![],
            plain_text: Some(
                (0..20)
                    .map(|i| format!("line {i}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        };
        let provider = Arc::new(Measured {
            calls: AtomicUsize::new(0),
            active: AtomicUsize::new(0),
            maximum: AtomicUsize::new(0),
        });
        let dir = tempfile::tempdir().unwrap();
        let job = TranslationJob::start(
            Document::from_lyrics(&lyrics).unwrap(),
            provider.clone(),
            "auto".into(),
            "zh-CN".into(),
            dir.path().into(),
            12,
        )
        .unwrap();
        let mut rows = BTreeMap::new();
        loop {
            match job.events.recv_blocking().unwrap() {
                Progress::Rows(batch) => rows.extend(batch),
                Progress::Finished => break,
                Progress::Error(error) => panic!("{error}"),
            }
        }
        assert_eq!(rows.len(), 20);
        assert_eq!(provider.calls.load(Ordering::Relaxed), 4);
        assert!(provider.maximum.load(Ordering::Relaxed) <= 2);
    }
    #[test]
    fn cancellation_stops_pending_batches() {
        struct Waiting {
            entered: std::sync::mpsc::Sender<()>,
            calls: AtomicUsize,
        }
        impl Translator for Waiting {
            fn cache_id(&self) -> String {
                "waiting".into()
            }
            fn translate_batch(
                &self,
                texts: &[String],
                _: &str,
                _: &str,
                cancel: &Cancellation,
            ) -> Result<Vec<String>> {
                cancel.check()?;
                self.calls.fetch_add(1, Ordering::Relaxed);
                let _ = self.entered.send(());
                cancel.wait(std::time::Duration::from_secs(5))?;
                Ok(texts.to_vec())
            }
        }
        let (sender, entered) = std::sync::mpsc::channel();
        let provider = Arc::new(Waiting {
            entered: sender,
            calls: AtomicUsize::new(0),
        });
        let lyrics = Lyrics {
            lines: vec![],
            plain_text: Some(
                (0..60)
                    .map(|i| format!("line {i}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        };
        let dir = tempfile::tempdir().unwrap();
        let job = TranslationJob::start(
            Document::from_lyrics(&lyrics).unwrap(),
            provider.clone(),
            "auto".into(),
            "zh-CN".into(),
            dir.path().into(),
            0,
        )
        .unwrap();
        entered
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        job.token().cancel();
        while !matches!(job.events.recv_blocking().unwrap(), Progress::Finished) {}
        assert!(provider.calls.load(Ordering::Relaxed) <= 2);
    }
    #[test]
    fn malformed_batch_does_not_get_cached_or_attached_to_wrong_rows() {
        struct Bad;
        impl Translator for Bad {
            fn cache_id(&self) -> String {
                "bad".into()
            }
            fn translate_batch(
                &self,
                _: &[String],
                _: &str,
                _: &str,
                _: &Cancellation,
            ) -> Result<Vec<String>> {
                Ok(vec![])
            }
        }
        let lyrics = Lyrics::parse_lrc("[00:01]Hello", 10.).unwrap();
        let doc = Document::from_lyrics(&lyrics).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let job = TranslationJob::start(
            doc.clone(),
            Arc::new(Bad),
            "auto".into(),
            "zh-CN".into(),
            dir.path().into(),
            0,
        )
        .unwrap();
        let mut failed = false;
        loop {
            match job.events.recv_blocking().unwrap() {
                Progress::Error(_) => failed = true,
                Progress::Finished => break,
                Progress::Rows(_) => panic!("malformed result accepted"),
            }
        }
        assert!(failed);
        assert!(
            TranslationJob::cached_rows(&doc, "bad", "auto", "zh-CN", dir.path())
                .unwrap()
                .is_empty()
        );
    }
}

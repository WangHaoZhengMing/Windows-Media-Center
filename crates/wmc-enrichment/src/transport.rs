//! Shared synchronous transport. All calls belong on background workers.
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{
    io::Read,
    sync::{Arc, Condvar, Mutex, OnceLock},
    time::Duration,
};
use ureq::{
    Agent, RequestExt,
    tls::{RootCerts, TlsConfig},
};

struct Shared {
    agent: Agent,
    active: Mutex<(usize, usize)>,
    available: Condvar,
}
#[derive(Clone)]
pub(crate) struct Client {
    shared: Arc<Shared>,
}
#[derive(Clone)]
pub(crate) struct Request {
    method: &'static str,
    url: String,
    headers: Vec<(String, String)>,
    query: Vec<(String, String)>,
    body: Vec<u8>,
    timeout: Duration,
    translation: bool,
    cancel: Cancellation,
}

#[derive(Clone, Default)]
pub struct Cancellation(Arc<(Mutex<bool>, Condvar)>);
impl Cancellation {
    pub fn cancel(&self) {
        *self.0.0.lock().unwrap_or_else(|e| e.into_inner()) = true;
        self.0.1.notify_all();
    }
    pub fn is_cancelled(&self) -> bool {
        *self.0.0.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub fn check(&self) -> Result<()> {
        ensure!(!self.is_cancelled(), "任务已取消");
        Ok(())
    }
    pub fn wait(&self, duration: Duration) -> Result<()> {
        let guard = self.0.0.lock().unwrap_or_else(|e| e.into_inner());
        let (cancelled, _) = self
            .0
            .1
            .wait_timeout_while(guard, duration, |cancelled| !*cancelled)
            .unwrap_or_else(|e| e.into_inner());
        ensure!(!*cancelled, "任务已取消");
        Ok(())
    }
}
#[derive(Debug)]
pub(crate) struct HttpFailure {
    pub status: u16,
    pub retry_after: Option<Duration>,
}
impl std::fmt::Display for HttpFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HTTP {}", self.status)
    }
}
impl std::error::Error for HttpFailure {}
impl Client {
    pub(crate) fn shared() -> Self {
        static SHARED: OnceLock<Arc<Shared>> = OnceLock::new();
        Self {
            shared: SHARED
                .get_or_init(|| {
                    let agent = Agent::config_builder()
                        .user_agent("Windows-Media-Center/0.1")
                        .tls_config(
                            TlsConfig::builder()
                                .root_certs(RootCerts::PlatformVerifier)
                                .build(),
                        )
                        .timeout_global(Some(Duration::from_secs(15)))
                        .timeout_connect(Some(Duration::from_secs(5)))
                        .timeout_resolve(Some(Duration::from_secs(5)))
                        .max_redirects(3)
                        .build()
                        .new_agent();
                    Arc::new(Shared {
                        agent,
                        active: Mutex::new((0, 0)),
                        available: Condvar::new(),
                    })
                })
                .clone(),
        }
    }
    pub(crate) fn get(&self, url: impl AsRef<str>) -> Request {
        Request::new("GET", url.as_ref())
    }
    pub(crate) fn post(&self, url: impl AsRef<str>) -> Request {
        Request::new("POST", url.as_ref())
    }
    pub(crate) fn json(&self, request: Request) -> Result<Value> {
        serde_json::from_slice(&self.bytes(request, 4 * 1024 * 1024)?)
            .context("来源没有返回有效 JSON")
    }
    pub(crate) fn bytes(&self, request: Request, limit: usize) -> Result<Vec<u8>> {
        let _slot = self.shared.acquire(&request.cancel, request.translation)?;
        request.cancel.check()?;
        let url = request.url()?;
        let mut builder = ureq::http::Request::builder()
            .method(request.method)
            .uri(url.as_str());
        for (name, value) in &request.headers {
            builder = builder.header(name.as_str(), value.as_str());
        }
        let authenticated = request.headers.iter().any(|(name, _)| {
            name.eq_ignore_ascii_case("authorization") || name.to_lowercase().contains("key")
        });
        let timeout = request.timeout;
        let cancel = request.cancel;
        let request = builder.body(request.body)?;
        let mut response = request
            .with_agent(&self.shared.agent)
            .configure()
            .timeout_global(Some(timeout))
            .max_redirects(if authenticated { 0 } else { 3 })
            .http_status_as_error(false)
            .run()?;
        if !response.status().is_success() {
            return Err(HttpFailure {
                status: response.status().as_u16(),
                retry_after: response
                    .headers()
                    .get("retry-after")
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok())
                    .map(Duration::from_secs),
            }
            .into());
        }
        let mut bytes = Vec::new();
        let mut reader = response.body_mut().as_reader().take(limit as u64 + 1);
        let mut chunk = [0; 16 * 1024];
        loop {
            cancel.check()?;
            let count = reader.read(&mut chunk)?;
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..count]);
        }
        cancel.check()?;
        ensure!(bytes.len() <= limit, "响应超过大小限制");
        Ok(bytes)
    }
}
impl Request {
    fn new(method: &'static str, url: &str) -> Self {
        Self {
            method,
            url: url.into(),
            headers: vec![],
            query: vec![],
            body: vec![],
            timeout: Duration::from_secs(15),
            translation: false,
            cancel: Cancellation::default(),
        }
    }
    pub(crate) fn header(mut self, name: &str, value: &str) -> Self {
        self.headers
            .retain(|(existing, _)| !existing.eq_ignore_ascii_case(name));
        self.headers.push((name.into(), value.into()));
        self
    }
    pub(crate) fn query(mut self, pairs: &[(&str, &str)]) -> Self {
        self.query.extend(
            pairs
                .iter()
                .map(|(key, value)| ((*key).into(), (*value).into())),
        );
        self
    }
    pub(crate) fn json(self, value: &Value) -> Self {
        self.header("Content-Type", "application/json")
            .body(value.to_string())
    }
    pub(crate) fn body(mut self, body: String) -> Self {
        self.body = body.into_bytes();
        self
    }
    pub(crate) fn translation(mut self, cancel: Cancellation) -> Self {
        self.translation = true;
        self.cancel = cancel;
        self.timeout = Duration::from_secs(30);
        self
    }
    fn url(&self) -> Result<url::Url> {
        let mut url = url::Url::parse(&self.url).context("请求 URL 无效")?;
        if !self.query.is_empty() {
            url.query_pairs_mut().extend_pairs(&self.query);
        }
        Ok(url)
    }
}
impl Shared {
    fn acquire(&self, cancel: &Cancellation, translation: bool) -> Result<Slot<'_>> {
        let started = std::time::Instant::now();
        let mut active = self.active.lock().unwrap_or_else(|e| e.into_inner());
        while active.0 >= 3 || (translation && active.1 >= 2) {
            cancel.check()?;
            ensure!(
                started.elapsed() < Duration::from_secs(30),
                "网络任务排队超时"
            );
            active = self
                .available
                .wait_timeout(active, Duration::from_millis(50))
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
        cancel.check()?;
        active.0 += 1;
        if translation {
            active.1 += 1;
        }
        Ok(Slot(self, translation))
    }
}
struct Slot<'a>(&'a Shared, bool);
impl Drop for Slot<'_> {
    fn drop(&mut self) {
        let mut active = self.0.active.lock().unwrap_or_else(|e| e.into_inner());
        active.0 -= 1;
        if self.1 {
            active.1 -= 1;
        }
        self.0.available.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    #[test]
    fn shared_agent_and_unicode_query_encoding() {
        let a = Client::shared();
        let b = Client::shared();
        assert!(Arc::ptr_eq(&a.shared, &b.shared));
        let url = a
            .get("https://example.com/?existing=1")
            .query(&[("s", "你好 &世界")])
            .url()
            .unwrap();
        assert_eq!(
            url.query_pairs().find(|(key, _)| key == "s").unwrap().1,
            "你好 &世界"
        );
        assert!(
            url.query_pairs()
                .any(|(key, value)| key == "existing" && value == "1")
        );
    }
    fn server(status: &str, body: &[u8]) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes();
        let body = body.to_vec();
        let task = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut input = [0; 4096];
            let _ = stream.read(&mut input).unwrap();
            stream.write_all(&response).unwrap();
            stream.write_all(&body).unwrap();
        });
        (url, task)
    }
    #[test]
    fn response_limits_and_http_errors_are_enforced() {
        let client = Client::shared();
        let (url, task) = server("200 OK", b"12345");
        assert!(client.bytes(client.get(url), 4).is_err());
        task.join().unwrap();
        let (url, task) = server("404 Not Found", b"missing");
        assert!(client.bytes(client.get(url), 100).is_err());
        task.join().unwrap();
        let (url, task) = server("200 OK", b"fine");
        assert_eq!(client.bytes(client.get(url), 4).unwrap(), b"fine");
        task.join().unwrap();
    }
    #[test]
    fn rate_limit_metadata_and_cancellation_are_preserved() {
        let client = Client::shared();
        let (url, task) = server("429 Too Many Requests\r\nRetry-After: 7", b"limited");
        let error = client.bytes(client.get(url), 100).unwrap_err();
        let failure = error.downcast_ref::<HttpFailure>().unwrap();
        assert_eq!(failure.status, 429);
        assert_eq!(failure.retry_after, Some(Duration::from_secs(7)));
        task.join().unwrap();
        let cancel = Cancellation::default();
        cancel.cancel();
        let error = client
            .bytes(client.get("http://127.0.0.1:1/").translation(cancel), 100)
            .unwrap_err();
        assert!(error.to_string().contains("取消"));
    }
}

//! Stand-in for the generated client's transport: base URL, D12 host policy,
//! D13 deadline, redirects disabled (D4), and an observation log for tests.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use reqwest::header::HeaderMap;
use serde_json::Value;
use tokio::time::Instant;
use url::{Origin, Url};

use crate::error::PagerError;

pub(crate) const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Delete,
}

/// What the client dispatched, recorded before the network send so tests can
/// compare the client's view with the mock server's.
#[derive(Debug, Clone)]
pub struct SentRequest {
    pub method: Method,
    pub url: String,
    pub timeout: Duration,
}

#[derive(Clone)]
pub struct Client {
    pub(crate) inner: Arc<ClientInner>,
}

pub(crate) struct ClientInner {
    http: reqwest::Client,
    pub(crate) base: Url,
    /// `None`: no host check (D12: today's behavior when the policy is off).
    policy: Option<Vec<Origin>>,
    pub(crate) sent: Mutex<Vec<SentRequest>>,
    pub(crate) leak_warnings: Mutex<Vec<String>>,
    pub(crate) close_on_drop: bool,
}

impl Client {
    pub fn new(base: &str) -> Self {
        Self::build(base, None, false)
    }

    /// D12. Credentials (and, in this spike, every request) go only to the base
    /// origin or one of `extra_origins`, compared by scheme, host, and
    /// effective port.
    pub fn with_host_policy(base: &str, extra_origins: &[&str]) -> Self {
        let extra = extra_origins
            .iter()
            .map(|o| Url::parse(o).expect("extra origin is a URL").origin())
            .collect();
        Self::build(base, Some(extra), false)
    }

    /// Q4 option (b), for comparison only: an early-dropped session spawns a
    /// detached close task on the current runtime.
    pub fn with_close_on_drop(base: &str) -> Self {
        Self::build(base, None, true)
    }

    fn build(base: &str, policy: Option<Vec<Origin>>, close_on_drop: bool) -> Self {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("reqwest client");
        Self {
            inner: Arc::new(ClientInner {
                http,
                base: Url::parse(base).expect("base URL"),
                policy,
                sent: Mutex::new(Vec::new()),
                leak_warnings: Mutex::new(Vec::new()),
                close_on_drop,
            }),
        }
    }

    pub fn sent(&self) -> Vec<SentRequest> {
        self.inner.sent.lock().unwrap().clone()
    }

    /// Stand-in for `tracing::warn!` from a dropped, still-open session.
    pub fn leak_warnings(&self) -> Vec<String> {
        self.inner.leak_warnings.lock().unwrap().clone()
    }
}

pub(crate) struct Raw {
    pub headers: HeaderMap,
    pub body: Value,
}

pub(crate) struct Req {
    pub method: Method,
    pub url: Url,
    pub body: Option<Value>,
}

/// D13: one budget for the whole operation, on tokio's monotonic clock.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Deadline {
    pub started: Instant,
    pub budget: Duration,
}

impl Deadline {
    pub fn remaining(&self) -> Duration {
        self.budget.saturating_sub(self.started.elapsed())
    }
}

impl ClientInner {
    pub(crate) fn resolve(&self, link: &str) -> Result<Url, PagerError> {
        self.base
            .join(link)
            .map_err(|e| PagerError::InvalidContinuation(format!("{link:?} is not a URL: {e}")))
    }

    fn check_origin(&self, url: &Url) -> Result<(), PagerError> {
        let Some(extra) = &self.policy else {
            return Ok(());
        };
        let origin = url.origin();
        if origin == self.base.origin() || extra.contains(&origin) {
            return Ok(());
        }
        Err(PagerError::HostPolicy {
            url: url.to_string(),
            origin: origin.ascii_serialization(),
        })
    }

    /// Sends one request with `timeout` covering the send and the body read.
    /// The returned future owns the in-flight connection: dropping it aborts
    /// the request, and nothing is spawned.
    pub(crate) async fn send(&self, req: Req, timeout: Duration) -> Result<Result<Raw, PagerError>, tokio::time::error::Elapsed> {
        if let Err(e) = self.check_origin(&req.url) {
            return Ok(Err(e));
        }
        self.sent.lock().unwrap().push(SentRequest {
            method: req.method,
            url: req.url.to_string(),
            timeout,
        });
        let method = match req.method {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
            Method::Delete => reqwest::Method::DELETE,
        };
        let mut builder = self.http.request(method, req.url);
        if let Some(body) = &req.body {
            builder = builder.json(body);
        }
        tokio::time::timeout(timeout, async move {
            let response = builder.send().await.map_err(|e| PagerError::Transport(e.to_string()))?;
            let status = response.status().as_u16();
            let headers = response.headers().clone();
            let bytes = response.bytes().await.map_err(|e| PagerError::Transport(e.to_string()))?;
            if !(200..300).contains(&status) {
                return Err(PagerError::Http {
                    status,
                    body: String::from_utf8_lossy(&bytes).into_owned(),
                });
            }
            let body = if bytes.is_empty() {
                Value::Null
            } else {
                serde_json::from_slice(&bytes).map_err(|e| PagerError::Decode(e.to_string()))?
            };
            Ok(Raw { headers, body })
        })
        .await
    }
}

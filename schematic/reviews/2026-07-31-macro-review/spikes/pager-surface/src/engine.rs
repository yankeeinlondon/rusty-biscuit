//! The one pagination engine. Nothing here is generic over the endpoint, so
//! 157 endpoints share one copy of this code; the generic layer in
//! [`crate::pager`] only decodes items into `E::Item`.

use std::collections::HashSet;
use std::time::Duration;

use reqwest::header::HeaderMap;
use serde_json::{Map, Value};
use tokio::time::Instant;
use url::Url;

use crate::client::{Client, DEFAULT_REQUEST_TIMEOUT, Deadline, Method, Req};
use crate::error::PagerError;
use crate::spec::{CursorLeg, CursorSpec, EndpointSpec, Input, LinkFrom, LinkSpec, More, PageNumberSpec, ScrollSpec, ValueAt};

pub(crate) struct RawPage {
    pub items: Vec<Value>,
    pub headers: HeaderMap,
    pub total: Option<u64>,
}

#[doc(hidden)]
pub enum KindState {
    PageNumber {
        spec: &'static PageNumberSpec,
        next: u64,
    },
    Cursor {
        spec: &'static CursorSpec,
        next: Option<String>,
        /// `.before(c)`: the first request goes backward from `c` on this leg.
        start_backward: Option<&'static CursorLeg>,
    },
    Link {
        spec: &'static LinkSpec,
        next: Option<Url>,
    },
    Scroll {
        spec: &'static ScrollSpec,
        handle: Option<String>,
        keep_alive: String,
        closed: bool,
    },
}

impl KindState {
    pub(crate) fn page_number(spec: &'static PageNumberSpec) -> Self {
        Self::PageNumber { spec, next: spec.base }
    }
    pub(crate) fn cursor(spec: &'static CursorSpec) -> Self {
        Self::Cursor {
            spec,
            next: None,
            start_backward: None,
        }
    }
    pub(crate) fn link(spec: &'static LinkSpec) -> Self {
        Self::Link { spec, next: None }
    }
    pub(crate) fn scroll(spec: &'static ScrollSpec) -> Self {
        Self::Scroll {
            spec,
            handle: None,
            keep_alive: spec.default_keep_alive.to_owned(),
            closed: false,
        }
    }
}

#[doc(hidden)]
pub enum PrevState {
    Cursor(&'static CursorLeg),
    Link(&'static LinkFrom),
}

pub(crate) struct Engine {
    client: Client,
    endpoint: &'static EndpointSpec,
    pub(crate) state: KindState,
    /// The last response, kept so `prev()` can read the backward
    /// continuation on demand (the backward leg is only known to the gated
    /// `prev()` call, not to the engine).
    last: Option<(Value, HeaderMap)>,
    pub(crate) size: Option<(Input, u32)>,
    pub(crate) query: Vec<(String, String)>,
    pub(crate) deadline: Option<Duration>,
    clock: Option<Deadline>,
    completed_pages: usize,
    /// Every forward continuation followed. A repeat is a loop, whether it is
    /// the same value twice in a row or a longer cycle.
    seen: HashSet<String>,
    started: bool,
    done: bool,
    failed: Option<String>,
}

#[derive(Default)]
struct Draft {
    query: Vec<(String, String)>,
    body: Map<String, Value>,
}

impl Draft {
    fn put(&mut self, input: Input, value: Value) {
        match input {
            Input::Query(name) => self.query.push((name.to_owned(), scalar_text(&value))),
            Input::Body(name) => {
                self.body.insert(name.to_owned(), value);
            }
        }
    }

    fn into_req(self, method: Method, mut url: Url) -> Req {
        if !self.query.is_empty() {
            url.query_pairs_mut().extend_pairs(self.query);
        }
        Req {
            method,
            url,
            body: (!self.body.is_empty()).then_some(Value::Object(self.body)),
        }
    }
}

fn scalar_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn lookup<'v>(body: &'v Value, path: &[&str]) -> Option<&'v Value> {
    path.iter().try_fold(body, |v, key| v.get(key))
}

fn read(at: ValueAt, raw_body: &Value, headers: &HeaderMap) -> Option<Value> {
    match at {
        ValueAt::Body(path) => lookup(raw_body, path).cloned(),
        ValueAt::Header(name) => headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|s| Value::String(s.to_owned())),
    }
}

/// A continuation token: missing, null, and `""` are terminal; scalars are
/// accepted; anything else is invalid.
fn token(value: Option<Value>) -> Result<Option<String>, PagerError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s.is_empty() => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        Some(Value::Number(n)) => Ok(Some(n.to_string())),
        Some(other) => Err(PagerError::InvalidContinuation(format!("expected a string or number, got {other}"))),
    }
}

/// Minimal RFC 8288 parse: `<url>; rel="next", <url>; rel="prev"`.
fn link_rel(headers: &HeaderMap, rel: &str) -> Option<String> {
    let header = headers.get("link")?.to_str().ok()?;
    header.split(',').find_map(|part| {
        let (target, params) = part.split_once(';')?;
        let has_rel = params.split(';').any(|p| {
            let p = p.trim();
            p.strip_prefix("rel=")
                .map(|v| v.trim_matches('"').split_whitespace().any(|r| r == rel))
                .unwrap_or(false)
        });
        has_rel.then(|| target.trim().trim_start_matches('<').trim_end_matches('>').to_owned())
    })
}

fn link_from(from: LinkFrom, body: &Value, headers: &HeaderMap) -> Result<Option<String>, PagerError> {
    match from {
        LinkFrom::Header(rel) => Ok(link_rel(headers, rel)),
        LinkFrom::Body(path) => token(lookup(body, path).cloned()),
    }
}

fn more(signal: More, items: &[Value], body: &Value, headers: &HeaderMap) -> bool {
    match signal {
        More::NonEmptyPage => !items.is_empty(),
        More::LinkRel(rel) => link_rel(headers, rel).is_some(),
        More::Flag(at) => read(at, body, headers).and_then(|v| v.as_bool()).unwrap_or(false),
    }
}

impl Engine {
    pub(crate) fn new(client: Client, endpoint: &'static EndpointSpec, state: KindState) -> Self {
        Self {
            client,
            endpoint,
            state,
            last: None,
            size: None,
            query: Vec::new(),
            deadline: None,
            clock: None,
            completed_pages: 0,
            seen: HashSet::new(),
            started: false,
            done: false,
            failed: None,
        }
    }

    pub(crate) fn start_cursor(&mut self, cursor: String, backward: Option<&'static CursorLeg>) {
        if let KindState::Cursor {
            next, start_backward, ..
        } = &mut self.state
        {
            *next = Some(cursor);
            *start_backward = backward;
        }
    }

    pub(crate) fn start_page(&mut self, page: u64) {
        if let KindState::PageNumber { next, .. } = &mut self.state {
            *next = page;
        }
    }

    pub(crate) fn set_keep_alive(&mut self, value: String) {
        if let KindState::Scroll { keep_alive, .. } = &mut self.state {
            *keep_alive = value;
        }
    }

    fn deadline_error(&self) -> PagerError {
        PagerError::DeadlineExceeded {
            completed_pages: self.completed_pages,
            elapsed: self.clock.map(|c| c.started.elapsed()).unwrap_or_default(),
        }
    }

    /// One request under the shared deadline. Refuses to dispatch once the
    /// budget is spent; otherwise the request gets exactly the remaining time.
    async fn fetch(&mut self, req: Req) -> Result<crate::client::Raw, PagerError> {
        if self.clock.is_none()
            && let Some(budget) = self.deadline
        {
            self.clock = Some(Deadline {
                started: Instant::now(),
                budget,
            });
        }
        let timeout = match self.clock {
            Some(clock) => {
                let remaining = clock.remaining();
                if remaining.is_zero() {
                    return Err(self.deadline_error());
                }
                remaining
            }
            None => DEFAULT_REQUEST_TIMEOUT,
        };
        match self.client.inner.send(req, timeout).await {
            Ok(result) => result,
            Err(_elapsed) if self.clock.is_some() => Err(self.deadline_error()),
            Err(_elapsed) => Err(PagerError::Transport(format!("request timed out after {timeout:?}"))),
        }
    }

    fn base_url(&self, path: &str) -> Result<Url, PagerError> {
        self.client.inner.resolve(path)
    }

    fn first_draft(&self) -> Draft {
        let mut draft = Draft {
            query: self.query.clone(),
            ..Draft::default()
        };
        if let Some((input, n)) = self.size {
            draft.put(input, Value::from(n));
        }
        draft
    }

    fn forward_request(&self) -> Result<Req, PagerError> {
        let endpoint_url = self.base_url(self.endpoint.path)?;
        let method = self.endpoint.method;
        Ok(match &self.state {
            KindState::PageNumber { spec, next } => {
                let mut draft = self.first_draft();
                draft.put(spec.page, Value::from(*next));
                draft.into_req(method, endpoint_url)
            }
            KindState::Cursor {
                spec,
                next,
                start_backward,
                ..
            } => {
                let mut draft = self.first_draft();
                if let Some(cursor) = next {
                    let param = start_backward.map_or(spec.forward.param, |leg| leg.param);
                    draft.put(param, Value::String(cursor.clone()));
                }
                draft.into_req(method, endpoint_url)
            }
            KindState::Link { next, .. } => match next {
                // NextLink follow-ups are a verbatim GET of the link.
                Some(url) => Req {
                    method: Method::Get,
                    url: url.clone(),
                    body: None,
                },
                None => self.first_draft().into_req(method, endpoint_url),
            },
            KindState::Scroll {
                spec,
                handle,
                keep_alive,
                ..
            } => match handle {
                None => {
                    let mut draft = self.first_draft();
                    draft.put(spec.open_keep_alive, Value::String(keep_alive.clone()));
                    draft.into_req(method, endpoint_url)
                }
                Some(handle) => {
                    let mut draft = Draft::default();
                    draft.put(spec.keep_alive_param, Value::String(keep_alive.clone()));
                    draft.put(spec.handle_param, Value::String(handle.clone()));
                    draft.into_req(Method::Post, self.base_url(spec.follow_path)?)
                }
            },
        })
    }

    pub(crate) async fn next_raw(&mut self) -> Result<Option<RawPage>, PagerError> {
        if let Some(previous) = &self.failed {
            return Err(match self.clock {
                Some(clock) if clock.remaining().is_zero() => self.deadline_error(),
                _ => PagerError::Failed {
                    previous: previous.clone(),
                },
            });
        }
        if self.done {
            return Ok(None);
        }
        let result = self.next_inner().await;
        match result {
            Err(e) => {
                self.failed = Some(e.to_string());
                // Sessions close on any error; the close error is secondary.
                let _ = self.close().await;
                Err(e)
            }
            Ok(None) => {
                self.done = true;
                self.close().await?;
                Ok(None)
            }
            ok => ok,
        }
    }

    async fn next_inner(&mut self) -> Result<Option<RawPage>, PagerError> {
        let req = self.forward_request()?;
        self.started = true;
        let raw = self.fetch(req).await?;
        let page = self.page_from(&raw.body, &raw.headers)?;
        let end = self.advance(&page.items, &raw.body, &raw.headers)?;
        self.last = Some((raw.body, raw.headers));
        if end && page.items.is_empty() {
            return Ok(None);
        }
        if end {
            self.done = true;
            self.close().await?;
        }
        self.completed_pages += 1;
        Ok(Some(page))
    }

    fn page_from(&self, body: &Value, headers: &HeaderMap) -> Result<RawPage, PagerError> {
        let items = match lookup(body, self.endpoint.items) {
            Some(Value::Array(items)) => items.clone(),
            Some(other) => return Err(PagerError::Decode(format!("expected an item array, got {other}"))),
            None => return Err(PagerError::Decode(format!("no item array at {:?}", self.endpoint.items))),
        };
        let total = self
            .endpoint
            .total
            .and_then(|at| read(at, body, headers))
            .and_then(|v| v.as_u64().or_else(|| v.as_str()?.parse().ok()));
        Ok(RawPage {
            items,
            headers: headers.clone(),
            total,
        })
    }

    fn mark_seen(&mut self, continuation: &str) -> Result<(), PagerError> {
        if !self.seen.insert(continuation.to_owned()) {
            return Err(PagerError::Stalled {
                continuation: continuation.to_owned(),
                completed_pages: self.completed_pages,
            });
        }
        Ok(())
    }

    /// Reads the continuation from this page; returns `true` when this page is
    /// the last.
    fn advance(&mut self, items: &[Value], body: &Value, headers: &HeaderMap) -> Result<bool, PagerError> {
        match &mut self.state {
            KindState::PageNumber { spec, next } => {
                if !more(spec.more, items, body, headers) {
                    return Ok(true);
                }
                *next += 1;
                Ok(false)
            }
            KindState::Cursor {
                spec,
                next,
                start_backward,
            } => {
                *start_backward = None;
                let cont = token(read(spec.forward.from, body, headers))?;
                let more = spec.more.map(|m| more(m, items, body, headers));
                let cont = match (more, cont) {
                    (Some(false), _) => return Ok(true),
                    (Some(true), None) => return Err(PagerError::MissingContinuation),
                    (None, None) => return Ok(true),
                    (_, Some(c)) => c,
                };
                *next = Some(cont.clone());
                self.mark_seen(&cont)?;
                Ok(false)
            }
            KindState::Link { spec, next } => {
                let Some(link) = link_from(spec.next, body, headers)? else {
                    return Ok(true);
                };
                let url = self.client.inner.resolve(&link)?;
                *next = Some(url.clone());
                self.mark_seen(url.as_str())?;
                Ok(false)
            }
            KindState::Scroll { spec, handle, .. } => {
                // The handle may change per page; the latest wins. A repeated
                // handle is normal for scroll, so it is not loop evidence: the
                // end signal is the empty page.
                if let Some(h) = token(read(spec.handle_from, body, headers))? {
                    *handle = Some(h);
                }
                if handle.is_none() {
                    return Err(PagerError::MissingContinuation);
                }
                Ok(items.is_empty())
            }
        }
    }

    /// Moves one page backward along `prev`. `Ok(None)` when the last page
    /// carried no previous continuation.
    pub(crate) async fn prev_raw(&mut self, prev: PrevState) -> Result<Option<RawPage>, PagerError> {
        let Some((body, headers)) = &self.last else {
            return Ok(None);
        };
        let req = match prev {
            PrevState::Cursor(leg) => {
                let Some(cursor) = token(read(leg.from, body, headers))? else {
                    return Ok(None);
                };
                let mut draft = self.first_draft();
                draft.put(leg.param, Value::String(cursor));
                draft.into_req(self.endpoint.method, self.base_url(self.endpoint.path)?)
            }
            PrevState::Link(from) => {
                let Some(link) = link_from(*from, body, headers)? else {
                    return Ok(None);
                };
                Req {
                    method: Method::Get,
                    url: self.client.inner.resolve(&link)?,
                    body: None,
                }
            }
        };
        let raw = self.fetch(req).await?;
        let page = self.page_from(&raw.body, &raw.headers)?;
        // Going backward revisits continuations legitimately.
        self.seen.clear();
        self.done = self.advance(&page.items, &raw.body, &raw.headers)?;
        self.last = Some((raw.body, raw.headers));
        self.completed_pages += 1;
        Ok(Some(page))
    }

    /// Releases a server-side session. Idempotent; a no-op for kinds without
    /// sessions and for a session that never opened.
    pub(crate) async fn close(&mut self) -> Result<(), PagerError> {
        let KindState::Scroll {
            spec, handle, closed, ..
        } = &mut self.state
        else {
            return Ok(());
        };
        if *closed {
            return Ok(());
        }
        *closed = true;
        let Some(handle) = handle.clone() else {
            return Ok(());
        };
        let spec = *spec;
        let mut draft = Draft::default();
        draft.put(spec.handle_param, Value::String(handle));
        let req = draft.into_req(Method::Delete, self.base_url(spec.close_path)?);
        self.fetch(req).await.map(|_| ()).map_err(|e| PagerError::Close(Box::new(e)))
    }

    fn open_session(&self) -> Option<(&'static ScrollSpec, String)> {
        match &self.state {
            KindState::Scroll {
                spec,
                handle: Some(handle),
                closed: false,
                ..
            } => Some((*spec, handle.clone())),
            _ => None,
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let Some((spec, handle)) = self.open_session() else {
            return;
        };
        let inner = &self.client.inner;
        if inner.close_on_drop
            && let Ok(runtime) = tokio::runtime::Handle::try_current()
        {
            // Q4 option (b): detached, unobservable, and outlives the caller.
            let client = self.client.clone();
            let mut draft = Draft::default();
            draft.put(spec.handle_param, Value::String(handle));
            if let Ok(url) = inner.resolve(spec.close_path) {
                let req = draft.into_req(Method::Delete, url);
                runtime.spawn(async move {
                    let _ = client.inner.send(req, DEFAULT_REQUEST_TIMEOUT).await;
                });
            }
            return;
        }
        inner.leak_warnings.lock().unwrap().push(format!(
            "session {handle} on {} dropped without close(); the server lease expires on its own",
            spec.close_path
        ));
    }
}

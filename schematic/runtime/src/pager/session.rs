//! Scoped sessions: a server-side result set (an Elasticsearch scroll or
//! point in time) iterated only inside [`Pager::with_session`], which
//! attempts to close it when the scope ends.
//!
//! There is no `open()` or `close()` to call and nothing to drop. The scope
//! opens the session when its first page is requested, tracks the latest
//! handle the server returned, and, once the closure's future completes,
//! returns an error, or panics, sends one close request bounded by its own
//! grace. Rust has no asynchronous drop and the runtime spawns no task, so a
//! scope whose future is cancelled, a process that aborts on panic or
//! terminates, and a cancellation while opening or closing all leave the
//! session to the server's lease.

use std::future::poll_fn;
use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::{Mutex, PoisonError};
use std::task::Poll;
use std::time::Duration;

use http::HeaderMap;
use serde_json::Value;

use super::engine::Engine;
use super::error::{PagerError, SessionError};
use super::json::{self, Read};
use super::pages::{CollectResult, PageResult, Pager, Pages, collect, fetch_first, partition_count};
use super::request::{StepDocument, StepRequest, fill_path, write_input};
use super::spec::{Input, Session, SessionOpen, SessionStep, Source};
use super::{KeepAliveKnob, Paginated, Partitioned, Renewable, Sessioned};
use crate::call::CallFuture;
use crate::client::Client;
use crate::deadline::Deadline;
use crate::endpoint::{Api, EndpointSpec, RequestBody, RequestParts};
use crate::error::InvalidArgument;
use crate::meta::ResponseMeta;
use crate::options::CallOptions;
use crate::response::Outcome;

/// How long a session close may take when the caller sets nothing, bounded
/// further by the client's per-request timeout.
pub const DEFAULT_CLOSE_GRACE: Duration = Duration::from_secs(5);

/// What [`Pager::with_session`] resolves to: the closure's output, or that
/// output with the close failure.
pub type SessionResult<O, R> = Result<O, SessionError<O, <R as EndpointSpec>::Failure>>;

impl<'a, A: Api, R: Paginated<Api = A>> Pager<'a, A, R> {
    /// Sends `value` as the session's keep-alive instead of the authored
    /// default, in the provider's own format (for example `"5m"`).
    pub fn keep_alive(mut self, value: impl Into<String>) -> Self
    where
        R: KeepAliveKnob,
    {
        self.keep_alive = Some(value.into());
        self
    }

    /// Bounds the close request of [`with_session`](Self::with_session) by
    /// `grace` (default [`DEFAULT_CLOSE_GRACE`]), further limited by the
    /// client's per-request timeout. It is separate from any shared
    /// deadline: the close is still sent after that deadline expired, and
    /// the grace never extends page fetching.
    pub fn close_grace(mut self, grace: Duration) -> Self
    where
        R: Sessioned,
    {
        self.close_grace = grace;
        self
    }

    /// The grace [`with_session`](Self::with_session) gives its close
    /// request.
    pub fn configured_close_grace(&self) -> Duration {
        self.close_grace
    }

    /// Runs `scope` with this endpoint's server-side session, then closes
    /// the session.
    ///
    /// Inside `scope`, the [`SessionScope`] iterates, collects, partitions,
    /// and renews; nothing outside it can request a page of a session
    /// endpoint. The session opens with the first page request, by a
    /// separate open request or by the page itself, as the definition
    /// authors it.
    ///
    /// When the closure's future returns, or panics while it is built or
    /// polled under panic unwinding, one close request is sent with the
    /// latest handle the server returned, if any was returned and the
    /// definition authors a close. That request is bounded by the
    /// [close grace](Self::close_grace) rather than by the pages' deadline.
    /// A panic resumes after the close attempt; its close failure cannot be
    /// returned.
    ///
    /// The closure returns a boxed future that may borrow the scope:
    ///
    /// ```text
    /// let hits = es.search(index).page_size(500)
    ///     .with_session(|scope| Box::pin(async move { scope.collect_all().await }))
    ///     .await?;
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns the closure's output unchanged when the close succeeds or no
    /// close is needed, and [`SessionError`] holding that output and the
    /// close failure when the close fails or runs out of grace.
    pub fn with_session<F, O>(self, scope: F) -> CallFuture<'a, SessionResult<O, R>>
    where
        R: Sessioned,
        // A boxed `Send` future rather than an async closure: rustc cannot
        // prove an async closure's future `Send` for every borrow of the
        // scope, which would make a session impossible to spawn.
        F: for<'s> FnOnce(&'s mut SessionScope<'a, A, R>) -> CallFuture<'s, O> + Send + 'a,
        O: Send + 'a,
    {
        Box::pin(self.run_session(scope))
    }

    async fn run_session<F, O>(self, scope: F) -> SessionResult<O, R>
    where
        F: for<'s> FnOnce(&'s mut SessionScope<'a, A, R>) -> CallFuture<'s, O>,
    {
        let grace = self.close_grace;
        let keep_alive = self.keep_alive;
        let mut session = SessionScope {
            client: self.client,
            base: self.request.into_parts(),
            options: self.options,
            engine: self.engine,
            page_limit: self.page_limit,
            shared: SessionShared::new(R::STRATEGY.session.as_ref(), keep_alive),
            endpoint: PhantomData,
        };
        let outcome = run_unwinding(&mut session, scope).await;
        let close = session.shared.close::<A, R>(session.client, &session.base, &session.options, grace).await;
        match (outcome, close) {
            (Err(panic), _) => resume_unwind(panic),
            (Ok(outcome), Ok(())) => Ok(outcome),
            (Ok(outcome), Err(close)) => Err(SessionError { outcome, close }),
        }
    }
}

/// Runs `scope` on `state`, catching a panic while its future is built or
/// polled, so the caller can clean up before resuming it.
async fn run_unwinding<S, F, O>(state: &mut S, scope: F) -> std::thread::Result<O>
where
    F: for<'s> FnOnce(&'s mut S) -> CallFuture<'s, O>,
{
    let mut future = catch_unwind(AssertUnwindSafe(|| scope(state)))?;
    poll_fn(|cx| match catch_unwind(AssertUnwindSafe(|| future.as_mut().poll(cx))) {
        Ok(Poll::Ready(output)) => Poll::Ready(Ok(output)),
        Ok(Poll::Pending) => Poll::Pending,
        Err(panic) => Poll::Ready(Err(panic)),
    })
    .await
}

/// The session a [`Pager::with_session`] closure works in.
///
/// Every traversal it starts shares the one session: its open request, its
/// latest handle, and the close sent when the scope ends. Each traversal
/// starts at the position the pager was configured with.
pub struct SessionScope<'a, A: Api, R: EndpointSpec> {
    client: &'a Client<A>,
    base: Result<RequestParts, InvalidArgument>,
    options: CallOptions,
    engine: Engine,
    page_limit: u32,
    shared: SessionShared<R::Failure>,
    endpoint: PhantomData<fn() -> R>,
}

impl<'a, A: Api, R: Paginated<Api = A>> SessionScope<'a, A, R> {
    /// Starts a traversal of the session; nothing is sent until
    /// [`Pages::next`].
    pub fn pages(&mut self) -> Pages<'_, A, R> {
        Pages::new(
            self.client,
            self.base.clone(),
            self.options.clone(),
            self.engine.clone(),
            Some(&self.shared),
            None,
        )
    }

    /// See [`Pager::fetch_page`].
    pub fn fetch_page(&mut self) -> CallFuture<'_, PageResult<R>> {
        fetch_first(self.pages())
    }

    /// See [`Pager::collect_all`]; the page limit is the pager's.
    pub fn collect_all(&mut self) -> CallFuture<'_, CollectResult<R>> {
        let limit = self.page_limit;
        collect(self.pages(), limit)
    }

    /// `count` independent traversals of the session's slices: traversal
    /// `i` sends slice index `i` and the slice count `count` on every
    /// request. They share the session and its latest handle; the caller
    /// polls them, in any order or concurrently, and nothing is spawned.
    ///
    /// ## Errors
    ///
    /// Returns [`InvalidArgument`] when `count` is zero or above the
    /// definition's maximum; nothing is sent.
    pub fn partitioned(&mut self, count: u32) -> Result<Vec<Pages<'_, A, R>>, InvalidArgument>
    where
        R: Partitioned,
    {
        partition_count::<R>(count)?;
        let shared = &self.shared;
        Ok((0..count)
            .map(|index| {
                Pages::new(
                    self.client,
                    self.base.clone(),
                    self.options.clone(),
                    self.engine.clone(),
                    Some(shared),
                    Some((index, count)),
                )
            })
            .collect())
    }

    /// Sends the definition's renew request with the latest handle; `false`
    /// when no session handle is known yet, so nothing was sent.
    ///
    /// ## Errors
    ///
    /// Returns [`PagerError::Transport`] or [`PagerError::Api`] when the
    /// renew request fails.
    pub fn renew(&self) -> CallFuture<'_, Result<bool, PagerError<R::Failure>>>
    where
        R: Renewable,
    {
        Box::pin(self.shared.renew::<A, R>(self.client, &self.base, &self.options))
    }
}

impl<A: Api, R: EndpointSpec> std::fmt::Debug for SessionScope<'_, A, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionScope")
            .field("api", &A::NAME)
            .field("endpoint", &R::ENDPOINT_ID)
            .field("open", &self.shared.handle().is_some())
            .finish_non_exhaustive()
    }
}

/// What every traversal of one scope shares.
pub(crate) struct SessionShared<E> {
    /// `None` only for a `Paginated` impl whose `Session` type disagrees
    /// with its strategy; the scope then neither opens nor closes.
    spec: Option<&'static Session>,
    /// The keep-alive value sent: the caller's, else the authored default.
    keep_alive: Option<String>,
    /// The latest handle any traversal observed; it replaces the previous
    /// one in the order responses arrive.
    handle: Mutex<Option<String>>,
    /// Held while the open request is in flight, so concurrent partitions
    /// send it once.
    open: tokio::sync::Mutex<OpenState<E>>,
}

#[derive(Debug)]
enum OpenState<E> {
    Pending,
    Open,
    Failed(PagerError<E>),
}

impl<E> SessionShared<E> {
    fn new(spec: Option<&'static Session>, keep_alive: Option<String>) -> Self {
        let keep_alive = spec
            .and_then(|spec| spec.keep_alive)
            .map(|authored| keep_alive.unwrap_or_else(|| authored.default.to_owned()));
        Self {
            spec,
            keep_alive,
            handle: Mutex::new(None),
            open: tokio::sync::Mutex::new(OpenState::Pending),
        }
    }

    fn handle(&self) -> Option<String> {
        self.handle.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn set_handle(&self, handle: String) {
        *self.handle.lock().unwrap_or_else(PoisonError::into_inner) = Some(handle);
    }

    /// Records the handle a page carries. Called before the page's items
    /// and continuation are read, so a page that then proves malformed
    /// still updates the handle the close uses.
    pub(crate) fn observe_page(&self, body: &Value, headers: &HeaderMap) {
        if let Some(source) = self.spec.and_then(|spec| spec.handle_from_page)
            && let Some(handle) = handle_text(source, body, headers)
        {
            self.set_handle(handle);
        }
    }

    /// Whether a page request opens the session: the first request of a
    /// traversal, for a session its first page opens.
    pub(crate) fn opens_with(&self, follow_up: bool) -> bool {
        !follow_up && self.spec.is_some_and(|spec| matches!(spec.open, SessionOpen::FirstPage))
    }

    /// Writes the session's part of a page request: the keep-alive, and,
    /// once the session is open, the session page target and the handle.
    pub(crate) fn apply_to_page(
        &self,
        parts: &mut RequestParts,
        opening: bool,
        followed_link: bool,
        endpoint_template: &str,
        endpoint_path: &str,
    ) -> Result<(), InvalidArgument> {
        let Some(spec) = self.spec else {
            return Ok(());
        };
        let keep_alive = spec.keep_alive.and_then(|keep_alive| if opening { keep_alive.on_open } else { keep_alive.on_page });
        if !opening {
            if !followed_link && let Some((method, template)) = &spec.pages {
                parts.method = method.clone();
                parts.path = fill_path(template, endpoint_template, endpoint_path)
                    .ok_or_else(|| InvalidArgument::new("the session page path cannot be filled from the endpoint path"))?;
            }
            if let Some(input) = spec.send_handle_as
                && let Some(handle) = self.handle()
            {
                write_input(parts, input, &Value::String(handle))?;
            }
        }
        if let (Some(input), Some(value)) = (keep_alive, &self.keep_alive) {
            write_input(parts, input, &Value::from(value.as_str()))?;
        }
        Ok(())
    }

    /// Opens the shared session once; concurrent traversals wait for it.
    ///
    /// ## Errors
    ///
    /// Returns the open request's failure, [`PagerError::MissingSessionHandle`]
    /// when its reply holds no handle. Later callers retain the complete
    /// failure unless explicitly retrying.
    pub(crate) async fn ensure_open<A: Api, R: Paginated<Api = A, Failure = E>>(
        &self,
        client: &Client<A>,
        base: &Result<RequestParts, InvalidArgument>,
        options: &CallOptions,
        retry: bool,
    ) -> Result<(), PagerError<R::Failure>> {
        let Some(SessionOpen::Request { step, handle }) = self.spec.map(|spec| &spec.open) else {
            return Ok(());
        };
        let mut state = self.open.lock().await;
        match &*state {
            OpenState::Open => return Ok(()),
            OpenState::Failed(error) if !retry => return Err(error.clone()),
            OpenState::Failed(_) | OpenState::Pending => {}
        }
        let keep_alive = self
            .spec
            .and_then(|spec| spec.keep_alive)
            .and_then(|keep_alive| keep_alive.on_open)
            .zip(self.keep_alive.as_deref());
        let parts = step_parts(step, base, R::PATH, None, keep_alive);
        let result = match send_step::<A, R>(client, options, parts).await {
            Ok((meta, document)) => match handle_text(*handle, &document.json, &document.headers) {
                Some(handle) => {
                    self.set_handle(handle);
                    Ok(())
                }
                None => Err(PagerError::MissingSessionHandle { meta: Box::new(meta) }),
            },
            Err(error) => Err(error),
        };
        *state = match &result {
            Ok(()) => OpenState::Open,
            Err(error) => OpenState::Failed(error.clone()),
        };
        result
    }

    /// Sends the renew request with the latest handle; `false` without one.
    pub(crate) async fn renew<A: Api, R: Paginated<Api = A, Failure = E>>(
        &self,
        client: &Client<A>,
        base: &Result<RequestParts, InvalidArgument>,
        options: &CallOptions,
    ) -> Result<bool, PagerError<R::Failure>> {
        let (Some(step), Some(handle)) = (self.spec.and_then(|spec| spec.renew.as_ref()), self.handle()) else {
            return Ok(false);
        };
        let parts = step_parts(step, base, R::PATH, Some(&handle), None);
        send_step::<A, R>(client, options, parts).await.map(|_| true)
    }

    /// Sends the close request with the latest handle, bounded by `grace`
    /// and the client's per-request timeout instead of the pages' deadline;
    /// nothing when no handle was observed or no close is authored.
    async fn close<A: Api, R: Paginated<Api = A, Failure = E>>(
        &self,
        client: &Client<A>,
        base: &Result<RequestParts, InvalidArgument>,
        options: &CallOptions,
        grace: Duration,
    ) -> Result<(), PagerError<R::Failure>> {
        let (Some(step), Some(handle)) = (self.spec.and_then(|spec| spec.close.as_ref()), self.handle()) else {
            return Ok(());
        };
        let options = CallOptions {
            auth: options.auth.clone(),
            deadline: Some(Deadline::after(grace)),
        };
        let parts = step_parts(step, base, R::PATH, Some(&handle), None);
        send_step::<A, R>(client, &options, parts).await.map(|_| ())
    }
}

/// A handle read from `source`: a non-empty string, or a number, as text.
fn handle_text(source: Source, body: &Value, headers: &HeaderMap) -> Option<String> {
    match json::read(source, body, headers) {
        Read::Json(Value::String(text)) if !text.is_empty() => Some(text.clone()),
        Read::Header(text) if !text.is_empty() => Some(text),
        Read::Json(value @ Value::Number(_)) => Some(value.to_string()),
        _ => None,
    }
}

/// The parts of a session step: its method and path (filled from the
/// endpoint's path values), the endpoint's headers, the handle, the step's
/// constants, and a keep-alive; no query or body of the endpoint.
fn step_parts(
    step: &SessionStep,
    base: &Result<RequestParts, InvalidArgument>,
    endpoint_template: &str,
    handle: Option<&str>,
    keep_alive: Option<(Input, &str)>,
) -> Result<RequestParts, InvalidArgument> {
    let base = base.as_ref().map_err(Clone::clone)?;
    let mut parts = base.clone();
    parts.method = step.method.clone();
    parts.path = fill_path(step.path, endpoint_template, &base.path)
        .ok_or_else(|| InvalidArgument::new("the session step path cannot be filled from the endpoint path"))?;
    parts.query = Vec::new();
    parts.body = RequestBody::Empty;
    if let (Some(input), Some(handle)) = (step.handle, handle) {
        write_input(&mut parts, input, &Value::from(handle))?;
    }
    for fixed in step.constants {
        write_input(&mut parts, fixed.param, &Value::from(fixed.value))?;
    }
    if let Some((input, value)) = keep_alive {
        write_input(&mut parts, input, &Value::from(value))?;
    }
    Ok(parts)
}

/// Sends one session step as an ordinary send of the listing endpoint.
async fn send_step<A: Api, R: Paginated<Api = A>>(
    client: &Client<A>,
    options: &CallOptions,
    parts: Result<RequestParts, InvalidArgument>,
) -> Result<(ResponseMeta, StepDocument), PagerError<R::Failure>> {
    let request = StepRequest::<R> {
        parts,
        endpoint: PhantomData,
    };
    let (response, _) = client.send_located(request, options).await.map_err(PagerError::Transport)?;
    let (meta, outcome) = response.into_parts();
    match outcome {
        Outcome::Success(document) => Ok((meta, document)),
        Outcome::Failure(error) => Err(PagerError::Api {
            error: error.into(),
            meta: Box::new(meta),
        }),
    }
}

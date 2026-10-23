//! The pager's public surface: [`Pager`] configures a traversal, [`Pages`]
//! walks it one page per request, and [`Items`] flattens it.

use std::collections::VecDeque;
use std::fmt;
use std::marker::PhantomData;
use std::time::Duration;

use super::engine::{Deferred, DeferredKind, Direction, Engine, PageView, Plan, Planned, Rejected};
use super::error::{Incomplete, Page, PagerError};
use super::request::{PageRequest, build_parts, decode_items, write_input};
use super::session::{DEFAULT_CLOSE_GRACE, SessionShared};
use super::{
    AdjustablePageSize, HasPrev, HasTotal, PageRootError, PageSizeKnob, Paginated, Partitioned, Renewable, ResumeAfter,
    ResumeBefore, Sessionless, StartOffset, StartPage, cursor_values,
};
use crate::call::{Call, CallFuture};
use crate::client::Client;
use crate::credentials::Credential;
use crate::deadline::Deadline;
use crate::endpoint::{Api, EndpointSpec, RequestParts};
use crate::error::{InvalidArgument, TransportError, TransportErrorKind};
use crate::meta::ResponseMeta;
use crate::options::{CallAuth, CallOptions};
use crate::response::{ApiError, ErrorKind, Outcome};

/// The page limit of [`Pager::collect_all`] when the caller sets none: a
/// guard against a runaway traversal, not a recommendation.
pub const DEFAULT_PAGE_LIMIT: u32 = 100;

/// What [`Pager::fetch_page`] resolves to.
pub type PageResult<R> = Result<Page<<R as Paginated>::Item>, PagerError<<R as EndpointSpec>::Failure>>;

/// What [`Pager::collect_all`] resolves to.
pub type CollectResult<R> =
    Result<Vec<<R as Paginated>::Item>, Incomplete<<R as Paginated>::Item, <R as EndpointSpec>::Failure>>;

impl<'a, A: Api, R: EndpointSpec<Api = A>> Call<'a, A, R> {
    /// Turns this call into a pager for its list endpoint, keeping the
    /// call's request and per-call settings.
    pub fn pager(self) -> Pager<'a, A, R>
    where
        R: Paginated,
    {
        let (client, request, options) = self.into_inner();
        Pager {
            client,
            request,
            options,
            engine: Engine::new(R::STRATEGY, R::ITEMS),
            page_limit: DEFAULT_PAGE_LIMIT,
            keep_alive: None,
            close_grace: DEFAULT_CLOSE_GRACE,
        }
    }
}

/// A configured traversal of a list endpoint, before its first request.
///
/// Each control exists only where the endpoint's definition authors it (see
/// the [module documentation](super)). Sizes and start positions are checked
/// before the request that would carry them is sent: a value outside the
/// declared bounds is a [`TransportError`] of kind
/// [`TransportErrorKind::InvalidArgument`] carrying the request metadata,
/// and nothing is sent.
#[must_use = "a pager sends nothing until it is iterated or fetched"]
pub struct Pager<'a, A: Api, R> {
    pub(super) client: &'a Client<A>,
    pub(super) request: R,
    pub(super) options: CallOptions,
    pub(super) engine: Engine,
    pub(super) page_limit: u32,
    /// The caller's session keep-alive; `None` sends the authored default.
    pub(super) keep_alive: Option<String>,
    pub(super) close_grace: Duration,
}

impl<'a, A: Api, R: Paginated<Api = A>> Pager<'a, A, R> {
    /// Requests pages of `size` items. An optional size is otherwise
    /// omitted, and a size the provider requires is sent at its authored
    /// default.
    pub fn page_size(mut self, size: u32) -> Self
    where
        R: PageSizeKnob,
    {
        self.engine.set_page_size(size);
        self
    }

    /// Starts at page `number`, in the strategy's own index base.
    pub fn page(mut self, number: u64) -> Self
    where
        R: StartPage,
    {
        self.engine.start_at_page(number);
        self
    }

    /// Starts at item `offset`, in the strategy's own index base.
    pub fn offset(mut self, offset: u64) -> Self
    where
        R: StartOffset,
    {
        self.engine.start_at_offset(offset);
        self
    }

    /// Starts forward from `cursor`, a cursor kept from an earlier
    /// traversal. Replaces [`before`](Self::before).
    pub fn after(mut self, cursor: impl Into<<R as ResumeAfter>::Cursor>) -> Self
    where
        R: ResumeAfter,
    {
        self.engine.start_after(cursor_values(cursor.into()));
        self
    }

    /// Starts backward from `cursor`: the first page is the one before it.
    /// Replaces [`after`](Self::after).
    pub fn before(mut self, cursor: impl Into<<R as ResumeBefore>::Cursor>) -> Self
    where
        R: ResumeBefore,
    {
        self.engine.start_before(cursor_values(cursor.into()));
        self
    }

    /// Sets how many page requests [`collect_all`](Self::collect_all) may
    /// make; it must be greater than zero. The default is
    /// [`DEFAULT_PAGE_LIMIT`].
    pub fn page_limit(mut self, limit: u32) -> Self {
        self.page_limit = limit;
        self
    }

    /// The page limit [`collect_all`](Self::collect_all) applies.
    pub fn configured_page_limit(&self) -> u32 {
        self.page_limit
    }

    /// See [`Call::anonymous`].
    pub fn anonymous(mut self) -> Self {
        self.options.auth = CallAuth::Anonymous;
        self
    }

    /// See [`Call::credential`].
    pub fn credential(mut self, credential: Credential) -> Self {
        self.options = self.options.credential(credential);
        self
    }

    /// Bounds every page request by `deadline`; see [`Call::deadline`].
    pub fn deadline(mut self, deadline: &Deadline) -> Self {
        self.options = self.options.deadline(deadline);
        self
    }

    /// Starts iterating; nothing is sent until [`Pages::next`].
    pub fn pages(self) -> Pages<'a, A, R>
    where
        R: Sessionless,
    {
        Pages::new(self.client, self.request.into_parts(), self.options, self.engine, None, None)
    }

    /// `count` independent traversals of the list's slices: traversal `i`
    /// sends slice index `i` and the slice count `count` on every request.
    /// The caller polls them, in any order or concurrently; nothing is
    /// spawned.
    ///
    /// ## Errors
    ///
    /// Returns [`InvalidArgument`] when `count` is zero or above the
    /// definition's maximum; nothing is sent.
    pub fn partitioned(self, count: u32) -> Result<Vec<Pages<'a, A, R>>, InvalidArgument>
    where
        R: Partitioned + Sessionless,
    {
        partition_count::<R>(count)?;
        let base = self.request.into_parts();
        Ok((0..count)
            .map(|index| {
                Pages::new(
                    self.client,
                    base.clone(),
                    self.options.clone(),
                    self.engine.clone(),
                    None,
                    Some((index, count)),
                )
            })
            .collect())
    }

    /// Sends exactly one request with this pager's settings and returns its
    /// page, following no continuation.
    ///
    /// It is the way to fetch one customized page; the page is a
    /// [`Page`], not the endpoint's own response type.
    ///
    /// ## Errors
    ///
    /// Returns the [`PagerError`] the first page of [`pages`](Self::pages)
    /// would.
    pub fn fetch_page(self) -> CallFuture<'a, PageResult<R>>
    where
        R: Sessionless,
    {
        fetch_first(self.pages())
    }

    /// Fetches every page and returns all items, only after the provider
    /// signaled the end of the list.
    ///
    /// ## Errors
    ///
    /// Returns [`Incomplete`] with the items gathered so far when any page
    /// fails, a continuation is missing, unusable, or repeated, or the page
    /// limit is reached first. The limit counts page requests, empty pages
    /// included; the last permitted page completes the list when it shows
    /// the end.
    pub fn collect_all(self) -> CallFuture<'a, CollectResult<R>>
    where
        R: Sessionless,
    {
        let limit = self.page_limit;
        collect(self.pages(), limit)
    }
}

/// Sends the first request of `pages` and returns its page.
pub(super) fn fetch_first<'s, A: Api, R: Paginated<Api = A>>(mut pages: Pages<'s, A, R>) -> CallFuture<'s, PageResult<R>> {
    Box::pin(async move {
        let page = pages.next().await?;
        // The first request of a traversal is always sent.
        Ok(page.expect("the first page of a traversal is always requested"))
    })
}

/// Walks `pages` to the provider's end, within `limit` page requests.
pub(super) fn collect<'s, A: Api, R: Paginated<Api = A>>(
    mut pages: Pages<'s, A, R>,
    limit: u32,
) -> CallFuture<'s, CollectResult<R>> {
    if limit == 0 {
        pages.engine_refuse("the page limit must be greater than zero");
    }
    Box::pin(async move {
        let mut items = Vec::new();
        loop {
            let error = match pages.next().await {
                Ok(Some(page)) => {
                    items.extend(page.items);
                    if pages.is_finished() {
                        return Ok(items);
                    }
                    if pages.pages_fetched() < limit || pages.has_deferred_failure() {
                        continue;
                    }
                    return Err(Incomplete {
                        reason: super::IncompleteReason::PageLimitReached,
                        items,
                        pages: pages.pages_fetched(),
                        error: None,
                    });
                }
                Ok(None) => return Ok(items),
                Err(error) => error,
            };
            return Err(Incomplete {
                reason: error.incomplete_reason(),
                items,
                pages: pages.pages_fetched(),
                error: Some(error),
            });
        }
    })
}

/// Checks a partition count against the definition: at least one, at most
/// the authored maximum.
pub(super) fn partition_count<R: Paginated>(count: u32) -> Result<(), InvalidArgument> {
    if count == 0 {
        return Err(InvalidArgument::new("the partition count must be at least 1"));
    }
    if let Some(max) = R::STRATEGY.partition.and_then(|partition| partition.max)
        && count > max
    {
        return Err(InvalidArgument::new(format!("the partition count is above the declared maximum, {max}")));
    }
    Ok(())
}

impl<A: Api, R: EndpointSpec + fmt::Debug> fmt::Debug for Pager<'_, A, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pager")
            .field("api", &A::NAME)
            .field("endpoint", &R::ENDPOINT_ID)
            .field("request", &self.request)
            .field("page_limit", &self.page_limit)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
struct Retry {
    engine: Engine,
    plan: Plan,
    delivered: bool,
}

/// A traversal in progress: one page per [`next`](Self::next).
///
/// Every page request is an ordinary send through the client, with the
/// call's credential, deadline, and the API's origin policy, and its request
/// metadata carries the page's number. Once a request fails, every later
/// call returns the same complete error without sending, until an explicit
/// [`retry`](Self::retry).
pub struct Pages<'a, A: Api, R: Paginated> {
    client: &'a Client<A>,
    base: Result<RequestParts, InvalidArgument>,
    options: CallOptions,
    engine: Engine,
    /// The scope's session, for a traversal inside `with_session`.
    session: Option<&'a SessionShared<R::Failure>>,
    /// This traversal's slice index and the slice count.
    partition: Option<(u32, u32)>,
    failed: Option<PagerError<R::Failure>>,
    retry: Option<Retry>,
    last_delivered: Option<Retry>,
    buffered: Option<Page<R::Item>>,
    attempted: u32,
    received: u32,
    endpoint: PhantomData<fn() -> R>,
}

impl<'a, A: Api, R: Paginated<Api = A>> Pages<'a, A, R> {
    pub(super) fn new(
        client: &'a Client<A>,
        base: Result<RequestParts, InvalidArgument>,
        options: CallOptions,
        engine: Engine,
        session: Option<&'a SessionShared<R::Failure>>,
        partition: Option<(u32, u32)>,
    ) -> Self {
        Self {
            client,
            base,
            options,
            engine,
            session,
            partition,
            failed: None,
            retry: None,
            last_delivered: None,
            buffered: None,
            attempted: 0,
            received: 0,
            endpoint: PhantomData,
        }
    }

    /// The next page forward, or `None` after the provider signaled the end.
    ///
    /// ## Errors
    ///
    /// Returns a [`PagerError`]; see its variants.
    pub async fn next(&mut self) -> Result<Option<Page<R::Item>>, PagerError<R::Failure>> {
        self.fetch(Direction::Forward).await
    }

    /// The page before the earliest page seen, or `None` when no earlier
    /// page is known.
    ///
    /// ## Errors
    ///
    /// Returns a [`PagerError`]; see its variants.
    pub async fn prev(&mut self) -> Result<Option<Page<R::Item>>, PagerError<R::Failure>>
    where
        R: HasPrev,
    {
        self.fetch(Direction::Backward).await
    }

    /// The latest total a page reported, in the unit the definition
    /// declares; `None` before one arrives.
    pub fn total(&self) -> Option<u64>
    where
        R: HasTotal,
    {
        self.engine.total()
    }

    /// Changes the page size from the next request on; it is checked
    /// against the declared bounds before that request is sent.
    pub fn set_page_size(&mut self, size: u32)
    where
        R: AdjustablePageSize,
    {
        self.engine.adjust_page_size(size);
        // A continuation failure may select the last delivered checkpoint
        // after this setter, so both recovery snapshots need the new setting.
        for retry in self.retry.iter_mut().chain(self.last_delivered.iter_mut()) {
            retry.engine.adjust_retry_size(&mut retry.plan, size);
        }
    }

    /// Page requests made so far, including any that failed.
    pub fn pages_fetched(&self) -> u32 {
        self.attempted
    }

    /// Items delivered so far, excluding a recovered page awaiting iteration.
    pub fn items_delivered(&self) -> u64 {
        self.engine.items() - self.buffered.as_ref().map_or(0, |page| page.items.len() as u64)
    }

    /// Whether the provider has signaled the end of the list.
    pub fn is_finished(&self) -> bool {
        self.engine.is_finished()
    }

    /// The items of every remaining page, one at a time, reading each page
    /// once.
    pub fn items(self) -> Items<'a, A, R> {
        Items {
            pages: self,
            buffer: VecDeque::new(),
        }
    }

    /// See [`SessionScope::renew`](super::SessionScope::renew); available
    /// while this traversal of the session is borrowed.
    ///
    /// ## Errors
    ///
    /// Returns [`PagerError::Transport`] or [`PagerError::Api`] when the
    /// renew request fails.
    pub async fn renew(&self) -> Result<bool, PagerError<R::Failure>>
    where
        R: Renewable,
    {
        match self.session {
            Some(session) => session.renew::<A, R>(self.client, &self.base, &self.options).await,
            None => Ok(false),
        }
    }

    fn engine_refuse(&mut self, reason: &'static str) {
        self.base = Err(InvalidArgument::new(reason));
    }

    fn has_deferred_failure(&self) -> bool {
        self.engine.has_deferred_failure()
    }

    /// Explicitly retries the failed position, for every error category.
    ///
    /// A recovered page is returned by the next iteration call. If items of
    /// the source page were already delivered before its continuation failed,
    /// this refreshes only its continuation and never delivers those items again.
    /// Calling this without a failure sends nothing. A failed attempt replaces
    /// the retained error. An expired deadline remains expired.
    pub async fn retry(&mut self) -> Result<(), PagerError<R::Failure>> {
        if self.failed.is_none() {
            return Ok(());
        }
        let retry = self.retry.clone().expect("every failure has a retry position");
        let retained = self.engine.clone();
        let mut working = retry.engine.clone();
        let result = self.send_plan(&mut working, retry.plan.clone(), true).await;
        match result {
            Ok(Some(_)) if retry.delivered => {
                self.received = self.received.saturating_sub(1);
                let mut probe = working.clone();
                if let Planned::Failed(deferred) = probe.plan(retry.plan.direction) {
                    let error = deferred_error(deferred);
                    self.failed = Some(error.clone());
                    return Err(error);
                }
                // Refreshing continuation evidence must not rewrite delivery
                // counters or the identities of items already handed to callers.
                working.restore_delivery_from(&retained);
                self.engine = working;
                self.failed = None;
                self.retry = None;
                Ok(())
            }
            Ok(page) => {
                self.engine = working;
                self.last_delivered = Some(Retry { delivered: true, ..retry });
                self.buffered = page;
                self.failed = None;
                self.retry = None;
                Ok(())
            }
            Err(error) => {
                self.failed = Some(error.clone());
                Err(error)
            }
        }
    }

    async fn fetch(&mut self, direction: Direction) -> Result<Option<Page<R::Item>>, PagerError<R::Failure>> {
        if let Some(error) = &self.failed {
            return Err(error.clone());
        }
        if let Some(page) = self.buffered.take() {
            return Ok(Some(page));
        }
        let plan = match self.engine.plan(direction) {
            Planned::Done => return Ok(None),
            Planned::Failed(deferred) => {
                self.retry = self.last_delivered.clone();
                let error = deferred_error(deferred);
                self.failed = Some(error.clone());
                return Err(error);
            }
            Planned::Send(plan) => plan,
        };
        let checkpoint = Retry { engine: self.engine.clone(), plan: plan.clone(), delivered: false };
        let mut working = self.engine.clone();
        let result = self.send_plan(&mut working, plan, false).await;
        match &result {
            Err(error) => {
                self.engine = checkpoint.engine.clone();
                self.retry = Some(checkpoint);
                self.failed = Some(error.clone());
            }
            Ok(Some(_)) => {
                self.engine = working;
                self.last_delivered = Some(Retry { delivered: true, ..checkpoint });
            }
            Ok(None) => {}
        }
        result
    }

    async fn send_plan(&mut self, engine: &mut Engine, plan: Plan, retry: bool) -> Result<Option<Page<R::Item>>, PagerError<R::Failure>> {
        self.attempted = self.attempted.saturating_add(1);
        let number = engine.pages();
        if let Some(session) = self.session
            && let Err(error) = session.ensure_open::<A, R>(self.client, &self.base, &self.options, retry).await
        {
            return Err(match error {
                PagerError::Transport(error) => self.transport_error(error),
                error => error,
            });
        }
        let parts = match &self.base {
            Ok(base) => build_parts(base, &plan, R::STRATEGY, R::PATH).and_then(|mut parts| {
                if let Some(session) = self.session {
                    let opening = session.opens_with(plan.follow_up);
                    session.apply_to_page(&mut parts, opening, plan.link.is_some(), R::PATH, &base.path)?;
                }
                if let (Some((index, count)), Some(partition)) = (self.partition, R::STRATEGY.partition) {
                    write_input(&mut parts, partition.index, &index.into())?;
                    write_input(&mut parts, partition.count, &count.into())?;
                }
                Ok(parts)
            }),
            Err(argument) => Err(argument.clone()),
        };
        // A followed link is checked as it will be sent, after the carry rule
        // and managed inputs changed its query.
        if plan.link.is_some()
            && let Ok(parts) = &parts
            && let Ok(url) = url::Url::parse(&parts.path)
            && let Err(mut error) = self.client.check_link::<R>(&url)
        {
            error.request.page = Some(number);
            return Err(PagerError::Transport(error));
        }
        let request = PageRequest::<R> {
            parts,
            endpoint: PhantomData,
        };
        let (mut response, url) = match self.client.send_located(request, &self.options).await {
            Ok(sent) => sent,
            Err(mut error) => {
                error.request.page = Some(number);
                return Err(self.transport_error(error));
            }
        };
        response.meta.request.page = Some(number);
        let (meta, outcome) = response.into_parts();
        let mut document = match outcome {
            Outcome::Success(document) => document,
            Outcome::Failure(error) => {
                return Err(PagerError::Api {
                    error: error.into(),
                    meta: Box::new(meta),
                });
            }
        };
        document.json = match R::page_root(std::mem::take(&mut document.json)) {
            Ok(root) => root,
            Err(PageRootError::Malformed) => return Err(malformed(document.bytes, meta)),
            Err(PageRootError::Declared(failure)) => {
                return Err(PagerError::Api {
                    error: ApiError {
                        kind: ErrorKind::Declared(failure),
                        body: document.bytes,
                    }
                    .into(),
                    meta: Box::new(meta),
                });
            }
        };
        if let Some(session) = self.session {
            session.observe_page(&document.json, &document.headers);
        }
        let view = PageView {
            body: &document.json,
            headers: &document.headers,
            url: url.as_ref(),
            meta: &meta,
        };
        let accepted = match engine.accept(&view) {
            Ok(accepted) => accepted,
            Err(Rejected::Malformed) => return Err(malformed(document.bytes, meta)),
            Err(Rejected::BoundaryNoProgress) => return Err(PagerError::BoundaryNoProgress { meta: Box::new(meta) }),
            Err(Rejected::EmptyPageClaimsMore) => return Err(PagerError::EmptyPageClaimsMore { meta: Box::new(meta) }),
        };
        let Some(items) = decode_items::<R::Item>(accepted.items) else {
            return Err(malformed(document.bytes, meta));
        };
        self.received = self.received.saturating_add(1);
        Ok(Some(Page {
            items,
            total: accepted.total,
            number,
            meta,
        }))
    }

    fn transport_error(&self, error: TransportError) -> PagerError<R::Failure> {
        match error.kind {
            TransportErrorKind::DeadlineExceeded { .. } => PagerError::DeadlineExceeded {
                completed_pages: self.received,
                error,
            },
            _ => PagerError::Transport(error),
        }
    }
}

impl<A: Api, R: Paginated> fmt::Debug for Pages<'_, A, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pages")
            .field("api", &A::NAME)
            .field("endpoint", &R::ENDPOINT_ID)
            .field("pages", &self.engine.pages())
            .field("failed", &self.failed.as_ref().map(PagerError::kind))
            .finish_non_exhaustive()
    }
}

/// A 2xx page whose collection or items do not match the definition: the
/// envelope's malformed-success failure, with the raw body.
fn malformed<E>(body: bytes::Bytes, meta: ResponseMeta) -> PagerError<E> {
    PagerError::Api {
        error: ApiError {
            kind: ErrorKind::MalformedSuccess,
            body,
        }.into(),
        meta: Box::new(meta),
    }
}

fn deferred_error<E>(deferred: Deferred) -> PagerError<E> {
    let meta = deferred.meta;
    match deferred.kind {
        DeferredKind::Missing => PagerError::MissingContinuation { meta },
        DeferredKind::Invalid(problem) => PagerError::InvalidContinuation { problem, meta },
        DeferredKind::Repeated => PagerError::RepeatedContinuation { meta },
    }
}

/// The items of a traversal, one at a time; see [`Pages::items`].
pub struct Items<'a, A: Api, R: Paginated> {
    pages: Pages<'a, A, R>,
    buffer: VecDeque<R::Item>,
}

impl<A: Api, R: Paginated<Api = A>> Items<'_, A, R> {
    /// The next item, or `None` after the last page's last item.
    ///
    /// ## Errors
    ///
    /// Returns the [`PagerError`] of the page request that failed, after
    /// every item of the earlier pages.
    pub async fn next(&mut self) -> Result<Option<R::Item>, PagerError<R::Failure>> {
        loop {
            if let Some(item) = self.buffer.pop_front() {
                return Ok(Some(item));
            }
            match self.pages.next().await? {
                Some(page) => self.buffer.extend(page.items),
                None => return Ok(None),
            }
        }
    }

    /// See [`Pages::retry`]. A recovered page is consumed by [`next`](Self::next).
    pub async fn retry(&mut self) -> Result<(), PagerError<R::Failure>> {
        self.pages.retry().await
    }

    /// Page requests made so far.
    pub fn pages_fetched(&self) -> u32 {
        self.pages.pages_fetched()
    }

    /// Items returned by [`next`](Self::next) so far.
    pub fn items_delivered(&self) -> u64 {
        self.pages.items_delivered() - self.buffer.len() as u64
    }
}

impl<A: Api, R: Paginated> fmt::Debug for Items<'_, A, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Items")
            .field("pages", &self.pages)
            .field("buffered", &self.buffer.len())
            .finish()
    }
}

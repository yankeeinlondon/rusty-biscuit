//! The pager's public surface: [`Pager`] configures a traversal, [`Pages`]
//! walks it one page per request, and [`Items`] flattens it.

use std::collections::VecDeque;
use std::fmt;
use std::marker::PhantomData;

use super::engine::{Deferred, DeferredKind, Direction, Engine, PageView, Planned, Rejected};
use super::error::{Incomplete, Page, PagerError, PagerErrorKind};
use super::request::{PageRequest, build_parts, decode_items};
use super::{
    AdjustablePageSize, HasPrev, HasTotal, PageSizeKnob, Paginated, ResumeAfter, ResumeBefore, Sessionless,
    StartOffset, StartPage, cursor_values,
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
    client: &'a Client<A>,
    request: R,
    options: CallOptions,
    engine: Engine,
    page_limit: u32,
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
        Pages {
            client: self.client,
            base: self.request.into_parts(),
            options: self.options,
            engine: self.engine,
            failed: None,
            received: 0,
            endpoint: PhantomData,
        }
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
        let mut pages = self.pages();
        Box::pin(async move {
            let page = pages.next().await?;
            // The first request of a traversal is always sent.
            Ok(page.expect("the first page of a traversal is always requested"))
        })
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
        let mut pages = self.pages();
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

/// A traversal in progress: one page per [`next`](Self::next).
///
/// Every page request is an ordinary send through the client, with the
/// call's credential, deadline, and the API's origin policy, and its request
/// metadata carries the page's number. Once a request fails, every later
/// call returns [`PagerError::AlreadyFailed`] without sending.
pub struct Pages<'a, A: Api, R: EndpointSpec> {
    client: &'a Client<A>,
    base: Result<RequestParts, InvalidArgument>,
    options: CallOptions,
    engine: Engine,
    failed: Option<PagerErrorKind>,
    received: u32,
    endpoint: PhantomData<fn() -> R>,
}

impl<'a, A: Api, R: Paginated<Api = A>> Pages<'a, A, R> {
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
    }

    /// Page requests made so far, including any that failed.
    pub fn pages_fetched(&self) -> u32 {
        self.engine.pages()
    }

    /// Items delivered so far.
    pub fn items_delivered(&self) -> u64 {
        self.engine.items()
    }

    /// Whether the provider has signaled the end of the list.
    pub fn is_finished(&self) -> bool {
        self.engine.is_finished()
    }

    /// The items of every remaining page, one at a time, reading each page
    /// once.
    pub fn items(self) -> Items<'a, A, R>
    where
        R: Sessionless,
    {
        Items {
            pages: self,
            buffer: VecDeque::new(),
        }
    }

    fn engine_refuse(&mut self, reason: &'static str) {
        self.base = Err(InvalidArgument::new(reason));
    }

    fn has_deferred_failure(&self) -> bool {
        self.engine.has_deferred_failure()
    }

    async fn fetch(&mut self, direction: Direction) -> Result<Option<Page<R::Item>>, PagerError<R::Failure>> {
        if let Some(first) = self.failed {
            return Err(PagerError::AlreadyFailed { first });
        }
        let result = self.fetch_once(direction).await;
        if let Err(error) = &result {
            self.failed = Some(error.kind());
        }
        result
    }

    async fn fetch_once(&mut self, direction: Direction) -> Result<Option<Page<R::Item>>, PagerError<R::Failure>> {
        let plan = match self.engine.plan(direction) {
            Planned::Done => return Ok(None),
            Planned::Failed(deferred) => return Err(deferred_error(deferred)),
            Planned::Send(plan) => plan,
        };
        let number = self.engine.pages();
        if let Some(link) = &plan.link
            && let Err(mut error) = self.client.check_link::<R>(link)
        {
            error.request.page = Some(number);
            return Err(PagerError::Transport(error));
        }
        let parts = match &self.base {
            Ok(base) => build_parts(base, &plan, R::STRATEGY, R::PATH),
            Err(argument) => Err(argument.clone()),
        };
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
        let document = match outcome {
            Outcome::Success(document) => document,
            Outcome::Failure(error) => {
                return Err(PagerError::Api {
                    error,
                    meta: Box::new(meta),
                });
            }
        };
        let view = PageView {
            body: &document.json,
            headers: &document.headers,
            url: url.as_ref(),
            meta: &meta,
        };
        let accepted = match self.engine.accept(&view) {
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

impl<A: Api, R: EndpointSpec> fmt::Debug for Pages<'_, A, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pages")
            .field("api", &A::NAME)
            .field("endpoint", &R::ENDPOINT_ID)
            .field("pages", &self.engine.pages())
            .field("failed", &self.failed)
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
        },
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

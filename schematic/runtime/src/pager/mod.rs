//! Pagers for list endpoints.
//!
//! A generated request whose definition authors a pagination strategy
//! implements [`Paginated`], whose constants describe the strategy and where
//! the items are. A [`Call`](crate::Call) of such a request becomes a
//! [`Pager`] (configuration) through [`Call::pager`](crate::Call::pager),
//! and a pager becomes [`Pages`] (iteration), one page per request:
//!
//! ```text
//! let releases = gh.list_releases("owner", "repo").page_size(50).collect_all().await?;
//!
//! let mut pages = gh.list_releases("owner", "repo").pages();
//! while let Some(page) = pages.next().await? {
//!     for release in page.items { /* ... */ }
//! }
//!
//! let one = openai.list_files().after("file-abc").page_size(20).fetch_page().await?;
//! ```
//!
//! ## Only authored controls compile
//!
//! Every control exists only where the definition authors it. Each is a
//! method bounded by its own capability trait, so calling one an endpoint
//! does not author is a compile error naming the missing capability:
//!
//! | Control | Capability |
//! | --- | --- |
//! | `.page_size(n)` | [`PageSizeKnob`] |
//! | `.page(n)` | [`StartPage`] (page-number strategies) |
//! | `.offset(n)` | [`StartOffset`] (offset strategies) |
//! | `.after(cursor)` | [`ResumeAfter`] |
//! | `.before(cursor)` | [`ResumeBefore`] |
//! | `Pages::prev()` | [`HasPrev`] |
//! | `Pages::total()` | [`HasTotal`] |
//! | `Pages::set_page_size(n)` | [`AdjustablePageSize`] |
//! | `.partitioned(n)` | [`Partitioned`] |
//! | `.pages()`, `.fetch_page()`, `.collect_all()` | [`Sessionless`] |
//! | `.with_session(..)`, `.close_grace(..)` | [`Sessioned`] |
//! | `.keep_alive(..)` | [`KeepAliveKnob`] |
//! | `SessionScope::renew()`, `Pages::renew()` | [`Renewable`] |
//!
//! Configuration happens on the [`Pager`], before iteration starts; [`Pages`]
//! has no configuration method, so a control called after `.pages()` does
//! not compile either.
//!
//! ## Sessions and partitions
//!
//! A strategy with a server-side session (a scroll or point in time) is
//! iterated only inside [`Pager::with_session`]: the closure gets a
//! [`SessionScope`] to page, collect, partition, and renew, and one close
//! request is sent when it ends, bounded by its own grace. See the
//! `with_session` documentation for what cancellation and panics leave to
//! the server's lease.
//!
//! [`Pager::partitioned`] and [`SessionScope::partitioned`] give `n`
//! independent traversals, each sending its slice index and the slice count
//! on every request; the caller polls them, and nothing is spawned.
//!
//! ## Completeness
//!
//! [`Pager::collect_all`] returns `Ok` only after the provider signaled the
//! end of the list. Otherwise it returns [`Incomplete`] with the reason, the
//! items gathered, and the error, never a partial list presented as complete.
//! A pager that returned an error stays failed. See [`PagerError`] for what
//! each failure keeps.

mod engine;
mod error;
mod json;
mod link;
mod pages;
mod progress;
mod request;
mod session;
pub mod spec;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::endpoint::EndpointSpec;

pub use error::{ContinuationProblem, Incomplete, IncompleteReason, Page, PagerError, PagerErrorKind, SessionError};
pub use pages::{CollectResult, DEFAULT_PAGE_LIMIT, Items, PageResult, Pager, Pages};
pub use session::{DEFAULT_CLOSE_GRACE, SessionResult, SessionScope};
pub use spec::{ItemSelector, Strategy};

/// A request of a paginated list endpoint.
///
/// The generator implements it, with the capability traits of the controls
/// the definition authors, for exactly the endpoints whose resolved
/// pagination is a strategy. Its constants must agree with its associated
/// types: [`Kind`](Self::Kind) names the kind of
/// [`STRATEGY`](Self::STRATEGY), and [`Session`](Self::Session) whether it
/// has a session.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not paginated",
    label = "no pagination strategy is authored for this endpoint",
    note = "the definition authors no pagination strategy for `{Self}`, so the generated client has no pager for it"
)]
pub trait Paginated: EndpointSpec {
    /// The type of one item of a page.
    type Item: DeserializeOwned + Send + 'static;
    /// The strategy's kind: [`PageNumberKind`], [`OffsetKind`],
    /// [`CursorKind`], or [`NextLinkKind`].
    type Kind: StrategyKind;
    /// [`NoSession`] or [`WithSession`].
    type Session: SessionMode;

    /// The strategy.
    const STRATEGY: &'static Strategy;
    /// Where a page's items are.
    const ITEMS: &'static ItemSelector;
    /// The endpoint's path template, whose `{placeholders}` a follow-up path
    /// may use.
    const PATH: &'static str;
}

mod sealed {
    pub trait Sealed {}
}

/// A strategy kind, named at the type level so kind-only controls can be
/// bounded on it.
pub trait StrategyKind: sealed::Sealed + 'static {}

/// Page-number strategies.
pub enum PageNumberKind {}
/// Offset strategies.
pub enum OffsetKind {}
/// Cursor strategies.
pub enum CursorKind {}
/// Next-link strategies.
pub enum NextLinkKind {}

impl sealed::Sealed for PageNumberKind {}
impl sealed::Sealed for OffsetKind {}
impl sealed::Sealed for CursorKind {}
impl sealed::Sealed for NextLinkKind {}
impl StrategyKind for PageNumberKind {}
impl StrategyKind for OffsetKind {}
impl StrategyKind for CursorKind {}
impl StrategyKind for NextLinkKind {}

/// Whether a strategy has a server-side session.
pub trait SessionMode: sealed::Sealed + 'static {}

/// A strategy without a session: iterated with `pages()`.
pub enum NoSession {}
/// A strategy with a session: iterated only inside its scoped helper.
pub enum WithSession {}

impl sealed::Sealed for NoSession {}
impl sealed::Sealed for WithSession {}
impl SessionMode for NoSession {}
impl SessionMode for WithSession {}

/// `.page(n)`: start at a page number, in the strategy's own index base.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not paginated by page number",
    label = "`.page(n)` exists only for page-number strategies",
    note = "the definition authors a different pagination strategy for `{Self}`"
)]
pub trait StartPage: Paginated {}
#[diagnostic::do_not_recommend]
impl<E: Paginated<Kind = PageNumberKind>> StartPage for E {}

/// `.offset(n)`: start at an item offset, in the strategy's own index base.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not paginated by offset",
    label = "`.offset(n)` exists only for offset strategies",
    note = "the definition authors a different pagination strategy for `{Self}`"
)]
pub trait StartOffset: Paginated {}
#[diagnostic::do_not_recommend]
impl<E: Paginated<Kind = OffsetKind>> StartOffset for E {}

/// `.pages()`, `.fetch_page()`, and `.collect_all()`: iteration outside a
/// session scope.
#[diagnostic::on_unimplemented(
    message = "`{Self}` pages only inside its session scope",
    label = "a session endpoint is iterated only through `with_session`",
    note = "the definition authors a server-side session for `{Self}`; its pages exist only inside `with_session`, which closes the session"
)]
pub trait Sessionless: Paginated {}
#[diagnostic::do_not_recommend]
impl<E: Paginated<Session = NoSession>> Sessionless for E {}

/// `with_session(..)`: iteration inside a session scope.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no server-side session",
    label = "this exists only for strategies with a session",
    note = "the definition authors no `Session` for `{Self}`"
)]
pub trait Sessioned: Paginated {}
#[diagnostic::do_not_recommend]
impl<E: Paginated<Session = WithSession>> Sessioned for E {}

/// `.page_size(n)`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no authored page size",
    label = "`.page_size(..)` needs a `PageSize` in the definition",
    note = "the definition for `{Self}` declares no page size, so its pager has no `.page_size(..)`"
)]
pub trait PageSizeKnob: Paginated {}

/// `.after(cursor)`: start forward from a cursor the caller kept.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no resumable cursor",
    label = "`.after(..)` needs a resumable cursor in the definition",
    note = "the definition for `{Self}` does not mark a cursor strategy `resumable`"
)]
pub trait ResumeAfter: Paginated<Kind = CursorKind> {
    /// The forward cursor: a `String`, or a tuple of them for a composite
    /// cursor.
    type Cursor: CursorValue;
}

/// `.before(cursor)`: start backward from a cursor the caller kept.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no resumable backward cursor",
    label = "`.before(..)` needs a resumable cursor with a backward leg in the definition",
    note = "the definition for `{Self}` authors no resumable backward cursor"
)]
pub trait ResumeBefore: Paginated<Kind = CursorKind> {
    /// The backward cursor: a `String`, or a tuple of them.
    type Cursor: CursorValue;
}

/// `Pages::prev()`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no backward direction",
    label = "`prev()` needs a backward cursor leg or a previous link in the definition",
    note = "the definition for `{Self}` authors no backward direction"
)]
pub trait HasPrev: Paginated {}

/// `Pages::total()`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no authored total",
    label = "`total()` needs a total in the definition",
    note = "the definition for `{Self}` authors no total"
)]
pub trait HasTotal: Paginated {}

/// `Pages::set_page_size(n)`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no adjustable page size",
    label = "`set_page_size(..)` needs an adjustable `PageSize` in the definition",
    note = "the definition for `{Self}` does not mark its page size adjustable"
)]
pub trait AdjustablePageSize: PageSizeKnob {}

/// `.keep_alive(value)` on a session: the keep-alive sent instead of the
/// authored default.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no adjustable keep-alive",
    label = "`.keep_alive(..)` needs an adjustable `KeepAlive` in the definition",
    note = "the definition for `{Self}` authors no adjustable session keep-alive"
)]
pub trait KeepAliveKnob: Sessioned {}

/// `.partitioned(n)`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no partitions",
    label = "`.partitioned(..)` needs a `Partition` in the definition",
    note = "the definition for `{Self}` authors no partitioned iteration"
)]
pub trait Partitioned: Paginated {}

/// `renew()` inside a session scope.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no session renew step",
    label = "`renew()` needs a renew step on the session in the definition",
    note = "the definition for `{Self}` authors no session renew step"
)]
pub trait Renewable: Sessioned {}

/// A cursor value a caller passes to `.after(..)` or `.before(..)`: one
/// `String` per cursor part.
pub trait CursorValue: Send + 'static {
    /// The parts, in the definition's order.
    fn into_parts(self) -> Vec<String>;
}

impl CursorValue for String {
    fn into_parts(self) -> Vec<String> {
        vec![self]
    }
}

impl CursorValue for (String, String) {
    fn into_parts(self) -> Vec<String> {
        vec![self.0, self.1]
    }
}

impl CursorValue for (String, String, String) {
    fn into_parts(self) -> Vec<String> {
        vec![self.0, self.1, self.2]
    }
}

impl CursorValue for (String, String, String, String) {
    fn into_parts(self) -> Vec<String> {
        vec![self.0, self.1, self.2, self.3]
    }
}

fn cursor_values(cursor: impl CursorValue) -> Vec<Value> {
    cursor.into_parts().into_iter().map(Value::String).collect()
}

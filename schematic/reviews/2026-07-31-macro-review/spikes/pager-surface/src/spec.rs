//! Generator-emitted static data and the capability traits (encoding A).
//!
//! These are a const-friendly projection of the pagination spike's model
//! (`spikes/pagination/prototype`): `&'static` slices instead of `Vec`, so the
//! generator can emit them as associated consts with no runtime construction.
//! Only the dimensions this spike exercises are projected.

use serde::de::DeserializeOwned;

use crate::client::Method;
use crate::engine::{KindState, PrevState};

/// Where a request input goes.
#[derive(Debug, Clone, Copy)]
pub enum Input {
    Query(&'static str),
    Body(&'static str),
}

/// Where a response value is read.
#[derive(Debug, Clone, Copy)]
pub enum ValueAt {
    Body(&'static [&'static str]),
    Header(&'static str),
}

#[derive(Debug, Clone, Copy)]
pub struct EndpointSpec {
    pub method: Method,
    pub path: &'static str,
    /// Path to the item array; empty for a root array.
    pub items: &'static [&'static str],
    pub total: Option<ValueAt>,
}

/// Every generated endpoint, paginated or not.
pub trait Endpoint {
    type Item: DeserializeOwned;
    const SPEC: &'static EndpointSpec;
}

// --- strategy kinds -------------------------------------------------------

/// Marker for a strategy kind. Uninhabited: never constructed, only named.
pub trait Kind: 'static {
    type Spec: 'static;
    /// The backward leg's data; `Never` for kinds with no backward direction,
    /// which makes `HasPrev` unimplementable for them.
    type Prev: 'static;
    #[doc(hidden)]
    fn start(spec: &'static Self::Spec) -> KindState;
    #[doc(hidden)]
    fn prev(prev: &'static Self::Prev) -> PrevState;
}

#[derive(Debug, Clone, Copy)]
pub enum Never {}

#[derive(Debug, Clone, Copy)]
pub enum More {
    /// End on an empty page.
    NonEmptyPage,
    /// More while the `Link` header carries this rel.
    LinkRel(&'static str),
    /// More while this boolean is true.
    Flag(ValueAt),
}

pub enum PageNumber {}
#[derive(Debug, Clone, Copy)]
pub struct PageNumberSpec {
    pub page: Input,
    pub base: u64,
    pub more: More,
}

pub enum Cursor {}
#[derive(Debug, Clone, Copy)]
pub struct CursorLeg {
    pub param: Input,
    pub from: ValueAt,
}
#[derive(Debug, Clone, Copy)]
pub struct CursorSpec {
    pub forward: CursorLeg,
    pub more: Option<More>,
}

pub enum NextLink {}
#[derive(Debug, Clone, Copy)]
pub enum LinkFrom {
    /// RFC 8288 `Link` header with this rel.
    Header(&'static str),
    /// A URL in the body.
    Body(&'static [&'static str]),
}
#[derive(Debug, Clone, Copy)]
pub struct LinkSpec {
    pub next: LinkFrom,
}

/// Elasticsearch-scroll-shaped session: the first page opens it, follow-ups
/// go to `follow_path` carrying the latest handle, `close_path` releases it.
pub enum Scroll {}
#[derive(Debug, Clone, Copy)]
pub struct ScrollSpec {
    pub open_keep_alive: Input,
    pub default_keep_alive: &'static str,
    pub follow_path: &'static str,
    pub handle_from: ValueAt,
    pub handle_param: Input,
    pub keep_alive_param: Input,
    pub close_path: &'static str,
}

// --- the paginated contract and knob capabilities -------------------------

/// Implemented only for endpoints whose definition authors a strategy. An
/// unpaginated list endpoint does not implement it, so `Pager<E>` cannot be
/// built for it.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not paginated",
    label = "no pagination strategy is authored for `{Self}`",
    note = "the definition does not author a pagination strategy for this endpoint, so the generated client has no pager for it"
)]
pub trait Paginated: Endpoint {
    type Kind: Kind;
    const STRATEGY: &'static <Self::Kind as Kind>::Spec;
}

#[derive(Debug, Clone, Copy)]
pub struct SizeSpec {
    pub param: Input,
    pub min: u32,
    pub max: Option<u32>,
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` has no authored page size",
    label = "`.page_size(..)` needs a `PageSize` in the definition",
    note = "the definition for `{Self}` does not declare a `PageSize`, so the generated pager has no `.page_size(..)`"
)]
pub trait PageSizeKnob: Paginated {
    const PAGE_SIZE: SizeSpec;
}

/// A resumable cursor: the caller may start from a cursor it kept.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no resumable cursor",
    label = "`.after(..)` needs a resumable cursor in the definition",
    note = "the definition for `{Self}` does not mark its cursor `resumable`"
)]
pub trait ResumeAfter: Paginated<Kind = Cursor> {}

#[diagnostic::on_unimplemented(
    message = "`{Self}` has no backward direction",
    label = "this needs a backward leg (`Cursor.backward` or `NextLink.prev`) in the definition"
)]
pub trait HasPrev: Paginated {
    const PREV: &'static <Self::Kind as Kind>::Prev;
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` has no adjustable keep-alive",
    label = "`.keep_alive(..)` needs an adjustable `KeepAlive` in the definition"
)]
pub trait KeepAliveKnob: Paginated<Kind = Scroll> {}

/// Named kind capabilities. Blanket-implemented from `Kind`, so the
/// generator emits nothing for them; `do_not_recommend` makes rustc report
/// the named trait (with its message) instead of an `E0271` kind mismatch.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not paginated by page number",
    label = "`.page(n)` exists only for page-number strategies",
    note = "the definition authors a different strategy for `{Self}`"
)]
pub trait PageNumbered: Paginated {}
#[diagnostic::do_not_recommend]
impl<E: Paginated<Kind = PageNumber>> PageNumbered for E {}

#[diagnostic::on_unimplemented(
    message = "`{Self}` has no server-side session",
    label = "this exists only for sessioned strategies",
    note = "the definition for `{Self}` authors no `Session`"
)]
pub trait Sessioned: Paginated {}
#[diagnostic::do_not_recommend]
impl<E: Paginated<Kind = Scroll>> Sessioned for E {}

// --- Kind impls: the only place a kind's data becomes engine state ---------

impl Kind for PageNumber {
    type Spec = PageNumberSpec;
    type Prev = Never;
    fn start(spec: &'static PageNumberSpec) -> KindState {
        KindState::page_number(spec)
    }
    fn prev(never: &'static Never) -> PrevState {
        match *never {}
    }
}

impl Kind for Cursor {
    type Spec = CursorSpec;
    type Prev = CursorLeg;
    fn start(spec: &'static CursorSpec) -> KindState {
        KindState::cursor(spec)
    }
    fn prev(leg: &'static CursorLeg) -> PrevState {
        PrevState::Cursor(leg)
    }
}

impl Kind for NextLink {
    type Spec = LinkSpec;
    type Prev = LinkFrom;
    fn start(spec: &'static LinkSpec) -> KindState {
        KindState::link(spec)
    }
    fn prev(from: &'static LinkFrom) -> PrevState {
        PrevState::Link(from)
    }
}

impl Kind for Scroll {
    type Spec = ScrollSpec;
    type Prev = Never;
    fn start(spec: &'static ScrollSpec) -> KindState {
        KindState::scroll(spec)
    }
    fn prev(never: &'static Never) -> PrevState {
        match *never {}
    }
}

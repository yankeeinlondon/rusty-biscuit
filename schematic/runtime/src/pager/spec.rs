//! The `&'static` projection of a pagination strategy.
//!
//! These types mirror the definition model in `schematic-define`'s
//! `pagination` module, with slices and `&'static str` in place of `Vec` and
//! `String`, so the generator can emit a strategy as an associated constant
//! of [`Paginated`](super::Paginated) and the runtime never builds one at run
//! time. The runtime does not depend on `schematic-define`; the generator
//! translates one into the other, and a rule that the define side validates
//! (for example that a `FullPage` signal has a page size) is assumed here.

use http::Method;

/// Where the pager writes a value on a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Input {
    /// A query-string parameter, matched by exact name.
    Query(&'static str),
    /// A request header, matched without regard to case.
    Header(&'static str),
    /// A JSON request-body field, as its object keys from the root.
    Body(&'static [&'static str]),
}

/// Where the pager reads a value on a response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Source {
    /// A JSON body field, as its object keys from the root.
    Body(&'static [&'static str]),
    /// A response header, matched without regard to case.
    Header(&'static str),
}

/// A request input whose value the strategy fixes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fixed {
    pub param: Input,
    pub value: &'static str,
}

/// Index origin of page numbers and item offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexBase {
    ZeroBased,
    OneBased,
}

impl IndexBase {
    /// The first index: `0` or `1`.
    pub const fn first(self) -> u64 {
        match self {
            Self::ZeroBased => 0,
            Self::OneBased => 1,
        }
    }
}

/// What a `true` flag means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    TrueMeansMore,
    TrueMeansLast,
}

/// What a total counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountUnit {
    Items,
    Pages,
}

/// The caller-settable page size.
///
/// `server_default` documents the provider's default and is never sent;
/// `sent_default` is sent when the caller sets no size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageSize {
    pub param: Input,
    pub server_default: Option<u32>,
    pub sent_default: Option<u32>,
    pub min: u32,
    pub max: Option<u32>,
    pub adjustable: bool,
}

/// How a page says whether another page follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MoreSignal {
    /// A boolean field or header.
    Flag { source: Source, polarity: Polarity },
    /// More follows while the value is present and non-empty.
    Present { source: Source },
    /// More follows while a `Link` header has this relation.
    LinkRel { rel: &'static str },
    /// A total of items or pages.
    Total { source: Source, unit: CountUnit },
    /// More follows while a page has records.
    NonEmptyPage,
    /// More follows while a page is full at the effective page size.
    FullPage,
}

/// Where a cursor value comes from. Item paths are relative to the record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CursorSource {
    Field(Source),
    LastItem(&'static [&'static str]),
    FirstItem(&'static [&'static str]),
}

/// One value of a possibly composite cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPart {
    pub param: Input,
    pub source: CursorSource,
}

/// Whether the server's position predicate includes the cursor position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Boundary {
    /// The boundary record is excluded.
    Exclusive,
    /// The boundary records come back; records whose `identity` (relative
    /// to the record) was already delivered at the same key are dropped.
    Inclusive { identity: &'static [&'static str] },
}

/// One cursor direction: its parts, sent together, and its boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorLeg {
    pub parts: &'static [CursorPart],
    pub boundary: Boundary,
}

/// Where a next- or previous-page URL comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LinkSource {
    /// A `Link` header entry with this relation.
    LinkHeader { rel: &'static str },
    /// A body field or header holding the URL.
    Field(Source),
}

/// The strategy kind and its kind-specific dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Kind {
    PageNumber {
        page: Input,
        base: IndexBase,
        more: MoreSignal,
    },
    Offset {
        offset: Input,
        base: IndexBase,
        more: MoreSignal,
    },
    Cursor {
        forward: CursorLeg,
        backward: Option<CursorLeg>,
        more: Option<MoreSignal>,
        resumable: bool,
    },
    NextLink {
        next: LinkSource,
        prev: Option<LinkSource>,
    },
}

/// Which of the first request's ordinary inputs follow-up requests re-send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Carry {
    All,
    Only(&'static [&'static str]),
    AllExcept(&'static [&'static str]),
    Nothing,
}

impl Carry {
    /// Whether the ordinary input `name` is re-sent.
    pub fn keeps(self, name: &str) -> bool {
        match self {
            Self::All => true,
            Self::Only(names) => names.contains(&name),
            Self::AllExcept(names) => !names.contains(&name),
            Self::Nothing => false,
        }
    }
}

/// How follow-up requests differ from the first one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FollowUp {
    /// `None` keeps the endpoint's method.
    pub method: Option<Method>,
    /// A path template using only the endpoint's own `{placeholders}`;
    /// `None` keeps the endpoint's path.
    pub path: Option<&'static str>,
    pub carry: Carry,
}

/// A non-page request a session issues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionStep {
    pub method: Method,
    pub path: &'static str,
    pub handle: Option<Input>,
    pub constants: &'static [Fixed],
}

/// How a session is opened.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SessionOpen {
    /// The first page request opens it.
    FirstPage,
    /// A separate request opens it and returns the handle.
    Request { step: SessionStep, handle: Source },
}

/// A session lease kept alive by sending a duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeepAlive {
    pub default: &'static str,
    pub on_open: Option<Input>,
    pub on_page: Option<Input>,
    pub adjustable: bool,
}

/// A server-side result set with a lifetime, iterated only inside a scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub open: SessionOpen,
    pub pages: Option<(Method, &'static str)>,
    pub handle_from_page: Option<Source>,
    pub send_handle_as: Option<Input>,
    pub keep_alive: Option<KeepAlive>,
    pub renew: Option<SessionStep>,
    pub close: Option<SessionStep>,
}

/// Disjoint slices iterated independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Partition {
    pub index: Input,
    pub count: Input,
    pub max: Option<u32>,
}

/// One way an endpoint paginates: its kind plus the shared dimensions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Strategy {
    pub kind: Kind,
    /// The page size of any kind.
    pub size: Option<PageSize>,
    pub constants: &'static [Fixed],
    /// An informational total, separate from a [`MoreSignal::Total`].
    pub total: Option<Source>,
    /// Continuation values meaning "none", beyond missing, null, and empty.
    pub terminal_values: &'static [&'static str],
    /// `None`: carry everything, except for a next link, which is followed
    /// verbatim with GET.
    pub follow_up: Option<FollowUp>,
    pub session: Option<Session>,
    pub partition: Option<Partition>,
}

impl Strategy {
    /// The authored end signal; a next link has none.
    pub fn more(&self) -> Option<MoreSignal> {
        match self.kind {
            Kind::PageNumber { more, .. } | Kind::Offset { more, .. } => Some(more),
            Kind::Cursor { more, .. } => more,
            Kind::NextLink { .. } => None,
        }
    }

    /// The carry rule actually applied to follow-up requests.
    pub fn effective_carry(&self) -> Carry {
        match (&self.follow_up, self.kind) {
            (Some(follow_up), _) => follow_up.carry,
            (None, Kind::NextLink { .. }) => Carry::Nothing,
            (None, _) => Carry::All,
        }
    }
}

/// One step of an [`ItemSelector`]'s record path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Descend into an object key.
    Key(&'static str),
    /// Fan out over every element of an array.
    Each,
}

/// Where a page's items are: steps to each record (ending with a fan-out),
/// then a projection from record to the item the caller receives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemSelector {
    pub record: &'static [Step],
    pub projection: &'static [&'static str],
}

impl ItemSelector {
    /// The page body is the array of items.
    pub const ROOT: Self = Self {
        record: &[Step::Each],
        projection: &[],
    };
}

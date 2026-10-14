//! Pagination strategies: the four kinds and the dimensions they share.

use serde::{Deserialize, Serialize};

use super::selector::JsonPath;
use crate::types::RestMethod;

// ---------------------------------------------------------------------------
// Request and response locations
// ---------------------------------------------------------------------------

/// Where the pager writes a value on a request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "in", content = "name", rename_all = "snake_case")]
#[non_exhaustive]
pub enum InputParam {
    /// A query-string parameter, matched by exact name.
    Query(String),
    /// A request header, matched without regard to case.
    Header(String),
    /// A JSON request-body field; the only place a structured value may go.
    Body(JsonPath),
}

impl InputParam {
    pub fn query(name: &str) -> Self {
        Self::Query(name.to_owned())
    }

    pub fn header(name: &str) -> Self {
        Self::Header(name.to_owned())
    }

    /// A body field; `path` is split on `.`.
    pub fn body(path: &str) -> Self {
        Self::Body(JsonPath::parse(path))
    }

    /// The name as written in a definition (`after`, `x-ms-continuation`,
    /// `pit.id`).
    pub fn display_name(&self) -> String {
        match self {
            Self::Query(name) | Self::Header(name) => name.clone(),
            Self::Body(path) => path.to_string(),
        }
    }

    pub fn is_body(&self) -> bool {
        matches!(self, Self::Body(_))
    }

    /// Whether writing both inputs would write one destination twice.
    ///
    /// Query names compare exactly (a repeated query name is two values, not
    /// one destination, but the pager owns the name either way); header names
    /// compare without case; a body path conflicts with itself and with any
    /// path it is a prefix of (`pit` and `pit.id`).
    pub(crate) fn same_destination(&self, other: &InputParam) -> bool {
        match (self, other) {
            (Self::Query(a), Self::Query(b)) => a == b,
            (Self::Header(a), Self::Header(b)) => a.eq_ignore_ascii_case(b),
            (Self::Body(a), Self::Body(b)) => a.is_prefix_of(b) || b.is_prefix_of(a),
            _ => false,
        }
    }

    /// Whether the destination is unusable: an empty name or body path, or a
    /// body path with an empty segment.
    pub(crate) fn is_empty_destination(&self) -> bool {
        match self {
            Self::Query(name) | Self::Header(name) => name.is_empty(),
            Self::Body(path) => path.is_root() || path.has_empty_segment(),
        }
    }
}

impl From<&str> for InputParam {
    fn from(name: &str) -> Self {
        Self::query(name)
    }
}

/// Where the pager reads a value on a response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "from", content = "at", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ValueSource {
    /// A JSON body field.
    Body(JsonPath),
    /// A response header, matched without regard to case.
    Header(String),
}

impl ValueSource {
    /// A body field; `path` is split on `.`.
    pub fn body(path: &str) -> Self {
        Self::Body(JsonPath::parse(path))
    }

    pub fn header(name: &str) -> Self {
        Self::Header(name.to_owned())
    }

    pub(crate) fn is_empty_location(&self) -> bool {
        match self {
            Self::Body(path) => path.is_root() || path.has_empty_segment(),
            Self::Header(name) => name.is_empty(),
        }
    }
}

/// A request input whose value the strategy fixes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixedParam {
    pub param: InputParam,
    pub value: String,
}

// ---------------------------------------------------------------------------
// Small supporting types
// ---------------------------------------------------------------------------

/// Index origin for page numbers and item offsets (SCIM `startIndex` is
/// 1-based; Hugging Face `p` is a 0-based page).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexBase {
    ZeroBased,
    OneBased,
}

/// What a `true` boolean flag means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Polarity {
    TrueMeansMore,
    TrueMeansLast,
}

/// What a total counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CountUnit {
    Items,
    Pages,
}

/// The caller-settable page size.
///
/// `server_default` documents the provider's own default and is never sent.
/// `sent_default` is an authored value the pager sends when the caller sets no
/// size; it is required when the provider requires the parameter (R2a), and is
/// otherwise left unset so an unset size is omitted. `adjustable` lets the
/// caller change the size between pages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageSize {
    pub param: InputParam,
    pub server_default: Option<u32>,
    pub sent_default: Option<u32>,
    pub min: u32,
    pub max: Option<u32>,
    pub adjustable: bool,
}

impl PageSize {
    /// A size written to `param`, minimum 1, no maximum, no defaults.
    pub fn new(param: impl Into<InputParam>) -> Self {
        Self {
            param: param.into(),
            server_default: None,
            sent_default: None,
            min: 1,
            max: None,
            adjustable: false,
        }
    }

    pub fn server_default(mut self, value: u32) -> Self {
        self.server_default = Some(value);
        self
    }

    pub fn sent_default(mut self, value: u32) -> Self {
        self.sent_default = Some(value);
        self
    }

    pub fn max(mut self, value: u32) -> Self {
        self.max = Some(value);
        self
    }

    pub fn min(mut self, value: u32) -> Self {
        self.min = value;
        self
    }

    pub fn adjustable(mut self) -> Self {
        self.adjustable = true;
        self
    }
}

/// How a page says whether another page follows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "signal", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum MoreSignal {
    /// A boolean field or header.
    Flag { source: ValueSource, polarity: Polarity },
    /// More follows while the value is present and non-empty
    /// (GitLab's `X-Next-Page`).
    Present { source: ValueSource },
    /// More follows while a `Link` header has this relation.
    LinkRel { rel: String },
    /// A total of items or pages.
    Total { source: ValueSource, unit: CountUnit },
    /// More follows while a page has items.
    NonEmptyPage,
    /// More follows while a page is full at the effective page size.
    FullPage,
}

impl MoreSignal {
    /// A body flag that is `true` while more pages follow.
    pub fn has_more(path: &str) -> Self {
        Self::Flag {
            source: ValueSource::body(path),
            polarity: Polarity::TrueMeansMore,
        }
    }

    /// A body flag that is `true` on the last page.
    pub fn is_last(path: &str) -> Self {
        Self::Flag {
            source: ValueSource::body(path),
            polarity: Polarity::TrueMeansLast,
        }
    }

    pub fn link_rel(rel: &str) -> Self {
        Self::LinkRel { rel: rel.to_owned() }
    }

    pub fn header_present(name: &str) -> Self {
        Self::Present {
            source: ValueSource::header(name),
        }
    }

    pub fn total_items(source: ValueSource) -> Self {
        Self::Total {
            source,
            unit: CountUnit::Items,
        }
    }

    pub fn total_pages(source: ValueSource) -> Self {
        Self::Total {
            source,
            unit: CountUnit::Pages,
        }
    }
}

/// Where a cursor value comes from.
///
/// `LastItem`/`FirstItem` paths are relative to the *record* (see
/// [`ItemSelector`](super::ItemSelector)).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "from", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum CursorSource {
    Field { source: ValueSource },
    LastItem { path: JsonPath },
    FirstItem { path: JsonPath },
}

impl CursorSource {
    /// A body field; `path` is split on `.`.
    pub fn body(path: &str) -> Self {
        Self::Field {
            source: ValueSource::body(path),
        }
    }

    pub fn header(name: &str) -> Self {
        Self::Field {
            source: ValueSource::header(name),
        }
    }

    /// A field of the last record; `path` is split on `.`.
    pub fn last_item(path: &str) -> Self {
        Self::LastItem {
            path: JsonPath::parse(path),
        }
    }

    /// A field of the first record; `path` is split on `.`.
    pub fn first_item(path: &str) -> Self {
        Self::FirstItem {
            path: JsonPath::parse(path),
        }
    }

    pub fn is_item_derived(&self) -> bool {
        matches!(self, Self::LastItem { .. } | Self::FirstItem { .. })
    }
}

/// One value of a (possibly composite) cursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CursorPart {
    pub param: InputParam,
    pub source: CursorSource,
}

/// Whether the server's position predicate includes the cursor position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "boundary", rename_all = "snake_case", deny_unknown_fields)]
pub enum Boundary {
    /// `after`/`starting_after`/`search_after`: the boundary row is excluded.
    Exclusive,
    /// `since >= t`: the boundary rows come back; the client drops records
    /// whose `identity` it already delivered at the same key.
    Inclusive { identity: JsonPath },
}

/// A cursor direction: one or more parts (a sort key plus tiebreakers) sent
/// together, and the boundary semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CursorLeg {
    pub parts: Vec<CursorPart>,
    pub boundary: Boundary,
}

impl CursorLeg {
    /// One exclusive part.
    pub fn single(param: impl Into<InputParam>, source: CursorSource) -> Self {
        Self {
            parts: vec![CursorPart {
                param: param.into(),
                source,
            }],
            boundary: Boundary::Exclusive,
        }
    }

    /// Adds a tiebreaker part.
    pub fn tiebreak(mut self, param: impl Into<InputParam>, source: CursorSource) -> Self {
        self.parts.push(CursorPart {
            param: param.into(),
            source,
        });
        self
    }

    /// Marks the boundary inclusive, deduplicating on `identity` (split on
    /// `.`).
    pub fn inclusive(mut self, identity: &str) -> Self {
        self.boundary = Boundary::Inclusive {
            identity: JsonPath::parse(identity),
        };
        self
    }
}

/// Where a next- or previous-page URL comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "from", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum LinkSource {
    /// A `Link` header entry with this relation.
    LinkHeader { rel: String },
    /// A body field or header holding the URL.
    Field { source: ValueSource },
}

impl LinkSource {
    pub fn link_header(rel: &str) -> Self {
        Self::LinkHeader { rel: rel.to_owned() }
    }

    /// A body field; `path` is split on `.`.
    pub fn body(path: &str) -> Self {
        Self::Field {
            source: ValueSource::body(path),
        }
    }

    pub fn header(name: &str) -> Self {
        Self::Field {
            source: ValueSource::header(name),
        }
    }

    /// A body field named by whole keys (`@odata.nextLink`).
    pub fn body_segments<I, S>(segments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::Field {
            source: ValueSource::Body(JsonPath::segments(segments)),
        }
    }
}

// ---------------------------------------------------------------------------
// The four kinds
// ---------------------------------------------------------------------------

/// Increment a page index from its base.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageNumberStrategy {
    pub page: InputParam,
    pub base: IndexBase,
    pub size: Option<PageSize>,
    pub more: MoreSignal,
}

/// Advance an item index by the number of records received.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OffsetStrategy {
    pub offset: InputParam,
    pub base: IndexBase,
    pub size: Option<PageSize>,
    pub more: MoreSignal,
}

/// Copy one or more values from the response into request inputs.
///
/// Without `more`, a missing or terminal continuation ends traversal.
/// `resumable` lets the caller start from a cursor (`.after(..)`, and
/// `.before(..)` when a backward leg exists).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CursorStrategy {
    pub forward: CursorLeg,
    pub backward: Option<CursorLeg>,
    pub size: Option<PageSize>,
    pub more: Option<MoreSignal>,
    pub resumable: bool,
}

/// Request a server-provided URL. Followed links obey the API's
/// [`CrossOrigin`](crate::CrossOrigin) policy; the strategy has no trust
/// setting of its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NextLinkStrategy {
    pub next: LinkSource,
    pub prev: Option<LinkSource>,
    pub size: Option<PageSize>,
}

/// How the next request is built.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum PageStrategy {
    PageNumber(PageNumberStrategy),
    Offset(OffsetStrategy),
    Cursor(CursorStrategy),
    NextLink(NextLinkStrategy),
}

impl PageStrategy {
    pub fn size(&self) -> Option<&PageSize> {
        match self {
            Self::PageNumber(kind) => kind.size.as_ref(),
            Self::Offset(kind) => kind.size.as_ref(),
            Self::Cursor(kind) => kind.size.as_ref(),
            Self::NextLink(kind) => kind.size.as_ref(),
        }
    }

    fn size_mut(&mut self) -> &mut Option<PageSize> {
        match self {
            Self::PageNumber(kind) => &mut kind.size,
            Self::Offset(kind) => &mut kind.size,
            Self::Cursor(kind) => &mut kind.size,
            Self::NextLink(kind) => &mut kind.size,
        }
    }

    /// The forward cursor leg, then the backward one; empty for other kinds.
    pub fn legs(&self) -> Vec<&CursorLeg> {
        match self {
            Self::Cursor(kind) => std::iter::once(&kind.forward)
                .chain(kind.backward.iter())
                .collect(),
            _ => Vec::new(),
        }
    }

    /// The authored end signal; `NextLink` has none (link absence ends it).
    pub fn more(&self) -> Option<&MoreSignal> {
        match self {
            Self::PageNumber(kind) => Some(&kind.more),
            Self::Offset(kind) => Some(&kind.more),
            Self::Cursor(kind) => kind.more.as_ref(),
            Self::NextLink(_) => None,
        }
    }

    /// The kind's stable snake-case name.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::PageNumber(_) => "page_number",
            Self::Offset(_) => "offset",
            Self::Cursor(_) => "cursor",
            Self::NextLink(_) => "next_link",
        }
    }
}

// ---------------------------------------------------------------------------
// Shared dimensions: follow-up shape, session lifecycle, partitioning
// ---------------------------------------------------------------------------

/// Which of the first request's ordinary inputs (query parameters and
/// top-level body fields) follow-up requests re-send.
///
/// Continuation inputs, session inputs, keep-alive, and an adjusted page size
/// are always sent; this governs everything else.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "carry", content = "names", rename_all = "snake_case")]
pub enum Carry {
    All,
    Only(Vec<String>),
    AllExcept(Vec<String>),
    Nothing,
}

/// How follow-up requests differ from the first one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FollowUp {
    /// `None` keeps the endpoint's method (`NextLink`: GET).
    pub method: Option<RestMethod>,
    /// `None` keeps the endpoint's path. Must be `None` for `NextLink`.
    pub path: Option<String>,
    pub carry: Carry,
}

impl FollowUp {
    /// Same method and path; `carry` decides what is re-sent.
    pub fn carry(carry: Carry) -> Self {
        Self {
            method: None,
            path: None,
            carry,
        }
    }

    /// A different method and path that carries nothing.
    pub fn to(method: RestMethod, path: &str) -> Self {
        Self {
            method: Some(method),
            path: Some(path.to_owned()),
            carry: Carry::Nothing,
        }
    }

    pub fn method(mut self, method: RestMethod) -> Self {
        self.method = Some(method);
        self
    }

    pub fn with_carry(mut self, carry: Carry) -> Self {
        self.carry = carry;
        self
    }
}

/// A non-page request the pager issues for a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionStep {
    pub method: RestMethod,
    /// May use only the endpoint's own path parameters.
    pub path: String,
    /// Where the session handle goes on this request.
    pub handle: Option<InputParam>,
    pub constants: Vec<FixedParam>,
}

impl SessionStep {
    pub fn new(method: RestMethod, path: &str) -> Self {
        Self {
            method,
            path: path.to_owned(),
            handle: None,
            constants: Vec::new(),
        }
    }

    pub fn handle(mut self, param: impl Into<InputParam>) -> Self {
        self.handle = Some(param.into());
        self
    }

    pub fn constant(mut self, param: impl Into<InputParam>, value: &str) -> Self {
        self.constants.push(FixedParam {
            param: param.into(),
            value: value.to_owned(),
        });
        self
    }
}

/// How a session is opened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "open", rename_all = "snake_case", deny_unknown_fields)]
pub enum SessionOpen {
    /// The first page request opens the session (Elasticsearch scroll).
    FirstPage,
    /// A separate request opens it and returns the handle (point in time).
    Request { step: SessionStep, handle: ValueSource },
}

/// Keeps a session alive by sending a duration.
///
/// `on_open` goes on the opening request; `on_page` goes on every page request
/// that is not the opening one, so each page renews the lease.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeepAlive {
    pub default: String,
    pub on_open: Option<InputParam>,
    pub on_page: Option<InputParam>,
    pub adjustable: bool,
}

impl KeepAlive {
    pub fn new(default: &str) -> Self {
        Self {
            default: default.to_owned(),
            on_open: None,
            on_page: None,
            adjustable: false,
        }
    }

    pub fn on_open(mut self, param: impl Into<InputParam>) -> Self {
        self.on_open = Some(param.into());
        self
    }

    pub fn on_page(mut self, param: impl Into<InputParam>) -> Self {
        self.on_page = Some(param.into());
        self
    }

    pub fn adjustable(mut self) -> Self {
        self.adjustable = true;
        self
    }
}

/// A server-side result set with a lifetime: opened, optionally renewed,
/// paged through with a handle, and closed.
///
/// A session endpoint is iterated only inside the generated scoped helper,
/// which attempts the close when the scope ends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub open: SessionOpen,
    /// Where page requests go while the session is open (point-in-time pages
    /// go to `/_search`, not the endpoint path). `None` keeps the endpoint.
    pub pages: Option<(RestMethod, String)>,
    /// An updated handle on each page; the latest wins.
    pub handle_from_page: Option<ValueSource>,
    /// Where page requests carry the handle; `None` when the continuation
    /// itself carries it (scroll id as cursor, locator in a next link).
    pub send_handle_as: Option<InputParam>,
    pub keep_alive: Option<KeepAlive>,
    /// Out-of-band renewal, exposed as `renew()` inside the scope.
    pub renew: Option<SessionStep>,
    pub close: Option<SessionStep>,
}

impl Session {
    pub fn opened_by_first_page() -> Self {
        Self {
            open: SessionOpen::FirstPage,
            pages: None,
            handle_from_page: None,
            send_handle_as: None,
            keep_alive: None,
            renew: None,
            close: None,
        }
    }

    pub fn opened_by(step: SessionStep, handle: ValueSource) -> Self {
        Self {
            open: SessionOpen::Request { step, handle },
            ..Self::opened_by_first_page()
        }
    }

    pub fn pages_at(mut self, method: RestMethod, path: &str) -> Self {
        self.pages = Some((method, path.to_owned()));
        self
    }

    pub fn handle_from_page(mut self, source: ValueSource) -> Self {
        self.handle_from_page = Some(source);
        self
    }

    pub fn send_handle_as(mut self, param: impl Into<InputParam>) -> Self {
        self.send_handle_as = Some(param.into());
        self
    }

    pub fn keep_alive(mut self, keep_alive: KeepAlive) -> Self {
        self.keep_alive = Some(keep_alive);
        self
    }

    pub fn renew(mut self, step: SessionStep) -> Self {
        self.renew = Some(step);
        self
    }

    pub fn close(mut self, step: SessionStep) -> Self {
        self.close = Some(step);
        self
    }
}

/// Disjoint slices iterated independently (Elasticsearch sliced scroll or
/// point in time). Each iterator sends its slice index and the slice count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Partition {
    pub index: InputParam,
    pub count: InputParam,
    pub max: Option<u32>,
}

impl Partition {
    pub fn new(index: impl Into<InputParam>, count: impl Into<InputParam>) -> Self {
        Self {
            index: index.into(),
            count: count.into(),
            max: None,
        }
    }

    pub fn max(mut self, value: u32) -> Self {
        self.max = Some(value);
        self
    }
}

// ---------------------------------------------------------------------------
// Strategy
// ---------------------------------------------------------------------------

/// One way an endpoint paginates: a [`PageStrategy`] kind plus the dimensions
/// every kind shares.
///
/// Built through the kind constructors ([`Strategy::page_number`],
/// [`Strategy::offset`], [`Strategy::cursor`], [`Strategy::next_link`]);
/// there is no empty default strategy.
///
/// ## Examples
///
/// ```
/// use schematic_define::pagination::{MoreSignal, PageSize, Strategy, ValueSource};
///
/// let github = Strategy::page_number("page", MoreSignal::link_rel("next"))
///     .page_size(PageSize::new("per_page").server_default(30).max(100))
///     .build();
/// let workflow_runs = github.clone().total(ValueSource::body("total_count"));
///
/// assert!(github.total.is_none());
/// assert!(workflow_runs.total.is_some());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Strategy {
    pub kind: PageStrategy,
    pub constants: Vec<FixedParam>,
    pub total: Option<ValueSource>,
    /// Continuation values that mean "none", beyond missing, null, and empty
    /// (Salesforce Bulk 2.0 sends the string `null`).
    pub terminal_values: Vec<String>,
    /// `None`: carry everything (`NextLink`: nothing; the URL verbatim, GET).
    pub follow_up: Option<FollowUp>,
    pub session: Option<Session>,
    pub partition: Option<Partition>,
}

impl Strategy {
    /// A 1-based page number written to `page`.
    pub fn page_number(page: impl Into<InputParam>, more: MoreSignal) -> PageNumberBuilder {
        PageNumberBuilder {
            kind: PageNumberStrategy {
                page: page.into(),
                base: IndexBase::OneBased,
                size: None,
                more,
            },
        }
    }

    /// A 0-based item offset written to `offset`.
    pub fn offset(offset: impl Into<InputParam>, more: MoreSignal) -> OffsetBuilder {
        OffsetBuilder {
            kind: OffsetStrategy {
                offset: offset.into(),
                base: IndexBase::ZeroBased,
                size: None,
                more,
            },
        }
    }

    /// A single-part exclusive cursor written to `param`.
    pub fn cursor(param: impl Into<InputParam>, source: CursorSource) -> CursorBuilder {
        Self::cursor_leg(CursorLeg::single(param, source))
    }

    pub fn cursor_leg(forward: CursorLeg) -> CursorBuilder {
        CursorBuilder {
            kind: CursorStrategy {
                forward,
                backward: None,
                size: None,
                more: None,
                resumable: false,
            },
        }
    }

    pub fn next_link(next: LinkSource) -> NextLinkBuilder {
        NextLinkBuilder {
            kind: NextLinkStrategy {
                next,
                prev: None,
                size: None,
            },
        }
    }

    pub fn constant(mut self, param: impl Into<InputParam>, value: &str) -> Self {
        self.constants.push(FixedParam {
            param: param.into(),
            value: value.to_owned(),
        });
        self
    }

    pub fn total(mut self, source: ValueSource) -> Self {
        self.total = Some(source);
        self
    }

    pub fn terminal_value(mut self, value: &str) -> Self {
        self.terminal_values.push(value.to_owned());
        self
    }

    pub fn follow_up(mut self, follow_up: FollowUp) -> Self {
        self.follow_up = Some(follow_up);
        self
    }

    pub fn session(mut self, session: Session) -> Self {
        self.session = Some(session);
        self
    }

    pub fn partition(mut self, partition: Partition) -> Self {
        self.partition = Some(partition);
        self
    }

    /// Replaces the page size of any kind (an API helper's override).
    pub fn with_page_size(mut self, size: PageSize) -> Self {
        *self.kind.size_mut() = Some(size);
        self
    }

    /// The carry rule actually applied to follow-up requests.
    pub fn effective_carry(&self) -> Carry {
        match (&self.follow_up, &self.kind) {
            (Some(follow_up), _) => follow_up.carry.clone(),
            (None, PageStrategy::NextLink(_)) => Carry::Nothing,
            (None, _) => Carry::All,
        }
    }

    fn from_kind(kind: PageStrategy) -> Self {
        Self {
            kind,
            constants: Vec::new(),
            total: None,
            terminal_values: Vec::new(),
            follow_up: None,
            session: None,
            partition: None,
        }
    }
}

/// Builder methods shared by the four kind builders. Kind-specific setters
/// return the builder; shared-dimension setters finish it into a [`Strategy`].
macro_rules! kind_builder {
    ($builder:ident, $kind:ident, $variant:ident) => {
        /// Builds one strategy kind; see [`Strategy`].
        #[derive(Debug, Clone)]
        pub struct $builder {
            kind: $kind,
        }

        impl $builder {
            pub fn build(self) -> Strategy {
                Strategy::from_kind(PageStrategy::$variant(self.kind))
            }

            pub fn page_size(mut self, size: PageSize) -> Self {
                self.kind.size = Some(size);
                self
            }

            pub fn constant(self, param: impl Into<InputParam>, value: &str) -> Strategy {
                self.build().constant(param, value)
            }

            pub fn total(self, source: ValueSource) -> Strategy {
                self.build().total(source)
            }

            pub fn terminal_value(self, value: &str) -> Strategy {
                self.build().terminal_value(value)
            }

            pub fn follow_up(self, follow_up: FollowUp) -> Strategy {
                self.build().follow_up(follow_up)
            }

            pub fn session(self, session: Session) -> Strategy {
                self.build().session(session)
            }

            pub fn partition(self, partition: Partition) -> Strategy {
                self.build().partition(partition)
            }
        }

        impl From<$builder> for Strategy {
            fn from(builder: $builder) -> Self {
                builder.build()
            }
        }
    };
}

kind_builder!(PageNumberBuilder, PageNumberStrategy, PageNumber);
kind_builder!(OffsetBuilder, OffsetStrategy, Offset);
kind_builder!(CursorBuilder, CursorStrategy, Cursor);
kind_builder!(NextLinkBuilder, NextLinkStrategy, NextLink);

impl PageNumberBuilder {
    pub fn zero_based(mut self) -> Self {
        self.kind.base = IndexBase::ZeroBased;
        self
    }
}

impl OffsetBuilder {
    pub fn one_based(mut self) -> Self {
        self.kind.base = IndexBase::OneBased;
        self
    }
}

impl CursorBuilder {
    pub fn backward(self, param: impl Into<InputParam>, source: CursorSource) -> Self {
        self.backward_leg(CursorLeg::single(param, source))
    }

    pub fn backward_leg(mut self, leg: CursorLeg) -> Self {
        self.kind.backward = Some(leg);
        self
    }

    /// Adds a tiebreaker part to the forward cursor.
    pub fn tiebreak(mut self, param: impl Into<InputParam>, source: CursorSource) -> Self {
        self.kind.forward = self.kind.forward.tiebreak(param, source);
        self
    }

    /// Marks the forward boundary inclusive, deduplicating on `identity`.
    pub fn inclusive(mut self, identity: &str) -> Self {
        self.kind.forward = self.kind.forward.inclusive(identity);
        self
    }

    pub fn more_when(mut self, signal: MoreSignal) -> Self {
        self.kind.more = Some(signal);
        self
    }

    pub fn resumable(mut self) -> Self {
        self.kind.resumable = true;
        self
    }
}

impl NextLinkBuilder {
    pub fn prev(mut self, prev: LinkSource) -> Self {
        self.kind.prev = Some(prev);
        self
    }
}

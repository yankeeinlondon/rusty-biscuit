//! Throwaway prototype of a pagination model for `schematic-define`.
//!
//! See `../design.md`. A [`Collection`] marks an array-bearing response and
//! selects its items; its [`Pagination`] is `Inherit`, `Unpaginated`, or
//! `Explicit(Strategy)`. A [`Strategy`] wraps the one enumeration,
//! [`PageStrategy`] (how the next request is built), and adds the
//! dimensions every kind shares: constants, totals, terminal values,
//! follow-up request shape, server-side session lifecycle, and partitioning.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

/// A path of object keys into a JSON document. `parse` splits on `.`; use
/// [`JsonPath::segments`] for keys containing a dot (`@odata.nextLink`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JsonPath(Vec<String>);

impl JsonPath {
    pub fn root() -> Self {
        Self(Vec::new())
    }

    pub fn parse(dotted: &str) -> Self {
        Self(
            dotted
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_owned)
                .collect(),
        )
    }

    pub fn segments<I, S>(segments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self(segments.into_iter().map(Into::into).collect())
    }

    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    pub fn as_segments(&self) -> &[String] {
        &self.0
    }

    pub fn lookup<'a>(&self, value: &'a Value) -> Option<&'a Value> {
        self.0.iter().try_fold(value, |node, key| node.get(key))
    }
}

impl From<&str> for JsonPath {
    fn from(dotted: &str) -> Self {
        Self::parse(dotted)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    Key(String),
    /// Fan out over every element of an array (`[*]`).
    Each,
}

/// Selects the items of a page with any number of array fan-outs.
///
/// The element reached by the *last* `[*]` is the **record**; whatever
/// follows is a projection from record to the item handed to the caller.
/// Item-derived cursors read from the record, so Elasticsearch's `sort`
/// (on the hit) and Relay's `cursor` (on the edge) work while the caller
/// sees `_source` / `node`.
///
/// `parse("data")` means `data[*]`; `parse("")` is a root array;
/// `parse("responses[*].hits.hits[*]._source")` flattens two levels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemSelector {
    pub record: Vec<Step>,
    pub projection: JsonPath,
}

impl Default for ItemSelector {
    fn default() -> Self {
        Self::root()
    }
}

impl ItemSelector {
    pub fn root() -> Self {
        Self {
            record: vec![Step::Each],
            projection: JsonPath::root(),
        }
    }

    pub fn parse(expression: &str) -> Self {
        let mut steps = Vec::new();
        for segment in expression.split('.').filter(|segment| !segment.is_empty()) {
            let (name, each) = match segment.strip_suffix("[*]") {
                Some(name) => (name, true),
                None => (segment, false),
            };
            if !name.is_empty() {
                steps.push(Step::Key(name.to_owned()));
            }
            if each {
                steps.push(Step::Each);
            }
        }
        match steps.iter().rposition(|step| *step == Step::Each) {
            None => {
                steps.push(Step::Each);
                Self {
                    record: steps,
                    projection: JsonPath::root(),
                }
            }
            Some(last_each) => {
                let projected = steps.split_off(last_each + 1);
                let keys = projected.into_iter().filter_map(|step| match step {
                    Step::Key(key) => Some(key),
                    Step::Each => None,
                });
                Self {
                    record: steps,
                    projection: JsonPath::segments(keys),
                }
            }
        }
    }

    /// Recognizes a root array from a `Vec<T>` response type name.
    pub fn infer_root(response_type: &str) -> Option<Self> {
        let trimmed = response_type.trim();
        (trimmed.starts_with("Vec<") && trimmed.ends_with('>')).then(Self::root)
    }

    pub fn fan_outs(&self) -> usize {
        self.record.iter().filter(|step| **step == Step::Each).count()
    }

    pub fn is_root_array(&self) -> bool {
        self.record == [Step::Each] && self.projection.is_root()
    }

    pub fn records<'a>(&self, page: &'a Value) -> Vec<&'a Value> {
        self.record.iter().fold(vec![page], |nodes, step| match step {
            Step::Key(key) => nodes.into_iter().filter_map(|node| node.get(key)).collect(),
            Step::Each => nodes
                .into_iter()
                .filter_map(Value::as_array)
                .flatten()
                .collect(),
        })
    }

    pub fn items<'a>(&self, page: &'a Value) -> Vec<&'a Value> {
        self.records(page)
            .into_iter()
            .filter_map(|record| self.projection.lookup(record))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Request and response locations
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "in", content = "name", rename_all = "snake_case")]
#[non_exhaustive]
pub enum InputParam {
    Query(String),
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
    pub fn body(path: &str) -> Self {
        Self::Body(JsonPath::parse(path))
    }
    pub fn display_name(&self) -> String {
        match self {
            Self::Query(name) | Self::Header(name) => name.clone(),
            Self::Body(path) => path.as_segments().join("."),
        }
    }
    pub fn is_body(&self) -> bool {
        matches!(self, Self::Body(_))
    }
}

impl From<&str> for InputParam {
    fn from(name: &str) -> Self {
        Self::query(name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "from", content = "at", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ValueSource {
    Body(JsonPath),
    Header(String),
}

impl ValueSource {
    pub fn body(path: &str) -> Self {
        Self::Body(JsonPath::parse(path))
    }
    pub fn header(name: &str) -> Self {
        Self::Header(name.to_owned())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
}

/// A request input whose value the strategy fixes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Polarity {
    TrueMeansMore,
    TrueMeansLast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CountUnit {
    Items,
    Pages,
}

/// The caller-settable page size. `server_default` is documentation only.
/// `adjustable` lets the caller change it between pages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageSize {
    pub param: InputParam,
    pub server_default: Option<u32>,
    pub min: u32,
    pub max: Option<u32>,
    #[serde(default)]
    pub adjustable: bool,
}

impl PageSize {
    pub fn new(param: impl Into<InputParam>) -> Self {
        Self {
            param: param.into(),
            server_default: None,
            min: 1,
            max: None,
            adjustable: false,
        }
    }
    pub fn server_default(mut self, value: u32) -> Self {
        self.server_default = Some(value);
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "signal", rename_all = "snake_case")]
#[non_exhaustive]
pub enum MoreSignal {
    Flag { source: ValueSource, polarity: Polarity },
    Present { source: ValueSource },
    LinkRel { rel: String },
    Total { source: ValueSource, unit: CountUnit },
    NonEmptyPage,
    FullPage,
}

impl MoreSignal {
    pub fn has_more(path: &str) -> Self {
        Self::Flag {
            source: ValueSource::body(path),
            polarity: Polarity::TrueMeansMore,
        }
    }
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

/// Where a cursor value comes from. `LastItem`/`FirstItem` paths are
/// relative to the *record* (see [`ItemSelector`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "from", rename_all = "snake_case")]
#[non_exhaustive]
pub enum CursorSource {
    Field { source: ValueSource },
    LastItem { path: JsonPath },
    FirstItem { path: JsonPath },
}

impl CursorSource {
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
    pub fn last_item(path: &str) -> Self {
        Self::LastItem {
            path: JsonPath::parse(path),
        }
    }
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
pub struct CursorPart {
    pub param: InputParam,
    pub source: CursorSource,
}

/// Whether the server's position predicate includes the cursor position.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "boundary", rename_all = "snake_case")]
pub enum Boundary {
    /// `after`/`starting_after`/`search_after`: the boundary row is excluded.
    #[default]
    Exclusive,
    /// `since >= t`: the boundary rows come back; the client drops records
    /// whose `identity` it already delivered at the same key.
    Inclusive { identity: JsonPath },
}

/// A cursor direction: one or more parts (a sort key plus tiebreakers) sent
/// together, and the boundary semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CursorLeg {
    pub parts: Vec<CursorPart>,
    pub boundary: Boundary,
}

impl CursorLeg {
    pub fn single(param: impl Into<InputParam>, source: CursorSource) -> Self {
        Self {
            parts: vec![CursorPart {
                param: param.into(),
                source,
            }],
            boundary: Boundary::Exclusive,
        }
    }
    pub fn tiebreak(mut self, param: impl Into<InputParam>, source: CursorSource) -> Self {
        self.parts.push(CursorPart {
            param: param.into(),
            source,
        });
        self
    }
    pub fn inclusive(mut self, identity: &str) -> Self {
        self.boundary = Boundary::Inclusive {
            identity: JsonPath::parse(identity),
        };
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "from", rename_all = "snake_case")]
#[non_exhaustive]
pub enum LinkSource {
    LinkHeader { rel: String },
    Field { source: ValueSource },
}

impl LinkSource {
    pub fn link_header(rel: &str) -> Self {
        Self::LinkHeader { rel: rel.to_owned() }
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "trust", content = "origins", rename_all = "snake_case")]
#[non_exhaustive]
pub enum LinkTrust {
    #[default]
    SameOrigin,
    AllowOrigins(Vec<String>),
}

// ---------------------------------------------------------------------------
// The one enumeration
// ---------------------------------------------------------------------------

/// How the next request is built.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum PageStrategy {
    /// Increment a page index.
    PageNumber {
        page: InputParam,
        base: IndexBase,
        size: Option<PageSize>,
        more: MoreSignal,
    },
    /// Advance an item index by the number of records received.
    Offset {
        offset: InputParam,
        base: IndexBase,
        size: Option<PageSize>,
        more: MoreSignal,
    },
    /// Copy one or more values from the response into request inputs.
    Cursor {
        forward: CursorLeg,
        backward: Option<CursorLeg>,
        size: Option<PageSize>,
        more: Option<MoreSignal>,
        resumable: bool,
    },
    /// Request a server-provided URL.
    NextLink {
        next: LinkSource,
        prev: Option<LinkSource>,
        size: Option<PageSize>,
        trust: LinkTrust,
    },
}

impl PageStrategy {
    pub fn size(&self) -> Option<&PageSize> {
        match self {
            Self::PageNumber { size, .. }
            | Self::Offset { size, .. }
            | Self::Cursor { size, .. }
            | Self::NextLink { size, .. } => size.as_ref(),
        }
    }

    fn size_mut(&mut self) -> &mut Option<PageSize> {
        match self {
            Self::PageNumber { size, .. }
            | Self::Offset { size, .. }
            | Self::Cursor { size, .. }
            | Self::NextLink { size, .. } => size,
        }
    }

    fn legs(&self) -> Vec<&CursorLeg> {
        match self {
            Self::Cursor {
                forward, backward, ..
            } => std::iter::once(forward).chain(backward.iter()).collect(),
            _ => Vec::new(),
        }
    }

    fn more(&self) -> Option<&MoreSignal> {
        match self {
            Self::PageNumber { more, .. } | Self::Offset { more, .. } => Some(more),
            Self::Cursor { more, .. } => more.as_ref(),
            Self::NextLink { .. } => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Shared dimensions: follow-up shape, session lifecycle, partitioning
// ---------------------------------------------------------------------------

/// Which of the first request's ordinary inputs (query params and top-level
/// body fields) are re-sent on follow-up requests. Continuation inputs and
/// session inputs are always sent; this governs everything else.
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
pub struct FollowUp {
    /// `None` keeps the endpoint's method.
    pub method: Option<Method>,
    /// `None` keeps the endpoint's path. Must be `None` for `NextLink`.
    pub path: Option<String>,
    pub carry: Carry,
}

impl FollowUp {
    pub fn carry(carry: Carry) -> Self {
        Self {
            method: None,
            path: None,
            carry,
        }
    }
    pub fn to(method: Method, path: &str) -> Self {
        Self {
            method: Some(method),
            path: Some(path.to_owned()),
            carry: Carry::Nothing,
        }
    }
    pub fn method(mut self, method: Method) -> Self {
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
pub struct SessionStep {
    pub method: Method,
    /// May use only the endpoint's own path parameters.
    pub path: String,
    /// Where the session handle goes on this request.
    pub handle: Option<InputParam>,
    pub constants: Vec<FixedParam>,
}

impl SessionStep {
    pub fn new(method: Method, path: &str) -> Self {
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "open", rename_all = "snake_case")]
pub enum SessionOpen {
    /// The first page request opens the session (scroll, cursors declared
    /// in a query).
    FirstPage,
    /// A separate request opens it and returns the handle (PIT).
    Request { step: SessionStep, handle: ValueSource },
}

/// Keeps a session alive by sending a duration. `on_open` goes on the
/// opening request; `on_page` goes on every page request that is not the
/// opening one. In-band renewal: each page request renews the lease.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub open: SessionOpen,
    /// Where page requests go while the session is open (PIT pages go to
    /// `/_search`, not the endpoint path). `None` keeps the endpoint.
    pub pages: Option<(Method, String)>,
    /// Updated handle on each page (latest wins).
    pub handle_from_page: Option<ValueSource>,
    /// Where page requests carry the handle; `None` when the continuation
    /// itself carries it (scroll id as cursor, locator in a next link).
    pub send_handle_as: Option<InputParam>,
    pub keep_alive: Option<KeepAlive>,
    /// Out-of-band renewal, exposed as `renew()`.
    pub renew: Option<SessionStep>,
    /// Called when iteration ends, on error, and from `close()`.
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
    pub fn pages_at(mut self, method: Method, path: &str) -> Self {
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

/// Disjoint slices iterated independently and in parallel (ES sliced
/// scroll/PIT). Each pager sends its slice index and the slice count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Partition {
    pub index: InputParam,
    pub count: InputParam,
    pub max: Option<u32>,
}

// ---------------------------------------------------------------------------
// Strategy
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Strategy {
    pub kind: PageStrategy,
    #[serde(default)]
    pub constants: Vec<FixedParam>,
    #[serde(default)]
    pub total: Option<ValueSource>,
    /// Continuation values that mean "none", beyond missing/null/empty
    /// (Salesforce Bulk 2.0 sends the string `null`).
    #[serde(default)]
    pub terminal_values: Vec<String>,
    /// `None`: carry everything (`NextLink`: nothing, URL verbatim, GET).
    #[serde(default)]
    pub follow_up: Option<FollowUp>,
    #[serde(default)]
    pub session: Option<Session>,
    #[serde(default)]
    pub partition: Option<Partition>,
}

impl Strategy {
    pub fn page_number(page: impl Into<InputParam>, more: MoreSignal) -> PageNumberBuilder {
        PageNumberBuilder {
            page: page.into(),
            base: IndexBase::OneBased,
            size: None,
            more,
        }
    }

    pub fn offset(offset: impl Into<InputParam>, more: MoreSignal) -> OffsetBuilder {
        OffsetBuilder {
            offset: offset.into(),
            base: IndexBase::ZeroBased,
            size: None,
            more,
        }
    }

    pub fn cursor(param: impl Into<InputParam>, source: CursorSource) -> CursorBuilder {
        Self::cursor_leg(CursorLeg::single(param, source))
    }

    pub fn cursor_leg(forward: CursorLeg) -> CursorBuilder {
        CursorBuilder {
            forward,
            backward: None,
            size: None,
            more: None,
            resumable: false,
        }
    }

    pub fn next_link(next: LinkSource) -> NextLinkBuilder {
        NextLinkBuilder {
            next,
            prev: None,
            size: None,
            trust: LinkTrust::SameOrigin,
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
    pub fn with_page_size(mut self, size: PageSize) -> Self {
        *self.kind.size_mut() = Some(size);
        self
    }

    /// The carry rule actually applied.
    pub fn effective_carry(&self) -> Carry {
        match (&self.follow_up, &self.kind) {
            (Some(follow_up), _) => follow_up.carry.clone(),
            (None, PageStrategy::NextLink { .. }) => Carry::Nothing,
            (None, _) => Carry::All,
        }
    }

    /// Whether `value` (already read from the response) means "no
    /// continuation".
    pub fn is_terminal(&self, value: Option<&Value>) -> bool {
        match value {
            None | Some(Value::Null) => true,
            Some(Value::String(text)) => {
                text.is_empty() || self.terminal_values.iter().any(|terminal| terminal == text)
            }
            Some(_) => false,
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

macro_rules! into_strategy {
    ($builder:ty) => {
        impl From<$builder> for Strategy {
            fn from(builder: $builder) -> Self {
                builder.build()
            }
        }
        impl $builder {
            pub fn page_size(mut self, size: PageSize) -> Self {
                self.size = Some(size);
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
    };
}

#[derive(Debug, Clone)]
pub struct PageNumberBuilder {
    page: InputParam,
    base: IndexBase,
    size: Option<PageSize>,
    more: MoreSignal,
}

impl PageNumberBuilder {
    pub fn zero_based(mut self) -> Self {
        self.base = IndexBase::ZeroBased;
        self
    }
    pub fn build(self) -> Strategy {
        Strategy::from_kind(PageStrategy::PageNumber {
            page: self.page,
            base: self.base,
            size: self.size,
            more: self.more,
        })
    }
}
into_strategy!(PageNumberBuilder);

#[derive(Debug, Clone)]
pub struct OffsetBuilder {
    offset: InputParam,
    base: IndexBase,
    size: Option<PageSize>,
    more: MoreSignal,
}

impl OffsetBuilder {
    pub fn one_based(mut self) -> Self {
        self.base = IndexBase::OneBased;
        self
    }
    pub fn build(self) -> Strategy {
        Strategy::from_kind(PageStrategy::Offset {
            offset: self.offset,
            base: self.base,
            size: self.size,
            more: self.more,
        })
    }
}
into_strategy!(OffsetBuilder);

#[derive(Debug, Clone)]
pub struct CursorBuilder {
    forward: CursorLeg,
    backward: Option<CursorLeg>,
    size: Option<PageSize>,
    more: Option<MoreSignal>,
    resumable: bool,
}

impl CursorBuilder {
    pub fn backward(self, param: impl Into<InputParam>, source: CursorSource) -> Self {
        self.backward_leg(CursorLeg::single(param, source))
    }
    pub fn backward_leg(mut self, leg: CursorLeg) -> Self {
        self.backward = Some(leg);
        self
    }
    /// Adds a tiebreaker part to the forward cursor.
    pub fn tiebreak(mut self, param: impl Into<InputParam>, source: CursorSource) -> Self {
        self.forward = self.forward.tiebreak(param, source);
        self
    }
    /// Marks the forward boundary inclusive, deduplicating on `identity`.
    pub fn inclusive(mut self, identity: &str) -> Self {
        self.forward = self.forward.inclusive(identity);
        self
    }
    pub fn more_when(mut self, signal: MoreSignal) -> Self {
        self.more = Some(signal);
        self
    }
    pub fn resumable(mut self) -> Self {
        self.resumable = true;
        self
    }
    pub fn build(self) -> Strategy {
        Strategy::from_kind(PageStrategy::Cursor {
            forward: self.forward,
            backward: self.backward,
            size: self.size,
            more: self.more,
            resumable: self.resumable,
        })
    }
}
into_strategy!(CursorBuilder);

#[derive(Debug, Clone)]
pub struct NextLinkBuilder {
    next: LinkSource,
    prev: Option<LinkSource>,
    size: Option<PageSize>,
    trust: LinkTrust,
}

impl NextLinkBuilder {
    pub fn prev(mut self, prev: LinkSource) -> Self {
        self.prev = Some(prev);
        self
    }
    pub fn trust(mut self, trust: LinkTrust) -> Self {
        self.trust = trust;
        self
    }
    pub fn build(self) -> Strategy {
        Strategy::from_kind(PageStrategy::NextLink {
            next: self.next,
            prev: self.prev,
            size: self.size,
            trust: self.trust,
        })
    }
}
into_strategy!(NextLinkBuilder);

// ---------------------------------------------------------------------------
// Collections and policy
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "policy", content = "strategy", rename_all = "snake_case")]
pub enum Pagination {
    #[default]
    Inherit,
    Unpaginated,
    Explicit(Strategy),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Alternate {
    pub label: String,
    pub strategy: Strategy,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Collection {
    pub items: ItemSelector,
    pub pagination: Pagination,
    pub alternates: Vec<Alternate>,
}

impl Collection {
    pub fn root() -> Self {
        Self::default()
    }
    /// `at("data")`, `at("edges[*].node")`, `at("data.children[*].data")`.
    pub fn at(expression: &str) -> Self {
        Self {
            items: ItemSelector::parse(expression),
            ..Self::default()
        }
    }
    pub fn paginate(mut self, strategy: impl Into<Strategy>) -> Self {
        self.pagination = Pagination::Explicit(strategy.into());
        self
    }
    pub fn unpaginated(mut self) -> Self {
        self.pagination = Pagination::Unpaginated;
        self
    }
    pub fn or_paginate(mut self, label: &str, strategy: impl Into<Strategy>) -> Self {
        self.alternates.push(Alternate {
            label: label.to_owned(),
            strategy: strategy.into(),
        });
        self
    }
}

/// Stand-in for `schematic_define::Endpoint`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointDef {
    pub id: String,
    pub method: Method,
    pub path: String,
    pub response_type: String,
    pub query_params: Vec<String>,
    pub collection: Option<Collection>,
}

impl EndpointDef {
    pub fn get(id: &str, path: &str, response_type: &str) -> Self {
        Self {
            id: id.to_owned(),
            method: Method::Get,
            path: path.to_owned(),
            response_type: response_type.to_owned(),
            query_params: Vec::new(),
            collection: None,
        }
    }
    pub fn post(id: &str, path: &str, response_type: &str) -> Self {
        Self {
            method: Method::Post,
            ..Self::get(id, path, response_type)
        }
    }
    pub fn query(mut self, names: &[&str]) -> Self {
        self.query_params
            .extend(names.iter().map(|name| (*name).to_owned()));
        self
    }
    pub fn collection(mut self, collection: Collection) -> Self {
        self.collection = Some(collection);
        self
    }
    pub fn path_params(&self) -> Vec<String> {
        path_params(&self.path)
    }
}

fn path_params(path: &str) -> Vec<String> {
    path.split('{')
        .skip(1)
        .filter_map(|rest| rest.split_once('}').map(|(name, _)| name.trim_start_matches('+').to_owned()))
        .collect()
}

/// Stand-in for `schematic_define::RestApi`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiDef {
    pub name: String,
    pub base_url: String,
    pub default_pagination: Option<Strategy>,
    pub endpoints: Vec<EndpointDef>,
}

impl ApiDef {
    pub fn new(name: &str, base_url: &str) -> Self {
        Self {
            name: name.to_owned(),
            base_url: base_url.to_owned(),
            default_pagination: None,
            endpoints: Vec::new(),
        }
    }
    pub fn default_pagination(mut self, strategy: impl Into<Strategy>) -> Self {
        self.default_pagination = Some(strategy.into());
        self
    }
    pub fn endpoint(mut self, endpoint: EndpointDef) -> Self {
        self.endpoints.push(endpoint);
        self
    }
    pub fn find(&self, id: &str) -> Option<&EndpointDef> {
        self.endpoints.iter().find(|endpoint| endpoint.id == id)
    }
    pub fn resolve(&self, id: &str) -> Option<Resolved> {
        self.find(id)
            .map(|endpoint| resolve(self.default_pagination.as_ref(), endpoint))
    }
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    ApiDefault,
    Endpoint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    NotACollection,
    Unpaginated {
        items: ItemSelector,
    },
    Paginated {
        items: ItemSelector,
        primary: Strategy,
        alternates: Vec<Alternate>,
        origin: Origin,
    },
}

pub fn resolve(api_default: Option<&Strategy>, endpoint: &EndpointDef) -> Resolved {
    let Some(collection) = &endpoint.collection else {
        return Resolved::NotACollection;
    };
    let items = collection.items.clone();
    let (primary, origin) = match (&collection.pagination, api_default) {
        (Pagination::Explicit(strategy), _) => (strategy.clone(), Origin::Endpoint),
        (Pagination::Unpaginated, _) | (Pagination::Inherit, None) => {
            return Resolved::Unpaginated { items };
        }
        (Pagination::Inherit, Some(strategy)) => (strategy.clone(), Origin::ApiDefault),
    };
    Resolved::Paginated {
        items,
        primary,
        alternates: collection.alternates.clone(),
        origin,
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PaginationError {
    CursorWithoutEndSignal,
    FullPageWithoutPageSize,
    FullPageOnCursor,
    PageCountOnNonPageNumber,
    InvalidPageSize {
        min: u32,
        server_default: Option<u32>,
        max: Option<u32>,
    },
    DuplicateInput(String),
    BodyInputOnGet(String),
    CollidesWithEndpointParam(String),
    AlternatesWithoutPrimary,
    AlternatesOnUnpaginated,
    DuplicateLabel(String),
    InvalidLabel(String),
    InvalidTrustedOrigin(String),
    // Composite / inclusive cursors
    EmptyCursorLeg,
    InclusiveBoundaryNeedsItemKey,
    // Page size changes
    AdjustablePageSizeOnPageNumber,
    AdjustableLinkSizeNotQuery,
    // Follow-up shape
    NextLinkWithFollowUpPath,
    NextLinkMethodUnspecified,
    UnknownCarryParam(String),
    UnknownPathParam(String),
    // Sessions
    SessionHandleUnreachable,
    SessionHandleNotSent,
    SessionStepNeedsHandle(String),
    KeepAliveWithoutInput,
    // Partitions
    InvalidPartitionMax,
}

fn page_size_errors(size: &PageSize, kind: &PageStrategy, errors: &mut Vec<PaginationError>) {
    let default_ok = size
        .server_default
        .is_none_or(|value| value >= size.min && size.max.is_none_or(|max| value <= max));
    let max_ok = size.max.is_none_or(|max| max >= size.min);
    if size.min == 0 || !default_ok || !max_ok {
        errors.push(PaginationError::InvalidPageSize {
            min: size.min,
            server_default: size.server_default,
            max: size.max,
        });
    }
    if size.adjustable {
        match kind {
            // Page k at a new size is not "the page after" page k-1.
            PageStrategy::PageNumber { .. } => {
                errors.push(PaginationError::AdjustablePageSizeOnPageNumber);
            }
            // The client can only override a query parameter on a URL it
            // does not own.
            PageStrategy::NextLink { .. } if !matches!(size.param, InputParam::Query(_)) => {
                errors.push(PaginationError::AdjustableLinkSizeNotQuery);
            }
            _ => {}
        }
    }
}

/// Inputs that page requests write (duplicates among them are errors).
fn page_inputs(strategy: &Strategy) -> Vec<&InputParam> {
    let kind = &strategy.kind;
    let mut inputs: Vec<&InputParam> = Vec::new();
    match kind {
        PageStrategy::PageNumber { page, .. } => inputs.push(page),
        PageStrategy::Offset { offset, .. } => inputs.push(offset),
        PageStrategy::Cursor { .. } => {
            // Forward and backward legs are never sent together, but share
            // names only by mistake.
            for leg in kind.legs() {
                inputs.extend(leg.parts.iter().map(|part| &part.param));
            }
        }
        PageStrategy::NextLink { .. } => {}
    }
    if let Some(size) = kind.size() {
        inputs.push(&size.param);
    }
    inputs.extend(strategy.constants.iter().map(|fixed| &fixed.param));
    if let Some(partition) = &strategy.partition {
        inputs.push(&partition.index);
        inputs.push(&partition.count);
    }
    if let Some(session) = &strategy.session {
        inputs.extend(session.send_handle_as.iter());
        if let Some(keep_alive) = &session.keep_alive {
            inputs.extend(keep_alive.on_page.iter());
        }
    }
    inputs
}

pub fn validate_strategy(strategy: &Strategy) -> Vec<PaginationError> {
    let mut errors = Vec::new();
    let kind = &strategy.kind;
    let cursor_like = matches!(kind, PageStrategy::Cursor { .. } | PageStrategy::NextLink { .. });

    for leg in kind.legs() {
        if leg.parts.is_empty() {
            errors.push(PaginationError::EmptyCursorLeg);
        }
        if matches!(leg.boundary, Boundary::Inclusive { .. })
            && !leg.parts.iter().all(|part| part.source.is_item_derived())
        {
            errors.push(PaginationError::InclusiveBoundaryNeedsItemKey);
        }
    }

    if let PageStrategy::Cursor { forward, more, .. } = kind
        && !forward.parts.is_empty()
        && forward.parts.iter().all(|part| part.source.is_item_derived())
        && more.is_none()
    {
        errors.push(PaginationError::CursorWithoutEndSignal);
    }

    match kind.more() {
        Some(MoreSignal::FullPage) if cursor_like => errors.push(PaginationError::FullPageOnCursor),
        Some(MoreSignal::FullPage) if kind.size().is_none() => {
            errors.push(PaginationError::FullPageWithoutPageSize)
        }
        Some(MoreSignal::Total {
            unit: CountUnit::Pages,
            ..
        }) if !matches!(kind, PageStrategy::PageNumber { .. }) => {
            errors.push(PaginationError::PageCountOnNonPageNumber)
        }
        _ => {}
    }

    if let Some(size) = kind.size() {
        page_size_errors(size, kind, &mut errors);
    }

    let mut seen = std::collections::HashSet::new();
    for input in page_inputs(strategy) {
        if !seen.insert(input.clone()) {
            errors.push(PaginationError::DuplicateInput(input.display_name()));
        }
    }

    if let PageStrategy::NextLink { trust, .. } = kind {
        if let LinkTrust::AllowOrigins(origins) = trust {
            for origin in origins {
                if !matches!(parse_origin(origin), Some(parsed) if parsed.scheme == "https") {
                    errors.push(PaginationError::InvalidTrustedOrigin(origin.clone()));
                }
            }
        }
        if strategy.follow_up.as_ref().is_some_and(|f| f.path.is_some()) {
            errors.push(PaginationError::NextLinkWithFollowUpPath);
        }
    }

    if let Some(session) = &strategy.session {
        errors.extend(session_errors(session, kind));
    }

    if let Some(Partition { max: Some(0), .. }) = &strategy.partition {
        errors.push(PaginationError::InvalidPartitionMax);
    }

    errors
}

fn session_errors(session: &Session, kind: &PageStrategy) -> Vec<PaginationError> {
    let mut errors = Vec::new();
    let opened_with_handle = matches!(session.open, SessionOpen::Request { .. });
    if !opened_with_handle && session.handle_from_page.is_none() {
        errors.push(PaginationError::SessionHandleUnreachable);
    }
    if session.send_handle_as.is_none() {
        // Acceptable only if the continuation itself carries the handle.
        let carried_by_cursor = matches!(kind, PageStrategy::Cursor { forward, .. }
            if forward.parts.iter().any(|part| matches!(&part.source,
                CursorSource::Field { source } if Some(source) == session.handle_from_page.as_ref())));
        let carried_by_link = matches!(kind, PageStrategy::NextLink { .. });
        if !(carried_by_cursor || carried_by_link) {
            errors.push(PaginationError::SessionHandleNotSent);
        }
    }
    for (name, step) in [("renew", &session.renew), ("close", &session.close)] {
        if let Some(step) = step
            && step.handle.is_none()
        {
            errors.push(PaginationError::SessionStepNeedsHandle(name.to_owned()));
        }
    }
    if let Some(keep_alive) = &session.keep_alive
        && keep_alive.on_open.is_none()
        && keep_alive.on_page.is_none()
    {
        errors.push(PaginationError::KeepAliveWithoutInput);
    }
    errors
}

fn check_body_on_get(input: &InputParam, method: Method, errors: &mut Vec<PaginationError>) {
    if input.is_body() && method == Method::Get {
        errors.push(PaginationError::BodyInputOnGet(input.display_name()));
    }
}

fn endpoint_strategy_errors(endpoint: &EndpointDef, strategy: &Strategy) -> Vec<PaginationError> {
    let mut errors = validate_strategy(strategy);
    let known_path = endpoint.path_params();
    let check_path = |path: &str, errors: &mut Vec<PaginationError>| {
        for name in path_params(path) {
            if !known_path.contains(&name) {
                errors.push(PaginationError::UnknownPathParam(name));
            }
        }
    };

    let follow_up_method = strategy
        .follow_up
        .as_ref()
        .and_then(|follow_up| follow_up.method)
        .unwrap_or(match strategy.kind {
            PageStrategy::NextLink { .. } => Method::Get,
            _ => endpoint.method,
        });
    let session_pages = strategy.session.as_ref().and_then(|session| session.pages.as_ref());
    let first_method = session_pages.map_or(endpoint.method, |(method, _)| *method);

    if matches!(strategy.kind, PageStrategy::NextLink { .. })
        && endpoint.method != Method::Get
        && strategy.follow_up.as_ref().and_then(|f| f.method).is_none()
    {
        errors.push(PaginationError::NextLinkMethodUnspecified);
    }
    if let Some(path) = strategy.follow_up.as_ref().and_then(|f| f.path.as_deref()) {
        check_path(path, &mut errors);
    }

    // First-request inputs.
    let mut first_inputs: Vec<&InputParam> = Vec::new();
    match &strategy.kind {
        PageStrategy::PageNumber { page, .. } => first_inputs.push(page),
        PageStrategy::Offset { offset, .. } => first_inputs.push(offset),
        _ => {}
    }
    first_inputs.extend(strategy.kind.size().map(|size| &size.param));
    first_inputs.extend(strategy.constants.iter().map(|fixed| &fixed.param));
    if let Some(partition) = &strategy.partition {
        first_inputs.extend([&partition.index, &partition.count]);
    }
    for input in &first_inputs {
        check_body_on_get(input, first_method, &mut errors);
    }

    // Follow-up-only inputs (cursor parts).
    for leg in strategy.kind.legs() {
        for part in &leg.parts {
            check_body_on_get(&part.param, follow_up_method, &mut errors);
        }
    }

    // Session steps.
    if let Some(session) = &strategy.session {
        let mut steps: Vec<&SessionStep> = Vec::new();
        if let SessionOpen::Request { step, .. } = &session.open {
            steps.push(step);
        }
        steps.extend(session.renew.iter());
        steps.extend(session.close.iter());
        for step in steps {
            check_path(&step.path, &mut errors);
            for input in step.handle.iter().chain(step.constants.iter().map(|fixed| &fixed.param)) {
                check_body_on_get(input, step.method, &mut errors);
            }
        }
        if let Some((_, path)) = &session.pages {
            check_path(path, &mut errors);
        }
        if let Some(handle) = &session.send_handle_as {
            check_body_on_get(handle, first_method, &mut errors);
        }
        if let Some(keep_alive) = &session.keep_alive {
            let open_method = match &session.open {
                SessionOpen::Request { step, .. } => step.method,
                SessionOpen::FirstPage => endpoint.method,
            };
            if let Some(input) = &keep_alive.on_open {
                check_body_on_get(input, open_method, &mut errors);
            }
            if let Some(input) = &keep_alive.on_page {
                check_body_on_get(input, follow_up_method, &mut errors);
            }
        }
    }

    // Collisions with ordinary params.
    for input in page_inputs(strategy) {
        if let InputParam::Query(name) = input
            && endpoint.query_params.iter().any(|declared| declared == name)
        {
            errors.push(PaginationError::CollidesWithEndpointParam(name.clone()));
        }
    }

    // Carry names must be ordinary params or the page size. Body field
    // names cannot be checked here (no body schema), so only GET is checked.
    if endpoint.method == Method::Get
        && let Carry::Only(names) | Carry::AllExcept(names) = strategy.effective_carry()
    {
        let size_name = strategy.kind.size().map(|size| size.param.display_name());
        for name in names {
            let known = endpoint.query_params.contains(&name) || size_name.as_deref() == Some(name.as_str());
            if !known {
                errors.push(PaginationError::UnknownCarryParam(name));
            }
        }
    }
    errors
}

pub fn validate_endpoint(api_default: Option<&Strategy>, endpoint: &EndpointDef) -> Vec<PaginationError> {
    let mut errors = Vec::new();
    let Some(collection) = &endpoint.collection else {
        return errors;
    };
    match (&collection.pagination, api_default) {
        (Pagination::Inherit, None) if !collection.alternates.is_empty() => {
            errors.push(PaginationError::AlternatesWithoutPrimary);
        }
        (Pagination::Unpaginated, _) if !collection.alternates.is_empty() => {
            errors.push(PaginationError::AlternatesOnUnpaginated);
        }
        _ => {}
    }
    let mut labels = std::collections::HashSet::new();
    for alternate in &collection.alternates {
        let label_ok = !alternate.label.is_empty()
            && alternate
                .label
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_');
        if !label_ok {
            errors.push(PaginationError::InvalidLabel(alternate.label.clone()));
        }
        if !labels.insert(alternate.label.as_str()) {
            errors.push(PaginationError::DuplicateLabel(alternate.label.clone()));
        }
    }
    let Resolved::Paginated {
        primary,
        alternates,
        ..
    } = resolve(api_default, endpoint)
    else {
        return errors;
    };
    for strategy in std::iter::once(&primary).chain(alternates.iter().map(|alt| &alt.strategy)) {
        errors.extend(endpoint_strategy_errors(endpoint, strategy));
    }
    errors
}

pub fn validate_api(api: &ApiDef) -> Vec<(String, PaginationError)> {
    let mut errors: Vec<(String, PaginationError)> = api
        .default_pagination
        .iter()
        .flat_map(validate_strategy)
        .map(|error| ("<api default>".to_owned(), error))
        .collect();
    for endpoint in &api.endpoints {
        errors.extend(
            validate_endpoint(api.default_pagination.as_ref(), endpoint)
                .into_iter()
                .map(|error| (endpoint.id.clone(), error)),
        );
    }
    errors
}

// ---------------------------------------------------------------------------
// Caller-visible surface (principle 4)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Knob {
    /// `.page_size(n)` before iterating.
    PageSize {
        param: String,
        min: u32,
        max: Option<u32>,
        server_default: Option<u32>,
    },
    /// `pages.set_page_size(n)` between pages.
    AdjustPageSize,
    /// `.page(n)`.
    StartPage { base: IndexBase },
    /// `.offset(n)`.
    StartOffset { base: IndexBase },
    /// `.after(..)` / `.before(..)`; `parts` > 1 means a tuple argument.
    ResumeCursor { direction: Direction, parts: usize },
    Next,
    Prev,
    Total,
    /// `.keep_alive(duration)`.
    KeepAlive { default: String },
    /// `pages.renew()`.
    Renew,
    /// `pages.close()` (also automatic at the end and on error).
    Close,
    /// `.partitioned(n) -> Vec<Pager>`.
    Partitions { max: Option<u32> },
    /// `.using_<label>()`.
    Alternate { label: String, knobs: Vec<Knob> },
}

pub fn strategy_knobs(strategy: &Strategy) -> Vec<Knob> {
    let mut knobs = Vec::new();
    match &strategy.kind {
        PageStrategy::PageNumber { base, .. } => knobs.push(Knob::StartPage { base: *base }),
        PageStrategy::Offset { base, .. } => knobs.push(Knob::StartOffset { base: *base }),
        PageStrategy::Cursor {
            forward,
            backward,
            resumable: true,
            ..
        } => {
            knobs.push(Knob::ResumeCursor {
                direction: Direction::Forward,
                parts: forward.parts.len(),
            });
            if let Some(leg) = backward {
                knobs.push(Knob::ResumeCursor {
                    direction: Direction::Backward,
                    parts: leg.parts.len(),
                });
            }
        }
        _ => {}
    }
    if let Some(size) = strategy.kind.size() {
        knobs.push(Knob::PageSize {
            param: size.param.display_name(),
            min: size.min,
            max: size.max,
            server_default: size.server_default,
        });
        if size.adjustable {
            knobs.push(Knob::AdjustPageSize);
        }
    }
    if let Some(keep_alive) = strategy.session.as_ref().and_then(|s| s.keep_alive.as_ref())
        && keep_alive.adjustable
    {
        knobs.push(Knob::KeepAlive {
            default: keep_alive.default.clone(),
        });
    }
    if let Some(partition) = &strategy.partition {
        knobs.push(Knob::Partitions { max: partition.max });
    }
    knobs.push(Knob::Next);
    let has_prev = match &strategy.kind {
        PageStrategy::Cursor { backward, .. } => backward.is_some(),
        PageStrategy::NextLink { prev, .. } => prev.is_some(),
        _ => false,
    };
    if has_prev {
        knobs.push(Knob::Prev);
    }
    if strategy.total.is_some() || matches!(strategy.kind.more(), Some(MoreSignal::Total { .. })) {
        knobs.push(Knob::Total);
    }
    if let Some(session) = &strategy.session {
        if session.renew.is_some() {
            knobs.push(Knob::Renew);
        }
        if session.close.is_some() {
            knobs.push(Knob::Close);
        }
    }
    knobs
}

pub fn caller_knobs(resolved: &Resolved) -> Vec<Knob> {
    let Resolved::Paginated {
        primary,
        alternates,
        ..
    } = resolved
    else {
        return Vec::new();
    };
    let mut knobs = strategy_knobs(primary);
    knobs.extend(alternates.iter().map(|alternate| Knob::Alternate {
        label: alternate.label.clone(),
        knobs: strategy_knobs(&alternate.strategy),
    }));
    knobs
}

// ---------------------------------------------------------------------------
// Runtime pieces worth proving now
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundaryError {
    /// A whole page shared the boundary key and was already delivered: the
    /// page size is too small to get past the tie.
    NoProgress,
    MissingKey,
}

/// Client-side deduplication for an inclusive composite cursor.
///
/// Remembers the identities delivered at the last key; on the next page it
/// drops records with that key whose identity was already delivered.
#[derive(Debug, Clone)]
pub struct BoundaryTracker {
    key_paths: Vec<JsonPath>,
    identity: JsonPath,
    last_key: Option<Vec<Value>>,
    delivered_at_key: Vec<Value>,
}

impl BoundaryTracker {
    pub fn new(leg: &CursorLeg) -> Option<Self> {
        let Boundary::Inclusive { identity } = &leg.boundary else {
            return None;
        };
        let key_paths = leg
            .parts
            .iter()
            .map(|part| match &part.source {
                CursorSource::LastItem { path } | CursorSource::FirstItem { path } => Some(path.clone()),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            key_paths,
            identity: identity.clone(),
            last_key: None,
            delivered_at_key: Vec::new(),
        })
    }

    fn key_of(&self, record: &Value) -> Result<Vec<Value>, BoundaryError> {
        self.key_paths
            .iter()
            .map(|path| path.lookup(record).cloned().ok_or(BoundaryError::MissingKey))
            .collect()
    }

    /// Filters `records` and returns the ones to deliver, plus the cursor
    /// values for the next request.
    pub fn accept<'a>(&mut self, records: Vec<&'a Value>) -> Result<(Vec<&'a Value>, Option<Vec<Value>>), BoundaryError> {
        let mut kept = Vec::new();
        for record in &records {
            let key = self.key_of(record)?;
            let identity = self.identity.lookup(record).cloned().unwrap_or(Value::Null);
            let duplicate = self.last_key.as_ref() == Some(&key) && self.delivered_at_key.contains(&identity);
            if !duplicate {
                kept.push(*record);
            }
        }
        if !records.is_empty() && kept.is_empty() {
            return Err(BoundaryError::NoProgress);
        }
        let Some(last) = records.last() else {
            return Ok((kept, None));
        };
        let new_key = self.key_of(last)?;
        if self.last_key.as_ref() != Some(&new_key) {
            self.delivered_at_key.clear();
        }
        for record in &records {
            if self.key_of(record)? == new_key
                && let Some(identity) = self.identity.lookup(record)
                && !self.delivered_at_key.contains(identity)
            {
                self.delivered_at_key.push(identity.clone());
            }
        }
        self.last_key = Some(new_key.clone());
        Ok((kept, Some(new_key)))
    }
}

// ---------------------------------------------------------------------------
// Link trust
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedOrigin {
    scheme: String,
    host: String,
    port: u16,
}

fn parse_origin(url: &str) -> Option<ParsedOrigin> {
    let (scheme, rest) = url.split_once("://")?;
    let scheme = scheme.to_ascii_lowercase();
    let authority = rest.split(['/', '?', '#']).next()?;
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    let default_port = match scheme.as_str() {
        "https" => 443,
        "http" => 80,
        _ => return None,
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => {
            (host, port.parse().ok()?)
        }
        _ => (authority, default_port),
    };
    Some(ParsedOrigin {
        scheme,
        host: host.to_ascii_lowercase(),
        port,
    })
}

pub fn is_link_trusted(base_url: &str, link: &str, trust: &LinkTrust) -> bool {
    let Some(base) = parse_origin(base_url) else {
        return false;
    };
    if link.starts_with('/') && !link.starts_with("//") {
        return true;
    }
    let absolute = match link.strip_prefix("//") {
        Some(rest) => format!("{}://{rest}", base.scheme),
        None => link.to_owned(),
    };
    let Some(target) = parse_origin(&absolute) else {
        return false;
    };
    if target == base {
        return true;
    }
    match trust {
        LinkTrust::SameOrigin => false,
        LinkTrust::AllowOrigins(origins) => {
            target.scheme == "https"
                && origins
                    .iter()
                    .filter_map(|origin| parse_origin(origin))
                    .any(|allowed| allowed == target)
        }
    }
}

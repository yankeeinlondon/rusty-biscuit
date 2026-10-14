//! Paginated endpoint fixtures shaped like generator output, and the mock
//! helpers the pager suites share.
//!
//! Each fixture is a request type with an `EndpointSpec` impl, a `Paginated`
//! impl whose strategy and selector are `&'static` constants, and only the
//! capability markers its "definition" authors. Fixtures named after a
//! provider model that provider's pagination mechanism, not its real
//! definition.

use http::Method;
use schematic_runtime::pager::spec::{
    Boundary, Carry, CountUnit, CursorLeg, CursorPart, CursorSource, Fixed, FollowUp, IndexBase, Input, Kind,
    LinkSource, MoreSignal, PageSize, Polarity, Source, Step,
};
use schematic_runtime::pager::{
    AdjustablePageSize, CursorKind, HasPrev, HasTotal, ItemSelector, NextLinkKind, NoSession, OffsetKind,
    PageNumberKind, PageSizeKnob, Paginated, ResumeAfter, ResumeBefore, Strategy,
};
use schematic_runtime::{Client, EndpointSpec, InvalidArgument, Json, RequestParts};
use serde::Deserialize;
use serde_json::Value;
use wiremock::matchers::{header, query_param};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

pub use crate::support::{Item, ProviderError, TestApi, plain_client};

/// An item whose id is a string.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Named {
    pub id: String,
}

/// Builds a [`Strategy`] in a constant from its `Copy` dimensions; a
/// constant cannot drop the non-`Copy` fields struct-update syntax would.
#[derive(Clone, Copy)]
pub struct Def {
    kind: Kind,
    size: Option<PageSize>,
    total: Option<Source>,
    constants: &'static [Fixed],
    terminal: &'static [&'static str],
}

impl Def {
    /// A strategy of `kind` with every shared dimension unset.
    pub const fn new(kind: Kind) -> Self {
        Self {
            kind,
            size: None,
            total: None,
            constants: &[],
            terminal: &[],
        }
    }

    pub const fn size(mut self, size: PageSize) -> Self {
        self.size = Some(size);
        self
    }

    pub const fn total(mut self, source: Source) -> Self {
        self.total = Some(source);
        self
    }

    pub const fn constants(mut self, constants: &'static [Fixed]) -> Self {
        self.constants = constants;
        self
    }

    pub const fn terminal(mut self, values: &'static [&'static str]) -> Self {
        self.terminal = values;
        self
    }

    pub const fn done(self) -> Strategy {
        Strategy {
            kind: self.kind,
            size: self.size,
            constants: self.constants,
            total: self.total,
            terminal_values: self.terminal,
            follow_up: None,
            session: None,
            partition: None,
        }
    }
}

/// An optional page size at `param` with a minimum of one and no maximum.
pub const fn size(param: Input) -> PageSize {
    PageSize {
        param,
        server_default: None,
        sent_default: None,
        min: 1,
        max: None,
        adjustable: false,
    }
}

const fn has_more(path: &'static [&'static str]) -> MoreSignal {
    MoreSignal::Flag {
        source: Source::Body(path),
        polarity: Polarity::TrueMeansMore,
    }
}

const fn exclusive(parts: &'static [CursorPart]) -> CursorLeg {
    CursorLeg {
        parts,
        boundary: Boundary::Exclusive,
    }
}

const fn forward_cursor(leg: CursorLeg, more: Option<MoreSignal>, resumable: bool) -> Kind {
    Kind::Cursor {
        forward: leg,
        backward: None,
        more,
        resumable,
    }
}

/// Declares a GET fixture endpoint of [`TestApi`] at `$path` whose request
/// is its parts, paginated by `$strategy` with items at `$items`.
macro_rules! endpoint {
    ($name:ident, $kind:ty, $item:ty, $path:literal, $strategy:expr, $items:expr) => {
        #[derive(Debug, Clone)]
        pub struct $name(pub RequestParts);

        impl $name {
            #[allow(dead_code, reason = "a fixture with ordinary inputs is built by `with_query`")]
            pub fn get() -> Self {
                Self(RequestParts::new(Method::GET, $path))
            }

            /// The request with ordinary query inputs.
            #[allow(dead_code, reason = "only some fixtures take ordinary inputs")]
            pub fn with_query(query: &[(&str, &str)]) -> Self {
                let mut parts = RequestParts::new(Method::GET, $path);
                parts.query = query.iter().map(|(name, value)| ((*name).to_owned(), (*value).to_owned())).collect();
                Self(parts)
            }
        }

        impl EndpointSpec for $name {
            type Api = TestApi;
            type Success = Value;
            type Failure = ProviderError;
            type Decoder = Json<Value>;

            const ENDPOINT_ID: &'static str = stringify!($name);
            const METHOD: Method = Method::GET;

            fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
                Ok(self.0)
            }
        }

        impl Paginated for $name {
            type Item = $item;
            type Kind = $kind;
            type Session = NoSession;

            const STRATEGY: &'static Strategy = &$strategy;
            const ITEMS: &'static ItemSelector = &$items;
            const PATH: &'static str = $path;
        }
    };
}

const fn under(record: &'static [Step]) -> ItemSelector {
    ItemSelector { record, projection: &[] }
}

// ---------------------------------------------------------------------------
// Inventory mechanisms (R30)
// ---------------------------------------------------------------------------

endpoint!(
    PageLinkRel,
    PageNumberKind,
    Item,
    "/page-link",
    Def::new(Kind::PageNumber {
        page: Input::Query("page"),
        base: IndexBase::OneBased,
        more: MoreSignal::LinkRel { rel: "next" },
    })
    .size(PageSize { max: Some(100), ..size(Input::Query("per_page")) })
    .done(),
    ItemSelector::ROOT
);
impl PageSizeKnob for PageLinkRel {}

endpoint!(
    PageNextHeader,
    PageNumberKind,
    Item,
    "/page-next-header",
    Def::new(Kind::PageNumber {
        page: Input::Query("page"),
        base: IndexBase::OneBased,
        more: MoreSignal::Present { source: Source::Header("X-Next-Page") },
    })
    .size(PageSize { max: Some(100), ..size(Input::Query("per_page")) })
    .total(Source::Header("X-Total"))
    .done(),
    ItemSelector::ROOT
);
impl PageSizeKnob for PageNextHeader {}
impl HasTotal for PageNextHeader {}

endpoint!(
    KeysetLink,
    NextLinkKind,
    Item,
    "/keyset",
    Def::new(Kind::NextLink {
        next: LinkSource::LinkHeader { rel: "next" },
        prev: None,
    })
    .size(size(Input::Query("per_page")))
    .constants(&[
        Fixed { param: Input::Query("pagination"), value: "keyset" },
        Fixed { param: Input::Query("order_by"), value: "id" },
    ])
    .done(),
    ItemSelector::ROOT
);
impl PageSizeKnob for KeysetLink {}

endpoint!(
    PageLimitLinkTotal,
    PageNumberKind,
    Item,
    "/page-limit",
    Def::new(Kind::PageNumber {
        page: Input::Query("page"),
        base: IndexBase::OneBased,
        more: MoreSignal::LinkRel { rel: "next" },
    })
    .size(size(Input::Query("limit")))
    .total(Source::Header("X-Total-Count"))
    .done(),
    ItemSelector::ROOT
);
impl PageSizeKnob for PageLimitLinkTotal {}
impl HasTotal for PageLimitLinkTotal {}

endpoint!(
    BodyNextPrevious,
    NextLinkKind,
    Item,
    "/v2/repos/list",
    Def::new(Kind::NextLink {
        next: LinkSource::Field(Source::Body(&["next"])),
        prev: Some(LinkSource::Field(Source::Body(&["previous"]))),
    })
    .size(PageSize { max: Some(100), ..size(Input::Query("pagelen")) })
    .total(Source::Body(&["size"]))
    .done(),
    under(&[Step::Key("values"), Step::Each])
);
impl PageSizeKnob for BodyNextPrevious {}
impl HasPrev for BodyNextPrevious {}
impl HasTotal for BodyNextPrevious {}

endpoint!(
    LastIdHasMore,
    CursorKind,
    Named,
    "/last-id",
    Def::new(forward_cursor(
        exclusive(&[CursorPart {
            param: Input::Query("after"),
            source: CursorSource::Field(Source::Body(&["last_id"])),
        }]),
        Some(has_more(&["has_more"])),
        true,
    ))
    .size(PageSize { max: Some(100), ..size(Input::Query("limit")) })
    .done(),
    under(&[Step::Key("data"), Step::Each])
);
impl PageSizeKnob for LastIdHasMore {}
impl ResumeAfter for LastIdHasMore {
    type Cursor = String;
}

endpoint!(
    ItemIdHasMore,
    CursorKind,
    Named,
    "/item-id",
    Def::new(forward_cursor(
        exclusive(&[CursorPart {
            param: Input::Query("after"),
            source: CursorSource::LastItem(&["id"]),
        }]),
        Some(has_more(&["has_more"])),
        false,
    ))
    .size(size(Input::Query("limit")))
    .done(),
    under(&[Step::Key("data"), Step::Each])
);
impl PageSizeKnob for ItemIdHasMore {}

endpoint!(
    NextField,
    CursorKind,
    Item,
    "/next-field",
    Def::new(forward_cursor(
        exclusive(&[CursorPart {
            param: Input::Query("after"),
            source: CursorSource::Field(Source::Body(&["next"])),
        }]),
        None,
        true,
    ))
    .size(size(Input::Query("limit")))
    .done(),
    under(&[Step::Key("data"), Step::Each])
);
impl PageSizeKnob for NextField {}
impl ResumeAfter for NextField {
    type Cursor = String;
}

endpoint!(
    NextPageNumber,
    CursorKind,
    Item,
    "/next-page",
    Def::new(forward_cursor(
        exclusive(&[CursorPart {
            param: Input::Query("page"),
            source: CursorSource::Field(Source::Body(&["next_page"])),
        }]),
        None,
        false,
    ))
    .done(),
    under(&[Step::Key("data"), Step::Each])
);

endpoint!(
    Bidirectional,
    CursorKind,
    Named,
    "/bidirectional",
    Def::new(Kind::Cursor {
        forward: exclusive(&[CursorPart {
            param: Input::Query("after_id"),
            source: CursorSource::Field(Source::Body(&["last_id"])),
        }]),
        backward: Some(exclusive(&[CursorPart {
            param: Input::Query("before_id"),
            source: CursorSource::Field(Source::Body(&["first_id"])),
        }])),
        more: Some(has_more(&["has_more"])),
        resumable: true,
    })
    .size(PageSize { max: Some(1000), ..size(Input::Query("limit")) })
    .done(),
    under(&[Step::Key("data"), Step::Each])
);
impl PageSizeKnob for Bidirectional {}
impl ResumeAfter for Bidirectional {
    type Cursor = String;
}
impl ResumeBefore for Bidirectional {
    type Cursor = String;
}
impl HasPrev for Bidirectional {}

endpoint!(
    HistoryCursor,
    CursorKind,
    Named,
    "/history",
    Def::new(forward_cursor(
        exclusive(&[CursorPart {
            param: Input::Query("start_after_history_item_id"),
            source: CursorSource::Field(Source::Body(&["last_history_item_id"])),
        }]),
        Some(has_more(&["has_more"])),
        false,
    ))
    .size(size(Input::Query("page_size")))
    .done(),
    under(&[Step::Key("history"), Step::Each])
);
impl PageSizeKnob for HistoryCursor {}

endpoint!(
    PageToken,
    CursorKind,
    Named,
    "/voices",
    Def::new(forward_cursor(
        exclusive(&[CursorPart {
            param: Input::Query("next_page_token"),
            source: CursorSource::Field(Source::Body(&["next_page_token"])),
        }]),
        Some(has_more(&["has_more"])),
        false,
    ))
    .size(size(Input::Query("page_size")))
    .total(Source::Body(&["total_count"]))
    .done(),
    under(&[Step::Key("voices"), Step::Each])
);
impl PageSizeKnob for PageToken {}
impl HasTotal for PageToken {}

endpoint!(
    ZeroBasedPage,
    PageNumberKind,
    Item,
    "/shared-voices",
    Def::new(Kind::PageNumber {
        page: Input::Query("page"),
        base: IndexBase::ZeroBased,
        more: has_more(&["has_more"]),
    })
    .size(size(Input::Query("page_size")))
    .done(),
    under(&[Step::Key("voices"), Step::Each])
);
impl PageSizeKnob for ZeroBasedPage {}

endpoint!(
    NestedMeta,
    PageNumberKind,
    Item,
    "/nested-meta",
    Def::new(Kind::PageNumber {
        page: Input::Query("page"),
        base: IndexBase::OneBased,
        more: has_more(&["meta", "hasnext"]),
    })
    .size(size(Input::Query("limit")))
    .total(Source::Body(&["meta", "count"]))
    .done(),
    under(&[Step::Key("data"), Step::Each])
);
impl PageSizeKnob for NestedMeta {}
impl HasTotal for NestedMeta {}

endpoint!(
    LinkCursor,
    NextLinkKind,
    Item,
    "/link-cursor",
    Def::new(Kind::NextLink {
        next: LinkSource::LinkHeader { rel: "next" },
        prev: None,
    })
    .size(size(Input::Query("limit")))
    .done(),
    ItemSelector::ROOT
);
impl PageSizeKnob for LinkCursor {}

endpoint!(
    ZeroBasedNonEmpty,
    PageNumberKind,
    Item,
    "/zero-based",
    Def::new(Kind::PageNumber {
        page: Input::Query("p"),
        base: IndexBase::ZeroBased,
        more: MoreSignal::NonEmptyPage,
    })
    .done(),
    ItemSelector::ROOT
);

// ---------------------------------------------------------------------------
// Outside patterns (R30)
// ---------------------------------------------------------------------------

endpoint!(
    TokenAllExceptSkip,
    CursorKind,
    Named,
    "/v1/shelves/s1/books",
    // Written out: a follow-up rule is not `Copy`, so `Def` cannot carry it.
    Strategy {
        kind: forward_cursor(
            exclusive(&[CursorPart {
                param: Input::Query("pageToken"),
                source: CursorSource::Field(Source::Body(&["nextPageToken"])),
            }]),
            None,
            false,
        ),
        size: Some(PageSize {
            max: Some(50),
            adjustable: true,
            ..size(Input::Query("pageSize"))
        }),
        constants: &[],
        total: Some(Source::Body(&["totalSize"])),
        terminal_values: &[],
        follow_up: Some(FollowUp {
            method: None,
            path: None,
            carry: Carry::AllExcept(&["skip"]),
        }),
        session: None,
        partition: None,
    },
    under(&[Step::Key("books"), Step::Each])
);
impl PageSizeKnob for TokenAllExceptSkip {}
impl AdjustablePageSize for TokenAllExceptSkip {}
impl HasTotal for TokenAllExceptSkip {}

endpoint!(
    LinkNextPrev,
    NextLinkKind,
    Item,
    "/api/v1/timelines",
    Def::new(Kind::NextLink {
        next: LinkSource::LinkHeader { rel: "next" },
        prev: Some(LinkSource::LinkHeader { rel: "prev" }),
    })
    .size(size(Input::Query("limit")))
    .done(),
    ItemSelector::ROOT
);
impl PageSizeKnob for LinkNextPrev {}
impl HasPrev for LinkNextPrev {}

endpoint!(
    OneBasedOffsetTotal,
    OffsetKind,
    Item,
    "/Users",
    Def::new(Kind::Offset {
        offset: Input::Query("startIndex"),
        base: IndexBase::OneBased,
        more: MoreSignal::Total {
            source: Source::Body(&["totalResults"]),
            unit: CountUnit::Items,
        },
    })
    .size(size(Input::Query("count")))
    .done(),
    under(&[Step::Key("Resources"), Step::Each])
);
impl PageSizeKnob for OneBasedOffsetTotal {}
impl HasTotal for OneBasedOffsetTotal {}

endpoint!(
    HeaderLocator,
    CursorKind,
    Item,
    "/jobs/query/j1/results",
    Def::new(forward_cursor(
        exclusive(&[CursorPart {
            param: Input::Query("locator"),
            source: CursorSource::Field(Source::Header("Sforce-Locator")),
        }]),
        None,
        false,
    ))
    .size(size(Input::Query("maxRecords")))
    .terminal(&["null"])
    .done(),
    ItemSelector::ROOT
);
impl PageSizeKnob for HeaderLocator {}

endpoint!(
    MetaNextToken,
    CursorKind,
    Named,
    "/2/tweets/search/recent",
    Def::new(forward_cursor(
        exclusive(&[CursorPart {
            param: Input::Query("pagination_token"),
            source: CursorSource::Field(Source::Body(&["meta", "next_token"])),
        }]),
        None,
        false,
    ))
    .size(PageSize { min: 10, max: Some(100), ..size(Input::Query("max_results")) })
    .done(),
    under(&[Step::Key("data"), Step::Each])
);
impl PageSizeKnob for MetaNextToken {}

endpoint!(
    TotalPagesHeader,
    PageNumberKind,
    Item,
    "/wp/v2/posts",
    Def::new(Kind::PageNumber {
        page: Input::Query("page"),
        base: IndexBase::OneBased,
        more: MoreSignal::Total {
            source: Source::Header("X-WP-TotalPages"),
            unit: CountUnit::Pages,
        },
    })
    .size(size(Input::Query("per_page")))
    .done(),
    ItemSelector::ROOT
);
impl PageSizeKnob for TotalPagesHeader {}
impl HasTotal for TotalPagesHeader {}

endpoint!(
    DottedNextLink,
    NextLinkKind,
    Item,
    "/v1.0/users",
    Def::new(Kind::NextLink {
        next: LinkSource::Field(Source::Body(&["@odata.nextLink"])),
        prev: None,
    })
    .done(),
    under(&[Step::Key("value"), Step::Each])
);

endpoint!(
    HeaderContinuation,
    CursorKind,
    Item,
    "/dbs/d/colls/c/docs",
    Def::new(forward_cursor(
        exclusive(&[CursorPart {
            param: Input::Header("x-ms-continuation"),
            source: CursorSource::Field(Source::Header("x-ms-continuation")),
        }]),
        None,
        false,
    ))
    .size(size(Input::Header("x-ms-max-item-count")))
    .done(),
    under(&[Step::Key("Documents"), Step::Each])
);
impl PageSizeKnob for HeaderContinuation {}

endpoint!(
    RelativeRecordsUrl,
    NextLinkKind,
    Item,
    "/services/data/v60.0/query",
    Def::new(Kind::NextLink {
        next: LinkSource::Field(Source::Body(&["nextRecordsUrl"])),
        prev: None,
    })
    .done(),
    under(&[Step::Key("records"), Step::Each])
);

endpoint!(
    NestedChildren,
    CursorKind,
    Named,
    "/r/rust/new",
    Def::new(forward_cursor(
        exclusive(&[CursorPart {
            param: Input::Query("after"),
            source: CursorSource::Field(Source::Body(&["data", "after"])),
        }]),
        None,
        false,
    ))
    .size(PageSize { max: Some(100), ..size(Input::Query("limit")) })
    .done(),
    ItemSelector {
        record: &[Step::Key("data"), Step::Key("children"), Step::Each],
        projection: &["data"],
    }
);
impl PageSizeKnob for NestedChildren {}

// ---------------------------------------------------------------------------
// Rule fixtures
// ---------------------------------------------------------------------------

endpoint!(
    IsLastOffset,
    OffsetKind,
    Item,
    "/is-last",
    Def::new(Kind::Offset {
        offset: Input::Query("startAt"),
        base: IndexBase::ZeroBased,
        more: MoreSignal::Flag {
            source: Source::Body(&["isLast"]),
            polarity: Polarity::TrueMeansLast,
        },
    })
    .size(size(Input::Query("maxResults")))
    .done(),
    under(&[Step::Key("values"), Step::Each])
);
impl PageSizeKnob for IsLastOffset {}

endpoint!(
    ProjectedOffset,
    OffsetKind,
    Item,
    "/projected-offset",
    Def::new(Kind::Offset {
        offset: Input::Query("offset"),
        base: IndexBase::ZeroBased,
        more: MoreSignal::NonEmptyPage,
    })
    .size(size(Input::Query("limit")))
    .done(),
    ItemSelector {
        record: &[Step::Key("rows"), Step::Each],
        projection: &["value"],
    }
);
impl PageSizeKnob for ProjectedOffset {}

endpoint!(
    SentDefaultFullPage,
    PageNumberKind,
    Item,
    "/sent-default",
    Def::new(Kind::PageNumber {
        page: Input::Query("page"),
        base: IndexBase::OneBased,
        more: MoreSignal::FullPage,
    })
    .size(PageSize {
        sent_default: Some(2),
        max: Some(10),
        ..size(Input::Query("limit"))
    })
    .done(),
    ItemSelector::ROOT
);
impl PageSizeKnob for SentDefaultFullPage {}

endpoint!(
    ServerDefaultFullPage,
    PageNumberKind,
    Item,
    "/server-default",
    Def::new(Kind::PageNumber {
        page: Input::Query("page"),
        base: IndexBase::OneBased,
        more: MoreSignal::FullPage,
    })
    .size(PageSize {
        server_default: Some(2),
        max: Some(10),
        ..size(Input::Query("limit"))
    })
    .done(),
    ItemSelector::ROOT
);
impl PageSizeKnob for ServerDefaultFullPage {}

endpoint!(
    CompositeField,
    CursorKind,
    Item,
    "/composite",
    Def::new(forward_cursor(
        exclusive(&[
            CursorPart {
                param: Input::Query("created_before"),
                source: CursorSource::Field(Source::Body(&["next", "created"])),
            },
            CursorPart {
                param: Input::Query("id_before"),
                source: CursorSource::Field(Source::Body(&["next", "id"])),
            },
        ]),
        None,
        true,
    ))
    .done(),
    under(&[Step::Key("items"), Step::Each])
);
impl ResumeAfter for CompositeField {
    type Cursor = (String, String);
}

endpoint!(
    InclusiveSince,
    CursorKind,
    Item,
    "/inclusive",
    Def::new(forward_cursor(
        CursorLeg {
            parts: &[CursorPart {
                param: Input::Query("since"),
                source: CursorSource::LastItem(&["t"]),
            }],
            boundary: Boundary::Inclusive { identity: &["id"] },
        },
        Some(MoreSignal::NonEmptyPage),
        false,
    ))
    .done(),
    ItemSelector::ROOT
);

endpoint!(
    FullyAuthored,
    CursorKind,
    Named,
    "/fully-authored",
    Def::new(Kind::Cursor {
        forward: exclusive(&[CursorPart {
            param: Input::Query("after"),
            source: CursorSource::LastItem(&["id"]),
        }]),
        backward: Some(exclusive(&[CursorPart {
            param: Input::Query("before"),
            source: CursorSource::FirstItem(&["id"]),
        }])),
        more: Some(has_more(&["has_more"])),
        resumable: true,
    })
    .size(PageSize {
        max: Some(100),
        adjustable: true,
        ..size(Input::Query("limit"))
    })
    .total(Source::Body(&["total"]))
    .done(),
    under(&[Step::Key("data"), Step::Each])
);
impl PageSizeKnob for FullyAuthored {}
impl AdjustablePageSize for FullyAuthored {}
impl ResumeAfter for FullyAuthored {
    type Cursor = String;
}
impl ResumeBefore for FullyAuthored {
    type Cursor = String;
}
impl HasPrev for FullyAuthored {}
impl HasTotal for FullyAuthored {}

// ---------------------------------------------------------------------------
// Mock helpers
// ---------------------------------------------------------------------------

/// A 200 JSON response.
pub fn json(body: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(body)
}

/// Serves `response` to requests matching `matcher`, ahead of any fallback.
pub async fn on(server: &MockServer, matcher: impl Match + 'static, response: ResponseTemplate) {
    Mock::given(matcher).respond_with(response).with_priority(1).mount(server).await;
}

/// Serves `response` to requests whose query has `name=value`.
pub async fn on_query(server: &MockServer, name: &str, value: &str, response: ResponseTemplate) {
    on(server, query_param(name, value), response).await;
}

/// Serves `response` to requests carrying header `name: value`.
pub async fn on_header(server: &MockServer, name: &'static str, value: &str, response: ResponseTemplate) {
    on(server, header(name, value), response).await;
}

/// Serves `response` to every request no other mock matches.
pub async fn otherwise(server: &MockServer, response: ResponseTemplate) {
    Mock::given(wiremock::matchers::any())
        .respond_with(response)
        .with_priority(10)
        .mount(server)
        .await;
}

/// The requests the server received, in order.
pub async fn received(server: &MockServer) -> Vec<Request> {
    server.received_requests().await.expect("recording is on")
}

/// The query pairs of each received request, in order.
pub async fn queries(server: &MockServer) -> Vec<Vec<(String, String)>> {
    received(server).await.iter().map(query).collect()
}

/// The query pairs of `request`, in order.
pub fn query(request: &Request) -> Vec<(String, String)> {
    request.url.query_pairs().map(|(name, value)| (name.into_owned(), value.into_owned())).collect()
}

/// `[(name, value), ..]` as owned pairs, for comparing with [`queries`].
pub fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected.iter().map(|(name, value)| ((*name).to_owned(), (*value).to_owned())).collect()
}

/// A client for the mock server.
pub fn client(server: &MockServer) -> Client<TestApi> {
    plain_client(&server.uri())
}

/// The ids of numbered items.
pub fn ids(items: &[Item]) -> Vec<u64> {
    items.iter().map(|item| item.id).collect()
}

/// The ids of named items.
pub fn names(items: &[Named]) -> Vec<&str> {
    items.iter().map(|item| item.id.as_str()).collect()
}

/// `[{"id": n}, ..]`.
pub fn numbered(range: impl IntoIterator<Item = u64>) -> Value {
    Value::Array(range.into_iter().map(|id| serde_json::json!({ "id": id })).collect())
}

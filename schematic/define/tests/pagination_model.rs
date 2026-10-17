//! The pagination model: strategy rules with their valid siblings, caller
//! knobs, and selector parsing.
//!
//! Ported from the pagination prototype's `patterns.rs` and
//! `generalizations.rs`. Link trust is gone (the API's cross-origin policy
//! governs links), and the runtime-only pieces (JSON lookup, the inclusive
//! boundary tracker, terminal-value checks) are tested with the runtime pager.

mod pagination_support;

use pagination_support::{endpoint_errors, get, post, with_query};
use schematic_define::RestMethod;
use schematic_define::pagination::{
    Boundary, Carry, Collection, CursorLeg, CursorSource, Direction, FollowUp, IndexBase, InputParam,
    ItemSelector, JsonPath, KeepAlive, Knob, LinkSource, MoreSignal, PageSize, PageStrategy,
    PaginationError, Partition, Polarity, Session, SessionStep, Step, Strategy, ValueSource,
    strategy_knobs, validate_strategy,
};

fn errors_of(strategy: impl Into<Strategy>) -> Vec<PaginationError> {
    validate_strategy(&strategy.into())
}

fn knobs_of(strategy: impl Into<Strategy>) -> Vec<Knob> {
    strategy_knobs(&strategy.into())
}

fn size_knob(param: &str, server_default: Option<u32>, max: Option<u32>) -> Knob {
    Knob::PageSize {
        param: param.to_owned(),
        min: 1,
        max,
        server_default,
    }
}

// ---------------------------------------------------------------------------
// Builders and named strategy structs
// ---------------------------------------------------------------------------

#[test]
fn builders_produce_named_kind_structs_with_documented_defaults() {
    let page = Strategy::page_number("page", MoreSignal::NonEmptyPage).build();
    let PageStrategy::PageNumber(page) = &page.kind else {
        panic!("page number expected")
    };
    assert_eq!(page.base, IndexBase::OneBased);
    assert_eq!(page.size, None);

    let offset = Strategy::offset("offset", MoreSignal::NonEmptyPage).build();
    assert!(matches!(&offset.kind, PageStrategy::Offset(kind) if kind.base == IndexBase::ZeroBased));

    let cursor = Strategy::cursor("after", CursorSource::body("next")).build();
    let PageStrategy::Cursor(cursor) = &cursor.kind else {
        panic!("cursor expected")
    };
    assert!(!cursor.resumable);
    assert_eq!(cursor.more, None);
    assert_eq!(cursor.backward, None);

    let link = Strategy::next_link(LinkSource::link_header("next")).build();
    assert_eq!(link.kind.kind_name(), "next_link");
    assert_eq!(link.kind.more(), None);

    let size = PageSize::new("limit");
    assert_eq!((size.min, size.max, size.server_default, size.sent_default), (1, None, None, None));
    assert!(!size.adjustable);
}

#[test]
fn shared_dimension_setters_finish_the_builder() {
    let strategy = Strategy::cursor("after", CursorSource::body("next"))
        .constant("order", "asc")
        .terminal_value("null")
        .total(ValueSource::body("total"));
    assert_eq!(strategy.constants[0].param, InputParam::query("order"));
    assert_eq!(strategy.terminal_values, ["null"]);
    assert_eq!(strategy.total, Some(ValueSource::body("total")));

    let resized = strategy.with_page_size(PageSize::new("per_page"));
    assert_eq!(resized.kind.size().unwrap().param, InputParam::query("per_page"));
}

#[test]
fn default_carry_depends_on_kind() {
    assert_eq!(Strategy::cursor("c", CursorSource::body("n")).build().effective_carry(), Carry::All);
    assert_eq!(Strategy::next_link(LinkSource::body("n")).build().effective_carry(), Carry::Nothing);
    let only = Strategy::next_link(LinkSource::body("n")).follow_up(FollowUp::carry(Carry::Only(vec!["q".into()])));
    assert_eq!(only.effective_carry(), Carry::Only(vec!["q".into()]));
}

// ---------------------------------------------------------------------------
// Selectors and paths
// ---------------------------------------------------------------------------

#[test]
fn selector_splits_records_and_projection() {
    let reddit = ItemSelector::parse("data.children[*].data");
    assert_eq!(reddit.record, [Step::Key("data".into()), Step::Key("children".into()), Step::Each]);
    assert_eq!(reddit.projection, JsonPath::parse("data"));
    assert_eq!(reddit.fan_outs(), 1);

    let msearch = ItemSelector::parse("responses[*].hits.hits[*]._source");
    assert_eq!(msearch.fan_outs(), 2);
    assert_eq!(msearch.projection, JsonPath::parse("_source"));
    assert_eq!(msearch.to_string(), "responses[*].hits.hits[*]._source");

    assert_eq!(ItemSelector::parse("data"), ItemSelector::parse("data[*]"));
    assert_eq!(ItemSelector::parse("[*]"), ItemSelector::root());
    assert!(ItemSelector::root().is_root_array());
    assert_eq!(ItemSelector::root().to_string(), "[*]");
}

#[test]
fn empty_selector_is_not_a_root_array() {
    let empty = ItemSelector::parse("");
    assert!(!empty.is_root_array());
    assert_eq!(empty.record, [Step::Key(String::new()), Step::Each]);
    assert_eq!(Collection::at("").items, empty);
    assert!(Collection::root().items.is_root_array());
}

#[test]
fn json_path_keeps_dotted_keys_whole_only_through_segments() {
    assert_eq!(JsonPath::segments(["@odata.nextLink"]).as_segments(), ["@odata.nextLink"]);
    assert_eq!(JsonPath::parse("@odata.nextLink").as_segments(), ["@odata", "nextLink"]);
    assert_eq!(JsonPath::parse("a..b").as_segments(), ["a", "", "b"]);
    assert!(!JsonPath::parse("").is_root());
}

// ---------------------------------------------------------------------------
// R2: each rule with its valid sibling
// ---------------------------------------------------------------------------

#[test]
fn cursor_read_only_from_items_needs_an_end_signal() {
    let without = Strategy::cursor("starting_after", CursorSource::last_item("id"));
    assert_eq!(errors_of(without), vec![PaginationError::CursorWithoutEndSignal]);
    let stripe = Strategy::cursor("starting_after", CursorSource::last_item("id"))
        .backward("ending_before", CursorSource::first_item("id"))
        .more_when(MoreSignal::has_more("has_more"));
    assert_eq!(errors_of(stripe), vec![]);
    // A field cursor ends when it is absent, so it needs no signal.
    let slack = Strategy::cursor("cursor", CursorSource::body("response_metadata.next_cursor"))
        .page_size(PageSize::new("limit").max(1000));
    assert_eq!(errors_of(slack), vec![]);
}

#[test]
fn short_page_signal_rules() {
    let cursor = Strategy::cursor("cursor", CursorSource::body("next")).more_when(MoreSignal::FullPage);
    assert_eq!(errors_of(cursor), vec![PaginationError::FullPageOnCursor]);
    let unsized_page = Strategy::page_number("page", MoreSignal::FullPage);
    assert_eq!(errors_of(unsized_page), vec![PaginationError::FullPageWithoutPageSize]);
    let sized = Strategy::page_number("page", MoreSignal::FullPage).page_size(PageSize::new("per_page"));
    assert_eq!(errors_of(sized), vec![]);
}

#[test]
fn page_totals_end_only_page_numbers() {
    let pages_header = Strategy::page_number("page", MoreSignal::total_pages(ValueSource::header("X-Total-Pages")));
    assert_eq!(errors_of(pages_header), vec![]);
    let pages_on_offset = Strategy::offset("offset", MoreSignal::total_pages(ValueSource::header("X-Total-Pages")));
    assert_eq!(errors_of(pages_on_offset), vec![PaginationError::PageCountOnNonPageNumber]);
    let items_on_offset = Strategy::offset("startAt", MoreSignal::total_items(ValueSource::body("total")));
    assert_eq!(errors_of(items_on_offset), vec![]);
}

#[test]
fn page_size_bounds_must_be_consistent() {
    let invalid = |size: PageSize| {
        let errors = errors_of(Strategy::offset("offset", MoreSignal::NonEmptyPage).page_size(size));
        assert!(matches!(errors.as_slice(), [PaginationError::InvalidPageSize { .. }]), "{errors:?}");
    };
    invalid(PageSize::new("limit").server_default(200).max(100));
    invalid(PageSize::new("limit").min(0));
    invalid(PageSize::new("limit").min(10).max(5));
    invalid(PageSize::new("limit").sent_default(0));
    invalid(PageSize::new("limit").max(100).sent_default(101));

    let valid = PageSize::new("limit").min(1).server_default(20).sent_default(100).max(100);
    assert_eq!(errors_of(Strategy::offset("offset", MoreSignal::NonEmptyPage).page_size(valid)), vec![]);
    // Boundaries are inclusive.
    let edges = PageSize::new("limit").min(5).server_default(5).sent_default(5).max(5);
    assert_eq!(errors_of(Strategy::offset("offset", MoreSignal::NonEmptyPage).page_size(edges)), vec![]);
}

#[test]
fn one_destination_may_be_written_once() {
    let legs = Strategy::cursor("after", CursorSource::body("last_id"))
        .backward("after", CursorSource::body("first_id"))
        .more_when(MoreSignal::has_more("has_more"));
    assert_eq!(errors_of(legs), vec![PaginationError::DuplicateInput("after".into())]);

    let constant = Strategy::page_number("page", MoreSignal::NonEmptyPage).constant("page", "1");
    assert_eq!(errors_of(constant), vec![PaginationError::DuplicateInput("page".into())]);

    // Header names compare without case.
    let headers = Strategy::cursor(InputParam::header("X-Cursor"), CursorSource::header("x-next"))
        .constant(InputParam::header("x-cursor"), "start");
    assert_eq!(errors_of(headers), vec![PaginationError::DuplicateInput("x-cursor".into())]);

    // A body path conflicts with a path beneath it.
    let body = Strategy::cursor(InputParam::body("search_after"), CursorSource::last_item("sort"))
        .more_when(MoreSignal::NonEmptyPage)
        .constant(InputParam::body("pit"), "x")
        .session(
            Session::opened_by(SessionStep::new(RestMethod::Post, "/_pit"), ValueSource::body("id"))
                .send_handle_as(InputParam::body("pit.id")),
        );
    assert_eq!(errors_of(body), vec![PaginationError::DuplicateInput("pit.id".into())]);

    let tiebreak = Strategy::cursor("since", CursorSource::last_item("t"))
        .tiebreak("since", CursorSource::last_item("id"))
        .more_when(MoreSignal::NonEmptyPage);
    assert_eq!(errors_of(tiebreak), vec![PaginationError::DuplicateInput("since".into())]);

    // Same names in different locations, and distinct body paths, are fine.
    let distinct = Strategy::cursor("cursor", CursorSource::body("next"))
        .constant(InputParam::header("cursor"), "x")
        .constant(InputParam::body("pit.id"), "a")
        .constant(InputParam::body("pit.keep_alive"), "1m");
    assert_eq!(errors_of(distinct), vec![]);
}

#[test]
fn empty_names_and_locations_are_rejected() {
    let empty = |strategy: Strategy, what: &'static str| {
        assert_eq!(errors_of(strategy), vec![PaginationError::EmptyValue(what)], "{what}");
    };
    empty(Strategy::page_number("", MoreSignal::NonEmptyPage).build(), "an input name or body path");
    empty(
        Strategy::cursor(InputParam::body("a..b"), CursorSource::body("next")).build(),
        "an input name or body path",
    );
    empty(Strategy::cursor("after", CursorSource::body("")).build(), "a response location");
    empty(Strategy::page_number("page", MoreSignal::has_more("meta..more")).build(), "a response location");
    empty(Strategy::next_link(LinkSource::link_header("")).build(), "link relation");
    empty(Strategy::page_number("page", MoreSignal::link_rel("")).build(), "link relation");
    empty(
        Strategy::cursor("since", CursorSource::last_item("a..b")).more_when(MoreSignal::NonEmptyPage).build(),
        "an item path segment",
    );
    // An item path may be the record itself.
    let whole_record = Strategy::cursor_leg(CursorLeg::single(
        "after",
        CursorSource::LastItem { path: JsonPath::root() },
    ))
    .more_when(MoreSignal::NonEmptyPage);
    assert_eq!(errors_of(whole_record), vec![]);
}

#[test]
fn body_inputs_need_a_request_with_a_body() {
    let notion = || {
        Strategy::cursor(InputParam::body("start_cursor"), CursorSource::body("next_cursor"))
            .more_when(MoreSignal::has_more("has_more"))
            .page_size(PageSize::new(InputParam::body("page_size")).server_default(100).max(100))
    };
    assert_eq!(endpoint_errors(post("Query", "/databases/{id}/query", "Results"), "results", notion()), vec![]);
    assert_eq!(
        endpoint_errors(get("Query", "/databases/{id}/query", "Results"), "results", notion()),
        vec![
            PaginationError::BodyInputOnGet("page_size".into()),
            PaginationError::BodyInputOnGet("start_cursor".into()),
        ]
    );
}

#[test]
fn next_link_on_a_non_get_endpoint_states_its_follow_up_method() {
    let link = || Strategy::next_link(LinkSource::body("next"));
    assert_eq!(
        endpoint_errors(post("Search", "/search", "Results"), "items", link()),
        vec![PaginationError::NextLinkMethodUnspecified]
    );
    let stated = link().follow_up(FollowUp::carry(Carry::Nothing).method(RestMethod::Get));
    assert_eq!(endpoint_errors(post("Search", "/search", "Results"), "items", stated), vec![]);
    assert_eq!(endpoint_errors(get("Search", "/search", "Results"), "items", link()), vec![]);
}

#[test]
fn next_link_follows_the_link_so_it_has_no_follow_up_path() {
    let with_path = Strategy::next_link(LinkSource::body("next")).follow_up(FollowUp::to(RestMethod::Get, "/elsewhere"));
    assert_eq!(errors_of(with_path), vec![PaginationError::NextLinkWithFollowUpPath]);
    let cursor_with_path = Strategy::cursor(InputParam::body("c"), CursorSource::body("n"))
        .follow_up(FollowUp::to(RestMethod::Post, "/elsewhere"));
    assert_eq!(errors_of(cursor_with_path), vec![]);
}

#[test]
fn carried_names_and_step_paths_must_belong_to_the_endpoint() {
    let unknown_carry = Strategy::cursor("c", CursorSource::body("n")).follow_up(FollowUp::carry(Carry::Only(vec!["nope".into()])));
    assert_eq!(
        endpoint_errors(get("L", "/l", "Vec<T>"), "", unknown_carry),
        vec![PaginationError::UnknownCarryParam("nope".into())]
    );
    let known_carry = Strategy::cursor("c", CursorSource::body("n"))
        .page_size(PageSize::new("limit"))
        .follow_up(FollowUp::carry(Carry::Only(vec!["filter".into(), "limit".into()])));
    assert_eq!(endpoint_errors(with_query(get("L", "/l", "Vec<T>"), &["filter"]), "", known_carry), vec![]);

    let unknown_path = Strategy::cursor(InputParam::body("c"), CursorSource::body("n"))
        .follow_up(FollowUp::to(RestMethod::Post, "/{other}/scroll"));
    assert_eq!(
        endpoint_errors(post("S", "/{index}/_search", "R"), "hits", unknown_path),
        vec![PaginationError::UnknownPathParam("other".into())]
    );
    let known_path = Strategy::cursor(InputParam::body("c"), CursorSource::body("n"))
        .follow_up(FollowUp::to(RestMethod::Post, "/{index}/_search/scroll"));
    assert_eq!(endpoint_errors(post("S", "/{index}/_search", "R"), "hits", known_path), vec![]);

    // A cursor sent in a body on a GET follow-up.
    let body_on_get = Strategy::cursor(InputParam::body("c"), CursorSource::body("n"))
        .follow_up(FollowUp::carry(Carry::All).method(RestMethod::Get));
    assert_eq!(
        endpoint_errors(post("S", "/s", "R"), "hits", body_on_get),
        vec![PaginationError::BodyInputOnGet("c".into())]
    );
}

#[test]
fn composite_and_inclusive_cursor_rules() {
    let empty_leg = Strategy::cursor_leg(CursorLeg { parts: vec![], boundary: Boundary::Exclusive });
    assert_eq!(errors_of(empty_leg), vec![PaginationError::EmptyCursorLeg]);

    let window = Strategy::cursor("since", CursorSource::last_item("updated_at"))
        .inclusive("id")
        .more_when(MoreSignal::NonEmptyPage);
    assert_eq!(errors_of(window), vec![]);
    let token_inclusive = Strategy::cursor("since", CursorSource::body("next")).inclusive("id");
    assert_eq!(errors_of(token_inclusive), vec![PaginationError::InclusiveBoundaryNeedsItemKey]);

    let keyset = Strategy::cursor("created_before", CursorSource::last_item("created_at"))
        .tiebreak("id_before", CursorSource::last_item("id"))
        .more_when(MoreSignal::NonEmptyPage)
        .page_size(PageSize::new("limit").max(100))
        .resumable();
    assert_eq!(errors_of(keyset.clone()), vec![]);
    assert!(knobs_of(keyset).contains(&Knob::ResumeCursor { direction: Direction::Forward, parts: 2 }));
}

#[test]
fn adjustable_page_size_rules() {
    let page_number = Strategy::page_number("page", MoreSignal::NonEmptyPage).page_size(PageSize::new("per_page").adjustable());
    assert_eq!(errors_of(page_number), vec![PaginationError::AdjustablePageSizeOnPageNumber]);

    let link_body = Strategy::next_link(LinkSource::body("next")).page_size(PageSize::new(InputParam::body("size")).adjustable());
    assert_eq!(errors_of(link_body), vec![PaginationError::AdjustableLinkSizeNotQuery]);
    let link_query = Strategy::next_link(LinkSource::body("next")).page_size(PageSize::new("limit").adjustable());
    assert_eq!(errors_of(link_query), vec![]);

    let offset = Strategy::offset("offset", MoreSignal::NonEmptyPage).page_size(PageSize::new("limit").adjustable());
    assert_eq!(errors_of(offset.clone()), vec![]);
    assert!(knobs_of(offset).contains(&Knob::AdjustPageSize));
    let fixed = Strategy::offset("offset", MoreSignal::NonEmptyPage).page_size(PageSize::new("limit"));
    assert!(!knobs_of(fixed).contains(&Knob::AdjustPageSize));
}

#[test]
fn partition_maximum_is_positive() {
    let zero = Strategy::cursor(InputParam::body("c"), CursorSource::body("n"))
        .partition(Partition::new(InputParam::body("slice.id"), InputParam::body("slice.max")).max(0));
    assert_eq!(errors_of(zero), vec![PaginationError::InvalidPartitionMax]);
    let bounded = Strategy::cursor(InputParam::body("c"), CursorSource::body("n"))
        .partition(Partition::new(InputParam::body("slice.id"), InputParam::body("slice.max")).max(1));
    assert_eq!(errors_of(bounded), vec![]);
}

// ---------------------------------------------------------------------------
// Sessions
// ---------------------------------------------------------------------------

/// Elasticsearch scroll: the first page opens the session; follow-ups go to
/// another path; the scroll id is both cursor and handle.
fn es_scroll() -> Strategy {
    Strategy::cursor(InputParam::body("scroll_id"), CursorSource::body("_scroll_id"))
        .more_when(MoreSignal::NonEmptyPage)
        .page_size(PageSize::new("size").max(10_000))
        .follow_up(FollowUp::to(RestMethod::Post, "/_search/scroll"))
        .session(
            Session::opened_by_first_page()
                .handle_from_page(ValueSource::body("_scroll_id"))
                .keep_alive(KeepAlive::new("1m").on_open("scroll").on_page(InputParam::body("scroll")).adjustable())
                .close(SessionStep::new(RestMethod::Delete, "/_search/scroll").handle(InputParam::body("scroll_id"))),
        )
        .total(ValueSource::body("hits.total.value"))
}

/// Elasticsearch point in time: a separate request opens the session; pages
/// go to `/_search`; the handle changes each page.
fn es_pit() -> Strategy {
    Strategy::cursor(InputParam::body("search_after"), CursorSource::last_item("sort"))
        .more_when(MoreSignal::NonEmptyPage)
        .page_size(PageSize::new(InputParam::body("size")).max(10_000))
        .session(
            Session::opened_by(SessionStep::new(RestMethod::Post, "/{index}/_pit"), ValueSource::body("id"))
                .pages_at(RestMethod::Post, "/_search")
                .handle_from_page(ValueSource::body("pit_id"))
                .send_handle_as(InputParam::body("pit.id"))
                .keep_alive(KeepAlive::new("1m").on_open("keep_alive").on_page(InputParam::body("pit.keep_alive")))
                .close(SessionStep::new(RestMethod::Delete, "/_pit").handle(InputParam::body("id"))),
        )
        .partition(Partition::new(InputParam::body("slice.id"), InputParam::body("slice.max")))
}

#[test]
fn scroll_session_is_valid_and_exposes_keep_alive_and_the_scope() {
    assert_eq!(endpoint_errors(post("SearchScroll", "/{index}/_search", "SearchResponse"), "hits.hits[*]._source", es_scroll()), vec![]);
    assert_eq!(
        knobs_of(es_scroll()),
        vec![
            size_knob("size", None, Some(10_000)),
            Knob::KeepAlive { default: "1m".into() },
            Knob::Next,
            Knob::Total,
            Knob::Session,
        ]
    );
}

#[test]
fn pit_session_is_valid_and_keep_alive_is_a_knob_only_when_adjustable() {
    assert_eq!(endpoint_errors(post("SearchPit", "/{index}/_search", "SearchResponse"), "hits.hits[*]._source", es_pit()), vec![]);
    let surface = knobs_of(es_pit());
    assert!(surface.contains(&Knob::Session));
    assert!(surface.contains(&Knob::Partitions { max: None }));
    assert!(!surface.iter().any(|knob| matches!(knob, Knob::KeepAlive { .. })));
    assert!(!surface.contains(&Knob::Renew));
}

#[test]
fn session_handle_rules() {
    let unreachable = Strategy::cursor("c", CursorSource::body("next")).session(Session::opened_by_first_page());
    assert!(errors_of(unreachable).contains(&PaginationError::SessionHandleUnreachable));

    let unsent = Strategy::cursor("c", CursorSource::body("next"))
        .session(Session::opened_by(SessionStep::new(RestMethod::Post, "/open"), ValueSource::body("id")));
    assert_eq!(errors_of(unsent), vec![PaginationError::SessionHandleNotSent]);
    let sent = Strategy::cursor("c", CursorSource::body("next")).session(
        Session::opened_by(SessionStep::new(RestMethod::Post, "/open"), ValueSource::body("id")).send_handle_as("session"),
    );
    assert_eq!(errors_of(sent), vec![]);

    let incomplete = Strategy::cursor("c", CursorSource::body("sid")).session(
        Session::opened_by_first_page()
            .handle_from_page(ValueSource::body("sid"))
            .keep_alive(KeepAlive::new("1m"))
            .close(SessionStep::new(RestMethod::Delete, "/close")),
    );
    assert_eq!(
        errors_of(incomplete),
        vec![PaginationError::SessionStepNeedsHandle("close".into()), PaginationError::KeepAliveWithoutInput]
    );
    let complete = Strategy::cursor("c", CursorSource::body("sid")).session(
        Session::opened_by_first_page()
            .handle_from_page(ValueSource::body("sid"))
            .keep_alive(KeepAlive::new("1m").on_page("keep"))
            .close(SessionStep::new(RestMethod::Delete, "/close").handle("sid")),
    );
    assert_eq!(errors_of(complete), vec![]);
}

#[test]
fn session_steps_use_endpoint_path_params_and_bodies_only_where_allowed() {
    let bad_step = Strategy::cursor("c", CursorSource::body("sid")).session(
        Session::opened_by_first_page()
            .handle_from_page(ValueSource::body("sid"))
            .renew(SessionStep::new(RestMethod::Get, "/sessions/{session}").handle(InputParam::body("sid"))),
    );
    assert_eq!(
        endpoint_errors(get("L", "/items", "Items"), "items", bad_step),
        vec![PaginationError::UnknownPathParam("session".into()), PaginationError::BodyInputOnGet("sid".into())]
    );

    let renewable = Strategy::cursor("cursor", CursorSource::body("cursor")).session(
        Session::opened_by_first_page()
            .handle_from_page(ValueSource::body("cursor"))
            .renew(SessionStep::new(RestMethod::Post, "/exports/{id}/heartbeat").handle(InputParam::body("cursor"))),
    );
    assert_eq!(endpoint_errors(get("Export", "/exports/{id}/rows", "Rows"), "rows", renewable.clone()), vec![]);
    assert!(knobs_of(renewable).contains(&Knob::Renew));
}

#[test]
fn salesforce_bulk_locator_with_a_terminal_value_is_valid() {
    let strategy = Strategy::cursor("locator", CursorSource::header("Sforce-Locator"))
        .page_size(PageSize::new("maxRecords"))
        .terminal_value("null");
    assert_eq!(endpoint_errors(get("GetQueryResults", "/jobs/query/{jobId}/results", "Vec<Row>"), "", strategy), vec![]);
}

// ---------------------------------------------------------------------------
// Knobs
// ---------------------------------------------------------------------------

#[test]
fn link_header_knobs() {
    let shopify = Strategy::next_link(LinkSource::link_header("next"))
        .prev(LinkSource::link_header("previous"))
        .page_size(PageSize::new("limit").server_default(50).max(250));
    assert_eq!(knobs_of(shopify), vec![size_knob("limit", Some(50), Some(250)), Knob::Next, Knob::Prev]);
}

#[test]
fn fixed_size_has_no_page_size_knob() {
    let fixed = Strategy::cursor("start_cursor", CursorSource::body("next_cursor"));
    assert_eq!(knobs_of(fixed), vec![Knob::Next]);
}

#[test]
fn start_knobs_follow_the_kind_and_base() {
    let jira = Strategy::offset("startAt", MoreSignal::is_last("isLast")).page_size(PageSize::new("maxResults").server_default(50));
    let jira = jira.build();
    assert!(matches!(
        &jira.kind,
        PageStrategy::Offset(kind) if matches!(kind.more, MoreSignal::Flag { polarity: Polarity::TrueMeansLast, .. })
    ));
    assert_eq!(strategy_knobs(&jira)[0], Knob::StartOffset { base: IndexBase::ZeroBased });

    let scim = Strategy::offset("startIndex", MoreSignal::total_items(ValueSource::body("totalResults"))).one_based();
    assert_eq!(knobs_of(scim), vec![Knob::StartOffset { base: IndexBase::OneBased }, Knob::Next, Knob::Total]);

    let shared_voices = Strategy::page_number("page", MoreSignal::has_more("has_more")).zero_based();
    assert_eq!(knobs_of(shared_voices)[0], Knob::StartPage { base: IndexBase::ZeroBased });

    let not_resumable = Strategy::cursor("after", CursorSource::body("next"))
        .backward("before", CursorSource::body("prev"));
    assert_eq!(knobs_of(not_resumable), vec![Knob::Next, Knob::Prev]);
}

// ---------------------------------------------------------------------------
// Outside patterns: the model expresses them without changes
// ---------------------------------------------------------------------------

#[test]
fn outside_patterns_validate_without_model_changes() {
    let cases = vec![
        // Google AIP-158: empty next_page_token ends; page_size may change.
        (get("AipList", "/v1/{parent}/things", "ListThingsResponse"), "things",
            Strategy::cursor("page_token", CursorSource::body("next_page_token")).page_size(PageSize::new("page_size").adjustable()).build()),
        // Mastodon: Link header next/prev.
        (get("HomeTimeline", "/api/v1/timelines/home", "Vec<Status>"), "",
            Strategy::next_link(LinkSource::link_header("next")).prev(LinkSource::link_header("prev")).page_size(PageSize::new("limit").server_default(20).max(40)).build()),
        // Mastodon alternative: max_id from the last status id.
        (get("HomeTimelineIds", "/api/v1/timelines/home", "Vec<Status>"), "",
            Strategy::cursor("max_id", CursorSource::last_item("id")).backward("min_id", CursorSource::first_item("id")).more_when(MoreSignal::NonEmptyPage).resumable().build()),
        // SCIM: 1-based startIndex with an item total.
        (with_query(get("ListUsers", "/Users", "ListResponse"), &["filter"]), "Resources",
            Strategy::offset("startIndex", MoreSignal::total_items(ValueSource::body("totalResults"))).one_based().page_size(PageSize::new("count")).build()),
        // Salesforce Bulk 2.0: Sforce-Locator header with the literal "null".
        (get("GetQueryResults", "/jobs/query/{jobId}/results", "Vec<Row>"), "",
            Strategy::cursor("locator", CursorSource::header("Sforce-Locator")).page_size(PageSize::new("maxRecords")).terminal_value("null")),
        // Unverified: X API v2, pagination_token <- meta.next_token.
        (get("UserTweets", "/2/users/{id}/tweets", "TweetsResponse"), "data",
            Strategy::cursor("pagination_token", CursorSource::body("meta.next_token")).page_size(PageSize::new("max_results").min(5).max(100)).build()),
        // Unverified: WordPress REST, X-WP-TotalPages.
        (get("ListPosts", "/wp/v2/posts", "Vec<Post>"), "",
            Strategy::page_number("page", MoreSignal::total_pages(ValueSource::header("X-WP-TotalPages")))
                .page_size(PageSize::new("per_page").server_default(10).max(100))
                .total(ValueSource::header("X-WP-Total"))),
        // Unverified: OData $skiptoken inside @odata.nextLink.
        (get("ListUsersOdata", "/users", "UsersPage"), "value",
            Strategy::next_link(LinkSource::body_segments(["@odata.nextLink"])).page_size(PageSize::new("$top")).build()),
        // Unverified: Azure Cosmos DB continuation header in and out.
        (get("ListDocuments", "/dbs/{db}/colls/{coll}/docs", "Docs"), "Documents",
            Strategy::cursor(InputParam::header("x-ms-continuation"), CursorSource::header("x-ms-continuation")).page_size(PageSize::new(InputParam::header("x-ms-max-item-count"))).build()),
        // Unverified: Salesforce REST query, relative nextRecordsUrl.
        (with_query(get("Query", "/services/data/v60.0/query", "QueryResult"), &["q"]), "records",
            Strategy::next_link(LinkSource::body("nextRecordsUrl")).total(ValueSource::body("totalSize"))),
        // Unverified: Reddit listing, after <- data.after.
        (get("Hot", "/r/{subreddit}/hot", "Listing"), "data.children[*].data",
            Strategy::cursor("after", CursorSource::body("data.after")).backward("before", CursorSource::body("data.before")).page_size(PageSize::new("limit").server_default(25).max(100)).build()),
    ];
    for (endpoint, items, strategy) in cases {
        let id = endpoint.id.clone();
        assert!(strategy_knobs(&strategy).contains(&Knob::Next), "{id}");
        assert_eq!(endpoint_errors(endpoint, items, strategy), vec![], "{id}");
    }
}

#[test]
fn a_provider_gap_is_not_a_model_gap() {
    // OpenAI documents no request input that consumes `next_page`. Were one
    // added (hypothetically `page` in the body), the model would express it.
    let hypothetical = Strategy::cursor(InputParam::body("page"), CursorSource::body("next_page"))
        .more_when(MoreSignal::has_more("has_more"));
    assert_eq!(
        endpoint_errors(post("SearchVectorStore", "/vector_stores/{id}/search", "VectorStoreSearchResultsPage"), "data", hypothetical),
        vec![]
    );
}

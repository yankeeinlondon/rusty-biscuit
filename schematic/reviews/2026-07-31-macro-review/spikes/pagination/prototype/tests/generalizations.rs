//! Generalizations added after the 2026-10-10 ruling, plus patterns from
//! outside research.md / inventory.md.

use pagination_prototype::*;
use serde_json::json;

fn ok(endpoint: &EndpointDef) {
    assert_eq!(validate_endpoint(None, endpoint), vec![], "{}", endpoint.id);
}

fn knobs_of(endpoint: &EndpointDef) -> Vec<Knob> {
    caller_knobs(&resolve(None, endpoint))
}

// ---------------------------------------------------------------------------
// G1. Session lifecycle (open, keep-alive, renew, close)
// ---------------------------------------------------------------------------

/// Elasticsearch scroll: the first page opens the session; follow-ups go to
/// another path with a different body; the scroll id is both cursor and
/// handle; keep-alive moves from query (open) to body (pages).
fn es_scroll() -> EndpointDef {
    let strategy = Strategy::cursor(InputParam::body("scroll_id"), CursorSource::body("_scroll_id"))
        .more_when(MoreSignal::NonEmptyPage)
        .page_size(PageSize::new("size").max(10_000))
        .follow_up(FollowUp::to(Method::Post, "/_search/scroll"))
        .session(
            Session::opened_by_first_page()
                .handle_from_page(ValueSource::body("_scroll_id"))
                .keep_alive(KeepAlive::new("1m").on_open("scroll").on_page(InputParam::body("scroll")).adjustable())
                .close(SessionStep::new(Method::Delete, "/_search/scroll").handle(InputParam::body("scroll_id"))),
        )
        .total(ValueSource::body("hits.total.value"));
    EndpointDef::post("SearchScroll", "/{index}/_search", "SearchResponse")
        .collection(Collection::at("hits.hits[*]._source").paginate(strategy))
}

/// Elasticsearch PIT + search_after with an explicit tiebreaker: a separate
/// request opens the session; pages go to `/_search`; the handle changes
/// each page; close is a DELETE with the handle in the body.
fn es_pit() -> EndpointDef {
    let strategy = Strategy::cursor(InputParam::body("search_after"), CursorSource::last_item("sort"))
        .more_when(MoreSignal::NonEmptyPage)
        .page_size(PageSize::new(InputParam::body("size")).max(10_000))
        .session(
            Session::opened_by(SessionStep::new(Method::Post, "/{index}/_pit"), ValueSource::body("id"))
                .pages_at(Method::Post, "/_search")
                .handle_from_page(ValueSource::body("pit_id"))
                .send_handle_as(InputParam::body("pit.id"))
                .keep_alive(KeepAlive::new("1m").on_open("keep_alive").on_page(InputParam::body("pit.keep_alive")))
                .close(SessionStep::new(Method::Delete, "/_pit").handle(InputParam::body("id"))),
        )
        .partition(Partition { index: InputParam::body("slice.id"), count: InputParam::body("slice.max"), max: None });
    EndpointDef::post("SearchPit", "/{index}/_search", "SearchResponse")
        .collection(Collection::at("hits.hits[*]._source").paginate(strategy))
}

#[test]
fn g1_scroll_session_is_valid_and_exposes_keep_alive_and_close() {
    let endpoint = es_scroll();
    ok(&endpoint);
    assert_eq!(
        knobs_of(&endpoint),
        vec![
            Knob::PageSize { param: "size".into(), min: 1, max: Some(10_000), server_default: None },
            Knob::KeepAlive { default: "1m".into() },
            Knob::Next,
            Knob::Total,
            Knob::Close,
        ]
    );
}

#[test]
fn g1_pit_session_is_valid_and_keep_alive_is_not_a_knob_unless_adjustable() {
    let endpoint = es_pit();
    ok(&endpoint);
    let surface = knobs_of(&endpoint);
    assert!(surface.contains(&Knob::Close));
    assert!(surface.contains(&Knob::Partitions { max: None }));
    assert!(!surface.iter().any(|knob| matches!(knob, Knob::KeepAlive { .. })));
    assert!(!surface.contains(&Knob::Renew));
}

#[test]
fn g1_session_validation() {
    // No way to learn the handle.
    let unreachable = Strategy::cursor("c", CursorSource::body("next")).session(Session::opened_by_first_page());
    assert!(validate_strategy(&unreachable).contains(&PaginationError::SessionHandleUnreachable));

    // Handle learned but never sent, and the cursor does not carry it.
    let unsent = Strategy::cursor("c", CursorSource::body("next"))
        .session(Session::opened_by(SessionStep::new(Method::Post, "/open"), ValueSource::body("id")));
    assert_eq!(validate_strategy(&unsent), vec![PaginationError::SessionHandleNotSent]);

    // Close without a place to put the handle; keep-alive with nowhere to go.
    let incomplete = Strategy::cursor("c", CursorSource::body("sid")).session(
        Session::opened_by_first_page()
            .handle_from_page(ValueSource::body("sid"))
            .keep_alive(KeepAlive::new("1m"))
            .close(SessionStep::new(Method::Delete, "/close")),
    );
    assert_eq!(
        validate_strategy(&incomplete),
        vec![PaginationError::SessionStepNeedsHandle("close".into()), PaginationError::KeepAliveWithoutInput]
    );

    // A step path may only use the endpoint's path parameters; body on GET.
    let bad_step = EndpointDef::get("L", "/items", "Items").collection(
        Collection::at("items").paginate(
            Strategy::cursor("c", CursorSource::body("sid")).session(
                Session::opened_by_first_page()
                    .handle_from_page(ValueSource::body("sid"))
                    .renew(SessionStep::new(Method::Get, "/sessions/{session}").handle(InputParam::body("sid"))),
            ),
        ),
    );
    assert_eq!(
        validate_endpoint(None, &bad_step),
        vec![PaginationError::UnknownPathParam("session".into()), PaginationError::BodyInputOnGet("sid".into())]
    );
}

#[test]
fn g1_renew_step_becomes_a_knob() {
    let endpoint = EndpointDef::get("Export", "/exports/{id}/rows", "Rows").collection(
        Collection::at("rows").paginate(
            Strategy::cursor("cursor", CursorSource::body("cursor")).session(
                Session::opened_by_first_page()
                    .handle_from_page(ValueSource::body("cursor"))
                    .renew(SessionStep::new(Method::Post, "/exports/{id}/heartbeat").handle(InputParam::body("cursor"))),
            ),
        ),
    );
    ok(&endpoint);
    assert!(knobs_of(&endpoint).contains(&Knob::Renew));
}

// ---------------------------------------------------------------------------
// G2. Composite cursors and boundary semantics (keyset with tiebreaker)
// ---------------------------------------------------------------------------

#[test]
fn g2_exclusive_composite_keyset() {
    // `created_before` + `id_before` taken from the last record.
    let keyset = Strategy::cursor("created_before", CursorSource::last_item("created_at"))
        .tiebreak("id_before", CursorSource::last_item("id"))
        .more_when(MoreSignal::NonEmptyPage)
        .page_size(PageSize::new("limit").max(100))
        .resumable();
    let endpoint = EndpointDef::get("ListEvents", "/events", "Vec<Event>").collection(Collection::root().paginate(keyset));
    ok(&endpoint);
    assert!(knobs_of(&endpoint).contains(&Knob::ResumeCursor { direction: Direction::Forward, parts: 2 }));
}

#[test]
fn g2_inclusive_time_window_needs_item_keys() {
    let window = Strategy::cursor("since", CursorSource::last_item("updated_at"))
        .inclusive("id")
        .more_when(MoreSignal::NonEmptyPage);
    assert_eq!(validate_strategy(&window.build()), vec![]);

    let token_inclusive = Strategy::cursor("since", CursorSource::body("next")).inclusive("id");
    assert_eq!(validate_strategy(&token_inclusive.build()), vec![PaginationError::InclusiveBoundaryNeedsItemKey]);

    let shadowed = Strategy::cursor("since", CursorSource::last_item("t")).tiebreak("since", CursorSource::last_item("id")).more_when(MoreSignal::NonEmptyPage);
    assert_eq!(validate_strategy(&shadowed.build()), vec![PaginationError::DuplicateInput("since".into())]);
}

#[test]
fn g2_boundary_tracker_drops_redelivered_ties_and_detects_no_progress() {
    let leg = CursorLeg::single("since", CursorSource::last_item("t")).inclusive("id");
    let mut tracker = BoundaryTracker::new(&leg).unwrap();

    let page1 = json!([{"id": 1, "t": 10}, {"id": 2, "t": 20}, {"id": 3, "t": 20}]);
    let (kept, next) = tracker.accept(page1.as_array().unwrap().iter().collect()).unwrap();
    assert_eq!(kept.len(), 3);
    assert_eq!(next, Some(vec![json!(20)]));

    // Server returns t >= 20: ids 2 and 3 again, then new rows.
    let page2 = json!([{"id": 2, "t": 20}, {"id": 3, "t": 20}, {"id": 4, "t": 20}, {"id": 5, "t": 30}]);
    let (kept, next) = tracker.accept(page2.as_array().unwrap().iter().collect()).unwrap();
    let ids: Vec<_> = kept.iter().map(|record| record["id"].clone()).collect();
    assert_eq!(ids, vec![json!(4), json!(5)]);
    assert_eq!(next, Some(vec![json!(30)]));

    // A page made only of already-delivered ties cannot make progress.
    let page3 = json!([{"id": 5, "t": 30}]);
    assert_eq!(tracker.accept(page3.as_array().unwrap().iter().collect()), Err(BoundaryError::NoProgress));
}

// ---------------------------------------------------------------------------
// G3. Follow-up request shape (carry, path, method)
// ---------------------------------------------------------------------------

#[test]
fn g3_shopify_link_carries_only_limit_and_size_is_adjustable() {
    let strategy = Strategy::next_link(LinkSource::link_header("next"))
        .prev(LinkSource::link_header("previous"))
        .page_size(PageSize::new("limit").server_default(50).max(250).adjustable())
        .follow_up(FollowUp::carry(Carry::Only(vec!["limit".into(), "fields".into()])));
    let endpoint = EndpointDef::get("ListProducts", "/products.json", "Products")
        .query(&["fields", "status", "vendor"])
        .collection(Collection::at("products").paginate(strategy));
    ok(&endpoint);
    let surface = knobs_of(&endpoint);
    assert!(surface.contains(&Knob::AdjustPageSize));
}

#[test]
fn g3_aip158_skip_is_first_request_only() {
    let strategy = Strategy::cursor("pageToken", CursorSource::body("nextPageToken"))
        .page_size(PageSize::new("pageSize").adjustable())
        .follow_up(FollowUp::carry(Carry::AllExcept(vec!["skip".into()])))
        .total(ValueSource::body("totalSize"));
    let endpoint = EndpointDef::get("ListBooks", "/v1/{parent}/books", "ListBooksResponse")
        .query(&["filter", "skip"])
        .collection(Collection::at("books").paginate(strategy));
    ok(&endpoint);
    let Resolved::Paginated { primary, .. } = resolve(None, &endpoint) else { panic!() };
    assert_eq!(primary.effective_carry(), Carry::AllExcept(vec!["skip".into()]));
    assert!(knobs_of(&endpoint).contains(&Knob::AdjustPageSize));
}

#[test]
fn g3_follow_up_validation() {
    let unknown_carry = EndpointDef::get("L", "/l", "Vec<T>").collection(Collection::root().paginate(
        Strategy::cursor("c", CursorSource::body("n")).follow_up(FollowUp::carry(Carry::Only(vec!["nope".into()]))),
    ));
    assert_eq!(validate_endpoint(None, &unknown_carry), vec![PaginationError::UnknownCarryParam("nope".into())]);

    let link_with_path = Strategy::next_link(LinkSource::body("next")).follow_up(FollowUp::to(Method::Get, "/elsewhere"));
    assert!(validate_strategy(&link_with_path).contains(&PaginationError::NextLinkWithFollowUpPath));

    let unknown_follow_up_path = EndpointDef::post("S", "/{index}/_search", "R").collection(Collection::at("hits").paginate(
        Strategy::cursor(InputParam::body("c"), CursorSource::body("n")).follow_up(FollowUp::to(Method::Post, "/{other}/scroll")),
    ));
    assert_eq!(validate_endpoint(None, &unknown_follow_up_path), vec![PaginationError::UnknownPathParam("other".into())]);

    // Cursor sent in a body on a GET follow-up.
    let body_on_get = EndpointDef::post("S", "/s", "R").collection(Collection::at("hits").paginate(
        Strategy::cursor(InputParam::body("c"), CursorSource::body("n")).follow_up(FollowUp::carry(Carry::All).method(Method::Get)),
    ));
    assert_eq!(validate_endpoint(None, &body_on_get), vec![PaginationError::BodyInputOnGet("c".into())]);
}

#[test]
fn g3_default_carry_depends_on_kind() {
    assert_eq!(Strategy::cursor("c", CursorSource::body("n")).build().effective_carry(), Carry::All);
    assert_eq!(Strategy::next_link(LinkSource::body("n")).build().effective_carry(), Carry::Nothing);
}

// ---------------------------------------------------------------------------
// G4. Multi-level item selection
// ---------------------------------------------------------------------------

#[test]
fn g4_selector_parses_records_and_projection() {
    let reddit = ItemSelector::parse("data.children[*].data");
    assert_eq!(reddit.fan_outs(), 1);
    assert_eq!(reddit.projection, JsonPath::parse("data"));
    let page = json!({"kind": "Listing", "data": {"after": "t3_b", "children": [
        {"kind": "t3", "data": {"name": "t3_a"}}, {"kind": "t3", "data": {"name": "t3_b"}}]}});
    let names: Vec<_> = reddit.items(&page).iter().map(|item| item["name"].clone()).collect();
    assert_eq!(names, vec![json!("t3_a"), json!("t3_b")]);

    // Two fan-outs flatten (multi-search: responses[].hits.hits[]).
    let msearch = ItemSelector::parse("responses[*].hits.hits[*]._source");
    assert_eq!(msearch.fan_outs(), 2);
    let page = json!({"responses": [
        {"hits": {"hits": [{"_source": {"n": 1}, "sort": [1]}]}},
        {"hits": {"hits": [{"_source": {"n": 2}, "sort": [2]}, {"_source": {"n": 3}, "sort": [3]}]}}]});
    assert_eq!(msearch.items(&page).len(), 3);
    // Item-derived cursors read the record (the hit), not the projected item.
    let last_record = *msearch.records(&page).last().unwrap();
    assert_eq!(JsonPath::parse("sort").lookup(last_record), Some(&json!([3])));

    assert!(ItemSelector::parse("").is_root_array());
    assert_eq!(ItemSelector::parse("data"), ItemSelector::parse("data[*]"));
}

// ---------------------------------------------------------------------------
// G5. Page size changed mid-iteration
// ---------------------------------------------------------------------------

#[test]
fn g5_adjustable_page_size_rules() {
    let page_number = Strategy::page_number("page", MoreSignal::NonEmptyPage).page_size(PageSize::new("per_page").adjustable());
    assert_eq!(validate_strategy(&page_number.build()), vec![PaginationError::AdjustablePageSizeOnPageNumber]);

    let link_body = Strategy::next_link(LinkSource::body("next")).page_size(PageSize::new(InputParam::body("size")).adjustable());
    assert_eq!(validate_strategy(&link_body.build()), vec![PaginationError::AdjustableLinkSizeNotQuery]);

    let offset = Strategy::offset("offset", MoreSignal::NonEmptyPage).page_size(PageSize::new("limit").adjustable()).build();
    assert_eq!(validate_strategy(&offset), vec![]);
    assert!(strategy_knobs(&offset).contains(&Knob::AdjustPageSize));

    let fixed = Strategy::offset("offset", MoreSignal::NonEmptyPage).page_size(PageSize::new("limit")).build();
    assert!(!strategy_knobs(&fixed).contains(&Knob::AdjustPageSize));
}

// ---------------------------------------------------------------------------
// G6. Terminal sentinel values and G7. index origin
// ---------------------------------------------------------------------------

#[test]
fn g6_salesforce_bulk_locator_null_string_is_terminal() {
    let strategy = Strategy::cursor("locator", CursorSource::header("Sforce-Locator"))
        .page_size(PageSize::new("maxRecords"))
        .terminal_value("null");
    let endpoint = EndpointDef::get("GetQueryResults", "/jobs/query/{jobId}/results", "Csv").collection(Collection::root().paginate(strategy.clone()));
    ok(&endpoint);
    assert!(strategy.is_terminal(Some(&json!("null"))));
    assert!(strategy.is_terminal(Some(&json!(""))));
    assert!(strategy.is_terminal(None));
    assert!(!strategy.is_terminal(Some(&json!("MjAwMDAw"))));
}

#[test]
fn g7_scim_one_based_start_index() {
    let strategy = Strategy::offset("startIndex", MoreSignal::total_items(ValueSource::body("totalResults")))
        .one_based()
        .page_size(PageSize::new("count"));
    let endpoint = EndpointDef::get("ListUsers", "/Users", "ListResponse").query(&["filter"]).collection(Collection::at("Resources").paginate(strategy));
    ok(&endpoint);
    assert_eq!(knobs_of(&endpoint)[0], Knob::StartOffset { base: IndexBase::OneBased });
}

// ---------------------------------------------------------------------------
// External patterns (not in research.md / inventory.md)
// ---------------------------------------------------------------------------

#[test]
fn ext_patterns_validate_without_model_changes() {
    let endpoints = vec![
        // Google AIP-158: empty next_page_token ends; page_size may change.
        EndpointDef::get("AipList", "/v1/{parent}/things", "ListThingsResponse").collection(Collection::at("things").paginate(
            Strategy::cursor("page_token", CursorSource::body("next_page_token")).page_size(PageSize::new("page_size").adjustable()),
        )),
        // Mastodon: Link header next/prev carrying max_id/min_id.
        EndpointDef::get("HomeTimeline", "/api/v1/timelines/home", "Vec<Status>").collection(Collection::root().paginate(
            Strategy::next_link(LinkSource::link_header("next")).prev(LinkSource::link_header("prev")).page_size(PageSize::new("limit").server_default(20).max(40)),
        )),
        // Mastodon alternative: max_id from the last status id.
        EndpointDef::get("HomeTimelineIds", "/api/v1/timelines/home", "Vec<Status>").collection(Collection::root().paginate(
            Strategy::cursor("max_id", CursorSource::last_item("id")).backward("min_id", CursorSource::first_item("id")).more_when(MoreSignal::NonEmptyPage).resumable(),
        )),
        // X API v2: pagination_token <- meta.next_token.
        EndpointDef::get("UserTweets", "/2/users/{id}/tweets", "TweetsResponse").collection(Collection::at("data").paginate(
            Strategy::cursor("pagination_token", CursorSource::body("meta.next_token")).page_size(PageSize::new("max_results").min(5).max(100)),
        )),
        // WordPress REST: X-WP-TotalPages.
        EndpointDef::get("ListPosts", "/wp/v2/posts", "Vec<Post>").collection(Collection::root().paginate(
            Strategy::page_number("page", MoreSignal::total_pages(ValueSource::header("X-WP-TotalPages")))
                .page_size(PageSize::new("per_page").server_default(10).max(100))
                .total(ValueSource::header("X-WP-Total")),
        )),
        // OData / Microsoft Graph $skiptoken inside @odata.nextLink.
        EndpointDef::get("ListUsers", "/users", "UsersPage").collection(Collection::at("value").paginate(
            Strategy::next_link(LinkSource::body_segments(["@odata.nextLink"])).page_size(PageSize::new("$top")),
        )),
        // Azure Cosmos DB: continuation token header in and out.
        EndpointDef::get("ListDocuments", "/dbs/{db}/colls/{coll}/docs", "Docs").collection(Collection::at("Documents").paginate(
            Strategy::cursor(InputParam::header("x-ms-continuation"), CursorSource::header("x-ms-continuation")).page_size(PageSize::new(InputParam::header("x-ms-max-item-count"))),
        )),
        // Salesforce REST query: relative nextRecordsUrl; `done` agrees.
        EndpointDef::get("Query", "/services/data/v60.0/query", "QueryResult").query(&["q"]).collection(Collection::at("records").paginate(
            Strategy::next_link(LinkSource::body("nextRecordsUrl")).total(ValueSource::body("totalSize")),
        )),
        // Reddit listing: after <- data.after, items data.children[].data.
        EndpointDef::get("Hot", "/r/{subreddit}/hot", "Listing").collection(Collection::at("data.children[*].data").paginate(
            Strategy::cursor("after", CursorSource::body("data.after")).backward("before", CursorSource::body("data.before")).page_size(PageSize::new("limit").server_default(25).max(100)),
        )),
    ];
    for endpoint in &endpoints {
        ok(endpoint);
        assert!(knobs_of(endpoint).contains(&Knob::Next), "{}", endpoint.id);
    }
}

// ---------------------------------------------------------------------------
// Provider gaps are not model gaps
// ---------------------------------------------------------------------------

#[test]
fn openai_vector_store_search_would_be_expressible_if_documented() {
    // OpenAI documents no request input that consumes `next_page`. Were one
    // added (hypothetically `page` in the body), the model already covers it.
    let hypothetical = EndpointDef::post("SearchVectorStore", "/vector_stores/{id}/search", "VectorStoreSearchResultsPage")
        .collection(Collection::at("data").paginate(
            Strategy::cursor(InputParam::body("page"), CursorSource::body("next_page")).more_when(MoreSignal::has_more("has_more")),
        ));
    ok(&hypothetical);
}

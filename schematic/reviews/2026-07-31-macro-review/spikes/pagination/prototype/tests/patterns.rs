//! research.md section 6 awkward patterns, default/override/opt-out
//! resolution, validation, and link trust.

use pagination_prototype::*;
use serde_json::json;

fn errors_of(strategy: impl Into<Strategy>) -> Vec<PaginationError> {
    validate_strategy(&strategy.into())
}

// ---------------------------------------------------------------------------
// Default, override, opt-out (principles 2 and 3)
// ---------------------------------------------------------------------------

#[test]
fn collection_without_api_default_is_unpaginated() {
    let api = ApiDef::new("Plain", "https://example.com")
        .endpoint(EndpointDef::get("List", "/things", "Vec<Thing>").collection(Collection::root()));
    assert_eq!(api.resolve("List"), Some(Resolved::Unpaginated { items: ItemSelector::root() }));
    assert_eq!(caller_knobs(&api.resolve("List").unwrap()), vec![]);
}

#[test]
fn api_default_applies_only_to_collections() {
    let default = Strategy::page_number("page", MoreSignal::NonEmptyPage).build();
    let api = ApiDef::new("Forge", "https://example.com")
        .default_pagination(default.clone())
        .endpoint(EndpointDef::get("List", "/things", "Vec<Thing>").collection(Collection::root()))
        .endpoint(EndpointDef::get("Get", "/things/{id}", "Thing"))
        .endpoint(EndpointDef::get("Override", "/others", "Others").collection(Collection::at("items").paginate(Strategy::offset("skip", MoreSignal::NonEmptyPage))))
        .endpoint(EndpointDef::get("OptOut", "/small", "Vec<Small>").collection(Collection::root().unpaginated()));

    assert!(matches!(api.resolve("List"), Some(Resolved::Paginated { origin: Origin::ApiDefault, ref primary, .. }) if *primary == default));
    assert_eq!(api.resolve("Get"), Some(Resolved::NotACollection));
    assert!(matches!(
        api.resolve("Override"),
        Some(Resolved::Paginated { origin: Origin::Endpoint, primary: Strategy { kind: PageStrategy::Offset { .. }, .. }, .. })
    ));
    assert!(matches!(api.resolve("OptOut"), Some(Resolved::Unpaginated { .. })));
    assert_eq!(validate_api(&api), vec![]);
}

#[test]
fn root_array_can_be_recognized_from_vec_type_name() {
    assert_eq!(ItemSelector::infer_root("Vec<Issue>"), Some(ItemSelector::root()));
    assert_eq!(ItemSelector::infer_root("ListModelsResponse"), None);
}

// ---------------------------------------------------------------------------
// research.md section 6, one test per pattern (numbers match the list)
// ---------------------------------------------------------------------------

#[test]
fn p01_next_url_in_body_with_dotted_key() {
    // Microsoft Graph: `@odata.nextLink` contains a dot; segments keep it whole.
    let graph = Strategy::next_link(LinkSource::body_segments(["@odata.nextLink"])).build();
    let PageStrategy::NextLink { next: LinkSource::Field { source: ValueSource::Body(path) }, .. } = &graph.kind else {
        panic!()
    };
    let page = json!({ "@odata.nextLink": "https://graph.microsoft.com/v1.0/users?$skiptoken=X", "value": [] });
    assert_eq!(path.lookup(&page), Some(&json!("https://graph.microsoft.com/v1.0/users?$skiptoken=X")));
    assert_eq!(JsonPath::parse("@odata.nextLink").lookup(&page), None, "parse would split the key");
}

#[test]
fn p02_cursor_in_post_body_is_valid_on_post_and_rejected_on_get() {
    let notion = Strategy::cursor(InputParam::body("start_cursor"), CursorSource::body("next_cursor"))
        .more_when(MoreSignal::has_more("has_more"))
        .page_size(PageSize::new(InputParam::body("page_size")).server_default(100).max(100));
    let post = EndpointDef::post("QueryDatabase", "/databases/{id}/query", "QueryResults").collection(Collection::at("results").paginate(notion.clone()));
    assert_eq!(validate_endpoint(None, &post), vec![]);
    let get = EndpointDef::get("QueryDatabase", "/databases/{id}/query", "QueryResults").collection(Collection::at("results").paginate(notion));
    assert_eq!(
        validate_endpoint(None, &get),
        vec![PaginationError::BodyInputOnGet("page_size".into()), PaginationError::BodyInputOnGet("start_cursor".into())]
    );
}

#[test]
fn next_link_on_post_must_state_the_follow_up_method() {
    let unspecified = EndpointDef::post("Search", "/search", "Results")
        .collection(Collection::at("items").paginate(Strategy::next_link(LinkSource::body("next"))));
    assert_eq!(validate_endpoint(None, &unspecified), vec![PaginationError::NextLinkMethodUnspecified]);
    // A POST search whose server returns a GET-able link (Graph-style).
    let stated = EndpointDef::post("Search", "/search", "Results").collection(
        Collection::at("items")
            .paginate(Strategy::next_link(LinkSource::body("next")).follow_up(FollowUp::carry(Carry::Nothing).method(Method::Get))),
    );
    assert_eq!(validate_endpoint(None, &stated), vec![]);
}

#[test]
fn p03_has_more_with_item_derived_cursor() {
    // Stripe: starting_after = data[-1].id, stop on has_more == false.
    let stripe = Strategy::cursor("starting_after", CursorSource::last_item("id"))
        .backward("ending_before", CursorSource::first_item("id"))
        .more_when(MoreSignal::has_more("has_more"));
    assert_eq!(errors_of(stripe), vec![]);
}

#[test]
fn p04_inverted_polarity() {
    // Jira legacy search: startAt + maxResults, `isLast` true on the last page.
    let jira = Strategy::offset("startAt", MoreSignal::is_last("isLast")).page_size(PageSize::new("maxResults").server_default(50));
    assert!(matches!(jira.build().kind, PageStrategy::Offset { more: MoreSignal::Flag { polarity: Polarity::TrueMeansLast, .. }, .. }));
}

#[test]
fn p05_nested_cursor_path() {
    let slack = Strategy::cursor("cursor", CursorSource::body("response_metadata.next_cursor"))
        .page_size(PageSize::new("limit").max(1000))
        .build();
    let PageStrategy::Cursor { forward, more: None, .. } = &slack.kind else { panic!() };
    let CursorSource::Field { source: ValueSource::Body(path) } = &forward.parts[0].source else { panic!() };
    let page = json!({ "members": [], "response_metadata": { "next_cursor": "dXNlcjpVMEc5V0ZYTlo=" } });
    assert_eq!(path.lookup(&page), Some(&json!("dXNlcjpVMEc5V0ZYTlo=")));
    assert_eq!(validate_strategy(&slack), vec![], "cursor absence is the end signal");
}

#[test]
fn p06_bidirectional_cursor_legs_must_differ() {
    let same = Strategy::cursor("after", CursorSource::body("last_id"))
        .backward("after", CursorSource::body("first_id"))
        .more_when(MoreSignal::has_more("has_more"));
    assert_eq!(errors_of(same), vec![PaginationError::DuplicateInput("after".into())]);
}

#[test]
fn p07_sort_value_array_cursor_into_body() {
    // Elasticsearch search_after: copy hits.hits[-1].sort into the body.
    let es = Strategy::cursor(InputParam::body("search_after"), CursorSource::last_item("sort"))
        .more_when(MoreSignal::NonEmptyPage)
        .page_size(PageSize::new(InputParam::body("size")).max(10_000));
    let endpoint = EndpointDef::post("Search", "/{index}/_search", "SearchResponse")
        .collection(Collection::at("hits.hits[*]._source").paginate(es));
    assert_eq!(validate_endpoint(None, &endpoint), vec![]);
}

#[test]
fn p08_short_page_is_rejected_for_cursor_and_needs_a_size() {
    let cursor = Strategy::cursor("cursor", CursorSource::body("next")).more_when(MoreSignal::FullPage);
    assert_eq!(errors_of(cursor), vec![PaginationError::FullPageOnCursor]);
    let unsized_page = Strategy::page_number("page", MoreSignal::FullPage);
    assert_eq!(errors_of(unsized_page), vec![PaginationError::FullPageWithoutPageSize]);
    let sized = Strategy::page_number("page", MoreSignal::FullPage).page_size(PageSize::new("per_page"));
    assert_eq!(errors_of(sized), vec![]);
}

#[test]
fn p10_link_header_rel_names_vary() {
    let shopify = Strategy::next_link(LinkSource::link_header("next"))
        .prev(LinkSource::link_header("previous"))
        .page_size(PageSize::new("limit").server_default(50).max(250))
        .build();
    assert_eq!(strategy_knobs(&shopify), vec![
        Knob::PageSize { param: "limit".into(), min: 1, max: Some(250), server_default: Some(50) },
        Knob::Next,
        Knob::Prev,
    ]);
}

#[test]
fn p11_total_variants() {
    let pages_header = Strategy::page_number("page", MoreSignal::total_pages(ValueSource::header("X-Total-Pages")));
    assert_eq!(errors_of(pages_header), vec![]);
    let pages_on_offset = Strategy::offset("offset", MoreSignal::total_pages(ValueSource::header("X-Total-Pages")));
    assert_eq!(errors_of(pages_on_offset), vec![PaginationError::PageCountOnNonPageNumber]);
    let items_on_offset = Strategy::offset("startAt", MoreSignal::total_items(ValueSource::body("total")));
    assert_eq!(errors_of(items_on_offset), vec![]);
}

#[test]
fn p12_time_window_as_item_cursor() {
    let window = Strategy::cursor("since", CursorSource::last_item("created_at")).more_when(MoreSignal::NonEmptyPage);
    assert_eq!(errors_of(window), vec![]);
}

#[test]
fn p13_mixed_strategies_need_unique_labels_and_a_primary() {
    let keyset = || Strategy::next_link(LinkSource::link_header("next")).constant("pagination", "keyset");
    let duplicate = EndpointDef::get("L", "/l", "Vec<T>").collection(
        Collection::root().paginate(Strategy::page_number("page", MoreSignal::NonEmptyPage)).or_paginate("keyset", keyset()).or_paginate("keyset", keyset()),
    );
    assert_eq!(validate_endpoint(None, &duplicate), vec![PaginationError::DuplicateLabel("keyset".into())]);

    let orphan = EndpointDef::get("L", "/l", "Vec<T>").collection(Collection::root().or_paginate("keyset", keyset()));
    assert_eq!(validate_endpoint(None, &orphan), vec![PaginationError::AlternatesWithoutPrimary]);

    let opted_out = EndpointDef::get("L", "/l", "Vec<T>").collection(Collection::root().unpaginated().or_paginate("keyset", keyset()));
    assert_eq!(validate_endpoint(None, &opted_out), vec![PaginationError::AlternatesOnUnpaginated]);

    let bad_label = EndpointDef::get("L", "/l", "Vec<T>")
        .collection(Collection::root().paginate(Strategy::page_number("page", MoreSignal::NonEmptyPage)).or_paginate("Key Set", keyset()));
    assert_eq!(validate_endpoint(None, &bad_label), vec![PaginationError::InvalidLabel("Key Set".into())]);
}

#[test]
fn p14_constants_may_not_shadow_strategy_inputs() {
    let clash = Strategy::page_number("page", MoreSignal::NonEmptyPage).constant("page", "1");
    assert_eq!(validate_strategy(&clash), vec![PaginationError::DuplicateInput("page".into())]);
}

#[test]
fn p16_page_size_bounds_must_be_consistent() {
    let default_over_max = Strategy::page_number("page", MoreSignal::NonEmptyPage).page_size(PageSize::new("per_page").server_default(200).max(100));
    assert!(matches!(errors_of(default_over_max).as_slice(), [PaginationError::InvalidPageSize { .. }]));
    let zero_min = Strategy::page_number("page", MoreSignal::NonEmptyPage).page_size(PageSize::new("per_page").min(0));
    assert!(matches!(errors_of(zero_min).as_slice(), [PaginationError::InvalidPageSize { .. }]));
    // Not client-selectable (fixed size): simply no PageSize, so no knob.
    let fixed = Strategy::cursor("start_cursor", CursorSource::body("next_cursor")).build();
    assert!(!strategy_knobs(&fixed).iter().any(|knob| matches!(knob, Knob::PageSize { .. })));
}

#[test]
fn p17_relay_connection_with_projection() {
    let relay = Strategy::cursor(InputParam::body("variables.after"), CursorSource::body("data.repository.issues.pageInfo.endCursor"))
        .more_when(MoreSignal::has_more("data.repository.issues.pageInfo.hasNextPage"))
        .page_size(PageSize::new(InputParam::body("variables.first")).max(100));
    let endpoint = EndpointDef::post("Graphql", "/graphql", "IssuesQuery")
        .collection(Collection::at("data.repository.issues.edges[*].node").paginate(relay));
    assert_eq!(validate_endpoint(None, &endpoint), vec![]);
    let collection = endpoint.collection.unwrap();
    assert_eq!(collection.items.projection, JsonPath::parse("node"));
}

#[test]
fn cursor_derived_from_items_without_more_signal_is_rejected() {
    let stripe_without_has_more = Strategy::cursor("starting_after", CursorSource::last_item("id"));
    assert_eq!(errors_of(stripe_without_has_more), vec![PaginationError::CursorWithoutEndSignal]);
}

// ---------------------------------------------------------------------------
// p18: link trust
// ---------------------------------------------------------------------------

#[test]
fn p18_next_links_are_untrusted_by_default() {
    let base = "https://api.bitbucket.org/2.0";
    let same = LinkTrust::SameOrigin;
    assert!(is_link_trusted(base, "https://api.bitbucket.org/2.0/repositories/w?page=2", &same));
    assert!(is_link_trusted(base, "https://API.bitbucket.org:443/x", &same));
    assert!(is_link_trusted(base, "/2.0/repositories/w?page=2", &same));
    assert!(!is_link_trusted(base, "https://evil.example/steal", &same));
    assert!(!is_link_trusted(base, "http://api.bitbucket.org/2.0/x", &same), "downgrade");
    assert!(!is_link_trusted(base, "https://user:pw@api.bitbucket.org/x", &same), "userinfo");
    assert!(!is_link_trusted(base, "//evil.example/x", &same), "scheme-relative");
    assert!(!is_link_trusted(base, "javascript:alert(1)", &same));

    let allow = LinkTrust::AllowOrigins(vec!["https://cdn.example".into()]);
    assert!(is_link_trusted(base, "https://cdn.example/page2", &allow));
    assert!(!is_link_trusted(base, "http://cdn.example/page2", &allow));
}

#[test]
fn p18_allow_list_entries_must_be_https_origins() {
    let bad = Strategy::next_link(LinkSource::link_header("next")).trust(LinkTrust::AllowOrigins(vec!["http://x.example".into(), "x.example".into()]));
    assert_eq!(
        errors_of(bad),
        vec![PaginationError::InvalidTrustedOrigin("http://x.example".into()), PaginationError::InvalidTrustedOrigin("x.example".into())]
    );
}

#[test]
fn serde_shape_is_tagged_and_readable() {
    let strategy = Strategy::cursor("after_id", CursorSource::body("last_id")).more_when(MoreSignal::has_more("has_more")).build();
    let value = serde_json::to_value(&strategy).unwrap();
    assert_eq!(value["kind"]["kind"], "cursor");
    assert_eq!(value["kind"]["forward"]["parts"][0]["param"], json!({ "in": "query", "name": "after_id" }));
    assert_eq!(value["kind"]["forward"]["parts"][0]["source"], json!({ "from": "field", "source": { "from": "body", "at": ["last_id"] } }));
    assert_eq!(value["kind"]["more"]["signal"], "flag");
}

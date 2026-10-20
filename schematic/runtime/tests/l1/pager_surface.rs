//! The pager's caller surface: single-page fetch (R11a), page metadata and
//! the item stream (R11), the two positive capability cases of R10, link
//! resolution and origin checks, and the privacy of pager error text (R36).

use std::time::Duration;

use serde_json::json;
use wiremock::MockServer;
use wiremock::matchers::path;

use schematic_runtime::pager::{ContinuationProblem, IncompleteReason, PagerError};
use schematic_runtime::{Credential, Deadline, TransportErrorKind};

use crate::pager_support::{
    BodyNextPrevious, FullyAuthored, LastIdHasMore, LinkCursor, NestedMeta, NextField, PageLinkRel, ProjectedOffset,
    ProviderError, client, ids, json, names, numbered, on, on_query, otherwise, pairs, queries, received,
};
use crate::support::SENTINEL;

type Error = PagerError<ProviderError>;

/// Asserts that neither the error's nor the incomplete result's text
/// carries the sentinel.
fn assert_quiet(error: &Error) {
    let text = error.to_string();
    assert!(!text.contains(SENTINEL), "{text}");
    assert!(!text.contains("127.0.0.1"), "no URL in {text}");
}

// ---------------------------------------------------------------------------
// Single-page fetch and page metadata (R11a, R11)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn fetch_page_sends_one_request_with_the_cursor_and_size_and_returns_that_page() {
    let server = MockServer::start().await;
    on_query(
        &server,
        "after",
        "c0",
        json(json!({ "data": [{ "id": "x" }, { "id": "y" }], "last_id": "y", "has_more": true })),
    )
    .await;
    let client = client(&server);

    let page = client.send(LastIdHasMore::get()).pager().after("c0").page_size(5).fetch_page().await.expect("page");

    assert_eq!(names(&page.items), ["x", "y"]);
    assert_eq!(page.number, 1);
    assert_eq!(page.meta.request.page, Some(1));
    assert_eq!(page.meta.status, 200);
    assert_eq!(queries(&server).await, [pairs(&[("after", "c0"), ("limit", "5")])], "no continuation is followed");
}

#[tokio::test]
async fn an_ordinary_send_of_a_paginated_request_returns_the_wrapper_without_a_page_number() {
    let server = MockServer::start().await;
    let body = json!({ "data": [{ "id": "x" }], "last_id": "x", "has_more": true });
    otherwise(&server, json(body.clone())).await;
    let client = client(&server);

    let response = client.send(LastIdHasMore::get()).full().await.expect("response");

    assert_eq!(response.data(), Some(&body));
    assert_eq!(response.meta.request.page, None);
    assert_eq!(queries(&server).await, [pairs(&[])], "an optional size is not sent");
}

#[tokio::test]
async fn each_page_carries_its_number_in_the_page_and_the_request_metadata() {
    let server = MockServer::start().await;
    on_query(&server, "page", "1", json(json!({ "data": numbered(1..=2), "meta": { "hasnext": true } }))).await;
    on_query(&server, "page", "2", json(json!({ "data": numbered(3..=3), "meta": { "hasnext": false } }))).await;
    let client = client(&server);

    let mut pages = client.send(NestedMeta::get()).pager().pages();
    for expected in 1..=2 {
        let page = pages.next().await.expect("page").expect("a page");
        assert_eq!(page.number, expected);
        assert_eq!(page.meta.request.page, Some(expected));
        assert_eq!(page.meta.request.endpoint_id, "NestedMeta");
    }
}

#[tokio::test]
async fn items_flattens_every_page_in_order() {
    let server = MockServer::start().await;
    on_query(&server, "page", "1", json(json!({ "data": numbered(1..=2), "meta": { "hasnext": true } }))).await;
    on_query(&server, "page", "2", json(json!({ "data": numbered(3..=3), "meta": { "hasnext": false } }))).await;
    let client = client(&server);

    let mut items = client.send(NestedMeta::get()).pager().pages().items();
    let mut seen = Vec::new();
    while let Some(item) = items.next().await.expect("item") {
        seen.push(item);
        assert_eq!(items.items_delivered(), seen.len() as u64);
    }

    assert_eq!(ids(&seen), [1, 2, 3]);
    assert_eq!(items.pages_fetched(), 2);
    assert_eq!(received(&server).await.len(), 2);
}

// ---------------------------------------------------------------------------
// Positive capability cases (R10)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn every_control_is_available_on_a_fully_authored_endpoint() {
    let server = MockServer::start().await;
    on_query(
        &server,
        "after",
        "a",
        json(json!({ "data": [{ "id": "b" }, { "id": "c" }], "has_more": true, "total": 4 })),
    )
    .await;
    on_query(&server, "after", "c", json(json!({ "data": [{ "id": "d" }], "has_more": false, "total": 4 }))).await;
    on_query(&server, "before", "d", json(json!({ "data": [{ "id": "b" }, { "id": "c" }], "has_more": false }))).await;
    on_query(&server, "before", "z", json(json!({ "data": [{ "id": "y" }], "has_more": false }))).await;
    let client = client(&server);
    let deadline = Deadline::after(Duration::from_secs(30));

    let configured = client.send(FullyAuthored::get()).pager().credential(Credential::access_token("unused"));
    assert_eq!(configured.page_limit(7).configured_page_limit(), 7);

    let mut pages = client
        .send(FullyAuthored::get())
        .pager()
        .page_size(2)
        .after("a")
        .page_limit(5)
        .anonymous()
        .deadline(&deadline)
        .pages();
    let first = pages.next().await.expect("page 1").expect("a page");
    pages.set_page_size(3);
    let second = pages.next().await.expect("page 2").expect("a page");
    assert_eq!((names(&first.items), names(&second.items)), (vec!["b", "c"], vec!["d"]));
    assert_eq!(pages.total(), Some(4));
    let back = pages.prev().await.expect("prev").expect("a page");
    assert_eq!(names(&back.items), ["b", "c"]);

    let before = client.send(FullyAuthored::get()).pager().before("z").fetch_page().await.expect("page");
    assert_eq!(names(&before.items), ["y"]);

    assert_eq!(
        queries(&server).await,
        [
            pairs(&[("after", "a"), ("limit", "2")]),
            pairs(&[("after", "c"), ("limit", "3")]),
            pairs(&[("before", "d"), ("limit", "3")]),
            pairs(&[("before", "z")]),
        ]
    );
}

#[tokio::test]
async fn page_number_and_offset_endpoints_take_their_start_and_size_controls() {
    let server = MockServer::start().await;
    on_query(&server, "page", "2", json(numbered(1..=1))).await;
    on_query(&server, "offset", "4", json(json!({ "rows": [{ "value": { "id": 5 } }] }))).await;
    let client = client(&server);

    let numbered_page = client.send(PageLinkRel::get()).pager().page(2).page_size(10).fetch_page().await.expect("page");
    let offset_page = client.send(ProjectedOffset::get()).pager().offset(4).page_size(2).fetch_page().await.expect("page");

    assert_eq!((ids(&numbered_page.items), ids(&offset_page.items)), (vec![1], vec![5]));
    assert_eq!(
        queries(&server).await,
        [pairs(&[("page", "2"), ("per_page", "10")]), pairs(&[("offset", "4"), ("limit", "2")])]
    );
}

// ---------------------------------------------------------------------------
// Links
// ---------------------------------------------------------------------------

#[tokio::test]
async fn relative_links_resolve_against_the_response_url() {
    for (link, followed) in [("?page=2", "/v2/repos/list"), ("../next", "/v2/next")] {
        let server = MockServer::start().await;
        otherwise(&server, json(json!({ "values": numbered(1..=1), "next": link }))).await;
        on(&server, path("/v2/next"), json(json!({ "values": numbered(2..=2) }))).await;
        on_query(&server, "page", "2", json(json!({ "values": numbered(2..=2) }))).await;
        let client = client(&server);

        let items = client.send(BodyNextPrevious::get()).pager().collect_all().await.expect("complete");

        assert_eq!(ids(&items), [1, 2], "{link}");
        let second = &received(&server).await[1];
        assert_eq!(second.url.path(), followed, "{link}");
    }
}

#[tokio::test]
async fn distinct_next_links_are_ambiguous_and_identical_ones_are_followed() {
    let server = MockServer::start().await;
    let uri = server.uri();
    otherwise(
        &server,
        json(numbered(1..=1))
            .append_header("link", format!(r#"<{uri}/link-cursor?cursor=a>; rel="next""#))
            .append_header("link", format!(r#"<{uri}/link-cursor?cursor=b>; rel="next""#)),
    )
    .await;
    let client = client(&server);

    let mut pages = client.send(LinkCursor::get()).pager().pages();
    pages.next().await.expect("page").expect("a page");
    let error = pages.next().await.expect_err("ambiguous");
    assert!(
        matches!(error, PagerError::InvalidContinuation { problem: ContinuationProblem::AmbiguousLink, .. }),
        "{error:?}"
    );

    let server = MockServer::start().await;
    let uri = server.uri();
    otherwise(
        &server,
        json(numbered(1..=1))
            .append_header("link", format!(r#"<{uri}/link-cursor?cursor=a>; rel="next""#))
            .append_header("link", format!(r#"<{uri}/link-cursor?cursor=a#frag>; rel="next""#)),
    )
    .await;
    on_query(&server, "cursor", "a", json(numbered(2..=2))).await;
    let client = crate::pager_support::client(&server);

    let items = client.send(LinkCursor::get()).pager().collect_all().await.expect("complete");
    assert_eq!(ids(&items), [1, 2]);
}

#[tokio::test]
async fn a_cross_origin_next_link_is_refused_before_any_request_reaches_it() {
    let origin_b = MockServer::start().await;
    otherwise(&origin_b, json(json!({ "values": [] }))).await;
    let server = MockServer::start().await;
    let next = format!("{}/v2/repos/list?page=2&token={SENTINEL}", origin_b.uri());
    otherwise(&server, json(json!({ "values": numbered(1..=1), "next": next }))).await;
    let client = client(&server);

    let incomplete = client.send(BodyNextPrevious::get()).pager().collect_all().await.expect_err("refused");

    assert_eq!(incomplete.reason, IncompleteReason::CrossOriginRejected);
    assert_eq!(ids(&incomplete.items), [1]);
    let error = incomplete.error.as_ref().expect("cause");
    let PagerError::Transport(transport) = error else { panic!("{error:?}") };
    assert!(matches!(transport.kind, TransportErrorKind::CrossOriginRejected { .. }), "{transport:?}");
    assert_eq!(transport.request.page, Some(2));
    assert_quiet(error);
    assert!(!incomplete.to_string().contains(SENTINEL));
    assert_eq!(received(&origin_b).await.len(), 0);
}

// ---------------------------------------------------------------------------
// Privacy of pager error text (R36)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_repeated_secret_cursor_is_not_in_the_error_text() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": numbered(1..=1), "next": SENTINEL }))).await;
    let client = client(&server);

    let incomplete = client.send(NextField::get()).pager().after(SENTINEL).collect_all().await.expect_err("repeat");

    let error = incomplete.error.as_ref().expect("cause");
    assert!(matches!(error, PagerError::RepeatedContinuation { .. }), "{error:?}");
    assert_quiet(error);
    assert!(!incomplete.to_string().contains(SENTINEL));
}

#[tokio::test]
async fn an_invalid_secret_continuation_is_not_in_the_error_text() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": numbered(1..=1), "next": { "token": SENTINEL } }))).await;
    let client = client(&server);
    let mut pages = client.send(NextField::get()).pager().pages();
    pages.next().await.expect("page").expect("a page");
    let structured = pages.next().await.expect_err("structured cursor for a query");
    assert!(
        matches!(structured, PagerError::InvalidContinuation { problem: ContinuationProblem::Cursor, .. }),
        "{structured:?}"
    );
    assert_quiet(&structured);

    let server = MockServer::start().await;
    otherwise(&server, json(numbered(1..=1)).insert_header("link", format!(r#"<http://[{SENTINEL}>; rel="next""#))).await;
    let client = crate::pager_support::client(&server);
    let mut pages = client.send(LinkCursor::get()).pager().pages();
    pages.next().await.expect("page").expect("a page");
    let bad_link = pages.next().await.expect_err("unparseable link");
    assert!(
        matches!(bad_link, PagerError::InvalidContinuation { problem: ContinuationProblem::Link, .. }),
        "{bad_link:?}"
    );
    assert_quiet(&bad_link);

    let server = MockServer::start().await;
    let uri = server.uri();
    otherwise(
        &server,
        json(numbered(1..=1)).insert_header(
            "link",
            format!(r#"<{uri}/x?s={SENTINEL}1>; rel="next", <{uri}/x?s={SENTINEL}2>; rel="next""#),
        ),
    )
    .await;
    let client = crate::pager_support::client(&server);
    let mut pages = client.send(LinkCursor::get()).pager().pages();
    pages.next().await.expect("page").expect("a page");
    assert_quiet(&pages.next().await.expect_err("ambiguous"));
}

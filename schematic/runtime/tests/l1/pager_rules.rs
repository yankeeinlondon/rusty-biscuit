//! Traversal rules through the runtime pager against a local mock server:
//! end detection (R12), page sizes (R13, R2a), loop and stall detection
//! (R14), composite and inclusive cursors (R15), malformed pages (R17), and
//! `collect_all` limits and reasons (R21).

use serde_json::json;
use wiremock::{MockServer, ResponseTemplate};

use schematic_runtime::pager::{ContinuationProblem, DEFAULT_PAGE_LIMIT, IncompleteReason, PagerError, PagerErrorKind};
use schematic_runtime::{ErrorKind, TransportError, TransportErrorKind};

use crate::pager_support::{
    BodyNextPrevious, CompositeField, InclusiveSince, IsLastOffset, LastIdHasMore, LinkCursor, NestedChildren, NestedMeta, NextField,
    OneBasedOffsetTotal, PageLimitLinkTotal, PageLinkRel, ProjectedOffset, ProviderError, SentDefaultFullPage,
    ServerDefaultFullPage, TokenAllExceptSkip, client, ids, json, names, numbered, on_query, otherwise, pairs,
    queries, received,
};

type Error = PagerError<ProviderError>;

/// The reason of a pre-send invalid-argument refusal.
fn refusal(error: &Error) -> String {
    match error {
        PagerError::Transport(TransportError {
            kind: TransportErrorKind::InvalidArgument(argument),
            ..
        }) => argument.reason().to_owned(),
        other => panic!("not an invalid argument: {other:?}"),
    }
}

fn problem(error: &Error) -> ContinuationProblem {
    match error {
        PagerError::InvalidContinuation { problem, .. } => *problem,
        other => panic!("not an invalid continuation: {other:?}"),
    }
}

async fn hits(server: &MockServer) -> usize {
    received(server).await.len()
}

// ---------------------------------------------------------------------------
// End detection (R12)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn an_explicit_false_ends_despite_a_stale_cursor() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": [{ "id": "a" }], "last_id": "a", "has_more": false }))).await;
    let client = client(&server);

    let items = client.send(LastIdHasMore::get()).pager().collect_all().await.expect("complete");

    assert_eq!(names(&items), ["a"]);
    assert_eq!(hits(&server).await, 1);
}

#[tokio::test]
async fn an_explicit_true_without_a_cursor_is_a_missing_continuation_after_the_page() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": [{ "id": "a" }], "has_more": true }))).await;
    let client = client(&server);

    let mut pages = client.send(LastIdHasMore::get()).pager().pages();
    let page = pages.next().await.expect("the page itself is valid").expect("a page");
    assert_eq!(names(&page.items), ["a"]);
    let error = pages.next().await.expect_err("no continuation");
    assert!(matches!(error, PagerError::MissingContinuation { .. }), "{error:?}");
    assert_eq!(error.response_meta().map(|meta| meta.request.page), Some(Some(1)), "the page that lacked it");

    let incomplete = client.send(LastIdHasMore::get()).pager().collect_all().await.expect_err("incomplete");
    assert_eq!(incomplete.reason, IncompleteReason::InvalidContinuation);
    assert_eq!(names(&incomplete.items), ["a"], "the valid page's items are kept");
    assert_eq!(incomplete.pages, 1);
    assert_eq!(hits(&server).await, 2, "one request per traversal; the error sends nothing");
}

#[tokio::test]
async fn without_a_signal_a_missing_null_or_empty_cursor_ends() {
    for body in [
        json!({ "data": numbered(1..=1) }),
        json!({ "data": numbered(1..=1), "next": null }),
        json!({ "data": numbered(1..=1), "next": "" }),
    ] {
        let server = MockServer::start().await;
        otherwise(&server, json(body.clone())).await;
        let client = client(&server);

        let items = client.send(NextField::get()).pager().collect_all().await.expect("complete");

        assert_eq!(ids(&items), [1], "{body}");
        assert_eq!(hits(&server).await, 1, "{body}");
    }
}

#[tokio::test]
async fn zero_and_false_cursors_are_values_not_ends() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": numbered(1..=1), "next": 0 }))).await;
    on_query(&server, "after", "0", json(json!({ "data": numbered(2..=2), "next": false }))).await;
    on_query(&server, "after", "false", json(json!({ "data": numbered(3..=3), "next": null }))).await;
    let client = client(&server);

    let items = client.send(NextField::get()).pager().collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2, 3]);
    assert_eq!(queries(&server).await, [pairs(&[]), pairs(&[("after", "0")]), pairs(&[("after", "false")])]);
}

#[tokio::test]
async fn a_flag_of_the_wrong_type_is_an_invalid_more_signal_never_completion() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": [{ "id": "a" }], "last_id": "a", "has_more": "no" }))).await;
    let client = client(&server);

    let mut pages = client.send(LastIdHasMore::get()).pager().pages();
    assert_eq!(names(&pages.next().await.expect("page").expect("a page").items), ["a"]);
    let error = pages.next().await.expect_err("wrong type");
    assert_eq!(problem(&error), ContinuationProblem::MoreSignal);
    assert!(!pages.is_finished());
    assert_eq!(hits(&server).await, 1);
}

#[tokio::test]
async fn a_true_means_last_flag_ends_when_true() {
    let server = MockServer::start().await;
    on_query(&server, "startAt", "0", json(json!({ "values": numbered(1..=2), "isLast": false }))).await;
    on_query(&server, "startAt", "2", json(json!({ "values": numbered(3..=3), "isLast": true }))).await;
    let client = client(&server);

    let items = client.send(IsLastOffset::get()).pager().collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2, 3]);
    assert_eq!(queries(&server).await, [pairs(&[("startAt", "0")]), pairs(&[("startAt", "2")])]);
}

#[tokio::test]
async fn an_empty_page_number_page_that_claims_more_is_an_error() {
    let server = MockServer::start().await;
    on_query(&server, "page", "1", json(json!({ "data": numbered(1..=1), "meta": { "hasnext": true } }))).await;
    on_query(&server, "page", "2", json(json!({ "data": [], "meta": { "hasnext": true } }))).await;
    let client = client(&server);

    let incomplete = client.send(NestedMeta::get()).pager().collect_all().await.expect_err("contradiction");

    assert!(matches!(incomplete.error, Some(PagerError::EmptyPageClaimsMore { .. })), "{incomplete:?}");
    assert_eq!(incomplete.reason, IncompleteReason::RepeatedContinuation);
    assert_eq!(ids(&incomplete.items), [1]);
    assert_eq!(hits(&server).await, 2);
}

#[tokio::test]
async fn an_empty_cursor_page_with_a_continuation_continues() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": [], "next": "n2" }))).await;
    on_query(&server, "after", "n2", json(json!({ "data": numbered(1..=1), "next": null }))).await;
    let client = client(&server);

    let items = client.send(NextField::get()).pager().collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1]);
    assert_eq!(hits(&server).await, 2);
}

#[tokio::test]
async fn a_missing_total_falls_back_to_a_non_empty_page_for_offsets() {
    let server = MockServer::start().await;
    on_query(&server, "startIndex", "1", json(json!({ "Resources": numbered(1..=2) }))).await;
    on_query(&server, "startIndex", "3", json(json!({ "Resources": [] }))).await;
    let client = client(&server);

    let mut pages = client.send(OneBasedOffsetTotal::get()).pager().page_size(2).pages();
    let mut all = Vec::new();
    while let Some(page) = pages.next().await.expect("page") {
        all.extend(page.items);
    }

    assert_eq!(ids(&all), [1, 2]);
    assert_eq!(pages.total(), None);
    assert_eq!(hits(&server).await, 2);
}

#[tokio::test]
async fn a_malformed_or_negative_total_is_an_invalid_total() {
    for total in [json!(-1), json!("many"), json!(1.5), json!(true)] {
        let server = MockServer::start().await;
        otherwise(&server, json(json!({ "Resources": numbered(1..=2), "totalResults": total }))).await;
        let client = client(&server);

        let incomplete = client.send(OneBasedOffsetTotal::get()).pager().collect_all().await.expect_err("bad total");

        assert_eq!(problem(incomplete.error.as_ref().expect("cause")), ContinuationProblem::Total, "{total}");
        assert_eq!(incomplete.reason, IncompleteReason::InvalidContinuation);
        assert_eq!(ids(&incomplete.items), [1, 2], "{total}");
        assert_eq!(hits(&server).await, 1, "{total}");
    }

    let server = MockServer::start().await;
    otherwise(&server, json(numbered(1..=1)).insert_header("x-total-count", "-3")).await;
    let client = client(&server);
    let mut pages = client.send(PageLimitLinkTotal::get()).pager().pages();
    pages.next().await.expect("page").expect("a page");
    assert_eq!(problem(&pages.next().await.expect_err("informational total")), ContinuationProblem::Total);
}

#[tokio::test]
async fn a_full_page_rule_uses_the_sent_default_size_on_every_request_unless_overridden() {
    let server = MockServer::start().await;
    on_query(&server, "page", "1", json(numbered(1..=2))).await;
    on_query(&server, "page", "2", json(numbered(3..=3))).await;
    let client = client(&server);

    let items = client.send(SentDefaultFullPage::get()).pager().collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2, 3], "the short second page ends");
    let page = client.send(SentDefaultFullPage::get()).pager().fetch_page().await.expect("one page");
    assert_eq!(ids(&page.items), [1, 2]);
    assert_eq!(
        queries(&server).await,
        [
            pairs(&[("page", "1"), ("limit", "2")]),
            pairs(&[("page", "2"), ("limit", "2")]),
            pairs(&[("page", "1"), ("limit", "2")]),
        ]
    );

    let server = MockServer::start().await;
    on_query(&server, "page", "1", json(numbered(1..=3))).await;
    on_query(&server, "page", "2", json(numbered(4..=4))).await;
    let client = crate::pager_support::client(&server);

    let items = client.send(SentDefaultFullPage::get()).pager().page_size(3).collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2, 3, 4]);
    assert_eq!(
        queries(&server).await,
        [pairs(&[("page", "1"), ("limit", "3")]), pairs(&[("page", "2"), ("limit", "3")])]
    );
}

#[tokio::test]
async fn a_server_default_is_never_sent_and_does_not_establish_a_full_page() {
    let server = MockServer::start().await;
    on_query(&server, "page", "1", json(numbered(1..=2))).await;
    on_query(&server, "page", "2", json(numbered(3..=3))).await;
    on_query(&server, "page", "3", json(json!([]))).await;
    let client = client(&server);

    let items = client.send(ServerDefaultFullPage::get()).pager().collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2, 3]);
    assert_eq!(
        queries(&server).await,
        [pairs(&[("page", "1")]), pairs(&[("page", "2")]), pairs(&[("page", "3")])],
        "the short page does not end without a known size, and the server default is not sent"
    );

    let server = MockServer::start().await;
    on_query(&server, "page", "1", json(numbered(1..=2))).await;
    on_query(&server, "page", "2", json(numbered(3..=3))).await;
    let client = crate::pager_support::client(&server);

    let items = client.send(ServerDefaultFullPage::get()).pager().page_size(2).collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2, 3]);
    assert_eq!(hits(&server).await, 2, "with the caller's size the short page ends");
}

#[tokio::test]
async fn page_number_arithmetic_is_checked_before_the_next_send() {
    let server = MockServer::start().await;
    let uri = server.uri();
    otherwise(&server, json(numbered(1..=1)).insert_header("link", format!(r#"<{uri}/page-link?page=x>; rel="next""#)))
        .await;
    let client = client(&server);

    let mut pages = client.send(PageLinkRel::get()).pager().page(u64::MAX).pages();
    let last = pages.next().await.expect("the last representable page").expect("a page");
    assert_eq!(ids(&last.items), [1]);
    let error = pages.next().await.expect_err("overflow");
    assert!(refusal(&error).contains("overflow"), "{error:?}");
    assert_eq!(queries(&server).await, [pairs(&[("page", &u64::MAX.to_string())])]);

    let incomplete = client.send(PageLinkRel::get()).pager().page(u64::MAX).collect_all().await.expect_err("overflow");
    assert_eq!(incomplete.reason, IncompleteReason::InvalidArgument);
    assert_eq!(ids(&incomplete.items), [1]);
}

#[tokio::test]
async fn a_start_page_below_the_base_is_refused_without_a_request() {
    let server = MockServer::start().await;
    otherwise(&server, json(numbered(1..=1))).await;
    let client = client(&server);

    let error = client.send(PageLinkRel::get()).pager().page(0).fetch_page().await.expect_err("below base");

    assert!(refusal(&error).contains("first page, 1"), "{error:?}");
    assert_eq!(hits(&server).await, 0);
}

#[tokio::test]
async fn an_offset_advances_by_source_records_from_its_zero_base() {
    let server = MockServer::start().await;
    let rows = json!([{ "value": { "id": 1 } }, { "value": { "id": 2 } }, { "value": { "id": 3 } }]);
    on_query(&server, "offset", "0", json(json!({ "rows": rows }))).await;
    on_query(&server, "offset", "3", json(json!({ "rows": [] }))).await;
    let client = client(&server);

    let items = client.send(ProjectedOffset::get()).pager().collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2, 3], "the caller receives the projection");
    assert_eq!(queries(&server).await, [pairs(&[("offset", "0")]), pairs(&[("offset", "3")])]);
}

// ---------------------------------------------------------------------------
// Page sizes (R13)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_size_outside_the_declared_bounds_is_refused_naming_the_bound_with_zero_requests() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": [], "has_more": false }))).await;
    let client = client(&server);

    let above = client.send(LastIdHasMore::get()).pager().page_size(101).fetch_page().await.expect_err("above");
    assert!(refusal(&above).contains("maximum, 100"), "{above:?}");
    let PagerError::Transport(transport) = &above else { unreachable!() };
    assert_eq!(transport.request.endpoint_id, "LastIdHasMore", "the error carries request metadata");
    assert_eq!(transport.request.page, Some(1));

    let below = client.send(LastIdHasMore::get()).pager().page_size(0).pages().next().await.expect_err("below");
    assert!(refusal(&below).contains("minimum, 1"), "{below:?}");

    let incomplete = client.send(LastIdHasMore::get()).pager().page_size(101).collect_all().await.expect_err("above");
    assert_eq!(incomplete.reason, IncompleteReason::InvalidArgument);
    assert_eq!((incomplete.items.len(), incomplete.pages), (0, 1));

    assert_eq!(hits(&server).await, 0);
}

#[tokio::test]
async fn an_adjusted_size_applies_from_the_next_request_and_is_validated() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "books": [{ "id": "a" }], "nextPageToken": "t2" }))).await;
    let client = client(&server);

    let mut pages = client.send(TokenAllExceptSkip::get()).pager().page_size(1).pages();
    pages.next().await.expect("page").expect("a page");
    pages.set_page_size(51);
    let error = pages.next().await.expect_err("above the maximum");

    assert!(refusal(&error).contains("maximum, 50"), "{error:?}");
    assert_eq!(hits(&server).await, 1);
}

#[tokio::test]
async fn an_optional_size_is_omitted_when_unset() {
    let server = MockServer::start().await;
    otherwise(&server, json(numbered(1..=1))).await;
    let client = client(&server);

    client.send(PageLinkRel::get()).pager().collect_all().await.expect("complete");

    assert_eq!(queries(&server).await, [pairs(&[("page", "1")])]);
}

// ---------------------------------------------------------------------------
// Composite and inclusive cursors (R15)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_composite_cursor_sends_every_part_and_ends_when_all_are_absent() {
    let server = MockServer::start().await;
    on_query(&server, "id_before", "9", json(json!({ "items": numbered(1..=1), "next": { "created": "t1", "id": 7 } })))
        .await;
    on_query(&server, "id_before", "7", json(json!({ "items": numbered(2..=2), "next": null }))).await;
    let client = client(&server);

    let items = client
        .send(CompositeField::get())
        .pager()
        .after(("t9".to_owned(), "9".to_owned()))
        .collect_all()
        .await
        .expect("complete");

    assert_eq!(ids(&items), [1, 2]);
    assert_eq!(
        queries(&server).await,
        [
            pairs(&[("created_before", "t9"), ("id_before", "9")]),
            pairs(&[("created_before", "t1"), ("id_before", "7")]),
        ]
    );
}

#[tokio::test]
async fn a_composite_cursor_with_only_some_parts_is_a_missing_continuation() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "items": numbered(1..=1), "next": { "created": "t1" } }))).await;
    let client = client(&server);

    let mut pages = client.send(CompositeField::get()).pager().pages();
    assert_eq!(ids(&pages.next().await.expect("page").expect("a page").items), [1]);
    let error = pages.next().await.expect_err("partial cursor");

    assert!(matches!(error, PagerError::MissingContinuation { .. }), "{error:?}");
    assert_eq!(hits(&server).await, 1);
}

#[tokio::test]
async fn an_inclusive_boundary_drops_redelivered_ties_and_fails_on_a_page_of_only_ties() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!([{ "id": 1, "t": 10 }, { "id": 2, "t": 20 }, { "id": 3, "t": 20 }]))).await;
    on_query(
        &server,
        "since",
        "20",
        json(json!([{ "id": 2, "t": 20 }, { "id": 3, "t": 20 }, { "id": 4, "t": 20 }, { "id": 5, "t": 30 }])),
    )
    .await;
    on_query(&server, "since", "30", json(json!([{ "id": 5, "t": 30 }]))).await;
    let client = client(&server);

    let mut pages = client.send(InclusiveSince::get()).pager().pages();
    let first = pages.next().await.expect("page 1").expect("a page");
    let second = pages.next().await.expect("page 2").expect("a page");
    assert_eq!((ids(&first.items), ids(&second.items)), (vec![1, 2, 3], vec![4, 5]));
    let error = pages.next().await.expect_err("only ties");
    assert!(matches!(error, PagerError::BoundaryNoProgress { .. }), "{error:?}");
    assert_eq!(
        queries(&server).await,
        [pairs(&[]), pairs(&[("since", "20")]), pairs(&[("since", "30")])]
    );

    let incomplete = client.send(InclusiveSince::get()).pager().collect_all().await.expect_err("no progress");
    assert_eq!(incomplete.reason, IncompleteReason::RepeatedContinuation);
    assert_eq!(ids(&incomplete.items), [1, 2, 3, 4, 5]);
}

// ---------------------------------------------------------------------------
// Loops and terminal failure (R14)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_cursor_cycle_is_a_repeated_continuation() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": numbered(1..=1), "next": "b" }))).await;
    on_query(&server, "after", "b", json(json!({ "data": numbered(2..=2), "next": "c" }))).await;
    on_query(&server, "after", "c", json(json!({ "data": numbered(3..=3), "next": "b" }))).await;
    let client = client(&server);

    let incomplete = client.send(NextField::get()).pager().collect_all().await.expect_err("a to b to a");

    assert!(matches!(incomplete.error, Some(PagerError::RepeatedContinuation { .. })), "{incomplete:?}");
    assert_eq!(incomplete.reason, IncompleteReason::RepeatedContinuation);
    assert_eq!(ids(&incomplete.items), [1, 2, 3]);
    assert_eq!(hits(&server).await, 3);
}

#[tokio::test]
async fn a_return_to_the_starting_cursor_is_a_repeated_continuation() {
    let server = MockServer::start().await;
    on_query(&server, "after", "a", json(json!({ "data": numbered(1..=1), "next": "a" }))).await;
    let client = client(&server);

    let mut pages = client.send(NextField::get()).pager().after("a").pages();
    pages.next().await.expect("page").expect("a page");
    let error = pages.next().await.expect_err("back to the start");

    assert!(matches!(error, PagerError::RepeatedContinuation { .. }), "{error:?}");
    assert_eq!(hits(&server).await, 1);
}

#[tokio::test]
async fn a_repeated_link_is_a_repeated_continuation() {
    let server = MockServer::start().await;
    let uri = server.uri();
    let link = format!(r#"<{uri}/link-cursor?cursor=c2>; rel="next""#);
    otherwise(&server, json(numbered(1..=1)).insert_header("link", link)).await;
    let client = client(&server);

    let incomplete = client.send(LinkCursor::get()).pager().collect_all().await.expect_err("repeated link");

    assert!(matches!(incomplete.error, Some(PagerError::RepeatedContinuation { .. })), "{incomplete:?}");
    assert_eq!(ids(&incomplete.items), [1, 1]);
    assert_eq!(hits(&server).await, 2);
}

#[tokio::test]
async fn stepping_back_then_forward_again_is_not_a_repeated_continuation() {
    let server = MockServer::start().await;
    let uri = server.uri();
    let page_one = json(json!({
        "values": numbered(1..=2),
        "next": format!("{uri}/v2/repos/list?page=2"),
    }));
    on_query(&server, "page", "1", page_one.clone()).await;
    otherwise(&server, page_one).await;
    on_query(
        &server,
        "page",
        "2",
        json(json!({ "values": numbered(3..=3), "previous": format!("{uri}/v2/repos/list?page=1") })),
    )
    .await;
    let first_client = client(&server);

    let mut pages = first_client.send(BodyNextPrevious::get()).pager().pages();
    let mut seen = Vec::new();
    seen.push(ids(&pages.next().await.expect("page 1").expect("a page").items));
    seen.push(ids(&pages.next().await.expect("page 2").expect("a page").items));
    seen.push(ids(&pages.prev().await.expect("back to page 1").expect("a page").items));
    seen.push(ids(&pages.next().await.expect("page 2 again").expect("a page").items));
    assert_eq!(seen, [vec![1, 2], vec![3], vec![1, 2], vec![3]]);
    assert!(pages.next().await.expect("end").is_none());

    // A cycle inside one forward run after turning around is still caught.
    let looping = MockServer::start().await;
    let loop_uri = looping.uri();
    on_query(
        &looping,
        "page",
        "2",
        json(json!({ "values": numbered(3..=3), "next": format!("{loop_uri}/v2/repos/list?page=2"), "previous": format!("{loop_uri}/v2/repos/list?page=1") })),
    )
    .await;
    otherwise(&looping, json(json!({ "values": numbered(1..=2), "next": format!("{loop_uri}/v2/repos/list?page=2") }))).await;
    let looping_client = client(&looping);
    let mut pages = looping_client.send(BodyNextPrevious::get()).pager().pages();
    pages.next().await.expect("page 1").expect("a page");
    pages.next().await.expect("page 2").expect("a page");
    pages.prev().await.expect("back to page 1").expect("a page");
    pages.next().await.expect("page 2 again").expect("a page");
    let error = pages.next().await.expect_err("page 2 names itself");
    assert!(matches!(error, PagerError::RepeatedContinuation { .. }), "{error:?}");
}

#[tokio::test]
async fn a_failed_pager_stays_failed_without_another_request() {
    let server = MockServer::start().await;
    otherwise(&server, ResponseTemplate::new(500).set_body_json(json!({ "message": "boom" }))).await;
    let client = client(&server);

    let mut pages = client.send(LastIdHasMore::get()).pager().pages();
    let error = pages.next().await.expect_err("500");
    let PagerError::Api { error: api, meta } = &error else { panic!("{error:?}") };
    assert_eq!(meta.status, 500);
    assert_eq!(api.declared(), Some(&ProviderError { message: "boom".into() }));
    for _ in 0..2 {
        let again = pages.next().await.expect_err("still failed");
        assert_eq!(again, PagerError::AlreadyFailed { first: PagerErrorKind::Api });
        assert_eq!(again.incomplete_reason(), IncompleteReason::HttpError);
    }
    assert_eq!(hits(&server).await, 1);
}

// ---------------------------------------------------------------------------
// Malformed pages (R17)
// ---------------------------------------------------------------------------

/// Asserts `error` is a malformed success that kept `body`.
fn assert_malformed(error: &Error, body: &[u8]) {
    let PagerError::Api { error: api, meta } = error else { panic!("not an API error: {error:?}") };
    assert_eq!(api.kind, ErrorKind::MalformedSuccess, "{error:?}");
    assert_eq!(&api.body[..], body);
    assert_eq!(meta.status, 200);
}

#[tokio::test]
async fn a_malformed_collection_is_a_malformed_success_with_its_body_and_no_items() {
    let cases: [&[u8]; 5] = [
        br#"{"has_more":false}"#,
        br#"{"data":{"id":"a"},"has_more":false}"#,
        br#"{"data":[{"id":"a"},{"id":5}],"has_more":false}"#,
        br#"{"data":[{"id":"a"}],"has_more":false"#,
        b"not json",
    ];
    for body in cases {
        let server = MockServer::start().await;
        otherwise(&server, ResponseTemplate::new(200).set_body_raw(body, "application/json")).await;
        let client = client(&server);

        let error = client.send(LastIdHasMore::get()).pager().fetch_page().await.expect_err("malformed");

        assert_malformed(&error, body);
    }
}

#[tokio::test]
async fn a_record_without_its_projection_is_malformed() {
    let body: &[u8] = br#"{"data":{"after":null,"children":[{"data":{"id":"a"}},{"kind":"t3"}]}}"#;
    let server = MockServer::start().await;
    otherwise(&server, ResponseTemplate::new(200).set_body_raw(body, "application/json")).await;
    let client = client(&server);

    let error = client.send(NestedChildren::get()).pager().fetch_page().await.expect_err("malformed");

    assert_malformed(&error, body);
}

#[tokio::test]
async fn a_malformed_second_page_contributes_no_items_to_collect_all() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": [{ "id": "a" }], "last_id": "a", "has_more": true }))).await;
    on_query(&server, "after", "a", json(json!({ "data": [{ "id": "b" }, { "id": 2 }], "has_more": false }))).await;
    let client = client(&server);

    let incomplete = client.send(LastIdHasMore::get()).pager().collect_all().await.expect_err("malformed");

    assert_eq!(incomplete.reason, IncompleteReason::HttpError);
    assert_eq!(names(&incomplete.items), ["a"]);
    assert_eq!(incomplete.pages, 2);
    let cause = incomplete.error.expect("cause");
    assert_eq!(cause.response_meta().map(|meta| meta.request.page), Some(Some(2)));
}

// ---------------------------------------------------------------------------
// collect_all limits (R21)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn the_default_page_limit_is_one_hundred_and_zero_is_refused_without_a_request() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": numbered(1..=1), "meta": { "hasnext": false } }))).await;
    let client = client(&server);

    assert_eq!(DEFAULT_PAGE_LIMIT, 100);
    assert_eq!(client.send(NestedMeta::get()).pager().configured_page_limit(), DEFAULT_PAGE_LIMIT);
    let incomplete = client.send(NestedMeta::get()).pager().page_limit(0).collect_all().await.expect_err("zero");

    assert_eq!(incomplete.reason, IncompleteReason::InvalidArgument);
    assert!(refusal(incomplete.error.as_ref().expect("cause")).contains("page limit"));
    assert!(incomplete.items.is_empty());
    assert_eq!(hits(&server).await, 0);
}

#[tokio::test]
async fn completion_exactly_at_the_page_limit_is_complete() {
    let server = MockServer::start().await;
    on_query(&server, "page", "1", json(json!({ "data": numbered(1..=1), "meta": { "hasnext": true } }))).await;
    on_query(&server, "page", "2", json(json!({ "data": numbered(2..=2), "meta": { "hasnext": false } }))).await;
    let client = client(&server);

    let items = client.send(NestedMeta::get()).pager().page_limit(2).collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2]);
}

#[tokio::test]
async fn reaching_the_page_limit_first_is_incomplete_with_the_items_so_far() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": numbered(9..=9), "meta": { "hasnext": true } }))).await;
    on_query(&server, "page", "1", json(json!({ "data": numbered(1..=1), "meta": { "hasnext": true } }))).await;
    on_query(&server, "page", "2", json(json!({ "data": numbered(2..=2), "meta": { "hasnext": true } }))).await;
    let client = client(&server);

    let incomplete = client.send(NestedMeta::get()).pager().page_limit(2).collect_all().await.expect_err("limit");

    assert_eq!(incomplete.reason, IncompleteReason::PageLimitReached);
    assert_eq!(ids(&incomplete.items), [1, 2]);
    assert_eq!(incomplete.pages, 2);
    assert!(incomplete.error.is_none());
    assert_eq!(hits(&server).await, 2);
}

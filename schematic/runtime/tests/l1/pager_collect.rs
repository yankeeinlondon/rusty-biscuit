//! `collect_all` outcomes (R21) and follow-up request shapes (R16) through
//! the runtime pager against a local mock server.
//!
//! The first section is the R21 table: one traversal per
//! [`IncompleteReason`], asserting the reason, the items kept, the pages
//! counted, and the error with its evidence. Some reasons are also covered by
//! the rule suites; this file keeps the whole table in one place.

use std::time::Duration;

use bytes::Bytes;
use http::Method;
use schematic_runtime::pager::spec::{
    Boundary, Carry, CursorLeg, CursorPart, CursorSource, FollowUp, Input, Kind, LinkSource, PageSize, Source, Step,
};
use schematic_runtime::pager::{
    ContinuationProblem, CursorKind, IncompleteReason, ItemSelector, NextLinkKind, PageSizeKnob, PagerError, Strategy,
};
use schematic_runtime::{
    ApiDefinition, Client, Deadline, ErrorKind, FailureCategory, FormPart, OriginPolicy, RequestBody, RequestParts,
    TransportError, TransportErrorKind,
};
use serde_json::{Value, json};
use wiremock::matchers::{path, query_param};
use wiremock::{MockServer, Request, ResponseTemplate};

use crate::pager_support::{
    BodyNextPrevious, Def, LastIdHasMore, LinkCursor, NestedMeta, NextField, ProviderError, TestApi, client, endpoint,
    ids, json, names, numbered, on, on_query, otherwise, pairs, queries, query, received, size,
};
use crate::support::{builder, closed_port_url};

type Error = PagerError<ProviderError>;

const DATA: ItemSelector = ItemSelector {
    record: &[Step::Key("data"), Step::Each],
    projection: &[],
};

/// A cursor sent at `param`, read from the body's `next`.
const fn next_at(param: Input) -> CursorPart {
    CursorPart {
        param,
        source: CursorSource::Field(Source::Body(&["next"])),
    }
}

const CURSOR_QUERY: &[CursorPart] = &[next_at(Input::Query("cursor"))];
const AFTER_QUERY: &[CursorPart] = &[next_at(Input::Query("after"))];

const fn next_cursor(parts: &'static [CursorPart]) -> Kind {
    Kind::Cursor {
        forward: CursorLeg {
            parts,
            boundary: Boundary::Exclusive,
        },
        backward: None,
        more: None,
        resumable: false,
    }
}

endpoint!(
    ShopifyProducts,
    NextLinkKind,
    crate::support::Item,
    "/products",
    Strategy {
        kind: Kind::NextLink {
            next: LinkSource::LinkHeader { rel: "next" },
            prev: None,
        },
        size: Some(size(Input::Query("limit"))),
        constants: &[],
        total: None,
        terminal_values: &[],
        follow_up: Some(FollowUp {
            method: Some(Method::GET),
            path: None,
            carry: Carry::Only(&["fields"]),
        }),
        session: None,
        partition: None,
    },
    ItemSelector::ROOT
);
impl PageSizeKnob for ShopifyProducts {}

endpoint!(
    SearchNext,
    CursorKind,
    crate::pager_support::Named,
    "/search/{id}",
    Strategy {
        kind: Kind::Cursor {
            forward: CursorLeg {
                parts: &[
                    CursorPart {
                        param: Input::Query("cursor"),
                        source: CursorSource::Field(Source::Body(&["next", "q"])),
                    },
                    CursorPart {
                        param: Input::Header("X-Cursor"),
                        source: CursorSource::Field(Source::Body(&["next", "h"])),
                    },
                    CursorPart {
                        param: Input::Body(&["page", "cursor"]),
                        source: CursorSource::Field(Source::Body(&["next", "b"])),
                    },
                ],
                boundary: Boundary::Exclusive,
            },
            backward: None,
            more: None,
            resumable: false,
        },
        size: None,
        constants: &[],
        total: None,
        terminal_values: &[],
        follow_up: Some(FollowUp {
            method: Some(Method::POST),
            path: Some("/search/{id}/next"),
            carry: Carry::Only(&["q", "filter", "page"]),
        }),
        session: None,
        partition: None,
    },
    DATA
);

endpoint!(
    FormAllExcept,
    CursorKind,
    crate::support::Item,
    "/form",
    Strategy {
        kind: next_cursor(CURSOR_QUERY),
        size: None,
        constants: &[],
        total: None,
        terminal_values: &[],
        follow_up: Some(FollowUp {
            method: None,
            path: None,
            carry: Carry::AllExcept(&["once"]),
        }),
        session: None,
        partition: None,
    },
    DATA
);

endpoint!(
    UploadOnly,
    CursorKind,
    crate::support::Item,
    "/upload",
    Strategy {
        kind: next_cursor(CURSOR_QUERY),
        size: None,
        constants: &[],
        total: None,
        terminal_values: &[],
        follow_up: Some(FollowUp {
            method: None,
            path: None,
            carry: Carry::Only(&["fields"]),
        }),
        session: None,
        partition: None,
    },
    DATA
);

endpoint!(
    SizedDefault,
    CursorKind,
    crate::support::Item,
    "/sized",
    Def::new(next_cursor(AFTER_QUERY))
        .size(PageSize {
            sent_default: Some(20),
            max: Some(100),
            ..size(Input::Query("per_page"))
        })
        .done(),
    DATA
);
impl PageSizeKnob for SizedDefault {}

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

/// A client whose cross-origin policy allows `origin`.
fn allowing(server: &MockServer, origin: &str) -> Client<TestApi> {
    let definition = ApiDefinition::new("https://api.example.invalid").cross_origin(OriginPolicy::allow_origins([origin]));
    builder(definition, &server.uri()).build().expect("client builds")
}

fn body_json(request: &Request) -> Value {
    serde_json::from_slice(&request.body).expect("a JSON body")
}

fn header_values(request: &Request, name: &str) -> Vec<String> {
    request
        .headers
        .get_all(name)
        .iter()
        .map(|value| value.to_str().expect("text").to_owned())
        .collect()
}

fn content_type(request: &Request) -> Option<String> {
    request.headers.get("content-type").map(|value| value.to_str().expect("text").to_owned())
}

fn values_of(query: &[(String, String)], name: &str) -> Vec<String> {
    query.iter().filter(|(key, _)| key == name).map(|(_, value)| value.clone()).collect()
}

/// Page one of a `next`-cursor endpoint: item 1, continuing at `b`.
async fn first_page_continues_at_b(server: &MockServer) {
    otherwise(server, json(json!({ "data": numbered(1..=1), "next": "b" }))).await;
}

// ---------------------------------------------------------------------------
// The R21 table: one test per incomplete reason
// ---------------------------------------------------------------------------

#[tokio::test]
async fn page_limit_reached_keeps_every_page_counts_an_empty_one_and_has_no_error() {
    let server = MockServer::start().await;
    first_page_continues_at_b(&server).await;
    on_query(&server, "after", "b", json(json!({ "data": [], "next": "c" }))).await;
    on_query(&server, "after", "c", json(json!({ "data": numbered(3..=3), "next": "d" }))).await;
    on_query(&server, "after", "d", json(json!({ "data": numbered(4..=4), "next": null }))).await;
    let client = client(&server);

    let incomplete = client.send(NextField::get()).pager().page_limit(3).collect_all().await.expect_err("limit");

    assert_eq!(incomplete.reason, IncompleteReason::PageLimitReached);
    assert_eq!(ids(&incomplete.items), [1, 3]);
    assert_eq!(incomplete.pages, 3);
    assert!(incomplete.error.is_none());
    assert_eq!(received(&server).await.len(), 3, "no request past the limit");
}

#[tokio::test]
async fn repeated_continuation_keeps_the_page_that_repeated_and_sends_nothing_more() {
    let server = MockServer::start().await;
    first_page_continues_at_b(&server).await;
    on_query(&server, "after", "b", json(json!({ "data": numbered(2..=2), "next": "b" }))).await;
    let client = client(&server);

    let incomplete = client.send(NextField::get()).pager().collect_all().await.expect_err("loop");

    assert_eq!(incomplete.reason, IncompleteReason::RepeatedContinuation);
    assert_eq!(ids(&incomplete.items), [1, 2]);
    assert_eq!(incomplete.pages, 2);
    let error = incomplete.error.as_ref().expect("cause");
    assert!(matches!(error, PagerError::RepeatedContinuation { .. }), "{error:?}");
    let meta = error.response_meta().expect("the repeating response");
    assert_eq!((meta.status.as_u16(), meta.request.page), (200, Some(2)));
    assert_eq!(received(&server).await.len(), 2);
}

#[tokio::test]
async fn http_error_keeps_earlier_items_and_the_failed_page_body() {
    let body: &[u8] = br#"{"message":"boom"}"#;
    let server = MockServer::start().await;
    first_page_continues_at_b(&server).await;
    on_query(&server, "after", "b", ResponseTemplate::new(500).set_body_raw(body, "application/json")).await;
    let client = client(&server);

    let incomplete = client.send(NextField::get()).pager().collect_all().await.expect_err("500");

    assert_eq!(incomplete.reason, IncompleteReason::HttpError);
    assert_eq!(ids(&incomplete.items), [1]);
    assert_eq!(incomplete.pages, 2);
    let Some(PagerError::Api { error, meta }) = &incomplete.error else { panic!("{incomplete:?}") };
    assert_eq!((meta.status.as_u16(), meta.request.page), (500, Some(2)));
    assert_eq!(error.body, Bytes::from_static(body));
    assert_eq!(error.declared(), Some(&ProviderError { message: "boom".into() }));
}

#[tokio::test]
async fn invalid_continuation_keeps_the_items_of_the_page_that_carried_it() {
    let server = MockServer::start().await;
    first_page_continues_at_b(&server).await;
    on_query(&server, "after", "b", json(json!({ "data": numbered(2..=2), "next": { "not": "a cursor" } }))).await;
    let client = client(&server);

    let incomplete = client.send(NextField::get()).pager().collect_all().await.expect_err("invalid");

    assert_eq!(incomplete.reason, IncompleteReason::InvalidContinuation);
    assert_eq!(ids(&incomplete.items), [1, 2]);
    assert_eq!(incomplete.pages, 2);
    let Some(PagerError::InvalidContinuation { problem, meta }) = &incomplete.error else { panic!("{incomplete:?}") };
    assert_eq!(*problem, ContinuationProblem::Cursor);
    assert_eq!((meta.status.as_u16(), meta.request.page), (200, Some(2)));
    assert_eq!(received(&server).await.len(), 2);
}

#[tokio::test]
async fn invalid_argument_is_refused_before_the_request_that_would_carry_it() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": [], "has_more": false }))).await;
    let client = client(&server);

    let incomplete = client.send(LastIdHasMore::get()).pager().page_size(101).collect_all().await.expect_err("max");

    assert_eq!(incomplete.reason, IncompleteReason::InvalidArgument);
    assert!(incomplete.items.is_empty());
    let error = incomplete.error.as_ref().expect("cause");
    assert!(refusal(error).contains("maximum, 100"), "{error:?}");
    assert!(error.response_meta().is_none());
    assert_eq!(received(&server).await.len(), 0);
}

#[tokio::test]
async fn transport_error_when_the_next_page_cannot_connect() {
    let closed = closed_port_url().await;
    let server = MockServer::start().await;
    otherwise(
        &server,
        json(json!({ "values": numbered(1..=1), "next": format!("{closed}/v2/repos/list?page=2") })),
    )
    .await;
    let client = allowing(&server, &closed);

    let incomplete = client.send(BodyNextPrevious::get()).pager().collect_all().await.expect_err("refused");

    assert_eq!(incomplete.reason, IncompleteReason::TransportError);
    assert_eq!(ids(&incomplete.items), [1]);
    assert_eq!(incomplete.pages, 2);
    let Some(PagerError::Transport(transport)) = &incomplete.error else { panic!("{incomplete:?}") };
    assert_eq!(transport.kind, TransportErrorKind::Connect);
    assert_eq!(transport.request.page, Some(2));
}

#[tokio::test]
async fn transport_error_when_a_redirect_hop_fails_keeps_the_redirect_as_evidence() {
    let closed = closed_port_url().await;
    let server = MockServer::start().await;
    first_page_continues_at_b(&server).await;
    on_query(
        &server,
        "after",
        "b",
        ResponseTemplate::new(302)
            .insert_header("Location", format!("{closed}/moved").as_str())
            .set_body_raw("moving", "text/plain"),
    )
    .await;
    let client = allowing(&server, &closed);

    let incomplete = client.send(NextField::get()).pager().collect_all().await.expect_err("hop failed");

    assert_eq!(incomplete.reason, IncompleteReason::TransportError);
    assert_eq!(ids(&incomplete.items), [1]);
    assert_eq!(incomplete.pages, 2);
    let Some(PagerError::Api { error, meta }) = &incomplete.error else { panic!("{incomplete:?}") };
    assert!(
        matches!(error.kind, ErrorKind::RedirectFailed { category: FailureCategory::Connect, .. }),
        "{:?}",
        error.kind
    );
    assert_eq!(error.body, Bytes::from_static(b"moving"));
    assert_eq!((meta.status.as_u16(), meta.request.page), (302, Some(2)));
}

#[tokio::test]
async fn cross_origin_rejected_sends_nothing_to_the_other_origin() {
    let other = MockServer::start().await;
    otherwise(&other, json(numbered(9..=9))).await;
    let server = MockServer::start().await;
    let link = format!(r#"<{}/link-cursor?cursor=c2>; rel="next""#, other.uri());
    otherwise(&server, json(numbered(1..=1)).insert_header("link", link.as_str())).await;
    let client = client(&server);

    let incomplete = client.send(LinkCursor::get()).pager().collect_all().await.expect_err("refused");

    assert_eq!(incomplete.reason, IncompleteReason::CrossOriginRejected);
    assert_eq!(ids(&incomplete.items), [1]);
    assert_eq!(incomplete.pages, 2);
    let Some(PagerError::Transport(transport)) = &incomplete.error else { panic!("{incomplete:?}") };
    assert!(matches!(transport.kind, TransportErrorKind::CrossOriginRejected { .. }), "{transport:?}");
    assert_eq!(received(&other).await.len(), 0);
}

#[tokio::test]
async fn deadline_exceeded_keeps_the_pages_that_arrived_in_time() {
    let server = MockServer::start().await;
    first_page_continues_at_b(&server).await;
    on_query(
        &server,
        "after",
        "b",
        json(json!({ "data": numbered(2..=2), "next": null })).set_delay(Duration::from_millis(800)),
    )
    .await;
    let client = client(&server);
    let deadline = Deadline::after(Duration::from_millis(300));

    let incomplete = client.send(NextField::get()).pager().deadline(&deadline).collect_all().await.expect_err("late");

    assert_eq!(incomplete.reason, IncompleteReason::DeadlineExceeded);
    assert_eq!(ids(&incomplete.items), [1]);
    assert_eq!(incomplete.pages, 2);
    let Some(PagerError::DeadlineExceeded { completed_pages, error }) = &incomplete.error else {
        panic!("{incomplete:?}")
    };
    assert_eq!(*completed_pages, 1);
    assert!(matches!(error.kind, TransportErrorKind::DeadlineExceeded { .. }), "{:?}", error.kind);
    assert!(deadline.elapsed() < Duration::from_millis(700), "the deadline, not the delay, ended it");
}

// ---------------------------------------------------------------------------
// Start positions and the page limit
// ---------------------------------------------------------------------------

#[tokio::test]
async fn collect_all_starts_from_a_resumed_cursor_or_a_start_page() {
    let cursor = MockServer::start().await;
    on_query(&cursor, "after", "c", json(json!({ "data": numbered(3..=3), "next": "d" }))).await;
    on_query(&cursor, "after", "d", json(json!({ "data": numbered(4..=4), "next": null }))).await;
    let cursor_client = client(&cursor);

    let items = cursor_client.send(NextField::get()).pager().after("c").collect_all().await.expect("complete");

    assert_eq!(ids(&items), [3, 4]);
    assert_eq!(queries(&cursor).await, [pairs(&[("after", "c")]), pairs(&[("after", "d")])]);

    let numbered_pages = MockServer::start().await;
    on_query(&numbered_pages, "page", "3", json(json!({ "data": numbered(5..=5), "meta": { "hasnext": true } }))).await;
    on_query(&numbered_pages, "page", "4", json(json!({ "data": numbered(6..=6), "meta": { "hasnext": false } }))).await;
    let page_client = client(&numbered_pages);

    let items = page_client.send(NestedMeta::get()).pager().page(3).collect_all().await.expect("complete");

    assert_eq!(ids(&items), [5, 6]);
    assert_eq!(queries(&numbered_pages).await, [pairs(&[("page", "3")]), pairs(&[("page", "4")])]);
}

#[tokio::test]
async fn a_zero_page_limit_is_an_invalid_argument_with_no_request() {
    let server = MockServer::start().await;
    first_page_continues_at_b(&server).await;
    let client = client(&server);

    let incomplete = client.send(NextField::get()).pager().page_limit(0).collect_all().await.expect_err("zero");

    assert_eq!(incomplete.reason, IncompleteReason::InvalidArgument);
    assert!(refusal(incomplete.error.as_ref().expect("cause")).contains("page limit"));
    assert!(incomplete.items.is_empty());
    assert_eq!(received(&server).await.len(), 0);
}

#[tokio::test]
async fn the_last_permitted_page_completes_the_list_when_it_shows_the_end_after_an_empty_page() {
    let server = MockServer::start().await;
    first_page_continues_at_b(&server).await;
    on_query(&server, "after", "b", json(json!({ "data": [], "next": "c" }))).await;
    on_query(&server, "after", "c", json(json!({ "data": numbered(3..=3), "next": null }))).await;
    let client = client(&server);

    let items = client.send(NextField::get()).pager().page_limit(3).collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 3]);
    assert_eq!(received(&server).await.len(), 3);
}

// ---------------------------------------------------------------------------
// Follow-up request shapes (R16)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn only_carry_sets_the_named_ordinary_inputs_on_a_next_link_and_drops_the_rest() {
    let server = MockServer::start().await;
    let link = format!(r#"<{}/products?limit=2&page_info=abc>; rel="next""#, server.uri());
    on(&server, query_param("status", "active"), json(numbered(1..=2)).insert_header("link", link.as_str())).await;
    on_query(&server, "page_info", "abc", json(numbered(3..=3))).await;
    let client = client(&server);

    let request = ShopifyProducts::with_query(&[("status", "active"), ("fields", "id,title")]);
    let items = client.send(request).pager().page_size(2).collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2, 3]);
    let sent = received(&server).await;
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[1].method, Method::GET);
    assert_eq!(sent[1].url.path(), "/products");
    assert_eq!(
        queries(&server).await,
        [
            pairs(&[("status", "active"), ("fields", "id,title"), ("limit", "2")]),
            pairs(&[("limit", "2"), ("page_info", "abc"), ("fields", "id,title")]),
        ]
    );
}

#[tokio::test]
async fn a_follow_up_takes_its_method_filled_path_carried_inputs_and_each_cursor_destination() {
    let server = MockServer::start().await;
    on(
        &server,
        path("/search/42"),
        json(json!({ "data": [{ "id": "a" }], "next": { "q": "Q2", "h": "H2", "b": "B2" } })),
    )
    .await;
    on(&server, path("/search/42/next"), json(json!({ "data": [{ "id": "b" }] }))).await;
    let client = client(&server);
    let request = SearchNext(RequestParts {
        method: Method::PUT,
        path: "/search/42".into(),
        query: vec![("q".into(), "rust".into()), ("debug".into(), "1".into())],
        headers: vec![("x-cursor".into(), "stale".into())],
        body: RequestBody::Json(r#"{"filter":"open","verbose":true,"page":{"cursor":"stale","size":5}}"#.into()),
    });

    let items = client.send(request).pager().collect_all().await.expect("complete");

    assert_eq!(names(&items), ["a", "b"]);
    let sent = received(&server).await;
    assert_eq!(sent.len(), 2);
    assert_eq!((&sent[0].method, sent[0].url.path()), (&Method::PUT, "/search/42"));
    assert_eq!((&sent[1].method, sent[1].url.path()), (&Method::POST, "/search/42/next"));
    let follow_up = queries(&server).await.remove(1);
    assert_eq!(follow_up, pairs(&[("q", "rust"), ("cursor", "Q2")]));
    assert_eq!(header_values(&sent[1], "x-cursor"), ["H2"], "the managed header replaced the ordinary one");
    assert_eq!(body_json(&sent[1]), json!({ "filter": "open", "page": { "cursor": "B2", "size": 5 } }));
}

#[tokio::test]
async fn all_except_carry_drops_the_named_url_encoded_pairs_and_keeps_repeated_ones() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": numbered(1..=1), "next": "c2" }))).await;
    on_query(&server, "cursor", "c2", json(json!({ "data": numbered(2..=2) }))).await;
    let client = client(&server);
    let mut parts = RequestParts::new(Method::POST, "/form");
    parts.body = RequestBody::UrlEncoded(pairs(&[("q", "x"), ("once", "1"), ("tag", "a"), ("tag", "b")]));

    let items = client.send(FormAllExcept(parts)).pager().collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2]);
    let sent = received(&server).await;
    assert_eq!(sent[1].method, Method::POST);
    assert_eq!(String::from_utf8_lossy(&sent[1].body), "q=x&tag=a&tag=b");
    assert_eq!(content_type(&sent[1]).as_deref(), Some("application/x-www-form-urlencoded"));
    assert_eq!(query(&sent[1]), pairs(&[("cursor", "c2")]));
}

#[tokio::test]
async fn a_body_the_carry_rule_leaves_empty_is_dropped() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": numbered(1..=1), "next": "c2" }))).await;
    on_query(&server, "cursor", "c2", json(json!({ "data": numbered(2..=2) }))).await;
    let client = client(&server);
    let mut parts = RequestParts::new(Method::POST, "/form");
    parts.body = RequestBody::UrlEncoded(pairs(&[("once", "1")]));

    let items = client.send(FormAllExcept(parts)).pager().collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2]);
    let sent = received(&server).await;
    assert_eq!(String::from_utf8_lossy(&sent[0].body), "once=1");
    assert!(sent[1].body.is_empty(), "{:?}", String::from_utf8_lossy(&sent[1].body));
    assert_eq!(content_type(&sent[1]), None);
}

#[tokio::test]
async fn only_carry_keeps_the_named_multipart_parts() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": numbered(1..=1), "next": "c2" }))).await;
    on_query(&server, "cursor", "c2", json(json!({ "data": numbered(2..=2) }))).await;
    let client = client(&server);
    let mut parts = RequestParts::new(Method::POST, "/upload");
    parts.body = RequestBody::Multipart(vec![
        FormPart::Text { name: "fields".into(), value: "id".into() },
        FormPart::File {
            name: "upload".into(),
            file_name: "a.txt".into(),
            mime: Some("text/plain".into()),
            bytes: Bytes::from_static(b"hello"),
        },
        FormPart::Json { name: "meta".into(), value: "{}".into() },
    ]);

    let items = client.send(UploadOnly(parts)).pager().collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2]);
    let sent = received(&server).await;
    let first = String::from_utf8_lossy(&sent[0].body).into_owned();
    assert!(first.contains(r#"name="upload""#) && first.contains(r#"name="meta""#), "{first}");
    let follow_up = String::from_utf8_lossy(&sent[1].body).into_owned();
    assert!(follow_up.contains(r#"name="fields""#), "{follow_up}");
    assert!(!follow_up.contains(r#"name="upload""#) && !follow_up.contains("hello"), "{follow_up}");
    assert!(!follow_up.contains(r#"name="meta""#), "{follow_up}");
    assert!(content_type(&sent[1]).expect("a content type").starts_with("multipart/form-data"));
}

#[tokio::test]
async fn a_raw_or_non_object_json_body_is_refused_for_a_follow_up_after_the_first_page() {
    let raw = {
        let mut parts = RequestParts::new(Method::POST, "/upload");
        parts.body = RequestBody::Raw {
            content_type: "application/octet-stream".into(),
            bytes: Bytes::from_static(b"\x00\x01"),
        };
        UploadOnly(parts)
    };
    let array = {
        let mut parts = RequestParts::new(Method::POST, "/form");
        parts.body = RequestBody::Json("[1,2]".into());
        FormAllExcept(parts)
    };
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": numbered(1..=1), "next": "c2" }))).await;
    let client = client(&server);

    let raw_incomplete = client.send(raw).pager().collect_all().await.expect_err("raw body");
    assert_eq!(received(&server).await.len(), 1, "page one goes out, page two does not");
    let array_incomplete = client.send(array).pager().collect_all().await.expect_err("array body");
    assert_eq!(received(&server).await.len(), 2);

    for incomplete in [raw_incomplete, array_incomplete] {
        assert_eq!(incomplete.reason, IncompleteReason::InvalidArgument);
        assert_eq!(ids(&incomplete.items), [1]);
        assert_eq!(incomplete.pages, 2);
        let error = incomplete.error.as_ref().expect("cause");
        assert!(refusal(error).contains("carry rule"), "{error:?}");
        let PagerError::Transport(transport) = error else { unreachable!() };
        assert_eq!(transport.request.page, Some(2));
    }
}

#[tokio::test]
async fn a_repeated_ordinary_query_value_stays_repeated_on_an_all_carry_follow_up() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": numbered(1..=1), "next": "n2" }))).await;
    on_query(&server, "after", "n2", json(json!({ "data": numbered(2..=2), "next": null }))).await;
    let client = client(&server);

    let request = NextField::with_query(&[("tag", "a"), ("tag", "b")]);
    let items = client.send(request).pager().collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2]);
    assert_eq!(
        queries(&server).await,
        [pairs(&[("tag", "a"), ("tag", "b")]), pairs(&[("tag", "a"), ("tag", "b"), ("after", "n2")])]
    );
}

// ---------------------------------------------------------------------------
// Managed inputs win over ordinary ones (R16)
// ---------------------------------------------------------------------------

async fn two_sized_pages(server: &MockServer) {
    otherwise(server, json(json!({ "data": numbered(1..=1), "next": "n2" }))).await;
    on_query(server, "after", "n2", json(json!({ "data": numbered(2..=2), "next": null }))).await;
}

#[tokio::test]
async fn a_caller_page_size_replaces_an_ordinary_value_of_the_same_name_exactly_once() {
    let server = MockServer::start().await;
    two_sized_pages(&server).await;
    let client = client(&server);

    let request = SizedDefault::with_query(&[("per_page", "50"), ("q", "x")]);
    let items = client.send(request).pager().page_size(10).collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2]);
    for query in queries(&server).await {
        assert_eq!(values_of(&query, "per_page"), ["10"], "{query:?}");
        assert_eq!(values_of(&query, "q"), ["x"], "{query:?}");
    }
}

#[tokio::test]
async fn without_a_caller_size_the_sent_default_replaces_an_ordinary_value_exactly_once() {
    let server = MockServer::start().await;
    two_sized_pages(&server).await;
    let client = client(&server);

    let items = client
        .send(SizedDefault::with_query(&[("per_page", "50")]))
        .pager()
        .collect_all()
        .await
        .expect("complete");

    assert_eq!(ids(&items), [1, 2]);
    let sent = queries(&server).await;
    assert_eq!(sent.len(), 2);
    for query in sent {
        assert_eq!(values_of(&query, "per_page"), ["20"], "{query:?}");
    }
}

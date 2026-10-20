//! The inventoried pagination mechanisms through the runtime pager against a
//! local mock server: the values each request sends, the items delivered in
//! order, end detection, and `collect_all` completeness (R30, R11).
//!
//! Each fixture models a provider's mechanism, not its real definition.

use serde_json::json;
use wiremock::MockServer;

use crate::pager_support::{
    Bidirectional, BodyNextPrevious, HistoryCursor, ItemIdHasMore, KeysetLink, LastIdHasMore, LinkCursor, NestedMeta,
    NextField, NextPageNumber, PageLimitLinkTotal, PageLinkRel, PageNextHeader, PageToken, ZeroBasedNonEmpty,
    ZeroBasedPage, client, ids, json, names, numbered, on_query, otherwise, pairs, queries,
};

#[tokio::test]
async fn github_page_number_follows_link_rel_next_with_per_page() {
    let server = MockServer::start().await;
    let uri = server.uri();
    let first = format!(r#"<{uri}/page-link?page=2&per_page=2>; rel="next", <{uri}/page-link?page=2&per_page=2>; rel="last""#);
    on_query(&server, "page", "1", json(numbered(1..=2)).insert_header("link", first)).await;
    let last = format!(r#"<{uri}/page-link?page=1&per_page=2>; rel="first""#);
    on_query(&server, "page", "2", json(numbered(3..=3)).insert_header("link", last)).await;
    let client = client(&server);

    let items = client.send(PageLinkRel::get()).pager().page_size(2).collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2, 3]);
    assert_eq!(
        queries(&server).await,
        [pairs(&[("page", "1"), ("per_page", "2")]), pairs(&[("page", "2"), ("per_page", "2")])]
    );
}

#[tokio::test]
async fn gitlab_page_number_continues_while_x_next_page_is_present_and_reports_x_total() {
    let server = MockServer::start().await;
    let page = |range, next: &str| json(numbered(range)).insert_header("x-next-page", next).insert_header("x-total", "3");
    on_query(&server, "page", "1", page(1..=2, "2")).await;
    on_query(&server, "page", "2", page(3..=3, "")).await;
    let client = client(&server);

    let mut pages = client.send(PageNextHeader::get()).pager().page_size(2).pages();
    assert_eq!(pages.total(), None, "no total before the first page");
    let first = pages.next().await.expect("page 1").expect("a page");
    assert_eq!((ids(&first.items), first.total), (vec![1, 2], Some(3)));
    assert_eq!(pages.total(), Some(3));
    assert!(!pages.is_finished());
    let second = pages.next().await.expect("page 2").expect("a page");
    assert_eq!(ids(&second.items), [3]);
    assert!(pages.is_finished());
    assert!(pages.next().await.expect("end").is_none());
    assert_eq!((pages.pages_fetched(), pages.items_delivered()), (2, 3));
    assert_eq!(
        queries(&server).await,
        [pairs(&[("page", "1"), ("per_page", "2")]), pairs(&[("page", "2"), ("per_page", "2")])]
    );
}

#[tokio::test]
async fn gitlab_keyset_follows_the_link_verbatim_after_a_first_request_with_constants() {
    let server = MockServer::start().await;
    let uri = server.uri();
    let next = format!(r#"<{uri}/keyset?id_after=2&order_by=id&pagination=keyset&per_page=2>; rel="next""#);
    otherwise(&server, json(numbered(1..=2)).insert_header("link", next)).await;
    on_query(&server, "id_after", "2", json(numbered(3..=3))).await;
    let client = client(&server);

    let items = client.send(KeysetLink::get()).pager().page_size(2).collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2, 3]);
    assert_eq!(
        queries(&server).await,
        [
            pairs(&[("per_page", "2"), ("pagination", "keyset"), ("order_by", "id")]),
            pairs(&[("id_after", "2"), ("order_by", "id"), ("pagination", "keyset"), ("per_page", "2")]),
        ],
        "the provider's link is followed as issued"
    );
}

#[tokio::test]
async fn gitea_page_and_limit_follow_link_next_and_report_x_total_count() {
    let server = MockServer::start().await;
    let uri = server.uri();
    let next = format!(r#"<{uri}/page-limit?page=2&limit=2>; rel="next""#);
    on_query(&server, "page", "1", json(numbered(1..=2)).insert_header("link", next).insert_header("x-total-count", "3")).await;
    on_query(&server, "page", "2", json(numbered(3..=3)).insert_header("x-total-count", "3")).await;
    let client = client(&server);

    let mut pages = client.send(PageLimitLinkTotal::get()).pager().page_size(2).pages();
    let first = pages.next().await.expect("page 1").expect("a page");
    assert_eq!(first.total, Some(3));
    let second = pages.next().await.expect("page 2").expect("a page");
    assert_eq!([ids(&first.items), ids(&second.items)].concat(), [1, 2, 3]);
    assert!(pages.next().await.expect("end").is_none());
    assert_eq!(pages.total(), Some(3));
    assert_eq!(
        queries(&server).await,
        [pairs(&[("page", "1"), ("limit", "2")]), pairs(&[("page", "2"), ("limit", "2")])]
    );
}

#[tokio::test]
async fn bitbucket_follows_the_body_next_url_and_steps_back_through_previous() {
    let server = MockServer::start().await;
    let uri = server.uri();
    let page_one = json(json!({
        "values": numbered(1..=2), "size": 3, "pagelen": 2,
        "next": format!("{uri}/v2/repos/list?page=2&pagelen=2"),
    }));
    otherwise(&server, page_one.clone()).await;
    on_query(&server, "page", "1", page_one).await;
    on_query(
        &server,
        "page",
        "2",
        json(json!({
            "values": numbered(3..=3), "size": 3, "pagelen": 2,
            "previous": format!("{uri}/v2/repos/list?page=1&pagelen=2"),
        })),
    )
    .await;
    let client = client(&server);

    let mut pages = client.send(BodyNextPrevious::get()).pager().page_size(2).pages();
    let first = pages.next().await.expect("page 1").expect("a page");
    let second = pages.next().await.expect("page 2").expect("a page");
    assert_eq!((ids(&first.items), ids(&second.items)), (vec![1, 2], vec![3]));
    assert_eq!(pages.total(), Some(3));
    assert!(pages.next().await.expect("end").is_none());
    assert!(pages.is_finished());

    let back = pages.prev().await.expect("previous page").expect("a page");
    assert_eq!(ids(&back.items), [1, 2]);
    assert_eq!(back.number, 3);
    assert_eq!(
        queries(&server).await,
        [
            pairs(&[("pagelen", "2")]),
            pairs(&[("page", "2"), ("pagelen", "2")]),
            pairs(&[("page", "1"), ("pagelen", "2")]),
        ]
    );

    let complete = client.send(BodyNextPrevious::get()).pager().collect_all().await.expect("complete");
    assert_eq!(ids(&complete), [1, 2, 3]);
}

#[tokio::test]
async fn openai_after_takes_last_id_while_has_more() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": [{ "id": "a" }, { "id": "b" }], "last_id": "b", "has_more": true }))).await;
    on_query(&server, "after", "b", json(json!({ "data": [{ "id": "c" }], "last_id": "c", "has_more": false }))).await;
    let client = client(&server);

    let items = client.send(LastIdHasMore::get()).pager().page_size(2).collect_all().await.expect("complete");

    assert_eq!(names(&items), ["a", "b", "c"]);
    assert_eq!(
        queries(&server).await,
        [pairs(&[("limit", "2")]), pairs(&[("after", "b"), ("limit", "2")])]
    );
}

#[tokio::test]
async fn openai_item_id_variant_takes_the_last_record_id_while_has_more() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": [{ "id": "a" }, { "id": "b" }], "has_more": true }))).await;
    on_query(&server, "after", "b", json(json!({ "data": [{ "id": "c" }], "has_more": false }))).await;
    let client = client(&server);

    let items = client.send(ItemIdHasMore::get()).pager().page_size(2).collect_all().await.expect("complete");

    assert_eq!(names(&items), ["a", "b", "c"]);
    assert_eq!(
        queries(&server).await,
        [pairs(&[("limit", "2")]), pairs(&[("after", "b"), ("limit", "2")])]
    );
}

#[tokio::test]
async fn openai_s7_after_takes_next_until_it_is_null() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": numbered(1..=2), "next": "n2" }))).await;
    on_query(&server, "after", "n2", json(json!({ "data": numbered(3..=3), "next": null }))).await;
    let client = client(&server);

    let items = client.send(NextField::get()).pager().collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2, 3]);
    assert_eq!(queries(&server).await, [pairs(&[]), pairs(&[("after", "n2")])]);
}

#[tokio::test]
async fn openai_s8_page_takes_a_numeric_next_page() {
    let server = MockServer::start().await;
    otherwise(&server, json(json!({ "data": numbered(1..=2), "next_page": 2 }))).await;
    on_query(&server, "page", "2", json(json!({ "data": numbered(3..=3), "next_page": null }))).await;
    let client = client(&server);

    let items = client.send(NextPageNumber::get()).pager().collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2, 3]);
    assert_eq!(queries(&server).await, [pairs(&[]), pairs(&[("page", "2")])]);
}

#[tokio::test]
async fn anthropic_ids_page_forward_by_last_id_and_back_by_first_id() {
    let server = MockServer::start().await;
    let page_one = json(json!({ "data": [{ "id": "a" }, { "id": "b" }], "first_id": "a", "last_id": "b", "has_more": true }));
    otherwise(&server, page_one.clone()).await;
    on_query(
        &server,
        "after_id",
        "b",
        json(json!({ "data": [{ "id": "c" }], "first_id": "c", "last_id": "c", "has_more": false })),
    )
    .await;
    on_query(&server, "before_id", "c", page_one).await;
    let client = client(&server);

    let mut pages = client.send(Bidirectional::get()).pager().page_size(2).pages();
    let first = pages.next().await.expect("page 1").expect("a page");
    let second = pages.next().await.expect("page 2").expect("a page");
    assert_eq!((names(&first.items), names(&second.items)), (vec!["a", "b"], vec!["c"]));
    assert!(pages.next().await.expect("end").is_none());
    let back = pages.prev().await.expect("previous").expect("a page");
    assert_eq!(names(&back.items), ["a", "b"]);
    assert_eq!(
        queries(&server).await,
        [
            pairs(&[("limit", "2")]),
            pairs(&[("after_id", "b"), ("limit", "2")]),
            pairs(&[("before_id", "c"), ("limit", "2")]),
        ]
    );
}

#[tokio::test]
async fn anthropic_before_starts_backward_and_prev_continues_by_first_id() {
    let server = MockServer::start().await;
    on_query(
        &server,
        "before_id",
        "z",
        json(json!({ "data": [{ "id": "x" }, { "id": "y" }], "first_id": "x", "last_id": "y", "has_more": true })),
    )
    .await;
    on_query(
        &server,
        "before_id",
        "x",
        json(json!({ "data": [{ "id": "w" }], "first_id": "w", "last_id": "w", "has_more": false })),
    )
    .await;
    let client = client(&server);

    let mut pages = client.send(Bidirectional::get()).pager().before("z").pages();
    let first = pages.next().await.expect("first page").expect("a page");
    assert_eq!(names(&first.items), ["x", "y"]);
    let earlier = pages.prev().await.expect("earlier page").expect("a page");
    assert_eq!(names(&earlier.items), ["w"]);
    assert!(pages.prev().await.expect("start of list").is_none());
    assert_eq!(queries(&server).await, [pairs(&[("before_id", "z")]), pairs(&[("before_id", "x")])]);
}

#[tokio::test]
async fn elevenlabs_history_starts_after_the_last_history_item_id() {
    let server = MockServer::start().await;
    otherwise(
        &server,
        json(json!({ "history": [{ "id": "h1" }, { "id": "h2" }], "last_history_item_id": "h2", "has_more": true })),
    )
    .await;
    on_query(
        &server,
        "start_after_history_item_id",
        "h2",
        json(json!({ "history": [{ "id": "h3" }], "last_history_item_id": "h3", "has_more": false })),
    )
    .await;
    let client = client(&server);

    let items = client.send(HistoryCursor::get()).pager().page_size(2).collect_all().await.expect("complete");

    assert_eq!(names(&items), ["h1", "h2", "h3"]);
    assert_eq!(
        queries(&server).await,
        [
            pairs(&[("page_size", "2")]),
            pairs(&[("start_after_history_item_id", "h2"), ("page_size", "2")]),
        ]
    );
}

#[tokio::test]
async fn elevenlabs_voices_pass_the_next_page_token_and_report_total_count() {
    let server = MockServer::start().await;
    otherwise(
        &server,
        json(json!({ "voices": [{ "id": "v1" }], "next_page_token": "t2", "has_more": true, "total_count": 2 })),
    )
    .await;
    on_query(
        &server,
        "next_page_token",
        "t2",
        json(json!({ "voices": [{ "id": "v2" }], "next_page_token": null, "has_more": false, "total_count": 2 })),
    )
    .await;
    let client = client(&server);

    let mut pages = client.send(PageToken::get()).pager().page_size(1).pages();
    let first = pages.next().await.expect("page 1").expect("a page");
    let second = pages.next().await.expect("page 2").expect("a page");
    assert_eq!((names(&first.items), names(&second.items)), (vec!["v1"], vec!["v2"]));
    assert_eq!(pages.total(), Some(2));
    assert!(pages.next().await.expect("end").is_none());
    assert_eq!(
        queries(&server).await,
        [pairs(&[("page_size", "1")]), pairs(&[("next_page_token", "t2"), ("page_size", "1")])]
    );
}

#[tokio::test]
async fn elevenlabs_shared_voices_count_pages_from_zero_while_has_more() {
    let server = MockServer::start().await;
    on_query(&server, "page", "0", json(json!({ "voices": numbered(1..=2), "has_more": true }))).await;
    on_query(&server, "page", "1", json(json!({ "voices": numbered(3..=3), "has_more": false }))).await;
    let client = client(&server);

    let items = client.send(ZeroBasedPage::get()).pager().collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2, 3]);
    assert_eq!(queries(&server).await, [pairs(&[("page", "0")]), pairs(&[("page", "1")])]);
}

#[tokio::test]
async fn emqx_pages_by_page_and_limit_while_nested_meta_hasnext() {
    let server = MockServer::start().await;
    on_query(
        &server,
        "page",
        "1",
        json(json!({ "data": numbered(1..=2), "meta": { "page": 1, "limit": 2, "count": 3, "hasnext": true } })),
    )
    .await;
    on_query(
        &server,
        "page",
        "2",
        json(json!({ "data": numbered(3..=3), "meta": { "page": 2, "limit": 2, "count": 3, "hasnext": false } })),
    )
    .await;
    let client = client(&server);

    let mut pages = client.send(NestedMeta::get()).pager().page_size(2).pages();
    let mut all = Vec::new();
    while let Some(page) = pages.next().await.expect("page") {
        all.extend(page.items);
    }
    assert_eq!(ids(&all), [1, 2, 3]);
    assert_eq!(pages.total(), Some(3));
    assert_eq!(
        queries(&server).await,
        [pairs(&[("page", "1"), ("limit", "2")]), pairs(&[("page", "2"), ("limit", "2")])]
    );
}

#[tokio::test]
async fn hugging_face_follows_the_link_cursor_with_its_own_limit() {
    let server = MockServer::start().await;
    let uri = server.uri();
    let next = format!(r#"<{uri}/link-cursor?cursor=c2&limit=2>; rel="next""#);
    otherwise(&server, json(numbered(1..=2)).insert_header("link", next)).await;
    on_query(&server, "cursor", "c2", json(numbered(3..=3))).await;
    let client = client(&server);

    let items = client.send(LinkCursor::get()).pager().page_size(2).collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2, 3]);
    assert_eq!(
        queries(&server).await,
        [pairs(&[("limit", "2")]), pairs(&[("cursor", "c2"), ("limit", "2")])]
    );
}

#[tokio::test]
async fn hugging_face_zero_based_p_ends_on_an_empty_page() {
    let server = MockServer::start().await;
    on_query(&server, "p", "0", json(numbered(1..=2))).await;
    on_query(&server, "p", "1", json(numbered(3..=3))).await;
    on_query(&server, "p", "2", json(json!([]))).await;
    let client = client(&server);

    let items = client.send(ZeroBasedNonEmpty::get()).pager().collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2, 3]);
    assert_eq!(
        queries(&server).await,
        [pairs(&[("p", "0")]), pairs(&[("p", "1")]), pairs(&[("p", "2")])]
    );
}

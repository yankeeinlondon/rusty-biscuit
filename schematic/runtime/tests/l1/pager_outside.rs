//! The outside pagination patterns through the runtime pager against a
//! local mock server (R30, R16).
//!
//! AIP-158, Mastodon, SCIM, and Salesforce Bulk 2.0 are verified patterns.
//! The `unverified_*` tests prove the mechanism only, not the named
//! provider's real behavior.

use serde_json::json;
use wiremock::MockServer;
use wiremock::matchers::path;

use crate::pager_support::{
    DottedNextLink, HeaderContinuation, HeaderLocator, LinkNextPrev, MetaNextToken, NestedChildren,
    OneBasedOffsetTotal, RelativeRecordsUrl, TokenAllExceptSkip, TotalPagesHeader, client, ids, json, names,
    numbered, on, on_header, on_query, otherwise, pairs, queries, received,
};

#[tokio::test]
async fn aip_158_follow_ups_carry_everything_except_skip_and_take_an_adjusted_size() {
    let server = MockServer::start().await;
    otherwise(
        &server,
        json(json!({ "books": [{ "id": "a" }, { "id": "b" }], "nextPageToken": "t2", "totalSize": 3 })),
    )
    .await;
    on_query(&server, "pageToken", "t2", json(json!({ "books": [{ "id": "c" }], "nextPageToken": "", "totalSize": 3 })))
        .await;
    let client = client(&server);

    let request = TokenAllExceptSkip::with_query(&[("filter", "x"), ("skip", "5")]);
    let mut pages = client.send(request).pager().page_size(2).pages();
    let first = pages.next().await.expect("page 1").expect("a page");
    pages.set_page_size(3);
    let second = pages.next().await.expect("page 2").expect("a page");
    assert_eq!((names(&first.items), names(&second.items)), (vec!["a", "b"], vec!["c"]));
    assert!(pages.next().await.expect("an empty token ends").is_none());
    assert_eq!(pages.total(), Some(3));
    assert_eq!(
        queries(&server).await,
        [
            pairs(&[("filter", "x"), ("skip", "5"), ("pageSize", "2")]),
            pairs(&[("filter", "x"), ("pageToken", "t2"), ("pageSize", "3")]),
        ]
    );
}

#[tokio::test]
async fn mastodon_follows_link_next_by_max_id_and_steps_back_by_prev() {
    let server = MockServer::start().await;
    let uri = server.uri();
    let first_links = format!(
        r#"<{uri}/api/v1/timelines?max_id=7>; rel="next", <{uri}/api/v1/timelines?min_id=9>; rel="prev""#
    );
    otherwise(&server, json(numbered([9, 8])).insert_header("link", first_links)).await;
    let second_links = format!(r#"<{uri}/api/v1/timelines?min_id=7>; rel="prev""#);
    on_query(&server, "max_id", "7", json(numbered([7, 6])).insert_header("link", second_links)).await;
    on_query(&server, "min_id", "7", json(numbered([9, 8]))).await;
    let client = client(&server);

    let mut pages = client.send(LinkNextPrev::get()).pager().page_size(2).pages();
    let first = pages.next().await.expect("page 1").expect("a page");
    let second = pages.next().await.expect("page 2").expect("a page");
    assert_eq!([ids(&first.items), ids(&second.items)].concat(), [9, 8, 7, 6]);
    assert!(pages.next().await.expect("no next link ends").is_none());
    let newer = pages.prev().await.expect("prev").expect("a page");
    assert_eq!(ids(&newer.items), [9, 8]);
    assert_eq!(
        queries(&server).await,
        [pairs(&[("limit", "2")]), pairs(&[("max_id", "7")]), pairs(&[("min_id", "7")])]
    );
}

#[tokio::test]
async fn scim_one_based_start_index_advances_by_count_until_total_results() {
    let server = MockServer::start().await;
    let page = |range| json(json!({ "Resources": numbered(range), "totalResults": 5 }));
    on_query(&server, "startIndex", "1", page(1..=2)).await;
    on_query(&server, "startIndex", "3", page(3..=4)).await;
    on_query(&server, "startIndex", "5", page(5..=5)).await;
    let client = client(&server);

    let mut pages = client.send(OneBasedOffsetTotal::get()).pager().page_size(2).pages();
    let mut all = Vec::new();
    while let Some(page) = pages.next().await.expect("page") {
        all.extend(page.items);
    }
    assert_eq!(ids(&all), [1, 2, 3, 4, 5]);
    assert_eq!(pages.total(), Some(5));
    assert_eq!(
        queries(&server).await,
        [
            pairs(&[("startIndex", "1"), ("count", "2")]),
            pairs(&[("startIndex", "3"), ("count", "2")]),
            pairs(&[("startIndex", "5"), ("count", "2")]),
        ]
    );
}

#[tokio::test]
async fn salesforce_bulk_locator_header_ends_at_the_literal_null() {
    let server = MockServer::start().await;
    otherwise(&server, json(numbered(1..=2)).insert_header("sforce-locator", "MjAwMDAw")).await;
    on_query(&server, "locator", "MjAwMDAw", json(numbered(3..=3)).insert_header("sforce-locator", "null")).await;
    let client = client(&server);

    let items = client.send(HeaderLocator::get()).pager().page_size(2).collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2, 3]);
    assert_eq!(
        queries(&server).await,
        [pairs(&[("maxRecords", "2")]), pairs(&[("locator", "MjAwMDAw"), ("maxRecords", "2")])]
    );
}

#[tokio::test]
async fn unverified_x_api_v2_pagination_token_takes_meta_next_token() {
    let server = MockServer::start().await;
    otherwise(
        &server,
        json(json!({ "data": [{ "id": "1" }], "meta": { "result_count": 1, "next_token": "nt2" } })),
    )
    .await;
    on_query(
        &server,
        "pagination_token",
        "nt2",
        json(json!({ "data": [{ "id": "2" }], "meta": { "result_count": 1 } })),
    )
    .await;
    let client = client(&server);

    let items = client.send(MetaNextToken::get()).pager().page_size(10).collect_all().await.expect("complete");

    assert_eq!(names(&items), ["1", "2"]);
    assert_eq!(
        queries(&server).await,
        [pairs(&[("max_results", "10")]), pairs(&[("pagination_token", "nt2"), ("max_results", "10")])]
    );
}

#[tokio::test]
async fn unverified_wordpress_pages_until_x_wp_total_pages() {
    let server = MockServer::start().await;
    on_query(&server, "page", "1", json(numbered(1..=2)).insert_header("x-wp-totalpages", "2")).await;
    on_query(&server, "page", "2", json(numbered(3..=3)).insert_header("x-wp-totalpages", "2")).await;
    let client = client(&server);

    let mut pages = client.send(TotalPagesHeader::get()).pager().page_size(2).pages();
    let mut all = Vec::new();
    while let Some(page) = pages.next().await.expect("page") {
        all.extend(page.items);
    }
    assert_eq!(ids(&all), [1, 2, 3]);
    assert_eq!(pages.total(), Some(2), "the total counts pages");
    assert_eq!(
        queries(&server).await,
        [pairs(&[("page", "1"), ("per_page", "2")]), pairs(&[("page", "2"), ("per_page", "2")])]
    );
}

#[tokio::test]
async fn unverified_odata_follows_a_dotted_body_next_link_with_its_skiptoken() {
    let server = MockServer::start().await;
    let uri = server.uri();
    otherwise(
        &server,
        json(json!({ "value": numbered(1..=2), "@odata.nextLink": format!("{uri}/v1.0/users?$skiptoken=X") })),
    )
    .await;
    on_query(&server, "$skiptoken", "X", json(json!({ "value": numbered(3..=3) }))).await;
    let client = client(&server);

    let items = client.send(DottedNextLink::get()).pager().collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2, 3]);
    assert_eq!(queries(&server).await, [pairs(&[]), pairs(&[("$skiptoken", "X")])]);
}

#[tokio::test]
async fn unverified_cosmos_db_continuation_moves_from_response_header_to_request_header() {
    let server = MockServer::start().await;
    // No comma: wiremock's header matcher splits values on commas.
    let token = r#"{"token":"+RID:abc"}"#;
    otherwise(&server, json(json!({ "Documents": numbered(1..=2) })).insert_header("x-ms-continuation", token)).await;
    on_header(&server, "x-ms-continuation", token, json(json!({ "Documents": numbered(3..=3) }))).await;
    let client = client(&server);

    let items = client.send(HeaderContinuation::get()).pager().page_size(2).collect_all().await.expect("complete");

    assert_eq!(ids(&items), [1, 2, 3]);
    let requests = received(&server).await;
    assert_eq!(requests.len(), 2);
    let header = |index: usize, name: &str| {
        requests[index].headers.get(name).map(|value| value.to_str().expect("text").to_owned())
    };
    assert_eq!(header(0, "x-ms-continuation"), None);
    assert_eq!(header(0, "x-ms-max-item-count").as_deref(), Some("2"));
    assert_eq!(header(1, "x-ms-continuation").as_deref(), Some(token));
    assert_eq!(header(1, "x-ms-max-item-count").as_deref(), Some("2"));
}

#[tokio::test]
async fn unverified_salesforce_rest_query_follows_a_relative_next_records_url() {
    let server = MockServer::start().await;
    on(
        &server,
        path("/services/data/v60.0/query"),
        json(json!({
            "records": numbered(1..=2), "done": false,
            "nextRecordsUrl": "/services/data/v60.0/query/01gX-2000",
        })),
    )
    .await;
    on(
        &server,
        path("/services/data/v60.0/query/01gX-2000"),
        json(json!({ "records": numbered(3..=3), "done": true })),
    )
    .await;
    let client = client(&server);

    let items = client
        .send(RelativeRecordsUrl::with_query(&[("q", "SELECT Id FROM Account")]))
        .pager()
        .collect_all()
        .await
        .expect("complete");

    assert_eq!(ids(&items), [1, 2, 3]);
    let paths: Vec<String> = received(&server).await.iter().map(|request| request.url.path().to_owned()).collect();
    assert_eq!(paths, ["/services/data/v60.0/query", "/services/data/v60.0/query/01gX-2000"]);
    assert_eq!(
        queries(&server).await[1],
        pairs(&[]),
        "the link is followed verbatim, without the first request's query"
    );
}

#[tokio::test]
async fn unverified_reddit_after_takes_data_after_and_items_are_children_data() {
    let server = MockServer::start().await;
    otherwise(
        &server,
        json(json!({ "kind": "Listing", "data": { "after": "t3_b", "children": [
            { "kind": "t3", "data": { "id": "t3_a" } },
            { "kind": "t3", "data": { "id": "t3_b" } },
        ] } })),
    )
    .await;
    on_query(
        &server,
        "after",
        "t3_b",
        json(json!({ "kind": "Listing", "data": { "after": null, "children": [
            { "kind": "t3", "data": { "id": "t3_c" } },
        ] } })),
    )
    .await;
    let client = client(&server);

    let items = client.send(NestedChildren::get()).pager().page_size(2).collect_all().await.expect("complete");

    assert_eq!(names(&items), ["t3_a", "t3_b", "t3_c"]);
    assert_eq!(
        queries(&server).await,
        [pairs(&[("limit", "2")]), pairs(&[("after", "t3_b"), ("limit", "2")])]
    );
}

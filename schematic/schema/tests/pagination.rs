//! Pagination through the committed generated clients against a local
//! wiremock server (R31): one forge endpoint per strategy family is paged end
//! to end with the strategy its definition authors.
//!
//! - GitHub `ListReleases`: page number from 1, `per_page` 1..=100, more
//!   while the `Link` header names `rel="next"`.
//! - OpenAI `ListBatches`: cursor `after` from body `last_id`, more while
//!   `has_more`, `limit` 1..=100.
//! - Bitbucket `ListTags`: follows body `next`, `pagelen` 1..=100, total in
//!   body `size`.
//!
//! Every client is built with an empty env-var order, so a host token never
//! reaches the mock and no test needs `#[serial]`.

use schematic_schema::bitbucket::Bitbucket;
use schematic_schema::github::GitHub;
use schematic_schema::openai::OpenAI;
use schematic_schema::schematic_runtime::pager::{IncompleteReason, PagerError};
use schematic_schema::schematic_runtime::{TransportError, TransportErrorKind};
use serde_json::{Value, json};
use wiremock::matchers::{method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const RELEASES_PATH: &str = "/repos/octo/widgets/releases";
const BATCHES_PATH: &str = "/batches";
const TAGS_PATH: &str = "/repositories/team/widgets/refs/tags";

fn github(base_url: &str) -> GitHub {
    GitHub::new().base_url(base_url).env_order(Vec::<String>::new()).build().expect("client builds")
}

fn openai(base_url: &str) -> OpenAI {
    OpenAI::new().base_url(base_url).env_order(Vec::<String>::new()).build().expect("client builds")
}

fn bitbucket(base_url: &str) -> Bitbucket {
    Bitbucket::new().base_url(base_url).env_order(Vec::<String>::new()).build().expect("client builds")
}

fn release(id: u64) -> Value {
    json!({
        "id": id,
        "tag_name": format!("v{id}"),
        "created_at": "2026-10-13T00:00:00Z",
        "html_url": format!("https://github.com/octo/widgets/releases/v{id}")
    })
}

fn batch(id: &str) -> Value {
    json!({
        "id": id,
        "object": "batch",
        "endpoint": "/v1/responses",
        "input_file_id": "file-1",
        "completion_window": "24h",
        "status": "completed",
        "created_at": 1_760_000_000
    })
}

fn tag(name: &str) -> Value {
    json!({ "name": name, "type": "tag" })
}

/// The decoded query of a received request, in wire order.
fn query(request: &Request) -> Vec<(String, String)> {
    request.url.query_pairs().map(|(key, value)| (key.into_owned(), value.into_owned())).collect()
}

fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect()
}

/// Mounts GitHub release pages 1 and 2 at `per_page=2`; page 1 links to page
/// 2 with `rel="next"`, page 2 names only `rel="prev"`, so it is the last.
async fn mount_release_pages(server: &MockServer) {
    let next = format!("<{}{RELEASES_PATH}?page=2&per_page=2>; rel=\"next\"", server.uri());
    let prev = format!("<{}{RELEASES_PATH}?page=1&per_page=2>; rel=\"prev\"", server.uri());
    Mock::given(method("GET"))
        .and(path(RELEASES_PATH))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200).insert_header("link", next.as_str()).set_body_json(json!([release(1), release(2)])))
        .expect(1)
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(RELEASES_PATH))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).insert_header("link", prev.as_str()).set_body_json(json!([release(3)])))
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn github_page_number_collect_all_returns_every_release_in_order() {
    let server = MockServer::start().await;
    mount_release_pages(&server).await;

    let releases = github(&server.uri())
        .list_releases("octo", "widgets")
        .page_size(2)
        .collect_all()
        .await
        .expect("the provider signaled the end");

    let ids: Vec<u64> = releases.iter().map(|release| release.id).collect();
    assert_eq!(ids, [1, 2, 3]);
    let received = server.received_requests().await.unwrap();
    let queries: Vec<_> = received.iter().map(query).collect();
    assert_eq!(queries, [pairs(&[("page", "1"), ("per_page", "2")]), pairs(&[("page", "2"), ("per_page", "2")])]);
}

#[tokio::test]
async fn github_pages_report_each_page_and_stop_after_the_last() {
    let server = MockServer::start().await;
    mount_release_pages(&server).await;
    let client = github(&server.uri());

    let mut pages = client.list_releases("octo", "widgets").page_size(2).pages();
    let first = pages.next().await.unwrap().expect("page 1");
    assert_eq!((first.number, first.items.len(), first.total), (1, 2, None));
    assert_eq!(first.meta.status.as_u16(), 200);
    let second = pages.next().await.unwrap().expect("page 2");
    assert_eq!((second.number, second.items[0].id), (2, 3));
    assert!(pages.next().await.unwrap().is_none(), "no `rel=\"next\"` ends the list");
    assert!(pages.is_finished());
    assert_eq!((pages.pages_fetched(), pages.items_delivered()), (2, 3));
    assert_eq!(server.received_requests().await.unwrap().len(), 2, "the end is known without a third request");
}

/// R11a: a configured single page sends exactly one request carrying both
/// knob values and returns that page's items, not the endpoint's own type.
#[tokio::test]
async fn github_fetch_page_sends_one_request_with_the_configured_page_and_size() {
    let server = MockServer::start().await;
    let next = format!("<{}{RELEASES_PATH}?page=4&per_page=5>; rel=\"next\"", server.uri());
    Mock::given(method("GET"))
        .and(path(RELEASES_PATH))
        .respond_with(ResponseTemplate::new(200).insert_header("link", next.as_str()).set_body_json(json!([release(11)])))
        .mount(&server)
        .await;

    let page = github(&server.uri()).list_releases("octo", "widgets").page(3).page_size(5).fetch_page().await.unwrap();

    assert_eq!(page.items.iter().map(|release| release.tag_name.as_str()).collect::<Vec<_>>(), ["v11"]);
    assert_eq!(page.number, 1, "the first page of this traversal");
    let received = server.received_requests().await.unwrap();
    assert_eq!(received.len(), 1, "a following page is never requested");
    assert_eq!(query(&received[0]), pairs(&[("page", "3"), ("per_page", "5")]));
}

/// An ordinary typed send of a paginated endpoint is one plain request: the
/// GitHub strategy authors no sent default, so no pager input is added.
#[tokio::test]
async fn github_ordinary_send_adds_no_pager_input() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(RELEASES_PATH))
        .and(query_param_is_missing("page"))
        .and(query_param_is_missing("per_page"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([release(1)])))
        .expect(1)
        .mount(&server)
        .await;

    let releases = github(&server.uri()).list_releases("octo", "widgets").await.unwrap();

    assert_eq!(releases.len(), 1);
}

/// Sizes outside the authored 1..=100 are refused before anything is sent;
/// the bounds themselves are accepted.
#[tokio::test]
async fn github_page_size_outside_the_authored_bounds_is_refused_before_sending() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(RELEASES_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&server)
        .await;
    let client = github(&server.uri());

    for (refused, reason) in [
        (0, "the page size is below the declared minimum, 1"),
        (101, "the page size is above the declared maximum, 100"),
        (500, "the page size is above the declared maximum, 100"),
    ] {
        let error = client.list_releases("octo", "widgets").page_size(refused).fetch_page().await.unwrap_err();
        let PagerError::Transport(TransportError {
            kind: TransportErrorKind::InvalidArgument(argument),
            ..
        }) = &error
        else {
            panic!("size {refused}: {error:?}");
        };
        assert_eq!(argument.reason(), reason, "size {refused}");
    }
    assert!(server.received_requests().await.unwrap().is_empty(), "a refused size sends nothing");

    for accepted in [1, 100] {
        let page = client.list_releases("octo", "widgets").page_size(accepted).fetch_page().await.unwrap();
        assert!(page.items.is_empty());
    }
    let sizes: Vec<_> = server.received_requests().await.unwrap().iter().map(query).collect();
    assert_eq!(sizes, [pairs(&[("page", "1"), ("per_page", "1")]), pairs(&[("page", "1"), ("per_page", "100")])]);
}

/// A page limit reached while `rel="next"` still names a page is not a
/// complete list: the items gathered so far come back inside `Incomplete`.
#[tokio::test]
async fn github_collect_all_stopped_by_the_page_limit_is_incomplete() {
    let server = MockServer::start().await;
    let next = format!("<{}{RELEASES_PATH}?page=2&per_page=2>; rel=\"next\"", server.uri());
    Mock::given(method("GET"))
        .and(path(RELEASES_PATH))
        .respond_with(ResponseTemplate::new(200).insert_header("link", next.as_str()).set_body_json(json!([release(1), release(2)])))
        .mount(&server)
        .await;

    let incomplete = github(&server.uri())
        .list_releases("octo", "widgets")
        .page_size(2)
        .page_limit(1)
        .collect_all()
        .await
        .unwrap_err();

    assert_eq!(incomplete.reason, IncompleteReason::PageLimitReached);
    assert_eq!(incomplete.items.iter().map(|release| release.id).collect::<Vec<_>>(), [1, 2]);
    assert_eq!(incomplete.pages, 1);
    assert!(incomplete.error.is_none());
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

/// An error status on a later page fails the traversal with the page-1 items
/// kept as evidence, never as a complete list.
#[tokio::test]
async fn github_collect_all_with_a_failing_second_page_is_incomplete() {
    let server = MockServer::start().await;
    let next = format!("<{}{RELEASES_PATH}?page=2&per_page=2>; rel=\"next\"", server.uri());
    Mock::given(method("GET"))
        .and(path(RELEASES_PATH))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200).insert_header("link", next.as_str()).set_body_json(json!([release(1)])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(RELEASES_PATH))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(502).set_body_string("bad gateway"))
        .mount(&server)
        .await;

    let incomplete = github(&server.uri()).list_releases("octo", "widgets").page_size(2).collect_all().await.unwrap_err();

    assert_eq!(incomplete.reason, IncompleteReason::HttpError);
    assert_eq!(incomplete.items.len(), 1);
    assert_eq!(incomplete.pages, 2);
    let error = incomplete.error.expect("the failing page's error");
    assert_eq!(error.response_meta().expect("a response arrived").status.as_u16(), 502);
}

/// Mounts OpenAI batch pages: the first (no `after`) says more follows and
/// ends at `b2`; the page after `b2` is the last.
async fn mount_batch_pages(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path(BATCHES_PATH))
        .and(query_param_is_missing("after"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [batch("b1"), batch("b2")],
            "first_id": "b1",
            "last_id": "b2",
            "has_more": true
        })))
        .expect(1)
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(BATCHES_PATH))
        .and(query_param("after", "b2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [batch("b3")],
            "first_id": "b3",
            "last_id": "b3",
            "has_more": false
        })))
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn openai_cursor_collect_all_follows_last_id_until_has_more_is_false() {
    let server = MockServer::start().await;
    mount_batch_pages(&server).await;

    let batches = openai(&server.uri()).list_batches().page_size(2).collect_all().await.unwrap();

    assert_eq!(batches.iter().map(|batch| batch.id.as_str()).collect::<Vec<_>>(), ["b1", "b2", "b3"]);
    let queries: Vec<_> = server.received_requests().await.unwrap().iter().map(query).collect();
    assert_eq!(queries, [pairs(&[("limit", "2")]), pairs(&[("after", "b2"), ("limit", "2")])]);
}

/// R11a for a cursor: resuming after a caller-held cursor sends exactly one
/// request carrying the cursor and the size.
#[tokio::test]
async fn openai_fetch_page_resumes_after_the_given_cursor_with_one_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(BATCHES_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [batch("b3")],
            "last_id": "b3",
            "has_more": true
        })))
        .mount(&server)
        .await;

    let page = openai(&server.uri()).list_batches().after("b2").page_size(5).fetch_page().await.unwrap();

    assert_eq!(page.items.iter().map(|batch| batch.id.as_str()).collect::<Vec<_>>(), ["b3"]);
    let received = server.received_requests().await.unwrap();
    assert_eq!(received.len(), 1);
    assert_eq!(query(&received[0]), pairs(&[("after", "b2"), ("limit", "5")]));
}

/// `has_more: true` with no `last_id` cannot continue: the traversal stops
/// with `Incomplete` instead of reporting the first page as the whole list.
#[tokio::test]
async fn openai_collect_all_without_a_continuation_is_incomplete() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(BATCHES_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [batch("b1")],
            "has_more": true
        })))
        .mount(&server)
        .await;

    let incomplete = openai(&server.uri()).list_batches().collect_all().await.unwrap_err();

    assert_eq!(incomplete.reason, IncompleteReason::InvalidContinuation);
    assert_eq!(incomplete.items.len(), 1);
    assert!(matches!(incomplete.error, Some(PagerError::MissingContinuation { .. })), "{:?}", incomplete.error);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

/// Mounts Bitbucket tag pages: page 1 (no `page` query) names an absolute
/// `next` link on the same origin; page 2 has no `next`.
async fn mount_tag_pages(server: &MockServer) {
    let next = format!("{}{TAGS_PATH}?pagelen=2&page=2", server.uri());
    Mock::given(method("GET"))
        .and(path(TAGS_PATH))
        .and(query_param_is_missing("page"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "pagelen": 2,
            "size": 3,
            "page": 1,
            "values": [tag("v1"), tag("v2")],
            "next": next
        })))
        .expect(1)
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(TAGS_PATH))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "pagelen": 2,
            "size": 3,
            "page": 2,
            "values": [tag("v3")],
            "previous": format!("{}{TAGS_PATH}?pagelen=2&page=1", server.uri())
        })))
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn bitbucket_next_link_collect_all_follows_next_until_it_is_absent() {
    let server = MockServer::start().await;
    mount_tag_pages(&server).await;

    let tags = bitbucket(&server.uri()).list_tags("team", "widgets").page_size(2).collect_all().await.unwrap();

    assert_eq!(tags.iter().map(|tag| tag.name.as_deref().unwrap()).collect::<Vec<_>>(), ["v1", "v2", "v3"]);
    let queries: Vec<_> = server.received_requests().await.unwrap().iter().map(query).collect();
    assert_eq!(queries[0], pairs(&[("pagelen", "2")]));
    assert_eq!(queries[1], pairs(&[("pagelen", "2"), ("page", "2")]), "the next link is followed as given");
}

#[tokio::test]
async fn bitbucket_pages_report_the_body_total_on_every_page() {
    let server = MockServer::start().await;
    mount_tag_pages(&server).await;
    let client = bitbucket(&server.uri());

    let mut pages = client.list_tags("team", "widgets").page_size(2).pages();
    let first = pages.next().await.unwrap().expect("page 1");
    assert_eq!((first.items.len(), first.total), (2, Some(3)));
    assert_eq!(pages.total(), Some(3));
    let second = pages.next().await.unwrap().expect("page 2");
    assert_eq!((second.items[0].name.as_deref(), second.total), (Some("v3"), Some(3)));
    assert!(pages.next().await.unwrap().is_none(), "no `next` ends the list");
}

/// R11a for a next-link endpoint: one request with the configured size.
#[tokio::test]
async fn bitbucket_fetch_page_sends_one_request_with_the_configured_size() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(TAGS_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "size": 40,
            "values": [tag("v1")],
            "next": format!("{}{TAGS_PATH}?page=2", server.uri())
        })))
        .mount(&server)
        .await;

    let page = bitbucket(&server.uri()).list_tags("team", "widgets").page_size(7).fetch_page().await.unwrap();

    assert_eq!((page.items.len(), page.total), (1, Some(40)));
    let received = server.received_requests().await.unwrap();
    assert_eq!(received.len(), 1);
    assert_eq!(query(&received[0]), pairs(&[("pagelen", "7")]));
}

/// A `next` link to another origin is refused under Bitbucket's default
/// same-origin policy; nothing is sent to it and the list is incomplete.
#[tokio::test]
async fn bitbucket_next_link_to_another_origin_is_refused() {
    let server = MockServer::start().await;
    let elsewhere = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(TAGS_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "values": [tag("v1")],
            "next": format!("{}{TAGS_PATH}?page=2", elsewhere.uri())
        })))
        .mount(&server)
        .await;

    let incomplete = bitbucket(&server.uri()).list_tags("team", "widgets").collect_all().await.unwrap_err();

    assert_eq!(incomplete.reason, IncompleteReason::CrossOriginRejected);
    assert_eq!(incomplete.items.len(), 1);
    assert!(elsewhere.received_requests().await.unwrap().is_empty());
}

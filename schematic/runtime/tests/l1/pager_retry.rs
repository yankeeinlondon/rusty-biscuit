//! Caller-controlled recovery retains evidence and the failed position.

use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use schematic_runtime::pager::{PagerError, PagerErrorKind};
use schematic_runtime::Deadline;
use serde_json::{Value, json};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};
use crate::pager_support::{BodyNextPrevious, InclusiveSince, LastIdHasMore, PitSearch, client, ids, names, queries, pairs};

#[derive(Clone)]
struct Sequence {
    index: Arc<AtomicUsize>,
    replies: Vec<ResponseTemplate>,
}
impl Respond for Sequence {
    fn respond(&self, _: &Request) -> ResponseTemplate {
        let index = self.index.fetch_add(1, Ordering::SeqCst);
        self.replies[index.min(self.replies.len() - 1)].clone()
    }
}
async fn sequence(server: &MockServer, replies: Vec<ResponseTemplate>) {
    Mock::given(wiremock::matchers::any()).respond_with(Sequence {
        index: Arc::new(AtomicUsize::new(0)), replies,
    }).mount(server).await;
}
fn doc(value: Value) -> ResponseTemplate { ResponseTemplate::new(200).set_body_json(value) }

#[tokio::test]
async fn retries_keep_new_error_evidence_and_the_failed_cursor_without_duplicate_items() {
    let server = MockServer::start().await;
    sequence(&server, vec![
        doc(json!({"data":[{"id":"one"}],"last_id":"one","has_more":true})),
        ResponseTemplate::new(500).set_body_string("first failure"),
        ResponseTemplate::new(503).set_body_string("new failure"),
        doc(json!({"data":[{"id":"two"}],"has_more":false})),
    ]).await;
    let client = client(&server);
    let mut pages = client.send(LastIdHasMore::get()).pager().pages();
    assert_eq!(names(&pages.next().await.unwrap().unwrap().items), ["one"]);
    let first = pages.next().await.unwrap_err();
    assert_eq!(pages.next().await.unwrap_err(), first);
    assert_eq!(pages.items_delivered(), 1);
    let second = pages.retry().await.unwrap_err();
    let PagerError::Api { error: new, meta } = &second else { panic!("{second:?}") };
    assert_eq!(meta.status, 503);
    assert_eq!(&new.body[..], b"new failure");
    assert_eq!(pages.next().await.unwrap_err(), second);
    pages.retry().await.unwrap();
    let recovered = pages.next().await.unwrap().unwrap();
    assert_eq!(recovered.number, 2);
    assert_eq!(names(&recovered.items), ["two"]);
    assert!(pages.next().await.unwrap().is_none());
    assert_eq!(pages.items_delivered(), 2);
    assert_eq!(pages.pages_fetched(), 4);
    assert_eq!(queries(&server).await, [vec![], pairs(&[("after","one")]), pairs(&[("after","one")]), pairs(&[("after","one")])]);
}

#[tokio::test]
async fn malformed_typed_items_roll_back_inclusive_boundary_and_delivery_state() {
    let server = MockServer::start().await;
    sequence(&server, vec![
        doc(json!([{"id":1,"t":10}])),
        doc(json!([{"id":1,"t":10},{"id":"invalid","t":20}])),
        doc(json!([{"id":1,"t":10},{"id":2,"t":20}])),
        doc(json!([])),
    ]).await;
    let client = client(&server);
    let mut pages = client.send(InclusiveSince::get()).pager().pages();
    assert_eq!(ids(&pages.next().await.unwrap().unwrap().items), [1]);
    assert!(matches!(pages.next().await, Err(PagerError::Api { .. })));
    assert_eq!(pages.items_delivered(), 1);
    pages.retry().await.unwrap();
    assert_eq!(ids(&pages.next().await.unwrap().unwrap().items), [2]);
    assert!(pages.next().await.unwrap().unwrap().items.is_empty());
    assert_eq!(pages.items_delivered(), 2);
    assert_eq!(queries(&server).await, [vec![], pairs(&[("since","10")]), pairs(&[("since","10")]), pairs(&[("since","20")])]);
}

macro_rules! continuation_retry {
    ($name:ident, $bad:expr, $kind:ident) => {
        #[tokio::test]
        async fn $name() {
            let server = MockServer::start().await;
            sequence(&server, vec![
                doc(json!({"data":[{"id":"one"}],"last_id":"a","has_more":true})),
                doc($bad),
                doc(json!({"data":[{"id":"two"}],"last_id":"b","has_more":true})),
                doc(json!({"data":[{"id":"three"}],"has_more":false})),
            ]).await;
            let client = client(&server);
            let mut items = client.send(LastIdHasMore::get()).pager().pages().items();
            assert_eq!(items.next().await.unwrap().unwrap().id, "one");
            assert_eq!(items.next().await.unwrap().unwrap().id, "two");
            let error = items.next().await.unwrap_err();
            assert_eq!(error.kind(), PagerErrorKind::$kind);
            assert_eq!(items.next().await.unwrap_err(), error);
            assert_eq!(server.received_requests().await.unwrap().len(), 2);
            items.retry().await.unwrap();
            assert_eq!(items.items_delivered(), 2);
            assert_eq!(items.next().await.unwrap().unwrap().id, "three");
            assert!(items.next().await.unwrap().is_none());
            assert_eq!(items.items_delivered(), 3);
            assert_eq!(queries(&server).await, [vec![], pairs(&[("after","a")]), pairs(&[("after","a")]), pairs(&[("after","b")])]);
        }
    }
}
continuation_retry!(missing_continuation_refresh_does_not_redeliver, json!({"data":[{"id":"two"}],"has_more":true}), MissingContinuation);
continuation_retry!(invalid_continuation_refresh_does_not_redeliver, json!({"data":[{"id":"two"}],"last_id":[],"has_more":true}), InvalidContinuation);
continuation_retry!(loop_retry_refreshes_source_without_redelivery, json!({"data":[{"id":"two"}],"last_id":"a","has_more":true}), RepeatedContinuation);

#[tokio::test]
async fn backward_retry_preserves_direction_and_complete_error() {
    let server = MockServer::start().await;
    sequence(&server, vec![
        doc(json!({"values":[{"id":2}],"previous":"/earlier"})),
        ResponseTemplate::new(503).set_body_string("backward failure"),
        doc(json!({"values":[{"id":1}]})),
    ]).await;
    let client = client(&server);
    let mut pages = client.send(BodyNextPrevious::get()).pager().pages();
    pages.next().await.unwrap();
    let error = pages.prev().await.unwrap_err();
    assert_eq!(pages.next().await.unwrap_err(), error);
    assert_eq!(pages.prev().await.unwrap_err(), error);
    pages.retry().await.unwrap();
    assert_eq!(ids(&pages.prev().await.unwrap().unwrap().items), [1]);
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.iter().map(|r| r.url.path()).collect::<Vec<_>>(), ["/v2/repos/list","/earlier","/earlier"]);
}

#[tokio::test]
async fn every_attempt_can_retry_an_expired_deadline_or_invalid_argument_without_sending() {
    let server = MockServer::start().await;
    let client = client(&server);
    let expired = Deadline::after(std::time::Duration::ZERO);
    let mut deadline = client.send(LastIdHasMore::get()).pager().deadline(&expired).pages();
    let first = deadline.next().await.unwrap_err();
    assert_eq!(first.kind(), PagerErrorKind::DeadlineExceeded);
    let second = deadline.retry().await.unwrap_err();
    assert_eq!(second.kind(), PagerErrorKind::DeadlineExceeded);
    assert_eq!(deadline.next().await.unwrap_err(), second);
    let mut invalid = client.send(LastIdHasMore::get()).pager().page_size(0).pages();
    let error = invalid.next().await.unwrap_err();
    assert_eq!(error.kind(), PagerErrorKind::Transport);
    let retried = invalid.retry().await.unwrap_err();
    assert_eq!(retried.kind(), error.kind());
    assert_eq!(invalid.next().await.unwrap_err(), retried);
    assert_eq!(invalid.pages_fetched(), 2);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn session_open_retry_replaces_shared_failure_and_closes_recovered_handle() {
    let server = MockServer::start().await;
    sequence(&server, vec![ResponseTemplate::new(503).set_body_string("open failed"),
        doc(json!({"id":"recovered"})), doc(json!({"hits":{"hits":[]}})), doc(json!({}))]).await;
    let client = client(&server);
    client.send(PitSearch::get()).pager().with_session(|scope| Box::pin(async move {
        let mut pages = scope.pages();
        let first = pages.next().await.unwrap_err();
        assert_eq!(pages.next().await.unwrap_err(), first);
        pages.retry().await.unwrap();
        assert!(pages.next().await.unwrap().unwrap().items.is_empty());
        assert!(pages.next().await.unwrap().is_none());
    })).await.unwrap();
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.iter().map(|r| r.url.path()).collect::<Vec<_>>(), ["/logs/_pit","/logs/_pit","/_search","/_pit"]);
    assert_eq!(serde_json::from_slice::<Value>(&requests[3].body).unwrap(), json!({"id":"recovered"}));
}

#[tokio::test]
async fn boundary_stall_retry_keeps_prior_identity_history() {
    let server = MockServer::start().await;
    sequence(&server, vec![doc(json!([{"id":1,"t":10}])), doc(json!([{"id":1,"t":10}])),
        doc(json!([{"id":1,"t":10},{"id":2,"t":10}])), doc(json!([]))]).await;
    let client = client(&server);
    let mut pages = client.send(InclusiveSince::get()).pager().pages();
    pages.next().await.unwrap();
    let failed = pages.next().await.unwrap_err();
    assert_eq!(failed.kind(), PagerErrorKind::BoundaryNoProgress);
    assert_eq!(pages.next().await.unwrap_err(), failed);
    pages.retry().await.unwrap();
    assert_eq!(ids(&pages.next().await.unwrap().unwrap().items), [2]);
    assert_eq!(pages.items_delivered(), 2);
    assert_eq!(queries(&server).await, [vec![], pairs(&[("since","10")]), pairs(&[("since","10")])]);
}

#[tokio::test]
async fn empty_page_claiming_more_can_be_retried_at_the_same_offset() {
    use crate::pager_support::IsLastOffset;
    let server = MockServer::start().await;
    sequence(&server, vec![doc(json!({"values":[],"isLast":false})),
        doc(json!({"values":[{"id":1}],"isLast":true}))]).await;
    let client = client(&server);
    let mut pages = client.send(IsLastOffset::get()).pager().pages();
    let failed = pages.next().await.unwrap_err();
    assert_eq!(failed.kind(), PagerErrorKind::EmptyPageClaimsMore);
    pages.retry().await.unwrap();
    assert_eq!(ids(&pages.next().await.unwrap().unwrap().items), [1]);
    assert!(pages.next().await.unwrap().is_none());
    let requests = queries(&server).await;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0], requests[1]);
}

#[tokio::test]
async fn missing_session_handle_and_new_open_failures_remain_retryable() {
    let server = MockServer::start().await;
    sequence(&server, vec![doc(json!({"acknowledged":true})), ResponseTemplate::new(503).set_body_string("new open error"),
        doc(json!({"id":"recovered"})), doc(json!({"hits":{"hits":[]}})), doc(json!({}))]).await;
    let client = client(&server);
    client.send(PitSearch::get()).pager().with_session(|scope| Box::pin(async move {
        let mut pages = scope.pages();
        let first = pages.next().await.unwrap_err();
        assert_eq!(first.kind(), PagerErrorKind::MissingSessionHandle);
        assert_eq!(pages.next().await.unwrap_err(), first);
        let second = pages.retry().await.unwrap_err();
        let PagerError::Api { error, meta } = &second else { panic!("{second:?}") };
        assert_eq!(meta.status, 503);
        assert_eq!(&error.body[..], b"new open error");
        assert_eq!(pages.next().await.unwrap_err(), second);
        pages.retry().await.unwrap();
        assert!(pages.next().await.unwrap().unwrap().items.is_empty());
    })).await.unwrap();
    assert_eq!(server.received_requests().await.unwrap().len(), 5);
}

#[tokio::test]
async fn deferred_retry_failure_replaces_evidence_and_no_failure_retry_is_a_noop() {
    let server = MockServer::start().await;
    sequence(&server, vec![doc(json!({"data":[{"id":"one"}],"has_more":true})),
        doc(json!({"data":[{"id":"one"}],"last_id":[],"has_more":true})),
        doc(json!({"data":[{"id":"one"}],"has_more":false}))]).await;
    let client = client(&server);
    let mut pages = client.send(LastIdHasMore::get()).pager().pages();
    pages.retry().await.unwrap();
    assert!(server.received_requests().await.unwrap().is_empty());
    pages.next().await.unwrap();
    assert_eq!(pages.next().await.unwrap_err().kind(), PagerErrorKind::MissingContinuation);
    let second = pages.retry().await.unwrap_err();
    assert_eq!(second.kind(), PagerErrorKind::InvalidContinuation);
    assert_eq!(pages.next().await.unwrap_err(), second);
    pages.retry().await.unwrap();
    assert!(pages.next().await.unwrap().is_none());
    assert_eq!(pages.items_delivered(), 1);
    pages.retry().await.unwrap();
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}

#[tokio::test]
async fn retained_provider_payload_clones_without_provider_clone() {
    #[derive(Debug)]
    struct NoClone;
    let server = MockServer::start().await;
    sequence(&server, vec![ResponseTemplate::new(500)]).await;
    let client = client(&server);
    let PagerError::Api { meta, .. } = client.send(LastIdHasMore::get()).pager().fetch_page().await.unwrap_err() else { panic!("API failure") };
    let error = PagerError::Api { error: Arc::new(schematic_runtime::ApiError {
        kind: schematic_runtime::ErrorKind::Declared(NoClone), body: "evidence".into(),
    }), meta };
    let copy = error.clone();
    let (PagerError::Api { error: a, .. }, PagerError::Api { error: b, .. }) = (error, copy) else { panic!("API failure") };
    assert!(Arc::ptr_eq(&a, &b));
}

#[tokio::test]
async fn partition_retry_does_not_reset_successful_siblings() {
    let server = MockServer::start().await;
    sequence(&server, vec![doc(json!({"id":"session"})), ResponseTemplate::new(503),
        doc(json!({"hits":{"hits":[]}})), doc(json!({"hits":{"hits":[]}})), doc(json!({}))]).await;
    let client = client(&server);
    client.send(PitSearch::get()).pager().with_session(|scope| Box::pin(async move {
        let mut parts = scope.partitioned(2).unwrap();
        assert!(parts[0].next().await.is_err());
        assert!(parts[1].next().await.unwrap().unwrap().items.is_empty());
        parts[0].retry().await.unwrap();
        assert!(parts[0].next().await.unwrap().unwrap().items.is_empty());
        assert!(parts[1].next().await.unwrap().is_none());
        assert_eq!(parts[0].pages_fetched(), 2);
        assert_eq!(parts[1].pages_fetched(), 1);
    })).await.unwrap();
    let requests = server.received_requests().await.unwrap();
    let bodies = requests[1..4].iter().map(|request| serde_json::from_slice::<Value>(&request.body).unwrap()["slice"].clone()).collect::<Vec<_>>();
    assert_eq!(bodies, [json!({"id":0,"max":2}), json!({"id":1,"max":2}), json!({"id":0,"max":2})]);
}

#[tokio::test]
async fn adjustable_size_can_repair_an_argument_before_explicit_retry() {
    use crate::pager_support::FullyAuthored;
    let server = MockServer::start().await;
    sequence(&server, vec![doc(json!({"data":[{"id":"one"}],"has_more":false}))]).await;
    let client = client(&server);
    let mut pages = client.send(FullyAuthored::get()).pager().page_size(0).pages();
    assert!(pages.next().await.is_err());
    pages.set_page_size(5);
    pages.retry().await.unwrap();
    assert_eq!(names(&pages.next().await.unwrap().unwrap().items), ["one"]);
    assert_eq!(queries(&server).await, [pairs(&[("limit","5")])]);
}

#[tokio::test]
async fn refused_link_errors_remain_complete_and_explicitly_retryable() {
    let server = MockServer::start().await;
    sequence(&server, vec![doc(json!({"values":[{"id":1}],"next":"http://example.invalid/next"}))]).await;
    let client = client(&server);
    let mut pages = client.send(BodyNextPrevious::get()).pager().pages();
    pages.next().await.unwrap();
    let first = pages.next().await.unwrap_err();
    assert_eq!(first.kind(), PagerErrorKind::Transport);
    assert_eq!(pages.next().await.unwrap_err(), first);
    let retried = pages.retry().await.unwrap_err();
    assert_eq!(retried.kind(), PagerErrorKind::Transport);
    assert_eq!(pages.prev().await.unwrap_err(), retried);
    assert_eq!(pages.items_delivered(), 1);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn cancelling_a_deferred_retry_preserves_delivered_items_and_the_retained_failure() {
    let server = MockServer::start().await;
    sequence(&server, vec![doc(json!({"data":[{"id":"one"}],"has_more":true})),
        doc(json!({"data":[{"id":"one"}],"has_more":false})).set_delay(std::time::Duration::from_secs(10))]).await;
    let client = client(&server);
    let mut pages = client.send(LastIdHasMore::get()).pager().pages();
    pages.next().await.unwrap();
    let original = pages.next().await.unwrap_err();
    {
        let retry = pages.retry();
        tokio::pin!(retry);
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            tokio::select! {
                result = &mut retry => panic!("delayed retry unexpectedly completed: {result:?}"),
                _ = async {
                    while server.received_requests().await.unwrap().len() != 2 {
                        tokio::task::yield_now().await;
                    }
                } => {}
            }
        }).await.unwrap();
    }
    assert_eq!(pages.items_delivered(), 1);
    assert_eq!(pages.next().await.unwrap_err(), original);
    assert_eq!(pages.pages_fetched(), 2);
}

#[tokio::test]
async fn recovered_page_items_count_only_after_the_page_is_delivered() {
    let server = MockServer::start().await;
    sequence(&server, vec![
        doc(json!({"data":[{"id":"one"}],"last_id":"a","has_more":true})),
        ResponseTemplate::new(503),
        doc(json!({"data":[{"id":"two"},{"id":"three"}],"has_more":false})),
    ]).await;
    let client = client(&server);
    let mut pages = client.send(LastIdHasMore::get()).pager().pages();
    assert_eq!(pages.items_delivered(), 0);
    assert_eq!(names(&pages.next().await.unwrap().unwrap().items), ["one"]);
    assert_eq!(pages.items_delivered(), 1);
    assert!(pages.next().await.is_err());
    assert_eq!(pages.items_delivered(), 1);
    pages.retry().await.unwrap();
    assert_eq!(pages.items_delivered(), 1);
    assert_eq!(names(&pages.next().await.unwrap().unwrap().items), ["two", "three"]);
    assert_eq!(pages.items_delivered(), 3);
    assert!(pages.next().await.unwrap().is_none());
    assert_eq!(pages.items_delivered(), 3);
    assert_eq!(queries(&server).await, [vec![], pairs(&[("after","a")]), pairs(&[("after","a")])]);
}

#[tokio::test]
async fn recovered_items_count_only_as_each_buffered_item_is_delivered() {
    let server = MockServer::start().await;
    sequence(&server, vec![
        doc(json!({"data":[{"id":"one"}],"last_id":"a","has_more":true})),
        ResponseTemplate::new(503),
        doc(json!({"data":[{"id":"two"},{"id":"three"}],"has_more":false})),
    ]).await;
    let client = client(&server);
    let mut items = client.send(LastIdHasMore::get()).pager().pages().items();
    assert_eq!(items.items_delivered(), 0);
    assert_eq!(items.next().await.unwrap().unwrap().id, "one");
    assert_eq!(items.items_delivered(), 1);
    assert!(items.next().await.is_err());
    assert_eq!(items.items_delivered(), 1);
    items.retry().await.unwrap();
    assert_eq!(items.items_delivered(), 1);
    assert_eq!(items.next().await.unwrap().unwrap().id, "two");
    assert_eq!(items.items_delivered(), 2);
    assert_eq!(items.next().await.unwrap().unwrap().id, "three");
    assert_eq!(items.items_delivered(), 3);
    assert!(items.next().await.unwrap().is_none());
    assert_eq!(items.items_delivered(), 3);
    assert_eq!(queries(&server).await, [vec![], pairs(&[("after","a")]), pairs(&[("after","a")])]);
}

#[tokio::test]
async fn recovered_page_continuation_refresh_keeps_the_recovered_source_checkpoint() {
    let server = MockServer::start().await;
    sequence(&server, vec![
        doc(json!({"data":[{"id":"one"}],"last_id":"a","has_more":true})),
        ResponseTemplate::new(503),
        doc(json!({"data":[{"id":"two"},{"id":"three"}],"has_more":true})),
        doc(json!({"data":[{"id":"two"},{"id":"three"}],"last_id":"b","has_more":true})),
        doc(json!({"data":[{"id":"four"}],"has_more":false})),
    ]).await;
    let client = client(&server);
    let mut items = client.send(LastIdHasMore::get()).pager().pages().items();
    assert_eq!(items.next().await.unwrap().unwrap().id, "one");
    assert!(matches!(items.next().await, Err(PagerError::Api { .. })));
    items.retry().await.unwrap();
    assert_eq!(items.next().await.unwrap().unwrap().id, "two");
    assert_eq!(items.next().await.unwrap().unwrap().id, "three");
    let missing = items.next().await.unwrap_err();
    assert_eq!(missing.kind(), PagerErrorKind::MissingContinuation);
    assert_eq!(items.next().await.unwrap_err(), missing);
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
    items.retry().await.unwrap();
    assert_eq!(items.items_delivered(), 3);
    assert_eq!(items.next().await.unwrap().unwrap().id, "four");
    assert!(items.next().await.unwrap().is_none());
    assert_eq!(items.items_delivered(), 4);
    assert_eq!(queries(&server).await, [vec![], pairs(&[("after","a")]), pairs(&[("after","a")]), pairs(&[("after","a")]), pairs(&[("after","b")])]);
}

macro_rules! adjustable_continuation_retry {
    ($name:ident, $change_after_failure:expr, $recover_source:expr, $flatten:expr) => {
        #[tokio::test]
        async fn $name() {
            use crate::pager_support::FullyAuthored;
            let server = MockServer::start().await;
            let mut replies = Vec::new();
            if $recover_source {
                replies.push(ResponseTemplate::new(503));
            }
            replies.extend([
                doc(json!({"data":[{"id":"a"}],"has_more":true})),
                doc(json!({"data":[{"id":"b"}],"has_more":true})),
                doc(json!({"data":[{"id":"c"}],"has_more":false})),
            ]);
            sequence(&server, replies).await;
            let client = client(&server);
            let mut pages = client.send(FullyAuthored::get()).pager().after("a").page_size(5).pages();
            if $recover_source {
                assert!(matches!(pages.next().await, Err(PagerError::Api { .. })));
                pages.retry().await.unwrap();
            }
            assert_eq!(names(&pages.next().await.unwrap().unwrap().items), ["a"]);
            if !$change_after_failure {
                pages.set_page_size(7);
            }
            assert_eq!(pages.next().await.unwrap_err().kind(), PagerErrorKind::RepeatedContinuation);
            if $change_after_failure {
                pages.set_page_size(7);
            }
            let mut delivered = vec!["a".to_string()];
            if $flatten {
                let mut items = pages.items();
                items.retry().await.unwrap();
                while let Some(item) = items.next().await.unwrap() {
                    delivered.push(item.id);
                }
                assert_eq!(items.items_delivered(), 2);
            } else {
                pages.retry().await.unwrap();
                while let Some(page) = pages.next().await.unwrap() {
                    delivered.extend(page.items.into_iter().map(|item| item.id));
                }
                assert_eq!(pages.items_delivered(), 2);
            }
            assert_eq!(delivered, ["a", "c"]);
            let mut expected = vec![pairs(&[("after","a"),("limit","5")])];
            if $recover_source {
                expected.push(pairs(&[("after","a"),("limit","5")]));
            }
            expected.extend([
                pairs(&[("after","a"),("limit","7")]),
                pairs(&[("after","b"),("limit","7")]),
            ]);
            assert_eq!(queries(&server).await, expected);
        }
    };
}
adjustable_continuation_retry!(size_change_before_deferred_failure_survives_source_refresh, false, false, false);
adjustable_continuation_retry!(size_change_after_deferred_failure_survives_source_refresh, true, false, false);
adjustable_continuation_retry!(recovered_source_keeps_size_change_through_items_refresh, false, true, true);

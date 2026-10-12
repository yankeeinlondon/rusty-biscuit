//! Q5: the sniff-style call site ported onto the envelope.

use std::time::Duration;

use serde_json::json;
use typed_send_spike::client::DemoClient;
use typed_send_spike::runtime::Secret;
use typed_send_spike::sniff_port::{SniffError, get_pull, list_pulls};
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn pull(n: u64) -> serde_json::Value {
    json!({"number": n, "title": format!("PR {n}"), "state": "open"})
}

#[tokio::test]
async fn get_pull_found_and_404_is_none() {
    let server = MockServer::start().await;
    Mock::given(path("/repos/o/r/pulls/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(pull(1)))
        .mount(&server)
        .await;
    Mock::given(path("/repos/o/r/pulls/2"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"message": "Not Found"})))
        .mount(&server)
        .await;
    let c = DemoClient::builder(server.uri()).build();
    assert_eq!(get_pull(&c, "o", "r", 1).await.unwrap().unwrap().number, 1);
    assert_eq!(get_pull(&c, "o", "r", 2).await.unwrap(), None);
}

#[tokio::test]
async fn rate_limit_reads_retry_after() {
    let server = MockServer::start().await;
    Mock::given(path("/repos/o/r/pulls/3"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "7")
                .set_body_json(json!({"message": "slow down"})),
        )
        .mount(&server)
        .await;
    Mock::given(path("/repos/o/r/pulls/4"))
        .respond_with(
            ResponseTemplate::new(403)
                .insert_header("x-ratelimit-remaining", "0")
                .set_body_string("<html>not json</html>"),
        )
        .mount(&server)
        .await;
    let c = DemoClient::builder(server.uri()).build();
    assert_eq!(
        get_pull(&c, "o", "r", 3).await.unwrap_err(),
        SniffError::RateLimited {
            retry_after: Some(Duration::from_secs(7))
        }
    );
    assert_eq!(
        get_pull(&c, "o", "r", 4).await.unwrap_err(),
        SniffError::RateLimited { retry_after: None }
    );
}

#[tokio::test]
async fn unauthorized_depends_on_auth_method() {
    let server = MockServer::start().await;
    Mock::given(path("/repos/o/r/pulls/5"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({"message": "Bad credentials"})))
        .mount(&server)
        .await;
    let anon = DemoClient::builder(server.uri()).build();
    let keyed = DemoClient::builder(server.uri())
        .credential(Secret::new("t"))
        .build();
    assert!(matches!(
        get_pull(&anon, "o", "r", 5).await.unwrap_err(),
        SniffError::MissingCredentials { .. }
    ));
    assert_eq!(
        get_pull(&keyed, "o", "r", 5).await.unwrap_err(),
        SniffError::InvalidCredentials
    );
}

#[tokio::test]
async fn list_follows_link_next_and_reports_incomplete() {
    let server = MockServer::start().await;
    let next = format!("<{}/repos/o/r/pulls?page=2>; rel=\"next\"", server.uri());
    Mock::given(path("/repos/o/r/pulls"))
        .and(query_param("page", "1"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("link", next.as_str())
                .set_body_json(json!([pull(1), pull(2)])),
        )
        .mount(&server)
        .await;
    Mock::given(path("/repos/o/r/pulls"))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([pull(3)])))
        .mount(&server)
        .await;
    let c = DemoClient::builder(server.uri()).build();
    let all = list_pulls(&c, "o", "r", 10).await.unwrap();
    assert_eq!(all.iter().map(|p| p.number).collect::<Vec<_>>(), vec![1, 2, 3]);
    assert_eq!(
        list_pulls(&c, "o", "r", 1).await.unwrap_err(),
        SniffError::Incomplete { pages: 1 }
    );
}

#[tokio::test]
async fn unreachable_maps_from_transport() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let c = DemoClient::builder(base).build();
    assert!(matches!(
        get_pull(&c, "o", "r", 1).await.unwrap_err(),
        SniffError::RemoteUnreachable { .. }
    ));
}

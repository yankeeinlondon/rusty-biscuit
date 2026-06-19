use biscuit_icon::Icon;
use biscuit_icon::cache::IconCache;
use biscuit_icon::iconify::IconifyClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn miss_fetches_then_hit_uses_cache() {
    let server = MockServer::start().await;
    let json = serde_json::json!({
        "prefix": "mdi", "width": 24, "height": 24,
        "icons": { "home": { "body": "<path d=\"M1 1\"/>" } }
    });
    // Respond at most once: the second lookup must hit the cache, not the network.
    Mock::given(method("GET"))
        .and(path("/mdi.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json))
        .expect(1)
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let cache = IconCache::open_at(dir.path().join("c.db")).unwrap();
    let client = IconifyClient::with_base(server.uri());

    let first = Icon::iconify_with("mdi:home", &cache, &client).await.unwrap();
    assert!(first.svg().contains("M1 1"));
    let second = Icon::iconify_with("mdi:home", &cache, &client).await.unwrap();
    assert!(second.svg().contains("M1 1"));
    // `expect(1)` is verified on server drop: exactly one network call occurred.
}

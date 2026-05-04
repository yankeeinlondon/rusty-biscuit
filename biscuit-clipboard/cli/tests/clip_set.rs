//! Phase 6 / Test Coverage #5: `clip set <text>` POSTs the tagged-enum
//! body shape `{"content_type":"text","data":"..."}` to `/set`.

use assert_cmd::Command;
use predicates::prelude::*;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, ResponseTemplate};

mod common;

#[tokio::test(flavor = "multi_thread")]
async fn clip_set_posts_tagged_enum_body() {
    let svc = common::MockService::start().await;

    let expected = serde_json::json!({
        "content_type": "text",
        "data": "hello world",
    });

    Mock::given(method("POST"))
        .and(path("/set"))
        .and(header("content-type", "application/json"))
        .and(body_json(&expected))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-clipper", "1")
                .set_body_json(serde_json::json!({
                    "id": "deadbeefcafef00d",
                })),
        )
        .expect(1)
        .mount(&svc.server)
        .await;

    let runtime_path = svc.runtime_path().to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut cmd = Command::cargo_bin("clip").expect("locate clip binary");
        cmd.args(["set", "hello world"])
            .env("CLIP_RUNTIME_DIR", &runtime_path)
            .assert()
            .success()
            .stdout(predicate::str::contains("deadbeefcafef00d"));
    })
    .await
    .unwrap();
}

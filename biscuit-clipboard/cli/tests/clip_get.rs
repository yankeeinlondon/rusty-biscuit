//! Phase 6 / Test Coverage #5: `clip get` against a wiremock-backed
//! mock service prints the expected stdout.

use assert_cmd::Command;
use predicates::prelude::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

mod common;

#[tokio::test(flavor = "multi_thread")]
async fn clip_get_prints_current_clipboard_text() {
    let svc = common::MockService::start().await;

    Mock::given(method("GET"))
        .and(path("/current"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-clipper", "1")
                .insert_header("content-type", "text/plain; charset=utf-8")
                .set_body_string("hello from mock"),
        )
        .mount(&svc.server)
        .await;

    let runtime_path = svc.runtime_path().to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut cmd = Command::cargo_bin("clip").expect("locate clip binary");
        cmd.arg("get")
            .env("CLIP_RUNTIME_DIR", &runtime_path)
            .assert()
            .success()
            .stdout(predicate::str::contains("hello from mock"));
    })
    .await
    .unwrap();
}

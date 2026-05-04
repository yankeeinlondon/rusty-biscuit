//! Phase 6 / Test Coverage #5: `clip history --json` produces valid JSON
//! that round-trips back to a `Vec<EntrySummary>`-shaped array.

use assert_cmd::Command;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

mod common;

#[tokio::test(flavor = "multi_thread")]
async fn clip_history_json_emits_valid_json_array() {
    let svc = common::MockService::start().await;

    Mock::given(method("GET"))
        .and(path("/history"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-clipper", "1")
                .set_body_json(serde_json::json!({
                    "entries": [
                        {
                            "id": "0000000000000001",
                            "timestamp": "2026-05-03T00:00:00Z",
                            "content_type": "text",
                            "preview": "first",
                            "size_bytes": 5,
                        },
                        {
                            "id": "0000000000000002",
                            "timestamp": "2026-05-03T00:00:01Z",
                            "content_type": "text",
                            "preview": "second",
                            "size_bytes": 6,
                        },
                    ],
                })),
        )
        .mount(&svc.server)
        .await;

    let runtime_path = svc.runtime_path().to_path_buf();
    let output = tokio::task::spawn_blocking(move || {
        let mut cmd = Command::cargo_bin("clip").expect("locate clip binary");
        cmd.args(["history", "--json"])
            .env("CLIP_RUNTIME_DIR", &runtime_path)
            .assert()
            .success()
            .get_output()
            .clone()
    })
    .await
    .unwrap();

    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("clip history --json must emit valid JSON");
    let arr = parsed
        .as_array()
        .expect("history --json must emit a JSON array");
    assert_eq!(arr.len(), 2, "expected 2 entries, got {arr:?}");
    assert_eq!(arr[0]["preview"], "first");
    assert_eq!(arr[1]["preview"], "second");
}

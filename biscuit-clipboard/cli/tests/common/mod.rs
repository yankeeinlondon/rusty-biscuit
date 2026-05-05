//! Shared test fixtures for `clip` CLI integration tests.
//!
//! Exposes [`MockService`], a wiremock-backed stand-in for the real
//! `clipper` service. The fixture writes `clipper.pid` (the test's own
//! PID — guaranteed alive) and `clipper.port` (the mock server's port)
//! into a temp `runtime_dir` so the `CLIP_RUNTIME_DIR` env var points
//! the `clip` binary at the mock.
//!
//! ## Notes
//!
//! - The mock always responds to `/health` with `200` + `X-Clipper: 1`
//!   so `ClipperClient::ensure_running` short-circuits at `try_connect`
//!   and never tries to spawn a real `clipper` binary.
//! - Each `MockService` owns the `TempDir`; drop both together.

use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub struct MockService {
    pub server: MockServer,
    pub runtime_dir: TempDir,
}

impl MockService {
    pub async fn start() -> Self {
        let server = MockServer::start().await;
        let runtime_dir = tempfile::tempdir().expect("tempdir");

        // Write pid + port files so `ClipperClient::try_connect` finds us.
        let pid = std::process::id();
        std::fs::write(runtime_dir.path().join("clipper.pid"), pid.to_string()).unwrap();
        let port = server.address().port();
        std::fs::write(runtime_dir.path().join("clipper.port"), port.to_string()).unwrap();

        // Default `/health` mock — every CLI command calls
        // `ensure_running` which calls `health_check` first.
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-clipper", "1")
                    .set_body_json(serde_json::json!({
                        "status": "ok",
                        "entries": 0,
                        "watcher": "running",
                    })),
            )
            .mount(&server)
            .await;

        Self {
            server,
            runtime_dir,
        }
    }

    pub fn runtime_path(&self) -> &std::path::Path {
        self.runtime_dir.path()
    }
}

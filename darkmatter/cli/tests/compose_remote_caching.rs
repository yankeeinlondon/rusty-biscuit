mod common;

use common::{
    CliProcessFixture, MOCK_HTTP_SHUTDOWN_BOUND, MockHttpResponse, MockHttpServer, mock_http_server,
};
use predicates::prelude::*;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn assert_get_requests(server: &MockHttpServer, paths: &[&str]) {
    let requests = server.requests();
    assert_eq!(requests.len(), paths.len(), "captured HTTP requests");
    for (request, path) in requests.iter().zip(paths) {
        assert!(
            request.starts_with(&format!("GET {path} HTTP/1.1\r\n")),
            "unexpected request line: {request:?}"
        );
        assert!(
            request
                .lines()
                .any(|line| line.to_ascii_lowercase().starts_with("host: 127.0.0.1:")),
            "request must target only the local fixture: {request:?}"
        );
    }
}

#[test]
fn mock_http_server_without_expected_request_shuts_down_within_bound() {
    let server = mock_http_server(vec![MockHttpResponse {
        status: 200,
        body: "never requested\n",
        cache_control: None,
    }]);
    let started = Instant::now();

    server.shutdown();

    assert!(
        started.elapsed() <= MOCK_HTTP_SHUTDOWN_BOUND + Duration::from_millis(250),
        "mock server exceeded its documented shutdown bound"
    );
}

#[test]
fn test_compose_remote_allowed_host_fetches_url() {
    let fixture = CliProcessFixture::named("test_compose_remote_allowed_host_fetches_url");
    let server = mock_http_server(vec![MockHttpResponse {
        status: 200,
        body: "Remote body\n",
        cache_control: None,
    }]);
    let url = server.url("/remote.md");

    fixture
        .command()
        .args(["compose", "-", "--allow-host", "127.0.0.1"])
        .write_stdin(format!("# Local\n\n::file {url}\n"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Remote body"));

    assert_eq!(server.request_count(), 1);
    assert_get_requests(&server, &["/remote.md"]);
}

#[test]
fn test_compose_remote_deny_all_fails_without_request() {
    let fixture = CliProcessFixture::named("test_compose_remote_deny_all_fails_without_request");
    let server = mock_http_server(vec![MockHttpResponse {
        status: 200,
        body: "should not be fetched\n",
        cache_control: None,
    }]);
    let url = server.url("/blocked.md");

    fixture
        .command()
        .args(["compose", "-"])
        .write_stdin(format!("# Local\n\n::file {url}\n"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("remote read denied"))
        .stderr(predicate::str::contains("127.0.0.1"));

    assert_eq!(server.request_count(), 0);
    assert!(server.requests().is_empty());
}

#[test]
fn test_compose_remote_expression_function_reads_url() {
    let fixture = CliProcessFixture::named("test_compose_remote_expression_function_reads_url");
    // Read-side expression functions must work through the real `md compose`
    // pipeline, not just helper-level unit tests. The URL argument is quoted
    // because the interpolation expression parser requires a string literal.
    let server = mock_http_server(vec![MockHttpResponse {
        status: 200,
        body: "# Remote Heading\n",
        cache_control: None,
    }]);
    let url = server.url("/remote.md");

    fixture
        .command()
        .args(["compose", "-", "--allow-host", "127.0.0.1"])
        .write_stdin(format!("Title: {{{{ markdown_title(\"{url}\") }}}}\n"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Title: Remote Heading"));

    assert_eq!(server.request_count(), 1);
    assert_get_requests(&server, &["/remote.md"]);
}

#[test]
fn test_compose_remote_expression_function_denied_host_reads_false() {
    let fixture =
        CliProcessFixture::named("test_compose_remote_expression_function_denied_host_reads_false");
    // `file_exists` against a host that is not allowed must read as `false`
    // (the fetch is policy-denied, never issued) rather than failing compose.
    let server = mock_http_server(vec![MockHttpResponse {
        status: 200,
        body: "should not be fetched\n",
        cache_control: None,
    }]);
    let url = server.url("/blocked.md");

    fixture
        .command()
        .args(["compose", "-"])
        .write_stdin(format!("Exists: {{{{ file_exists(\"{url}\") }}}}\n"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Exists: false"));

    assert_eq!(server.request_count(), 0);
    assert!(server.requests().is_empty());
}

#[test]
fn test_compose_remote_prologue_allowed_host_fetches_url() {
    let fixture = CliProcessFixture::named("test_compose_remote_prologue_allowed_host_fetches_url");
    // A remote `prologue` URL on an allowed host must be registered, fetched,
    // and prepended to the body.
    let server = mock_http_server(vec![MockHttpResponse {
        status: 200,
        body: "Prologue body\n",
        cache_control: None,
    }]);
    let url = server.url("/intro.md");

    fixture
        .command()
        .args(["compose", "-", "--allow-host", "127.0.0.1"])
        .write_stdin(format!("---\nprologue: {url}\n---\n# Local\n\nBody.\n"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Prologue body"))
        .stdout(predicate::str::contains("Local"));

    assert_eq!(server.request_count(), 1);
    assert_get_requests(&server, &["/intro.md"]);
}

#[test]
fn test_compose_remote_epilogue_deny_all_fails_without_request() {
    let fixture =
        CliProcessFixture::named("test_compose_remote_epilogue_deny_all_fails_without_request");
    // A remote `epilogue` on a non-allowed host must fail by policy and never
    // issue a request — not fail with an internal "not registered" error.
    let server = mock_http_server(vec![MockHttpResponse {
        status: 200,
        body: "should not be fetched\n",
        cache_control: None,
    }]);
    let url = server.url("/outro.md");

    fixture
        .command()
        .args(["compose", "-"])
        .write_stdin(format!("---\nepilogue: {url}\n---\n# Local\n\nBody.\n"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("remote read denied"))
        .stderr(predicate::str::contains("127.0.0.1"));

    assert_eq!(server.request_count(), 0);
    assert!(server.requests().is_empty());
}

#[test]
fn test_compose_remote_refresh_revalidates_cached_url() {
    let fixture = CliProcessFixture::named("test_compose_remote_refresh_revalidates_cached_url");
    let cache_dir = tempfile::TempDir::new().unwrap();
    let server = mock_http_server(vec![
        MockHttpResponse {
            status: 200,
            body: "First remote body\n",
            cache_control: Some("max-age=3600"),
        },
        MockHttpResponse {
            status: 200,
            body: "Second remote body\n",
            cache_control: Some("max-age=3600"),
        },
    ]);
    let url = server.url("/cached.md");
    let input = format!("# Local\n\n::file {url}\n");

    fixture
        .command()
        .args(["compose", "-", "--allow-host", "127.0.0.1", "--cache-root"])
        .arg(cache_dir.path())
        .write_stdin(input.clone())
        .assert()
        .success()
        .stdout(predicate::str::contains("First remote body"));

    fixture
        .command()
        .args(["compose", "-", "--allow-host", "127.0.0.1", "--cache-root"])
        .arg(cache_dir.path())
        .args(["--remote-refresh"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("Second remote body"));

    assert_eq!(server.request_count(), 2);
    assert_get_requests(&server, &["/cached.md", "/cached.md"]);
}

#[test]
fn test_compose_remote_fallback_serves_stale_cache_on_failure() {
    let fixture =
        CliProcessFixture::named("test_compose_remote_fallback_serves_stale_cache_on_failure");
    let cache_dir = tempfile::TempDir::new().unwrap();
    let server = mock_http_server(vec![
        MockHttpResponse {
            status: 200,
            body: "Cached remote body\n",
            cache_control: Some("max-age=0"),
        },
        MockHttpResponse {
            status: 500,
            body: "server unavailable\n",
            cache_control: None,
        },
    ]);
    let url = server.url("/stale.md");
    let input = format!("# Local\n\n::file {url}\n");

    fixture
        .command()
        .args(["compose", "-", "--allow-host", "127.0.0.1", "--cache-root"])
        .arg(cache_dir.path())
        .write_stdin(input.clone())
        .assert()
        .success()
        .stdout(predicate::str::contains("Cached remote body"));

    fixture
        .command()
        .args(["compose", "-", "--allow-host", "127.0.0.1", "--cache-root"])
        .arg(cache_dir.path())
        .args(["--remote-freshness", "fallback"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("Cached remote body"));

    assert_eq!(server.request_count(), 2);
    assert_get_requests(&server, &["/stale.md", "/stale.md"]);
}

#[test]
fn test_compose_remote_ttl_serves_cached_url_without_second_request() {
    let fixture = CliProcessFixture::named(
        "test_compose_remote_ttl_serves_cached_url_without_second_request",
    );
    let cache_dir = tempfile::TempDir::new().unwrap();
    let server = mock_http_server(vec![MockHttpResponse {
        status: 200,
        body: "TTL remote body\n",
        cache_control: Some("max-age=0"),
    }]);
    let url = server.url("/ttl.md");
    let input = format!("# Local\n\n::file {url}\n");

    for _ in 0..2 {
        fixture
            .command()
            .args(["compose", "-", "--allow-host", "127.0.0.1", "--cache-root"])
            .arg(cache_dir.path())
            .args(["--remote-ttl", "300"])
            .write_stdin(input.clone())
            .assert()
            .success()
            .stdout(predicate::str::contains("TTL remote body"));
    }

    assert_eq!(server.request_count(), 1);
    assert_get_requests(&server, &["/ttl.md"]);
}

/// Acceptance criterion 4 through the normal `md compose` path: `--remote-ttl`
/// with the most permissive freshness mode neither stores a `no-store`
/// response nor serves a `no-cache` one without going back to the origin.
/// `max-age=3600, no-store` is the original defect: first-match parsing read
/// it as a one-hour lifetime.
#[test]
fn test_compose_remote_ttl_does_not_override_no_store_or_no_cache() {
    let fixture =
        CliProcessFixture::named("test_compose_remote_ttl_does_not_override_no_store_or_no_cache");
    let cases = [
        ("max-age=3600, no-store", false),
        ("no-cache", true),
    ];
    for (cache_control, storable) in cases {
        let cache_dir = tempfile::TempDir::new().unwrap();
        let server = mock_http_server(vec![
            MockHttpResponse {
                status: 200,
                body: "First remote body\n",
                cache_control: Some(cache_control),
            },
            MockHttpResponse {
                status: 200,
                body: "Second remote body\n",
                cache_control: Some(cache_control),
            },
        ]);
        let url = server.url("/directive.md");
        let input = format!("# Local\n\n::file {url}\n");

        for expected in ["First remote body", "Second remote body"] {
            fixture
                .command()
                .args(["compose", "-", "--allow-host", "127.0.0.1", "--cache-root"])
                .arg(cache_dir.path())
                .args(["--remote-ttl", "3600", "--remote-freshness", "optimistic"])
                .write_stdin(input.clone())
                .assert()
                .success()
                .stdout(predicate::str::contains(expected))
                .stderr(predicate::str::contains("remote cache").not());
        }

        assert_eq!(server.request_count(), 2, "{cache_control}");
        assert_get_requests(&server, &["/directive.md", "/directive.md"]);
        let remote_manifests = cache_dir.path().join(".darkmatter/cache/v1/manifests/remote");
        assert_eq!(
            remote_manifests.exists(),
            storable,
            "{cache_control}: {:?}",
            snapshot_tree(cache_dir.path())
        );
        let stored = snapshot_tree(cache_dir.path());
        if !storable {
            assert!(
                stored.iter().all(|(_, is_dir, _)| *is_dir),
                "{cache_control}: a no-store artifact was written: {stored:?}"
            );
        }
    }
}

/// Acceptance criterion 6 through the normal `md compose` path: a remote URL
/// carrying credentials and a query token is fetched with both (the request
/// line keeps the query), cached, and served warm, yet no byte under the cache
/// root holds them. The manifest keeps a diagnostic form naming host and path.
#[test]
fn test_compose_cache_manifest_never_persists_url_userinfo_or_query() {
    let fixture = CliProcessFixture::named(
        "test_compose_cache_manifest_never_persists_url_userinfo_or_query",
    );
    let cache_dir = tempfile::TempDir::new().unwrap();
    let server = mock_http_server(vec![MockHttpResponse {
        status: 200,
        body: "Private remote body\n",
        cache_control: Some("max-age=3600"),
    }]);
    let plain_url = server.url("/doc.md");
    let url = server
        .url("/doc.md?token=abc")
        .replacen("http://", "http://user:secret@", 1);
    let input = format!("# Local\n\n::file {url}\n");

    for _ in 0..2 {
        fixture
            .command()
            .args(["compose", "-", "--allow-host", "127.0.0.1", "--cache-root"])
            .arg(cache_dir.path())
            .write_stdin(input.clone())
            .assert()
            .success()
            .stdout(predicate::str::contains("Private remote body"));
    }

    assert_eq!(server.request_count(), 1, "the warm run must be served from cache");
    assert_get_requests(&server, &["/doc.md?token=abc"]);

    let stored = snapshot_tree(cache_dir.path());
    assert!(stored.iter().any(|(_, is_dir, _)| !is_dir), "nothing was cached: {stored:?}");
    for (path, _, bytes) in &stored {
        let text = String::from_utf8_lossy(bytes);
        for secret in ["user", "secret", "token", "abc"] {
            assert!(!text.contains(secret), "{} holds {secret:?}", path.display());
        }
    }

    let remote_manifests = cache_dir
        .path()
        .join(".darkmatter")
        .join("cache")
        .join("v1")
        .join("manifests")
        .join("remote");
    let manifests: Vec<serde_json::Value> = snapshot_tree(&remote_manifests)
        .into_iter()
        .filter(|(_, is_dir, _)| !is_dir)
        .map(|(_, _, bytes)| serde_json::from_slice(&bytes).unwrap())
        .collect();
    assert_eq!(manifests.len(), 1, "{manifests:?}");
    assert_eq!(manifests[0]["redacted_url"], format!("{plain_url}?<redacted>"));
}

/// An unusable `--cache-root` (a regular file) does not fail the compose: the
/// remote body is served from the network and the failed write surfaces as a
/// warning on stderr instead of being swallowed.
#[test]
fn test_compose_unusable_cache_root_warns_and_still_composes() {
    let fixture = CliProcessFixture::named("test_compose_unusable_cache_root_warns_and_still_composes");
    let parent = tempfile::TempDir::new().unwrap();
    let cache_root = parent.path().join("not-a-directory");
    std::fs::write(&cache_root, "plain\n").unwrap();
    let server = mock_http_server(vec![MockHttpResponse {
        status: 200,
        body: "Remote body\n",
        cache_control: Some("max-age=3600"),
    }]);
    let url = server.url("/remote.md");

    fixture
        .command()
        .args(["compose", "-", "--allow-host", "127.0.0.1", "--cache-root"])
        .arg(&cache_root)
        .write_stdin(format!("# Local\n\n::file {url}\n"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Remote body"))
        .stderr(predicate::str::contains("failed to write remote cache entry"));

    assert_eq!(server.request_count(), 1);
    assert_eq!(std::fs::read_to_string(&cache_root).unwrap(), "plain\n");
}

/// R18 and AC36: `--cache-root` persists raw remote bodies only. A warm run
/// still serves the remote body from the cache, but draws a new execution
/// identity and recomposes the local `::file` child, the `::code` operation,
/// and the `current_env` probe instead of replaying the cold run's output,
/// and no composed, operation, or snapshot manifest is ever written.
#[test]
fn test_compose_cache_root_never_replays_composed_local_output() {
    let fixture =
        CliProcessFixture::named("test_compose_cache_root_never_replays_composed_local_output");
    let cache_dir = tempfile::TempDir::new().unwrap();
    let server = mock_http_server(vec![MockHttpResponse {
        status: 200,
        body: "Remote body\n",
        cache_control: Some("max-age=0"),
    }]);
    let url = server.url("/remote.md");
    let root = fixture.write_file(
        "cwd/root.md",
        &format!(
            "# Root\n\nid=[{{{{ ctx.id }}}}] sid=[{{{{ ctx.sid }}}}] env=[{{{{ current_env.AC36_PROBE }}}}]\n\n\
             ::file ./child.md\n\n::code ./main.rs\n\n::file {url}\n"
        ),
    );
    fixture.write_file("cwd/child.md", "stamp=[{{ ctx.timestamp_ms }}]\n");
    fixture.write_file("cwd/main.rs", "fn main() {}\n");

    let run = |env_probe: &str| {
        let output = fixture
            .command()
            .env("AC36_PROBE", env_probe)
            .arg("compose")
            .arg(&root)
            .args(["--allow-host", "127.0.0.1", "--remote-ttl", "300", "--cache-root"])
            .arg(cache_dir.path())
            .output()
            .expect("md compose runs");
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap()
    };
    let probe = |stdout: &str, name: &str| {
        let start = stdout
            .find(&format!("{name}=["))
            .unwrap_or_else(|| panic!("no `{name}` probe in {stdout}"))
            + name.len()
            + 2;
        stdout[start..start + stdout[start..].find(']').unwrap()].to_string()
    };

    let cold = run("cold");
    std::thread::sleep(Duration::from_millis(5));
    // Probes whose results change between executions.
    fixture.write_file("cwd/main.rs", "fn main() { warm() }\n");
    let warm = run("warm");

    for stdout in [&cold, &warm] {
        assert!(stdout.contains("Remote body"), "{stdout}");
    }
    assert!(cold.contains("fn main() {}"), "{cold}");
    assert!(
        warm.contains("fn main() { warm() }") && !warm.contains("fn main() {}"),
        "the warm run replayed the cold run's ::code result: {warm}"
    );
    assert_eq!(probe(&cold, "env"), "cold", "{cold}");
    assert_eq!(probe(&warm, "env"), "warm", "the warm run replayed the cold run's probe: {warm}");
    assert_ne!(probe(&cold, "stamp"), probe(&warm, "stamp"), "the warm run replayed composed output");
    for key in ["id", "sid"] {
        assert!(!probe(&cold, key).is_empty(), "ctx.{key} rendered: {cold}");
        assert_ne!(probe(&cold, key), probe(&warm, key), "the warm run replayed the cold run's ctx.{key}");
    }
    assert_ne!(probe(&warm, "id"), probe(&warm, "sid"), "{warm}");
    assert_eq!(server.request_count(), 1, "the remote body is still cached");

    let manifests = cache_dir.path().join(".darkmatter/cache/v1/manifests");
    assert!(manifests.join("remote").is_dir(), "remote bodies persist under --cache-root");
    for class in ["composed", "operation", "snapshot"] {
        assert!(!manifests.join(class).exists(), "a local {class} artifact was persisted");
    }
}

/// Acceptance criterion 3 (R-Q1: keep) through the plain
/// `md compose FILE --cache-root DIR --allow-host 127.0.0.1` invocation, with
/// no TTL or freshness override. A fresh remote body is reused from the
/// transport cache while the local `::file` child, whose source and runtime
/// value both change between runs, is recomposed. Only the raw remote bytes
/// reach disk.
#[test]
fn test_compose_mixed_document_reuses_remote_bytes_and_recomposes_local_child() {
    let fixture = CliProcessFixture::named(
        "test_compose_mixed_document_reuses_remote_bytes_and_recomposes_local_child",
    );
    let cache_dir = tempfile::TempDir::new().unwrap();
    // The second response exists so that a warm-run network fetch would be
    // both counted and visible in the output.
    let server = mock_http_server(vec![
        MockHttpResponse {
            status: 200,
            body: "Remote body v1\n",
            cache_control: Some("max-age=3600"),
        },
        MockHttpResponse {
            status: 200,
            body: "Remote body v2\n",
            cache_control: Some("max-age=3600"),
        },
    ]);
    let url = server.url("/remote.md");
    let root = fixture.write_file("cwd/root.md", &format!("# Root\n\n::file ./child.md\n\n::file {url}\n"));

    let run = |child: &str| {
        fixture.write_file(
            "cwd/child.md",
            &format!("child=[{child}] stamp=[{{{{ ctx.timestamp_ms }}}}]\n"),
        );
        let output = fixture
            .command()
            .arg("compose")
            .arg(&root)
            .arg("--cache-root")
            .arg(cache_dir.path())
            .args(["--allow-host", "127.0.0.1"])
            .output()
            .expect("md compose runs");
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8(output.stdout).unwrap()
    };
    let stamp = |stdout: &str| {
        let start = stdout.find("stamp=[").unwrap_or_else(|| panic!("no stamp in {stdout}")) + 7;
        stdout[start..start + stdout[start..].find(']').unwrap()].to_string()
    };

    let cold = run("cold");
    std::thread::sleep(Duration::from_millis(5));
    let warm = run("warm");

    for stdout in [&cold, &warm] {
        assert!(stdout.contains("Remote body v1") && !stdout.contains("Remote body v2"), "{stdout}");
    }
    assert_eq!(server.request_count(), 1, "the warm run must reuse the transport cache");
    assert_get_requests(&server, &["/remote.md"]);
    assert!(cold.contains("child=[cold]"), "{cold}");
    assert!(
        warm.contains("child=[warm]") && !warm.contains("child=[cold]"),
        "the warm run replayed the cold run's local child: {warm}"
    );
    assert_ne!(stamp(&cold), stamp(&warm), "the warm run replayed a runtime value");

    let versioned = Path::new(".darkmatter").join("cache").join("v1");
    let files: Vec<_> = snapshot_tree(cache_dir.path())
        .into_iter()
        .filter(|(_, is_dir, _)| !is_dir)
        .collect();
    assert_eq!(files.len(), 2, "one remote manifest and one blob: {files:?}");
    let manifest = files
        .iter()
        .find(|(path, _, _)| path.starts_with(versioned.join("manifests").join("remote")))
        .unwrap_or_else(|| panic!("no remote manifest: {files:?}"));
    let blob = files
        .iter()
        .find(|(path, _, _)| path.starts_with(versioned.join("blobs")))
        .unwrap_or_else(|| panic!("no remote blob: {files:?}"));
    assert_eq!(blob.2, b"Remote body v1\n", "the blob holds the raw remote bytes");
    for (path, _, bytes) in [manifest, blob] {
        let text = String::from_utf8_lossy(bytes);
        assert!(
            !text.contains("child=") && !text.contains("# Root"),
            "{} persisted local content",
            path.display()
        );
    }
}

/// Acceptance criterion 5 through the normal `md compose` path: a denied host
/// fails before the transport cache is read. A real allowed run seeds a fresh
/// entry, and a second allowed run proves it is served without a request. The
/// same URL composed without `--allow-host` then fails under every freshness
/// mode, including `fallback` (which serves stale bytes after a failure) and a
/// TTL override. The seeded body never appears, no request reaches the
/// origin, and the seeded tree is left byte-identical.
#[test]
fn test_compose_denied_host_never_reads_a_seeded_cache_entry() {
    let fixture =
        CliProcessFixture::named("test_compose_denied_host_never_reads_a_seeded_cache_entry");
    let cache_dir = tempfile::TempDir::new().unwrap();
    // The second response exists so that any request after seeding is counted.
    let server = mock_http_server(vec![
        MockHttpResponse {
            status: 200,
            body: "Seeded remote body\n",
            cache_control: Some("max-age=3600"),
        },
        MockHttpResponse {
            status: 200,
            body: "Network remote body\n",
            cache_control: Some("max-age=3600"),
        },
    ]);
    let url = server.url("/seeded.md");
    let transclusion = format!("# Local\n\n::file {url}\n");

    for _ in 0..2 {
        fixture
            .command()
            .args(["compose", "-", "--allow-host", "127.0.0.1", "--cache-root"])
            .arg(cache_dir.path())
            .write_stdin(transclusion.clone())
            .assert()
            .success()
            .stdout(predicate::str::contains("Seeded remote body"));
    }
    assert_eq!(server.request_count(), 1, "the seeded entry is served warm to an allowed host");
    let seeded = snapshot_tree(cache_dir.path());
    assert!(seeded.iter().any(|(_, is_dir, _)| !is_dir), "nothing was seeded: {seeded:?}");

    let flag_sets: [&[&str]; 4] = [
        &[],
        &["--remote-freshness", "strict"],
        &["--remote-freshness", "fallback"],
        &["--remote-freshness", "optimistic", "--remote-ttl", "3600"],
    ];
    for flags in flag_sets {
        fixture
            .command()
            .args(["compose", "-", "--cache-root"])
            .arg(cache_dir.path())
            .args(flags)
            .write_stdin(transclusion.clone())
            .assert()
            .failure()
            .stdout(predicate::str::contains("remote body").not())
            .stderr(predicate::str::contains("remote read denied"))
            .stderr(predicate::str::contains("127.0.0.1"));

        fixture
            .command()
            .args(["compose", "-", "--cache-root"])
            .arg(cache_dir.path())
            .args(flags)
            .write_stdin(format!("Exists: {{{{ file_exists(\"{url}\") }}}}\n"))
            .assert()
            .success()
            .stdout(predicate::str::contains("Exists: false"));

        assert_eq!(server.request_count(), 1, "{flags:?}: a denied host reached the network");
        assert_eq!(snapshot_tree(cache_dir.path()), seeded, "{flags:?}: the seeded cache changed");
    }
    assert_get_requests(&server, &["/seeded.md"]);
}

/// Acceptance criterion 5, companion case: `--cache-root` alone authorizes no
/// host. The compose fails by policy without a request, a missing root is not
/// created, and an existing one is left byte-identical.
#[test]
fn test_compose_cache_root_alone_never_authorizes_a_host() {
    let fixture = CliProcessFixture::named("test_compose_cache_root_alone_never_authorizes_a_host");
    let server = mock_http_server(vec![MockHttpResponse {
        status: 200,
        body: "should not be fetched\n",
        cache_control: Some("max-age=3600"),
    }]);
    let url = server.url("/blocked.md");
    let parent = tempfile::TempDir::new().unwrap();
    let missing = parent.path().join("never-created");
    let existing = tempfile::TempDir::new().unwrap();
    std::fs::write(existing.path().join("sentinel.txt"), "keep me\n").unwrap();
    let before = snapshot_tree(existing.path());

    for cache_root in [missing.as_path(), existing.path()] {
        fixture
            .command()
            .args(["compose", "-", "--cache-root"])
            .arg(cache_root)
            .write_stdin(format!("# Local\n\n::file {url}\n"))
            .assert()
            .failure()
            .stdout(predicate::str::contains("should not be fetched").not())
            .stderr(predicate::str::contains("remote read denied"))
            .stderr(predicate::str::contains("127.0.0.1"));
    }

    assert_eq!(server.request_count(), 0);
    assert!(server.requests().is_empty());
    let metadata = std::fs::symlink_metadata(&missing);
    assert!(
        metadata.as_ref().is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
        "--cache-root was created: {metadata:?}"
    );
    assert_eq!(snapshot_tree(parent.path()), Vec::new());
    assert_eq!(snapshot_tree(existing.path()), before);
}

/// Relative path, directory flag, and bytes for every entry under `root`,
/// sorted. Modification times are excluded: directory mtimes vary by
/// filesystem (notably Windows and WSL2 `drvfs`).
fn snapshot_tree(root: &Path) -> Vec<(PathBuf, bool, Vec<u8>)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, bool, Vec<u8>)>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            if path.is_dir() {
                out.push((relative, true, Vec::new()));
                walk(root, &path, out);
            } else {
                out.push((relative, false, std::fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

/// Acceptance criterion 1 through the normal `md compose` path: a local-only
/// compose never creates a missing `--cache-root` and leaves an existing one
/// byte-identical, including when the transport-cache flags are also set.
#[test]
fn test_compose_local_only_cache_root_is_never_created_or_modified() {
    let fixture =
        CliProcessFixture::named("test_compose_local_only_cache_root_is_never_created_or_modified");
    let root = fixture.write_file("cwd/root.md", "# Root\n\n::file ./child.md\n\n::code ./main.rs\n");
    fixture.write_file("cwd/child.md", "Local child\n");
    fixture.write_file("cwd/main.rs", "fn main() {}\n");
    let flag_sets: [&[&str]; 2] = [
        &[],
        &["--allow-host", "127.0.0.1", "--remote-ttl", "300", "--remote-freshness", "strict"],
    ];
    let compose = |cache_root: &Path, flags: &[&str]| {
        fixture
            .command()
            .arg("compose")
            .arg(&root)
            .args(flags)
            .arg("--cache-root")
            .arg(cache_root)
            .assert()
            .success()
            .stdout(predicate::str::contains("Local child").and(predicate::str::contains("fn main() {}")));
    };

    for flags in flag_sets {
        let parent = tempfile::TempDir::new().unwrap();
        let missing = parent.path().join("never-created");
        compose(&missing, flags);
        let metadata = std::fs::symlink_metadata(&missing);
        assert!(
            metadata.as_ref().is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
            "{flags:?}: --cache-root was created: {metadata:?}"
        );
        assert_eq!(snapshot_tree(parent.path()), Vec::new(), "{flags:?}");

        for seeded in [false, true] {
            let existing = tempfile::TempDir::new().unwrap();
            if seeded {
                std::fs::create_dir_all(existing.path().join("nested/deeper")).unwrap();
                std::fs::write(existing.path().join("sentinel.txt"), "keep me\n").unwrap();
                std::fs::write(existing.path().join("nested/deeper/note.md"), "# Note\n").unwrap();
            }
            let before = snapshot_tree(existing.path());
            compose(existing.path(), flags);
            assert_eq!(
                snapshot_tree(existing.path()),
                before,
                "{flags:?}, seeded={seeded}: --cache-root changed"
            );
        }
    }
}

#[test]
fn test_compose_invalid_remote_freshness_fails_fast() {
    let fixture = CliProcessFixture::named("test_compose_invalid_remote_freshness_fails_fast");
    // A typo must fail with a non-zero exit and list the accepted values,
    // rather than silently degrading to a single freshness mode.
    fixture
        .command()
        .args(["compose", "-", "--remote-freshness", "fallbak"])
        .write_stdin("# Local\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("optimistic"))
        .stderr(predicate::str::contains("strict"))
        .stderr(predicate::str::contains("fallback"));
}

#[test]
fn test_compose_preserves_rendered_remote_links() {
    let fixture = CliProcessFixture::named("test_compose_preserves_rendered_remote_links");
    fixture
        .command()
        .args(["compose", "-"])
        .write_stdin("[Remote](https://example.com/path?q=1)\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "[Remote](https://example.com/path?q=1)",
        ));
}

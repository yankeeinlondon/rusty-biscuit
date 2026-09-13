mod common;

use common::{
    CliProcessFixture, MOCK_HTTP_SHUTDOWN_BOUND, MockHttpResponse, MockHttpServer, mock_http_server,
};
use predicates::prelude::*;
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

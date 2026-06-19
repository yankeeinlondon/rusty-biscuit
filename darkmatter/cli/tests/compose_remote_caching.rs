mod common;

use common::{md_cmd, mock_http_server, MockHttpResponse};
use predicates::prelude::*;

#[test]
fn test_compose_remote_allowed_host_fetches_url() {
    let server = mock_http_server(vec![MockHttpResponse {
        status: 200,
        body: "Remote body\n",
        cache_control: None,
    }]);
    let url = server.url("/remote.md");

    md_cmd()
        .args(["compose", "-", "--allow-host", "127.0.0.1"])
        .write_stdin(format!("# Local\n\n::file {url}\n"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Remote body"));

    assert_eq!(server.request_count(), 1);
}

#[test]
fn test_compose_remote_deny_all_fails_without_request() {
    let server = mock_http_server(vec![MockHttpResponse {
        status: 200,
        body: "should not be fetched\n",
        cache_control: None,
    }]);
    let url = server.url("/blocked.md");

    md_cmd()
        .args(["compose", "-"])
        .write_stdin(format!("# Local\n\n::file {url}\n"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("remote read denied"))
        .stderr(predicate::str::contains("127.0.0.1"));

    assert_eq!(server.request_count(), 0);
}

#[test]
fn test_compose_remote_expression_function_reads_url() {
    // Read-side expression functions must work through the real `md compose`
    // pipeline, not just helper-level unit tests. The URL argument is quoted
    // because the interpolation expression parser requires a string literal.
    let server = mock_http_server(vec![MockHttpResponse {
        status: 200,
        body: "# Remote Heading\n",
        cache_control: None,
    }]);
    let url = server.url("/remote.md");

    md_cmd()
        .args(["compose", "-", "--allow-host", "127.0.0.1"])
        .write_stdin(format!("Title: {{{{ markdown_title(\"{url}\") }}}}\n"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Title: Remote Heading"));

    assert_eq!(server.request_count(), 1);
}

#[test]
fn test_compose_remote_expression_function_denied_host_reads_false() {
    // `file_exists` against a host that is not allowed must read as `false`
    // (the fetch is policy-denied, never issued) rather than failing compose.
    let server = mock_http_server(vec![MockHttpResponse {
        status: 200,
        body: "should not be fetched\n",
        cache_control: None,
    }]);
    let url = server.url("/blocked.md");

    md_cmd()
        .args(["compose", "-"])
        .write_stdin(format!("Exists: {{{{ file_exists(\"{url}\") }}}}\n"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Exists: false"));

    assert_eq!(server.request_count(), 0);
}

#[test]
fn test_compose_remote_prologue_allowed_host_fetches_url() {
    // A remote `prologue` URL on an allowed host must be registered, fetched,
    // and prepended to the body.
    let server = mock_http_server(vec![MockHttpResponse {
        status: 200,
        body: "Prologue body\n",
        cache_control: None,
    }]);
    let url = server.url("/intro.md");

    md_cmd()
        .args(["compose", "-", "--allow-host", "127.0.0.1"])
        .write_stdin(format!("---\nprologue: {url}\n---\n# Local\n\nBody.\n"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Prologue body"))
        .stdout(predicate::str::contains("Local"));

    assert_eq!(server.request_count(), 1);
}

#[test]
fn test_compose_remote_epilogue_deny_all_fails_without_request() {
    // A remote `epilogue` on a non-allowed host must fail by policy and never
    // issue a request — not fail with an internal "not registered" error.
    let server = mock_http_server(vec![MockHttpResponse {
        status: 200,
        body: "should not be fetched\n",
        cache_control: None,
    }]);
    let url = server.url("/outro.md");

    md_cmd()
        .args(["compose", "-"])
        .write_stdin(format!("---\nepilogue: {url}\n---\n# Local\n\nBody.\n"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("remote read denied"))
        .stderr(predicate::str::contains("127.0.0.1"));

    assert_eq!(server.request_count(), 0);
}

#[test]
fn test_compose_remote_refresh_revalidates_cached_url() {
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

    md_cmd()
        .args([
            "compose",
            "-",
            "--allow-host",
            "127.0.0.1",
            "--cache-root",
        ])
        .arg(cache_dir.path())
        .write_stdin(input.clone())
        .assert()
        .success()
        .stdout(predicate::str::contains("First remote body"));

    md_cmd()
        .args([
            "compose",
            "-",
            "--allow-host",
            "127.0.0.1",
            "--cache-root",
        ])
        .arg(cache_dir.path())
        .args(["--remote-refresh"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("Second remote body"));

    assert_eq!(server.request_count(), 2);
}

#[test]
fn test_compose_remote_fallback_serves_stale_cache_on_failure() {
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

    md_cmd()
        .args([
            "compose",
            "-",
            "--allow-host",
            "127.0.0.1",
            "--cache-root",
        ])
        .arg(cache_dir.path())
        .write_stdin(input.clone())
        .assert()
        .success()
        .stdout(predicate::str::contains("Cached remote body"));

    md_cmd()
        .args([
            "compose",
            "-",
            "--allow-host",
            "127.0.0.1",
            "--cache-root",
        ])
        .arg(cache_dir.path())
        .args(["--remote-freshness", "fallback"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("Cached remote body"));

    assert_eq!(server.request_count(), 2);
}

#[test]
fn test_compose_invalid_remote_freshness_fails_fast() {
    // A typo must fail with a non-zero exit and list the accepted values,
    // rather than silently degrading to a single freshness mode.
    md_cmd()
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
    md_cmd()
        .args(["compose", "-"])
        .write_stdin("[Remote](https://example.com/path?q=1)\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "[Remote](https://example.com/path?q=1)",
        ));
}


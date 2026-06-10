// The resolved-layout-precedence tests exercise `DarkmatterPage`'s
// `page_margin()`, `page_padding()`, and component-policy helpers.
// These replaced the deprecated `margin()`, `padding()`, `fill_for()`,
// and `alignment_for()` getters after the migration to
// `renderable::layout::Layout`.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::io::Write;
use std::net::TcpListener;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::thread;

/// Helper to create a `md` command from cargo bin.
fn md_cmd() -> assert_cmd::Command {
    cargo_bin_cmd!("md")
}

/// Creates a temporary markdown file with the given content.
fn md_file(content: &str) -> tempfile::NamedTempFile {
    use std::io::Write;
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "{}", content).unwrap();
    tmp
}

struct MockHttpResponse {
    status: u16,
    body: &'static str,
    cache_control: Option<&'static str>,
}

struct MockHttpServer {
    base_url: String,
    requests: Arc<AtomicUsize>,
}

impl MockHttpServer {
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn request_count(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }
}

fn mock_http_server(responses: Vec<MockHttpResponse>) -> MockHttpServer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let request_count = Arc::clone(&requests);

    thread::spawn(move || {
        for response in responses {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            request_count.fetch_add(1, Ordering::SeqCst);

            let mut buf = [0_u8; 4096];
            let _ = std::io::Read::read(&mut stream, &mut buf);

            let status_text = match response.status {
                200 => "OK",
                304 => "Not Modified",
                500 => "Internal Server Error",
                _ => "OK",
            };
            let mut headers = format!(
                "HTTP/1.1 {} {}\r\nContent-Length: {}\r\n",
                response.status,
                status_text,
                response.body.len()
            );
            if let Some(cache_control) = response.cache_control {
                headers.push_str(&format!("Cache-Control: {cache_control}\r\n"));
            }
            headers.push_str("\r\n");
            let _ = stream.write_all(headers.as_bytes());
            let _ = stream.write_all(response.body.as_bytes());
        }
    });

    MockHttpServer {
        base_url: format!("http://{addr}"),
        requests,
    }
}

// =============================================================================
//                          BASIC FUNCTIONALITY TESTS
// =============================================================================

#[test]
fn test_help_flag() {
    md_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("markdown"))
        .stdout(predicate::str::contains("render"))
        .stdout(predicate::str::contains("clean"))
        .stdout(predicate::str::contains("compose"))
        .stdout(predicate::str::contains("toc"))
        .stdout(predicate::str::contains("delta"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("set"))
        .stdout(predicate::str::contains("hash"))
        .stdout(predicate::str::contains("graph"));
}

#[test]
fn test_version_flag() {
    md_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("md"));
}

#[test]
fn test_stdin_rendering_auto_non_tty_outputs_markdown() {
    md_cmd()
        .arg("-")
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_file_rendering() {
    let tmp = md_file("# Test File\n\nSome content here.\n");

    md_cmd()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("# Test File"));
}

#[test]
fn test_file_not_found() {
    md_cmd()
        .arg("/tmp/nonexistent-darkmatter-test-file.md")
        .assert()
        .failure();
}

#[test]
fn test_output_markdown_alias_text() {
    md_cmd()
        .args(["--output", "text", "-"])
        .write_stdin("# Alias Test")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Alias Test"));
}

#[test]
fn test_output_html() {
    md_cmd()
        .args(["--output", "html", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("<style>"))
        .stdout(predicate::str::contains("<h1 id=\"hello\">Hello</h1>"));
}

#[test]
fn test_output_json_alias_ast() {
    md_cmd()
        .args(["--output", "ast", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"type\""));
}

#[test]
fn test_show_option_with_markdown_output() {
    md_cmd()
        .env("MD_DRY_RUN", "1")
        .args(["--output", "markdown", "--show", "-"])
        .write_stdin("# Show Test")
        .assert()
        .success();
}

#[test]
fn test_removed_flags_are_rejected() {
    for flag in [
        "--html",
        "--show-html",
        "--ast",
        "--json",
        "--no-images",
        "--toc",
        "--delta",
        "--clean",
        "--clean-save",
        "--fm-merge-with",
        "--fm-defaults",
    ] {
        md_cmd()
            .args([flag, "-"])
            .write_stdin("# Test")
            .assert()
            .failure()
            .stderr(predicate::str::contains("unexpected argument"));
    }
}

#[test]
fn test_subcommand_rejects_render_options() {
    md_cmd()
        .args(["--output", "html", "toc", "-"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("subcommands cannot be combined"));
}

// =============================================================================
//                          SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_toc_subcommand_output() {
    md_cmd()
        .args(["toc", "-"])
        .write_stdin("# Top\n\n## Section A\n\n## Section B\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Top"))
        .stdout(predicate::str::contains("Section A"));
}

#[test]
fn test_toc_subcommand_json_output() {
    md_cmd()
        .args(["toc", "--json", "-"])
        .write_stdin("# Top\n\n## Section A\n\n## Section B\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"structure\""));
}

#[test]
fn test_toc_subcommand_ignores_tab_indented_frontmatter() {
    let input = "---\nprompt: |-\n\tLine one\n\tLine two\nlast_updated: 2026-02-27\n---\n# macOS Audio\n\n## Details\n";

    md_cmd()
        .args(["toc", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("macOS Audio"))
        .stdout(predicate::str::contains("Details"))
        .stdout(predicate::str::contains("last_updated").not());
}

#[test]
fn test_delta_subcommand_output() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.md");
    let updated = dir.path().join("updated.md");

    std::fs::write(&base, "# Title\n\nHello\n").unwrap();
    std::fs::write(&updated, "# Title\n\nHello there\n").unwrap();

    md_cmd()
        .arg("delta")
        .arg(&base)
        .arg(&updated)
        .assert()
        .success()
        .stdout(predicate::str::contains("Modified"));
}

#[test]
fn test_delta_subcommand_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.md");
    let updated = dir.path().join("updated.md");

    std::fs::write(&base, "# Title\n\nHello\n").unwrap();
    std::fs::write(&updated, "# Title\n\nHello there\n").unwrap();

    md_cmd()
        .arg("delta")
        .arg(&base)
        .arg(&updated)
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"classification\""));
}

// =============================================================================
//                          CLEAN SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_clean_subcommand_stdin() {
    md_cmd()
        .args(["clean", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_clean_subcommand_file() {
    let tmp = md_file("# Hello \n\nWorld  \n");

    md_cmd()
        .arg("clean")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_clean_subcommand_indent() {
    md_cmd()
        .args(["clean", "-", "--indent", "4"])
        .write_stdin("- Parent\n  - Child\n    - Grandchild\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\n    - Child"))
        .stdout(predicate::str::contains("\n        - Grandchild"));
}

#[test]
fn test_clean_subcommand_rejects_invalid_indent() {
    md_cmd()
        .args(["clean", "-", "--indent", "3"])
        .write_stdin("- Parent\n  - Child\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("indent must be one of: 2, 4, 8"));
}

#[test]
fn test_clean_subcommand_save_in_place_reports_delta() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "# Hello \n\nWorld  \n").unwrap();

    md_cmd()
        .arg("clean")
        .arg(tmp.path())
        .arg("--save")
        .assert()
        .success()
        .stdout(predicate::str::contains("Whitespace changes only"));

    let updated = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(updated.contains("# Hello"));
    assert!(updated.contains("World"));
    assert!(!updated.contains("# Hello "));
    assert!(!updated.contains("World  "));
    assert!(updated.ends_with('\n'));
    assert!(!updated.ends_with("\n\n"));
}

#[test]
fn test_clean_subcommand_save_verbose_after_subcommand_shows_visual_diff() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "# Hello \n\nWorld  \n").unwrap();

    md_cmd()
        .args(["clean", "--save", "-v"])
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Whitespace only"))
        .stdout(predicate::str::contains("original"))
        .stdout(predicate::str::contains("updated"))
        .stdout(predicate::str::contains("Content Visual Diff:").not());
}

#[test]
fn test_save_shorthand_cleans_in_place_and_reports_delta() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "# Hello \n\nWorld  \n").unwrap();

    md_cmd()
        .arg(tmp.path())
        .arg("--save")
        .assert()
        .success()
        .stdout(predicate::str::contains("Whitespace changes only"));

    let updated = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(updated.contains("# Hello"));
    assert!(updated.contains("World"));
    assert!(!updated.contains("# Hello "));
    assert!(!updated.contains("World  "));
}

#[test]
fn test_clean_save_rejects_stdin() {
    md_cmd()
        .args(["clean", "-", "--save"])
        .write_stdin("# Hello\n\nWorld\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--save requires an input file path (stdin is not supported)",
        ));
}

// =============================================================================
//                          READ SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_render_explicit() {
    md_cmd()
        .args(["render", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_render_default_backward_compat() {
    // md file.md (no subcommand) still works
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(tmp, "# Backward\n\nCompat test.").unwrap();

    md_cmd()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("# Backward"));
}

#[test]
fn test_render_explicit_with_output() {
    md_cmd()
        .args(["render", "--output", "html", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("<h1 id=\"hello\">Hello</h1>"));
}

// =============================================================================
//                          COMPOSE SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_compose_basic() {
    md_cmd()
        .args(["compose", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello"))
        .stdout(predicate::str::contains("World"));
}

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

#[test]
fn test_compose_with_state() {
    md_cmd()
        .args(["compose", "-", "--state", r#"{"name":"Alice"}"#])
        .write_stdin("# Hello {{ name }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello Alice"));
}

#[test]
fn test_compose_output_html() {
    md_cmd()
        .args(["compose", "-", "--output", "html"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("<h1 id=\"hello\">Hello</h1>"));
}

#[test]
fn test_compose_perf_emits_report_to_stderr() {
    md_cmd()
        .args(["compose", "-", "--perf"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello"))
        .stdout(predicate::str::contains("World"))
        .stderr(predicate::str::contains("Command Setup"))
        .stderr(predicate::str::contains("Compose Pipeline"))
        .stderr(predicate::str::contains("elapsed"));
}

#[test]
fn test_compose_without_perf_no_report_on_stderr() {
    md_cmd()
        .args(["compose", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello"))
        .stderr(predicate::str::contains("Command Setup").not());
}

#[test]
fn test_compose_strips_frontmatter() {
    md_cmd()
        .args(["compose", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"))
        .stdout(predicate::str::contains("---").not());
}

/// Regression test for silent YAML parse failure when frontmatter values
/// contain shell substitutions with nested double quotes.
///
/// Before the fix, `"$(cmd "arg")"` broke the YAML parser and the
/// `impl From<String> for Markdown` fallback left the entire `---...---`
/// block inside `content()`, so default compose output leaked the raw
/// frontmatter block.
#[test]
fn test_compose_strips_frontmatter_when_values_contain_shell_substitution() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    std::fs::write(
        &md_path,
        "---\nreview: \"\"\ndir: \"$(dirname \"{{review}}\")\"\n---\nBody: {{review}}\n",
    )
    .unwrap();

    // Approve shell commands so the pipeline runs to completion.
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");
    std::fs::write(&whitelist_path, "prefix dirname\n").unwrap();

    md_cmd()
        .current_dir(temp_dir.path())
        .arg("compose")
        .arg(&md_path)
        .arg("review=docs/foo.md")
        .assert()
        .success()
        .stdout(predicate::str::contains("Body: docs/foo.md"))
        .stdout(predicate::str::contains("---").not());
}

/// Regression test for double-frontmatter output.
///
/// Before the fix, `--frontmatter` emitted the state-populated frontmatter
/// on top of a raw, unparsed frontmatter block that still lived in
/// `content()`, producing two `---...---` fences. The fix makes parsing
/// succeed so exactly one frontmatter block is emitted.
#[test]
fn test_compose_frontmatter_flag_emits_single_block_with_nested_quotes() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    std::fs::write(
        &md_path,
        "---\nreview: \"\"\ndir: \"$(dirname \"{{review}}\")\"\n---\nBody\n",
    )
    .unwrap();

    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");
    std::fs::write(&whitelist_path, "prefix dirname\n").unwrap();

    let output = md_cmd()
        .current_dir(temp_dir.path())
        .arg("compose")
        .arg("--frontmatter")
        .arg(&md_path)
        .arg("review=docs/foo.md")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();

    // Count frontmatter fence pairs: each frontmatter block has two `---`
    // lines. Before the fix we saw four (two fences). Expect exactly two.
    let fence_count = stdout.lines().filter(|line| line.trim() == "---").count();
    assert_eq!(
        fence_count, 2,
        "expected exactly one frontmatter block (two fences), got {fence_count}.\nstdout:\n{stdout}"
    );
    assert!(
        stdout.contains("review: docs/foo.md"),
        "state-populated review should appear in frontmatter.\nstdout:\n{stdout}"
    );
}

/// Regression test for silent shell-expansion skip.
///
/// Before the fix, malformed frontmatter meant `$(...)` in values was
/// never discovered for execution. This test proves shell expansion runs
/// on frontmatter values authored with nested quotes by observing the
/// expanded result in the `--frontmatter` output.
#[test]
fn test_compose_runs_frontmatter_shell_with_nested_quotes() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    std::fs::write(
        &md_path,
        "---\npath: \"docs/foo.md\"\ndir: \"$(dirname \"{{path}}\")\"\n---\nok\n",
    )
    .unwrap();

    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");
    std::fs::write(&whitelist_path, "prefix dirname\n").unwrap();

    md_cmd()
        .current_dir(temp_dir.path())
        .arg("compose")
        .arg("--frontmatter")
        .arg(&md_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("dir: docs"));
}

#[test]
fn test_compose_invalid_state() {
    md_cmd()
        .args(["compose", "-", "--state", "bad json"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid JSON"));
}

#[test]
fn test_compose_state_requires_json_object() {
    md_cmd()
        .args(["compose", "-", "--state", "[1,2,3]"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected a JSON object"));
}

#[test]
fn test_compose_with_set_overwrites_frontmatter() {
    md_cmd()
        .args([
            "compose",
            "-",
            "--set",
            r#"{"name":"Bob"}"#,
            "--frontmatter",
        ])
        .write_stdin("---\nname: Alice\n---\n# Hello {{ name }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello Bob"))
        .stdout(predicate::str::contains("name: Bob"));
}

#[test]
fn test_compose_with_set_adds_missing_keys() {
    md_cmd()
        .args(["compose", "-", "--set", r#"{"name":"Bob"}"#])
        .write_stdin("# Hello {{ name }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello Bob"));
}

#[test]
fn test_compose_set_and_state_combined() {
    // --state fills defaults, --set overwrites; --set wins on overlap
    md_cmd()
        .args([
            "compose",
            "-",
            "--state",
            r#"{"greeting":"Hi","name":"Alice"}"#,
            "--set",
            r#"{"name":"Bob"}"#,
        ])
        .write_stdin("# {{ greeting }} {{ name }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hi Bob"));
}

// =============================================================================
//              COMPOSE SHORTHAND SETTER TESTS
// =============================================================================

#[test]
fn test_compose_shorthand_basic_file_input() {
    let tmp = md_file("# Hello {{ iteration }}\n");
    md_cmd()
        .args(["compose"])
        .arg(tmp.path())
        .arg("iteration=1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello 1"));
}

#[test]
fn test_compose_shorthand_basic_stdin() {
    md_cmd()
        .args(["compose", "iteration=1"])
        .write_stdin("# Hello {{ iteration }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello 1"));
}

#[test]
fn test_compose_shorthand_multiple_setters_mixed_types() {
    md_cmd()
        .args(["compose", "iteration=1", "draft=false", "name=Alice"])
        .write_stdin("{{ iteration }} {{ draft }} {{ name }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 false Alice"));
}

#[test]
fn test_compose_shorthand_json5_value() {
    md_cmd()
        .args(["compose", r#"meta={author:"Alice"}"#])
        .write_stdin("{{ meta.author }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Alice"));
}

#[test]
fn test_compose_shorthand_participates_in_validation() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir(temp_dir.path().join("features")).unwrap();
    std::fs::write(
        temp_dir.path().join("features/my-plan.md"),
        "# My Plan\n\nPlan content here.",
    )
    .unwrap();

    let template_path = temp_dir.path().join("template.md");
    std::fs::write(&template_path, "# Task\n\n::file features/{{plan}}\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&template_path)
        .arg("plan=my-plan.md")
        .assert()
        .success()
        .stdout(predicate::str::contains("Plan content here."));
}

#[test]
fn test_compose_shorthand_wins_over_state() {
    md_cmd()
        .args([
            "compose",
            "-",
            "--state",
            r#"{"iteration":0}"#,
            "iteration=1",
        ])
        .write_stdin("{{ iteration }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("1"));
}

#[test]
fn test_compose_shorthand_wins_over_set() {
    md_cmd()
        .args(["compose", "-", "--set", r#"{"iteration":1}"#, "iteration=2"])
        .write_stdin("{{ iteration }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("2"));
}

#[test]
fn test_compose_shorthand_duplicate_keys_last_write_wins() {
    md_cmd()
        .args(["compose", "iteration=1", "iteration=2"])
        .write_stdin("{{ iteration }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("2"));
}

#[test]
fn test_compose_shorthand_empty_value() {
    md_cmd()
        .args(["compose", "empty="])
        .write_stdin("'{{ empty }}'")
        .assert()
        .success()
        .stdout(predicate::str::contains("''"));
}

#[test]
fn test_compose_shorthand_empty_key_errors() {
    md_cmd()
        .args(["compose", "=value"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid setter '=value'"));
}

#[test]
fn test_compose_shorthand_numeric_leading_key_is_treated_as_input_path() {
    md_cmd()
        .args(["compose", "9key=value"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to load"))
        .stderr(predicate::str::contains("9key=value"));
}

#[test]
fn test_compose_shorthand_setter_before_file_input() {
    let tmp = md_file("# Hello {{ iteration }}\n");
    md_cmd()
        .args(["compose", "iteration=1"])
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello 1"));
}

#[test]
fn test_compose_shorthand_multiple_non_setter_tokens_error() {
    let tmp = md_file("# Test\n");
    md_cmd()
        .args(["compose"])
        .arg(tmp.path())
        .arg("other.md")
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected at most one input path"));
}

#[test]
fn test_compose_shorthand_path_escape_hatch() {
    let tmp = md_file("# Content\n");
    let path_str = format!("./{}", tmp.path().file_name().unwrap().to_string_lossy());
    md_cmd()
        .args(["compose"])
        .arg(&path_str)
        .arg("key=val")
        .current_dir(tmp.path().parent().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Content"));
}

#[test]
fn test_compose_set_invalid_json() {
    md_cmd()
        .args(["compose", "-", "--set", "bad json"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid JSON"));
}

#[test]
fn test_compose_set_requires_json_object() {
    md_cmd()
        .args(["compose", "-", "--set", "[1,2,3]"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected a JSON object"));
}

// =============================================================================
//              SET OVERLAY TRANCLUSION CLI TESTS
// =============================================================================

#[test]
fn test_set_overlay_child_interpolation() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("parent.md"),
        r#"::file child.md set.name="Bob""#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("child.md"),
        "---\nname: Alice\n---\n\nHello {{ name }}\n",
    )
    .unwrap();

    md_cmd()
        .arg("compose")
        .arg(dir.path().join("parent.md"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello Bob"))
        .stdout(predicate::str::contains("Alice").not());
}

#[test]
fn test_set_overlay_strict_rejects_invalid() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("parent.md"), r#"::file child.md set=42"#).unwrap();
    std::fs::write(dir.path().join("child.md"), "body\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(dir.path().join("parent.md"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid frontmatter assignment"));
}

#[test]
fn test_set_overlay_permissive_invalid_warns() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("parent.md"),
        r#"::file child.md set=42 set.name="Bob""#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("child.md"),
        "---\nname: Alice\n---\n\n{{ name }}\n",
    )
    .unwrap();

    md_cmd()
        .arg("compose")
        .arg(dir.path().join("parent.md"))
        .arg("--allow-invalid-frontmatter-assignment")
        .assert()
        .success()
        .stdout(predicate::str::contains("Bob"))
        .stdout(predicate::str::contains("Alice").not());
}

#[test]
fn test_set_overlay_strict_rejects_reassigned() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("parent.md"),
        r#"::file child.md set.name="Bob" set.name="Mary""#,
    )
    .unwrap();
    std::fs::write(dir.path().join("child.md"), "---\n---\nbody\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(dir.path().join("parent.md"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("duplicate frontmatter property"));
}

#[test]
fn test_set_overlay_permissive_reassigned_warns_and_rightmost_wins() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("parent.md"),
        r#"::file child.md set.name="Bob" set.name="Mary""#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("child.md"),
        "---\nname: Alice\n---\n\n{{ name }}\n",
    )
    .unwrap();

    md_cmd()
        .arg("compose")
        .arg(dir.path().join("parent.md"))
        .arg("--allow-reassigned-frontmatter-property")
        .assert()
        .success()
        .stdout(predicate::str::contains("Mary"))
        .stdout(predicate::str::contains("Bob").not())
        .stdout(predicate::str::contains("Alice").not());
}

// =============================================================================
//                  FRONTMATTER INTERPOLATION TESTS
// =============================================================================

#[test]
fn test_compose_frontmatter_interpolation_basic() {
    md_cmd()
        .args(["compose", "-"])
        .write_stdin("---\nbase: /docs\nspec: \"{{base}}/spec.md\"\n---\nSpec: {{spec}}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Spec: /docs/spec.md"));
}

#[test]
fn test_compose_frontmatter_interpolation_nested_state() {
    md_cmd()
        .args([
            "compose",
            "-",
            "--state",
            r#"{"meta":{"base":"/root","author":"Parent"}}"#,
        ])
        .write_stdin("---\nmeta:\n  author: Local\nspec: \"{{meta.base}}/spec.md\"\n---\n{{spec}}")
        .assert()
        .success()
        .stdout(predicate::str::contains("/root/spec.md"));
}

#[test]
fn test_compose_frontmatter_interpolation_ctx_in_frontmatter_only() {
    // ctx.today referenced only in frontmatter — must still resolve
    md_cmd()
        .args(["compose", "-"])
        .write_stdin("---\nstamp: \"{{ctx.today}}\"\n---\nDate: {{stamp}}")
        .assert()
        .success()
        // The date should not be empty
        .stdout(predicate::str::contains("Date: ").and(predicate::str::contains("Date: \n").not()));
}

// =============================================================================
//                FRONTMATTER FALLBACK INTERPOLATION TESTS
// =============================================================================

#[test]
fn test_compose_frontmatter_double_pipe_fallback() {
    // || in frontmatter interpolation should work the same as | (fallback operator).
    // When the variable is empty, the fallback value should be used.
    md_cmd()
        .args(["compose", "-"])
        .write_stdin(
            "---\nplan: \"\"\nresolved: '{{plan || \"plan.md\"}}'\n---\nFile: {{resolved}}",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("File: plan.md"));
}

#[test]
fn test_compose_frontmatter_double_pipe_with_set_value() {
    // When --set provides a non-empty value, it should take precedence over the fallback
    md_cmd()
        .args(["compose", "-", "--set", r#"{"plan":"custom.md"}"#])
        .write_stdin(
            "---\nplan: \"\"\nresolved: '{{plan || \"plan.md\"}}'\n---\nFile: {{resolved}}",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("File: custom.md"));
}

#[test]
fn test_compose_frontmatter_nested_quotes_in_interpolation() {
    // Regression test: double quotes inside {{ }} expressions in YAML
    // frontmatter values (e.g., {{ plan || "plan.md" }}) should not break
    // YAML parsing. The frontmatter parser protects expressions before parsing.
    md_cmd()
        .args([
            "compose",
            "-",
            "--set",
            r#"{"topic":"refactor","phase":1}"#,
            "--frontmatter",
        ])
        .write_stdin(
            "---\ntopic: \"\"\nplan: \"\"\nresolved: \"prefix/{{topic}}/{{plan || \"plan.md\"}}\"\n---\nBody: {{topic}}",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("resolved: prefix/refactor/plan.md"))
        .stdout(predicate::str::contains("Body: refactor"))
        // Must NOT produce double frontmatter
        .stdout(predicate::str::contains("---\n---").not());
}

// =============================================================================
//              TRANSCLUSION + INTERPOLATION VALIDATION TESTS
// =============================================================================

#[test]
fn test_compose_set_variables_available_during_validation() {
    // Regression test: --set variables must be available during reference
    // validation so that interpolated transclusion paths resolve correctly.
    // Previously, validation ran before --set was parsed, causing
    // `::file features/{{plan}}` to resolve to `features/` (empty plan).
    let temp_dir = tempfile::TempDir::new().unwrap();

    // Create the target file that will be transcluded
    std::fs::create_dir(temp_dir.path().join("features")).unwrap();
    std::fs::write(
        temp_dir.path().join("features/my-plan.md"),
        "# My Plan\n\nPlan content here.",
    )
    .unwrap();

    // Create a template that uses --set variable in a ::file directive
    let template_path = temp_dir.path().join("template.md");
    std::fs::write(&template_path, "# Task\n\n::file features/{{plan}}\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&template_path)
        .args(["--set", r#"{"plan":"my-plan.md"}"#])
        .assert()
        .success()
        .stdout(predicate::str::contains("Plan content here."));
}

#[test]
fn test_compose_state_variables_available_during_validation() {
    // Same as above but using --state instead of --set
    let temp_dir = tempfile::TempDir::new().unwrap();

    std::fs::create_dir(temp_dir.path().join("docs")).unwrap();
    std::fs::write(
        temp_dir.path().join("docs/readme.md"),
        "# Readme\n\nReadme content.",
    )
    .unwrap();

    let template_path = temp_dir.path().join("template.md");
    std::fs::write(&template_path, "# Docs\n\n::file docs/{{doc}}\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&template_path)
        .args(["--state", r#"{"doc":"readme.md"}"#])
        .assert()
        .success()
        .stdout(predicate::str::contains("Readme content."));
}

// =============================================================================
//                      CTX OVERRIDE BEHAVIOR TESTS
// =============================================================================

#[test]
fn test_compose_scalar_ctx_without_allow_override_fails() {
    md_cmd()
        .args(["compose", "-"])
        .write_stdin("---\nctx: hello\n---\n# Test {{ ctx.today }}")
        .assert()
        .failure()
        .stderr(predicate::str::contains("CtxMergeError"))
        .stderr(predicate::str::contains("JSON object"));
}

#[test]
fn test_compose_scalar_ctx_with_allow_override_succeeds() {
    // --allow-ctx-override downgrades the error to a warning
    md_cmd()
        .args(["compose", "-", "--allow-ctx-override"])
        .write_stdin("---\nctx: hello\n---\n# Test")
        .assert()
        .success()
        .stderr(predicate::str::contains("Document ctx was not an object"));
}

#[test]
fn test_compose_object_ctx_collision_emits_warning() {
    // A document with an object ctx that collides with runtime keys should
    // succeed but emit a collision warning on stderr.
    md_cmd()
        .args(["compose", "-"])
        .write_stdin("---\nctx:\n  today: custom-value\n---\n# Test")
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "conflict with those provided by Darkmatter",
        ));
}

// =============================================================================
//                          HASH SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_hash_default_outputs_two_hashes() {
    // Default mode: frontmatter_hash-body_hash (each 16 hex chars)
    md_cmd()
        .args(["hash", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_body_only() {
    md_cmd()
        .args(["hash", "--body", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_frontmatter_only() {
    md_cmd()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_no_frontmatter() {
    // Document with no frontmatter should still produce a valid hash pair
    md_cmd()
        .args(["hash", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_deterministic() {
    // Same input should produce the same hash
    let result1 = md_cmd()
        .args(["hash", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .output()
        .unwrap();
    let result2 = md_cmd()
        .args(["hash", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .output()
        .unwrap();

    assert_eq!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_frontmatter_reordering() {
    // Frontmatter with different key ordering should produce the same hash
    let result1 = md_cmd()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin("---\ntitle: Hello\nauthor: Alice\n---\n# Content")
        .output()
        .unwrap();
    let result2 = md_cmd()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin("---\nauthor: Alice\ntitle: Hello\n---\n# Content")
        .output()
        .unwrap();

    assert_eq!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_body_whitespace_insensitive() {
    // Body with different whitespace should produce the same hash (non-strict)
    let result1 = md_cmd()
        .args(["hash", "--body", "-"])
        .write_stdin("# Hello\n\nWorld")
        .output()
        .unwrap();
    let result2 = md_cmd()
        .args(["hash", "--body", "-"])
        .write_stdin("# Hello\n\n\nWorld")
        .output()
        .unwrap();

    assert_eq!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_strict_whitespace_sensitive() {
    // With --strict, different whitespace should produce different hashes
    let result1 = md_cmd()
        .args(["hash", "--body", "--strict", "-"])
        .write_stdin("# Hello\n\nWorld")
        .output()
        .unwrap();
    let result2 = md_cmd()
        .args(["hash", "--body", "--strict", "-"])
        .write_stdin("# Hello\n\n\nWorld")
        .output()
        .unwrap();

    assert_ne!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_strict_frontmatter_differs_from_normalized() {
    // Strict and non-strict use different serialization strategies, so their
    // hashes should differ (strict uses serde_yaml, non-strict uses sorted canonical JSON)
    let input = "---\ntitle: Hello\nauthor: Alice\n---\n# Content";
    let strict = md_cmd()
        .args(["hash", "--frontmatter", "--strict", "-"])
        .write_stdin(input)
        .output()
        .unwrap();
    let normal = md_cmd()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin(input)
        .output()
        .unwrap();

    assert_ne!(strict.stdout, normal.stdout);
}

#[test]
fn test_hash_from_file() {
    let tmp = md_file("---\ntitle: File Test\n---\n# Hello\n\nWorld\n");

    md_cmd()
        .arg("hash")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

// =============================================================================
//                      HASH DIRECTORY MODE TESTS
// =============================================================================

/// Helper: create a temp directory with markdown files and return the dir.
fn create_hash_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.md"),
        "---\ntitle: Alpha\n---\n# Alpha\n\nFirst file.",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("b.md"),
        "---\ntitle: Beta\n---\n# Beta\n\nSecond file.",
    )
    .unwrap();
    // Nested subdirectory
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    std::fs::write(
        dir.path().join("sub/c.md"),
        "---\ntitle: Gamma\n---\n# Gamma\n\nThird file.",
    )
    .unwrap();
    // Non-markdown file (should be ignored)
    std::fs::write(dir.path().join("notes.txt"), "not markdown").unwrap();
    dir
}

#[test]
fn test_hash_directory_default() {
    let dir = create_hash_dir();

    md_cmd()
        .arg("hash")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_directory_body_only() {
    let dir = create_hash_dir();

    md_cmd()
        .args(["hash", "--body"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_directory_frontmatter_only() {
    let dir = create_hash_dir();

    md_cmd()
        .args(["hash", "--frontmatter"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_directory_deterministic() {
    let dir = create_hash_dir();

    let result1 = md_cmd().arg("hash").arg(dir.path()).output().unwrap();
    let result2 = md_cmd().arg("hash").arg(dir.path()).output().unwrap();

    assert_eq!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_directory_differs_from_single_file() {
    let dir = create_hash_dir();

    let dir_result = md_cmd().arg("hash").arg(dir.path()).output().unwrap();
    let file_result = md_cmd()
        .arg("hash")
        .arg(dir.path().join("a.md"))
        .output()
        .unwrap();

    // Directory aggregate should differ from any single file hash
    assert_ne!(dir_result.stdout, file_result.stdout);
}

#[test]
fn test_hash_directory_skips_hidden_dirs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.md"), "# Visible").unwrap();
    std::fs::create_dir(dir.path().join(".hidden")).unwrap();
    std::fs::write(dir.path().join(".hidden/secret.md"), "# Secret").unwrap();

    // Hash with only visible file
    let with_hidden = md_cmd().arg("hash").arg(dir.path()).output().unwrap();

    // Hash a dir that only has the visible file (no hidden dir)
    let dir2 = tempfile::tempdir().unwrap();
    std::fs::write(dir2.path().join("a.md"), "# Visible").unwrap();

    let without_hidden = md_cmd().arg("hash").arg(dir2.path()).output().unwrap();

    assert_eq!(with_hidden.stdout, without_hidden.stdout);
}

#[test]
fn test_hash_directory_strict() {
    let dir = create_hash_dir();

    let normal = md_cmd().arg("hash").arg(dir.path()).output().unwrap();
    let strict = md_cmd()
        .args(["hash", "--strict"])
        .arg(dir.path())
        .output()
        .unwrap();

    // Strict and normal should produce different hashes (different normalization)
    assert_ne!(normal.stdout, strict.stdout);
}

#[test]
fn test_hash_directory_ignores_non_markdown() {
    // A directory with only non-markdown files should still produce a valid hash
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("notes.txt"), "not markdown").unwrap();
    std::fs::write(dir.path().join("data.json"), "{}").unwrap();

    md_cmd()
        .arg("hash")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_directory_ignores_managed_keys() {
    // Adding the managed `hash` / `last_updated` baseline fields must not move
    // the directory aggregate — the hash never hashes itself.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.md"),
        "---\ntitle: Alpha\n---\n# Alpha\n\nFirst file.",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("b.md"),
        "---\ntitle: Beta\n---\n# Beta\n\nSecond file.",
    )
    .unwrap();

    let before = md_cmd().arg("hash").arg(dir.path()).output().unwrap();
    assert!(before.status.success());

    std::fs::write(
        dir.path().join("a.md"),
        "---\ntitle: Alpha\nhash: 1111111111111111-2222222222222222\nlast_updated: 2020-01-01\n---\n# Alpha\n\nFirst file.",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("b.md"),
        "---\ntitle: Beta\nhash: 3333333333333333-4444444444444444\nlast_updated: 2020-01-01\n---\n# Beta\n\nSecond file.",
    )
    .unwrap();

    let after = md_cmd().arg("hash").arg(dir.path()).output().unwrap();
    assert_eq!(
        before.stdout, after.stdout,
        "managed keys must not change the directory aggregate",
    );
}

#[test]
fn test_hash_directory_honors_ignore_properties() {
    // A file differing only in an ignored property must aggregate identically.
    let with_draft = tempfile::tempdir().unwrap();
    std::fs::write(
        with_draft.path().join("a.md"),
        "---\ntitle: Alpha\ndraft: true\n---\n# Alpha\n\nFirst file.",
    )
    .unwrap();
    let without_draft = tempfile::tempdir().unwrap();
    std::fs::write(
        without_draft.path().join("a.md"),
        "---\ntitle: Alpha\n---\n# Alpha\n\nFirst file.",
    )
    .unwrap();

    let a = md_cmd()
        .env("HASH_IGNORE_PROPERTIES", "draft")
        .arg("hash")
        .arg(with_draft.path())
        .output()
        .unwrap();
    let b = md_cmd()
        .env("HASH_IGNORE_PROPERTIES", "draft")
        .arg("hash")
        .arg(without_draft.path())
        .output()
        .unwrap();
    assert_eq!(
        a.stdout, b.stdout,
        "HASH_IGNORE_PROPERTIES must apply in directory mode",
    );
}

// =============================================================================
//                     HASH KIND / SAVE / DIFF TESTS
// =============================================================================

#[test]
fn test_hash_kind_structured_outputs_four_parts() {
    md_cmd()
        .args(["hash", "--kind", "structured", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(
            predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}-[0-9a-f]{16}-[0-9a-f]{16}\n$")
                .unwrap(),
        );
}

#[test]
fn test_hash_kind_structured_strict_outputs_four_parts() {
    md_cmd()
        .args(["hash", "--kind", "structured", "--strict", "-"])
        .write_stdin("---\nbeta: 1\nalpha: 2\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(
            predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}-[0-9a-f]{16}-[0-9a-f]{16}\n$")
                .unwrap(),
        );
}

#[test]
fn test_hash_kind_structured_strict_respects_key_order() {
    let reordered = |args: &[&str]| {
        let beta_first = md_cmd()
            .args(args)
            .write_stdin("---\nbeta: 1\nalpha: 2\n---\n# H\n\nBody.")
            .output()
            .unwrap()
            .stdout;
        let alpha_first = md_cmd()
            .args(args)
            .write_stdin("---\nalpha: 2\nbeta: 1\n---\n# H\n\nBody.")
            .output()
            .unwrap()
            .stdout;
        (beta_first, alpha_first)
    };

    // Strict preserves key order, so reordering keys changes the hash.
    let (strict_beta, strict_alpha) = reordered(&["hash", "--kind", "structured", "--strict", "-"]);
    assert_ne!(strict_beta, strict_alpha, "strict must not reorder keys");

    // Non-strict sorts keys, so reordering is a no-op.
    let (ns_beta, ns_alpha) = reordered(&["hash", "--kind", "structured", "-"]);
    assert_eq!(ns_beta, ns_alpha, "non-strict sorts keys");
}

#[test]
fn test_hash_diff_malformed_stored_hash_exits_one() {
    // A corrupt stored hash is an operational error (exit 1), never a content
    // difference (exit 2).
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.md");
    std::fs::write(
        &file,
        "---\ntitle: T\nhash: not-a-real-hash-but-two-parts\n---\n# H\n\nBody.\n",
    )
    .unwrap();

    md_cmd().arg("hash").arg("--diff").arg(&file).assert().code(1);
}

#[test]
fn test_hash_diff_detailed_bad_section_level_exits_one() {
    // A stored detailed hash whose section level is outside 1-6 is a malformed
    // baseline (operational error, exit 1), never a content difference (exit 2).
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.md");
    std::fs::write(
        &file,
        concat!(
            "---\n",
            "title: T\n",
            "hash:\n",
            "  kind: detailed\n",
            "  value:\n",
            "    frontmatter:\n",
            "      fm: \"0000000000000001\"\n",
            "      keys: \"0000000000000002\"\n",
            "    preamble: null\n",
            "    sections:\n",
            "      - [9, \"Bad\", \"0000000000000004\"]\n",
            "---\n",
            "# H\n\nBody.\n",
        ),
    )
    .unwrap();

    md_cmd().arg("hash").arg("--diff").arg(&file).assert().code(1);
}

#[test]
fn test_hash_kind_detailed_outputs_nested_yaml() {
    md_cmd()
        .args(["hash", "--kind", "detailed", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("frontmatter:"))
        .stdout(predicate::str::contains("sections:"));
}

#[test]
fn test_hash_kind_fm_matches_frontmatter_flag() {
    let input = "---\ntitle: Hello\n---\n# Content";
    let by_kind = md_cmd()
        .args(["hash", "--kind", "fm", "-"])
        .write_stdin(input)
        .output()
        .unwrap();
    let by_flag = md_cmd()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert_eq!(by_kind.stdout, by_flag.stdout);
}

#[test]
fn test_hash_kind_conflicts_with_body() {
    md_cmd()
        .args(["hash", "--kind", "fm", "--body", "-"])
        .write_stdin("# H")
        .assert()
        .failure();
}

#[test]
fn test_hash_save_and_diff_conflict() {
    md_cmd()
        .args(["hash", "--save", "--diff", "-"])
        .write_stdin("# H")
        .assert()
        .failure();
}

#[test]
fn test_hash_env_property_override() {
    // HASH_PROPERTY changes which frontmatter key the hash is written to.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.md");
    std::fs::write(&file, "---\ntitle: T\n---\n# H\n\nBody.\n").unwrap();

    md_cmd()
        .env("HASH_PROPERTY", "fingerprint")
        .arg("hash")
        .arg("--save")
        .arg(&file)
        .assert()
        .success();

    let written = std::fs::read_to_string(&file).unwrap();
    assert!(written.contains("fingerprint:"), "got:\n{written}");
    assert!(!written.contains("\nhash:"), "got:\n{written}");
}

#[test]
fn test_hash_ignore_properties_excludes_key() {
    // A document differing only in an ignored property hashes identically.
    let with_draft = md_cmd()
        .env("HASH_IGNORE_PROPERTIES", "draft")
        .args(["hash", "--frontmatter", "-"])
        .write_stdin("---\ntitle: T\ndraft: true\n---\n# H")
        .output()
        .unwrap();
    let without_draft = md_cmd()
        .env("HASH_IGNORE_PROPERTIES", "draft")
        .args(["hash", "--frontmatter", "-"])
        .write_stdin("---\ntitle: T\n---\n# H")
        .output()
        .unwrap();
    assert_eq!(with_draft.stdout, without_draft.stdout);
}

#[test]
fn test_hash_save_writes_baseline_and_exits_zero() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.md");
    std::fs::write(&file, "---\ntitle: T\n---\n# H\n\nBody.\n").unwrap();

    md_cmd()
        .arg("hash")
        .arg("--save")
        .arg(&file)
        .assert()
        .success()
        .stdout(predicate::str::contains("baseline"));

    let written = std::fs::read_to_string(&file).unwrap();
    assert!(written.contains("hash:"), "got:\n{written}");
}

#[test]
fn test_hash_save_requires_file_not_stdin() {
    md_cmd()
        .args(["hash", "--save", "-"])
        .write_stdin("# H")
        .assert()
        .failure();
}

#[test]
fn test_hash_diff_no_stored_hash_exits_two() {
    md_cmd()
        .args(["hash", "--diff", "-"])
        .write_stdin("---\ntitle: T\n---\n# H\n\nBody.")
        .assert()
        .code(2)
        .stdout(predicate::str::contains("No stored hash to compare against"));
}

#[test]
fn test_hash_diff_unchanged_exits_zero() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.md");
    std::fs::write(&file, "---\ntitle: T\n---\n# H\n\nBody.\n").unwrap();

    // Establish a baseline, then diff against it without any edit.
    md_cmd().arg("hash").arg("--save").arg(&file).assert().success();

    md_cmd()
        .arg("hash")
        .arg("--diff")
        .arg(&file)
        .assert()
        .success()
        .stdout(predicate::str::contains("No semantic changes detected"));
}

#[test]
fn test_hash_diff_changed_exits_two() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("doc.md");
    std::fs::write(&file, "---\ntitle: T\n---\n# H\n\nBody.\n").unwrap();

    md_cmd().arg("hash").arg("--save").arg(&file).assert().success();

    // Edit the body, leaving the stored hash in place.
    let stored = std::fs::read_to_string(&file).unwrap();
    let edited = stored.replace("Body.", "Different body.");
    std::fs::write(&file, edited).unwrap();

    md_cmd().arg("hash").arg("--diff").arg(&file).assert().code(2);
}

#[test]
fn test_hash_directory_rejects_save() {
    let dir = create_hash_dir();
    md_cmd()
        .args(["hash", "--save"])
        .arg(dir.path())
        .assert()
        .failure();
}

#[test]
fn test_hash_directory_rejects_structured_kind() {
    let dir = create_hash_dir();
    md_cmd()
        .args(["hash", "--kind", "structured"])
        .arg(dir.path())
        .assert()
        .failure();
}

/// A single malformed Markdown file must fail the whole directory aggregate
/// rather than being silently hashed as an empty document. Otherwise a CI /
/// release check using `md hash <dir>` could pass on a broken file and record
/// a false baseline.
#[test]
fn test_hash_directory_malformed_frontmatter_exits_one() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("good.md"),
        "---\ntitle: Alpha\n---\n# Alpha\n",
    )
    .unwrap();
    // Quoted scalar followed by trailing unquoted text: a frontmatter parse error.
    std::fs::write(
        dir.path().join("bad.md"),
        "---\nphases: 5\nfindings:\n  - id: '@' magic lookup emits results\n---\n# Doc\n",
    )
    .unwrap();

    md_cmd()
        .arg("hash")
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("bad.md"));
}

// =============================================================================
//                          GET SUBCOMMAND TESTS
// =============================================================================

const FM_DOC: &str = "---\ntitle: Hello World\nauthor: Alice\ncount: 42\ntags:\n  - rust\n  - cli\n---\n# Content\n\nBody text.";
const FM_DOC_TAB_INDENT: &str = "---\nprompt: |-\n\tLine one\n\tLine two\nlast_updated: 2026-02-27\nmodel: Gemini 3 Pro\n---\n# Content\n";

#[test]
fn test_get_single_property_string() {
    md_cmd()
        .args(["get", "-", "title"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"Hello World\""));
}

#[test]
fn test_get_single_property_number() {
    md_cmd()
        .args(["get", "-", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("42"));
}

#[test]
fn test_get_single_property_array() {
    md_cmd()
        .args(["get", "-", "tags"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("rust"))
        .stdout(predicate::str::contains("cli"));
}

#[test]
fn test_get_missing_property_returns_empty_string() {
    md_cmd()
        .args(["get", "-", "nonexistent"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"\""));
}

#[test]
fn test_get_multiple_properties_returns_object() {
    md_cmd()
        .args(["get", "-", "title", "author"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\""))
        .stdout(predicate::str::contains("\"Hello World\""))
        .stdout(predicate::str::contains("\"author\""))
        .stdout(predicate::str::contains("\"Alice\""));
}

#[test]
fn test_get_multiple_with_missing_includes_empty_string() {
    md_cmd()
        .args(["get", "-", "title", "missing"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\""))
        .stdout(predicate::str::contains("\"Hello World\""))
        .stdout(predicate::str::contains("\"missing\""))
        .stdout(predicate::str::contains("\"\""));
}

#[test]
fn test_get_json5_output() {
    md_cmd()
        .args(["get", "--json5", "-", "title", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        // JSON5 uses unquoted keys for valid identifiers
        .stdout(predicate::str::contains("title:"))
        .stdout(predicate::str::contains("count:"));
}

#[test]
fn test_get_yaml_output() {
    md_cmd()
        .args(["get", "--yaml", "-", "title"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello World"));
}

#[test]
fn test_get_toml_output() {
    md_cmd()
        .args(["get", "--toml", "-", "title", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("title"))
        .stdout(predicate::str::contains("Hello World"))
        .stdout(predicate::str::contains("count"))
        .stdout(predicate::str::contains("42"));
}

#[test]
fn test_get_from_file() {
    let tmp = md_file("---\nversion: 2\n---\n# Doc\n");

    md_cmd()
        .args(["get"])
        .arg(tmp.path())
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("2"));
}

#[test]
fn test_get_no_frontmatter_returns_empty_string() {
    md_cmd()
        .args(["get", "-", "title"])
        .write_stdin("# No frontmatter")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"\""));
}

/// Regression: malformed frontmatter (a quoted scalar followed by trailing
/// unquoted text) used to be silently treated as "no frontmatter", so
/// `md get phases` returned `""` even when the file clearly defined `phases`.
/// The fix surfaces a `MarkdownError::FrontmatterParse` with the offending
/// YAML line in the rendered StatusBlock.
#[test]
fn test_get_malformed_frontmatter_renders_status_block_with_offending_line() {
    use darkmatter::testing::strip_ansi_codes;

    let yaml = "---\nphases: 5\nfindings:\n  - id: '@' magic lookup emits results\n---\n# Doc\n";

    let output = md_cmd()
        .args(["get", "-", "phases"])
        .write_stdin(yaml)
        .output()
        .expect("md get should run");

    assert!(!output.status.success(), "expected a failure exit status");

    // The offending YAML line is syntax-highlighted, so its characters are
    // interleaved with SGR escapes in the raw stderr; strip ANSI before
    // asserting on the visible text.
    let stderr = strip_ansi_codes(&String::from_utf8_lossy(&output.stderr));
    assert!(stderr.contains("MarkdownError"), "stderr: {stderr}");
    assert!(stderr.contains("frontmatter parse failed"), "stderr: {stderr}");
    assert!(
        stderr.contains("'@' magic lookup emits results"),
        "offending line must be shown. stderr: {stderr}"
    );
}

#[test]
fn test_get_tab_indented_frontmatter_property_is_populated() {
    md_cmd()
        .args(["get", "-", "last_updated"])
        .write_stdin(FM_DOC_TAB_INDENT)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"2026-02-27\""));
}

// -- raw flag tests --

#[test]
fn test_get_raw_string_unquoted() {
    md_cmd()
        .args(["get", "--raw", "-", "title"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout("Hello World\n");
}

#[test]
fn test_get_raw_number() {
    md_cmd()
        .args(["get", "--raw", "-", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout("42\n");
}

#[test]
fn test_get_raw_null_returns_empty() {
    md_cmd()
        .args(["get", "--raw", "-", "nonexistent"])
        .write_stdin("---\nnonexistent: null\n---\n# Doc")
        .assert()
        .success()
        .stdout("\n");
}

#[test]
fn test_get_raw_array_one_per_line() {
    md_cmd()
        .args(["get", "--raw", "-", "tags"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout("rust\ncli\n");
}

#[test]
fn test_get_raw_object_key_value_lines() {
    md_cmd()
        .args(["get", "--raw", "-", "title", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("title: Hello World"))
        .stdout(predicate::str::contains("count: 42"));
}

// -- compact flag tests --

#[test]
fn test_get_compact_array() {
    md_cmd()
        .args(["get", "--compact", "-", "tags"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout("[\"rust\",\"cli\"]\n");
}

#[test]
fn test_get_compact_object() {
    md_cmd()
        .args(["get", "--compact", "-", "title", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\":\"Hello World\""))
        .stdout(predicate::str::contains("\"count\":42"));
}

#[test]
fn test_get_compact_scalar_unchanged() {
    md_cmd()
        .args(["get", "--compact", "-", "title"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout("\"Hello World\"\n");
}

// =============================================================================
//                          SET SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_set_string_value_via_stdin() {
    md_cmd()
        .args(["set", "-", "title", "New Title"])
        .write_stdin("---\ntitle: Old Title\n---\n# Content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("title: New Title"))
        .stdout(predicate::str::contains("# Content"));
}

#[test]
fn test_set_adds_new_property_via_stdin() {
    md_cmd()
        .args(["set", "-", "author", "Alice"])
        .write_stdin("---\ntitle: Hello\n---\n# Content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("author: Alice"))
        .stdout(predicate::str::contains("title: Hello"));
}

#[test]
fn test_set_numeric_value() {
    md_cmd()
        .args(["set", "-", "count", "42"])
        .write_stdin("---\ntitle: Test\n---\n# Content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("count: 42"));
}

#[test]
fn test_set_boolean_value() {
    md_cmd()
        .args(["set", "-", "draft", "true"])
        .write_stdin("---\ntitle: Test\n---\n# Content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("draft: true"));
}

#[test]
fn test_set_json_array_value() {
    md_cmd()
        .args(["set", "-", "tags", r#"["rust","cli"]"#])
        .write_stdin("---\ntitle: Test\n---\n# Content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("rust"))
        .stdout(predicate::str::contains("cli"));
}

#[test]
fn test_set_creates_frontmatter_when_none_exists() {
    md_cmd()
        .args(["set", "-", "title", "Brand New"])
        .write_stdin("# No Frontmatter\n\nJust content.\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("title: Brand New"))
        .stdout(predicate::str::contains("# No Frontmatter"));
}

#[test]
fn test_set_updates_file_in_place() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "---\ntitle: Original\n---\n# Content\n").unwrap();

    md_cmd()
        .arg("set")
        .arg(tmp.path())
        .args(["title", "Updated", "--save"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    let updated = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(updated.contains("title: Updated"));
    assert!(updated.contains("# Content"));
    assert!(!updated.contains("Original"));
}

#[test]
fn test_set_without_save_does_not_mutate_file() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "---\ntitle: Original\n---\n# Content\n").unwrap();

    md_cmd()
        .arg("set")
        .arg(tmp.path())
        .args(["title", "Updated"])
        .assert()
        .success()
        .stdout(predicate::str::contains("title: Updated"))
        .stdout(predicate::str::contains("# Content"));

    // File should be unchanged
    let on_disk = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(on_disk.contains("title: Original"));
}

#[test]
fn test_set_preserves_body_content() {
    let input =
        "---\ntitle: Test\n---\n# Heading\n\nParagraph with **bold** text.\n\n- list item\n";
    md_cmd()
        .args(["set", "-", "version", "2"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("# Heading"))
        .stdout(predicate::str::contains("**bold**"))
        .stdout(predicate::str::contains("- list item"));
}

#[test]
fn test_get_requires_at_least_one_prop() {
    md_cmd()
        .args(["get", "-"])
        .write_stdin(FM_DOC)
        .assert()
        .failure();
}

// =============================================================================
//                      RM SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_rm_removes_single_property() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(
        &file,
        "---\ntitle: Hello\nauthor: Alice\ndate: 2024-01-01\n---\n\n# Content\n",
    )
    .unwrap();

    md_cmd()
        .args(["rm", file.to_str().unwrap(), "author"])
        .assert()
        .success();

    let content = std::fs::read_to_string(&file).unwrap();
    assert!(!content.contains("author:"));
    assert!(content.contains("title: Hello"));
    assert!(content.contains("date: 2024-01-01"));
    assert!(content.contains("# Content"));
}

#[test]
fn test_rm_removes_multiple_properties() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(
        &file,
        "---\ntitle: Hello\nauthor: Alice\ndate: 2024-01-01\ntags: [rust, cli]\n---\n\n# Content\n",
    )
    .unwrap();

    md_cmd()
        .args(["rm", file.to_str().unwrap(), "author", "tags"])
        .assert()
        .success();

    let content = std::fs::read_to_string(&file).unwrap();
    assert!(!content.contains("author:"));
    assert!(!content.contains("tags:"));
    assert!(content.contains("title: Hello"));
    assert!(content.contains("date: 2024-01-01"));
    assert!(content.contains("# Content"));
}

#[test]
fn test_rm_nonexistent_key_fails() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(
        &file,
        "---\ntitle: Hello\nauthor: Alice\n---\n\n# Content\n",
    )
    .unwrap();

    md_cmd()
        .args(["rm", file.to_str().unwrap(), "missing"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found in frontmatter"));
}

#[test]
fn test_rm_partial_nonexistent_fails() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(
        &file,
        "---\ntitle: Hello\nauthor: Alice\n---\n\n# Content\n",
    )
    .unwrap();

    md_cmd()
        .args(["rm", file.to_str().unwrap(), "title", "missing"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found in frontmatter"));

    // File should be unchanged when command fails
    let content = std::fs::read_to_string(&file).unwrap();
    assert!(content.contains("title: Hello"));
    assert!(content.contains("author: Alice"));
}

#[test]
fn test_rm_with_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(
        &file,
        "---\ntitle: Hello\nauthor: Alice\ndate: 2024-01-01\n---\n\n# Content\n",
    )
    .unwrap();

    let output = md_cmd()
        .args(["rm", file.to_str().unwrap(), "author", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = String::from_utf8_lossy(&output);
    assert!(json_str.contains("\"removed\""));
    assert!(json_str.contains("\"remaining\""));
    assert!(json_str.contains("\"filename\""));
    assert!(json_str.contains("\"author\""));
    assert!(json_str.contains("\"title\""));
    assert!(json_str.contains("\"date\""));
}

#[test]
fn test_rm_with_verbose_output() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(
        &file,
        "---\ntitle: Hello\nauthor: Alice\ndate: 2024-01-01\n---\n\n# Content\n",
    )
    .unwrap();

    md_cmd()
        .args(["rm", file.to_str().unwrap(), "author", "-v"])
        .assert()
        .success()
        .stderr(predicate::str::contains("removed"));
}

#[test]
fn test_rm_preserves_body_content() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    let body = "# Heading\n\nParagraph with **bold** text.\n\n- list item\n";
    std::fs::write(
        &file,
        format!("---\ntitle: Test\nauthor: Alice\n---\n\n{}", body),
    )
    .unwrap();

    md_cmd()
        .args(["rm", file.to_str().unwrap(), "author"])
        .assert()
        .success();

    let content = std::fs::read_to_string(&file).unwrap();
    assert!(content.contains("# Heading"));
    assert!(content.contains("**bold**"));
    assert!(content.contains("- list item"));
}

#[test]
fn test_rm_requires_at_least_one_prop() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(&file, "---\ntitle: Hello\n---\n\n# Content\n").unwrap();

    md_cmd()
        .args(["rm", file.to_str().unwrap()])
        .assert()
        .failure();
}

// =============================================================================
//                      SHELL EXPANSION TESTS
// =============================================================================

#[test]
fn test_compose_with_whitelisted_command_succeeds() {
    let temp_dir = tempfile::TempDir::new().unwrap();

    // Write a markdown file with a shell directive
    let md_path = temp_dir.path().join("test.md");
    std::fs::write(&md_path, "# Test\n::shell echo hello\n").unwrap();

    // Write a whitelist
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");
    std::fs::write(&whitelist_path, "prefix echo\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("hello"));
}

#[test]
fn test_compose_with_blacklisted_command_fails() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    std::fs::write(&md_path, "# Test\n::shell rm -rf /\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Blacklisted").or(predicate::str::contains("dangerous")));
}

#[test]
fn test_compose_stdin_unapproved_command_fails_with_guidance() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");

    md_cmd()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .arg("compose")
        .arg("-")
        .write_stdin("# Test\n::shell echo hello\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Approval required for 'echo hello'.",
        ))
        .stderr(predicate::str::contains(
            "To allow in non-interactive mode, add one of these to",
        ))
        .stderr(predicate::str::contains(
            whitelist_path.display().to_string(),
        ))
        .stderr(predicate::str::contains("exact echo hello"))
        .stderr(predicate::str::contains("prefix echo"));
}

#[test]
fn test_compose_file_unapproved_command_fails_with_guidance() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");

    std::fs::write(&md_path, "# Test\n::shell echo hello\n").unwrap();

    md_cmd()
        .current_dir(temp_dir.path())
        .arg("compose")
        .arg(&md_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Approval required for 'echo hello'.",
        ))
        .stderr(predicate::str::contains(
            "To allow in non-interactive mode, add one of these to",
        ))
        .stderr(predicate::str::contains(
            whitelist_path.display().to_string(),
        ))
        .stderr(predicate::str::contains("exact echo hello"))
        .stderr(predicate::str::contains("prefix echo"));
}

#[test]
fn test_compose_with_nonexistent_command_fails() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    std::fs::write(&md_path, "# Test\n::shell nonexistent_command_xyz\n").unwrap();

    // Write whitelist to approve the command
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");
    std::fs::write(&whitelist_path, "prefix nonexistent_command_xyz\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Command not found"));
}

#[test]
fn test_compose_timeout_flag_fails_timed_out_shell() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");

    std::fs::write(&md_path, "# Test\n::shell sleep 2\n").unwrap();
    std::fs::write(&whitelist_path, "prefix sleep\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&md_path)
        .arg("--timeout")
        .arg("1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("timed out"));
}

#[test]
fn test_compose_allow_shell_timeout_emits_warning() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");

    std::fs::write(&md_path, "# Test\n::shell sleep 2\nAfter\n").unwrap();
    std::fs::write(&whitelist_path, "prefix sleep\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&md_path)
        .arg("--timeout")
        .arg("1")
        .arg("--allow-shell-timeout")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Test"))
        .stdout(predicate::str::contains("After"))
        .stderr(predicate::str::contains("timed out"))
        .stderr(predicate::str::contains("replaced with an empty"));
}

#[test]
fn test_compose_shell_reports_discovered_commands_without_executing() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let root_path = temp_dir.path().join("root.md");
    let child_path = temp_dir.path().join("child.md");

    std::fs::write(
        &root_path,
        "---\nroot_cmd: \"$(echo root-frontmatter)\"\n---\n# Root\n::shell echo root-body\n::file ./child.md\n",
    )
    .unwrap();
    std::fs::write(
        &child_path,
        "---\nchild_cmd: \"$(echo child-frontmatter)\"\n---\n# Child\n::shell echo child-body\n",
    )
    .unwrap();

    md_cmd()
        .arg("compose")
        .arg(&root_path)
        .arg("--shell")
        .assert()
        .success()
        .stdout(predicate::str::contains("Shell commands discovered: 4"))
        .stdout(predicate::str::contains("echo root-frontmatter"))
        .stdout(predicate::str::contains("frontmatter.root_cmd"))
        .stdout(predicate::str::contains("echo root-body"))
        .stdout(predicate::str::contains("echo child-frontmatter"))
        .stdout(predicate::str::contains("frontmatter.child_cmd"))
        .stdout(predicate::str::contains("echo child-body"))
        .stdout(predicate::str::contains("root-frontmatter\n").not())
        .stdout(predicate::str::contains("child-frontmatter\n").not());
}

// =============================================================================
//                     VALIDATE REFS CLI TESTS (rec #15)
// =============================================================================

#[test]
fn validate_refs_text_output() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("test.md");
    std::fs::write(&md_path, "# Heading\n\n[link](https://example.com)\n").unwrap();

    md_cmd()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .assert()
        .success();
}

#[test]
fn validate_refs_json_output() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("test.md");
    std::fs::write(&md_path, "[link](https://example.com)\n").unwrap();

    let output = md_cmd()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // JSON output should be parseable
    let _: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("Expected valid JSON, got error: {e}\nOutput: {stdout}");
    });
}

#[test]
fn validate_refs_nonzero_exit_on_errors() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("test.md");
    std::fs::write(&md_path, "[broken](./nonexistent.md)\n").unwrap();

    md_cmd()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .assert()
        .failure();
}

#[test]
fn validate_refs_with_fragments() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("test.md");
    std::fs::write(&md_path, "# Hello\n\n[link](#hello)\n").unwrap();

    md_cmd()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .arg("--fragments")
        .assert()
        .success();
}

#[test]
fn validate_refs_graph_mermaid() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("test.md");
    std::fs::write(&md_path, "# Test\n\n[link](https://example.com)\n").unwrap();

    let output = md_cmd()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .arg("--graph")
        .arg("mermaid")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("flowchart TD"),
        "Expected mermaid flowchart output, got: {stdout}"
    );
}

#[test]
fn validate_refs_graph_dot() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("test.md");
    std::fs::write(&md_path, "# Test\n\n[link](https://example.com)\n").unwrap();

    let output = md_cmd()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .arg("--graph")
        .arg("dot")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("digraph"),
        "Expected dot graph output, got: {stdout}"
    );
}

// =============================================================================
//                          GRAPH COMMAND TESTS
// =============================================================================

#[test]
fn test_graph_basic() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(
        tmp,
        "# Test\n\n[link](https://example.com)\n\n![img](./logo.png)"
    )
    .unwrap();

    md_cmd()
        .arg("graph")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("example.com"))
        .stdout(predicate::str::contains("logo.png"));
}

#[test]
fn test_graph_follow() {
    let dir = tempfile::TempDir::new().unwrap();
    let parent = dir.path().join("parent.md");
    let child = dir.path().join("child.md");
    std::fs::write(&parent, "# Parent\n\n::file child.md").unwrap();
    std::fs::write(&child, "# Child\n\n[link](https://child.example.com)").unwrap();

    md_cmd()
        .arg("graph")
        .arg(&parent)
        .arg("--follow")
        .assert()
        .success()
        .stdout(predicate::str::contains("parent.md"))
        .stdout(predicate::str::contains("child.md"));
}

#[test]
fn test_graph_validate_valid() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("valid.md");
    let linked = dir.path().join("linked.md");
    std::fs::write(&md_path, "# Valid\n\n[link](./linked.md)").unwrap();
    std::fs::write(&linked, "# Linked").unwrap();

    md_cmd()
        .arg("graph")
        .arg(&md_path)
        .arg("--validate")
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"))
        .stdout(predicate::str::contains("0 issues"));
}

#[test]
fn test_graph_validate_invalid() {
    let tmp = md_file("# Test\n\n[broken](./nonexistent.md)\n");

    let output = md_cmd()
        .arg("graph")
        .arg(tmp.path())
        .arg("--validate")
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit code 2 for validation errors"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[missing]"),
        "expected [missing] suffix in output"
    );
    assert!(
        stdout.contains("1 issues"),
        "expected issue count in summary"
    );
}

#[test]
fn test_graph_follow_toc_linking() {
    let dir = tempfile::TempDir::new().unwrap();
    let parent = dir.path().join("root.md");
    let child = dir.path().join("child.md");
    std::fs::write(&parent, "# Root\n\n::toc-linking child.md").unwrap();
    std::fs::write(
        &child,
        "# Child\n\n## Section A\n\n## Section B\n\n[link](https://child.example.com)",
    )
    .unwrap();

    md_cmd()
        .arg("graph")
        .arg(&parent)
        .arg("--follow")
        .assert()
        .success()
        .stdout(predicate::str::contains("root.md"))
        .stdout(predicate::str::contains("child.md"))
        .stdout(predicate::str::contains("child.example.com"));
}

#[test]
fn test_graph_follow_validate_child_broken_link() {
    let dir = tempfile::TempDir::new().unwrap();
    let parent = dir.path().join("root.md");
    let child = dir.path().join("child.md");
    std::fs::write(&parent, "# Root\n\n::toc-linking child.md").unwrap();
    std::fs::write(&child, "# Child\n\n[broken](./missing.md)").unwrap();

    let output = md_cmd()
        .arg("graph")
        .arg(&parent)
        .arg("--follow")
        .arg("--validate")
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit code 2 when followed child has a broken link"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[missing]"),
        "expected [missing] suffix for broken link in child"
    );
}

#[test]
fn test_graph_follow_multiple_prologues() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path().join("root.md");
    let a = dir.path().join("a.md");
    let b = dir.path().join("b.md");
    std::fs::write(&root, "---\nprologue:\n  - a.md\n  - b.md\n---\n\n# Root").unwrap();
    std::fs::write(&a, "# A\n\n[a-link](https://a.example.com)").unwrap();
    std::fs::write(&b, "# B\n\n[b-link](https://b.example.com)").unwrap();

    md_cmd()
        .arg("graph")
        .arg(&root)
        .arg("--follow")
        .assert()
        .success()
        .stdout(predicate::str::contains("a.md"))
        .stdout(predicate::str::contains("b.md"))
        .stdout(predicate::str::contains("a.example.com"))
        .stdout(predicate::str::contains("b.example.com"));
}

#[test]
fn test_graph_follow_epilogue() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path().join("root.md");
    let epilogue = dir.path().join("epilogue.md");
    std::fs::write(
        &root,
        "---\nepilogue: epilogue.md\n---\n\n# Root\n\n[main](https://main.example.com)",
    )
    .unwrap();
    std::fs::write(
        &epilogue,
        "# Epilogue\n\n[epi-link](https://epilogue.example.com)",
    )
    .unwrap();

    md_cmd()
        .arg("graph")
        .arg(&root)
        .arg("--follow")
        .assert()
        .success()
        .stdout(predicate::str::contains("root.md"))
        .stdout(predicate::str::contains("epilogue.md"))
        .stdout(predicate::str::contains("epilogue.example.com"));
}

#[test]
fn test_graph_file_not_found() {
    md_cmd()
        .arg("graph")
        .arg("/nonexistent/file.md")
        .assert()
        .failure();
}

#[test]
fn test_graph_help() {
    md_cmd()
        .arg("graph")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--follow"))
        .stdout(predicate::str::contains("--validate"));
}

// =============================================================================
//                          UNTESTED FLAG COVERAGE
// =============================================================================

#[test]
fn test_list_themes() {
    md_cmd()
        .arg("--list-themes")
        .assert()
        .success()
        .stdout(predicate::str::contains("Available themes"))
        .stdout(predicate::str::contains("github"))
        .stdout(predicate::str::contains("solarized"));
}

#[test]
fn test_completions_bash() {
    md_cmd()
        .args(["--completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_completions_zsh() {
    md_cmd()
        .args(["--completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_completions_fish() {
    md_cmd()
        .args(["--completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_line_numbers_html_output() {
    let input = "```rust\nfn main() {}\n```";
    // Use `--line-numbers=true` to avoid the optional-arg ambiguity with the
    // `-` stdin marker positional. The bare form `--line-numbers -` would let
    // clap consume `-` as the optional value.
    md_cmd()
        .args(["--output", "html", "--line-numbers=true", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("<style>"))
        .stdout(predicate::str::contains("main"));
}

#[test]
fn test_compose_compact() {
    let input = "---\n---\n\n- item 1\n\n- item 2\n\n- item 3";
    md_cmd()
        .args(["compose", "--compact", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("- item 1\n- item 2\n- item 3"));
}

#[test]
fn test_compose_loose() {
    let input = "---\n---\n\n- item 1\n- item 2\n- item 3";
    md_cmd()
        .args(["compose", "--loose", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("- item 1\n\n- item 2\n\n- item 3"));
}

#[test]
fn test_graph_json_output() {
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(&file, "# Test\n\n[link](https://example.com)").unwrap();

    md_cmd()
        .args(["graph", "--json"])
        .arg(&file)
        .assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("\"references\""))
        .stdout(predicate::str::contains("example.com"));
}

// =============================================================================
//                  BLOCK RENDERING END-TO-END TESTS
// =============================================================================

/// End-to-end test for the CLI block rendering path.
///
/// Creates a transclusion cycle (A includes B, B includes A), runs
/// `md compose`, and asserts that the CLI:
/// - exits with a non-zero code,
/// - emits the error type name (`TransclusionError`) on stderr,
/// - emits a human-readable summary (`cycle detected`),
/// - emits a hint-tagged token from the rendered block.
#[test]
fn test_block_rendering_transclusion_cycle_tty() {
    let dir = tempfile::TempDir::new().unwrap();
    let a = dir.path().join("a.md");
    let b = dir.path().join("b.md");
    std::fs::write(&a, "# A\n\n::file b.md\n").unwrap();
    std::fs::write(&b, "# B\n\n::file a.md\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&a)
        .assert()
        .failure()
        .stderr(predicate::str::contains("TransclusionError"))
        .stderr(predicate::str::contains("cycle detected"))
        .stderr(predicate::str::contains("Break the cycle"));
}

/// Non-TTY block rendering: the same cycle error must still produce
/// readable plain text (optimistic 80-column render) when stderr is
/// piped. `assert_cmd` runs commands with piped stdio by default, so
/// this test naturally exercises the non-TTY branch in `main.rs`.
#[test]
fn test_block_rendering_transclusion_cycle_non_tty() {
    let dir = tempfile::TempDir::new().unwrap();
    let a = dir.path().join("a.md");
    let b = dir.path().join("b.md");
    std::fs::write(&a, "# A\n\n::file b.md\n").unwrap();
    std::fs::write(&b, "# B\n\n::file a.md\n").unwrap();

    let output = md_cmd().arg("compose").arg(&a).output().unwrap();

    assert!(!output.status.success(), "expected non-zero exit code");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cycle detected"),
        "stderr should contain human-readable summary in non-TTY mode\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Break the cycle"),
        "stderr should contain hint from rendered block in non-TTY mode\nstderr:\n{stderr}"
    );
}

// =============================================================================
//              FILEPATH INTERPOLATION CLI TESTS
// =============================================================================

#[test]
fn test_compose_link_relative_same_repo() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    let docs = repo.join("docs");
    let assets = repo.join("assets");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::create_dir_all(&assets).unwrap();

    let source_file = docs.join("source.md");
    let logo_file = assets.join("logo.png");
    std::fs::write(&source_file, "# Source\n\n![img](../assets/logo.png)\n").unwrap();
    std::fs::write(&logo_file, "png").unwrap();

    let output = md_cmd().arg("compose").arg(&source_file).output().unwrap();

    assert!(output.status.success(), "command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stdout.contains("../assets/logo.png"),
        "stdout should contain relative path, got:\n{stdout}"
    );
    // Should not contain absolute path
    assert!(
        !stdout.contains(assets.to_string_lossy().as_ref()),
        "stdout should not contain absolute asset path, got:\n{stdout}"
    );
    // No diagnostics in stdout
    assert!(
        !stdout.contains("Total records"),
        "stdout should not contain diagnostics, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("Record kind"),
        "stdout should not contain diagnostics, got:\n{stdout}"
    );
    // No unexpected stderr
    assert!(
        !stderr.contains("link_normalization"),
        "stderr should not contain raw warning tokens, got:\n{stderr}"
    );
}

#[test]
fn test_compose_link_transcluded_child() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(repo.join(".git")).unwrap();

    let docs = repo.join("docs");
    let components = repo.join("components");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::create_dir_all(&components).unwrap();

    let parent_file = docs.join("parent.md");
    let child_file = components.join("child.md");
    let sibling_file = components.join("sibling.md");

    std::fs::write(&parent_file, "# Parent\n\n::file ../components/child.md\n").unwrap();
    std::fs::write(&child_file, "[link](./sibling.md)\n").unwrap();
    std::fs::write(&sibling_file, "sibling content\n").unwrap();

    let output = md_cmd().arg("compose").arg(&parent_file).output().unwrap();

    assert!(output.status.success(), "command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("../components/sibling.md"),
        "stdout should contain normalized sibling path relative to parent, got:\n{stdout}"
    );
    // Should not contain absolute path
    let abs_sibling = std::fs::canonicalize(&sibling_file).unwrap();
    assert!(
        !stdout.contains(abs_sibling.to_string_lossy().as_ref()),
        "stdout should not contain absolute path, got:\n{stdout}"
    );
}

#[test]
fn test_compose_env_var_substitution_one_warning() {
    let dir = tempfile::tempdir().unwrap();
    let project_root = dir.path().join("project");
    std::fs::create_dir_all(&project_root).unwrap();
    let target_file = project_root.join("config.json");
    std::fs::write(&target_file, "{}").unwrap();

    let abs_target = std::fs::canonicalize(&target_file).unwrap();
    let abs_root = std::fs::canonicalize(&project_root).unwrap();

    let md_file = dir.path().join("test.md");
    std::fs::write(&md_file, format!("[config]({})\n", abs_target.display())).unwrap();

    let output = md_cmd()
        .env("PROJECT_ROOT", &abs_root)
        .arg("compose")
        .arg(&md_file)
        .output()
        .unwrap();

    assert!(output.status.success(), "command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    eprintln!("DEBUG stdout:\n{stdout}");
    eprintln!("DEBUG stderr:\n{stderr}");

    assert!(
        stdout.contains("${PROJECT_ROOT}/config.json"),
        "stdout should contain env-var abstraction, got:\n{stdout}"
    );
    // Warning text should NOT be in stdout
    assert!(
        !stdout.contains("environment variable"),
        "stdout should not contain warning text, got:\n{stdout}"
    );

    // stderr should contain exactly one warning about the env var
    let warning_count = stderr.matches("environment variable").count();
    assert_eq!(
        warning_count, 1,
        "stderr should contain exactly one env-var warning, got {warning_count} occurrences:\n{stderr}"
    );
}

#[test]
fn test_compose_html_spaced_attributes() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(repo.join(".git")).unwrap();

    let page_file = repo.join("page.md");
    let other_file = repo.join("other.md");
    let img_file = repo.join("img.png");

    std::fs::write(
        &page_file,
        "# Page\n\n<a href = \"./other.md\">link</a>\n\n<img src = \"./img.png\">\n",
    )
    .unwrap();
    std::fs::write(&other_file, "other content\n").unwrap();
    std::fs::write(&img_file, "png").unwrap();

    let output = md_cmd().arg("compose").arg(&page_file).output().unwrap();

    assert!(output.status.success(), "command should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("other.md"),
        "stdout should contain normalized other.md path, got:\n{stdout}"
    );
    assert!(
        stdout.contains("img.png"),
        "stdout should contain normalized img.png path, got:\n{stdout}"
    );
    // Should not contain the spaced attribute syntax unprocessed
    assert!(
        !stdout.contains("href = \"./other.md\""),
        "stdout should not contain unprocessed spaced href, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("src = \"./img.png\""),
        "stdout should not contain unprocessed spaced src, got:\n{stdout}"
    );
}

// =============================================================================
//                          LAYOUT FLAGS (Phase 5)
// =============================================================================

#[test]
fn layout_margin_shorthand_overrides_axis_and_side() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--margin")
        .arg("4")
        .arg("--mx")
        .arg("2")
        .arg("--mt")
        .arg("1")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "margin flags should parse and be accepted"
    );
}

#[test]
fn layout_padding_shorthand_overrides_axis_and_side() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--padding")
        .arg("4")
        .arg("--px")
        .arg("2")
        .arg("--pt")
        .arg("1")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "padding flags should parse and be accepted"
    );
}

#[test]
fn layout_max_width_zero_rejected() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--max-width")
        .arg("0")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("max-width") || stderr.contains("0"),
        "should reject --max-width 0, got: {stderr}"
    );
}

#[test]
fn layout_max_width_positive_accepted() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--max-width")
        .arg("80")
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn layout_fill_full_accepted() {
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--fill")
        .arg("full")
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn layout_fill_pad_fixed_accepted() {
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--fill")
        .arg("pad=4")
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn layout_fill_pad_percent_accepted() {
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--fill")
        .arg("pad=10%")
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn layout_fill_indent_max_explicit_accepted() {
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    for fill in ["indent=2", "max=40", "explicit=60"] {
        let output = md_cmd()
            .arg(tmp.path())
            .arg("--fill")
            .arg(fill)
            .output()
            .unwrap();
        assert!(output.status.success(), "--fill {fill} should succeed");
    }
}

#[test]
fn layout_fill_unknown_kind_rejected() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--fill")
        .arg("unknown=4")
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn layout_fill_percent_over_100_rejected() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--fill")
        .arg("pad=150%")
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn layout_fill_negative_rejected() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--fill")
        .arg("pad=-1")
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn layout_alignment_global_accepted() {
    let tmp = md_file("| A | B |\n|---|---|\n| 1 | 2 |\n");
    for align in ["left", "center", "right"] {
        let output = md_cmd()
            .arg(tmp.path())
            .arg("--alignment")
            .arg(align)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "--alignment {align} should succeed"
        );
    }
}

#[test]
fn layout_align_component_overrides_global() {
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--alignment")
        .arg("center")
        .arg("--align-code-blocks")
        .arg("left")
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn layout_page_bg_accepted() {
    let tmp = md_file("# Hello\n");
    for bg in ["transparent", "subtle", "pronounced"] {
        let output = md_cmd()
            .arg(tmp.path())
            .arg("--page-bg")
            .arg(bg)
            .output()
            .unwrap();
        assert!(output.status.success(), "--page-bg {bg} should succeed");
    }
}

#[test]
fn layout_page_background_alias_works() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--page-background")
        .arg("subtle")
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn layout_line_numbers_flag_accepted() {
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--line-numbers")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "--line-numbers flag should be accepted"
    );
}

#[test]
fn layout_line_numbers_true_accepted() {
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--line-numbers")
        .arg("true")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "--line-numbers true should be accepted"
    );
}

#[test]
fn layout_line_numbers_false_accepted() {
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--line-numbers")
        .arg("false")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "--line-numbers false should be accepted"
    );
}

#[test]
fn layout_fill_component_specific_accepted() {
    let tmp = md_file("```rust\nfn main() {}\n```\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--fill-code-blocks")
        .arg("max=40")
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn layout_margin_negative_rejected() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--margin")
        .arg("-1")
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn layout_no_flags_preserves_existing_behavior() {
    let tmp = md_file("# Hello World\n\nSome prose here.\n");
    let output = md_cmd().arg(tmp.path()).output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Hello World"),
        "output should contain heading without layout flags"
    );
}

#[test]
fn layout_combined_margin_padding_bg() {
    let tmp = md_file("# Hello\n");
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--margin")
        .arg("2")
        .arg("--padding")
        .arg("1")
        .arg("--page-bg")
        .arg("subtle")
        .output()
        .unwrap();

    assert!(output.status.success());
}

// =============================================================================
//                          RESOLVED LAYOUT PRECEDENCE TESTS
// =============================================================================
//
// These tests assert that CLI precedence rules produce the documented
// observable resolved page state — not just that the CLI parses successfully.
// They drive `apply_cli_layout_flags` against parsed `Cli` values and verify
// the final `DarkmatterPage` getters (`page_margin()`, `page_padding()`, `fill_for()`,
// `alignment_for()`, `max_width()`, `line_numbers()`).

use biscuit_terminal::terminal::Terminal;
use clap::Parser;
use darkmatter::layout::{DarkmatterPage, PageComponent};
use darkmatter_cli::Cli;
use darkmatter_cli::output::apply_cli_layout_flags;
use renderable::layout::{Alignment, Edges, Length, TargetValue, Width};

fn parse_cli(args: &[&str]) -> Cli {
    let mut full = vec!["md"];
    full.extend_from_slice(args);
    Cli::try_parse_from(full).expect("CLI args must parse")
}

fn resolved_page(args: &[&str]) -> DarkmatterPage {
    let cli = parse_cli(args);
    let term = Terminal::new_optimistic(120);
    apply_cli_layout_flags(DarkmatterPage::new(&term), &cli)
}

fn tv_cells(tv: &TargetValue<Length>) -> u16 {
    match tv {
        TargetValue::Universal(Length::Ch(n)) => u16::try_from(*n).unwrap_or(u16::MAX),
        _ => 0,
    }
}

fn alignment_for(page: &DarkmatterPage, component: PageComponent) -> Alignment {
    page.component_policy(component)
        .map(|p| p.layout.alignment)
        .unwrap_or_default()
}

#[derive(Debug, PartialEq)]
enum TestFill {
    Full,
    Pad(Length),
    Indent(Length),
    Max(Length),
    Explicit(Length),
}

fn fill_for(page: &DarkmatterPage, component: PageComponent) -> TestFill {
    match page.component_policy(component) {
        None => TestFill::Full,
        Some(p) => {
            let l = &p.layout;
            if l.width == Width::Auto && l.max_width.is_none() && l.padding == Edges::default() {
                TestFill::Full
            } else if l.width == Width::Auto
                && l.max_width.is_none()
                && l.padding != Edges::default()
            {
                // Pad: symmetric horizontal padding (left == right, top/bottom zero)
                if l.padding.top == TargetValue::universal(Length::Zero)
                    && l.padding.bottom == TargetValue::universal(Length::Zero)
                    && l.padding.left == l.padding.right
                {
                    TestFill::Pad(tv_length(&l.padding.left))
                } else {
                    // Asymmetric padding — treat as indent if only left is non-zero
                    TestFill::Indent(tv_length(&l.padding.left))
                }
            } else if let Some(max_width) = &l.max_width && l.width == Width::Auto {
                TestFill::Max(tv_length(max_width))
            } else if matches!(l.width, Width::Fixed(_)) {
                TestFill::Explicit(width_length(&l.width))
            } else {
                TestFill::Full
            }
        }
    }
}

fn tv_length(tv: &TargetValue<Length>) -> Length {
    match tv {
        TargetValue::Universal(l) => l.clone(),
        _ => Length::Zero,
    }
}

fn width_length(w: &Width) -> Length {
    match w {
        Width::Fixed(tv) => tv_length(tv),
        _ => Length::Zero,
    }
}


#[test]
fn layout_resolved_margin_shorthand_then_top_override() {
    // `-m 2 --mt 0`: shorthand sets all sides to 2, then --mt clears just the
    // top. The reviewer specifically called this out: precedence checks
    // should assert observable resolved behavior, not parse success.
    let page = resolved_page(&["fixture.md", "-m", "2", "--mt", "0"]);
    let m = page.page_margin();
    assert_eq!(tv_cells(&m.top), 0, "--mt 0 must override -m 2 on the top edge");
    assert_eq!(tv_cells(&m.bottom), 2, "-m 2 must apply to the bottom edge");
    assert_eq!(tv_cells(&m.left), 2, "-m 2 must apply to the left edge");
    assert_eq!(tv_cells(&m.right), 2, "-m 2 must apply to the right edge");
}

#[test]
fn layout_resolved_margin_axis_then_side() {
    // `-m 4 --mx 2 --mt 1`: shorthand 4 everywhere, then horizontal axis to 2,
    // then top to 1.
    let page = resolved_page(&["fixture.md", "-m", "4", "--mx", "2", "--mt", "1"]);
    let m = page.page_margin();
    assert_eq!(tv_cells(&m.top), 1, "--mt 1 overrides axis and shorthand on top");
    assert_eq!(tv_cells(&m.bottom), 4, "shorthand survives on bottom (no override)");
    assert_eq!(tv_cells(&m.left), 2, "--mx 2 overrides shorthand on left");
    assert_eq!(tv_cells(&m.right), 2, "--mx 2 overrides shorthand on right");
}

#[test]
fn layout_resolved_padding_axis_then_side() {
    let page = resolved_page(&["fixture.md", "--padding", "4", "--px", "2", "--pt", "1"]);
    let p = page.page_padding();
    assert_eq!(tv_cells(&p.top), 1);
    assert_eq!(tv_cells(&p.bottom), 4);
    assert_eq!(tv_cells(&p.left), 2);
    assert_eq!(tv_cells(&p.right), 2);
}

#[test]
fn layout_resolved_fill_global_then_component_specific() {
    // `--fill max=40 --fill-code-blocks max=30`: global fill applies to all
    // components, then code-block-specific fill overrides only that one.
    let page = resolved_page(&[
        "fixture.md",
        "--fill",
        "max=40",
        "--fill-code-blocks",
        "max=30",
    ]);
    assert_eq!(
        fill_for(&page, PageComponent::CodeBlocks),
        TestFill::Max(Length::ch(30)),
        "code-block-specific fill must override global"
    );
    for component in [PageComponent::Ul, PageComponent::Ol, PageComponent::Li] {
        assert_eq!(
            fill_for(&page, component),
            TestFill::Max(Length::ch(40)),
            "{:?} must still see the global fill",
            component
        );
    }
    assert_eq!(
        fill_for(&page, PageComponent::Tables),
        TestFill::Max(Length::ch(40)),
        "tables must still see the global fill"
    );
}

#[test]
fn layout_resolved_alignment_global_then_component_specific() {
    let page = resolved_page(&[
        "fixture.md",
        "--alignment",
        "center",
        "--align-code-blocks",
        "left",
    ]);
    assert_eq!(
        alignment_for(&page, PageComponent::CodeBlocks),
        Alignment::Left,
        "code-block-specific alignment must override global"
    );
    for component in [PageComponent::Ul, PageComponent::Ol, PageComponent::Li] {
        assert_eq!(
            alignment_for(&page, component),
            Alignment::Center,
            "{:?} must still see the global alignment",
            component
        );
    }
    assert_eq!(
        alignment_for(&page, PageComponent::BlockQuotes),
        Alignment::Center,
        "blockquotes must still see the global alignment"
    );
}



#[test]
fn layout_resolved_align_lists_broadcast_then_granular_override() {
    // `--align-lists right --align-ul left`: broadcast sets all three list
    // components to Right, then the granular flag overrides only Ul.
    let page = resolved_page(&[
        "fixture.md",
        "--align-lists",
        "right",
        "--align-ul",
        "left",
    ]);
    assert_eq!(
        alignment_for(&page, PageComponent::Ul),
        Alignment::Left,
        "granular --align-ul must override broadcast"
    );
    assert_eq!(
        alignment_for(&page, PageComponent::Ol),
        Alignment::Right,
        "Ol must still see the broadcast"
    );
    assert_eq!(
        alignment_for(&page, PageComponent::Li),
        Alignment::Right,
        "Li must still see the broadcast"
    );
}

#[test]
fn layout_resolved_fill_lists_broadcast_then_granular_override() {
    // `--fill-lists max=40 --fill-ol max=30`: broadcast sets all three list
    // components to Max(40), then the granular flag overrides only Ol.
    let page = resolved_page(&[
        "fixture.md",
        "--fill-lists",
        "max=40",
        "--fill-ol",
        "max=30",
    ]);
    assert_eq!(
        fill_for(&page, PageComponent::Ul),
        TestFill::Max(Length::ch(40)),
        "Ul must still see the broadcast"
    );
    assert_eq!(
        fill_for(&page, PageComponent::Ol),
        TestFill::Max(Length::ch(30)),
        "granular --fill-ol must override broadcast"
    );
    assert_eq!(
        fill_for(&page, PageComponent::Li),
        TestFill::Max(Length::ch(40)),
        "Li must still see the broadcast"
    );
}

#[test]
fn layout_resolved_max_width() {
    let page = resolved_page(&["fixture.md", "--max-width", "80"]);
    assert_eq!(page.max_width(), Some(80));
}

#[test]
fn layout_parsed_line_numbers_flag_values() {
    // `--line-numbers` (no value) defaults to true; `--line-numbers false`
    // explicitly disables. Verified against the parsed CLI struct since the
    // CLI's `render_terminal_output` applies this flag separately from the
    // layout-flag pipeline.
    assert_eq!(
        parse_cli(&["fixture.md", "--line-numbers"]).line_numbers,
        Some(true)
    );
    assert_eq!(
        parse_cli(&["fixture.md", "--line-numbers", "true"]).line_numbers,
        Some(true)
    );
    assert_eq!(
        parse_cli(&["fixture.md", "--line-numbers", "false"]).line_numbers,
        Some(false)
    );
    assert_eq!(parse_cli(&["fixture.md"]).line_numbers, None);
}

#[test]
fn layout_resolved_mt_alone_does_not_set_other_sides() {
    // `--mt 3` alone must leave other edges at default (0); no implicit
    // bleed from shorthand.
    let page = resolved_page(&["fixture.md", "--mt", "3"]);
    let m = page.page_margin();
    assert_eq!(tv_cells(&m.top), 3);
    assert_eq!(tv_cells(&m.bottom), 0);
    assert_eq!(tv_cells(&m.left), 0);
    assert_eq!(tv_cells(&m.right), 0);
}

// =============================================================================
//                  STYLE FRONTMATTER (Sub-Spec #2) END-TO-END
// =============================================================================
//
// These tests drive the `md` CLI with the canonical fixture
// (`darkmatter/example-docs/rendering/style-prop.md`) to confirm that the
// parse → CLI override → apply_page_style → render pipeline behaves as the
// sub-spec requires.

/// Locate the canonical fixture relative to the workspace root.
fn style_prop_fixture() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("example-docs")
        .join("rendering")
        .join("style-prop.md")
}

#[test]
fn style_fixture_cli_pipe_smoke_passes() {
    // Smoke check that `md style-prop.md` exits successfully and emits a
    // non-empty stdout when stdout is a pipe (the CLI test runner captures
    // stdout, so `OutputFormat::Auto` takes the markdown pass-through path
    // here — this test does NOT exercise the terminal renderer). Terminal
    // layout coverage lives in the Level 2 WezTerm pane tests
    // (`darkmatter/cli/tests/level2_layout.rs::level2_style_fixture_*`).
    let output = md_cmd().arg(style_prop_fixture()).output().unwrap();
    assert!(
        output.status.success(),
        "md style-prop.md must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "rendered output must not be empty");
    assert!(
        stdout.contains("Testing the `style` Property") || stdout.contains("Testing the"),
        "rendered output should contain the page title"
    );
}

#[test]
fn style_fixture_renders_html_successfully() {
    // Acceptance: `md --output html style-prop.md` uses the same page-level
    // frontmatter values through `render_to_browser`. MD_DRY_RUN avoids
    // launching a browser.
    let output = md_cmd()
        .arg(style_prop_fixture())
        .arg("--output")
        .arg("html")
        .env("MD_DRY_RUN", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "md --output html style-prop.md must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn style_fixture_strict_style_passes_on_schema_clean_doc() {
    // The fixture only generates `KnownButInactive` warnings (the ul / ol
    // keys are wired in later sub-specs). `--strict-style` must NOT fail on
    // `KnownButInactive`.
    let output = md_cmd()
        .arg(style_prop_fixture())
        .arg("--strict-style")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "--strict-style must succeed on schema-clean fixture: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn style_strict_style_fails_on_unknown_key() {
    // Spec test #5: `--strict-style` fails on `UnknownKey`. We route through
    // `--output html` so the frontmatter pipeline (which lives in the
    // terminal / HTML render paths) runs. The markdown-only artifact path
    // intentionally short-circuits to source pass-through.
    let tmp = md_file(
        "---\n\
        style:\n\
        \x20   page:\n\
        \x20       made-up-key: 2ch\n\
        ---\n\n# Doc\n",
    );

    let output = md_cmd()
        .arg(tmp.path())
        .arg("--output")
        .arg("html")
        .arg("--strict-style")
        .env("MD_DRY_RUN", "1")
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "--strict-style must fail on unknown key"
    );
}

#[test]
fn style_strict_style_fails_on_deprecated_key() {
    // `--strict-style` promotes `Deprecated` warnings to errors. The
    // canonical key is `style.page.left-margin`; the alias
    // `style.page.left_margin` should trigger a Deprecated warning, which
    // strict mode turns into an error. Route through `--output html` to
    // exercise the frontmatter pipeline.
    let tmp = md_file(
        "---\n\
        style:\n\
        \x20   page:\n\
        \x20       left_margin: 2ch\n\
        ---\n\n# Doc\n",
    );

    let output = md_cmd()
        .arg(tmp.path())
        .arg("--output")
        .arg("html")
        .arg("--strict-style")
        .env("MD_DRY_RUN", "1")
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "--strict-style must fail on deprecated snake-case alias"
    );
}

#[test]
fn style_strict_style_fails_on_top_level_hr_alias() {
    // Sub-spec #6: `--strict-style` must reject the deprecated top-level
    // `hr:` block, even when its contents would type-check as `HrStyle`.
    let tmp = md_file(
        "---\n\
        hr:\n\
        \x20   style: waves\n\
        ---\n\n# Doc\n\n---\n",
    );

    let output = md_cmd()
        .arg(tmp.path())
        .arg("--output")
        .arg("html")
        .arg("--strict-style")
        .env("MD_DRY_RUN", "1")
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "--strict-style must fail on top-level `hr:` alias"
    );
}

#[test]
fn style_strict_style_fails_on_top_level_hr_alias_with_invalid_field() {
    // `alignment: true` cannot deserialize as `HrAlignment`. The alias
    // presence — not typed migration success — is what must be rejected.
    let tmp = md_file(
        "---\n\
        hr:\n\
        \x20   style: waves\n\
        \x20   alignment: true\n\
        ---\n\n# Doc\n\n---\n",
    );

    let output = md_cmd()
        .arg(tmp.path())
        .arg("--output")
        .arg("html")
        .arg("--strict-style")
        .env("MD_DRY_RUN", "1")
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "--strict-style must fail on top-level `hr:` alias even when the \
         typed migration would drop fields"
    );
}

#[test]
fn style_strict_style_fails_on_non_mapping_top_level_hr() {
    // A scalar `hr: dashes` is still alias usage and must trip strict mode.
    let tmp = md_file(
        "---\n\
        hr: dashes\n\
        ---\n\n# Doc\n\n---\n",
    );

    let output = md_cmd()
        .arg(tmp.path())
        .arg("--output")
        .arg("html")
        .arg("--strict-style")
        .env("MD_DRY_RUN", "1")
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "--strict-style must fail on non-mapping top-level `hr:`"
    );
}

#[test]
fn style_non_strict_renders_with_unknown_key() {
    // Without `--strict-style`, an unknown key must NOT fail the render; it
    // becomes an informational warning. Route through `--output html` to
    // exercise the frontmatter pipeline.
    let tmp = md_file(
        "---\n\
        style:\n\
        \x20   page:\n\
        \x20       made-up-key: 2ch\n\
        ---\n\n# Doc\n",
    );
    let output = md_cmd()
        .arg(tmp.path())
        .arg("--output")
        .arg("html")
        .env("MD_DRY_RUN", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "unknown key without --strict-style must still render: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn style_cli_margin_overrides_frontmatter() {
    // Spec test #2: CLI flag overrides frontmatter. The fixture has
    // `left-margin: 2ch`; `--ml 7` claims that field via
    // `PageStyleOverrides::margin_left = true`, so the CLI value (7) wins.
    let page = {
        // Mirror the cli's parse → apply_cli → apply_style_frontmatter
        // pipeline as in the public API.
        use darkmatter::markdown::Markdown;
        use darkmatter::style::{PageStyleOverrides, apply_page_style, from_frontmatter};
        let raw = std::fs::read_to_string(style_prop_fixture()).unwrap();
        let md = Markdown::try_from_content(&raw).unwrap();
        let (style, _) = from_frontmatter(md.frontmatter()).unwrap();

        let term = Terminal::new_optimistic(80);
        let page = DarkmatterPage::new(&term).with_margin_left(7);
        let overrides = PageStyleOverrides {
            margin_left: true,
            ..PageStyleOverrides::default()
        };
        apply_page_style(page, &style, overrides).expect("apply")
    };
    assert_eq!(
        tv_cells(&page.page_margin().left),
        7,
        "CLI override must win over frontmatter left-margin"
    );
    assert_eq!(
        tv_cells(&page.page_margin().right),
        4,
        "frontmatter right-margin (4ch) must still apply when not claimed"
    );
}

// =============================================================================
//          COMPONENT STYLE FRONTMATTER (Sub-Spec #3) END-TO-END
// =============================================================================
//
// These tests verify that `style.table.*`, `style.images.*`, and
// `style.block-quote.*` frontmatter reaches the page builder, and that
// component-specific CLI flags (`--align-tables`, `--fill-tables`, ...) plus
// the global `--alignment` / `--fill` claim the matching frontmatter fields
// via `ComponentStyleOverrides`.

/// Drive the full CLI integration: parse args, apply CLI layout flags, then
/// apply the `style:` frontmatter (page + component). Returns the resolved
/// page for assertions.
fn apply_style_for(raw: &str, args: &[&str]) -> DarkmatterPage {
    use darkmatter::markdown::Markdown;
    use darkmatter_cli::output::apply_style_frontmatter;
    let cli = parse_cli(args);
    let md = Markdown::try_from_content(raw).unwrap();
    let term = Terminal::new_optimistic(80);
    let page = apply_cli_layout_flags(DarkmatterPage::new(&term), &cli);
    apply_style_frontmatter(page, &md, &cli, None).expect("style apply")
}

#[test]
fn component_overrides_global_alignment_claims_every_bucket() {
    use darkmatter::style::ComponentStyleOverrides;
    use darkmatter_cli::output::component_style_overrides_from_cli;

    let cli = parse_cli(&["doc.md", "--alignment", "center"]);
    let o = component_style_overrides_from_cli(&cli);
    assert_eq!(
        o,
        ComponentStyleOverrides {
            tables_alignment: true,
            images_alignment: true,
            block_quotes_alignment: true,
            tables_fill: false,
            images_fill: false,
            block_quotes_fill: false,
        }
    );
}

#[test]
fn component_overrides_global_fill_claims_every_bucket() {
    use darkmatter::style::ComponentStyleOverrides;
    use darkmatter_cli::output::component_style_overrides_from_cli;

    let cli = parse_cli(&["doc.md", "--fill", "max=60"]);
    let o = component_style_overrides_from_cli(&cli);
    assert_eq!(
        o,
        ComponentStyleOverrides {
            tables_fill: true,
            images_fill: true,
            block_quotes_fill: true,
            tables_alignment: false,
            images_alignment: false,
            block_quotes_alignment: false,
        }
    );
}

#[test]
fn component_overrides_component_specific_alignment_claims_one_bucket() {
    use darkmatter_cli::output::component_style_overrides_from_cli;

    let cli = parse_cli(&["doc.md", "--align-tables", "right"]);
    let o = component_style_overrides_from_cli(&cli);
    assert!(o.tables_alignment);
    assert!(!o.images_alignment);
    assert!(!o.block_quotes_alignment);
    assert!(!o.tables_fill && !o.images_fill && !o.block_quotes_fill);
}

#[test]
fn component_overrides_component_specific_fill_claims_one_bucket() {
    use darkmatter_cli::output::component_style_overrides_from_cli;

    let cli = parse_cli(&["doc.md", "--fill-images", "max=40"]);
    let o = component_style_overrides_from_cli(&cli);
    assert!(o.images_fill);
    assert!(!o.tables_fill && !o.block_quotes_fill);
    assert!(!o.tables_alignment && !o.images_alignment && !o.block_quotes_alignment);
}

#[test]
fn frontmatter_table_alignment_reaches_page_when_no_cli_flag() {
    let raw = "---\n\
style:\n\
\x20   table:\n\
\x20       alignment: left\n\
---\n\n# Doc\n";
    let page = apply_style_for(raw, &["doc.md"]);
    assert_eq!(
        alignment_for(&page, PageComponent::Tables),
        Alignment::Left,
        "frontmatter table.alignment must reach the page when no CLI claim",
    );
}

#[test]
fn cli_align_tables_overrides_frontmatter_table_alignment() {
    // Plan ask: `--align-tables right` overriding frontmatter
    // `style.table.alignment: left`. The CLI flag wins.
    let raw = "---\n\
style:\n\
\x20   table:\n\
\x20       alignment: left\n\
---\n\n# Doc\n";
    let page = apply_style_for(raw, &["doc.md", "--align-tables", "right"]);
    assert_eq!(
        alignment_for(&page, PageComponent::Tables),
        Alignment::Right,
        "--align-tables right must override frontmatter table.alignment: left",
    );
}

#[test]
fn cli_global_fill_overrides_frontmatter_table_max_width() {
    // Plan ask: `--fill max=60` overriding frontmatter
    // `style.table.max-width: 50%` for all components.
    let raw = "---\n\
style:\n\
\x20   table:\n\
\x20       max-width: 50%\n\
---\n\n# Doc\n";
    let page = apply_style_for(raw, &["doc.md", "--fill", "max=60"]);
    assert_eq!(
        fill_for(&page, PageComponent::Tables),
        TestFill::Max(Length::ch(60)),
        "--fill max=60 (global) must claim the table fill slot",
    );
}

#[test]
fn frontmatter_table_max_width_reaches_page_when_no_cli_flag() {
    let raw = "---\n\
style:\n\
\x20   table:\n\
\x20       max-width: 50%\n\
---\n\n# Doc\n";
    let page = apply_style_for(raw, &["doc.md"]);
    assert_eq!(
        fill_for(&page, PageComponent::Tables),
        TestFill::Max(Length::Percent(50.0)),
        "frontmatter table.max-width must reach the page when no CLI claim",
    );
}

#[test]
fn frontmatter_images_alignment_and_fill_reach_page() {
    let raw = "---\n\
style:\n\
\x20   images:\n\
\x20       alignment: center\n\
\x20       max-width: 40ch\n\
---\n\n# Doc\n";
    let page = apply_style_for(raw, &["doc.md"]);
    assert_eq!(
        alignment_for(&page, PageComponent::Images),
        Alignment::Center,
    );
    assert_eq!(
        fill_for(&page, PageComponent::Images),
        TestFill::Max(Length::ch(40)),
    );
}

#[test]
fn frontmatter_block_quote_max_width_reaches_page() {
    let raw = "---\n\
style:\n\
\x20   block-quote:\n\
\x20       max-width: 75%\n\
---\n\n# Doc\n";
    let page = apply_style_for(raw, &["doc.md"]);
    assert_eq!(
        fill_for(&page, PageComponent::BlockQuotes),
        TestFill::Max(Length::Percent(75.0)),
    );
}

#[test]
fn style_fixture_renders_with_align_tables_override() {
    // End-to-end sanity check: the canonical fixture renders successfully
    // when the user overrides table alignment from the CLI.
    let output = md_cmd()
        .arg(style_prop_fixture())
        .arg("--align-tables")
        .arg("center")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "md --align-tables center style-prop.md must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn style_fixture_renders_html_with_fill_override() {
    // End-to-end sanity check: --output html runs the same component-style
    // path as the terminal pipeline.
    let output = md_cmd()
        .arg(style_prop_fixture())
        .arg("--output")
        .arg("html")
        .arg("--fill")
        .arg("max=60")
        .env("MD_DRY_RUN", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "md --output html --fill max=60 must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn style_frontmatter_html_emits_component_layout_css() {
    // Sub-spec #3 acceptance (review-3 finding #3): `md --output html` on a
    // document carrying `style.table.*`, `style.images.*`, and
    // `style.block-quote.*` must emit the matching component layout CSS
    // selectors and declarations, not just succeed silently.
    //
    // Mirrors `darkmatter/lib/src/layout/page.rs::browser_render_with_component_*_css`
    // but drives the assertion through the public CLI surface so the
    // frontmatter → page-style → HTML pipeline is exercised end-to-end.
    let tmp = md_file(
        "---\n\
        style:\n\
        \x20   table:\n\
        \x20       alignment: center\n\
        \x20       max-width: 60ch\n\
        \x20   images:\n\
        \x20       alignment: right\n\
        \x20       max-width: 40ch\n\
        \x20   block-quote:\n\
        \x20       alignment: right\n\
        \x20       max-width: 50ch\n\
        ---\n\n\
        # Doc\n\n\
        | A | B |\n\
        | - | - |\n\
        | 1 | 2 |\n\n\
        ![alt](./x.png)\n\n\
        > quote\n",
    );

    let output = md_cmd()
        .arg(tmp.path())
        .arg("--output")
        .arg("html")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "md --output html must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let html = String::from_utf8(output.stdout).expect("html stdout must be utf-8");

    // Component layout is now emitted as inline `style` attributes by the
    // renderable browser fold (build_component_css was deleted in the cutover).
    // Table: center alignment + max-width: 60ch → margin-left:auto;margin-right:auto.
    assert!(
        html.contains("<table") && html.contains("max-width:60ch") && html.contains("margin-left:auto") && html.contains("margin-right:auto"),
        "expected centered table with inline max-width and auto margins in HTML. html:\n{html}",
    );
    // Block-quote: right alignment + max-width: 50ch → margin-left:auto.
    assert!(
        html.contains("<blockquote") && html.contains("max-width:50ch") && html.contains("margin-left:auto"),
        "expected right-aligned blockquote with inline max-width and auto margin in HTML. html:\n{html}",
    );
    // Image: max-width and alignment are applied to the wrapping paragraph via
    // the lone-image layout path (alignment without max-width does not emit
    // margin styles in the current fold).
    assert!(
        html.contains("<img") && html.contains("src=\"./x.png\""),
        "expected image element in HTML. html:\n{html}",
    );
}

#[test]
fn style_prop_fixture_html_emits_table_layout_css() {
    // Acceptance from sub-spec #3: `md --output html style-prop.md` must emit
    // the expected table layout CSS (right alignment + 50% max-width that
    // lowers to the page-content base, i.e. 50ch when the page builds at
    // its 120-col default for HTML).
    let output = md_cmd()
        .arg(style_prop_fixture())
        .arg("--output")
        .arg("html")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "md --output html style-prop.md must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let html = String::from_utf8(output.stdout).expect("html stdout must be utf-8");

    // Component layout is now emitted as inline `style` attributes by the
    // renderable browser fold (build_component_css was deleted in the cutover).
    // Right alignment + max-width: 50% → margin-left:auto on the table element.
    assert!(
        html.contains("<table") && html.contains("margin-left:auto"),
        "expected right-aligned table with inline auto margin in HTML. html:\n{html}",
    );
    // The fixture sets `max-width: 50%`; the fold preserves the percent on HTML.
    assert!(
        html.contains("max-width:50%"),
        "expected `max-width:50%` declaration in HTML. html:\n{html}",
    );
}

// =============================================================================
//          PHASE 5 REGRESSION TESTS (Sub-Spec #3)
// =============================================================================
//
// These tests cover the Phase 5 acceptance criteria:
//
//   1. `apply_cli_layout_flags` behavior is unchanged for documents without a
//      `style:` frontmatter (no silent state drift from the new
//      `apply_component_style` integration).
//   2. The canonical `style-prop.md` fixture resolves to the structural
//      page state the spec promises (table right-aligned, capped at 50%).
//   3. Component fill from `style.block-quote.max-width` flows into the
//      page builder and matches the structural shape used by the terminal
//      blockquote renderer.

#[test]
fn no_style_frontmatter_leaves_cli_layout_state_intact() {
    // Phase 5 acceptance: documents without a `style:` block must observe the
    // same resolved page state with vs. without `apply_style_frontmatter`.
    // Guards against silent state drift from the component-style integration.
    use darkmatter::markdown::Markdown;
    use darkmatter_cli::output::apply_style_frontmatter;

    let raw = "# No Style Doc\n\nBody.\n";
    let md = Markdown::try_from_content(raw).unwrap();
    let cli = parse_cli(&[
        "doc.md",
        "-m",
        "3",
        "--max-width",
        "70",
        "--alignment",
        "center",
        "--fill",
        "max=50",
    ]);

    let term = Terminal::new_optimistic(120);
    let cli_only = apply_cli_layout_flags(DarkmatterPage::new(&term), &cli);
    let after_style =
        apply_style_frontmatter(cli_only.clone(), &md, &cli, None).expect("style apply");

    assert_eq!(
        after_style.page_margin(),
        cli_only.page_margin(),
        "no `style:` frontmatter must leave CLI-resolved margins untouched",
    );
    assert_eq!(
        after_style.page_padding(),
        cli_only.page_padding(),
        "no `style:` frontmatter must leave CLI-resolved padding untouched",
    );
    assert_eq!(
        after_style.max_width(),
        cli_only.max_width(),
        "no `style:` frontmatter must leave CLI-resolved max-width untouched",
    );
    for component in PageComponent::ALL {
        assert_eq!(
            alignment_for(&after_style, component),
            alignment_for(&cli_only, component),
            "no `style:` frontmatter must leave CLI-resolved alignment untouched: {component:?}",
        );
        assert_eq!(
            fill_for(&after_style, component),
            fill_for(&cli_only, component),
            "no `style:` frontmatter must leave CLI-resolved fill untouched: {component:?}",
        );
    }
}

#[test]
fn style_prop_fixture_resolves_to_expected_table_layout() {
    // Phase 5 acceptance: the canonical `style-prop.md` fixture must produce
    // a page where the table is right-aligned and capped at 50% max-width via
    // the new component-style apply path.
    let raw = std::fs::read_to_string(style_prop_fixture()).unwrap();
    let page = apply_style_for(&raw, &["doc.md"]);

    assert_eq!(
        alignment_for(&page, PageComponent::Tables),
        Alignment::Right,
        "fixture must resolve to a right-aligned table",
    );
    assert_eq!(
        fill_for(&page, PageComponent::Tables),
        TestFill::Max(Length::Percent(50.0)),
        "fixture must cap the table at 50% max-width",
    );
}

#[test]
fn style_prop_fixture_resolves_to_expected_page_margins() {
    // Phase 5 acceptance: page-level margins from the fixture survive the
    // full CLI -> page-style -> component-style pipeline.
    let raw = std::fs::read_to_string(style_prop_fixture()).unwrap();
    let page = apply_style_for(&raw, &["doc.md"]);

    let m = page.page_margin();
    assert_eq!(tv_cells(&m.left), 2, "fixture left-margin: 2ch must reach the page");
    assert_eq!(tv_cells(&m.right), 4, "fixture right-margin: 4ch must reach the page");
    assert_eq!(tv_cells(&m.top), 1, "fixture top-margin: 1 must reach the page");
    assert_eq!(tv_cells(&m.bottom), 0, "fixture bottom-margin: 0 must reach the page");
}

#[test]
fn block_quote_max_width_caps_terminal_render_wrap_width() {
    // Phase 5 acceptance: `style.block-quote.max-width` reaches the page and
    // caps visible wrap width when the terminal renders a top-level
    // blockquote. We use a 100-col terminal so 50% resolves cleanly, then
    // assert that no rendered (ANSI-stripped) blockquote line exceeds the
    // resolved fill width.
    use darkmatter::layout::{DarkmatterPage, PageComponent};
    use darkmatter::markdown::Markdown;
    use darkmatter::testing::strip_ansi_codes;
    use darkmatter_cli::output::apply_style_frontmatter;

    let raw = "---\n\
style:\n\
\x20   block-quote:\n\
\x20       max-width: 50%\n\
---\n\n\
> This is a long quoted paragraph intended to wrap onto multiple visible \
rows once the blockquote fill caps the render width well below the page width. \
Add filler text to guarantee the wrap point is reached even when the \
terminal is reasonably wide.\n";

    let cli = parse_cli(&["doc.md"]);
    let md = Markdown::try_from_content(raw).unwrap();
    let term = Terminal::new_optimistic(100);
    let page = DarkmatterPage::new(&term);
    let page = apply_cli_layout_flags(page, &cli);
    let page = apply_style_frontmatter(page, &md, &cli, None).expect("style apply");

    // Structural guard: the apply pipeline put the fill where the renderer
    // will look for it.
    assert_eq!(
        fill_for(&page, PageComponent::BlockQuotes),
        TestFill::Max(Length::Percent(50.0)),
        "block-quote.max-width must reach the page fill slot",
    );

    let rendered = page.render(&md).expect("render to terminal");
    let plain = strip_ansi_codes(&rendered);

    // The blockquote should wrap onto at least two visible lines.
    let quote_lines: Vec<String> = plain
        .lines()
        .filter(|l| {
            let trimmed = l.trim_start();
            // Common blockquote indicators across themes (`│`, `▌`, `▐`).
            trimmed.starts_with('│')
                || trimmed.starts_with('▌')
                || trimmed.starts_with('▐')
        })
        .map(|l| l.trim_end().to_string())
        .collect();

    assert!(
        quote_lines.len() >= 2,
        "blockquote should wrap onto >=2 visible lines under max-width: 50%. plain:\n{plain}",
    );

    // The renderer chooses 50% of the available content width; with a 100-col
    // terminal and no other margins/padding that's at most 50 cells of *text*
    // beyond the indicator + leading space. Allow generous slack for indent +
    // alignment padding by upper-bounding total visible width at 60.
    let max_len = quote_lines.iter().map(|l| l.chars().count()).max().unwrap();
    assert!(
        max_len <= 60,
        "blockquote visible width should be capped under max-width: 50% on a 100-col terminal, got max={max_len}. plain:\n{plain}",
    );
}

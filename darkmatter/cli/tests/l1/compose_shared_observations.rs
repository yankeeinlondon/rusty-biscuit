//! `md compose` observes a URL once per run (L1): the reference check and
//! composition share one remote-fetch runtime and one data memo, and so does
//! the passive composition a combined failure report runs after a reference
//! stop. A loopback responder counts every request under
//! `permissions.network: allow`; the frontmatter reads the URL and the body
//! shows the value.

use std::process::Output;

use serde_json::Value;

use crate::common::{self, CliProcessFixture, MockHttpResponse};

/// `md compose doc.md --output json` with `body` after a frontmatter that
/// reads the responder: the output and the number of requests the
/// responder received.
fn compose(name: &str, body: &str) -> (Output, usize, String) {
    let fixture = CliProcessFixture::named(name);
    std::fs::write(fixture.home().join(".darkmatter.yaml"), "permissions:\n  network: allow\n").unwrap();
    let responses = (0..3).map(|_| MockHttpResponse { status: 200, body: "[\"a\", \"b\"]", cache_control: None }).collect();
    let server = common::mock_http_server(responses);
    let url = server.url("/data.json");
    fixture.write_file("cwd/doc.md", &format!("---\nitems: \"{{{{ load_data('{url}', 'string[]') }}}}\"\n---\nItems {{{{ items }}}}.\n{body}"));
    let output = fixture.command().args(["compose", "doc.md", "--output", "json"]).output().expect("md runs");
    let requests = server.request_count();
    server.shutdown();
    (output, requests, url)
}

fn envelope(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stdout)))
}

/// R2-F5.8: a clean run requests the URL once and lists it once.
#[test]
fn a_clean_run_requests_a_frontmatter_url_once() {
    let (output, requests, url) = compose("compose-shared-clean", "");
    let value = envelope(&output);
    assert!(output.status.success(), "{value:#}");
    assert!(value["document"].as_str().is_some_and(|text| text.contains("Items a, b.") || text.contains("Items [")), "{value:#}");
    assert_eq!(requests, 1, "{value:#}");
    let listed = value["read_set"].as_array().unwrap().iter().filter(|entry| entry.to_string().contains(&url)).count();
    assert_eq!(listed, 1, "{value:#}");
}

/// R2-F5.8: a missing local inclusion stops the run with the combined
/// report, whose passive composition reads the URL the reference check
/// already fetched.
#[test]
fn a_combined_failure_report_requests_a_frontmatter_url_once() {
    let (output, requests, _) = compose("compose-shared-failure", "\n::file ./missing.md\n");
    let value = envelope(&output);
    assert_eq!(output.status.code(), Some(1), "{value:#}");
    assert!(value["diagnostics"].to_string().contains("missing.md"), "{value:#}");
    assert_eq!(requests, 1, "{value:#}");
}

/// R3-F4.5: a good local inclusion beside an independent missing one and a
/// failing expression is opened once by the reference check and the
/// combined report's passive composition, which keeps the missing-reference
/// row. The `test-fixtures` seam `MD_TEST_SOURCE_OPENS` logs every open of
/// the shared reader.
#[cfg(feature = "test-fixtures")]
#[test]
fn a_combined_failure_report_opens_a_good_local_inclusion_once() {
    let fixture = CliProcessFixture::named("compose-shared-local");
    fixture.write_file("cwd/inc.md", "Included.\n");
    fixture.write_file("cwd/doc.md", "---\ntitle: t\n---\nGood.\n\n::file ./inc.md\n\n::file ./missing.md\n\n{{ 3 / 0 }}\n");
    let log = fixture.home().join("opens.log");

    let output = fixture
        .command_builder()
        .application_input("MD_TEST_SOURCE_OPENS", &log)
        .build()
        .args(["compose", "doc.md", "--output", "json"])
        .output()
        .expect("md runs");

    let value = envelope(&output);
    assert_eq!(output.status.code(), Some(1), "{value:#}");
    let diagnostics = value["diagnostics"].to_string();
    assert!(diagnostics.contains("dm.reference.missing_transclusion") && diagnostics.contains("missing.md"), "{value:#}");
    let opened = std::fs::read_to_string(&log).unwrap_or_default();
    let inclusion = opened.lines().filter(|line| std::path::Path::new(line).file_name().is_some_and(|name| name == "inc.md")).count();
    assert_eq!(inclusion, 1, "{opened}\n{value:#}");
}

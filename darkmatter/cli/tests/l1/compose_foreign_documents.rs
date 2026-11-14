//! `md compose` of a document a registered host identifies (a foreign
//! document): `md` validates it against the host's schemas, composes its
//! body, and runs only the lifecycle events it shares with the host, with
//! its own bindings. The host's globals are declared for analysis and fail
//! when read; its own events, renamed spellings, and functions never run.
//! One `dm.host.foreign_document` notice reports this per invocation: on
//! stderr for a formatted output, in `diagnostics` for a data output.
//!
//! The `relay` host is shaped like Claudine (`composed` → `start`, `failed`
//! → `blocked`, `success`/`failure` of its own, a `seq` global); it is
//! installed in the fixture home and registered there. No agent, speech, or
//! notification is ever launched: `md` runs none of them.

use crate::common;
use crate::host_fixtures::relay_files;

use common::CliProcessFixture;
use serde_json::Value;
use simplified_schema::SimplifiedSchema;
use std::process::Output;

const NOTICE: &str = "dm.host.foreign_document";
const UNDECLARED: &str = "dm.expression.undeclared_property";

/// A fixture whose home registers the relay host, installed under
/// `home/.relay/`.
fn registered(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    for (file, text) in relay_files() {
        fixture.write_file(format!("home/.relay/{file}"), &text);
    }
    fixture.write_file("home/.darkmatter.yaml", "register:\n  - ./.relay/darkmatter-host.yaml\n");
    fixture
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> Output {
    fixture.command().args(args).output().expect("md runs")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Stdout's one compose envelope, judged against its schema, with stderr
/// empty.
fn envelope(output: &Output, schema_text: &str) -> Value {
    assert!(output.stderr.is_empty(), "a data output owns stderr too: {}", stderr_of(output));
    let value: Value = serde_json::from_slice(&output.stdout).expect("one JSON value");
    let schema = SimplifiedSchema::try_from(schema_text).expect("the schema file parses");
    if let Err(failure) = schema.validate_strict(&value) {
        panic!("{failure:?}\n{value:#}");
    }
    value
}

fn records<'v>(value: &'v Value, code: &str) -> Vec<&'v Value> {
    value["diagnostics"].as_array().expect("diagnostics").iter().filter(|record| record["code"] == code).collect()
}

/// `(stage, text)` of every collected lifecycle line.
fn lifecycle_lines(value: &Value) -> Vec<(String, String)> {
    value["lifecycle_output"]
        .as_array()
        .map(|lines| {
            lines
                .iter()
                .map(|line| (line["stage"].as_str().unwrap().to_string(), line["text"].as_str().unwrap().to_string()))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_foreign_document_composes_its_body_and_runs_only_the_shared_events() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("foreign-shared-events");
    fixture.write_file(
        "cwd/relay.md",
        "---\nsequence:\n  - name: one\n    prompt: hi\ntitle: Relay\ninitialize:\n  - info: \"initialize {{ title }}\"\n  - set: { greeting: hello }\nstart:\n  - info: start ran\nblocked:\n  - info: blocked ran\nsuccess:\n  - info: success ran\nfailure:\n  - info: failure ran\nfinalize:\n  - info: \"finalize {{ greeting }}\"\n---\n# {{ title }}\n\nSaid {{ greeting }}.\n",
    );

    let output = run(&fixture, &["compose", "relay.md", "--output", "json"]);

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stdout));
    let value = envelope(&output, include_str!("../../schemas/output/compose.yaml"));
    assert_eq!(value["document"], "# Relay\n\nSaid hello.", "{value:#}");
    assert_eq!(
        lifecycle_lines(&value),
        [
            ("initialize".to_string(), "initialize Relay".to_string()),
            ("finalize".to_string(), "finalize hello".to_string()),
        ],
        "only the shared events ran, with md's bindings: {value:#}"
    );
    assert_eq!(records(&value, NOTICE).len(), 1, "{value:#}");
    assert_eq!(records(&value, NOTICE)[0]["severity"], "info");
    assert!(records(&value, NOTICE)[0]["message"].as_str().unwrap().contains("`relay compose`"), "{value:#}");
}

#[test]
fn the_motivating_document_is_clean_in_analysis_and_fails_normally_reading_seq() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("foreign-motivating");
    // The motivating document, plus an `info` that reads `seq` (`md` skips
    // `say`, so only a verb `md` runs can reach the read).
    fixture.write_file(
        "cwd/motivating.md",
        "---\nsequence:\n  - name: one\n    prompt: hi\ninitialize:\n  - say: \"{{ seq.name }}\"\n  - info: \"{{ seq.name }}\"\nstart:\n  - say: \"{{ seq.name }}\"\n---\n# Body\n",
    );

    let output = run(&fixture, &["compose", "motivating.md", "--output", "json"]);

    assert_eq!(output.status.code(), Some(1), "{}", String::from_utf8_lossy(&output.stdout));
    let value = envelope(&output, include_str!("../../schemas/output/compose.yaml"));
    assert_eq!(value["outcome"], "failure", "{value:#}");
    assert!(value["document"].is_null(), "nothing is written: {value:#}");
    assert!(records(&value, UNDECLARED).is_empty(), "`seq` is declared by the host: {value:#}");
    let text = value["diagnostics"].to_string();
    assert!(
        text.contains("reads the global `seq`, which is unavailable") && text.contains("dm.host.runtime_only"),
        "the read fails as an unavailable global: {value:#}"
    );
    assert_eq!(
        lifecycle_lines(&value),
        [("initialize".to_string(), "`md` does not run `say`; skipped it in `initialize`".to_string())],
        "`say` is skipped, `start` never runs: {value:#}"
    );
}

#[test]
fn the_host_schema_rejects_a_wrong_type_before_the_document_is_written() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("foreign-schema");
    fixture.write_file("cwd/timeout.md", "---\nsequence: []\ntimeout: [1, 2]\nfinalize:\n  - info: finalize ran\n---\n# Body\n");

    let output = run(&fixture, &["compose", "timeout.md"]);

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("timeout") && stderr.contains("string"), "the host's `timeout: string` judges it: {stderr}");
    assert!(stderr.contains("finalize ran"), "the shared `finalize` still runs after the failure: {stderr}");
    assert!(output.stdout.is_empty(), "no document is written");
}

#[test]
fn composed_and_failed_are_rejected_by_the_host_schema_never_run_as_md_stacks() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("foreign-composed-failed");
    fixture.write_file(
        "cwd/relay.md",
        "---\nsequence: []\ncomposed:\n  - info: composed ran\nfailed:\n  - info: failed ran\n---\n# Body\n",
    );

    let output = run(&fixture, &["compose", "relay.md", "--output", "json"]);

    assert_eq!(output.status.code(), Some(1), "{}", String::from_utf8_lossy(&output.stdout));
    let value = envelope(&output, include_str!("../../schemas/output/compose.yaml"));
    let mismatches: Vec<u64> = records(&value, "dm.schema.type_mismatch")
        .iter()
        .filter(|record| record["message"].as_str().unwrap().contains("is not of type \"null\""))
        .map(|record| record["source"]["line"].as_u64().unwrap())
        .collect();
    assert_eq!(mismatches, [4, 6], "the host schema rejects `composed` and `failed`: {value:#}");
    assert!(!value["diagnostics"].to_string().contains("renamed"), "no md lifecycle collision is reported: {value:#}");
    assert!(lifecycle_lines(&value).is_empty(), "neither key runs: {value:#}");
}

#[test]
fn host_only_stacks_and_host_functions_never_run() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("foreign-functions");
    fixture.write_file(
        "cwd/functions.md",
        "---\nstart:\n  - info: \"{{ acme_label('start') }}\"\nsuccess:\n  - shell: echo never\n---\n# {{ acme_label('body') }}\n",
    );

    let output = run(&fixture, &["compose", "functions.md", "--output", "json"]);

    assert_eq!(output.status.code(), Some(1), "{}", String::from_utf8_lossy(&output.stdout));
    let value = envelope(&output, include_str!("../../schemas/output/compose.yaml"));
    let text = value["diagnostics"].to_string();
    assert!(
        text.contains("acme_label() has no implementation in this process"),
        "the host function is typed but has no handler: {value:#}"
    );
    assert!(lifecycle_lines(&value).is_empty(), "no host stack ran: {value:#}");
}

#[test]
fn a_formatted_run_prints_the_notice_once_on_stderr() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("foreign-notice-formatted");
    fixture.write_file("cwd/relay.md", "---\nsequence: []\n---\n# Relay\n\n::file ./child.md\n");
    fixture.write_file("cwd/child.md", "---\nstart: []\n---\nChild body.\n");

    let output = run(&fixture, &["compose", "relay.md"]);

    let stderr = stderr_of(&output);
    assert!(output.status.success(), "{stderr}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("Child body."));
    assert_eq!(stderr.matches("`relay compose`").count(), 1, "one notice for the request: {stderr}");
}

#[test]
fn a_data_run_carries_the_notice_once_with_a_foreign_child() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("foreign-notice-data");
    fixture.write_file("cwd/relay.md", "---\nsequence: []\n---\n# Relay\n\n::file ./child.md\n");
    fixture.write_file("cwd/child.md", "---\nstart: []\n---\nChild body.\n");

    let output = run(&fixture, &["compose", "relay.md", "--output", "json"]);

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stdout));
    let value = envelope(&output, include_str!("../../schemas/output/compose.yaml"));
    assert_eq!(records(&value, NOTICE).len(), 1, "{value:#}");
    assert!(value["document"].as_str().unwrap().contains("Child body."), "{value:#}");
}

#[test]
fn an_md_document_with_shared_events_alone_is_not_foreign() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("foreign-md-document");
    fixture.write_file(
        "cwd/notes.md",
        "---\ninitialize:\n  - info: initialize ran\ncomposed:\n  - info: composed ran\n---\n# Notes\n\n::file ./child.md\n",
    );
    fixture.write_file("cwd/child.md", "---\nsequence: []\n---\nChild body.\n");

    let output = run(&fixture, &["compose", "notes.md", "--output", "json"]);

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stdout));
    let value = envelope(&output, include_str!("../../schemas/output/compose.yaml"));
    assert!(records(&value, NOTICE).is_empty(), "an md root gets no notice, whatever it includes: {value:#}");
    let stages: Vec<String> = lifecycle_lines(&value).into_iter().map(|(stage, _)| stage).collect();
    assert_eq!(stages, ["initialize", "composed"], "md runs its own `composed`: {value:#}");
}

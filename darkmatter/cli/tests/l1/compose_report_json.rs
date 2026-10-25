//! `md compose --output report-json` end to end (criteria 19, 22, 28, 29, 39):
//! every parsed invocation writes exactly one JSON object to stdout, with
//! `outcome`, every diagnostic with all six fields, and the document only on
//! success; failures exit non-zero with `document: null` and nothing on
//! stderr that repeats the report. Other commands reject the format before
//! any work.

use crate::common;

use common::CliProcessFixture;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest, RequestSnapshot};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use test_toolkit::argv_recorder::ARGV_RECORDER_FILE;

/// The report on stdout: exactly one JSON object and nothing else.
fn report_of(output: &std::process::Output) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut stream = serde_json::Deserializer::from_str(&stdout).into_iter::<Value>();
    let report = stream
        .next()
        .unwrap_or_else(|| panic!("no report on stdout; stderr: {}", String::from_utf8_lossy(&output.stderr)))
        .unwrap_or_else(|error| panic!("stdout is not JSON ({error}): {stdout}"));
    assert!(stream.next().is_none(), "one object only: {stdout}");
    let keys: Vec<&str> = report.as_object().expect("an object").keys().map(String::as_str).collect();
    assert_eq!(keys, ["outcome", "diagnostics", "document"], "{report:#}");
    for diagnostic in report["diagnostics"].as_array().expect("an array") {
        let keys: Vec<&str> = diagnostic.as_object().expect("an object").keys().map(String::as_str).collect();
        assert_eq!(keys, ["code", "severity", "message", "source", "expected_type", "actual_type"], "{diagnostic:#}");
        if let Some(source) = diagnostic["source"].as_object() {
            let keys: Vec<&str> = source.keys().map(String::as_str).collect();
            assert_eq!(keys, ["file", "line", "column"], "{diagnostic:#}");
        }
    }
    report
}

fn run_report(fixture: &CliProcessFixture, document: &Path) -> std::process::Output {
    fixture.command().args(["compose", "--output", "report-json"]).arg(document).output().unwrap()
}

fn codes(report: &Value) -> Vec<&str> {
    report["diagnostics"].as_array().unwrap().iter().map(|diagnostic| diagnostic["code"].as_str().unwrap()).collect()
}

/// How `md` names a transcluded document: by its resolved path. The root is
/// named by its argument rejoined segment by segment, never canonicalized, so
/// it matches [`given`] only for a path spelled with the platform separator:
/// write the root through [`cwd_file`].
fn resolved(path: &Path) -> String {
    biscuit_file::canonicalize_simplified(path).unwrap().display().to_string()
}

fn given(path: &Path) -> String {
    path.display().to_string()
}

/// `name` under the fixture `cwd`, joined with the platform separator; a
/// literal `"cwd/{name}"` keeps its `/` on Windows, where `md` reports `\`.
fn cwd_file(name: &str) -> PathBuf {
    Path::new("cwd").join(name)
}

const CATEGORY_3: &str = "---\n$schema:\n  label: string\nlabel: \"7\"\n---\nBody {{ min(label, 5) }}\n";
const CATEGORY_5: &str = "---\n$schema:\n  flag: boolean(required)\nflag: true\n---\nBody {{ min(flag, 5) }}\n";

#[test]
fn success_reports_the_document_and_leaves_other_outputs_unchanged() {
    let fixture = CliProcessFixture::named("report_json_success");
    let document = fixture.write_file("cwd/doc.md", "---\ntitle: Notes\n---\n# {{ title }}\n");

    let output = run_report(&fixture, &document);
    assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(report_of(&output), json!({ "outcome": "success", "diagnostics": [], "document": "# Notes" }));
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));

    // Human Markdown and the AST JSON keep their own formats.
    fixture.command().arg("compose").arg(&document).assert().success().stdout("# Notes\n");
    let ast = fixture.command().args(["compose", "--output", "json"]).arg(&document).output().unwrap();
    let ast: Value = serde_json::from_slice(&ast.stdout).expect("AST JSON");
    assert!(ast.get("outcome").is_none(), "{ast:#}");
}

#[test]
fn a_category_3_warning_is_reported_once_with_every_field() {
    let fixture = CliProcessFixture::named("report_json_warning");
    let document = fixture.write_file(cwd_file("doc.md"), CATEGORY_3);

    let output = run_report(&fixture, &document);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    let report = report_of(&output);
    assert_eq!(report["outcome"], "success");
    assert_eq!(report["document"], "Body 5");
    assert_eq!(
        report["diagnostics"][0],
        json!({
            "code": "dm.expression.uncertain_call",
            "severity": "warning",
            "message": report["diagnostics"][0]["message"],
            "source": { "file": given(&document), "line": 6, "column": 13 },
            "expected_type": "number",
            "actual_type": "string | null",
        })
    );
    assert_eq!(codes(&report).len(), 1, "{report:#}");
    assert!(!stderr.contains("category 3"), "no human footer repeats the report: {stderr}");
}

/// Criterion 19's report leg: the rejected call's record is the library's,
/// field for field (DMLS's leg is `expression_call_diagnostics::shared_fields`).
#[test]
fn a_rejected_call_reports_the_shared_record() {
    let fixture = CliProcessFixture::named("report_json_rejected");
    let document = fixture.write_file(cwd_file("doc.md"), CATEGORY_5);

    let output = run_report(&fixture, &document);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    let report = report_of(&output);

    let dir = document.parent().unwrap();
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, "")).with_source_file(&document);
    let request = ComposeRequest::prepare(options, &RequestSnapshot::new(dir)).unwrap();
    let error = Markdown::try_from(document.as_path()).unwrap().compose_with(&request).expect_err("rejected");
    let expected = serde_json::to_value(error_diagnostics(&error)).unwrap();

    assert_eq!(report, json!({ "outcome": "failure", "diagnostics": expected, "document": null }));
    assert_eq!(report["diagnostics"][0]["code"], "dm.expression.incompatible_call");
    assert_eq!(report["diagnostics"][0]["expected_type"], "number");
    assert_eq!(report["diagnostics"][0]["actual_type"], "boolean");
    assert_eq!(report["diagnostics"][0]["source"], json!({ "file": given(&document), "line": 6, "column": 13 }));
}

/// The lifecycle leg of criterion 19: a lifecycle rejection reports the
/// library's located record (DMLS's leg, one test per surface, is
/// `expression_call_diagnostics::shared_fields`), and no effect launches.
#[test]
fn a_rejected_lifecycle_call_reports_the_shared_record() {
    use darkmatter::lifecycle::md_host;

    let fixture = CliProcessFixture::named("report_json_lifecycle_rejected");
    let document = fixture.write_file(
        cwd_file("doc.md"),
        "---\ncomposed:\n  - ensure_file: [\"marker\", \"{{ min(true,5) }}\"]\n---\nBody\n",
    );

    let output = run_report(&fixture, &document);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    let report = report_of(&output);

    let markdown = Markdown::try_from(document.as_path()).unwrap();
    let frontmatter: Value =
        markdown.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect();
    let config = md_host::parse_lifecycle(&frontmatter, &document, None).expect("the lifecycle parses");
    let error = darkmatter::lifecycle::reject_incompatible_calls(
        &frontmatter,
        &config,
        &document,
        markdown.loaded_text(),
        md_host::registry(),
        None,
    )
    .expect_err("rejected");
    let expected = serde_json::to_value(error_diagnostics(&error)).unwrap();

    assert_eq!(report, json!({ "outcome": "failure", "diagnostics": expected, "document": null }));
    assert_eq!(report["diagnostics"][0]["code"], "dm.expression.incompatible_call");
    assert_eq!(report["diagnostics"][0]["expected_type"], "number");
    assert_eq!(report["diagnostics"][0]["actual_type"], "literal true");
    assert_eq!(report["diagnostics"][0]["source"], json!({ "file": given(&document), "line": 3, "column": 37 }));
    assert!(!document.with_file_name("marker").exists(), "no lifecycle effect ran");
}

#[test]
fn an_unreadable_input_is_one_failure_with_null_fields() {
    let fixture = CliProcessFixture::named("report_json_input");
    let output = fixture.command().args(["compose", "--output", "report-json", "missing.md"]).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report = report_of(&output);
    assert_eq!(report["outcome"], "failure");
    assert_eq!(report["document"], Value::Null);
    let diagnostic = &report["diagnostics"][0];
    assert_eq!(diagnostic["code"], "dm.input.read_failure");
    assert_eq!(diagnostic["severity"], "error");
    assert!(diagnostic["message"].as_str().unwrap().contains("missing.md"), "{diagnostic:#}");
    for field in ["source", "expected_type", "actual_type"] {
        assert_eq!(diagnostic[field], Value::Null, "{field}");
    }
}

#[test]
fn schema_and_parse_failures_are_located_in_the_frontmatter() {
    let fixture = CliProcessFixture::named("report_json_schema");
    for (name, content, code, line, column) in [
        ("schema.md", "---\ncount: hello\n$schema:\n  count: number(required)\n---\nbody\n", "dm.schema.type_mismatch", 2, 8),
        ("yaml.md", "---\ntitle: [unclosed\n---\nbody\n", "dm.frontmatter.yaml_parse", 3, 1),
    ] {
        let document = fixture.write_file(cwd_file(name), content);
        let output = run_report(&fixture, &document);
        assert_eq!(output.status.code(), Some(1), "{name}");
        assert!(output.stderr.is_empty(), "{name}: {}", String::from_utf8_lossy(&output.stderr));
        let report = report_of(&output);
        assert_eq!(report["document"], Value::Null, "{name}");
        assert_eq!(codes(&report), [code], "{name}: {report:#}");
        assert_eq!(report["diagnostics"][0]["source"], json!({ "file": given(&document), "line": line, "column": column }), "{name}");
    }
}

/// A warning reported before a runtime failure stays in the failure report,
/// ahead of the failure.
#[test]
fn an_earlier_warning_survives_a_runtime_failure() {
    let fixture = CliProcessFixture::named("report_json_runtime");
    let document = fixture.write_file(cwd_file("doc.md"), &format!("{CATEGORY_3}\n{{{{ 1 / 0 }}}}\n"));

    let output = run_report(&fixture, &document);
    assert_eq!(output.status.code(), Some(1));
    let report = report_of(&output);
    assert_eq!(report["document"], Value::Null);
    assert_eq!(codes(&report), ["dm.expression.uncertain_call", "dm.expression.evaluation_failure"], "{report:#}");
    assert_eq!(report["diagnostics"][1]["source"], json!({ "file": given(&document), "line": 8, "column": 1 }));
    assert_eq!(report["diagnostics"][1]["expected_type"], Value::Null);
}

/// `(code, line)` of every diagnostic, in report order.
fn code_lines(report: &Value) -> Vec<(&str, Option<u64>)> {
    report["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|diagnostic| (diagnostic["code"].as_str().unwrap(), diagnostic["source"]["line"].as_u64()))
        .collect()
}

/// A warning found before a rejected call stays in the failure report,
/// ahead of the rejection; a successful report lists passive warnings in
/// authored order, whichever surface holds them; a lifecycle rejection
/// keeps the lifecycle warnings before it.
#[test]
fn passive_warnings_keep_their_authored_order_through_a_rejection() {
    let fixture = CliProcessFixture::named("report_json_passive_history");
    for (name, content, outcome, expected) in [
        (
            "rejected.md",
            "{{ file_exists(\"\") }}\n\n{{ min(true, 5) }}\n",
            "failure",
            vec![("dm.expression.always_false_call", Some(1)), ("dm.expression.incompatible_call", Some(3))],
        ),
        (
            "block_then_rejected.md",
            "::block when='file_exists(\"\")'\nx\n::end-block\n\n{{ min(true, 5) }}\n",
            "failure",
            vec![("dm.expression.always_false_call", Some(1)), ("dm.expression.incompatible_call", Some(5))],
        ),
        (
            "ordered.md",
            "{{ file_exists(\"\") }}\n\n::block when='file_exists(\"\")'\nx\n::end-block\n",
            "success",
            vec![("dm.expression.always_false_call", Some(1)), ("dm.expression.always_false_call", Some(3))],
        ),
        (
            "lifecycle.md",
            "---\ncomposed:\n  - stdout: '{{ file_exists(\"\") }}'\n  - stdout: '{{ min(true, 5) }}'\n---\nBody\n",
            "failure",
            vec![("dm.expression.always_false_call", Some(3)), ("dm.expression.incompatible_call", Some(4))],
        ),
    ] {
        let document = fixture.write_file(format!("cwd/{name}"), content);
        let output = run_report(&fixture, &document);
        let report = report_of(&output);
        assert_eq!(report["outcome"], outcome, "{name}: {report:#}");
        assert_eq!(code_lines(&report), expected, "{name}: {report:#}");
    }
}

/// Positions in transcluded content name the child file and its own line.
#[test]
fn transcluded_findings_are_located_in_the_child() {
    let fixture = CliProcessFixture::named("report_json_transcluded");
    let parent = fixture.write_file("cwd/parent.md", "# Parent\n\n::file ./child.md\n");
    for (content, outcome, code) in
        [(CATEGORY_3, "success", "dm.expression.uncertain_call"), (CATEGORY_5, "failure", "dm.expression.incompatible_call")]
    {
        let child = fixture.write_file("cwd/child.md", content);
        let report = report_of(&run_report(&fixture, &parent));
        assert_eq!(report["outcome"], outcome, "{report:#}");
        assert_eq!(codes(&report), [code], "{report:#}");
        assert_eq!(report["diagnostics"][0]["source"], json!({ "file": resolved(&child), "line": 6, "column": 13 }), "{report:#}");
    }
}

/// A child rejected during the pre-flight walk, which `md compose` runs
/// before composing, reports what the compose pass would: every warning of
/// the parent (its passive check covers the whole parent before any child
/// composes), then the child's earlier warning and its rejection, each at
/// its authored position in its own file.
#[test]
fn a_rejected_child_keeps_its_parent_warnings_first() {
    let fixture = CliProcessFixture::named("report_json_transcluded_history");
    let child = fixture.write_file("cwd/child.md", "{{ file_exists(\"https://\") }}\n\n{{ min(true, 5) }}\n");
    let body = "{{ file_exists(\"\") }}\n\n::file ./child.md\n\n{{ file_exists(\"http://\") }}\n";
    // The plain route, and the lifecycle route, whose pre-flight walk is the
    // same; its four frontmatter lines move the parent's body down.
    for (name, frontmatter, offset) in [("plain.md", "", 0), ("lifecycle.md", "---\ncomposed:\n  - stdout: done\n---\n", 4)] {
        let parent = fixture.write_file(cwd_file(name), &format!("{frontmatter}{body}"));
        let output = run_report(&fixture, &parent);
        assert_eq!(output.status.code(), Some(1), "{name}");
        let report = report_of(&output);
        assert_eq!(report["outcome"], "failure", "{name}: {report:#}");
        let records: Vec<(&str, &str, u64)> = report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic["code"].as_str().unwrap(),
                    diagnostic["source"]["file"].as_str().unwrap(),
                    diagnostic["source"]["line"].as_u64().unwrap(),
                )
            })
            .collect();
        let (parent, child) = (given(&parent), resolved(&child));
        assert_eq!(
            records,
            [
                ("dm.expression.always_false_call", parent.as_str(), 1 + offset),
                ("dm.expression.always_false_call", parent.as_str(), 5 + offset),
                ("dm.expression.always_false_call", child.as_str(), 1),
                ("dm.expression.incompatible_call", child.as_str(), 3),
            ],
            "{name}: {report:#}"
        );
    }
}

/// A policy stop reports each warning once, keeping its severity; the
/// command fails with no document (criterion 28's report leg).
#[test]
fn deny_all_reports_the_blocking_warning_once_and_no_document() {
    let fixture = CliProcessFixture::named("report_json_denied");
    let lifecycle = "---\n$schema:\n  label: string\nlabel: \"7\"\ncomposed:\n  - stdout: done\n---\nBody {{ min(label, 5) }}\n";
    for (name, content) in [("plain.md", CATEGORY_3), ("lifecycle.md", lifecycle)] {
        let document = fixture.write_file(format!("cwd/{name}"), content);
        let output = fixture
            .command_builder()
            // A regression that launched a viewer would skip it.
            .application_input("MD_DRY_RUN", "1")
            .build()
            .args(["compose", "--output", "report-json", "--deny", "all"])
            .arg(&document)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{name}");
        let report = report_of(&output);
        assert_eq!(report["outcome"], "failure", "{name}");
        assert_eq!(report["document"], Value::Null, "{name}");
        assert_eq!(codes(&report), ["dm.expression.uncertain_call"], "{name}: {report:#}");
        assert_eq!(report["diagnostics"][0]["severity"], "warning", "{name}");
        assert!(output.stderr.is_empty(), "{name}: {}", String::from_utf8_lossy(&output.stderr));
    }
}

/// A runtime warning that blocks stops the command before its later effects:
/// the `::shell` after the undeclared read never runs, and in a lifecycle run
/// neither does `composed`. Without the denial the same documents run both.
///
/// The shell effect is the native argv recorder creating `marker`, because
/// Windows has no `touch` on the fixture's minimal `PATH`.
#[test]
fn a_blocking_runtime_warning_stops_the_later_shell() {
    const BODY: &str = "{{ missing }}\n\n::shell argv-recorder marker\n";
    let lifecycle = format!("---\ncomposed:\n  - ensure_file: [\"composed-marker\", \"x\"]\n---\n{BODY}");
    for (name, content) in [("plain.md", BODY.to_string()), ("lifecycle.md", lifecycle)] {
        let fixture = CliProcessFixture::named(&format!("report_json_runtime_stop_{name}"));
        std::fs::copy(
            biscuit_test_harness::bin_exe!("darkmatter-argv-recorder"),
            fixture.bin_dir().join(format!("argv-recorder{}", std::env::consts::EXE_SUFFIX)),
        )
        .expect("the recorder fixture binary is built with `test-fixtures`");
        fixture.write_file("cwd/.darkmatter-shell-whitelist", "exact argv-recorder marker\n");
        let document = fixture.write_file(format!("cwd/{name}"), &content);
        let marker = fixture.cwd().join("marker");
        let composed = fixture.cwd().join("composed-marker");
        let compose = |extra: &[&str]| {
            fixture
                .command()
                .env(ARGV_RECORDER_FILE, &marker)
                .args(["compose", "--output", "report-json"])
                .args(extra)
                .arg(&document)
                .output()
                .unwrap()
        };

        let output = compose(&["--deny", "all"]);
        assert_eq!(output.status.code(), Some(1), "{name}: {}", String::from_utf8_lossy(&output.stderr));
        let report = report_of(&output);
        assert_eq!(report["outcome"], "failure", "{name}");
        assert_eq!(report["document"], Value::Null, "{name}");
        assert_eq!(codes(&report), ["dm.expression.undeclared_property"], "{name}: {report:#}");
        assert!(!marker.exists(), "{name}: the shell after the blocking warning ran");
        assert!(!composed.exists(), "{name}: `composed` ran after the blocking warning");

        let output = compose(&[]);
        assert_eq!(output.status.code(), Some(0), "{name}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(report_of(&output)["outcome"], "success", "{name}");
        assert!(marker.exists(), "{name}: the control runs its shell");
        assert_eq!(composed.exists(), name == "lifecycle.md", "{name}: the control runs `composed`");
    }
}

/// A lifecycle run keeps stdout for the report: an action's `stdout` line goes
/// to stderr, and a failed event is one diagnostic.
#[test]
fn a_lifecycle_run_writes_only_the_report_to_stdout() {
    let fixture = CliProcessFixture::named("report_json_lifecycle");
    let success = fixture.write_file("cwd/ok.md", "---\ncomposed:\n  - stdout: from the lifecycle\n---\nbody\n");
    let output = run_report(&fixture, &success);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(report_of(&output), json!({ "outcome": "success", "diagnostics": [], "document": "body" }));
    assert!(String::from_utf8_lossy(&output.stderr).contains("from the lifecycle"));

    let failing = fixture.write_file("cwd/fail.md", "---\ncomposed:\n  - error: the gate refused\n---\nbody\n");
    let output = run_report(&fixture, &failing);
    assert_eq!(output.status.code(), Some(1));
    let report = report_of(&output);
    assert_eq!(report["document"], Value::Null);
    assert_eq!(codes(&report), ["dm.lifecycle.failure"]);
    assert_eq!(report["diagnostics"][0]["message"], "the gate refused");
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn an_unresolved_reference_keeps_its_exit_status() {
    let fixture = CliProcessFixture::named("report_json_reference");
    let document = fixture.write_file("cwd/doc.md", "# Doc\n\n[link](./nope.md)\n");
    let output = run_report(&fixture, &document);
    assert_eq!(output.status.code(), Some(2));
    let report = report_of(&output);
    assert_eq!(report["document"], Value::Null);
    assert_eq!(codes(&report), ["dm.reference.missing_hyperlink"]);
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
}

/// Criterion 22's report leg: no frontmatter value or transcluded file content
/// behind a flagged call appears in the report.
#[test]
fn the_report_holds_no_value_or_file_content() {
    const SECRET: &str = "s3cr3t-value";
    let fixture = CliProcessFixture::named("report_json_privacy");
    fixture.write_file("cwd/child.md", &format!("---\n$schema:\n  flag: boolean(required)\nflag: true\nnote: {SECRET}\n---\n{SECRET} {{{{ min(flag, 5) }}}}\n"));
    let document = fixture.write_file(
        "cwd/doc.md",
        &format!("---\n$schema:\n  label: string\nlabel: \"{SECRET}\"\n---\n{{{{ min(label, 5) }}}}\n\n::file ./child.md\n"),
    );
    let output = run_report(&fixture, &document);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    // The parent's passive warning, then the child's rejection.
    assert_eq!(codes(&report_of(&output)), ["dm.expression.uncertain_call", "dm.expression.incompatible_call"], "{stdout}");
    assert!(!stdout.contains(SECRET), "{stdout}");
}

#[test]
fn report_conflicts_and_other_commands_are_usage_errors_before_any_work() {
    let fixture = CliProcessFixture::named("report_json_rejections");
    let document = fixture.write_file("cwd/doc.md", "---\ntitle:   Spaced\n---\n# Doc\n");
    let compose = |extra: &str| fixture.command().args(["compose", "--output", "report-json", extra]).arg(&document).output().unwrap();
    let render = fixture.command().args(["render", "--output", "report-json"]).arg(&document).output().unwrap();
    let top_level = fixture.command().args(["--output", "report-json"]).arg(&document).output().unwrap();
    let clean = fixture.command().args(["clean", "--save", "--output", "report-json"]).arg(&document).output().unwrap();
    let validate = fixture.command().args(["schema", "validate", "--format", "report-json"]).arg(&document).output().unwrap();
    for (label, output, names) in [
        ("compose --show", compose("--show"), "'--output report-json' cannot be used with '--show'"),
        ("compose --shell", compose("--shell"), "'--output report-json' cannot be used with '--shell'"),
        ("render", render, "invalid value 'report-json' for '--output"),
        ("md FILE", top_level, "invalid value 'report-json' for '--output"),
        // `md clean` has no output selector at all.
        ("clean", clean, "unexpected argument '--output'"),
        ("schema validate", validate, "invalid value 'report-json' for '--format"),
    ] {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "{label}: {stderr}");
        assert!(output.stdout.is_empty(), "{label}: {}", String::from_utf8_lossy(&output.stdout));
        assert!(stderr.contains(names), "{label}: {stderr}");
    }
    assert_eq!(std::fs::read_to_string(&document).unwrap(), "---\ntitle:   Spaced\n---\n# Doc\n", "clean saved nothing");
}

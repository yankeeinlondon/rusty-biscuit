//! What `md compose --output json` says about each problem family:
//! reference details and authored positions, null positions, inserted
//! provenance, and a reference check that could not run.

use crate::common;

use common::CliProcessFixture;
use serde_json::{Value, json};
use std::path::Path;

const ROOT: &str = include_str!("../../../lib/tests/fixtures/composition_errors/root.md");
const CHILD: &str = include_str!("../../../lib/tests/fixtures/composition_errors/child.md");

/// A warning only the root's passive call check finds.
const WARNS: &str = "$schema:\n  label: string\nlabel: \"7\"\n";

fn report(fixture: &CliProcessFixture, document: &Path) -> (Option<i32>, Value) {
    let output = fixture.command().args(["compose", "--output", "json"]).arg(document).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.is_empty(), "the report holds everything: {stderr}");
    (output.status.code(), serde_json::from_slice(&output.stdout).expect("one JSON report"))
}

fn codes(report: &Value) -> Vec<&str> {
    report["diagnostics"].as_array().unwrap().iter().map(|diagnostic| diagnostic["code"].as_str().unwrap()).collect()
}

/// The record about `reference`.
fn about<'r>(report: &'r Value, reference: &str) -> &'r Value {
    let diagnostics = report["diagnostics"].as_array().unwrap();
    diagnostics.iter().find(|diagnostic| diagnostic["reference"] == reference).unwrap_or_else(|| panic!("{reference}: {report:#}"))
}

fn file_of(value: &Value) -> &Path {
    Path::new(value.as_str().unwrap_or_else(|| panic!("a file: {value}")))
}

fn authored_text<'t>(text: &'t str, diagnostic: &Value) -> &'t str {
    let (start, end) = (diagnostic["span"]["start"].as_u64().unwrap(), diagnostic["span"]["end"].as_u64().unwrap());
    &text[start as usize..end as usize]
}

/// `text` under `ending`, whatever line ending the checkout gave the fixture.
fn with_ending(text: &str, ending: &str) -> String {
    text.replace("\r\n", "\n").replace('\n', ending)
}

#[test]
fn reference_diagnostics_are_authored_under_either_line_ending() {
    for (name, ending) in [("lf", "\n"), ("crlf", "\r\n")] {
        let fixture = CliProcessFixture::named("report_reference_positions");
        let (root_text, child_text) = (with_ending(ROOT, ending), with_ending(CHILD, ending));
        let root = fixture.write_file("cwd/root.md", &root_text);
        fixture.write_file("cwd/child.md", &child_text);

        let (status, report) = report(&fixture, &root);

        assert_eq!(status, Some(1), "{name}");
        assert_eq!(report["outcome"], "failure", "{name}");

        let typed = about(&report, "./missing-one.md");
        assert_eq!(typed["code"], "dm.reference.missing_hyperlink", "{name}");
        assert!(file_of(&typed["source"]["file"]).ends_with("root.md"), "{name}: {typed:#}");
        assert_eq!(typed["source"]["line"], 9, "{name}");
        assert_eq!(typed["source"]["column"], 5, "{name}");
        assert_eq!(typed["source"]["section"], "intro-root", "{name}");
        assert_eq!(typed["source"]["inserted"], false, "{name}");
        assert_eq!(typed["reason"], "file not found", "{name}");
        assert_eq!(typed["construct"], "hyperlink", "{name}");
        assert_eq!(typed["failure"], "no-match", "{name}");
        assert_eq!(typed["included_via"], json!([]), "{name}");
        assert!(authored_text(&root_text, typed).starts_with("[missing one]"), "{name}: {typed:#}");

        let inserted = about(&report, "./missing-ins.md");
        assert!(file_of(&inserted["source"]["file"]).ends_with("child.md"), "{name}: {inserted:#}");
        assert_eq!(inserted["source"]["line"], 9, "{name}");
        assert_eq!(inserted["source"]["column"], 7, "{name}");
        assert_eq!(inserted["source"]["inserted"], true, "{name}");
        assert_eq!(authored_text(&child_text, inserted), "{{ inserted }}", "{name}");
        let [step] = inserted["included_via"].as_array().unwrap().as_slice() else {
            panic!("{name}: one step: {inserted:#}");
        };
        assert!(file_of(&step["file"]).ends_with("root.md"), "{name}: {step:#}");
        assert_eq!(step["line"], 26, "{name}");
    }
}

/// A file-resolution failure has no prose row in the reference report, so the
/// class is kept here; a reason with no class omits the field.
#[test]
fn only_a_file_resolution_failure_reports_a_class() {
    let fixture = CliProcessFixture::named("report_reference_failure_class");
    let document = fixture.write_file("cwd/doc.md", "# Doc\n\n[missing](./nope.md)\n\n[bad](http://[)\n");

    let (_, report) = report(&fixture, &document);

    assert_eq!(about(&report, "./nope.md")["failure"], "no-match");
    let malformed = about(&report, "http://[");
    assert_eq!(malformed["reason"], "malformed URL", "{malformed:#}");
    assert!(malformed.get("failure").is_none(), "{malformed:#}");
}

/// Above the first heading a problem has no section, and the key is omitted.
#[test]
fn a_problem_above_the_first_heading_omits_its_section() {
    let fixture = CliProcessFixture::named("report_no_section");
    let document = fixture.write_file("cwd/doc.md", "[missing](./nope.md)\n\n# Later\n");

    let (_, report) = report(&fixture, &document);

    let source = about(&report, "./nope.md")["source"].as_object().unwrap();
    assert!(!source.contains_key("section"), "{source:#?}");
}

#[test]
fn a_prologue_problem_is_reported_at_its_value() {
    let fixture = CliProcessFixture::named("report_prologue");
    let document = fixture.write_file("cwd/doc.md", "---\nprologue: ./nope.md\n---\n# Doc\n");

    let (status, report) = report(&fixture, &document);

    assert_eq!(status, Some(1));
    let found = about(&report, "./nope.md");
    assert_eq!((&found["source"]["line"], &found["source"]["column"]), (&json!(2), &json!(11)), "{found:#}");
    assert_eq!(found["construct"], "prologue:");
}

/// Each family's failure: a position only where its producer proves one, its
/// file when it knows only that, and none of the reference fields.
#[test]
fn other_families_report_their_positions_and_no_reference_details() {
    let fixture = CliProcessFixture::named("report_families");
    fixture.write_file("cwd/cycle.md", "::file ./cycle.md\n");
    fixture.write_file("cwd/x.md", "# X\n");
    for (name, content, code, file, line, column) in [
        ("schema.md", "---\ncount: hello\n$schema:\n  count: number(required)\n---\nbody\n", "dm.schema.type_mismatch", Some("schema.md"), Some(2), Some(8)),
        ("prepare.md", "---\n$schema:\n  count: definitely_unknown_type\n---\nbody\n", "dm.schema.prepare", Some("prepare.md"), None, None),
        ("expression.md", "# Doc\n\n{{ 1 / 0 }}\n", "dm.expression.evaluation_failure", Some("expression.md"), Some(3), Some(1)),
        ("condition.md", "---\nmulti: \"a\\n\\nb\"\n---\n{{ multi }}\n\n::file ./x.md when=\"current.nosuch\"\n", "dm.expression.evaluation_failure", Some("condition.md"), Some(6), Some(1)),
        ("block.md", "---\nmulti: \"a\\n\\nb\"\n---\n{{ multi }}\n\n::block when=\"current.nosuch\"\nhi\n::end-block\n", "dm.expression.evaluation_failure", Some("block.md"), Some(6), Some(1)),
        ("transclusion.md", "::file ./cycle.md\n", "dm.transclusion.failure", Some("cycle.md"), None, None),
        ("shell.md", "::shell not-a-real-command --flag\n", "dm.shell.failure", Some("shell.md"), None, None),
        ("frontmatter_shell.md", "---\nx: \"$(echo hello)\"\n---\nbody\n", "dm.shell.failure", Some("frontmatter_shell.md"), Some(2), None),
        ("lifecycle_shell.md", "---\ncomposed:\n  - shell: \"false\"\n---\nbody\n", "dm.shell.failure", Some("lifecycle_shell.md"), None, None),
        // Names nothing a document wrote: a lifecycle `error:` is a message.
        ("lifecycle.md", "---\ncomposed:\n  - error: boom\n---\nbody\n", "dm.lifecycle.failure", None, None, None),
    ] {
        let document = fixture.write_file(format!("cwd/{name}"), content);

        let (status, report) = report(&fixture, &document);

        assert_eq!(status, Some(1), "{name}: {report:#}");
        assert_eq!(report["outcome"], "failure", "{name}");
        let [diagnostic] = report["diagnostics"].as_array().unwrap().as_slice() else {
            panic!("{name}: one record: {report:#}");
        };
        assert_eq!(diagnostic["code"], code, "{name}: {diagnostic:#}");
        match file {
            Some(file) => {
                assert_eq!(file_of(&diagnostic["source"]["file"]).file_name().unwrap(), file, "{name}: {diagnostic:#}");
                assert_eq!(diagnostic["source"]["line"], json!(line), "{name}: {diagnostic:#}");
                assert_eq!(diagnostic["source"]["column"], json!(column), "{name}: {diagnostic:#}");
                assert_eq!(diagnostic["source"]["inserted"], false, "{name}");
            }
            None => assert_eq!(diagnostic["source"], Value::Null, "{name}: {diagnostic:#}"),
        }
        for key in ["reason", "construct", "reference", "failure", "included_via"] {
            assert!(diagnostic.get(key).is_none(), "{name}: {key}: {diagnostic:#}");
        }
    }
}

/// Warnings and a failure holding no file of its own reach the envelope with
/// the document's name, and a policy stop reports the same records.
#[test]
fn warnings_and_a_typeless_failure_name_the_document() {
    let fixture = CliProcessFixture::named("report_document_fallback");
    let content = "---\na: 1\n---\n\n{{ nosuch }}\n\n::block when=\"ctx.toady.x == 1\"\nhi\n::end-block\n";
    let document = fixture.write_file("cwd/warns.md", content);

    let (status, warned) = report(&fixture, &document);

    assert_eq!(status, Some(0), "{warned:#}");
    let sources = |report: &Value| -> Vec<(u64, Option<u64>)> {
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|diagnostic| {
                assert_eq!(file_of(&diagnostic["source"]["file"]).file_name().unwrap(), "warns.md", "{diagnostic:#}");
                (diagnostic["source"]["line"].as_u64().unwrap(), diagnostic["source"]["column"].as_u64())
            })
            .collect()
    };
    assert_eq!(sources(&warned), [(7, Some(1)), (5, None)], "{warned:#}");

    let output = fixture
        .command()
        .args(["compose", "--output", "json", "--deny", "all"])
        .arg(&document)
        .output()
        .unwrap();
    let blocked: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(1), "{blocked:#}");
    assert_eq!(sources(&blocked), [(7, Some(1))], "the stop is before the undeclared read is judged: {blocked:#}");

    // The typed error names only its target, not the document that wrote it.
    fixture.write_file("cwd/picture.png", "x");
    let (status, failed) = report(&fixture, &fixture.write_file("cwd/includes.md", "::file ./picture.png\n"));
    assert_eq!(status, Some(1), "{failed:#}");
    let [record] = failed["diagnostics"].as_array().unwrap().as_slice() else {
        panic!("one record: {failed:#}");
    };
    assert_eq!(record["code"], "dm.transclusion.failure", "{record:#}");
    assert_eq!(file_of(&record["source"]["file"]).file_name().unwrap(), "includes.md", "{record:#}");
    assert_eq!(record["source"]["line"], Value::Null, "{record:#}");
}

/// `md render` delivers the same fallback for a record about its document.
#[test]
fn a_render_style_warning_names_the_document() {
    let fixture = CliProcessFixture::named("report_render_style");
    let document = fixture.write_file("cwd/styled.md", "---\nstyle:\n  bogus_key: 1\n---\n# T\n");

    let output = fixture.command().args(["render", "--output", "json"]).arg(&document).output().unwrap();

    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let [record] = value["diagnostics"].as_array().unwrap().as_slice() else {
        panic!("one record: {value:#}");
    };
    assert_eq!(record["code"], "dm.style.unknown_key", "{record:#}");
    assert_eq!(file_of(&record["source"]["file"]).file_name().unwrap(), "styled.md", "{record:#}");
    // The authored key: `bogus_key` on the document's third line.
    assert_eq!(record["source"]["line"], 3, "{record:#}");
    assert_eq!(record["source"]["column"], 3, "{record:#}");
}

/// Input with no file reports positions with a null file, and otherwise a null
/// source, never a placeholder name.
#[test]
fn stdin_names_no_file() {
    let fixture = CliProcessFixture::named("report_stdin");
    for (content, expected) in [
        ("---\ncount: hello\n$schema:\n  count: number(required)\n---\nbody\n", json!({ "file": null, "line": 2, "column": 8, "inserted": false })),
        ("---\n$schema:\n  count: definitely_unknown_type\n---\nbody\n", Value::Null),
    ] {
        let output = fixture.command().args(["compose", "--output", "json", "-"]).write_stdin(content).output().unwrap();
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["diagnostics"][0]["source"], expected, "{value:#}");
    }
}

/// A title an expression wrote is reported at that expression and marked.
#[test]
fn a_link_title_warning_marks_a_title_a_stage_wrote() {
    let fixture = CliProcessFixture::named("report_link_title_inserted");
    let document = fixture.write_file(
        "cwd/doc.md",
        "---\ntip: \"class=x\"\n---\n\n[written](./doc.md \"{{ tip }}\")\n\n[typed](./doc.md \"class=y\")\n",
    );

    let (status, report) = report(&fixture, &document);

    assert_eq!(status, Some(0), "{report:#}");
    let sources: Vec<_> = report["diagnostics"].as_array().unwrap().iter().map(|diagnostic| &diagnostic["source"]).collect();
    let [typed, written] = sources.as_slice() else { panic!("two warnings: {report:#}") };
    assert_eq!((&written["line"], &written["column"], &written["inserted"]), (&json!(5), &json!(21), &json!(true)), "{report:#}");
    assert_eq!((&typed["line"], &typed["column"], &typed["inserted"]), (&json!(7), &json!(1), &json!(false)), "{report:#}");
}

/// The two ways a run reaches the reference check.
const PATHS: [(&str, &str); 2] = [("direct", ""), ("lifecycle", "composed:\n  - stdout: done\n")];

/// A repository gives a relative reference a tree to leave.
fn in_a_repository(fixture: &CliProcessFixture) {
    fixture.write_file("cwd/.git/HEAD", "ref: refs/heads/main\n");
    fixture.write_file("cwd/.git/config", "[core]\nrepositoryformatversion = 0\nbare = false\n");
    std::fs::create_dir_all(fixture.cwd().join(".git/objects")).unwrap();
    std::fs::create_dir_all(fixture.cwd().join(".git/refs/heads")).unwrap();
}

/// A hyperlink leaving the repository is a resolution error, so
/// `validate_references` returns `Err` after its graph is built. Composition
/// never resolves a hyperlink: a check that read `Err` as clean let this
/// document through with status 0.
#[test]
fn a_reference_check_that_cannot_run_fails_the_run() {
    for (path, lifecycle) in PATHS {
        let fixture = CliProcessFixture::named("report_check_failure");
        in_a_repository(&fixture);
        let escape = "../".repeat(40);
        let document = fixture.write_file(
            "cwd/doc.md",
            &format!("---\n{WARNS}{lifecycle}---\n{{{{ min(label, 5) }}}}\n\n[out]({escape}outside.md)\n"),
        );

        let (status, report) = report(&fixture, &document);

        assert_eq!(status, Some(1), "{path}: {report:#}");
        assert_eq!(report["outcome"], "failure", "{path}: {report:#}");
        assert_eq!(report["document"], Value::Null, "{path}");
        assert_eq!(codes(&report), ["dm.expression.uncertain_call", "dm.reference.failure"], "the earlier warning survives; {path}: {report:#}");
        assert_eq!(report["diagnostics"][1]["failure"], "invalid-reference", "{path}: {report:#}");

        let output = fixture.command().arg("compose").arg(&document).output().unwrap();
        let stderr = common::unwrapped(&String::from_utf8_lossy(&output.stderr));
        assert_eq!(output.status.code(), Some(1), "{path}: {stderr}");
        assert!(output.stdout.is_empty(), "{path}: no document");
        assert!(stderr.contains("file reference failure"), "{path}: {stderr}");
    }
}

/// The graph build itself fails on a transcluded child's invalid reference.
#[test]
fn an_invalid_transclusion_in_a_child_fails_the_check_with_its_class() {
    for (path, lifecycle) in PATHS {
        let fixture = CliProcessFixture::named("report_check_invalid_reference");
        in_a_repository(&fixture);
        let escape = "../".repeat(40);
        fixture.write_file("cwd/child.md", &format!("# Child\n\n::file {escape}outside.md\n"));
        let document = fixture.write_file("cwd/doc.md", &format!("---\ntitle: doc\n{lifecycle}---\n::file ./child.md\n"));

        let (status, report) = report(&fixture, &document);

        assert_eq!(status, Some(1), "{path}: {report:#}");
        assert_eq!(report["outcome"], "failure", "{path}: {report:#}");
        assert_eq!(codes(&report), ["dm.reference.failure"], "{path}: {report:#}");
        assert_eq!(report["diagnostics"][0]["failure"], "invalid-reference", "{path}: {report:#}");
    }
}

/// A child failing its own preparation keeps its typed record, located in the
/// child, beside the root's earlier warning.
#[test]
fn a_failed_preparation_keeps_its_typed_diagnostic() {
    for (path, lifecycle) in PATHS {
        let fixture = CliProcessFixture::named("report_check_preparation");
        fixture.write_file("cwd/child.md", "---\n$schema:\n  count: number(required)\n---\n# Child\n");
        let document = fixture.write_file(
            "cwd/doc.md",
            &format!("---\n{WARNS}{lifecycle}---\n{{{{ min(label, 5) }}}}\n\n::file ./child.md\n"),
        );

        let (status, report) = report(&fixture, &document);

        assert_eq!(status, Some(1), "{path}: {report:#}");
        assert_eq!(codes(&report), ["dm.expression.uncertain_call", "dm.schema.missing_required"], "{path}: {report:#}");
        assert!(file_of(&report["diagnostics"][1]["source"]["file"]).ends_with("child.md"), "{path}: {report:#}");
    }
}

/// The check is passive: an unapproved body command is refused by the approval
/// walk that follows it, and nothing ever ran it.
#[test]
fn the_check_runs_no_shell_directive() {
    let fixture = CliProcessFixture::named("report_check_passive");
    let marker = fixture.cwd().join("ran");
    let document = fixture.write_file("cwd/doc.md", &format!("::shell touch {}\n", marker.display()));

    let (status, report) = report(&fixture, &document);

    assert_eq!(status, Some(1), "{report:#}");
    assert_eq!(codes(&report), ["dm.shell.failure"], "{report:#}");
    assert!(!marker.exists(), "the reference check ran a shell directive");
}

/// A reference the file-reference grammar rejects: the graph build fails with
/// a typed error that holds no file, on a document the command opened itself.
const UNPARSEABLE_PROLOGUE: &str = "---\ntitle: x\nprologue: \"{{ title }}.md\"\n---\n# Top\n";

fn failure_source(fixture: &CliProcessFixture, args: &[&str], document: &Path) -> Value {
    let output = fixture.command().args(args).arg(document).args(["--output", "json"]).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let payload: Value = serde_json::from_slice(&output.stdout).expect("one JSON payload");
    let diagnostics = payload["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 1, "{payload:#}");
    diagnostics[0]["source"].clone()
}

#[test]
fn validate_refs_names_its_document_on_a_failure_with_no_file() {
    let fixture = CliProcessFixture::named("report_validate_refs_fallback");
    let document = fixture.write_file("doc.md", UNPARSEABLE_PROLOGUE);

    let source = failure_source(&fixture, &["validate", "refs"], &document);

    assert_eq!(file_of(&source["file"]).file_name().unwrap(), "doc.md");
    assert_eq!(source["line"], Value::Null);
    assert_eq!(source["column"], Value::Null);
}

#[test]
fn graph_names_its_document_on_a_failure_with_no_file() {
    let fixture = CliProcessFixture::named("report_graph_fallback");
    let document = fixture.write_file("doc.md", UNPARSEABLE_PROLOGUE);

    let source = failure_source(&fixture, &["graph"], &document);

    assert_eq!(file_of(&source["file"]).file_name().unwrap(), "doc.md");
    assert_eq!(source["line"], Value::Null);
    assert_eq!(source["column"], Value::Null);
}

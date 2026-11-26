//! `md compose`'s combined report after a reference stop, through the binary.
//!
//! A broken reference stops the run. The checks that do not depend on it
//! (the call check, the schema verdict, the prospective empty body) still
//! run passively, and when one of them adds a finding the run prints one
//! report for the document instead of the reference report alone. When they
//! add nothing, the run prints what it always printed. The `notes.md` fixture
//! under `tests/fixtures/findings/` is the spec's case; its expected outputs
//! are beside it, with the fixture workspace written `<WS>`.

use crate::common;

use common::CliProcessFixture;
use std::path::Path;

/// One `md` run: exit status and both streams, the workspace written `<WS>`.
struct Run {
    status: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Runs `md compose` with `args` on a plain 80-column terminal, with
/// `document` written as `cwd/<name>` and passed after `args`. Stderr is no
/// terminal here, so its reports are never wrapped.
fn compose(test: &str, name: &str, document: &str, args: &[&str]) -> Run {
    let fixture = CliProcessFixture::named(test);
    // A Windows checkout may carry CRLF; the JSON baseline's spans count LF.
    let path = fixture.write_file(format!("cwd/{name}"), &document.replace("\r\n", "\n"));
    let output = fixture
        .command_builder()
        .plain_terminal(80, 40)
        .build()
        .arg("compose")
        .args(args)
        .arg(&path)
        .output()
        .unwrap();
    observed(&fixture, &output)
}

fn observed(fixture: &CliProcessFixture, output: &std::process::Output) -> Run {
    Run {
        status: output.status.code(),
        stdout: normalize(&String::from_utf8_lossy(&output.stdout), fixture.workspace_path()),
        stderr: normalize(&String::from_utf8_lossy(&output.stderr), fixture.workspace_path()),
    }
}

/// `text` with every spelling of the fixture workspace written `<WS>` and
/// Windows separators inside it written `/`; line endings normalized.
fn normalize(text: &str, workspace: &Path) -> String {
    let mut spellings = vec![workspace.to_path_buf()];
    if let Ok(canonical) = biscuit_file::canonicalize_simplified(workspace) {
        spellings.push(canonical);
    }
    let mut normalized = text.replace("\r\n", "\n");
    for spelling in spellings {
        let display = spelling.display().to_string();
        normalized = normalized
            .replace(&display.replace('\\', "\\\\"), "<WS>")
            .replace(&display, "<WS>")
            .replace(&biscuit_file::to_forward_slash_string(&spelling), "<WS>");
    }
    // Only a workspace path holds a backslash in these outputs (a JSON quote
    // escape is kept).
    normalized.replace("\\\\", "/").replace("\\\"", "\u{0}").replace('\\', "/").replace('\u{0}', "\\\"")
}

/// `notes.md` with `draft` set to `value`.
fn notes_with(value: &str) -> String {
    include_str!("../fixtures/findings/notes.md").replace("draft: false", &format!("draft: {value}"))
}

#[test]
fn notes_md_reports_the_missing_file_and_the_empty_body_together() {
    let run = compose("combined-notes-plain", "notes.md", include_str!("../fixtures/findings/notes.md"), &[]);
    assert_eq!(run.status, Some(1), "{}", run.stderr);
    assert_eq!(run.stdout, "");
    assert_eq!(run.stderr, include_str!("../fixtures/findings/notes_plain.txt").replace("\r\n", "\n"));
}

#[test]
fn notes_md_json_carries_both_findings_in_one_envelope() {
    let run = compose(
        "combined-notes-json",
        "notes.md",
        include_str!("../fixtures/findings/notes.md"),
        &["--output", "json"],
    );
    assert_eq!(run.status, Some(1), "{}", run.stdout);
    assert_eq!(run.stderr, "");
    assert_eq!(run.stdout, include_str!("../fixtures/findings/notes_json.json").replace("\r\n", "\n"));
}

/// A true condition includes the broken inclusion, so nothing but the
/// reference is wrong: the run prints the reference report it always printed.
#[test]
fn a_true_condition_prints_the_reference_report_alone() {
    let run = compose("combined-notes-true", "notes.md", &notes_with("true"), &[]);
    assert_eq!(run.status, Some(1), "{}", run.stderr);
    assert_eq!(run.stdout, "");
    assert_eq!(
        run.stderr,
        "Broken references in notes.md\n\nnotes.md\n  file not found in `::file`  `./missing.md`  notes.md:6:1\n\n\
         Checked 1 reference across 1 document: 0 ok, 1 broken\n"
    );
}

/// An expression that fails only when it is evaluated belongs to no passive
/// check: the run prints the reference report alone, and the expression's
/// error appears once the references are fixed.
#[test]
fn a_later_expression_failure_keeps_the_reference_report_alone() {
    let document = "[one](./missing-one.md)\n[two](./missing-two.md)\n\n{{ 1 / 0 }}\n";
    let run = compose("combined-expression", "doc.md", document, &[]);
    assert_eq!(run.status, Some(1), "{}", run.stderr);
    assert_eq!(
        run.stderr,
        "Broken references in doc.md\n\ndoc.md\n  file not found in hyperlink  `./missing-one.md`  doc.md:1:1\n  \
         file not found in hyperlink  `./missing-two.md`  doc.md:2:1\n\n\
         Checked 2 references across 1 document: 0 ok, 2 broken\n"
    );
}

/// A condition that reads a command's output is not known until the document
/// runs, so the body is never proven empty: the check is reported not run.
#[test]
fn a_deferred_condition_leaves_the_empty_body_unchecked() {
    let run = compose("combined-notes-deferred", "notes.md", &notes_with("$(echo false)"), &[]);
    assert_eq!(run.status, Some(1), "{}", run.stderr);
    assert!(run.stderr.contains("✗ file not found in `::file`"), "{}", run.stderr);
    assert!(run.stderr.contains("ℹ not checked: empty body (reads `draft`"), "{}", run.stderr);
    assert!(!run.stderr.contains("would be empty"), "{}", run.stderr);
    assert!(run.stderr.ends_with("Checked notes.md: 1 error, 0 warnings, 1 check not run\n"), "{}", run.stderr);
}

/// The missing-transclusion recovery still lets the document compose; the
/// composed body is then empty, which `md` reports as a warning.
#[test]
fn the_missing_transclusion_recovery_still_composes() {
    let run = compose(
        "combined-notes-recovered",
        "notes.md",
        include_str!("../fixtures/findings/notes.md"),
        &["--allow", "missing-transclusions"],
    );
    assert_eq!(run.status, Some(0), "{}", run.stderr);
    assert_eq!(run.stdout, "\n");
    assert!(run.stderr.contains("the composed body is empty"), "{}", run.stderr);
    assert!(!run.stderr.contains("Checked"), "{}", run.stderr);
}

const EMPTY_BODY: &str = "---\ndraft: false\n---\n::block when=\"draft\"\nhidden\n::end-block\n";

#[test]
fn an_empty_composed_body_warns() {
    let run = compose("combined-empty-warns", "empty.md", EMPTY_BODY, &["--output", "json"]);
    assert_eq!(run.status, Some(0), "{}", run.stdout);
    let report: serde_json::Value = serde_json::from_str(&run.stdout).unwrap();
    let [record] = report["diagnostics"].as_array().unwrap().as_slice() else { panic!("{report:#}") };
    assert_eq!(record["code"], "dm.compose.empty_body", "{record:#}");
    assert_eq!(record["severity"], "warning", "{record:#}");
    assert_eq!((&record["source"]["line"], &record["source"]["column"]), (&4.into(), &1.into()), "{record:#}");
}

#[test]
fn denying_the_empty_body_warning_fails_the_run() {
    let run = compose("combined-empty-denied", "empty.md", EMPTY_BODY, &["--deny", "dm.compose.empty_body"]);
    assert_eq!(run.status, Some(1), "{}", run.stderr);
    assert_eq!(run.stdout, "");
    assert!(run.stderr.contains("(dm.compose.empty_body)"), "{}", run.stderr);
}

/// A call the check proves wrong stops the reference check itself; the
/// check is repeated without it, so both findings are reported, and the
/// checks the refusal stops are reported not run.
#[test]
fn a_call_refusal_beside_a_broken_reference_reports_both() {
    let document = "---\ntitle: x\n---\n{{ upper(1, 2, 3) }}\n\n::file ./missing.md\n";
    let run = compose("combined-call", "call.md", document, &[]);
    assert_eq!(run.status, Some(1), "{}", run.stderr);
    let rows: Vec<&str> = run.stderr.lines().filter(|line| line.starts_with("  ") && !line.starts_with("   ")).collect();
    assert_eq!(
        rows,
        [
            "  ✗ file not found in `::file`",
            "  ✗ upper: no signature takes 3 arguments; expected upper(x: string | null): string | null",
            "  ℹ not checked: schema, empty body (a call the check proves wrong stops the document first)",
        ],
        "{}",
        run.stderr
    );
    assert!(run.stderr.ends_with("Checked call.md: 2 errors, 0 warnings, 2 checks not run\n"), "{}", run.stderr);
}

#[test]
fn a_schema_problem_beside_a_broken_reference_reports_both() {
    let document = "---\n$schema:\n  count: number(required)\ncount: \"abc\"\n---\nhello\n::file ./missing.md\n";
    let run = compose("combined-schema", "schema.md", document, &["--output", "json"]);
    assert_eq!(run.status, Some(1), "{}", run.stdout);
    let report: serde_json::Value = serde_json::from_str(&run.stdout).unwrap();
    let codes: Vec<&str> =
        report["diagnostics"].as_array().unwrap().iter().map(|record| record["code"].as_str().unwrap()).collect();
    assert_eq!(
        codes,
        ["dm.reference.missing_transclusion", "dm.schema.type_mismatch", "dm.check.not_checked"],
        "{report:#}"
    );
}

/// Standard input has no reference check, so nothing changes for it.
#[test]
fn standard_input_is_unchanged() {
    let fixture = CliProcessFixture::named("combined-stdin");
    let output = fixture
        .command_builder()
        .plain_terminal(80, 40)
        .build()
        .arg("compose")
        .write_stdin(include_str!("../fixtures/findings/notes.md"))
        .output()
        .unwrap();
    let run = observed(&fixture, &output);
    assert_eq!(run.status, Some(1), "{}", run.stderr);
    assert!(run.stderr.contains("TransclusionError"), "{}", run.stderr);
    assert!(!run.stderr.contains("Checked"), "{}", run.stderr);
}

/// A lifecycle run checks references after `initialize`, and reports the
/// combined findings there, before its `failed` stack runs.
#[test]
fn the_lifecycle_route_reports_the_combined_findings_after_initialize() {
    let document = notes_with("false").replace(
        "draft: false\n",
        "draft: false\ninitialize:\n  - info: \"event:initialize\"\nfailed:\n  - info: \"event:failed\"\n",
    );
    let run = compose("combined-lifecycle", "notes.md", &document, &[]);
    assert_eq!(run.status, Some(1), "{}", run.stderr);
    let initialize = run.stderr.find("event:initialize").expect("initialize ran");
    let report = run.stderr.find("Checked notes.md: 1 error, 1 warning").expect("the combined report");
    let failed = run.stderr.find("event:failed").expect("failed ran");
    assert!(initialize < report && report < failed, "{}", run.stderr);
    assert!(run.stderr.contains("⚠ the body would be empty"), "{}", run.stderr);
}

/// `initialize` can create the file the body includes: the check that runs
/// after it finds the reference resolved.
#[test]
fn initialize_can_create_an_included_file() {
    let document = "---\ninitialize:\n  - ensure_file: [\"included.md\", \"from initialize\"]\n---\n::file ./included.md\n";
    let run = compose("combined-lifecycle-creates", "doc.md", document, &[]);
    assert_eq!(run.status, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("from initialize"), "{}", run.stdout);
}

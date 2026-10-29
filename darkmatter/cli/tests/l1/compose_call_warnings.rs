//! End-to-end `md compose` coverage for the call warnings (criterion 20): a
//! category 3 call composes successfully, and each such argument, in the
//! body or in a lifecycle `when`, is one warning on stderr naming the call,
//! its file and line, and its category; under `--deny all` the same document
//! fails. A category 5 call in the same position fails the command with no
//! document on stdout.

use crate::common;

use common::CliProcessFixture;

const DOCUMENT: &str = "---
$schema:
  label: string
label: \"7\"
composed:
  - when: min(label, 5) > 100
    then:
      - stdout: never
---
Body {{ min(label, 5) }}
";

#[test]
fn a_category_3_call_composes_with_one_warning_per_argument() {
    let fixture = CliProcessFixture::named("compose_call_warnings");
    let document = fixture.write_file("cwd/doc.md", DOCUMENT);

    let output = fixture.command().arg("compose").arg(&document).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(0), "stderr: {stderr}");
    assert_eq!(stdout.trim_end(), "Body 5", "stderr: {stderr}");
    assert_eq!(stderr.matches("category 3").count(), 2, "{stderr}");
    assert!(stderr.contains("doc.md:6"), "the lifecycle call's line: {stderr}");
    assert!(stderr.contains("doc.md:10"), "the body call's line: {stderr}");
    assert!(stderr.contains("lifecycle `composed[0].when`"), "{stderr}");
}

#[test]
fn a_category_5_call_fails_with_no_document() {
    let fixture = CliProcessFixture::named("compose_call_warnings_rejected");
    let document = fixture.write_file("cwd/doc.md", &DOCUMENT.replace("Body {{ min(label, 5) }}", "Body {{ min(true, 5) }}"));

    let output = fixture.command().arg("compose").arg(&document).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_ne!(output.status.code(), Some(0), "stderr: {stderr}");
    assert!(output.stdout.is_empty(), "no document: {}", String::from_utf8_lossy(&output.stdout));
    assert!(stderr.contains("min") && stderr.contains("expected number"), "{stderr}");
}

#[test]
fn deny_all_fails_the_category_3_document() {
    let fixture = CliProcessFixture::named("compose_call_warnings_denied");
    let document = fixture.write_file("cwd/doc.md", DOCUMENT);
    // The lifecycle-free body alone takes the composition path.
    let body_only = fixture.write_file("cwd/body.md", "---\n$schema:\n  label: string\nlabel: \"7\"\n---\nBody {{ min(label, 5) }}\n");

    for path in [&document, &body_only] {
        let output = fixture.command().args(["compose", "--deny", "all"]).arg(path).output().unwrap();
        let stderr = common::unwrapped(&String::from_utf8_lossy(&output.stderr));
        assert_eq!(output.status.code(), Some(1), "{}: {stderr}", path.display());
        assert!(output.stdout.is_empty(), "no document: {}", String::from_utf8_lossy(&output.stdout));
        assert!(stderr.contains("blocking diagnostics"), "{stderr}");
        assert!(stderr.contains("dm.expression.uncertain_call"), "the code is reported: {stderr}");
        assert!(stderr.contains("category 3"), "the severity and message are unchanged: {stderr}");
    }
}

#[test]
fn an_invalid_call_in_an_empty_callback_rejects_before_execution() {
    let fixture = CliProcessFixture::named("compose_callback_call_rejected");
    let document = fixture.write_file("cwd/doc.md", "Body {{ map([], x => min(true, 5)) }}\n");
    let output = fixture.command().args(["compose", "--output", "report-json"]).arg(&document).output().unwrap();
    assert_ne!(output.status.code(), Some(0));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(report["document"].is_null(), "{report}");
    assert!(report["diagnostics"].as_array().unwrap().iter().any(|diagnostic| {
        diagnostic["code"] == "dm.expression.incompatible_call"
            && diagnostic["message"].as_str().unwrap().contains("min")
    }), "{report}");
}

//! End-to-end `md compose` coverage for `dm.expression.undeclared_member`: a
//! constant member selector whose base has a known closed object type that
//! does not declare the member composes with one located warning, and
//! `--deny` with the code or `all` turns that warning into a failure with no
//! deliverable.

use crate::common;

use common::CliProcessFixture;

const CODE: &str = "dm.expression.undeclared_member";

const DOCUMENT: &str = "---
$schema:
  server: \"{ name: string, port: number }\"
server: { name: web, port: 80 }
---
x {{ server.nmae }}
";

#[test]
fn an_undeclared_member_composes_with_a_located_warning() {
    let fixture = CliProcessFixture::named("compose_member_warnings");
    let document = fixture.write_file("cwd/doc.md", DOCUMENT);

    let output = fixture.command().arg("compose").arg(&document).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = common::unwrapped(&String::from_utf8_lossy(&output.stderr));

    assert_eq!(output.status.code(), Some(0), "stderr: {stderr}");
    assert_eq!(stdout.trim_end(), "x", "the missing member renders empty; stderr: {stderr}");
    assert!(stderr.contains("Member 'nmae' is not declared by the known object type of 'server'"), "{stderr}");
    assert!(stderr.contains("doc.md:6"), "the selector's line: {stderr}");
}

fn assert_denied_without_a_document(fixture_name: &str, selector: &str) {
    let fixture = CliProcessFixture::named(fixture_name);
    let document = fixture.write_file("cwd/doc.md", DOCUMENT);

    let output = fixture.command().args(["compose", "--deny", selector]).arg(&document).output().unwrap();
    let stderr = common::unwrapped(&String::from_utf8_lossy(&output.stderr));

    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(output.stdout.is_empty(), "no document: {}", String::from_utf8_lossy(&output.stdout));
    assert!(stderr.contains("blocking diagnostics"), "{stderr}");
    assert!(stderr.contains(CODE), "the code is reported: {stderr}");
    assert!(stderr.contains("Member 'nmae'"), "the message is unchanged: {stderr}");
}

#[test]
fn denying_the_code_fails_with_no_document() {
    assert_denied_without_a_document("compose_member_warnings_deny_code", CODE);
}

#[test]
fn denying_all_fails_with_no_document() {
    assert_denied_without_a_document("compose_member_warnings_deny_all", "all");
}

#[test]
fn the_json_envelope_carries_the_located_warning() {
    let fixture = CliProcessFixture::named("compose_member_warnings_json");
    let document = fixture.write_file("cwd/doc.md", DOCUMENT);

    let output = fixture.command().args(["compose", "--output", "json"]).arg(&document).output().unwrap();

    assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stdout));
    assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let warnings: Vec<_> = report["diagnostics"].as_array().unwrap().iter().filter(|diagnostic| diagnostic["code"] == CODE).collect();
    assert_eq!(warnings.len(), 1, "{report}");
    assert_eq!(warnings[0]["severity"], "warning", "{report}");
    // Line 6 is the body line; column 13 is the `nmae` segment after `x {{ server.`.
    assert_eq!(warnings[0]["source"]["line"], 6, "{report}");
    assert_eq!(warnings[0]["source"]["column"], 13, "{report}");
}

#[test]
fn compose_help_lists_the_code() {
    let fixture = CliProcessFixture::named("compose_member_warnings_help");
    let output = fixture.command().args(["compose", "--help"]).output().unwrap();
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(help.contains(CODE), "{help}");
}

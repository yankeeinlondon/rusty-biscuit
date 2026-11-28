//! `md compose`'s report of an inclusion whose target composition could not
//! open, through the binary, for each cause: a target proven missing names
//! its construct, the resolved path, and the file to create, under the
//! missing-transclusion code; an unreadable target keeps its read failure and
//! the permissions advice; an unparseable one keeps its parse failure in its
//! own file. Plain text and JSON are checked for each.

use crate::common;

use common::CliProcessFixture;
use serde_json::Value;
use std::path::Path;

/// `md compose` of `document` with `args`: status, stdout, stderr.
fn compose(fixture: &CliProcessFixture, document: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    let output = fixture
        .command_builder()
        .plain_terminal(100, 40)
        .build()
        .arg("compose")
        .args(args)
        .arg(document)
        .output()
        .unwrap();
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n"),
        String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n"),
    )
}

/// The one error record of a `--output json` run.
fn error_record(stdout: &str) -> Value {
    let report: Value = serde_json::from_str(stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"));
    let errors: Vec<&Value> =
        report["diagnostics"].as_array().unwrap().iter().filter(|record| record["severity"] == "error").collect();
    assert_eq!(errors.len(), 1, "{report:#}");
    errors[0].clone()
}

/// `path` with `/` separators, so it compares on every host.
fn forward(path: &str) -> String {
    path.replace('\\', "/")
}

#[test]
fn a_missing_file_reference_record_names_the_resolved_path_and_the_fix() {
    let fixture = CliProcessFixture::named("inclusion_findings_missing_json");
    let document = fixture.write_file("cwd/notes/doc.md", "---\ntitle: x\n---\n\n::file ./missing.md\n");
    let (status, stdout, stderr) = compose(&fixture, &document, &["--output", "json"]);

    assert_eq!(status, Some(1), "{stdout}");
    assert!(stderr.is_empty(), "{stderr}");
    let record = error_record(&stdout);
    assert_eq!(record["code"], "dm.reference.missing_transclusion", "{record:#}");
    assert_eq!(record["construct"], "::file", "{record:#}");
    assert_eq!(record["reference"], "./missing.md", "{record:#}");
    assert_eq!(record["failure"], "no-match", "{record:#}");
    assert_eq!(record["hint"], "create missing.md, or correct the reference", "{record:#}");
    assert!(forward(record["resolved"].as_str().unwrap()).ends_with("/cwd/notes/missing.md"), "{record:#}");
    assert!(forward(record["base"].as_str().unwrap()).ends_with("/cwd/notes"), "{record:#}");
    assert_eq!((record["source"]["line"].as_u64(), record["source"]["column"].as_u64()), (Some(5), Some(1)));
}

/// A child's missing link reached through one inclusion: `md compose`
/// publishes the inclusion as a `trail` beside `included_via`, while
/// `md validate refs` keeps its legacy record with neither `trail` nor the
/// resolver evidence.
#[test]
fn an_included_missing_link_publishes_its_inclusion_trail_on_compose_only() {
    let fixture = CliProcessFixture::named("inclusion_findings_child_trail");
    fixture.write_file("cwd/docs/child.md", "# Child\n\n[x](./child-gone.md)\n");
    let root = fixture.write_file("cwd/docs/root.md", "# Root\n\n::file ./child.md\n");

    let (status, stdout, stderr) = compose(&fixture, &root, &["--output", "json"]);

    assert_eq!(status, Some(1), "{stdout}");
    assert!(stderr.is_empty(), "{stderr}");
    let record = error_record(&stdout);
    assert_eq!(record["code"], "dm.reference.missing_hyperlink", "{record:#}");
    let [step] = record["included_via"].as_array().expect("an inclusion path").as_slice() else {
        panic!("one step: {record:#}")
    };
    assert_eq!(step["line"], 3, "{record:#}");
    let [frame] = record["trail"].as_array().expect("a trail").as_slice() else { panic!("one frame: {record:#}") };
    assert_eq!(frame["kind"], "included", "{record:#}");
    assert_eq!(frame["file"], step["file"], "{record:#}");
    assert_eq!(frame["line"], 3, "{record:#}");
    assert_eq!(record["hint"], "create child-gone.md, or correct the reference", "{record:#}");

    let output = fixture
        .command_builder()
        .plain_terminal(100, 40)
        .build()
        .args(["validate", "refs"])
        .arg(&root)
        .args(["--output", "json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let legacy = error_record(&String::from_utf8_lossy(&output.stdout));
    assert_eq!(legacy["included_via"], record["included_via"], "{legacy:#}");
    for added in ["trail", "base", "resolved", "hint"] {
        assert!(legacy.get(added).is_none(), "legacy record has no {added}: {legacy:#}");
    }
}

#[test]
fn a_missing_target_the_reference_check_cannot_see_fails_with_its_fix() {
    // Standard input has no file, so the reference check cannot resolve a
    // relative target; an absolute one is left to composition.
    let fixture = CliProcessFixture::named("inclusion_findings_missing_stdin");
    let missing = fixture.cwd().join("nope.md");
    let body = format!("# Doc\n\n::file {}\n", forward(&missing.display().to_string()));
    let run = |args: &[&str]| {
        fixture
            .command_builder()
            .plain_terminal(100, 40)
            .build()
            .arg("compose")
            .args(args)
            .arg("-")
            .write_stdin(body.clone())
            .output()
            .unwrap()
    };

    let plain = run(&[]);
    let stderr = String::from_utf8_lossy(&plain.stderr);
    assert_eq!(plain.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("TransclusionError: file not found in ::file"), "{stderr}");
    assert!(stderr.contains("Resolved:"), "{stderr}");
    assert!(stderr.contains("Create nope.md, or correct the reference."), "{stderr}");
    assert!(!stderr.contains("permissions"), "{stderr}");

    let json = run(&["--output", "json"]);
    let record = error_record(&String::from_utf8_lossy(&json.stdout));
    assert_eq!(record["code"], "dm.reference.missing_transclusion", "{record:#}");
    assert_eq!(record["message"], "file not found in `::file`", "{record:#}");
    assert_eq!(record["hint"], "create nope.md, or correct the reference", "{record:#}");
}

#[test]
fn an_unparseable_target_keeps_its_parse_failure() {
    let fixture = CliProcessFixture::named("inclusion_findings_parse");
    fixture.write_file("cwd/bad.md", "---\ntitle: [\n---\nx\n");
    let document = fixture.write_file("cwd/doc.md", "# Doc\n\n::file ./bad.md\n");

    let (status, _, stderr) = compose(&fixture, &document, &[]);
    assert_eq!(status, Some(1), "{stderr}");
    assert!(stderr.contains("frontmatter parse failed"), "{stderr}");
    assert!(!stderr.contains("correct the reference"), "{stderr}");

    let (_, stdout, _) = compose(&fixture, &document, &["--output", "json"]);
    let record = error_record(&stdout);
    assert_eq!(record["code"], "dm.frontmatter.yaml_parse", "{record:#}");
    assert!(forward(record["source"]["file"].as_str().unwrap()).ends_with("/cwd/bad.md"), "{record:#}");
    assert!(record.get("hint").is_none(), "{record:#}");
}

/// A target that exists but cannot be read keeps the read failure's own
/// advice. Unix only: a mode-`000` file is readable to an elevated Windows
/// token, and to root.
#[cfg(unix)]
#[test]
fn an_unreadable_target_keeps_its_read_failure_and_hint() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = CliProcessFixture::named("inclusion_findings_unreadable");
    let locked = fixture.write_file("cwd/locked.md", "# Locked\n");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::read(&locked).is_ok() {
        return;
    }
    let document = fixture.write_file("cwd/doc.md", "# Doc\n\n::file ./locked.md\n");

    let (status, _, stderr) = compose(&fixture, &document, &[]);
    let (_, stdout, _) = compose(&fixture, &document, &["--output", "json"]);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o644)).unwrap();

    assert_eq!(status, Some(1), "{stderr}");
    assert!(stderr.contains("TransclusionError: I/O failure"), "{stderr}");
    assert!(stderr.contains("Check file existence and permissions."), "{stderr}");
    assert!(!stderr.contains("correct the reference"), "{stderr}");
    let record = error_record(&stdout);
    assert_eq!(record["code"], "dm.input.read_failure", "{record:#}");
    assert!(record.get("hint").is_none(), "{record:#}");
    assert_eq!((record["source"]["line"].as_u64(), record["source"]["column"].as_u64()), (Some(3), Some(1)));
}

#[test]
fn a_target_built_from_a_supplied_value_is_reported_at_its_authored_directive() {
    // The directive is authored; the value that names its target is
    // supplied on the command line, so the record names the supplied target
    // at the directive's line, never a position in the value.
    let fixture = CliProcessFixture::named("inclusion_findings_supplied");
    let document = fixture.write_file("cwd/doc.md", "---\nname: chapter\n---\n\n::file ./{{ name }}.md\n");
    let (status, stdout, _) = compose(&fixture, &document, &["--set", r#"{"name":"absent"}"#, "--output", "json"]);

    assert_eq!(status, Some(1), "{stdout}");
    let record = error_record(&stdout);
    assert_eq!(record["code"], "dm.reference.missing_transclusion", "{record:#}");
    assert_eq!(record["reference"], "./absent.md", "{record:#}");
    assert!(forward(record["resolved"].as_str().unwrap()).ends_with("/cwd/absent.md"), "{record:#}");
    assert_eq!(record["hint"], "create absent.md, or correct the reference", "{record:#}");
    assert_eq!((record["source"]["line"].as_u64(), record["source"]["column"].as_u64()), (Some(5), Some(1)));
    assert_eq!(record["source"]["inserted"], false, "{record:#}");
}

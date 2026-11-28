//! A composition source whose text looks like a glob names that file
//! literally; when it matches nothing, the `claudine compose --dry-run` check
//! reports the miss with biscuit-file's literal-glob hint. A plain missing
//! name and a failure other than a miss do not carry it.

use std::fs;

use serde_json::Value;

use crate::common;
use common::{CliProcessFixture, strip_ansi, write};

/// Text unique to the literal-glob hint.
const HINT: &str = "::file-links";

/// The first diagnostic of the `claudine compose --dry-run --json` report on
/// `reference`, after asserting the check failed with exit 2.
fn check_failure(fixture: &CliProcessFixture, reference: &str) -> Value {
    let output = fixture
        .command()
        .args(["compose", "--dry-run", "--json", reference])
        .output()
        .expect("claudine must run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(2), "{stdout}");
    let report: Value = serde_json::from_str(&stdout).expect("the JSON report parses");
    assert_eq!(report["outcome"], "failure", "{report:#}");
    report["diagnostics"][0].clone()
}

fn hint(diagnostic: &Value) -> &str {
    diagnostic["hint"].as_str().unwrap_or_default()
}

/// A `claude` on the fixture `PATH`, which the check only looks up.
fn install_claude(fixture: &CliProcessFixture) {
    let path = fixture.bin_dir().join(format!("claude{}", std::env::consts::EXE_SUFFIX));
    fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

#[test]
fn a_composition_source_that_looks_like_a_glob_hints_at_glob_forms() {
    let fixture = CliProcessFixture::named("literal-glob-hint");
    fixture.initialize_repository();
    write(&fixture.cwd().join("docs/a.md"), "Alpha body\n");

    let miss = check_failure(&fixture, "docs/*.md");
    assert_eq!(miss["code"], "dm.reference.missing_document", "{miss:#}");
    assert_eq!(miss["failure"], "no-match", "{miss:#}");
    assert!(hint(&miss).contains(HINT), "{miss:#}");

    // A bare glob-looking name is reported with the hint rather than handed
    // to the (here non-interactive) picker.
    let miss = check_failure(&fixture, "*.md");
    assert!(hint(&miss).contains(HINT), "{miss:#}");

    let miss = check_failure(&fixture, "docs/missing.md");
    assert_eq!(miss["failure"], "no-match", "{miss:#}");
    assert!(!hint(&miss).contains(HINT), "a plain miss has no glob hint: {miss:#}");

    let escape = check_failure(&fixture, "../*.md");
    assert_ne!(escape["failure"], "no-match", "{escape:#}");
    assert!(!hint(&escape).contains(HINT), "a failure other than a miss has no glob hint: {escape:#}");

    install_claude(&fixture);
    let output = fixture
        .command()
        .args(["compose", "--dry-run", "--provider", "claude", "docs/a.md"])
        .output()
        .expect("claudine must run");
    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    assert!(output.status.success(), "the control resolves: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("Alpha body"), "{stdout}");
}

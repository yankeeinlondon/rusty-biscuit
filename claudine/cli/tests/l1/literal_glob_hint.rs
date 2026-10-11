//! A composition source whose text looks like a glob names that file
//! literally; when it matches nothing, `claudine compose` reports the miss
//! with biscuit-file's literal-glob hint. A plain missing name and a failure
//! other than a miss do not carry it.

use crate::common;
use common::{CliProcessFixture, strip_ansi, write};

/// Text unique to the literal-glob hint.
const HINT: &str = "::file-links";

/// Runs `claudine compose --dry-run` on `reference`; returns whether it
/// succeeded, its plain stdout, and its plain stderr.
fn compose_dry_run(fixture: &CliProcessFixture, reference: &str) -> (bool, String, String) {
    let output = fixture
        .command()
        // A wide terminal keeps the hint on one line.
        .env("COLUMNS", "500")
        .args(["compose", "--dry-run", reference])
        .output()
        .expect("claudine must run");
    (
        output.status.success(),
        strip_ansi(&String::from_utf8_lossy(&output.stdout)),
        strip_ansi(&String::from_utf8_lossy(&output.stderr)),
    )
}

#[test]
fn a_composition_source_that_looks_like_a_glob_hints_at_glob_forms() {
    let fixture = CliProcessFixture::named("literal-glob-hint");
    fixture.initialize_repository();
    write(&fixture.cwd().join("docs/a.md"), "Alpha body\n");

    let (ok, _, stderr) = compose_dry_run(&fixture, "docs/*.md");
    assert!(!ok, "{stderr}");
    assert!(stderr.contains("failure: no-match"), "{stderr}");
    assert!(stderr.contains(HINT), "{stderr}");

    // A bare glob-looking name is reported with the hint rather than handed
    // to the (here non-interactive) picker.
    let (ok, _, stderr) = compose_dry_run(&fixture, "*.md");
    assert!(!ok, "{stderr}");
    assert!(stderr.contains(HINT), "{stderr}");

    let (ok, _, stderr) = compose_dry_run(&fixture, "docs/missing.md");
    assert!(!ok, "{stderr}");
    assert!(stderr.contains("failure: no-match"), "{stderr}");
    assert!(!stderr.contains(HINT), "a plain miss has no glob hint: {stderr}");

    let (ok, _, stderr) = compose_dry_run(&fixture, "../*.md");
    assert!(!ok, "{stderr}");
    assert!(!stderr.contains("failure: no-match"), "{stderr}");
    assert!(!stderr.contains(HINT), "a failure other than a miss has no glob hint: {stderr}");

    let (ok, stdout, stderr) = compose_dry_run(&fixture, "docs/a.md");
    assert!(ok, "the control resolves: {stderr}");
    assert!(stdout.contains("Alpha body"), "{stdout}");
}

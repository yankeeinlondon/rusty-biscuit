//! A single-file reference whose text looks like a glob reads the wildcards
//! literally; when it then matches nothing, every `md` surface that reports
//! the miss carries biscuit-file's literal-glob hint. A plain missing name and
//! a failure other than a miss do not.

use crate::common::{CliProcessFixture, unwrapped};

/// Text unique to the literal-glob hint.
const HINT: &str = "::file-links";

/// A repository fixture whose `docs/a.md` is the positive control.
fn repository(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    assert!(fixture.initialize_repository(), "git must be available");
    fixture.write_file("cwd/docs/a.md", "---\ntitle: Alpha\n---\n# Alpha\n\n## Section\n");
    fixture
}

/// Runs `md compose` on `argument` from the fixture `cwd`; returns whether it
/// succeeded and its stderr.
fn compose(fixture: &CliProcessFixture, argument: &str) -> (bool, String) {
    let output = fixture
        .command()
        .args(["compose", argument, "--no-baseline-schema"])
        .output()
        .expect("md must run");
    (output.status.success(), unwrapped(&String::from_utf8_lossy(&output.stderr)))
}

/// Writes `body` as `cwd/page.md` and composes it.
fn compose_document(fixture: &CliProcessFixture, body: &str) -> (bool, String) {
    fixture.write_file("cwd/page.md", body);
    compose(fixture, "page.md")
}

#[test]
fn a_file_argument_that_looks_like_a_glob_hints_at_glob_forms() {
    let fixture = repository("literal_glob_hint_argument");

    let (ok, stderr) = compose(&fixture, "docs/*.md");
    assert!(!ok, "{stderr}");
    assert!(stderr.contains("failure:no-match") || stderr.contains("failure: no-match"), "{stderr}");
    assert!(stderr.contains(HINT), "{stderr}");

    let (ok, stderr) = compose(&fixture, "docs/missing.md");
    assert!(!ok, "{stderr}");
    assert!(stderr.contains("failure:no-match") || stderr.contains("failure: no-match"), "{stderr}");
    assert!(!stderr.contains(HINT), "a plain miss has no glob hint: {stderr}");

    let (ok, stderr) = compose(&fixture, "../*.md");
    assert!(!ok, "{stderr}");
    assert!(stderr.contains("failure:invalid-reference") || stderr.contains("failure: invalid-reference"), "{stderr}");
    assert!(!stderr.contains(HINT), "a failure other than a miss has no glob hint: {stderr}");

    let (ok, stderr) = compose(&fixture, "docs/a.md");
    assert!(ok, "the control resolves: {stderr}");
}

#[test]
fn a_file_transclusion_that_looks_like_a_glob_hints_at_glob_forms() {
    let fixture = repository("literal_glob_hint_file");

    let (ok, stderr) = compose_document(&fixture, "# Page\n\n::file ./docs/*.md\n");
    assert!(!ok, "{stderr}");
    assert!(stderr.contains(HINT), "{stderr}");

    let (ok, stderr) = compose_document(&fixture, "# Page\n\n::file ./docs/missing.md\n");
    assert!(!ok, "{stderr}");
    assert!(!stderr.contains(HINT), "{stderr}");

    let (ok, stderr) = compose_document(&fixture, "# Page\n\n::file ./docs/a.md\n");
    assert!(ok, "{stderr}");
}

#[test]
fn a_code_transclusion_that_looks_like_a_glob_hints_at_glob_forms() {
    let fixture = repository("literal_glob_hint_code");

    let (ok, stderr) = compose_document(&fixture, "# Page\n\n::code ./docs/*.md\n");
    assert!(!ok, "{stderr}");
    assert!(stderr.contains(HINT), "{stderr}");

    let (ok, stderr) = compose_document(&fixture, "# Page\n\n::code ./docs/missing.md\n");
    assert!(!ok, "{stderr}");
    assert!(!stderr.contains(HINT), "{stderr}");

    let (ok, stderr) = compose_document(&fixture, "# Page\n\n::code ./docs/a.md\n");
    assert!(ok, "{stderr}");
}

#[test]
fn a_toc_linking_target_that_looks_like_a_glob_hints_at_glob_forms() {
    let fixture = repository("literal_glob_hint_toc");

    let (ok, stderr) = compose_document(&fixture, "# Page\n\n::toc-linking ./docs/*.md\n");
    assert!(!ok, "{stderr}");
    assert!(stderr.contains(HINT), "{stderr}");

    let (ok, stderr) = compose_document(&fixture, "# Page\n\n::toc-linking ./docs/missing.md\n");
    assert!(!ok, "{stderr}");
    assert!(!stderr.contains(HINT), "{stderr}");

    let (ok, stderr) = compose_document(&fixture, "# Page\n\n::toc-linking ./docs/a.md\n");
    assert!(ok, "{stderr}");
}

#[test]
fn a_document_function_argument_that_looks_like_a_glob_hints_at_glob_forms() {
    let fixture = repository("literal_glob_hint_expression");

    let (ok, stderr) = compose_document(&fixture, "# {{ markdown_title('docs/*.md') }}\n");
    assert!(!ok, "{stderr}");
    assert!(stderr.contains(HINT), "{stderr}");

    let (ok, stderr) = compose_document(&fixture, "# {{ markdown_title('docs/missing.md') }}\n");
    assert!(!ok, "{stderr}");
    assert!(!stderr.contains(HINT), "{stderr}");

    let (ok, stderr) = compose_document(&fixture, "# {{ markdown_title('docs/a.md') }}\n");
    assert!(ok, "{stderr}");
}

#[test]
fn a_schema_reference_that_looks_like_a_glob_hints_at_glob_forms() {
    let fixture = repository("literal_glob_hint_schema");

    let (ok, stderr) = compose_document(&fixture, "---\n$schema: ./docs/*.yaml\n---\n# Page\n");
    assert!(!ok, "{stderr}");
    assert!(stderr.contains(HINT), "{stderr}");

    let (ok, stderr) =
        compose_document(&fixture, "---\n$schema: ./docs/missing.yaml\n---\n# Page\n");
    assert!(!ok, "{stderr}");
    assert!(!stderr.contains(HINT), "{stderr}");
}

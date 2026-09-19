mod common;

use common::CliProcessFixture;
use predicates::prelude::*;

// =============================================================================
//                      RM SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_rm_removes_single_property() {
    let fixture = CliProcessFixture::named("rm-test-rm-removes-single-property");
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(
        &file,
        "---\ntitle: Hello\nauthor: Alice\ndate: 2024-01-01\n---\n\n# Content\n",
    )
    .unwrap();

    fixture
        .command()
        .args(["rm", file.to_str().unwrap(), "author"])
        .assert()
        .success();

    let content = std::fs::read_to_string(&file).unwrap();
    assert!(!content.contains("author:"));
    assert!(content.contains("title: Hello"));
    assert!(content.contains("date: 2024-01-01"));
    assert!(content.contains("# Content"));
}

#[test]
fn test_rm_removes_multiple_properties() {
    let fixture = CliProcessFixture::named("rm-test-rm-removes-multiple-properties");
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(
        &file,
        "---\ntitle: Hello\nauthor: Alice\ndate: 2024-01-01\ntags: [rust, cli]\n---\n\n# Content\n",
    )
    .unwrap();

    fixture
        .command()
        .args(["rm", file.to_str().unwrap(), "author", "tags"])
        .assert()
        .success();

    let content = std::fs::read_to_string(&file).unwrap();
    assert!(!content.contains("author:"));
    assert!(!content.contains("tags:"));
    assert!(content.contains("title: Hello"));
    assert!(content.contains("date: 2024-01-01"));
    assert!(content.contains("# Content"));
}

#[test]
fn test_rm_nonexistent_key_fails() {
    let fixture = CliProcessFixture::named("rm-test-rm-nonexistent-key-fails");
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(
        &file,
        "---\ntitle: Hello\nauthor: Alice\n---\n\n# Content\n",
    )
    .unwrap();

    fixture
        .command()
        .args(["rm", file.to_str().unwrap(), "missing"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found in frontmatter"));
}

#[test]
fn test_rm_partial_nonexistent_fails() {
    let fixture = CliProcessFixture::named("rm-test-rm-partial-nonexistent-fails");
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(
        &file,
        "---\ntitle: Hello\nauthor: Alice\n---\n\n# Content\n",
    )
    .unwrap();

    fixture
        .command()
        .args(["rm", file.to_str().unwrap(), "title", "missing"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found in frontmatter"));

    // File should be unchanged when command fails
    let content = std::fs::read_to_string(&file).unwrap();
    assert!(content.contains("title: Hello"));
    assert!(content.contains("author: Alice"));
}

#[test]
fn test_rm_with_json_output() {
    let fixture = CliProcessFixture::named("rm-test-rm-with-json-output");
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(
        &file,
        "---\ntitle: Hello\nauthor: Alice\ndate: 2024-01-01\n---\n\n# Content\n",
    )
    .unwrap();

    let output = fixture
        .command()
        .args(["rm", file.to_str().unwrap(), "author", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = String::from_utf8_lossy(&output);
    assert!(json_str.contains("\"removed\""));
    assert!(json_str.contains("\"remaining\""));
    assert!(json_str.contains("\"filename\""));
    assert!(json_str.contains("\"author\""));
    assert!(json_str.contains("\"title\""));
    assert!(json_str.contains("\"date\""));
}

#[test]
fn test_rm_with_verbose_output() {
    let fixture = CliProcessFixture::named("rm-test-rm-with-verbose-output");
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(
        &file,
        "---\ntitle: Hello\nauthor: Alice\ndate: 2024-01-01\n---\n\n# Content\n",
    )
    .unwrap();

    fixture
        .command()
        .args(["rm", file.to_str().unwrap(), "author", "-v"])
        .assert()
        .success()
        .stderr(predicate::str::contains("removed"));
}

#[test]
fn test_rm_preserves_body_content() {
    let fixture = CliProcessFixture::named("rm-test-rm-preserves-body-content");
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    let body = "# Heading\n\nParagraph with **bold** text.\n\n- list item\n";
    std::fs::write(
        &file,
        format!("---\ntitle: Test\nauthor: Alice\n---\n\n{}", body),
    )
    .unwrap();

    fixture
        .command()
        .args(["rm", file.to_str().unwrap(), "author"])
        .assert()
        .success();

    let content = std::fs::read_to_string(&file).unwrap();
    assert!(content.contains("# Heading"));
    assert!(content.contains("**bold**"));
    assert!(content.contains("- list item"));
}

#[test]
fn test_rm_requires_at_least_one_prop() {
    let fixture = CliProcessFixture::named("rm-test-rm-requires-at-least-one-prop");
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(&file, "---\ntitle: Hello\n---\n\n# Content\n").unwrap();

    fixture
        .command()
        .args(["rm", file.to_str().unwrap()])
        .assert()
        .failure();
}

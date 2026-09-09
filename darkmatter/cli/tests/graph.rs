mod common;

use common::CliProcessFixture;
use predicates::prelude::*;

#[test]
fn test_graph_basic() {
    let fixture = CliProcessFixture::named("test_graph_basic");
    let input = fixture.write_file(
        "cwd/test.md",
        "# Test\n\n[link](https://example.com)\n\n![img](./logo.png)",
    );

    fixture
        .command()
        .arg("graph")
        .arg(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("example.com"))
        .stdout(predicate::str::contains("logo.png"));
}

#[test]
fn test_graph_follow() {
    let fixture = CliProcessFixture::named("test_graph_follow");
    assert!(fixture.initialize_repository());
    let parent = fixture.write_file("cwd/parent.md", "# Parent\n\n::file child.md");
    fixture.write_file(
        "cwd/child.md",
        "# Child\n\n[link](https://child.example.com)",
    );

    fixture
        .command()
        .arg("graph")
        .arg(&parent)
        .arg("--follow")
        .assert()
        .success()
        .stdout(predicate::str::contains("parent.md"))
        .stdout(predicate::str::contains("child.md"));
}

#[test]
fn test_graph_validate_valid() {
    let fixture = CliProcessFixture::named("test_graph_validate_valid");
    assert!(fixture.initialize_repository());
    let md_path = fixture.write_file("cwd/valid.md", "# Valid\n\n[link](./linked.md)");
    fixture.write_file("cwd/linked.md", "# Linked");

    fixture
        .command()
        .arg("graph")
        .arg(&md_path)
        .arg("--validate")
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"))
        .stdout(predicate::str::contains("0 issues"));
}

#[test]
fn test_graph_validate_invalid() {
    let fixture = CliProcessFixture::named("test_graph_validate_invalid");
    assert!(fixture.initialize_repository());
    let input = fixture.write_file("cwd/test.md", "# Test\n\n[broken](./nonexistent.md)\n");

    let output = fixture
        .command()
        .arg("graph")
        .arg(input)
        .arg("--validate")
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit code 2 for validation errors"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[missing]"),
        "expected [missing] suffix in output"
    );
    assert!(
        stdout.contains("1 issues"),
        "expected issue count in summary"
    );
}

#[test]
fn test_graph_follow_toc_linking() {
    let fixture = CliProcessFixture::named("test_graph_follow_toc_linking");
    assert!(fixture.initialize_repository());
    let parent = fixture.write_file("cwd/root.md", "# Root\n\n::toc-linking child.md");
    fixture.write_file(
        "cwd/child.md",
        "# Child\n\n## Section A\n\n## Section B\n\n[link](https://child.example.com)",
    );

    fixture
        .command()
        .arg("graph")
        .arg(&parent)
        .arg("--follow")
        .assert()
        .success()
        .stdout(predicate::str::contains("root.md"))
        .stdout(predicate::str::contains("child.md"))
        .stdout(predicate::str::contains("child.example.com"));
}

#[test]
fn test_graph_follow_validate_child_broken_link() {
    let fixture = CliProcessFixture::named("test_graph_follow_validate_child_broken_link");
    assert!(fixture.initialize_repository());
    let parent = fixture.write_file("cwd/root.md", "# Root\n\n::toc-linking child.md");
    fixture.write_file("cwd/child.md", "# Child\n\n[broken](./missing.md)");

    let output = fixture
        .command()
        .arg("graph")
        .arg(&parent)
        .arg("--follow")
        .arg("--validate")
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit code 2 when followed child has a broken link"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[missing]"),
        "expected [missing] suffix for broken link in child"
    );
}

#[test]
fn test_graph_follow_multiple_prologues() {
    let fixture = CliProcessFixture::named("test_graph_follow_multiple_prologues");
    assert!(fixture.initialize_repository());
    let root = fixture.write_file(
        "cwd/root.md",
        "---\nprologue:\n  - a.md\n  - b.md\n---\n\n# Root",
    );
    fixture.write_file("cwd/a.md", "# A\n\n[a-link](https://a.example.com)");
    fixture.write_file("cwd/b.md", "# B\n\n[b-link](https://b.example.com)");

    fixture
        .command()
        .arg("graph")
        .arg(&root)
        .arg("--follow")
        .assert()
        .success()
        .stdout(predicate::str::contains("a.md"))
        .stdout(predicate::str::contains("b.md"))
        .stdout(predicate::str::contains("a.example.com"))
        .stdout(predicate::str::contains("b.example.com"));
}

#[test]
fn test_graph_follow_epilogue() {
    let fixture = CliProcessFixture::named("test_graph_follow_epilogue");
    assert!(fixture.initialize_repository());
    let root = fixture.write_file(
        "cwd/root.md",
        "---\nepilogue: epilogue.md\n---\n\n# Root\n\n[main](https://main.example.com)",
    );
    fixture.write_file(
        "cwd/epilogue.md",
        "# Epilogue\n\n[epi-link](https://epilogue.example.com)",
    );

    fixture
        .command()
        .arg("graph")
        .arg(&root)
        .arg("--follow")
        .assert()
        .success()
        .stdout(predicate::str::contains("root.md"))
        .stdout(predicate::str::contains("epilogue.md"))
        .stdout(predicate::str::contains("epilogue.example.com"));
}

#[test]
fn test_graph_file_not_found() {
    let fixture = CliProcessFixture::named("test_graph_file_not_found");
    fixture
        .command()
        .arg("graph")
        .arg("/nonexistent/file.md")
        .assert()
        .failure();
}

#[test]
fn test_graph_help() {
    let fixture = CliProcessFixture::named("test_graph_help");
    fixture
        .command()
        .arg("graph")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--follow"))
        .stdout(predicate::str::contains("--validate"));
}

#[test]
fn test_graph_json_output() {
    let fixture = CliProcessFixture::named("test_graph_json_output");
    let file = fixture.write_file("cwd/test.md", "# Test\n\n[link](https://example.com)");

    fixture
        .command()
        .args(["graph", "--json"])
        .arg(&file)
        .assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("\"references\""))
        .stdout(predicate::str::contains("example.com"));
}

// ── JSON baseline fixtures (byte-for-byte compatibility) ────────────────
//
// The Phase 1 leak extraction (CLI Atheist / Leak 3) replaced the
// hand-rolled CLI JSON serializers with `#[derive(serde::Serialize)]`
// on the library reference types. These tests pin the public JSON shape
// of `md graph --json` against the captured baseline fixtures under
// `darkmatter/features/2026-06-17-cli-atheist/baseline/json/`.

use common::baseline;

/// Runs `md graph` with the supplied args, parses stdout as JSON,
/// normalizes temp paths / hash prefixes, and compares the full value
/// against the named baseline fixture.
fn assert_graph_json_matches_baseline(
    fixture: &CliProcessFixture,
    args: &[&str],
    temp_dir: &std::path::Path,
    baseline_name: &str,
) {
    let output = fixture
        .command()
        .args(args)
        .output()
        .expect("md command failed to spawn");

    assert!(
        output.status.success(),
        "md graph --json failed with status {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let actual: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("md graph output must be valid JSON");

    let redact = baseline::paths_to_redact(temp_dir);
    let redact_refs: Vec<&str> = redact.iter().map(|s| s.as_str()).collect();
    let actual_norm = baseline::normalize(actual, &redact_refs);
    let expected_norm = baseline::normalize(baseline::load_json(baseline_name), &redact_refs);

    assert_eq!(
        actual_norm, expected_norm,
        "md graph --json output did not match baseline {baseline_name}\n\
         raw output:\n{stdout}",
    );
}

#[test]
fn graph_json_local_baseline() {
    let fixture = CliProcessFixture::named("graph_json_local_baseline");
    assert!(fixture.initialize_repository());
    let root = fixture.cwd();
    std::fs::write(
        root.join("local.md"),
        "# Local Test\n\n[local link](./other.md)\n![local image](./img.png)\n::file other.md\n",
    )
    .unwrap();
    std::fs::write(root.join("other.md"), "# Other\n").unwrap();
    let local = root.join("local.md");
    assert_graph_json_matches_baseline(
        &fixture,
        &["graph", "--json", local.to_str().unwrap()],
        root,
        "graph_local.json",
    );
}

#[test]
fn graph_json_follow_baseline() {
    let fixture = CliProcessFixture::named("graph_json_follow_baseline");
    assert!(fixture.initialize_repository());
    let root = fixture.cwd();
    std::fs::write(
        root.join("prologue.md"),
        "---\nprologue: other.md\n---\n\n# Prologue\n",
    )
    .unwrap();
    std::fs::write(root.join("other.md"), "# Other\n").unwrap();
    let prologue = root.join("prologue.md");
    assert_graph_json_matches_baseline(
        &fixture,
        &["graph", "--json", "--follow", prologue.to_str().unwrap()],
        root,
        "graph_follow.json",
    );
}

#[test]
fn graph_json_validate_baseline() {
    let fixture = CliProcessFixture::named("graph_json_validate_baseline");
    assert!(fixture.initialize_repository());
    let root = fixture.cwd();
    std::fs::write(
        root.join("errors.md"),
        "# Errors\n\n[missing](./missing.md)\n",
    )
    .unwrap();
    let errors = root.join("errors.md");
    assert_graph_json_matches_baseline(
        &fixture,
        &["graph", "--json", "--validate", errors.to_str().unwrap()],
        root,
        "graph_validate.json",
    );
}

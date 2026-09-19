mod common;

use common::{CliProcessFixture, md_file};
use predicates::prelude::*;

#[test]
fn test_compose_invalid_state() {
    let fixture = CliProcessFixture::named("test_compose_invalid_state");
    fixture
        .command()
        .args(["compose", "-", "--state", "bad json"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid JSON"));
}

#[test]
fn test_compose_state_requires_json_object() {
    let fixture = CliProcessFixture::named("test_compose_state_requires_json_object");
    fixture
        .command()
        .args(["compose", "-", "--state", "[1,2,3]"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected a JSON object"));
}

#[test]
fn test_compose_with_set_overwrites_frontmatter() {
    let fixture = CliProcessFixture::named("test_compose_with_set_overwrites_frontmatter");
    fixture
        .command()
        .args([
            "compose",
            "-",
            "--set",
            r#"{"name":"Bob"}"#,
            "--frontmatter",
        ])
        .write_stdin("---\nname: Alice\n---\n# Hello {{ name }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello Bob"))
        .stdout(predicate::str::contains("name: Bob"));
}

#[test]
fn test_compose_with_set_adds_missing_keys() {
    let fixture = CliProcessFixture::named("test_compose_with_set_adds_missing_keys");
    fixture
        .command()
        .args(["compose", "-", "--set", r#"{"name":"Bob"}"#])
        .write_stdin("# Hello {{ name }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello Bob"));
}

#[test]
fn test_compose_set_and_state_combined() {
    let fixture = CliProcessFixture::named("test_compose_set_and_state_combined");
    // --state fills defaults, --set overwrites; --set wins on overlap
    fixture
        .command()
        .args([
            "compose",
            "-",
            "--state",
            r#"{"greeting":"Hi","name":"Alice"}"#,
            "--set",
            r#"{"name":"Bob"}"#,
        ])
        .write_stdin("# {{ greeting }} {{ name }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hi Bob"));
}

// =============================================================================
//              COMPOSE SHORTHAND SETTER TESTS
// =============================================================================

#[test]
fn test_compose_shorthand_basic_file_input() {
    let fixture = CliProcessFixture::named("test_compose_shorthand_basic_file_input");
    let tmp = md_file("# Hello {{ iteration }}\n");
    fixture
        .command()
        .args(["compose"])
        .arg(tmp.path())
        .arg("iteration=1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello 1"));
}

#[test]
fn test_compose_shorthand_basic_stdin() {
    let fixture = CliProcessFixture::named("test_compose_shorthand_basic_stdin");
    fixture
        .command()
        .args(["compose", "iteration=1"])
        .write_stdin("# Hello {{ iteration }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello 1"));
}

#[test]
fn test_compose_shorthand_multiple_setters_mixed_types() {
    let fixture = CliProcessFixture::named("test_compose_shorthand_multiple_setters_mixed_types");
    fixture
        .command()
        .args(["compose", "iteration=1", "draft=false", "name=Alice"])
        .write_stdin("{{ iteration }} {{ draft }} {{ name }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 false Alice"));
}

#[test]
fn test_compose_shorthand_json5_value() {
    let fixture = CliProcessFixture::named("test_compose_shorthand_json5_value");
    fixture
        .command()
        .args(["compose", r#"meta={author:"Alice"}"#])
        .write_stdin("{{ meta.author }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Alice"));
}

#[test]
fn test_compose_shorthand_participates_in_validation() {
    let fixture = CliProcessFixture::named("test_compose_shorthand_participates_in_validation");
    let temp_dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir(temp_dir.path().join("features")).unwrap();
    std::fs::write(
        temp_dir.path().join("features/my-plan.md"),
        "# My Plan\n\nPlan content here.",
    )
    .unwrap();

    let template_path = temp_dir.path().join("template.md");
    std::fs::write(&template_path, "# Task\n\n::file features/{{plan}}\n").unwrap();

    fixture
        .command()
        .arg("compose")
        .arg(&template_path)
        .arg("plan=my-plan.md")
        .assert()
        .success()
        .stdout(predicate::str::contains("Plan content here."));
}

#[test]
fn test_compose_shorthand_wins_over_state() {
    let fixture = CliProcessFixture::named("test_compose_shorthand_wins_over_state");
    fixture
        .command()
        .args([
            "compose",
            "-",
            "--state",
            r#"{"iteration":0}"#,
            "iteration=1",
        ])
        .write_stdin("{{ iteration }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("1"));
}

#[test]
fn test_compose_shorthand_wins_over_set() {
    let fixture = CliProcessFixture::named("test_compose_shorthand_wins_over_set");
    fixture
        .command()
        .args(["compose", "-", "--set", r#"{"iteration":1}"#, "iteration=2"])
        .write_stdin("{{ iteration }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("2"));
}

#[test]
fn test_compose_shorthand_duplicate_keys_last_write_wins() {
    let fixture = CliProcessFixture::named("test_compose_shorthand_duplicate_keys_last_write_wins");
    fixture
        .command()
        .args(["compose", "iteration=1", "iteration=2"])
        .write_stdin("{{ iteration }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("2"));
}

#[test]
fn test_compose_shorthand_empty_value() {
    let fixture = CliProcessFixture::named("test_compose_shorthand_empty_value");
    fixture
        .command()
        .args(["compose", "empty="])
        .write_stdin("'{{ empty }}'")
        .assert()
        .success()
        .stdout(predicate::str::contains("''"));
}

#[test]
fn test_compose_shorthand_empty_key_errors() {
    let fixture = CliProcessFixture::named("test_compose_shorthand_empty_key_errors");
    fixture
        .command()
        .args(["compose", "=value"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid setter '=value'"));
}

#[test]
fn test_compose_shorthand_numeric_leading_key_is_treated_as_input_path() {
    let fixture = CliProcessFixture::named(
        "test_compose_shorthand_numeric_leading_key_is_treated_as_input_path",
    );
    fixture
        .command()
        .args(["compose", "9key=value"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to load"))
        .stderr(predicate::str::contains("9key=value"));
}

#[test]
fn test_compose_shorthand_setter_before_file_input() {
    let fixture = CliProcessFixture::named("test_compose_shorthand_setter_before_file_input");
    let tmp = md_file("# Hello {{ iteration }}\n");
    fixture
        .command()
        .args(["compose", "iteration=1"])
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello 1"));
}

#[test]
fn test_compose_shorthand_multiple_non_setter_tokens_error() {
    let fixture =
        CliProcessFixture::named("test_compose_shorthand_multiple_non_setter_tokens_error");
    let tmp = md_file("# Test\n");
    fixture
        .command()
        .args(["compose"])
        .arg(tmp.path())
        .arg("other.md")
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected at most one input path"));
}

#[test]
fn test_compose_shorthand_path_escape_hatch() {
    let fixture = CliProcessFixture::named("test_compose_shorthand_path_escape_hatch");
    fixture.write_file("cwd/content.md", "# Content\n");
    let path_str = "./content.md";
    fixture
        .command_builder()
        .ambient_context(fixture.cwd())
        .build()
        .args(["compose"])
        .arg(path_str)
        .arg("key=val")
        .assert()
        .success()
        .stdout(predicate::str::contains("Content"));
}

#[test]
fn test_compose_set_invalid_json() {
    let fixture = CliProcessFixture::named("test_compose_set_invalid_json");
    fixture
        .command()
        .args(["compose", "-", "--set", "bad json"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid JSON"));
}

#[test]
fn test_compose_set_requires_json_object() {
    let fixture = CliProcessFixture::named("test_compose_set_requires_json_object");
    fixture
        .command()
        .args(["compose", "-", "--set", "[1,2,3]"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected a JSON object"));
}

// =============================================================================
//              SET OVERLAY TRANCLUSION CLI TESTS
// =============================================================================

#[test]
fn test_set_overlay_child_interpolation() {
    let fixture = CliProcessFixture::named("test_set_overlay_child_interpolation");
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("parent.md"),
        r#"::file child.md set.name="Bob""#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("child.md"),
        "---\nname: Alice\n---\n\nHello {{ name }}\n",
    )
    .unwrap();

    fixture
        .command()
        .arg("compose")
        .arg(dir.path().join("parent.md"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello Bob"))
        .stdout(predicate::str::contains("Alice").not());
}

#[test]
fn test_set_overlay_strict_rejects_invalid() {
    let fixture = CliProcessFixture::named("test_set_overlay_strict_rejects_invalid");
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("parent.md"), r#"::file child.md set=42"#).unwrap();
    std::fs::write(dir.path().join("child.md"), "body\n").unwrap();

    fixture
        .command()
        .arg("compose")
        .arg(dir.path().join("parent.md"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid frontmatter assignment"));
}

#[test]
fn test_set_overlay_permissive_invalid_warns() {
    let fixture = CliProcessFixture::named("test_set_overlay_permissive_invalid_warns");
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("parent.md"),
        r#"::file child.md set=42 set.name="Bob""#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("child.md"),
        "---\nname: Alice\n---\n\n{{ name }}\n",
    )
    .unwrap();

    fixture
        .command()
        .arg("compose")
        .arg(dir.path().join("parent.md"))
        .arg("--allow-invalid-frontmatter-assignment")
        .assert()
        .success()
        .stdout(predicate::str::contains("Bob"))
        .stdout(predicate::str::contains("Alice").not());
}

#[test]
fn test_set_overlay_strict_rejects_reassigned() {
    let fixture = CliProcessFixture::named("test_set_overlay_strict_rejects_reassigned");
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("parent.md"),
        r#"::file child.md set.name="Bob" set.name="Mary""#,
    )
    .unwrap();
    std::fs::write(dir.path().join("child.md"), "---\n---\nbody\n").unwrap();

    fixture
        .command()
        .arg("compose")
        .arg(dir.path().join("parent.md"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("duplicate frontmatter property"));
}

#[test]
fn test_set_overlay_permissive_reassigned_warns_and_rightmost_wins() {
    let fixture =
        CliProcessFixture::named("test_set_overlay_permissive_reassigned_warns_and_rightmost_wins");
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("parent.md"),
        r#"::file child.md set.name="Bob" set.name="Mary""#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("child.md"),
        "---\nname: Alice\n---\n\n{{ name }}\n",
    )
    .unwrap();

    fixture
        .command()
        .arg("compose")
        .arg(dir.path().join("parent.md"))
        .arg("--allow-reassigned-frontmatter-property")
        .assert()
        .success()
        .stdout(predicate::str::contains("Mary"))
        .stdout(predicate::str::contains("Bob").not())
        .stdout(predicate::str::contains("Alice").not());
}

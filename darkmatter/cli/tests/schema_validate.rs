//! Integration tests for `md schema validate`.

use predicates::prelude::*;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn md_cmd() -> assert_cmd::Command {
    assert_cmd::Command::cargo_bin("md").unwrap()
}

fn write_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

#[test]
fn schema_validate_valid_inline_succeeds() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "post.md",
        "---\n$schema:\n  title: 'string(required)'\ntitle: Hello\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
}

#[test]
fn schema_validate_failing_returns_exit_1() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "draft.md",
        "---\n$schema:\n  title: 'string(required)'\nother: stuff\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1);
}

#[test]
fn schema_validate_json_format_emits_ndjson() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "post.md",
        "---\n$schema:\n  title: 'string(required)'\ntitle: Hello\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate", "--format", "json"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"valid\":true"))
        .stdout(predicate::str::contains("\"problems\":[]"));
}

#[test]
fn schema_validate_json_format_reports_problems() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "draft.md",
        "---\n$schema:\n  title: 'string(required)'\nother: stuff\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate", "--format", "json"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\"valid\":false"))
        .stdout(predicate::str::contains("\"problems\""));
}

#[test]
fn schema_validate_no_schema_no_baseline_is_vacuous_success() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(&tmp, "no-schema.md", "---\nname: alice\n---\nBody\n");

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .success();
}

#[test]
fn schema_validate_quiet_suppresses_success_lines() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "post.md",
        "---\n$schema:\n  title: 'string(required)'\ntitle: Hello\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate", "--quiet"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

/// Advisories name the referenced file in its resolved spelling, which differs
/// from the temp path a test wrote on Windows (8.3 `RUNNER~1` versus the long
/// name) and macOS (`/var` versus `/private/var`).
fn resolved_display(path: &Path) -> String {
    biscuit_file::canonicalize_simplified(path)
        .unwrap()
        .display()
        .to_string()
}

#[test]
fn schema_validate_pretty_reports_bare_sidecar_advisory_without_failing() {
    let tmp = TempDir::new().unwrap();
    let sidecar = write_file(
        &tmp,
        "schema.yaml",
        "source_marker: string(required)\nspec: 'file(eager; required)'\ncaller_spec: 'file(eager; required)'\n",
    );
    let doc = write_file(
        &tmp,
        "doc.md",
        "---\n$schema: ./schema.yaml\ntitle: Hello\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::contains("darkmatter.schema"))
        .stdout(predicate::str::contains(
            "dm.schema.missing_simplified_envelope",
        ))
        .stdout(predicate::str::contains(resolved_display(&sidecar)))
        .stdout(predicate::str::contains(
            "looks like a SimplifiedSchema but has no envelope",
        ))
        .stdout(predicate::str::contains("root `$schema:` key"))
        .stdout(predicate::str::contains("`kind: schema` + `types:`"));
}

#[test]
fn schema_validate_json_reports_structured_bare_sidecar_advisory() {
    let tmp = TempDir::new().unwrap();
    let sidecar = write_file(
        &tmp,
        "schema.yaml",
        "source_marker: string(required)\nspec: 'file(eager; required)'\ncaller_spec: 'file(eager; required)'\n",
    );
    let doc = write_file(
        &tmp,
        "doc.md",
        "---\n$schema: ./schema.yaml\ntitle: Hello\n---\nBody\n",
    );

    let output = md_cmd()
        .args(["schema", "validate", "--format", "json"])
        .arg(&doc)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(value["valid"], true);
    assert_eq!(value["problems"], serde_json::json!([]));
    assert_eq!(value["warnings"].as_array().unwrap().len(), 1);
    let warning = &value["warnings"][0];
    assert_eq!(warning["source"], "darkmatter.schema");
    assert_eq!(
        warning["code"],
        "dm.schema.missing_simplified_envelope"
    );
    assert_eq!(warning["path"], resolved_display(&sidecar));
    assert!(
        warning["message"]
            .as_str()
            .unwrap()
            .contains("looks like a SimplifiedSchema but has no envelope")
    );
}

#[test]
fn schema_validate_quiet_suppresses_bare_sidecar_advisory() {
    let tmp = TempDir::new().unwrap();
    write_file(
        &tmp,
        "schema.yaml",
        "source_marker: string(required)\nspec: 'file(eager; required)'\ncaller_spec: 'file(eager; required)'\n",
    );
    let doc = write_file(
        &tmp,
        "doc.md",
        "---\n$schema: ./schema.yaml\ntitle: Hello\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate", "--quiet"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn schema_validate_baseline_from_flag() {
    let tmp = TempDir::new().unwrap();
    let baseline = write_file(
        &tmp,
        "baseline.yaml",
        "$schema:\n  owner: 'string(required)'\n",
    );
    let doc = write_file(&tmp, "doc.md", "---\ntitle: hi\n---\nBody\n");

    md_cmd()
        .args(["schema", "validate", "--schema"])
        .arg(&baseline)
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("owner"));
}

#[test]
fn schema_validate_baseline_from_env_var() {
    let tmp = TempDir::new().unwrap();
    let baseline = write_file(
        &tmp,
        "baseline.yaml",
        "$schema:\n  owner: 'string(required)'\n",
    );
    let doc = write_file(&tmp, "doc.md", "---\ntitle: hi\n---\nBody\n");

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .env("BASELINE_SCHEMA", &baseline)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("owner"));
}

#[test]
fn schema_validate_bad_baseline_exits_2() {
    let tmp = TempDir::new().unwrap();
    let bad = write_file(&tmp, "baseline.yaml", "not: a-schema\n");
    let doc = write_file(&tmp, "doc.md", "---\ntitle: hi\n---\nBody\n");

    md_cmd()
        .args(["schema", "validate", "--schema"])
        .arg(&bad)
        .arg(&doc)
        .assert()
        .code(2);
}

#[test]
fn schema_validate_unparseable_frontmatter_exits_3() {
    let tmp = TempDir::new().unwrap();
    // Intentionally malformed YAML inside frontmatter delimiters.
    let doc = write_file(
        &tmp,
        "bad.md",
        "---\n: : : not valid yaml ::\n  - [unbalanced\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(3);
}

#[test]
fn schema_validate_pretty_reports_line_for_type_mismatch() {
    let tmp = TempDir::new().unwrap();
    // `rating` lands on line 3 of the canonical re-serialised frontmatter.
    let doc = write_file(
        &tmp,
        "draft.md",
        "---\n$schema:\n  rating: number\nrating: nope\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("at line"))
        .stdout(predicate::str::contains("of frontmatter"));
}

#[test]
fn schema_validate_json_reports_arm_index_for_root_union() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "draft.md",
        "---\n$schema:\n  - title: 'string(required)'\n  - name: 'string(required)'\nother: value\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate", "--format", "json"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\"arm_index\":0"));
}

#[test]
fn schema_validate_unresolved_document_schema_exits_2() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "post.md",
        "---\n$schema: ./missing.yaml\ntitle: hi\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(2);
}

#[test]
fn schema_validate_unresolved_document_schema_outranks_validation_failure() {
    let tmp = TempDir::new().unwrap();
    // One file has a missing `$schema` reference (schema-load error → 2);
    // the other has a normal validation failure (→ 1). Schema-load errors
    // outrank validation failures so the overall exit code must be 2.
    let bad_schema = write_file(
        &tmp,
        "bad_schema.md",
        "---\n$schema: ./missing.yaml\ntitle: hi\n---\nBody\n",
    );
    let bad_value = write_file(
        &tmp,
        "bad_value.md",
        "---\n$schema:\n  title: 'string(required)'\nother: stuff\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&bad_schema)
        .arg(&bad_value)
        .assert()
        .code(2);
}

#[test]
fn schema_validate_pretty_prefixes_root_union_problems_with_arm_index() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "draft.md",
        "---\n$schema:\n  - title: 'string(required)'\n  - name: 'string(required)'\nother: value\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("arm["));
}

#[test]
fn schema_validate_pretty_does_not_render_root_label_as_markup() {
    let tmp = TempDir::new().unwrap();
    // Missing-required at the root produced `<root>` markup which the
    // Prose renderer interpreted as a tag. The rendered output must not
    // leak a closing `</root>` (or any other angle-bracketed artifact).
    let doc = write_file(
        &tmp,
        "draft.md",
        "---\n$schema:\n  title: 'string(required)'\nother: stuff\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("</root>").not())
        .stdout(predicate::str::contains("<root>").not());
}

#[test]
fn schema_validate_pretty_reports_source_line_for_problem() {
    let tmp = TempDir::new().unwrap();
    // The opening `---` is line 1. `$schema:` is line 2, the inline
    // mapping spans lines 3-4, the blank comment is line 5, and the
    // invalid `rating: nope` value is on line 6 of the source. The
    // position must be reported against the original source, not against
    // a re-serialised view.
    let doc = write_file(
        &tmp,
        "draft.md",
        "---\n$schema:\n  rating: number\n# important: do not reorder\n\nrating: nope\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate", "--format", "json"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\"line\":6"));
}

#[test]
fn schema_validate_multiple_files_aggregates_failure() {
    let tmp = TempDir::new().unwrap();
    let good = write_file(
        &tmp,
        "good.md",
        "---\n$schema:\n  title: 'string(required)'\ntitle: ok\n---\n",
    );
    let bad = write_file(
        &tmp,
        "bad.md",
        "---\n$schema:\n  title: 'string(required)'\nother: stuff\n---\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&good)
        .arg(&bad)
        .assert()
        .code(1);
}

#[test]
fn schema_validate_assignment_satisfies_required_property() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "draft.md",
        "---\n$schema:\n  title: 'string(required)'\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .arg("title=Hello")
        .assert()
        .success();
}

#[test]
fn schema_validate_assignment_parses_yaml_scalar_types() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "post.md",
        "---\n$schema:\n  count: 'number(integer; required)'\n  flag: 'boolean(required)'\n---\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .arg("count=5")
        .arg("flag=true")
        .assert()
        .success();
}

#[test]
fn schema_validate_assignment_parses_flow_sequence() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "post.md",
        "---\n$schema:\n  tags: 'string[](min(2); required)'\n---\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .arg("tags=[a, b, c]")
        .assert()
        .success();
}

#[test]
fn schema_validate_assignment_overrides_existing_value() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "post.md",
        "---\n$schema:\n  title: 'string(min(5))'\ntitle: Hi\n---\n",
    );

    // Without override, "Hi" fails min(5).
    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1);

    // Overriding with a longer value makes the document valid.
    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .arg("title=HelloWorld")
        .assert()
        .success();
}

#[test]
fn schema_validate_assignment_nested_dot_notation() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "post.md",
        "---\n$schema:\n  user: 'object(required)'\n---\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .arg("user.email=ken@ken.net")
        .arg("user.name=Ken")
        .assert()
        .success();
}

#[test]
fn schema_validate_assignment_applies_to_multiple_files() {
    let tmp = TempDir::new().unwrap();
    let a = write_file(
        &tmp,
        "a.md",
        "---\n$schema:\n  title: 'string(required)'\n---\n",
    );
    let b = write_file(
        &tmp,
        "b.md",
        "---\n$schema:\n  title: 'string(required)'\n---\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&a)
        .arg(&b)
        .arg("title=shared")
        .assert()
        .success();
}

#[test]
fn schema_validate_assignment_failing_value_still_exits_1() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "post.md",
        "---\n$schema:\n  count: 'number(min(10); required)'\n---\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .arg("count=3")
        .assert()
        .code(1);
}

#[test]
fn schema_validate_invalid_assignment_yaml_returns_usage_error() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(&tmp, "post.md", "---\n$schema:\n  title: 'string'\n---\n");

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        // Unclosed flow mapping is invalid YAML and is reported as a usage
        // error rather than silently treated as a file path.
        .arg("user={broken")
        .assert()
        .code(64);
}

#[test]
fn schema_validate_assignment_coerces_to_string_for_string_typed_property() {
    // Regression: `bar=true` against `bar: string(required)` used to be
    // parsed as a YAML boolean and fail validation. The CLI now consults
    // the schema and stores the raw RHS as a string when the property is
    // declared as a string-shaped scalar.
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "needs-bar.md",
        "---\n$schema:\n  bar: 'string(required)'\n---\nBody\n",
    );

    for rhs in ["true", "false", "42"] {
        md_cmd()
            .args(["schema", "validate"])
            .arg(&doc)
            .arg(format!("bar={rhs}"))
            .assert()
            .success();
    }
}

#[test]
fn schema_validate_assignment_keeps_boolean_for_boolean_typed_property() {
    // Counterpart to the coercion test: when the schema declares a boolean
    // property, a bare `flag=true` still parses as a YAML boolean and the
    // document validates.
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "needs-flag.md",
        "---\n$schema:\n  flag: 'boolean(required)'\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .arg("flag=true")
        .assert()
        .success();
}

#[test]
fn schema_validate_pretty_surfaces_property_description() {
    // Track A: a `-> {description}` arrow on the failing property surfaces as
    // a dimmed sub-line beneath the problem bullet.
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "post.md",
        "---\n$schema:\n  title: 'string(required) -> The headline shown in listing pages'\nother: stuff\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "The headline shown in listing pages",
        ));
}

#[test]
fn schema_validate_pretty_omits_sub_line_when_no_description() {
    // Track A: a description-less schema renders no description text — the
    // feature is purely additive (Decision #8).
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "draft.md",
        "---\n$schema:\n  rating: number\nrating: nope\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("rating"))
        .stdout(predicate::str::contains("The headline").not());
}

#[test]
fn schema_validate_json_carries_description_field() {
    // Track B: the JSON problem object gains a `"description"` field with the
    // declared description string.
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "post.md",
        "---\n$schema:\n  title: 'string(required) -> The headline shown in listing pages'\nother: stuff\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate", "--format", "json"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "\"description\":\"The headline shown in listing pages\"",
        ));
}

#[test]
fn schema_validate_json_description_is_null_when_absent() {
    // Track B: the JSON problem object carries `"description":null` when the
    // property declares no description.
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "draft.md",
        "---\n$schema:\n  rating: number\nrating: nope\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate", "--format", "json"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\"description\":null"));
}

#[test]
fn schema_validate_path_with_equals_disambiguated_by_dot_slash() {
    // A file literally named `weird=name.md` would otherwise look like an
    // assignment, but the `./` prefix forces it to be classified as a file
    // because the LHS-before-`=` is not a valid identifier.
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("weird=name.md");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(b"---\n$schema:\n  title: 'string(required)'\ntitle: ok\n---\n")
        .unwrap();

    md_cmd()
        .current_dir(tmp.path())
        .args(["schema", "validate", "./weird=name.md"])
        .assert()
        .success();
}

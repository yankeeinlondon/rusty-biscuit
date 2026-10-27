//! Integration tests for `md schema validate`.

use crate::common;

use common::CliProcessFixture;
use predicates::prelude::*;
use std::io::Write;
use std::path::{Path, PathBuf};

fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

#[test]
fn schema_validate_valid_inline_succeeds() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "post.md",
        "---\n$schema:\n  title: 'string(required)'\ntitle: Hello\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
}

#[test]
fn schema_validate_failing_returns_exit_1() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "draft.md",
        "---\n$schema:\n  title: 'string(required)'\nother: stuff\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1);
}

#[test]
fn schema_validate_json_format_emits_ndjson() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "post.md",
        "---\n$schema:\n  title: 'string(required)'\ntitle: Hello\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate", "--format", "json"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"valid\":true"))
        .stdout(predicate::str::contains("\"problems\":[]"));
}

#[test]
fn schema_validate_json_format_reports_problems() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "draft.md",
        "---\n$schema:\n  title: 'string(required)'\nother: stuff\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate", "--format", "json"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\"valid\":false"))
        .stdout(predicate::str::contains("\"problems\""));
}

#[test]
fn schema_validate_no_schema_with_baseline_opt_out_is_vacuous_success() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    // `last_updated` would fail the embedded baseline's `date`; the opt-out
    // leaves no effective schema to check it.
    let doc = write_file(tmp, "no-schema.md", "---\nname: alice\nlast_updated: not-a-date\n---\nBody\n");

    process
        .command()
        .args(["schema", "validate", "--disable", "baseline-schema"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::contains("doesn't have a schema definition"));
}

#[test]
fn schema_validate_quiet_suppresses_success_lines() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "post.md",
        "---\n$schema:\n  title: 'string(required)'\ntitle: Hello\n---\nBody\n",
    );

    process
        .command()
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
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let sidecar = write_file(
        tmp,
        "schema.yaml",
        "source_marker: string(required)\nspec: 'file(eager; required)'\ncaller_spec: 'file(eager; required)'\n",
    );
    let doc = write_file(
        tmp,
        "doc.md",
        "---\n$schema: ./schema.yaml\ntitle: Hello\n---\nBody\n",
    );

    process
        .command()
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
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let sidecar = write_file(
        tmp,
        "schema.yaml",
        "source_marker: string(required)\nspec: 'file(eager; required)'\ncaller_spec: 'file(eager; required)'\n",
    );
    let doc = write_file(
        tmp,
        "doc.md",
        "---\n$schema: ./schema.yaml\ntitle: Hello\n---\nBody\n",
    );

    let output = process
        .command()
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
    assert_eq!(warning["code"], "dm.schema.missing_simplified_envelope");
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
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    write_file(
        tmp,
        "schema.yaml",
        "source_marker: string(required)\nspec: 'file(eager; required)'\ncaller_spec: 'file(eager; required)'\n",
    );
    let doc = write_file(
        tmp,
        "doc.md",
        "---\n$schema: ./schema.yaml\ntitle: Hello\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate", "--quiet"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn schema_validate_baseline_from_flag() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let baseline = write_file(
        tmp,
        "baseline.yaml",
        "$schema:\n  owner: 'string(required)'\n",
    );
    let doc = write_file(tmp, "doc.md", "---\ntitle: hi\n---\nBody\n");

    process
        .command()
        .args(["schema", "validate", "--schema"])
        .arg(&baseline)
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("owner"));
}

#[test]
fn schema_validate_baseline_from_env_var() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let baseline = write_file(
        tmp,
        "baseline.yaml",
        "$schema:\n  owner: 'string(required)'\n",
    );
    let doc = write_file(tmp, "doc.md", "---\ntitle: hi\n---\nBody\n");

    process
        .command_builder()
        .application_input("BASELINE_SCHEMA", &baseline)
        .build()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("owner"));
}

#[test]
fn schema_validate_bad_baseline_exits_2() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let bad = write_file(tmp, "baseline.yaml", "not: a-schema\n");
    let doc = write_file(tmp, "doc.md", "---\ntitle: hi\n---\nBody\n");

    process
        .command()
        .args(["schema", "validate", "--schema"])
        .arg(&bad)
        .arg(&doc)
        .assert()
        .code(2);
}

#[test]
fn schema_validate_unparseable_frontmatter_exits_3() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    // Intentionally malformed YAML inside frontmatter delimiters.
    let doc = write_file(
        tmp,
        "bad.md",
        "---\n: : : not valid yaml ::\n  - [unbalanced\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(3);
}

#[test]
fn schema_validate_pretty_reports_line_and_column_for_type_mismatch() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    // The `nope` value starts on document line 4, byte column 9.
    let doc = write_file(
        tmp,
        "draft.md",
        "---\n$schema:\n  rating: number\nrating: nope\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("(at line 4, column 9)"));
}

#[test]
fn schema_validate_json_reports_arm_index_for_root_union() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "draft.md",
        "---\n$schema:\n  - title: 'string(required)'\n  - name: 'string(required)'\nother: value\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate", "--format", "json"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\"arm_index\":0"));
}

#[test]
fn schema_validate_unresolved_document_schema_exits_2() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "post.md",
        "---\n$schema: ./missing.yaml\ntitle: hi\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(2);
}

#[test]
fn schema_validate_unresolved_document_schema_outranks_validation_failure() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    // One file has a missing `$schema` reference (schema-load error → 2);
    // the other has a normal validation failure (→ 1). Schema-load errors
    // outrank validation failures so the overall exit code must be 2.
    let bad_schema = write_file(
        tmp,
        "bad_schema.md",
        "---\n$schema: ./missing.yaml\ntitle: hi\n---\nBody\n",
    );
    let bad_value = write_file(
        tmp,
        "bad_value.md",
        "---\n$schema:\n  title: 'string(required)'\nother: stuff\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate"])
        .arg(&bad_schema)
        .arg(&bad_value)
        .assert()
        .code(2);
}

#[test]
fn schema_validate_pretty_prefixes_root_union_problems_with_arm_index() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "draft.md",
        "---\n$schema:\n  - title: 'string(required)'\n  - name: 'string(required)'\nother: value\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("arm["));
}

#[test]
fn schema_validate_pretty_does_not_render_root_label_as_markup() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    // Missing-required at the root produced `<root>` markup which the
    // Prose renderer interpreted as a tag. The rendered output must not
    // leak a closing `</root>` (or any other angle-bracketed artifact).
    let doc = write_file(
        tmp,
        "draft.md",
        "---\n$schema:\n  title: 'string(required)'\nother: stuff\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("</root>").not())
        .stdout(predicate::str::contains("<root>").not());
}

#[test]
fn schema_validate_pretty_reports_source_line_for_problem() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    // The opening `---` is line 1. `$schema:` is line 2, the inline
    // mapping spans lines 3-4, the blank comment is line 5, and the
    // invalid `rating: nope` value is on line 6 of the source. The
    // position must be reported against the original source, not against
    // a re-serialised view.
    let doc = write_file(
        tmp,
        "draft.md",
        "---\n$schema:\n  rating: number\n# important: do not reorder\n\nrating: nope\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate", "--format", "json"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\"line\":6"));
}

#[test]
fn schema_validate_multiple_files_aggregates_failure() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let good = write_file(
        tmp,
        "good.md",
        "---\n$schema:\n  title: 'string(required)'\ntitle: ok\n---\n",
    );
    let bad = write_file(
        tmp,
        "bad.md",
        "---\n$schema:\n  title: 'string(required)'\nother: stuff\n---\n",
    );

    process
        .command()
        .args(["schema", "validate"])
        .arg(&good)
        .arg(&bad)
        .assert()
        .code(1);
}

#[test]
fn schema_validate_assignment_satisfies_required_property() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "draft.md",
        "---\n$schema:\n  title: 'string(required)'\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .arg("title=Hello")
        .assert()
        .success();
}

#[test]
fn schema_validate_assignment_reads_digits_as_alphanumeric_text() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "slug.md",
        "---\n$schema:\n  slug: 'alphanumeric(required)'\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .arg("slug=123")
        .assert()
        .success();
    // A native number in the document itself is still a type mismatch.
    let native = write_file(
        tmp,
        "native.md",
        "---\n$schema:\n  slug: 'alphanumeric(required)'\nslug: 123\n---\nBody\n",
    );
    process
        .command()
        .args(["schema", "validate"])
        .arg(&native)
        .assert()
        .code(1);
}

#[test]
fn schema_validate_assignment_parses_yaml_scalar_types() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "post.md",
        "---\n$schema:\n  count: 'number(integer; required)'\n  flag: 'boolean(required)'\n---\n",
    );

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .arg("count=5")
        .arg("flag=true")
        .assert()
        .success();
}

#[test]
fn schema_validate_assignment_parses_flow_sequence() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "post.md",
        "---\n$schema:\n  tags: 'string[](min(2); required)'\n---\n",
    );

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .arg("tags=[a, b, c]")
        .assert()
        .success();
}

#[test]
fn schema_validate_assignment_overrides_existing_value() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "post.md",
        "---\n$schema:\n  title: 'string(min(5))'\ntitle: Hi\n---\n",
    );

    // Without override, "Hi" fails min(5).
    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1);

    // Overriding with a longer value makes the document valid.
    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .arg("title=HelloWorld")
        .assert()
        .success();
}

#[test]
fn schema_validate_assignment_nested_dot_notation() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "post.md",
        "---\n$schema:\n  user: 'object(required)'\n---\n",
    );

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .arg("user.email=ken@ken.net")
        .arg("user.name=Ken")
        .assert()
        .success();
}

#[test]
fn schema_validate_assignment_applies_to_multiple_files() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let a = write_file(
        tmp,
        "a.md",
        "---\n$schema:\n  title: 'string(required)'\n---\n",
    );
    let b = write_file(
        tmp,
        "b.md",
        "---\n$schema:\n  title: 'string(required)'\n---\n",
    );

    process
        .command()
        .args(["schema", "validate"])
        .arg(&a)
        .arg(&b)
        .arg("title=shared")
        .assert()
        .success();
}

#[test]
fn schema_validate_assignment_failing_value_still_exits_1() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "post.md",
        "---\n$schema:\n  count: 'number(min(10); required)'\n---\n",
    );

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .arg("count=3")
        .assert()
        .code(1);
}

#[test]
fn schema_validate_invalid_assignment_yaml_returns_usage_error() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let doc = write_file(tmp, "post.md", "---\n$schema:\n  title: 'string'\n---\n");

    process
        .command()
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
    let process = CliProcessFixture::new();
    // Regression: `bar=true` against `bar: string(required)` used to be
    // parsed as a YAML boolean and fail validation. The CLI now consults
    // the schema and stores the raw RHS as a string when the property is
    // declared as a string-shaped scalar.
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "needs-bar.md",
        "---\n$schema:\n  bar: 'string(required)'\n---\nBody\n",
    );

    for rhs in ["true", "false", "42"] {
        process
            .command()
            .args(["schema", "validate"])
            .arg(&doc)
            .arg(format!("bar={rhs}"))
            .assert()
            .success();
    }
}

#[test]
fn schema_validate_assignment_keeps_boolean_for_boolean_typed_property() {
    let process = CliProcessFixture::new();
    // Counterpart to the coercion test: when the schema declares a boolean
    // property, a bare `flag=true` still parses as a YAML boolean and the
    // document validates.
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "needs-flag.md",
        "---\n$schema:\n  flag: 'boolean(required)'\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .arg("flag=true")
        .assert()
        .success();
}

#[test]
fn schema_validate_pretty_surfaces_property_description() {
    let process = CliProcessFixture::new();
    // Track A: a `-> {description}` arrow on the failing property surfaces as
    // a dimmed sub-line beneath the problem bullet.
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "post.md",
        "---\n$schema:\n  title: 'string(required) -> The headline shown in listing pages'\nother: stuff\n---\nBody\n",
    );

    process
        .command()
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
    let process = CliProcessFixture::new();
    // Track A: a description-less schema renders no description text — the
    // feature is purely additive (Decision #8).
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "draft.md",
        "---\n$schema:\n  rating: number\nrating: nope\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("rating"))
        .stdout(predicate::str::contains("The headline").not());
}

#[test]
fn schema_validate_json_carries_description_field() {
    let process = CliProcessFixture::new();
    // Track B: the JSON problem object gains a `"description"` field with the
    // declared description string.
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "post.md",
        "---\n$schema:\n  title: 'string(required) -> The headline shown in listing pages'\nother: stuff\n---\nBody\n",
    );

    process
        .command()
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
    let process = CliProcessFixture::new();
    // Track B: the JSON problem object carries `"description":null` when the
    // property declares no description.
    let tmp = process.cwd();
    let doc = write_file(
        tmp,
        "draft.md",
        "---\n$schema:\n  rating: number\nrating: nope\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate", "--format", "json"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\"description\":null"));
}

#[test]
fn schema_validate_path_with_equals_disambiguated_by_dot_slash() {
    let process = CliProcessFixture::new();
    // A file literally named `weird=name.md` would otherwise look like an
    // assignment, but the `./` prefix forces it to be classified as a file
    // because the LHS-before-`=` is not a valid identifier.
    let path = process.cwd().join("weird=name.md");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(b"---\n$schema:\n  title: 'string(required)'\ntitle: ok\n---\n")
        .unwrap();

    process
        .command()
        .args(["schema", "validate", "./weird=name.md"])
        .assert()
        .success();
}

/// Every row of the contested-`match` table through `md schema validate`.
///
/// Only the `fix` arm requires `severity`, so a missing `severity` failing a
/// `fixes/…` path proves the `feature` arm was ruled out by its glob.
#[test]
fn schema_validate_contested_file_match_selects_and_rejects_union_arms() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    for tree in ["features", "fixes"] {
        std::fs::create_dir_all(tmp.join(tree).join("x")).unwrap();
        write_file(&tmp.join(tree).join("x"), "spec.md", "# Spec\n");
    }
    let single = write_file(
        tmp,
        "single.md",
        "---\n$schema:\n  spec: 'file(required; eager; match(**/fixes/**/spec.md))'\n---\nBody\n",
    );
    let union = "$schema:\n  - kind: 'literal(feature)'\n    spec: 'file(required; eager; match(**/features/**/spec.md))'\n  - kind: 'literal(fix)'\n    spec: 'file(required; eager; match(**/fixes/**/spec.md))'\n    severity: 'string(required)'\n";
    let settled = write_file(
        tmp,
        "settled.md",
        &format!("---\n{union}kind: fix\nseverity: high\n---\nBody\n"),
    );
    let undecided = write_file(tmp, "undecided.md", &format!("---\n{union}---\nBody\n"));

    let validate = |doc: &Path, spec: &str| {
        let output = process
            .command()
            .args(["schema", "validate", "--disable", "schema-triggers", "--format", "json"])
            .arg(doc)
            .arg(format!("spec={spec}"))
            .output()
            .unwrap();
        (output.status.code(), String::from_utf8_lossy(&output.stdout).into_owned())
    };

    for (doc, spec) in [
        (&single, "fixes/x/spec.md"),
        (&single, "features/x/spec.md"),
        (&settled, "fixes/x/spec.md"),
        (&undecided, "features/x/spec.md"),
    ] {
        let (code, stdout) = validate(doc, spec);
        assert_eq!(code, Some(0), "{} with {spec}: {stdout}", doc.display());
        assert!(stdout.contains("\"valid\":true"), "{stdout}");
    }

    let (code, stdout) = validate(&settled, "features/x/spec.md");
    assert_eq!(code, Some(1), "{stdout}");
    assert!(stdout.contains("match(**/fixes/**/spec.md)"), "{stdout}");

    let (code, stdout) = validate(&undecided, "fixes/x/spec.md");
    assert_eq!(code, Some(1), "{stdout}");
    assert!(stdout.contains("severity"), "{stdout}");
    assert!(!stdout.contains("match("), "the feature arm is ruled out: {stdout}");
}

#[test]
fn schema_validate_enforces_each_root_union_arm_match() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    for tree in ["features", "fixes"] {
        std::fs::create_dir_all(tmp.join(tree).join("x")).unwrap();
        write_file(&tmp.join(tree).join("x"), "spec.md", "# Spec\n");
    }
    write_file(
        tmp,
        "raw.json",
        r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","properties":{"kind":{"const":"other"}},"required":["kind"]}"#,
    );
    for (name, schema, matching, outside) in [
        (
            "identical",
            "$schema:\n  - kind: 'literal(fix)'\n    spec: 'file(required; eager; match(**/fixes/**/spec.md))'\n  - kind: 'literal(other)'\n    spec: 'file(required; eager; match(**/fixes/**/spec.md))'\nkind: fix\n",
            "fixes/x/spec.md",
            "features/x/spec.md",
        ),
        (
            "one-declaration",
            "$schema:\n  - kind: 'literal(feature)'\n    spec: 'file(required; eager; match(**/fixes/**/spec.md))'\n  - kind: 'literal(fix)'\nkind: feature\n",
            "fixes/x/spec.md",
            "features/x/spec.md",
        ),
        (
            "mixed-schema",
            "$schema:\n  - kind: 'literal(fix)'\n    spec: 'file(required; eager; match(**/fixes/**/spec.md))'\n  - ./raw.json\nkind: fix\n",
            "fixes/x/spec.md",
            "features/x/spec.md",
        ),
    ] {
        let doc = write_file(tmp, &format!("{name}.md"), &format!("---\n{schema}---\nBody\n"));
        for (spec, accepted) in [(matching, true), (outside, false)] {
            let output = process
                .command()
                .args(["schema", "validate", "--disable", "schema-triggers", "--format", "json"])
                .arg(&doc)
                .arg(format!("spec={spec}"))
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert_eq!(output.status.success(), accepted, "{name}, {spec}: {stdout}");
            if !accepted {
                assert!(stdout.contains("match(**/fixes/**/spec.md)"), "{name}, {spec}: {stdout}");
            }
        }
    }
}

// Standalone schema files in the two YAML presentations the source-aware loader
// accepts beyond the conventional spelling: a block sequence at its parent
// key's column (indentless) and a plain scalar continued onto more-indented
// lines. Each presented spelling has a conventional twin with the same meaning,
// and every list and continuation is followed by a sibling (`next`) whose own
// constraint is asserted, so a loader that swallowed the sibling cannot pass.

/// Tagged envelope: an indentless property union, and a definition whose
/// constraint list and description continue onto later physical lines.
const TAGGED_PRESENTED: &str = "kind: schema\ntypes:\n  title: string(min(3);\n    max(8)) -> the display\n    title of the post\n  status:\n  - number\n  - code: number(min(1))\n  next: boolean\n";
const TAGGED_CONVENTIONAL: &str = "kind: schema\ntypes:\n  title: string(min(3); max(8)) -> the display title of the post\n  status:\n    - number\n    - code: number(min(1))\n  next: boolean\n";

/// Pure `$schema` envelope: an indentless root union of compact mapping arms,
/// the first carrying an indentless property union followed by a sibling.
const ROOT_UNION_PRESENTED: &str = "$schema:\n- title: string(required)\n  status:\n  - number\n  - boolean\n  next: boolean\n- count: number(required)\n";
const ROOT_UNION_CONVENTIONAL: &str = "$schema:\n  - title: string(required)\n    status:\n      - number\n      - boolean\n    next: boolean\n  - count: number(required)\n";

/// The expected outcome of validating one instance: `None` is valid, otherwise
/// the failing JSON pointer and a fragment of its reported message.
type Expected = Option<(&'static str, &'static str)>;

/// Writes `schema` as `schema.yaml`, then validates one document per instance
/// that references it by relative path, asserting the exit status, the
/// reported schema, and — for an invalid instance — the problem's path and
/// message, so success cannot stand in for a schema that was never applied.
fn assert_spelling_applied(schema: &str, instances: &[(&str, Expected)]) {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    write_file(tmp, "schema.yaml", schema);
    for (index, (instance, expected)) in instances.iter().enumerate() {
        let doc = write_file(
            tmp,
            &format!("doc-{index}.md"),
            &format!("---\n$schema: ./schema.yaml\n{instance}\n---\nBody\n"),
        );
        let output = process
            .command()
            .args(["schema", "validate", "--disable", "schema-triggers", "--format", "json"])
            .arg(&doc)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let report: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|error| panic!("{instance:?}: {error}: {stdout}"));
        assert_eq!(report["schema"], "./schema.yaml", "{instance:?}: {stdout}");
        match expected {
            None => {
                assert_eq!(output.status.code(), Some(0), "{instance:?}: {stdout}");
                assert_eq!(report["valid"], true, "{instance:?}: {stdout}");
            }
            Some((path, message)) => {
                assert_eq!(output.status.code(), Some(1), "{instance:?}: {stdout}");
                let problems = report["problems"].as_array().unwrap();
                assert_eq!(problems.len(), 1, "{instance:?}: {stdout}");
                assert_eq!(problems[0]["path"], *path, "{instance:?}: {stdout}");
                let reported = problems[0]["message"].as_str().unwrap();
                assert!(reported.contains(message), "{instance:?}: {stdout}");
            }
        }
    }
}

const TAGGED_INSTANCES: &[(&str, Expected)] = &[
    ("title: abcd\nstatus:\n  code: 2\nnext: true", None),
    ("title: ab", Some(("/title", "shorter than 3 characters"))),
    // `max(8)` sits on the continuation line.
    ("title: abcdefghij", Some(("/title", "longer than 8 characters"))),
    ("status: word", Some(("/status", "anyOf"))),
    ("status:\n  code: 0", Some(("/status/code", "less than the minimum of 1"))),
    ("next: nope", Some(("/next", "is not of type \"boolean\""))),
];

#[test]
fn schema_validate_applies_tagged_indentless_union_and_continued_scalar() {
    assert_spelling_applied(TAGGED_PRESENTED, TAGGED_INSTANCES);
}

#[test]
fn schema_validate_applies_tagged_presented_spelling_with_crlf() {
    assert_spelling_applied(&TAGGED_PRESENTED.replace('\n', "\r\n"), TAGGED_INSTANCES);
}

/// The folded description is the property's description, so the continued
/// text was read as part of the scalar rather than dropped or mis-keyed.
#[test]
fn schema_validate_reports_folded_description_of_continued_scalar() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    write_file(tmp, "schema.yaml", TAGGED_PRESENTED);
    let doc = write_file(
        tmp,
        "doc.md",
        "---\n$schema: ./schema.yaml\ntitle: ab\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate", "--disable", "schema-triggers", "--format", "json"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "\"description\":\"the display title of the post\"",
        ));
}

#[test]
fn schema_validate_applies_indentless_pure_root_union() {
    assert_spelling_applied(
        ROOT_UNION_PRESENTED,
        &[
            ("title: Hi\nstatus: true\nnext: false", None),
            ("count: 2", None),
            ("title: Hi\nstatus: word", Some(("/status", "anyOf"))),
            ("title: Hi\nnext: nope", Some(("/next", "is not of type \"boolean\""))),
            ("other: 1", Some(("", "\"title\" is a required property"))),
        ],
    );
}

#[test]
fn schema_validate_rejects_indentless_types_sequence() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    write_file(tmp, "schema.yaml", "kind: schema\ntypes:\n- title: string\n");
    let doc = write_file(
        tmp,
        "doc.md",
        "---\n$schema: ./schema.yaml\ntitle: Hello\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "validate", "--disable", "schema-triggers"])
        .arg(&doc)
        .assert()
        .code(2)
        .stdout(predicate::str::contains("has a schema definition error"))
        .stdout(predicate::str::contains("must be a mapping"));
}

/// Each presented spelling, loaded from a real file through the library's
/// normal reference resolution, compiles to the same JSON Schema value as its
/// conventional twin. Source spans differ by design; the value must not.
#[test]
fn schema_yaml_spellings_resolve_to_conventional_json_schema() {
    use darkmatter::markdown::schemas::resolve::resolve_schema_with_roots;

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let context = biscuit_file::FileResolutionContext::new(dir);
    let resolve = |name: &str, source: &str| {
        write_file(dir, name, source);
        let reference = serde_json::json!(format!("./{name}"));
        let resolved = resolve_schema_with_roots(&reference, dir, &[], &context)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(resolved.simplified.is_some(), "{name} must load as a SimplifiedSchema");
        resolved.json_schema
    };

    for (label, conventional, presented) in [
        ("tagged", TAGGED_CONVENTIONAL, TAGGED_PRESENTED.to_owned()),
        ("tagged-crlf", TAGGED_CONVENTIONAL, TAGGED_PRESENTED.replace('\n', "\r\n")),
        ("root-union", ROOT_UNION_CONVENTIONAL, ROOT_UNION_PRESENTED.to_owned()),
    ] {
        let expected = resolve(&format!("{label}.conventional.yaml"), conventional);
        let actual = resolve(&format!("{label}.presented.yaml"), &presented);
        assert_eq!(actual, expected, "{label}");
    }
}

// Reused named types through normal `md schema validate` resolution
// (`2026-10-13-named-type-presence-and-local-names`). `md schema validate`
// judges the authored document, so an absent `generated` property passes here
// even when it is also `required`; the runtime phase that enforces it is
// checked in `schema_validate_phase`.

/// A types-only schema file: `kind: schema` cannot also carry `$schema`.
/// `title_base` reaches `label` by a bare name, which resolves in this file.
const NAMED_TYPES: &str = "kind: schema\ntypes:\n  code: string(required; generated)\n  title: string(required)\n  label: string(min(3))\n  title_base: label\n";

/// The document's own `$schema` defines `title`, `code`, and `label` as
/// sibling entries and reuses each through a bare name, `@this`, and
/// `@./types.yaml`; `hop` and `heading` are multi-hop chains.
const NAMED_TYPE_SCHEMA: &str = "$schema:\n  title: string(required)\n  a: title\n  b: title@this\n  c: title@./types.yaml\n  code: string(required; generated)\n  ga: code\n  gb: code@this\n  gc: code@./types.yaml\n  label: string(min(3))\n  title_base: label\n  hop: title_base@./types.yaml\n  heading: title_base(max(5))\n";

/// Values that satisfy every required property and bound of
/// [`NAMED_TYPE_SCHEMA`], leaving every generated property absent.
const NAMED_TYPE_REQUIRED_VALUES: [&str; 4] = ["title=t", "a=t", "b=t", "c=t"];

/// Runs `md schema validate --format json` on `doc` with `extra` positional
/// inputs, returning the exit code and the parsed report.
fn validate_json(
    process: &CliProcessFixture,
    doc: &Path,
    extra: &[&str],
) -> (Option<i32>, serde_json::Value) {
    let output = process
        .command()
        .args([
            "schema",
            "validate",
            "--disable", "schema-triggers",
            "--format",
            "json",
        ])
        .arg(doc)
        .args(extra)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report =
        serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: stdout {stdout:?}"));
    (output.status.code(), report)
}

/// The `(path, message)` of every reported problem, sorted by path.
fn problems(report: &serde_json::Value) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = report["problems"]
        .as_array()
        .unwrap_or_else(|| panic!("report has no problems array: {report}"))
        .iter()
        .map(|problem| {
            let path = match problem["property"].as_str() {
                Some(property) => format!("/{property}"),
                None => problem["path"].as_str().unwrap().to_owned(),
            };
            (path, problem["message"].as_str().unwrap().to_owned())
        })
        .collect();
    found.sort();
    found
}

fn named_type_document(process: &CliProcessFixture, values: &str) -> PathBuf {
    let tmp = process.cwd();
    write_file(tmp, "types.yaml", NAMED_TYPES);
    write_file(
        tmp,
        "doc.md",
        &format!("---\n{NAMED_TYPE_SCHEMA}{values}---\nBody\n"),
    )
}

#[test]
fn schema_validate_inherited_required_is_reported_for_every_reference_spelling() {
    let process = CliProcessFixture::new();
    let doc = named_type_document(&process, "");

    let (code, report) = validate_json(&process, &doc, &[]);

    assert_eq!(code, Some(1), "{report}");
    let missing: Vec<String> = problems(&report)
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    // `ga`, `gb`, `gc`, and `code` inherit `generated` too, so authored
    // validation lets them stay absent.
    assert_eq!(missing, ["/a", "/b", "/c", "/title"], "{report}");
    for problem in report["problems"].as_array().unwrap() {
        assert_eq!(problem["kind"], "missing", "{report}");
    }
}

#[test]
fn schema_validate_supplied_required_and_absent_generated_pass_authored_validation() {
    let process = CliProcessFixture::new();
    let doc = named_type_document(&process, "");

    let (code, report) = validate_json(&process, &doc, &NAMED_TYPE_REQUIRED_VALUES);

    assert_eq!(code, Some(0), "{report}");
    assert_eq!(report["valid"], true, "{report}");
}

#[test]
fn schema_validate_rejects_a_wrong_typed_value_through_every_generated_spelling() {
    let process = CliProcessFixture::new();
    // A sequence, because a scalar would be coerced to the declared string.
    let doc = named_type_document(&process, "code: [1]\nga: [1]\ngb: [1]\ngc: [1]\n");

    let (code, report) = validate_json(&process, &doc, &NAMED_TYPE_REQUIRED_VALUES);

    assert_eq!(code, Some(1), "{report}");
    let expected: Vec<(String, String)> = ["/code", "/ga", "/gb", "/gc"]
        .into_iter()
        .map(|path| (path.to_owned(), "[1] is not of type \"string\"".to_owned()))
        .collect();
    assert_eq!(problems(&report), expected, "{report}");
}

#[test]
fn schema_validate_enforces_inherited_and_use_site_length_bounds_across_hops() {
    let process = CliProcessFixture::new();
    let doc = named_type_document(&process, "");

    let mut inputs = NAMED_TYPE_REQUIRED_VALUES.to_vec();
    // `title_base` and `hop` inherit `min(3)` from `label` (one hop locally,
    // two across files); `heading` adds `max(5)` on top of it.
    inputs.extend(["title_base=ab", "hop=ab", "heading=abcdef"]);
    let (code, report) = validate_json(&process, &doc, &inputs);

    assert_eq!(code, Some(1), "{report}");
    assert_eq!(
        problems(&report),
        [
            (
                "/heading".to_owned(),
                "\"abcdef\" is longer than 5 characters".to_owned()
            ),
            (
                "/hop".to_owned(),
                "\"ab\" is shorter than 3 characters".to_owned()
            ),
            (
                "/title_base".to_owned(),
                "\"ab\" is shorter than 3 characters".to_owned()
            ),
        ],
        "{report}"
    );
}

/// `tag` reuses `code(pattern(^a))` from `types.yaml` and adds `pattern(z$)`;
/// a value must satisfy both. `None` is valid, otherwise the pattern the
/// value violates.
fn assert_conjunctive_pattern(value: &str, violated: Option<&str>) {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    write_file(
        tmp,
        "types.yaml",
        "kind: schema\ntypes:\n  code: string(pattern(^a))\n",
    );
    let doc = write_file(
        tmp,
        "doc.md",
        "---\n$schema:\n  tag: code(pattern(z$))@./types.yaml\n---\nBody\n",
    );

    let (code, report) = validate_json(&process, &doc, &[&format!("tag={value}")]);

    match violated {
        None => assert_eq!(code, Some(0), "{value}: {report}"),
        Some(pattern) => {
            assert_eq!(code, Some(1), "{value}: {report}");
            assert_eq!(
                problems(&report),
                [(
                    "/tag".to_owned(),
                    format!("\"{value}\" does not match \"{pattern}\"")
                )],
                "{value}: {report}"
            );
        }
    }
}

macro_rules! conjunctive_pattern_rows {
    ($($name:ident: $value:literal => $violated:expr;)*) => {$(
        #[test]
        fn $name() {
            assert_conjunctive_pattern($value, $violated);
        }
    )*};
}

conjunctive_pattern_rows! {
    schema_validate_conjunctive_patterns_accept_a_value_matching_both: "az" => None;
    schema_validate_conjunctive_patterns_reject_a_value_missing_the_use_site_pattern: "ab" => Some("z$");
    schema_validate_conjunctive_patterns_reject_a_value_missing_the_inherited_pattern: "bz" => Some("^a");
}

#[test]
fn schema_validate_multi_hop_bound_contradiction_names_both_origins() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let types = write_file(
        tmp,
        "types.yaml",
        "kind: schema\ntypes:\n  label: string(min(10))\n  title_base: label\n",
    );
    let doc = write_file(
        tmp,
        "doc.md",
        "---\n$schema:\n  heading: title_base(max(5))@./types.yaml\nheading: abc\n---\nBody\n",
    );

    let (code, report) = validate_json(&process, &doc, &[]);

    assert_eq!(code, Some(2), "{report}");
    assert_eq!(report["error"], "schema", "{report}");
    let message = report["message"].as_str().unwrap();
    for fragment in [
        "heading: contradictory bounds on named type `title_base`".to_owned(),
        format!("`min(10)` in `{}`", resolved_display(&types)),
        "`max(5)` in the inline `$schema`".to_owned(),
    ] {
        assert!(
            message.contains(&fragment),
            "missing {fragment:?} in {message}"
        );
    }
}

#[test]
fn schema_validate_unknown_bare_name_in_an_inline_schema_is_a_schema_error() {
    let process = CliProcessFixture::new();
    let doc = write_file(
        process.cwd(),
        "doc.md",
        "---\n$schema:\n  heading: nosuchtype\nheading: abc\n---\nBody\n",
    );

    let (code, report) = validate_json(&process, &doc, &[]);

    assert_eq!(code, Some(2), "{report}");
    assert_eq!(report["error"], "schema", "{report}");
    assert_eq!(
        report["message"], "heading: unknown named type `nosuchtype` in the inline `$schema`",
        "{report}"
    );
}

#[test]
fn schema_validate_unknown_bare_name_in_a_referenced_schema_names_that_file() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let broken = write_file(
        tmp,
        "broken.yaml",
        "kind: schema\ntypes:\n  heading: nosuchtype\n",
    );
    let doc = write_file(tmp, "doc.md", "---\n$schema: ./broken.yaml\n---\nBody\n");

    let (code, report) = validate_json(&process, &doc, &[]);

    assert_eq!(code, Some(2), "{report}");
    let message = report["message"].as_str().unwrap();
    let fragment = format!(
        "heading: unknown named type `nosuchtype` in `{}`",
        resolved_display(&broken)
    );
    assert!(
        message.contains(&fragment),
        "missing {fragment:?} in {message}"
    );
}

/// The `guide` arm's `spec` reuses `doc: file(match('*.md'))` and adds
/// `match('guide*')`; the `other` arm needs `owner`, so a file the `guide` arm
/// rules out is judged by the `other` arm when `kind` is not set.
const GROUPED_UNION_SCHEMA: &str = "$schema:\n  - kind: 'literal(guide)'\n    spec: \"doc(required; eager; match('guide*'))@./types.yaml\"\n  - kind: 'literal(other)'\n    owner: 'string(required)'\n";

/// Real files for every row of the grouped-`match` table, plus the shared
/// `types.yaml`, the root-union document, and a single-schema document.
fn grouped_match_documents(process: &CliProcessFixture) -> (PathBuf, PathBuf) {
    let tmp = process.cwd();
    for name in ["guide-intro.md", "notes.md", "guide.txt"] {
        write_file(tmp, name, "# File\n");
    }
    write_file(
        tmp,
        "types.yaml",
        "kind: schema\ntypes:\n  doc: \"file(match('*.md'))\"\n",
    );
    let union = write_file(
        tmp,
        "union.md",
        &format!("---\n{GROUPED_UNION_SCHEMA}---\nBody\n"),
    );
    let single = write_file(
        tmp,
        "single.md",
        "---\n$schema:\n  spec: \"doc(required; eager; match('guide*'))@./types.yaml\"\n---\nBody\n",
    );
    (union, single)
}

/// With `kind: guide` settled, the arm accepts `spec` only for a path every
/// `match` group admits. `None` is accepted.
fn assert_guide_arm(spec: &str, rejected_as: Option<&str>) {
    let process = CliProcessFixture::new();
    let (union, _) = grouped_match_documents(&process);

    let (code, report) = validate_json(&process, &union, &["kind=guide", &format!("spec={spec}")]);

    match rejected_as {
        None => assert_eq!(code, Some(0), "{spec}: {report}"),
        Some(message) => {
            assert_eq!(code, Some(1), "{spec}: {report}");
            assert_eq!(
                problems(&report),
                [("/spec".to_owned(), message.to_owned())],
                "{spec}: {report}"
            );
            assert_eq!(report["problems"][0]["arm_index"], 0, "{spec}: {report}");
        }
    }
}

macro_rules! grouped_match_rows {
    ($($name:ident: $spec:literal => $rejected_as:expr;)*) => {$(
        #[test]
        fn $name() {
            assert_guide_arm($spec, $rejected_as);
        }
    )*};
}

grouped_match_rows! {
    schema_validate_grouped_match_arm_accepts_a_path_every_group_admits:
        "guide-intro.md" => None;
    schema_validate_grouped_match_arm_rejects_a_path_outside_the_use_site_group:
        "notes.md" => Some("`notes.md` is outside this schema arm's `match(*.md)` and `match(guide*)`");
    schema_validate_grouped_match_arm_rejects_a_path_outside_the_inherited_group:
        "guide.txt" => Some("`guide.txt` is outside this schema arm's `match(*.md)` and `match(guide*)`");
}

/// Without `kind`, a path one group rejects rules the `guide` arm out, so the
/// `other` arm judges the document and reports its own missing `owner`.
#[test]
fn schema_validate_grouped_match_rules_out_the_arm_for_an_undecided_document() {
    let process = CliProcessFixture::new();
    let (union, _) = grouped_match_documents(&process);

    let (code, report) = validate_json(&process, &union, &["spec=guide.txt"]);

    assert_eq!(code, Some(1), "{report}");
    assert_eq!(
        problems(&report),
        [(
            "/owner".to_owned(),
            "\"owner\" is a required property".to_owned()
        )],
        "{report}"
    );
    assert_eq!(report["problems"][0]["arm_index"], 1, "{report}");
}

/// In a plain schema `match` stays passive: an existing file neither group
/// admits is still a valid value.
#[test]
fn schema_validate_grouped_match_is_passive_in_a_single_schema() {
    let process = CliProcessFixture::new();
    let (_, single) = grouped_match_documents(&process);

    let (code, report) = validate_json(&process, &single, &["spec=guide.txt"]);

    assert_eq!(code, Some(0), "{report}");
    assert_eq!(report["valid"], true, "{report}");
}

/// Claudine's complete schema bundle.
const CLAUDINE_BUNDLE: [(&str, &str); 5] = [
    ("action.yaml", include_str!("../../../../claudine/schemas/action.yaml")),
    ("claudine.yaml", include_str!("../../../../claudine/schemas/claudine.yaml")),
    ("inline-compose.yaml", include_str!("../../../../claudine/schemas/inline-compose.yaml")),
    ("lifecycle.yaml", include_str!("../../../../claudine/schemas/lifecycle.yaml")),
    ("linking.yaml", include_str!("../../../../claudine/schemas/linking.yaml")),
];

/// Makes the fixture `cwd` a repository whose `schemas/` root holds Claudine's
/// bundle, and writes `doc.md` there holding `values` with no `$schema`, so
/// the `claudine.yaml` trigger (whose payload reuses `lifecycle.yaml` and
/// `action.yaml` types through relative imports) is the schema that judges it.
fn shipped_claudine_document(process: &CliProcessFixture, values: &str) -> PathBuf {
    assert!(process.initialize_repository(), "git init");
    let schemas = process.cwd().join("schemas");
    std::fs::create_dir_all(&schemas).unwrap();
    for (name, text) in CLAUDINE_BUNDLE {
        write_file(&schemas, name, text);
    }
    write_file(process.cwd(), "doc.md", &format!("---\n{values}---\nBody\n"))
}

/// Like [`validate_json`], with trigger schemas applied.
fn validate_triggered_json(process: &CliProcessFixture, doc: &Path) -> (Option<i32>, serde_json::Value) {
    let output = process
        .command()
        .args(["schema", "validate", "--format", "json"])
        .arg(doc)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report =
        serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: stdout {stdout:?}"));
    (output.status.code(), report)
}

#[test]
fn schema_validate_shipped_claudine_schema_accepts_reused_lifecycle_and_loop_types() {
    let process = CliProcessFixture::new();
    let doc = shipped_claudine_document(
        &process,
        "initialize:\n  - say: hello\n  - when: 'true'\n    then:\n      notify: hi\nloop:\n  max: 3\n  action:\n    op: increment\n    prop: n\n  gate:\n    - stop\n",
    );

    let (code, report) = validate_triggered_json(&process, &doc);

    assert_eq!(code, Some(0), "{report}");
    assert_eq!(report["valid"], true, "{report}");
}

/// Every arm of an `initialize` stack item is a closed object or a bare verb,
/// so an item naming an unknown verb matches no arm of the item union.
#[test]
fn schema_validate_shipped_claudine_schema_rejects_an_unknown_lifecycle_verb() {
    let process = CliProcessFixture::new();
    let doc = shipped_claudine_document(&process, "initialize:\n  - bogus: 1\n");

    let (code, report) = validate_triggered_json(&process, &doc);

    assert_eq!(code, Some(1), "{report}");
    let found = problems(&report);
    assert_eq!(found.len(), 1, "{report}");
    assert_eq!(found[0].0, "/initialize/0", "{report}");
    assert!(found[0].1.contains("anyOf"), "{report}");
}

#[test]
fn schema_validate_shipped_claudine_schema_rejects_values_inside_a_reused_loop_event() {
    let process = CliProcessFixture::new();
    let doc = shipped_claudine_document(
        &process,
        "loop:\n  max: 0\n  action:\n    op: explode\n    prop: n\n",
    );

    let (code, report) = validate_triggered_json(&process, &doc);

    assert_eq!(code, Some(1), "{report}");
    let found = problems(&report);
    let paths: Vec<&str> = found.iter().map(|(path, _)| path.as_str()).collect();
    assert_eq!(paths, ["/loop/action/op", "/loop/max"], "{report}");
    assert!(found[0].1.contains("\"explode\" is not one of"), "{report}");
    // `max` is a union (a count, or a `{{ cap }}` template resolved before the
    // loop starts), so a rejected count is reported against the union, and the
    // problem's description names both arms.
    assert_eq!(found[1].1, "0 is not valid under any of the schemas listed in the 'anyOf' keyword", "{report}");
    assert!(report.to_string().contains("the most iterations to run | a cap such as {{ cap }}"), "{report}");
}

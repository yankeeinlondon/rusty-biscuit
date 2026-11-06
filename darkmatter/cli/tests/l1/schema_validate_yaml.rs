//! `md schema validate` over whole YAML documents (`.yaml`, `.yml`).
//!
//! Wiring only: the loader's document shapes and the selection matrix are
//! judged in-process by the library suites (`yaml_document_loading.rs`,
//! `yaml_document_selection.rs`, `yaml_kind_registry.rs`). Each test here
//! proves one route or exit status reaches the rendered output, with the
//! location of the problem where there is one.

use crate::common;

use common::CliProcessFixture;
use predicates::prelude::*;
use std::path::PathBuf;

/// A user `yaml-schema` claiming `roster`; its document is [`ROSTER_DOCUMENT`].
const ROSTER_DEFINITION: &str = "\
kind: yaml-schema
$schema:
    kind: literal(roster; required)
    roster_version: literal(1; required)
    platforms: string[](required)
";

const ROSTER_DOCUMENT: &str = "kind: roster\nroster_version: 1\nplatforms: [slack]\n";

fn write(process: &CliProcessFixture, name: &str, content: &str) -> PathBuf {
    let path = process.cwd().join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, content).unwrap();
    path
}

fn stdout_of(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The one file record `md schema validate --output json` printed.
fn json_record(output: &std::process::Output) -> serde_json::Value {
    common::schema_validate_record(&output.stdout)
}

// ── Kind route ─────────────────────────────────────────────────────────────

#[test]
fn a_claimed_shipped_kind_validates_against_its_definition() {
    let process = CliProcessFixture::new();
    let doc = write(&process, "types.yaml", "kind: schema\ntypes:\n    title: string\n");

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::contains("has a schema definition and is valid based on its definition"));
}

#[test]
fn a_kind_definition_failure_exits_1_located_on_the_value() {
    let process = CliProcessFixture::new();
    let doc = write(&process, "types.yaml", "kind: schema\ntypes: 42\n");

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("failed schema validation"))
        .stdout(predicate::str::contains("line 2, column 8"));
}

#[test]
fn a_kind_semantic_check_failure_exits_1_as_a_kind_check_record() {
    let process = CliProcessFixture::new();
    let doc = write(
        &process,
        "types.yaml",
        "kind: schema\ncategories: [text]\ntypes:\n    title: string(category(media))\n",
    );

    let output = process.command().args(["schema", "validate", "--output", "json"]).arg(&doc).output().unwrap();
    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
    let record = json_record(&output);
    assert_eq!(record["valid"], false);
    let problems = record["problems"].as_array().unwrap();
    assert_eq!(problems.len(), 1, "{record}");
    assert_eq!(problems[0]["kind"], "kind_check");
    assert_eq!(problems[0]["path"], "/types/title");
    assert_eq!((problems[0]["line"].as_u64(), problems[0]["column"].as_u64()), (Some(4), Some(12)));
}

/// A checkout copy of the shipped definition `shipped` at its
/// repository-relative path `relative`, with its root closure removed, is
/// the selected definition: `document` passes it (exit `0`), and so does
/// `document` with an added `note`, which only the compiled closed
/// definition would refuse.
fn assert_an_opened_checkout_copy_admits_an_added_property(relative: &str, shipped: &str, document: &str) {
    let process = CliProcessFixture::new();
    if !process.initialize_repository() {
        eprintln!("skipping: git is unavailable");
        return;
    }
    let opened: String =
        shipped.lines().filter(|line| line.trim() != "$constraints: { closed: true }").map(|line| format!("{line}\n")).collect();
    assert_ne!(opened, shipped, "the definition had a root closure to remove");
    let copy = write(&process, relative, &opened);
    let roots = copy.parent().unwrap().to_path_buf();
    for (name, text) in [("data/control.yaml", document.to_string()), ("data/added.yaml", format!("{document}note: hi\n"))] {
        let doc = write(&process, name, &text);
        let output = process
            .command_builder()
            .application_input("SCHEMAS_DIR", &roots)
            .build()
            .args(["schema", "validate"])
            .arg(&doc)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0), "{name}: {}", stdout_of(&output));
    }
}

#[test]
fn an_opened_checkout_schema_definition_admits_an_added_property() {
    assert_an_opened_checkout_copy_admits_an_added_property(
        "simplified-schema/schemas/schema.yaml",
        include_str!("../../../../simplified-schema/schemas/schema.yaml"),
        "kind: schema\n$schema:\n    title: string\n",
    );
}

#[test]
fn an_opened_checkout_yaml_schema_definition_admits_an_added_property() {
    assert_an_opened_checkout_copy_admits_an_added_property(
        "simplified-schema/schemas/yaml-schema.yaml",
        include_str!("../../../../simplified-schema/schemas/yaml-schema.yaml"),
        "kind: yaml-schema\n$schema:\n    kind: literal(inventory; required)\n",
    );
}

#[test]
fn an_opened_checkout_trigger_definition_admits_an_added_property() {
    assert_an_opened_checkout_copy_admits_an_added_property(
        "darkmatter/schemas/schema-trigger.yaml",
        include_str!("../../../schemas/schema-trigger.yaml"),
        "kind: schema-trigger\nmatch:\n    all:\n        - $path: \"docs/**\"\n        - title: string(required)\n",
    );
}

#[test]
fn a_repository_kind_claim_selects_its_definition_and_a_duplicate_claim_exits_1() {
    let process = CliProcessFixture::new();
    if !process.initialize_repository() {
        eprintln!("skipping: git is unavailable");
        return;
    }
    let first = write(&process, "schemas/roster.yaml", ROSTER_DEFINITION);
    let doc = write(&process, "data/platforms.yaml", ROSTER_DOCUMENT);

    // Control: one claim, the document passes it, and a roster break fails it.
    process.command().args(["schema", "validate"]).arg(&doc).assert().success();
    let broken = write(&process, "data/broken.yaml", "kind: roster\nroster_version: 2\nplatforms: [slack]\n");
    process
        .command()
        .args(["schema", "validate"])
        .arg(&broken)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("line 2, column 17"));

    // A second definition claiming the same kind: neither applies.
    let second = write(&process, "schemas/roster-copy.yaml", ROSTER_DEFINITION);
    let output = process.command().args(["schema", "validate", "--output", "json"]).arg(&doc).output().unwrap();
    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
    let record = json_record(&output);
    assert_eq!(record["error"], "schema");
    let message = record["message"].as_str().unwrap();
    for file in [&first, &second] {
        let name = file.file_name().unwrap().to_string_lossy();
        assert!(message.contains(name.as_ref()), "{message}");
    }
}

#[test]
fn a_kind_reserved_by_a_failed_definition_exits_1_naming_the_definition() {
    let process = CliProcessFixture::new();
    if !process.initialize_repository() {
        eprintln!("skipping: git is unavailable");
        return;
    }
    let definition = ROSTER_DEFINITION.replace("platforms: string[](required)", "platforms: platform@./absent.yaml");
    write(&process, "schemas/roster.yaml", &definition);
    let doc = write(&process, "data/platforms.yaml", ROSTER_DOCUMENT);

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("has a schema definition error"))
        .stdout(predicate::str::contains("roster.yaml"));
}

// ── Root `$schema` route and no schema ─────────────────────────────────────

#[test]
fn a_root_schema_validates_softly_and_a_failure_exits_1_located_on_the_value() {
    let process = CliProcessFixture::new();
    let schema = "$schema:\n    port: number(integer)\n";
    let passing = write(&process, "service.yaml", &format!("{schema}port: \"8080\"\n"));
    let failing = write(&process, "broken.yml", &format!("{schema}port: \"2.5\"\n"));

    process.command().args(["schema", "validate"]).arg(&passing).assert().success();
    // Validation is soft and passive: the file is not rewritten.
    assert_eq!(std::fs::read_to_string(&passing).unwrap(), format!("{schema}port: \"8080\"\n"));
    process
        .command()
        .args(["schema", "validate"])
        .arg(&failing)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("port"))
        .stdout(predicate::str::contains("line 3, column 7"));
}

/// A document holding only its root `$schema` is judged as data against
/// that schema, so a required property it lacks fails it.
/// A field named like schema metadata is data of the selected contract:
/// an optional `string` description accepts a value and null alike.
#[test]
fn a_metadata_named_field_is_data_of_the_root_schema() {
    let process = CliProcessFixture::new();
    for (name, value) in [("control.yaml", "ops"), ("null.yaml", "null")] {
        let doc = write(&process, name, &format!("$schema:\n    description: string\ndescription: {value}\n"));
        process.command().args(["schema", "validate"]).arg(&doc).assert().success();
    }
}

#[test]
fn a_root_schema_without_data_fails_its_required_property() {
    let process = CliProcessFixture::new();
    let doc = write(&process, "team.yaml", "$schema:\n    owner: string(required)\n");

    let output = process.command().args(["schema", "validate", "--output", "json"]).arg(&doc).output().unwrap();
    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
    let record = json_record(&output);
    let problems = record["problems"].as_array().unwrap();
    assert_eq!(problems.len(), 1, "{record}");
    assert!(problems[0]["message"].as_str().is_some_and(|message| message.contains("owner")), "{record}");
}

#[test]
fn a_document_without_kind_or_schema_is_valid_and_its_json_record_says_no_schema() {
    let process = CliProcessFixture::new();
    let doc = write(&process, "plain.yaml", "name: x\n");

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::contains("doesn't have a schema definition so is valid by default."));

    let output = process.command().args(["schema", "validate", "--output", "json"]).arg(&doc).output().unwrap();
    assert!(output.status.success());
    let record = json_record(&output);
    assert_eq!(record["valid"], true);
    assert_eq!(record["schema"], serde_json::Value::Null);
    assert_eq!(record["problems"], serde_json::json!([]));
    assert!(record.get("error").is_none(), "{record}");
}

/// A null `kind` claims nothing, in a schema root as anywhere: with no
/// `$schema` the file is not validated, and with an unresolvable one that
/// reference is its one error.
#[test]
fn a_null_kind_in_a_schema_root_claims_nothing() {
    let process = CliProcessFixture::new();
    if !process.initialize_repository() {
        eprintln!("skipping: git is unavailable");
        return;
    }
    let doc = write(&process, "schemas/data.yaml", "kind:\nowner: ops\n");
    let output = process.command().args(["schema", "validate", "--output", "json"]).arg(&doc).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
    let record = json_record(&output);
    assert_eq!(record["schema"], serde_json::Value::Null, "{record}");
    assert_eq!(record["problems"], serde_json::json!([]), "{record}");

    write(&process, "schemas/data.yaml", "kind:\n$schema: ./missing.yaml\nowner: ops\n");
    let output = process.command().args(["schema", "validate", "--output", "json"]).arg(&doc).output().unwrap();
    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
    let record = json_record(&output);
    assert_eq!(record["error"], "schema", "{record}");
    let message = record["message"].as_str().unwrap();
    assert!(message.contains("missing.yaml") && !message.contains("kind"), "{message}");
}

#[test]
fn an_unresolvable_root_schema_reference_exits_1() {
    let process = CliProcessFixture::new();
    let doc = write(&process, "service.yaml", "$schema: ./missing.yaml\nname: x\n");

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("has a schema definition error"))
        .stdout(predicate::str::contains("missing.yaml"));
}

/// Outside a repository a bare root `$schema` is not looked up in
/// `SCHEMAS_DIR`, for a YAML document exactly as for Markdown frontmatter;
/// the schema's path still names it.
#[test]
fn outside_a_repository_a_bare_root_schema_resolves_as_frontmatter_does() {
    let process = CliProcessFixture::new();
    let defs = process.workspace_path().join("dm-defs");
    std::fs::create_dir_all(&defs).unwrap();
    let contract = defs.join("contract.yaml");
    std::fs::write(&contract, "kind: schema\n$schema:\n    owner: string(required)\n").unwrap();
    let yaml = write(&process, "service.yaml", "$schema: contract.yaml\nowner: ops\n");
    let markdown = write(&process, "service.md", "---\n$schema: contract.yaml\nowner: ops\n---\n");
    let explicit = write(&process, "explicit.yaml", &format!("$schema: {}\nowner: ops\n", contract.display()));
    let exit_code = |document: &PathBuf| {
        let output = process
            .command_builder()
            .application_input("SCHEMAS_DIR", &defs)
            .build()
            .args(["schema", "validate"])
            .arg(document)
            .output()
            .unwrap();
        (output.status.code(), stdout_of(&output))
    };

    let (markdown_code, markdown_stdout) = exit_code(&markdown);
    assert_eq!(markdown_code, Some(1), "{markdown_stdout}");
    let (yaml_code, yaml_stdout) = exit_code(&yaml);
    assert_eq!(yaml_code, markdown_code, "{yaml_stdout}");
    let (explicit_code, explicit_stdout) = exit_code(&explicit);
    assert_eq!(explicit_code, Some(0), "{explicit_stdout}");
}

/// A referenced schema that is not valid YAML is the schema's failure, not
/// the document's, though both are YAML load errors.
#[test]
fn a_root_schema_naming_malformed_yaml_is_a_schema_definition_error() {
    let process = CliProcessFixture::new();
    write(&process, "contract.yaml", "a: [1\n");
    let doc = write(&process, "service.yaml", "$schema: ./contract.yaml\nname: x\n");

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("has a schema definition error"));
}

// ── Loading ────────────────────────────────────────────────────────────────

#[test]
fn malformed_yaml_exits_1_located_and_reported_as_yaml_parse() {
    let process = CliProcessFixture::new();
    let doc = write(&process, "broken.yaml", "a: [1\n");

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("is not a valid YAML document"))
        .stdout(predicate::str::contains("(at line 2, column 1)"));

    let output = process.command().args(["schema", "validate", "--output", "json"]).arg(&doc).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let record = json_record(&output);
    assert_eq!(record["error"], "yaml_parse");
    assert_eq!((record["line"].as_u64(), record["column"].as_u64()), (Some(2), Some(1)));
}

#[test]
fn leading_and_trailing_document_markers_are_yaml_not_frontmatter() {
    let process = CliProcessFixture::new();
    let doc = write(&process, "marked.yaml", "---\n$schema:\n    name: string(required)\nother: 1\n...\n");

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("name"));
}

// ── Routing by the resolved path ───────────────────────────────────────────

/// A Markdown reading of these files would find no frontmatter and pass;
/// only the YAML route fails them.
#[test]
fn upper_and_mixed_case_extensions_route_as_yaml() {
    let process = CliProcessFixture::new();
    let content = "$schema:\n    name: string(required)\nother: 1\n";
    for name in ["upper.YAML", "mixed.Yml"] {
        let doc = write(&process, name, content);
        process.command().args(["schema", "validate"]).arg(&doc).assert().code(1);
    }
}

#[test]
fn a_magic_reference_resolving_to_a_yaml_file_routes_as_yaml() {
    let process = CliProcessFixture::new();
    write(&process, "service.yaml", "$schema:\n    name: string(required)\nother: 1\n");

    process
        .command()
        .args(["schema", "validate", "@service.yaml"])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("@service.yaml"))
        .stdout(predicate::str::contains("name"));
}

// ── Preflight usage errors ─────────────────────────────────────────────────

#[test]
fn an_assignment_with_a_yaml_input_exits_2_before_any_file_is_validated() {
    let process = CliProcessFixture::new();
    let markdown = "---\ntitle: before\n---\nBody\n";
    let earlier = write(&process, "post.md", markdown);
    let yaml = write(&process, "plain.yaml", "name: x\n");

    process
        .command()
        .args(["schema", "validate"])
        .arg(&earlier)
        .arg("title=after")
        .arg(&yaml)
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("assignments"));
    assert_eq!(std::fs::read_to_string(&earlier).unwrap(), markdown);
}

#[test]
fn a_custom_baseline_with_a_yaml_input_exits_2() {
    let process = CliProcessFixture::new();
    let baseline = write(&process, "baseline.yaml", "$schema:\n    title: string\n");
    let doc = write(&process, "post.md", "---\ntitle: hi\n---\nBody\n");
    let yaml = write(&process, "plain.yaml", "name: x\n");

    process
        .command()
        .args(["schema", "validate", "--schema"])
        .arg(&baseline)
        .arg(&doc)
        .arg(&yaml)
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("--schema"));
}

// ── Baseline independence ──────────────────────────────────────────────────

#[test]
fn a_yaml_only_batch_never_loads_the_baseline() {
    let process = CliProcessFixture::new();
    let broken = write(&process, "broken-baseline.yaml", "not: a-schema\n");
    let doc = write(&process, "plain.yaml", "name: x\n");

    process
        .command_builder()
        .application_input("BASELINE_SCHEMA", &broken)
        .build()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .success();

    // A Markdown input in the batch loads it, and fails on it.
    let markdown = write(&process, "post.md", "---\ntitle: hi\n---\nBody\n");
    process
        .command_builder()
        .application_input("BASELINE_SCHEMA", &broken)
        .build()
        .args(["schema", "validate"])
        .arg(&doc)
        .arg(&markdown)
        .assert()
        .code(1);
}

#[test]
fn disabling_the_baseline_is_accepted_with_a_yaml_input() {
    let process = CliProcessFixture::new();
    let doc = write(&process, "types.yaml", "kind: schema\ntypes:\n    title: string\n");

    process
        .command()
        .args(["schema", "validate", "--disable", "baseline-schema"])
        .arg(&doc)
        .assert()
        .success();
}

// ── Batch exit precedence ──────────────────────────────────────────────────

#[test]
fn a_mixed_batch_exits_1_and_keeps_each_failure_kind_in_its_record() {
    let process = CliProcessFixture::new();
    let invalid = write(&process, "invalid.yaml", "$schema:\n    name: string(required)\nother: 1\n");
    let bad_reference = write(&process, "reference.yaml", "$schema: ./missing.yaml\n");
    let malformed = write(&process, "malformed.yaml", "a: [1\n");
    let markdown = write(&process, "post.md", "---\n$schema:\n  title: 'string(required)'\n---\nBody\n");

    let run = |files: &[&PathBuf]| {
        let mut command = process.command();
        command.args(["schema", "validate", "--disable", "baseline-schema"]);
        for file in files {
            command.arg(file);
        }
        command.output().unwrap().status.code()
    };
    assert_eq!(run(&[&invalid, &markdown]), Some(1));
    assert_eq!(run(&[&invalid, &bad_reference, &markdown]), Some(1));
    assert_eq!(run(&[&invalid, &bad_reference, &malformed, &markdown]), Some(1));

    let output = process
        .command()
        .args(["schema", "validate", "--disable", "baseline-schema", "--output", "json"])
        .args([&invalid, &bad_reference, &malformed, &markdown])
        .output()
        .unwrap();
    let errors: Vec<Option<String>> = common::schema_validate_records(&output.stdout)
        .iter()
        .map(|record| record["error"].as_str().map(str::to_string))
        .collect();
    assert_eq!(errors, [None, Some("schema".to_string()), Some("yaml_parse".to_string()), None]);
}

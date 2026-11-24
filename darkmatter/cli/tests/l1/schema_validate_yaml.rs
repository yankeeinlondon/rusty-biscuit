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

// ── Namespace files: checked as a host's registration prepares them ────────

/// Claudine's published `loop` namespace, unedited.
const PUBLISHED_NAMESPACE: &str = include_str!("../../../../claudine/registration/loop.yaml");

#[test]
fn control_an_unedited_published_namespace_validates_as_itself() {
    let process = CliProcessFixture::new();
    let doc = write(&process, "loop.yaml", PUBLISHED_NAMESPACE);

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::contains("is valid based on its definition built-in simplified-schema/schemas/namespace-schema.yaml"));
}

#[test]
fn a_namespace_with_a_null_description_or_an_undeclared_type_exits_1_at_the_field() {
    for (text, path, line, needle) in [
        ("kind: namespace-schema\nnamespace: thing\ndescription:\n$schema: {}\n", "/description", 3, "must not be null"),
        ("kind: namespace-schema\nnamespace: thing\n$schema:\n    item: Missing\n", "/$schema/item", 4, "unknown named type `Missing`"),
    ] {
        let process = CliProcessFixture::new();
        let doc = write(&process, "thing.yaml", text);
        let output = process.command().args(["schema", "validate", "--output", "json"]).arg(&doc).output().unwrap();
        assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
        let record = json_record(&output);
        let problems = record["problems"].as_array().unwrap();
        assert_eq!(problems.len(), 1, "{record}");
        assert_eq!(problems[0]["kind"], "kind_check", "{record}");
        assert_eq!(problems[0]["path"], path, "{record}");
        assert_eq!(problems[0]["line"].as_u64(), Some(line), "{record}");
        assert!(problems[0]["message"].as_str().unwrap().contains(needle), "{record}");
    }
}

/// A declaration that fails inside a union arm is reported at that arm,
/// not at the carrier or a valid sibling: an unresolved type in a
/// namespace's second property arm (kind check), and a constraint typo in a
/// trigger payload's second root-union arm (the definition's parse), at the
/// parser's rejected token.
#[test]
fn a_failure_inside_a_union_arm_exits_1_at_that_arm() {
    for (name, text, path, needle) in [
        (
            "thing.yaml",
            "kind: namespace-schema\nnamespace: thing\n$schema:\n    count:\n        - number\n        - Missing\n    label: string\n",
            "/$schema/count/1",
            "Missing\n",
        ),
        (
            "gadget.yaml",
            "kind: schema-trigger\nmatch:\n    gadget: string(required)\n$schema:\n    - gadget: string(required)\n    - gadget: string(requird)\n",
            "/$schema/1/gadget",
            "requird)",
        ),
    ] {
        let process = CliProcessFixture::new();
        let doc = write(&process, name, text);
        let output = process.command().args(["schema", "validate", "--output", "json"]).arg(&doc).output().unwrap();
        assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
        let record = json_record(&output);
        let problems = record["problems"].as_array().unwrap();
        assert_eq!(problems.len(), 1, "{record}");
        assert_eq!(problems[0]["path"], path, "{record}");
        let offset = text.find(needle).unwrap();
        let line = text[..offset].matches('\n').count() + 1;
        let column = offset - text[..offset].rfind('\n').map_or(0, |at| at + 1) + 1;
        assert_eq!(
            (problems[0]["line"].as_u64(), problems[0]["column"].as_u64()),
            (Some(line as u64), Some(column as u64)),
            "{record}"
        );
    }
}

#[test]
fn a_document_naming_a_namespace_as_its_schema_is_still_refused() {
    let process = CliProcessFixture::new();
    write(&process, "loop.yaml", PUBLISHED_NAMESPACE);
    let doc = write(&process, "service.yaml", "$schema: ./loop.yaml\ncount: 1\n");

    process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("has a schema definition error"))
        .stdout(predicate::str::contains("namespace-schema"));
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
        "simplified-schema/schemas/schema-trigger.yaml",
        include_str!("../../../../simplified-schema/schemas/schema-trigger.yaml"),
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

/// R4-F2.7: the load error's column counts characters, so `é` and `🧪`
/// before the problem each move it by one, not by their UTF-8 width.
#[test]
fn malformed_yaml_column_counts_characters() {
    let process = CliProcessFixture::new();
    let doc = write(&process, "broken.yaml", "ok: 1\ncafé🧪: x: y\n");

    let output = process.command().args(["schema", "validate", "--output", "json"]).arg(&doc).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let record = json_record(&output);
    assert_eq!(record["error"], "yaml_parse", "{record:#}");
    assert_eq!((record["line"].as_u64(), record["column"].as_u64()), (Some(2), Some(9)), "{record:#}");
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

// ── Callable catalogs ──────────────────────────────────────────────────────
//
// A `kind: expression-schema` or `kind: mutation-schema` file is judged by
// its kind contract and then the catalog reader, never resolved as a schema
// file. Every edited control below starts from the shared host catalog
// fixture with one authored change, and carries a slot and a shell sentinel
// in a category description: a run that resolved or composed the catalog
// would fail on the missing `sentinel.yaml` or create `created.txt`.

/// The shared valid host catalog (also DMLS's control).
const HOST_CATALOG: &str = include_str!("../../../lib/tests/fixtures/host_functions/project.yaml");
const FUNCTIONS_CATALOG: &str = include_str!("../../../schemas/partials/functions.yaml");
const MUTATIONS_CATALOG: &str = include_str!("../../../schemas/partials/mutations.yaml");
const FUNCTIONS_PATH: &str = "darkmatter/schemas/partials/functions.yaml";
const MUTATIONS_PATH: &str = "darkmatter/schemas/partials/mutations.yaml";
const CATALOG_CODE: &str = "dm.schema.callable_catalog";
/// The fixture's one category line, which the sentinel edit extends.
const CATEGORY_LINE: &str = "    - Project -> Ask about the files and names of the test host's project.";
/// A slot naming a file that does not exist and a shell step that creates
/// one, behind multi-byte text that makes every later byte offset differ
/// from its character offset.
const SENTINEL_TAIL: &str = " Ünïcödé 🧪🧪🧪🧪🧪🧪🧪🧪🧪🧪 {{ load_data(\"./sentinel.yaml\") }} $(touch created.txt)";

/// The host catalog with the sentinel tail on its category line, then
/// `edit` applied (`from` replaced by `to`, which must occur).
fn sentinel_catalog(from: &str, to: &str) -> String {
    assert!(HOST_CATALOG.contains(CATEGORY_LINE), "the fixture's category line");
    let text = HOST_CATALOG.replacen(CATEGORY_LINE, &format!("{CATEGORY_LINE}{SENTINEL_TAIL}"), 1);
    if from.is_empty() {
        return text;
    }
    assert!(text.contains(from), "the edit's anchor `{from}`");
    text.replacen(from, to, 1)
}

/// Neither sentinel fired: nothing was created, and the slot's file was
/// never needed (it does not exist).
fn assert_sentinels_untouched(process: &CliProcessFixture) {
    assert!(!process.cwd().join("created.txt").exists(), "the shell sentinel ran");
    assert!(!process.cwd().join("sentinel.yaml").exists(), "the slot sentinel was created");
}

/// Runs `md schema validate <args> --output json`, checking the exit status
/// and that the report satisfies the command's output contract.
fn catalog_json(process: &CliProcessFixture, args: &[&str], status: i32) -> serde_json::Value {
    let output = process.command().args(["schema", "validate", "--output", "json"]).args(args).output().unwrap();
    let stdout = stdout_of(&output);
    assert_eq!(output.status.code(), Some(status), "{stdout}{}", String::from_utf8_lossy(&output.stderr));
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"));
    let contract = simplified_schema::SimplifiedSchema::try_from(include_str!("../../schemas/output/schema-validate.yaml"))
        .expect("the output contract parses");
    if let Err(failure) = contract.validate_strict(&report) {
        panic!("{failure:?}\n{report:#}");
    }
    report
}

/// Runs `md schema validate <args> --output plain`, checking the exit status.
fn catalog_plain(process: &CliProcessFixture, args: &[&str], status: i32) -> String {
    let output = process.command().args(["schema", "validate", "--output", "plain"]).args(args).output().unwrap();
    let stdout = stdout_of(&output);
    assert_eq!(output.status.code(), Some(status), "{stdout}{}", String::from_utf8_lossy(&output.stderr));
    stdout
}

/// The 1-based line of byte `offset` in `text`.
fn line_of(text: &str, offset: usize) -> u64 {
    text[..offset].matches('\n').count() as u64 + 1
}

/// The byte span `{start, end}` of the first `needle` in `text`.
fn span_of(text: &str, needle: &str) -> serde_json::Value {
    let start = text.find(needle).unwrap_or_else(|| panic!("`{needle}` in the text"));
    serde_json::json!({ "start": start, "end": start + needle.len() })
}

#[test]
fn catalog_shipped_functions_and_mutations_validate_from_the_checkout_root() {
    let process = CliProcessFixture::new();
    if !process.initialize_repository() {
        eprintln!("skipping: git is unavailable");
        return;
    }
    write(&process, FUNCTIONS_PATH, FUNCTIONS_CATALOG);
    write(&process, MUTATIONS_PATH, MUTATIONS_CATALOG);

    let report = catalog_json(&process, &[FUNCTIONS_PATH, MUTATIONS_PATH], 0);
    let records = report["files"].as_array().expect("files");
    assert_eq!(records.len(), 2, "{report:#}");
    for record in records {
        assert_eq!(record["valid"], true, "{report:#}");
        assert!(record.get("error").is_none(), "{report:#}");
    }
}

#[test]
fn catalog_the_shared_host_fixture_is_valid_and_its_sentinels_stay_passive() {
    let process = CliProcessFixture::new();
    write(&process, "project.yaml", HOST_CATALOG);
    write(&process, "sentinel-project.yaml", &sentinel_catalog("", ""));

    for file in ["project.yaml", "sentinel-project.yaml"] {
        let stdout = catalog_plain(&process, &[file], 0);
        assert!(stdout.contains("is valid based on its definition built-in simplified-schema/schemas/expression-schema.yaml"), "{stdout}");
        let record = &catalog_json(&process, &[file], 0)["files"][0];
        assert_eq!(record["valid"], true, "{record:#}");
    }
    assert_sentinels_untouched(&process);
}

/// `Is_Project_File` folds onto the earlier `is_project_file`: the failure
/// is at the later key and the related location at the first, both in this
/// file. The multi-byte sentinel text above them proves the lines come
/// from byte offsets.
#[test]
fn catalog_a_folded_name_collision_exits_1_at_the_key_with_the_earlier_declaration_related() {
    let process = CliProcessFixture::new();
    let text = sentinel_catalog("    project_label:", "    Is_Project_File:");
    write(&process, "fold.yaml", &text);
    let (line, related_line) = (line_of(&text, text.find("Is_Project_File").unwrap()), line_of(&text, text.find("is_project_file").unwrap()));

    let stdout = catalog_plain(&process, &["fold.yaml"], 1);
    assert!(stdout.contains("is not a valid callable catalog"), "{stdout}");
    assert!(stdout.contains(&format!("[{CATALOG_CODE}]")), "{stdout}");
    assert!(stdout.contains(&format!("(at fold.yaml:{line}:5)")), "{stdout}");
    assert!(stdout.contains(&format!("related: `is_project_file` is declared here (at fold.yaml:{related_line}:5)")), "{stdout}");

    let report = catalog_json(&process, &["fold.yaml"], 1);
    let record = &report["files"][0];
    assert_eq!(record["error"], "schema", "{record:#}");
    assert_eq!(record["code"], CATALOG_CODE, "{record:#}");
    assert!(record["message"].as_str().unwrap().contains("name the same callable"), "{record:#}");
    assert_eq!((record["line"].as_u64(), record["column"].as_u64()), (Some(line), Some(5)), "{record:#}");
    assert_eq!(record["span"], span_of(&text, "Is_Project_File"), "{record:#}");
    let related = record["related"].as_array().expect("related locations");
    assert_eq!(related.len(), 1, "{record:#}");
    assert_eq!(related[0]["file"], "fold.yaml", "{record:#}");
    assert_eq!((related[0]["line"].as_u64(), related[0]["column"].as_u64()), (Some(related_line), Some(5)), "{record:#}");
    assert_eq!(related[0]["span"], span_of(&text, "is_project_file"), "{record:#}");
    assert_sentinels_untouched(&process);
}

/// A host catalog may not reuse a built-in function's name. In a checkout
/// the related location is read from the repository's `functions.yaml`;
/// outside one it keeps that repository path, unlocated.
#[test]
fn catalog_a_built_in_name_reuse_exits_1_with_the_built_in_declaration_related() {
    let text = sentinel_catalog("    project_label:", "    min:");
    let line = line_of(&text, text.find("    min:").unwrap() + 4);

    // Outside a repository.
    let process = CliProcessFixture::new();
    write(&process, "reuse.yaml", &text);
    let stdout = catalog_plain(&process, &["reuse.yaml"], 1);
    assert!(stdout.contains(&format!("[{CATALOG_CODE}] `types.min` reuses the name of the built-in function `min` (at reuse.yaml:{line}:5)")), "{stdout}");
    assert!(stdout.contains(&format!("(at {FUNCTIONS_PATH})")), "{stdout}");
    let record = catalog_json(&process, &["reuse.yaml"], 1)["files"][0].clone();
    assert_eq!(record["code"], CATALOG_CODE, "{record:#}");
    assert_eq!((record["line"].as_u64(), record["column"].as_u64()), (Some(line), Some(5)), "{record:#}");
    assert_eq!(record["related"][0]["file"], FUNCTIONS_PATH, "{record:#}");
    assert!(record["related"][0]["line"].is_null(), "{record:#}");
    assert!(record["related"][0]["span"].is_object(), "{record:#}");
    assert_sentinels_untouched(&process);

    // In a checkout holding the built-in catalog.
    let process = CliProcessFixture::new();
    if !process.initialize_repository() {
        eprintln!("skipping the checkout half: git is unavailable");
        return;
    }
    write(&process, FUNCTIONS_PATH, FUNCTIONS_CATALOG);
    write(&process, "reuse.yaml", &text);
    let record = catalog_json(&process, &["reuse.yaml"], 1)["files"][0].clone();
    let related = &record["related"][0];
    let file = PathBuf::from(related["file"].as_str().expect("a related file"));
    assert!(file.ends_with(FUNCTIONS_PATH), "{record:#}");
    let related_line = related["line"].as_u64().expect("a located built-in declaration") as usize;
    let declared = FUNCTIONS_CATALOG.lines().nth(related_line - 1).expect("the line exists");
    assert!(declared.trim_start().starts_with("min:"), "line {related_line} is `{declared}`: {record:#}");
    assert_eq!(related["column"].as_u64(), Some((declared.len() - declared.trim_start().len() + 1) as u64), "{record:#}");
    let stdout = catalog_plain(&process, &["reuse.yaml"], 1);
    assert!(stdout.contains(&format!("functions.yaml:{related_line}:")), "{stdout}");
    assert_sentinels_untouched(&process);
}

/// Shape failures stay what they were: an unknown top-level `$schema` is a
/// located kind-contract problem, a duplicate key a located load error.
/// Neither reaches the catalog reader's code, nor any sentinel.
#[test]
fn catalog_a_malformed_catalog_exits_1_located_as_a_contract_or_load_failure() {
    let process = CliProcessFixture::new();
    write(&process, "unknown.yaml", &sentinel_catalog("categories:", "$schema: {}\ncategories:"));
    let duplicate = sentinel_catalog("", "") + "    project_label: x\n";
    write(&process, "duplicate.yaml", &duplicate);

    let stdout = catalog_plain(&process, &["unknown.yaml"], 1);
    assert!(stdout.contains("failed schema validation") && stdout.contains("(at line 2, column 1)"), "{stdout}");
    let record = catalog_json(&process, &["unknown.yaml"], 1)["files"][0].clone();
    assert!(record.get("error").is_none() && record.get("code").is_none(), "{record:#}");
    let problems = record["problems"].as_array().expect("problems");
    assert_eq!(problems.len(), 1, "{record:#}");
    assert!(problems[0]["message"].as_str().unwrap().contains("'$schema' was unexpected"), "{record:#}");
    assert_eq!((problems[0]["line"].as_u64(), problems[0]["column"].as_u64()), (Some(2), Some(1)), "{record:#}");

    let line = duplicate.lines().count() as u64;
    let stdout = catalog_plain(&process, &["duplicate.yaml"], 1);
    assert!(stdout.contains("is not a valid YAML document") && stdout.contains(&format!("(at line {line}, column 5)")), "{stdout}");
    let record = catalog_json(&process, &["duplicate.yaml"], 1)["files"][0].clone();
    assert_eq!(record["error"], "yaml_parse", "{record:#}");
    assert_eq!((record["line"].as_u64(), record["column"].as_u64()), (Some(line), Some(5)), "{record:#}");
    assert_sentinels_untouched(&process);
}

/// Control for the catalog route: an ordinary schema file holding a
/// template still resolves its slot, so a slot whose data file is missing
/// leaves the check unavailable (`error: slots`), never valid as written.
#[test]
fn catalog_route_leaves_an_ordinary_template_schema_resolved() {
    let process = CliProcessFixture::new();
    write(&process, "schema.yaml", "kind: schema\n$schema:\n    status: \"{{ load_data('./status.yaml', '{ type: string }').type }}(required)\"\n");

    let record = catalog_json(&process, &["schema.yaml"], 1)["files"][0].clone();
    assert_eq!(record["error"], "slots", "{record:#}");
    assert!(record.get("code").is_none(), "{record:#}");
}

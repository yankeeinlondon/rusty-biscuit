//! Unit and integration tests for the schemas subsystem.
//!
//! `clean_quoting` and `clean_suggestions` cover the Phase 5
//! schema-aware clean repair layer (acceptance matrix A-4 / B-5);
//! the remaining tests exercise [`super`] validation, resolution,
//! and effective-schema assembly.

mod clean_quoting;
mod clean_suggestions;

use super::*;
use crate::markdown::Markdown;

fn md_with_schema(yaml_body: &str) -> Markdown {
    let content = format!("---\n{yaml_body}---\nbody\n");
    let md: Markdown = content.as_str().into();
    md
}

#[test]
fn validates_inline_schema_success() {
    let md = md_with_schema("$schema:\n  title: 'string(required)'\ntitle: Hello\n");
    let api = DarkmatterSchemas::new();
    let report = api.validate(&md).unwrap();
    assert!(report.valid, "expected valid: {:?}", report.problems);
}

#[test]
fn validates_inline_schema_missing_required() {
    let md = md_with_schema("$schema:\n  title: 'string(required)'\nother: stuff\n");
    let api = DarkmatterSchemas::new();
    let report = api.validate(&md).unwrap();
    assert!(!report.valid);
    assert!(!report.problems.is_empty());
}

#[test]
fn no_schema_no_baseline_is_vacuously_valid() {
    let md = md_with_schema("name: alice\n");
    let api = DarkmatterSchemas::new();
    let report = api.validate(&md).unwrap();
    assert!(report.valid);
    assert!(report.problems.is_empty());
}

#[test]
fn baseline_applies_when_document_has_no_schema() {
    let md = md_with_schema("title: hi\n");
    let baseline = SimplifiedSchema::Single(SchemaShape {
        properties: {
            let mut m = indexmap::IndexMap::new();
            m.insert(
                "owner".into(),
                PropertyDef::Single(PropertyAtom {
                    ty: TypeExpr::Primitive(SimplifiedType::String),
                    is_array: false,
                    constraints: vec![Constraint::Required],
                    array_constraints: vec![],
                    description: None,
                }),
            );
            m
        },
        ..Default::default()
    });
    let api = DarkmatterSchemas::new().with_baseline(baseline).unwrap();
    let report = api.validate(&md).unwrap();
    assert!(!report.valid);
    assert!(
        report
            .problems
            .iter()
            .any(|p| p.message.to_ascii_lowercase().contains("owner"))
    );
}

#[test]
fn baseline_merges_with_document_schema() {
    let md = md_with_schema("$schema:\n  title: 'string(required)'\ntitle: hi\n");
    let baseline = SimplifiedSchema::Single(SchemaShape {
        properties: {
            let mut m = indexmap::IndexMap::new();
            m.insert(
                "owner".into(),
                PropertyDef::Single(PropertyAtom {
                    ty: TypeExpr::Primitive(SimplifiedType::String),
                    is_array: false,
                    constraints: vec![Constraint::Required],
                    array_constraints: vec![],
                    description: None,
                }),
            );
            m
        },
        ..Default::default()
    });
    let api = DarkmatterSchemas::new().with_baseline(baseline).unwrap();
    let report = api.validate(&md).unwrap();
    assert!(!report.valid);
    assert!(
        report
            .problems
            .iter()
            .any(|p| p.message.to_ascii_lowercase().contains("owner"))
    );
}

#[test]
fn validator_cache_reuses_across_documents() {
    let api = DarkmatterSchemas::new();
    let md1 = md_with_schema("$schema:\n  x: number\nx: 1\n");
    let md2 = md_with_schema("$schema:\n  x: number\nx: 2\n");
    api.validate(&md1).unwrap();
    // Measured as a delta, not an absolute count: the cache is now shared
    // process-wide (F8), so unrelated schemas compiled elsewhere in this
    // test process may already occupy entries. The invariant is that a
    // second document sharing md1's schema adds no new entry.
    let after_first = api.cache().len();
    api.validate(&md2).unwrap();
    assert_eq!(api.cache().len(), after_first);
}

#[test]
fn coerces_boolish_string_against_inline_schema() {
    let md = md_with_schema("$schema:\n  flag: boolean\nflag: \"true\"\n");
    let api = DarkmatterSchemas::new();
    let report = api.validate(&md).unwrap();
    assert!(report.valid, "expected valid: {:?}", report.problems);
}

#[test]
fn coerces_numeric_string_against_inline_schema() {
    let md = md_with_schema("$schema:\n  n: number\nn: \"42\"\n");
    let api = DarkmatterSchemas::new();
    let report = api.validate(&md).unwrap();
    assert!(report.valid, "expected valid: {:?}", report.problems);
}

#[test]
fn ambiguous_string_still_reports_type_problem() {
    let md = md_with_schema("$schema:\n  flag: boolean(required)\nflag: \"yes\"\n");
    let api = DarkmatterSchemas::new();
    let report = api.validate(&md).unwrap();
    assert!(!report.valid);
    assert!(
        report
            .problems
            .iter()
            .any(|p| p.kind == ValidationProblemKind::Type && p.path == "/flag"),
        "expected Type problem on /flag: {:?}",
        report.problems
    );
}

#[test]
fn coerces_baseline_merged_field_without_document_schema() {
    // Document has no `$schema`; the boolean field comes solely from the
    // baseline. Coercion reads the post-merge json_schema, so it fires even
    // though the document's `simplified` AST never declares `enabled`.
    let md = md_with_schema("enabled: \"false\"\n");
    let baseline = SimplifiedSchema::Single(SchemaShape {
        properties: {
            let mut m = indexmap::IndexMap::new();
            m.insert(
                "enabled".into(),
                PropertyDef::Single(PropertyAtom {
                    ty: TypeExpr::Primitive(SimplifiedType::Boolean),
                    is_array: false,
                    constraints: vec![],
                    array_constraints: vec![],
                    description: None,
                }),
            );
            m
        },
        ..Default::default()
    });
    let api = DarkmatterSchemas::new().with_baseline(baseline).unwrap();
    let report = api.validate(&md).unwrap();
    assert!(report.valid, "expected valid: {:?}", report.problems);
}

#[test]
fn coerces_raw_json_schema_baseline_with_no_simplified_ast() {
    // Raw JSON Schema baseline → `simplified` is None on the effective
    // schema. Coercion never consults the AST, so the boolish string still
    // coerces to a real boolean and validates.
    let md = md_with_schema("flag: \"true\"\n");
    let raw = serde_json::json!({
        "type": "object",
        "properties": { "flag": {"type": "boolean"} }
    });
    let api = DarkmatterSchemas::new()
        .with_baseline_json_schema(raw)
        .unwrap();
    let effective = api.effective_for(&md).unwrap().unwrap();
    assert!(
        effective.simplified.is_none(),
        "raw JSON Schema baseline should have no simplified AST"
    );
    let report = api.validate(&md).unwrap();
    assert!(report.valid, "expected valid: {:?}", report.problems);
}

#[test]
fn enriches_problem_with_property_description() {
    // End-to-end: the SimplifiedSchema `->` description surfaces on the
    // populated `ValidationProblem.description`.
    let md = md_with_schema(
        "$schema:\n  title: 'string(required) -> The headline shown in listings'\nother: x\n",
    );
    let api = DarkmatterSchemas::new();
    let report = api.validate(&md).unwrap();
    assert!(!report.valid);
    assert!(
        report
            .problems
            .iter()
            .any(|p| p.description.as_deref() == Some("The headline shown in listings")),
        "expected a populated description: {:?}",
        report.problems
    );
}

#[test]
fn description_equal_to_message_is_suppressed() {
    // Decision #9: a description byte-for-byte equal to the rendered message
    // is dropped so the same sentence is not printed twice. The missing
    // `title` message is `"title" is a required property`.
    let md = md_with_schema("other: x\n");
    let raw = serde_json::json!({
        "type": "object",
        "properties": {
            "title": { "type": "string", "description": "\"title\" is a required property" }
        },
        "required": ["title"]
    });
    let api = DarkmatterSchemas::new()
        .with_baseline_json_schema(raw)
        .unwrap();
    let report = api.validate(&md).unwrap();
    let problem = report
        .problems
        .iter()
        .find(|p| p.property.as_deref() == Some("title"))
        .expect("missing-title problem");
    assert_eq!(problem.message, "\"title\" is a required property");
    assert_eq!(
        problem.description, None,
        "description equal to the message must be suppressed"
    );
}

#[test]
fn whitespace_only_description_is_suppressed() {
    // Decision #8: a whitespace-only description renders nothing.
    let md = md_with_schema("title:\n  - 1\n  - 2\n");
    let raw = serde_json::json!({
        "type": "object",
        "properties": {
            "title": { "type": "string", "description": "   " }
        }
    });
    let api = DarkmatterSchemas::new()
        .with_baseline_json_schema(raw)
        .unwrap();
    let report = api.validate(&md).unwrap();
    let problem = report
        .problems
        .iter()
        .find(|p| p.path == "/title")
        .expect("title type problem");
    assert_eq!(
        problem.description, None,
        "whitespace-only description must be suppressed"
    );
}

// ── file_ref_fallback_dir threading (Phase 2 Track B) ───────────────

/// RAII guard that restores the process CWD on drop, even on panic.
/// Tests that mutate CWD are annotated with
/// `#[serial_test::serial("darkmatter-file-cwd")]` to prevent races with
/// the ambient-CWD tests in `format::tests` and `validate::tests`.
struct CwdGuard {
    prior: std::path::PathBuf,
}

impl CwdGuard {
    fn enter(dir: &std::path::Path) -> Self {
        let prior = std::env::current_dir().expect("read CWD");
        std::env::set_current_dir(dir).expect("set CWD");
        Self { prior }
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.prior);
    }
}

/// A `file`-typed schema property value resolves via the captured
/// fallback even when the process CWD is an unrelated directory
/// (verification goal #7 precursor). The format validator closure
/// captures the fallback dir and resolves via `FileReference::resolve_from`
/// instead of the ambient-CWD `resolve()`.
#[test]
#[serial_test::serial(darkmatter_file_cwd)]
fn file_format_resolves_via_fallback_when_cwd_unrelated() {
    let launch_dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(launch_dir.path().join("spec.md"), "# Spec\n").expect("write spec");
    let unrelated_dir = tempfile::tempdir().expect("tempdir");

    let md = md_with_schema("$schema:\n  spec: 'file(eager; required)'\nspec: spec.md\n");
    let api = DarkmatterSchemas::new()
        .with_file_ref_fallback_dir(launch_dir.path().to_path_buf());

    let _cwd = CwdGuard::enter(unrelated_dir.path());
    let report = api.validate(&md).expect("validate");

    assert!(
        report.valid,
        "expected file-format validation to resolve via fallback: {:?}",
        report.problems,
    );
}

/// A `file`-typed schema property value that exists under the ambient CWD
/// but NOT under the prompt directory or the fallback dir fails validation
/// — proving the document-first / fallback anchors (not the ambient CWD)
/// drive resolution when configured. The prompt has a real file source in
/// a third directory so its `base_dir` is distinct from the CWD.
#[test]
#[serial_test::serial(darkmatter_file_cwd)]
fn file_format_fallback_rejects_when_not_under_fallback() {
    let prompt_dir = tempfile::tempdir().expect("tempdir");
    let fallback_dir = tempfile::tempdir().expect("tempdir");
    let cwd_dir = tempfile::tempdir().expect("tempdir");
    // File exists under CWD but NOT under the prompt dir or the fallback dir.
    std::fs::write(cwd_dir.path().join("ambient.md"), "# Ambient\n").expect("write");

    let md = prompt_with_source(prompt_dir.path(), "$schema:\n  spec: 'file(eager; required)'\nspec: ambient.md\n");
    let api = DarkmatterSchemas::new()
        .with_file_ref_fallback_dir(fallback_dir.path().to_path_buf());

    let _cwd = CwdGuard::enter(cwd_dir.path());
    let report = api.validate(&md).expect("validate");

    assert!(
        !report.valid,
        "expected validation to fail because ambient.md is under neither the prompt dir nor the fallback dir: {:?}",
        report.problems,
    );
}

/// `$schema: ./schema.yaml` resolves relative to the document directory,
/// NOT the fallback dir (verification goal #6). Only `file`-typed property
/// VALUES use the fallback; `$schema` REFERENCE resolution stays
/// document-relative in `schemas::resolve`.
#[test]
fn schema_reference_stays_document_relative_with_fallback() {
    let doc_dir = tempfile::tempdir().expect("tempdir");
    let fallback_dir = tempfile::tempdir().expect("tempdir");
    // schema.yaml lives ONLY in the document dir.
    std::fs::write(
        doc_dir.path().join("schema.yaml"),
        "title: string(required)\n",
    )
    .expect("write schema");
    let doc_path = doc_dir.path().join("doc.md");
    std::fs::write(
        &doc_path,
        "---\n$schema: ./schema.yaml\ntitle: Hello\n---\nbody\n",
    )
    .expect("write doc");

    let md = Markdown::try_from(doc_path.as_path()).expect("read doc");
    // Fallback points at a dir WITHOUT schema.yaml — if the $schema
    // reference were resolved via the fallback, this would fail.
    let api = DarkmatterSchemas::new()
        .with_file_ref_fallback_dir(fallback_dir.path().to_path_buf());
    let report = api.validate(&md).expect("validate");
    assert!(
        report.valid,
        "$schema reference must resolve from the document dir, not the fallback: {:?}",
        report.problems,
    );
}

/// A root-union `$schema` with a string arm referencing a YAML file
/// still resolves that arm relative to the document directory, not the
/// fallback (verification goal #6, root-union variant).
#[test]
fn root_union_schema_string_arm_stays_document_relative_with_fallback() {
    let doc_dir = tempfile::tempdir().expect("tempdir");
    let fallback_dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        doc_dir.path().join("arm-a.yaml"),
        "kind: string(required)\n",
    )
    .expect("write arm-a");
    let doc_path = doc_dir.path().join("doc.md");
    std::fs::write(
        &doc_path,
        "---\n$schema:\n  - ./arm-a.yaml\n  - fallback: string\nkind: feature\n---\nbody\n",
    )
    .expect("write doc");

    let md = Markdown::try_from(doc_path.as_path()).expect("read doc");
    let api = DarkmatterSchemas::new()
        .with_file_ref_fallback_dir(fallback_dir.path().to_path_buf());
    let report = api.validate(&md).expect("validate");
    assert!(
        report.valid,
        "root-union string arm must resolve from the document dir: {:?}",
        report.problems,
    );
}

// ── file-property document-first resolution (Finding 1) ──────────────
//
// A `file`-typed schema property VALUE must resolve document-first (the
// prompt directory) before the launch-area fallback, matching the
// expression path's `file_exists`/`frontmatter` order. These L1 tests
// write a real prompt file so `base_dir` is the prompt directory, point
// the fallback at a separate directory, and assert each rung of the
// resolution order independently of the ambient process CWD.

/// Writes `---\n{frontmatter}---\nbody\n` to `dir/prompt.md` and reads it
/// back as a `Markdown` whose source is that file (so `base_dir_for`
/// returns the prompt directory).
fn prompt_with_source(dir: &std::path::Path, frontmatter: &str) -> Markdown {
    let path = dir.join("prompt.md");
    std::fs::write(&path, format!("---\n{frontmatter}---\nbody\n")).expect("write prompt");
    Markdown::try_from(path.as_path()).expect("read prompt")
}

/// Document-first precedence: a `spec.md` present in BOTH the prompt
/// directory and the launch-area fallback must resolve to the prompt-dir
/// copy. We prove the prompt-dir copy wins by deleting it afterwards is
/// unnecessary — instead we assert validation succeeds while the fallback
/// copy is the only other candidate, and the conflicting-filename
/// precedence is covered structurally by the resolver unit test
/// `resolve_ctx::document_relative_hit_wins_over_fallback_conflict`.
/// Here we additionally guard that resolution does not depend on the
/// ambient CWD by switching it to an unrelated directory.
#[test]
#[serial_test::serial(darkmatter_file_cwd)]
fn file_property_present_in_both_resolves_prompt_dir() {
    let prompt_dir = tempfile::tempdir().expect("tempdir");
    let fallback_dir = tempfile::tempdir().expect("tempdir");
    let unrelated = tempfile::tempdir().expect("tempdir");
    std::fs::write(prompt_dir.path().join("spec.md"), "# prompt copy\n").expect("write prompt spec");
    std::fs::write(fallback_dir.path().join("spec.md"), "# fallback copy\n").expect("write fallback spec");

    let md = prompt_with_source(prompt_dir.path(), "$schema:\n  spec: 'file(eager; required)'\nspec: spec.md\n");
    let api = DarkmatterSchemas::new()
        .with_file_ref_fallback_dir(fallback_dir.path().to_path_buf());

    let _cwd = CwdGuard::enter(unrelated.path());
    let report = api.validate(&md).expect("validate");
    assert!(
        report.valid,
        "a file value present in both dirs must validate via the prompt dir: {:?}",
        report.problems,
    );
}

/// A `file` value that exists ONLY in the prompt directory still validates
/// when a fallback is configured (document-first hit). Independent of the
/// ambient CWD.
#[test]
#[serial_test::serial(darkmatter_file_cwd)]
fn file_property_present_only_in_prompt_dir_validates() {
    let prompt_dir = tempfile::tempdir().expect("tempdir");
    let fallback_dir = tempfile::tempdir().expect("tempdir");
    let unrelated = tempfile::tempdir().expect("tempdir");
    std::fs::write(prompt_dir.path().join("local.md"), "# local\n").expect("write local");

    let md = prompt_with_source(prompt_dir.path(), "$schema:\n  spec: 'file(eager; required)'\nspec: ./local.md\n");
    let api = DarkmatterSchemas::new()
        .with_file_ref_fallback_dir(fallback_dir.path().to_path_buf());

    let _cwd = CwdGuard::enter(unrelated.path());
    let report = api.validate(&md).expect("validate");
    assert!(
        report.valid,
        "a file value beside the prompt must validate even with a fallback set: {:?}",
        report.problems,
    );
}

/// A `file` value that exists ONLY under the launch-area fallback (not the
/// prompt directory) still validates via the fallback rung. Independent of
/// the ambient CWD.
#[test]
#[serial_test::serial(darkmatter_file_cwd)]
fn file_property_present_only_in_fallback_validates() {
    let prompt_dir = tempfile::tempdir().expect("tempdir");
    let fallback_dir = tempfile::tempdir().expect("tempdir");
    let unrelated = tempfile::tempdir().expect("tempdir");
    std::fs::write(fallback_dir.path().join("caller.md"), "# caller\n").expect("write caller");

    let md = prompt_with_source(prompt_dir.path(), "$schema:\n  spec: 'file(eager; required)'\nspec: caller.md\n");
    let api = DarkmatterSchemas::new()
        .with_file_ref_fallback_dir(fallback_dir.path().to_path_buf());

    let _cwd = CwdGuard::enter(unrelated.path());
    let report = api.validate(&md).expect("validate");
    assert!(
        report.valid,
        "a file value under the launch-area fallback must validate: {:?}",
        report.problems,
    );
}

/// Guard: with both a prompt-dir anchor and a fallback configured, a value
/// that exists ONLY under the process CWD (neither the prompt dir nor the
/// fallback) must NOT validate — there is no ambient-CWD rung on the
/// production path.
#[test]
#[serial_test::serial(darkmatter_file_cwd)]
fn file_property_present_only_in_cwd_does_not_validate() {
    let prompt_dir = tempfile::tempdir().expect("tempdir");
    let fallback_dir = tempfile::tempdir().expect("tempdir");
    let cwd_dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(cwd_dir.path().join("ambient.md"), "# ambient\n").expect("write ambient");

    let md = prompt_with_source(prompt_dir.path(), "$schema:\n  spec: 'file(eager; required)'\nspec: ambient.md\n");
    let api = DarkmatterSchemas::new()
        .with_file_ref_fallback_dir(fallback_dir.path().to_path_buf());

    let _cwd = CwdGuard::enter(cwd_dir.path());
    let report = api.validate(&md).expect("validate");
    assert!(
        !report.valid,
        "a file value found only under the ambient CWD must NOT validate: {:?}",
        report.problems,
    );
}

// ── Phase 3: normalize_frontmatter + read-only validation contract ────

/// Writes a `---\n{frontmatter}---\nbody\n` document to `dir/prompt.md`
/// and returns the `EffectiveSchema` for it, with the prompt directory as
/// `base_dir`.
fn effective_for_prompt(
    dir: &std::path::Path,
    frontmatter: &str,
) -> EffectiveSchema {
    let path = dir.join("prompt.md");
    std::fs::write(&path, format!("---\n{frontmatter}---\nbody\n")).expect("write prompt");
    let md = Markdown::try_from(path.as_path()).expect("read prompt");
    DarkmatterSchemas::new()
        .effective_for(&md)
        .expect("effective_for")
        .expect("schema present")
}

/// `normalize_frontmatter` rewrites a present eager-`file` value to its
/// repo-relative resolved path, and the caller's input `Value` is left
/// byte-identical (Decision #3: pure).
#[test]
fn normalize_frontmatter_rewrites_eager_file_and_leaves_input_untouched() {
    let repo = tempfile::tempdir().expect("tempdir");
    // A `.git` marker makes the projection git-root-relative.
    std::fs::create_dir_all(repo.path().join(".git")).expect("git marker");
    std::fs::create_dir_all(repo.path().join("area")).expect("area dir");
    std::fs::write(repo.path().join("area/spec.md"), "# Spec\n").expect("write spec");

    let effective = effective_for_prompt(
        &repo.path().join("area"),
        "$schema:\n  spec: 'file(eager; required)'\nspec: ./spec.md\n",
    );

    let input = serde_json::json!({ "spec": "./spec.md" });
    let snapshot = input.clone();
    let pending = HashSet::new();
    let outcome = effective.normalize_frontmatter(&input, &pending);
    assert!(outcome.changed, "expected the eager-file value to be rewritten");
    assert_eq!(outcome.value["spec"], serde_json::json!("area/spec.md"));
    // Decision #3: the caller's input is never mutated.
    assert_eq!(input, snapshot, "normalize_frontmatter must not mutate its input");
}

/// A `composition_pending` set containing the eager-file key leaves that
/// key verbatim while a concrete sibling eager-file value is still
/// rewritten (Decision #4).
#[test]
fn normalize_frontmatter_skips_pending_key_and_rewrites_sibling() {
    let repo = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(repo.path().join(".git")).expect("git marker");
    std::fs::create_dir_all(repo.path().join("area")).expect("area dir");
    std::fs::write(repo.path().join("area/spec.md"), "# Spec\n").expect("write spec");
    std::fs::write(repo.path().join("area/design.md"), "# Design\n").expect("write design");

    let effective = effective_for_prompt(
        &repo.path().join("area"),
        "$schema:\n  spec: 'file(eager; required)'\n  design: 'file(eager)'\nspec: ./spec.md\ndesign: ./design.md\n",
    );

    let input = serde_json::json!({
        "spec": "$(echo spec.md)",
        "design": "./design.md"
    });
    let pending: HashSet<String> = ["spec".to_string()].into_iter().collect();
    let outcome = effective.normalize_frontmatter(&input, &pending);
    assert!(outcome.changed, "concrete sibling must still be rewritten");
    // Pending key is left verbatim.
    assert_eq!(outcome.value["spec"], serde_json::json!("$(echo spec.md)"));
    // Concrete sibling is rewritten.
    assert_eq!(outcome.value["design"], serde_json::json!("area/design.md"));
}

/// Decision #3 regression: `validate_with_positions` keeps the documented
/// read-only contract even when the schema has eager `file` properties.
/// The caller's `serde_json::Value` is byte-identical afterward — the
/// rewrite never runs inside validation.
#[test]
fn validate_with_positions_does_not_mutate_eager_file_input() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("spec.md"), "# Spec\n").expect("write spec");

    let effective = effective_for_prompt(
        dir.path(),
        "$schema:\n  spec: 'file(eager; required)'\nspec: ./spec.md\n",
    );

    let input = serde_json::json!({ "spec": "./spec.md" });
    let snapshot = input.clone();
    let report = effective.validate_with_positions(&input, &PositionMap::new());
    assert!(report.valid, "expected valid: {:?}", report.problems);
    assert_eq!(
        input, snapshot,
        "validate_with_positions must not mutate its input, even with eager-file properties",
    );
}

// ── Phase 4: wider read-only validation sweep ───────────────────────
//
// Complements the unit-level regression above with a wider call-site
// sweep (spec Integration bullet 4): every validation-only entry point
// keeps the documented read-only contract. The eager-`file` rewrite is
// opt-in via `normalize_frontmatter`; it must never fire from a
// validation-only call.

/// `EffectiveSchema::validate` (the thin wrapper) does not mutate the
/// caller's `serde_json::Value`, even when an eager-`file` property's
/// raw value is eligible for rewriting.
#[test]
fn validate_does_not_mutate_eager_file_input() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("spec.md"), "# Spec\n").expect("write spec");

    let effective = effective_for_prompt(
        dir.path(),
        "$schema:\n  spec: 'file(eager; required)'\nspec: ./spec.md\n",
    );

    let input = serde_json::json!({ "spec": "./spec.md" });
    let snapshot = input.clone();
    let report = effective.validate(&input);
    assert!(report.valid, "expected valid: {:?}", report.problems);
    assert_eq!(
        input, snapshot,
        "validate must not mutate its input, even with eager-file properties",
    );
}

/// `DarkmatterSchemas::validate` (the top-level API) does not mutate the
/// document's stored frontmatter, even when an eager-`file` property's
/// raw value is eligible for rewriting. Compose owns the write-back; the
/// validation-only library surface stays read-only.
#[test]
fn darkmatter_schemas_validate_does_not_mutate_eager_file_frontmatter() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("spec.md"), "# Spec\n").expect("write spec");
    let prompt_path = dir.path().join("prompt.md");
    std::fs::write(
        &prompt_path,
        "---\n$schema:\n  spec: 'file(eager; required)'\nspec: ./spec.md\n---\nbody\n",
    )
    .expect("write prompt");

    let md = Markdown::try_from(prompt_path.as_path()).expect("read prompt");
    let api = DarkmatterSchemas::new();
    let report = api.validate(&md).expect("validate");
    assert!(report.valid, "expected valid: {:?}", report.problems);
    // The stored frontmatter still carries the raw reference — the
    // rewrite only runs through `normalize_frontmatter` / compose.
    assert_eq!(
        md.frontmatter().as_map().get("spec"),
        Some(&serde_json::json!("./spec.md")),
        "DarkmatterSchemas::validate must not rewrite the stored eager-file value",
    );
}

/// A root-union schema whose committed arm carries an eager-`file`
/// property also keeps the read-only contract: validation reports
/// validity without rewriting the caller's value.
#[test]
fn validate_root_union_eager_file_arm_does_not_mutate_input() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("spec.md"), "# Spec\n").expect("write spec");

    let effective = effective_for_prompt(
        dir.path(),
        "$schema:\n  - spec: 'file(eager; required)'\n    kind: 'string(required)'\nspec: ./spec.md\nkind: feature\n",
    );

    let input = serde_json::json!({ "spec": "./spec.md", "kind": "feature" });
    let snapshot = input.clone();
    let report = effective.validate(&input);
    assert!(report.valid, "expected valid: {:?}", report.problems);
    assert_eq!(
        input, snapshot,
        "validate must not mutate root-union eager-file input",
    );
}

// ── `generated` baseline integration (Phase 2) ─────────────────────────
//
// End-to-end at the public `DarkmatterSchemas` API: an authored document
// omitting a baseline `ctx.today` `generated; required` property validates
// cleanly (spec semantics point 1), while a wrongly-typed `ctx.today`
// value fails type validation (point 3).

/// Builds a baseline `SimplifiedSchema` from a YAML body.
fn baseline_from_yaml(yaml_body: &str) -> SimplifiedSchema {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml_body).expect("yaml parse");
    super::simplified::parse_yaml_schema(&value).expect("baseline schema parse")
}

#[test]
fn baseline_generated_ctx_validates_when_authored_doc_omits_ctx() {
    let baseline = baseline_from_yaml(
        "ctx:\n  today: \"date(generated; required) -> today's date, host-supplied\"",
    );
    let api = DarkmatterSchemas::new()
        .with_baseline(baseline)
        .expect("baseline converts");

    // Authored document omits `ctx` entirely — validates (spec point 1).
    let md = md_with_schema("title: Hello\n");
    let report = api.validate(&md).expect("validate");
    assert!(
        report.valid,
        "authored doc omitting generated ctx must validate: {:?}",
        report.problems,
    );
}

#[test]
fn baseline_generated_ctx_type_checks_wrongly_typed_value() {
    let baseline = baseline_from_yaml(
        "ctx:\n  today: \"date(generated; required) -> today's date, host-supplied\"",
    );
    let api = DarkmatterSchemas::new()
        .with_baseline(baseline)
        .expect("baseline converts");

    // Host supplies a wrongly-typed `ctx.today` — fails validation
    // (spec point 3: the non-nullable type semantics of `required` are
    // preserved even though static presence is suppressed). A numeric
    // value against `date` (`{ type: "string", format: "date" }`) is
    // rejected by the format/type check.
    let md = md_with_schema("ctx:\n  today: 42\n");
    let report = api.validate(&md).expect("validate");
    assert!(
        !report.valid,
        "wrongly-typed ctx.today must fail validation: {:?}",
        report.problems,
    );
    assert!(
        report
            .problems
            .iter()
            .any(|p| p.path == "/ctx/today"),
        "expected a problem on /ctx/today: {:?}",
        report.problems,
    );
}

#[test]
fn baseline_generated_ctx_accepts_correctly_typed_value() {
    let baseline = baseline_from_yaml(
        "ctx:\n  today: \"date(generated; required) -> today's date, host-supplied\"",
    );
    let api = DarkmatterSchemas::new()
        .with_baseline(baseline)
        .expect("baseline converts");

    // Host supplies a correctly-typed `ctx.today` — validates.
    let md = md_with_schema("ctx:\n  today: 2026-07-04\n");
    let report = api.validate(&md).expect("validate");
    assert!(
        report.valid,
        "correctly-typed ctx.today must validate: {:?}",
        report.problems,
    );
}

// ── Phase 4: library base-schema accessors ───────────────────────────

/// `darkmatter_base_schema()` returns a non-empty baseline schema with the
/// expected top-level properties present (spec testing requirement 1).
#[test]
fn darkmatter_base_schema_returns_expected_properties() {
    let schema = super::darkmatter_base_schema();
    let shape = match schema {
        super::SimplifiedSchema::Single(s) => s,
        other => panic!("expected Single, got {other:?}"),
    };
    assert!(
        !shape.properties.is_empty(),
        "baseline schema must declare properties"
    );
    for key in ["$schema", "title", "ctx"] {
        assert!(
            shape.properties.contains_key(key),
            "baseline schema missing property `{key}`"
        );
    }
}

/// `darkmatter_base_json_schema()` returns a compiled JSON Schema that
/// validates known-good frontmatter and rejects a wrongly-typed `title`
/// (spec testing requirements 3 and 4).
#[test]
fn darkmatter_base_json_schema_validates_known_samples() {
    let json = super::darkmatter_base_json_schema();
    let validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .build(&json)
        .expect("compiled baseline must build a validator");

    let valid = serde_json::json!({
        "title": "Hello",
        "draft": false,
        "tags": ["a", "b"],
    });
    assert!(
        validator.is_valid(&valid),
        "known-good frontmatter must validate"
    );

    let invalid_title = serde_json::json!({ "title": 42 });
    assert!(
        !validator.is_valid(&invalid_title),
        "wrongly-typed title must be rejected"
    );
}

/// The borrowed accessor returns the same cached allocation, while the
/// owned accessor preserves independent-value semantics.
#[test]
fn darkmatter_base_json_schema_is_cached() {
    let borrowed_a = super::darkmatter_base_json_schema_ref();
    let borrowed_b = super::darkmatter_base_json_schema_ref();
    assert!(std::ptr::eq(borrowed_a, borrowed_b));

    let mut owned = super::darkmatter_base_json_schema();
    owned["title"] = serde_json::json!("independent clone");
    assert_ne!(&owned, borrowed_a);
}

#[test]
fn darkmatter_baseline_builder_shares_cached_json_schema() {
    let first = DarkmatterSchemas::new()
        .with_darkmatter_baseline_json_schema()
        .expect("built-in baseline must be valid");
    let second = DarkmatterSchemas::new()
        .with_darkmatter_baseline_json_schema()
        .expect("built-in baseline must be valid");
    let first = &first.baseline.expect("baseline must be configured").json_schema;
    let second = &second
        .baseline
        .expect("baseline must be configured")
        .json_schema;
    assert!(Arc::ptr_eq(first, second));
}

/// The compiled baseline allows unknown user-defined frontmatter keys
/// (Non-Goal 1; spec testing requirement 5).
#[test]
fn darkmatter_base_json_schema_allows_unknown_keys() {
    let json = super::darkmatter_base_json_schema();
    let validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .build(&json)
        .expect("compiled baseline must build a validator");

    let with_unknown = serde_json::json!({
        "custom_key": 42,
        "my_custom_namespace": { "nested": true },
    });
    assert!(
        validator.is_valid(&with_unknown),
        "unknown user keys must remain accepted"
    );
}

/// Document-level `$schema` definitions override baseline properties on
/// conflict (Non-Goal 5; spec testing requirement 6).
#[test]
fn document_schema_overrides_baseline_title() {
    let api = DarkmatterSchemas::new()
        .with_baseline(super::darkmatter_base_schema())
        .expect("baseline converts");

    // Baseline says `title` is a string. The document redeclares it as a
    // number and supplies a number; validation must follow the document
    // schema, so this is valid.
    let md = md_with_schema("$schema:\n  title: number\ntitle: 42\n");
    let report = api.validate(&md).expect("validate");
    assert!(
        report.valid,
        "document schema should override baseline title type: {:?}",
        report.problems
    );
}

// ── R-5 Priority 3: validate_with_options pending + deferral ─────────

fn effective_number_field() -> EffectiveSchema {
    let md = md_with_schema("$schema:\n  n: number\n");
    DarkmatterSchemas::new()
        .effective_for(&md)
        .expect("effective_for")
        .expect("schema present")
}

#[test]
fn validate_with_options_defers_shell_pending_value() {
    let effective = effective_number_field();
    let instance = serde_json::json!({ "n": "$(echo 1)" });
    let report = effective.validate_with_options(
        &instance,
        &PositionMap::new(),
        &ValidationOptions::default(),
    );
    assert!(report.valid, "shell-pending value deferred: {:?}", report.problems);
    assert_eq!(report.pending.len(), 1);
    assert_eq!(report.pending[0].key, "n");
    assert_eq!(report.pending[0].reason, PendingValueReason::ShellExpression);
    assert_eq!(report.pending[0].path.segments(), ["n"]);
}

#[test]
fn validate_with_options_classifies_template_pending_value() {
    let effective = effective_number_field();
    let instance = serde_json::json!({ "n": "{{ x }}" });
    let report = effective.validate_with_options(
        &instance,
        &PositionMap::new(),
        &ValidationOptions::default(),
    );
    assert!(report.valid, "template-pending value deferred: {:?}", report.problems);
    assert_eq!(report.pending.len(), 1);
    assert_eq!(report.pending[0].reason, PendingValueReason::UnresolvedTemplate);
}

#[test]
fn validate_with_options_report_policy_keeps_pending_problem() {
    let effective = effective_number_field();
    let instance = serde_json::json!({ "n": "$(echo 1)" });
    let options = ValidationOptions {
        pending_policy: PendingPolicy::Report,
        excluded_keys: HashSet::new(),
    };
    let report =
        effective.validate_with_options(&instance, &PositionMap::new(), &options);
    assert!(!report.valid, "Report policy keeps the type problem");
    assert!(report.problems.iter().any(|p| p.path == "/n"));
    // The pending value is still surfaced as data.
    assert_eq!(report.pending.len(), 1);
}

#[test]
fn validate_with_options_excludes_caller_owned_key() {
    let effective = effective_number_field();
    let instance = serde_json::json!({ "n": "not-a-number" });
    let options = ValidationOptions {
        pending_policy: PendingPolicy::Defer,
        excluded_keys: ["n".to_string()].into_iter().collect(),
    };
    let report =
        effective.validate_with_options(&instance, &PositionMap::new(), &options);
    assert!(report.valid, "excluded key's problem is dropped: {:?}", report.problems);
    // Excluded, not pending — the raw value holds no `$(...)`/`{{ }}`.
    assert!(report.pending.is_empty());
}

#[test]
fn validate_with_options_coerced_value_passes_with_no_pending() {
    let effective = effective_number_field();
    // A coercible string is not pending and validates after coercion.
    let instance = serde_json::json!({ "n": "42" });
    let report = effective.validate_with_options(
        &instance,
        &PositionMap::new(),
        &ValidationOptions::default(),
    );
    assert!(report.valid, "{:?}", report.problems);
    assert!(report.pending.is_empty());
}

#[test]
fn plain_validate_report_has_empty_pending() {
    // Compose-parity: the non-options entry points never populate `pending`.
    let effective = effective_number_field();
    let report = effective.validate(&serde_json::json!({ "n": 1 }));
    assert!(report.pending.is_empty());
}

// ── R-5 Priority 2: schema origins ──────────────────────────────────

#[test]
fn origins_attribute_document_and_baseline_properties() {
    let md = md_with_schema("$schema:\n  title: 'string(required)'\ntitle: hi\n");
    let baseline = baseline_from_yaml("owner: 'string(required)'");
    let api = DarkmatterSchemas::new().with_baseline(baseline).unwrap();
    let effective = api.effective_for(&md).unwrap().unwrap();
    assert_eq!(
        effective.origins.get("title").map(|o| o.kind),
        Some(SchemaOriginKind::Document),
    );
    assert_eq!(
        effective.origins.get("owner").map(|o| o.kind),
        Some(SchemaOriginKind::Baseline),
    );
}

#[test]
fn origins_referenced_file_carries_uri() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("schema.yaml"),
        "$schema:\n  title: 'string(required)'\n",
    )
    .unwrap();
    let doc_path = dir.path().join("doc.md");
    std::fs::write(
        &doc_path,
        "---\n$schema: ./schema.yaml\ntitle: Hello\n---\nbody\n",
    )
    .unwrap();
    let md = Markdown::try_from(doc_path.as_path()).unwrap();
    let effective = DarkmatterSchemas::new()
        .effective_for(&md)
        .unwrap()
        .unwrap();
    let origin = effective.origins.get("title").expect("title origin");
    assert_eq!(origin.kind, SchemaOriginKind::ReferencedFile);
    assert!(
        origin.uri.as_ref().map(|p| p.ends_with("schema.yaml")).unwrap_or(false),
        "expected the referenced file path, got {:?}",
        origin.uri,
    );
}

#[test]
fn dependencies_surface_import_and_example_edges() {
    // A `$schema` that pulls in a `Name@file` import and an `example(...)`
    // artifact surfaces both resolved paths on `EffectiveSchema::dependencies`
    // so a warm cache can invalidate the schema when either file changes.
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("types.yaml"), "$schema:\n  type: 'enum(a, b)'\n")
        .expect("write types");
    std::fs::write(
        dir.path().join("today-example.yaml"),
        "kind: example\ninvocation: \"{{ ctx.today }}\"\nreturns: \"2024-12-25\"\ndescription: d\n",
    )
    .expect("write example");
    let md = prompt_with_source(
        dir.path(),
        "$schema:\n  value: type@./types.yaml\n  today: \"date(example(./today-example.yaml))\"\nvalue: a\n",
    );
    let effective = DarkmatterSchemas::new()
        .effective_for(&md)
        .unwrap()
        .unwrap();
    let deps = effective.dependencies();
    assert!(
        deps.iter().any(|p| p.ends_with("types.yaml")),
        "import edge missing: {deps:?}",
    );
    assert!(
        deps.iter().any(|p| p.ends_with("today-example.yaml")),
        "example edge missing: {deps:?}",
    );
    // The union is sorted and deduplicated.
    let mut expected = deps.to_vec();
    expected.sort();
    expected.dedup();
    assert_eq!(deps, expected.as_slice());
}

#[test]
fn dependencies_surface_referenced_schema_file() {
    // A `$schema: ./schema.yaml` document depends on that file's content —
    // editing the referenced schema's own type must invalidate a warm cache,
    // so the resolved file surfaces on `EffectiveSchema::dependencies`.
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("schema.yaml"),
        "$schema:\n  title: 'string(required)'\n",
    )
    .expect("write schema");
    let md = prompt_with_source(dir.path(), "$schema: ./schema.yaml\ntitle: hi\n");
    let effective = DarkmatterSchemas::new()
        .effective_for(&md)
        .unwrap()
        .unwrap();
    let deps = effective.dependencies();
    assert!(
        deps.iter().any(|p| p.ends_with("schema.yaml")),
        "referenced schema file missing: {deps:?}",
    );
}

#[test]
fn dependencies_empty_without_imports_or_examples() {
    // The no-dependency fast path: a plain inline `$schema` records no edges.
    let md = md_with_schema("$schema:\n  title: 'string(required)'\ntitle: hi\n");
    let effective = DarkmatterSchemas::new()
        .effective_for(&md)
        .unwrap()
        .unwrap();
    assert!(effective.dependencies().is_empty());
}

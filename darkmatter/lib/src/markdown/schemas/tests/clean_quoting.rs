//! Schema-proven scalar quoting (acceptance matrix A-4, safety class S2).
//!
//! Covers the flagship `release: 1.20` → `release: "1.20"` repair and every
//! guard around it: the sole-string-mismatch requirement, the complete-schema
//! proof, the lexeme guards, unconstrained-key silence (D8), coercion
//! interplay, and each schema-resolution mode (default baseline, inline,
//! referenced, root union, custom/disabled baseline, triggers on/off).

use std::fs;

use biscuit_file::{YamlCertainty, YamlDiagnosticCode, apply_edit_set};
use tempfile::TempDir;

use super::super::*;
use crate::markdown::{Markdown, extract_frontmatter_block};

/// Analyzes `yaml_body` (raw frontmatter text, `$schema` included) against
/// the effective schema the document itself resolves.
fn analyze(yaml_body: &str) -> SchemaCleanAnalysis {
    let md = super::md_with_schema(yaml_body);
    let effective = DarkmatterSchemas::new()
        .effective_for(&md)
        .unwrap()
        .expect("document must resolve an effective schema");
    analyze_frontmatter(&effective, yaml_body)
}

/// Analyzes `yaml_body` against an explicit baseline config (no document
/// schema); `doc` supplies the parsed document and its source path context.
fn analyze_with(
    config: &CleanSchemaConfig,
    md: &Markdown,
    yaml_body: &str,
) -> SchemaCleanAnalysis {
    let context = config
        .resolve(md.source().as_ref().and_then(|source| match source {
            crate::markdown::compose::ComposeSource::File(path) => Some(path.as_path()),
            _ => None,
        }))
        .unwrap();
    context.analyze(md, yaml_body).unwrap()
}

/// The one deterministic diagnostic with its single repair, or a panic
/// describing the actual outcome.
fn proven_repair(analysis: &SchemaCleanAnalysis) -> &biscuit_file::YamlRepair {
    assert_eq!(
        analysis.diagnostics().len(),
        1,
        "expected exactly one diagnostic: {:?}",
        analysis.diagnostics()
    );
    let diagnostic = &analysis.diagnostics()[0];
    assert_eq!(diagnostic.code, YamlDiagnosticCode::SchemaTypeMismatch);
    assert_eq!(diagnostic.classification, YamlCertainty::Deterministic);
    assert_eq!(
        diagnostic.repairs.len(),
        1,
        "deterministic diagnostic must carry exactly one repair"
    );
    &diagnostic.repairs[0]
}

/// Every diagnostic is report-only and carries no repairs.
fn assert_all_report_only(analysis: &SchemaCleanAnalysis) {
    assert!(!analysis.is_clean(), "expected findings");
    for diagnostic in analysis.diagnostics() {
        assert_ne!(
            diagnostic.classification,
            YamlCertainty::Deterministic,
            "no finding may be auto-apply eligible: {:?}",
            diagnostic
        );
        assert!(
            diagnostic.repairs.is_empty(),
            "report-only findings never carry repairs: {:?}",
            diagnostic
        );
    }
}

#[test]
fn test_flagship_release_version_is_quoted() {
    // The original failing input, exactly as produced by agents.
    let analysis = analyze("$schema:\n  release: string\nrelease: 1.20\n");
    let repair = proven_repair(&analysis);
    assert_eq!(repair.replacement, "\"1.20\"");
}

#[test]
fn test_repair_span_covers_exact_lexeme() {
    let yaml = "$schema:\n  release: string\nrelease: 1.20\n";
    let analysis = analyze(yaml);
    let repair = proven_repair(&analysis);
    assert_eq!(&yaml[repair.span.clone()], "1.20");
}

#[test]
fn test_repair_application_passes_complete_schema() {
    let yaml = "$schema:\n  release: string\nrelease: 1.20\n";
    let md = super::md_with_schema(yaml);
    let effective = DarkmatterSchemas::new()
        .effective_for(&md)
        .unwrap()
        .unwrap();
    let analysis = analyze_frontmatter(&effective, yaml);
    let repair = proven_repair(&analysis);
    let outcome = apply_edit_set(yaml, std::slice::from_ref(repair));
    assert!(outcome.changed());
    // Candidate passes the complete effective schema.
    let instance: serde_json::Value = serde_yaml_ng::from_str(&outcome.source).unwrap();
    let mut instance = instance;
    instance.as_object_mut().unwrap().remove("$schema");
    assert!(effective.validate_raw(&instance).valid);
}

#[test]
fn test_native_vs_quoted_variants() {
    // Already-quoted form is clean.
    assert!(analyze("$schema:\n  release: string\nrelease: \"1.20\"\n").is_clean());
    assert!(analyze("$schema:\n  release: string\nrelease: '1.20'\n").is_clean());
    // Single-quoted replacement never appears: quoting is always the
    // escape-free double-quoted form.
    let analysis = analyze("$schema:\n  release: string\nrelease: 1.20\n");
    assert_eq!(proven_repair(&analysis).replacement, "\"1.20\"");
}

#[test]
fn test_boolean_scalar_variant() {
    let analysis = analyze("$schema:\n  draft: string\ndraft: true\n");
    assert_eq!(proven_repair(&analysis).replacement, "\"true\"");
}

#[test]
fn test_two_type_problems_block_auto_apply() {
    // `release` fails string-required and `count` fails integer-required:
    // the "exactly one problem" gate fails, so both are report-only.
    let analysis =
        analyze("$schema:\n  release: string\n  count: number(integer)\nrelease: 1.20\ncount: abc\n");
    assert_all_report_only(&analysis);
    assert_eq!(analysis.diagnostics().len(), 2);
    assert!(
        analysis
            .diagnostics()
            .iter()
            .all(|d| d.code == YamlDiagnosticCode::SchemaTypeMismatch)
    );
}

#[test]
fn test_candidate_must_pass_complete_schema() {
    // Quoting fixes the type but violates the pattern constraint, so the
    // complete-schema proof rejects the candidate.
    let analysis =
        analyze("$schema:\n  release: string(pattern(^[0-9]+\\.[0-9]+\\.[0-9]+$))\nrelease: 1.20\n");
    assert_all_report_only(&analysis);
}

#[test]
fn test_union_type_is_not_solely_string() {
    // `string | number` accepts the number natively in one arm — never the
    // S2 family.
    let analysis = analyze("$schema:\n  release:\n    - string\n    - number\nrelease: 1.20\n");
    assert!(analysis.is_clean());
}

#[test]
fn test_unconstrained_key_is_silently_skipped() {
    // The schema constrains only `title`; `release` is unconstrained (D8).
    let analysis = analyze("$schema:\n  title: string\nrelease: 1.20\n");
    assert!(analysis.is_clean());
}

#[test]
fn test_anchored_value_is_report_only() {
    // The authored text includes an anchor token; quoting it would corrupt.
    let analysis = analyze("$schema:\n  release: string\nrelease: &ver 1.20\n");
    assert_all_report_only(&analysis);
}

#[test]
fn test_trailing_comment_is_preserved_outside_repair() {
    let yaml = "$schema:\n  release: string\nrelease: 1.20 # pinned\n";
    let analysis = analyze(yaml);
    let repair = proven_repair(&analysis);
    assert_eq!(&yaml[repair.span.clone()], "1.20");
    let outcome = apply_edit_set(yaml, std::slice::from_ref(repair));
    assert!(outcome.source.contains("\"1.20\" # pinned"));
}

#[test]
fn test_block_sequence_entry_variant() {
    let analysis = analyze("$schema:\n  tags: string[]\ntags:\n  - alpha\n  - 42\n");
    let repair = proven_repair(&analysis);
    assert_eq!(repair.replacement, "\"42\"");
}

#[test]
fn test_flow_sequence_entry_is_report_only() {
    // Flow contexts are outside the v1 quoting grammar.
    let analysis = analyze("$schema:\n  tags: string[]\ntags: [alpha, 42]\n");
    assert_all_report_only(&analysis);
}

#[test]
fn test_nested_mapping_variant() {
    let analysis = analyze(
        "$schema:\n  site:\n    title: string\n    year: string\nsite:\n  title: Docs\n  year: 2026\n",
    );
    let repair = proven_repair(&analysis);
    assert_eq!(repair.replacement, "\"2026\"");
}

#[test]
fn test_crlf_source_variant() {
    let yaml = "$schema:\r\n  release: string\r\nrelease: 1.20\r\n";
    let analysis = analyze(yaml);
    let repair = proven_repair(&analysis);
    assert_eq!(&yaml[repair.span.clone()], "1.20");
    assert_eq!(repair.replacement, "\"1.20\"");
}

#[test]
fn test_multibyte_key_variant() {
    let analysis = analyze("$schema:\n  clé: string\nclé: 1.20\n");
    let repair = proven_repair(&analysis);
    assert_eq!(repair.replacement, "\"1.20\"");
}

#[test]
fn test_coercion_sensitive_string_is_silent() {
    // `"true"` against boolean is accepted by the shared coercing semantics;
    // clean does not second-guess compose.
    assert!(analyze("$schema:\n  enabled: boolean\nenabled: \"true\"\n").is_clean());
    // A numeric string against integer coerces cleanly as well.
    assert!(analyze("$schema:\n  count: number(integer)\ncount: \"42\"\n").is_clean());
}

#[test]
fn test_null_value_is_never_quoted() {
    // Null against an optional (nullable) string is valid; against a
    // required string it is a different repair class entirely.
    assert!(analyze("$schema:\n  release: string\nrelease: null\n").is_clean());
    let analysis = analyze("$schema:\n  release: string(required)\nrelease: null\n");
    assert_all_report_only(&analysis);
}

#[test]
fn test_default_baseline_keys_are_constrained() {
    // The real shipped baseline (`docs/schemas/darkmatter.yaml`) constrains
    // `title` to a string through the normal config-resolution path.
    let analysis = analyze_with(
        &CleanSchemaConfig::new(),
        &Markdown::from("---\ntitle: 42\n---\nbody\n"),
        "title: 42\n",
    );
    let repair = proven_repair(&analysis);
    assert_eq!(repair.replacement, "\"42\"");
}

#[test]
fn test_default_baseline_leaves_unknown_keys_unconstrained() {
    let analysis = analyze_with(
        &CleanSchemaConfig::new(),
        &Markdown::from("---\nrelease: 1.20\n---\nbody\n"),
        "release: 1.20\n",
    );
    assert!(analysis.is_clean());
}

#[test]
fn test_disabled_baseline_is_silent() {
    let analysis = analyze_with(
        &CleanSchemaConfig::new().without_baseline_schema(),
        &Markdown::from("---\ntitle: 42\n---\nbody\n"),
        "title: 42\n",
    );
    assert!(analysis.is_clean());
}

#[test]
fn test_custom_baseline_schema_object() {
    let schema_value = serde_yaml_ng::from_str("release: string").unwrap();
    let schema = parse_yaml_schema(&schema_value).unwrap();
    let config = CleanSchemaConfig::new()
        .without_baseline_schema()
        .with_baseline_schema(schema);
    let analysis = analyze_with(
        &config,
        &Markdown::from("---\nrelease: 1.20\n---\nbody\n"),
        "release: 1.20\n",
    );
    assert_eq!(proven_repair(&analysis).replacement, "\"1.20\"");
}

#[test]
fn test_custom_baseline_schema_file() {
    let dir = TempDir::new().unwrap();
    let baseline = dir.path().join("baseline.yaml");
    fs::write(&baseline, "$schema:\n  release: string\n").unwrap();
    let config = CleanSchemaConfig::new()
        .without_baseline_schema()
        .with_baseline_schema_file(&baseline);
    let analysis = analyze_with(
        &config,
        &Markdown::from("---\nrelease: 1.20\n---\nbody\n"),
        "release: 1.20\n",
    );
    assert_eq!(proven_repair(&analysis).replacement, "\"1.20\"");
}

#[test]
fn test_referenced_schema_file() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap();
    fs::write(dir.path().join("schema.yaml"), "$schema:\n  release: string\n").unwrap();
    let doc_path = dir.path().join("post.md");
    fs::write(&doc_path, "---\n$schema: schema.yaml\nrelease: 1.20\n---\nbody\n").unwrap();
    let md = Markdown::try_from(doc_path.as_path()).unwrap();
    let yaml = extract_frontmatter_block(&fs::read_to_string(&doc_path).unwrap())
        .unwrap()
        .unwrap()
        .yaml
        .to_string();
    let analysis = analyze_with(&CleanSchemaConfig::new(), &md, &yaml);
    assert_eq!(proven_repair(&analysis).replacement, "\"1.20\"");
}

#[test]
fn test_root_union_with_literal_discriminant() {
    let yaml = "$schema:\n  - kind: literal(a)\n    release: string\n  - kind: literal(b)\n    count: number(integer)\nkind: a\nrelease: 1.20\n";
    let analysis = analyze(yaml);
    assert_eq!(proven_repair(&analysis).replacement, "\"1.20\"");
}

/// Writes a repo-shaped fixture: `.git` marker, a `schemas/` trigger root,
/// and a document; returns (dir, doc_path).
fn trigger_fixture(dir: &TempDir, trigger: &str, payload: &str, doc: &str) -> std::path::PathBuf {
    fs::create_dir_all(dir.path().join(".git")).unwrap();
    fs::create_dir_all(dir.path().join("schemas")).unwrap();
    fs::write(dir.path().join("schemas").join("release.yaml"), trigger).unwrap();
    fs::write(dir.path().join("schemas").join("payload.yaml"), payload).unwrap();
    let doc_path = dir.path().join("post.md");
    fs::write(&doc_path, doc).unwrap();
    doc_path
}

const TRIGGER: &str = "kind: trigger-schema\nmatch:\n  layout: string(required)\n$schema: payload.yaml\n";
const PAYLOAD: &str = "$schema:\n  release: string\n";

#[test]
fn test_matching_trigger_constrains_document() {
    let dir = TempDir::new().unwrap();
    let doc_path = trigger_fixture(
        &dir,
        TRIGGER,
        PAYLOAD,
        "---\nlayout: post\nrelease: 1.20\n---\nbody\n",
    );
    let md = Markdown::try_from(doc_path.as_path()).unwrap();
    let analysis = analyze_with(&CleanSchemaConfig::new(), &md, "layout: post\nrelease: 1.20\n");
    assert_eq!(proven_repair(&analysis).replacement, "\"1.20\"");
}

#[test]
fn test_nonmatching_trigger_is_silent() {
    let dir = TempDir::new().unwrap();
    let doc_path = trigger_fixture(
        &dir,
        TRIGGER,
        PAYLOAD,
        "---\nrelease: 1.20\n---\nbody\n",
    );
    let md = Markdown::try_from(doc_path.as_path()).unwrap();
    let analysis = analyze_with(&CleanSchemaConfig::new(), &md, "release: 1.20\n");
    assert!(analysis.is_clean());
}

#[test]
fn test_disabled_triggers_are_silent() {
    let dir = TempDir::new().unwrap();
    let doc_path = trigger_fixture(
        &dir,
        TRIGGER,
        PAYLOAD,
        "---\nlayout: post\nrelease: 1.20\n---\nbody\n",
    );
    let md = Markdown::try_from(doc_path.as_path()).unwrap();
    let config = CleanSchemaConfig::new().with_trigger_schemas(false);
    let analysis = analyze_with(&config, &md, "layout: post\nrelease: 1.20\n");
    assert!(analysis.is_clean());
}

#[test]
fn test_schema_override_replaces_document_schema() {
    // The document's own `$schema` constrains nothing; the override does.
    let md = Markdown::from("---\n$schema:\n  title: string\nrelease: 1.20\n---\nbody\n");
    let override_value = serde_json::json!({ "release": "string" });
    let config = CleanSchemaConfig::new()
        .without_baseline_schema()
        .with_trigger_schemas(false)
        .with_schema_override(override_value);
    let analysis = analyze_with(&config, &md, "$schema:\n  title: string\nrelease: 1.20\n");
    assert_eq!(proven_repair(&analysis).replacement, "\"1.20\"");
}

#[test]
fn test_stdin_has_no_trigger_discovery() {
    // No document path: trigger discovery is silently inert (D7).
    let analysis = analyze_with(
        &CleanSchemaConfig::new(),
        &Markdown::from("---\ntitle: 42\n---\nbody\n"),
        "title: 42\n",
    );
    assert_eq!(proven_repair(&analysis).replacement, "\"42\"");
}

#[test]
fn test_diagnostics_are_deterministic_across_runs() {
    let yaml = "$schema:\n  release: string\n  count: number(integer)\nrelease: 1.20\ncount: abc\n";
    let first = analyze(yaml);
    let second = analyze(yaml);
    assert_eq!(first.diagnostics(), second.diagnostics());
}

#[test]
fn test_unparseable_source_yields_empty_analysis() {
    // The schema-agnostic layer owns unparseable input; the schema layer
    // defers entirely.
    let md = super::md_with_schema("$schema:\n  release: string\nrelease: ok\n");
    let effective = DarkmatterSchemas::new()
        .effective_for(&md)
        .unwrap()
        .unwrap();
    let analysis = analyze_frontmatter(&effective, "release: [unclosed\n");
    assert!(analysis.is_clean());
    assert!(analysis.effective().is_none());
}

#[test]
fn test_context_analysis_retains_effective_schema() {
    let context = CleanSchemaConfig::new().resolve(None).unwrap();
    let md = Markdown::from("---\ntitle: 42\n---\nbody\n");
    let analysis = context.analyze(&md, "title: 42\n").unwrap();
    assert!(analysis.effective().is_some());
    assert_eq!(proven_repair(&analysis).replacement, "\"42\"");
}

#[test]
fn test_no_effective_schema_means_no_analysis() {
    let context = CleanSchemaConfig::new()
        .without_baseline_schema()
        .with_trigger_schemas(false)
        .resolve(None)
        .unwrap();
    let md = Markdown::from("---\nrelease: 1.20\n---\nbody\n");
    let analysis = context.analyze(&md, "release: 1.20\n").unwrap();
    assert!(analysis.is_clean());
    assert!(analysis.effective().is_none());
}

#[test]
fn test_s1_schema_result_set_check() {
    // The ratified schema half of the parse-equivalence proof (D2-S1):
    // value-identical candidates yield identical result sets; a candidate
    // that changes the schema outcome fails the check.
    let md = super::md_with_schema("$schema:\n  title: string\ntitle: hi\n");
    let effective = DarkmatterSchemas::new()
        .effective_for(&md)
        .unwrap()
        .unwrap();
    let original = "$schema:\n  title: string\ntitle:   hi\n";
    let equivalent = "$schema:\n  title: string\ntitle: hi\n";
    assert!(schema_result_set_identical(&effective, original, equivalent));
    let regressed = "$schema:\n  title: string\ntitle: 42\n";
    assert!(!schema_result_set_identical(&effective, original, regressed));
}

#[test]
fn test_s1_check_does_not_block_s2_transition() {
    // The schema-quoting transition intentionally changes the result set
    // (one problem → zero); the S1 check must report that difference, not
    // gate it — the caller owns the classification-aware application.
    let md = super::md_with_schema("$schema:\n  release: string\nrelease: 1.20\n");
    let effective = DarkmatterSchemas::new()
        .effective_for(&md)
        .unwrap()
        .unwrap();
    let original = "$schema:\n  release: string\nrelease: 1.20\n";
    let quoted = "$schema:\n  release: string\nrelease: \"1.20\"\n";
    assert!(!schema_result_set_identical(&effective, original, quoted));
}

#[test]
fn test_s1_check_rejects_unparseable_candidate() {
    let md = super::md_with_schema("$schema:\n  title: string\ntitle: hi\n");
    let effective = DarkmatterSchemas::new()
        .effective_for(&md)
        .unwrap()
        .unwrap();
    assert!(!schema_result_set_identical(
        &effective,
        "$schema:\n  title: string\ntitle: hi\n",
        "title: [unclosed\n"
    ));
}

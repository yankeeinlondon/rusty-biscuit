//! Repository-root `schemas/` corpus + review-fixture contracts.
//!
//! The `meta_schema_phase4` corpus test walks only `darkmatter/docs/schemas`
//! and never sees the repo-root `schemas/` directory that ships
//! `feature-review.yaml` / `suggestion-review.yaml` — the schemas that this
//! repo's own review files declare via `$schema: feature-review.yaml`. These
//! tests pin that directory: every plain schema file must classify through the
//! standalone-schema recognizer, and a bare-name reference to
//! `feature-review.yaml` must resolve and actively validate review frontmatter.

use std::{fs, path::Path};

use darkmatter::markdown::{
    Markdown,
    compose::ComposeSource,
    schemas::{
        DarkmatterSchemas, StandaloneSchemaEnvelope, parse_standalone_schema_document,
        resolve::resolve_schema_with_roots,
    },
};
use serde_json::json;

/// Repo-root `schemas/` directory, relative to `darkmatter/lib`.
fn repo_schemas_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas")
}

/// A `.yaml` file in `schemas/` is a plain schema unless it carries a `kind`
/// naming a different envelope (e.g. `memory.yaml` is `kind: schema-trigger`,
/// discovered by placement, not by the standalone-schema recognizer). Files
/// with no `kind`, or `kind: schema`, are the plain schemas under test.
fn is_plain_schema(source: &str) -> bool {
    let Ok(value) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(source) else {
        return false;
    };
    let Some(map) = value.as_mapping() else {
        return false;
    };
    match map
        .get(serde_yaml_ng::Value::String("kind".into()))
        .and_then(serde_yaml_ng::Value::as_str)
    {
        Some(kind) => kind == "schema",
        None => true,
    }
}

#[test]
fn repo_root_schemas_all_classify_as_standalone_schemas() {
    let root = repo_schemas_dir();
    let mut paths = fs::read_dir(&root)
        .expect("repo-root schemas directory")
        .map(|entry| entry.expect("schemas entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("yaml"))
        .collect::<Vec<_>>();
    paths.sort();
    assert!(!paths.is_empty(), "repo-root schemas/ must contain .yaml files");

    let mut classified = 0usize;
    for path in &paths {
        let source = fs::read_to_string(path).expect("read schema artifact");
        if !is_plain_schema(&source) {
            // `memory.yaml` (kind: schema-trigger) is not a standalone schema.
            continue;
        }
        let document = parse_standalone_schema_document(&source, path)
            .unwrap_or_else(|error| panic!("{} must classify: {error}", path.display()))
            .unwrap_or_else(|| panic!("{} must be a standalone schema", path.display()));
        // Both shipped review schemas migrated to the pure `$schema` envelope.
        assert_eq!(
            document.envelope,
            StandaloneSchemaEnvelope::Pure,
            "{} must be a pure `$schema` envelope",
            path.display()
        );
        classified += 1;
    }
    assert!(classified >= 2, "expected both shipped review schemas, saw {classified}");
}

#[test]
fn feature_review_resolves_as_a_bare_name_reference() {
    let schemas_dir = repo_schemas_dir();
    // A document elsewhere on disk references the schema by bare name; the repo
    // `schemas/` directory is the schema root that resolves it. The file itself
    // is a root union, so resolution yields an `anyOf` JSON Schema.
    let doc_dir = tempfile::tempdir().expect("tempdir");
    let resolved = resolve_schema_with_roots(
        &json!("feature-review.yaml"),
        doc_dir.path(),
        std::slice::from_ref(&schemas_dir),
    )
    .expect("feature-review.yaml must resolve as a bare-name schema reference");
    assert!(
        resolved.json_schema.get("anyOf").is_some(),
        "the shipped root union must lower to anyOf: {}",
        resolved.json_schema
    );
    assert!(resolved.simplified.is_some(), "the union must project a SimplifiedSchema");
}

#[test]
fn feature_review_reference_validates_a_review_document() {
    // A review document sited in the repo `schemas/` directory references the
    // shipped schema by relative path; the real file is loaded through the full
    // document pipeline and drives frontmatter validation. The fictitious `.md`
    // path is never read — only its parent anchors `./feature-review.yaml`.
    let doc_path = repo_schemas_dir().join("__e2e_review_fixture.md");
    let api = DarkmatterSchemas::new();

    let valid = Markdown::from(concat!(
        "---\n",
        "$schema: ./feature-review.yaml\n",
        "description: A representative feature review.\n",
        "ready: true\n",
        "spec: spec.md\n",
        "created: \"2026-07-20T10:00:00-07:00\"\n",
        "agent: claude/opus\n",
        "feature: shipped-review-schema-migration\n",
        "implemented: false\n",
        "next: review-10.md\n",
        "previous: review-8.md\n",
        "---\nBody\n",
    ))
    .with_source(ComposeSource::File(doc_path.clone()));
    let report = api
        .validate(&valid)
        .expect("validation runs against the shipped feature-review schema");
    assert!(report.valid, "a well-formed review must validate: {:?}", report.problems);

    // `ready` is typed `boolean` in both union arms, so a numeric value fails
    // every arm — proof the loaded schema is active, not a permissive pass.
    let invalid = Markdown::from("---\n$schema: ./feature-review.yaml\nready: 42\n---\nBody\n")
        .with_source(ComposeSource::File(doc_path));
    let report = api.validate(&invalid).expect("validation runs");
    assert!(!report.valid, "a type-violating review must be rejected");
}

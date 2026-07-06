//! Drift guard for the committed model-catalog artifact.
//!
//! Rebuilds the catalog offline from the generated provider enums and asserts
//! it byte-equals `unchained-ai/artifacts/*.json`. Failure means someone
//! changed the catalog data, schema types, or curation tables without
//! re-emitting the artifact.

use std::path::PathBuf;

use unchained_ai_gen::catalog::{
    build_catalog, catalog_generated_at, schema_json, to_canonical_json, SCHEMA_VERSION,
};

const EXPECTED_SCHEMA_VERSION: u32 = 2;

fn artifact_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("artifacts")
        .join(name)
}

fn committed(name: &str) -> String {
    let path = artifact_path(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read committed artifact {}: {e}", path.display()))
}

#[test]
fn committed_catalog_matches_rebuild() {
    let generated_at = catalog_generated_at().expect("derive generated_at from provider files");
    let rebuilt = to_canonical_json(&build_catalog(generated_at));
    assert_eq!(
        committed("models-catalog.json"),
        rebuilt,
        "committed models-catalog.json is stale; re-run \
         `cargo run -p unchained-ai-gen --bin emit-catalog`"
    );
}

#[test]
fn committed_schema_matches_rebuild() {
    assert_eq!(
        committed("models-catalog.schema.json"),
        schema_json(),
        "committed models-catalog.schema.json is stale; re-run \
         `cargo run -p unchained-ai-gen --bin emit-catalog`"
    );
}

#[test]
fn build_catalog_emits_current_schema_version() {
    let catalog = build_catalog(catalog_generated_at().expect("derive generated_at"));
    assert_eq!(catalog.schema_version, EXPECTED_SCHEMA_VERSION);
    assert_eq!(SCHEMA_VERSION, EXPECTED_SCHEMA_VERSION);
}

#[test]
fn catalog_shape_sanity_floors() {
    let catalog = build_catalog(catalog_generated_at().expect("derive generated_at"));
    assert_eq!(catalog.schema_version, EXPECTED_SCHEMA_VERSION);
    // Loose empirical floors (662 offerings / 1 gap / 131 groups at first
    // emission) so routine regeneration does not flake.
    assert!(
        catalog.offerings.len() > 600,
        "expected > 600 offerings, got {}",
        catalog.offerings.len()
    );
    assert!(
        catalog.gaps.len() < 5,
        "expected < 5 gap ids, got {}: {:?}",
        catalog.gaps.len(),
        catalog.gaps
    );
    assert!(
        catalog.duplicate_groups.len() > 100,
        "expected > 100 duplicate groups, got {}",
        catalog.duplicate_groups.len()
    );
}

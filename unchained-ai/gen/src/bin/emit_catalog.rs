//! Offline emitter for the committed model-catalog artifact.
//!
//! Rebuilds `unchained-ai/artifacts/models-catalog.json` (plus its JSON
//! Schema) from the compiled-in generated catalog — no network, no API keys.
//! Refreshing the underlying data is `gen-models`' job.

use std::path::PathBuf;

use unchained_ai_gen::catalog::{
    build_catalog, candidate_variant_tokens, catalog_generated_at, schema_json, to_canonical_json,
};

/// Resolve `unchained-ai/artifacts/` relative to the gen crate's manifest,
/// not the caller's working directory.
fn artifacts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("artifacts")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generated_at = catalog_generated_at()?;
    let catalog = build_catalog(generated_at);

    let dir = artifacts_dir();
    std::fs::create_dir_all(&dir)?;
    let catalog_path = dir.join("models-catalog.json");
    let schema_path = dir.join("models-catalog.schema.json");
    std::fs::write(&catalog_path, to_canonical_json(&catalog))?;
    std::fs::write(&schema_path, schema_json())?;

    let total = catalog.offerings.len();
    let inferred = total - catalog.gaps.len();
    println!("=== Model Catalog Artifact ===");
    println!();
    println!("Wrote {}", catalog_path.display());
    println!("Wrote {}", schema_path.display());
    println!();
    println!("generated_at:     {}", catalog.generated_at);
    println!("offerings:        {}", total);
    println!(
        "family inferred:  {} ({:.1}%)",
        inferred,
        if total == 0 {
            0.0
        } else {
            inferred as f64 * 100.0 / total as f64
        }
    );
    println!("families:         {}", catalog.families.len());
    println!("duplicate groups: {}", catalog.duplicate_groups.len());
    if catalog.gaps.is_empty() {
        println!("gaps:             none");
    } else {
        println!("gaps:             {}", catalog.gaps.len());
        for id in &catalog.gaps {
            println!("  {}", id);
        }
    }

    let candidates = candidate_variant_tokens(&catalog);
    println!();
    println!("=== Candidate Variant Tokens (top 20) ===");
    println!();
    if candidates.is_empty() {
        println!("none");
    } else {
        for candidate in candidates.iter().take(20) {
            println!(
                "  {:<16} {} families",
                candidate.token, candidate.family_count
            );
        }
    }

    Ok(())
}

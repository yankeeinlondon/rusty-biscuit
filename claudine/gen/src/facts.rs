//! One-time facts-file bootstrap (the "facts scraper").
//!
//! Transcribes today's hand-written provider constants into
//! `docs/providers/facts/<slug>.yaml` by consuming the serialized
//! `ProviderInfo` catalog (`claudine providers --describe --format json`)
//! rather than parsing Rust source. Only facts-declared registry fields
//! are extracted. This subcommand exists for the bootstrap step of the
//! field-source matrix and is expected to be retired once every provider's
//! facts file is seeded.

use serde_json::Value;

use crate::errors::GenError;
use crate::registry::{DeclaredSource, REGISTRY};

/// Renders the facts YAML for `slug` from a serialized `ProviderInfo`
/// array (the `claudine providers --describe --format json` payload).
///
/// ## Errors
///
/// Fails when the payload is not an array, carries no entry whose `slug`
/// matches, or lacks a facts-declared field.
pub fn scrape_facts(providers_json: &Value, slug: &str) -> Result<String, GenError> {
    let providers = providers_json
        .as_array()
        .ok_or_else(|| GenError::Json {
            message: "expected a top-level ProviderInfo array".into(),
        })?;
    let provider = providers
        .iter()
        .find(|p| p.get("slug").and_then(Value::as_str) == Some(slug))
        .ok_or_else(|| GenError::Json {
            message: format!("no provider with slug `{slug}` in the payload"),
        })?;

    let mut out = String::new();
    out.push_str(&format!(
        "# Topic-less catalog facts for the {slug} provider.\n\
         #\n\
         # Field-source matrix (design/catalog-generation.md): this file owns values\n\
         # whose research topic has not landed yet. Keys are catalog field names.\n\
         # Seeded from the hand-written `CLAUDE_INFO` constants via\n\
         # `claudine providers --describe --format json | claudine-gen scrape-facts {slug}`.\n\
         # When a field's research topic lands, the mapping registry re-points the\n\
         # field and generation ERRORS while this file still carries the key\n\
         # (delete-on-graduate).\n"
    ));
    for entry in REGISTRY {
        let DeclaredSource::Facts { key } = entry.source else {
            continue;
        };
        let value = provider.get(key).ok_or_else(|| GenError::Json {
            message: format!("provider `{slug}` payload has no `{key}` field"),
        })?;
        out.push_str(&format!("{key}: {}\n", yaml_scalar(value)?));
    }
    Ok(out)
}

/// Minimal deterministic YAML scalar rendering for the fact shapes the
/// registry consumes (bool, string, string array).
fn yaml_scalar(value: &Value) -> Result<String, GenError> {
    match value {
        Value::Bool(b) => Ok(b.to_string()),
        Value::String(s) => Ok(format!("{s:?}")),
        Value::Array(items) => {
            let rendered: Result<Vec<String>, GenError> = items.iter().map(yaml_scalar).collect();
            Ok(format!("[{}]", rendered?.join(", ")))
        }
        other => Err(GenError::Json {
            message: format!("unsupported fact value shape: `{other}`"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn scrape_extracts_facts_declared_fields_only() {
        let payload = json!([
            { "slug": "claude", "supports_skills": true, "binary": "claude" }
        ]);
        let yaml = scrape_facts(&payload, "claude").unwrap();
        assert!(yaml.contains("supports_skills: true"));
        // binary is roster-declared, never scraped into facts.
        assert!(!yaml.contains("binary"));
    }

    #[test]
    fn scrape_rejects_unknown_slug() {
        let payload = json!([{ "slug": "codex", "supports_skills": true }]);
        assert!(matches!(
            scrape_facts(&payload, "claude"),
            Err(GenError::Json { .. })
        ));
    }
}

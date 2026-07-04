//! One-time facts-file bootstrap (the "facts scraper").
//!
//! Transcribes today's hand-written provider constants into
//! `docs/providers/facts/<slug>.yaml` by consuming the serialized
//! `ProviderInfo` catalog (`claudine providers --describe --format json`)
//! rather than parsing Rust source. This subcommand exists for the
//! bootstrap step of the field-source matrix and is expected to be
//! retired once every provider's facts file is seeded.
//!
//! Two scopes, per the Phase B field-source matrix
//! (`features/2026-07-02-provider-metadata/field-source-matrix.md`):
//!
//! - `claude` scrapes only facts-declared [`REGISTRY`] fields — the A1
//!   collision gate (`UnknownFactsKey`) rejects any other key in
//!   `claude.yaml` until generator v1 extends the registry.
//! - every other provider scrapes the full [`MATRIX_FACTS_FIELDS`] set;
//!   those files are not consumed by any gate yet.

use serde_json::Value;

use crate::errors::GenError;
use crate::registry::{DeclaredSource, REGISTRY};

/// Every facts-declared matrix field that exists in today's serialized
/// `ProviderInfo` payload, in payload field order (which follows the
/// hand-written constant's field order).
///
/// Source of truth: the field-source matrix's `facts` rows. Table-A-only
/// facts fields (no current constant, e.g. `platform_kind`) are absent by
/// construction — there is nothing to scrape.
pub const MATRIX_FACTS_FIELDS: &[&str] = &[
    "supports_skills",
    "stream_protocol",
    "event_mapping",
    "resource_support",
    "session_locations",
    "memory_files",
    "output_formats",
    "entrypoints",
    "system_prompt",
    "yolo",
    "reasoning",
    "known_gaps",
    "acp",
    "prompt_arg_conventions",
    "cli_sensitive_axes",
    "repo_home_root_files",
];

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

    let keys: Vec<&str> = if slug == "claude" {
        REGISTRY
            .iter()
            .filter_map(|entry| match entry.source {
                DeclaredSource::Facts { key } => Some(key),
                _ => None,
            })
            .collect()
    } else {
        MATRIX_FACTS_FIELDS.to_vec()
    };

    // The seed-constant wording stays byte-identical for claude (its
    // committed facts file predates the matrix).
    let seed = if slug == "claude" {
        "the hand-written `CLAUDE_INFO` constants"
    } else {
        "the hand-written provider constants"
    };
    let mut out = format!(
        "# Topic-less catalog facts for the {slug} provider.\n\
         #\n\
         # Field-source matrix (design/catalog-generation.md): this file owns values\n\
         # whose research topic has not landed yet. Keys are catalog field names.\n\
         # Seeded from {seed} via\n\
         # `claudine providers --describe --format json | claudine-gen scrape-facts {slug}`.\n\
         # When a field's research topic lands, the mapping registry re-points the\n\
         # field and generation ERRORS while this file still carries the key\n\
         # (delete-on-graduate).\n"
    );

    let mut mapping = serde_yaml_ng::Mapping::with_capacity(keys.len());
    for key in keys {
        let value = provider.get(key).ok_or_else(|| GenError::Json {
            message: format!("provider `{slug}` payload has no `{key}` field"),
        })?;
        let yaml: serde_yaml_ng::Value =
            serde_yaml_ng::to_value(value).map_err(|err| GenError::Json {
                message: format!("fact `{key}` is not YAML-representable: {err}"),
            })?;
        mapping.insert(serde_yaml_ng::Value::String(key.to_string()), yaml);
    }
    let body = serde_yaml_ng::to_string(&mapping).map_err(|err| GenError::Json {
        message: format!("facts mapping is not YAML-serializable: {err}"),
    })?;
    out.push_str(&body);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    use crate::registry::entry_for;

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

    /// The claude facts file predates the matrix and is gate-constrained;
    /// its scrape output (header wording included) must stay byte-stable.
    #[test]
    fn claude_scrape_output_is_byte_stable() {
        let payload = json!([{ "slug": "claude", "supports_skills": true }]);
        let yaml = scrape_facts(&payload, "claude").unwrap();
        assert_eq!(
            yaml,
            "# Topic-less catalog facts for the claude provider.\n\
             #\n\
             # Field-source matrix (design/catalog-generation.md): this file owns values\n\
             # whose research topic has not landed yet. Keys are catalog field names.\n\
             # Seeded from the hand-written `CLAUDE_INFO` constants via\n\
             # `claudine providers --describe --format json | claudine-gen scrape-facts claude`.\n\
             # When a field's research topic lands, the mapping registry re-points the\n\
             # field and generation ERRORS while this file still carries the key\n\
             # (delete-on-graduate).\n\
             supports_skills: true\n"
        );
    }

    fn full_provider(slug: &str) -> Value {
        json!({
            "slug": slug,
            "binary": "codex",
            "supports_skills": true,
            "stream_protocol": null,
            "event_mapping": { "mappings": [
                { "event": "session_start", "support_level": "not_supported",
                  "parse_aliases": [], "registration_target": false }
            ] },
            "resource_support": { "provider": slug },
            "session_locations": [ { "raw": "~/.codex/log", "segments": [] } ],
            "memory_files": [],
            "output_formats": [],
            "entrypoints": [],
            "system_prompt": { "append": { "interactive": "unsupported" } },
            "yolo": "none",
            "reasoning": { "named_levels": { "flag": "effort", "levels": ["low"] } },
            "known_gaps": [],
            "acp": { "server_mode": "not_supported" },
            "prompt_arg_conventions": { "prompt_flags": [] },
            "cli_sensitive_axes": { "read_path": true },
            "repo_home_root_files": [],
        })
    }

    #[test]
    fn non_claude_scrape_emits_every_matrix_facts_field() {
        let payload = json!([full_provider("codex")]);
        let yaml = scrape_facts(&payload, "codex").unwrap();
        for key in MATRIX_FACTS_FIELDS {
            assert!(
                yaml.contains(&format!("{key}:")),
                "missing matrix facts field `{key}` in:\n{yaml}"
            );
        }
        // Roster-declared identity never leaks into facts.
        assert!(!yaml.contains("binary:"));
        // Null and nested shapes render as YAML, not error.
        assert!(yaml.contains("stream_protocol: null"));
        assert!(yaml.contains("raw: ~/.codex/log"));
    }

    #[test]
    fn non_claude_scrape_fails_on_missing_matrix_field() {
        let mut provider = full_provider("codex");
        provider.as_object_mut().unwrap().remove("yolo");
        let err = scrape_facts(&json!([provider]), "codex").unwrap_err();
        assert!(matches!(err, GenError::Json { .. }));
    }

    /// Guards matrix↔registry consistency: a matrix facts field must not be
    /// declared roster- or research-sourced in the registry.
    #[test]
    fn matrix_facts_fields_never_shadow_non_facts_registry_entries() {
        for key in MATRIX_FACTS_FIELDS {
            if let Some(entry) = entry_for(key) {
                assert!(
                    matches!(entry.source, DeclaredSource::Facts { .. }),
                    "`{key}` is registry-declared {} but matrix-declared facts",
                    entry.source.kind()
                );
            }
        }
    }
}

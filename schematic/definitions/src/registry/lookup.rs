//! Registry lookup and dispatch functions.
//!
//! This module provides the central lookup table that maps API names to their
//! schema registries, plus helpers for module-grouped export.

use super::SchemaRegistry;

/// Returns the OpenAPI schema registry for the specified API.
///
/// Central dispatch from a kebab-case API key to that API's
/// [`SchemaRegistry`]. Every API defined in this crate is wired up here; the
/// accepted keys are exactly the arms of the match below (`"anthropic"`,
/// `"bitbucket"`, `"elevenlabs"`, `"artificial-analysis-data"`,
/// `"artificial-analysis-critpt"`, `"emqx-basic"`, `"emqx-bearer"`,
/// `"eversolo"`, `"gitea"`, `"github"`, `"gitlab"`, `"huggingface"`,
/// `"lmstudio"`, `"ollama-native"`, `"ollama-openai"`, `"openai"`,
/// `"samsung-smart-tv"`, and `"unfolded-circle-core-rest"`).
///
/// The lookup is case-insensitive: `api_name` is lowercased before matching.
/// Use [`registry_key_for`] to translate a `RestApi::name` (e.g.
/// `"OllamaNative"`) into the key this function expects.
///
/// ## Returns
///
/// `Some(SchemaRegistry)` for a known API key; `None` for any unrecognized key.
///
/// ## Examples
///
/// ```
/// use schematic_definitions::registry::get_registry;
///
/// let registry = get_registry("openai");
/// assert!(registry.is_some());
///
/// let unknown = get_registry("unknown-api");
/// assert!(unknown.is_none());
/// ```
#[must_use]
pub fn get_registry(api_name: &str) -> Option<SchemaRegistry> {
    match api_name.to_lowercase().as_str() {
        "anthropic" => Some(crate::anthropic::openapi_registry()),
        "bitbucket" => Some(crate::bitbucket::openapi_registry()),
        "elevenlabs" => Some(crate::elevenlabs::openapi_registry()),
        "artificial-analysis-data" | "artificial-analysis-critpt" => {
            Some(crate::artificial_analysis::openapi_registry())
        }
        "emqx-basic" | "emqx-bearer" => Some(crate::emqx::openapi_registry()),
        "eversolo" => Some(crate::eversolo::openapi_registry()),
        "gitea" => Some(crate::gitea::openapi_registry()),
        "github" => Some(crate::github::openapi_registry()),
        "gitlab" => Some(crate::gitlab::openapi_registry()),
        "huggingface" => Some(crate::huggingface::openapi_registry()),
        "lmstudio" => Some(crate::lmstudio::openapi_registry()),
        "ollama-native" | "ollama-openai" => Some(crate::ollama::openapi_registry()),
        "openai" => Some(crate::openai::openapi_registry()),
        "samsung-smart-tv" => Some(crate::samsung_smart_tv::openapi_registry()),
        "unfolded-circle-core-rest" => Some(crate::unfolded_circle::core_rest::openapi_registry()),
        _ => None,
    }
}

/// Returns the canonical registry-lookup key for a given API name.
///
/// `RestApi::name` uses the API's struct-style name (e.g. `"OllamaNative"`),
/// but [`get_registry`] is keyed by kebab-cased identifiers
/// (e.g. `"ollama-native"`). This function bridges the two so callers do not
/// need to maintain their own translation table.
///
/// ## Returns
///
/// The kebab-case key suitable for [`get_registry`] when `api_name` is a
/// known API; otherwise returns the lowercased input as a best-effort
/// fallback.
///
/// ## Examples
///
/// ```
/// use schematic_definitions::registry::{get_registry, registry_key_for};
///
/// assert_eq!(registry_key_for("OllamaNative"), "ollama-native");
/// assert_eq!(registry_key_for("HuggingFaceHub"), "huggingface");
/// assert!(get_registry(&registry_key_for("OpenAI")).is_some());
/// ```
#[must_use]
pub fn registry_key_for(api_name: &str) -> String {
    match api_name {
        "Anthropic" => "anthropic".to_string(),
        "ArtificialAnalysisData" => "artificial-analysis-data".to_string(),
        "ArtificialAnalysisCritPt" => "artificial-analysis-critpt".to_string(),
        "Bitbucket" => "bitbucket".to_string(),
        "OpenAI" => "openai".to_string(),
        "ElevenLabs" => "elevenlabs".to_string(),
        "Gitea" => "gitea".to_string(),
        "GitHub" => "github".to_string(),
        "GitLab" => "gitlab".to_string(),
        "HuggingFaceHub" => "huggingface".to_string(),
        "LmStudio" => "lmstudio".to_string(),
        "OllamaNative" => "ollama-native".to_string(),
        "OllamaOpenAI" => "ollama-openai".to_string(),
        "EmqxBasic" => "emqx-basic".to_string(),
        "EmqxBearer" => "emqx-bearer".to_string(),
        "Eversolo" => "eversolo".to_string(),
        "SamsungSmartTv" => "samsung-smart-tv".to_string(),
        "UnfoldedCircleCoreRest" => "unfolded-circle-core-rest".to_string(),
        other => other.to_lowercase(),
    }
}

/// Returns the merged OpenAPI schema registry for every API in a module.
///
/// Module-grouped OpenAPI export emits a single document per resolved module
/// path (e.g. `ollama.json` covers both `OllamaNative` and `OllamaOpenAI`).
/// This helper looks up each member API's per-API registry and merges them
/// into one [`SchemaRegistry`] suitable for that grouped export.
///
/// The merge uses `IndexMap` insertion semantics: identical schemas
/// registered under the same name collapse to a single entry, while distinct
/// schemas are unioned in insertion order.
///
/// ## Returns
///
/// `Some(SchemaRegistry)` containing the union of every member API's
/// registry when the module is known **and** at least one member API has a
/// registered schema registry. `None` is returned only when:
///
/// - the module name is unknown to [`crate::apis_by_module`], **or**
/// - every member API in the module has no registry available via
///   [`get_registry`].
///
/// ## Examples
///
/// ```
/// use schematic_definitions::registry::get_registries_for_module;
///
/// let ollama = get_registries_for_module("ollama").expect("ollama module");
/// assert!(!ollama.is_empty());
///
/// assert!(get_registries_for_module("unknown-module").is_none());
/// ```
#[must_use]
pub fn get_registries_for_module(module_name: &str) -> Option<SchemaRegistry> {
    let normalized = module_name.to_lowercase();
    let grouped = crate::apis_by_module();

    // Find the matching module bucket case-insensitively.
    let module_apis = grouped
        .iter()
        .find(|(name, _)| name.to_lowercase() == normalized)
        .map(|(_, apis)| apis)?;

    let mut merged = SchemaRegistry::new();
    let mut found_any = false;

    for api in module_apis {
        let key = registry_key_for(&api.name);
        if let Some(reg) = get_registry(&key) {
            merged.extend(reg);
            found_any = true;
        }
    }

    if found_any { Some(merged) } else { None }
}

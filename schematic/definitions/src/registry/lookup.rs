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
    let key = api_name.to_lowercase();
    let module = match key.as_str() {
        "artificial-analysis-data" | "artificial-analysis-critpt" => "artificial_analysis",
        "emqx-basic" | "emqx-bearer" => "emqx",
        "ollama-native" | "ollama-openai" => "ollama",
        "samsung-smart-tv" => "samsung_smart_tv",
        "unfolded-circle-core-rest" => "unfolded_circle_core_rest",
        other => other,
    };
    get_module_group(module).map(|group| group.registry)
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
/// The module is assembled once, yielding the registry from the same context
/// that validated all of its APIs.
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
    get_module_group(module_name).map(|group| group.registry)
}

/// Assemble the definition and registry together for a known generated module.
pub fn get_module_group(module: &str) -> Option<super::ModuleGroup> {
    let key = module.to_lowercase();
    MODULES.iter().find(|(name, _)| *name == key)
        .map(|(_, assemble)| assemble().expect("valid shipped module group"))
}

type Assemble = fn() -> Result<super::ModuleGroup, schematic_define::DefinitionError>;

pub(crate) const MODULES: &[(&str, Assemble)] = &[
    ("anthropic", crate::anthropic::module_group),
    ("artificial_analysis", crate::artificial_analysis::module_group),
    ("bitbucket", crate::bitbucket::module_group),
    ("openai", crate::openai::module_group),
    ("elevenlabs", crate::elevenlabs::module_group),
    ("gitea", crate::gitea::module_group),
    ("github", crate::github::module_group),
    ("gitlab", crate::gitlab::module_group),
    ("huggingface", crate::huggingface::module_group),
    ("lmstudio", crate::lmstudio::module_group),
    ("ollama", crate::ollama::module_group),
    ("emqx", crate::emqx::module_group),
    ("eversolo", crate::eversolo::module_group),
    ("samsung_smart_tv", crate::samsung_smart_tv::module_group),
    ("unfolded_circle_core_rest", crate::unfolded_circle::core_rest::module_group),
];

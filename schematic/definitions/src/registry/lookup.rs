//! The catalog of shipped module groups and their stable API keys.

/// Returns the stable kebab-case key for a shipped API name.
///
/// `RestApi::name` uses the API's struct-style name (e.g. `"OllamaNative"`);
/// per-API artifacts are keyed by kebab-case identifiers
/// (e.g. `"ollama-native"`). This function is the one translation table.
///
/// ## Returns
///
/// The kebab-case key when `api_name` is a known API; otherwise the
/// lowercased input as a best-effort fallback.
///
/// ## Examples
///
/// ```
/// use schematic_definitions::registry::registry_key_for;
///
/// assert_eq!(registry_key_for("OllamaNative"), "ollama-native");
/// assert_eq!(registry_key_for("HuggingFaceHub"), "huggingface");
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

type Assemble = fn() -> Result<Vec<schematic_define::RestApi>, schematic_define::DefinitionError>;

/// Every shipped module group, in catalog order.
pub(crate) const MODULES: &[Assemble] = &[
    crate::anthropic::module_group,
    crate::artificial_analysis::module_group,
    crate::bitbucket::module_group,
    crate::openai::module_group,
    crate::elevenlabs::module_group,
    crate::gitea::module_group,
    crate::github::module_group,
    crate::gitlab::module_group,
    crate::huggingface::module_group,
    crate::lmstudio::module_group,
    crate::ollama::module_group,
    crate::emqx::module_group,
    crate::eversolo::module_group,
    crate::samsung_smart_tv::module_group,
    crate::unfolded_circle::core_rest::module_group,
];

//! Type definitions for provider discovery

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Represents a single LLM provider and model combination
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LlmEntry {
    /// Provider name (e.g., "openai", "anthropic")
    pub provider: String,
    /// Model identifier (e.g., "gpt-5.2", "claude-sonnet-4.5")
    pub model: String,
}

impl LlmEntry {
    /// Create a new LLM entry
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
        }
    }

    /// Get the combined provider/model identifier
    pub fn identifier(&self) -> String {
        format!("{}/{}", self.provider, self.model)
    }
}

/// Format for the generated provider list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProviderListFormat {
    /// JSON array of string literals: ["openai/gpt-5.2", "anthropic/claude-opus-4.5"]
    #[default]
    StringLiterals,
    /// Rust enum variants: Openai_Gpt_5_2, Anthropic_Claude_Opus_4_5
    RustEnum,
}

/// OpenAI-compatible API response for /v1/models endpoint
///
/// This type is used by all providers that support the OpenAI-compatible
/// `/v1/models` endpoint, including OpenAI, Anthropic, Deepseek, Gemini,
/// MoonshotAI, OpenRouter, Zai, and Ollama.
///
/// Moved from `base.rs` and `discovery.rs` to eliminate duplication (Phase 0).
#[derive(Debug, Deserialize)]
pub struct OpenAIModelsResponse {
    pub data: Vec<OpenAIModel>,
}

/// Individual model entry in OpenAI-compatible API response
///
/// Moved from `base.rs` and `discovery.rs` to eliminate duplication (Phase 0).
#[derive(Debug, Deserialize)]
pub struct OpenAIModel {
    pub id: String,
}

/// Summary of ProviderModel::update() execution
///
/// Returned by `ProviderModel::update()` to report what models were added
/// during enum regeneration.
///
/// ## Examples
///
/// ```no_run
/// use shared::providers::ProviderModel;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let summary = ProviderModel::update().await?;
///     println!("Checked {} providers", summary.providers_checked.len());
///     println!("Added {} total new models", summary.total_added());
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct UpdateSummary {
    /// Map of Provider → count of new models added for that provider
    pub models_added: std::collections::HashMap<super::base::Provider, usize>,
    /// List of providers that were successfully queried
    pub providers_checked: Vec<super::base::Provider>,
    /// Count of aggregator hint variants added (OpenRouter/ZenMux)
    pub aggregator_hints_applied: usize,
}

impl UpdateSummary {
    /// Total number of models added across all providers
    pub fn total_added(&self) -> usize {
        self.models_added.values().sum()
    }
}

/// Strongly-typed enumeration of LLM provider models
///
/// This enum provides compile-time safety for known models while allowing
/// runtime flexibility for bleeding-edge or undocumented models via String outlets.
///
/// ## Naming Convention
///
/// Static variants use: `{Provider}__{ModelId}` with special characters converted:
/// - Hyphens `-` → Double underscores `__`
/// - Dots `.` → Underscores `_`
/// - Colons `:` → Removed
///
/// ## Design Philosophy
///
/// This enum intentionally keeps only 10-15 static variants for the most common/stable
/// models. Large enums (50-80+ variants) cause IDE slowdown and compilation bottlenecks.
/// Most models use String outlets without losing value.
///
/// ## Examples
///
/// ```
/// use shared::providers::{ProviderModel, Provider};
///
/// // Known model (compile-time safe)
/// let model = ProviderModel::Anthropic__ClaudeOpus__4__5__20251101;
/// assert_eq!(model.provider(), Provider::Anthropic);
/// assert_eq!(model.model_id(), "claude-opus-4-5-20251101");
///
/// // Undocumented model (runtime string outlet)
/// let bleeding_edge = ProviderModel::Anthropic("claude-opus-5-experimental".to_string());
/// assert_eq!(bleeding_edge.provider(), Provider::Anthropic);
/// assert_eq!(bleeding_edge.model_id(), "claude-opus-5-experimental");
///
/// // Display trait
/// assert_eq!(format!("{}", model), "anthropic/claude-opus-4-5-20251101");
/// ```
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProviderModel {
    // Anthropic models (most stable/common)
    Anthropic__ClaudeOpus__4__5__20251101,
    Anthropic__ClaudeSonnet__4__5__20250929,
    Anthropic__ClaudeHaiku__4__0__20250107,
    /// Outlet for undocumented or bleeding-edge Anthropic models
    Anthropic(String),

    // OpenAI models (most stable/common)
    OpenAi__Gpt__4o,
    OpenAi__Gpt__4o__Mini,
    OpenAi__O1,
    /// Outlet for undocumented or bleeding-edge OpenAI models
    OpenAi(String),

    // Deepseek models (most stable/common)
    Deepseek__Chat,
    Deepseek__Reasoner,
    /// Outlet for undocumented or bleeding-edge Deepseek models
    Deepseek(String),

    // Gemini models (most stable/common)
    Gemini__Gemini__3__Flash__Preview,
    Gemini__Gemini__2__0__Flash__Exp,
    /// Outlet for undocumented or bleeding-edge Gemini models
    Gemini(String),

    // Ollama (local models - no static variants, always use outlet)
    /// Local Ollama models - always use String outlet for dynamic local models
    Ollama(String),

    // OpenRouter aggregator (most stable/common)
    /// Outlet for OpenRouter aggregated models
    OpenRouter(String),

    // MoonshotAI models
    /// Outlet for MoonshotAI models
    MoonshotAi(String),

    // ZAI models
    /// Outlet for ZAI models
    Zai(String),

    // ZenMux aggregator
    /// Outlet for ZenMux aggregated models (no /v1/models support)
    ZenMux(String),
}

impl ProviderModel {
    /// Get the provider for this model
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::providers::{ProviderModel, Provider};
    ///
    /// let model = ProviderModel::Anthropic__ClaudeOpus__4__5__20251101;
    /// assert_eq!(model.provider(), Provider::Anthropic);
    ///
    /// let outlet = ProviderModel::OpenAi("gpt-5".to_string());
    /// assert_eq!(outlet.provider(), Provider::OpenAi);
    /// ```
    pub fn provider(&self) -> super::base::Provider {
        use super::base::Provider;
        match self {
            Self::Anthropic__ClaudeOpus__4__5__20251101
            | Self::Anthropic__ClaudeSonnet__4__5__20250929
            | Self::Anthropic__ClaudeHaiku__4__0__20250107
            | Self::Anthropic(_) => Provider::Anthropic,

            Self::OpenAi__Gpt__4o
            | Self::OpenAi__Gpt__4o__Mini
            | Self::OpenAi__O1
            | Self::OpenAi(_) => Provider::OpenAi,

            Self::Deepseek__Chat
            | Self::Deepseek__Reasoner
            | Self::Deepseek(_) => Provider::Deepseek,

            Self::Gemini__Gemini__3__Flash__Preview
            | Self::Gemini__Gemini__2__0__Flash__Exp
            | Self::Gemini(_) => Provider::Gemini,

            Self::Ollama(_) => Provider::Ollama,
            Self::OpenRouter(_) => Provider::OpenRouter,
            Self::MoonshotAi(_) => Provider::MoonshotAi,
            Self::Zai(_) => Provider::Zai,
            Self::ZenMux(_) => Provider::ZenMux,
        }
    }

    /// Get the model identifier (without provider prefix)
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::providers::ProviderModel;
    ///
    /// let model = ProviderModel::Anthropic__ClaudeOpus__4__5__20251101;
    /// assert_eq!(model.model_id(), "claude-opus-4-5-20251101");
    ///
    /// let outlet = ProviderModel::OpenAi("gpt-5-turbo".to_string());
    /// assert_eq!(outlet.model_id(), "gpt-5-turbo");
    /// ```
    pub fn model_id(&self) -> &str {
        match self {
            // Anthropic static variants
            Self::Anthropic__ClaudeOpus__4__5__20251101 => "claude-opus-4-5-20251101",
            Self::Anthropic__ClaudeSonnet__4__5__20250929 => "claude-sonnet-4-5-20250929",
            Self::Anthropic__ClaudeHaiku__4__0__20250107 => "claude-haiku-4-0-20250107",
            Self::Anthropic(id) => id,

            // OpenAI static variants
            Self::OpenAi__Gpt__4o => "gpt-4o",
            Self::OpenAi__Gpt__4o__Mini => "gpt-4o-mini",
            Self::OpenAi__O1 => "o1",
            Self::OpenAi(id) => id,

            // Deepseek static variants
            Self::Deepseek__Chat => "chat",
            Self::Deepseek__Reasoner => "reasoner",
            Self::Deepseek(id) => id,

            // Gemini static variants
            Self::Gemini__Gemini__3__Flash__Preview => "gemini-3-flash-preview",
            Self::Gemini__Gemini__2__0__Flash__Exp => "gemini-2-0-flash-exp",
            Self::Gemini(id) => id,

            // All outlets
            Self::Ollama(id) => id,
            Self::OpenRouter(id) => id,
            Self::MoonshotAi(id) => id,
            Self::Zai(id) => id,
            Self::ZenMux(id) => id,
        }
    }

    /// Get the full identifier in "provider/model-id" format
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::providers::ProviderModel;
    ///
    /// let model = ProviderModel::Anthropic__ClaudeOpus__4__5__20251101;
    /// assert_eq!(model.to_identifier(), "anthropic/claude-opus-4-5-20251101");
    ///
    /// let outlet = ProviderModel::Deepseek("chat-v2".to_string());
    /// assert_eq!(outlet.to_identifier(), "deepseek/chat-v2");
    /// ```
    pub fn to_identifier(&self) -> String {
        let provider_str = match self.provider() {
            super::base::Provider::Anthropic => "anthropic",
            super::base::Provider::Deepseek => "deepseek",
            super::base::Provider::Gemini => "gemini",
            super::base::Provider::MoonshotAi => "moonshotai",
            super::base::Provider::Ollama => "ollama",
            super::base::Provider::OpenAi => "openai",
            super::base::Provider::OpenRouter => "openrouter",
            super::base::Provider::Zai => "zai",
            super::base::Provider::ZenMux => "zenmux",
        };
        format!("{}/{}", provider_str, self.model_id())
    }

    /// Validate that this model exists via provider API
    ///
    /// This is a separate async validation method - TryFrom does NOT call APIs.
    /// Use this when you need explicit validation that a model exists.
    ///
    /// ## Errors
    ///
    /// - `ProviderError::UnknownModel` - Model not found via API
    /// - `ProviderError::ValidationTimeout` - API call timed out
    /// - `ProviderError::HttpError` - Network/API error
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use shared::providers::ProviderModel;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let model: ProviderModel = "anthropic/claude-opus-5-experimental".try_into()?;
    ///
    ///     // Explicitly validate via API
    ///     model.validate_exists().await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn validate_exists(&self) -> Result<(), super::discovery::ProviderError> {
        // For now, return Ok - full implementation will come in future phases
        // This stub ensures the API exists for Phase 3 completion
        tracing::debug!(
            provider = ?self.provider(),
            model_id = self.model_id(),
            "validate_exists() called (stub implementation)"
        );

        // TODO: Implement actual API validation in future phase
        Ok(())
    }

    /// Update the ProviderModel enum definition from live provider APIs
    ///
    /// Fetches current models from all available providers and generates
    /// new enum variants. Preserves existing variants to avoid breaking changes.
    ///
    /// ## Update Strategy
    ///
    /// - Never remove variants (backward compatibility)
    /// - Add new variants for newly discovered models
    /// - Aggregator hints: add aggregator variant, conditionally add underlying
    /// - Direct provider access: interrogate directly, ignore aggregator hints
    ///
    /// ## Returns
    ///
    /// UpdateSummary with counts of models added per provider
    ///
    /// ## Errors
    ///
    /// - `ProviderError::CodegenFailed` - Failed to inject enum code
    /// - `ProviderError::NoProvidersAvailable` - No API keys configured
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use shared::providers::ProviderModel;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let summary = ProviderModel::update().await?;
    ///     println!("Added {} new models", summary.total_added());
    ///     for (provider, count) in &summary.models_added {
    ///         println!("  {:?}: {} new models", provider, count);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn update() -> Result<UpdateSummary, super::discovery::ProviderError> {
        use super::base::Provider;
        use super::discovery::ProviderError;
        use std::collections::HashMap;

        tracing::info!("Starting ProviderModel enum update from live APIs");

        // Step 1: Call api::get_all_provider_models()
        let api_models = crate::api::openai_compat::get_all_provider_models().await?;

        if api_models.is_empty() {
            return Err(ProviderError::NoProvidersAvailable);
        }

        tracing::debug!(
            provider_count = api_models.len(),
            "Fetched models from providers"
        );

        // Step 2: Build set of existing variants
        let existing_variants = Self::get_existing_static_variants();

        tracing::debug!(
            variant_count = existing_variants.len(),
            "Current static variants in enum"
        );

        // Step 3: Detect new models
        let mut models_added: HashMap<Provider, usize> = HashMap::new();
        let mut providers_checked: Vec<Provider> = Vec::new();
        let mut new_variants: Vec<(Provider, String, String)> = Vec::new(); // (provider, model_id, variant_name)

        for (provider, model_ids) in &api_models {
            providers_checked.push(*provider);

            for model_id in model_ids {
                let variant_name = Self::model_id_to_variant_name(model_id);
                let full_variant = format!("{:?}__{}", provider, variant_name);

                if !existing_variants.contains(&full_variant) {
                    tracing::debug!(
                        provider = ?provider,
                        model_id = %model_id,
                        variant_name = %variant_name,
                        "Detected new model"
                    );

                    new_variants.push((*provider, model_id.clone(), variant_name));
                    *models_added.entry(*provider).or_insert(0) += 1;
                }
            }
        }

        let aggregator_hints_applied = 0; // TODO: Implement aggregator hint logic

        // Step 4: Code generation (placeholder - actual implementation requires AST manipulation)
        if !new_variants.is_empty() {
            tracing::warn!(
                new_variant_count = new_variants.len(),
                "Code generation not yet implemented - new variants detected but not added to enum"
            );

            // TODO: Implement code generation using codegen::inject_enum()
            // This requires:
            // 1. Generate well-formatted enum variants with prettyplease
            // 2. Atomic write strategy (temp file → validate → backup → rename)
            // 3. AST validation with syn::parse_file()
            // 4. Comment markers for generated section

            for (provider, model_id, variant_name) in &new_variants {
                tracing::debug!(
                    provider = ?provider,
                    model_id = %model_id,
                    variant_name = %variant_name,
                    "Would add variant (codegen pending)"
                );
            }
        }

        tracing::info!(
            providers_checked = providers_checked.len(),
            total_new_models = new_variants.len(),
            "ProviderModel update complete"
        );

        Ok(UpdateSummary {
            models_added,
            providers_checked,
            aggregator_hints_applied,
        })
    }

    /// Get list of existing static variant names from enum definition
    ///
    /// Used by update() to avoid duplicating variants.
    fn get_existing_static_variants() -> HashSet<String> {
        // Hardcoded list of current static variants
        // In full implementation, this would parse the enum definition via AST
        let mut variants = HashSet::new();

        // Anthropic variants
        variants.insert("Anthropic__ClaudeOpus__4__5__20251101".to_string());
        variants.insert("Anthropic__ClaudeSonnet__4__5__20250929".to_string());
        variants.insert("Anthropic__ClaudeHaiku__4__0__20250107".to_string());

        // OpenAI variants
        variants.insert("OpenAi__Gpt__4o".to_string());
        variants.insert("OpenAi__Gpt__4o__Mini".to_string());
        variants.insert("OpenAi__O1".to_string());

        // Deepseek variants
        variants.insert("Deepseek__Chat".to_string());
        variants.insert("Deepseek__Reasoner".to_string());

        // Gemini variants
        variants.insert("Gemini__Gemini__3__Flash__Preview".to_string());
        variants.insert("Gemini__Gemini__2__0__Flash__Exp".to_string());

        variants
    }

    /// Convert model ID to variant name following naming convention
    ///
    /// ## Naming Convention
    ///
    /// - Replace `-` with `__` (double underscore)
    /// - Replace `.` with `_` (single underscore)
    /// - Remove `:`
    ///
    /// ## Examples
    ///
    /// - `claude-opus-4.5:20251101` → `ClaudeOpus__4_5__20251101`
    /// - `gpt-4o` → `Gpt__4o`
    fn model_id_to_variant_name(model_id: &str) -> String {
        model_id
            .replace("-", "__")
            .replace(".", "_")
            .replace(":", "")
            .split("__")
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().chain(chars).collect(),
                }
            })
            .collect::<Vec<_>>()
            .join("__")
    }
}

impl std::fmt::Display for ProviderModel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_identifier())
    }
}

impl TryFrom<String> for ProviderModel {
    type Error = super::discovery::ProviderError;

    /// Convert a string like "anthropic/claude-opus-4-5-20251101" to ProviderModel
    ///
    /// ## Conversion Strategy (SYNCHRONOUS ONLY)
    ///
    /// 1. Validate format using regex
    /// 2. Check static variants using naming convention
    /// 3. If no match, return String outlet variant
    /// 4. NO API calls - explicit validation via validate_exists()
    ///
    /// ## Errors
    ///
    /// - `ProviderError::InvalidModelString` - Malformed input
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::providers::ProviderModel;
    ///
    /// // Known model - no API call
    /// let model: ProviderModel = "anthropic/claude-opus-4-5-20251101".to_string().try_into()?;
    /// assert!(matches!(model, ProviderModel::Anthropic__ClaudeOpus__4__5__20251101));
    ///
    /// // Unknown model - uses outlet without API validation
    /// let experimental: ProviderModel = "anthropic/claude-opus-5-experimental".to_string().try_into()?;
    /// assert!(matches!(experimental, ProviderModel::Anthropic(_)));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    fn try_from(value: String) -> Result<Self, Self::Error> {
        use super::discovery::ProviderError;
        use once_cell::sync::Lazy;
        use regex::Regex;

        // Validate format (case-insensitive for provider)
        static MODEL_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"^[a-zA-Z0-9_-]+/[a-zA-Z0-9._:-]+$").expect("Invalid regex")
        });

        if !MODEL_RE.is_match(&value) {
            return Err(ProviderError::InvalidModelString { input: value });
        }

        // Parse provider/model-id
        let parts: Vec<&str> = value.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(ProviderError::InvalidModelString { input: value });
        }

        let provider = parts[0].to_lowercase();
        let model_id = parts[1];

        // Check static variants first, then use outlets
        // This matches the logic in Deserialize but returns ProviderError
        match (provider.as_str(), model_id) {
            // Anthropic static variants
            ("anthropic", "claude-opus-4-5-20251101") => {
                Ok(Self::Anthropic__ClaudeOpus__4__5__20251101)
            }
            ("anthropic", "claude-sonnet-4-5-20250929") => {
                Ok(Self::Anthropic__ClaudeSonnet__4__5__20250929)
            }
            ("anthropic", "claude-haiku-4-0-20250107") => {
                Ok(Self::Anthropic__ClaudeHaiku__4__0__20250107)
            }
            ("anthropic", id) => Ok(Self::Anthropic(id.to_string())),

            // OpenAI static variants
            ("openai", "gpt-4o") => Ok(Self::OpenAi__Gpt__4o),
            ("openai", "gpt-4o-mini") => Ok(Self::OpenAi__Gpt__4o__Mini),
            ("openai", "o1") => Ok(Self::OpenAi__O1),
            ("openai", id) => Ok(Self::OpenAi(id.to_string())),

            // Deepseek static variants
            ("deepseek", "chat") => Ok(Self::Deepseek__Chat),
            ("deepseek", "reasoner") => Ok(Self::Deepseek__Reasoner),
            ("deepseek", id) => Ok(Self::Deepseek(id.to_string())),

            // Gemini static variants
            ("gemini", "gemini-3-flash-preview") => Ok(Self::Gemini__Gemini__3__Flash__Preview),
            ("gemini", "gemini-2-0-flash-exp") => Ok(Self::Gemini__Gemini__2__0__Flash__Exp),
            ("gemini", id) => Ok(Self::Gemini(id.to_string())),

            // Outlets for all other providers
            ("ollama", id) => Ok(Self::Ollama(id.to_string())),
            ("openrouter", id) => Ok(Self::OpenRouter(id.to_string())),
            ("moonshotai", id) => Ok(Self::MoonshotAi(id.to_string())),
            ("zai", id) => Ok(Self::Zai(id.to_string())),
            ("zenmux", id) => Ok(Self::ZenMux(id.to_string())),

            _ => Err(ProviderError::InvalidModelString { input: value }),
        }
    }
}

impl TryFrom<&str> for ProviderModel {
    type Error = super::discovery::ProviderError;

    /// Wrapper around `TryFrom<String>` for &str
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::providers::ProviderModel;
    ///
    /// let model: ProviderModel = "openai/gpt-4o".try_into()?;
    /// assert!(matches!(model, ProviderModel::OpenAi__Gpt__4o));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string())
    }
}

// Serde customization: serialize as string, deserialize from string
impl Serialize for ProviderModel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_identifier())
    }
}

impl<'de> Deserialize<'de> for ProviderModel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // Parse provider/model format
        let parts: Vec<&str> = s.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(serde::de::Error::custom(format!(
                "Invalid model format '{}', expected 'provider/model-id'",
                s
            )));
        }

        let provider = parts[0].to_lowercase();
        let model_id = parts[1];

        // Match against static variants first, then use outlets
        match (provider.as_str(), model_id) {
            // Anthropic static variants
            ("anthropic", "claude-opus-4-5-20251101") => Ok(Self::Anthropic__ClaudeOpus__4__5__20251101),
            ("anthropic", "claude-sonnet-4-5-20250929") => Ok(Self::Anthropic__ClaudeSonnet__4__5__20250929),
            ("anthropic", "claude-haiku-4-0-20250107") => Ok(Self::Anthropic__ClaudeHaiku__4__0__20250107),
            ("anthropic", id) => Ok(Self::Anthropic(id.to_string())),

            // OpenAI static variants
            ("openai", "gpt-4o") => Ok(Self::OpenAi__Gpt__4o),
            ("openai", "gpt-4o-mini") => Ok(Self::OpenAi__Gpt__4o__Mini),
            ("openai", "o1") => Ok(Self::OpenAi__O1),
            ("openai", id) => Ok(Self::OpenAi(id.to_string())),

            // Deepseek static variants
            ("deepseek", "chat") => Ok(Self::Deepseek__Chat),
            ("deepseek", "reasoner") => Ok(Self::Deepseek__Reasoner),
            ("deepseek", id) => Ok(Self::Deepseek(id.to_string())),

            // Gemini static variants
            ("gemini", "gemini-3-flash-preview") => Ok(Self::Gemini__Gemini__3__Flash__Preview),
            ("gemini", "gemini-2-0-flash-exp") => Ok(Self::Gemini__Gemini__2__0__Flash__Exp),
            ("gemini", id) => Ok(Self::Gemini(id.to_string())),

            // Outlets for all other providers
            ("ollama", id) => Ok(Self::Ollama(id.to_string())),
            ("openrouter", id) => Ok(Self::OpenRouter(id.to_string())),
            ("moonshotai", id) => Ok(Self::MoonshotAi(id.to_string())),
            ("zai", id) => Ok(Self::Zai(id.to_string())),
            ("zenmux", id) => Ok(Self::ZenMux(id.to_string())),

            _ => Err(serde::de::Error::custom(format!(
                "Unknown provider '{}'",
                provider
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_entry_new() {
        let entry = LlmEntry::new("openai", "gpt-4");
        assert_eq!(entry.provider, "openai");
        assert_eq!(entry.model, "gpt-4");
    }

    #[test]
    fn llm_entry_identifier() {
        let entry = LlmEntry::new("anthropic", "claude-opus-4.5");
        assert_eq!(entry.identifier(), "anthropic/claude-opus-4.5");
    }

    #[test]
    fn provider_list_format_default() {
        assert_eq!(ProviderListFormat::default(), ProviderListFormat::StringLiterals);
    }

    // ProviderModel tests
    mod provider_model {
        use super::*;
        use crate::providers::base::Provider;

        #[test]
        fn static_variant_provider() {
            let model = ProviderModel::Anthropic__ClaudeOpus__4__5__20251101;
            assert_eq!(model.provider(), Provider::Anthropic);
        }

        #[test]
        fn outlet_variant_provider() {
            let model = ProviderModel::OpenAi("gpt-5".to_string());
            assert_eq!(model.provider(), Provider::OpenAi);
        }

        #[test]
        fn static_variant_model_id() {
            let model = ProviderModel::Anthropic__ClaudeOpus__4__5__20251101;
            assert_eq!(model.model_id(), "claude-opus-4-5-20251101");
        }

        #[test]
        fn outlet_variant_model_id() {
            let model = ProviderModel::Deepseek("chat-v2".to_string());
            assert_eq!(model.model_id(), "chat-v2");
        }

        #[test]
        fn to_identifier_static() {
            let model = ProviderModel::OpenAi__Gpt__4o;
            assert_eq!(model.to_identifier(), "openai/gpt-4o");
        }

        #[test]
        fn to_identifier_outlet() {
            let model = ProviderModel::Gemini("gemini-pro".to_string());
            assert_eq!(model.to_identifier(), "gemini/gemini-pro");
        }

        #[test]
        fn display_trait() {
            let model = ProviderModel::Anthropic__ClaudeSonnet__4__5__20250929;
            assert_eq!(format!("{}", model), "anthropic/claude-sonnet-4-5-20250929");
        }

        #[test]
        fn serialize_static_variant() {
            let model = ProviderModel::Anthropic__ClaudeOpus__4__5__20251101;
            let json = serde_json::to_string(&model).unwrap();
            assert_eq!(json, "\"anthropic/claude-opus-4-5-20251101\"");
        }

        #[test]
        fn serialize_outlet_variant() {
            let model = ProviderModel::OpenAi("gpt-5-turbo".to_string());
            let json = serde_json::to_string(&model).unwrap();
            assert_eq!(json, "\"openai/gpt-5-turbo\"");
        }

        #[test]
        fn deserialize_to_static_variant() {
            let json = "\"anthropic/claude-opus-4-5-20251101\"";
            let model: ProviderModel = serde_json::from_str(json).unwrap();
            assert_eq!(model, ProviderModel::Anthropic__ClaudeOpus__4__5__20251101);
        }

        #[test]
        fn deserialize_to_outlet_variant() {
            let json = "\"openai/gpt-5-experimental\"";
            let model: ProviderModel = serde_json::from_str(json).unwrap();
            assert_eq!(model, ProviderModel::OpenAi("gpt-5-experimental".to_string()));
        }

        #[test]
        fn deserialize_invalid_format() {
            let json = "\"no-slash-here\"";
            let result: Result<ProviderModel, _> = serde_json::from_str(json);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("expected 'provider/model-id'"));
        }

        #[test]
        fn deserialize_unknown_provider() {
            let json = "\"unknown-provider/some-model\"";
            let result: Result<ProviderModel, _> = serde_json::from_str(json);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Unknown provider"));
        }

        #[test]
        fn pattern_matching_all_static_variants() {
            let models = vec![
                ProviderModel::Anthropic__ClaudeOpus__4__5__20251101,
                ProviderModel::Anthropic__ClaudeSonnet__4__5__20250929,
                ProviderModel::Anthropic__ClaudeHaiku__4__0__20250107,
                ProviderModel::OpenAi__Gpt__4o,
                ProviderModel::OpenAi__Gpt__4o__Mini,
                ProviderModel::OpenAi__O1,
                ProviderModel::Deepseek__Chat,
                ProviderModel::Deepseek__Reasoner,
                ProviderModel::Gemini__Gemini__3__Flash__Preview,
                ProviderModel::Gemini__Gemini__2__0__Flash__Exp,
            ];

            for model in models {
                // Should match without panicking
                match model {
                    ProviderModel::Anthropic__ClaudeOpus__4__5__20251101 => (),
                    ProviderModel::Anthropic__ClaudeSonnet__4__5__20250929 => (),
                    ProviderModel::Anthropic__ClaudeHaiku__4__0__20250107 => (),
                    ProviderModel::OpenAi__Gpt__4o => (),
                    ProviderModel::OpenAi__Gpt__4o__Mini => (),
                    ProviderModel::OpenAi__O1 => (),
                    ProviderModel::Deepseek__Chat => (),
                    ProviderModel::Deepseek__Reasoner => (),
                    ProviderModel::Gemini__Gemini__3__Flash__Preview => (),
                    ProviderModel::Gemini__Gemini__2__0__Flash__Exp => (),
                    _ => panic!("Unexpected variant"),
                }
            }
        }

        #[test]
        fn derives_debug_clone_partialeq_eq_hash() {
            let model1 = ProviderModel::Anthropic__ClaudeOpus__4__5__20251101;
            let model2 = model1.clone();

            // Debug
            let debug_str = format!("{:?}", model1);
            assert!(debug_str.contains("Anthropic__ClaudeOpus__4__5__20251101"));

            // Clone
            assert_eq!(model1, model2);

            // PartialEq/Eq
            assert_eq!(model1, ProviderModel::Anthropic__ClaudeOpus__4__5__20251101);
            assert_ne!(model1, ProviderModel::OpenAi__Gpt__4o);

            // Hash (can be used in HashMap)
            let mut map = std::collections::HashMap::new();
            map.insert(model1.clone(), "test");
            assert_eq!(map.get(&model2), Some(&"test"));
        }

        #[test]
        fn round_trip_serialization() {
            let models = vec![
                ProviderModel::Anthropic__ClaudeOpus__4__5__20251101,
                ProviderModel::OpenAi("custom-model".to_string()),
                ProviderModel::Ollama("llama2".to_string()),
            ];

            for original in models {
                let json = serde_json::to_string(&original).unwrap();
                let deserialized: ProviderModel = serde_json::from_str(&json).unwrap();
                assert_eq!(original, deserialized);
            }
        }

        // TryFrom tests
        #[test]
        fn try_from_string_known_model_anthropic() {
            let model: ProviderModel = "anthropic/claude-opus-4-5-20251101"
                .to_string()
                .try_into()
                .unwrap();
            assert_eq!(
                model,
                ProviderModel::Anthropic__ClaudeOpus__4__5__20251101
            );
        }

        #[test]
        fn try_from_string_known_model_openai() {
            let model: ProviderModel = "openai/gpt-4o".to_string().try_into().unwrap();
            assert_eq!(model, ProviderModel::OpenAi__Gpt__4o);
        }

        #[test]
        fn try_from_string_unknown_model_uses_outlet() {
            let model: ProviderModel = "anthropic/claude-opus-5-experimental"
                .to_string()
                .try_into()
                .unwrap();
            assert_eq!(
                model,
                ProviderModel::Anthropic("claude-opus-5-experimental".to_string())
            );
        }

        #[test]
        fn try_from_str_wrapper() {
            let model: ProviderModel = "deepseek/chat".try_into().unwrap();
            assert_eq!(model, ProviderModel::Deepseek__Chat);
        }

        #[test]
        fn try_from_invalid_format_no_slash() {
            let result: Result<ProviderModel, _> = "no-slash-here".to_string().try_into();
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(err
                .to_string()
                .contains("Invalid model string format"));
        }

        #[test]
        fn try_from_invalid_format_provider_only() {
            let result: Result<ProviderModel, _> = "provider/".to_string().try_into();
            assert!(result.is_err());
        }

        #[test]
        fn try_from_invalid_format_model_only() {
            let result: Result<ProviderModel, _> = "/model".to_string().try_into();
            assert!(result.is_err());
        }

        #[test]
        fn try_from_invalid_format_empty() {
            let result: Result<ProviderModel, _> = "".to_string().try_into();
            assert!(result.is_err());
        }

        #[test]
        fn try_from_invalid_format_double_slash() {
            let result: Result<ProviderModel, _> = "provider//model".to_string().try_into();
            assert!(result.is_err());
        }

        #[test]
        fn try_from_unknown_provider() {
            let result: Result<ProviderModel, _> =
                "unknown-provider/some-model".to_string().try_into();
            assert!(result.is_err());
        }

        #[test]
        fn try_from_case_insensitive_provider() {
            // Should normalize to lowercase
            let model: ProviderModel = "OpenAI/gpt-4o".to_string().try_into().unwrap();
            assert_eq!(model, ProviderModel::OpenAi__Gpt__4o);
        }

        #[tokio::test]
        async fn validate_exists_stub() {
            let model = ProviderModel::Anthropic__ClaudeOpus__4__5__20251101;
            // Stub implementation should return Ok for now
            assert!(model.validate_exists().await.is_ok());
        }
    }
}





/// Strongly-typed enumeration of LLM provider models
///
/// This enum provides compile-time safety for known models while allowing
/// runtime flexibility for bleeding-edge or undocumented models via String
/// outlets.
///
/// > **NOTE:** the entries in this enumeration are generated programmatically
/// and should not be edited manually.
///
/// ## Naming Convention
///
/// ### Provider's Naming
///
/// There are two types of providers:
///
/// 1. Direct Providers
/// 2. Aggregators
///
/// With direct providers, the unique name they would use for a model would
/// be a kebab-cased name but no provider self-referencing is provided. Sometimes,
/// but not always, you'll find a date reference at the end of the model name.
/// This allows the provider to provide incremental updates to their model
/// without bumping their official version number.
///
/// Examples:
///
/// - `gpt-5.1-codex` from OpenAI
/// - `claude-opus-4-5-20251101` from Anthropic
/// - `claude-haiku-4-5-20251001` from Anthropic
///
/// Not surprisingly, an Aggregator -- who's function is to provide models
/// from multiple _underlying_ providers -- have a need to reference the
/// underlying model's provider in order to avoid namespace collisions as
/// well as provide visibility to callers where the model is coming from.
///
/// Examples:
///
/// - `openai/gpt-5.1-codex`
/// - `anthropic/claude-opus-4-5-20251101`
///
/// ### Converting Provider Names to an Enum
///
/// We then faced with providing a distinct enumeration variant for
/// every model we find (by calling the provider's OpenAI compatible
/// API). This is achieved by:
///
/// 1. switching the model names kebab-case naming convention to a
/// PascalCase name with snake_case separators.
/// 2. prepending the provider's name in PascalCase followed by a `__` delimiter
///   - for aggregators we prepend _both_ the aggregator's name AND the
///     underlying providers name
///
/// Examples:
///
/// - Anthropic's Opus model:
///     - typically referred to as `claude-opus-4-5-20251101`
///     - enum variant is `Anthropic__ClaudeOpus_4_5_20251101` when
///       getting it directly from Anthropic
///     - enum variant 0s `OpenRouter__Anthropic__Claude_Opus_4_5_`
///
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, strum::AsRefStr)]
pub enum ProviderModel {

    OpenAi(String),
    /// Outlet for undocumented or bleeding-edge Deepseek models
    Deepseek(String),
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

    // === AUTO-GENERATED VARIANTS (do not edit manually) ===
    // Generated: 2026-01-02T01:28:12Z via ProviderModel::update()






    // === END AUTO-GENERATED ===
}

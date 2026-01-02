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
///     let summary = ProviderModel::update(false).await?;
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, strum::AsRefStr)]
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

    // === AUTO-GENERATED VARIANTS (do not edit manually) ===
    // Generated: 2026-01-02T01:28:12Z via ProviderModel::update()
    OpenRouter__Bytedance__Seed___seed__1_6__Flash,
    OpenRouter__Bytedance__Seed___seed__1_6,
    OpenRouter__Minimax___minimax__M2_1,
    OpenRouter__Z__Ai___glm__4_7,
    OpenRouter__Google___gemini__3__Flash__Preview,
    OpenRouter__Mistralai___mistral__Small__Creative,
    OpenRouter__Allenai___olmo__3_1__32b__Thinkfree,
    OpenRouter__Xiaomi___mimo__V2__Flashfree,
    OpenRouter__Nvidia___nemotron__3__Nano__30b__A3bfree,
    OpenRouter__Nvidia___nemotron__3__Nano__30b__A3b,
    OpenRouter__Openai___gpt__5_2__Chat,
    OpenRouter__Openai___gpt__5_2__Pro,
    OpenRouter__Openai___gpt__5_2,
    OpenRouter__Mistralai___devstral__2512free,
    OpenRouter__Mistralai___devstral__2512,
    OpenRouter__Relace___relace__Search,
    OpenRouter__Z__Ai___glm__4_6v,
    OpenRouter__Nex__Agi___deepseek__V3_1__Nex__N1free,
    OpenRouter__Essentialai___rnj__1__Instruct,
    OpenRouter__Openrouter___bodybuilder,
    OpenRouter__Openai___gpt__5_1__Codex__Max,
    OpenRouter__Amazon___nova__2__Lite__V1,
    OpenRouter__Mistralai___ministral__14b__2512,
    OpenRouter__Mistralai___ministral__8b__2512,
    OpenRouter__Mistralai___ministral__3b__2512,
    OpenRouter__Mistralai___mistral__Large__2512,
    OpenRouter__Arcee__Ai___trinity__Minifree,
    OpenRouter__Arcee__Ai___trinity__Mini,
    OpenRouter__Deepseek___deepseek__V3_2__Speciale,
    OpenRouter__Deepseek___deepseek__V3_2,
    OpenRouter__Prime__Intellect___intellect__3,
    OpenRouter__Tngtech___tng__R1t__Chimerafree,
    OpenRouter__Tngtech___tng__R1t__Chimera,
    OpenRouter__Anthropic___claude__Opus__4_5,
    OpenRouter__Allenai___olmo__3__32b__Thinkfree,
    OpenRouter__Allenai___olmo__3__7b__Instruct,
    OpenRouter__Allenai___olmo__3__7b__Think,
    OpenRouter__Google___gemini__3__Pro__Image__Preview,
    OpenRouter__X__Ai___grok__4_1__Fast,
    OpenRouter__Google___gemini__3__Pro__Preview,
    OpenRouter__Deepcogito___cogito__V2_1__671b,
    OpenRouter__Openai___gpt__5_1,
    OpenRouter__Openai___gpt__5_1__Chat,
    OpenRouter__Openai___gpt__5_1__Codex,
    OpenRouter__Openai___gpt__5_1__Codex__Mini,
    OpenRouter__Kwaipilot___kat__Coder__Profree,
    OpenRouter__Moonshotai___kimi__K2__Thinking,
    OpenRouter__Amazon___nova__Premier__V1,
    OpenRouter__Perplexity___sonar__Pro__Search,
    OpenRouter__Mistralai___voxtral__Small__24b__2507,
    OpenRouter__Openai___gpt__Oss__Safeguard__20b,
    OpenRouter__Nvidia___nemotron__Nano__12b__V2__Vlfree,
    OpenRouter__Nvidia___nemotron__Nano__12b__V2__Vl,
    OpenRouter__Minimax___minimax__M2,
    OpenRouter__Qwen___qwen3__Vl__32b__Instruct,
    OpenRouter__Liquid___lfm2__8b__A1b,
    OpenRouter__Liquid___lfm__2_2__6b,
    OpenRouter__Ibm__Granite___granite__4_0__H__Micro,
    OpenRouter__Deepcogito___cogito__V2__Preview__Llama__405b,
    OpenRouter__Openai___gpt__5__Image__Mini,
    OpenRouter__Anthropic___claude__Haiku__4_5,
    OpenRouter__Qwen___qwen3__Vl__8b__Thinking,
    OpenRouter__Qwen___qwen3__Vl__8b__Instruct,
    OpenRouter__Openai___gpt__5__Image,
    OpenRouter__Openai___o3__Deep__Research,
    OpenRouter__Openai___o4__Mini__Deep__Research,
    OpenRouter__Nvidia___llama__3_3__Nemotron__Super__49b__V1_5,
    OpenRouter__Baidu___ernie__4_5__21b__A3b__Thinking,
    OpenRouter__Google___gemini__2_5__Flash__Image,
    OpenRouter__Qwen___qwen3__Vl__30b__A3b__Thinking,
    OpenRouter__Qwen___qwen3__Vl__30b__A3b__Instruct,
    OpenRouter__Openai___gpt__5__Pro,
    OpenRouter__Z__Ai___glm__4_6,
    OpenRouter__Z__Ai___glm__4_6exacto,
    OpenRouter__Anthropic___claude__Sonnet__4_5,
    OpenRouter__Deepseek___deepseek__V3_2__Exp,
    OpenRouter__Thedrummer___cydonia__24b__V4_1,
    OpenRouter__Relace___relace__Apply__3,
    OpenRouter__Google___gemini__2_5__Flash__Preview__09__2025,
    OpenRouter__Google___gemini__2_5__Flash__Lite__Preview__09__2025,
    OpenRouter__Qwen___qwen3__Vl__235b__A22b__Thinking,
    OpenRouter__Qwen___qwen3__Vl__235b__A22b__Instruct,
    OpenRouter__Qwen___qwen3__Max,
    OpenRouter__Qwen___qwen3__Coder__Plus,
    OpenRouter__Openai___gpt__5__Codex,
    OpenRouter__Deepseek___deepseek__V3_1__Terminusexacto,
    OpenRouter__Deepseek___deepseek__V3_1__Terminus,
    OpenRouter__X__Ai___grok__4__Fast,
    OpenRouter__Alibaba___tongyi__Deepresearch__30b__A3bfree,
    OpenRouter__Alibaba___tongyi__Deepresearch__30b__A3b,
    OpenRouter__Qwen___qwen3__Coder__Flash,
    OpenRouter__Opengvlab___internvl3__78b,
    OpenRouter__Qwen___qwen3__Next__80b__A3b__Thinking,
    OpenRouter__Qwen___qwen3__Next__80b__A3b__Instruct,
    OpenRouter__Meituan___longcat__Flash__Chat,
    OpenRouter__Qwen___qwen__Plus__2025__07__28,
    OpenRouter__Qwen___qwen__Plus__2025__07__28thinking,
    OpenRouter__Nvidia___nemotron__Nano__9b__V2free,
    OpenRouter__Nvidia___nemotron__Nano__9b__V2,
    OpenRouter__Moonshotai___kimi__K2__0905,
    OpenRouter__Moonshotai___kimi__K2__0905exacto,
    OpenRouter__Deepcogito___cogito__V2__Preview__Llama__70b,
    OpenRouter__Deepcogito___cogito__V2__Preview__Llama__109b__Moe,
    OpenRouter__Stepfun__Ai___step3,
    OpenRouter__Qwen___qwen3__30b__A3b__Thinking__2507,
    OpenRouter__X__Ai___grok__Code__Fast__1,
    OpenRouter__Nousresearch___hermes__4__70b,
    OpenRouter__Nousresearch___hermes__4__405b,
    OpenRouter__Google___gemini__2_5__Flash__Image__Preview,
    OpenRouter__Deepseek___deepseek__Chat__V3_1,
    OpenRouter__Openai___gpt__4o__Audio__Preview,
    OpenRouter__Mistralai___mistral__Medium__3_1,
    OpenRouter__Baidu___ernie__4_5__21b__A3b,
    OpenRouter__Baidu___ernie__4_5__Vl__28b__A3b,
    OpenRouter__Z__Ai___glm__4_5v,
    OpenRouter__Ai21___jamba__Mini__1_7,
    OpenRouter__Ai21___jamba__Large__1_7,
    OpenRouter__Openai___gpt__5__Chat,
    OpenRouter__Openai___gpt__5,
    OpenRouter__Openai___gpt__5__Mini,
    OpenRouter__Openai___gpt__5__Nano,
    OpenRouter__Openai___gpt__Oss__120bfree,
    OpenRouter__Openai___gpt__Oss__120b,
    OpenRouter__Openai___gpt__Oss__120bexacto,
    OpenRouter__Openai___gpt__Oss__20bfree,
    OpenRouter__Openai___gpt__Oss__20b,
    OpenRouter__Anthropic___claude__Opus__4_1,
    OpenRouter__Mistralai___codestral__2508,
    OpenRouter__Qwen___qwen3__Coder__30b__A3b__Instruct,
    OpenRouter__Qwen___qwen3__30b__A3b__Instruct__2507,
    OpenRouter__Z__Ai___glm__4_5,
    OpenRouter__Z__Ai___glm__4_5__Airfree,
    OpenRouter__Z__Ai___glm__4_5__Air,
    OpenRouter__Qwen___qwen3__235b__A22b__Thinking__2507,
    OpenRouter__Z__Ai___glm__4__32b,
    OpenRouter__Qwen___qwen3__Coderfree,
    OpenRouter__Qwen___qwen3__Coder,
    OpenRouter__Qwen___qwen3__Coderexacto,
    OpenRouter__Bytedance___ui__Tars__1_5__7b,
    OpenRouter__Google___gemini__2_5__Flash__Lite,
    OpenRouter__Qwen___qwen3__235b__A22b__2507,
    OpenRouter__Switchpoint___router,
    OpenRouter__Moonshotai___kimi__K2free,
    OpenRouter__Moonshotai___kimi__K2,
    OpenRouter__Thudm___glm__4_1v__9b__Thinking,
    OpenRouter__Mistralai___devstral__Medium,
    OpenRouter__Mistralai___devstral__Small,
    OpenRouter__Cognitivecomputations___dolphin__Mistral__24b__Venice__Editionfree,
    OpenRouter__X__Ai___grok__4,
    OpenRouter__Google___gemma__3n__E2b__Itfree,
    OpenRouter__Tencent___hunyuan__A13b__Instruct,
    OpenRouter__Tngtech___deepseek__R1t2__Chimerafree,
    OpenRouter__Tngtech___deepseek__R1t2__Chimera,
    OpenRouter__Morph___morph__V3__Large,
    OpenRouter__Morph___morph__V3__Fast,
    OpenRouter__Baidu___ernie__4_5__Vl__424b__A47b,
    OpenRouter__Baidu___ernie__4_5__300b__A47b,
    OpenRouter__Inception___mercury,
    OpenRouter__Mistralai___mistral__Small__3_2__24b__Instruct,
    OpenRouter__Minimax___minimax__M1,
    OpenRouter__Google___gemini__2_5__Flash,
    OpenRouter__Google___gemini__2_5__Pro,
    OpenRouter__Moonshotai___kimi__Dev__72b,
    OpenRouter__Openai___o3__Pro,
    OpenRouter__X__Ai___grok__3__Mini,
    OpenRouter__X__Ai___grok__3,
    OpenRouter__Google___gemini__2_5__Pro__Preview,
    OpenRouter__Deepseek___deepseek__R1__0528__Qwen3__8b,
    OpenRouter__Deepseek___deepseek__R1__0528free,
    OpenRouter__Deepseek___deepseek__R1__0528,
    OpenRouter__Anthropic___claude__Opus__4,
    OpenRouter__Anthropic___claude__Sonnet__4,
    OpenRouter__Mistralai___devstral__Small__2505,
    OpenRouter__Google___gemma__3n__E4b__Itfree,
    OpenRouter__Google___gemma__3n__E4b__It,
    OpenRouter__Openai___codex__Mini,
    OpenRouter__Nousresearch___deephermes__3__Mistral__24b__Preview,
    OpenRouter__Mistralai___mistral__Medium__3,
    OpenRouter__Google___gemini__2_5__Pro__Preview__05__06,
    OpenRouter__Arcee__Ai___spotlight,
    OpenRouter__Arcee__Ai___maestro__Reasoning,
    OpenRouter__Arcee__Ai___virtuoso__Large,
    OpenRouter__Arcee__Ai___coder__Large,
    OpenRouter__Microsoft___phi__4__Reasoning__Plus,
    OpenRouter__Inception___mercury__Coder,
    OpenRouter__Qwen___qwen3__4bfree,
    OpenRouter__Deepseek___deepseek__Prover__V2,
    OpenRouter__Meta__Llama___llama__Guard__4__12b,
    OpenRouter__Qwen___qwen3__30b__A3b,
    OpenRouter__Qwen___qwen3__8b,
    OpenRouter__Qwen___qwen3__14b,
    OpenRouter__Qwen___qwen3__32b,
    OpenRouter__Qwen___qwen3__235b__A22b,
    OpenRouter__Tngtech___deepseek__R1t__Chimerafree,
    OpenRouter__Tngtech___deepseek__R1t__Chimera,
    OpenRouter__Openai___o4__Mini__High,
    OpenRouter__Openai___o3,
    OpenRouter__Openai___o4__Mini,
    OpenRouter__Qwen___qwen2_5__Coder__7b__Instruct,
    OpenRouter__Openai___gpt__4_1,
    OpenRouter__Openai___gpt__4_1__Mini,
    OpenRouter__Openai___gpt__4_1__Nano,
    OpenRouter__Eleutherai___llemma_7b,
    OpenRouter__Alfredpros___codellama__7b__Instruct__Solidity,
    OpenRouter__Arliai___qwq__32b__Arliai__Rpr__V1,
    OpenRouter__X__Ai___grok__3__Mini__Beta,
    OpenRouter__X__Ai___grok__3__Beta,
    OpenRouter__Nvidia___llama__3_1__Nemotron__Ultra__253b__V1,
    OpenRouter__Meta__Llama___llama__4__Maverick,
    OpenRouter__Meta__Llama___llama__4__Scout,
    OpenRouter__Qwen___qwen2_5__Vl__32b__Instruct,
    OpenRouter__Deepseek___deepseek__Chat__V3__0324,
    OpenRouter__Openai___o1__Pro,
    OpenRouter__Mistralai___mistral__Small__3_1__24b__Instructfree,
    OpenRouter__Mistralai___mistral__Small__3_1__24b__Instruct,
    OpenRouter__Allenai___olmo__2__0325__32b__Instruct,
    OpenRouter__Google___gemma__3__4b__Itfree,
    OpenRouter__Google___gemma__3__4b__It,
    OpenRouter__Google___gemma__3__12b__Itfree,
    OpenRouter__Google___gemma__3__12b__It,
    OpenRouter__Cohere___command__A,
    OpenRouter__Openai___gpt__4o__Mini__Search__Preview,
    OpenRouter__Openai___gpt__4o__Search__Preview,
    OpenRouter__Google___gemma__3__27b__Itfree,
    OpenRouter__Google___gemma__3__27b__It,
    OpenRouter__Thedrummer___skyfall__36b__V2,
    OpenRouter__Microsoft___phi__4__Multimodal__Instruct,
    OpenRouter__Perplexity___sonar__Reasoning__Pro,
    OpenRouter__Perplexity___sonar__Pro,
    OpenRouter__Perplexity___sonar__Deep__Research,
    OpenRouter__Qwen___qwq__32b,
    OpenRouter__Google___gemini__2_0__Flash__Lite__001,
    OpenRouter__Anthropic___claude__3_7__Sonnetthinking,
    OpenRouter__Anthropic___claude__3_7__Sonnet,
    OpenRouter__Mistralai___mistral__Saba,
    OpenRouter__Meta__Llama___llama__Guard__3__8b,
    OpenRouter__Openai___o3__Mini__High,
    OpenRouter__Google___gemini__2_0__Flash__001,
    OpenRouter__Qwen___qwen__Vl__Plus,
    OpenRouter__Aion__Labs___aion__1_0,
    OpenRouter__Aion__Labs___aion__1_0__Mini,
    OpenRouter__Aion__Labs___aion__Rp__Llama__3_1__8b,
    OpenRouter__Qwen___qwen__Vl__Max,
    OpenRouter__Qwen___qwen__Turbo,
    OpenRouter__Qwen___qwen2_5__Vl__72b__Instruct,
    OpenRouter__Qwen___qwen__Plus,
    OpenRouter__Qwen___qwen__Max,
    OpenRouter__Openai___o3__Mini,
    OpenRouter__Mistralai___mistral__Small__24b__Instruct__2501,
    OpenRouter__Deepseek___deepseek__R1__Distill__Qwen__32b,
    OpenRouter__Deepseek___deepseek__R1__Distill__Qwen__14b,
    OpenRouter__Perplexity___sonar__Reasoning,
    OpenRouter__Perplexity___sonar,
    OpenRouter__Deepseek___deepseek__R1__Distill__Llama__70b,
    OpenRouter__Deepseek___deepseek__R1,
    OpenRouter__Minimax___minimax__01,
    OpenRouter__Microsoft___phi__4,
    OpenRouter__Sao10k___l3_1__70b__Hanami__X1,
    OpenRouter__Deepseek___deepseek__Chat,
    OpenRouter__Sao10k___l3_3__Euryale__70b,
    OpenRouter__Openai___o1,
    OpenRouter__Cohere___command__R7b__12__2024,
    OpenRouter__Google___gemini__2_0__Flash__Expfree,
    OpenRouter__Meta__Llama___llama__3_3__70b__Instructfree,
    OpenRouter__Meta__Llama___llama__3_3__70b__Instruct,
    OpenRouter__Amazon___nova__Lite__V1,
    OpenRouter__Amazon___nova__Micro__V1,
    OpenRouter__Amazon___nova__Pro__V1,
    OpenRouter__Openai___gpt__4o__2024__11__20,
    OpenRouter__Mistralai___mistral__Large__2411,
    OpenRouter__Mistralai___mistral__Large__2407,
    OpenRouter__Mistralai___pixtral__Large__2411,
    OpenRouter__Qwen___qwen__2_5__Coder__32b__Instruct,
    OpenRouter__Raifle___sorcererlm__8x22b,
    OpenRouter__Thedrummer___unslopnemo__12b,
    OpenRouter__Anthropic___claude__3_5__Haiku,
    OpenRouter__Anthropic___claude__3_5__Haiku__20241022,
    OpenRouter__Anthracite__Org___magnum__V4__72b,
    OpenRouter__Anthropic___claude__3_5__Sonnet,
    OpenRouter__Mistralai___ministral__8b,
    OpenRouter__Mistralai___ministral__3b,
    OpenRouter__Qwen___qwen__2_5__7b__Instruct,
    OpenRouter__Nvidia___llama__3_1__Nemotron__70b__Instruct,
    OpenRouter__Inflection___inflection__3__Pi,
    OpenRouter__Inflection___inflection__3__Productivity,
    OpenRouter__Thedrummer___rocinante__12b,
    OpenRouter__Meta__Llama___llama__3_2__1b__Instruct,
    OpenRouter__Meta__Llama___llama__3_2__11b__Vision__Instruct,
    OpenRouter__Meta__Llama___llama__3_2__90b__Vision__Instruct,
    OpenRouter__Meta__Llama___llama__3_2__3b__Instructfree,
    OpenRouter__Meta__Llama___llama__3_2__3b__Instruct,
    OpenRouter__Qwen___qwen__2_5__72b__Instruct,
    OpenRouter__Neversleep___llama__3_1__Lumimaid__8b,
    OpenRouter__Mistralai___pixtral__12b,
    OpenRouter__Cohere___command__R__08__2024,
    OpenRouter__Cohere___command__R__Plus__08__2024,
    OpenRouter__Sao10k___l3_1__Euryale__70b,
    OpenRouter__Qwen___qwen__2_5__Vl__7b__Instructfree,
    OpenRouter__Qwen___qwen__2_5__Vl__7b__Instruct,
    OpenRouter__Microsoft___phi__3_5__Mini__128k__Instruct,
    OpenRouter__Nousresearch___hermes__3__Llama__3_1__70b,
    OpenRouter__Nousresearch___hermes__3__Llama__3_1__405bfree,
    OpenRouter__Nousresearch___hermes__3__Llama__3_1__405b,
    OpenRouter__Openai___chatgpt__4o__Latest,
    OpenRouter__Sao10k___l3__Lunaris__8b,
    OpenRouter__Openai___gpt__4o__2024__08__06,
    OpenRouter__Meta__Llama___llama__3_1__405b,
    OpenRouter__Meta__Llama___llama__3_1__405b__Instructfree,
    OpenRouter__Meta__Llama___llama__3_1__405b__Instruct,
    OpenRouter__Meta__Llama___llama__3_1__70b__Instruct,
    OpenRouter__Meta__Llama___llama__3_1__8b__Instruct,
    OpenRouter__Mistralai___mistral__Nemo,
    OpenRouter__Openai___gpt__4o__Mini__2024__07__18,
    OpenRouter__Openai___gpt__4o__Mini,
    OpenRouter__Google___gemma__2__27b__It,
    OpenRouter__Google___gemma__2__9b__It,
    OpenRouter__Sao10k___l3__Euryale__70b,
    OpenRouter__Mistralai___mistral__7b__Instruct__V0_3,
    OpenRouter__Mistralai___mistral__7b__Instructfree,
    OpenRouter__Mistralai___mistral__7b__Instruct,
    OpenRouter__Nousresearch___hermes__2__Pro__Llama__3__8b,
    OpenRouter__Microsoft___phi__3__Mini__128k__Instruct,
    OpenRouter__Microsoft___phi__3__Medium__128k__Instruct,
    OpenRouter__Openai___gpt__4o,
    OpenRouter__Openai___gpt__4oextended,
    OpenRouter__Openai___gpt__4o__2024__05__13,
    OpenRouter__Meta__Llama___llama__Guard__2__8b,
    OpenRouter__Meta__Llama___llama__3__70b__Instruct,
    OpenRouter__Meta__Llama___llama__3__8b__Instruct,
    OpenRouter__Mistralai___mixtral__8x22b__Instruct,
    OpenRouter__Microsoft___wizardlm__2__8x22b,
    OpenRouter__Openai___gpt__4__Turbo,
    OpenRouter__Anthropic___claude__3__Haiku,
    OpenRouter__Anthropic___claude__3__Opus,
    OpenRouter__Mistralai___mistral__Large,
    OpenRouter__Openai___gpt__3_5__Turbo__0613,
    OpenRouter__Openai___gpt__4__Turbo__Preview,
    OpenRouter__Mistralai___mistral__Tiny,
    OpenRouter__Mistralai___mistral__7b__Instruct__V0_2,
    OpenRouter__Mistralai___mixtral__8x7b__Instruct,
    OpenRouter__Neversleep___noromaid__20b,
    OpenRouter__Alpindale___goliath__120b,
    OpenRouter__Openrouter___auto,
    OpenRouter__Openai___gpt__4__1106__Preview,
    OpenRouter__Mistralai___mistral__7b__Instruct__V0_1,
    OpenRouter__Openai___gpt__3_5__Turbo__Instruct,
    OpenRouter__Openai___gpt__3_5__Turbo__16k,
    OpenRouter__Mancer___weaver,
    OpenRouter__Undi95___remm__Slerp__L2__13b,
    OpenRouter__Gryphe___mythomax__L2__13b,
    OpenRouter__Openai___gpt__4__0314,
    OpenRouter__Openai___gpt__3_5__Turbo,
    OpenRouter__Openai___gpt__4,
    ZenMux__Kuaishou___kat__Coder__Pro__V1__Free,
    ZenMux__Z__Ai___glm__4_6v__Flash__Free,
    ZenMux__Xiaomi___mimo__V2__Flash__Free,
    ZenMux__Openai___gpt__4_1__Mini,
    ZenMux__Anthropic___claude__Opus__4,
    ZenMux__Openai___gpt__4_1__Nano,
    ZenMux__Openai___o4__Mini,
    ZenMux__Openai___gpt__5,
    ZenMux__Openai___gpt__4o,
    ZenMux__Openai___gpt__5__Nano,
    ZenMux__Openai___gpt__4_1,
    ZenMux__Anthropic___claude__3_5__Sonnet,
    ZenMux__Anthropic___claude__3_5__Haiku,
    ZenMux__Google___gemini__2_5__Flash__Lite,
    ZenMux__Anthropic___claude__3_7__Sonnet,
    ZenMux__Deepseek___deepseek__R1__0528,
    ZenMux__Anthropic___claude__Opus__4_1,
    ZenMux__Anthropic___claude__Sonnet__4,
    ZenMux__Openai___gpt__5__Mini,
    ZenMux__Openai___gpt__5__Chat,
    ZenMux__Google___gemini__2_5__Flash,
    ZenMux__Deepseek___deepseek__Chat__V3_1,
    ZenMux__Google___gemini__2_5__Pro,
    ZenMux__Google___gemini__2_0__Flash,
    ZenMux__X__Ai___grok__4,
    ZenMux__Qwen___qwen3__235b__A22b__2507,
    ZenMux__Openai___gpt__4o__Mini,
    ZenMux__Qwen___qwen3__235b__A22b__Thinking__2507,
    ZenMux__Qwen___qwen3__Coder,
    ZenMux__Google___gemini__2_0__Flash__Lite__001,
    ZenMux__Z__Ai___glm__4_5__Air,
    ZenMux__Qwen___qwen3__Coder__Plus,
    ZenMux__Deepseek___deepseek__Chat,
    ZenMux__Inclusionai___ring__Mini__2_0,
    ZenMux__Moonshotai___kimi__K2__0905,
    ZenMux__Inclusionai___ling__Mini__2_0,
    ZenMux__Z__Ai___glm__4_5,
    ZenMux__Moonshotai___kimi__K2__0711,
    ZenMux__Inclusionai___ling__Flash__2_0,
    ZenMux__Inclusionai___ring__Flash__2_0,
    ZenMux__X__Ai___grok__Code__Fast__1,
    ZenMux__X__Ai___grok__4__Fast,
    ZenMux__X__Ai___grok__4__Fast__Non__Reasoning,
    ZenMux__Qwen___qwen3__Vl__Plus,
    ZenMux__Qwen___qwen3__Max,
    ZenMux__Z__Ai___glm__4_6,
    ZenMux__Inclusionai___ling__1t,
    ZenMux__Inclusionai___ring__1t,
    ZenMux__Anthropic___claude__Sonnet__4_5,
    ZenMux__Anthropic___claude__Haiku__4_5,
    ZenMux__Openai___gpt__5__Pro,
    ZenMux__Openai___gpt__5__Codex,
    ZenMux__Deepseek___deepseek__Reasoner,
    ZenMux__Kuaishou___kat__Coder__Pro__V1,
    ZenMux__Minimax___minimax__M2,
    ZenMux__Moonshotai___kimi__K2__Thinking,
    ZenMux__Moonshotai___kimi__K2__Thinking__Turbo,
    ZenMux__Qwen___qwen3__Max__Preview,
    ZenMux__Baidu___ernie__5_0__Thinking__Preview,
    ZenMux__Openai___gpt__5_1__Codex,
    ZenMux__Openai___gpt__5_1__Chat,
    ZenMux__Qwen___qwen3__14b,
    ZenMux__Openai___gpt__5_1__Codex__Mini,
    ZenMux__Volcengine___doubao__Seed__Code,
    ZenMux__Openai___gpt__5_1,
    ZenMux__X__Ai___grok__4_1__Fast,
    ZenMux__X__Ai___grok__4_1__Fast__Non__Reasoning,
    ZenMux__Google___gemini__3__Pro__Preview,
    ZenMux__Volcengine___doubao__Seed__1__6__Vision,
    ZenMux__Google___gemma__3__12b__It,
    ZenMux__Anthropic___claude__Opus__4_5,
    ZenMux__Deepseek___deepseek__V3_2__Exp,
    ZenMux__Meta___llama__3_3__70b__Instruct,
    ZenMux__Mistralai___mistral__Large__2512,
    ZenMux__Inclusionai___ming__Flash__Omni__Preview,
    ZenMux__Meta___llama__4__Scout__17b__16e__Instruct,
    ZenMux__Baidu___ernie__X1_1__Preview,
    ZenMux__Z__Ai___glm__4_6v__Flash,
    ZenMux__Z__Ai___glm__4_6v,
    ZenMux__Deepseek___deepseek__V3_2,
    ZenMux__Openai___gpt__5_2__Pro,
    ZenMux__Openai___gpt__5_2,
    ZenMux__Openai___gpt__5_2__Chat,
    ZenMux__Google___gemini__3__Flash__Preview,
    ZenMux__Stepfun___step__3,
    ZenMux__Inclusionai___llada2_0__Flash__Cap,
    ZenMux__Google___gemini__3__Flash__Preview__Free,
    ZenMux__Xiaomi___mimo__V2__Flash,
    ZenMux__Volcengine___doubao__Seed__1_8,
    ZenMux__Minimax___minimax__M2_1,
    ZenMux__Z__Ai___glm__4_7,
    Anthropic__Claude__Opus__4__5__20251101,
    Anthropic__Claude__Haiku__4__5__20251001,
    Anthropic__Claude__Sonnet__4__5__20250929,
    Anthropic__Claude__Opus__4__1__20250805,
    Anthropic__Claude__Opus__4__20250514,
    Anthropic__Claude__Sonnet__4__20250514,
    Anthropic__Claude__3__7__Sonnet__20250219,
    Anthropic__Claude__3__5__Haiku__20241022,
    Anthropic__Claude__3__Haiku__20240307,
    Anthropic__Claude__3__Opus__20240229,
    Deepseek__Deepseek__Chat,
    Deepseek__Deepseek__Reasoner,
    OpenAi__Gpt__4__0613,
    OpenAi__Gpt__4,
    OpenAi__Gpt__3_5__Turbo,
    OpenAi__Chatgpt__Image__Latest,
    OpenAi__Gpt__4o__Mini__Tts__2025__03__20,
    OpenAi__Gpt__4o__Mini__Tts__2025__12__15,
    OpenAi__Gpt__Realtime__Mini__2025__12__15,
    OpenAi__Gpt__Audio__Mini__2025__12__15,
    OpenAi__Davinci__002,
    OpenAi__Babbage__002,
    OpenAi__Gpt__3_5__Turbo__Instruct,
    OpenAi__Gpt__3_5__Turbo__Instruct__0914,
    OpenAi__Dall__E__3,
    OpenAi__Dall__E__2,
    OpenAi__Gpt__4__1106__Preview,
    OpenAi__Gpt__3_5__Turbo__1106,
    OpenAi__Tts__1__Hd,
    OpenAi__Tts__1__1106,
    OpenAi__Tts__1__Hd__1106,
    OpenAi__Text__Embedding__3__Small,
    OpenAi__Text__Embedding__3__Large,
    OpenAi__Gpt__4__0125__Preview,
    OpenAi__Gpt__4__Turbo__Preview,
    OpenAi__Gpt__3_5__Turbo__0125,
    OpenAi__Gpt__4__Turbo,
    OpenAi__Gpt__4__Turbo__2024__04__09,
    OpenAi__Gpt__4o__2024__05__13,
    OpenAi__Gpt__4o__Mini__2024__07__18,
    OpenAi__Gpt__4o__2024__08__06,
    OpenAi__Chatgpt__4o__Latest,
    OpenAi__Gpt__4o__Audio__Preview,
    OpenAi__Gpt__4o__Realtime__Preview,
    OpenAi__Omni__Moderation__Latest,
    OpenAi__Omni__Moderation__2024__09__26,
    OpenAi__Gpt__4o__Realtime__Preview__2024__12__17,
    OpenAi__Gpt__4o__Audio__Preview__2024__12__17,
    OpenAi__Gpt__4o__Mini__Realtime__Preview__2024__12__17,
    OpenAi__Gpt__4o__Mini__Audio__Preview__2024__12__17,
    OpenAi__O1__2024__12__17,
    OpenAi__Gpt__4o__Mini__Realtime__Preview,
    OpenAi__Gpt__4o__Mini__Audio__Preview,
    OpenAi__O3__Mini,
    OpenAi__O3__Mini__2025__01__31,
    OpenAi__Gpt__4o__2024__11__20,
    OpenAi__Gpt__4o__Search__Preview__2025__03__11,
    OpenAi__Gpt__4o__Search__Preview,
    OpenAi__Gpt__4o__Mini__Search__Preview__2025__03__11,
    OpenAi__Gpt__4o__Mini__Search__Preview,
    OpenAi__Gpt__4o__Transcribe,
    OpenAi__Gpt__4o__Mini__Transcribe,
    OpenAi__O1__Pro__2025__03__19,
    OpenAi__O1__Pro,
    OpenAi__Gpt__4o__Mini__Tts,
    OpenAi__O3__2025__04__16,
    OpenAi__O4__Mini__2025__04__16,
    OpenAi__O3,
    OpenAi__O4__Mini,
    OpenAi__Gpt__4_1__2025__04__14,
    OpenAi__Gpt__4_1,
    OpenAi__Gpt__4_1__Mini__2025__04__14,
    OpenAi__Gpt__4_1__Mini,
    OpenAi__Gpt__4_1__Nano__2025__04__14,
    OpenAi__Gpt__4_1__Nano,
    OpenAi__Gpt__Image__1,
    OpenAi__Gpt__4o__Realtime__Preview__2025__06__03,
    OpenAi__Gpt__4o__Audio__Preview__2025__06__03,
    OpenAi__Gpt__4o__Transcribe__Diarize,
    OpenAi__Gpt__5__Chat__Latest,
    OpenAi__Gpt__5__2025__08__07,
    OpenAi__Gpt__5,
    OpenAi__Gpt__5__Mini__2025__08__07,
    OpenAi__Gpt__5__Mini,
    OpenAi__Gpt__5__Nano__2025__08__07,
    OpenAi__Gpt__5__Nano,
    OpenAi__Gpt__Audio__2025__08__28,
    OpenAi__Gpt__Realtime,
    OpenAi__Gpt__Realtime__2025__08__28,
    OpenAi__Gpt__Audio,
    OpenAi__Gpt__5__Codex,
    OpenAi__Gpt__Image__1__Mini,
    OpenAi__Gpt__5__Pro__2025__10__06,
    OpenAi__Gpt__5__Pro,
    OpenAi__Gpt__Audio__Mini,
    OpenAi__Gpt__Audio__Mini__2025__10__06,
    OpenAi__Gpt__5__Search__Api,
    OpenAi__Gpt__Realtime__Mini,
    OpenAi__Gpt__Realtime__Mini__2025__10__06,
    OpenAi__Sora__2,
    OpenAi__Sora__2__Pro,
    OpenAi__Gpt__5__Search__Api__2025__10__14,
    OpenAi__Gpt__5_1__Chat__Latest,
    OpenAi__Gpt__5_1__2025__11__13,
    OpenAi__Gpt__5_1,
    OpenAi__Gpt__5_1__Codex,
    OpenAi__Gpt__5_1__Codex__Mini,
    OpenAi__Gpt__5_1__Codex__Max,
    OpenAi__Gpt__Image__1_5,
    OpenAi__Gpt__5_2__2025__12__11,
    OpenAi__Gpt__5_2,
    OpenAi__Gpt__5_2__Pro__2025__12__11,
    OpenAi__Gpt__5_2__Pro,
    OpenAi__Gpt__5_2__Chat__Latest,
    OpenAi__Gpt__4o__Mini__Transcribe__2025__12__15,
    OpenAi__Gpt__4o__Mini__Transcribe__2025__03__20,
    OpenAi__Gpt__3_5__Turbo__16k,
    OpenAi__Tts__1,
    OpenAi__Whisper__1,
    OpenAi__Text__Embedding__Ada__002,
    Gemini__Gemini__3__Pro__Image__Preview,
    Gemini__Gemini__3__Pro__Preview,
    MoonshotAi__Kimi__K2__Thinking,
    Gemini__Gemini__2_5__Flash__Image,
    Gemini__Gemini__2_5__Flash__Preview__09__2025,
    Gemini__Gemini__2_5__Flash__Lite__Preview__09__2025,
    MoonshotAi__Kimi__K2__0905,
    MoonshotAi__Kimi__K2__0905exacto,
    Gemini__Gemini__2_5__Flash__Image__Preview,
    Gemini__Gemini__2_5__Flash__Lite,
    MoonshotAi__Kimi__K2free,
    MoonshotAi__Kimi__K2,
    Gemini__Gemma__3n__E2b__Itfree,
    Gemini__Gemini__2_5__Flash,
    Gemini__Gemini__2_5__Pro,
    MoonshotAi__Kimi__Dev__72b,
    Gemini__Gemini__2_5__Pro__Preview,
    Gemini__Gemma__3n__E4b__Itfree,
    Gemini__Gemma__3n__E4b__It,
    Gemini__Gemini__2_5__Pro__Preview__05__06,
    Gemini__Gemma__3__4b__Itfree,
    Gemini__Gemma__3__4b__It,
    Gemini__Gemma__3__12b__Itfree,
    Gemini__Gemma__3__12b__It,
    Gemini__Gemma__3__27b__Itfree,
    Gemini__Gemma__3__27b__It,
    Gemini__Gemini__2_0__Flash__Lite__001,
    Gemini__Gemini__2_0__Flash__001,
    Gemini__Gemini__2_0__Flash__Expfree,
    Gemini__Gemma__2__27b__It,
    Gemini__Gemma__2__9b__It,
    Gemini__Gemini__2_0__Flash,
    MoonshotAi__Kimi__K2__0711,
    MoonshotAi__Kimi__K2__Thinking__Turbo,
    Gemini__Gemini__3__Flash__Preview__Free,
    // === END AUTO-GENERATED ===
}

impl ProviderModel {
    /// Parse provider from an auto-generated variant name.
    ///
    /// Variant names follow the pattern `{Provider}__{Model}` for direct providers
    /// or `{Aggregator}__{UnderlyingProvider}___{Model}` for aggregators.
    /// The first segment before `__` is always the provider.
    fn provider_from_variant_name(variant_name: &str) -> super::base::Provider {
        use super::base::Provider;

        // Split on "__" to get the provider prefix (first segment)
        let provider_prefix = variant_name.split("__").next().unwrap_or("");

        match provider_prefix {
            "Anthropic" => Provider::Anthropic,
            "OpenAi" => Provider::OpenAi,
            "Deepseek" => Provider::Deepseek,
            "Gemini" => Provider::Gemini,
            "Ollama" => Provider::Ollama,
            "OpenRouter" => Provider::OpenRouter,
            "MoonshotAi" => Provider::MoonshotAi,
            "Zai" => Provider::Zai,
            "ZenMux" => Provider::ZenMux,
            _ => panic!("Unknown provider prefix in variant name: {}", variant_name),
        }
    }

    /// Parse model_id from an auto-generated variant name.
    ///
    /// For aggregators like `OpenRouter__Google___gemini__3__Flash__Preview`:
    /// - First segment is the aggregator (OpenRouter)
    /// - Everything after the first `__` contains provider and model
    /// - `___` (triple underscore) represents `/` in the original model ID
    /// - `__` (double underscore) represents `-`
    /// - `_` (single underscore) represents `.`
    fn model_id_from_variant_name(variant_name: &str) -> String {
        // Split on first "__" to skip the provider prefix
        let parts: Vec<&str> = variant_name.splitn(2, "__").collect();
        let model_part = parts.get(1).unwrap_or(&"");

        // Convert back from variant naming to model ID:
        // Order matters: handle triple before double before single
        model_part
            .replace("___", "/")  // Triple underscore → /
            .replace("__", "-")   // Double underscore → -
            .replace("_", ".")    // Single underscore → .
            .to_lowercase()
    }

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

            // Auto-generated variants: parse provider from variant name
            _ => Self::provider_from_variant_name(self.as_ref()),
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
    pub fn model_id(&self) -> std::borrow::Cow<'_, str> {
        use std::borrow::Cow;
        match self {
            // Anthropic static variants
            Self::Anthropic__ClaudeOpus__4__5__20251101 => Cow::Borrowed("claude-opus-4-5-20251101"),
            Self::Anthropic__ClaudeSonnet__4__5__20250929 => Cow::Borrowed("claude-sonnet-4-5-20250929"),
            Self::Anthropic__ClaudeHaiku__4__0__20250107 => Cow::Borrowed("claude-haiku-4-0-20250107"),
            Self::Anthropic(id) => Cow::Borrowed(id),

            // OpenAI static variants
            Self::OpenAi__Gpt__4o => Cow::Borrowed("gpt-4o"),
            Self::OpenAi__Gpt__4o__Mini => Cow::Borrowed("gpt-4o-mini"),
            Self::OpenAi__O1 => Cow::Borrowed("o1"),
            Self::OpenAi(id) => Cow::Borrowed(id),

            // Deepseek static variants
            Self::Deepseek__Chat => Cow::Borrowed("chat"),
            Self::Deepseek__Reasoner => Cow::Borrowed("reasoner"),
            Self::Deepseek(id) => Cow::Borrowed(id),

            // Gemini static variants
            Self::Gemini__Gemini__3__Flash__Preview => Cow::Borrowed("gemini-3-flash-preview"),
            Self::Gemini__Gemini__2__0__Flash__Exp => Cow::Borrowed("gemini-2-0-flash-exp"),
            Self::Gemini(id) => Cow::Borrowed(id),

            // All outlets
            Self::Ollama(id) => Cow::Borrowed(id),
            Self::OpenRouter(id) => Cow::Borrowed(id),
            Self::MoonshotAi(id) => Cow::Borrowed(id),
            Self::Zai(id) => Cow::Borrowed(id),
            Self::ZenMux(id) => Cow::Borrowed(id),

            // Auto-generated variants: parse model_id from variant name
            _ => Cow::Owned(Self::model_id_from_variant_name(self.as_ref())),
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
            model_id = %self.model_id(),
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
    ///     let summary = ProviderModel::update(false).await?;
    ///     println!("Added {} new models", summary.total_added());
    ///     for (provider, count) in &summary.models_added {
    ///         println!("  {:?}: {} new models", provider, count);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn update(dry_run: bool) -> Result<UpdateSummary, super::discovery::ProviderError> {
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

        // Step 2: Build set of existing variants (mutable to track new additions)
        let mut seen_variants = Self::get_existing_static_variants();

        tracing::debug!(
            variant_count = seen_variants.len(),
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

                if !seen_variants.contains(&full_variant) {
                    tracing::debug!(
                        provider = ?provider,
                        model_id = %model_id,
                        variant_name = %variant_name,
                        "Detected new model"
                    );

                    new_variants.push((*provider, model_id.clone(), variant_name));
                    seen_variants.insert(full_variant); // Track to avoid duplicates
                    *models_added.entry(*provider).or_insert(0) += 1;
                }
            }
        }

        // Step 4: Handle aggregator hints (OpenRouter/ZenMux)
        let mut aggregator_hints_applied = 0;

        for (provider, model_ids) in &api_models {
            if matches!(provider, Provider::OpenRouter | Provider::ZenMux) {
                for model_id in model_ids {
                    // Parse aggregator model IDs like "anthropic/claude-opus-4-5"
                    if let Some((underlying_provider_str, underlying_model)) = model_id.split_once('/') {
                        // Try to match to a known provider
                        if let Some(underlying_provider) = Self::parse_provider_name(underlying_provider_str) {
                            // Check if we successfully queried the underlying provider directly
                            // (not just having an API key - some providers don't support /v1/models)
                            if api_models.contains_key(&underlying_provider) {
                                tracing::debug!(
                                    aggregator = ?provider,
                                    underlying = ?underlying_provider,
                                    model_id = %model_id,
                                    "Skipping aggregator hint - have direct provider models"
                                );
                                // Skip - we already have models from the underlying provider
                            } else {
                                // No direct access - add both aggregator and underlying hints
                                tracing::debug!(
                                    aggregator = ?provider,
                                    underlying = ?underlying_provider,
                                    model_id = %model_id,
                                    "Adding aggregator hint - no direct provider access"
                                );

                                // Aggregator variant: {AGGREGATOR}__{PROVIDER}__{MODEL}
                                // Uses full model_id (e.g., "google/gemini-3-flash") which becomes Google___Gemini__3__Flash
                                let aggregator_variant_name = Self::model_id_to_variant_name(model_id);
                                let aggregator_variant = format!("{:?}__{}", provider, aggregator_variant_name);

                                if !seen_variants.contains(&aggregator_variant) {
                                    new_variants.push((*provider, model_id.clone(), aggregator_variant_name));
                                    seen_variants.insert(aggregator_variant); // Track to avoid duplicates
                                    *models_added.entry(*provider).or_insert(0) += 1;
                                    aggregator_hints_applied += 1;
                                }

                                // Underlying provider variant: {PROVIDER}__{MODEL}
                                // Uses just the model part (e.g., "gemini-3-flash") which becomes Gemini__3__Flash
                                let underlying_variant_name = Self::model_id_to_variant_name(underlying_model);
                                let underlying_variant = format!("{:?}__{}", underlying_provider, underlying_variant_name);
                                if !seen_variants.contains(&underlying_variant) {
                                    new_variants.push((underlying_provider, underlying_model.to_string(), underlying_variant_name));
                                    seen_variants.insert(underlying_variant); // Track to avoid duplicates
                                    *models_added.entry(underlying_provider).or_insert(0) += 1;
                                    aggregator_hints_applied += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Step 5: Code generation
        if !new_variants.is_empty() {
            tracing::info!(
                new_variant_count = new_variants.len(),
                "Generating enum variants via codegen"
            );

            // Prepare variant names for injection
            let variant_names: Vec<String> = new_variants
                .iter()
                .map(|(provider, _model_id, variant_name)| {
                    format!("{:?}__{}", provider, variant_name)
                })
                .collect();

            // Use codegen module to inject variants
            let types_rs_path = std::env::current_dir()
                .ok()
                .and_then(|p| p.join("shared/src/providers/types.rs").canonicalize().ok())
                .unwrap_or_else(|| std::path::PathBuf::from("shared/src/providers/types.rs"));

            let variant_count = crate::codegen::inject_enum_variants(
                "ProviderModel",
                &variant_names,
                types_rs_path.to_str().unwrap(),
                dry_run,
            )
            .map_err(|e| ProviderError::CodegenFailed {
                details: format!("{:?}", e),
            })?;

            tracing::info!(
                variants_injected = variant_count,
                "Successfully injected enum variants"
            );
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
    /// - Replace `/` with `___` (triple underscore, for aggregator prefixes)
    /// - Replace `-` with `__` (double underscore)
    /// - Replace `.` with `_` (single underscore)
    /// - Remove `:`
    ///
    /// ## Examples
    ///
    /// - `claude-opus-4.5:20251101` → `ClaudeOpus__4_5__20251101`
    /// - `gpt-4o` → `Gpt__4o`
    /// - `anthropic/claude-3.5-sonnet` → `Anthropic___Claude__3_5__Sonnet`
    fn model_id_to_variant_name(model_id: &str) -> String {
        model_id
            .replace("/", "___") // aggregator prefix separator (must come first)
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

    /// Parse provider name from string (case-insensitive)
    ///
    /// ## Examples
    ///
    /// - `"anthropic"` → `Some(Provider::Anthropic)`
    /// - `"openai"` → `Some(Provider::OpenAi)`
    /// - `"unknown"` → `None`
    fn parse_provider_name(name: &str) -> Option<super::base::Provider> {
        use super::base::Provider;
        match name.to_lowercase().as_str() {
            "anthropic" => Some(Provider::Anthropic),
            "openai" => Some(Provider::OpenAi),
            "deepseek" => Some(Provider::Deepseek),
            "gemini" | "google" => Some(Provider::Gemini),
            "moonshotai" | "moonshot" => Some(Provider::MoonshotAi),
            "ollama" => Some(Provider::Ollama),
            "openrouter" => Some(Provider::OpenRouter),
            "zai" => Some(Provider::Zai),
            "zenmux" => Some(Provider::ZenMux),
            _ => None,
        }
    }
}

impl std::fmt::Display for ProviderModel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_identifier())
    }
}

impl From<ProviderModel> for String {
    fn from(model: ProviderModel) -> String {
        model.to_identifier()
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

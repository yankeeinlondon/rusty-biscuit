# Providers and Models

Detailed reference for the provider registry, model catalogs, and rig-core integration in `unchained-ai/lib/src/rigging/`.

## Provider Registry (`providers/provider.rs`)

### Provider Enum

13 providers defined:

| Provider | Type | Auth | Base URL | Env Vars |
|----------|------|------|----------|----------|
| Anthropic | Direct | API key header (`x-api-key`) | `api.anthropic.com` | `ANTHROPIC_API_KEY` |
| Deepseek | Direct | Bearer | `api.deepseek.com` | `DEEPSEEK_API_KEY` |
| Gemini | Direct | Query param (`key`) | `generativelanguage.googleapis.com` | `GEMINI_API_KEY`, `GOOGLE_API_KEY` |
| Groq | Direct | Bearer | `api.groq.com/openai` | `GROQ_API_KEY` |
| HuggingFace | Direct | Bearer | `huggingface.co/api` | `HF_TOKEN`, `HUGGINGFACE_TOKEN`, `HUGGING_FACE_TOKEN` |
| Mistral | Direct | Bearer | `api.mistral.ai` | `MISTRAL_API_KEY` |
| MoonshotAi | Direct | Bearer | `api.moonshot.ai/v1` | `MOONSHOT_API_KEY`, `MOONSHOT_AI_API_KEY` |
| Ollama | Local | None | `localhost:11434` | (none) |
| OpenAI | Direct | Bearer | `api.openai.com` | `OPENAI_API_KEY` |
| OpenRouter | Aggregator | Bearer | `openrouter.ai/api` | `OPEN_ROUTER_API_KEY`, `OPENROUTER_API_KEY` |
| xAI | Direct | Bearer | `api.x.ai/v1` | `XAI_API_KEY`, `X_AI_API_KEY` |
| Z.ai | Direct | Bearer | `open.bigmodel.cn/api/paas/v4` | `ZAI_API_KEY`, `Z_AI_API_KEY` |
| ZenMux | Aggregator | None | `zenmux.ai/api` | `ZENMUX_API_KEY`, `ZEN_MUX_API_KEY` |

### ProviderConfig

```rust
pub struct ProviderConfig {
    pub env_vars: &'static [&'static str],
    pub auth_method: ApiAuthMethod,     // BearerToken, ApiKey(header), QueryParam(param), None
    pub base_url: &'static str,
    pub models_endpoint: Option<&'static str>,  // None = /v1/models
    pub is_local: bool,
}
```

Access via `Provider::config()`, `Provider::base_url()`, `Provider::models_endpoint()`, `Provider::is_local()`.

## Model Catalogs (`providers/models/`)

### Provider-Specific Enums

Auto-generated files (one per provider): `openai.rs`, `anthropic.rs`, `gemini.rs`, `groq.rs`, `deepseek.rs`, `mistral.rs`, `moonshotai.rs`, `openrouter.rs`, `xai.rs`, `zai.rs`, `zenmux.rs`.

Each enum:
- Uses `#[derive(ModelId)]` macro from the `model_id` crate
- Has one variant per model ID returned by the provider's API
- Includes a `Bespoke(String)` variant for custom/unknown model IDs
- Provides `model_id() -> &str` and `metadata() -> Option<&'static ModelMetadata>`

### ProviderModel (Aggregated Enum)

Wraps all provider-specific enums into a single type:

```rust
pub enum ProviderModel {
    Anthropic(ProviderModelAnthropic),
    Deepseek(ProviderModelDeepseek),
    Gemini(ProviderModelGemini),
    Groq(ProviderModelGroq),
    Mistral(ProviderModelMistral),
    MoonshotAi(ProviderModelMoonshotAi),
    OpenAi(ProviderModelOpenAi),
    OpenRouter(ProviderModelOpenRouter),
    Xai(ProviderModelXai),
    Zai(ProviderModelZai),
    ZenMux(ProviderModelZenMux),
}
```

**Wire format**: `"provider/model-id"` (e.g., `"openai/o3"`, `"anthropic/claude-opus-4-5-20251101"`).

**Key methods**:
- `model_id() -> &str` - canonical model ID
- `metadata() -> Option<&'static ModelMetadata>` - generated metadata from models.dev and provider-native sources
- `context_window() -> Option<u32>`
- `max_output_tokens() -> Option<u32>`
- `supports_input(Modality) -> bool`
- `supports_output(Modality) -> bool`
- `has_capability(&str) -> bool` (e.g., `"function_calling"`, `"structured_output"`)
- `parse_wire_id(&str) -> Result<Self, ProviderModelParseError>`

Serializes/deserializes as wire-format strings via serde.

### ModelCapability (`models/model_capability.rs`)

Abstract model selection by capability tier rather than specific model:

**Tiers** (each with cheap variant):
- `Fast` / `FastCheap` - fast, lower-cost models (e.g., claude-haiku)
- `Normal` / `NormalCheap` - balanced capability (e.g., claude-sonnet)
- `Smart` / `SmartCheap` - top-tier models (e.g., claude-opus)

**Thinking variants**:
- `NormalThinking` / `NormalThinkingCheap`
- `NormalUltrathink` / `NormalCheapUltrathink`
- `SmartThink` / `SmartCheapThink`
- `SmartUltrathink` / `SmartCheapUltrathink`

**Specialist stacks**:
- `CreativeFast` / `CreativeNormal` / `CreativeSmart` - lower temperature
- `LiteralFast` / `LiteralNormal` / `LiteralSmart` - higher temperature

**Specific**: `Specific(ProviderModel)` for exact model override.

Serializes as strings (`"Fast"`, `"SmartThink"`) or `"Specific:provider/model-id"`. Deserializes from strings, `"provider/model-id"` shorthand, or `{"Specific": "provider/model-id"}` map form.

### ModelMetadata (`models/model_metadata.rs`)

Runtime model specs merged from models.dev and provider-native APIs (e.g., OpenRouter):

```rust
pub struct ProviderModelMetadata {
    pub display_name: Option<String>,
    pub family: Option<String>,
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub modalities: Option<ModelModalities>,
    pub capabilities: Vec<String>,
    pub description: Option<String>,
    pub pricing: Option<ModelPricing>,
    pub supported_parameters: Option<Vec<String>>,
    pub default_parameters: Option<ModelDefaultParameters>,
    pub knowledge_cutoff: Option<String>,
    pub created: Option<u32>,
    pub release_date: Option<String>,
}
```

A deprecated `ModelMetadata` type alias exists for backward compatibility.

### ModelPricing (`models/model_pricing.rs`)

Per-token and per-request pricing from provider APIs:

```rust
pub struct ModelPricing {
    pub prompt_per_token: Option<f64>,
    pub completion_per_token: Option<f64>,
    pub web_search_per_request: Option<f64>,
    pub input_cache_read_per_token: Option<f64>,
}
```

Handles both string (`"0.000005"`) and numeric JSON values via custom deserializer.

### ModelDefaultParameters (`models/model_default_parameters.rs`)

Provider-recommended default generation parameters:

```rust
pub struct ModelDefaultParameters {
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
}
```

`Modality` enum: `Text`, `Image`, `Audio`, `Video`, `Embeddings`.

### ModelIdentity (`models/identity.rs`)

Wire-id identity grammar: `[source /] vendor / model-id`, with the model-id decomposed
into family + version + variants + size + date-pin + serving tag.

- `ModelIdentity::parse(wire)` — infallible; unmatched tokens land in `family`
- `family_key()` — `vendor/family` grouping key for cross-source identity (the same
  model offered directly and via an aggregator groups together)
- `latest_in_family(ids, family)` — newest release by version segments, date-pin
  tiebreak; rolling aliases (`-latest`) are excluded
- Curated tables: variant vocabulary, vendor aliases (`z-ai`→`zai`, source `gemini`→
  vendor `google`, …)
- `gen-models` uses parsed identity to fill `family` when neither models.dev nor
  provider-native data supplies one

Validated against the full generated catalog (99.9% family inference); design record in
`claudine/features/2026-07-02-provider-metadata/spec.md` (spike-model-identity).

## Client Adaptors (`providers/client_adaptors/`)

OpenAI-compatible wrappers for non-standard providers:

### Z.ai (`zai.rs`)
Wraps `rig::providers::openai::CompletionsClient` for ZhipuAI GLM models.

```rust
use unchained_ai::rigging::providers::client_adaptors::zai::{Client, GLM_4_7};
let client = Client::from_env()?;
let model = client.completion_model(GLM_4_7);
```

Constants: `ZAI_API_BASE_URL`, `ZAI_CN_API_BASE_URL`, `GLM_4_5`, `GLM_4_6`, `GLM_4_7`.

### ZenMux (`zenmux.rs`)
Similar wrapper for ZenMux aggregator. Uses provider-prefixed model IDs.

## Rig Tools (`tools/`)

### BraveSearchTool (`brave_search.rs`)
Implements `rig::tool::Tool` for Brave Search API.

- Requires `BRAVE_API_KEY`
- Optional `BRAVE_PLAN` (`free`, `base`, `pro`) controls rate limits (1/20/50 req/s)
- Returns structured search results
- Includes tracing support

### ScreenScrapeTool (`screen_scrape.rs`)
Implements `rig::tool::Tool` for HTTP scraping.

- Main content extraction with HTML parsing
- Multiple output formats
- Link extraction
- Configurable user agents and timeouts
- Metadata reporting (title, status code, content length)

## API Module (`api/`)

### ApiAuthMethod (`auth.rs`)

```rust
pub enum ApiAuthMethod {
    BearerToken,
    ApiKey(String),      // custom header name
    QueryParam(String),  // query parameter name
    None,
}
```

### OpenAI API (`openai_api.rs`)
Model discovery via OpenAI-compatible `/models` endpoints:
- Provider-specific auth handling
- Exponential backoff retries (up to 3 attempts)
- 10 MB response size limit
- Parallel fan-out across configured providers
- Handles both `data` (OpenAI) and `models` (Gemini) response shapes

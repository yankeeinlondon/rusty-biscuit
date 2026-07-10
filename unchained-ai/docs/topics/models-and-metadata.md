# Model Metadata System

This document describes how `unchained-ai` collects, synthesizes, stores, and presents metadata about Large Language Models (LLMs).

## Goals and Intent

The model metadata system exists to answer a simple question at runtime: **"What can this model do, and what does it cost?"**

Specifically, we want to know:

- **Capabilities**: Does it support vision? Function calling? Structured output?
- **Capacity**: What is its context window? Max output tokens?
- **Modalities**: Does it accept images, audio, or video as input? What can it output?
- **Pricing**: How much does it cost per token (or per request)?
- **Parameters**: What generation parameters does it support? What are the defaults?
- **Identity**: What family does it belong to? What is its display name? Description?

This metadata is used in three ways:

1. **Code generation**: At build time, `unchained-ai-gen` queries provider APIs and external sources to generate Rust enums and a static metadata lookup table.
2. **Runtime validation**: The library uses metadata to validate provider capabilities, filter models by feature, and inform pipeline construction.
3. **User inspection**: The CLI (`unchained models` and `unchained model <model>`) lets users browse and inspect this metadata.

## Metadata Sources

### 1. Provider Native APIs

At code-generation time, `unchained-ai-gen` queries the `/v1/models` endpoint (or equivalent) for every configured provider that has an API key. This includes:

- OpenAI
- Anthropic
- Google Gemini
- Groq
- Mistral
- Moonshot AI
- DeepSeek
- OpenRouter
- xAI
- Z.ai
- ZenMux

**The asymmetry**: Most providers return minimal metadata — typically just `id`, `created`, and `owned_by`. **Only OpenRouter returns rich metadata**, including pricing, architecture/modalities, supported parameters, default parameters, descriptions, and context windows.

Because of this, the generator intentionally discards native metadata from all providers **except** OpenRouter:

```rust
// unchained-ai/gen/src/main.rs
if provider == Provider::OpenRouter {
    provider_native_raw.extend(result.raw_metadata);
}
```

This means direct-provider models (e.g., `o3` from OpenAI) get **no pricing, no description, and no parameter metadata** from their native APIs.

### 2. models.dev

The generator fetches specifications from `https://models.dev/api.json` at generation time. This external source provides:

- `display_name`
- `family`
- `context_window`
- `max_output_tokens`
- `modalities` (input/output)
- `capabilities`
- `pricing` (per-million token costs converted to per-token USD)
- `knowledge_cutoff`
- `release_date`

models.dev entries are bucketed by provider key. The generator maps local provider names into those buckets (`gemini -> google`, `x-ai -> xai`, `z-ai -> zai`) and matches only within the mapped bucket. `ollama` and `zenmux` have no models.dev bucket.

### 3. model-citizen (Local Models)

The `model-citizen` crate in this monorepo scans local model runners (Ollama, LM Studio, Llama.cpp) for installed GGUF models. It tracks file parameters, quantization, architecture detection, and GGUF metadata.

**This source is completely separate from the unchained-ai metadata pipeline.** It has no pricing information and no awareness of API providers.

## Metadata Synthesis

### Merge Priority

For each model ID discovered from provider APIs, the generator attempts to merge metadata from two sources:

**Priority: Provider-Native > models.dev**

```rust
// unchained-ai/gen/src/metadata_generator.rs
pub fn merge_metadata(
    provider_native: Option<ProviderModelMetadata>,
    enrichment: Option<ProviderModelMetadata>,
) -> Option<ProviderModelMetadata> {
    match (provider_native, enrichment) {
        (None, None) => None,
        (Some(native), None) => Some(native),
        (None, Some(enrichment)) => Some(enrichment),
        (Some(native), Some(enrichment)) => Some(merge_native_with_enrichment(native, enrichment)),
    }
}
```

When both sources provide the same field (e.g., `context_window`), the provider-native value wins. Fields only available from one source are used as-is.

### The Merge Algorithm

```rust
fn merge_native_with_enrichment(native, enrichment) -> ProviderModelMetadata {
    ProviderModelMetadata {
        display_name: native.display_name.or(enrichment.display_name),
        family: native.family.or(enrichment.family),
        context_window: native.context_window.or(enrichment.context_window),
        max_output_tokens: native.max_output_tokens.or(enrichment.max_output_tokens),
        modalities: native.modalities.or(enrichment.modalities),
        capabilities: if native.capabilities.is_empty() { enrichment.capabilities } else { native.capabilities },
        pricing: native.pricing.or(enrichment.pricing),
        knowledge_cutoff: native.knowledge_cutoff.or(enrichment.knowledge_cutoff),
        release_date: enrichment.release_date,
        // The following are provider-native only today:
        description: native.description,
        supported_parameters: native.supported_parameters,
        default_parameters: native.default_parameters,
        created: native.created,
    }
}
```

### Storage: The Global Lookup Table

All merged metadata is stored in a single static HashMap:

```rust
// unchained-ai/lib/src/rigging/providers/models/metadata_generated.rs
pub static MODEL_METADATA: LazyLock<HashMap<&'static str, ModelMetadata>> = ...;
```

**This table is keyed exclusively by model ID string** (e.g., `"o3"` or `"openai/o3"`). There is **no provider dimension**.

This creates several structural issues:

1. **Pricing is stored as a model property**, but pricing is inherently provider-specific. The same model may cost different amounts via OpenAI direct vs. OpenRouter vs. Groq.
2. **Provider-native richness is asymmetric** because only OpenRouter currently provides broad rich native metadata.
3. **Direct provider coverage depends on models.dev** for metadata that native `/models` endpoints do not expose.

### Concrete Example: `o3`

| Source | ID | family | capabilities | pricing | description | supported_parameters |
|--------|-----|--------|--------------|---------|-------------|---------------------|
| OpenAI direct | `o3` | `o3` | `batch`, `function_calling`, `structured_output` | None | None | None |
| OpenRouter | `openai/o3` | None | (empty) | `$0.000002`/`$0.000008` | Full description | `include_reasoning`, `max_tokens`, `reasoning`, ... |
| models.dev | `o3` | `o3` | `function_calling`, `structured_output`, `reasoning` | Per-token USD from per-million source costs | N/A | N/A |

Notice that `o3` (direct) and `openai/o3` (OpenRouter) are **entirely separate entries** with **disjoint metadata**. There is no mechanism to say "`o3` has these capabilities AND this pricing via OpenRouter."

## Runtime Data Structures

### `ProviderModelMetadata`

The primary metadata type lives in `unchained-ai/lib/src/models/model_metadata.rs`:

```rust
pub struct ProviderModelMetadata {
    pub display_name: Option<String>,
    pub family: Option<String>,
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub modalities: Option<ModelModalities>,
    pub capabilities: Vec<String>,
    pub description: Option<String>,
    pub pricing: Option<ModelPricing>,          // provider-specific!
    pub supported_parameters: Option<Vec<String>>, // provider-specific!
    pub default_parameters: Option<ModelDefaultParameters>, // provider-specific!
    pub knowledge_cutoff: Option<String>,
    pub created: Option<u32>,
    pub release_date: Option<String>,
}
```

### `ModelPricing`

```rust
pub struct ModelPricing {
    pub prompt_per_token: Option<f64>,
    pub completion_per_token: Option<f64>,
    pub web_search_per_request: Option<f64>,
    pub input_cache_read_per_token: Option<f64>,
}
```

Costs are in USD. OpenRouter returns pricing as JSON strings (e.g., `"0.000005"`), so a custom deserializer handles both string and numeric representations.

### `ModelDefaultParameters`

```rust
pub struct ModelDefaultParameters {
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
}
```

## CLI: Viewing Metadata

The CLI provides two commands for inspecting model metadata:

### `unchained models [--provider <name>]`

Lists all models grouped by provider. With `--verbose` (or `-v`), metadata is shown beneath each model ID. With `--json`, outputs a JSON array of all models with their metadata.

### `unchained model <model-id>`

Shows detailed metadata for a single model. The `<model-id>` argument uses the `provider/model-id` wire format (e.g., `openai/o3`, `anthropic/claude-opus-4`). Full shell completion is available for model IDs.

**Default output** uses `Prose` and `UnorderedList` from `biscuit-terminal` to render a formatted terminal report.

**JSON output** (with `--json`) presents the complete metadata object as JSON.

## Known Structural Issues

1. **Asymmetric metadata**: Only OpenRouter models get rich metadata. Direct provider models are metadata-poor.

2. **No provider scoping**: The metadata table has no provider dimension. Pricing, parameters, and defaults — all provider-specific — are stored as if they are properties of the model itself.

3. **Metadata remains offering-scoped**: the artifact can group duplicate logical models, but runtime metadata is still keyed by provider/model offering.

4. **Hardcoded OpenRouter parsing**: The merge loop always passes `Provider::OpenRouter` to the metadata parser, preventing extension to other providers.

5. **No cross-provider deduplication**: The same logical model (e.g., `o3`) exists as separate entries (`o3` and `openai/o3`) with no mechanism to reconcile their metadata.

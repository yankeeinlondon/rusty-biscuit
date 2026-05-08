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

### 2. Parsera LLM Specs API

The generator fetches specifications from `https://api.parsera.org/v1/llm-specs` at build time. This external source provides:

- `display_name`
- `family`
- `context_window`
- `max_output_tokens`
- `modalities` (input/output)
- `capabilities`

**Parsera does NOT provide pricing, supported parameters, default parameters, descriptions, or knowledge cutoffs.**

Parsera entries are indexed by **bare model ID** (e.g., `gpt-4o`, `claude-3-5-sonnet`) with no provider prefix. This causes a matching failure for OpenRouter models, which use prefixed IDs like `openai/gpt-4o`.

### 3. model-citizen (Local Models)

The `model-citizen` crate in this monorepo scans local model runners (Ollama, LM Studio, Llama.cpp) for installed GGUF models. It tracks file parameters, quantization, architecture detection, and GGUF metadata.

**This source is completely separate from the unchained-ai metadata pipeline.** It has no pricing information and no awareness of API providers.

## Metadata Synthesis

### Merge Priority

For each model ID discovered from provider APIs, the generator attempts to merge metadata from two sources:

**Priority: Provider-Native > Parsera**

```rust
// unchained-ai/gen/src/metadata_generator.rs
pub fn merge_metadata(
    provider_native: Option<ProviderModelMetadata>,
    parsera: Option<&ParseraModel>,
) -> Option<ProviderModelMetadata> {
    match (provider_native, parsera) {
        (None, None) => None,
        (Some(native), None) => Some(native),
        (None, Some(parsera)) => Some(parsera_to_metadata(parsera)),
        (Some(native), Some(parsera)) => Some(merge_native_with_parsera(native, parsera)),
    }
}
```

When both sources provide the same field (e.g., `context_window`), the provider-native value wins. Fields only available from one source are used as-is.

### The Merge Algorithm

```rust
fn merge_native_with_parsera(native, parsera) -> ProviderModelMetadata {
    ProviderModelMetadata {
        display_name: native.display_name.or(parsera.display_name),
        family: native.family.or(parsera.family),
        context_window: native.context_window.or(parsera.context_window),
        max_output_tokens: native.max_output_tokens.or(parsera.max_output_tokens),
        modalities: native.modalities.or(parsera.modalities),
        capabilities: if native.capabilities.is_empty() { parsera.capabilities } else { native.capabilities },
        // The following are ONLY available from provider-native (OpenRouter):
        description: native.description,
        pricing: native.pricing,
        supported_parameters: native.supported_parameters,
        default_parameters: native.default_parameters,
        knowledge_cutoff: native.knowledge_cutoff,
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
2. **OpenRouter models get no Parsera data** because the lookup fails for prefixed IDs like `openai/o3`.
3. **Direct provider models often have no metadata** because their APIs return minimal data and Parsera may not cover them.

### Concrete Example: `o3`

| Source | ID | family | capabilities | pricing | description | supported_parameters |
|--------|-----|--------|--------------|---------|-------------|---------------------|
| OpenAI direct | `o3` | `o3` | `batch`, `function_calling`, `structured_output` | None | None | None |
| OpenRouter | `openai/o3` | None | (empty) | `$0.000002`/`$0.000008` | Full description | `include_reasoning`, `max_tokens`, `reasoning`, ... |
| Parsera | `o3` | `o3` | `batch`, `function_calling`, `structured_output` | N/A | N/A | N/A |

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

3. **Parsera lookup fails for prefixed IDs**: OpenRouter models like `openai/o3` cannot match Parsera's `gpt-4o` entries, so they miss capabilities and family data.

4. **Hardcoded OpenRouter parsing**: The merge loop always passes `Provider::OpenRouter` to the metadata parser, preventing extension to other providers.

5. **No cross-provider deduplication**: The same logical model (e.g., `o3`) exists as separate entries (`o3` and `openai/o3`) with no mechanism to reconcile their metadata.

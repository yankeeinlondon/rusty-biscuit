# Better Model Metadata

## Problem

The model generator (`gen-models`) discards all provider-native metadata from API responses, relying exclusively on Parsera (`api.parsera.org/v1/llm-specs`) for metadata. This results in extremely sparse coverage: the generated `metadata_generated.rs` has capacity for 580 models but only ~16 entries contain actual metadata. The remaining 564 models have zero metadata.

The root cause is in `openai_api.rs:39-42`:

```rust
pub struct OpenAIModel {
    pub id: String,  // <-- Only field. Everything else is silently dropped by serde.
}
```

OpenRouter alone provides 371 models with rich per-model metadata — pricing, context length, architecture, supported parameters, descriptions, and more — all of which is thrown away during deserialization.

## OpenRouter API Shape (Reference)

The OpenRouter `/v1/models` response provides 16 top-level fields per model:

| Field | Type | Coverage | Value |
|-------|------|----------|-------|
| `id` | string | 371/371 | Model ID (e.g., `"x-ai/grok-4.3"`) |
| `name` | string | 371/371 | Human-readable name (e.g., `"xAI: Grok 4.3"`) |
| `description` | string | 371/371 | Model description |
| `context_length` | number | 371/371 | Context window in tokens |
| `pricing` | object | 371/371 | Per-token costs (prompt, completion, web_search, input_cache_read) |
| `architecture` | object | 371/371 | Modality, input/output modalities, tokenizer, instruct_type |
| `supported_parameters` | string[] | 369/371 | Supported request parameters (temperature, tools, reasoning, etc.) |
| `default_parameters` | object | 240/371 | Default values for temperature, top_p, top_k, etc. |
| `top_provider` | object | 371/371 | max_completion_tokens, is_moderated |
| `canonical_slug` | string | 371/371 | Stable canonical ID |
| `created` | number | 371/371 | Creation timestamp |
| `knowledge_cutoff` | string | 214/371 | Knowledge cutoff date |
| `hugging_face_id` | string? | varies | HuggingFace model ID |
| `expiration_date` | string? | varies | Model deprecation date |
| `per_request_limits` | object? | varies | Rate limits |
| `links` | object | 371/371 | Related API links |

### Example Entry

```json
{
  "id": "x-ai/grok-4.3",
  "name": "xAI: Grok 4.3",
  "description": "Grok 4.3 is a reasoning model from xAI...",
  "context_length": 1000000,
  "pricing": {
    "prompt": "0.00000125",
    "completion": "0.0000025",
    "web_search": "0.005",
    "input_cache_read": "0.0000002"
  },
  "architecture": {
    "modality": "text+image->text",
    "input_modalities": ["text", "image"],
    "output_modalities": ["text"],
    "tokenizer": "Grok",
    "instruct_type": null
  },
  "supported_parameters": [
    "frequency_penalty", "include_reasoning", "logprobs",
    "max_tokens", "presence_penalty", "reasoning",
    "response_format", "seed", "stop", "structured_outputs",
    "temperature", "tool_choice", "tools", "top_logprobs", "top_p"
  ],
  "default_parameters": {
    "temperature": null, "top_p": null, "top_k": null,
    "frequency_penalty": null, "presence_penalty": null,
    "repetition_penalty": null
  },
  "top_provider": {
    "context_length": 1000000,
    "max_completion_tokens": null,
    "is_moderated": false
  },
  "canonical_slug": "x-ai/grok-4.3-20260430",
  "created": 1777591821,
  "knowledge_cutoff": null
}
```

## Goals

1. **Capture provider-native metadata** from OpenRouter (and other providers that return rich data) instead of discarding it
2. **Expand `ModelMetadata`** to include pricing, parameters, descriptions, and architecture information
3. **Merge data from multiple sources**: provider-native metadata as primary, Parsera as supplementary fallback
4. **Maintain backward compatibility**: existing `ModelMetadata` accessors continue to work

## Approach: Provider-Native Metadata Layer

### Strategy

Rather than trying to normalize every provider's response into a single universal shape, introduce a **provider-native metadata layer** alongside the existing Parsera-based layer. OpenRouter is the primary beneficiary; other providers (OpenAI, Anthropic, etc.) return minimal metadata from their `/v1/models` endpoints and will continue to rely on Parsera.

The generator pipeline becomes:

```
1. Fetch Parsera specs (existing, unchanged)
2. Fetch provider models (existing, but now capture full response bodies)
3. For OpenRouter: parse rich response into OpenRouterMetadata
4. For all providers: match Parsera metadata (existing logic)
5. Merge: provider-native metadata + Parsera metadata → unified ModelMetadata
6. Generate expanded metadata_generated.rs
```

### New Types

#### `ProviderModelMetadata` (union type in `models/provider_model_metadata.rs`)

Holds per-model metadata, sourced from the richest available provider-native data plus Parsera.

```rust
pub struct ProviderModelMetadata {
    // Existing fields (backward compatible)
    pub display_name: Option<String>,
    pub family: Option<String>,
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub modalities: Option<ModelModalities>,
    pub capabilities: Vec<String>,

    // New fields from provider-native metadata (all optional, only populated
    // when provider-native data is available)
    pub description: Option<String>,
    pub pricing: Option<ModelPricing>,
    pub supported_parameters: Vec<String>,
    pub default_parameters: Option<ModelDefaultParameters>,
    pub knowledge_cutoff: Option<String>,
    pub created: Option<i64>,
}
```

#### `ModelPricing` (in `models/model_pricing.rs`)

OpenRouter returns pricing values as JSON strings (e.g., `"prompt": "0.00000125"`). A custom serde deserializer parses these string values into `f64`. This is acceptable because there is no near-term requirement for exact decimal arithmetic on pricing data, and `f64` precision is sufficient for informational display.

```rust
pub struct ModelPricing {
    pub prompt_per_token: Option<f64>,
    pub completion_per_token: Option<f64>,
    pub web_search_per_request: Option<f64>,
    pub input_cache_read_per_token: Option<f64>,
}
```

#### `ModelDefaultParameters` (in `models/model_default_parameters.rs`)

```rust
pub struct ModelDefaultParameters {
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<f64>,
    pub frequency_penalty: Option<f64>,
    pub presence_penalty: Option<f64>,
    pub repetition_penalty: Option<f64>,
}
```

### Changes to Existing Code

#### `lib/src/api/openai_api.rs`

The `OpenAIModelsResponse` and `OpenAIModel` structs need to be replaced with provider-aware deserialization:

```rust
/// Response from provider /models endpoints.
#[derive(Debug, Deserialize)]
pub struct ProviderModelsResponse {
    #[serde(default)]
    pub data: Vec<serde_json::Value>,  // <-- Parse raw JSON, not just id
    #[serde(default)]
    pub models: Vec<GeminiModel>,       // Gemini format unchanged
}

/// Parsed model with optional provider-native metadata.
pub struct ProviderModelEntry {
    pub id: String,
    pub raw_metadata: Option<serde_json::Value>,
}
```

The `get_provider_models_from_api` function return type changes from `Vec<String>` to `Vec<ProviderModelEntry>`.

#### `gen/src/main.rs`

The `ProviderResult` struct gains a `raw_metadata` field:

```rust
struct ProviderResult {
    model_count: usize,
    model_ids: Vec<String>,
    raw_metadata: HashMap<String, serde_json::Value>,  // <-- new; keys are unprefixed
}
```

The metadata merge logic becomes:

```rust
for model_id in &result.all_model_ids {
    let parsera_data = find_parsera_metadata(model_id, &parsera_index);

    // Check for provider-native metadata
    let provider_native = result.raw_metadata.get(model_id);

    let merged = merge_metadata(model_id, parsera_data, provider_native, provider);
    metadata_gen.register(model_id.clone(), merged);
}
```

#### `gen/src/metadata_generator.rs`

The `MetadataGenerator::register` signature changes to accept `ProviderModelMetadata` instead of `Option<ParseraModel>`. A new `merge_metadata` function handles the merge logic:

```
fn merge_metadata(
    model_id: &str,
    parsera: Option<&ParseraModel>,
    provider_native: Option<&serde_json::Value>,
    provider: Provider,
) -> ProviderModelMetadata
```

Merge priority (provider-native wins on conflict):
1. `display_name` — provider-native `name` > Parsera `name`
2. `context_window` — provider-native `context_length` > Parsera `context_window`
3. `max_output_tokens` — provider-native `top_provider.max_completion_tokens` > Parsera `max_output_tokens`
4. `modalities` — provider-native `architecture.input_modalities/output_modalities` > Parsera `modalities`
5. `pricing` — provider-native only (Parsera has no pricing)
6. `supported_parameters` — provider-native only
7. `default_parameters` — provider-native only
8. `capabilities` — Parsera only (OpenRouter doesn't have this field)
9. `family` — Parsera only
10. `description` — provider-native only

The generated code output expands to include all new fields.

#### `lib/src/models/model_metadata.rs`

The `ModelMetadata` struct is renamed to `ProviderModelMetadata` and gains the new fields. A type alias preserves backward compatibility:

```rust
#[deprecated(since = "0.2.0", note = "Use ProviderModelMetadata instead")]
pub type ModelMetadata = ProviderModelMetadata;
```

The `ProviderModel` enum's `metadata()` method return type updates to `Option<&'static ProviderModelMetadata>`.

### Provider-Specific Parsing

Only OpenRouter needs special parsing. All other providers continue using the existing Parsera-only path.

**ID normalization:** OpenRouter returns prefixed IDs like `"x-ai/grok-4.3"` and `"anthropic/claude-3.5-sonnet"`. The OpenRouter parser strips the provider prefix from the `id` field before using it as a key. The existing ID normalization logic in the codebase handles this deterministically, so `raw_metadata` keys are always unprefixed (e.g., `"grok-4.3"`, `"claude-3.5-sonnet"`).

#### `gen/src/provider_metadata/openrouter.rs` (new file)

Parses the OpenRouter-specific JSON structure into `ProviderModelMetadata`:

```rust
pub fn parse_openrouter_model(value: &serde_json::Value) -> ProviderModelMetadata {
    // Extract pricing from value["pricing"]
    // Extract architecture from value["architecture"]
    // Extract supported_parameters from value["supported_parameters"]
    // Extract context_length, name, description, etc.
}
```

A dispatcher function routes by provider:

```rust
// gen/src/provider_metadata/mod.rs
pub fn parse_provider_metadata(
    provider: Provider,
    value: &serde_json::Value,
) -> Option<ProviderModelMetadata> {
    match provider {
        Provider::OpenRouter => Some(parse_openrouter_model(value)),
        _ => None,  // Other providers: rely on Parsera
    }
}
```

### Generated File Format

To avoid bloating `metadata_generated.rs` with thousands of lines of rich metadata, the generator produces **two separate files**:

1. **`metadata_generated.rs`** — Compact `ProviderModelMetadata` for **all** models (~580 entries). New fields are optional and only populated when provider-native data is available. Entries use `..Default::default()` to omit `None` fields:

```rust
m.insert("grok-4.3", ProviderModelMetadata {
    display_name: Some("xAI: Grok 4.3".to_string()),
    context_window: Some(1000000),
    modalities: Some(ModelModalities {
        input: vec![Modality::Text, Modality::Image],
        output: vec![Modality::Text],
    }),
    description: Some("Grok 4.3 is a reasoning model from xAI...".to_string()),
    pricing: Some(ModelPricing {
        prompt_per_token: Some(0.00000125),
        completion_per_token: Some(0.0000025),
        web_search_per_request: Some(0.005),
        input_cache_read_per_token: Some(0.0000002),
    }),
    supported_parameters: vec![
        "frequency_penalty".to_string(),
        "include_reasoning".to_string(),
        // ... etc
    ],
    default_parameters: Some(ModelDefaultParameters {
        temperature: None,
        top_p: None,
        // ... etc
    }),
    created: Some(1777591821),
    ..Default::default()
});
```

2. **`metadata_openrouter_generated.rs`** — Rich OpenRouter-native metadata for the subset of models that have OpenRouter data (~371 entries). This file contains the full `OpenRouterModelMetadata` struct and a static lookup keyed by unprefixed model ID:

```rust
use std::sync::LazyLock;
use std::collections::HashMap;

pub struct OpenRouterModelMetadata {
    pub name: String,
    pub description: String,
    pub context_length: u32,
    pub pricing: ModelPricing,
    pub architecture: ModelArchitecture,
    pub supported_parameters: Vec<String>,
    pub default_parameters: Option<ModelDefaultParameters>,
    pub top_provider: TopProviderInfo,
    pub canonical_slug: String,
    pub created: i64,
    pub knowledge_cutoff: Option<String>,
}

pub static OPENROUTER_MODEL_METADATA: LazyLock<HashMap<&str, OpenRouterModelMetadata>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("grok-4.3", OpenRouterModelMetadata {
        name: "xAI: Grok 4.3".to_string(),
        description: "Grok 4.3 is a reasoning model from xAI...".to_string(),
        context_length: 1000000,
        pricing: ModelPricing {
            prompt_per_token: Some(0.00000125),
            completion_per_token: Some(0.0000025),
            web_search_per_request: Some(0.005),
            input_cache_read_per_token: Some(0.0000002),
        },
        architecture: ModelArchitecture {
            modality: "text+image->text".to_string(),
            input_modalities: vec!["text".to_string(), "image".to_string()],
            output_modalities: vec!["text".to_string()],
            tokenizer: Some("Grok".to_string()),
            instruct_type: None,
        },
        supported_parameters: vec![
            "frequency_penalty".to_string(),
            "include_reasoning".to_string(),
            // ... etc
        ],
        default_parameters: Some(ModelDefaultParameters {
            temperature: None,
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            repetition_penalty: None,
        }),
        top_provider: TopProviderInfo {
            context_length: 1000000,
            max_completion_tokens: None,
            is_moderated: false,
        },
        canonical_slug: "x-ai/grok-4.3-20260430".to_string(),
        created: 1777591821,
        knowledge_cutoff: None,
    });
    // ... 370 more entries
    m
});
```

Consumers who only need compact metadata import `metadata_generated.rs`. Consumers who need rich OpenRouter-specific fields import `metadata_openrouter_generated.rs`.

## Phases

### Phase 1: Expand `ModelMetadata` and add new types

**Goal**: Add the new fields and types to the library without changing the generator.

**Files**:
- `lib/src/models/mod.rs` — add new module declarations
- `lib/src/models/model_metadata.rs` — rename struct, add new fields, add deprecation alias
- `lib/src/models/model_pricing.rs` — new file
- `lib/src/models/model_default_parameters.rs` — new file

**Test**: Existing tests pass, new struct fields have defaults so no breakage.

### Phase 2: Capture raw provider responses

**Goal**: Change `openai_api.rs` to preserve the full JSON response instead of stripping to just IDs.

**Files**:
- `lib/src/api/openai_api.rs` — change `OpenAIModel` to preserve raw JSON, change return type of `get_provider_models_from_api`

**Impact**: The `gen/src/main.rs` consumer needs to handle the new return type. Other callers of `get_provider_models_from_api` (if any) need updating.

**Test**: Unit tests for the new deserialization, existing integration tests pass.

### Phase 3: OpenRouter metadata parser

**Goal**: Add the provider-specific parser for OpenRouter and the merge logic.

**Files**:
- `gen/src/provider_metadata/mod.rs` — new file, dispatcher
- `gen/src/provider_metadata/openrouter.rs` — new file, OpenRouter-specific parsing
- `gen/src/metadata_generator.rs` — update to accept `ProviderModelMetadata`, add merge logic
- `gen/src/main.rs` — wire up provider-native metadata into the pipeline

**Test**: Unit tests for `parse_openrouter_model` with fixture data, test merge priority logic.

### Phase 4: Generator output expansion

**Goal**: Update the code generator to emit both the compact `metadata_generated.rs` and the rich `metadata_openrouter_generated.rs`.

**Files**:
- `gen/src/metadata_generator.rs` — update `generate_entry` to emit compact `ProviderModelMetadata`; add `generate_openrouter_entry` for rich `OpenRouterModelMetadata`
- `gen/src/main.rs` — wire up second file generation path

**Test**: Run `gen-models --providers openrouter --dry-run` and verify both files are produced. Run full `gen-models` and verify both generated files compile. Confirm that consumers who do not import `metadata_openrouter_generated.rs` do not pay the compile-time cost.

### Phase 5: Cleanup and documentation

**Goal**: Update skill docs, README, and remove deprecated type alias if appropriate.

**Files**:
- `.opencode/skill/unchained-ai/providers-and-models.md`
- `.opencode/skill/unchained-ai/model-generator.md`
- `AGENTS.md` if workspace layout changes

## Non-Goals

- **Parsing non-OpenRouter provider-native metadata**: OpenAI, Anthropic, Groq, xAI, etc. return minimal metadata (just `id`, `owned_by`, `created`). Not worth special-casing until they provide richer responses.
- **Runtime metadata queries**: This spec focuses on build-time generation. The separate generated file approach means metadata is baked into the binary at compile time. Runtime API queries for fresh metadata are out of scope.
- **Merging metadata from aggregators**: ZenMux may also provide rich metadata similar to OpenRouter, but it's not in scope for this initial implementation.
- **OpenRouter-specific models endpoint**: The current `/v1/models` endpoint already returns all the metadata we need. No need for additional API calls.

## Risks

- **Generated file size**: OpenRouter alone has 371 models. With expanded metadata, the combined generated files would be ~3000–5000 lines, but the separate-file approach mitigates compile-time impact: only consumers who import `metadata_openrouter_generated.rs` pay the cost. The compact `metadata_generated.rs` remains small. The LazyLock ensures each table is only built on first access.
- **Parsera API instability**: No change — Parsera remains optional with graceful degradation.
- **OpenRouter API changes**: Fields could be added/removed. The serde-based parsing uses `#[serde(default)]` throughout so missing fields degrade gracefully.

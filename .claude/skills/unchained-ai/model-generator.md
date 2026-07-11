# Model Generator (gen-models)

Detailed reference for the `unchained-ai-gen` binary in `unchained-ai/gen/`.

## Purpose

Auto-generates provider model enum files and metadata lookup tables by querying live provider APIs and the models.dev catalog.

## CLI Usage

```bash
# Generate all providers
cargo run -p unchained-ai-gen

# Generate specific providers
cargo run -p unchained-ai-gen -- --providers openai,anthropic

# Skip specific providers
cargo run -p unchained-ai-gen -- --skip zenmux,ollama

# Custom output directory
cargo run -p unchained-ai-gen -- --output ./models

# Dry run (preview without writing files)
cargo run -p unchained-ai-gen -- --dry-run

# Verbose output
cargo run -p unchained-ai-gen -- -v    # INFO
cargo run -p unchained-ai-gen -- -vv   # DEBUG
cargo run -p unchained-ai-gen -- -vvv  # TRACE
```

## Architecture

### Source Files

```
gen/src/
├── main.rs                 # CLI entrypoint (clap-based)
├── generator.rs            # ModelEnumGenerator - produces Rust enum source
├── metadata_generator.rs   # MetadataGenerator - produces metadata lookup tables
├── provider_metadata/      # Provider-native metadata parsers
│   ├── mod.rs              # Dispatcher routing to provider-specific parsers
│   └── openrouter.rs       # OpenRouter metadata extraction (pricing, params, etc.)
├── models_dev.rs           # models.dev catalog fetch, matching, guards, and field mapping
└── errors.rs               # GeneratorError type
```

### Pipeline

1. **Fetch models.dev catalog** - Single fetch at startup from `https://models.dev/api.json` (30s timeout, 2s delay + 1 retry on failure); degraded responses fail generation loudly
2. **Fetch provider models** - Parallel queries to each provider's `/models` endpoint using API keys from environment; raw metadata preserved per model
3. **Parse provider-native metadata** - Route raw JSON through provider-specific parsers (e.g., OpenRouter) to extract pricing, architecture, default parameters
4. **Match and merge metadata** - Match models.dev rows within each provider bucket; priority is provider-native > models.dev for overlapping fields, with both sources filling gaps
5. **Generate enum files** - One `.rs` file per provider with `#[derive(ModelId)]` enum
6. **Generate compact metadata file** - `metadata_generated.rs` mapping model IDs to `ModelMetadata` (with `..Default::default()` for brevity)
7. **Generate rich OpenRouter metadata file** - `metadata_openrouter_generated.rs` with full `ProviderModelMetadata` entries including pricing, parameters, etc.
8. **Write atomically** - Temp file + rename to prevent corruption

### ModelEnumGenerator (`generator.rs`)

Takes a provider name and model ID list, produces Rust source containing:
- Auto-generated header with timestamp and version
- Enum with one variant per model ID
- `#[derive(ModelId)]` macro for ID lookup and metadata binding
- `Bespoke(String)` catch-all variant for custom model IDs
- Documentation comments with original wire-format model IDs

Variant naming uses `enum_variant_name_from_wire_id()` from `build/enum_name.rs` which converts wire IDs to valid Rust identifiers (e.g., `gpt-4o` becomes `Gpt__4o`, `claude-3-5-haiku-20241022` becomes `Claude__3__5__Haiku__20241022`).

### models.dev Integration (`models_dev.rs`)

Fetches model specs from [models.dev](https://models.dev) and maps them into `ProviderModelMetadata`.

**Fetch strategy**:
- Fetched once at startup before processing any providers
- 30-second request timeout
- On failure: wait 2 seconds, retry once; if both attempts fail, return an error
- Validate the live response has at least 50 provider buckets
- Validate roster-critical buckets are present and non-empty: `anthropic`, `google`, `moonshotai`, `openai`, `openrouter`, `xai`, `zai`, `deepseek`, `groq`, and `mistral`

**Provider bucketing**:
- Direct keys: `anthropic`, `deepseek`, `groq`, `mistral`, `moonshotai`, `openai`, `openrouter`
- Renamed keys: `gemini -> google`, `x-ai -> xai`, `z-ai -> zai`
- No models.dev bucket: `ollama`, `zenmux`

**Model ID matching** (`find_models_dev_metadata`):
1. **Exact match** - Direct lookup by provider-local model ID
2. **Identity-aware match** - Parse both generated and models.dev IDs with `ModelIdentity::parse`, compare identity keys, prefer exact date-pin agreement, then the unpinned row
3. **Ambiguity refusal** - If identity candidates remain ambiguous, report no match instead of guessing

**Metadata fields from models.dev**:
- `display_name` (e.g., "GPT-4o mini")
- `family` (e.g., "gpt-4o-mini")
- `context_window` (tokens)
- `max_output_tokens` (tokens)
- `modalities` (input/output arrays: text, image, audio, video)
- `capabilities` from canonical tokens: `function_calling`, `structured_output`, `reasoning`, `file_input`
- `pricing` from per-million USD models.dev costs converted to per-token USD by dividing by `1e6`
- `knowledge_cutoff`
- `release_date` as a source release-date string

Note: `temperature`, `reasoning_options`, and `last_updated` from models.dev are intentionally not mapped. `created` remains native-source metadata; models.dev `release_date` is kept separate.

## Output

Generated files are placed in `unchained-ai/lib/src/rigging/providers/models/` by default.

### Provider Enum Files

Example (`openai.rs`):
```rust
// Auto-generated by gen-models
// Generated: 2025-01-15T12:00:00Z
// Version: 0.1.0

use model_id::ModelId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, ModelId)]
#[model_id_metadata(
    lookup = "super::metadata_generated::MODEL_METADATA",
    returns = "crate::models::model_metadata::ModelMetadata"
)]
pub enum ProviderModelOpenAi {
    /// Model ID: `gpt-4o`
    Gpt__4o,
    /// Model ID: `o3`
    O3,
    // ... more variants ...
    Bespoke(String),
}
```

### Metadata Lookup Table (`metadata_generated.rs`)

Static `MODEL_METADATA` hashmap mapping model ID strings to `ModelMetadata` structs. Uses `..Default::default()` for compact entries. Referenced by the `#[model_id_metadata]` attribute on provider enums.

### Rich OpenRouter Metadata (`metadata_openrouter_generated.rs`)

Static `OPENROUTER_MODEL_METADATA` hashmap with full `ProviderModelMetadata` entries for OpenRouter models, including pricing, architecture, default parameters, knowledge cutoff, and description. Populated only when OpenRouter API access is available during generation.

### Provider Metadata Parsing (`provider_metadata/`)

Provider-specific parsers extract rich metadata from raw API responses:

- **OpenRouter parser** (`openrouter.rs`): Extracts pricing (prompt/completion/cache_read/web_search), architecture (modalities, tokenizer), top_provider (context_window, max_completion_tokens), supported_parameters, default_parameters, description, and knowledge_cutoff.
- **Dispatcher** (`mod.rs`): Routes to the appropriate parser based on `Provider` enum.

## Environment Variables

| Provider | Variable |
|----------|----------|
| OpenAI | `OPENAI_API_KEY` |
| Anthropic | `ANTHROPIC_API_KEY` |
| Groq | `GROQ_API_KEY` |
| Mistral | `MISTRAL_API_KEY` |
| Deepseek | `DEEPSEEK_API_KEY` |
| Gemini | `GEMINI_API_KEY` or `GOOGLE_API_KEY` |
| OpenRouter | `OPENROUTER_API_KEY` or `OPEN_ROUTER_API_KEY` |
| xAI | `XAI_API_KEY` or `X_AI_API_KEY` |
| Z.ai | `ZAI_API_KEY` or `Z_AI_API_KEY` |
| MoonshotAI | `MOONSHOT_API_KEY` or `MOONSHOT_AI_API_KEY` |
| HuggingFace | `HF_TOKEN`, `HUGGINGFACE_TOKEN`, or `HUGGING_FACE_TOKEN` |

Providers without configured API keys are skipped. Local providers (Ollama) are skipped by default.

## Dependencies

- `unchained-ai` (path: `../lib`) - provider registry and model types
- `clap` v4.5 (derive) - CLI argument parsing
- `tokio` v1.49 - async runtime
- `chrono` v0.4.43 - timestamp generation
- `reqwest` v0.12 (rustls-tls, json) - HTTP client
- `serde`/`serde_json` v1.0 - JSON handling
- `strum` v0.27 - enum iteration
- `tracing`/`tracing-subscriber` - logging
- `thiserror` v2.0 - error types

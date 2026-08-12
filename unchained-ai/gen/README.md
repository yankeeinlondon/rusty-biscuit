# gen-models

Generator for provider model enum files and metadata lookup tables.

## Usage

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

## Environment Variables

Set API keys for providers you want to generate:

| Provider | Variable |
|----------|----------|
| OpenAI | `OPENAI_API_KEY` |
| Anthropic | `ANTHROPIC_API_KEY` |
| Groq | `GROQ_API_KEY` |
| Mistral | `MISTRAL_API_KEY` |
| Deepseek | `DEEPSEEK_API_KEY` |
| Gemini | `GEMINI_API_KEY` or `GOOGLE_API_KEY` |
| OpenRouter | `OPENROUTER_API_KEY` or `OPEN_ROUTER_API_KEY` |
| XAI | `XAI_API_KEY` or `X_AI_API_KEY` |
| ZAI | `ZAI_API_KEY` or `Z_AI_API_KEY` |
| MoonshotAI | `MOONSHOT_API_KEY` or `MOONSHOT_AI_API_KEY` |
| HuggingFace | `HF_TOKEN`, `HUGGINGFACE_TOKEN`, or `HUGGING_FACE_TOKEN` |

## Output

Generated files are placed in `unchained-ai/lib/src/rigging/providers/models/` by default.

### Provider Enum Files

Each provider file (e.g., `openai.rs`, `anthropic.rs`) contains:

- Auto-generated header with timestamp and version
- Enum with `ModelId` derive macro
- One variant per model ID
- `Bespoke(String)` variant for custom model IDs

### Metadata Lookup Table

The generator also produces `metadata_generated.rs` containing a static lookup table
with rich model metadata fetched from models.dev and provider-native APIs.

Metadata includes:

- `display_name` - Human-readable model name (e.g., "GPT-4o mini")
- `family` - Model family (e.g., "gpt-4o-mini", "claude-3")
- `context_window` - Maximum context size in tokens
- `max_output_tokens` - Maximum output generation length
- `modalities` - Input/output modalities (text, image, audio, video)
- `capabilities` - Features like "function_calling", "structured_output"
- `pricing` - Per-token USD pricing where available
- `knowledge_cutoff` - Source knowledge cutoff date where available
- `release_date` - Source release-date string where available

### Metadata Sourcing from models.dev

The generator fetches model specifications from [models.dev](https://models.dev)
at generation time and merges them with provider-native metadata.

- **API endpoint**: https://models.dev/api.json
- **Provider buckets**: direct provider keys plus local mappings such as `gemini -> google`, `x-ai -> xai`, and `z-ai -> zai`
- **Pricing units**: models.dev costs are per million tokens and are converted to per-token USD by dividing by `1e6`

**Fetch Strategy:**

1. models.dev data is fetched once at startup, before processing any providers
2. On failure, the generator waits 2 seconds and retries once
3. If both attempts fail, generation exits with an error
4. Request timeout is 30 seconds
5. The response must pass anti-sunset guards: at least 50 provider buckets and non-empty roster-critical buckets

**Model ID Matching:**

Since provider APIs return model IDs that may differ from models.dev IDs,
matching is scoped to the mapped provider bucket:

1. **Exact match** - Direct lookup by provider-local model ID
2. **Identity-aware match** - Parse both sides with `ModelIdentity::parse` and compare identity keys
3. **Tie-breaks** - Prefer exact date-pin agreement, then the unpinned row; ambiguous matches are refused

**Example models.dev Response Shape:**

```json
{
  "openai": {
    "models": {
      "gpt-4o-mini": {
        "name": "GPT-4o mini",
        "family": "gpt-4o-mini",
        "limit": {
          "context": 128000,
          "output": 16384
        },
        "modalities": {
          "input": ["text", "image"],
          "output": ["text"]
        },
        "cost": {
          "input": 0.15,
          "output": 0.6
        },
        "tool_call": true,
        "structured_output": true
      }
    }
  },
  "...": {}
}
```

**Note:** `created` stays provider-native. models.dev `release_date` is stored
separately and serialized into the v2 model-catalog artifact when present.

## Runtime API

After generation, use the accessor methods on `ProviderModel`:

```rust
use unchained_ai::rigging::providers::models::ProviderModel;
use unchained_ai::models::model_metadata::Modality;

let model = ProviderModel::OpenAi(ProviderModelOpenAi::O3);

// Get full metadata
if let Some(meta) = model.metadata() {
    println!("Name: {:?}", meta.display_name);
    println!("Context: {:?}", meta.context_window);
}

// Convenience methods
let ctx = model.context_window();           // Option<u32>
let max_out = model.max_output_tokens();    // Option<u32>
let has_vision = model.supports_input(Modality::Image);
let has_fc = model.has_capability("function_calling");
```

## Notes

- Providers without API keys configured will be skipped
- Local providers (Ollama) are skipped by default
- Failed providers are logged but don't stop the generation
- Files are written atomically (temp file + rename) to prevent corruption
- models.dev source degradation fails generation loudly
- Model ID matching uses exact provider-local match followed by identity-aware matching

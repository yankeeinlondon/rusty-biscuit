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
| Mira | `MIRA_API_KEY` |

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
with rich model metadata fetched from the [Parsera LLM Specs API](https://api.parsera.org/v1/llm-specs).

Metadata includes:

- `display_name` - Human-readable model name (e.g., "GPT-4o mini")
- `family` - Model family (e.g., "gpt-4o-mini", "claude-3")
- `context_window` - Maximum context size in tokens
- `max_output_tokens` - Maximum output generation length
- `modalities` - Input/output modalities (text, image, audio, video)
- `capabilities` - Features like "function_calling", "structured_output"

### Metadata Sourcing from Parsera

The generator fetches model specifications from [Parsera's LLM Specs API](https://api.parsera.org/v1/llm-specs)
at build time. This API is maintained by [Parsera](https://parsera.org) (the web scraping company) in
partnership with [Carmine Paolino](https://paolino.me/standard-api-llm-capabilities-pricing-live/).
They use automated scraping to keep model metadata current by pulling from provider documentation.

- **API endpoint**: https://api.parsera.org/v1/llm-specs
- **Web comparison tool**: https://llmspecs.parsera.org/
- **GitHub**: [parsera-labs](https://github.com/parsera-labs)

**Fetch Strategy:**

1. Parsera specs are fetched once at startup, before processing any providers
2. On failure, the generator waits 2 seconds and retries once
3. If both attempts fail, generation continues with empty metadata (graceful degradation)
4. Request timeout is 30 seconds

**Model ID Matching:**

Since provider APIs return model IDs that may differ from Parsera's canonical IDs,
we use a multi-step matching strategy:

1. **Exact match** - Direct lookup by model ID (e.g., `gpt-4o` → `gpt-4o`)
2. **Date suffix stripping** - Remove `-YYYYMMDD` suffixes common in versioned models
   (e.g., `claude-3-5-haiku-20241022` → `claude-3-5-haiku`)
3. **Family fallback** - Match against Parsera's `family` field for model variants
   (e.g., looking up `claude-3-5-sonnet` finds a model with `family: "claude-3-5-sonnet"`)

**Example Parsera Response:**

```json
{
  "id": "gpt-4o-mini",
  "name": "GPT-4o mini",
  "provider": "openai",
  "family": "gpt-4o-mini",
  "context_window": 128000,
  "max_output_tokens": 16384,
  "modalities": {
    "input": ["text", "image"],
    "output": ["text"]
  },
  "capabilities": ["function_calling", "structured_output"]
}
```

**Note:** Parsera does not provide `default_temperature` - that field is only available
from Mistral's native API.

## Runtime API

After generation, use the accessor methods on `ProviderModel`:

```rust
use unchained_ai::rigging::providers::models::ProviderModel;
use unchained_ai::models::model_metadata::Modality;

let model = ProviderModel::OpenAi(ProviderModelOpenAi::Gpt__4o);

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
- Parsera API failures are handled gracefully (metadata will be empty)
- Model ID matching uses exact match, date-suffix stripping, and family fallback

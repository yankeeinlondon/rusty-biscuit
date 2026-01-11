# Biscuit
<img src="../assets/biscuit.png" style="position: fixed; max-width: 30%; height: 150px; right: 0; top: 0; opacity: 0.75"></img>

A shared library which includes:

- TTS
- Markdown Pipelining
- AI Pipelining

## Modules

### API Module (`api/`)

OpenAI-compatible API utilities for fetching models from LLM providers.

**Key Features:**

- Fetch models from individual providers via `/v1/models` endpoint
- Parallel fetching from all configured providers
- Retry logic with exponential backoff
- Type-safe model representations

**Example:**

```rust
use shared::api::openai_compat::{get_provider_models_from_api, get_all_provider_models};
use shared::providers::base::Provider;

// Fetch from single provider
let models = get_provider_models_from_api(Provider::OpenAI, "sk-...").await?;

// Fetch from all providers (parallel)
let all_models = get_all_provider_models().await?;
```

**See:** [`api/README.md`](./src/api/README.md) for architecture details.

### Providers Module (`providers/`)

Provider discovery, authentication, and model management for LLM APIs.

**Supported Providers:**

- Anthropic (Claude)
- Deepseek
- Gemini (Google)
- MoonshotAI
- Ollama (local)
- OpenAI
- OpenRouter
- ZAI
- ZenMux

**Key Components:**

**`providers/base.rs`** - Base provider infrastructure:

- Environment-based API key detection
- Provider base URL configuration
- Artificial Analysis URL generation
- API key validation

**`providers/discovery.rs`** - Dynamic model discovery:

- OpenAI-compatible `/v1/models` endpoint fetching
- 24-hour cache with automatic expiration
- Rate limiting with exponential backoff
- Parallel provider queries

**`providers/curated.rs`** - Hardcoded model registry:

- Manually curated model lists
- Last updated timestamp tracking
- Fallback when API discovery unavailable

**`providers/types.rs`** - Type definitions:

- `ProviderModel` enum for type-safe model references
- `OpenAIModelsResponse` for API responses
- Naming convention: `Provider_Model_Name` → `provider/model-name`

**`providers/cache.rs`** - Provider cache management:

- 24-hour TTL for discovered models
- Path: `~/.cache/dockhand/provider_list.json`
- Automatic invalidation

**`providers/retry.rs`** - Retry logic:

- Exponential backoff with jitter
- Configurable max retries
- Rate limit handling

**Example:**

```rust
use shared::providers::base::{Provider, has_provider_api_key};
use shared::providers::types::ProviderModel;
use shared::providers::discovery::fetch_all_models;

// Check API key availability
if has_provider_api_key(Provider::OpenAI) {
    // Fetch models with caching
    let models = fetch_all_models().await?;
}

// Type-safe model reference
let model = ProviderModel::Openai_Gpt_4o;
assert_eq!(model.to_string(), "openai/gpt-4o");
```

### Tools Module (`tools/`)

LLM agent tools compatible with the `rig-core` framework.

**BraveSearchTool (`tools/brave_search.rs`):**

- Web search via Brave Search API
- Plan-based rate limiting (free: 1/sec, base: 20/sec, pro: 50/sec)
- Comprehensive tracing instrumentation
- Environment variables: `BRAVE_API_KEY`, `BRAVE_PLAN`

**ScreenScrapeTool (`tools/screen_scrape.rs`):**

- Web scraping with multiple output formats
- Formats: Markdown, HTML, PlainText, JSON, Links
- CSS selector support for targeted extraction
- Pre-scrape actions (click, scroll, wait)

**Example:**

```rust
use shared::tools::brave_search::BraveSearchTool;
use rig::tool::Tool;

let tool = BraveSearchTool::from_env();
let results = tool.call(serde_json::json!({
    "query": "rust async programming"
})).await?;
```

### Codegen Module (`codegen/`)

Safe code injection using AST manipulation.

**Features:**

- AST-based code modification (no regex)
- Injection point validation
- Duplicate detection
- Pretty-printing with `prettyplease`

**Example:**

```rust
use shared::codegen::inject::inject_enum;

inject_enum("MyEnum", "NewVariant", "path/to/file.rs")?;
```

### Model Module (`model/`)

Model selection and management utilities.

**Components:**

- `types.rs` - Model-related type definitions
- `selection.rs` - Interactive model selection (future)

### TTS Module (`tts.rs`)

Cross-platform text-to-speech using system TTS.

**Example:**

```rust
use shared::tts::announce;

announce("Research complete").await;
```

**Used by:**

- `research` CLI for completion announcements
- `speak` CLI for text-to-speech conversion

## Architecture

### Provider Discovery System

The provider system has three layers:

1. **Base layer** (`providers/base.rs`):
   - Environment-based API key management
   - Provider base URLs
   - OpenAI-compatible endpoint configuration

2. **Curated registry** (`providers/curated.rs`):
   - Hardcoded model list with timestamps
   - Fallback when API discovery fails

3. **Discovery layer** (`providers/discovery.rs`):
   - API fetching with 24-hour cache
   - Rate limiting with exponential backoff
   - Parallel provider queries

### API Module Architecture

The `api` module provides a centralized location for OpenAI-compatible API utilities:

**Design Principles:**

- DRY: Eliminate duplicate HTTP request code
- Type-safe: Use `ProviderModel` enum instead of strings
- Instrumented: OpenTelemetry tracing with semantic conventions
- Secure: API keys never logged (`#[tracing::instrument(skip(api_key))]`)

**Key Types:**

- `ProviderModelList` - List of models from all providers
- Uses `providers/types::OpenAIModelsResponse` for API responses

### Naming Convention (ProviderModel)

The `ProviderModel` enum uses a specific naming convention:

**Format:** `Provider_Model_Name`

- Provider: PascalCase prefix
- Model name: snake_case or kebab-case
- Separator: Underscore (`_`)
- Special chars: Periods (`.`), colons (`:`), hyphens (`-`) preserved

**Examples:**

- `Openai_Gpt_4o` → `"openai/gpt-4o"`
- `Anthropic_Claude_Opus_4_5` → `"anthropic/claude-opus-4.5"`
- `Gemini_1_5_Flash_001` → `"gemini/1.5-flash-001"`
- `Deepseek_Chat` → `"deepseek/chat"`

**Conversion:**

```rust
// String → ProviderModel
let model: ProviderModel = "openai/gpt-4o".to_string().try_into()?;

// ProviderModel → String
assert_eq!(model.to_string(), "openai/gpt-4o");

// Extract parts
assert_eq!(model.provider(), "openai");
assert_eq!(model.model_name(), "gpt-4o");
```

## Environment Variables

### Provider API Keys

| Variable | Provider | Alternative |
|----------|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic | - |
| `DEEPSEEK_API_KEY` | Deepseek | - |
| `GEMINI_API_KEY` | Gemini | `GOOGLE_API_KEY` |
| `MOONSHOT_API_KEY` | MoonshotAI | `MOONSHOT_AI_API_KEY` |
| `OPENAI_API_KEY` | OpenAI | - |
| `OPEN_ROUTER_API_KEY` | OpenRouter | `OPENROUTER_API_KEY` |
| `ZAI_API_KEY` | ZAI | `Z_AI_API_KEY` |
| `ZENMUX_API_KEY` | ZenMux | `ZEN_MUX_API_KEY` |

### Tool Configuration

| Variable | Tool | Default |
|----------|------|---------|
| `BRAVE_API_KEY` | BraveSearchTool | (required) |
| `BRAVE_PLAN` | BraveSearchTool | `free` |

**Brave Plan Options:** `free` (1/sec), `base` (20/sec), `pro` (50/sec)

## Usage

### Building

```bash
# Build shared library
cargo build -p shared

# With specific features (if any)
cargo build -p shared --features <feature>
```

### Testing

```bash
# All tests
cargo test -p shared

# Specific module
cargo test -p shared --lib providers
cargo test -p shared --lib api
cargo test -p shared --lib tools

# With output
cargo test -p shared -- --nocapture

# Single test
cargo test -p shared --lib providers::base::tests::test_has_provider_api_key_with_set_env_var
```

### Documentation

```bash
# Generate rustdoc
cargo doc -p shared --no-deps --open

# Check doc examples
cargo test -p shared --doc
```

## Dependencies

### Key Dependencies

**LLM & AI:**

- `rig-core` (v0.27.0) - LLM agent framework

**HTTP & Web:**

- `reqwest` (v0.12) - HTTP client for API requests
- `scraper` (v0.20) - HTML parsing for web scraping

**Async Runtime:**

- `tokio` (v1.48.0) - Async runtime with full features

**Serialization:**

- `serde` / `serde_json` (v1.0) - JSON serialization

**Code Generation:**

- `syn` - Rust AST parsing
- `prettyplease` - Code formatting

**Tracing:**

- `tracing` - Structured logging and instrumentation
- OpenTelemetry semantic conventions

For complete dependency information, see [`/docs/dependencies.md`](../docs/dependencies.md).

## Tracing

All public APIs in the shared library emit tracing events following OpenTelemetry semantic conventions:

**Key Principles:**

- Libraries emit, applications configure
- Structured fields over messages
- Spans for context and duration
- API keys always skipped

**Example Instrumentation:**

```rust
#[tracing::instrument(skip(api_key))]
pub async fn get_provider_models_from_api(
    provider: Provider,
    api_key: &str,
) -> Result<Vec<String>, ProviderError> {
    // Function emits spans and events
}
```

**Semantic Conventions:**

- `tool.name` - Tool identifier
- `tool.query` - Search query or URL
- `tool.duration_ms` - Execution time
- `http.status_code` - HTTP response code

For complete tracing documentation, see [`/docs/tracing.md`](../docs/tracing.md).

## Recent Changes

### Provider/Model Refactoring (2025-12-30)

Major refactoring to eliminate code duplication and improve type safety:

**Changes:**

- Created `api` module for OpenAI-compatible utilities
- Moved `OpenAIModelsResponse` to `providers/types.rs`
- Introduced `ProviderModel` enum for type-safe model references
- Extracted duplicate HTTP code to `api::openai_compat`

**Migration Guide:** See [`.ai/docs/provider-model-migration.md`](../.ai/docs/provider-model-migration.md)

## License

See repository LICENSE file.

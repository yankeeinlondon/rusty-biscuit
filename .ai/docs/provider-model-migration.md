# Provider & Model System Migration Guide

**Date:** 2025-12-30
**Plan:** `.ai/plans/2025-12-30.provider-model-refactoring.md`
**Phases:** 0-6 (Implementation complete)

This guide documents the refactoring of the provider and model system in the Dockhand monorepo. The changes eliminate code duplication, introduce type-safe model references, and improve the overall architecture.

## Overview

The refactoring touched three main areas:

1. **Provider/Model System** (`shared/src/providers/`, `shared/src/api/`)
   - Eliminated duplicate code between `base.rs` and `discovery.rs`
   - Introduced `ProviderModel` enum for type-safe model references
   - Extracted shared types to `types.rs`
   - Created new `api` module for OpenAI-compatible utilities

2. **Research Library** (`research/lib/src/`)
   - Refactored filename generation to use `ProviderModel`
   - Enhanced custom prompt naming syntax

3. **Error Handling**
   - Improved error types across provider and research modules

## Breaking Changes

### Import Path Changes

**Types moved from `providers/base.rs` and `providers/discovery.rs` to `providers/types.rs`:**

```rust
// Before
use shared::providers::discovery::OpenAIModelsResponse;

// After
use shared::providers::types::OpenAIModelsResponse;
```

**New `api` module for OpenAI-compatible utilities:**

```rust
// Before
// Functions were in providers/base.rs and providers/discovery.rs

// After
use shared::api::openai_compat::get_provider_models_from_api;
use shared::api::types::ProviderModelList;
```

### Function Signature Changes

**Provider model fetching now uses extracted functions:**

```rust
// Before (duplicated code in base.rs and discovery.rs)
// Direct inline HTTP requests

// After (centralized in api module)
use shared::api::openai_compat::get_provider_models_from_api;

async fn fetch_models(provider: Provider, api_key: &str) -> Result<Vec<String>, ProviderError> {
    get_provider_models_from_api(provider, api_key).await
}
```

### Removed Duplicated Code

The following duplicate implementations were removed:

1. **`OpenAIModelsResponse` type** - Was defined in both `base.rs` and `discovery.rs`, now in `types.rs`
2. **HTTP client setup** - Extracted to `api::openai_compat::get_provider_models_from_api`
3. **Rate limiting logic** - Consolidated in `api` module

## New Features

### 1. ProviderModel Enum

A type-safe enum for representing provider/model combinations using naming convention:

```rust
use shared::providers::types::ProviderModel;
use shared::providers::Provider;

// Using static variant (compile-time safe)
let model = ProviderModel::OpenAi__Gpt__4o;

// Or parsing from string (TryFrom)
let model: ProviderModel = "openai/gpt-4o".to_string().try_into().unwrap();

// Convert back to string format (Display trait)
assert_eq!(format!("{}", model), "openai/gpt-4o");

// Get provider and model parts
assert_eq!(model.provider(), Provider::OpenAi);
assert_eq!(model.model_id(), "gpt-4o");
```

**Naming Convention:**
- Provider prefix in PascalCase
- Model name segments separated by **double underscores** (`__`)
- Special character conversions:
  - Hyphens `-` → Double underscores `__`
  - Dots `.` → Single underscores `_`
  - Colons `:` → Removed

**Examples:**
- `OpenAi__Gpt__4o` → `openai/gpt-4o`
- `Anthropic__ClaudeOpus__4__5__20251101` → `anthropic/claude-opus-4-5-20251101`
- `Deepseek__Chat` → `deepseek/chat`
- `Gemini__Gemini__2__0__Flash__Exp` → `gemini/gemini-2-0-flash-exp`

**Features:**
- **Type Safety**: Static variants provide compile-time guarantees for known models
- **Flexibility**: String outlets allow runtime use of bleeding-edge/undocumented models
- **Serde Support**: Automatic JSON serialization/deserialization (uses `provider/model` string format)
- **Display Trait**: Format as `provider/model-id` string via `format!("{}", model)`
- **TryFrom<String>**: Parse from string format with validation
- **Helper Methods**:
  - `provider()` → Returns the `Provider` enum
  - `model_id()` → Returns the model identifier string
  - `to_identifier()` → Returns full `provider/model-id` string

### 2. API Module (`shared/src/api/`)

New module providing OpenAI-compatible API utilities:

**Module Structure:**
```
shared/src/api/
├── mod.rs              - Module exports
├── openai_compat.rs    - OpenAI-compatible /v1/models fetching
├── types.rs            - API-related types (ProviderModelList, etc.)
└── README.md           - Module documentation
```

**Key Functions:**

```rust
use shared::api::openai_compat::{get_provider_models_from_api, get_all_provider_models};
use shared::providers::base::Provider;

// Fetch models from a single provider
let models = get_provider_models_from_api(Provider::OpenAI, "sk-...").await?;

// Fetch models from all configured providers (parallel)
let all_models = get_all_provider_models().await?;
```

### 3. Enhanced Research Filename Generation

The research library now supports custom prompt naming syntax:

**Custom Naming Syntax:**
```bash
# Before: Prompts were always named question_N.md
research library clap "How does it compare to structopt?"
# Generated: question_1.md

# After: Use arrow syntax to specify filename
research library clap "comparison -> How does it compare to structopt?"
# Generated: comparison.md (with "How does it compare to structopt?" as the prompt)
```

**Implementation:**

```rust
use research_lib::utils::filename::extract_prompt_name;
use shared::providers::types::ProviderModel;

// With custom name - extract before LLM generation
let (prompt, custom_filename) = extract_prompt_name(
    "comparison -> How does it compare to structopt?"
);
assert_eq!(custom_filename, Some("comparison.md".to_string()));
assert_eq!(prompt, "How does it compare to structopt?");

// Without custom name (returns None)
let (prompt, custom_filename) = extract_prompt_name(
    "How does it compare to structopt?"
);
assert_eq!(custom_filename, None);
// Falls back to LLM-generated filename via choose_filename()
```

## Migration Steps

### Step 1: Update Imports

Replace old import paths with new ones:

```rust
// Old imports
use shared::providers::discovery::OpenAIModelsResponse;

// New imports
use shared::providers::types::{OpenAIModelsResponse, ProviderModel};
use shared::api::openai_compat::get_provider_models_from_api;
```

### Step 2: Replace String Model References

Convert string-based model references to `ProviderModel`:

```rust
// Before
let model_name = "openai/gpt-4o";

// After - using TryFrom
let model: ProviderModel = "openai/gpt-4o".to_string().try_into()?;

// Or using static variant (compile-time safe)
let model = ProviderModel::OpenAi__Gpt__4o;

// Or using String outlet for non-static models
let model = ProviderModel::OpenAi("gpt-5-experimental".to_string());
```

### Step 3: Use API Module for Model Fetching

Replace inline HTTP requests with API module functions:

```rust
// Before (manual HTTP request)
let client = reqwest::Client::new();
let response = client.get(&url)
    .header("Authorization", format!("Bearer {}", api_key))
    .send()
    .await?;
let models: OpenAIModelsResponse = response.json().await?;

// After (using API module)
use shared::api::openai_compat::get_provider_models_from_api;

let models = get_provider_models_from_api(Provider::OpenAI, api_key).await?;
```

### Step 4: Update Research CLI Usage (Optional)

Take advantage of custom prompt naming:

```bash
# Old way (still works)
research library clap "How does it compare to structopt?"

# New way (custom filename)
research library clap "comparison -> How does it compare to structopt?"
research library clap "derive-macros -> What are the derive macros?"
```

## Code Examples

### Before: Duplicated Code

**providers/base.rs:**
```rust
#[derive(Debug, Deserialize)]
pub struct OpenAIModelsResponse {
    pub data: Vec<OpenAIModel>,
}

// HTTP request code inline...
```

**providers/discovery.rs:**
```rust
#[derive(Debug, Deserialize)]
pub struct OpenAIModelsResponse {  // DUPLICATE
    pub data: Vec<OpenAIModel>,
}

// Same HTTP request code inline...  // DUPLICATE
```

### After: Centralized Code

**providers/types.rs:**
```rust
/// OpenAI-compatible API response for /v1/models endpoint
#[derive(Debug, Deserialize)]
pub struct OpenAIModelsResponse {
    pub data: Vec<OpenAIModel>,
}
```

**api/openai_compat.rs:**
```rust
/// Fetch models from a single provider's OpenAI-compatible API
#[tracing::instrument(skip(api_key))]
pub async fn get_provider_models_from_api(
    provider: Provider,
    api_key: &str,
) -> Result<Vec<String>, ProviderError> {
    // Centralized HTTP request logic
    // Used by both base.rs and discovery.rs
}
```

### ProviderModel Usage Example

```rust
use shared::providers::types::ProviderModel;
use shared::providers::Provider;

// Parse from string (TryFrom)
let model: ProviderModel = "anthropic/claude-opus-4-5-20251101".to_string().try_into()?;

// Or use static variant
let model = ProviderModel::Anthropic__ClaudeOpus__4__5__20251101;

// Get provider and model ID
assert_eq!(model.provider(), Provider::Anthropic);
assert_eq!(model.model_id(), "claude-opus-4-5-20251101");

// Display as provider/model string
assert_eq!(format!("{}", model), "anthropic/claude-opus-4-5-20251101");

// Serde serialization (automatically converts to/from string)
let json = serde_json::to_string(&model)?;
assert_eq!(json, "\"anthropic/claude-opus-4-5-20251101\"");

let deserialized: ProviderModel = serde_json::from_str(&json)?;
assert_eq!(deserialized, model);

// Use in research - extract custom filename
use research_lib::utils::filename::extract_prompt_name;

let (prompt, filename) = extract_prompt_name("features -> What are the key features?");
assert_eq!(filename, Some("features.md".to_string()));
assert_eq!(prompt, "What are the key features?");
```

## Testing Changes

### New Test Coverage

**Provider Tests:**
- `ProviderModel` parsing and serialization
- Custom prompt filename generation
- API module functions

**Run Tests:**
```bash
# All tests
cargo test

# Provider module tests
cargo test -p shared --lib providers

# API module tests
cargo test -p shared --lib api

# Research filename tests
cargo test -p research-lib --lib utils::filename
```

## Known Issues

### Remaining Code Quality Issues

The following issues were identified during code review and should be addressed:

**`.expect()` in production code (5 instances):**

1. `shared/src/tools/brave_search.rs:154`
   ```rust
   .expect("BRAVE_API_KEY environment variable must be set")
   ```

2. `research/lib/src/providers/zai.rs:38,51,103` (3 instances)
   ```rust
   .expect("Failed to build Z.ai client")
   .expect("Failed to create Z.ai client from environment")
   ```

3. `research/lib/src/lib.rs:1998`
   ```rust
   .expect("Neither RESEARCH_DIR nor HOME environment variable is set")
   ```

4. `research/lib/src/link/mod.rs:92`
   ```rust
   .expect("Neither RESEARCH_DIR nor HOME environment variable is set")
   ```

**Recommendation:** Convert these to proper error handling with `Result` types.

## Additional Resources

- **Phase Plan:** `.ai/plans/2025-12-30.provider-model-refactoring.md`
- **Code Review:** `.ai/code-reviews/20251230.provider-base-implementation.md`
- **Architecture Docs:** `research/docs/architecture.md`
- **Tracing Docs:** `docs/tracing.md`
- **Dependencies:** `docs/dependencies.md`

## Questions?

For questions or issues related to this migration:

1. Check the phase plan for technical details
2. Review the code review document for identified issues
3. See `CLAUDE.md` for development guidelines
4. Run tests to verify your changes: `cargo test`

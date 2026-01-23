# schematic-definitions

Pre-built REST API definitions using `schematic-define` primitives.

## Overview

`schematic-definitions` contains actual API definitions that use the primitives from `schematic-define`. Each API is organized in its own module with:

- A `define_*_api()` function that returns a `RestApi` definition
- Response types (structs) for the API endpoints

These definitions are consumed by `schematic-gen` to generate strongly-typed Rust clients.

## Available APIs

| API | Module | Definition Function | Endpoints | Description |
|-----|--------|---------------------|-----------|-------------|
| OpenAI | `openai` | `define_openai_api()` | 3 | OpenAI Models API (list, retrieve, delete models) |
| HuggingFace Hub | `huggingface` | `define_huggingface_hub_api()` | 26 | Hugging Face Hub API (models, datasets, spaces, repos) |
| Ollama Native | `ollama` | `define_ollama_native_api()` | 11 | Ollama local inference API (generate, chat, embeddings) |
| Ollama OpenAI | `ollama` | `define_ollama_openai_api()` | 4 | Ollama OpenAI-compatible API |
| ElevenLabs | `elevenlabs` | `define_elevenlabs_api()` | 42 | ElevenLabs TTS API (voices, text-to-speech, audio) |

## Usage

### Using the Prelude

```rust
use schematic_definitions::prelude::*;

// Get the OpenAI API definition
let api = define_openai_api();
println!("API: {} with {} endpoints", api.name, api.endpoints.len());

// Response types are also available
let model = Model {
    id: "gpt-4".to_string(),
    object: "model".to_string(),
    created: 1687882411,
    owned_by: "openai".to_string(),
};
```

### Direct Module Access

```rust
use schematic_definitions::openai::{define_openai_api, Model, ListModelsResponse};

let api = define_openai_api();
assert_eq!(api.name, "OpenAI");
assert_eq!(api.base_url, "https://api.openai.com/v1");
```

```rust
use schematic_definitions::ollama::{define_ollama_native_api, define_ollama_openai_api};

let native_api = define_ollama_native_api();
assert_eq!(native_api.name, "OllamaNative");
assert_eq!(native_api.endpoints.len(), 11);

let openai_api = define_ollama_openai_api();
assert_eq!(openai_api.name, "OllamaOpenAI");
```

```rust
use schematic_definitions::elevenlabs::define_elevenlabs_api;

let api = define_elevenlabs_api();
assert_eq!(api.name, "ElevenLabs");
assert_eq!(api.endpoints.len(), 42);
```

## OpenAI API

The OpenAI module provides a definition for the OpenAI Models API.

### Endpoints

| Endpoint | Method | Path | Response Type |
|----------|--------|------|---------------|
| ListModels | GET | `/models` | `ListModelsResponse` |
| RetrieveModel | GET | `/models/{model}` | `Model` |
| DeleteModel | DELETE | `/models/{model}` | `DeleteModelResponse` |

### Response Types

```rust
use schematic_definitions::openai::{Model, ListModelsResponse, DeleteModelResponse};

/// A model available through the OpenAI API
pub struct Model {
    pub id: String,        // e.g., "gpt-4"
    pub object: String,    // always "model"
    pub created: i64,      // Unix timestamp
    pub owned_by: String,  // e.g., "openai"
}

/// Response from ListModels endpoint
pub struct ListModelsResponse {
    pub object: String,    // always "list"
    pub data: Vec<Model>,
}

/// Response from DeleteModel endpoint
pub struct DeleteModelResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
```

### Authentication

The OpenAI API uses Bearer token authentication:

```rust
use schematic_definitions::openai::define_openai_api;
use schematic_define::AuthStrategy;

let api = define_openai_api();

// Uses Bearer token auth
assert!(matches!(api.auth, AuthStrategy::BearerToken { .. }));

// Reads token from OPENAI_API_KEY environment variable
assert_eq!(api.env_auth, vec!["OPENAI_API_KEY"]);
```

## Critical Configuration Requirements

> **⚠️ WARNING**: Incorrect configuration here causes runtime failures or compile errors!

### Response Types

Choose the correct `ApiResponse` variant for each endpoint:

| Response Type | When to Use | Generated Method |
|---------------|-------------|------------------|
| `ApiResponse::Json(Schema)` | JSON responses (most common) | `request<T>()` |
| `ApiResponse::Binary` | Audio files, images, ZIP archives | `request_bytes()` |
| `ApiResponse::Text` | Plain text responses | `request_text()` |
| `ApiResponse::Empty` | 204 No Content, fire-and-forget | `request_empty()` |

**Common Mistakes:**

```rust
// ❌ WRONG - Audio endpoints returning binary data
Endpoint {
    id: "CreateSpeech".to_string(),
    response: ApiResponse::json_type("AudioResponse"),  // Will fail at runtime!
    ...
}

// ✅ CORRECT
Endpoint {
    id: "CreateSpeech".to_string(),
    response: ApiResponse::Binary,  // Returns bytes::Bytes
    ...
}
```

### Module Path Configuration

The `module_path` field controls where the generator imports types from:

| Scenario | Configuration |
|----------|---------------|
| API name matches module name | `module_path: None` (auto-inferred) |
| API name differs from module | **MUST set `module_path`** |
| Multiple APIs in one module | **MUST set `module_path` for each** |

**Example - Ollama has two APIs in one module:**

```rust
// definitions/src/ollama/mod.rs exports both APIs

pub fn define_ollama_native_api() -> RestApi {
    RestApi {
        name: "OllamaNative".to_string(),
        module_path: Some("ollama".to_string()),  // ← REQUIRED
        ...
    }
}

pub fn define_ollama_openai_api() -> RestApi {
    RestApi {
        name: "OllamaOpenAI".to_string(),
        module_path: Some("ollama".to_string()),  // ← REQUIRED
        ...
    }
}
```

**What happens without explicit `module_path`:**

| API Name | Inferred Path | Actual Module | Result |
|----------|---------------|---------------|--------|
| `OllamaNative` | `ollamanative` | `ollama` | ❌ Compile error: `schematic_definitions::ollamanative` not found |
| `ElevenLabs` | `elevenlabs` | `elevenlabs` | ✅ Works (names match) |

### Verification Checklist

After adding or modifying an API definition:

```bash
# 1. Generate the code
cargo run -p schematic-gen -- --api YOUR_API --output schematic/schema/src

# 2. Check for correct response methods
grep -n "request_bytes\|request_text\|request_empty" schematic/schema/src/YOUR_API.rs

# 3. Verify it compiles
cargo check -p schematic-schema

# 4. For binary endpoints, verify convenience methods exist
grep -n "pub async fn create_speech\|pub async fn download" schematic/schema/src/YOUR_API.rs
```

## Adding New APIs

To add a new API definition:

1. Create a new module directory: `src/{api_name}/`
2. Add `mod.rs` with the `define_{api_name}_api()` function
3. Add `types.rs` with response types
4. **Choose correct `ApiResponse` for each endpoint** (see above)
5. **Set `module_path` if API name differs from module name**
6. Export from `src/lib.rs`
7. Add to the prelude in `src/prelude.rs`
8. **Run verification checklist above**

### Example Structure

```
src/
├── lib.rs
├── prelude.rs
├── openai/
│   ├── mod.rs      # define_openai_api()
│   └── types.rs    # Model, ListModelsResponse, etc.
└── anthropic/      # Future API
    ├── mod.rs
    └── types.rs
```

## Dependencies

- `schematic-define` - Provides the `RestApi`, `Endpoint`, `AuthStrategy` primitives
- `serde` - Serialization for response types

## License

AGPL-3.0-only

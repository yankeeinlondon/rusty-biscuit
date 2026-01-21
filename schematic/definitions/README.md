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

## Adding New APIs

To add a new API definition:

1. Create a new module directory: `src/{api_name}/`
2. Add `mod.rs` with the `define_{api_name}_api()` function
3. Add `types.rs` with response types
4. Export from `src/lib.rs`
5. Add to the prelude in `src/prelude.rs`

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

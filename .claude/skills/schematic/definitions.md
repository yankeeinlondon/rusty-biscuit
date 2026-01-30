# schematic-definitions

Pre-built REST API definitions using `schematic-define` primitives.

## Available APIs

| API | Function | Endpoints | Auth | Key Env Var |
|-----|----------|-----------|------|-------------|
| Anthropic | `define_anthropic_api()` | 4 | `ApiKey { header: "X-Api-Key" }` | `ANTHROPIC_API_KEY` |
| OpenAI | `define_openai_api()` | 3 | `BearerToken` | `OPENAI_API_KEY` |
| HuggingFace Hub | `define_huggingface_hub_api()` | 28+ | `BearerToken` | `HUGGINGFACE_API_KEY` |
| ElevenLabs REST | `define_elevenlabs_rest_api()` | 45+ | `ApiKey { header: "xi-api-key" }` | `ELEVEN_LABS_API_KEY` |
| ElevenLabs WebSocket | `define_elevenlabs_websocket_api()` | 2 | `ApiKey` | `ELEVEN_LABS_API_KEY` |
| Ollama Native | `define_ollama_native_api()` | 11 | `None` | (local) |
| Ollama OpenAI | `define_ollama_openai_api()` | 4 | `None` | (local) |
| EMQX Basic | `define_emqx_basic_api()` | 30+ | `Basic` | `EMQX_USER`, `EMQX_PASSWORD` |
| EMQX Bearer | `define_emqx_bearer_api()` | 32+ | `BearerToken` | `EMQX_API_KEY` |

## Anthropic API

**Base URL**: `https://api.anthropic.com/v1`

**Default Headers**: `anthropic-version: 2023-06-01`

| Endpoint | Method | Path | Response |
|----------|--------|------|----------|
| CreateMessage | POST | `/messages` | `CreateMessageResponse` |
| CountTokens | POST | `/messages/count_tokens` | `CountTokensResponse` |
| ListModels | GET | `/models` | `ListModelsResponse` |
| RetrieveModel | GET | `/models/{model_id}` | `Model` |

```rust
use schematic_definitions::anthropic::define_anthropic_api;

let api = define_anthropic_api();
assert_eq!(api.name, "Anthropic");
assert_eq!(api.endpoints.len(), 4);
```

## OpenAI API

**Base URL**: `https://api.openai.com/v1`

| Endpoint | Method | Path | Response |
|----------|--------|------|----------|
| ListModels | GET | `/models` | `ListModelsResponse` |
| RetrieveModel | GET | `/models/{model}` | `Model` |
| DeleteModel | DELETE | `/models/{model}` | `DeleteModelResponse` |

## ElevenLabs API

**Base URL**: `https://api.elevenlabs.io/v1`

**REST Categories**:
- **TTS**: CreateSpeech, StreamSpeech, etc.
- **Voices**: ListVoices, GetVoice, CreateVoice, etc.
- **PVC**: CreatePVC, GetPVC, DeletePVC
- **History**: ListHistory, GetHistoryItem, etc.
- **Models**: ListModels
- **Workspace**: Account, subscription management

**WebSocket Endpoints**:
- `TextToSpeech` - Stream text and receive audio chunks
- `MultiContextTextToSpeech` - Multi-context streaming with voice switching

```rust
use schematic_definitions::elevenlabs::{define_elevenlabs_rest_api, define_elevenlabs_websocket_api};

let rest_api = define_elevenlabs_rest_api();
let ws_api = define_elevenlabs_websocket_api();
assert_eq!(ws_api.endpoints.len(), 2);
```

## HuggingFace Hub API

**Base URL**: `https://huggingface.co/api`

Categories:
- **Models**: List, search, download model files
- **Datasets**: List, search datasets
- **Spaces**: List, search spaces
- **Repos**: Create, delete repositories
- **Inference**: Run inference on hosted models

## Ollama APIs

**Base URL**: `http://localhost:11434`

Two API variants in shared `ollama` module:

```rust
use schematic_definitions::ollama::{define_ollama_native_api, define_ollama_openai_api};

let native = define_ollama_native_api();  // 11 endpoints
let openai = define_ollama_openai_api();  // 4 endpoints (OpenAI-compatible)
```

**Native Endpoints**: Generate, Chat, Embed, ListModels, ShowModel, CreateModel, CopyModel, DeleteModel, PullModel, PushModel, ListRunning

**OpenAI-Compatible**: Chat, Completions, Embeddings, Models

## EMQX APIs

**Base URL**: `http://localhost:18083/api/v5`

Two auth variants in shared `emqx` module:

```rust
use schematic_definitions::emqx::{define_emqx_basic_api, define_emqx_bearer_api};

let basic = define_emqx_basic_api();   // Basic auth
let bearer = define_emqx_bearer_api(); // Bearer token (has Login/Logout)
```

Categories: Clients, Topics, Subscriptions, Rules, Bridges, Plugins, Stats, Alarms

## Adding New APIs

### Directory Structure

```
definitions/src/
├── lib.rs          # Module exports
├── prelude.rs      # Re-exports for convenience
├── myapi/
│   ├── mod.rs      # define_myapi_api() function
│   └── types.rs    # Response types (serde structs)
```

### Checklist

1. Create module directory with `mod.rs` and `types.rs`
2. Define `define_myapi_api() -> RestApi` function
3. Choose correct `ApiResponse` for each endpoint
4. Set `module_path` if API name differs from module name
5. Add types with `#[derive(Debug, Clone, Default, Serialize, Deserialize)]`
6. Export from `lib.rs` and `prelude.rs`
7. Run verification:
   ```bash
   cargo run -p schematic-gen -- generate --api myapi --dry-run
   cargo check -p schematic-schema
   ```

### Type Naming

```rust
// Response types - any name works
pub struct User { ... }
pub struct ListUsersResponse { ... }

// Request body types - use *Body suffix to avoid collision
pub struct CreateUserBody { ... }  // NOT CreateUserRequest
```

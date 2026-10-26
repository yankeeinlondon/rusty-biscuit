# schematic-definitions

Pre-built REST API definitions using `schematic-define` primitives.

## Available APIs

| API | Function | Endpoints | Auth | Env vars the client reads (in order) |
|-----|----------|-----------|------|-------------|
| Anthropic | `define_anthropic_api()` | 4 | `ApiKey { header: "X-Api-Key" }` | `ANTHROPIC_API_KEY` |
| OpenAI | `define_openai_api()` | 258 | `BearerToken` | none (imported definition has no env mapping; pass a `Credential`) |
| HuggingFace Hub | `define_huggingface_hub_api()` | 26 | `BearerToken` | `HF_TOKEN`, `HUGGING_FACE_API_KEY`, `HF_API_KEY` |
| ElevenLabs REST | `define_elevenlabs_rest_api()` | 42 | `ApiKey { header: "xi-api-key" }` | `ELEVEN_LABS_API_KEY`, `ELEVENLABS_API_KEY` |
| ElevenLabs WebSocket | `define_elevenlabs_websocket_api()` | 2 | `ApiKey` | `ELEVEN_LABS_API_KEY` |
| Ollama Native | `define_ollama_native_api()` | 11 | `None` | (local) |
| Ollama OpenAI | `define_ollama_openai_api()` | 4 | `None` | (local) |
| EMQX Basic | `define_emqx_basic_api()` | 36 | `Basic` | username `EMQX_API_KEY`, password `EMQX_API_SECRET` |
| EMQX Bearer | `define_emqx_bearer_api()` | 38 | `BearerToken` | `EMQX_TOKEN` |

## Anthropic API

**Base URL**: `https://api.anthropic.com/v1`

**Default Headers**: `anthropic-version: 2023-06-01`

| Endpoint | Method | Path | Response |
|----------|--------|------|----------|
| CreateMessage | POST | `/messages` | `MessageResponse`, or an SSE stream of `MessageStreamEvent` ending at `message_stop` (`CreateMessageResponse` enum) |
| CountTokens | POST | `/messages/count_tokens` | `CountTokensResponse` |
| ListModels | GET | `/models` | `ListModelsResponse` |
| RetrieveModel | GET | `/models/{model_id}` | `ModelInfo` |

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

Two API variants in shared `ollama` module with distinct request suffixes:

```rust
use schematic_definitions::ollama::{define_ollama_native_api, define_ollama_openai_api};

let native = define_ollama_native_api();  // 11 endpoints, suffix: "NativeRequest"
let openai = define_ollama_openai_api();  // 4 endpoints, suffix: "OaiRequest"
// Both share module_path: "ollama" → combined into ollama.rs
```

**Native Endpoints**: Generate, Chat, Embeddings, ListModels, ShowModel, PullModel, PushModel, CopyModel, DeleteModel, CreateModel, ListRunningModels

**OpenAI-Compatible**: ChatCompletions, Completions, Embeddings, ListModels

`Generate`, `Chat`, `PullModel`, `PushModel`, and `CreateModel` declare an
NDJSON stream plus a buffered JSON reply; `ChatCompletions` and `Completions`
declare JSON plus SSE ending at `[DONE]`. No registered type may be named
`{Id}Response`, the generated success enum's name, so the native buffered
replies are `GenerateCompletion`/`ChatCompletion` (formerly
`GenerateResponse`/`ChatResponse`).

## EMQX APIs

**Base URL**: `http://localhost:18083/api/v5`

Two auth variants in shared `emqx` module:

```rust
use schematic_definitions::emqx::{define_emqx_basic_api, define_emqx_bearer_api};

let basic = define_emqx_basic_api();   // Basic auth
let bearer = define_emqx_bearer_api(); // Bearer token (has Login/Logout)
```

Categories: Clients, Topics, Subscriptions, Rules, Bridges, Plugins, Stats, Alarms

## Facts that frequently drift

- OpenAI is **generated** from `schematic/specs/openai/openapi.yaml`; never
  hand-edit `definitions/src/openai/`, re-run
  `just -f schematic/justfile import-openai`. It excludes the deprecated
  Assistants family. Its imported definition has no env mapping, so the
  client reads no variable. The hand-written `openai_pagination` overlay
  assigns collections, rejects unknown ids, and narrows every endpoint to one
  success shape and no errors except deliberately migrated ones
  (`GetResponse`).
- LM Studio: 6 endpoints, bearer via `LM_API_TOKEN`. GitHub: 16 endpoints,
  bearer via `GITHUB_TOKEN` or `GH_TOKEN`. Gitea: 15 endpoints, API key via
  `GITEA_TOKEN` with the `token ` prefix (not Bearer); `GetGitTree` takes an
  optional Boolean `recursive`. GitLab's wire `ref` query parameter is
  spelled `git_ref` in Rust.
- Bitbucket `GetSource` declares `Directory` (JSON listing) and `File`
  success shapes.
- No shipped definition declares GraphQL operations yet.

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
2. Write a fallible `module_group() -> Result<Vec<RestApi>, DefinitionError>`
   over one `DefinitionContext`, and `define_myapi_api() -> RestApi` that
   unwraps it
3. Choose correct success shapes (`returns_*`/`returns(Success::…)`, streams
   as `Success::sse`/`ndjson`/`bytes`) and declared errors (`.errors(..)`,
   `.default_errors(..)`) for each endpoint; see [`responses.md`](./responses.md)
4. Set `module_path` if API name differs from module name. Without it,
   `schematic_definitions::naming::resolve_module_name` (the only module-name
   resolver; Rust, OpenAPI, Postman, and `import` call it) drops a
   Native/Client/Service/Hub/Api/Sdk suffix or lowercases the name. Grouping
   (`apis_by_module`) reads the registration's `module`, which an L1 test
   requires to equal it; names differing only by case are refused
5. Add types with `#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]`
   and a `Registration` impl (`definition_type!` or the nominal macro)
6. Export from `lib.rs` and `prelude.rs`, register the API with one entry in
   `registry/inventory.rs` (`rest(..)` or `websocket(..)`; every API list,
   `--api` name, Cargo feature, and output derives from it), and add the API's
   row to `definitions/tests/api_policy.rs` (and `pagination_coverage/` for
   lists). Test tables that need literal artifact paths are not derived but
   are guarded against the inventory, so they fail until the module is added:
   `shipped_modules!` in `gen/tests/l1/postman_runnable.rs`, the artifact
   lists in `extension_round_trip.rs` and `fixed_header_export.rs`
7. Run verification:
   ```bash
   cargo run -p schematic-gen -- generate --api myapi --dry-run
   just -f schematic/justfile generate
   cargo check --manifest-path schematic/schema/Cargo.toml   # excluded from the workspace
   ```

### Type Naming

```rust
// Response types - any name works
pub struct User { ... }
pub struct ListUsersResponse { ... }

// Request body types - use *Body suffix to avoid collision
pub struct CreateUserBody { ... }  // NOT CreateUserRequest
```

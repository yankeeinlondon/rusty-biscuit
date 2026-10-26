# schematic-definitions

Pre-built REST and WebSocket API definitions for Schematic's generated clients.
REST modules expose a fallible `module_group()` that builds their API definitions
from one registration context. Every API it returns carries that context's
schema components in `RestApi::schemas`, and each endpoint's JSON body names
those components. Infallible factories such as `define_github_api()` unwrap the
same assembly.

```rust
let apis = schematic_definitions::github::module_group()?;
let api = &apis[0];
assert!(api.validate_references().is_ok());
```

Every shipped API is one entry in `registry::registrations()`, from which
`all_apis()`, `apis_by_module()`, and the generator's API lists derive. See
[Register an API](../docs/generation/workflow.md#register-an-api).

## Available APIs

| API | Module | Definition Function | Endpoints | Description |
|-----|--------|---------------------|-----------|-------------|
| Anthropic | `anthropic` | `define_anthropic_api()` | 4 | Anthropic Messages API for Claude AI and agent tool use |
| Artificial Analysis Data | `artificial_analysis` | `define_artificial_analysis_data_api()` | 6 | Artificial Analysis free data API for LLM and media-model benchmarks |
| Artificial Analysis CritPt | `artificial_analysis` | `define_artificial_analysis_critpt_api()` | 1 | Artificial Analysis CritPt benchmark evaluation API |
| Bitbucket | `bitbucket` | `define_bitbucket_api()` | 15 | Bitbucket Cloud REST API for repos, PRs, issues, tags |
| OpenAI | `openai` | `define_openai_api()` | 258 | Full OpenAI platform surface, imported from the vendored spec (see `specs/openai/`) |
| HuggingFace Hub | `huggingface` | `define_huggingface_hub_api()` | 26 | Hugging Face Hub API (models, datasets, spaces, repos) |
| LM Studio | `lmstudio` | `define_lmstudio_api()` | 6 | LM Studio local inference API |
| Ollama Native | `ollama` | `define_ollama_native_api()` | 11 | Ollama local inference API (generate, chat, embeddings) |
| Ollama OpenAI | `ollama` | `define_ollama_openai_api()` | 4 | Ollama OpenAI-compatible API |
| ElevenLabs REST | `elevenlabs` | `define_elevenlabs_rest_api()` | 42 | ElevenLabs TTS REST API (voices, text-to-speech, audio) |
| ElevenLabs WebSocket | `elevenlabs` | `define_elevenlabs_websocket_api()` | 2 | ElevenLabs TTS WebSocket streaming API |
| EMQX Basic | `emqx` | `define_emqx_basic_api()` | 36 | EMQX MQTT Broker REST API with Basic auth |
| EMQX Bearer | `emqx` | `define_emqx_bearer_api()` | 38 | EMQX MQTT Broker REST API with Bearer token auth |
| GitHub | `github` | `define_github_api()` | 16 | GitHub REST API for repos, PRs, issues, releases |
| GitLab | `gitlab` | `define_gitlab_api()` | 19 | GitLab REST API for repos, MRs, issues, releases |
| Gitea | `gitea` | `define_gitea_api()` | 15 | Gitea REST API for self-hosted Git forge instances |
| Eversolo | `eversolo` | `define_eversolo_api()` | 24 | Eversolo DMP-A8 local HTTP control API |
| Samsung Smart TV REST | `samsung_smart_tv` | `define_samsung_smart_tv_api()` | 7 | Samsung S95C-focused LAN API (Smart View) |
| Samsung Smart TV Remote WS | `samsung_smart_tv::remote_ws` | `define_samsung_smart_tv_remote_ws_api()` | 2 | Samsung remote control WebSocket channel |
| Unfolded Circle Core REST | `unfolded_circle::core_rest` | `define_unfolded_circle_core_rest_api()` | 11 | Unfolded Circle Core REST API |
| Unfolded Circle Core WS | `unfolded_circle::core_ws` | `define_unfolded_circle_core_ws_api()` | 4 | Unfolded Circle Core WebSocket API |
| Unfolded Circle Dock WS | `unfolded_circle::dock_ws` | `define_unfolded_circle_dock_ws_api()` | 1 | Unfolded Circle Dock WebSocket API |
| Unfolded Circle Integration WS | `unfolded_circle::integration_ws` | `define_unfolded_circle_integration_ws_api()` | 1 | Unfolded Circle Integration WS API |

## Authoring and generation

Use [typed registration](../docs/typed-registration.md) and
[definition builders](../docs/definition-builders.md) for new REST APIs. Explicitly
register models in the intended component order before building endpoints. Shared
API variants assemble in one context; endpoint references are checked against
that context before the registry is returned.

[Pagination](../docs/pagination.md) uses provider-local strategy helpers,
API defaults, and endpoint `.pagination(Collection)` assignments. Gitea's
`GetGitTree` accepts an optional `recursive` Boolean query parameter. GitLab's
repository-file endpoint uses the wire query name `ref` with the Rust name
`git_ref`; route paths contain no query strings.

GitHub declares its rate-limit headers (`x-ratelimit-limit`,
`x-ratelimit-remaining`, `x-ratelimit-reset` in epoch seconds, and
`retry-after`); this is documentation metadata, and nothing retries. Bitbucket
and EMQX Basic keep their username and password variable lists inside
`AuthStrategy::Basic`. Required query parameters, query enums, and array
`style`/`explode` flags in these definitions shape the generated constructors
and wire form; see [request inputs](../docs/generation/request-side.md).

Several endpoints declare more than one [success shape](../docs/responses-and-errors.md).
Anthropic's `CreateMessage` and Ollama's `Generate` and `Chat` return JSON or
a stream, chosen by the reply's `Content-Type`; Ollama's pull, push, and create
endpoints, LM Studio's `Chat`, and ElevenLabs' `StreamSpeech` and
`StreamSpeechWithTimestamps` stream; and Bitbucket's `GetSource` returns a
JSON directory listing or the file's bytes. A provider's error event inside a
stream is a variant of the stream's item type.

OpenAI's 258 endpoints are [imported](../docs/io/import-openapi.md) from the
vendored document. Reimport with `just -f schematic/justfile import-openai`;
its handwritten pagination overlay is preserved. Generate client and export
artifacts with `just -f schematic/justfile generate`.

See [validation](../docs/generation/validation.md),
[request inputs](../docs/generation/request-side.md), and
[dependencies](../docs/dependencies.md) for behavior and checks. Generated
sources belong to `schematic-schema` and are never edited by hand.

See [the compiling authoring example](../docs/authoring-examples.md) for nominal models, public
aliases, containers, and complete context-based API assembly.

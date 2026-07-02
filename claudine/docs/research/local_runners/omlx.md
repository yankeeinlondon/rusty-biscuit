---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7

summary: oMLX is an open-source, macOS-native MLX model server with paged SSD KV caching, continuous batching, and OpenAI- and Anthropic-compatible endpoints.
homepage: https://omlx.ai
docs_url: https://github.com/jundot/omlx
repo_url: https://github.com/jundot/omlx
api_reference_url: http://localhost:8000/openapi.json
open_source: full

has_official_schema: formal
schema_url: http://localhost:8000/openapi.json

default_port: 8000
default_bind: 127.0.0.1
auth: optional_api_key
auth_notes: >
  `omlx serve` starts without auth unless `--api-key` is passed. The macOS app setup
  wizard and Homebrew service flow prompt for an API key and write it to
  `~/.omlx/settings.json` with `skip_api_key_verification: false`, making auth
  required on hosts installed that way. When auth is enabled, both OpenAI
  (`Authorization: Bearer <key>`) and Anthropic (`x-api-key <key>`) headers are
  accepted. The `skip_api_key_verification` toggle skips verification for
  localhost requests only.

platforms:
  - os: macos
    support: native
    binary: omlx
    alt_binaries: ["oMLX.app/Contents/MacOS/oMLX", "oMLX.app/Contents/MacOS/omlx-cli", "omlx-server"]
    install: ["DMG from GitHub releases", "brew tap jundot/omlx https://github.com/jundot/omlx && brew install omlx", "pip install -e . from source"]
    process_model: both
    service: brew services (launchd) when using Homebrew; macOS app menubar/login item when using the DMG; foreground `omlx serve` from CLI
    notes: Observed on this host as /Applications/oMLX.app (v0.4.4) with a `~/.omlx/bin/omlx` shim. Requires macOS 15+ and Apple Silicon (M1/M2/M3/M4). Intel Macs are not supported.
  - os: linux
    support: unsupported
    service: none
    notes: No Linux artifact or supported build path. Source install is macOS-only (Apple Silicon + MLX).
  - os: windows
    support: unsupported
    service: none
    notes: No Windows artifact or supported build path.

api_standards:
  - standard: openai_compatible
    supported: yes
    base_url: http://localhost:8000/v1
    key_paths:
      - /v1/models
      - /v1/models/status
      - /v1/chat/completions
      - /v1/completions
      - /v1/embeddings
      - /v1/rerank
      - /v1/audio/speech
      - /v1/audio/transcriptions
      - /v1/audio/process
      - /v1/responses
    auth: optional_api_key
    since_version: "v0.1.0"
    deviations:
      - /v1/responses is supported as a stateful OpenAI-compatible endpoint.
      - /v1/rerank follows the Cohere/Jina-compatible shape.
      - Audio speech/transcriptions endpoints stream/generated audio and ASR output.
    docs_url: https://github.com/jundot/omlx/blob/main/README.md#api-compatibility
  - standard: anthropic_compatible
    supported: yes
    base_url: http://localhost:8000
    key_paths:
      - /v1/messages
      - /v1/messages/count_tokens
    auth: optional_api_key
    since_version: "v0.1.0"
    deviations:
      - Anthropic SDKs append /v1/messages themselves, so base_url omits it.
      - Thinking blocks emit a stable placeholder signature (`omlx-reasoning`) because some models do not produce signatures.
      - Prefix-cache usage is reported via `cache_creation_input_tokens` and `cache_read_input_tokens`.
    docs_url: https://github.com/jundot/omlx/blob/main/README.md#api-compatibility
  - standard: native
    supported: yes
    base_url: http://localhost:8000/admin/api
    key_paths:
      - /admin/api/models
      - /admin/api/models/{model_id}/load
      - /admin/api/models/{model_id}/unload
      - /admin/api/models/{model_id}/settings
      - /admin/api/global-settings
      - /admin/api/server-info
      - /admin/api/stats
      - /admin/api/hf/models
      - /admin/api/hf/download
      - /admin/api/cache/probe
    auth: required_api_key
    since_version: "v0.1.0"
    deviations:
      - Native admin endpoints require admin authentication (the configured API key is not sufficient; admin login/session is required).
      - Many admin endpoints overlap with the web dashboard at /admin.
    docs_url: http://localhost:8000/docs

metadata_endpoints:
  - purpose: health
    method: get
    path: /health
    gated_by: ""
    auth_gated: false
    response_hint: '{"status":"healthy","default_model":"...","engine_pool":{"model_count":N,"loaded_count":N,...}}'
    notes: Observed on this host. Ungated; ideal detector probe when auth is enabled because it does not require a key.
  - purpose: model_list
    method: get
    path: /v1/models
    gated_by: ""
    auth_gated: true
    response_hint: '{"object":"list","data":[{"id":"...","object":"model","owned_by":"omlx"}]}'
    notes: Observed on this host with Authorization header. Lists all discovered models, including LLM, VLM, embedding, reranker, audio, and builtin MarkItDown.
  - purpose: loaded_models
    method: get
    path: /v1/models/status
    gated_by: ""
    auth_gated: true
    response_hint: '{"model_count":N,"loaded_count":N,"models":[{"id":"...","loaded":true|false,"model_type":"..."}]}'
    notes: Observed on this host. Returns per-model load state, model_type, engine_type, source_type, and memory estimates.
  - purpose: version
    method: get
    path: /openapi.json
    gated_by: ""
    auth_gated: false
    response_hint: '{"info":{"title":"oMLX API","version":"..."}}'
    notes: No dedicated HTTP version endpoint. Version is available from `omlx --version` and from the `version` field inside `/openapi.json`.
  - purpose: admin_ui
    method: get
    path: /admin
    gated_by: ""
    auth_gated: false
    response_hint: "oMLX Admin Dashboard HTML"
    notes: Web dashboard for model management, chat, benchmarks, settings, and one-click agent integrations.
  - purpose: load_model
    method: post
    path: /v1/models/{model_id}/load
    gated_by: ""
    auth_gated: true
    response_hint: '{"id":"...","loaded":true}'
    notes: Load a model into the engine pool.
  - purpose: unload_model
    method: post
    path: /v1/models/{model_id}/unload
    gated_by: ""
    auth_gated: true
    response_hint: '{"id":"...","loaded":false}'
    notes: Unload a model from the engine pool.
  - purpose: metrics
    method: get
    path: /admin/api/stats
    gated_by: ""
    auth_gated: true
    response_hint: "server statistics JSON"
    notes: Admin-auth gated; runtime stats are also surfaced via /health.engine_pool for unauthenticated health checks.
  - purpose: other
    method: post
    path: /admin/api/hf/download
    gated_by: ""
    auth_gated: true
    response_hint: '{"task_id":"..."}'
    notes: Download a model from HuggingFace by repo_id.
  - purpose: other
    method: post
    path: /admin/api/cache/probe
    gated_by: ""
    auth_gated: true
    response_hint: '{"blocks":[{"location":"hot_ssd|disk_ssd|cold",...}]}'
    notes: Classify per-prompt cache state for a model/message list.
  - purpose: other
    method: get
    path: /v1/mcp/servers
    gated_by: ""
    auth_gated: true
    response_hint: "MCP server list JSON"
    notes: MCP server list.
  - purpose: other
    method: get
    path: /v1/mcp/tools
    gated_by: ""
    auth_gated: true
    response_hint: "MCP tool list JSON"
    notes: MCP tool list.
  - purpose: other
    method: post
    path: /v1/mcp/execute
    gated_by: ""
    auth_gated: true
    response_hint: "MCP tool execution JSON"
    notes: MCP tool execution.

detection:
  - os: macos
    method: binary
    target: omlx
    expect: "omlx: Production-ready LLM server for Apple Silicon"
    confidence: observed
    notes: Observed on this host at /opt/homebrew/bin/omlx symlinked to ~/.omlx/bin/omlx, which execs /Applications/oMLX.app/Contents/MacOS/omlx-cli.
  - os: macos
    method: app_bundle
    target: /Applications/oMLX.app
    expect: 'CFBundleIdentifier: app.omlx'
    confidence: observed
    notes: Observed on this host. Bundle version 0.4.4.
  - os: macos
    method: process
    target: omlx-server
    expect: "omlx-server process running"
    confidence: observed
    notes: Observed on this host alongside /Applications/oMLX.app/Contents/MacOS/oMLX.
  - os: all
    method: port
    target: "8000"
    expect: ""
    confidence: documented
    notes: Default port is 8000 but shared with vLLM and other runners; an HTTP probe is required to disambiguate.
  - os: all
    method: http
    target: GET /health
    expect: '{"status":"healthy","engine_pool":{"model_count":N,"loaded_count":N}}'
    confidence: observed
    notes: Strong identity marker and ungated even when auth is enabled. Observed locally.
  - os: all
    method: http
    target: GET /openapi.json
    expect: '{"info":{"title":"oMLX API"}}'
    confidence: observed
    notes: FastAPI/OpenAPI formal schema; confirms identity regardless of auth state.
  - os: macos
    method: config_file
    target: ~/.omlx/settings.json
    expect: '"version": "1.0"'
    confidence: observed
    notes: Observed on this host. Holds server host/port, auth, model_dirs, cache, and integration settings.
  - os: macos
    method: config_file
    target: ~/Library/Application Support/oMLX/base-path
    expect: "path to oMLX base directory"
    confidence: source_code
    notes: A single-line bootstrap file read by the ~/.omlx/bin/omlx shim; if present, it relocates the effective OMLX_BASE_PATH and therefore ~/.omlx/settings.json.

config_mechanism: mixed

config_files:
  - os: macos
    path: ~/.omlx/settings.json
    format: json
    role: primary server settings
    notes: Observed on this host. Holds server host/port, log level, model_dirs, auth, cache, sampling defaults, and integration model mappings.
  - os: macos
    path: ~/.omlx/model_settings.json
    format: json
    role: per-model settings
    notes: Observed on this host. Holds per-model overrides such as aliases, thinking, pinning, TTL, and draft-model options.
  - os: macos
    path: ~/Library/Application Support/oMLX/config.json
    format: json
    role: app bootstrap config
    notes: App support directory used by the native SwiftUI menubar app.
  - os: macos
    path: ~/Library/Application Support/oMLX/base-path
    format: text
    role: base-path pointer
    notes: If present, the omlx CLI shim exports OMLX_BASE_PATH from this file, relocating config and data directories.

env_vars:
  - name: OMLX_BASE_PATH
    effect: "Relocates the oMLX data/config home (default ~/.omlx). Read from ~/Library/Application Support/oMLX/base-path by the CLI shim."
  - name: OMLX_HOST
    effect: "Overrides server.bind host (default 127.0.0.1)."
  - name: OMLX_PORT
    effect: "Overrides server.port (default 8000)."
  - name: OMLX_LOG_LEVEL
    effect: "Overrides server.log_level (trace/debug/info/warning/error)."
  - name: OMLX_MODEL_DIR
    effect: "Overrides the model directory (legacy single path; model_dirs array in settings.json is preferred)."
  - name: OMLX_PAGED_SSD_CACHE_DIR
    effect: "Directory for paged SSD KV cache (enables persistent prefix cache)."
  - name: OMLX_PAGED_SSD_CACHE_MAX_SIZE
    effect: "Maximum SSD cache size, e.g. 100GB."
  - name: OMLX_HOT_CACHE_ONLY
    effect: "When true, disables SSD cache tier and keeps KV blocks in memory only."
  - name: OMLX_MAX_CONCURRENT_REQUESTS
    effect: "Maximum requests processed simultaneously (default 8)."
  - name: OMLX_EMBEDDING_BATCH_SIZE
    effect: "Embedding forward-pass batch size (default 32)."
  - name: OMLX_API_KEY
    effect: "Sets the API key for client authentication."
  - name: OMLX_SECRET_KEY
    effect: "Sets the admin/session signing secret."
  - name: OMLX_MCP_CONFIG
    effect: "Path to MCP server configuration file (JSON/YAML)."
  - name: OMLX_HF_ENDPOINT
    effect: "Custom HuggingFace Hub endpoint URL (e.g. mirror)."
  - name: OMLX_HF_CACHE_ENABLED
    effect: "Whether to discover models from the local HuggingFace cache."
  - name: OMLX_HTTP_PROXY / OMLX_HTTPS_PROXY / OMLX_NO_PROXY / OMLX_CA_BUNDLE
    effect: "Network proxy and TLS bundle overrides."

model_id_grammar: |
  Models are addressed by the directory name under the configured model directory.
  Examples: `Qwen3.6-35B-A3B-oQ6`, `mlx-community/MiniCPM-V-4.6-bf16` (two-level
  `{owner}/{model}` subfolders supported since v0.3.9.dev2). A model alias can be
  set per-model in `model_settings.json` or the admin UI; `/v1/models` returns the
  alias, and requests accept both alias and directory name. Profiles can be exposed
  as `<model>:<profile>` or `<alias>:<profile>` and served from the same loaded
  engine with per-request settings overlays.

model_formats:
  - mlx

model_acquisition:
  - method: manual
    example: "Place a MLX-format model directory under ~/.omlx/models or the configured model_dirs path: config.json + *.safetensors files."
    notes: Observed on this host under /Volumes/Fast Bastard/models/omlx/.
  - method: huggingface
    example: "POST /admin/api/hf/download with {\"repo_id\":\"mlx-community/Llama-3.2-3B-Instruct-4bit\"}"
    notes: Downloads from HuggingFace via the admin dashboard or native API.
  - method: in_app
    example: "Click Download in the admin dashboard model browser."
    notes: Stores under the configured model directory, using `{owner}/{model}` subfolders since v0.3.9.dev2.

model_store_paths:
  - os: macos
    path: ~/.omlx/models
    notes: Default model store. Configurable via settings.json `model_dirs` (array) or legacy `model_dir`; observed on this host relocated to /Volumes/Fast Bastard/models/omlx/.

hardware_acceleration:
  - metal
  - cpu

concurrency:
  multi_model: true
  parallel_requests: true
  notes: EnginePool loads LLM, VLM, embedding, reranker, and audio models simultaneously with LRU eviction; continuous batching via mlx-lm BatchGenerator.

streaming_sse: true
tool_calling: conditional
tool_calling_notes: Depends on the model's chat template and mlx-lm tool parsers. Auto-detected for Llama/Qwen/DeepSeek JSON, Qwen3.5 XML, Gemma, GLM, MiniMax, Mistral, Kimi K2, and Longcat formats.

embeddings: true
rerank: true
web_ui_url: http://localhost:8000/admin

integration_hooks:
  - command: omlx launch claude
    effect: "Interactively select a model and exec Claude Code with ANTHROPIC_BASE_URL, ANTHROPIC_AUTH_TOKEN, and model tiers pointing at oMLX."
    notes: Added in v0.3.9.dev1. Supports --opus/--sonnet/--haiku model overrides.
  - command: omlx launch codex
    effect: "Launch Codex CLI configured to use the running oMLX server."
    notes: ""
  - command: omlx launch opencode
    effect: "Launch OpenCode configured to use the running oMLX server."
    notes: ""
  - command: omlx launch openclaw
    effect: "Launch OpenClaw configured to use the running oMLX server."
    notes: Supports --tools-profile minimal/coding/messaging/full.
  - command: omlx launch pi
    effect: "Launch Pi CLI configured to use the running oMLX server."
    notes: ""
  - command: omlx launch hermes
    effect: "Launch Hermes Agent configured to use the running oMLX server."
    notes: ""
  - command: omlx launch copilot
    effect: "Launch GitHub Copilot CLI configured to use the running oMLX server."
    notes: Added in v0.3.9.dev2.
  - command: omlx launch codex_app
    effect: "Launch Codex App Desktop configured to use the running oMLX server."
    notes: ""

traps:
  - "The CLI shim reads ~/Library/Application Support/oMLX/base-path and exports OMLX_BASE_PATH, so ~/.omlx is not a fixed location."
  - "settings.json has both `model_dirs` (active array) and legacy `model_dir`; edits must target `model_dirs`."
  - "Default port 8000 is shared with vLLM and other runners; /health or /openapi.json is required to confirm identity."
  - "API-key enforcement depends on install path: `omlx serve` defaults to no auth, but the macOS app/Homebrew setup writes a required key."
  - "Admin endpoints under /admin/api require admin session auth, not the same Bearer token used for /v1/*."
  - "The web UI is offline-first (vendored CDN assets) but still served from /admin."

opencode_example: '{"provider":{"omlx":{"npm":"@ai-sdk/openai-compatible","name":"oMLX (local)","options":{"baseURL":"http://localhost:8000/v1"},"models":{"Qwen3.6-35B-A3B-oQ6":{"name":"Qwen3.6-35B-A3B-oQ6 (local)","limit":{"context":262144,"output":98304}}}}}}'

changes: []
requires_claudine_update: true
reason: New local runner entry. Claudine's sniff detection surface should add probes for the omlx binary, /Applications/oMLX.app bundle (CFBundleIdentifier app.omlx), default port 8000 with /health identity marker, and ~/.omlx/settings.json; the model_catalog should be aware of oMLX's directory-name model ID grammar and aliases/profiles.
---

# oMLX

## Introduction to oMLX

[oMLX](https://omlx.ai) is an open-source, macOS-native local model server optimized for Apple Silicon. It loads and serves MLX-format text LLMs, vision-language models, embedding models, rerankers, audio TTS/STT models, and OCR models over HTTP, with continuous batching and a tiered RAM/SSD KV cache designed for agent-style workloads. The project is Apache 2.0 licensed.

| Resource | URL |
| --- | --- |
| Homepage | https://omlx.ai |
| Repository | https://github.com/jundot/omlx |
| Releases | https://github.com/jundot/omlx/releases |
| Issues | https://github.com/jundot/omlx/issues |
| API reference (when running) | http://localhost:8000/openapi.json |
| Swagger UI (when running) | http://localhost:8000/docs |

## Platforms and Installation

| OS | Support | Binary | Install methods | Process model | Service |
| --- | --- | --- | --- | --- | --- |
| macOS | native | `omlx` | DMG, `brew tap jundot/omlx https://github.com/jundot/omlx && brew install omlx`, source `pip install -e .` | both | brew services / launchd (Homebrew); menubar/login item (DMG); foreground `omlx serve` (CLI) |
| Linux | unsupported | — | — | — | none |
| Windows | unsupported | — | — | — | none |

Observed on this host:

- `/Applications/oMLX.app` (CFBundleIdentifier `app.omlx`, version 0.4.4)
- `/Applications/oMLX.app/Contents/MacOS/oMLX` (menubar app process)
- `omlx-server` background server process
- `/opt/homebrew/bin/omlx` → `~/.omlx/bin/omlx` shim → `/Applications/oMLX.app/Contents/MacOS/omlx-cli`

## API Surface

The server listens on **127.0.0.1:8000** by default.

| API family | Supported | Base URL | Auth |
| --- | --- | --- | --- |
| OpenAI-compatible | yes | `http://localhost:8000/v1` | optional API key |
| Anthropic Messages | yes | `http://localhost:8000` | optional API key |
| Native admin API | yes | `http://localhost:8000/admin/api` | admin session |

### OpenAI-compatible endpoints

| Method | Path | Notes |
| --- | --- | --- |
| GET | `/v1/models` | Lists discovered models |
| GET | `/v1/models/status` | Load state and memory estimates |
| POST | `/v1/models/{model_id}/load` | Load a model |
| POST | `/v1/models/{model_id}/unload` | Unload a model |
| POST | `/v1/chat/completions` | Chat completions (streaming) |
| POST | `/v1/completions` | Text completions (streaming) |
| POST | `/v1/embeddings` | Text/multimodal embeddings |
| POST | `/v1/rerank` | Document reranking |
| POST | `/v1/audio/speech` | Text-to-speech |
| POST | `/v1/audio/transcriptions` | Speech-to-text |
| POST | `/v1/audio/process` | Audio processing |
| POST | `/v1/responses` | Stateful OpenAI Responses API |
| GET | `/v1/mcp/servers` | MCP server list |
| GET | `/v1/mcp/tools` | MCP tool list |
| POST | `/v1/mcp/execute` | MCP tool execution |

### Anthropic-compatible endpoints

| Method | Path | Notes |
| --- | --- | --- |
| POST | `/v1/messages` | Anthropic Messages API with streaming and tool use |
| POST | `/v1/messages/count_tokens` | Token counting |

### Native admin endpoints

| Method | Path | Purpose |
| --- | --- | --- |
| GET | `/admin/api/models` | Model discovery and load state |
| POST | `/admin/api/models/{model_id}/load` | Load model |
| POST | `/admin/api/models/{model_id}/unload` | Unload model |
| POST | `/admin/api/models/{model_id}/settings` | Per-model settings |
| POST | `/admin/api/global-settings` | Update server settings |
| GET | `/admin/api/server-info` | Server runtime info |
| GET | `/admin/api/stats` | Runtime statistics |
| POST | `/admin/api/hf/download` | HuggingFace download |
| POST | `/admin/api/cache/probe` | Per-prompt cache hierarchy probe |

## Detection

1. **Binary on PATH** — `omlx` (observed at `/opt/homebrew/bin/omlx`).
2. **macOS app bundle** — `/Applications/oMLX.app` with bundle ID `app.omlx`.
3. **Process** — `oMLX` (menubar app) and `omlx-server` (background server).
4. **Port** — TCP 8000 (ambiguous; requires HTTP probe).
5. **HTTP probe** — `GET /health` returns `{"status":"healthy","default_model":"...","engine_pool":{...}}` and is ungated even when auth is enabled.
6. **HTTP probe** — `GET /openapi.json` returns FastAPI schema with `"title":"oMLX API"`.
7. **Config file** — `~/.omlx/settings.json`, or the path pointed to by `~/Library/Application Support/oMLX/base-path` via `OMLX_BASE_PATH`.

## Configuration

Settings are stored in `~/.omlx/settings.json` by default. The effective base path can be relocated by `OMLX_BASE_PATH` (set via `~/Library/Application Support/oMLX/base-path` when using the app shim). CLI flags take precedence over settings, and `OMLX_*` environment variables are read during startup.

Key settings observed in `~/.omlx/settings.json`:

- `server.host` / `server.port` — bind address and port.
- `model.model_dirs` — array of model directories (preferred over legacy `model_dir`).
- `auth.api_key` / `auth.skip_api_key_verification` — API-key auth toggle; the skip only applies to localhost requests.
- `cache.ssd_cache_dir` / `cache.ssd_cache_max_size` — persistent SSD KV cache.
- `scheduler.max_concurrent_requests` — continuous batching concurrency.
- `integrations.opencode_model` / `integrations.pi_model` — default models for `omlx launch`.

## Models

Models are discovered as subdirectories under the configured `model_dirs`. Each directory must contain a `config.json` and `*.safetensors` files. Two-level organization (`owner/model-name/`) is supported.

Examples observed on this host:

- `Qwen3.6-35B-A3B-oQ6`
- `mlx-community/MiniCPM-V-4.6-bf16`
- `Marvis-AI/marvis-tts-250m-v0.1-MLX-8bit`

A model alias can be configured per-model, and profiles can be exposed as `<model>:<profile>` without loading a second engine.

## Capabilities

| Capability | Support | Notes |
| --- | --- | --- |
| Hardware acceleration | Metal, CPU | Apple Silicon only |
| Multi-model serving | yes | LLM, VLM, embedding, reranker, audio TTS/STT, OCR, builtin MarkItDown |
| Parallel requests | yes | Continuous batching via mlx-lm BatchGenerator |
| SSE streaming | yes | OpenAI, Anthropic, and native endpoints |
| Tool/function calling | conditional | Chat-template dependent; many formats auto-detected |
| Embeddings | yes | `/v1/embeddings` |
| Reranking | yes | `/v1/rerank` |
| Web UI | yes | http://localhost:8000/admin |

## Agentic CLI Integration

### OpenCode

```json
{
  "provider": {
    "omlx": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "oMLX (local)",
      "options": {
        "baseURL": "http://localhost:8000/v1"
      },
      "models": {
        "Qwen3.6-35B-A3B-oQ6": {
          "name": "Qwen3.6-35B-A3B-oQ6 (local)",
          "limit": { "context": 262144, "output": 98304 }
        }
      }
    }
  }
}
```

### Claude Code via Anthropic-compatible endpoint

```bash
export ANTHROPIC_BASE_URL=http://localhost:8000
export ANTHROPIC_AUTH_TOKEN=<your-omlx-api-key>
claude --model Qwen3.6-35B-A3B-oQ6
```

### Runner-native integration

oMLX provides `omlx launch <tool>` commands that set the required environment variables and exec into the agent:

```bash
omlx launch claude    # ANTHROPIC_BASE_URL + ANTHROPIC_AUTH_TOKEN
omlx launch codex     # OPENAI_BASE_URL + OPENAI_API_KEY
omlx launch opencode  # provider config injection
omlx launch openclaw  # with optional --tools-profile
omlx launch pi
omlx launch hermes
omlx launch copilot
omlx launch codex_app
```

## Sources

- [oMLX homepage](https://omlx.ai)
- [oMLX GitHub repository](https://github.com/jundot/omlx)
- [oMLX releases](https://github.com/jundot/omlx/releases)
- [oMLX README / install and API compatibility](https://github.com/jundot/omlx/blob/main/README.md)
- [oMLX v0.1.0 release notes (OpenAI + Anthropic endpoints)](https://github.com/jundot/omlx/releases/tag/v0.1.0)
- [oMLX v0.3.9.dev1 release notes (`omlx launch claude`)](https://github.com/jundot/omlx/releases/tag/v0.3.9.dev1)
- [oMLX v0.3.9.dev2 release notes (`omlx launch copilot`, `{owner}/{model}` subfolders)](https://github.com/jundot/omlx/releases/tag/v0.3.9.dev2)
- [Homebrew tap](https://github.com/jundot/omlx)

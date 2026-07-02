---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7

summary: Ollama is an open-source local LLM runner that serves GGUF models over a native REST API and provides OpenAI- and Anthropic-compatible endpoints.
homepage: https://ollama.com
docs_url: https://docs.ollama.com
repo_url: https://github.com/ollama/ollama
api_reference_url: https://docs.ollama.com/api
open_source: full

has_official_schema: none

default_port: 11434
default_bind: 127.0.0.1
auth: none
auth_notes: >
  The local server has no required authentication. API keys sent to OpenAI- or
  Anthropic-compatible endpoints are accepted but ignored. Cloud features (model
  pull/push, private models, cloud inference) authenticate to ollama.com via an
  SSH key pair stored at ~/.ollama/id_ed25519.pub.

platforms:
  - os: macos
    support: native
    binary: ollama
    alt_binaries: ["Ollama.app/Contents/MacOS/Ollama", "Ollama.app/Contents/Resources/ollama"]
    install: ["DMG from ollama.com/download/mac", "curl -fsSL https://ollama.com/install.sh | sh", "brew install ollama"]
    process_model: both
    service: macOS app menubar/login item; foreground `ollama serve` from CLI
    notes: Requires macOS 14 Sonoma or later. Observed on this host as /Applications/Ollama.app (v0.30.11) with background `ollama serve` and per-model `llama-server` child processes.
  - os: linux
    support: native
    binary: ollama
    alt_binaries: ["ollama"]
    install: ["curl -fsSL https://ollama.com/install.sh | sh", "Docker ollama/ollama", "package managers (pacman, nix, etc.)"]
    process_model: both
    service: systemd service when installed via official script; foreground `ollama serve` from CLI
    notes: Official installer creates an `ollama` user and systemd service. Manual install instructions are at https://docs.ollama.com/linux.
  - os: windows
    support: native
    binary: ollama.exe
    alt_binaries: ["Ollama.exe"]
    install: ["OllamaSetup.exe from ollama.com/download/windows", "irm https://ollama.com/install.ps1 | iex", "Docker ollama/ollama"]
    process_model: both
    service: tray app / login item; foreground `ollama serve` from terminal
    notes: Requires Windows 10 or later. GPU acceleration available for NVIDIA and AMD.

api_standards:
  - standard: openai_compatible
    supported: yes
    base_url: http://localhost:11434/v1
    key_paths:
      - /v1/models
      - /v1/models/{model}
      - /v1/completions
      - /v1/chat/completions
      - /v1/embeddings
      - /v1/images/generations
      - /v1/responses
    auth: none
    since_version: "v0.1.24"
    deviations:
      - "tool_choice, logit_bias, user, n, echo, best_of, logprobs are not supported."
      - "Image URL content is not supported; base64 data URIs are."
      - "/v1/responses was added in v0.13.3 and is non-stateful (no previous_response_id)."
      - "/v1/images/generations is experimental and may change."
    docs_url: https://docs.ollama.com/api/openai-compatibility
  - standard: anthropic_compatible
    supported: yes
    base_url: http://localhost:11434
    key_paths:
      - /v1/messages
    auth: none
    since_version: "v0.14.0"
    deviations:
      - "Anthropic SDKs append /v1/messages themselves, so base_url omits /v1."
      - "API key and anthropic-version header are accepted but ignored."
      - "tool_choice, metadata, prompt caching, batches, citations, PDF content, and /v1/messages/count_tokens are not supported."
      - "Extended thinking budget_tokens is accepted but not enforced."
    docs_url: https://docs.ollama.com/api/anthropic-compatibility
  - standard: native
    supported: yes
    base_url: http://localhost:11434/api
    key_paths:
      - /api/generate
      - /api/chat
      - /api/create
      - /api/pull
      - /api/push
      - /api/delete
      - /api/copy
      - /api/show
      - /api/tags
      - /api/ps
      - /api/embed
      - /api/embeddings
      - /api/version
    auth: none
    since_version: "unknown"
    deviations:
      - "Streaming endpoints return newline-delimited JSON objects by default."
      - "Image generation via /api/generate is experimental."
    docs_url: https://docs.ollama.com/api

metadata_endpoints:
  - purpose: version
    method: get
    path: /api/version
    gated_by: ""
    auth_gated: false
    response_hint: '{"version":"0.30.11"}'
    notes: Observed on this host. Returns the running server version.
  - purpose: model_list
    method: get
    path: /api/tags
    gated_by: ""
    auth_gated: false
    response_hint: '{"models":[{"name":"...","model":"...","details":{"format":"gguf",...}}]}'
    notes: Ollama-specific model list. Observed on this host returning 18 models.
  - purpose: loaded_models
    method: get
    path: /api/ps
    gated_by: ""
    auth_gated: false
    response_hint: '{"models":[]}'
    notes: Lists models currently resident in memory. Empty when no models are loaded; observed with loaded models on this host.
  - purpose: model_info
    method: post
    path: /api/show
    gated_by: ""
    auth_gated: false
    response_hint: '{"license":"...","modelfile":"...","parameters":"...","template":"..."}'
    notes: Request body must include {"model":"<name>"}. Returns model metadata, license, and template.
  - purpose: load_model
    method: post
    path: /api/generate
    gated_by: ""
    auth_gated: false
    response_hint: '{"done":true,"done_reason":"load"}'
    notes: Send an empty prompt to load a model; use /api/chat with empty messages array for chat models. Native API only.
  - purpose: unload_model
    method: post
    path: /api/generate
    gated_by: ""
    auth_gated: false
    response_hint: '{"done":true,"done_reason":"unload"}'
    notes: Send empty prompt with keep_alive=0. Also works with /api/chat and empty messages array plus keep_alive=0.
  - purpose: health
    method: get
    path: /
    gated_by: ""
    auth_gated: false
    response_hint: "Ollama is running"
    notes: No dedicated /health endpoint exists (/health and /api/health return 404 on this host). The root GET is the recommended detector probe.
  - purpose: metrics
    method: get
    path: /metrics
    gated_by: ""
    auth_gated: false
    response_hint: "404 page not found"
    notes: No Prometheus-style /metrics endpoint on this host. Docs do not describe a metrics endpoint.

detection:
  - os: macos
    method: binary
    target: ollama
    expect: "ollama version is 0.30.11"
    confidence: observed
    notes: Observed on this host at /usr/local/bin/ollama.
  - os: macos
    method: app_bundle
    target: /Applications/Ollama.app
    expect: "CFBundleIdentifier: com.ollama.ollama"
    confidence: documented
    notes: macOS distribution installs the app bundle; the server binary is inside Contents/Resources.
  - os: macos
    method: process
    target: ollama serve
    expect: "/Applications/Ollama.app/Contents/Resources/ollama serve"
    confidence: observed
    notes: Observed on this host. Actual inference is handled by child `llama-server` processes spawned per loaded model.
  - os: linux
    method: binary
    target: ollama
    expect: "ollama version is ..."
    confidence: documented
    notes: Installed to /usr/local/bin/ollama by the official install script.
  - os: linux
    method: service
    target: ollama.service
    expect: "systemd unit running /usr/local/bin/ollama serve"
    confidence: documented
    notes: Official installer creates and starts a systemd service.
  - os: windows
    method: binary
    target: ollama.exe
    expect: "ollama version is ..."
    confidence: documented
    notes: Installed via OllamaSetup.exe or install.ps1; typically in %LOCALAPPDATA%\Programs\Ollama.
  - os: all
    method: port
    target: "11434"
    expect: ""
    confidence: observed
    notes: Default port 11434 is shared with other runners (e.g., some local OpenAI-compatible servers); an HTTP probe is required to disambiguate.
  - os: all
    method: http
    target: GET /
    expect: "Ollama is running"
    confidence: observed
    notes: Strong identity marker observed on this host. Ungated and unauthenticated.
  - os: all
    method: http
    target: GET /api/version
    expect: '{"version":"..."}'
    confidence: observed
    notes: Observed on this host. Version endpoint is unauthenticated.
  - os: all
    method: http
    target: GET /api/tags
    expect: '{"models":[{"name":"...","details":{"format":"gguf"}}]}'
    confidence: observed
    notes: "Observed on this host. The `format: gguf` field inside `details` helps confirm Ollama."
  - os: macos
    method: config_file
    target: ~/.ollama/server.json
    expect: '{"disable_ollama_cloud":true}'
    confidence: documented
    notes: Optional file; docs mention it for disabling cloud features. Not observed on this host.

config_mechanism: env_vars

config_files:
  - os: macos
    path: ~/.ollama/server.json
    format: json
    role: optional cloud toggle
    notes: If present, can set disable_ollama_cloud. Not observed on this host; Ollama is otherwise configured via environment variables.
  - os: linux
    path: /etc/systemd/system/ollama.service.d/environment.conf
    format: ini
    role: systemd override for server env vars
    notes: Standard way to persist OLLAMA_* variables when running under systemd.
  - os: windows
    path: '%USERPROFILE%\.ollama\server.json'
    format: json
    role: optional cloud toggle
    notes: Same optional cloud-disable file as macOS.

env_vars:
  - name: OLLAMA_HOST
    effect: "Bind address and port for the server (default 127.0.0.1:11434). Must include both host and port."
  - name: OLLAMA_MODELS
    effect: "Path to the model store directory (default ~/.ollama/models on macOS/Windows; /usr/share/ollama/.ollama/models on Linux)."
  - name: OLLAMA_CONTEXT_LENGTH
    effect: "Default context length. Default is VRAM-dependent: <24 GiB = 4k, 24-48 GiB = 32k, >=48 GiB = 256k."
  - name: OLLAMA_KEEP_ALIVE
    effect: "Duration a model stays loaded after last use (default 5m). Use -1 or a negative duration to keep loaded indefinitely; 0 unloads immediately."
  - name: OLLAMA_MAX_LOADED_MODELS
    effect: "Maximum number of models loaded concurrently (default 3 per GPU or 3 for CPU)."
  - name: OLLAMA_NUM_PARALLEL
    effect: "Maximum parallel requests per loaded model (default 1)."
  - name: OLLAMA_MAX_QUEUE
    effect: "Maximum queued requests before rejecting with 503 (default 512)."
  - name: OLLAMA_ORIGINS
    effect: "Comma-separated list of allowed CORS origins."
  - name: OLLAMA_FLASH_ATTENTION
    effect: "Enable flash attention when set to 1."
  - name: OLLAMA_KV_CACHE_TYPE
    effect: "K/V cache quantization type (default f16); supports q8_0 and q4_0."
  - name: OLLAMA_NO_CLOUD
    effect: "Disable Ollama cloud features (cloud models and web search)."
  - name: OLLAMA_NOPRUNE
    effect: "Do not prune unused model blobs on startup."
  - name: OLLAMA_DEBUG
    effect: "Show additional debug information."
  - name: OLLAMA_LLM_LIBRARY
    effect: "Override llama.cpp backend autodetection."
  - name: OLLAMA_GPU_OVERHEAD
    effect: "Reserve a portion of VRAM per GPU (bytes)."
  - name: OLLAMA_SCHED_SPREAD
    effect: "Always schedule a model across all available GPUs."

model_id_grammar: |
  Ollama model identifiers follow a `name[:tag]` pattern, where `name` can include a
  namespace (`namespace/model`) and `tag` defaults to `latest`. Examples:
  `llama3.2`, `llama3.2:70b`, `qwen3:1.7b`, `qwen3-coder:30b-a3b-q8_0`,
  `alibayram/medgemma:latest`. Ollama also accepts HuggingFace GGUF repositories
  as `hf.co/{user}/{repo}[:quant]` or `huggingface.co/{user}/{repo}[:quant]`, where
  `quant` can be a quantization string (case-insensitive) or a full GGUF filename.

model_formats:
  - gguf

model_acquisition:
  - method: registry
    example: "ollama pull llama3.2"
    notes: Pulls from the default ollama.com registry.
  - method: huggingface
    example: "ollama run hf.co/bartowski/Llama-3.2-1B-Instruct-GGUF:Q8_0"
    notes: Both `hf.co` and `huggingface.co` domains are accepted; quantization tag is optional.
  - method: manual
    example: "Create a Modelfile with `FROM ./model.gguf` or `FROM /path/to/safetensors`, then `ollama create my-model`."
    notes: Supports importing GGUF files and Safetensors weights for select architectures.
  - method: in_app
    example: "Use the Ollama macOS/Windows app model browser or `ollama pull` to download models."
    notes: Same registry-backed pull as the CLI.

model_store_paths:
  - os: macos
    path: ~/.ollama/models
    notes: Default on macOS. Observed on this host relocated via a symlink to /Volumes/Fast Bastard/models/ollama.
  - os: linux
    path: /usr/share/ollama/.ollama/models
    notes: Default when installed via the official Linux install script under the `ollama` user.
  - os: windows
    path: C:\Users\%username%\.ollama\models
    notes: Default on Windows.

hardware_acceleration:
  - metal
  - cuda
  - rocm
  - vulkan
  - cpu

concurrency:
  multi_model: true
  parallel_requests: true
  notes: Multiple models can be loaded simultaneously if memory permits. Parallel requests per model are controlled by OLLAMA_NUM_PARALLEL and scale context allocation accordingly.

streaming_sse: true
tool_calling: yes
tool_calling_notes: Supported natively via /api/chat tools and through OpenAI/Anthropic-compatible endpoints for models that advertise tool-calling capability.

embeddings: true
rerank: false

integration_hooks:
  - command: ollama launch claude
    effect: "Interactively select a model and launch Claude Code configured to use Ollama via ANTHROPIC_BASE_URL and ANTHROPIC_AUTH_TOKEN."
    notes: Added in v0.15+. Equivalent manual setup uses ANTHROPIC_BASE_URL=http://localhost:11434 and ANTHROPIC_AUTH_TOKEN=ollama.
  - command: ollama launch opencode
    effect: "Launch OpenCode configured to use Ollama as the model provider."
    notes: Added in v0.15+.
  - command: ollama launch codex
    effect: "Launch Codex CLI configured to use Ollama as the model provider."
    notes: Added in v0.15+.
  - command: ollama launch codex-app
    effect: "Launch the Codex desktop app configured to use Ollama."
    notes: Aliases codex-desktop and codex-gui.

opencode_example: '{"provider":{"ollama":{"npm":"@ai-sdk/openai-compatible","name":"Ollama (local)","options":{"baseURL":"http://localhost:11434/v1"},"models":{"qwen3:1.7b":{"name":"Qwen3 1.7B (local)"}}}}}'

changes: []
requires_claudine_update: true
reason: New local runner entry. Claudine's sniff detection surface should add probes for the ollama binary, /Applications/Ollama.app bundle, the default port 11434 with the `GET /` "Ollama is running" identity marker, /api/version, /api/tags, and the ~/.ollama model store; the model_catalog should be aware of Ollama's `name[:tag]` and `hf.co/{user}/{repo}[:quant]` model ID grammar.
---

# Ollama

## Introduction to Ollama

[Ollama](https://ollama.com) is an open-source local large language model runner.
It downloads, serves, and manages GGUF-format models over a native REST API and
also exposes OpenAI- and Anthropic-compatible endpoints so existing tools can use
local models without code changes. The project is licensed under the MIT license
and is developed at [ollama/ollama](https://github.com/ollama/ollama).

| Resource | URL |
| --- | --- |
| Homepage | https://ollama.com |
| Documentation | https://docs.ollama.com |
| Repository | https://github.com/ollama/ollama |
| Releases | https://github.com/ollama/ollama/releases |
| REST API reference | https://docs.ollama.com/api |
| OpenAI compatibility | https://docs.ollama.com/api/openai-compatibility |
| Anthropic compatibility | https://docs.ollama.com/api/anthropic-compatibility |

## Platforms and Installation

| OS | Support | Binary | Install methods | Process model | Service |
| --- | --- | --- | --- | --- | --- |
| macOS | native | `ollama` | Ollama.dmg, `curl -fsSL https://ollama.com/install.sh \| sh`, `brew install ollama` | both | macOS app menubar/login item; foreground `ollama serve` |
| Linux | native | `ollama` | `curl -fsSL https://ollama.com/install.sh \| sh`, Docker, package managers | both | systemd service (official installer); foreground `ollama serve` |
| Windows | native | `ollama.exe` | OllamaSetup.exe, `irm https://ollama.com/install.ps1 \| iex`, Docker | both | tray app / login item; foreground `ollama serve` |

Observed on this host:

- `/Applications/Ollama.app` (macOS app bundle)
- `/usr/local/bin/ollama` (CLI binary)
- `/Applications/Ollama.app/Contents/Resources/ollama serve` (background server process)
- Per-model `llama-server` child processes spawned for loaded models

## API Surface

Ollama listens on `127.0.0.1:11434` by default. The server exposes three API
families:

- **Native API** at `/api/*` for model management and inference.
- **OpenAI-compatible API** at `/v1/*` (base URL `http://localhost:11434/v1`).
- **Anthropic-compatible API** at `/v1/messages` (base URL
  `http://localhost:11434`, because Anthropic SDKs append `/v1/messages`
  themselves).

No API key is required for local inference; keys sent by OpenAI or Anthropic SDKs
are accepted but ignored.

### Key metadata and management endpoints

| Method | Path | Purpose | Notes |
| --- | --- | --- | --- |
| GET | `/` | Health / identity | Returns `Ollama is running`. No dedicated `/health` endpoint. |
| GET | `/api/version` | Version | Returns `{"version":"..."}`. |
| GET | `/api/tags` | List local models | Ollama-specific; includes `details.format: gguf`. |
| GET | `/api/ps` | List loaded models | Empty array when nothing is resident in memory. |
| POST | `/api/show` | Model info | Body `{"model":"..."}` returns license, modelfile, parameters, template. |
| POST | `/api/generate` | Text completion / load / unload | Empty prompt loads; `keep_alive: 0` unloads. |
| POST | `/api/chat` | Chat completion / load / unload | Empty messages load; `keep_alive: 0` unloads. |

### OpenAI-compatible endpoints

| Path | Status | Notes |
| --- | --- | --- |
| `/v1/models` | Supported | Lists local models with `owned_by: library` or namespace. |
| `/v1/completions` | Supported | `prompt` must be a string. |
| `/v1/chat/completions` | Supported | Streaming, tools, vision, JSON mode, reasoning effort. |
| `/v1/embeddings` | Supported | String or array of strings input. |
| `/v1/images/generations` | Experimental | Image-generation models only. |
| `/v1/responses` | Supported (v0.13.3+) | Non-stateful only. |

### Anthropic-compatible endpoints

| Path | Status | Notes |
| --- | --- | --- |
| `/v1/messages` | Supported (v0.14.0+) | Messages, streaming, system prompts, tools, vision, thinking. |

Unsupported Anthropic features include `tool_choice`, `metadata`, prompt caching,
batches, citations, PDF content, and `/v1/messages/count_tokens`.

## Detection

A detector should probe in this order:

1. **Binary on PATH**: `ollama` (macOS/Linux) or `ollama.exe` (Windows).
2. **App bundle / tray app**: `/Applications/Ollama.app` on macOS, tray executable
   on Windows.
3. **Process**: `ollama serve` (and child `llama-server` processes when models are
   loaded).
4. **Port**: TCP 11434. This port is shared with some other local runners, so the
   HTTP identity marker is required.
5. **HTTP identity**: `GET /` returning `Ollama is running` confirms Ollama.
   `GET /api/version` and `GET /api/tags` (with `details.format: gguf`) provide
   additional confirmation.
6. **Model store**: `~/.ollama/models` on macOS/Windows,
   `/usr/share/ollama/.ollama/models` on Linux.

Observed on this host: the binary, app bundle, server process, and port 11434 all
respond with the expected markers.

## Configuration

Ollama is configured almost entirely through environment variables. There is no
primary config file for the server, although an optional `~/.ollama/server.json`
can set `disable_ollama_cloud: true`.

Important environment variables:

| Variable | Effect |
| --- | --- |
| `OLLAMA_HOST` | Bind address and port (`127.0.0.1:11434` by default). |
| `OLLAMA_MODELS` | Model store path. |
| `OLLAMA_CONTEXT_LENGTH` | Default context window (VRAM-dependent default). |
| `OLLAMA_KEEP_ALIVE` | How long models stay loaded (`5m` default). |
| `OLLAMA_MAX_LOADED_MODELS` | Concurrent loaded models. |
| `OLLAMA_NUM_PARALLEL` | Parallel requests per model. |
| `OLLAMA_MAX_QUEUE` | Request queue size before 503. |
| `OLLAMA_ORIGINS` | Allowed CORS origins. |
| `OLLAMA_FLASH_ATTENTION` | Enable flash attention. |
| `OLLAMA_KV_CACHE_TYPE` | K/V cache quantization. |
| `OLLAMA_NO_CLOUD` | Disable cloud features. |

Per-platform persistence:

- **macOS app**: quit the app, run `launchctl setenv OLLAMA_HOST ...`, then
  restart.
- **Linux systemd**: `systemctl edit ollama.service` and add `Environment=` lines.
- **Windows**: set user environment variables and restart the tray app.

## Models

### Model ID grammar

Ollama uses `name[:tag]`, where `name` may include a namespace:

- `llama3.2`
- `llama3.2:70b`
- `qwen3:1.7b`
- `qwen3-coder:30b-a3b-q8_0`
- `alibayram/medgemma:latest`

HuggingFace GGUF repositories can be referenced directly:

- `hf.co/{user}/{repo}`
- `hf.co/{user}/{repo}:{quant}`
- `huggingface.co/{user}/{repo}:{quant}`

The quantization tag can be a short quant string (e.g., `Q8_0`) or a full GGUF
filename. Tags default to `latest` for registry models and to an available quant
for HuggingFace repos.

### Model formats

Runtime format is **GGUF**. Ollama can import models from:

- **Ollama registry**: `ollama pull <model>`
- **HuggingFace**: `ollama run hf.co/<user>/<repo>`
- **Manual import**: create a `Modelfile` pointing at a local GGUF file or
  Safetensors directory, then `ollama create <name>`.

### Model store paths

| OS | Default path |
| --- | --- |
| macOS | `~/.ollama/models` |
| Linux | `/usr/share/ollama/.ollama/models` |
| Windows | `C:\Users\%username%\.ollama\models` |

The store can be relocated with `OLLAMA_MODELS`. On this host, `~/.ollama` is a
symlink pointing to `/Volumes/Fast Bastard/models/ollama`.

## Capabilities

| Capability | Status | Notes |
| --- | --- | --- |
| Hardware backends | Metal, CUDA, ROCm, Vulkan, CPU | Metal on Apple Silicon; CUDA/ROCm/Vulkan on Linux and Windows. |
| Multi-model serving | Yes | Limited by available VRAM/system RAM. |
| Parallel requests | Yes | Controlled by `OLLAMA_NUM_PARALLEL`. |
| SSE streaming | Yes | Native API streams NDJSON; OpenAI/Anthropic endpoints stream SSE. |
| Tool calling | Yes | Model-dependent; supported in native, OpenAI, and Anthropic APIs. |
| Embeddings | Yes | `/api/embed`, `/api/embeddings`, and `/v1/embeddings`. |
| Reranking | No | Not supported natively. |
| Web UI | No | Ollama has no built-in web UI; community projects such as Open WebUI provide one. |
| Image generation | Experimental | Native `/api/generate` and OpenAI `/v1/images/generations`. |

## Agentic CLI Integration

### OpenCode provider block

```json
{
  "provider": {
    "ollama": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "Ollama (local)",
      "options": { "baseURL": "http://localhost:11434/v1" },
      "models": {
        "qwen3:1.7b": { "name": "Qwen3 1.7B (local)" }
      }
    }
  }
}
```

### Claude Code via Anthropic compatibility

```bash
export ANTHROPIC_AUTH_TOKEN=ollama
export ANTHROPIC_BASE_URL=http://localhost:11434
claude --model qwen3-coder
```

Ollama accepts either `Authorization: Bearer ollama` or `x-api-key: ollama` for
Anthropic-style requests.

### Runner-native integration hooks

Since v0.15+, Ollama provides one-command setup for several coding agents:

```bash
ollama launch claude
ollama launch opencode
ollama launch codex
ollama launch codex-app
```

These prompts for a model and configure the target agent automatically.

## Traps

- `OLLAMA_HOST` sets **both** host and port (`127.0.0.1:11434`), not just the host.
- The default context length is VRAM-dependent and may be much smaller than a
  model's maximum; agentic coding tools should set `OLLAMA_CONTEXT_LENGTH=64000`
  or higher.
- `keep_alive: 0` unloads a model immediately; `keep_alive: -1` keeps it loaded
  indefinitely.
- `OLLAMA_NUM_PARALLEL` scales context allocation, so memory use grows with the
  product of context length and parallelism.
- A port-only probe on 11434 is not enough to identify Ollama; always verify with
  the `GET /` identity marker or `/api/version`.
- There is no built-in web UI; references to "Ollama WebUI" are third-party
  projects.

## Sources

- [Ollama homepage](https://ollama.com)
- [Ollama documentation](https://docs.ollama.com)
- [Ollama GitHub repository](https://github.com/ollama/ollama)
- [Ollama REST API docs](https://docs.ollama.com/api)
- [Ollama OpenAI compatibility docs](https://docs.ollama.com/api/openai-compatibility)
- [Ollama Anthropic compatibility docs](https://docs.ollama.com/api/anthropic-compatibility)
- [Ollama FAQ / configuration](https://docs.ollama.com/faq)
- [Ollama GPU support](https://docs.ollama.com/gpu)
- [Ollama context length docs](https://docs.ollama.com/context-length)
- [OpenAI compatibility announcement (Feb 2024)](https://ollama.com/blog/openai-compatibility)
- [Claude Code / Anthropic compatibility announcement (Jan 2026)](https://ollama.com/blog/claude)
- [ollama launch announcement (Jan 2026)](https://ollama.com/blog/launch)
- [HuggingFace Ollama integration docs](https://huggingface.co/docs/hub/ollama)
- [Ollama v0.1.24 release notes (OpenAI compat)](https://github.com/ollama/ollama/releases/tag/v0.1.24)
- [Ollama v0.14.0 release notes (Anthropic compat)](https://github.com/ollama/ollama/releases/tag/v0.14.0)

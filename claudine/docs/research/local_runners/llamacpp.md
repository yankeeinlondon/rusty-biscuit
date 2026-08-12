---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: codex
model: default

summary: Llama.cpp is an MIT-licensed local inference engine whose `llama-server` binary serves local GGUF models through OpenAI-compatible, Anthropic-compatible, and native HTTP APIs.
homepage: https://llama.app
docs_url: https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md
repo_url: https://github.com/ggml-org/llama.cpp
api_reference_url: https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md
open_source: full

has_official_schema: none

default_port: 8080
default_bind: 127.0.0.1
auth: optional_api_key
auth_notes: >
  Authentication is disabled by default. When `--api-key`, `--api-key-file`, or
  `LLAMA_API_KEY` is set, protected endpoints accept `Authorization: Bearer KEY`
  or `X-Api-Key: KEY`. Source inspection shows `/health`, `/v1/health`,
  `/models`, `/v1/models`, `/api/tags`, and `/` remain public; inference
  endpoints, `/props`, `/metrics`, `/slots`, `/models/load`, and
  `/models/unload` require the key.

platforms:
  - os: macos
    support: native
    binary: llama-server
    alt_binaries: ["server"]
    install: ["brew install llama.cpp", "pre-built tar.gz from GitHub releases", "build from source with CMake", "conda-forge: conda install -c conda-forge llama-cpp", "MacPorts: sudo port install llama.cpp", "nix profile install nixpkgs#llama-cpp"]
    process_model: foreground
    service: none
    notes: First-class Apple Silicon support through Metal. Observed on this host at /Users/ken/coding/ai/llama.cpp/build/bin/llama-server, version 8168 (723c71064), built for Darwin arm64.
  - os: linux
    support: native
    binary: llama-server
    alt_binaries: ["server"]
    install: ["pre-built tar.gz from GitHub releases", "build from source with CMake", "conda-forge: conda install -c conda-forge llama-cpp", "nix profile install nixpkgs#llama-cpp", "Docker ghcr.io/ggml-org/llama.cpp:server"]
    process_model: foreground
    service: none
    notes: No official systemd unit; users usually run it directly, under a process manager, or in Docker. CUDA, ROCm, Vulkan, SYCL, and CPU builds are common.
  - os: windows
    support: native
    binary: llama-server.exe
    alt_binaries: ["server.exe"]
    install: ["winget install llama.cpp", "pre-built zip from GitHub releases", "build from source with CMake", "conda-forge: conda install -c conda-forge llama-cpp"]
    process_model: foreground
    service: none
    notes: No official Windows service or tray app; run from a terminal or wrap with a user-managed service.

api_standards:
  - standard: openai_compatible
    supported: yes
    base_url: http://localhost:8080/v1
    key_paths: ["/v1/models", "/v1/completions", "/v1/chat/completions", "/v1/responses", "/v1/embeddings", "/v1/rerank", "/v1/reranking"]
    auth: optional_api_key
    since_version: unknown
    deviations:
      - "OpenAI-compatible endpoints coexist with non-OpenAI native endpoints."
      - "`/v1/responses` exists in current source, but old builds returned 404; no exact release tag was verified from release notes."
      - "The `/v1/chat/completions/input_tokens` helper is intentionally not an official OpenAI endpoint."
      - "Tool calls require the Jinja chat-template path; `--jinja` is enabled by default in current builds."
      - "In single-model mode the request `model` is not a router selector; `/v1/models` reports the model path or `--alias`."
    docs_url: https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md
  - standard: anthropic_compatible
    supported: yes
    base_url: http://localhost:8080
    key_paths: ["/v1/messages", "/v1/messages/count_tokens"]
    auth: optional_api_key
    since_version: b7187
    deviations:
      - "Anthropic SDK base URL should omit `/v1` because SDKs append `/v1/messages`."
      - "The implementation converts Anthropic requests to the existing OpenAI-compatible internal request path."
      - "Current docs say it is compatible enough for many apps but do not claim complete Anthropic API coverage."
      - "Tool use requires Jinja; vision requires a multimodal model; extended-thinking behavior has had active bug reports."
    docs_url: https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md
  - standard: native
    supported: yes
    base_url: http://localhost:8080
    key_paths: ["/health", "/models", "/api/tags", "/completion", "/completions", "/chat/completions", "/api/chat", "/tokenize", "/detokenize", "/embedding", "/embeddings", "/rerank", "/reranking", "/infill", "/apply-template", "/lora-adapters", "/slots", "/props", "/models/load", "/models/unload"]
    auth: optional_api_key
    since_version: unknown
    deviations:
      - "`/api/tags`, `/api/show`, and `/api/chat` are Ollama-style compatibility aliases registered by llama-server source."
      - "Embeddings require embedding mode or an embedding-capable model; reranking requires `--rerank` / `--reranking`."
      - "Router-only endpoints require `--models-dir` or `--models-preset`."
    docs_url: https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md

metadata_endpoints:
  - purpose: health
    method: get
    path: /health
    gated_by: ""
    auth_gated: false
    response_hint: '{"status":"ok"}'
    notes: Public health endpoint. Returns 503 with `Loading model` while the model is still loading.
  - purpose: health
    method: get
    path: /v1/health
    gated_by: ""
    auth_gated: false
    response_hint: '{"status":"ok"}'
    notes: Public alias for `/health`.
  - purpose: model_list
    method: get
    path: /v1/models
    gated_by: ""
    auth_gated: false
    response_hint: '{"object":"list","data":[{"owned_by":"llamacpp"}]}'
    notes: Public OpenAI-compatible model list. Single-model mode returns one loaded model; router mode returns router-visible model entries.
  - purpose: model_list
    method: get
    path: /models
    gated_by: ""
    auth_gated: false
    response_hint: '{"data":[{"id":"...","status":{"value":"loaded"}}]}'
    notes: Public model list. In router mode it lists cached and directory models with status, path, and `in_cache`; in single-model mode it aliases the loaded model list.
  - purpose: model_list
    method: get
    path: /api/tags
    gated_by: ""
    auth_gated: false
    response_hint: '{"object":"list","data":[...]}'
    notes: Ollama-style compatibility alias registered by llama-server source; this is not Ollama-only in current llama.cpp.
  - purpose: model_info
    method: get
    path: /props
    gated_by: ""
    auth_gated: true
    response_hint: '{"model_path":"...","build_info":"b...","endpoint_metrics":false}'
    notes: Returns model path, alias, modalities, chat template, generation defaults, endpoint flags, Web UI settings, build info, and sleep state. Router mode accepts `?model=<id>`.
  - purpose: other
    method: post
    path: /props
    gated_by: --props
    auth_gated: true
    response_hint: '{"success":true}'
    notes: Changing global properties is currently gated by `--props`; GET `/props` is always registered.
  - purpose: loaded_models
    method: get
    path: /slots
    gated_by: "--slots; disabled by --no-slots"
    auth_gated: true
    response_hint: '[{"id":0,"is_processing":false}]'
    notes: Exposes slot and processing state rather than a dedicated loaded-model inventory.
  - purpose: metrics
    method: get
    path: /metrics
    gated_by: --metrics
    auth_gated: true
    response_hint: "# HELP llamacpp:prompt_tokens_total"
    notes: Prometheus exporter. Without `--metrics`, the endpoint is registered but returns a not-supported error.
  - purpose: load_model
    method: post
    path: /models/load
    gated_by: "--models-dir or --models-preset"
    auth_gated: true
    response_hint: '{"success":true}'
    notes: Router-mode endpoint; payload is `{\"model\":\"<id>\"}`.
  - purpose: unload_model
    method: post
    path: /models/unload
    gated_by: "--models-dir or --models-preset"
    auth_gated: true
    response_hint: '{"success":true}'
    notes: Router-mode endpoint; payload is `{\"model\":\"<id>\"}`.
  - purpose: admin_ui
    method: get
    path: /
    gated_by: "--webui; disabled by --no-webui"
    auth_gated: false
    response_hint: "Server: llama.cpp"
    notes: Serves the built-in Web UI by default. Static assets under the UI path are also public when auth is enabled.

detection:
  - os: macos
    method: binary
    target: llama-server
    expect: "version: 8168 (723c71064)"
    confidence: observed
    notes: Observed on this host at /Users/ken/coding/ai/llama.cpp/build/bin/llama-server.
  - os: linux
    method: binary
    target: llama-server
    expect: "version:"
    confidence: documented
    notes: Pre-built archives and package-manager installs use the same binary name.
  - os: windows
    method: binary
    target: llama-server.exe
    expect: "version:"
    confidence: documented
    notes: Pre-built Windows archives and winget installs use the `.exe` suffix.
  - os: all
    method: process
    target: llama-server
    expect: "argv contains -m, --model, -hf, --hf-repo, --models-dir, --host, or --port"
    confidence: observed
    notes: Process probe found no running llama-server on this host on 2026-07-03; process name and argv markers are source/documentation-derived.
  - os: all
    method: port
    target: "8080"
    expect: ""
    confidence: observed
    notes: Default port 8080 was not open on this host on 2026-07-03. Port alone is ambiguous and must be followed by HTTP identity probes.
  - os: all
    method: http
    target: GET /health
    expect: '{"status":"ok"} or 503 Loading model from a Server: llama.cpp response'
    confidence: documented
    notes: Strong running-server probe; unauthenticated even when API-key auth is enabled. Local negative probe showed no server listening on 8080.
  - os: all
    method: http
    target: GET /v1/models
    expect: '"owned_by":"llamacpp"'
    confidence: source_code
    notes: Public endpoint. Single-model mode is the clearest identity marker; router mode may return richer router metadata.
  - os: all
    method: http
    target: GET /props
    expect: '"build_info" and "model_path"'
    confidence: source_code
    notes: Good identity marker when auth is disabled or a key is available; auth-gated when `--api-key` is set.
  - os: all
    method: http
    target: GET /api/tags
    expect: "llama-server model list response"
    confidence: source_code
    notes: Source-verified Ollama-style alias. Use only as secondary evidence because other runners also implement this path.
  - os: all
    method: config_file
    target: none
    expect: ""
    confidence: source_code
    notes: No primary server config file. Configuration is by CLI flags, `LLAMA_ARG_*` environment variables, and optional Web UI JSON or router preset files.

identity_probes:
  - rank: 1
    request: ANY /health
    match_in: header
    field: Server
    marker: llama.cpp
    uniqueness: unique
    zero_model_ok: true
    auth_gated: false
    confidence: source_code
    notes: "server-http.cpp sets `Server: llama.cpp` on every response with no flag to disable — the cheapest possible probe (any request, even /health while the model is still loading and returning 503). No other fleet runner sends it."
  - rank: 2
    request: GET /props
    match_in: json_field
    field: build_info
    marker: '"build_info":"bNNNN-<commit>"'
    uniqueness: unique
    zero_model_ok: true
    auth_gated: true
    confidence: source_code
    notes: /props is llama-only and also carries total_slots, chat_template, and default_generation_settings; in router mode (/models-dir, no -m) it returns "role":"router". Auth-gated when --api-key is set.
  - rank: 3
    request: GET /v1/models
    match_in: json_field
    field: data[].owned_by
    marker: '"owned_by":"llamacpp"'
    uniqueness: strong
    zero_model_ok: true
    auth_gated: false
    confidence: source_code
    notes: Public endpoint; single-model mode is the clearest case. Generic path, llama-specific value.
  - rank: 4
    request: GET /api/tags
    match_in: json_field
    field: models[].digest
    marker: empty digest/modified_at/size fields
    uniqueness: weak
    zero_model_ok: true
    auth_gated: false
    confidence: source_code
    notes: Deliberate Ollama mimicry — the path cannot distinguish llama-server from Ollama, but llama.cpp's dummy empty digest/modified_at fields are a reverse-tell when Ollama's banner probe (rank 1 in ollama.md) fails on port 11434.

version_probe:
  - os: all
    method: cli
    command: llama-server --version
    pattern: "version: (\\d+) \\(([0-9a-f]+)\\)"
    confidence: observed
    notes: "Observed `version: 8168 (723c71064)` — the version is a BUILD NUMBER plus commit sha, not a semver. On this host the binary prints Metal/backend init lines BEFORE the version line, so match the `version:` line anywhere in the output, never the first line. For a running server, `GET /props` → build_info carries the same `bNNNN-<sha>` identity (identity_probes rank 2)."
  - os: windows
    method: cli
    command: llama-server.exe --version
    pattern: "version: (\\d+) \\(([0-9a-f]+)\\)"
    confidence: documented
    notes: Pre-built Windows archives and winget installs use the .exe suffix; same output shape.

config_mechanism: mixed

config_files:
  - os: macos
    path: "none by default; path supplied to --webui-config-file or --models-preset"
    format: other
    role: optional_webui_or_router_config
    notes: "`--webui-config-file` is JSON for Web UI defaults; `--models-preset` is an INI-like preset file for router mode. No default path is created."
  - os: linux
    path: "none by default; path supplied to --webui-config-file or --models-preset"
    format: other
    role: optional_webui_or_router_config
    notes: Same mechanism as macOS; no system-wide server config file or unit is shipped.
  - os: windows
    path: "none by default; path supplied to --webui-config-file or --models-preset"
    format: other
    role: optional_webui_or_router_config
    notes: Same mechanism as macOS/Linux; no registry-backed or app-bundle config was found.

env_vars:
  - name: LLAMA_ARG_HOST
    effect: "HTTP bind address or Unix socket path; default is 127.0.0.1."
  - name: LLAMA_ARG_PORT
    effect: "HTTP listen port; default is 8080."
  - name: LLAMA_ARG_API_PREFIX
    effect: "Prefix path the server serves from, without a trailing slash."
  - name: LLAMA_API_KEY
    effect: "Comma-separated optional API keys; equivalent to repeated `--api-key` values."
  - name: LLAMA_ARG_MODEL
    effect: "GGUF model path to load; equivalent to `-m` / `--model`."
  - name: LLAMA_ARG_MODEL_URL
    effect: "Direct model download URL; equivalent to `--model-url`."
  - name: LLAMA_ARG_HF_REPO
    effect: "Hugging Face repo in `<user>/<model>[:quant]` form; equivalent to `-hf` / `--hf-repo`."
  - name: LLAMA_ARG_HF_FILE
    effect: "Specific GGUF file inside the Hugging Face repo; overrides the quant selector."
  - name: LLAMA_ARG_DOCKER_REPO
    effect: "Docker Hub model repository selector in `[repo/]model[:quant]` form."
  - name: LLAMA_CACHE
    effect: "Overrides the llama.cpp model cache directory used by `-hf`, `--model-url`, cache listing, and router cached-model discovery."
  - name: LLAMA_OFFLINE
    effect: "Forces use of cached model files and prevents network access."
  - name: HF_TOKEN
    effect: "Hugging Face access token for private or gated downloads."
  - name: LLAMA_ARG_MODELS_DIR
    effect: "Directory containing GGUF files for router mode."
  - name: LLAMA_ARG_MODELS_PRESET
    effect: "Path to router model preset file."
  - name: LLAMA_ARG_MODELS_MAX
    effect: "Maximum number of simultaneously loaded router models; 0 means unlimited."
  - name: LLAMA_ARG_MODELS_AUTOLOAD
    effect: "Controls whether router mode auto-loads a model on request."
  - name: LLAMA_ARG_EMBEDDINGS
    effect: "Restricts the server to embedding use cases; required for dedicated embedding models."
  - name: LLAMA_ARG_RERANKING
    effect: "Enables reranking and rank pooling behavior."
  - name: LLAMA_ARG_ENDPOINT_METRICS
    effect: "Enables Prometheus metrics at GET /metrics."
  - name: LLAMA_ARG_ENDPOINT_PROPS
    effect: "Enables POST /props for changing global properties."
  - name: LLAMA_ARG_ENDPOINT_SLOTS
    effect: "Controls exposure of GET /slots; default is enabled."
  - name: LLAMA_ARG_N_PARALLEL
    effect: "Number of server slots / parallel requests."
  - name: LLAMA_ARG_N_GPU_LAYERS
    effect: "Number of layers to offload to GPU; current help default is auto."
  - name: LLAMA_ARG_CTX_SIZE
    effect: "Prompt context size; default 0 means loaded from model metadata."
  - name: LLAMA_ARG_JINJA
    effect: "Controls Jinja chat-template engine; default is enabled and tool use depends on it."

model_id_grammar: |
  `llama-server` model IDs are the identifiers the server was configured to expose:

  1. `--alias <id>` / `LLAMA_ARG_ALIAS` sets the model id reported by `/v1/models`.
  2. Without an alias, single-model mode reports the loaded GGUF path or filename.
  3. `-hf` / `--hf-repo` accepts `<user>/<model>[:quant]`, for example `unsloth/phi-4-GGUF:q4_k_m`; `-hff` can select a specific GGUF file inside the repo.
  4. `--model-url <url>` accepts a direct URL to a model file.
  5. `--docker-repo [repo/]model[:quant]` accepts Docker Hub model references, defaulting the repo to `ai/` and quant to `latest`.
  6. Router mode model IDs are the cached Hugging Face/Docker selectors or local GGUF filenames discovered through `LLAMA_CACHE` and `--models-dir`.

  There is no Ollama-style first-party `name:tag` registry for llama.cpp itself.

model_formats:
  - gguf

model_acquisition:
  - method: huggingface
    example: "llama-server -hf unsloth/phi-4-GGUF:q4_k_m"
    notes: Downloads a GGUF and optional multimodal projector into the llama.cpp cache directory. `HF_TOKEN` is used for gated repos.
  - method: manual
    example: "llama-server -m /path/to/model-Q4_K_M.gguf --alias local-coder"
    notes: Loads any local GGUF path directly.
  - method: registry
    example: "llama-server --docker-repo ai/gemma3:Q4_K_M"
    notes: Current help exposes Docker Hub model repository references; this is separate from the runner container image `ghcr.io/ggml-org/llama.cpp:server`.
  - method: in_app
    example: "Use the Web UI at http://localhost:8080 after starting llama-server."
    notes: The built-in Web UI chats with already available or router-visible models; it is not the primary model acquisition mechanism.

model_store_paths:
  - os: macos
    path: ~/Library/Caches/llama.cpp
    notes: Fresh default from source `fs_get_cache_directory()`. `LLAMA_CACHE` overrides it. Observed directory exists on this host but contains no files.
  - os: linux
    path: "${XDG_CACHE_HOME:-~/.cache}/llama.cpp"
    notes: Fresh default from source. `LLAMA_CACHE` overrides it.
  - os: windows
    path: "%LOCALAPPDATA%\\llama.cpp"
    notes: Fresh default from source. `LLAMA_CACHE` overrides it.

hardware_acceleration:
  - metal
  - cuda
  - rocm
  - vulkan
  - sycl
  - openvino
  - musa
  - cann
  - opencl
  - rpc
  - cpu

concurrency:
  multi_model: true
  parallel_requests: true
  notes: Single-model mode serves one model per process. Router mode can discover, load, and unload multiple models with `--models-dir` or `--models-preset`; `--models-max` caps simultaneously loaded models. Parallel request handling uses server slots through `-np` / `--parallel` and continuous batching.

streaming_sse: true
tool_calling: conditional
tool_calling_notes: >
  OpenAI function calling and Anthropic tool_use are available through Jinja chat
  templates, which are enabled by default in current builds. Disabling Jinja with
  `--no-jinja` removes tool support. Docs and recent community notes call out
  tool-use quirks for some fine-tunes because llama-server may add tool-oriented
  prompting when tools are present.
embeddings: true
rerank: true
web_ui_url: http://localhost:8080

integration_hooks: []

traps:
  - "`/api/tags` is registered by llama-server as an Ollama-style compatibility alias, so it is not an Ollama-only path; use response shape plus `Server: llama.cpp` or `/props` for stronger identification."
  - "`GET /props` is not gated by `--props`; only `POST /props` is. However, `/props` is auth-gated when `--api-key` is configured."
  - "`--embedding` / `--embeddings` restricts the server to embedding use cases and is not a normal chat-serving mode."
  - "`--rerank` / `--reranking` enables reranking behavior and rank pooling; treat it as a specialized serving mode."
  - "`LLAMA_CACHE` changes the llama.cpp cache directory. It is not the same as Hugging Face's standard cache path, and current source defaults to `~/Library/Caches/llama.cpp` on macOS, not `~/.cache/huggingface/hub`."
  - "`LLAMA_ARG_PORT` is the API port. There is no `LLAMA_PORT`; environment variables generally use the generated `LLAMA_ARG_*` names."
  - "The built-in Web UI is public when API-key auth is enabled; API calls it makes still need the configured key for protected endpoints."
  - "In single-model mode, the request `model` field is mostly an identifier for clients; router mode is where model IDs select which backend model to load or route to."
  - "`--api-prefix` can move every API path under a prefix, so detectors should try the unprefixed default first but allow configured deployments to differ."

opencode_example: '{"provider":{"llamacpp":{"npm":"@ai-sdk/openai-compatible","name":"Llama.cpp (local)","options":{"baseURL":"http://localhost:8080/v1"},"models":{"local-coder":{"name":"Local coder via llama-server"}}}}}'

changes:
  - "Refreshed host evidence: `llama-server` is installed at /Users/ken/coding/ai/llama.cpp/build/bin/llama-server, version 8168 (723c71064), but no server was listening on port 8080 during the 2026-07-03 probe."
  - "Corrected model cache paths from Hugging Face cache defaults to llama.cpp's own `LLAMA_CACHE` / platform cache directory logic."
  - "Added source-verified Ollama-style compatibility aliases, including `/api/tags`, `/api/show`, and `/api/chat`, while marking `/api/tags` as ambiguous for detection."
  - "Expanded router-mode metadata with `/models/load`, `/models/unload`, `--models-dir`, `--models-preset`, `--models-max`, and `LLAMA_ARG_MODELS_*` details."
  - "Verified Anthropic Messages API release tag `b7187` from the GitHub release notes and PR #17570."
  - "Clarified API-key public allowlist and auth-gated metadata endpoints from current source."
requires_claudine_update: true
reason: >
  Claudine and sniff should include llama-server detection, but the detector
  should account for the current source surface: the llama.cpp cache directory
  is platform-specific and overrideable by `LLAMA_CACHE`, `/api/tags` is a
  llama-server alias as well as an Ollama marker, router-mode model management
  adds `/models/load` and `/models/unload`, and HTTP identity should prefer
  `/health`, `/v1/models` with `owned_by: llamacpp`, `/props` with `build_info`,
  or the `Server: llama.cpp` header over port 8080 alone.
---

# Llama.cpp Local Runner

## Introduction to Llama.cpp

[Llama.cpp](https://llama.app) is an open-source C/C++ inference engine for local LLMs, distributed under the MIT license. Its `llama-server` binary is a foreground HTTP server for local GGUF models, with a built-in Web UI plus OpenAI-compatible, Anthropic-compatible, and native REST endpoints.

| Resource | URL |
| --- | --- |
| Homepage | https://llama.app |
| Repository | https://github.com/ggml-org/llama.cpp |
| Releases | https://github.com/ggml-org/llama.cpp/releases |
| Server documentation and API reference | https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md |
| Function calling documentation | https://github.com/ggml-org/llama.cpp/blob/master/docs/function-calling.md |
| REST API changelog | https://github.com/ggml-org/llama.cpp/issues/9291 |

The project does not publish a machine-readable OpenAPI or configuration schema for `llama-server`; the README and source are the authoritative API references.

## Platforms and Installation

| OS | Support | Binary | Install methods | Process model | Service |
| --- | --- | --- | --- | --- | --- |
| macOS | native | `llama-server` | Homebrew, release tarballs, CMake source build, conda-forge, MacPorts, Nix | foreground | none |
| Linux | native | `llama-server` | release tarballs, CMake source build, conda-forge, Nix, Docker image | foreground | none |
| Windows | native | `llama-server.exe` | winget, release zip, CMake source build, conda-forge | foreground | none |

There is no first-party daemon, tray app, launchd plist, systemd unit, or Windows service. Users run `llama-server` directly, under their own service manager, or in a container. The historical server binary name was `server` / `server.exe`; modern releases use `llama-server`.

Observed on this host on 2026-07-03:

- Binary: `/Users/ken/coding/ai/llama.cpp/build/bin/llama-server`
- Version: `8168 (723c71064)`
- Build: AppleClang for Darwin arm64 with Metal initialization
- Running state: no `llama-server` process found and no listener on `localhost:8080`

## API Surface

Default listen address: `127.0.0.1:8080`. `--host`, `--port`, and `--api-prefix` can change the client-visible base URL.

### OpenAI-Compatible API

Client base URL: `http://localhost:8080/v1`

Key paths:

- `GET /v1/models`
- `POST /v1/completions`
- `POST /v1/chat/completions`
- `POST /v1/responses`
- `POST /v1/embeddings`
- `POST /v1/rerank`
- `POST /v1/reranking`

Authentication is optional. If configured, use `Authorization: Bearer <key>`.

Known deviations:

- `/v1/responses` is present in current source, but older builds returned 404; no exact release tag was verified for its introduction.
- `/v1/chat/completions/input_tokens` is a convenience token-counting helper, not an official OpenAI endpoint.
- Tool calls require the Jinja chat-template path. Current builds enable `--jinja` by default.
- In single-model mode, the `model` request field is not a backend selector. Router mode uses model IDs for routing and loading.

### Anthropic Messages API

Client base URL: `http://localhost:8080`

Anthropic SDKs append `/v1/messages`, so do not include `/v1` in the base URL. Supported paths are:

- `POST /v1/messages`
- `POST /v1/messages/count_tokens`

This support was verified in release tag [`b7187`](https://github.com/ggml-org/llama.cpp/releases/tag/b7187), which names PR #17570, "server : add Anthropic Messages API support." The implementation converts Anthropic requests to the OpenAI-compatible internal path. `x-api-key` and `Authorization: Bearer` both work when server auth is enabled.

Unsupported or partial areas are the parts of Anthropic's API not represented by this conversion layer. Current docs and issues show active work around thinking/reasoning handling and feature parity. Tool use depends on Jinja; vision depends on a multimodal model.

### Native API Family

Native paths include:

- Health and metadata: `GET /health`, `GET /v1/health`, `GET /models`, `GET /props`, `GET /slots`, `GET /metrics`
- Generation: `POST /completion`, `POST /completions`, `POST /chat/completions`, `POST /infill`, `POST /apply-template`
- Embeddings and ranking: `POST /embedding`, `POST /embeddings`, `POST /rerank`, `POST /reranking`
- Token utilities: `POST /tokenize`, `POST /detokenize`
- LoRA and slots: `GET /lora-adapters`, `POST /lora-adapters`, `POST /slots/{id_slot}?action=save|restore|erase`
- Router mode: `POST /models/load`, `POST /models/unload`
- Ollama-style aliases registered by llama-server: `GET /api/tags`, `POST /api/show`, `POST /api/chat`

### Metadata Endpoints

| Method | Path | Purpose | Gated by | Auth-gated when API key is set | Identity marker |
| --- | --- | --- | --- | --- | --- |
| GET | `/health`, `/v1/health` | health | none | no | `{"status":"ok"}` |
| GET | `/v1/models` | OpenAI model list | none | no | `owned_by: llamacpp` in single-model mode |
| GET | `/models` | model list / router status | none | no | router `status.value`, `path`, `in_cache` fields |
| GET | `/api/tags` | Ollama-style model-list alias | none | no | llama-server model-list shape |
| GET | `/props` | model/server info | none | yes | `build_info`, `model_path`, endpoint flags |
| POST | `/props` | mutable global properties | `--props` | yes | `success: true` |
| GET | `/slots` | slot processing state | `--slots`; default enabled | yes | slot array with `id` and processing state |
| GET | `/metrics` | Prometheus metrics | `--metrics` | yes | `llamacpp:` metric names |
| POST | `/models/load` | router load model | `--models-dir` or `--models-preset` | yes | `success: true` |
| POST | `/models/unload` | router unload model | `--models-dir` or `--models-preset` | yes | `success: true` |
| GET | `/` | Web UI | `--webui`; default enabled | no | `Server: llama.cpp` header / Web UI HTML |

## Detection

Recommended ordered probes:

1. Check for `llama-server` on macOS/Linux or `llama-server.exe` on Windows. Treat historical `server` / `server.exe` as a weak legacy signal.
2. Check for a `llama-server` process with model or server arguments such as `-m`, `--model`, `-hf`, `--hf-repo`, `--models-dir`, `--host`, or `--port`.
3. Probe TCP port 8080. This is only a weak signal because many tools and development servers use 8080.
4. Probe `GET /health`. A ready server returns `{"status":"ok"}`; a loading server returns a llama-server-shaped 503.
5. Probe `GET /v1/models`. In single-model mode, `owned_by: llamacpp` is a strong marker.
6. If auth is disabled or a key is known, probe `GET /props` for `build_info`, `model_path`, and endpoint flags.
7. Use `GET /api/tags` only as secondary evidence. Current llama-server implements it, but Ollama and compatible servers also use that path.
8. Do not rely on a config file. There is no default server config file.

Observed negative evidence on this host: `curl` to `/`, `/health`, `/v1/health`, `/models`, `/v1/models`, `/props`, and `/metrics` on `127.0.0.1:8080` failed to connect because no server was running.

### Port identity

Port 8080 collides with LocalAI and countless development servers, so the
ranked `identity_probes` frontmatter block is the canonical strategy for
answering "which runner is listening on this port?":

1. Header check — `server-http.cpp` sets `Server: llama.cpp` on **every**
   response (even the 503 "Loading model" during startup), with no flag to
   disable it. This is the cheapest identity probe in the fleet: one request
   to any path, and no other runner sends the header.
2. `GET /props` — llama-only path carrying `build_info` (`bNNNN-<commit>`),
   `total_slots`, and `default_generation_settings`; router mode returns
   `"role":"router"`. Auth-gated when `--api-key` is set.
3. `GET /v1/models` — `"owned_by":"llamacpp"` on a generic path.
4. `GET /api/tags` — deliberate Ollama mimicry; the empty dummy
   `digest`/`modified_at` fields are a reverse-tell distinguishing it from
   real Ollama.

## Configuration

`llama-server` is configured primarily by command-line flags. Many flags have generated `LLAMA_ARG_*` environment variables. There is no default server config file.

Optional file-based inputs:

- `--webui-config-file <path>`: JSON defaults for the Web UI.
- `--models-preset <path>`: router model presets.
- `--api-key-file <path>`: one or more API keys.
- `--slot-save-path <path>`: enables slot KV-cache save/restore files.

Important environment variables:

| Variable | Effect |
| --- | --- |
| `LLAMA_ARG_HOST` | Bind address or Unix socket path; default `127.0.0.1`. |
| `LLAMA_ARG_PORT` | Listen port; default `8080`. |
| `LLAMA_ARG_API_PREFIX` | Prefix for served API paths. |
| `LLAMA_API_KEY` | Optional comma-separated API keys. |
| `LLAMA_ARG_MODEL` | GGUF model path. |
| `LLAMA_ARG_MODEL_URL` | Direct model download URL. |
| `LLAMA_ARG_HF_REPO` | Hugging Face repo in `<user>/<model>[:quant]` form. |
| `LLAMA_ARG_HF_FILE` | Specific GGUF file inside a Hugging Face repo. |
| `LLAMA_ARG_DOCKER_REPO` | Docker Hub model reference. |
| `LLAMA_CACHE` | Overrides the llama.cpp model cache directory. |
| `LLAMA_OFFLINE` | Uses cache only and prevents network downloads. |
| `HF_TOKEN` | Hugging Face token for gated downloads. |
| `LLAMA_ARG_MODELS_DIR` | Router-mode local GGUF directory. |
| `LLAMA_ARG_MODELS_PRESET` | Router preset file. |
| `LLAMA_ARG_MODELS_MAX` | Router simultaneous loaded-model cap. |
| `LLAMA_ARG_MODELS_AUTOLOAD` | Router automatic model loading. |
| `LLAMA_ARG_EMBEDDINGS` | Embedding-serving mode. |
| `LLAMA_ARG_RERANKING` | Reranking-serving mode. |
| `LLAMA_ARG_ENDPOINT_METRICS` | Enables `/metrics`. |
| `LLAMA_ARG_ENDPOINT_PROPS` | Enables `POST /props`. |
| `LLAMA_ARG_ENDPOINT_SLOTS` | Controls `/slots`; default enabled. |
| `LLAMA_ARG_N_PARALLEL` | Number of server slots. |
| `LLAMA_ARG_N_GPU_LAYERS` | GPU-offloaded layer count. |
| `LLAMA_ARG_CTX_SIZE` | Context size. |
| `LLAMA_ARG_JINJA` | Jinja chat-template engine; tool use depends on it. |

Traps:

- `LLAMA_CACHE` is llama.cpp's own model cache, not the Hugging Face cache. Current source defaults to `~/Library/Caches/llama.cpp` on macOS, `${XDG_CACHE_HOME:-~/.cache}/llama.cpp` on Linux, and `%LOCALAPPDATA%\llama.cpp` on Windows.
- `GET /props` is always registered, but `POST /props` requires `--props`.
- `--api-prefix` changes all endpoint paths for clients and detectors.
- The Web UI root remains public under API-key auth.

## Models

### Model ID Grammar

Accepted model identifier forms:

- `--alias <id>`: the id clients see in `/v1/models`.
- Local GGUF path or filename: `llama-server -m /models/model-Q4_K_M.gguf`.
- Hugging Face shorthand: `<user>/<model>[:quant]`, for example `unsloth/phi-4-GGUF:q4_k_m`.
- Specific Hugging Face file: `-hf <repo> -hff <file.gguf>`.
- Direct URL: `--model-url https://.../model.gguf`.
- Docker Hub selector: `--docker-repo [repo/]model[:quant]`.
- Router mode: model ids discovered from the llama.cpp cache or from `--models-dir`.

Runtime model format is GGUF. Safetensors or PyTorch checkpoints must be converted before serving.

### Acquisition Paths

| Method | Example | Notes |
| --- | --- | --- |
| Hugging Face | `llama-server -hf unsloth/phi-4-GGUF:q4_k_m` | Downloads GGUF into the llama.cpp cache; `HF_TOKEN` supports gated repos. |
| Manual | `llama-server -m /path/to/model.gguf --alias local-coder` | Loads a local GGUF directly. |
| Registry | `llama-server --docker-repo ai/gemma3:Q4_K_M` | Current help exposes Docker Hub model references. |
| In app | Web UI at `http://localhost:8080` | Chat UI for available models, not the primary acquisition path. |

### Model Store Paths

| OS | Default cache path | Notes |
| --- | --- | --- |
| macOS | `~/Library/Caches/llama.cpp` | Observed directory exists on this host. Override with `LLAMA_CACHE`. |
| Linux | `${XDG_CACHE_HOME:-~/.cache}/llama.cpp` | Source-derived. Override with `LLAMA_CACHE`. |
| Windows | `%LOCALAPPDATA%\llama.cpp` | Source-derived. Override with `LLAMA_CACHE`. |

## Capabilities

| Capability | Status | Notes |
| --- | --- | --- |
| Hardware acceleration | Metal, CUDA, ROCm/HIP, Vulkan, SYCL, OpenVINO, MUSA, CANN, OpenCL, RPC, CPU | Backend depends on build artifact and flags. |
| Multi-model | yes | Router mode through `--models-dir` or `--models-preset`; single-model mode serves one model per process. |
| Parallel requests | yes | `-np` / `--parallel` slots and continuous batching. |
| SSE streaming | yes | OpenAI, Anthropic, and native completion paths can stream. |
| Tool/function calling | conditional | Requires Jinja chat templates; enabled by default in current builds. |
| Embeddings | yes | Use embedding-capable models and `--embedding` / `--embeddings` where appropriate. |
| Reranking | yes | Requires `--rerank` / `--reranking`. |
| Web UI | yes | Default `http://localhost:8080`; disable with `--no-webui`. |

## Agentic CLI Integration

### OpenCode Provider Block

```json
{
  "provider": {
    "llamacpp": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "Llama.cpp (local)",
      "options": { "baseURL": "http://localhost:8080/v1" },
      "models": {
        "local-coder": { "name": "Local coder via llama-server" }
      }
    }
  }
}
```

Set the model key to the `--alias` value, the GGUF filename/path reported by `/v1/models`, or the router model id.

### Claude Code Via Anthropic Endpoint

`llama-server` can be used with Anthropic-compatible clients through `/v1/messages`:

```bash
export ANTHROPIC_BASE_URL=http://localhost:8080
export ANTHROPIC_AUTH_TOKEN=<value-of-LLAMA_API_KEY-if-set>
claude
```

The official Hugging Face blog example uses `ANTHROPIC_BASE_URL=http://127.0.0.1:8080 claude`. Use a tool-capable coding model and keep `--jinja` enabled for agentic tool use.

### Runner-Native Hooks

`llama-server` has no built-in `launch codex`, `launch claude`, or similar agent-wiring command. Integration is by starting the server and pointing clients at its HTTP API.

## Changelog

- 2026-07-03: Refreshed by Codex against local `llama-server` binary/source and current upstream documentation. Confirmed the host has build `8168 (723c71064)` installed but no running listener on port 8080.
- 2026-07-03: Corrected model cache paths to llama.cpp's `LLAMA_CACHE` / platform cache behavior instead of Hugging Face cache defaults.
- 2026-07-03: Added router-mode model-management endpoints and environment variables.
- 2026-07-03: Added source-verified Ollama-style compatibility aliases and noted the detection ambiguity of `/api/tags`.
- 2026-07-03: Verified Anthropic Messages API `since_version` as release tag `b7187`.
- 2026-07-03: Clarified API-key public allowlist and auth-gated metadata endpoints from current source.

## Sources

- [Llama.cpp homepage](https://llama.app)
- [Llama.cpp repository](https://github.com/ggml-org/llama.cpp)
- [Llama.cpp releases](https://github.com/ggml-org/llama.cpp/releases)
- [llama-server README and API reference](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md)
- [llama-server source route registration](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/server.cpp)
- [llama-server auth middleware](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/server-http.cpp)
- [llama.cpp cache directory source](https://github.com/ggml-org/llama.cpp/blob/master/common/common.cpp)
- [Function calling documentation](https://github.com/ggml-org/llama.cpp/blob/master/docs/function-calling.md)
- [Llama.cpp REST API changelog](https://github.com/ggml-org/llama.cpp/issues/9291)
- [Release b7187](https://github.com/ggml-org/llama.cpp/releases/tag/b7187)
- [PR #17570: Anthropic Messages API support](https://github.com/ggml-org/llama.cpp/pull/17570)
- [Hugging Face blog: New in llama.cpp: Anthropic Messages API](https://huggingface.co/blog/ggml-org/anthropic-messages-api-in-llamacpp)
- [OpenCode provider documentation](https://opencode.ai/docs/providers/)
- [Offline agentic coding with llama-server discussion](https://github.com/ggml-org/llama.cpp/discussions/14758)

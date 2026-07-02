---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7

summary: Llama.cpp is an open-source C/C++ LLM inference engine whose `llama-server` binary exposes OpenAI- and Anthropic-compatible HTTP endpoints for local GGUF models.
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
  Optional via `--api-key KEY`, `--api-key-file FNAME`, or `LLAMA_API_KEY`. When
  set, the server still allows unauthenticated `GET /health` and `GET /models`
  (observed on this host in router mode); authenticated requests must send
  `Authorization: Bearer KEY` or `X-Api-Key: KEY`. All inference endpoints and
  metadata endpoints outside the public allowlist, including `/props`, `/slots`,
  and `/metrics`, require the key when auth is enabled.

platforms:
  - os: macos
    support: native
    binary: llama-server
    alt_binaries: ["server"]
    install: ["brew install llama.cpp", "pre-built tar.gz from GitHub releases", "build from source with CMake", "conda-forge: conda install -c conda-forge llama-cpp", "MacPorts: sudo port install llama.cpp", "nix profile install nixpkgs#llama-cpp"]
    process_model: foreground
    service: none
    notes: First-class Apple Silicon support (Metal). Observed on this host at /Users/ken/coding/ai/llama.cpp/build/bin/llama-server (build 8168).
  - os: linux
    support: native
    binary: llama-server
    alt_binaries: ["server"]
    install: ["pre-built tar.gz from GitHub releases", "build from source with CMake", "conda-forge", "nix profile install nixpkgs#llama-cpp", "Docker ghcr.io/ggml-org/llama.cpp:server"]
    process_model: foreground
    service: none
    notes: No official systemd unit; users typically run under process managers or Docker. CUDA, ROCm, Vulkan, SYCL builds available.
  - os: windows
    support: native
    binary: llama-server.exe
    alt_binaries: ["server.exe"]
    install: ["winget install llama.cpp", "pre-built zip from GitHub releases", "build from source with CMake", "conda-forge"]
    process_model: foreground
    service: none
    notes: No Windows service or tray app; run from terminal or wrap manually.

api_standards:
  - standard: openai_compatible
    supported: yes
    base_url: http://localhost:8080/v1
    key_paths:
      - /v1/models
      - /v1/completions
      - /v1/chat/completions
      - /v1/responses
      - /v1/embeddings
      - /v1/rerank
    auth: optional_api_key
    since_version: unknown
    deviations:
      - "Streaming uses SSE, not JSONL."
      - "Some optional OpenAI fields (e.g. certain logprobs shapes) may not be implemented; see server README for current coverage."
      - "Tool calling requires --jinja (enabled by default)."
      - "Model ID in requests is the alias set with --alias, the GGUF filename, or whatever --served-model-name equivalent is configured."
    docs_url: https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md
  - standard: anthropic_compatible
    supported: yes
    base_url: http://localhost:8080
    key_paths:
      - /v1/messages
      - /v1/messages/count_tokens
    auth: optional_api_key
    since_version: b7187
    deviations:
      - "Anthropic SDKs append /v1/messages themselves, so base_url omits /v1."
      - "Implemented by converting Anthropic requests to OpenAI chat completions internally; tool_use requires --jinja."
      - "Not all Anthropic features (e.g. extended thinking, prompt caching, citations, PDF content) are supported."
    docs_url: https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md
  - standard: native
    supported: yes
    base_url: http://localhost:8080
    key_paths:
      - /health
      - /models
      - /completion
      - /tokenize
      - /detokenize
      - /embedding
      - /reranking
      - /infill
      - /apply-template
      - /slots
      - /props
    auth: optional_api_key
    since_version: unknown
    deviations:
      - "Native endpoints coexist with /v1 paths; /completion and /embedding are not OpenAI-shaped."
      - "Embeddings and reranking endpoints require --embeddings/--rerank flags."
    docs_url: https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md

metadata_endpoints:
  - purpose: health
    method: get
    path: /health
    gated_by: ""
    auth_gated: false
    response_hint: '{"status":"ok"}'
    notes: Also available as /v1/health. Returns 503 with loading message while the model is being loaded. Observed unauthenticated on this host.
  - purpose: health
    method: get
    path: /v1/health
    gated_by: ""
    auth_gated: false
    response_hint: '{"status":"ok"}'
    notes: Alias for /health.
  - purpose: version
    method: get
    path: /props
    gated_by: ""
    auth_gated: true
    response_hint: '{"build_info":"b..."}'
    notes: No dedicated /version endpoint. Build info is inside /props and as system_fingerprint in completion responses. Auth-gated when --api-key is set (observed 401 without key in router mode).
  - purpose: model_list
    method: get
    path: /models
    gated_by: ""
    auth_gated: false
    response_hint: '{"object":"list","data":[{"owned_by":"llamacpp"}]} or router metadata with status/path fields'
    notes: Also available as /v1/models. In single-model mode, the OpenAI-style response includes owned_by=llamacpp. In router mode, GET /models returns richer router metadata with per-model status/path fields. There is no /api/tags route; that is Ollama-only. The public allowlist in server-http.cpp is /health, /v1/health, /models, /v1/models, /, plus embedded UI assets. Observed unauthenticated on this host in router mode.
  - purpose: model_list
    method: get
    path: /v1/models
    gated_by: ""
    auth_gated: false
    response_hint: '{"object":"list","data":[{"owned_by":"llamacpp"}]}'
    notes: Reliable owned_by=llamacpp marker in single-model mode. In router mode, the response is forwarded from the selected loaded model, or empty when no model is loaded.
  - purpose: model_info
    method: get
    path: /props
    gated_by: ""
    auth_gated: true
    response_hint: '{"model_alias":"...","model_path":"...","modalities":{"vision":false,"audio":false}}'
    notes: Returns loaded model path, alias, generation defaults, chat template, and capabilities. Auth-gated when --api-key is set.
  - purpose: loaded_models
    method: get
    path: /slots
    gated_by: ""
    auth_gated: true
    response_hint: '[{"id":0,"is_processing":false},...]'
    notes: No dedicated loaded-models endpoint; /slots exposes per-slot processing state. Can be disabled with --no-slots. Requires auth when --api-key is set.
  - purpose: metrics
    method: get
    path: /metrics
    gated_by: --metrics
    auth_gated: true
    response_hint: "# HELP llamacpp:prompt_tokens_total"
    notes: Prometheus-compatible metrics. Returns 501 when the server was not started with --metrics. Requires auth when --api-key is set; observed 401 without key in router mode.
  - purpose: admin_ui
    method: get
    path: /
    gated_by: ""
    auth_gated: false
    response_hint: "Error: gzip is not supported by this browser"
    notes: Serves the built-in Web UI by default; can be disabled with --no-webui. Observed on this host returning the gzipped SPA root.

detection:
  - os: macos
    method: binary
    target: llama-server
    expect: "version: 8168 (723c71064)"
    confidence: observed
    notes: Observed on this host at /Users/ken/coding/ai/llama.cpp/build/bin/llama-server. Binary name is the same on Linux.
  - os: linux
    method: binary
    target: llama-server
    expect: "version: ..."
    confidence: documented
    notes: Pre-built binaries and package managers install llama-server to PATH (location varies by install method).
  - os: windows
    method: binary
    target: llama-server.exe
    expect: "version: ..."
    confidence: documented
    notes: Pre-built zip contains llama-server.exe.
  - os: all
    method: process
    target: llama-server
    expect: "command line contains -m, --model, -hf, or --host"
    confidence: observed
    notes: Observed on this host in router mode. The process name is the same on all platforms.
  - os: all
    method: port
    target: "8080"
    expect: ""
    confidence: documented
    notes: Default port 8080 is shared with several other local runners and services; must confirm with an HTTP probe.
  - os: all
    method: http
    target: GET /health
    expect: '{"status":"ok"}'
    confidence: observed
    notes: Strong identity marker. Ungated even when --api-key is enabled. Observed on this host in router mode.
  - os: all
    method: http
    target: GET /v1/models
    expect: '{"owned_by":"llamacpp"}'
    confidence: documented
    notes: OpenAI-style model list in single-model mode. Ungated even when --api-key is enabled. Router-mode GET /models returns richer per-model metadata with status/path fields instead.
  - os: all
    method: http
    target: GET /props
    expect: '{"build_info":"b..."}'
    confidence: documented
    notes: Contains build_info with build number and commit hash in single-model mode. Requires API key when auth is enabled; router mode exposes router properties and was observed gated by auth.
  - os: all
    method: http
    target: GET /metrics
    expect: "llamacpp:prompt_tokens_total"
    confidence: documented
    notes: Only present when server started with --metrics; 501 otherwise.
  - os: all
    method: config_file
    target: none
    expect: ""
    confidence: inferred
    notes: llama-server has no primary config file. Configuration is via CLI flags and LLAMA_ARG_* / LLAMA_API_KEY environment variables.

config_mechanism: mixed

config_files:
  - os: all
    path: ""
    format: other
    role: none
    notes: llama-server has no primary config file. The --webui-config-file and --webui-config flags accept JSON for WebUI defaults only.
  - os: all
    path: "path supplied to --models-preset"
    format: ini
    role: router_model_preset
    notes: Router mode can load model presets from an INI file supplied with --models-preset.

env_vars:
  - name: LLAMA_ARG_HOST
    effect: "Bind address for the HTTP server (default 127.0.0.1). Accepts an IP or a .sock path for UNIX socket."
  - name: LLAMA_ARG_PORT
    effect: "Listen port for the HTTP server (default 8080)."
  - name: LLAMA_API_KEY
    effect: "API key(s) for optional authentication. Multiple keys can be comma-separated; equivalent to --api-key."
  - name: LLAMA_ARG_MODEL
    effect: "Path to the GGUF model to load; equivalent to -m / --model."
  - name: LLAMA_ARG_HF_REPO
    effect: "Hugging Face repo to download/load in the form <user>/<model>[:quant]; equivalent to -hf."
  - name: LLAMA_ARG_HF_FILE
    effect: "Specific GGUF file inside the Hugging Face repo; equivalent to -hff."
  - name: LLAMA_ARG_EMBEDDINGS
    effect: "Enable embedding-only mode; required for /v1/embeddings."
  - name: LLAMA_ARG_RERANKING
    effect: "Enable reranking endpoint and embedding mode."
  - name: LLAMA_ARG_ENDPOINT_METRICS
    effect: "Enable GET /metrics Prometheus endpoint."
  - name: LLAMA_ARG_ENDPOINT_PROPS
    effect: "Enable POST /props for changing global properties."
  - name: LLAMA_ARG_ENDPOINT_SLOTS
    effect: "Expose GET /slots endpoint (default enabled)."
  - name: LLAMA_ARG_MODELS_PRESET
    effect: "Path to an INI file containing model presets for router mode; equivalent to --models-preset."
  - name: LLAMA_ARG_N_PARALLEL
    effect: "Number of server slots / parallel requests (default -1 auto); equivalent to -np."
  - name: LLAMA_ARG_N_GPU_LAYERS
    effect: "Number of layers to offload to GPU (default auto); equivalent to -ngl."
  - name: LLAMA_ARG_CTX_SIZE
    effect: "Context size (default 0 = loaded from model); equivalent to -c."
  - name: LLAMA_OFFLINE
    effect: "Force use of cache and prevent network access."
  - name: HF_TOKEN
    effect: "Hugging Face access token used when downloading models with -hf."

model_id_grammar: |
  Model IDs for `llama-server` are determined by whatever the server was told to
  serve. The canonical forms are:

  1. A filesystem path to a GGUF file, e.g. `/path/to/model.gguf`.
  2. A Hugging Face repo shorthand with optional quantization tag,
     e.g. `ggml-org/gemma-3-1b-it-GGUF` or `unsloth/phi-4-GGUF:q4_k_m`
     (used with `-hf`).
  3. An explicit alias set with `--alias` (or `LLAMA_ARG_ALIAS`), which is what
     clients see in `/v1/models` and use in chat-completion `model` fields.
  4. If no alias is set, the GGUF filename (e.g. `model-Q4_K_M.gguf`) becomes the
     model ID.

  There is no registry namespace grammar like Ollama's `name:tag`.

model_formats:
  - gguf

model_acquisition:
  - method: huggingface
    example: "llama-server -hf ggml-org/gemma-3-1b-it-GGUF"
    notes: Downloads the GGUF into the standard Hugging Face cache directory and loads it. Optional `:quant` tag selects a quantization.
  - method: manual
    example: "llama-server -m /path/to/model-Q4_K_M.gguf"
    notes: Load a locally downloaded or converted GGUF file directly.
  - method: registry
    example: "Docker: docker run ghcr.io/ggml-org/llama.cpp:server -m /models/model.gguf"
    notes: No first-party model registry; Docker images and GitHub releases provide the runner binary only.
  - method: in_app
    example: "Web UI at http://localhost:8080 can be used to chat once the server is started with a model."
    notes: The Web UI is a client for the running server; it does not download models itself.

model_store_paths:
  - os: all
    path: ""
    notes: llama-server has no dedicated model store. It loads the GGUF path supplied via -m or downloads from Hugging Face into the standard HF cache (~/.cache/huggingface/hub on Linux/macOS, %USERPROFILE%\.cache\huggingface\hub on Windows).
  - os: macos
    path: ~/.cache/huggingface/hub
    notes: Observed HF cache location on this host when using -hf. Symlinks or relocated caches are common.
  - os: linux
    path: ~/.cache/huggingface/hub
    notes: Standard Hugging Face cache location.
  - os: windows
    path: C:\Users\%username%\.cache\huggingface\hub
    notes: Standard Hugging Face cache location.

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
  - cpu

concurrency:
  multi_model: true
  parallel_requests: true
  notes: Single-model mode serves one model per process, but router mode can load multiple models in one process. Both modes support multiple parallel decoding slots via `-np` / `--parallel` (continuous batching).

streaming_sse: true
tool_calling: conditional
tool_calling_notes: >
  OpenAI-style function calling and Anthropic tool_use are supported when the
  server runs with `--jinja` (enabled by default). Native tool formats are
  recognized for Llama 3.1/3.2/3.3, Functionary v3.1/v3.2, Hermes 2/3, Qwen 2.5
  (incl. Coder), Mistral Nemo, Firefunction v2, Command R7B, and DeepSeek R1.
  Generic fallback works with `--chat-template-file`. Sending `tools` with
  `--no-jinja` results in an error.

embeddings: true
rerank: true
web_ui_url: http://localhost:8080

integration_hooks: []

traps:
  - "`--embedding` / `--embeddings` puts the server into embedding-only mode; chat/completion endpoints will fail because the loaded model does not compute logits."
  - "`--rerank` / `--reranking` enables reranking but also forces embedding mode and pooling type rank; it is not a chat server."
  - "`LLAMA_ARG_PORT` sets the API port. There is no `LLAMA_PORT` variable; the similarly-named `VLLM_PORT` from vLLM is irrelevant here."
  - "`--api-key` does not gate `/health`, `/v1/health`, `/models`, `/v1/models`, `/`, or embedded UI assets; `/props`, `/slots`, and `/metrics` require the key when auth is enabled."
  - "`-hf` downloads share the Hugging Face cache directory; there is no separate `llama.cpp` model store unless the user creates one."
  - "The model ID seen by clients is the `--alias` value or the GGUF filename, not a registry name."
  - "In single-model mode, the request `model` field is ignored and any value is accepted; it only routes requests in router mode."

opencode_example: '{"provider":{"llamacpp":{"npm":"@ai-sdk/openai-compatible","name":"Llama.cpp (local)","options":{"baseURL":"http://localhost:8080/v1"},"models":{"gemma-3-1b-it.Q4_K_M.gguf":{"name":"Gemma 3 1B Q4_K_M (local)"}}}}}'

changes: []
requires_claudine_update: true
reason: >
  New local runner entry. Claudine's `sniff` detection surface should add probes
  for the `llama-server` binary / process, TCP port 8080 (with HTTP
  disambiguation), the ungated `GET /health` identity marker, `GET /v1/models`
  with `owned_by: llamacpp` in single-model mode, and the model-path/alias
  grammar used by `llama-server`.
---

# Llama.cpp

## Introduction to Llama.cpp

[Llama.cpp](https://llama.app) is an open-source C/C++ inference engine for
large language models, distributed under the MIT license. It is the reference
runtime for the [ggml](https://github.com/ggml-org/ggml) tensor library and is
best known for running quantized GGUF models efficiently on consumer hardware.
The project includes `llama-server`, a lightweight HTTP server that exposes
OpenAI-compatible, Anthropic-compatible, and native REST endpoints for local
models.

| Resource | URL |
| --- | --- |
| Homepage | https://llama.app |
| Repository | https://github.com/ggml-org/llama.cpp |
| Releases | https://github.com/ggml-org/llama.cpp/releases |
| Server documentation | https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md |
| Function calling docs | https://github.com/ggml-org/llama.cpp/blob/master/docs/function-calling.md |
| REST API changelog | https://github.com/ggml-org/llama.cpp/issues/9291 |

## Platforms and Installation

| OS | Support | Binary | Install methods | Process model | Service |
| --- | --- | --- | --- | --- | --- |
| macOS | native | `llama-server` | Homebrew, GitHub releases tar.gz, build from source, conda-forge, MacPorts, Nix | foreground | none |
| Linux | native | `llama-server` | GitHub releases tar.gz, build from source, conda-forge, Nix, Docker | foreground | none |
| Windows | native | `llama-server.exe` | winget, GitHub releases zip, build from source, conda-forge | foreground | none |

Notes:

- There is no first-party daemon, tray app, or service. Users run
  `llama-server` in a terminal, under a process manager, or in a container.
- Pre-built releases are published as `llama-b<build>-bin-<os>-<arch>.tar.gz`
  / `.zip` with CPU, CUDA, Vulkan, ROCm, SYCL, and other backend variants.
- Observed on this host: `/Users/ken/coding/ai/llama.cpp/build/bin/llama-server`,
  version `8168 (723c71064)`, Apple Silicon Metal build.

## API Surface

Default listen address is `127.0.0.1:8080`.

### OpenAI-compatible API

Base URL: `http://localhost:8080/v1`

Supported paths include `/v1/models`, `/v1/completions`, `/v1/chat/completions`,
`/v1/responses`, `/v1/embeddings`, and `/v1/rerank`. Authentication is optional;
when `--api-key` is set, clients must send `Authorization: Bearer KEY` or
`X-Api-Key: KEY`.

### Anthropic Messages API

Base URL: `http://localhost:8080` (Anthropic SDKs append `/v1/messages`
themselves).

Supported paths: `POST /v1/messages` and `POST /v1/messages/count_tokens`.
Added in build `b7187` (2025-11-28). Tool use requires `--jinja`.

### Native API family

Base URL: `http://localhost:8080`

Native endpoints include `/health`, `/models`, `/completion`, `/tokenize`,
`/detokenize`, `/embedding`, `/reranking`, `/infill`, `/apply-template`,
`/slots`, and `/props`. These are documented in the
[server README](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md).

### Metadata endpoints

| Method | Path | Purpose | Gated by | Auth gated |
| --- | --- | --- | --- | --- |
| GET | `/health`, `/v1/health` | Health check | — | no |
| GET | `/models`, `/v1/models` | Model list | — | no |
| GET | `/props` | Model info / build info | — | yes when auth enabled |
| GET | `/slots` | Slot / processing state | `--no-slots` disables | yes when auth enabled |
| GET | `/metrics` | Prometheus metrics | `--metrics` | yes when auth enabled |
| GET | `/` | Web UI | `--no-webui` disables | no |

## Detection

A detector should probe in this order:

1. **Binary on PATH**: `llama-server` (macOS/Linux) or `llama-server.exe`
   (Windows). Historical name `server` may exist in very old builds.
2. **Process**: `llama-server` running with model arguments (`-m`, `-hf`,
   `--host`, `--port`).
3. **Port**: TCP 8080. This port is shared with other local runners and
   services, so an HTTP probe is required for disambiguation.
4. **HTTP identity**:
   - `GET /health` returning `{"status":"ok"}` confirms the server is ready.
   - `GET /v1/models` returning `owned_by: llamacpp` is a reliable
     single-model-mode marker for Llama.cpp.
   - In router mode, `GET /models` returns richer per-model metadata such as
     status and path instead of relying on the `owned_by` marker.
   - `GET /props` returns `build_info` with the build number and commit hash.
5. **Config file / model store**: none by default; check the path supplied to
   `-m` or the Hugging Face cache when `-hf` is used.

Observed on this host: binary at `/Users/ken/coding/ai/llama.cpp/build/bin/llama-server`,
version `8168 (723c71064)`. A router-mode probe on port 18080 confirmed
`/health`, `/models`, and `/v1/models` are public under `--api-key`, while
`/props`, `/slots`, and `/metrics` require the key.

## Configuration

`llama-server` is configured primarily with command-line flags, with matching
`LLAMA_ARG_*` environment variables for many options. Router mode can also load
model presets from `--models-preset` INI files. CLI flags take precedence over
environment variables when both are set.

Important environment variables:

| Variable | Effect |
| --- | --- |
| `LLAMA_ARG_HOST` | Bind address (default `127.0.0.1`). |
| `LLAMA_ARG_PORT` | Listen port (default `8080`). |
| `LLAMA_API_KEY` | Optional API key(s). |
| `LLAMA_ARG_MODEL` | GGUF path to load. |
| `LLAMA_ARG_HF_REPO` | Hugging Face repo to download/load. |
| `LLAMA_ARG_EMBEDDINGS` | Enable embedding mode. |
| `LLAMA_ARG_RERANKING` | Enable reranking endpoint. |
| `LLAMA_ARG_ENDPOINT_METRICS` | Enable `/metrics`. |
| `LLAMA_ARG_MODELS_PRESET` | INI file containing model presets for router mode. |
| `LLAMA_ARG_N_PARALLEL` | Number of parallel slots. |
| `LLAMA_ARG_N_GPU_LAYERS` | GPU layer offload count. |
| `LLAMA_OFFLINE` | Disable network access. |

## Models

### Model ID grammar

`llama-server` does not use a registry namespace. Clients reference the model by:

1. The value of `--alias` if set.
2. The GGUF filename if no alias is set (e.g. `model-Q4_K_M.gguf`).
3. For `-hf` loads, the Hugging Face repo shorthand with optional `:quant` tag
   (e.g. `ggml-org/gemma-3-1b-it-GGUF` or `bartowski/phi-4-GGUF:q4_k_m`).

### Model formats

Runtime format is GGUF only.

### Acquisition paths

- **Hugging Face**: `llama-server -hf <user>/<repo>[:quant]` downloads into the
  standard Hugging Face cache.
- **Manual**: `llama-server -m /path/to/model.gguf` loads a local GGUF.
- **Conversion**: Models in PyTorch/Safetensors can be converted to GGUF with
  the `convert_*.py` scripts or the GGUF-my-repo Hugging Face space.

### Model store paths

There is no default Llama.cpp model store. Locally supplied GGUFs are loaded
from the path given to `-m`. Hugging Face downloads use the standard HF cache:

| OS | Path |
| --- | --- |
| macOS / Linux | `~/.cache/huggingface/hub` |
| Windows | `C:\Users\%username%\.cache\huggingface\hub` |

## Capabilities

| Capability | Status | Notes |
| --- | --- | --- |
| Hardware backends | Metal, CUDA, ROCm, Vulkan, SYCL, OpenVINO, MUSA, CANN, OpenCL, CPU | Backend selected at build time. |
| Multi-model | Yes, in router mode | Single-model mode serves one model per process; router mode (`--models-dir` / `--models-preset`) can load multiple models. |
| Parallel requests | Yes | Via `-np` / `--parallel` slots with continuous batching. |
| SSE streaming | Yes | Used by `/v1/chat/completions`, `/v1/completions`, `/completion`, and Anthropic `/v1/messages`. |
| Tool/function calling | Conditional | Requires `--jinja` (default enabled). Native formats for Llama 3.x, Functionary, Hermes, Qwen 2.5, Mistral Nemo, Firefunction, Command R7B, DeepSeek R1; generic fallback available. |
| Embeddings | Yes | Requires `--embedding` / `--embeddings`. |
| Reranking | Yes | Requires `--rerank` / `--reranking`. |
| Web UI | Yes | Built-in UI at `http://localhost:8080`; disable with `--no-webui`. |

## Agentic CLI Integration

### OpenCode provider block

```json
{
  "provider": {
    "llamacpp": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "Llama.cpp (local)",
      "options": { "baseURL": "http://localhost:8080/v1" },
      "models": {
        "gemma-3-1b-it.Q4_K_M.gguf": { "name": "Gemma 3 1B Q4_K_M (local)" }
      }
    }
  }
}
```

Set the model key to the GGUF filename or the `--alias` value configured when
starting `llama-server`. In single-model mode, `llama-server` ignores the
request `model` field and accepts any value; that field only selects a backend
model in router mode.

### Claude Code via Anthropic endpoint

Because `llama-server` supports `POST /v1/messages`, Claude Code can be pointed
at it directly:

```bash
export ANTHROPIC_BASE_URL=http://localhost:8080
export ANTHROPIC_AUTH_TOKEN=<value-of-LLAMA_API_KEY-if-set>
claude
```

### Runner-native hooks

`llama-server` has no built-in `launch` subcommand or coding-agent integration
hook. It is intended to be started manually or by a wrapper and consumed via its
HTTP API.

## Sources

- [Llama.cpp homepage](https://llama.app)
- [Llama.cpp repository](https://github.com/ggml-org/llama.cpp)
- [Llama.cpp releases](https://github.com/ggml-org/llama.cpp/releases)
- [llama-server README](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md)
- [Function calling documentation](https://github.com/ggml-org/llama.cpp/blob/master/docs/function-calling.md)
- [Install documentation](https://github.com/ggml-org/llama.cpp/blob/master/docs/install.md)
- [Llama.cpp REST API changelog](https://github.com/ggml-org/llama.cpp/issues/9291)
- [Build b7187 release tag for PR #17570 merge](https://github.com/ggml-org/llama.cpp/releases/tag/b7187)

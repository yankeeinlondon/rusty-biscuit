---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7

summary: vLLM is an open-source high-throughput inference and serving engine for large language models. It exposes OpenAI-compatible, Anthropic-compatible, pooling, and speech-to-text HTTP endpoints from a single-model server.
homepage: https://vllm.ai
docs_url: https://docs.vllm.ai
repo_url: https://github.com/vllm-project/vllm
api_reference_url: https://docs.vllm.ai/en/latest/online_serving/
open_source: full

has_official_schema: formal
schema_url: http://localhost:8000/openapi.json

default_port: 8000
default_bind: 0.0.0.0
auth: optional_api_key
auth_notes: >
  Authentication is disabled by default. Pass --api-key or set VLLM_API_KEY to
  require an Authorization: Bearer <key> header. Multiple keys can be accepted.

platforms:
  - os: linux
    support: native
    binary: vllm
    alt_binaries: ["python", "uv"]
    install: ["pip install vllm", "uv pip install vllm", "Docker vllm/vllm-openai"]
    process_model: foreground
    service: user-managed (systemd, Docker, Kubernetes, or foreground `vllm serve`)
    notes: Primary supported platform. CUDA, ROCm, CPU, and Intel XPU backends are available.
  - os: macos
    support: separate_project
    binary: vllm
    alt_binaries: ["vllm-metal"]
    install: ["Build official Apple Silicon CPU support from source; follow vLLM-Metal install guide at https://github.com/vllm-project/vllm-metal for GPU acceleration"]
    process_model: foreground
    service: user-managed foreground process
    notes: Official experimental Apple Silicon CPU support exists but is build-from-source only with no pre-built wheels; Apple Silicon GPU acceleration is the separate vLLM-Metal project, which uses MLX instead of PyTorch and requires mlx-community models.
  - os: windows
    support: wsl
    binary: vllm
    alt_binaries: []
    install: ["Run inside WSL2 with Linux install steps; native Windows is not officially supported."]
    process_model: foreground
    service: user-managed WSL2 process
    notes: Use WSL2 with CUDA or CPU backend. Native Windows is not a supported target.

api_standards:
  - standard: openai_compatible
    supported: yes
    base_url: http://localhost:8000/v1
    key_paths:
      - /v1/models
      - /v1/completions
      - /v1/chat/completions
      - /v1/chat/completions/batch
      - /v1/embeddings
      - /v1/audio/transcriptions
      - /v1/audio/translations
      - /v1/responses
      - /v1/load_lora_adapter
      - /v1/unload_lora_adapter
    auth: optional_api_key
    since_version: "unknown"
    deviations:
      - "One vLLM server process hosts exactly one model at a time."
      - "suffix parameter is not supported for /v1/completions."
      - "user parameter is ignored for /v1/chat/completions."
      - "/v1/responses is non-stateful (no previous_response_id storage)."
    docs_url: https://docs.vllm.ai/en/latest/online_serving/
  - standard: anthropic_compatible
    supported: yes
    base_url: http://localhost:8000
    key_paths:
      - /v1/messages
      - /v1/messages/count_tokens
    auth: optional_api_key
    since_version: "v0.11.1"
    deviations:
      - "Anthropic SDKs append /v1/messages themselves, so base_url omits /v1."
      - "/v1/messages/count_tokens requires vLLM >= v0.17.0."
      - "Streaming, tools, system prompts, and thinking blocks are supported."
      - "Prompt caching, batches, citations, and PDF content blocks are not supported."
    docs_url: https://docs.vllm.ai/en/latest/online_serving/
  - standard: native
    supported: yes
    base_url: http://localhost:8000
    key_paths:
      - /health
      - /version
      - /metrics
      - /v1/models
      - /tokenize
      - /detokenize
      - /tokenizer_info
      - /pooling
      - /classify
      - /score
      - /rerank
      - /v1/score
      - /v1/rerank
      - /v2/rerank
      - /v1/realtime
      - /generative_scoring
    auth: optional_api_key
    since_version: "unknown"
    deviations:
      - "Native pooling endpoints require an embedding, classification, or scoring model."
      - "Speech endpoints require an ASR model."
    docs_url: https://docs.vllm.ai/en/latest/online_serving/

metadata_endpoints:
  - purpose: health
    method: get
    path: /health
    gated_by: ""
    auth_gated: true
    response_hint: "empty HTTP 200 body"
    notes: Returns an empty 200 response when the engine is healthy; returns 503 when the engine is dead.
  - purpose: version
    method: get
    path: /version
    gated_by: ""
    auth_gated: true
    response_hint: '{"version": "0.x.x"}'
    notes: Returns the running vLLM version string.
  - purpose: model_list
    method: get
    path: /v1/models
    gated_by: ""
    auth_gated: true
    response_hint: '{"object":"list","data":[{"id":"...","object":"model"}]}'
    notes: Lists the single model served by this instance (or aliases set via --served-model-name).
  - purpose: metrics
    method: get
    path: /metrics
    gated_by: ""
    auth_gated: true
    response_hint: "# HELP vllm ..."
    notes: Prometheus-compatible metrics; exposes GPU, scheduler, and request-level metrics.
  - purpose: model_info
    method: get
    path: /tokenizer_info
    gated_by: "--enable-tokenizer-info-endpoint"
    auth_gated: true
    response_hint: '{"chat_template":"...","tokenizer_class":"..."}'
    notes: Optional endpoint returning tokenizer and chat-template metadata.
  - purpose: load_model
    method: post
    path: /v1/load_lora_adapter
    gated_by: "--enable-lora and VLLM_ALLOW_RUNTIME_LORA_UPDATING=True"
    auth_gated: true
    response_hint: '{"success": true}'
    notes: Dynamically load a LoRA adapter at runtime. Not for production use without safeguards.
  - purpose: unload_model
    method: post
    path: /v1/unload_lora_adapter
    gated_by: "--enable-lora and VLLM_ALLOW_RUNTIME_LORA_UPDATING=True"
    auth_gated: true
    response_hint: '{"success": true}'
    notes: Unload a previously loaded LoRA adapter.
  - purpose: admin_ui
    method: get
    path: /docs
    gated_by: ""
    auth_gated: true
    response_hint: "FastAPI Swagger UI"
    notes: Auto-generated OpenAPI/Swagger documentation. Requires internet by default unless --enable-offline-docs is set.

detection:
  - os: linux
    method: binary
    target: vllm
    expect: "vllm serve [MODEL] [OPTIONS]"
    confidence: documented
    notes: Installed via pip/uv as an entry-point script. Not observed on this host.
  - os: linux
    method: process
    target: "vllm serve"
    expect: "vllm serve <model_id> --host ... --port 8000"
    confidence: documented
    notes: Foreground server process; may be wrapped by uv, docker, or systemd.
  - os: all
    method: port
    target: "8000"
    expect: ""
    confidence: observed
    notes: Default port is shared with other servers. On this host, TCP 8000 is owned by oMLX (`omlx-server`), so an HTTP probe is required to confirm vLLM.
  - os: all
    method: http
    target: GET /version
    expect: '{"version":"..."}'
    confidence: documented
    notes: Strong identity marker. Ungated when auth is off; auth_gated when --api-key is set.
  - os: all
    method: http
    target: GET /v1/models
    expect: '{"object":"list","data":[{"id":"..."}]}'
    confidence: documented
    notes: Confirms an OpenAI-compatible vLLM instance when a single model is returned.
  - os: all
    method: config_file
    target: ~/.config/vllm
    expect: ""
    confidence: documented
    notes: vLLM config root used at installation/runtime; actual contents depend on usage.

config_mechanism: mixed

config_files:
  - os: all
    path: ~/.config/vllm
    format: other
    role: vLLM configuration root
    notes: Defaults to ~/.config/vllm unless XDG_CONFIG_HOME is set. Used for installation and runtime config discovery.
  - os: all
    path: serve_args.yaml
    format: yaml
    role: CLI argument config file
    notes: Pass with `vllm serve --config serve_args.yaml`. Schema is documented at https://docs.vllm.ai/en/latest/configuration/serve_args.html.

env_vars:
  - name: VLLM_API_KEY
    effect: "API key required by the server when set. Equivalent to passing --api-key."
  - name: VLLM_HOST_IP
    effect: "Internal distributed-communication IP only. NOT the API server bind address."
  - name: VLLM_PORT
    effect: "Internal distributed-communication port only. NOT the API server port. This is a common trap."
  - name: VLLM_CACHE_ROOT
    effect: "Root directory for vLLM runtime cache (default ~/.cache/vllm)."
  - name: VLLM_CONFIG_ROOT
    effect: "Root directory for vLLM configuration files (default ~/.config/vllm)."
  - name: VLLM_USE_MODELSCOPE
    effect: "If true, download models from ModelScope instead of Hugging Face."
  - name: HF_HOME
    effect: "Hugging Face cache directory; controls where models are downloaded and stored."
  - name: VLLM_LOGGING_LEVEL
    effect: "Default log level for vLLM (default INFO)."
  - name: VLLM_CPU_KVCACHE_SPACE
    effect: "CPU backend key-value cache size in GiB (for example, 40 means 40 GiB)."
  - name: VLLM_WORKER_MULTIPROC_METHOD
    effect: "spawn or fork (default fork); controls worker process spawning."
  - name: VLLM_ALLOW_LONG_MAX_MODEL_LEN
    effect: "Set to 1 to allow --max-model-len beyond the model config maximum."

model_id_grammar: |
  vLLM model identifiers are HuggingFace model IDs (e.g. `Qwen/Qwen2.5-1.5B-Instruct`),
  local filesystem paths, or ModelScope IDs when `VLLM_USE_MODELSCOPE=true`. The API
  exposes the model under the value of `--model` unless overridden by one or more
  `--served-model-name` values. Examples: `meta-llama/Llama-3.1-8B-Instruct`,
  `/path/to/local/model`, `./my-gguf-model.gguf`.

model_formats:
  - safetensors
  - pytorch
  - gguf

model_acquisition:
  - method: huggingface
    example: "vllm serve Qwen/Qwen2.5-1.5B-Instruct"
    notes: Downloads from Hugging Face Hub to the HF cache on first load.
  - method: manual
    example: "vllm serve /path/to/local/model --served-model-name my-model"
    notes: Local Safetensors/PyTorch checkpoint or GGUF file. Chat template may need --chat-template.

model_store_paths:
  - os: all
    path: ~/.cache/huggingface/hub
    notes: Default Hugging Face Hub cache. Relocatable via HF_HOME or HF_HUB_CACHE.
  - os: all
    path: ~/.cache/vllm
    notes: vLLM runtime cache root, configurable via VLLM_CACHE_ROOT.

hardware_acceleration:
  - cuda
  - rocm
  - tpu
  - npu
  - cpu
  - metal

concurrency:
  multi_model: false
  parallel_requests: true
  notes: One vLLM server process serves exactly one model. Parallel requests against that model are supported via continuous batching.

streaming_sse: true
tool_calling: conditional
tool_calling_notes: >
  Tool calling is not enabled by default. Start the server with
  `--enable-auto-tool-choice --tool-call-parser <parser> [--chat-template <path>]`
  to enable it. Supported parsers include hermes, llama3_json, pythonic, mistral,
  qwen3_xml, deepseek_v3, and others. Named and required tool_choice use structured
  outputs and do not require --enable-auto-tool-choice.

embeddings: true
rerank: true
web_ui_url: ""

integration_hooks: []

traps:
  - "VLLM_PORT and VLLM_HOST_IP configure internal distributed communication, not the API server. Use --host and --port for the HTTP server."
  - "The default --host value is None, which binds all interfaces (displayed as 0.0.0.0), unlike many local runners that default to 127.0.0.1."
  - "Each vLLM server process hosts one model. To serve multiple models concurrently, run multiple server processes on different ports."
  - "Models without a chat template in tokenizer_config.json require --chat-template or chat requests will fail."
  - "--api-key (or VLLM_API_KEY) makes all endpoints, including /health and /version, require an Authorization header."
  - "Tool calling must be explicitly enabled with parser and chat-template flags; it does not work out of the box."
  - "Offline /docs requires --enable-offline-docs; otherwise Swagger UI tries to fetch CDN assets."

opencode_example: '{"provider":{"vllm":{"npm":"@ai-sdk/openai-compatible","name":"vLLM (local)","options":{"baseURL":"http://localhost:8000/v1","apiKey":"EMPTY"},"models":{"Qwen/Qwen2.5-1.5B-Instruct":{"name":"Qwen2.5 1.5B Instruct (local vLLM)"}}}}}'

changes: []
requires_claudine_update: true
reason: New local runner entry. Claudine's sniff detection surface should add probes for the `vllm` binary/entry point, the default port 8000, the `GET /version` identity marker, `GET /v1/models`, and the HuggingFace/vLLM cache paths; the model_catalog should treat HuggingFace model IDs and --served-model-name aliases as vLLM model identifiers.
---

# vLLM

[vLLM](https://vllm.ai) is an open-source high-throughput inference and serving
engine for large language models. It implements PagedAttention for efficient KV-cache
management and exposes OpenAI-compatible, Anthropic-compatible, pooling, and
speech-to-text HTTP endpoints. vLLM is developed at
[vllm-project/vllm](https://github.com/vllm-project/vllm) under the Apache 2.0 license.

| Resource | URL |
| --- | --- |
| Homepage | https://vllm.ai |
| Documentation | https://docs.vllm.ai |
| Repository | https://github.com/vllm-project/vllm |
| Releases | https://github.com/vllm-project/vllm/releases |
| Online serving reference | https://docs.vllm.ai/en/latest/online_serving/ |
| CLI reference (`vllm serve`) | https://docs.vllm.ai/en/latest/cli/serve.html |

## Platforms and Installation

| OS | Support | Binary | Install methods | Process model | Service |
| --- | --- | --- | --- | --- | --- |
| Linux | native | `vllm` | `pip install vllm`, `uv pip install vllm`, Docker `vllm/vllm-openai` | foreground | user-managed (systemd/Docker/K8s/foreground) |
| macOS | separate_project | `vllm` | Source build for official CPU support; vLLM-Metal install guide for GPU | foreground | user-managed foreground process |
| Windows | wsl | `vllm` | WSL2 with Linux install steps | foreground | user-managed WSL2 process |

vLLM is primarily a Linux project. Official experimental Apple Silicon CPU
support exists, but it is build-from-source only with no pre-built wheels. Apple
Silicon GPU acceleration remains the separate
[vLLM-Metal](https://github.com/vllm-project/vllm-metal) project, which replaces
PyTorch with MLX and requires MLX-optimized models from the
[mlx-community](https://huggingface.co/mlx-community) HuggingFace organization.
Windows users should run vLLM inside WSL2.

## API Surface

By default vLLM listens on **all interfaces** at port **8000** (`http://0.0.0.0:8000`).
The `--host` and `--port` flags control the HTTP server; note that `VLLM_HOST_IP`
and `VLLM_PORT` are for internal distributed communication only.

The server exposes three API families:

- **OpenAI-compatible API** at `/v1/*` (base URL `http://localhost:8000/v1`).
- **Anthropic-compatible API** at `/v1/messages` (base URL `http://localhost:8000`,
  because Anthropic SDKs append `/v1/messages`).
- **Native/pooling/speech endpoints** at `/health`, `/version`, `/metrics`, `/pooling`,
  `/score`, `/rerank`, `/tokenize`, `/v1/realtime`, etc.

Authentication is optional. When `--api-key` or `VLLM_API_KEY` is provided, every
endpoint (including `/health` and `/version`) requires `Authorization: Bearer <key>`.

### OpenAI-compatible endpoints

| Path | Status | Notes |
| --- | --- | --- |
| `/v1/models` | Supported | Single-model list. |
| `/v1/completions` | Supported | `suffix` is not supported. |
| `/v1/chat/completions` | Supported | Streaming, tools, vision, JSON mode. |
| `/v1/chat/completions/batch` | Supported | Batch API. |
| `/v1/embeddings` | Supported | Requires an embedding model. |
| `/v1/audio/transcriptions` | Supported | Requires an ASR model. |
| `/v1/audio/translations` | Supported | Requires an ASR model. |
| `/v1/responses` | Supported | Non-stateful. |
| `/v1/load_lora_adapter` | Gated | Requires `--enable-lora` and `VLLM_ALLOW_RUNTIME_LORA_UPDATING=True`; local dev only. |
| `/v1/unload_lora_adapter` | Gated | Requires `--enable-lora` and `VLLM_ALLOW_RUNTIME_LORA_UPDATING=True`. |

### Anthropic-compatible endpoints

| Path | Status | Notes |
| --- | --- | --- |
| `/v1/messages` | Supported (v0.11.1+) | Messages, streaming, tools, system prompts, thinking. |
| `/v1/messages/count_tokens` | Supported (v0.17.0+) | Token-count helper added by PR #35588. |

Unsupported Anthropic features include prompt caching, batches, citations, and PDF
content blocks.

### Native and pooling endpoints

| Path | Purpose | Notes |
| --- | --- | --- |
| `/health` | Health check | Empty HTTP 200 body when healthy; 503 when the engine is dead. |
| `/version` | Version | Strong identity marker. |
| `/metrics` | Prometheus metrics | GPU, scheduler, request metrics. |
| `/tokenize` / `/detokenize` | Tokenization | Native token utilities. |
| `/tokenizer_info` | Tokenizer metadata | Gated by `--enable-tokenizer-info-endpoint`. |
| `/pooling` | Generic pooling | For all pooling models. |
| `/classify` | Classification | For classification models. |
| `/score` / `/v1/score` | Scoring | For score models. |
| `/rerank` / `/v1/rerank` / `/v2/rerank` | Reranking | Compatible with Cohere/Jina rerank APIs. |
| `/v1/realtime` | Realtime STT | For realtime ASR models. |
| `/generative_scoring` | Generative scoring | Next-token probabilities for generative models. |

## Detection

A detector should probe in this order:

1. **Binary on PATH**: `vllm` (Linux/macOS/WSL). It is a Python entry-point script.
2. **Process**: `vllm serve <model>` running in foreground, systemd, Docker, or WSL2.
3. **Port**: TCP 8000. This port is shared with other servers; on this host it is
   owned by oMLX (`omlx-server`), so an HTTP identity marker is required.
4. **HTTP identity**: `GET /version` returns `{"version":"..."}`. `GET /v1/models`
   returns an OpenAI-style model list with a single model.
5. **Model cache**: `~/.cache/huggingface/hub` (or `HF_HOME`) contains downloaded
   weights.

Not observed on this host: the `vllm` command is not on PATH.

## Configuration

vLLM is configured primarily through CLI flags. A subset of flags can be persisted in
a YAML file and loaded with `--config serve_args.yaml`. Common important flags:

| Flag | Effect |
| --- | --- |
| `--host` | Bind host (default binds all interfaces). |
| `--port` | HTTP port (default 8000). |
| `--api-key` | Required API key(s). Also settable via `VLLM_API_KEY`. |
| `--model` | HuggingFace model ID or local path to serve. |
| `--served-model-name` | API-facing model name(s). |
| `--chat-template` | Path or inline Jinja2 chat template. |
| `--enable-auto-tool-choice` | Enable automatic tool calling. |
| `--tool-call-parser` | Parser for model-specific tool output. |
| `--max-model-len` | Maximum context length. |
| `--tensor-parallel-size` / `-tp` | Tensor parallelism. |
| `--dtype` | Weight/activation dtype. |
| `--download-dir` | Directory to download/load weights (default HF cache). |

Important environment variables:

| Variable | Effect |
| --- | --- |
| `VLLM_API_KEY` | API key required when set. |
| `VLLM_HOST_IP` | Internal distributed IP, **not** the API bind address. |
| `VLLM_PORT` | Internal distributed port, **not** the API server port. |
| `VLLM_CACHE_ROOT` | vLLM runtime cache root. |
| `VLLM_CONFIG_ROOT` | vLLM configuration root. |
| `VLLM_USE_MODELSCOPE` | Use ModelScope instead of HuggingFace. |
| `HF_HOME` | HuggingFace cache directory. |
| `VLLM_LOGGING_LEVEL` | Default log level. |
| `VLLM_CPU_KVCACHE_SPACE` | CPU backend KV cache size in GiB; `40` means 40 GiB. |

## Models

### Model ID grammar

vLLM uses HuggingFace model IDs, local paths, or ModelScope IDs:

- `Qwen/Qwen2.5-1.5B-Instruct`
- `meta-llama/Llama-3.1-8B-Instruct`
- `/path/to/local/model`
- `./my-model.gguf`

The API-facing name is `--model` unless `--served-model-name` is provided. Multiple
`served-model-name` aliases can be configured.

### Model formats

Runtime formats are **Safetensors**, **PyTorch bin**, and **GGUF**. vLLM downloads
from HuggingFace Hub by default (or ModelScope when `VLLM_USE_MODELSCOPE=true`).

### Model store paths

| Path | Notes |
| --- | --- |
| `~/.cache/huggingface/hub` | Default HuggingFace Hub cache. |
| `~/.cache/vllm` | Default vLLM runtime cache (`VLLM_CACHE_ROOT`). |
| `--download-dir` | Per-invocation override for weights directory. |

## Capabilities

| Capability | Status | Notes |
| --- | --- | --- |
| Hardware backends | CUDA, ROCm, TPU, NPU, CPU, Metal | Metal via vLLM-Metal; CPU backend available. |
| Multi-model serving | No | One model per server process. |
| Parallel requests | Yes | Continuous batching. |
| SSE streaming | Yes | OpenAI/Anthropic endpoints stream SSE. |
| Tool calling | Conditional | Requires `--enable-auto-tool-choice`, parser, and compatible chat template. |
| Embeddings | Yes | `/v1/embeddings` and `/pooling`. |
| Reranking | Yes | `/rerank`, `/v1/rerank`, `/v2/rerank`. |
| Web UI | No | FastAPI `/docs` only; no built-in chat UI. |
| Speech to text | Yes | `/v1/audio/transcriptions`, `/v1/audio/translations`, `/v1/realtime`. |

## Agentic CLI Integration

### OpenCode provider block

```json
{
  "provider": {
    "vllm": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "vLLM (local)",
      "options": {
        "baseURL": "http://localhost:8000/v1",
        "apiKey": "EMPTY"
      },
      "models": {
        "Qwen/Qwen2.5-1.5B-Instruct": {
          "name": "Qwen2.5 1.5B Instruct (local vLLM)"
        }
      }
    }
  }
}
```

### Claude Code via Anthropic compatibility

```bash
export ANTHROPIC_AUTH_TOKEN=EMPTY
export ANTHROPIC_BASE_URL=http://localhost:8000
claude --model Qwen/Qwen2.5-1.5B-Instruct
```

vLLM does not provide runner-native integration commands for agentic CLIs today.

## Traps

- `VLLM_PORT` and `VLLM_HOST_IP` are for **internal distributed communication**, not
  the HTTP API server. Use `--host` and `--port` for the API.
- The default `--host` value binds **all interfaces** (displayed as `0.0.0.0`), not
  localhost.
- One vLLM process serves **one model**. Run multiple processes on different ports
  for multi-model serving.
- `--api-key` gates **all** endpoints, including `/health` and `/version`.
- Models served for chat must include a chat template in `tokenizer_config.json` or
  pass `--chat-template`.
- Tool calling requires explicit flags (`--enable-auto-tool-choice`,
  `--tool-call-parser`, and often `--chat-template`).
- FastAPI `/docs` needs internet unless `--enable-offline-docs` is set.

## Sources

- [vLLM homepage](https://vllm.ai)
- [vLLM documentation](https://docs.vllm.ai)
- [vLLM GitHub repository](https://github.com/vllm-project/vllm)
- [vLLM online serving reference](https://docs.vllm.ai/en/latest/online_serving/)
- [vLLM CLI serve reference](https://docs.vllm.ai/en/latest/cli/serve.html)
- [vLLM environment variables](https://docs.vllm.ai/en/latest/configuration/env_vars.html)
- [vLLM CPU installation](https://docs.vllm.ai/en/latest/getting_started/installation/cpu/)
- [vLLM quickstart](https://docs.vllm.ai/en/latest/getting_started/quickstart.html)
- [vLLM LoRA adapters](https://docs.vllm.ai/en/latest/features/lora/)
- [vLLM tool calling](https://docs.vllm.ai/en/latest/features/tool_calling.html)
- [vLLM health endpoint API docs](https://docs.vllm.ai/en/v0.14.1/api/vllm/entrypoints/serve/instrumentator/health/)
- [vLLM Anthropic /v1/messages PR #22627](https://github.com/vllm-project/vllm/pull/22627)
- [vLLM v0.11.1 release notes](https://github.com/vllm-project/vllm/releases/tag/v0.11.1)
- [vLLM Anthropic count_tokens PR #35588](https://github.com/vllm-project/vllm/pull/35588)
- [vLLM v0.17.0 release notes](https://github.com/vllm-project/vllm/releases/tag/v0.17.0)

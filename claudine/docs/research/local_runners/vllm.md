---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: codex
model: default

summary: vLLM is an open-source high-throughput model serving engine that exposes local and self-hosted Hugging Face-style models through OpenAI-compatible, Anthropic-compatible, and task-specific HTTP APIs.
homepage: https://vllm.ai
docs_url: https://docs.vllm.ai
repo_url: https://github.com/vllm-project/vllm
api_reference_url: https://docs.vllm.ai/en/latest/serving/online_serving/
open_source: full

has_official_schema: formal
schema_url: http://localhost:8000/openapi.json

default_port: 8000
default_bind: 0.0.0.0
auth: optional_api_key
auth_notes: >
  Authentication is disabled by default. Pass --api-key or set VLLM_API_KEY to
  require Authorization: Bearer <key> on guarded API prefixes. Current source
  guards /v1, /v2, and /inference; detector endpoints such as /health,
  /version, /load, and /metrics are not guarded by that middleware.

platforms:
  - os: linux
    support: native
    binary: vllm
    alt_binaries: ["python", "uv"]
    install: ["uv pip install vllm --torch-backend=auto", "pip install vllm --extra-index-url https://download.pytorch.org/whl/cu129", "Docker image vllm/vllm-openai", "Docker image vllm/vllm-openai-rocm", "build from source"]
    process_model: foreground
    service: user-managed foreground process, Docker/Podman container, Kubernetes deployment, or custom systemd unit
    notes: Primary supported platform. Official installation docs list Linux for CUDA/ROCm/XPU wheels and containers; WSL is the documented path for Windows hosts.
  - os: macos
    support: separate_project
    binary: vllm
    alt_binaries: ["vllm-metal"]
    install: ["vLLM-Metal project for Apple Silicon GPU acceleration", "source build paths for CPU/Apple Silicon where documented"]
    process_model: foreground
    service: user-managed foreground process
    notes: Current vLLM docs list Apple Silicon GPU support via the separate vLLM-Metal project. Treat that as separate_project for detection because it is outside the main vLLM repository and uses an MLX-based backend.
  - os: windows
    support: wsl
    binary: vllm
    alt_binaries: ["python"]
    install: ["Install and run inside WSL2 with the Linux vLLM steps", "community-maintained native forks exist but are not official vLLM artifacts"]
    process_model: foreground
    service: user-managed WSL2 process or custom Linux service inside WSL
    notes: Official docs state vLLM does not support Windows natively; WSL with a compatible Linux distribution is the supported route.

api_standards:
  - standard: openai_compatible
    supported: yes
    base_url: http://localhost:8000/v1
    key_paths:
      - /v1/models
      - /v1/completions
      - /v1/chat/completions
      - /v1/chat/completions/batch
      - /v1/responses
      - /v1/responses/{response_id}
      - /v1/responses/{response_id}/cancel
      - /v1/embeddings
      - /v1/audio/transcriptions
      - /v1/audio/translations
      - /v1/realtime
      - /v1/load_lora_adapter
      - /v1/unload_lora_adapter
      - /v1/score
      - /v1/rerank
    auth: optional Authorization Bearer token via --api-key or VLLM_API_KEY
    since_version: unknown
    deviations:
      - "A normal `vllm serve` instance hosts one model at a time; API-facing aliases can be added with --served-model-name."
      - "The completions API does not support the OpenAI `suffix` parameter."
      - "The chat completions API ignores the OpenAI `user` parameter."
      - "The responses API is non-stateful in vLLM; previous_response_id storage is not implemented."
      - "OpenAI clients can pass vLLM-only sampling and structured-output parameters through extra_body or raw JSON."
      - "Only X-Request-Id is documented as an extra request header, and it requires --enable-request-id-headers."
    docs_url: https://docs.vllm.ai/en/latest/serving/online_serving/openai_compatible_server/
  - standard: anthropic_compatible
    supported: yes
    base_url: http://localhost:8000
    key_paths:
      - /v1/messages
      - /v1/messages/count_tokens
    auth: optional Authorization Bearer token via --api-key or VLLM_API_KEY; x-api-key is not accepted by vLLM's built-in auth middleware
    since_version: v0.11.1
    deviations:
      - "Anthropic SDK base URLs normally omit /v1 because the SDK appends /v1/messages."
      - "Use an Anthropic client mode that sends Authorization: Bearer when vLLM auth is enabled; vLLM source checks the Authorization header, not x-api-key."
      - "The /v1/messages endpoint was verified in the v0.11.1 release notes. /v1/messages/count_tokens exists in current docs and source, but no release tag naming it was verified, so its exact since_version is unknown."
      - "Prompt caching, batches, citations, and PDF content blocks are not established as supported by the current vLLM Anthropic surface."
    docs_url: https://docs.vllm.ai/en/latest/serving/online_serving/
  - standard: native
    supported: yes
    base_url: http://localhost:8000
    key_paths:
      - /health
      - /version
      - /load
      - /metrics
      - /openapi.json
      - /docs
      - /redoc
      - /tokenize
      - /detokenize
      - /tokenizer_info
      - /pooling
      - /classify
      - /score
      - /rerank
      - /v2/embed
      - /v2/rerank
      - /generative_scoring
      - /start_profile
      - /stop_profile
      - /ping
      - /invocations
      - /inference/v1/generate
    auth: mixed; /v1, /v2, and /inference are guarded when auth is enabled, while /health, /version, /load, /metrics, /tokenize, and /detokenize are not guarded by the built-in auth middleware
    since_version: unknown
    deviations:
      - "Task-specific endpoints only exist or succeed for compatible model tasks, such as embedding, classification, score/rerank, ASR, or generate."
      - "Some administrative endpoints are gated by environment variables or server flags and are intended only for local development."
    docs_url: https://docs.vllm.ai/en/latest/serving/online_serving/

metadata_endpoints:
  - purpose: health
    method: get
    path: /health
    gated_by: ""
    auth_gated: false
    response_hint: "HTTP 200 with empty body when healthy"
    notes: Source registers a dedicated health endpoint. It is not under a guarded prefix, so it remains a good detector probe even when --api-key is set.
  - purpose: version
    method: get
    path: /version
    gated_by: ""
    auth_gated: false
    response_hint: '{"version":"0.x.x"}'
    notes: Strong identity marker for vLLM; not under the guarded /v1, /v2, or /inference prefixes.
  - purpose: other
    method: get
    path: /load
    gated_by: "--enable-server-load-tracking for meaningful load data"
    auth_gated: false
    response_hint: "server load metrics JSON"
    notes: Listed by current docs as a basic instrumentator API. Without load tracking, returned values may be absent or uninformative.
  - purpose: model_list
    method: get
    path: /v1/models
    gated_by: ""
    auth_gated: true
    response_hint: '{"object":"list","data":[{"id":"...","object":"model"}]}'
    notes: Lists the served model name or aliases from --served-model-name. It is guarded when API-key auth is enabled.
  - purpose: metrics
    method: get
    path: /metrics
    gated_by: ""
    auth_gated: false
    response_hint: "# HELP vllm"
    notes: Prometheus-compatible endpoint from vLLM's instrumentator. This host returned 404 because port 8000 is oMLX, not vLLM.
  - purpose: other
    method: get
    path: /openapi.json
    gated_by: "--disable-fastapi-docs disables it"
    auth_gated: false
    response_hint: '{"openapi":"3'
    notes: FastAPI's generated OpenAPI schema; formal machine-readable schema for the running API when docs are enabled.
  - purpose: admin_ui
    method: get
    path: /docs
    gated_by: "--disable-fastapi-docs disables it; --enable-offline-docs changes asset handling"
    auth_gated: false
    response_hint: "Swagger UI"
    notes: FastAPI Swagger UI. Current docs say it needs internet by default unless --enable-offline-docs is set.
  - purpose: model_info
    method: get
    path: /tokenizer_info
    gated_by: "--enable-tokenizer-info-endpoint"
    auth_gated: false
    response_hint: '{"chat_template":'
    notes: Registered only when the flag is enabled. It is not under a guarded auth prefix.
  - purpose: load_model
    method: post
    path: /v1/load_lora_adapter
    gated_by: "VLLM_ALLOW_RUNTIME_LORA_UPDATING=1 and LoRA support"
    auth_gated: true
    response_hint: "HTTP 200 on success"
    notes: Runtime LoRA adapter loading, not base-model loading. vLLM warns this is for local development only.
  - purpose: unload_model
    method: post
    path: /v1/unload_lora_adapter
    gated_by: "VLLM_ALLOW_RUNTIME_LORA_UPDATING=1 and LoRA support"
    auth_gated: true
    response_hint: "HTTP 200 on success"
    notes: Runtime LoRA adapter unloading, not base-model unloading.
  - purpose: other
    method: post
    path: /start_profile
    gated_by: "--profiler-config with profiler set"
    auth_gated: false
    response_hint: "HTTP 200"
    notes: Local-development profiler endpoint; source only attaches this router when profiler mode is configured.
  - purpose: other
    method: post
    path: /stop_profile
    gated_by: "--profiler-config with profiler set"
    auth_gated: false
    response_hint: "HTTP 200"
    notes: Local-development profiler endpoint; source only attaches this router when profiler mode is configured.
  - purpose: other
    method: post
    path: /tokenize
    gated_by: ""
    auth_gated: false
    response_hint: '{"tokens":'
    notes: Token utility endpoint registered by source; it is not under a guarded auth prefix.
  - purpose: other
    method: post
    path: /detokenize
    gated_by: ""
    auth_gated: false
    response_hint: '{"prompt":'
    notes: Token utility endpoint registered by source; it is not under a guarded auth prefix.

detection:
  - os: linux
    method: binary
    target: vllm
    expect: "vllm serve"
    confidence: documented
    notes: Installed by Python packages as the `vllm` entry-point script. Not observed on this macOS host.
  - os: macos
    method: binary
    target: vllm
    expect: "vllm serve"
    confidence: observed
    notes: "Negative observation on this host: `command -v vllm` failed and `vllm --version` returned command not found."
  - os: windows
    method: binary
    target: vllm
    expect: "vllm serve"
    confidence: documented
    notes: Probe inside WSL. Native Windows is not officially supported, so a Windows host detector should treat a native missing binary as expected.
  - os: all
    method: process
    target: "vllm serve"
    expect: "vLLM server on http://"
    confidence: source_code
    notes: Source logs the listen address while starting the API server. Process args may be hidden by Docker, uv, Python, systemd, or WSL wrappers.
  - os: all
    method: port
    target: "8000"
    expect: ""
    confidence: observed
    notes: Port 8000 alone is ambiguous. On this host it is live but belongs to oMLX (`omlx-server`), not vLLM.
  - os: all
    method: http
    target: GET /version
    expect: '{"version":"0.'
    confidence: source_code
    notes: "Best unauthenticated identity marker. On this host, localhost:8000/version returned 404, confirming the live port is not vLLM."
  - os: all
    method: http
    target: GET /health
    expect: "HTTP 200 with empty body"
    confidence: source_code
    notes: Good liveness probe but weaker identity marker. On this host, /health returned an oMLX JSON body with `default_model` and `engine_pool`, so body shape disambiguates it from vLLM.
  - os: all
    method: http
    target: GET /v1/models
    expect: '{"object":"list","data":[{"id":"...","object":"model"}]}'
    confidence: source_code
    notes: Good OpenAI-compatible model probe when auth is disabled. On this host, /v1/models returned 401 from oMLX, not vLLM.
  - os: all
    method: http
    target: GET /metrics
    expect: "# HELP vllm"
    confidence: documented
    notes: Useful secondary marker. On this host, /metrics returned 404 because the live server on port 8000 is oMLX.
  - os: macos
    method: config_file
    target: ~/.config/vllm
    expect: "vLLM config root"
    confidence: observed
    notes: "Negative observation on this host: ~/.config/vllm was absent."
  - os: macos
    method: config_file
    target: ~/.cache/vllm
    expect: "vLLM cache root"
    confidence: observed
    notes: "Negative observation on this host: ~/.cache/vllm was absent."

identity_probes:
  - rank: 1
    request: GET /version
    match_in: json_field
    field: version
    marker: '{"version":"0.x.x"}'
    uniqueness: unique
    zero_model_ok: true
    auth_gated: false
    confidence: source_code
    notes: 'The /version path with a bare {"version": ...} object is vLLM-only — oMLX, SGLang, LocalAI, and Lemonade (the other FastAPI-family servers found on 8000) do not mount it. On this host localhost:8000/version returned 404, confirming the live port is oMLX, not vLLM.'
  - rank: 2
    request: GET /openapi.json
    match_in: json_field
    field: paths
    marker: vLLM-only routes (/tokenize, /pooling, /rerank, /v1/load_lora_adapter, /score)
    uniqueness: strong
    zero_model_ok: true
    auth_gated: false
    confidence: source_code
    notes: Path-set fingerprint; absent when the server is started with --disable-fastapi-docs, in which case fall through to rank 3.
  - rank: 3
    request: GET /metrics
    match_in: body
    field: ""
    marker: "vllm:"
    uniqueness: strong
    zero_model_ok: true
    auth_gated: false
    confidence: documented
    notes: Prometheus metric families are vllm:-prefixed (vllm:num_requests_running, vllm:gpu_cache_usage_perc). On by default in v1 but may be stripped behind a proxy.
  - rank: 4
    request: GET /health
    match_in: status
    field: ""
    marker: "200 with empty body"
    uniqueness: weak
    zero_model_ok: true
    auth_gated: false
    confidence: source_code
    notes: Liveness only — an empty 200 identifies nothing (oMLX's /health also returns 200, with a JSON body). A 200 with a JSON body on /health positively EXCLUDES vLLM.
  - rank: 5
    request: ANY /
    match_in: header
    field: server
    marker: uvicorn
    uniqueness: weak
    zero_model_ok: true
    auth_gated: false
    confidence: source_code
    notes: vLLM sends uvicorn defaults with no custom identifying header (X-Request-Id only with --enable-request-id-headers) — corroborating evidence at best.

version_probe:
  - os: linux
    method: cli
    command: "python3 -c \"import vllm; print(vllm.__version__)\""
    pattern: "^(\\S+)$"
    confidence: documented
    notes: The reliable installed-version probe — works regardless of how the entry-point script is wrapped (uv, conda, containers). For the running server use `GET /version` (identity_probes rank 1).
  - os: linux
    method: package
    command: "pip show vllm"
    pattern: "^Version: (\\S+)"
    confidence: documented
    notes: Fallback when the python interpreter on PATH is not the one vLLM is installed into; use the environment's own pip.
  - os: windows
    method: cli
    command: "python3 -c \"import vllm; print(vllm.__version__)\""
    pattern: "^(\\S+)$"
    confidence: documented
    notes: Probe inside WSL — native Windows is not officially supported; a missing package is the expected negative.
  - os: macos
    method: cli
    command: "python3 -c \"import vllm; print(vllm.__version__)\""
    pattern: "^(\\S+)$"
    confidence: observed
    notes: "Negative observation on this host: vllm is not installed (`command -v vllm` failed). Native macOS requires the separate vLLM-Metal project."

config_mechanism: mixed

config_files:
  - os: linux
    path: ~/.config/vllm
    format: other
    role: Default vLLM configuration root
    notes: Defaults under XDG_CONFIG_HOME when set; VLLM_CONFIG_ROOT can redirect it. Used for runtime and installation-related configuration artifacts.
  - os: macos
    path: ~/.config/vllm
    format: other
    role: Default vLLM configuration root
    notes: Documented default through the shared envs.py logic; absent on this host.
  - os: windows
    path: ~/.config/vllm
    format: other
    role: WSL-side vLLM configuration root
    notes: Applies inside the Linux distribution used by WSL, not native Windows.
  - os: linux
    path: serve_args.yaml
    format: yaml
    role: Optional `vllm serve --config` argument file
    notes: The file can live anywhere; long-form CLI argument names are YAML keys. Command-line flags take precedence over config-file values.
  - os: macos
    path: serve_args.yaml
    format: yaml
    role: Optional `vllm serve --config` argument file
    notes: Applies to source builds or vLLM-Metal-style usage when the `vllm serve` command is available.
  - os: windows
    path: serve_args.yaml
    format: yaml
    role: Optional WSL-side `vllm serve --config` argument file
    notes: Applies inside WSL.

env_vars:
  - name: VLLM_API_KEY
    effect: "API key required by guarded API prefixes when set; CLI --api-key takes precedence."
  - name: VLLM_HOST_IP
    effect: "Internal distributed-communication IP only; it is not the API server bind address."
  - name: VLLM_PORT
    effect: "Internal distributed-communication port only; it is not the API server port and may be incremented for multiple internal ports."
  - name: VLLM_CACHE_ROOT
    effect: "Root directory for vLLM cache files; defaults to ~/.cache/vllm unless XDG_CACHE_HOME redirects the cache root."
  - name: VLLM_CONFIG_ROOT
    effect: "Root directory for vLLM configuration files; defaults to ~/.config/vllm unless XDG_CONFIG_HOME redirects the config root."
  - name: VLLM_USE_MODELSCOPE
    effect: "When true, loads models from ModelScope instead of Hugging Face Hub."
  - name: VLLM_MODEL_REDIRECT_PATH
    effect: "Optional JSON or whitespace table that redirects model IDs to local paths."
  - name: HF_HOME
    effect: "Hugging Face cache root used by huggingface_hub; affects where model snapshots and tokens are stored."
  - name: HF_HUB_CACHE
    effect: "Overrides the Hugging Face Hub snapshot cache directory."
  - name: VLLM_LOGGING_LEVEL
    effect: "Sets the default vLLM logging level."
  - name: VLLM_CPU_KVCACHE_SPACE
    effect: "CPU backend key-value cache size in GiB; when unset, the CPU backend uses its default."
  - name: VLLM_WORKER_MULTIPROC_METHOD
    effect: "Controls worker multiprocessing mode; accepted values are fork and spawn."
  - name: VLLM_ALLOW_LONG_MAX_MODEL_LEN
    effect: "Allows --max-model-len to exceed the maximum derived from the model config."
  - name: VLLM_ALLOW_RUNTIME_LORA_UPDATING
    effect: "Enables runtime LoRA load/unload API routes when set to 1 or true."
  - name: VLLM_SERVER_DEV_MODE
    effect: "Enables dangerous development/debug endpoints such as cache reset, pause/resume, RPC, sleep, and server_info."
  - name: VLLM_HTTP_TIMEOUT_KEEP_ALIVE
    effect: "HTTP keep-alive timeout in seconds for the API server."
  - name: VLLM_ENABLE_CUDA_COMPATIBILITY
    effect: "Enables CUDA compatibility-library handling in official Docker images for selected datacenter GPUs with older drivers."

model_id_grammar: |
  Accepted model identifiers are the `--model` value accepted by Hugging Face
  Transformers/vLLM: Hugging Face repository IDs such as
  `Qwen/Qwen3-0.6B` or `meta-llama/Llama-3.1-8B-Instruct`, local directories
  containing a supported model config and weights, local GGUF file paths such as
  `./model.gguf`, and ModelScope IDs when `VLLM_USE_MODELSCOPE=true`. The API
  model ID returned by `/v1/models` and accepted in inference requests is the
  `--model` value unless one or more `--served-model-name` aliases are supplied;
  when multiple aliases are supplied, vLLM accepts all of them and reports the
  first alias as the response model name and Prometheus model_name tag.

model_formats:
  - safetensors
  - pytorch
  - gguf
  - bitsandbytes
  - tensorizer
  - runai_streamer
  - sharded_state
  - mistral
  - model_from_plugin

model_acquisition:
  - method: huggingface
    example: "vllm serve Qwen/Qwen3-0.6B"
    notes: Downloads model files through Hugging Face Hub into the Hugging Face cache unless --download-dir, HF_HOME, or HF_HUB_CACHE redirects storage.
  - method: registry
    example: "VLLM_USE_MODELSCOPE=true vllm serve Qwen/Qwen3-0.6B"
    notes: Uses ModelScope instead of Hugging Face when VLLM_USE_MODELSCOPE is true.
  - method: manual
    example: "vllm serve /models/llama-3.1-8b --served-model-name llama-local"
    notes: Local model directories and local GGUF files are accepted when the selected loader/model implementation supports them.
  - method: manual
    example: "vllm serve ./qwen3.gguf --tokenizer Qwen/Qwen3-0.6B"
    notes: GGUF serving may need an explicit tokenizer or chat template depending on the model artifact.

model_store_paths:
  - os: linux
    path: ~/.cache/huggingface/hub
    notes: Default Hugging Face Hub cache for model snapshots; relocatable via HF_HOME or HF_HUB_CACHE.
  - os: macos
    path: ~/.cache/huggingface/hub
    notes: Documented Hugging Face default on Unix-like hosts; not specifically observed for vLLM on this host.
  - os: windows
    path: ~/.cache/huggingface/hub
    notes: WSL-side path when running vLLM in WSL; do not infer a native Windows path because native Windows is unsupported.
  - os: linux
    path: ~/.cache/vllm
    notes: vLLM runtime cache root; relocatable via VLLM_CACHE_ROOT or XDG_CACHE_HOME.
  - os: macos
    path: ~/.cache/vllm
    notes: vLLM runtime cache root by shared env logic; absent on this host.
  - os: windows
    path: ~/.cache/vllm
    notes: WSL-side vLLM runtime cache root.
  - os: linux
    path: "--download-dir <path>"
    notes: Per-server override for model weight download/load directory.
  - os: macos
    path: "--download-dir <path>"
    notes: Per-server override when a compatible vLLM build is available.
  - os: windows
    path: "--download-dir <path>"
    notes: Per-server override inside WSL.

hardware_acceleration:
  - cuda
  - rocm
  - xpu
  - tpu
  - npu
  - cpu
  - metal_via_vllm_metal
  - gaudi_plugin
  - spyre_plugin
  - ascend_plugin
  - rebellions_npu_plugin
  - metax_gpu_plugin

concurrency:
  multi_model: false
  parallel_requests: true
  notes: A normal vLLM server process serves one base model, with multiple API aliases and optional LoRA adapters. Parallel requests are supported through continuous batching and configurable distributed parallelism.

streaming_sse: true
tool_calling: conditional
tool_calling_notes: >
  Tool calling is available for compatible chat models and templates. Automatic
  tool choice requires --enable-auto-tool-choice and a matching --tool-call-parser;
  named and required tool choices use structured-output machinery and still
  depend on model/template compatibility.
embeddings: true
rerank: true
web_ui_url: ""

integration_hooks: []

traps:
  - "VLLM_PORT and VLLM_HOST_IP configure internal distributed communication, not the HTTP API server. Use --host and --port for the API bind."
  - "Default host binding uses an empty host internally and is displayed as 0.0.0.0, so a default vLLM server can bind all interfaces rather than localhost only."
  - "Port 8000 is not identifying. oMLX and other local runners also use or commonly occupy it; use /version and response shape to disambiguate."
  - "Current auth middleware only guards /v1, /v2, and /inference prefixes. /health, /version, /load, /metrics, /tokenize, and /detokenize are not protected by built-in --api-key auth."
  - "vLLM auth checks Authorization: Bearer, not x-api-key; Anthropic-compatible clients must be configured to send Bearer auth when --api-key is enabled."
  - "One normal server process serves one base model. Use --served-model-name for aliases, LoRA for adapters, or multiple processes/ports for multiple base models."
  - "Models without a chat template in tokenizer_config.json need --chat-template or chat and Anthropic Messages requests can fail."
  - "By default, vLLM applies a Hugging Face generation_config.json when present; pass --generation-config vllm to use vLLM defaults instead."
  - "Runtime LoRA load/unload requires VLLM_ALLOW_RUNTIME_LORA_UPDATING and is documented as local-development-only."
  - "FastAPI /docs needs internet for assets by default; use --enable-offline-docs for air-gapped environments."
  - "VLLM_SERVER_DEV_MODE exposes dangerous debugging endpoints and should not be used in production."

opencode_example: '{"provider":{"vllm":{"npm":"@ai-sdk/openai-compatible","name":"vLLM (local)","options":{"baseURL":"http://localhost:8000/v1"},"models":{"Qwen/Qwen3-0.6B":{"name":"Qwen3 0.6B (local vLLM)"}}}}}'

changes:
  - "Updated local observation: vllm is not installed on this macOS host, ~/.cache/vllm and ~/.config/vllm are absent, and localhost:8000 is oMLX rather than vLLM."
  - "Corrected auth behavior from current source: --api-key guards /v1, /v2, and /inference, not /health, /version, /load, /metrics, /tokenize, or /detokenize."
  - "Added current API surface details for /load, /openapi.json, /docs, /redoc, /v1/responses/{response_id}, /v1/responses/{response_id}/cancel, /v2/embed, and gated profiler/development endpoints."
  - "Updated platform notes to reflect current docs: Linux is primary, Windows is WSL, and Apple Silicon GPU support is via the separate vLLM-Metal project."
  - "Changed the OpenCode example to the required provider shape without a runner-specific JS client and with models as a map keyed by the served model ID."
requires_claudine_update: true
reason: Claudine/sniff detection should avoid treating port 8000 or /health alone as vLLM, should prefer GET /version with a vLLM version JSON marker, should account for unguarded /version and /metrics even when API-key auth is enabled, and should record that this host's port 8000 observation identifies oMLX rather than vLLM.
---

# vLLM Local Model Runner

## Introduction to vLLM

[vLLM](https://vllm.ai) is an Apache-2.0 open-source inference and serving engine for local or self-hosted LLMs. Its focus is high-throughput serving: continuous batching, efficient KV-cache management, tensor/data/pipeline/expert parallelism, quantization support, and production-oriented HTTP endpoints.

| Resource | URL |
| --- | --- |
| Homepage | https://vllm.ai |
| Documentation | https://docs.vllm.ai |
| API reference | https://docs.vllm.ai/en/latest/serving/online_serving/ |
| Repository | https://github.com/vllm-project/vllm |
| Releases | https://github.com/vllm-project/vllm/releases |
| Server arguments | https://docs.vllm.ai/en/latest/configuration/serve_args/ |

vLLM is a model server, not an agentic CLI. A user starts a server with `vllm serve <model>` and clients connect over HTTP.

## Platforms and Installation

| OS | Support | Binary | Installation | Process Model | Service |
| --- | --- | --- | --- | --- | --- |
| Linux | Native | `vllm` | `uv pip install vllm --torch-backend=auto`, `pip install vllm`, official Docker/Podman images | Foreground by default | User-managed, Docker/Kubernetes, or custom systemd |
| macOS | Separate project for GPU | `vllm` / `vllm-metal` | vLLM-Metal for Apple Silicon GPU; source/build paths for CPU use | Foreground | User-managed foreground process |
| Windows | WSL | `vllm` inside WSL | Install in a WSL2 Linux distribution | Foreground | User-managed WSL process |

The current installation docs list Linux as the native target for CUDA, ROCm, and Intel XPU builds. They explicitly say native Windows is not supported and recommend WSL. Apple Silicon GPU support is through [vLLM-Metal](https://github.com/vllm-project/vllm-metal), a separate MLX-based project; detector logic should not assume a normal Linux `vllm` install is present on macOS.

vLLM does not install a launchd service, tray app, or daemon by default. Production deployments usually wrap the foreground process in systemd, Docker, Kubernetes, or another supervisor.

## API Surface

Default server address:

| Setting | Value |
| --- | --- |
| Bind | `0.0.0.0` by default display; set with `--host` |
| Port | `8000`; set with `--port` |
| OpenAPI schema | `GET /openapi.json` unless FastAPI docs are disabled |
| Swagger UI | `GET /docs` unless FastAPI docs are disabled |

### OpenAI-Compatible API

Client base URL: `http://localhost:8000/v1`

| Path | Method | Notes |
| --- | --- | --- |
| `/v1/models` | GET | Lists served model ID or aliases. |
| `/v1/completions` | POST | Text generation; `suffix` is not supported. |
| `/v1/chat/completions` | POST | Chat generation, streaming, structured outputs, tool support when configured. |
| `/v1/chat/completions/batch` | POST | Batch chat API. |
| `/v1/responses` | POST | OpenAI Responses-style endpoint; vLLM does not persist response state. |
| `/v1/responses/{response_id}` | GET | Responses API path, but statefulness is limited. |
| `/v1/responses/{response_id}/cancel` | POST | Responses cancel path. |
| `/v1/embeddings` | POST | Requires an embedding-capable model. |
| `/v1/audio/transcriptions` | POST | Requires an ASR model. |
| `/v1/audio/translations` | POST | Requires an ASR model. |
| `/v1/realtime` | WebSocket | Realtime speech-to-text for realtime-capable ASR models. |
| `/v1/load_lora_adapter` | POST | Gated runtime LoRA loading. |
| `/v1/unload_lora_adapter` | POST | Gated runtime LoRA unloading. |

When `--api-key` or `VLLM_API_KEY` is configured, vLLM expects `Authorization: Bearer <key>` for `/v1` routes.

### Anthropic-Compatible API

Client base URL: `http://localhost:8000`

| Path | Method | Notes |
| --- | --- | --- |
| `/v1/messages` | POST | Anthropic Messages-compatible route; v0.11.1 release notes explicitly name this feature. |
| `/v1/messages/count_tokens` | POST | Present in current docs and source; exact first release was not verified from release notes. |

Anthropic SDKs usually append `/v1/messages`, so the base URL should omit `/v1`. vLLM's auth middleware checks `Authorization: Bearer`, not `x-api-key`; this matters for Claude Code and Anthropic SDK configuration when vLLM auth is enabled.

### Native and Task-Specific Endpoints

| Path | Method | Purpose | Gating |
| --- | --- | --- | --- |
| `/health` | GET | Health check | Always registered |
| `/version` | GET | Version and identity | Always registered |
| `/load` | GET | Server load metrics | More useful with load tracking enabled |
| `/metrics` | GET | Prometheus metrics | Always registered by instrumentator |
| `/tokenize` | POST | Tokenize text | Always registered |
| `/detokenize` | POST | Detokenize tokens | Always registered |
| `/tokenizer_info` | GET | Tokenizer/chat-template metadata | `--enable-tokenizer-info-endpoint` |
| `/pooling` | POST | Generic pooling | Pooling model task |
| `/classify` | POST | Classification | Classification model task |
| `/score`, `/v1/score` | POST | Scoring | Score-capable model task |
| `/rerank`, `/v1/rerank`, `/v2/rerank` | POST | Jina/Cohere-style reranking | Rerank/score model task |
| `/v2/embed` | POST | Cohere Embed-compatible route | Embedding model task |
| `/generative_scoring` | POST | Generative scoring | Generate task |
| `/start_profile`, `/stop_profile` | POST | PyTorch profiler controls | Profiler config |
| `/ping`, `/invocations` | GET/POST | SageMaker-compatible surface | Registered by SageMaker standards bootstrap |
| `/inference/v1/generate` | POST | Scale-out generate path | Generate/scale-out surface |

Development mode (`VLLM_SERVER_DEV_MODE=1`) adds dangerous local-debug endpoints such as cache resets, pause/resume, arbitrary collective RPC, sleep/wake, and `/server_info`. They should not be used as normal detection probes.

## Detection

Recommended ordered probes:

1. Check for `vllm` on PATH. On Windows, probe inside WSL; native Windows is not official.
2. Check processes for `vllm serve`, Python entry points, or container commands that include `vllm/vllm-openai`.
3. Check TCP port `8000`, but treat it as ambiguous.
4. Probe `GET /version`; vLLM returns a JSON version object. This is the best unauthenticated identity marker.
5. Probe `GET /v1/models` if auth is not enabled or a token is available.
6. Probe `GET /metrics` for Prometheus output containing `vllm` metric names.
7. Look for `~/.cache/vllm`, `~/.config/vllm`, and Hugging Face cache paths as supporting installation evidence.

Observed on this host on 2026-07-03:

| Probe | Result |
| --- | --- |
| `command -v vllm` | Not found |
| `vllm --version` | Command not found |
| `~/.cache/vllm` | Absent |
| `~/.config/vllm` | Absent |
| `GET http://localhost:8000/` | 404 JSON from a FastAPI/Uvicorn server |
| `GET http://localhost:8000/health` | 200 JSON with `default_model` and `engine_pool`, identifying oMLX rather than vLLM |
| `GET http://localhost:8000/version` | 404 |
| `GET http://localhost:8000/v1/models` | 401 `API key required` from the non-vLLM server |
| `GET http://localhost:8000/metrics` | 404 |

Port 8000 is therefore a negative vLLM observation on this host: it is occupied by `omlx-server`, so a detector must not infer vLLM from the port alone.

### Port identity

Port 8000 collides with oMLX (and both are FastAPI/uvicorn servers, so
headers cannot separate them), so the ranked `identity_probes` frontmatter
block is the canonical strategy for answering "which runner is listening on
this port?":

1. `GET /version` — the `/version` path returning a bare
   `{"version":"0.x.x"}` object is vLLM-only; oMLX, SGLang, LocalAI, and
   Lemonade do not mount it. Works with zero models loaded (it is the first
   endpoint live once the app is up, though no endpoint answers during
   engine init).
2. `GET /openapi.json` — the path set is a fingerprint (`/tokenize`,
   `/pooling`, `/rerank`, `/v1/load_lora_adapter`, `/score`); absent when
   started with `--disable-fastapi-docs`.
3. `GET /metrics` — Prometheus families are `vllm:`-prefixed; on by default
   but may be stripped behind a proxy.
4. `GET /health` — an empty 200 body is liveness only and identifies
   nothing; a 200 with a JSON body on `/health` positively *excludes* vLLM
   (that is the oMLX shape).

## Configuration

vLLM is primarily configured with CLI flags. A YAML file can be passed with `vllm serve --config config.yaml`; long-form CLI argument names become YAML keys, and command-line arguments take precedence over config-file values.

Example:

```yaml
model: Qwen/Qwen3-0.6B
host: "127.0.0.1"
port: 8000
served-model-name:
  - qwen3-local
uvicorn-log-level: "info"
```

Important flags:

| Flag | Effect |
| --- | --- |
| `--host` | HTTP bind address. |
| `--port` | HTTP port. |
| `--api-key` | One or more accepted bearer tokens. |
| `--model` | Hugging Face ID, ModelScope ID, local directory, or supported local file. |
| `--served-model-name` | API-facing model aliases. |
| `--chat-template` | Jinja chat template path or string. |
| `--enable-auto-tool-choice` | Enables automatic tool choice for compatible models. |
| `--tool-call-parser` | Model-specific tool-call parser. |
| `--generation-config vllm` | Ignores Hugging Face generation defaults and uses vLLM defaults. |
| `--download-dir` | Overrides weight download/load directory. |
| `--enable-tokenizer-info-endpoint` | Registers `GET /tokenizer_info`. |
| `--enable-offline-docs` | Uses bundled Swagger UI assets for offline `/docs`. |

Important environment variables are captured in frontmatter. The two most common traps are `VLLM_HOST_IP` and `VLLM_PORT`: they are for distributed internals, not the HTTP bind.

## Models

### Model ID Grammar

vLLM model IDs are the `--model` input plus optional API aliases:

- Hugging Face repository ID: `Qwen/Qwen3-0.6B`
- Hugging Face repository ID with size/family tags: `meta-llama/Llama-3.1-8B-Instruct`
- Local model directory: `/models/llama-3.1-8b`
- Local GGUF file: `./qwen3.gguf`
- ModelScope ID when `VLLM_USE_MODELSCOPE=true`
- API alias from `--served-model-name`, such as `qwen3-local`

The ID clients should send is exactly what `/v1/models` returns.

### Formats and Acquisition

Runtime loaders include Safetensors, PyTorch bin, GGUF, bitsandbytes, tensorizer, sharded state, Run:ai streamer, Mistral-style consolidated Safetensors, and plugin-provided formats. vLLM normally acquires models from Hugging Face Hub, optionally from ModelScope, or from a manually supplied local path.

Model storage is split between the Hugging Face cache (`~/.cache/huggingface/hub` by default), vLLM's cache root (`~/.cache/vllm` by default), and any per-run `--download-dir`.

## Capabilities

| Capability | Status | Notes |
| --- | --- | --- |
| Hardware acceleration | CUDA, ROCm, XPU, CPU, TPU/NPU/plugin backends, Apple Silicon via vLLM-Metal | Apple Silicon GPU is separate project support. |
| Multi-model serving | No for normal base models | Use aliases for one model, LoRA for adapters, or multiple server processes. |
| Parallel requests | Yes | Continuous batching and distributed parallelism are core features. |
| SSE streaming | Yes | OpenAI and Anthropic streaming use server-sent events. |
| Tool/function calling | Conditional | Requires compatible model/template; automatic tool choice needs explicit flags. |
| Embeddings | Yes | `/v1/embeddings`, `/v2/embed`, and pooling endpoints when model task supports it. |
| Reranking | Yes | `/rerank`, `/v1/rerank`, and `/v2/rerank` for score/rerank models. |
| Web UI | No chat UI | FastAPI `/docs` and `/redoc` are API docs, not a runner chat UI. |
| Speech to text | Yes | Transcription, translation, and realtime endpoints require ASR models. |

## Agentic CLI Integration

### OpenCode Provider Block

```json
{
  "provider": {
    "vllm": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "vLLM (local)",
      "options": {
        "baseURL": "http://localhost:8000/v1"
      },
      "models": {
        "Qwen/Qwen3-0.6B": {
          "name": "Qwen3 0.6B (local vLLM)"
        }
      }
    }
  }
}
```

Use the model ID returned by `curl http://localhost:8000/v1/models`. If the server was launched with `--served-model-name qwen3-local`, use `qwen3-local` as the model key instead.

### Claude Code Through Anthropic Compatibility

When vLLM is running without `--api-key`:

```bash
export ANTHROPIC_BASE_URL=http://localhost:8000
export ANTHROPIC_AUTH_TOKEN=EMPTY
claude --model Qwen/Qwen3-0.6B
```

When vLLM auth is enabled, use an Anthropic client configuration that sends `Authorization: Bearer <token>`. vLLM's built-in middleware does not accept `x-api-key`.

vLLM does not currently provide runner-native commands like `vllm launch codex` or `vllm launch claude` that configure coding agents automatically. The documented `vllm launch render` command is a vLLM render-server launcher, not an agentic-CLI integration hook.

## Changelog

- 2026-07-03: Refreshed against current vLLM docs/source and local host evidence. Corrected API-key auth gating to `/v1`, `/v2`, and `/inference`; added `/load`, OpenAPI/docs, Responses API subpaths, Cohere-compatible `/v2/*` task endpoints, and gated profiler/development endpoints; recorded that this host has oMLX rather than vLLM on port 8000; updated OpenCode and Claude Code integration notes.

## Sources

- [vLLM homepage](https://vllm.ai)
- [vLLM documentation](https://docs.vllm.ai)
- [vLLM GitHub repository](https://github.com/vllm-project/vllm)
- [vLLM installation docs](https://docs.vllm.ai/en/latest/getting_started/installation/)
- [vLLM GPU installation docs](https://docs.vllm.ai/en/latest/getting_started/installation/gpu/)
- [vLLM quickstart](https://docs.vllm.ai/en/latest/getting_started/quickstart/)
- [vLLM online serving docs](https://docs.vllm.ai/en/latest/serving/online_serving/)
- [vLLM OpenAI-compatible server docs](https://docs.vllm.ai/en/latest/serving/online_serving/openai_compatible_server/)
- [vLLM server arguments docs](https://docs.vllm.ai/en/latest/configuration/serve_args/)
- [vLLM `vllm serve` CLI reference](https://docs.vllm.ai/en/latest/cli/serve/)
- [vLLM environment variables docs](https://docs.vllm.ai/en/latest/configuration/env_vars/)
- [vLLM LoRA security note](https://docs.vllm.ai/en/latest/usage/security/)
- [vLLM v0.11.1 release notes](https://github.com/vllm-project/vllm/releases/tag/v0.11.1)
- [vLLM OpenAI API server source](https://github.com/vllm-project/vllm/blob/main/vllm/entrypoints/openai/api_server.py)
- [vLLM AuthenticationMiddleware source](https://github.com/vllm-project/vllm/blob/main/vllm/entrypoints/serve/utils/server_utils.py)
- [vLLM Anthropic API router source](https://github.com/vllm-project/vllm/blob/main/vllm/entrypoints/anthropic/api_router.py)
- [vLLM tokenization router source](https://github.com/vllm-project/vllm/blob/main/vllm/entrypoints/serve/tokenize/api_router.py)
- [vLLM LoRA router source](https://github.com/vllm-project/vllm/blob/main/vllm/entrypoints/serve/lora/api_router.py)
- [OpenCode providers documentation](https://opencode.ai/docs/providers/)

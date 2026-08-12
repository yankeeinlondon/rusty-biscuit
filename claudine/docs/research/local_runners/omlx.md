---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: codex
model: default

summary: oMLX is an open-source, macOS-native Apple Silicon model server with OpenAI-compatible, Anthropic-compatible, and admin APIs over HTTP.
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
  `omlx serve` accepts an optional `--api-key`; without one, API auth is not
  enforced. The macOS app/onboarding path can persist an API key in
  `~/.omlx/settings.json`, which makes `/v1/*`, `/api/status`, and admin APIs
  reject anonymous requests. `/health`, `/admin`, and `/openapi.json` remained
  ungated on the observed v0.4.4 host. The `skip_api_key_verification` setting
  bypasses API/admin checks when enabled.

platforms:
  - os: macos
    support: native
    binary: omlx
    alt_binaries: ["oMLX.app/Contents/MacOS/oMLX", "oMLX.app/Contents/MacOS/omlx-cli", "omlx-server"]
    install: ["Signed/notarized DMG from GitHub releases", "Homebrew tap/formula from jundot/omlx", "Source install with Python on Apple Silicon"]
    process_model: both
    service: Native Swift/SwiftUI menu bar app with supervised background server; Homebrew/start command can run a managed background server; `omlx serve` runs foreground.
    notes: Observed locally as `/Applications/oMLX.app` v0.4.4, `/usr/local/bin/omlx` shim, `oMLX` app process, and `omlx-server` process. Requires Apple Silicon; the homepage and repository position it as Mac/Apple Silicon software.
  - os: linux
    support: unsupported
    service: none
    notes: No official Linux release artifact found. The runner depends on Apple's MLX/Metal stack and is documented as Apple Silicon/macOS focused.
  - os: windows
    support: unsupported
    service: none
    notes: No official Windows release artifact found. Windows users would need a different runner; WSL does not provide Apple MLX/Metal acceleration.

api_standards:
  - standard: openai_compatible
    supported: yes
    base_url: http://localhost:8000/v1
    key_paths:
      - /v1/models
      - /v1/models/status
      - /v1/models/{model_id}/load
      - /v1/models/{model_id}/unload
      - /v1/chat/completions
      - /v1/completions
      - /v1/embeddings
      - /v1/rerank
      - /v1/audio/speech
      - /v1/audio/transcriptions
      - /v1/audio/process
      - /v1/responses
      - /v1/responses/{response_id}
    auth: "optional_api_key via `Authorization: Bearer <key>`"
    since_version: v0.1.0
    deviations:
      - The initial v0.1.0 release notes verify core OpenAI-compatible chat/completions/models/embeddings support; current v0.4.4 OpenAPI adds Responses, audio, rerank, model status, and load/unload helpers.
      - "`/v1/rerank` uses a Cohere/Jina-style request/response shape."
      - "`/v1/responses` is implemented for OpenAI Responses/Codex compatibility and stores response IDs for later `GET /v1/responses/{response_id}`."
      - Audio endpoints include oMLX-specific behavior such as `/v1/audio/process`.
    docs_url: https://github.com/jundot/omlx#api-compatibility
  - standard: anthropic_compatible
    supported: yes
    base_url: http://localhost:8000
    key_paths:
      - /v1/messages
      - /v1/messages/count_tokens
    auth: "optional API key; Anthropic clients use `x-api-key`/`ANTHROPIC_AUTH_TOKEN`, and Bearer auth is accepted by the FastAPI security layer."
    since_version: v0.1.0
    deviations:
      - Anthropic SDK base URL excludes `/v1` because clients append `/v1/messages`.
      - Current docs advertise streaming, adaptive thinking, vision inputs, and tool use, but exact behavior remains model-template dependent.
      - Token-counting is available at `/v1/messages/count_tokens`.
    docs_url: https://github.com/jundot/omlx#api-compatibility
  - standard: native
    supported: yes
    base_url: http://localhost:8000
    key_paths:
      - /health
      - /api/status
      - /admin
      - /admin/api/models
      - /admin/api/models/{model_id}/load
      - /admin/api/models/{model_id}/unload
      - /admin/api/models/{model_id}/settings
      - /admin/api/models/{model_id}/profiles
      - /admin/api/global-settings
      - /admin/api/server-info
      - /admin/api/device-info
      - /admin/api/stats
      - /admin/api/cache/probe
      - /admin/api/hf/download
      - /admin/api/hf/models
      - /admin/api/ms/download
      - /admin/api/ms/search
      - /admin/api/oq/start
      - /admin/api/upload/start
      - /admin/api/bench/start
    auth: "mixed; `/health`, `/admin`, and OpenAPI are public; `/api/status` uses API-key auth; most `/admin/api/*` routes require admin session auth."
    since_version: unknown
    deviations:
      - Native/admin routes are FastAPI routes surfaced by `/openapi.json`; many require a browser session cookie even though OpenAPI lists no HTTPBearer security requirement.
      - "`/admin/api/server/restart` is gated by the `OMLX_SUPERVISED` environment variable in source."
      - "`/metrics` is not an oMLX endpoint on the observed v0.4.4 server; it returned 404."
    docs_url: http://localhost:8000/openapi.json

metadata_endpoints:
  - purpose: health
    method: get
    path: /health
    gated_by: ""
    auth_gated: false
    response_hint: '{"status":"healthy","default_model":"...","engine_pool":{"model_count":N,"loaded_count":N}}'
    notes: Observed locally on v0.4.4. This is the best unauthenticated detector probe.
  - purpose: other
    method: get
    path: /api/status
    gated_by: ""
    auth_gated: true
    response_hint: server status JSON
    notes: Present in live OpenAPI. Observed anonymous request returned `{"detail":"API key required"}`.
  - purpose: version
    method: get
    path: /openapi.json
    gated_by: ""
    auth_gated: false
    response_hint: '{"info":{"title":"oMLX API","version":"0.4.4"}}'
    notes: No dedicated `/version` endpoint found; OpenAPI and `omlx --version` expose version data.
  - purpose: model_list
    method: get
    path: /v1/models
    gated_by: ""
    auth_gated: true
    response_hint: '{"object":"list","data":[{"id":"...","owned_by":"omlx"}]}'
    notes: Observed anonymous request returned an OpenAI-style authentication error because this host has an API key configured.
  - purpose: loaded_models
    method: get
    path: /v1/models/status
    gated_by: ""
    auth_gated: true
    response_hint: '{"model_count":N,"loaded_count":N,"models":[...]}'
    notes: Present in live OpenAPI and used by the CLI launcher; anonymous request returned authentication error locally.
  - purpose: load_model
    method: post
    path: /v1/models/{model_id}/load
    gated_by: ""
    auth_gated: true
    response_hint: '{"id":"...","loaded":true}'
    notes: Public API-key-gated model load helper.
  - purpose: unload_model
    method: post
    path: /v1/models/{model_id}/unload
    gated_by: ""
    auth_gated: true
    response_hint: '{"id":"...","loaded":false}'
    notes: Public API-key-gated model unload helper.
  - purpose: admin_ui
    method: get
    path: /admin
    gated_by: ""
    auth_gated: false
    response_hint: login/setup HTML for the oMLX admin dashboard
    notes: Observed public route; authenticated dashboard pages use a session cookie.
  - purpose: model_list
    method: get
    path: /admin/api/models
    gated_by: ""
    auth_gated: true
    response_hint: model settings and load-state JSON
    notes: Admin-session-gated in practice; anonymous request patterns for admin APIs returned `Admin authentication required`.
  - purpose: model_info
    method: get
    path: /admin/api/hf/model-info
    gated_by: ""
    auth_gated: true
    response_hint: HuggingFace model metadata JSON
    notes: Enumerated from v0.4.4 `/openapi.json`; admin-session-gated.
  - purpose: metrics
    method: get
    path: /admin/api/stats
    gated_by: ""
    auth_gated: true
    response_hint: persistent serving statistics JSON
    notes: Observed anonymous request returned `Admin authentication required`. `/metrics` returned 404 on this host.
  - purpose: other
    method: post
    path: /admin/api/cache/probe
    gated_by: ""
    auth_gated: true
    response_hint: cache-location/probe JSON
    notes: Admin-session-gated cache probe for prompt/cache state.
  - purpose: other
    method: post
    path: /admin/api/hf/download
    gated_by: ""
    auth_gated: true
    response_hint: '{"task_id":"..."}'
    notes: Starts a HuggingFace model download task.
  - purpose: other
    method: post
    path: /admin/api/ms/download
    gated_by: ""
    auth_gated: true
    response_hint: '{"task_id":"..."}'
    notes: Starts a ModelScope model download task; ModelScope support is present in v0.4.4 OpenAPI and settings.
  - purpose: other
    method: get
    path: /v1/mcp/tools
    gated_by: OMLX_MCP_CONFIG for useful configured tools
    auth_gated: true
    response_hint: MCP tools JSON
    notes: Route exists regardless of MCP configuration, but useful content depends on configured MCP servers.
  - purpose: other
    method: get
    path: /v1/mcp/servers
    gated_by: OMLX_MCP_CONFIG for useful configured servers
    auth_gated: true
    response_hint: MCP server status JSON
    notes: Route exists regardless of MCP configuration, but useful content depends on configured MCP servers.
  - purpose: other
    method: post
    path: /v1/mcp/execute
    gated_by: OMLX_MCP_CONFIG for useful configured tools
    auth_gated: true
    response_hint: MCP execution JSON
    notes: Executes a configured MCP tool.

detection:
  - os: macos
    method: binary
    target: omlx
    expect: "omlx: Production-ready LLM server for Apple Silicon"
    confidence: observed
    notes: Observed via `which omlx` at `/usr/local/bin/omlx`; `omlx --version` returned `0.4.4`.
  - os: macos
    method: app_bundle
    target: /Applications/oMLX.app
    expect: 'CFBundleIdentifier: app.omlx'
    confidence: observed
    notes: Observed locally with `CFBundleShortVersionString` 0.4.4.
  - os: macos
    method: process
    target: oMLX
    expect: /Applications/oMLX.app/Contents/MacOS/oMLX
    confidence: observed
    notes: Native menu bar app process observed locally.
  - os: macos
    method: process
    target: omlx-server
    expect: "omlx-server background server process"
    confidence: observed
    notes: Managed server process observed locally.
  - os: all
    method: port
    target: "8000"
    expect: ""
    confidence: documented
    notes: Default port is shared with vLLM and other servers; never identify oMLX from the port alone.
  - os: all
    method: http
    target: GET /health
    expect: '{"status":"healthy","engine_pool":{"model_count":N,"loaded_count":N}}'
    confidence: observed
    notes: Observed ungated on v0.4.4; strongest running-server identity probe.
  - os: all
    method: http
    target: GET /openapi.json
    expect: '{"info":{"title":"oMLX API"}}'
    confidence: observed
    notes: Observed ungated on v0.4.4; also provides version and complete route inventory.
  - os: all
    method: http
    target: GET /
    expect: "404 Not Found"
    confidence: observed
    notes: Observed root path returns FastAPI `{"detail":"Not Found"}`; this is negative evidence and should not be used as a positive identity marker.
  - os: macos
    method: config_file
    target: /Users/ken/.omlx/settings.json
    expect: '"version": "1.0"'
    confidence: observed
    notes: Observed actual config under `/Users/ken/.omlx` because this session's `HOME` is `/Users/ken/.claudine`; detectors should use the real user home, not the process `HOME` if it is intentionally redirected.
  - os: macos
    method: config_file
    target: ~/.omlx/settings.json
    expect: '"version": "1.0"'
    confidence: documented
    notes: Fresh default base path for settings unless `--base-path` or `OMLX_BASE_PATH` is used.

identity_probes:
  - rank: 1
    request: GET /health
    match_in: json_field
    field: engine_pool
    marker: '"engine_pool":{"model_count":N,"loaded_count":N,...}'
    uniqueness: unique
    zero_model_ok: true
    auth_gated: false
    confidence: observed
    notes: The engine_pool key exists in no other runner's health response (vLLM's /health returns an empty 200 body — body shape disambiguates the shared port 8000). Stays unauthenticated even when an API key is configured; verified live on oMLX 0.5.1.
  - rank: 2
    request: GET /openapi.json
    match_in: json_field
    field: info.title
    marker: "oMLX API"
    uniqueness: unique
    zero_model_ok: true
    auth_gated: false
    confidence: observed
    notes: Literal FastAPI title; also yields the exact server version and full route inventory. Unauthenticated even with an API key set.
  - rank: 3
    request: GET /api/status
    match_in: json_field
    field: detail
    marker: '{"detail":"API key required"}'
    uniqueness: strong
    zero_model_ok: true
    auth_gated: true
    confidence: observed
    notes: The /api/* and /admin/api/* namespaces are oMLX-only — even the 401 auth rejection proves the route exists and identifies the software when a key is configured.
  - rank: 4
    request: ANY /
    match_in: header
    field: server
    marker: uvicorn
    uniqueness: weak
    zero_model_ok: true
    auth_gated: false
    confidence: observed
    notes: "`server: uvicorn` is shared with vLLM and every FastAPI server — never use it to identify oMLX; identification must come from response bodies."

version_probe:
  - os: macos
    method: cli
    command: omlx --version
    pattern: "^(\\d+\\.\\d+\\.\\d+)$"
    confidence: observed
    notes: Observed `0.5.1` — a plain semver on stdout. For the running server's version use `GET /openapi.json` → info.version (identity_probes rank 2).
  - os: macos
    method: bundle
    command: "defaults read /Applications/oMLX.app/Contents/Info.plist CFBundleShortVersionString"
    pattern: "(\\S+)"
    confidence: observed
    notes: Observed 0.5.1; the menu-bar app and the omlx CLI version in lockstep on this host.
  - os: linux
    method: cli
    command: omlx --version
    pattern: "^(\\d+\\.\\d+\\.\\d+)$"
    confidence: inferred
    notes: oMLX is macOS-first (Apple Silicon MLX); Linux install paths are not an official released artifact — record honestly if observed.

config_mechanism: mixed

config_files:
  - os: macos
    path: ~/.omlx/settings.json
    format: json
    role: primary server settings
    notes: Observed v1 settings include server host/port, CORS aliases, auth/subkeys, model directories, cache, memory guard, scheduler, HuggingFace, ModelScope, network, logging, idle timeout, and agent integration defaults.
  - os: macos
    path: ~/.omlx/model_settings.json
    format: json
    role: per-model settings
    notes: Observed v1 file stores model defaults and overrides such as default model, trust_remote_code, guided grammar, speculative options, MTP/VLM MTP, TurboQuant KV, DFlash, and pinning.
  - os: macos
    path: ~/.omlx/stats.json
    format: json
    role: persistent serving statistics
    notes: Observed token/request/cache counters persist across restarts and feed admin stats.
  - os: macos
    path: ~/Library/Application Support/oMLX
    format: other
    role: native app support directory
    notes: App support path for the Swift menu bar app; the prior research found a `base-path` pointer here in source/older installs, but it was not observed in this run.

env_vars:
  - name: OMLX_HOST
    effect: Overrides the server bind host.
  - name: OMLX_PORT
    effect: Overrides the server port.
  - name: OMLX_LOG_LEVEL
    effect: Overrides server log level.
  - name: OMLX_BASE_PATH
    effect: Relocates the oMLX data/config home; equivalent CLI flag is `--base-path`.
  - name: OMLX_MODEL_DIR
    effect: Overrides model discovery directory; modern settings also support `model.model_dirs`.
  - name: OMLX_MAX_CONCURRENT_REQUESTS
    effect: Sets scheduler request concurrency.
  - name: OMLX_MAX_NUM_SEQS
    effect: Backward-compatible alias read as scheduler request concurrency.
  - name: OMLX_EMBEDDING_BATCH_SIZE
    effect: Sets embedding forward-pass batch size.
  - name: OMLX_CACHE_ENABLED
    effect: Enables or disables the oMLX cache layer.
  - name: OMLX_SSD_CACHE_DIR
    effect: Sets SSD cache directory.
  - name: OMLX_SSD_CACHE_MAX_SIZE
    effect: Sets SSD cache size limit.
  - name: OMLX_HOT_CACHE_ONLY
    effect: Keeps cache blocks in memory only instead of using SSD tiering.
  - name: OMLX_INITIAL_CACHE_BLOCKS
    effect: Sets cache blocks preallocated at startup.
  - name: OMLX_API_KEY
    effect: Sets API key for HTTP client authentication.
  - name: OMLX_SECRET_KEY
    effect: Sets admin/session signing secret.
  - name: OMLX_MCP_CONFIG
    effect: Path to MCP server config used by `/v1/mcp/*` routes and tool integration.
  - name: OMLX_HF_ENDPOINT
    effect: Custom HuggingFace Hub endpoint.
  - name: OMLX_HF_CACHE_ENABLED
    effect: Enables discovery from the standard HuggingFace cache.
  - name: OMLX_MS_ENDPOINT
    effect: Custom ModelScope endpoint.
  - name: OMLX_HTTP_PROXY
    effect: HTTP proxy for model downloads/network access.
  - name: OMLX_HTTPS_PROXY
    effect: HTTPS proxy for model downloads/network access.
  - name: OMLX_NO_PROXY
    effect: Comma-separated proxy bypass list.
  - name: OMLX_CA_BUNDLE
    effect: Custom TLS CA bundle.
  - name: OMLX_LOG_DIR
    effect: Overrides log directory.
  - name: OMLX_LOG_RETENTION_DAYS
    effect: Sets log retention period.
  - name: OMLX_SUPERVISED
    effect: Enables supervised-server-only behavior such as the admin restart endpoint.
  - name: OMLX_MARKITDOWN_ENABLED
    effect: Toggles MarkItDown attachment preprocessing integration.
  - name: OMLX_MARKITDOWN_EXPOSE_MODEL
    effect: Controls whether MarkItDown is exposed as a model.
  - name: OMLX_MARKITDOWN_PDF_PROCESSING_ENGINE
    effect: Selects MarkItDown PDF processing engine.
  - name: OMLX_DECODE_BURST_MAX_STEPS
    effect: Advanced burst-decode scheduler knob.
  - name: OMLX_DECODE_BURST_BUDGET_SINGLE_S
    effect: Advanced burst-decode single-stream time budget.
  - name: OMLX_DECODE_BURST_BUDGET_S
    effect: Advanced burst-decode multi-stream time budget.

model_id_grammar: >
  oMLX discovers MLX model directories from configured model directories. Model
  IDs are directory names such as `Qwen3.6-35B-A3B-oQ6`, two-level owner/model
  paths such as `mlx-community/MiniCPM-V-4.6-bf16`, per-model aliases configured
  in `model_settings.json` or the admin UI, and profile-qualified IDs such as
  `<model>:<profile>` or `<alias>:<profile>` when model profiles are configured.
  IDs commonly encode family, size, variant, dtype, and quantization tags
  (`bf16`, `4bit`, `8bit`, `mxfp4`, `nvfp4`, `oQ6`, `mtp`) but oMLX treats them
  as path/alias strings rather than a strict registry grammar.

model_formats:
  - mlx

model_acquisition:
  - method: manual
    example: "Place an MLX-format directory containing `config.json` and `*.safetensors` under `~/.omlx/models` or any configured `model_dirs` path."
    notes: Observed host uses `/Volumes/Fast Bastard/models/omlx` as the active model directory.
  - method: huggingface
    example: "Use the admin dashboard or `POST /admin/api/hf/download` with a HuggingFace `repo_id`, such as `mlx-community/Llama-3.2-3B-Instruct-4bit`."
    notes: v0.4.0 release notes added standard HuggingFace cache discovery; v0.4.4 settings expose `huggingface.hf_cache_enabled`.
  - method: registry
    example: "Use ModelScope via the admin dashboard or `POST /admin/api/ms/download` after configuring `OMLX_MS_ENDPOINT` if needed."
    notes: ModelScope routes and settings were present in the observed v0.4.4 OpenAPI/config.
  - method: in_app
    example: "Use the native app/admin dashboard model browser to download and manage models."
    notes: Official homepage and README describe dashboard-based model browsing/downloads.

model_store_paths:
  - os: macos
    path: ~/.omlx/models
    notes: Fresh-install default from CLI help and settings defaults.
  - os: macos
    path: /Users/ken/.omlx/models
    notes: Observed local default base path exists, but this host's active `model_dirs` points elsewhere.
  - os: macos
    path: /Volumes/Fast Bastard/models/omlx
    notes: Observed active host model directory from `/Users/ken/.omlx/settings.json`; host-specific, not a product default.
  - os: macos
    path: ~/.cache/huggingface/hub
    notes: Optional discovery source when HuggingFace cache discovery is enabled; documented in v0.4.0 release notes and visible in settings.

hardware_acceleration:
  - metal
  - cpu

concurrency:
  multi_model: true
  parallel_requests: true
  notes: oMLX advertises multi-model serving and continuous batching. Observed `/health` reported 21 discovered models and 1 loaded model; settings default to 8 concurrent requests.

streaming_sse: true
tool_calling: conditional
tool_calling_notes: Tool/function calling depends on the model chat template and parser support. Current README lists mlx-lm formats for Llama/Qwen/DeepSeek JSON, Qwen3.5 XML, Gemma, GLM, MiniMax, Mistral, Kimi K2, and Longcat; v0.4.4 release notes include additional tool-call parsing fixes.
embeddings: true
rerank: true
web_ui_url: http://localhost:8000/admin

integration_hooks:
  - command: omlx launch claude
    effect: Selects a model and execs Claude Code with `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, tier model variables, and context window settings for the running oMLX server.
    notes: Verified in v0.3.9.dev1 release notes and local `omlx launch --help`.
  - command: omlx launch codex
    effect: Launches Codex CLI configured for oMLX.
    notes: Verified in local `omlx launch --help`.
  - command: omlx launch codex_app
    effect: Launches Codex App Desktop configured for oMLX.
    notes: v0.4.4 release notes mention Codex App Desktop integration.
  - command: omlx launch opencode
    effect: Launches OpenCode configured for oMLX.
    notes: Verified in local `omlx launch --help`.
  - command: omlx launch openclaw
    effect: Launches OpenClaw configured for oMLX with an optional tools profile.
    notes: Local help supports `--tools-profile minimal|coding|messaging|full`.
  - command: omlx launch hermes
    effect: Launches Hermes Agent configured for oMLX.
    notes: v0.4.4 release notes mention Hermes launch-flow fixes.
  - command: omlx launch pi
    effect: Launches Pi configured for oMLX.
    notes: Verified in local `omlx launch --help`.
  - command: omlx launch copilot
    effect: Launches GitHub Copilot CLI configured for oMLX.
    notes: Verified in v0.3.9.dev2 release notes and local `omlx launch --help`.

traps:
  - "This session's `HOME` is `/Users/ken/.claudine`, but the actual oMLX files are under `/Users/ken/.omlx`; detectors should not rely blindly on an agent-wrapper HOME override."
  - "Port 8000 is ambiguous because vLLM also defaults there; require `/health` with `status: healthy` plus oMLX-shaped `engine_pool`, or `/openapi.json` with `info.title: oMLX API`."
  - "`/` is not a useful health endpoint; it returned 404 on the observed v0.4.4 server."
  - "`/metrics` is not present on observed v0.4.4; use `/admin/api/stats` for stats, but it is admin-auth gated."
  - "OpenAPI marks many `/admin/api/*` routes without HTTPBearer security, but local anonymous requests still returned `Admin authentication required`; admin routes are session-gated in practice."
  - "`model.model_dirs` is the modern multi-directory setting; `model.model_dir` may still exist for compatibility."
  - "`OMLX_MAX_NUM_SEQS` is accepted as a concurrency alias, but current CLI/settings wording prefers `OMLX_MAX_CONCURRENT_REQUESTS`."

opencode_example: '{"provider":{"omlx":{"npm":"@ai-sdk/openai-compatible","name":"oMLX (local)","options":{"baseURL":"http://localhost:8000/v1"},"models":{"Qwen3.6-35B-A3B-oQ6":{"name":"Qwen3.6-35B-A3B-oQ6 (local)"}}}}}'

changes:
  - "Updated research from the prior 2026-07-02 entry to observed oMLX v0.4.4 on 2026-07-03."
  - "Confirmed local install path, app bundle ID/version, running `oMLX` and `omlx-server` processes, and `omlx --version` 0.4.4."
  - "Confirmed `/health` and `/openapi.json` are ungated identity probes, while `/v1/models`, `/v1/models/status`, `/api/status`, and `/admin/api/*` are auth/session gated on this host."
  - "Recorded that `/` and `/metrics` are negative probes on v0.4.4 (`404 Not Found`)."
  - "Expanded the API inventory from live `/openapi.json`, including `/api/status`, Responses get/delete support, ModelScope, oQ quantization, upload, cache clear/probe, benchmark, device-info, update-check, and model profile/admin routes."
  - "Updated environment variables from installed source, including `OMLX_SSD_CACHE_DIR`, `OMLX_CACHE_ENABLED`, `OMLX_MS_ENDPOINT`, `OMLX_LOG_DIR`, `OMLX_SUPERVISED`, MarkItDown toggles, and burst-decode knobs."
  - "Updated installation/runtime notes for the native Swift/SwiftUI app introduced in v0.4.0 and the Codex App/Hermes integration updates in v0.4.4."
requires_claudine_update: true
reason: Claudine/sniff should add or update oMLX detection to handle the app bundle, `omlx-server` process, ambiguous port 8000, positive `/health` and `/openapi.json` markers, negative `/` and `/metrics` behavior, and wrapper-HOME-vs-real-home config discovery.
---

# oMLX Local Model Runner

## Introduction to oMLX

[oMLX](https://omlx.ai) is an Apache-2.0 local model runner for Apple Silicon Macs. It serves MLX-format LLM, VLM, embedding, reranking, audio, and OCR models through HTTP APIs, with continuous batching, multi-model serving, and a tiered RAM/SSD KV cache for repeated long-context workloads.

| Resource | URL |
| --- | --- |
| Homepage | https://omlx.ai |
| Documentation | https://github.com/jundot/omlx |
| Repository | https://github.com/jundot/omlx |
| Releases | https://github.com/jundot/omlx/releases |
| Live API schema | http://localhost:8000/openapi.json |
| Live Swagger UI | http://localhost:8000/docs |

The current official distribution is macOS-focused. The v0.4.0 release replaced the older PyObjC menu bar app with a native Swift/SwiftUI app, and the observed local installation is v0.4.4.

## Platforms and Installation

| OS | Support | Binary/processes | Install methods | Process model | Service management |
| --- | --- | --- | --- | --- | --- |
| macOS | Native | `omlx`, `oMLX`, `omlx-server` | Signed DMG, Homebrew tap/formula, source install | Both | Native menu bar app, managed background server, or foreground `omlx serve` |
| Linux | Unsupported | None official | None official | N/A | None |
| Windows | Unsupported | None official | None official | N/A | None |

Observed locally on July 3, 2026:

- `/usr/local/bin/omlx` points at `/Applications/oMLX.app/Contents/MacOS/omlx-cli`.
- `omlx --version` returns `0.4.4`.
- `/Applications/oMLX.app` has bundle ID `app.omlx` and short version `0.4.4`.
- Processes `oMLX` and `omlx-server` are running.

## API Surface

oMLX binds to `127.0.0.1:8000` by default. The OpenAI-compatible client base URL includes `/v1`; the Anthropic-compatible base URL omits `/v1` because Anthropic clients append `/v1/messages`.

| API family | Supported | Client base URL | Auth |
| --- | --- | --- | --- |
| OpenAI-compatible | Yes | `http://localhost:8000/v1` | Optional API key; required on this configured host |
| Anthropic Messages | Yes | `http://localhost:8000` | Optional API key; required on this configured host |
| Native/admin | Yes | `http://localhost:8000` | Mixed: public health/schema/admin login; API key or admin session for most data routes |

### OpenAI-Compatible Endpoints

| Method | Path | Notes |
| --- | --- | --- |
| GET | `/v1/models` | Lists available models; auth-gated locally |
| GET | `/v1/models/status` | Lists load state and model metadata; auth-gated locally |
| POST | `/v1/models/{model_id}/load` | Loads a model |
| POST | `/v1/models/{model_id}/unload` | Unloads a model |
| POST | `/v1/chat/completions` | Chat completions with streaming |
| POST | `/v1/completions` | Text completions with streaming |
| POST | `/v1/embeddings` | Text and multimodal embeddings |
| POST | `/v1/rerank` | Cohere/Jina-style reranking |
| POST | `/v1/audio/speech` | Text-to-speech |
| POST | `/v1/audio/transcriptions` | Speech-to-text |
| POST | `/v1/audio/process` | oMLX audio processing extension |
| POST | `/v1/responses` | OpenAI Responses API |
| GET | `/v1/responses/{response_id}` | Fetch stored response |
| DELETE | `/v1/responses/{response_id}` | Delete stored response |
| GET | `/v1/mcp/servers` | MCP server status |
| GET | `/v1/mcp/tools` | MCP tool list |
| POST | `/v1/mcp/execute` | MCP tool execution |

### Anthropic-Compatible Endpoints

| Method | Path | Notes |
| --- | --- | --- |
| POST | `/v1/messages` | Anthropic Messages API with streaming/tool support where the model template supports it |
| POST | `/v1/messages/count_tokens` | Anthropic token-counting endpoint |

### Native And Metadata Endpoints

| Method | Path | Purpose | Auth |
| --- | --- | --- | --- |
| GET | `/health` | Health and engine-pool summary | Public |
| GET | `/openapi.json` | FastAPI OpenAPI schema/version | Public |
| GET | `/api/status` | Server status | API key |
| GET | `/admin` | Admin login/setup page | Public |
| GET | `/admin/api/models` | Admin model list/settings | Admin session |
| POST | `/admin/api/models/{model_id}/load` | Admin model load | Admin session |
| POST | `/admin/api/models/{model_id}/unload` | Admin model unload | Admin session |
| PUT | `/admin/api/models/{model_id}/settings` | Per-model settings | Admin session |
| GET/POST | `/admin/api/models/{model_id}/profiles` | Model profile list/create | Admin session |
| GET/POST | `/admin/api/global-settings` | Server settings | Admin session |
| GET | `/admin/api/server-info` | Runtime info | Admin session |
| GET | `/admin/api/device-info` | Host/device info | Admin session |
| GET | `/admin/api/stats` | Persistent serving stats | Admin session |
| POST | `/admin/api/cache/probe` | Prefix/cache probe | Admin session |
| POST | `/admin/api/hf/download` | HuggingFace download | Admin session |
| POST | `/admin/api/ms/download` | ModelScope download | Admin session |
| POST | `/admin/api/oq/start` | oQ quantization task | Admin session |
| POST | `/admin/api/upload/start` | Upload task | Admin session |
| POST | `/admin/api/bench/start` | Benchmark task | Admin session |

`GET /` returned `404 Not Found` on the observed host. `GET /metrics` also returned `404 Not Found`; use `/admin/api/stats` for stats, with admin auth.

## Detection

Recommended probe order:

1. Check `omlx` on `PATH`; `omlx --help` includes `Production-ready LLM server for Apple Silicon`.
2. On macOS, check `/Applications/oMLX.app` and bundle ID `app.omlx`.
3. Check for process names `oMLX` and `omlx-server`.
4. Treat TCP port `8000` only as a hint; it collides with vLLM and other tools.
5. Probe `GET http://localhost:8000/health`; identify oMLX by `status: healthy` and an `engine_pool` object.
6. Probe `GET http://localhost:8000/openapi.json`; identify oMLX by `info.title: oMLX API`.
7. Check config under the real user home, usually `~/.omlx/settings.json`. Agent wrappers may override `HOME`; this session's `HOME` was `/Users/ken/.claudine`, while the actual oMLX config was `/Users/ken/.omlx/settings.json`.

### Port identity

Port 8000 collides with vLLM (and both are FastAPI/uvicorn servers, so the
`server: uvicorn` header cannot separate them), so the ranked
`identity_probes` frontmatter block is the canonical strategy for answering
"which runner is listening on this port?":

1. `GET /health` — a body containing an `engine_pool` object
   (`model_count`, `loaded_count`, memory ceilings) is unique to oMLX; vLLM's
   `/health` returns an empty 200 body. Ungated even when an API key is
   configured, and works with zero models loaded.
2. `GET /openapi.json` — `info.title: oMLX API` is a literal fingerprint and
   also yields the exact version and full route inventory; unauthenticated.
3. `GET /api/status` — even the 401 `{"detail":"API key required"}` rejection
   identifies oMLX, because the `/api/*` and `/admin/api/*` namespaces exist
   nowhere else.
4. Header check — `server: uvicorn` is shared with vLLM and every FastAPI
   server; never use it for identity.

## Configuration

The primary config mechanism is mixed: JSON settings files, CLI flags, environment variables, and the native app/admin UI all participate.

| Path | Format | Role |
| --- | --- | --- |
| `~/.omlx/settings.json` | JSON | Server, auth, model directories, cache, scheduler, network, integrations |
| `~/.omlx/model_settings.json` | JSON | Per-model overrides, default model, trust and advanced decoding settings |
| `~/.omlx/stats.json` | JSON | Persistent request/token/cache statistics |
| `~/Library/Application Support/oMLX` | App support | Native Swift app state and support files |

Important observed settings include `server.host`, `server.port`, `model.model_dirs`, `auth.api_key`, `auth.skip_api_key_verification`, `cache.ssd_cache_dir`, `scheduler.max_concurrent_requests`, `huggingface.hf_cache_enabled`, `modelscope.endpoint`, and integration model defaults for Codex/OpenCode/OpenClaw/Hermes/Pi/Copilot.

Important environment variables include `OMLX_HOST`, `OMLX_PORT`, `OMLX_BASE_PATH`, `OMLX_MODEL_DIR`, `OMLX_MAX_CONCURRENT_REQUESTS`, `OMLX_MAX_NUM_SEQS`, `OMLX_API_KEY`, `OMLX_SECRET_KEY`, `OMLX_MCP_CONFIG`, `OMLX_HF_ENDPOINT`, `OMLX_HF_CACHE_ENABLED`, `OMLX_MS_ENDPOINT`, proxy variables, logging variables, MarkItDown toggles, and advanced burst-decode/cache knobs.

## Models

oMLX serves MLX-format models. A model is usually a directory containing `config.json` and `*.safetensors` files under one of the configured model directories.

Accepted model ID forms:

- Directory name: `Qwen3.6-35B-A3B-oQ6`
- Two-level owner/model directory: `mlx-community/MiniCPM-V-4.6-bf16`
- Configured alias from `model_settings.json` or the admin UI
- Profile-qualified model or alias: `<model>:<profile>`

Model IDs commonly include size, family, precision, and quantization tags such as `bf16`, `4bit`, `8bit`, `mxfp4`, `nvfp4`, `oQ6`, and `mtp`, but the server treats IDs as path/alias strings.

Acquisition paths:

| Method | Example |
| --- | --- |
| Manual | Place an MLX model directory under `~/.omlx/models` or a configured `model_dirs` path |
| HuggingFace | Download `mlx-community/Llama-3.2-3B-Instruct-4bit` from the admin dashboard or `/admin/api/hf/download` |
| ModelScope | Download from ModelScope through `/admin/api/ms/download` |
| In app | Use the native/web admin model browser |

Fresh default model store is `~/.omlx/models`. This host has an active configured model directory at `/Volumes/Fast Bastard/models/omlx`.

## Capabilities

| Capability | Support | Notes |
| --- | --- | --- |
| Hardware acceleration | Metal, CPU | Apple Silicon/macOS focus |
| Multi-model serving | Yes | LLM, VLM, embedding, reranker, audio, OCR, and MarkItDown surfaces |
| Parallel requests | Yes | Continuous batching; default scheduler concurrency observed as 8 |
| SSE streaming | Yes | OpenAI, Anthropic, Responses, and benchmark streams |
| Tool/function calling | Conditional | Depends on model template and parser support |
| Embeddings | Yes | `/v1/embeddings` |
| Reranking | Yes | `/v1/rerank` |
| Web UI | Yes | `http://localhost:8000/admin` |

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
          "name": "Qwen3.6-35B-A3B-oQ6 (local)"
        }
      }
    }
  }
}
```

### Claude Code

```bash
export ANTHROPIC_BASE_URL=http://localhost:8000
export ANTHROPIC_AUTH_TOKEN=<your-omlx-api-key>
claude --model Qwen3.6-35B-A3B-oQ6
```

### Runner-Native Hooks

`omlx launch <tool>` can configure and exec agentic tools directly:

```bash
omlx launch claude
omlx launch codex
omlx launch codex_app
omlx launch opencode
omlx launch openclaw --tools-profile coding
omlx launch hermes
omlx launch pi
omlx launch copilot
```

## Changelog

- 2026-07-03: Updated from the 2026-07-02 research to observed oMLX v0.4.4. Confirmed local installation/processes/config, public `/health` and `/openapi.json` probes, auth-gated model/status/admin endpoints, negative `/` and `/metrics` probes, expanded v0.4.4 OpenAPI route coverage, ModelScope acquisition, Swift app notes, and current launch integrations.

## Sources

- [oMLX homepage](https://omlx.ai)
- [oMLX GitHub repository and README](https://github.com/jundot/omlx)
- [oMLX releases](https://github.com/jundot/omlx/releases)
- [oMLX v0.1.0 release notes](https://github.com/jundot/omlx/releases/tag/v0.1.0)
- [oMLX v0.3.9.dev1 release notes](https://github.com/jundot/omlx/releases/tag/v0.3.9.dev1)
- [oMLX v0.3.9.dev2 release notes](https://github.com/jundot/omlx/releases/tag/v0.3.9.dev2)
- [oMLX v0.4.0 release notes](https://github.com/jundot/omlx/releases/tag/v0.4.0)
- [oMLX v0.4.4 release notes](https://github.com/jundot/omlx/releases/tag/v0.4.4)
- Local observations on July 3, 2026: `which omlx`, `omlx --version`, `omlx --help`, `omlx serve --help`, `omlx launch --help`, `/Applications/oMLX.app` Info.plist, process list, `/Users/ken/.omlx/settings.json`, `/Users/ken/.omlx/model_settings.json`, `/Users/ken/.omlx/stats.json`, and live HTTP probes against `http://localhost:8000`.

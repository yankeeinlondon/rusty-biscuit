---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: codex
model: default

summary: Ollama is an open-source local model runner that serves GGUF models over native REST endpoints plus OpenAI- and Anthropic-compatible HTTP APIs.
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
  Local requests to http://localhost:11434 do not require authentication. OpenAI
  and Anthropic SDK keys are accepted for compatibility and ignored for local
  inference. Authentication is required for ollama.com cloud models, private
  model downloads, publishing, and direct cloud API access.

platforms:
  - os: macos
    support: native
    binary: ollama
    alt_binaries: ["Ollama.app/Contents/MacOS/Ollama", "Ollama.app/Contents/Resources/ollama"]
    install: ["DMG from https://ollama.com/download/mac", "brew install ollama", "curl -fsSL https://ollama.com/install.sh | sh"]
    process_model: both
    service: macOS menu bar app/login item, or foreground `ollama serve`
    notes: Requires macOS Sonoma 14 or newer. Apple M-series supports CPU and GPU; x86 Mac is CPU only. Observed on this host as `/Applications/Ollama.app` v0.30.11 with `/usr/local/bin/ollama` symlinked to the app resource binary.
  - os: linux
    support: native
    binary: ollama
    alt_binaries: []
    install: ["curl -fsSL https://ollama.com/install.sh | sh", "manual tar.zst extract to /usr", "Docker image ollama/ollama", "third-party distro packages"]
    process_model: both
    service: systemd unit `ollama.service` when installed by the official script or manual service instructions; foreground `ollama serve`
    notes: Official Linux docs create an `ollama` user with home `/usr/share/ollama`; AMD ROCm and ARM64 have separate download packages.
  - os: windows
    support: native
    binary: ollama.exe
    alt_binaries: ["Ollama.exe"]
    install: ["OllamaSetup.exe from https://ollama.com/download/windows", "standalone ollama-windows-amd64.zip", "Docker with WSL2 for GPU passthrough"]
    process_model: both
    service: Windows tray/background app after installer; foreground `ollama serve` from cmd, PowerShell, or terminal
    notes: Requires Windows 10 22H2 or newer. Native Windows app supports NVIDIA and AMD Radeon acceleration; Docker GPU acceleration requires Linux or Windows with WSL2.

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
    auth: none locally; optional ignored API key for SDK compatibility
    since_version: "v0.1.24"
    deviations:
      - "Initial OpenAI compatibility in v0.1.24 covered chat completions; later endpoints were added incrementally."
      - "/v1/completions prompt currently accepts a string, not the full OpenAI prompt union."
      - "/v1/responses is non-stateful; previous_response_id and conversation fields are accepted but not preserved. Current docs say it was added in v0.13.3, but that release note does not name the endpoint."
      - "/v1/images/generations is experimental and response_format only supports b64_json."
    docs_url: https://docs.ollama.com/api/openai-compatibility
  - standard: anthropic_compatible
    supported: yes
    base_url: http://localhost:11434
    key_paths:
      - /v1/messages
    auth: none locally; SDK api_key, Authorization bearer, and x-api-key are accepted or ignored locally
    since_version: "v0.14.0"
    deviations:
      - "Anthropic SDK base URL omits /v1 because SDKs append /v1/messages."
      - "Unsupported features include prompt caching, batches, citations, PDF content, and /v1/messages/count_tokens."
      - "tool_choice and metadata are not supported according to the compatibility docs."
      - "Extended thinking budget_tokens may be accepted but is not enforced like Anthropic's service."
    docs_url: https://docs.ollama.com/api/anthropic-compatibility
  - standard: native
    supported: yes
    base_url: http://localhost:11434/api
    key_paths:
      - /api/generate
      - /api/chat
      - /api/embed
      - /api/embeddings
      - /api/tags
      - /api/ps
      - /api/show
      - /api/create
      - /api/copy
      - /api/pull
      - /api/push
      - /api/delete
      - /api/version
    auth: none locally; cloud model access may authenticate through signed-in Ollama credentials
    since_version: "unknown"
    deviations:
      - "Streaming native inference endpoints return newline-delimited JSON by default, not SSE."
      - "Image generation through Ollama is experimental."
    docs_url: https://docs.ollama.com/api

metadata_endpoints:
  - purpose: health
    method: get
    path: /
    gated_by: ""
    auth_gated: false
    response_hint: "Ollama is running"
    notes: No dedicated `/health` endpoint exists; `/health` and `/api/health` returned 404 on this host. Use root GET as the identity and health probe.
  - purpose: version
    method: get
    path: /api/version
    gated_by: ""
    auth_gated: false
    response_hint: '{"version":"0.30.11"}'
    notes: Observed locally on 2026-07-03. Returns the running server version.
  - purpose: model_list
    method: get
    path: /api/tags
    gated_by: ""
    auth_gated: false
    response_hint: '{"models":[{"name":"...","model":"...","details":{"format":"gguf"}}]}'
    notes: Observed locally returning 18 models with GGUF details and capabilities fields.
  - purpose: model_list
    method: get
    path: /v1/models
    gated_by: ""
    auth_gated: false
    response_hint: '{"object":"list","data":[{"id":"...","object":"model"}]}'
    notes: OpenAI-compatible model list observed locally.
  - purpose: loaded_models
    method: get
    path: /api/ps
    gated_by: ""
    auth_gated: false
    response_hint: '{"models":[]}'
    notes: Lists models currently loaded in memory. Observed as an empty models array while idle.
  - purpose: model_info
    method: post
    path: /api/show
    gated_by: ""
    auth_gated: false
    response_hint: '{"license":"...","modelfile":"...","parameters":"...","template":"..."}'
    notes: Body must include `{"model":"<name>"}`. Observed locally with `qwen3:1.7b`.
  - purpose: load_model
    method: post
    path: /api/generate
    gated_by: ""
    auth_gated: false
    response_hint: '{"done":true,"done_reason":"load"}'
    notes: Send an empty request for a model to preload it; `/api/chat` can also preload with empty messages.
  - purpose: unload_model
    method: post
    path: /api/generate
    gated_by: ""
    auth_gated: false
    response_hint: '{"done":true,"done_reason":"unload"}'
    notes: "Send `keep_alive: 0` with `/api/generate` or `/api/chat`; `ollama stop <model>` is the CLI equivalent."
  - purpose: metrics
    method: get
    path: /metrics
    gated_by: ""
    auth_gated: false
    response_hint: "404 page not found"
    notes: Observed 404 locally on 2026-07-03; official API docs do not list a metrics endpoint.

detection:
  - os: macos
    method: binary
    target: ollama
    expect: "ollama version is ..."
    confidence: observed
    notes: Observed at `/usr/local/bin/ollama`, a symlink to `/Applications/Ollama.app/Contents/Resources/ollama`.
  - os: linux
    method: binary
    target: ollama
    expect: "ollama version is ..."
    confidence: documented
    notes: Official manual service uses `/usr/bin/ollama serve`; install-script installs may place it in `/usr/local/bin` or `/usr/bin` depending on layout.
  - os: windows
    method: binary
    target: ollama.exe
    expect: "ollama version is ..."
    confidence: documented
    notes: Windows installer puts binaries under `%LOCALAPPDATA%\\Programs\\Ollama` and adds them to the user PATH.
  - os: macos
    method: app_bundle
    target: /Applications/Ollama.app
    expect: "CFBundleIdentifier: com.electron.ollama"
    confidence: observed
    notes: Observed app bundle version 0.30.11 on this host.
  - os: macos
    method: process
    target: Ollama
    expect: "/Applications/Ollama.app/Contents/MacOS/Ollama"
    confidence: observed
    notes: GUI/menu bar process observed locally.
  - os: all
    method: process
    target: ollama serve
    expect: "ollama serve"
    confidence: observed
    notes: Observed locally as `/Applications/Ollama.app/Contents/Resources/ollama serve`. Loaded models spawn backend child processes such as `llama-server`.
  - os: linux
    method: service
    target: ollama.service
    expect: "ExecStart=/usr/bin/ollama serve"
    confidence: documented
    notes: Official Linux docs recommend a systemd service with user/group `ollama`.
  - os: all
    method: port
    target: "11434"
    expect: ""
    confidence: observed
    notes: Port alone is ambiguous; follow with `GET /` or `/api/version`.
  - os: all
    method: http
    target: GET /
    expect: "Ollama is running"
    confidence: observed
    notes: Strong unauthenticated identity marker observed locally.
  - os: all
    method: http
    target: GET /api/version
    expect: '{"version":"..."}'
    confidence: observed
    notes: Confirms a running Ollama-compatible server and gives exact version.
  - os: all
    method: http
    target: GET /api/tags
    expect: '{"models":[{"details":{"format":"gguf"}}]}'
    confidence: observed
    notes: "Ollama-specific endpoint; `details.format: gguf` is a useful response marker."
  - os: macos
    method: config_file
    target: ~/.ollama/config.json
    expect: '{"integrations":{...}}'
    confidence: observed
    notes: Observed local GUI/CLI state file; not the primary server configuration mechanism.
  - os: all
    method: config_file
    target: ~/.ollama/server.json
    expect: '{"disable_ollama_cloud":true}'
    confidence: documented
    notes: Optional local-only/cloud-disable file; absent on this host.

identity_probes:
  - rank: 1
    request: GET /
    match_in: body
    field: ""
    marker: "Ollama is running"
    uniqueness: unique
    zero_model_ok: true
    auth_gated: false
    confidence: observed
    notes: Plain-text banner observed live on this host (Ollama 0.32.1); no other runner emits it. Works with zero models pulled or loaded.
  - rank: 2
    request: GET /api/version
    match_in: json_field
    field: version
    marker: '{"version":"..."}'
    uniqueness: unique
    zero_model_ok: true
    auth_gated: false
    confidence: observed
    notes: The /api/version path itself is Ollama-only (llama.cpp, LM Studio, vLLM, oMLX, LocalAI do not mount it); observed live returning the exact server version.
  - rank: 3
    request: GET /api/tags
    match_in: json_field
    field: models[].digest
    marker: non-empty sha256 digests with real modified_at values
    uniqueness: strong
    zero_model_ok: false
    confidence: observed
    notes: llama.cpp mimics /api/tags but returns empty digest/modified_at/size fields — real digests confirm Ollama, empty digests are a reverse-tell for llama.cpp.
  - rank: 4
    request: ANY /
    match_in: header
    field: Server
    marker: header absent
    uniqueness: weak
    zero_model_ok: true
    auth_gated: false
    confidence: observed
    notes: Ollama (Go net/http) sends no Server header; corroborating only — absence of Server plus the banner is the combined signal.

version_probe:
  - os: all
    method: cli
    command: ollama --version
    pattern: "ollama version is (\\S+)"
    confidence: observed
    notes: Observed `ollama version is 0.32.1` on this host. The CLI and the running server can drift (the server is a long-lived daemon); for the running server's version use `GET /api/version` (identity_probes rank 2).
  - os: macos
    method: bundle
    command: "defaults read /Applications/Ollama.app/Contents/Info.plist CFBundleShortVersionString"
    pattern: "(\\S+)"
    confidence: observed
    notes: Observed 0.32.1; matches the CLI version on this host. The app bundles the CLI at Contents/Resources/ollama.
  - os: linux
    method: cli
    command: ollama --version
    pattern: "ollama version is (\\S+)"
    confidence: documented
    notes: Same output shape on Linux; install script places the binary in /usr/local/bin or /usr/bin.
  - os: windows
    method: cli
    command: ollama.exe --version
    pattern: "ollama version is (\\S+)"
    confidence: documented
    notes: Installer puts binaries under %LOCALAPPDATA%\\Programs\\Ollama on the user PATH.

config_mechanism: mixed
config_files:
  - os: macos
    path: ~/.ollama/server.json
    format: json
    role: optional server cloud-disable toggle
    notes: "Official FAQ documents `{\"disable_ollama_cloud\": true}`. Absent on this host."
  - os: macos
    path: ~/.ollama/config.json
    format: json
    role: app and launch integration state
    notes: Observed locally with `integrations`, `last_model`, and `last_selection`; not documented as the primary server config.
  - os: linux
    path: /etc/systemd/system/ollama.service.d/override.conf
    format: ini
    role: persisted systemd environment overrides
    notes: Official Linux docs show `systemctl edit ollama` and `Environment="OLLAMA_DEBUG=1"` style overrides.
  - os: linux
    path: /usr/share/ollama/.ollama/server.json
    format: json
    role: optional server cloud-disable toggle for the service user
    notes: Same JSON shape as `~/.ollama/server.json` when the server runs as the `ollama` system user.
  - os: windows
    path: '%HOMEPATH%\.ollama\server.json'
    format: json
    role: optional server cloud-disable toggle
    notes: Same optional cloud-disable file; Windows environment variables are configured through user/system environment settings.

env_vars:
  - name: OLLAMA_HOST
    effect: "Server bind address and port, default `127.0.0.1:11434`; use `0.0.0.0:11434` to expose on a network."
  - name: OLLAMA_MODELS
    effect: "Model store directory override."
  - name: OLLAMA_CONTEXT_LENGTH
    effect: "Default context length unless otherwise specified; current CLI help says default is 4k, 32k, or 256k based on VRAM."
  - name: OLLAMA_KEEP_ALIVE
    effect: "Duration models stay loaded in memory, default `5m`; accepts duration strings, seconds, negative values for indefinitely loaded, and `0` for immediate unload."
  - name: OLLAMA_MAX_LOADED_MODELS
    effect: "Maximum concurrently loaded models per GPU; documented default is 3 times GPU count or 3 for CPU inference."
  - name: OLLAMA_NUM_PARALLEL
    effect: "Maximum parallel requests per loaded model; memory scales with `OLLAMA_NUM_PARALLEL * OLLAMA_CONTEXT_LENGTH`."
  - name: OLLAMA_MAX_QUEUE
    effect: "Maximum queued requests before overload responses; documented default is 512."
  - name: OLLAMA_MAX_TRANSFER_STREAMS
    effect: "Maximum parallel transfer streams for safetensors model pulls and pushes; CLI help default is 4."
  - name: OLLAMA_ORIGINS
    effect: "Comma-separated CORS origins allowed to call the server."
  - name: OLLAMA_NO_CLOUD
    effect: "Disable cloud models, remote inference, and web search."
  - name: OLLAMA_NOPRUNE
    effect: "Do not prune unused model blobs on startup."
  - name: OLLAMA_DEBUG
    effect: "Enable additional debug output."
  - name: OLLAMA_FLASH_ATTENTION
    effect: "Force flash attention on or off where supported."
  - name: OLLAMA_KV_CACHE_TYPE
    effect: "K/V cache quantization type; default `f16`, with quantized cache requiring flash attention."
  - name: OLLAMA_LLM_LIBRARY
    effect: "Override llama.cpp backend/library autodetection."
  - name: OLLAMA_GPU_OVERHEAD
    effect: "Reserve VRAM per GPU in bytes."
  - name: OLLAMA_SCHED_SPREAD
    effect: "Always schedule a model across all available GPUs."
  - name: OLLAMA_IGPU_ENABLE
    effect: "Enable integrated GPUs."
  - name: OLLAMA_LOAD_TIMEOUT
    effect: "How long model loads may stall before failing; CLI help default is `5m`."
  - name: LLAMA_ARG_FIT
    effect: "Enable llama.cpp automatic fit of unset memory options; CLI help default is `on`."
  - name: LLAMA_ARG_FIT_TARGET
    effect: "Target free VRAM margin per device for llama.cpp fit, in MiB."
  - name: HTTPS_PROXY
    effect: "Proxy outbound HTTPS model pulls; official FAQ warns not to set HTTP_PROXY because it can interrupt client connections."
  - name: OLLAMA_API_KEY
    effect: "API key for direct access to the remote ollama.com API, not required for localhost inference."

model_id_grammar: |
  Ollama local and registry model identifiers are `name[:tag]`, with `tag`
  defaulting to `latest`. `name` may be a library model (`llama3.2`, `qwen3`,
  `gemma4`), a namespaced registry model (`namespace/model[:tag]`), or a cloud
  model tag such as `model:cloud` / `model:size-cloud`. Tags commonly encode
  size, architecture variant, instruction tuning, context or mixture shape, and
  quantization, for example `qwen3:1.7b`, `qwen3:30b-a3b-fp16`,
  `qwen3-coder:30b-a3b-q8_0`, and `deepseek-r1:14b-qwen-distill-q8_0`.
  Hugging Face GGUF repositories are accepted as
  `hf.co/{user}/{repo}[:quant_or_filename]` and
  `huggingface.co/{user}/{repo}[:quant_or_filename]`, where the suffix can be a
  quantization tag such as `Q8_0` or a full GGUF filename.

model_formats:
  - gguf

model_acquisition:
  - method: registry
    example: "ollama pull qwen3:1.7b"
    notes: Pulls from the default Ollama registry; `ollama list` and `/api/tags` enumerate local manifests.
  - method: huggingface
    example: "ollama run hf.co/bartowski/Llama-3.2-1B-Instruct-GGUF:Q8_0"
    notes: Direct Hugging Face GGUF references use `hf.co` or `huggingface.co` model-id grammar.
  - method: manual
    example: "Create a Modelfile with `FROM ./model.gguf` or `FROM /path/to/safetensors`, then run `ollama create my-model`."
    notes: Ollama can import GGUF files, GGUF adapters, safetensors adapters, and safetensors model directories for supported architectures; safetensors imports are converted/packed into Ollama's runtime store.
  - method: in_app
    example: "Use the Ollama app or `ollama launch` model selector to pick a local or cloud model."
    notes: The GUI and launch flows ultimately use the same local model store and registry/cloud identities.

model_store_paths:
  - os: macos
    path: ~/.ollama/models
    notes: Official default. Observed on this host as a symlinked `~/.ollama` pointing to `/Volumes/Fast Bastard/models/ollama`.
  - os: linux
    path: /usr/share/ollama/.ollama/models
    notes: Official default when running as the `ollama` service user created by the Linux installer/service instructions.
  - os: windows
    path: 'C:\Users\%username%\.ollama\models'
    notes: Official default; `%HOMEPATH%\.ollama` contains models and configuration.

hardware_acceleration:
  - metal
  - cuda
  - rocm
  - vulkan
  - cpu

concurrency:
  multi_model: true
  parallel_requests: true
  notes: Ollama supports multiple loaded models when memory permits and parallel requests per loaded model. `OLLAMA_MAX_LOADED_MODELS`, `OLLAMA_NUM_PARALLEL`, and `OLLAMA_MAX_QUEUE` control scheduling; Windows Radeon has documented lower default concurrency due to ROCm VRAM reporting limitations.

streaming_sse: true
tool_calling: yes
tool_calling_notes: Native `/api/chat` supports tools; OpenAI-compatible chat/responses and Anthropic-compatible messages expose tool calling for models with tool capability. Local `/api/tags` observed model `capabilities` including `tools`.
embeddings: true
rerank: false
web_ui_url: ""

integration_hooks:
  - command: ollama launch
    effect: "Open the Ollama interactive launch menu for integrations."
    notes: Observed in local CLI help on v0.30.11.
  - command: ollama launch claude
    effect: "Select/configure a model and launch Claude Code through Ollama's Anthropic-compatible API."
    notes: Manual equivalent uses `ANTHROPIC_BASE_URL=http://localhost:11434`, `ANTHROPIC_AUTH_TOKEN=ollama`, and empty `ANTHROPIC_API_KEY`.
  - command: ollama launch opencode
    effect: "Configure and launch OpenCode with Ollama as provider."
    notes: Official OpenCode integration docs also show an OpenCode `@ai-sdk/openai-compatible` provider block.
  - command: ollama launch codex
    effect: "Configure and launch Codex CLI with an Ollama launch profile and model catalog."
    notes: Supports `--config`, `--restore`, `--model`, `--yes`, and passthrough arguments after `--`.
  - command: ollama launch codex-app
    effect: "Configure and launch the Codex desktop app."
    notes: Local CLI help lists aliases `codex-desktop` and `codex-gui`.
  - command: ollama launch droid
    effect: "Configure and launch Droid."
    notes: Listed in local CLI help and launch blog as a supported integration.
  - command: ollama launch qwen
    effect: "Configure and launch Qwen Code."
    notes: Listed in local CLI help.
  - command: ollama launch kimi
    effect: "Configure and launch Kimi Code CLI."
    notes: Listed in local CLI help.
  - command: ollama launch copilot
    effect: "Configure and launch GitHub Copilot CLI."
    notes: Local CLI help lists alias `copilot-cli`.
  - command: ollama launch openclaw
    effect: "Configure and launch OpenClaw."
    notes: Local CLI help lists aliases `clawdbot` and `moltbot`.
  - command: ollama launch hermes
    effect: "Configure and launch Hermes Agent."
    notes: Local CLI help also lists `hermes-desktop`.
  - command: ollama launch vscode
    effect: "Configure VS Code integration."
    notes: Local CLI help lists alias `code`.

traps:
  - "`OLLAMA_HOST` sets both bind address and port; it is not just a host name knob."
  - "Do not identify Ollama by port 11434 alone; confirm `GET /` returns `Ollama is running` or call `/api/version`."
  - "There is no dedicated `/health`, `/api/health`, or `/metrics` endpoint on the observed v0.30.11 local server."
  - "`OLLAMA_CONTEXT_LENGTH` defaults can be far below a model's maximum context; agentic coding integrations often need 64k or higher."
  - "`OLLAMA_NUM_PARALLEL` increases memory use because context allocation scales by parallelism times context length."
  - "`keep_alive: 0` unloads a model immediately; negative keep_alive values keep it loaded indefinitely."
  - "Official FAQ warns to use `HTTPS_PROXY` for model pulls and avoid `HTTP_PROXY`, which may interrupt client connections."
  - "`~/.ollama/config.json` is app/launch state, not a full server config file; server behavior is mostly env vars plus optional `server.json` cloud-disable state."
  - "Local Anthropic API keys are compatibility placeholders; direct `https://ollama.com` cloud API access uses Ollama account/API-key auth."
  - "Ollama has no built-in browser web UI; projects branded as Ollama web UIs are separate applications."

opencode_example: '{"provider":{"ollama":{"npm":"@ai-sdk/openai-compatible","name":"Ollama","options":{"baseURL":"http://localhost:11434/v1"},"models":{"qwen3:1.7b":{"name":"Qwen3 1.7B"}}}}}'

changes:
  - "Updated observed local installation evidence to Ollama v0.30.11, including app bundle identifier `com.electron.ollama`, `/usr/local/bin/ollama` symlink, root health marker, `/api/version`, `/api/tags`, `/api/ps`, `/v1/models`, and `/metrics` 404."
  - "Corrected OpenAI Responses API metadata to cite current docs saying `/v1/responses` was added in v0.13.3 while noting the release notes do not name that endpoint."
  - "Expanded server environment variables from current `ollama serve --help`, including transfer streams, integrated GPU, load timeout, and llama.cpp fit controls."
  - "Expanded `ollama launch` integration hooks from local v0.30.11 help beyond Claude, OpenCode, and Codex."
  - "Updated macOS detection to the observed `com.electron.ollama` bundle identifier and recorded local `~/.ollama/config.json` state plus the symlinked model store."
  - "Recorded observed Anthropic-compatible `/v1/messages` success with `x-api-key` locally and documented local key handling separately from ollama.com cloud authentication."
requires_claudine_update: true
reason: Claudine and sniff should update Ollama detection metadata for the observed macOS bundle identifier, root/API response markers, optional config files, current env-var surface, and expanded `ollama launch` integration commands.
---

# Ollama Local Model Runner

## Introduction to Ollama

[Ollama](https://ollama.com) is an open-source local model runner for macOS,
Linux, and Windows. It downloads and manages GGUF model artifacts, serves
inference over a local HTTP server, and exposes native, OpenAI-compatible, and
Anthropic-compatible APIs. The project is developed in the
[ollama/ollama](https://github.com/ollama/ollama) repository.

| Resource | URL |
| --- | --- |
| Homepage | https://ollama.com |
| Documentation | https://docs.ollama.com |
| API reference | https://docs.ollama.com/api |
| OpenAI compatibility | https://docs.ollama.com/api/openai-compatibility |
| Anthropic compatibility | https://docs.ollama.com/api/anthropic-compatibility |
| Repository | https://github.com/ollama/ollama |
| Releases | https://github.com/ollama/ollama/releases |

## Platforms and Installation

| OS | Support | Binary | Install methods | Process model | Service |
| --- | --- | --- | --- | --- | --- |
| macOS | Native | `ollama` | DMG, Homebrew, install script | Both | Menu bar app/login item or foreground `ollama serve` |
| Linux | Native | `ollama` | Install script, manual `tar.zst`, Docker, distro packages | Both | `ollama.service` under systemd or foreground `ollama serve` |
| Windows | Native | `ollama.exe` | `OllamaSetup.exe`, standalone zip, Docker through WSL2 for GPU | Both | Tray/background app or foreground `ollama serve` |

macOS requires Sonoma 14 or newer. Apple M-series Macs get CPU and GPU support;
x86 Macs are CPU only. Linux installation can create an `ollama` system user and
a systemd service. Windows requires Windows 10 22H2 or newer and installs under
the user's profile without Administrator rights by default.

Observed on this host on 2026-07-03:

- `ollama --version` returned `ollama version is 0.30.11`.
- `/usr/local/bin/ollama` is a symlink to
  `/Applications/Ollama.app/Contents/Resources/ollama`.
- `/Applications/Ollama.app` has bundle identifier `com.electron.ollama`.
- The GUI process and server process were both running:
  `/Applications/Ollama.app/Contents/MacOS/Ollama` and
  `/Applications/Ollama.app/Contents/Resources/ollama serve`.

## API Surface

By default, Ollama binds `127.0.0.1:11434`. The native API base URL is
`http://localhost:11434/api`; OpenAI-compatible clients use
`http://localhost:11434/v1`; Anthropic SDKs use `http://localhost:11434` because
the SDK appends `/v1/messages`.

Local API requests do not require authentication. SDK API-key fields are
compatibility placeholders for localhost. Ollama account or API-key
authentication applies to ollama.com cloud models, private downloads, model
publishing, and direct remote API access.

### Native API

| Method | Path | Purpose |
| --- | --- | --- |
| `POST` | `/api/generate` | Text generation, load, unload |
| `POST` | `/api/chat` | Chat generation, tools, load, unload |
| `POST` | `/api/embed` | Embeddings |
| `POST` | `/api/embeddings` | Legacy embeddings |
| `GET` | `/api/tags` | Local model list |
| `GET` | `/api/ps` | Currently loaded models |
| `POST` | `/api/show` | Model metadata |
| `POST` | `/api/create` | Create/import model |
| `POST` | `/api/copy` | Copy model tag |
| `POST` | `/api/pull` | Pull model |
| `POST` | `/api/push` | Push model |
| `DELETE` | `/api/delete` | Delete model |
| `GET` | `/api/version` | Server version |

Native streaming endpoints return newline-delimited JSON by default.

### OpenAI-Compatible API

Ollama's OpenAI-compatible base URL is `http://localhost:11434/v1`.

| Path | Status | Notes |
| --- | --- | --- |
| `/v1/models` | Supported | Observed locally returning OpenAI-style model objects. |
| `/v1/models/{model}` | Supported | Returns one model's metadata. |
| `/v1/completions` | Supported | Prompt support is narrower than OpenAI; current docs call out string prompt handling. |
| `/v1/chat/completions` | Supported | Supports streaming, tools, vision inputs, JSON mode, logprobs, and related chat fields where model capability permits. |
| `/v1/embeddings` | Supported | Uses Ollama embedding models. |
| `/v1/images/generations` | Experimental | Image models only; `response_format` is limited to `b64_json`. |
| `/v1/responses` | Supported | Current docs say added in v0.13.3; only non-stateful behavior is supported. |

The initial OpenAI compatibility release was v0.1.24 and explicitly named the
Chat Completions API. Current docs document later endpoints, including
non-stateful Responses API support.

### Anthropic-Compatible API

Ollama v0.14.0 release notes name Anthropic API compatibility for
`/v1/messages`. Use base URL `http://localhost:11434` with Anthropic SDKs.

| Path | Status | Notes |
| --- | --- | --- |
| `/v1/messages` | Supported | Observed locally with `x-api-key: test`; docs and blog examples use API key/token placeholders that are ignored locally. |
| `/v1/messages/count_tokens` | Unsupported | Documented unsupported endpoint; detectors should not use it as a health probe. |

Unsupported or incomplete Anthropic features include prompt caching, batches,
citations, PDF content, `tool_choice`, metadata, and count-tokens.

### Metadata Endpoints

| Method | Path | Purpose | Response marker |
| --- | --- | --- | --- |
| `GET` | `/` | Health/identity | `Ollama is running` |
| `GET` | `/api/version` | Version | `{"version":"..."}` |
| `GET` | `/api/tags` | Local model list | `{"models":[...]}` with `details.format: gguf` |
| `GET` | `/v1/models` | OpenAI-style model list | `{"object":"list","data":[...]}` |
| `GET` | `/api/ps` | Loaded models | `{"models":[]}` when idle |
| `POST` | `/api/show` | Model info | `license`, `modelfile`, `parameters`, `template` |
| `POST` | `/api/generate` | Load/unload via empty prompt and `keep_alive` | `done_reason` load/unload markers |

Observed negative probes on v0.30.11: `/health`, `/api/health`, and `/metrics`
all returned `404 page not found`.

## Detection

Recommended ordered probes:

1. Check `ollama` on PATH for macOS/Linux or `ollama.exe` on Windows.
2. On macOS, check `/Applications/Ollama.app` and
   `CFBundleIdentifier = com.electron.ollama`.
3. Check for `ollama serve`; GUI installs may also have an `Ollama` process.
4. Check TCP port `11434`, but do not treat the port alone as identity.
5. Probe `GET /` and require `Ollama is running`.
6. Probe `GET /api/version` for `{"version":"..."}`.
7. Probe `GET /api/tags` and look for Ollama's model-list shape, especially
   `details.format: gguf`.
8. Optionally inspect model/config paths such as `~/.ollama/models`,
   `~/.ollama/config.json`, or optional `~/.ollama/server.json`.

Port 11434 is only a weak signal. The root HTTP marker is the fastest
Ollama-specific discriminator.

### Port identity

Because other servers can occupy 11434 (KoboldCpp is sometimes run there
deliberately), the ranked `identity_probes` frontmatter block is the canonical
strategy for answering "which runner is listening on this port?":

1. `GET /` — the exact plain-text body `Ollama is running` is unique to
   Ollama and needs no loaded model.
2. `GET /api/version` — the path itself is Ollama-only; the body carries the
   exact server version.
3. `GET /api/tags` — real sha256 `digest`/`modified_at` values confirm
   Ollama; llama.cpp mimics this path but returns empty dummy values (a
   reverse-tell).
4. Header check — Ollama (Go) sends no `Server` header; corroborating only.

## Configuration

Ollama server behavior is mostly configured through environment variables. The
optional `~/.ollama/server.json` can disable cloud features:

```json
{
  "disable_ollama_cloud": true
}
```

On macOS, app-launched environment variables should be set with
`launchctl setenv` before restarting the app. On Linux systemd installations,
use `systemctl edit ollama.service` or an override file under
`/etc/systemd/system/ollama.service.d/`. On Windows, set user or system
environment variables and restart the tray app.

Important server variables:

| Variable | Effect |
| --- | --- |
| `OLLAMA_HOST` | Bind address and port, default `127.0.0.1:11434`. |
| `OLLAMA_MODELS` | Model store path. |
| `OLLAMA_CONTEXT_LENGTH` | Default context length. |
| `OLLAMA_KEEP_ALIVE` | Model residency duration. |
| `OLLAMA_MAX_LOADED_MODELS` | Concurrent loaded model cap. |
| `OLLAMA_NUM_PARALLEL` | Parallel requests per loaded model. |
| `OLLAMA_MAX_QUEUE` | Queue limit before overload responses. |
| `OLLAMA_MAX_TRANSFER_STREAMS` | Parallel safetensors transfer streams. |
| `OLLAMA_ORIGINS` | CORS allowlist. |
| `OLLAMA_NO_CLOUD` | Disable cloud models, remote inference, and web search. |
| `OLLAMA_FLASH_ATTENTION` | Force flash attention on/off. |
| `OLLAMA_KV_CACHE_TYPE` | K/V cache quantization type. |
| `OLLAMA_LLM_LIBRARY` | Backend autodetection override. |
| `OLLAMA_GPU_OVERHEAD` | Reserved VRAM per GPU. |
| `OLLAMA_SCHED_SPREAD` | Spread scheduling across GPUs. |
| `OLLAMA_IGPU_ENABLE` | Enable integrated GPUs. |
| `OLLAMA_LOAD_TIMEOUT` | Stalled model-load timeout. |
| `LLAMA_ARG_FIT` | llama.cpp automatic memory-fit behavior. |
| `LLAMA_ARG_FIT_TARGET` | Free VRAM target for llama.cpp fit. |
| `HTTPS_PROXY` | Proxy outbound model pulls. |
| `OLLAMA_API_KEY` | Direct remote ollama.com API key, not localhost auth. |

Traps:

- `OLLAMA_HOST` includes the port.
- `HTTP_PROXY` can break local client connections; use `HTTPS_PROXY` for model
  pulls.
- `~/.ollama/config.json` is app/launch state, not a full server config file.
- Context length and parallelism multiply memory requirements.

## Models

Ollama model IDs use `name[:tag]`, with `latest` as the default tag for registry
models. Names may be library names or namespaced models, and tags often encode
size, architecture variant, context, and quantization:

- `llama3.2`
- `qwen3:1.7b`
- `qwen3:30b-a3b-fp16`
- `qwen3-coder:30b-a3b-q8_0`
- `deepseek-r1:14b-qwen-distill-q8_0`
- `alibayram/medgemma:latest`
- `gpt-oss:120b-cloud`

Hugging Face GGUF repositories are part of the accepted ID grammar:

- `hf.co/{user}/{repo}`
- `hf.co/{user}/{repo}:{quant_or_filename}`
- `huggingface.co/{user}/{repo}:{quant_or_filename}`

Runtime model format is GGUF. Ollama can import GGUF files directly, import
supported safetensors model directories or adapters through a `Modelfile`, and
quantize FP16/FP32 imported models during `ollama create`.

| Acquisition | Example | Notes |
| --- | --- | --- |
| Registry | `ollama pull qwen3:1.7b` | Pulls from Ollama registry. |
| Hugging Face | `ollama run hf.co/bartowski/Llama-3.2-1B-Instruct-GGUF:Q8_0` | Direct GGUF repository reference. |
| Manual | `FROM ./model.gguf` then `ollama create my-model` | Also supports safetensors imports for supported architectures. |
| In app | Choose a model in Ollama app or `ollama launch` | Uses the same store and registry/cloud identities. |

| OS | Default model store |
| --- | --- |
| macOS | `~/.ollama/models` |
| Linux | `/usr/share/ollama/.ollama/models` |
| Windows | `C:\Users\%username%\.ollama\models` |

On this host, `/Users/ken/.ollama` is a symlink to
`/Volumes/Fast Bastard/models/ollama`; the local model store contains registry
manifests and blob-addressed model layers.

## Capabilities

| Capability | Status | Notes |
| --- | --- | --- |
| Hardware acceleration | Metal, CUDA, ROCm, Vulkan, CPU | Platform and GPU dependent. |
| Multi-model serving | Yes | Memory permitting; controlled by `OLLAMA_MAX_LOADED_MODELS`. |
| Parallel requests | Yes | Controlled by `OLLAMA_NUM_PARALLEL`. |
| Streaming | Yes | Native API streams NDJSON; OpenAI and Anthropic compatibility stream SSE. |
| Tool/function calling | Yes | Model-dependent; exposed by native, OpenAI-compatible, and Anthropic-compatible APIs. |
| Embeddings | Yes | `/api/embed`, `/api/embeddings`, `/v1/embeddings`. |
| Reranking | No | No native rerank endpoint found. |
| Web UI | No | No built-in browser UI; Open WebUI and similar tools are separate projects. |
| Image generation | Experimental | Current docs mark OpenAI image generation support experimental. |

## Agentic CLI Integration

OpenCode can use Ollama through the AI SDK OpenAI-compatible adapter:

```json
{
  "provider": {
    "ollama": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "Ollama",
      "options": {
        "baseURL": "http://localhost:11434/v1"
      },
      "models": {
        "qwen3:1.7b": {
          "name": "Qwen3 1.7B"
        }
      }
    }
  }
}
```

Claude Code can use Ollama's Anthropic-compatible endpoint directly:

```bash
export ANTHROPIC_AUTH_TOKEN=ollama
export ANTHROPIC_API_KEY=""
export ANTHROPIC_BASE_URL=http://localhost:11434
claude --model qwen3-coder
```

Runner-native launch hooks observed in `ollama launch --help` on v0.30.11:

```bash
ollama launch
ollama launch claude
ollama launch opencode
ollama launch codex
ollama launch codex-app
ollama launch droid
ollama launch qwen
ollama launch kimi
ollama launch copilot
ollama launch openclaw
ollama launch hermes
ollama launch vscode
```

These commands select or configure a local/cloud model and wire the target tool
to Ollama. Non-interactive launch flows support options such as `--model`,
`--yes`, `--config`, `--restore`, and passthrough arguments after `--`.

## Changelog

- 2026-07-03: Refreshed against Ollama v0.30.11 local observations and current
  official docs. Updated metadata for OpenAI Responses API notes, Anthropic
  v0.14.0 compatibility, macOS bundle identity, optional config files, expanded
  server environment variables, expanded `ollama launch` integrations, and
  negative `/health` and `/metrics` probes.

## Sources

- [Ollama homepage](https://ollama.com)
- [Ollama documentation](https://docs.ollama.com)
- [Ollama GitHub repository](https://github.com/ollama/ollama)
- [Ollama API introduction](https://docs.ollama.com/api/introduction)
- [Ollama authentication docs](https://docs.ollama.com/api/authentication)
- [Ollama OpenAI compatibility docs](https://docs.ollama.com/api/openai-compatibility)
- [Ollama Anthropic compatibility docs](https://docs.ollama.com/api/anthropic-compatibility)
- [Ollama CLI reference](https://docs.ollama.com/cli)
- [Ollama FAQ](https://docs.ollama.com/faq)
- [Ollama macOS docs](https://docs.ollama.com/macos)
- [Ollama Linux docs](https://docs.ollama.com/linux)
- [Ollama Windows docs](https://docs.ollama.com/windows)
- [Ollama importing a model docs](https://docs.ollama.com/import)
- [Ollama OpenCode integration docs](https://docs.ollama.com/integrations/opencode)
- [Ollama Claude Code integration docs](https://docs.ollama.com/integrations/claude-code)
- [Ollama Codex integration docs](https://docs.ollama.com/integrations/codex)
- [Ollama Anthropic compatibility announcement](https://ollama.com/blog/claude)
- [Ollama launch announcement](https://ollama.com/blog/launch)
- [Ollama v0.1.24 release notes](https://github.com/ollama/ollama/releases/tag/v0.1.24)
- [Ollama v0.14.0 release notes](https://github.com/ollama/ollama/releases/tag/v0.14.0)

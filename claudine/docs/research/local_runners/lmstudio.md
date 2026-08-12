---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: codex
model: default

summary: LM Studio is a local LLM runner and server for macOS, Windows, and Linux with a closed-source app/server plus open-source CLI and SDKs, serving GGUF and MLX models through a native REST API plus OpenAI- and Anthropic-compatible endpoints.
homepage: https://lmstudio.ai
docs_url: https://lmstudio.ai/docs
repo_url: https://github.com/lmstudio-ai
api_reference_url: https://lmstudio.ai/docs/developer
open_source: partial

has_official_schema: informal

default_port: 1234
default_bind: 127.0.0.1
auth: optional_api_key
auth_notes: >
  Authentication is disabled by default. When "Require Authentication" is enabled in
  Developer > Server Settings, requests must include a valid API token via
  `Authorization: Bearer <token>` or `x-api-key: <token>`. The Anthropic-compatible
  endpoint also documents `x-api-key`. API tokens are created and permission-scoped in
  the app or via the LM Studio Hub.

platforms:
  - os: macos
    support: native
    binary: lms
    alt_binaries: ["LM Studio.app/Contents/MacOS/LM Studio", "llmster"]
    install: ["DMG from lmstudio.ai/download", "curl -fsSL https://lmstudio.ai/install.sh | bash"]
    process_model: both
    service: macOS app login item / tray; `lms daemon up` for llmster; foreground `lms server start`
    notes: >
      Requires Apple Silicon (M1/M2/M3/M4) and macOS 14.0+. Intel Macs are not
      supported. Observed on this host as `/Applications/LM Studio.app` (v0.4.12+1)
      running `--run-as-service` and `lms` at `/Users/ken/.cache/lm-studio/bin/lms`.
      `lms --version` printed CLI commit `0b2a176`.
  - os: linux
    support: native
    binary: lms
    alt_binaries: ["llmster"]
    install: ["AppImage from lmstudio.ai/download", "curl -fsSL https://lmstudio.ai/install.sh | bash"]
    process_model: both
    service: systemd when configured via headless docs; foreground `lms server start`
    notes: >
      Ubuntu 20.04+ is required; Ubuntu versions newer than 22 are documented as less
      tested. x64 and ARM64 (aarch64) are supported. The install script installs
      `llmster` and `lms` for headless/server use.
  - os: windows
    support: native
    binary: lms.exe
    alt_binaries: ["LM Studio.exe", "llmster.exe"]
    install: ["installer from lmstudio.ai/download", "irm https://lmstudio.ai/install.ps1 | iex"]
    process_model: both
    service: tray app / login item; `lms daemon up` for llmster; foreground `lms server start`
    notes: >
      x64 and ARM64 (Snapdragon X Elite) are supported. AVX2 is required on x64. The
      desktop app can minimize to tray and keep the server running.

api_standards:
  - standard: openai_compatible
    supported: yes
    base_url: http://localhost:1234/v1
    key_paths:
      - /v1/models
      - /v1/chat/completions
      - /v1/completions
      - /v1/embeddings
      - /v1/responses
    auth: optional_api_key
    since_version: "unknown"
    deviations:
      - "Base URL includes /v1; clients append the path after it."
      - "tool_choice, stream_options, and structured output are supported on recent versions."
      - "Remote MCP tools in /v1/responses require enabling per-request MCPs in server settings."
      - "JIT model loading affects whether /v1/models returns downloaded models or only loaded ones."
    docs_url: https://lmstudio.ai/docs/developer/openai-compat
  - standard: anthropic_compatible
    supported: yes
    base_url: http://localhost:1234
    key_paths:
      - /v1/messages
    auth: optional_api_key
    since_version: "v0.4.1"
    deviations:
      - "Anthropic SDKs append /v1/messages themselves, so base_url omits /v1."
      - "Both `Authorization: Bearer <token>` and `x-api-key: <token>` are accepted when auth is enabled."
      - "Claude Code is pointed at LM Studio via ANTHROPIC_BASE_URL=http://localhost:1234 and ANTHROPIC_AUTH_TOKEN=lmstudio."
      - "Stateful Anthropic features (prompt caching, extended thinking, batches, citations) are not supported."
    docs_url: https://lmstudio.ai/docs/developer/anthropic-compat
  - standard: native
    supported: yes
    base_url: http://localhost:1234/api/v1
    key_paths:
      - /api/v1/chat
      - /api/v1/models
      - /api/v1/models/load
      - /api/v1/models/unload
      - /api/v1/models/download
      - /api/v1/models/download/status
    auth: optional_api_key
    since_version: "v0.4.0"
    deviations:
      - "v1 REST API is stateful (response_id / previous_response_id) and supports local MCPs."
      - "Legacy v0 REST API at /api/v0/* still exists for backward compatibility."
      - "Model load/unload/download endpoints require LM Studio 0.4.0+."
    docs_url: https://lmstudio.ai/docs/developer/rest

metadata_endpoints:
  - purpose: model_list
    method: get
    path: /v1/models
    gated_by: ""
    auth_gated: true
    response_hint: '{"object":"list","data":[{"id":"...","object":"model","owned_by":"organization_owner"}]}'
    notes: >
      OpenAI-compatible model list. Observed on this host returning three models
      (qwen3-coder-next, openai/gpt-oss-20b, text-embedding-nomic-embed-text-v1.5).
      Returns downloaded models when JIT loading is on, otherwise only loaded models.
  - purpose: model_list
    method: get
    path: /api/v1/models
    gated_by: ""
    auth_gated: true
    response_hint: '{"models":[{"type":"llm|embedding","publisher":"...","key":"...","format":"gguf|mlx"}]}'
    notes: >
      Native v1 model list with richer metadata (quantization, capabilities,
      loaded_instances, max_context_length). Observed on this host; response root key is
      `models`.
  - purpose: model_list
    method: get
    path: /api/v0/models
    gated_by: ""
    auth_gated: true
    response_hint: '{"object":"list","data":[{"id":"...","object":"model","type":"llm|embedding","compatibility_type":"gguf|mlx"}]}'
    notes: >
      Legacy native v0 model list. Still present for backward compatibility. Returns
      both loaded and downloaded models.
  - purpose: model_info
    method: get
    path: /api/v0/models/{model}
    gated_by: ""
    auth_gated: true
    response_hint: '{"id":"...","object":"model","type":"llm|embedding","state":"not-loaded|loaded"}'
    notes: Returns detailed info for a single model. Part of the legacy v0 API.
  - purpose: loaded_models
    method: get
    path: /api/v1/models
    gated_by: ""
    auth_gated: true
    response_hint: 'loaded_instances array is non-empty for models currently in memory'
    notes: >
      The native v1 model list is the best source for loaded-model status; each entry
      has a `loaded_instances` array.
  - purpose: load_model
    method: post
    path: /api/v1/models/load
    gated_by: ""
    auth_gated: true
    response_hint: '{"instance_id":"..."}'
    notes: Load a model into memory with custom configuration.
  - purpose: unload_model
    method: post
    path: /api/v1/models/unload
    gated_by: ""
    auth_gated: true
    response_hint: '{}'
    notes: Unload one or all models from memory.
  - purpose: other
    method: get
    path: /
    gated_by: ""
    auth_gated: false
    response_hint: '{"error":"Unexpected endpoint or method. (GET /)"}'
    notes: >
      No dedicated health endpoint exists. Observed on this host after starting the
      server with `HOME=/Users/ken lms server start --port 1234`: GET / returns 200 with
      an "Unexpected endpoint or method" error body. The same response is returned for
      /health, so neither is a reliable positive health marker; use /v1/models or
      /api/v1/models instead.
  - purpose: other
    method: get
    path: /openapi.json
    gated_by: ""
    auth_gated: false
    response_hint: '{"error":"Unexpected endpoint or method. (GET /openapi.json)"}'
    notes: >
      Observed on this host with LM Studio 0.4.12+1 returning HTTP 200 with an
      "Unexpected endpoint or method" error body. It does not expose a
      machine-readable OpenAPI schema at this path.
  - purpose: metrics
    method: get
    path: /metrics
    gated_by: ""
    auth_gated: false
    response_hint: '{"error":"Unexpected endpoint or method. (GET /metrics)"}'
    notes: >
      Observed on this host with LM Studio 0.4.12+1 returning HTTP 200 with the standard
      LM Studio "Unexpected endpoint or method" error body. Official API docs do not list
      a metrics endpoint.

detection:
  - os: all
    method: binary
    target: lms
    expect: "CLI prints LM Studio banner and command groups (chat, get, load, server, daemon, runtime)"
    confidence: observed
    notes: >
      Observed at `/Users/ken/.cache/lm-studio/bin/lms` on this host. `lms --version`
      prints a CLI commit hash. The binary is installed alongside the app or llmster,
      and official docs say it only works after LM Studio has been run at least once.
  - os: macos
    method: app_bundle
    target: /Applications/LM Studio.app
    expect: "Bundle exists and contains Contents/MacOS/LM Studio"
    confidence: observed
    notes: >
      Observed on this host. The app can run as a foreground GUI or background service
      with `--run-as-service`.
  - os: macos
    method: process
    target: "LM Studio"
    expect: "Process command line contains --run-as-service"
    confidence: observed
    notes: >
      Observed PID 87280 on this host: `/Applications/LM Studio.app/Contents/MacOS/LM
      Studio --run-as-service`. On Linux/Windows the headless daemon is `llmster`.
  - os: all
    method: process
    target: llmster
    expect: "Background daemon process for headless deployments"
    confidence: documented
    notes: >
      `lms daemon up` starts `llmster` on Linux/Windows/macOS when installed via the
      install script. This host runs the desktop app as a service instead of llmster.
  - os: all
    method: port
    target: "1234"
    expect: "HTTP server responds on localhost:1234"
    confidence: observed
    notes: >
      Default port observed in `~/.cache/lm-studio/.internal/http-server-config.json`
      and confirmed when starting the server on this host.
  - os: all
    method: http
    target: "GET /v1/models"
    expect: '{"object":"list","data":[{"id":"...","object":"model","owned_by":"organization_owner"}]}'
    confidence: observed
    notes: >
      Observed on this host. Port 1234 is shared with other runners (e.g. none known at
      1234 specifically, but always verify the response shape). The `owned_by` value
      `organization_owner` is a strong identity marker. The server was initially stopped
      on this host; the positive probe was observed after starting it with
      `HOME=/Users/ken lms server start --port 1234`.
  - os: all
    method: http
    target: "GET /api/v1/models"
    expect: '{"models":[{"type":"llm|embedding","key":"...","format":"gguf|mlx"}]}'
    confidence: observed
    notes: >
      Observed on this host. The `models` root key and `format`/`capabilities` fields
      disambiguate LM Studio from other OpenAI-compatible servers.
  - os: all
    method: config_file
    target: "~/.lmstudio-home-pointer"
    expect: "File contains the absolute path to the active LM Studio home directory"
    confidence: observed
    notes: >
      Observed on this host pointing to `/Users/ken/.cache/lm-studio`, a
      legacy-migrated home. Official docs show fresh macOS/Linux paths under
      `~/.lmstudio` and Windows paths under `%USERPROFILE%\.lmstudio`; treating the
      pointer as relocating the entire configuration, model store, and CLI install is
      inferred from this host.
  - os: macos
    method: config_file
    target: "~/.cache/lm-studio/.internal/http-server-config.json"
    expect: '{"port":1234,"networkInterface":"127.0.0.1","justInTimeModelLoading":true}'
    confidence: observed
    notes: >
      Actual path on this host because `~/.lmstudio-home-pointer` targets
      `~/.cache/lm-studio`. Tracks the last-used port, bind address, CORS, and JIT
      loading setting.
  - os: macos
    method: service
    target: launchd
    expect: "No dedicated launchd plist; app registers itself as a login item"
    confidence: documented
    notes: >
      The desktop app uses macOS login items. The install script and headless docs show
      how to run `llmster` at boot via launchd or systemd.

identity_probes:
  - rank: 1
    request: GET /api/v0/models
    match_in: json_field
    field: data[].compatibility_type
    marker: '"compatibility_type":"gguf|mlx" with "state":"loaded|not-loaded"'
    uniqueness: unique
    zero_model_ok: true
    auth_gated: true
    confidence: observed
    notes: No other runner exposes /api/v0/* (mistral.rs, which shares port 1234, has only /v1/*); auth is off by default, so the probe is normally ungated. Verified live on this host.
  - rank: 2
    request: GET /api/v1/models
    match_in: json_field
    field: models
    marker: top-level "models" root key (not OpenAI "data")
    uniqueness: unique
    zero_model_ok: true
    auth_gated: true
    confidence: observed
    notes: Newer v1 REST API; the response shape (key, display_name, loaded_instances, quantization.bits_per_weight) matches nothing else.
  - rank: 3
    request: GET /v1/models
    match_in: json_field
    field: data[].owned_by
    marker: '"owned_by":"organization_owner"'
    uniqueness: strong
    zero_model_ok: true
    auth_gated: true
    confidence: observed
    notes: Literal string is an LM Studio fingerprint; vLLM uses "vllm", oMLX uses "omlx", llama.cpp uses "llamacpp". Generic path, unique value.
  - rank: 4
    request: ANY /
    match_in: header
    field: X-Powered-By
    marker: Express
    uniqueness: weak
    zero_model_ok: true
    auth_gated: false
    confidence: observed
    notes: "Every LM Studio response carries `X-Powered-By: Express`; the other fleet runners are Go/Rust/Python (uvicorn). Corroborating only — Express is not exclusive to LM Studio."

version_probe:
  - os: macos
    method: bundle
    command: "defaults read \"/Applications/LM Studio.app/Contents/Info.plist\" CFBundleShortVersionString"
    pattern: "(\\S+)"
    confidence: observed
    notes: Observed `0.4.12+1` on this host. This is the authoritative LM Studio version — the lms CLI cannot report it.
  - os: all
    method: cli
    command: lms --version
    pattern: "CLI commit: ([0-9a-f]+)"
    confidence: observed
    notes: "TRAP: prints the CLI's git commit hash (observed `CLI commit: 0b2a176`), NOT the LM Studio app version — `lms version` prints the same hash under a banner. The lms CLI and the app version on independent tracks; use the app bundle (macOS) or the app's About dialog elsewhere. There is no running-server version endpoint."
  - os: linux
    method: cli
    command: lms --version
    pattern: "CLI commit: ([0-9a-f]+)"
    confidence: documented
    notes: Same commit-hash caveat; headless daemon is llmster. No documented package/bundle channel reporting the app version.
  - os: windows
    method: cli
    command: lms --version
    pattern: "CLI commit: ([0-9a-f]+)"
    confidence: documented
    notes: Same commit-hash caveat. The installed app version is visible in Windows Apps & Features / the installer, not via the CLI.

config_mechanism: mixed

config_files:
  - os: macos
    path: "~/.lmstudio-home-pointer"
    format: text
    role: home directory pointer
    notes: >
      Observed on this legacy-migrated host containing `/Users/ken/.cache/lm-studio`.
      If this file exists, all other LM Studio paths live under the directory it points
      to. Official docs show the fresh-install home as `~/.lmstudio` on macOS/Linux and
      `%USERPROFILE%\.lmstudio` on Windows.
  - os: macos
    path: "{{lmstudio_home}}/.internal/http-server-config.json"
    format: json
    role: server runtime config
    notes: >
      Observed at `~/.cache/lm-studio/.internal/http-server-config.json`. Stores port,
      bind address (`networkInterface`), CORS, JIT loading, and logging settings.
  - os: macos
    path: "{{lmstudio_home}}/settings.json"
    format: json
    role: application settings
    notes: >
      Observed at `~/.cache/lm-studio/settings.json`. Stores UI, chat, developer, and
      download-folder preferences. The `downloadsFolder` key overrides the default
      models directory (observed as `/Volumes/coding/models/lm-studio` on this host).
  - os: linux
    path: "~/.lmstudio-home-pointer"
    format: text
    role: home directory pointer
    notes: >
      Inferred same relocation mechanism as observed on macOS. Official docs show the
      fresh-install home as `~/.lmstudio`.
  - os: linux
    path: "{{lmstudio_home}}/.internal/http-server-config.json"
    format: json
    role: server runtime config
    notes: Server port, bind, CORS, and JIT loading.
  - os: linux
    path: "{{lmstudio_home}}/settings.json"
    format: json
    role: application settings
    notes: App and download preferences.
  - os: windows
    path: '%USERPROFILE%\.lmstudio-home-pointer'
    format: text
    role: home directory pointer
    notes: >
      Inferred equivalent relocation mechanism from the macOS observation. Official docs
      show the fresh-install home as `%USERPROFILE%\.lmstudio`.
  - os: windows
    path: "{{lmstudio_home}}\\.internal\\http-server-config.json"
    format: json
    role: server runtime config
    notes: Server port, bind, CORS, and JIT loading.
  - os: windows
    path: "{{lmstudio_home}}\\settings.json"
    format: json
    role: application settings
    notes: App and download preferences.

env_vars:
  - name: LMS_SERVER_HOST
    effect: "Default bind address for `lms server start` (overridden by --bind)."
  - name: LM_API_TOKEN
    effect: "Example token variable used in docs; actual value is the API token created in Server Settings."
  - name: ANTHROPIC_BASE_URL
    effect: "Set to http://localhost:1234 to point Claude Code at LM Studio."
  - name: ANTHROPIC_AUTH_TOKEN
    effect: "Set to `lmstudio` or the configured API token when Claude Code talks to LM Studio."

model_id_grammar: >
  LM Studio identifies models with `publisher/model` strings such as
  `openai/gpt-oss-20b`, `ibm/granite-4-micro`, `lmstudio-community/qwen2.5-7b-instruct`,
  and `mlx-community/qwen2.5-7b-instruct-4bit`. File-backed imports use
  `publisher/repo/filename.gguf`. `lms get` accepts a quantization suffix using `@`,
  e.g. `llama-3.1-8b@q4_k_m` or `qwen2.5-32b-instruct@q5_k_m`. Models loaded through the
  CLI can be assigned an arbitrary alias with `lms load <id> --identifier=<alias>`,
  which then appears in `/v1/models` under that alias.

model_formats:
  - gguf
  - mlx

model_acquisition:
  - method: registry
    example: "lms get openai/gpt-oss-20b or ibm/granite-4-micro from the LM Studio catalog"
    notes: >
      Models are discovered through the in-app/model catalog at lmstudio.ai/models and
      downloaded from Hugging Face.
  - method: huggingface
    example: "lms get https://huggingface.co/mlx-community/Qwen2.5-7B-Instruct-4bit"
    notes: >
      Full Hugging Face URLs and `owner/repo` strings work in the app search bar and in
      `lms get`.
  - method: manual
    example: "lms import /path/to/model.gguf"
    notes: >
      GGUF files can be imported into the models directory. The expected structure is
      `publisher/model/model-file.gguf`.
  - method: in_app
    example: "Search and download from the Discover tab in LM Studio"
    notes: The GUI downloader stores files in the configured models directory.

model_store_paths:
  - os: macos
    path: "~/.lmstudio/models"
    notes: >
      Canonical fresh-install path shown in import docs. On this legacy-migrated host
      the live path is `~/.cache/lm-studio/models` because `~/.lmstudio-home-pointer`
      redirects the home directory. The user also changed the download folder to
      `/Volumes/coding/models/lm-studio`.
  - os: linux
    path: "~/.lmstudio/models"
    notes: >
      Canonical fresh-install default from official docs; redirecting via
      `~/.lmstudio-home-pointer` is inferred from this host's macOS observation.
  - os: windows
    path: "%USERPROFILE%\\.lmstudio\\models"
    notes: >
      Canonical fresh-install default from official docs; redirecting via
      `%USERPROFILE%\.lmstudio-home-pointer` is inferred from this host's macOS
      observation.

hardware_acceleration:
  - metal
  - cuda
  - rocm
  - vulkan
  - cpu

concurrency:
  multi_model: true
  parallel_requests: true
  notes: >
    Parallel requests with continuous batching were introduced in LM Studio 0.4.0 for
    the llama.cpp engine; MLX support was documented as in development. Max concurrent
    predictions is configured per model load.

streaming_sse: true

tool_calling: conditional
tool_calling_notes: >
  Tool calling works on `/v1/chat/completions` and `/v1/responses`. "Native" tool use
  is supported for models such as Qwen2.5-Instruct, Llama-3.1/3.2, and Mistral; other
  models fall back to a default tool-call format injected via system prompt. The native
  `/api/v1/chat` endpoint supports local MCPs but not custom client-provided tools.

embeddings: true

rerank: false

web_ui_url: ""

integration_hooks: []

traps:
  - "The LM Studio home directory is relocatable via `~/.lmstudio-home-pointer`; this host points it to `~/.cache/lm-studio`, so do not hard-code `~/.lmstudio`."
  - "Fresh installs use `~/.lmstudio` on macOS/Linux and `%USERPROFILE%\\.lmstudio` on Windows; `~/.cache/lm-studio` is this host's legacy-migrated home reached through the pointer file."
  - "The `lms` CLI ships with LM Studio but only works after LM Studio has been run at least once; bootstrap it with `~/.lmstudio/bin/lms bootstrap` on macOS/Linux or `%USERPROFILE%/.lmstudio/bin/lms.exe bootstrap` on Windows."
  - "`GET /` and `GET /health` return HTTP 200 with an `Unexpected endpoint or method` error body; they are not positive health checks."
  - "`lms server start` without `--port` reuses the last-used port (stored in http-server-config.json), not necessarily 1234."
  - "In wrapper sessions with a synthetic HOME, `lms` may look for `.lmstudio-home-pointer` and `.internal/lms-key-2` under the wrapper home. On this host, `HOME=/Users/ken` was required before calling `lms server start`."
  - "`justInTimeModelLoading` changes whether `/v1/models` lists all downloaded models or only loaded ones."
  - "The `/openapi.json` endpoint returns HTTP 200 with an `Unexpected endpoint or method` error body, so it cannot be used as a formal schema source today."

opencode_example: |
  {
    "provider": {
      "lmstudio": {
        "npm": "@ai-sdk/openai-compatible",
        "name": "LM Studio",
        "options": { "baseURL": "http://localhost:1234/v1" },
        "models": {
          "openai/gpt-oss-20b": { "name": "GPT-OSS 20B" },
          "qwen3-coder-next": { "name": "Qwen3 Coder Next" }
        }
      }
    }
  }

changes:
  - "Refreshed metadata on 2026-07-03 with current official docs and local LM Studio 0.4.12+1 observations."
  - "Confirmed the local server was installed but initially stopped; `HOME=/Users/ken lms server start --port 1234` was required in this session because synthetic HOME made `lms` look in `/Users/ken/.claudine/.lmstudio`."
  - "Observed live metadata endpoint shapes for `/v1/models`, `/api/v1/models`, `/api/v0/models`, `/`, `/health`, `/metrics`, `/v1`, and `/openapi.json`."
  - "Added `/metrics` as an observed negative metadata endpoint and clarified that `/` and `/health` are not positive health checks."
  - "Updated platform notes with current system requirement details and expanded hardware backends to include ROCm and Vulkan based on LM Studio release notes."
requires_claudine_update: true
reason: >
  LM Studio is a new local runner entry with distinct detection signals (the `lms`
  binary, `LM Studio.app`/`llmster` process, default port 1234, and the
  `~/.lmstudio-home-pointer` relocation file) that should be added to sniff's
  detection catalog so Claudine can discover and surface it consistently with other
  local runners.
---

# LM Studio

LM Studio is a local model runner with a closed-source desktop application and
headless daemon for running local large language models. It downloads and serves GGUF
(via llama.cpp) and MLX (on Apple Silicon) models and exposes them through a native
REST API plus OpenAI-compatible and Anthropic-compatible endpoints. A command-line
tool, `lms`, ships with the app and with the headless `llmster` daemon.

## Introduction to LM Studio

LM Studio is developed by Element Labs and is available for macOS, Windows, and Linux.
The core product is a GUI application, but the same runtime can be deployed headlessly
as `llmster` for servers and CI. The open-source pieces are the `lms` CLI
([lmstudio-ai/lms](https://github.com/lmstudio-ai/lms), MIT) and the language SDKs
([lmstudio-js](https://github.com/lmstudio-ai/lmstudio-js),
[lmstudio-python](https://github.com/lmstudio-ai/lmstudio-python)); the inference
server and desktop app are closed source.

- Homepage: [https://lmstudio.ai](https://lmstudio.ai)
- Docs: [https://lmstudio.ai/docs](https://lmstudio.ai/docs)
- Developer docs: [https://lmstudio.ai/docs/developer](https://lmstudio.ai/docs/developer)
- Downloads: [https://lmstudio.ai/download](https://lmstudio.ai/download)
- Community: [Discord](https://discord.gg/lmstudio)

## Platforms and Installation

| OS | Support | Binary | Install methods | Process model | Service |
|---|---|---|---|---|---|
| macOS | native | `lms` | DMG, `curl -fsSL https://lmstudio.ai/install.sh \| bash` | both | macOS login item / tray; `lms daemon up` |
| Linux | native | `lms` | AppImage, `curl -fsSL https://lmstudio.ai/install.sh \| bash` | both | systemd (headless docs); `lms daemon up` |
| Windows | native | `lms.exe` | installer, `irm https://lmstudio.ai/install.ps1 \| iex` | both | tray app / login item; `lms daemon up` |

On this host (macOS), `lms` is installed at `~/.cache/lm-studio/bin/lms` and the app is
at `/Applications/LM Studio.app`. This is a legacy-migrated install reached through
`~/.lmstudio-home-pointer`; official docs show fresh installs using `~/.lmstudio` on
macOS/Linux and `%USERPROFILE%\.lmstudio` on Windows. The active process was observed
as `/Applications/LM Studio.app/Contents/MacOS/LM Studio --run-as-service`.

## API Surface

### Default listening address

- Default port: `1234`
- Default bind: `127.0.0.1`
- Override via `lms server start --port <port> --bind <address>` or the
  `LMS_SERVER_HOST` environment variable.

### OpenAI-compatible API

Supported at `http://localhost:1234/v1`:

- `GET /v1/models`
- `POST /v1/chat/completions`
- `POST /v1/completions`
- `POST /v1/embeddings`
- `POST /v1/responses`

Authentication is optional by default; enable "Require Authentication" in Server
Settings to require `Authorization: Bearer <token>`.

### Anthropic-compatible API

Supported since LM Studio 0.4.1 at `http://localhost:1234` (the SDK appends
`/v1/messages`):

- `POST /v1/messages`

Both `x-api-key` and `Authorization: Bearer` are accepted when auth is enabled.

### Native REST API

Released in LM Studio 0.4.0 at `http://localhost:1234/api/v1`:

- `POST /api/v1/chat`
- `GET /api/v1/models`
- `POST /api/v1/models/load`
- `POST /api/v1/models/unload`
- `POST /api/v1/models/download`
- `GET /api/v1/models/download/status`

The legacy v0 API at `/api/v0/*` is still available.

## Detection

A detector should look in this order:

1. `lms` binary on `PATH` or in the LM Studio home `bin/` directory.
2. Running process: `LM Studio --run-as-service` on macOS, `llmster` on Linux/Windows.
3. TCP port `1234` (or the port stored in `http-server-config.json`).
4. HTTP probe `GET /v1/models`; expect `"object":"list"` and
   `"owned_by":"organization_owner"`.
5. HTTP probe `GET /api/v1/models`; expect root key `models` and fields like
   `format`, `capabilities`, `loaded_instances`.
6. Config pointer file `~/.lmstudio-home-pointer` and app bundle
   `/Applications/LM Studio.app` on macOS.

Port `1234` is the LM Studio default; the response shape is the strongest
identity marker because the port can be changed by the user.

### Port identity

Port 1234 collides with mistral.rs, so the ranked `identity_probes`
frontmatter block is the canonical strategy for answering "which runner is
listening on this port?":

1. `GET /api/v0/models` — the `/api/v0/*` namespace is LM-Studio-only, and
   entries carry `compatibility_type` (`gguf`/`mlx`) and `state`; works with
   zero models loaded (auth is off by default).
2. `GET /api/v1/models` — the newer v1 REST API answers with a top-level
   `models` root key rather than OpenAI's `data`.
3. `GET /v1/models` — generic path, but every entry's
   `"owned_by":"organization_owner"` is an LM Studio fingerprint (vLLM says
   `vllm`, oMLX says `omlx`, llama.cpp says `llamacpp`).
4. Header check — `X-Powered-By: Express` on every response; corroborating
   only, since Express is not exclusive to LM Studio.

## Configuration

Configuration is mixed: JSON files inside the LM Studio home directory, CLI flags for
`lms server start`, and GUI toggles. The home directory is relocatable via
`~/.lmstudio-home-pointer`; on this legacy-migrated host it points to
`~/.cache/lm-studio`. Fresh installs use `~/.lmstudio` on macOS/Linux and
`%USERPROFILE%\.lmstudio` on Windows. The `lms` CLI ships with the app but only works
after LM Studio has been run at least once; bootstrap it with
`~/.lmstudio/bin/lms bootstrap` on macOS/Linux or
`%USERPROFILE%/.lmstudio/bin/lms.exe bootstrap` on Windows.

Key files (paths relative to the resolved home directory):

| File | Role |
|---|---|
| `.internal/http-server-config.json` | Port, bind, CORS, JIT loading |
| `settings.json` | App preferences, including `downloadsFolder` override |
| `.internal/model-data.json` | Indexed models and source metadata |

## Operational Traps

The `lms` CLI ships with LM Studio, but official docs say LM Studio must be run at
least once before `lms` works. Bootstrap the CLI with `~/.lmstudio/bin/lms bootstrap`
on macOS/Linux or `cmd /c %USERPROFILE%/.lmstudio/bin/lms.exe bootstrap` on Windows.

`GET /openapi.json`, `GET /metrics`, `GET /`, and `GET /health` are not schema,
metrics, or health endpoints on LM Studio 0.4.12+1 as observed on this host. They
return HTTP 200 with an `Unexpected endpoint or method` error body, so detectors
should use `/v1/models` or `/api/v1/models` and check the response shape.

This session runs with a synthetic `HOME`. `lms server start --port 1234` initially
failed because the CLI looked under `/Users/ken/.claudine/.lmstudio`; setting
`HOME=/Users/ken` made it use `/Users/ken/.lmstudio-home-pointer` and start the
server correctly.

## Models

Model IDs are written as `publisher/model`, for example:

- `openai/gpt-oss-20b`
- `ibm/granite-4-micro`
- `lmstudio-community/qwen2.5-7b-instruct`
- `mlx-community/qwen2.5-7b-instruct-4bit`

`lms get` accepts a quantization suffix: `llama-3.1-8b@q4_k_m`. File imports expect a
`publisher/model/model-file.gguf` directory structure. Models can be pulled from the LM
Studio catalog, Hugging Face, or imported manually.

## Capabilities

| Capability | Support |
|---|---|
| Hardware acceleration | Metal (macOS), CUDA, ROCm, Vulkan, CPU fallback |
| Multi-model serving | Yes |
| Parallel requests | Yes (llama.cpp engine since 0.4.0) |
| SSE streaming | Yes |
| Tool/function calling | Conditional (native for some models, fallback format for others) |
| Embeddings | Yes (`/v1/embeddings`, `/api/v0/embeddings`, `/api/v1/models`) |
| Reranking | No |
| Web UI | No dedicated web UI; the desktop app provides the GUI |

## Agentic CLI Integration

### OpenCode provider block

```json
{
  "provider": {
    "lmstudio": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "LM Studio",
      "options": { "baseURL": "http://localhost:1234/v1" },
      "models": {
        "openai/gpt-oss-20b": { "name": "GPT-OSS 20B" },
        "qwen3-coder-next": { "name": "Qwen3 Coder Next" }
      }
    }
  }
}
```

### Claude Code via Anthropic-compatible endpoint

```bash
export ANTHROPIC_BASE_URL=http://localhost:1234
export ANTHROPIC_AUTH_TOKEN=lmstudio
export CLAUDE_CODE_ATTRIBUTION_HEADER=0
claude --model openai/gpt-oss-20b
```

If "Require Authentication" is enabled, set `ANTHROPIC_AUTH_TOKEN` to the LM Studio
API token.

### Codex via OpenAI-compatible endpoint

```bash
lms server start --port 1234
codex --oss -m openai/gpt-oss-20b
```

LM Studio does not ship a runner-native `lms launch <agent>` command; integration is
done by starting the server and pointing the agent CLI at it.

## Changelog

- 2026-07-03: Refreshed by Codex against current LM Studio docs and local LM Studio
  0.4.12+1 observations. Confirmed `lms` at `/Users/ken/.cache/lm-studio/bin/lms`,
  CLI commit `0b2a176`, `/Applications/LM Studio.app`, pointer file
  `/Users/ken/.lmstudio-home-pointer`, and server config port/bind/JIT settings.
- 2026-07-03: Observed the server was initially stopped; after starting it with
  `HOME=/Users/ken lms server start --port 1234`, confirmed `/v1/models`,
  `/api/v1/models`, and `/api/v0/models` response shapes and stopped the server again.
- 2026-07-03: Added observed negative endpoint notes for `/`, `/health`, `/metrics`,
  `/v1`, and `/openapi.json`; all return LM Studio's `Unexpected endpoint or method`
  response rather than a health, metrics, or schema document.
- 2026-07-03: Updated platform notes from current system requirements and expanded
  hardware backend metadata to include ROCm and Vulkan release-note evidence.

## Sources

- [LM Studio homepage](https://lmstudio.ai)
- [LM Studio docs](https://lmstudio.ai/docs)
- [LM Studio developer docs](https://lmstudio.ai/docs/developer)
- [lms CLI docs](https://lmstudio.ai/docs/cli)
- [lms server start](https://lmstudio.ai/docs/cli/serve/server-start)
- [lms server status](https://lmstudio.ai/docs/cli/serve/server-status)
- [lms daemon up](https://lmstudio.ai/docs/cli/daemon/daemon-up)
- [System requirements](https://lmstudio.ai/docs/app/system-requirements)
- [Download an LLM](https://lmstudio.ai/docs/app/basics/download-model)
- [Import models](https://lmstudio.ai/docs/app/advanced/import-model)
- [Introducing lms](https://lmstudio.ai/blog/lms)
- [OpenAI-compatible endpoints](https://lmstudio.ai/docs/developer/openai-compat)
- [Anthropic-compatible endpoints](https://lmstudio.ai/docs/developer/anthropic-compat)
- [Native REST API](https://lmstudio.ai/docs/developer/rest)
- [REST API v0 endpoints](https://lmstudio.ai/docs/developer/rest/endpoints)
- [Authentication](https://lmstudio.ai/docs/developer/core/authentication)
- [Server settings](https://lmstudio.ai/docs/developer/core/server/settings)
- [Serve on local network](https://lmstudio.ai/docs/developer/core/server/serve-on-network)
- [Headless mode / llmster](https://lmstudio.ai/docs/developer/core/headless)
- [Claude Code integration](https://lmstudio.ai/docs/integrations/claude-code)
- [Codex integration](https://lmstudio.ai/docs/integrations/codex)
- [API changelog](https://lmstudio.ai/docs/developer/api-changelog)
- [LM Studio 0.4.0 release notes](https://lmstudio.ai/blog/0.4.0)
- [LM Studio 0.4.1 changelog](https://lmstudio.ai/changelog/lmstudio-v0.4.1)
- [LM Studio 0.3.19 release notes](https://lmstudio.ai/blog/lmstudio-v0.3.19)
- [LM Studio 0.3.30 release notes](https://lmstudio.ai/blog/lmstudio-v0.3.30)
- [lms CLI on GitHub](https://github.com/lmstudio-ai/lms)

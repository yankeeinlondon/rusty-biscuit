---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
has_official_schema: formal
schema_url: https://raw.githubusercontent.com/openai/codex/main/codex-rs/core/config.schema.json

model_config_paths:
  - scope: user
    path: ~/.codex/config.toml
    format: toml
    notes: Global user defaults. Shared by the CLI and IDE extension. Observed on this host with model = "gpt-5.5", model_reasoning_effort = "medium", personality = "pragmatic", and per-project trust tables.
  - scope: repo
    path: .codex/config.toml
    format: toml
    notes: Project-scoped overrides. Loaded only when the project is trusted. Cannot override provider, auth, telemetry, profile, notification, or host-owned app request metadata keys.
  - scope: env
    path: $CODEX_HOME/profile-name.config.toml
    format: toml
    notes: Profile files selected with --profile profile-name. Overlay between user config and project/CLI overrides. Can also set model_catalog_json per profile.
  - scope: env
    path: /etc/codex/config.toml
    format: toml
    notes: System-wide defaults on Unix. Lowest precedence after built-in defaults.

api_standards:
  - standard: openai_compatible
    base_url_site: openai_base_url (built-in) or [model_providers.<id>.base_url]
    auth_site: OPENAI_API_KEY (built-in) or [model_providers.<id>.env_key]
    adapter: none
    notes: Codex speaks the OpenAI Responses API only. wire_api = "chat" was removed; wire_api = "responses" is the only supported value. Custom providers and all local runners must expose an OpenAI-compatible /v1/responses endpoint.

metadata_overrides:
  - slug
  - display_name
  - description
  - default_reasoning_level
  - supported_reasoning_levels
  - shell_type
  - visibility
  - supported_in_api
  - priority
  - additional_speed_tiers
  - service_tiers
  - default_service_tier
  - availability_nux
  - upgrade
  - base_instructions
  - model_messages
  - include_skills_usage_instructions
  - supports_reasoning_summaries
  - default_reasoning_summary
  - support_verbosity
  - default_verbosity
  - apply_patch_tool_type
  - web_search_tool_type
  - truncation_policy
  - supports_parallel_tool_calls
  - supports_image_detail_original
  - context_window
  - max_context_window
  - auto_compact_token_limit
  - comp_hash
  - effective_context_window_percent
  - experimental_supported_tools
  - input_modalities
  - supports_search_tool
  - use_responses_lite
  - auto_review_model_override
  - tool_mode
  - multi_agent_version

merge_semantics: replace

local_runners:
  - runner: ollama
    integration: first_class
    standard: openai_compatible
    example: |
      # ~/.codex/config.toml
      model = "qwen3:1.7b"
      model_provider = "ollama"
      oss_provider = "ollama"

      # Or interactively
      codex --oss -m qwen3:1.7b

      # Or via Ollama's native hook (v0.15+)
      ollama launch codex
    notes: Built-in ollama provider uses http://localhost:11434/v1 by default. Port is overridable via the experimental CODEX_OSS_PORT or CODEX_OSS_BASE_URL env vars. Model tags (e.g., :1.7b) are passed through unchanged.
  - runner: lmstudio
    integration: first_class
    standard: openai_compatible
    example: |
      # ~/.codex/config.toml
      model = "openai/gpt-oss-20b"
      model_provider = "lmstudio"
      oss_provider = "lmstudio"

      # Or interactively
      codex --oss --local-provider lmstudio -m openai/gpt-oss-20b
    notes: Built-in lmstudio provider uses http://localhost:1234/v1 by default. The lms CLI must be installed and LM Studio must have been run at least once.
  - runner: omlx
    integration: base_url_override
    standard: openai_compatible
    example: |
      # ~/.codex/config.toml
      model = "Qwen3.6-35B-A3B-oQ6"
      model_provider = "local_omlx"

      [model_providers.local_omlx]
      name = "oMLX"
      base_url = "http://localhost:8000/v1"
      # oMLX auth is optional; set env_key if you configured an API key.
    notes: No native Codex provider. oMLX exposes an OpenAI-compatible endpoint at /v1/* on port 8000. oMLX also ships omlx launch codex, which sets OPENAI_BASE_URL/OPENAI_API_KEY and execs Codex.
  - runner: llamacpp
    integration: base_url_override
    standard: openai_compatible
    example: |
      # ~/.codex/config.toml
      model = "gemma-3-1b-it.Q4_K_M.gguf"
      model_provider = "local_llamacpp"

      [model_providers.local_llamacpp]
      name = "llama.cpp"
      base_url = "http://localhost:8080/v1"
    notes: llama-server exposes an OpenAI-compatible endpoint at /v1/* on port 8080 (build b7187+). The request model field is ignored in single-model mode; the server uses whatever model it was started with.
  - runner: vllm
    integration: base_url_override
    standard: openai_compatible
    example: |
      # ~/.codex/config.toml
      model = "Qwen/Qwen2.5-1.5B-Instruct"
      model_provider = "local_vllm"

      [model_providers.local_vllm]
      name = "vLLM"
      base_url = "http://localhost:8000/v1"
    notes: vLLM exposes an OpenAI-compatible endpoint at /v1/* on port 8000. Each vLLM process hosts exactly one model; use --served-model-name aliases if needed.
  - runner: other
    integration: base_url_override
    standard: openai_compatible
    notes: Any local runner that exposes an OpenAI Responses API-compatible /v1/responses endpoint can be wired in as a custom provider. Runners that speak only the Anthropic Messages API require a translating gateway.

cloud_bridge:
  supported: true
  mechanism: openai_base_url or [model_providers.<id>.base_url] pointing at a Responses-API-compatible gateway/proxy
  example: |
    # ~/.codex/config.toml — Mistral's native API is Chat-Completions-shaped and
    # serves no /v1/responses, so a Responses-translating proxy (e.g. LiteLLM)
    # fronts it; base_url points at the proxy, never at api.mistral.ai directly.
    model = "mistral-large-latest"
    model_provider = "mistral"

    [model_providers.mistral]
    name = "Mistral (via LiteLLM proxy)"
    base_url = "http://localhost:4000/v1"
    env_key = "LITELLM_API_KEY"

default_model_site: Top-level model key in ~/.codex/config.toml (or .codex/config.toml); session override via --model/-m or /model in the TUI; CLI flags take precedence over files and env vars.

env_vars:
  - name: OPENAI_API_KEY
    effect: API key used by the built-in openai provider when authenticating with API-key mode.
  - name: CODEX_API_KEY
    effect: Provides an API key for a single non-interactive codex exec run.
  - name: CODEX_ACCESS_TOKEN
    effect: ChatGPT or Codex access token for trusted automation; can be piped to codex login --with-access-token.
  - name: CODEX_CA_CERTIFICATE
    effect: PEM CA bundle for HTTPS/WebSocket clients; takes precedence over SSL_CERT_FILE for TLS-intercepted networks.
  - name: SSL_CERT_FILE
    effect: Fallback PEM CA bundle path when CODEX_CA_CERTIFICATE is unset.
  - name: CODEX_HOME
    effect: Root directory for Codex state, including config.toml, auth, logs, and caches. The directory must already exist.
  - name: CODEX_OSS_BASE_URL
    effect: Experimental override for the base URL of built-in OSS providers (ollama/lmstudio). Takes precedence over the default localhost port.
  - name: CODEX_OSS_PORT
    effect: Experimental override for the localhost port used by built-in OSS providers when CODEX_OSS_BASE_URL is unset.

changes:
  - Confirmed that Codex CLI speaks only the OpenAI Responses API; wire_api = "chat" was removed and the legacy ollama-chat provider ID is no longer supported.
  - LM Studio is now a first-class built-in OSS provider alongside Ollama, surfaced via --oss --local-provider lmstudio or model_provider = "lmstudio" / oss_provider = "lmstudio".
  - Custom providers merge with built-in providers via entry-or-insert, but the reserved IDs openai, ollama, and lmstudio cannot be overridden. Amazon Bedrock only allows aws.profile and aws.region overrides.
  - Documented the experimental CODEX_OSS_BASE_URL and CODEX_OSS_PORT env vars that redirect built-in OSS provider endpoints.
  - Reclassified oMLX, llama.cpp, and vLLM as base-URL-override paths on Codex's OpenAI-compatible client, consistent with local-runner ground truth; they are not unsupported.
  - Updated config scope notes to reflect that project config cannot override provider/auth/telemetry/profile/notification keys and that system config lives at /etc/codex/config.toml on Unix.
  - Corrected the cross-cloud bridging example to route through a Responses-translating proxy (e.g. LiteLLM) — Mistral's native API serves no /v1/responses, so a direct base_url cannot work.

requires_claudine_update: true
reason: Claudine's Codex wrapper and model catalog logic should account for first-class Ollama/LM Studio OSS providers, the experimental CODEX_OSS_BASE_URL/CODEX_OSS_PORT redirect env vars, and the fact that Codex's only user-extension API standard is OpenAI-compatible /v1/responses (not Anthropic). Local runners that speak only Anthropic Messages require a gateway when used with Codex.
---

# Codex CLI User-Side Model Configuration

## Introduction to Codex CLI Model Configuration

Codex CLI (and the IDE extension) read durable configuration from TOML files layered by scope:

| Scope | Path | Format | Who it affects |
| :---- | :--- | :----- | :------------- |
| User | `~/.codex/config.toml` | TOML | You, across all projects |
| Project | `.codex/config.toml` | TOML | Everyone in the repository (only when the project is trusted) |
| Profile | `~/.codex/profile-name.config.toml` | TOML | You, when selected with `--profile profile-name` |
| System | `/etc/codex/config.toml` on Unix | TOML | All users on the machine |

Precedence is: CLI flags and `--config` overrides > project config (closest to CWD wins) > selected profile > user config > system config > built-in defaults.

The host's `~/.codex/config.toml` observed for this research sets `model = "gpt-5.5"`, `model_reasoning_effort = "medium"`, `personality = "pragmatic"`, and per-project trust tables. A separate `~/.codex/config.json` exists but contains only `"model": ""`, while the live model cache is stored in `~/.codex/models_cache.json`.

Codex publishes a formal JSON Schema for `config.toml`: the [`codex-rs/core/config.schema.json`](https://raw.githubusercontent.com/openai/codex/main/codex-rs/core/config.schema.json) file in the OpenAI/codex repository.

## Adding Cloud Models

Codex can talk to any provider that implements the OpenAI Responses API. You add a cloud model by defining a custom provider in `[model_providers.<id>]` and pointing `model_provider` at it.

### Concrete example: adding Mistral

Mistral's native API is Chat-Completions-shaped and does not serve `/v1/responses`,
so it cannot be targeted directly — a Responses-translating proxy (e.g. LiteLLM)
must front it, and `base_url` points at the proxy:

```toml
# ~/.codex/config.toml
model = "mistral-large-latest"
model_provider = "mistral"

[model_providers.mistral]
name = "Mistral (via LiteLLM proxy)"
base_url = "http://localhost:4000/v1"
env_key = "LITELLM_API_KEY"
```

### What each key means

| Key | Effect |
| :-- | :----- |
| `model` | The model slug sent to the provider's API. |
| `model_provider` | Which provider entry to use. Defaults to `openai`. |
| `[model_providers.<id>].name` | Human-readable label. |
| `[model_providers.<id>].base_url` | API base URL. For the built-in OpenAI provider, prefer the top-level `openai_base_url` key. |
| `[model_providers.<id>].env_key` | Environment variable that holds the API key. |
| `[model_providers.<id>].wire_api` | Protocol. `responses` is the only supported value and is the default. `chat` was removed. |
| `[model_providers.<id>].auth` | Optional command-backed bearer-token helper. Do not combine with `env_key`, `experimental_bearer_token`, or `requires_openai_auth`. |
| `[model_providers.<id>].http_headers` | Static extra headers added to every provider request. |
| `[model_providers.<id>].env_http_headers` | Extra headers whose values come from named environment variables. |
| `[model_providers.<id>].query_params` | Extra query parameters appended to every request. |
| `[model_providers.<id>].request_max_retries` | Retry count for HTTP requests (default 4). |
| `[model_providers.<id>].stream_max_retries` | Retry count for SSE stream reconnections (default 5). |
| `[model_providers.<id>].stream_idle_timeout_ms` | Idle timeout for SSE streams in milliseconds (default 300000). |

Reserved provider IDs (`openai`, `ollama`, `lmstudio`) cannot be overridden. The built-in `amazon-bedrock` provider only allows `aws.profile` and `aws.region` overrides.

### Adapter mechanism

There is **no adapter/package mechanism** like an npm ai-sdk key. Codex expects the upstream to speak OpenAI-compatible Responses API endpoints. The provider is responsible for translating requests if its native format differs.

### Per-model metadata

When you use a custom provider, Codex only knows the model string you supply. Rich metadata can be supplied in two ways:

1. **Full catalog replacement** via `model_catalog_json = "/path/to/catalog.json"`. The file must contain a top-level `models` array of records matching the `ModelInfo` type. Recognized fields include `slug`, `display_name`, `description`, `context_window`, `max_context_window`, `supported_reasoning_levels`, `input_modalities`, `supports_reasoning_summaries`, `supports_parallel_tool_calls`, and many others.
2. **Runtime overrides** via top-level keys such as `model_context_window`, `model_reasoning_effort`, `model_reasoning_summary`, `model_supports_reasoning_summaries`, and `model_verbosity`.

### Interaction with the built-in catalog

`model_catalog_json` is a **full replacement** of the bundled catalog for the current process, not a merge. If you rely on a custom provider without replacing the catalog, Codex simply sends the `model` string you provide to that provider's endpoint; the built-in catalog is not consulted for model resolution, but the TUI may still show its own list unless the catalog is replaced.

Custom providers defined in `model_providers` **merge** with the built-in provider list: user-defined entries are added unless they use a reserved ID (`openai`, `ollama`, `lmstudio`). The built-in `amazon-bedrock` provider only accepts `aws.profile` and `aws.region` overrides.

Because the bundled catalog is updated by Codex releases and remote fetches, a manual provider entry or catalog file should be removed once Codex natively supports that model. Codex does not automate this cleanup.

### Cross-cloud bridging

Codex CLI can be routed at a different cloud vendor's API by defining a custom provider whose `base_url` points at an OpenAI-compatible gateway or proxy, or by setting the top-level `openai_base_url` key to redirect the built-in `openai` provider.

```toml
# ~/.codex/config.toml
openai_base_url = "https://openai-gateway.example.com/v1"
model = "gpt-4.1"
```

Or, using a custom provider for a non-OpenAI cloud:

```toml
# ~/.codex/config.toml
model = "anthropic/claude-opus-4"
model_provider = "gateway"

[model_providers.gateway]
name = "OpenAI-compatible gateway"
base_url = "https://gateway.example.com/v1"
env_key = "GATEWAY_API_KEY"
```

The gateway is responsible for translating the OpenAI Responses API to the target vendor's native format.

## Adding Local Models

Local-runner support is a property of **API-standard bridging**, not of Codex CLI "knowing about" a runner. Codex CLI's model client speaks only the OpenAI Responses API. Any runner that exposes an OpenAI-compatible `/v1/responses` endpoint can be used either as a first-class built-in provider or as a custom provider.

| Runner | Integration path | Notes |
| :----- | :--------------- | :---- |
| Ollama | First-class | Built-in `ollama` provider; also supports `ollama launch codex` (v0.15+). |
| LM Studio | First-class | Built-in `lmstudio` provider; `lms` CLI must be bootstrapped. |
| oMLX | Base-URL override | No native provider; wire via custom provider on its OpenAI-compatible endpoint. |
| llama.cpp | Base-URL override | `llama-server` can be configured as a custom provider. |
| vLLM | Base-URL override | `vllm serve` can be configured as a custom provider. |
| Other | Base-URL override or unsupported | Works if the runner speaks the OpenAI Responses API; Anthropic-only runners require a translating gateway. |

### Practical example: Ollama

```toml
# ~/.codex/config.toml
model = "qwen3:1.7b"
model_provider = "ollama"
oss_provider = "ollama"
```

Or interactively:

```bash
codex --oss -m qwen3:1.7b
```

Or via Ollama's native hook:

```bash
ollama launch codex
```

The model id includes the size/quantization tag (`:1.7b`) exactly as Ollama reports it. The built-in provider defaults to `http://localhost:11434/v1`; the experimental `CODEX_OSS_BASE_URL` and `CODEX_OSS_PORT` env vars can redirect it.

### Practical example: LM Studio

```toml
# ~/.codex/config.toml
model = "openai/gpt-oss-20b"
model_provider = "lmstudio"
oss_provider = "lmstudio"
```

Or:

```bash
codex --oss --local-provider lmstudio -m openai/gpt-oss-20b
```

The built-in provider defaults to `http://localhost:1234/v1`.

### Practical example: oMLX

```toml
# ~/.codex/config.toml
model = "Qwen3.6-35B-A3B-oQ6"
model_provider = "local_omlx"

[model_providers.local_omlx]
name = "oMLX"
base_url = "http://localhost:8000/v1"
```

oMLX also ships `omlx launch codex`, which sets the required `OPENAI_BASE_URL`/`OPENAI_API_KEY` env vars and execs Codex.

## Environment Overrides

Environment variables override or sidestep config-file values for the current shell session.

| Variable | Effect |
| :------- | :----- |
| `OPENAI_API_KEY` | API key for the built-in `openai` provider. |
| `CODEX_API_KEY` | API key for a single non-interactive `codex exec` run. |
| `CODEX_ACCESS_TOKEN` | ChatGPT/Codex access token for automation; can be piped to `codex login --with-access-token`. |
| `CODEX_CA_CERTIFICATE` | Custom PEM CA bundle for HTTPS/WebSocket clients. |
| `SSL_CERT_FILE` | Fallback PEM CA bundle path. |
| `CODEX_HOME` | Root directory for all Codex state, including `config.toml`. |
| `CODEX_OSS_BASE_URL` | Experimental override for the base URL of built-in OSS providers. |
| `CODEX_OSS_PORT` | Experimental override for the localhost port of built-in OSS providers. |

For base-URL redirection of the built-in OpenAI provider, use the `openai_base_url` config key. For other providers, use a custom provider's `base_url`. CLI flags (`--model`, `--provider`, `--local-provider`, `--oss`, `-c/--config`) take precedence over both files and environment variables.

## Changelog

- **2026-07-02** — Confirmed Codex CLI speaks only the OpenAI Responses API; `wire_api = "chat"` and the `ollama-chat` provider ID were removed.
- **2026-07-02** — Added LM Studio as a first-class built-in OSS provider alongside Ollama, surfaced through `--oss --local-provider lmstudio` and `model_provider = "lmstudio"`.
- **2026-07-02** — Reclassified oMLX, llama.cpp, and vLLM as OpenAI-compatible base-URL-override paths rather than unsupported; added concrete config examples for each.
- **2026-07-02** — Documented the experimental `CODEX_OSS_BASE_URL` and `CODEX_OSS_PORT` env vars for redirecting built-in OSS providers.
- **2026-07-02** — Clarified provider merge semantics: custom providers extend the built-in provider list, but reserved IDs (`openai`, `ollama`, `lmstudio`) cannot be overridden; `model_catalog_json` remains a full catalog replacement.
- **2026-07-02** — Added system-scope config file (`/etc/codex/config.toml`) and noted project-config restrictions on provider/auth/telemetry/profile/notification keys.
- **2026-07-02** — Corrected the cross-cloud bridging example: Mistral's API is Chat-Completions-shaped and serves no `/v1/responses`, so the bridge must route through a Responses-translating proxy (e.g. LiteLLM) rather than pointing `base_url` at `api.mistral.ai` directly.

## Sources

- [Codex CLI repository](https://github.com/openai/codex)
- [Codex documentation](https://developers.openai.com/codex)
- [Codex config basics](https://developers.openai.com/codex/config-basic)
- [Codex advanced configuration](https://developers.openai.com/codex/config-advanced)
- [Codex configuration reference](https://developers.openai.com/codex/config-reference)
- [Codex environment variables](https://developers.openai.com/codex/environment-variables)
- [Codex models](https://developers.openai.com/codex/models)
- [Codex `config.schema.json`](https://raw.githubusercontent.com/openai/codex/main/codex-rs/core/config.schema.json)
- [Codex discussion on `wire_api = "chat"` removal](https://github.com/openai/codex/discussions/7782)
- [Ollama `ollama launch` blog](https://ollama.com/blog/launch)
- [Ollama OpenAI compatibility](https://docs.ollama.com/api/openai-compatibility)
- [oMLX GitHub repository](https://github.com/jundot/omlx)
- [LM Studio Codex integration](https://lmstudio.ai/docs/integrations/codex)
- [LM Studio OpenAI-compatible endpoints](https://lmstudio.ai/docs/developer/openai-compat)
- [llama.cpp server README](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md)
- [vLLM online serving reference](https://docs.vllm.ai/en/latest/online_serving/)

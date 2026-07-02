---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
has_official_schema: formal
schema_url: https://raw.githubusercontent.com/openai/codex/main/codex-rs/core/config.schema.json

config_files:
  - scope: user
    path: ~/.codex/config.toml
    format: toml
    notes: Global user defaults. The CLI and IDE extension share this file. Observed on this host with `model = "gpt-5.5"`, `model_reasoning_effort = "medium"`, and project-specific trust tables.
  - scope: repo
    path: .codex/config.toml
    format: toml
    notes: Project-scoped overrides. Loaded only when the project is trusted. Cannot override provider, auth, telemetry, or profile keys.
  - scope: env
    path: $CODEX_HOME/profile-name.config.toml
    format: toml
    notes: Profile files selected with `--profile profile-name`. Overlay between user config and project/CLI overrides. Can also set `model_catalog_json` per profile.

api_standards:
  - standard: openai_compatible
    base_url_site: openai_base_url (built-in) or [model_providers.<id>.base_url]
    auth_site: OPENAI_API_KEY (built-in) or [model_providers.<id>.env_key]
    adapter: none
    notes: Codex speaks the OpenAI Responses API by default. Chat Completions support is deprecated and will be removed. Custom providers use `wire_api = "responses"`.

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
    supported: native
    example: |
      # ~/.codex/config.toml
      model = "qwen2.5-coder:14b"
      model_provider = "ollama"
      oss_provider = "ollama"
      # or interactively: codex --oss -m qwen2.5-coder:14b
    notes: First-class built-in provider. Discovers models from Ollama's /api/tags endpoint and passes the returned name strings through unchanged.
  - runner: lmstudio
    supported: native
    example: |
      # ~/.codex/config.toml
      model = "qwen2.5-coder-14b-instruct"
      model_provider = "lmstudio"
      oss_provider = "lmstudio"
      # or interactively: codex --oss --provider lmstudio -m qwen2.5-coder-14b-instruct
    notes: First-class built-in provider. Discovers models from LM Studio's /models endpoint and uses the returned id strings unchanged.
  - runner: omlx
    supported: openai_compatible
    notes: No native provider. If the runner exposes an OpenAI-compatible endpoint, add it as a custom provider with wire_api = "responses".
  - runner: llamacpp
    supported: openai_compatible
    example: |
      # ~/.codex/config.toml
      model = "qwen2.5-coder-14b-instruct"
      model_provider = "local_llamacpp"

      [model_providers.local_llamacpp]
      name = "llama.cpp server"
      base_url = "http://localhost:8080/v1"
      env_key = "OPENAI_API_KEY"
    notes: llama.cpp's built-in server speaks OpenAI-compatible Chat Completions; Responses API support depends on the server version.
  - runner: vllm
    supported: openai_compatible
    example: |
      # ~/.codex/config.toml
      model = "qwen2.5-coder-32b-instruct"
      model_provider = "local_vllm"

      [model_providers.local_vllm]
      name = "vLLM"
      base_url = "http://localhost:8000/v1"
      env_key = "OPENAI_API_KEY"
    notes: vLLM serves an OpenAI-compatible API. Set the model string to whatever the vLLM server was started with.
  - runner: other
    supported: openai_compatible
    notes: Any local runner that implements the OpenAI Responses API (or Chat Completions, while that remains supported) can be wired in as a custom provider.

default_model_site: model key in ~/.codex/config.toml (or .codex/config.toml); session override via --model/-m or /model in the TUI

env_vars:
  - name: OPENAI_API_KEY
    effect: API key used by the built-in openai provider when authenticating with API-key mode.
  - name: CODEX_API_KEY
    effect: Provides an API key for a single non-interactive `codex exec` run.
  - name: CODEX_ACCESS_TOKEN
    effect: ChatGPT or Codex access token for trusted automation; can be piped to `codex login --with-access-token`.
  - name: CODEX_CA_CERTIFICATE
    effect: PEM CA bundle for HTTPS/WebSocket clients; takes precedence over SSL_CERT_FILE for TLS-intercepted networks.
  - name: SSL_CERT_FILE
    effect: Fallback PEM CA bundle path when CODEX_CA_CERTIFICATE is unset.
  - name: CODEX_HOME
    effect: Root directory for Codex state, including config.toml, auth, logs, and caches. The directory must already exist.

changes: []
requires_claudine_update: false
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

The host's `~/.codex/config.toml` observed for this research sets `model = "gpt-5.5"`, `model_reasoning_effort = "medium"`, and `personality = "pragmatic"`, plus per-project trust tables. A separate `~/.codex/config.json` exists but contains only `"model": ""`, while the live model cache is stored in `~/.codex/models_cache.json`.

Codex publishes a formal JSON Schema for `config.toml`: the [`codex-rs/core/config.schema.json`](https://raw.githubusercontent.com/openai/codex/main/codex-rs/core/config.schema.json) file in the OpenAI/codex repository.

## Adding Cloud Models

Codex can talk to any provider that implements the OpenAI Responses API (Chat Completions is still accepted but deprecated). You add a cloud model by defining a custom provider in `[model_providers.<id>]` and pointing `model_provider` at it.

### Concrete example: adding Mistral

```toml
# ~/.codex/config.toml
model = "mistral-large-latest"
model_provider = "mistral"

[model_providers.mistral]
name = "Mistral"
base_url = "https://api.mistral.ai/v1"
env_key = "MISTRAL_API_KEY"
```

### What each key means

| Key | Effect |
| :-- | :----- |
| `model` | The model slug sent to the provider's API. |
| `model_provider` | Which provider entry to use. Defaults to `openai`. |
| `[model_providers.<id>].name` | Human-readable label. |
| `[model_providers.<id>].base_url` | API base URL. For the built-in OpenAI provider, prefer the top-level `openai_base_url` key. |
| `[model_providers.<id>].env_key` | Environment variable that holds the API key. |
| `[model_providers.<id>].wire_api` | Protocol. `responses` is the only supported value and is the default. |
| `[model_providers.<id>].auth` | Optional command-backed bearer-token helper. Do not combine with `env_key`. |

Reserved provider IDs (`openai`, `ollama`, `lmstudio`) cannot be overridden.

### Adapter mechanism

There is **no adapter/package mechanism** like an npm ai-sdk key. Codex expects the upstream to speak OpenAI-compatible endpoints. The provider is responsible for translating requests if its native format differs.

### Per-model metadata

When you use a custom provider, Codex only knows the model string you supply. Rich metadata can be supplied in two ways:

1. **Full catalog replacement** via `model_catalog_json = "/path/to/catalog.json"`. The file must contain a top-level `models` array of records. The recognized fields match the Rust `ModelInfo` type and include `slug`, `display_name`, `description`, `context_window`, `max_context_window`, `supported_reasoning_levels`, `input_modalities`, `supports_reasoning_summaries`, `supports_parallel_tool_calls`, and many others.
2. **Runtime overrides** via top-level keys such as `model_context_window`, `model_reasoning_effort`, `model_supports_reasoning_summaries`, `model_reasoning_summary`, and `model_verbosity`.

### Interaction with the built-in catalog

`model_catalog_json` is a **full replacement** of the bundled catalog for the current process, not a merge. If you rely on a custom provider without replacing the catalog, Codex simply sends the `model` string you provide to that provider's endpoint; the built-in catalog is not consulted for model resolution, but the TUI may still show its own list unless the catalog is replaced.

Because the bundled catalog is updated by Codex releases and remote fetches, a manual provider entry or catalog file should be removed once Codex natively supports that model. Codex does not automate this cleanup.

## Adding Local Models

Codex has built-in native providers for Ollama and LM Studio, both surfaced through `--oss`. Other local runners can be used if they expose an OpenAI-compatible endpoint.

| Runner | Supported | Notes |
| :----- | :-------- | :---- |
| Ollama | Native | Built-in `ollama` provider; pass model tags as returned by `ollama list`. |
| LM Studio | Native | Built-in `lmstudio` provider; pass model ids as returned by the local server. |
| oMLX | OpenAI-compatible | No native provider; wire via custom provider if the runner exposes OpenAI-compatible endpoints. |
| llama.cpp | OpenAI-compatible | llama.cpp's server can be configured as a custom provider. |
| vLLM | OpenAI-compatible | vLLM's server can be configured as a custom provider. |

### Practical example: Ollama

```toml
# ~/.codex/config.toml
model = "qwen2.5-coder:14b"
model_provider = "ollama"
oss_provider = "ollama"
```

Or interactively:

```bash
codex --oss -m qwen2.5-coder:14b
```

The model id includes the size/quantization tag (`:14b`) exactly as Ollama reports it.

### Practical example: LM Studio

```toml
# ~/.codex/config.toml
model = "qwen2.5-coder-14b-instruct"
model_provider = "lmstudio"
oss_provider = "lmstudio"
```

Or:

```bash
codex --oss --provider lmstudio -m qwen2.5-coder-14b-instruct
```

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

For base-URL redirection, use the `openai_base_url` config key (or a custom provider's `base_url`) rather than an environment variable. CLI flags (`--model`, `--provider`, `-c/--config`) take precedence over both files and environment variables.

## Sources

- [Codex CLI repository](https://github.com/openai/codex)
- [Codex documentation](https://developers.openai.com/codex)
- [Codex config basics](https://developers.openai.com/codex/config-basic)
- [Codex advanced configuration](https://developers.openai.com/codex/config-advanced)
- [Codex configuration reference](https://developers.openai.com/codex/config-reference)
- [Codex environment variables](https://developers.openai.com/codex/environment-variables)
- [Codex models](https://developers.openai.com/codex/models)
- [Codex `config.schema.json`](https://raw.githubusercontent.com/openai/codex/main/codex-rs/core/config.schema.json)

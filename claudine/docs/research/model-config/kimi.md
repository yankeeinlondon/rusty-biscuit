---
$schema: ./_schema.yaml
created: "2026-07-02"
last_updated: "2026-07-02"
agent: open_code
model: kimi-for-coding/k2p7
has_official_schema: informal

model_config_paths:
  - scope: user
    path: ~/.kimi/config.toml
    format: toml
    notes: |
      Primary user configuration. Also accepts JSON (`~/.kimi/config.json`), which is auto-migrated to TOML on first run if `config.toml` is absent. Observed on this host with `default_model = "kimi-code/kimi-for-coding"` and managed provider/model entries.
  - scope: env
    path: $KIMI_SHARE_DIR/config.toml
    format: toml
    notes: |
      `KIMI_SHARE_DIR` relocates the entire share directory (default `~/.kimi`), which moves the config file with it.
  - scope: env
    path: --config-file /path/to/config.toml
    format: toml
    notes: |
      CLI flag that replaces the default config file for the launched session. `--config '<json/toml>'` can also supply raw config content inline for one session.

api_standards:
  - standard: openai_compatible
    base_url_site: providers.<name>.base_url (or OPENAI_BASE_URL env var for openai_legacy/openai_responses providers)
    auth_site: providers.<name>.api_key (or OPENAI_API_KEY env var)
    adapter: none
    notes: |
      Use provider type `openai_legacy` for Chat Completions-compatible services or `openai_responses` for the newer Responses API. Custom headers can be added with `custom_headers`.
  - standard: anthropic_compatible
    base_url_site: providers.<name>.base_url
    auth_site: providers.<name>.api_key
    adapter: none
    notes: |
      Use provider type `anthropic` for Anthropic Claude API-compatible endpoints. The base URL should omit `/v1`; the client appends `/v1/messages`.
  - standard: bespoke
    base_url_site: providers.<name>.base_url (or KIMI_BASE_URL env var for kimi providers)
    auth_site: providers.<name>.api_key (or KIMI_API_KEY env var for kimi providers)
    adapter: none
    notes: |
      Use provider type `kimi` for the native Kimi/Moonshot API, `gemini` for Google Gemini, or `vertexai` for Google Vertex AI. OAuth-backed managed providers (e.g. `managed:kimi-code`) store tokens separately under `~/.kimi/credentials`.

metadata_overrides:
  - provider
  - model
  - max_context_size
  - capabilities
  - display_name

merge_semantics: shadow

local_runners:
  - runner: ollama
    integration: base_url_override
    standard: openai_compatible
    example: |
      [providers.ollama-local]
      type = "openai_legacy"
      base_url = "http://localhost:11434/v1"
      api_key = "ollama"

      [models."ollama/qwen3:1.7b"]
      provider = "ollama-local"
      model = "qwen3:1.7b"
      max_context_size = 131072
      capabilities = ["thinking", "image_in", "tool_use"]
      display_name = "Qwen3 1.7B (Ollama)"
    notes: |
      Ollama exposes an OpenAI-compatible Chat Completions endpoint at `/v1/chat/completions`. Pass the Ollama model tag (including size/quantization suffix) as the API `model` value. The same runner also serves an Anthropic-compatible endpoint, so `type = "anthropic"` with `base_url = "http://localhost:11434"` works as well.
  - runner: lmstudio
    integration: base_url_override
    standard: openai_compatible
    example: |
      [providers.lmstudio-local]
      type = "openai_legacy"
      base_url = "http://localhost:1234/v1"
      api_key = "lmstudio"

      [models."lmstudio/openai/gpt-oss-20b"]
      provider = "lmstudio-local"
      model = "openai/gpt-oss-20b"
      max_context_size = 128000
      capabilities = ["thinking", "image_in", "tool_use"]
      display_name = "GPT-OSS 20B (LM Studio)"
    notes: |
      LM Studio's local server is OpenAI-compatible. Use the model id reported by LM Studio's `/v1/models` endpoint. LM Studio also supports the Anthropic Messages API, so `type = "anthropic"` with `base_url = "http://localhost:1234"` is an alternative.
  - runner: vllm
    integration: base_url_override
    standard: openai_compatible
    notes: |
      vLLM serves an OpenAI-compatible API by default. The `model` value is the served model name or a `--served-model-name` alias.
  - runner: llamacpp
    integration: base_url_override
    standard: openai_compatible
    notes: |
      llama-server exposes an OpenAI-compatible `/v1/chat/completions` endpoint. The client-visible model id is the `--alias` value or the GGUF filename.
  - runner: omlx
    integration: base_url_override
    standard: openai_compatible
    notes: |
      oMLX serves an OpenAI-compatible API at `/v1/chat/completions`. Use the model directory name or alias as the API `model` value.
  - runner: other
    integration: base_url_override
    standard: openai_compatible
    notes: |
      Any runner that implements the OpenAI Chat Completions API can be added via `openai_legacy`. Runners exposing the Anthropic Messages API can use `anthropic`.

cloud_bridge:
  supported: true
  mechanism: providers.<name>.base_url (or the OPENAI_BASE_URL env var for OpenAI-type providers) pointing at a compatible gateway or proxy
  example: |
    # Route Kimi Code CLI through a LiteLLM gateway that translates to a non-OpenAI cloud API
    [providers.litellm]
    type = "openai_legacy"
    base_url = "https://litellm.example.com/v1"
    api_key = "sk-litellm"

    [models."gateway/claude-sonnet"]
    provider = "litellm"
    model = "claude-3-7-sonnet-20250219"
    max_context_size = 200000
    capabilities = ["thinking", "image_in"]

default_model_site: |
  Top-level `default_model` key in `~/.kimi/config.toml`. Must reference a key declared in the `[models]` table. Session override via `--model <name>`; for kimi providers `KIMI_MODEL_NAME` overrides the API model identifier of the active model.

env_vars:
  - name: KIMI_BASE_URL
    effect: Overrides the `base_url` of a `kimi` type provider.
  - name: KIMI_API_KEY
    effect: Overrides the `api_key` of a `kimi` type provider.
  - name: KIMI_MODEL_NAME
    effect: Overrides the API `model` identifier for the active kimi provider model.
  - name: KIMI_MODEL_MAX_CONTEXT_SIZE
    effect: Overrides `max_context_size` for the active kimi provider model.
  - name: KIMI_MODEL_CAPABILITIES
    effect: Comma-separated override of `capabilities` for the active kimi provider model (e.g. `thinking,image_in,tool_use`).
  - name: KIMI_MODEL_TEMPERATURE
    effect: Sets the generation `temperature` parameter for kimi providers.
  - name: KIMI_MODEL_TOP_P
    effect: Sets the generation `top_p` parameter for kimi providers.
  - name: KIMI_MODEL_MAX_TOKENS
    effect: Sets the generation `max_tokens` parameter for kimi providers.
  - name: KIMI_MODEL_THINKING_KEEP
    effect: Forwards `thinking.keep` verbatim to Moonshot thinking models when thinking mode is active.
  - name: OPENAI_BASE_URL
    effect: Overrides the `base_url` of `openai_legacy`/`openai_responses` type providers.
  - name: OPENAI_API_KEY
    effect: Overrides the `api_key` of `openai_legacy`/`openai_responses` type providers.
  - name: KIMI_SHARE_DIR
    effect: Changes the share directory from `~/.kimi` to the specified path, moving config, sessions, logs, and credentials.

changes:
  - "Corrected config override precedence to env vars > CLI flags > config file (previously listed CLI flags above env vars)."
  - "Added `tool_use` to the observed capability set alongside `thinking`, `always_thinking`, `image_in`, and `video_in`."
  - "Documented `gemini` and `vertexai` as additional bespoke provider types for user-added cloud models."
  - "Clarified that local runners work by base-URL override on either `openai_legacy` or `anthropic` provider types; none have first-class Kimi integration hooks."
  - "Updated managed-provider notes: `/login` creates `managed:kimi-code` and `managed:moonshot-ai` entries and stores OAuth tokens under `~/.kimi/credentials`."

requires_claudine_update: true
reason: "Claudine's Kimi provider wrapper and model_catalog should reflect the corrected env/CLI/config precedence, the full KIMI_* and OPENAI_* override variable set, the gemini/vertexai provider types, and the base-URL-override local-runner integration paths for Ollama, oMLX, LM Studio, llama.cpp, and vLLM."
---

# Kimi Code CLI User-Side Model Configuration

## Introduction to Kimi Code CLI Model Configuration

Kimi Code CLI keeps model and provider configuration in a single TOML (or legacy JSON) file inside its share directory. The default location is `~/.kimi/config.toml`. On first run the CLI creates this file if it does not exist, and it will automatically migrate a legacy `config.json` to TOML.

| Scope | Path | Format | Notes |
| :---- | :--- | :----- | :---- |
| User | `~/.kimi/config.toml` | TOML or JSON | Primary configuration. Model and provider tables are defined here. Observed on this host with `default_model = "kimi-code/kimi-for-coding"` and managed provider entries. |
| Env-relocated | `$KIMI_SHARE_DIR/config.toml` | TOML | `KIMI_SHARE_DIR` moves the entire application data directory. |
| CLI override | `--config-file <path>` | TOML/JSON | Replaces the default config file for one session. |
| CLI inline | `--config '<json/toml>'` | TOML/JSON | Supplies config content inline for one session. Cannot be combined with `--config-file`. |

There is no published JSON Schema for the configuration file. The authoritative reference is the prose documentation plus the table definitions below, so `has_official_schema` is **informal**.

The top-level `default_model` key pins the default model. It must be a key declared in the `[models]` table, not a raw API model string.

```toml
default_model = "kimi-code/kimi-for-coding"
```

## Adding Cloud Models

Kimi Code CLI supports adding cloud models by declaring a new provider in `[providers]` and at least one model entry in `[models]` that references it. The CLI speaks several provider protocols, so the correct provider `type` depends on the upstream API.

### Concrete example: adding a custom OpenAI-compatible cloud model

```toml
[providers.my-openai-proxy]
type = "openai_legacy"
base_url = "https://gateway.example.com/v1"
api_key = "sk-gateway-key"
custom_headers = { "X-Route-To" = "custom-model" }

[models."my-org/custom-model"]
provider = "my-openai-proxy"
model = "custom-model"
max_context_size = 128000
capabilities = ["thinking", "image_in", "tool_use"]
display_name = "Custom Cloud Model"
```

After adding this block, set `default_model = "my-org/custom-model"` or select it at runtime with `--model "my-org/custom-model"`.

### What each key means

| Config key | Meaning |
| :--------- | :------ |
| `providers.<name>.type` | Wire protocol: `kimi`, `openai_legacy`, `openai_responses`, `anthropic`, `gemini`, or `vertexai`. |
| `providers.<name>.base_url` | API endpoint root. For `anthropic` the base URL should omit `/v1`. |
| `providers.<name>.api_key` | Static API key. OAuth-backed managed providers can leave this empty and use an `[providers.<name>.oauth]` block. |
| `providers.<name>.custom_headers` | Optional extra HTTP headers attached to every request. |
| `providers.<name>.env` | Optional environment variables to set before constructing the provider instance (used for Vertex AI). |
| `models.<key>.provider` | Reference to a key in `[providers]`. |
| `models.<key>.model` | The actual model identifier sent in API calls. |
| `models.<key>.max_context_size` | Context window in tokens; drives compaction behavior. |
| `models.<key>.capabilities` | Feature flags such as `thinking`, `always_thinking`, `image_in`, `video_in`, and `tool_use`. |
| `models.<key>.display_name` | Human-readable name shown in the welcome panel, status bar, and `/model` picker. |

If a provider or model key contains `.`, quote it in TOML (for example `[models."gpt-4.1"]`).

### Supported API standards

| Standard | Provider type | Base URL location | Auth location | Adapter |
| :------- | :------------ | :---------------- | :------------ | :------ |
| OpenAI Chat Completions | `openai_legacy` | `providers.<name>.base_url` or `OPENAI_BASE_URL` | `providers.<name>.api_key` or `OPENAI_API_KEY` | none |
| OpenAI Responses API | `openai_responses` | `providers.<name>.base_url` or `OPENAI_BASE_URL` | `providers.<name>.api_key` or `OPENAI_API_KEY` | none |
| Anthropic Messages API | `anthropic` | `providers.<name>.base_url` | `providers.<name>.api_key` | none |
| Native Kimi API | `kimi` | `providers.<name>.base_url` or `KIMI_BASE_URL` | `providers.<name>.api_key` or `KIMI_API_KEY` | none |
| Google Gemini API | `gemini` | `providers.<name>.base_url` | `providers.<name>.api_key` | none |
| Google Vertex AI | `vertexai` | `providers.<name>.base_url` | `providers.<name>.api_key` | none |

There is no package-based adapter mechanism such as an npm ai-sdk key. The translation is handled by selecting the correct `type` and, if necessary, pointing `base_url` at a gateway that normalizes the upstream protocol.

### Per-model metadata

The user can declare the following metadata when adding a model:

- `provider` — which provider block to use.
- `model` — the upstream API model identifier.
- `max_context_size` — required; used for compaction and context accounting.
- `capabilities` — optional; controls whether thinking mode, image input, video input, and tool use are advertised.
- `display_name` — optional; shown in the UI. For OAuth-managed providers this can be refreshed from the provider's `/models` endpoint at startup.

Cost fields, output token limits, and explicit reasoning-budget fields are not part of the documented model schema.

### Interaction with the built-in catalog

Kimi Code CLI does not ship a separate self-updating model catalog. The `[models]` table in the config file is the source of available models. Managed providers such as `managed:kimi-code` and `managed:moonshot-ai` are pre-integrated, but they still require entries in `[models]` (normally created by `/login` or `kimi login`). OAuth tokens for managed providers are stored under `~/.kimi/credentials` rather than inline in the config.

Because the config file is the catalog, a manually authored model entry with the same key as a managed/default entry **shadows** the managed metadata. Different keys simply coexist. Best practice is to remove or rename a manual block once a model is natively supported through `/login`, because the managed path will handle OAuth refresh and display-name updates automatically. The CLI does not warn about duplicate keys, so cleanup is manual.

### Cross-cloud bridging

Kimi Code CLI can be routed at a different cloud vendor's API by declaring a provider whose `base_url` points at a gateway or proxy that speaks one of the supported standards. The mechanism is the same `providers.<name>.base_url` key (or the `OPENAI_BASE_URL` environment variable for OpenAI-type providers; Anthropic-type providers are configured only via `providers.<name>.base_url` — env-var overrides are not supported for them).

If the target vendor's native API does not match one of the standards Kimi Code CLI speaks, place a translating proxy such as [LiteLLM](https://github.com/BerriAI/litellm) between the CLI and the vendor.

OpenAI-compatible gateway example:

```toml
[providers.litellm]
type = "openai_legacy"
base_url = "https://litellm.example.com/v1"
api_key = "sk-litellm"

[models."gateway/claude-sonnet"]
provider = "litellm"
model = "claude-3-7-sonnet-20250219"
max_context_size = 200000
capabilities = ["thinking", "image_in"]
```

Anthropic-compatible gateway example:

```toml
[providers.anthropic-gateway]
type = "anthropic"
base_url = "https://gateway.example.com"
api_key = "sk-gateway"

[models."gateway/gpt-4.1"]
provider = "anthropic-gateway"
model = "gpt-4.1"
max_context_size = 1047576
capabilities = ["thinking", "image_in", "tool_use"]
```

## Adding Local Models

Local-runner support is a property of **API-standard bridging**, not of Kimi Code CLI "knowing about" a runner. Most runners expose an OpenAI-compatible endpoint, and several also expose an Anthropic-compatible one, so any provider that allows a base-URL override can use them.

Kimi Code CLI has no first-class local-runner integration hooks (no `kimi launch ollama` equivalent). Local models are added by pointing an `openai_legacy`, `openai_responses`, or `anthropic` provider at the runner's local endpoint.

| Runner | Integration path | Standard used | Notes |
| :----- | :--------------- | :------------ | :---- |
| Ollama | Base-URL override | OpenAI-compatible | Native `/v1/chat/completions` at `http://localhost:11434/v1`. Also supports Anthropic Messages at `http://localhost:11434`. |
| LM Studio | Base-URL override | OpenAI-compatible | Local server at `http://localhost:1234/v1`. Also supports Anthropic Messages at `http://localhost:1234`. |
| vLLM | Base-URL override | OpenAI-compatible | Serves OpenAI-compatible API at `http://localhost:8000/v1` by default. |
| llama.cpp | Base-URL override | OpenAI-compatible | `llama-server` exposes `/v1/chat/completions` at `http://localhost:8080/v1`. |
| oMLX | Base-URL override | OpenAI-compatible | Serves OpenAI-compatible API at `http://localhost:8000/v1`. |

### Practical example: Ollama

```toml
[providers.ollama-local]
type = "openai_legacy"
base_url = "http://localhost:11434/v1"
api_key = "ollama"

[models."ollama/qwen3:1.7b"]
provider = "ollama-local"
model = "qwen3:1.7b"
max_context_size = 131072
capabilities = ["thinking", "image_in", "tool_use"]
display_name = "Qwen3 1.7B (Ollama)"
```

The `model` value is the exact Ollama tag, including the size/quantization suffix (`:1.7b`). The config key (`ollama/qwen3:1.7b`) is what you pass to `--model`.

### Practical example: LM Studio

```toml
[providers.lmstudio-local]
type = "openai_legacy"
base_url = "http://localhost:1234/v1"
api_key = "lmstudio"

[models."lmstudio/openai/gpt-oss-20b"]
provider = "lmstudio-local"
model = "openai/gpt-oss-20b"
max_context_size = 128000
capabilities = ["thinking", "image_in", "tool_use"]
display_name = "GPT-OSS 20B (LM Studio)"
```

Use the model id reported by LM Studio's `/v1/models` endpoint. If LM Studio has authentication enabled, set `api_key` to the configured API token instead of `lmstudio`.

### Model identifiers for local runners

- **Ollama**: `name[:tag]`, for example `qwen3:1.7b` or `llama3.2:70b`.
- **LM Studio**: `publisher/model`, for example `openai/gpt-oss-20b`.
- **vLLM**: HuggingFace model id, local path, or a `--served-model-name` alias, for example `Qwen/Qwen2.5-1.5B-Instruct`.
- **llama.cpp**: the `--alias` value or the GGUF filename, for example `gemma-3-1b-it.Q4_K_M.gguf`.
- **oMLX**: the model directory name or alias, for example `Qwen3.6-35B-A3B-oQ6`.

## Environment Overrides

Environment variables override the corresponding config-file fields for the launched session. The override behavior is provider-type specific.

For `kimi` type providers:

| Variable | Overrides |
| :------- | :-------- |
| `KIMI_BASE_URL` | `providers.<name>.base_url` |
| `KIMI_API_KEY` | `providers.<name>.api_key` |
| `KIMI_MODEL_NAME` | `models.<key>.model` |
| `KIMI_MODEL_MAX_CONTEXT_SIZE` | `models.<key>.max_context_size` |
| `KIMI_MODEL_CAPABILITIES` | `models.<key>.capabilities` |
| `KIMI_MODEL_TEMPERATURE` | Generation `temperature` |
| `KIMI_MODEL_TOP_P` | Generation `top_p` |
| `KIMI_MODEL_MAX_TOKENS` | Generation `max_tokens` |
| `KIMI_MODEL_THINKING_KEEP` | `thinking.keep` for supported Moonshot thinking models |

For `openai_legacy` and `openai_responses` providers:

| Variable | Overrides |
| :------- | :-------- |
| `OPENAI_BASE_URL` | `providers.<name>.base_url` |
| `OPENAI_API_KEY` | `providers.<name>.api_key` |

Other relevant variables:

| Variable | Effect |
| :------- | :----- |
| `KIMI_SHARE_DIR` | Changes the share directory from `~/.kimi` to the given path. |

### Precedence

From highest to lowest:

1. Environment variables (`KIMI_*`, `OPENAI_*`).
2. CLI flags (`--model`, `--config`, `--config-file`).
3. Configuration file (`~/.kimi/config.toml`).

Environment variable overrides are only supported for `kimi`, `openai_legacy`, and `openai_responses` provider types.

## Changelog

- **2026-07-02** — Corrected override precedence to env vars > CLI flags > config file.
- **2026-07-02** — Added `tool_use` to the observed capability set and documented `gemini`/`vertexai` provider types.
- **2026-07-02** — Reclassified all researched local runners (Ollama, oMLX, LM Studio, llama.cpp, vLLM) as base-URL-override paths on `openai_legacy`/`anthropic` providers; none have first-class Kimi integration hooks.
- **2026-07-02** — Updated managed-provider notes to reflect OAuth credential storage under `~/.kimi/credentials`.

## Sources

- [Kimi Code CLI — Providers and Models](https://moonshotai.github.io/kimi-cli/en/configuration/providers.md)
- [Kimi Code CLI — Config Files](https://moonshotai.github.io/kimi-cli/en/configuration/config-files.md)
- [Kimi Code CLI — Environment Variables](https://moonshotai.github.io/kimi-cli/en/configuration/env-vars.md)
- [Kimi Code CLI — Config Overrides](https://moonshotai.github.io/kimi-cli/en/configuration/overrides.md)
- [Kimi Code CLI — `kimi` Command Reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.md)
- [Kimi Code CLI — Wire Mode](https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.md)
- [Kimi Code CLI — Plugins](https://moonshotai.github.io/kimi-cli/en/customization/plugins.md)
- [Kimi CLI GitHub Repository](https://github.com/MoonshotAI/kimi-cli)

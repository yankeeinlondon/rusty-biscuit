---
$schema: ./_schema.yaml
created: "2026-07-02"
last_updated: "2026-07-02"
agent: open_code
model: kimi-for-coding/k2p7
has_official_schema: informal

config_files:
  - scope: user
    path: ~/.kimi/config.toml
    format: toml
    notes: |
      Primary user configuration. Also accepts JSON (`~/.kimi/config.json`), which is auto-migrated to TOML on first run if `config.toml` is absent. The host file declares a top-level `default_model` and a `[models]` table with explicit provider/model mappings. Use quoted TOML keys when the model or provider name contains `.`.
  - scope: env
    path: $KIMI_SHARE_DIR/config.toml
    format: toml
    notes: |
      `KIMI_SHARE_DIR` relocates the entire share directory (default `~/.kimi`), which moves the config file with it. Useful for isolated environments or CI.
  - scope: env
    path: --config-file /path/to/config.toml
    format: toml
    notes: |
      CLI flag that replaces the default config file for the launched session. Can also pass raw config content with `--config '{"default_model": ...}'`.

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
      Use provider type `anthropic` for Anthropic Claude API-compatible endpoints.
  - standard: bespoke
    base_url_site: providers.<name>.base_url (or KIMI_BASE_URL env var for kimi providers)
    auth_site: providers.<name>.api_key (or KIMI_API_KEY env var)
    adapter: none
    notes: |
      Use provider type `kimi` for the native Kimi/Moonshot API. OAuth-backed managed providers (e.g. `managed:kimi-code`) store tokens separately under `~/.kimi/credentials`/`~/.kimi/oauth`.

metadata_overrides:
  - provider
  - model
  - max_context_size
  - capabilities
  - display_name

merge_semantics: shadow

local_runners:
  - runner: ollama
    supported: openai_compatible
    example: |
      [providers.ollama-local]
      type = "openai_legacy"
      base_url = "http://localhost:11434/v1"
      api_key = "ollama"

      [models."ollama/gemma3:27b"]
      provider = "ollama-local"
      model = "gemma3:27b"
      max_context_size = 131072
      capabilities = ["thinking", "image_in"]
      display_name = "Gemma 3 27B (Ollama)"
    notes: |
      Ollama exposes an OpenAI-compatible Chat Completions endpoint at `/v1/chat/completions`. Pass the Ollama model tag (including size/quantization suffix) as the API `model` value.
  - runner: lmstudio
    supported: openai_compatible
    example: |
      [providers.lmstudio-local]
      type = "openai_legacy"
      base_url = "http://localhost:1234/v1"
      api_key = "lm-studio"

      [models."lmstudio/qwen2.5-coder-32b-instruct"]
      provider = "lmstudio-local"
      model = "qwen2.5-coder-32b-instruct"
      max_context_size = 32768
      capabilities = ["thinking", "image_in"]
    notes: |
      LM Studio's local server is OpenAI-compatible. Use the model id reported by LM Studio's `/models` endpoint.
  - runner: vllm
    supported: openai_compatible
    example: |
      [providers.vllm-local]
      type = "openai_legacy"
      base_url = "http://localhost:8000/v1"
      api_key = "vllm-token"

      [models."vllm/qwen2.5-coder-32b-instruct"]
      provider = "vllm-local"
      model = "qwen2.5-coder-32b-instruct"
      max_context_size = 32768
      capabilities = ["thinking", "image_in"]
    notes: |
      vLLM serves an OpenAI-compatible API by default. The `model` value is the served model name.
  - runner: llamacpp
    supported: openai_compatible
    example: |
      [providers.llamacpp-local]
      type = "openai_legacy"
      base_url = "http://localhost:8080/v1"
      api_key = "llama-cpp"

      [models."llamacpp/qwen2.5-coder-14b"]
      provider = "llamacpp-local"
      model = "qwen2.5-coder-14b"
      max_context_size = 32768
      capabilities = ["thinking"]
    notes: |
      llama.cpp server (`./server`) exposes an OpenAI-compatible `/v1/chat/completions` endpoint.
  - runner: omlx
    supported: openai_compatible
    notes: |
      No native `mlx` provider. If served via an OpenAI-compatible shim (for example `mlx-lm.server` or a wrapping gateway), configure it as `openai_legacy` with the local base URL.
  - runner: other
    supported: openai_compatible
    notes: |
      Any runner that implements the OpenAI Chat Completions API can be added via `openai_legacy`. Runners exposing the Anthropic Messages API can use `anthropic`.

default_model_site: |
  Top-level `default_model` key in `~/.kimi/config.toml`. Must reference a key declared in the `[models]` table. Session override via `--model <name>`; env override via `KIMI_MODEL_NAME` for kimi providers.

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
    effect: Comma-separated override of `capabilities` for the active kimi provider model (e.g. `thinking,image_in`).
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
  - name: KIMI_CLI_NO_AUTO_UPDATE
    effect: Disables auto-update checks when set to `1`, `true`, `t`, `yes`, or `y`.

changes: []
requires_claudine_update: false
---

# Kimi Code CLI User-Side Model Configuration

## Introduction to Kimi Code CLI Model Configuration

Kimi Code CLI keeps model and provider configuration in a single TOML (or legacy JSON) file inside its share directory. The default location is `~/.kimi/config.toml`. On first run the CLI creates this file if it does not exist, and it will automatically migrate a legacy `config.json` to TOML.

| Scope | Path | Format | Notes |
| :---- | :--- | :----- | :---- |
| User | `~/.kimi/config.toml` | TOML or JSON | Primary configuration. Model and provider tables are defined here. Observed on this host with `default_model = "kimi-code/kimi-for-coding"` and several `[models]` entries. |
| Env-relocated | `$KIMI_SHARE_DIR/config.toml` | TOML | `KIMI_SHARE_DIR` moves the entire application data directory. |
| CLI override | `--config-file <path>` or `--config '<json/toml>'` | TOML/JSON | Replaces the default config file or supplies config content inline for one session. |

There is no published JSON Schema for the configuration file. The authoritative reference is the prose documentation plus the table definitions shown below, so `has_official_schema` is **informal**.

The top-level `default_model` key pins the default model. It must be a key declared in the `[models]` table, not a raw API model string. For example, the observed host config uses:

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
capabilities = ["thinking", "image_in"]
display_name = "Custom Cloud Model"
```

After adding this block, set `default_model = "my-org/custom-model"` or select it at runtime with `--model "my-org/custom-model"`.

### What each key means

| Config key | Meaning |
| :--------- | :------ |
| `providers.<name>.type` | Wire protocol: `kimi`, `openai_legacy`, `openai_responses`, `anthropic`, `gemini`, or `vertexai`. |
| `providers.<name>.base_url` | API endpoint root. |
| `providers.<name>.api_key` | Static API key. OAuth-backed managed providers can leave this empty and use an `[providers.<name>.oauth]` block. |
| `providers.<name>.custom_headers` | Optional extra HTTP headers attached to every request. |
| `providers.<name>.env` | Optional environment variables to set before constructing the provider instance (used for Vertex AI). |
| `models.<key>.provider` | Reference to a key in `[providers]`. |
| `models.<key>.model` | The actual model identifier sent in API calls. |
| `models.<key>.max_context_size` | Context window in tokens; drives compaction behavior. |
| `models.<key>.capabilities` | Feature flags such as `thinking`, `always_thinking`, `image_in`, `video_in`. |
| `models.<key>.display_name` | Human-readable name shown in the welcome panel, status bar, and `/model` picker. |

### Supported API standards

| Standard | Provider type | Base URL location | Auth location | Adapter |
| :------- | :------------ | :---------------- | :------------ | :------ |
| OpenAI Chat Completions | `openai_legacy` | `providers.<name>.base_url` or `OPENAI_BASE_URL` | `providers.<name>.api_key` or `OPENAI_API_KEY` | none |
| OpenAI Responses API | `openai_responses` | `providers.<name>.base_url` or `OPENAI_BASE_URL` | `providers.<name>.api_key` or `OPENAI_API_KEY` | none |
| Anthropic Messages API | `anthropic` | `providers.<name>.base_url` | `providers.<name>.api_key` | none |
| Native Kimi API | `kimi` | `providers.<name>.base_url` or `KIMI_BASE_URL` | `providers.<name>.api_key` or `KIMI_API_KEY` | none |

There is no package-based adapter mechanism such as an npm ai-sdk key. The translation is handled by selecting the correct `type` and, if necessary, pointing `base_url` at a gateway that normalizes the upstream protocol.

### Per-model metadata

The user can declare the following metadata when adding a model:

- `provider` — which provider block to use.
- `model` — the upstream API model identifier.
- `max_context_size` — required; used for compaction and context accounting.
- `capabilities` — optional; controls whether thinking mode, image input, and video input are advertised.
- `display_name` — optional; shown in the UI. For OAuth-managed providers this can be refreshed from the provider's `/models` endpoint at startup.

Cost fields, output token limits, and explicit reasoning-budget fields are not part of the documented model schema.

### Interaction with the built-in catalog

Kimi Code CLI does not ship a separate self-updating model catalog. The `[models]` table in the config file is the source of available models. Managed providers such as `managed:kimi-code` and `managed:moonshot-ai` are pre-integrated, but they still require entries in `[models]` (normally created by `/login`).

Because the config file is the catalog, a manually authored model entry with the same key as a managed/default entry **shadows** the managed metadata. Different keys simply coexist. Best practice is to remove or rename a manual block once a model is natively supported through `/login`, because the managed path will handle OAuth refresh and display-name updates automatically. The CLI does not warn about duplicate keys, so cleanup is manual.

## Adding Local Models

Kimi Code CLI has no first-class local-runner provider type. Local models are added by pointing an `openai_legacy`, `openai_responses`, or `anthropic` provider at the runner's local endpoint.

| Runner | Support path | Notes |
| :----- | :----------- | :---- |
| Ollama | OpenAI-compatible shim | Native `/v1/chat/completions` endpoint. |
| LM Studio | OpenAI-compatible shim | Native local server exposes OpenAI-compatible `/v1/chat/completions`. |
| vLLM | OpenAI-compatible shim | Serves OpenAI-compatible API by default. |
| llama.cpp | OpenAI-compatible shim | `llama-server` exposes `/v1/chat/completions`. |
| oMLX / MLX | OpenAI-compatible shim | Use only if wrapped by an OpenAI-compatible server such as `mlx-lm.server`. |

### Practical example: Ollama

```toml
[providers.ollama-local]
type = "openai_legacy"
base_url = "http://localhost:11434/v1"
api_key = "ollama"

[models."ollama/gemma3:27b"]
provider = "ollama-local"
model = "gemma3:27b"
max_context_size = 131072
capabilities = ["thinking", "image_in"]
display_name = "Gemma 3 27B (Ollama)"
```

The `model` value is the exact Ollama tag, including the size/quantization suffix (`:27b`). The config key (`ollama/gemma3:27b`) is what you pass to `--model`.

### Practical example: LM Studio

```toml
[providers.lmstudio-local]
type = "openai_legacy"
base_url = "http://localhost:1234/v1"
api_key = "lm-studio"

[models."lmstudio/qwen2.5-coder-32b-instruct"]
provider = "lmstudio-local"
model = "qwen2.5-coder-32b-instruct"
max_context_size = 32768
capabilities = ["thinking", "image_in"]
```

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
| `KIMI_CLI_NO_AUTO_UPDATE` | Disables update checks. |

### Precedence

From highest to lowest:

1. CLI flags (`--model`, `--config`, `--config-file`).
2. Environment variables (`KIMI_*`, `OPENAI_*`).
3. Configuration file (`~/.kimi/config.toml`).

This means `KIMI_API_KEY=sk-env kimi --model my-model` will use the env key but still respect the config model entry for `my-model`.

## Sources

- [Kimi Code CLI — Providers and Models](https://moonshotai.github.io/kimi-cli/en/configuration/providers.md)
- [Kimi Code CLI — Config Files](https://moonshotai.github.io/kimi-cli/en/configuration/config-files.md)
- [Kimi Code CLI — Environment Variables](https://moonshotai.github.io/kimi-cli/en/configuration/env-vars.md)
- [Kimi Code CLI — Config Overrides](https://moonshotai.github.io/kimi-cli/en/configuration/overrides.md)
- [Kimi Code CLI — `kimi` Command Reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.md)
- [Kimi Code CLI — Wire Mode](https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.md)
- [Kimi Code CLI — Plugins](https://moonshotai.github.io/kimi-cli/en/customization/plugins.md)
- [Kimi CLI GitHub Repository](https://github.com/MoonshotAI/kimi-cli)

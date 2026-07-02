---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
has_official_schema: informal

config_files:
  - scope: user
    path: ~/.qwen/settings.json
    format: json
    notes: >-
      User-global settings. Holds `model.name`, `modelProviders`, `security.auth.selectedType`,
      and an `env` block for API keys. Observed on this host as `$version: 3` with
      `modelProviders.openai` stored as a legacy array of model objects; current docs describe
      a newer object shape `{ "protocol": "openai", "models": [...] }`.
  - scope: repo
    path: .qwen/settings.json
    format: json
    notes: >-
      Project-specific settings. Overrides the user file for that project.
  - scope: env
    path: ~/.qwen/.env / .qwen/.env / .env
    format: other
    notes: >-
      Dotenv files loaded into the Qwen Code process. Search order is `.qwen/.env` (walked upward),
      `.env`, `~/.qwen/.env`, then `~/.env`. Only sets variables not already present in the environment.
  - scope: env
    path: /etc/qwen-code/system-defaults.json (Linux), C:\ProgramData\qwen-code\system-defaults.json (Windows), /Library/Application Support/QwenCode/system-defaults.json (macOS)
    format: json
    notes: >-
      System-wide defaults. Lowest-precedence settings file; path overridable via `QWEN_CODE_SYSTEM_DEFAULTS_PATH`.
  - scope: env
    path: /etc/qwen-code/settings.json (Linux), C:\ProgramData\qwen-code\settings.json (Windows), /Library/Application Support/QwenCode/settings.json (macOS)
    format: json
    notes: >-
      System-wide override settings. Highest-precedence settings file; path overridable via `QWEN_CODE_SYSTEM_SETTINGS_PATH`.

api_standards:
  - standard: openai_compatible
    base_url_site: per-model `baseUrl` in `modelProviders.openai`; env var `OPENAI_BASE_URL`
    auth_site: per-model `envKey` (defaults to `OPENAI_API_KEY`)
    notes: >-
      Uses the official `openai` Node.js SDK. Supports OpenAI, Azure OpenAI, OpenRouter, Requesty,
      ModelScope, DeepSeek, Alibaba Cloud/DashScope, Coding Plan, and any local OpenAI-compatible server.
  - standard: anthropic_compatible
    base_url_site: per-model `baseUrl` in `modelProviders.anthropic`; env var `ANTHROPIC_BASE_URL`
    auth_site: per-model `envKey` (defaults to `ANTHROPIC_API_KEY`)
    notes: >-
      Uses the official `@anthropic-ai/sdk`. Supports Anthropic Claude and DeepSeek's
      Anthropic-compatible endpoint.
  - standard: bespoke
    base_url_site: per-model `baseUrl` in `modelProviders.gemini`; env var `GEMINI_MODEL` selects model
    auth_site: per-model `envKey` (defaults to `GEMINI_API_KEY`)
    notes: >-
      Google Gemini protocol via the `@google/genai` SDK.
  - standard: bespoke
    base_url_site: per-model `baseUrl` in `modelProviders.vertex-ai`; env var `GOOGLE_MODEL` selects model
    auth_site: per-model `envKey` (defaults to `GOOGLE_API_KEY`)
    notes: >-
      Google Vertex AI. Selecting this auth type sets `GOOGLE_GENAI_USE_VERTEXAI=true` internally
      and routes through the Gemini protocol in Vertex mode.

metadata_overrides:
  - id
  - name
  - description
  - envKey
  - baseUrl
  - capabilities
  - generationConfig
  - generationConfig.timeout
  - generationConfig.maxRetries
  - generationConfig.enableCacheControl
  - generationConfig.contextWindowSize
  - generationConfig.modalities
  - generationConfig.customHeaders
  - generationConfig.extra_body
  - generationConfig.samplingParams
  - generationConfig.reasoning
  - generationConfig.schemaCompliance

merge_semantics: merge

local_runners:
  - runner: ollama
    supported: openai_compatible
    example: >-
      `{ "id": "qwen2.5-7b", "name": "Qwen2.5 7B (Ollama)", "envKey": "OLLAMA_API_KEY",
      "baseUrl": "http://localhost:11434/v1" }` under `modelProviders.openai.models`.
    notes: >-
      Ollama serves an OpenAI-compatible API at `/v1`. Use any placeholder for the API key if Ollama
      requires none.
  - runner: vllm
    supported: openai_compatible
    example: >-
      `{ "id": "llama-3.1-8b", "name": "Llama 3.1 8B (vLLM)", "envKey": "VLLM_API_KEY",
      "baseUrl": "http://localhost:8000/v1" }` under `modelProviders.openai.models`.
    notes: >-
      vLLM exposes an OpenAI-compatible server by default.
  - runner: lmstudio
    supported: openai_compatible
    example: >-
      `{ "id": "local-model", "name": "Local Model (LM Studio)", "envKey": "LMSTUDIO_API_KEY",
      "baseUrl": "http://localhost:1234/v1" }` under `modelProviders.openai.models`.
    notes: >-
      LM Studio's local server is OpenAI-compatible.
  - runner: omlx
    supported: unsupported
    notes: >-
      No documented first-class or OpenAI-compatible integration in Qwen Code. A runner would need
      to expose an OpenAI-compatible endpoint and be registered under `modelProviders.openai`.
  - runner: llamacpp
    supported: unsupported
    notes: >-
      Not documented as a supported local runner. If the server exposes an OpenAI-compatible
      `/v1/chat/completions` endpoint, it can be registered under `modelProviders.openai`, but this
      is not an officially supported path.
  - runner: other
    supported: openai_compatible
    notes: >-
      Any self-hosted server with an OpenAI-compatible API can be added under `modelProviders.openai`.

default_model_site: >-
  `model.name` in `~/.qwen/settings.json` (or project/system settings). Session overrides via
  `--model` / `-m` and provider-specific env vars (`OPENAI_MODEL`/`QWEN_MODEL`, `ANTHROPIC_MODEL`,
  `GEMINI_MODEL`, `GOOGLE_MODEL`).

env_vars:
  - name: OPENAI_API_KEY
    effect: API key for the `openai` auth type and any OpenAI-compatible endpoint.
  - name: OPENAI_BASE_URL
    effect: Base URL override for the `openai` auth type.
  - name: OPENAI_MODEL
    effect: Model ID for the `openai` auth type. Aliased by `QWEN_MODEL`.
  - name: QWEN_MODEL
    effect: Alias for `OPENAI_MODEL`.
  - name: ANTHROPIC_API_KEY
    effect: API key for the `anthropic` auth type.
  - name: ANTHROPIC_BASE_URL
    effect: Base URL override for the `anthropic` auth type.
  - name: ANTHROPIC_MODEL
    effect: Model ID for the `anthropic` auth type.
  - name: GEMINI_API_KEY
    effect: API key for the `gemini` auth type.
  - name: GEMINI_MODEL
    effect: Model ID for the `gemini` auth type.
  - name: GOOGLE_API_KEY
    effect: API key for the `vertex-ai` auth type.
  - name: GOOGLE_MODEL
    effect: Model ID for the `vertex-ai` auth type.
  - name: BAILIAN_CODING_PLAN_API_KEY
    effect: API key for Alibaba Cloud Coding Plan; paired with the Coding Plan `baseUrl`.

changes: []
requires_claudine_update: true
reason: >-
  Claudine's Qwen provider definition currently relies on a single `QWEN_MODEL` env var and an
  `OpencodeCliQwenFiltered` dynamic catalog source. To wrap Qwen accurately, Claudine must honor
  provider-specific env vars (`OPENAI_MODEL`, `ANTHROPIC_MODEL`, `GEMINI_MODEL`, `GOOGLE_MODEL`),
  parse and merge `modelProviders` entries from both the documented object shape and the legacy
  array shape observed on this host, and pass `--auth-type` and `--model` correctly.
---

# Qwen CLI User-Side Model Configuration

## Introduction to Qwen CLI Model Configuration

Qwen Code stores model configuration in JSON `settings.json` files layered by scope, plus optional `.env` files and environment variables.

| Scope | Path | Format | Notes |
| :---- | :--- | :----- | :---- |
| System defaults | `/etc/qwen-code/system-defaults.json` (Linux), `C:\ProgramData\qwen-code\system-defaults.json` (Windows), `/Library/Application Support/QwenCode/system-defaults.json` (macOS) | JSON | Lowest-precedence file; path override via `QWEN_CODE_SYSTEM_DEFAULTS_PATH`. |
| User | `~/.qwen/settings.json` | JSON | Global user settings. |
| Project | `.qwen/settings.json` | JSON | Project-specific; overrides user settings. |
| System override | `/etc/qwen-code/settings.json` (Linux), `C:\ProgramData\qwen-code\settings.json` (Windows), `/Library/Application Support/QwenCode/settings.json` (macOS) | JSON | Highest-precedence file; path override via `QWEN_CODE_SYSTEM_SETTINGS_PATH`. |
| Dotenv | `.qwen/.env`, `.env`, `~/.qwen/.env`, `~/.env` | key=value | Loaded only for variables not already in the process environment. |

There is **no published formal schema** (JSON Schema, OpenAPI, or protobuf) for Qwen Code's config file. The authoritative contract is the prose-and-examples documentation on the [Model Providers](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/model-providers/) page.

On the host used for this research, `~/.qwen/settings.json` has `$version: 3` and stores `modelProviders.openai` as a **legacy array** of model objects:

```json
{
  "security": { "auth": { "selectedType": "openai" } },
  "$version": 3,
  "model": { "name": "qwen3.5-plus" },
  "env": { "DASHSCOPE_API_KEY": "sk-..." },
  "modelProviders": {
    "openai": [
      { "id": "qwen3.5-plus", "name": "[ModelStudio Standard] qwen3.5-plus",
        "baseUrl": "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
        "envKey": "DASHSCOPE_API_KEY" }
    ]
  }
}
```

Current documentation describes a newer shape where each auth type is an object containing `protocol` and `models`. Both shapes appear in the wild.

## Adding Cloud Models

User-added cloud models live in the `modelProviders` object (or legacy array) in `settings.json`. Each top-level key is an auth type / API protocol: `openai`, `anthropic`, `gemini`, or `vertex-ai`.

### Concrete example

Add a hypothetical OpenRouter model that is not in the built-in catalog:

```json
{
  "env": {
    "OPENROUTER_API_KEY": "sk-or-..."
  },
  "modelProviders": {
    "openai": {
      "protocol": "openai",
      "models": [
        {
          "id": "openai/gpt-4o",
          "name": "GPT-4o (via OpenRouter)",
          "description": "OpenAI GPT-4o routed through OpenRouter",
          "envKey": "OPENROUTER_API_KEY",
          "baseUrl": "https://openrouter.ai/api/v1",
          "generationConfig": {
            "timeout": 120000,
            "maxRetries": 3,
            "contextWindowSize": 128000,
            "modalities": { "image": true },
            "samplingParams": {
              "temperature": 0.7,
              "max_tokens": 4096
            }
          }
        }
      ]
    }
  },
  "security": {
    "auth": { "selectedType": "openai" }
  },
  "model": {
    "name": "openai/gpt-4o"
  }
}
```

### What each key means

| Key | Meaning |
| :-- | :------ |
| `modelProviders` | Per-protocol catalog of user-added models. |
| `protocol` | Wire protocol for this auth-type group: `openai`, `anthropic`, or `gemini`. |
| `models` | Array of model entries. |
| `id` | Model ID sent in the API request (required). |
| `name` | Display name in the `/model` picker. |
| `description` | Longer picker description. |
| `envKey` | Name of the environment variable holding the API key; defaults to the protocol's standard key if omitted. |
| `baseUrl` | Endpoint override. |
| `generationConfig` | Atomic settings package applied when this model is selected. |
| `capabilities` | Capability hints such as `{ "vision": true }`. |

### Supported API standards

| Standard | Auth type | SDK | Base URL / auth |
| :------- | :-------- | :-- | :---------------- |
| OpenAI-compatible | `openai` | `openai` Node.js SDK | `baseUrl` or `OPENAI_BASE_URL`; key via `envKey` or `OPENAI_API_KEY`. |
| Anthropic-compatible | `anthropic` | `@anthropic-ai/sdk` | `baseUrl` or `ANTHROPIC_BASE_URL`; key via `envKey` or `ANTHROPIC_API_KEY`. |
| Google Gemini | `gemini` | `@google/genai` | `baseUrl` or default Gemini endpoint; key via `envKey` or `GEMINI_API_KEY`. |
| Google Vertex AI | `vertex-ai` | `@google/genai` in Vertex mode | `baseUrl` or Vertex endpoint; key via `envKey` or `GOOGLE_API_KEY`. Sets `GOOGLE_GENAI_USE_VERTEXAI=true`. |

There is **no adapter plug-in mechanism** like an npm package key. The `protocol` field selects the SDK, and the endpoint must speak that protocol.

### Per-model metadata

Users can declare:

- `id`, `name`, `description`
- `envKey`, `baseUrl`
- `capabilities` (e.g. `vision`)
- `generationConfig`:
  - `timeout`, `maxRetries`, `enableCacheControl`, `contextWindowSize`
  - `modalities` (`image`, `pdf`, `audio`, `video`)
  - `customHeaders`, `extra_body` (OpenAI-compatible only)
  - `samplingParams` (`temperature`, `top_p`, `max_tokens`, etc.)
  - `reasoning` (`effort`, `budget_tokens`, or `false`)

When a `modelProviders` model is selected, its `generationConfig` is applied **atomically**; lower layers do not merge into it.

### Interaction with the built-in catalog

User-added models **merge** with built-in and auto-configured entries in the `/model` picker:

- Models from `modelProviders` appear alongside Alibaba Cloud Coding Plan auto-configured models.
- Within one auth type, models are uniquely identified by `id + baseUrl`. The same `id` can be declared multiple times with different `baseUrl` values; duplicates sharing both `id` and `baseUrl` are skipped, with the first occurrence winning.
- The entire `modelProviders` object is replaced across settings-file scopes (project overrides user, system override overrides project).
- Automatic Coding Plan updates replace auto-configured Coding Plan entries but preserve manually added entries. A manual entry can still be overwritten if it shares the same `envKey` and `baseUrl` as the auto config.
- `qwen-oauth` entries are hard-coded and cannot be overridden.

Because built-in and Coding Plan catalogs update automatically, manual blocks for models that later become built-in should be removed. Qwen Code does not automate this cleanup; users must delete the entry from `settings.json`.

## Adding Local Models

Qwen Code has **no native backends** for local runners. Any local model must expose an OpenAI-compatible API and be registered under the `openai` auth type.

| Runner | Support | Notes |
| :----- | :------ | :---- |
| Ollama | OpenAI-compatible shim | `http://localhost:11434/v1`; use any placeholder API key. |
| vLLM | OpenAI-compatible shim | `http://localhost:8000/v1`; use any placeholder API key if unauthenticated. |
| LM Studio | OpenAI-compatible shim | `http://localhost:1234/v1`. |
| oMLX | Unsupported | No documented OpenAI-compatible integration. |
| llama.cpp | Unsupported | Not documented; only usable if the server exposes an OpenAI-compatible endpoint. |
| Other | OpenAI-compatible shim | Any local server with `/v1/chat/completions` can be registered. |

### Practical example: Ollama

```json
{
  "env": { "OLLAMA_API_KEY": "ollama" },
  "modelProviders": {
    "openai": {
      "protocol": "openai",
      "models": [
        {
          "id": "gemma3:27b",
          "name": "Gemma 3 27B (Ollama)",
          "envKey": "OLLAMA_API_KEY",
          "baseUrl": "http://localhost:11434/v1",
          "generationConfig": {
            "contextWindowSize": 128000,
            "samplingParams": { "temperature": 0.7, "max_tokens": 4096 }
          }
        }
      ]
    }
  },
  "model": { "name": "gemma3:27b" }
}
```

### Practical example: vLLM

```json
{
  "env": { "VLLM_API_KEY": "not-needed" },
  "modelProviders": {
    "openai": {
      "protocol": "openai",
      "models": [
        {
          "id": "llama-3.1-8b",
          "name": "Llama 3.1 8B (vLLM)",
          "envKey": "VLLM_API_KEY",
          "baseUrl": "http://localhost:8000/v1",
          "generationConfig": {
            "contextWindowSize": 128000,
            "samplingParams": { "temperature": 0.6, "max_tokens": 8192 }
          }
        }
      ]
    }
  },
  "model": { "name": "llama-3.1-8b" }
}
```

Local model IDs are written exactly as the local server expects them, including size or quantization tags such as `gemma3:27b`.

## Environment Overrides

Environment variables take precedence over the corresponding `settings.json` fields when both exist.

| Variable | Effect | Precedence |
| :------- | :----- | :--------- |
| `OPENAI_API_KEY` | API key for OpenAI-compatible endpoints. | `envKey` or default. |
| `OPENAI_BASE_URL` | Base URL override for `openai` auth type. | Overrides per-model `baseUrl`. |
| `OPENAI_MODEL` / `QWEN_MODEL` | Model ID for `openai` auth type. | `--model` > env var > `model.name`. |
| `ANTHROPIC_API_KEY` | API key for Anthropic endpoints. | `envKey` or default. |
| `ANTHROPIC_BASE_URL` | Base URL override for `anthropic` auth type. | Overrides per-model `baseUrl`. |
| `ANTHROPIC_MODEL` | Model ID for `anthropic` auth type. | `--model` > env var > `model.name`. |
| `GEMINI_API_KEY` | API key for Gemini endpoints. | `envKey` or default. |
| `GEMINI_MODEL` | Model ID for `gemini` auth type. | `--model` > env var > `model.name`. |
| `GOOGLE_API_KEY` | API key for Vertex AI endpoints. | `envKey` or default. |
| `GOOGLE_MODEL` | Model ID for `vertex-ai` auth type. | `--model` > env var > `model.name`. |
| `BAILIAN_CODING_PLAN_API_KEY` | API key for Alibaba Cloud Coding Plan. | Used with Coding Plan `baseUrl`. |

Credentials can be set via shell `export`, `.env` files, or the `env` field in `settings.json`. The priority order is CLI flags > system environment > `.env` file > `settings.json` `env`.

## Sources

- [Qwen Code — Model Providers](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/model-providers/)
- [Qwen Code — Authentication](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/auth/)
- [Qwen Code — Settings](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/)
- [Qwen Code GitHub repository](https://github.com/QwenLM/qwen-code)

---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
has_official_schema: informal

model_config_paths:
  - scope: user
    path: ~/.qwen/settings.json
    format: json
    notes: >-
      User-global settings. Holds `model.name`, `model.baseUrl`, `modelProviders`,
      `security.auth.selectedType`, and an `env` block for API keys. On this host
      observed as `$version: 3` with `modelProviders.openai` stored as a legacy
      array of model objects; current docs describe a newer object shape
      `{ "protocol": "openai", "models": [...] }`.
  - scope: repo
    path: .qwen/settings.json
    format: json
    notes: >-
      Project-specific settings. The entire `modelProviders` object from project
      settings replaces the corresponding section in user settings, rather than
      merging.
  - scope: env
    path: .qwen/.env / .env / ~/.qwen/.env / ~/.env
    format: other
    notes: >-
      Dotenv files loaded into the Qwen Code process. Search order is `.qwen/.env`
      (walked upward from the current directory), `.env`, `~/.qwen/.env`, then
      `~/.env`. Only the first file found is loaded, and only variables not already
      present in the environment are set.
  - scope: env
    path: /etc/qwen-code/system-defaults.json (Linux), C:\ProgramData\qwen-code\system-defaults.json (Windows), /Library/Application Support/QwenCode/system-defaults.json (macOS)
    format: json
    notes: >-
      System-wide defaults. Lowest-precedence settings file; path overridable via
      `QWEN_CODE_SYSTEM_DEFAULTS_PATH`.
  - scope: env
    path: /etc/qwen-code/settings.json (Linux), C:\ProgramData\qwen-code\settings.json (Windows), /Library/Application Support/QwenCode/settings.json (macOS)
    format: json
    notes: >-
      System-wide override settings. Highest-precedence settings file; path
      overridable via `QWEN_CODE_SYSTEM_SETTINGS_PATH`.

api_standards:
  - standard: openai_compatible
    base_url_site: per-model `baseUrl` in `modelProviders.openai`; env var `OPENAI_BASE_URL`
    auth_site: per-model `envKey` (defaults to `OPENAI_API_KEY`)
    notes: >-
      Uses the official `openai` Node.js SDK. Supports OpenAI, Azure OpenAI,
      OpenRouter, Requesty, ModelScope, DeepSeek, Alibaba Cloud/DashScope, Coding
      Plan, and any local OpenAI-compatible server. `extra_body` is supported only
      for OpenAI-compatible providers.
  - standard: anthropic_compatible
    base_url_site: per-model `baseUrl` in `modelProviders.anthropic`; env var `ANTHROPIC_BASE_URL`
    auth_site: per-model `envKey` (defaults to `ANTHROPIC_API_KEY`)
    notes: >-
      Uses the official `@anthropic-ai/sdk`. Supports Anthropic Claude and any
      Anthropic-compatible endpoint.
  - standard: bespoke
    base_url_site: per-model `baseUrl` in `modelProviders.gemini`; default Gemini endpoint if omitted
    auth_site: per-model `envKey` (defaults to `GEMINI_API_KEY`)
    notes: >-
      Google Gemini protocol via the `@google/genai` SDK. Model selection env var
      is `GEMINI_MODEL`.
  - standard: bespoke
    base_url_site: per-model `baseUrl` in `modelProviders.vertex-ai`; default Vertex endpoint if omitted
    auth_site: per-model `envKey` (defaults to `GOOGLE_API_KEY`)
    notes: >-
      Google Vertex AI. Selecting this auth type sets `GOOGLE_GENAI_USE_VERTEXAI=true`
      internally and routes through the Gemini protocol in Vertex mode. Model
      selection env var is `GOOGLE_MODEL`.

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
  - generationConfig.reasoning.effort
  - generationConfig.reasoning.budget_tokens
  - generationConfig.schemaCompliance

merge_semantics: merge

local_runners:
  - runner: ollama
    integration: base_url_override
    standard: openai_compatible
    example: >-
      `{ "id": "gemma3:27b", "name": "Gemma 3 27B (Ollama)", "envKey": "OLLAMA_API_KEY",
      "baseUrl": "http://localhost:11434/v1" }` under `modelProviders.openai.models`.
    notes: >-
      Ollama serves an OpenAI-compatible API at `/v1`. Use any placeholder for the
      API key if Ollama requires none.
  - runner: omlx
    integration: base_url_override
    standard: openai_compatible
    example: >-
      `{ "id": "Qwen3.6-35B-A3B-oQ6", "name": "Qwen3.6 35B (oMLX)", "envKey": "OMLX_API_KEY",
      "baseUrl": "http://localhost:8000/v1" }` under `modelProviders.openai.models`.
    notes: >-
      oMLX serves an OpenAI-compatible API at `/v1`. Auth is optional unless
      configured; use a placeholder if none is required.
  - runner: lmstudio
    integration: base_url_override
    standard: openai_compatible
    example: >-
      `{ "id": "local-model", "name": "Local Model (LM Studio)", "envKey": "LMSTUDIO_API_KEY",
      "baseUrl": "http://localhost:1234/v1" }` under `modelProviders.openai.models`.
    notes: >-
      LM Studio's local server is OpenAI-compatible. Auth is optional; set
      `LMSTUDIO_API_KEY` to the configured token when required.
  - runner: llamacpp
    integration: base_url_override
    standard: openai_compatible
    example: >-
      `{ "id": "my-alias", "name": "Local Model (llama.cpp)", "envKey": "LLAMA_API_KEY",
      "baseUrl": "http://localhost:8080/v1" }` under `modelProviders.openai.models`.
    notes: >-
      llama-server exposes an OpenAI-compatible `/v1/chat/completions` endpoint.
      Model ID is the `--alias` value or the GGUF filename.
  - runner: vllm
    integration: base_url_override
    standard: openai_compatible
    example: >-
      `{ "id": "llama-3.1-8b", "name": "Llama 3.1 8B (vLLM)", "envKey": "VLLM_API_KEY",
      "baseUrl": "http://localhost:8000/v1" }` under `modelProviders.openai.models`.
    notes: >-
      vLLM exposes an OpenAI-compatible server by default. Each vLLM process hosts
      one model.
  - runner: other
    integration: base_url_override
    standard: openai_compatible
    example: >-
      `{ "id": "<runner-model-id>", "name": "Local Model", "envKey": "LOCAL_API_KEY",
      "baseUrl": "http://localhost:<port>/v1" }` under `modelProviders.openai.models`.
    notes: >-
      Any self-hosted server with an OpenAI-compatible API can be added under
      `modelProviders.openai`.

cloud_bridge:
  supported: true
  mechanism: per-model `baseUrl` under the matching auth type (`openai`, `anthropic`, `gemini`, or `vertex-ai`)
  example: |
    {
      "modelProviders": {
        "openai": {
          "protocol": "openai",
          "models": [
            {
              "id": "gpt-4o",
              "name": "GPT-4o (via OpenAI-compatible gateway)",
              "envKey": "OPENAI_API_KEY",
              "baseUrl": "https://gateway.example.com/v1"
            }
          ]
        }
      }
    }

default_model_site: >-
  `model.name` (and disambiguating `model.baseUrl`) in `~/.qwen/settings.json`
  (or project/system settings). Session overrides via `--model` / `-m` and
  provider-specific env vars (`OPENAI_MODEL`, `ANTHROPIC_MODEL`, `GEMINI_MODEL`,
  `GOOGLE_MODEL`).

env_vars:
  - name: OPENAI_API_KEY
    effect: API key for the `openai` auth type and any OpenAI-compatible endpoint.
  - name: OPENAI_BASE_URL
    effect: Base URL override for the `openai` auth type.
  - name: OPENAI_MODEL
    effect: Model ID for the `openai` auth type.
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
    effect: API key for Alibaba Cloud Coding Plan; paired with the Coding Plan baseUrl.
  - name: OLLAMA_API_KEY
    effect: API key placeholder for local Ollama endpoints.
  - name: VLLM_API_KEY
    effect: API key for local vLLM endpoints when authentication is enabled.
  - name: LMSTUDIO_API_KEY
    effect: API key for local LM Studio endpoints when authentication is enabled.

changes:
  - 'Qwen OAuth free tier was discontinued on 2026-04-15 and is no longer a selectable /auth entry; the standalone `qwen auth` CLI command has been removed.'
  - 'Current documentation describes a newer `modelProviders` object shape (`{ "protocol": "...", "models": [...] }`); the legacy array shape is still observed on this host and should still be parsed.'
  - '`modelProviders` is replaced across settings-file scopes (project overrides user, system override overrides project), not merged.'
  - 'Alibaba Cloud Coding Plan auto-updates preserve manually added entries but can overwrite entries sharing the same `envKey` and `baseUrl`.'
  - 'All major local runners (Ollama, oMLX, LM Studio, llama.cpp, vLLM) are supported via OpenAI-compatible base-URL override; the previous revision incorrectly classified oMLX and llama.cpp as unsupported.'
  - '`QWEN_MODEL` is no longer documented as an alias; model selection uses provider-specific env vars.'

requires_claudine_update: true
reason: >-
  Claudine's Qwen provider wrapper must adapt to the documented config shape:
  honor provider-specific env vars (`OPENAI_MODEL`, `ANTHROPIC_MODEL`,
  `GEMINI_MODEL`, `GOOGLE_MODEL`), parse and merge `modelProviders` entries from
  both the documented object shape and the legacy array shape observed on this
  host, and pass `--auth-type`, `--model`, and provider-specific `--*-api-key` /
  `--*-base-url` flags correctly. The removal of the `qwen auth` CLI command also
  affects auth setup assumptions.
---

# Qwen CLI User-Side Model Configuration

## Introduction to Qwen CLI Model Configuration

Qwen Code stores model configuration in JSON `settings.json` files layered by scope, plus optional `.env` files and environment variables. Configuration precedence, from lowest to highest, is:

| Level | Source |
| :---- | :----- |
| 1 | Hard-coded defaults |
| 2 | System defaults file |
| 3 | User settings file (`~/.qwen/settings.json`) |
| 4 | Project settings file (`.qwen/settings.json`) |
| 5 | System override file |
| 6 | Environment variables and `.env` files |
| 7 | CLI arguments |

| Scope | Path | Format | Notes |
| :---- | :--- | :----- | :---- |
| System defaults | `/etc/qwen-code/system-defaults.json` (Linux), `C:\ProgramData\qwen-code\system-defaults.json` (Windows), `/Library/Application Support/QwenCode/system-defaults.json` (macOS) | JSON | Lowest-precedence file; path override via `QWEN_CODE_SYSTEM_DEFAULTS_PATH`. |
| User | `~/.qwen/settings.json` | JSON | Global user settings. |
| Project | `.qwen/settings.json` | JSON | Project-specific; overrides user settings. The entire `modelProviders` object is replaced, not merged. |
| System override | `/etc/qwen-code/settings.json` (Linux), `C:\ProgramData\qwen-code\settings.json` (Windows), `/Library/Application Support/QwenCode/settings.json` (macOS) | JSON | Highest-precedence file; path override via `QWEN_CODE_SYSTEM_SETTINGS_PATH`. |
| Dotenv | `.qwen/.env`, `.env`, `~/.qwen/.env`, `~/.env` | key=value | First file found is loaded; variables already in the environment are not overwritten. |

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
        "envKey": "DASHSCOPE_API_KEY" },
      { "id": "glm-5", "name": "[ModelStudio Standard] glm-5",
        "baseUrl": "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
        "envKey": "DASHSCOPE_API_KEY" },
      { "id": "kimi-k2.5", "name": "[ModelStudio Standard] kimi-k2.5",
        "baseUrl": "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
        "envKey": "DASHSCOPE_API_KEY" }
    ]
  }
}
```

Current documentation describes a newer shape where each auth type is an object containing `protocol` and `models`. Both shapes appear in the wild; the CLI migrates legacy settings automatically and backs them up first.

## Adding Cloud Models

User-added cloud models live in the `modelProviders` object (or legacy array) in `settings.json`. Each top-level key is an auth type / API protocol: `openai`, `anthropic`, `gemini`, or `vertex-ai`. The `qwen-oauth` key is hard-coded and cannot be overridden.

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
  - `schemaCompliance` (e.g. `"auto"` for Gemini)

When a `modelProviders` model is selected, its `generationConfig` is applied **atomically**; lower layers do not merge into it. The atomic fields are `samplingParams`, `customHeaders`, and `extra_body`.

### Interaction with the built-in catalog

User-added models **merge** with built-in and auto-configured entries in the `/model` picker:

- Models from `modelProviders` appear alongside Alibaba Cloud Coding Plan auto-configured models.
- Within one auth type, models are uniquely identified by `id + baseUrl`. The same `id` can be declared multiple times with different `baseUrl` values; duplicates sharing both `id` and `baseUrl` are skipped, with the first occurrence winning.
- The entire `modelProviders` object is **replaced** across settings-file scopes (project overrides user, system override overrides project).
- Automatic Coding Plan updates replace auto-configured Coding Plan entries but preserve manually added entries. A manual entry can still be overwritten if it shares the same `envKey` and `baseUrl` as the auto config.
- `qwen-oauth` entries are hard-coded and cannot be overridden.

Because built-in and Coding Plan catalogs update automatically, manual blocks for models that later become built-in should be removed. Qwen Code does not automate this cleanup; users must delete the entry from `settings.json`.

### Cross-cloud bridging

Qwen CLI can be routed at a different cloud vendor's API by adding a per-model `baseUrl` under the auth type that matches the vendor's wire protocol. For example, a vendor exposing an OpenAI-compatible endpoint can be used directly:

```json
{
  "modelProviders": {
    "openai": {
      "protocol": "openai",
      "models": [
        {
          "id": "gpt-4o",
          "name": "GPT-4o (via OpenAI-compatible gateway)",
          "envKey": "OPENAI_API_KEY",
          "baseUrl": "https://gateway.example.com/v1"
        }
      ]
    }
  }
}
```

If the target vendor's native API does not speak OpenAI, Anthropic, or Gemini/Vertex format, route through a translating proxy such as [LiteLLM](https://docs.litellm.ai/) and point `baseUrl` at the proxy's compatible endpoint.

## Adding Local Models

Local-runner support is a property of **API-standard bridging**, not of Qwen Code "knowing about" a runner. Qwen Code has no native backends for local runners; any local model must expose an OpenAI-compatible API (the documented path) or one of the other supported protocols, and be registered under the matching auth type.

| Runner | Integration path | Notes |
| :----- | :--------------- | :---- |
| Ollama | Base-URL override | `http://localhost:11434/v1`; use any placeholder API key. |
| oMLX | Base-URL override | `http://localhost:8000/v1`; use any placeholder if unauthenticated. |
| LM Studio | Base-URL override | `http://localhost:1234/v1`; set token if auth is enabled. |
| llama.cpp | Base-URL override | `http://localhost:8080/v1`; model ID is the `--alias` or GGUF filename. |
| vLLM | Base-URL override | `http://localhost:8000/v1`; use any placeholder if unauthenticated. |
| Other | Base-URL override | Any local server with `/v1/chat/completions` can be registered. |

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

Environment variables take precedence over the corresponding `settings.json` fields when both exist. Credentials can be set via shell `export`, `.env` files, or the `env` field in `settings.json`. The priority order is CLI flags > system environment > `.env` file > `settings.json` `env`.

| Variable | Effect | Precedence |
| :------- | :----- | :--------- |
| `OPENAI_API_KEY` | API key for OpenAI-compatible endpoints. | `envKey` or default. |
| `OPENAI_BASE_URL` | Base URL override for `openai` auth type. | Overrides per-model `baseUrl`. |
| `OPENAI_MODEL` | Model ID for `openai` auth type. | `--model` > env var > `model.name`. |
| `ANTHROPIC_API_KEY` | API key for Anthropic endpoints. | `envKey` or default. |
| `ANTHROPIC_BASE_URL` | Base URL override for `anthropic` auth type. | Overrides per-model `baseUrl`. |
| `ANTHROPIC_MODEL` | Model ID for `anthropic` auth type. | `--model` > env var > `model.name`. |
| `GEMINI_API_KEY` | API key for Gemini endpoints. | `envKey` or default. |
| `GEMINI_MODEL` | Model ID for `gemini` auth type. | `--model` > env var > `model.name`. |
| `GOOGLE_API_KEY` | API key for Vertex AI endpoints. | `envKey` or default. |
| `GOOGLE_MODEL` | Model ID for `vertex-ai` auth type. | `--model` > env var > `model.name`. |
| `BAILIAN_CODING_PLAN_API_KEY` | API key for Alibaba Cloud Coding Plan. | Used with Coding Plan `baseUrl`. |
| `OLLAMA_API_KEY` | Placeholder key for local Ollama endpoints. | Referenced by `envKey`. |
| `VLLM_API_KEY` | Key for local vLLM endpoints when auth is enabled. | Referenced by `envKey`. |
| `LMSTUDIO_API_KEY` | Key for local LM Studio endpoints when auth is enabled. | Referenced by `envKey`. |

CLI flags include `--model` / `-m`, `--auth-type`, and provider-specific flags such as `--openai-api-key` and `--openai-base-url`.

## Changelog

- **2026-07-02** — Qwen OAuth free tier was discontinued on 2026-04-15 and is no longer a selectable entry in `/auth`; the standalone `qwen auth` CLI command has been removed.
- **2026-07-02** — Current documentation describes a newer `modelProviders` object shape (`{ "protocol": "...", "models": [...] }`); the legacy array shape is still observed on this host and should still be parsed.
- **2026-07-02** — Clarified that `modelProviders` is replaced across settings-file scopes, not merged, although entries still merge with the built-in catalog within the active settings file.
- **2026-07-02** — Alibaba Cloud Coding Plan auto-updates preserve manual entries but can overwrite entries sharing the same `envKey` and `baseUrl`.
- **2026-07-02** — Reclassified local runners: Ollama, oMLX, LM Studio, llama.cpp, and vLLM are all supported via OpenAI-compatible base-URL override; the previous revision incorrectly classified oMLX and llama.cpp as unsupported.
- **2026-07-02** — Removed `QWEN_MODEL`; model selection uses provider-specific env vars.

## Sources

- [Qwen Code — Model Providers](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/model-providers/)
- [Qwen Code — Authentication](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/auth/)
- [Qwen Code — Settings](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/)
- [Qwen Code GitHub repository](https://github.com/QwenLM/qwen-code)

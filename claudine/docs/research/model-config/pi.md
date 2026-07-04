---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
has_official_schema: none

model_config_paths:
  - scope: user
    path: ~/.pi/agent/models.json
    format: json
    notes: 'Primary file for adding custom cloud and local providers/models. Reloaded each time /model is opened; no restart needed. Observed on this host with an omlx provider entry using api: openai-completions.'
  - scope: user
    path: ~/.pi/agent/settings.json
    format: json
    notes: 'Global user settings. Pins defaultProvider and defaultModel. Observed on this host with defaultProvider: omlx and defaultModel: Qwen3.6-35B-A3B-oQ6.'
  - scope: repo
    path: .pi/settings.json
    format: json
    notes: 'Project-local settings that override global settings. Nested objects are merged. Subject to project trust flow on interactive startup.'
  - scope: user
    path: ~/.pi/agent/auth.json
    format: json
    notes: 'Credential store for API keys and OAuth tokens. Created with 0600 permissions. Not a model-config file per se, but credentials stored here unlock models declared in models.json.'

api_standards:
  - standard: openai_compatible
    base_url_site: baseUrl in ~/.pi/agent/models.json (provider or model level)
    auth_site: apiKey in models.json, or /login/auth.json, or provider-specific env var such as OPENAI_API_KEY
    notes: 'Covers api values openai-completions, openai-responses, azure-openai-responses, and openai-codex-responses. Most common path for local runners and proxies.'
  - standard: anthropic_compatible
    base_url_site: baseUrl in ~/.pi/agent/models.json
    auth_site: apiKey in models.json, or /login/auth.json, or ANTHROPIC_API_KEY
    notes: 'Use api: anthropic-messages for Anthropic-compatible proxies or custom endpoints. Compat flags control eager tool streaming, cache retention, adaptive thinking, and empty signatures.'
  - standard: bespoke
    base_url_site: baseUrl in ~/.pi/agent/models.json
    auth_site: apiKey in models.json, or /login/auth.json, or GEMINI_API_KEY
    notes: 'Covers google-generative-ai, google-vertex, mistral-conversations, bedrock-converse-stream, and fully custom APIs implemented via an extension streamSimple function.'

metadata_overrides:
  - id
  - name
  - api
  - reasoning
  - thinkingLevelMap
  - input
  - contextWindow
  - maxTokens
  - cost
  - compat
  - headers
  - baseUrl
  - modelOverrides

merge_semantics: shadow

local_runners:
  - runner: ollama
    integration: base_url_override
    standard: openai_compatible
    example: '{ "providers": { "ollama": { "baseUrl": "http://localhost:11434/v1", "api": "openai-completions", "apiKey": "ollama", "compat": { "supportsDeveloperRole": false, "supportsReasoningEffort": false }, "models": [{ "id": "llama3.1:8b" }] } } }'
    notes: 'Ollama exposes both OpenAI- and Anthropic-compatible endpoints; Pi documentation examples use openai-completions. Use a dummy apiKey because Ollama ignores it.'
  - runner: omlx
    integration: first_class
    standard: openai_compatible
    example: '{ "providers": { "omlx": { "baseUrl": "http://127.0.0.1:8000/v1", "api": "openai-completions", "apiKey": "VPZ-77XNJozqp4o", "authHeader": true, "models": [{ "id": "Qwen3.6-35B-A3B-oQ6", "name": "Qwen3.6-35B-A3B-oQ6", "reasoning": false, "input": ["text", "image"], "cost": {"input":0,"output":0,"cacheRead":0,"cacheWrite":0}, "contextWindow": 98304, "maxTokens": 98304 }] } } }'
    notes: 'First-class `omlx launch pi` hook launches Pi configured against the running oMLX server; the example shows the manual equivalent observed in the host ~/.pi/agent/models.json. oMLX also exposes an Anthropic-compatible endpoint.'
  - runner: lmstudio
    integration: base_url_override
    standard: openai_compatible
    example: '{ "providers": { "lmstudio": { "baseUrl": "http://localhost:1234/v1", "api": "openai-completions", "apiKey": "lmstudio", "models": [{ "id": "qwen2.5-coder-32b-instruct" }] } } }'
    notes: 'Routes through LM Studio OpenAI-compatible local server. LM Studio also exposes an Anthropic-compatible endpoint, so api: anthropic-messages with baseUrl http://localhost:1234 is also valid.'
  - runner: llamacpp
    integration: base_url_override
    standard: openai_compatible
    example: '{ "providers": { "llamacpp": { "baseUrl": "http://localhost:8080/v1", "api": "openai-completions", "apiKey": "llamacpp", "models": [{ "id": "llama-3-70b" }] } } }'
    notes: 'Works via llama.cpp OpenAI-compatible HTTP server. Size/quantization tags are part of the model id string. Anthropic-compatible endpoint is also available since build b7187.'
  - runner: vllm
    integration: base_url_override
    standard: openai_compatible
    example: '{ "providers": { "vllm": { "baseUrl": "http://localhost:8000/v1", "api": "openai-completions", "apiKey": "vllm", "compat": { "supportsDeveloperRole": false, "supportsReasoningEffort": false }, "models": [{ "id": "qwen2.5-72b" }] } } }'
    notes: 'Use the /v1 OpenAI-compatible endpoint. vLLM also exposes an Anthropic-compatible endpoint. Compat flags are commonly required because vLLM does not support the developer role or reasoning_effort natively.'
  - runner: other
    integration: base_url_override
    standard: openai_compatible
    notes: 'Any runner exposing an OpenAI-compatible, Anthropic-compatible, or other supported endpoint can be registered in models.json. For unsupported APIs, use a custom extension with streamSimple.'

cloud_bridge:
  supported: true
  mechanism: baseUrl override in ~/.pi/agent/models.json (provider-level or model-level)
  example: |
    {
      "providers": {
        "openrouter": {
          "baseUrl": "https://openrouter.ai/api/v1",
          "apiKey": "$OPENROUTER_API_KEY",
          "api": "openai-completions",
          "models": [
            {
              "id": "openrouter/anthropic/claude-3.5-sonnet",
              "name": "OpenRouter Claude 3.5 Sonnet",
              "reasoning": false,
              "input": ["text", "image"],
              "cost": { "input": 3, "output": 15, "cacheRead": 0.3, "cacheWrite": 3.75 },
              "contextWindow": 200000,
              "maxTokens": 16384,
              "compat": { "openRouterRouting": { "only": ["anthropic", "amazon-bedrock"] } }
            }
          ]
        }
      }
    }

default_model_site: 'defaultProvider and defaultModel keys in ~/.pi/agent/settings.json (or .pi/settings.json); session override via --provider and --model flags'

env_vars:
  - name: PI_CODING_AGENT_DIR
    effect: 'Relocates the config directory where models.json, settings.json, auth.json, and trust.json are loaded. Default is ~/.pi/agent.'
  - name: PI_CODING_AGENT_SESSION_DIR
    effect: 'Overrides the session storage directory. Lower precedence than --session-dir.'
  - name: PI_PACKAGE_DIR
    effect: 'Overrides the package directory, useful for Nix/Guix store paths.'
  - name: ANTHROPIC_API_KEY
    effect: 'Provides the API key for the built-in anthropic provider. Resolution order: --api-key > auth.json > env var > custom provider apiKey in models.json.'
  - name: OPENAI_API_KEY
    effect: 'Provides the API key for the built-in openai provider and OpenAI-compatible endpoints.'
  - name: GEMINI_API_KEY
    effect: 'Provides the API key for the built-in google provider and google-generative-ai custom endpoints.'
  - name: PI_OFFLINE
    effect: 'Disables startup network operations (update checks, package checks, telemetry). Does not change endpoint selection.'
  - name: PI_SKIP_VERSION_CHECK
    effect: 'Skips the Pi version update check at startup.'
  - name: PI_TELEMETRY
    effect: 'Overrides install/update telemetry and provider attribution headers: 1/true/yes or 0/false/no. Does not disable update checks.'
  - name: PI_CACHE_RETENTION
    effect: 'Set to long for extended prompt cache where supported.'
  - name: VISUAL / EDITOR
    effect: 'Fallback external editor for Ctrl+G when externalEditor is unset.'

changes:
  - 'Added cloud_bridge frontmatter and section documenting cross-vendor API routing via baseUrl override.'
  - 'Reclassified local_runners from supported boolean to integration enum; oMLX ships a first-class `omlx launch pi` hook, while Ollama, LM Studio, llama.cpp, and vLLM are base_url_override paths on openai_compatible endpoints.'
  - 'Expanded api_standards notes to include openai-responses, google-vertex, azure-openai-responses, openai-codex-responses, mistral-conversations, and bedrock-converse-stream under the openai_compatible and bespoke umbrellas.'
  - 'Added modelOverrides to metadata_overrides.'
  - 'Updated env_vars to current Pi 0.73.1 usage docs, adding PI_CODING_AGENT_SESSION_DIR, PI_PACKAGE_DIR, PI_TELEMETRY, PI_CACHE_RETENTION, PI_SKIP_VERSION_CHECK, and VISUAL/EDITOR.'
  - 'Observed host Pi version is 0.73.1 (settings.json lastChangelogVersion 0.73.0).'

requires_claudine_update: true
reason: 'Pi is not currently one of Claudine wrapped providers. If Claudine adds Pi support, it will need a new provider adapter, models.json/catalog integration, and credential/env resolution matching Pi config layout. The refreshed local-runner framing and cloud_bridge facts also affect any future Pi wrapper profile.'
---

# Pi User-Side Model Configuration

## Introduction to Pi Model Configuration

Pi stores model configuration in JSON files under the agent directory. The location defaults to `~/.pi/agent` and can be relocated with `PI_CODING_AGENT_DIR`.

| Scope | Path | Format | Purpose |
| :---- | :--- | :----- | :------ |
| User | `~/.pi/agent/models.json` | JSON | Add custom cloud/local providers and models |
| User | `~/.pi/agent/settings.json` | JSON | Pin default provider and model |
| User | `~/.pi/agent/auth.json` | JSON | Store API keys and OAuth tokens |
| Project | `.pi/settings.json` | JSON | Project overrides for defaults and resources |

On the host used for this research, `~/.pi/agent/settings.json` pins the default to an oMLX local model:

```json
{
  "lastChangelogVersion": "0.73.0",
  "defaultProvider": "omlx",
  "defaultModel": "Qwen3.6-35B-A3B-oQ6"
}
```

There is no published JSON Schema or other formal machine-readable schema for these files. Configuration is documented through prose and examples in the Pi docs.

## Adding Cloud Models

Cloud models are added by defining a new provider block in `~/.pi/agent/models.json`. A provider block needs at minimum `baseUrl`, `api`, and a `models` array.

### Concrete example: OpenRouter

```json
{
  "providers": {
    "openrouter": {
      "baseUrl": "https://openrouter.ai/api/v1",
      "apiKey": "$OPENROUTER_API_KEY",
      "api": "openai-completions",
      "models": [
        {
          "id": "openrouter/anthropic/claude-3.5-sonnet",
          "name": "OpenRouter Claude 3.5 Sonnet",
          "reasoning": false,
          "input": ["text", "image"],
          "cost": { "input": 3, "output": 15, "cacheRead": 0.3, "cacheWrite": 3.75 },
          "contextWindow": 200000,
          "maxTokens": 16384,
          "compat": {
            "openRouterRouting": {
              "only": ["anthropic", "amazon-bedrock"]
            }
          }
        }
      ]
    }
  }
}
```

### What each key means

| Key | Level | Effect |
| :-- | :---- | :----- |
| `baseUrl` | provider/model | API endpoint URL |
| `api` | provider/model | API standard: `openai-completions`, `openai-responses`, `anthropic-messages`, `google-generative-ai`, etc. |
| `apiKey` | provider | API key literal, env interpolation (`$VAR`), command (`!cmd`), or omitted to use `/login`/`auth.json`/`--api-key` |
| `authHeader` | provider | When `true`, sends `Authorization: Bearer <apiKey>` |
| `headers` | provider/model | Custom headers with the same value-resolution syntax as `apiKey` |
| `models` | provider | Array of model entries |

### Adapter mechanism

There is no npm-package-style adapter key. Pi selects the wire protocol through the `api` field and applies compatibility shims through `compat`. The `api` value determines which internal streaming implementation is used.

### Per-model metadata

| Metadata key | Meaning |
| :----------- | :------ |
| `id` | Model identifier passed to the API |
| `name` | Human-readable label used for matching and secondary detail text |
| `api` | Override the provider's API for this model |
| `reasoning` | Whether the model supports extended thinking |
| `thinkingLevelMap` | Maps Pi thinking levels to provider values; `null` hides unsupported levels |
| `input` | Input modalities: `["text"]` or `["text", "image"]` |
| `contextWindow` | Context window in tokens (default 128000) |
| `maxTokens` | Maximum output tokens (default 16384) |
| `cost` | Per-million-token costs: `input`, `output`, `cacheRead`, `cacheWrite` |
| `compat` | API-specific compatibility overrides |
| `headers` | Per-model custom headers |
| `baseUrl` | Per-model endpoint override |
| `modelOverrides` | Per-model overrides for built-in models without replacing the full model list |

### Interaction with the built-in catalog

User-added models use **shadow** semantics within a provider:

- Built-in models are kept.
- Custom models are upserted by `id`.
- A custom model with the same `id` as a built-in model replaces the built-in entry.
- New `id`s are added alongside built-in models.

Because Pi's built-in catalog updates with each release, a manual block for a model that later becomes built-in should be removed to avoid stale overrides. Pi does not automate this cleanup; the user must delete the entry from `models.json`.

### Cross-cloud bridging

Pi can be routed at a different cloud vendor's API by overriding `baseUrl` in `models.json`, provided the target speaks an API standard Pi's client understands (OpenAI-compatible, Anthropic-compatible, etc.). If the target's native API does not match one of those standards, put a translation proxy (for example LiteLLM or a custom extension with `streamSimple`) between Pi and the upstream.

```json
{
  "providers": {
    "openrouter": {
      "baseUrl": "https://openrouter.ai/api/v1",
      "apiKey": "$OPENROUTER_API_KEY",
      "api": "openai-completions",
      "models": [
        { "id": "openrouter/anthropic/claude-3.5-sonnet" }
      ]
    }
  }
}
```

## Adding Local Models

Local-runner support is a property of **API-standard bridging**, not of Pi "knowing about" a runner. Most runners expose an OpenAI-compatible endpoint, and some also expose an Anthropic-compatible one, so any provider that allows a base-URL override can use them. The question is never "does Pi support Ollama" — it is "which API standards can Pi's model client speak, and how is its base URL redirected to a local endpoint?"

| Runner | Integration path | Notes |
| :----- | :--------------- | :---- |
| Ollama | `base_url_override` on `openai-completions` | `http://localhost:11434/v1`; use dummy `apiKey` because Ollama ignores it |
| oMLX | `first_class` via `omlx launch pi` | The hook launches Pi against the running server; manual fallback is `base_url_override` on `http://127.0.0.1:8000/v1` (observed host config) |
| LM Studio | `base_url_override` on `openai-completions` | `http://localhost:1234/v1` |
| llama.cpp | `base_url_override` on `openai-completions` | Any port the server exposes `/v1` on |
| vLLM | `base_url_override` on `openai-completions` | `http://localhost:8000/v1`; commonly needs `compat.supportsDeveloperRole: false` |
| Other | `base_url_override` or extension | Works if the runner speaks a supported API; otherwise implement `streamSimple` |

### Practical example: Ollama

```json
{
  "providers": {
    "ollama": {
      "baseUrl": "http://localhost:11434/v1",
      "api": "openai-completions",
      "apiKey": "ollama",
      "compat": {
        "supportsDeveloperRole": false,
        "supportsReasoningEffort": false
      },
      "models": [
        { "id": "llama3.1:8b" },
        { "id": "qwen2.5-coder:7b" }
      ]
    }
  }
}
```

Model ids include the size/quantization tag, such as `llama3.1:8b` or `gemma3:27b`. These tags are opaque to Pi; they are passed through to the runner's API.

### Practical example: oMLX (observed host config)

```json
{
  "providers": {
    "omlx": {
      "baseUrl": "http://127.0.0.1:8000/v1",
      "api": "openai-completions",
      "apiKey": "VPZ-77XNJozqp4o",
      "authHeader": true,
      "models": [
        {
          "id": "Qwen3.6-35B-A3B-oQ6",
          "name": "Qwen3.6-35B-A3B-oQ6",
          "reasoning": false,
          "input": ["text", "image"],
          "cost": { "input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0 },
          "contextWindow": 98304,
          "maxTokens": 98304
        }
      ]
    }
  }
}
```

## Environment Overrides

Pi resolves auth and some endpoint settings from environment variables. The resolution order for credentials is:

1. CLI `--api-key`
2. `~/.pi/agent/auth.json`
3. Environment variable
4. Custom provider `apiKey` in `models.json`

| Variable | Effect |
| :------- | :----- |
| `PI_CODING_AGENT_DIR` | Relocates the directory where `models.json`, `settings.json`, `auth.json`, and `trust.json` are loaded |
| `PI_CODING_AGENT_SESSION_DIR` | Overrides the session storage directory |
| `PI_PACKAGE_DIR` | Overrides the package directory |
| `ANTHROPIC_API_KEY` | API key for the built-in Anthropic provider |
| `OPENAI_API_KEY` | API key for the built-in OpenAI provider |
| `GEMINI_API_KEY` | API key for the built-in Google provider and `google-generative-ai` endpoints |
| `PI_OFFLINE` | Disables startup network operations |
| `PI_SKIP_VERSION_CHECK` | Skips the Pi version update check |
| `PI_TELEMETRY` | Overrides install/update telemetry and provider attribution headers |
| `PI_CACHE_RETENTION` | Set to `long` for extended prompt cache where supported |
| `VISUAL` / `EDITOR` | Fallback external editor for Ctrl+G |

Variables can also be referenced inside `models.json` via `$VAR` or `${VAR}` interpolation in `apiKey`, `headers`, and credential `key` values.

## Changelog

- **2026-07-02** — Added `cloud_bridge` frontmatter and body section documenting cross-vendor API routing through `baseUrl` overrides.
- **2026-07-02** — Reclassified local runners from a `supported` boolean to the `integration` enum. oMLX ships a first-class `omlx launch pi` hook; Ollama, LM Studio, llama.cpp, and vLLM are `base_url_override` paths on OpenAI-compatible endpoints.
- **2026-07-02** — Expanded `api_standards` notes to cover `openai-responses`, `google-vertex`, `azure-openai-responses`, `openai-codex-responses`, `mistral-conversations`, and `bedrock-converse-stream`.
- **2026-07-02** — Added `modelOverrides` to the `metadata_overrides` list.
- **2026-07-02** — Updated environment variables to match Pi 0.73.1 usage docs.
- **2026-07-02** — Observed host Pi version is 0.73.1.

## Sources

- [Pi project website](https://pi.dev/)
- [Pi GitHub repository](https://github.com/earendil-works/pi)
- [Pi documentation index](https://pi.dev/docs/latest)
- [Pi custom models documentation](https://pi.dev/docs/latest/models)
- [Pi custom providers documentation](https://pi.dev/docs/latest/custom-provider)
- [Pi settings documentation](https://pi.dev/docs/latest/settings)
- [Pi usage documentation](https://pi.dev/docs/latest/usage)
- [Host `~/.pi/agent/models.json` observed on this host](file:///Users/ken/.pi/agent/models.json)
- [Host `~/.pi/agent/settings.json` observed on this host](file:///Users/ken/.pi/agent/settings.json)

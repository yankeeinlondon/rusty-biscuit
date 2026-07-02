---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
has_official_schema: none

config_files:
  - scope: user
    path: ~/.pi/agent/models.json
    format: json
    notes: 'Primary file for adding custom cloud and local providers/models. Reloaded each time /model is opened; no restart needed. Observed on this host with an omlx provider entry using api: openai-completions.'
  - scope: user
    path: ~/.pi/agent/settings.json
    format: json
    notes: 'Global user settings. Pins defaultProvider and defaultModel. Observed on this host with "defaultProvider": "omlx" and "defaultModel": "Qwen3.6-35B-A3B-oQ6".'
  - scope: repo
    path: .pi/settings.json
    format: json
    notes: 'Project-local settings that override global settings. Nested objects are merged. Subject to project trust flow on interactive startup.'
  - scope: user
    path: ~/.pi/agent/auth.json
    format: json
    notes: 'Credential store for API keys and OAuth tokens. Not a model-config file per se, but credentials stored here unlock models declared in models.json. Created with 0600 permissions.'

api_standards:
  - standard: openai_compatible
    base_url_site: baseUrl in ~/.pi/agent/models.json (provider or model level)
    auth_site: apiKey in models.json, or /login/auth.json, or provider-specific env var such as OPENAI_API_KEY
    notes: 'Most common path for local runners and proxies. Supports OpenAI Chat Completions and Responses API shapes.'
  - standard: anthropic_compatible
    base_url_site: baseUrl in ~/.pi/agent/models.json
    auth_site: apiKey in models.json, or /login/auth.json, or ANTHROPIC_API_KEY
    notes: 'Use api: anthropic-messages for Anthropic-compatible proxies or custom endpoints. Compat flags control eager tool streaming, cache retention, adaptive thinking, and empty signatures.'
  - standard: bespoke
    base_url_site: baseUrl in ~/.pi/agent/models.json
    auth_site: apiKey in models.json, or /login/auth.json, or GEMINI_API_KEY
    notes: 'google-generative-ai API type for Google AI Studio and custom Gemma entries. Requires a baseUrl when adding custom models.'

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

merge_semantics: shadow

local_runners:
  - runner: ollama
    supported: openai_compatible
    example: '{ "providers": { "ollama": { "baseUrl": "http://localhost:11434/v1", "api": "openai-completions", "apiKey": "ollama", "models": [{ "id": "llama3.1:8b" }] } } }'
    notes: 'Treats apiKey as a placeholder because Ollama ignores it. Use compat.supportsDeveloperRole: false for servers that do not understand the developer role.'
  - runner: omlx
    supported: openai_compatible
    example: '{ "providers": { "omlx": { "baseUrl": "http://127.0.0.1:8000/v1", "api": "openai-completions", "apiKey": "VPZ-77XNJozqp4o", "authHeader": true, "models": [{ "id": "Qwen3.6-35B-A3B-oQ6", "name": "Qwen3.6-35B-A3B-oQ6", "reasoning": false, "input": ["text", "image"], "cost": {"input":0,"output":0,"cacheRead":0,"cacheWrite":0}, "contextWindow": 98304, "maxTokens": 98304 }] } } }'
    notes: 'Observed in the host ~/.pi/agent/models.json. Uses the OpenAI-compatible endpoint exposed by oMLX.'
  - runner: lmstudio
    supported: openai_compatible
    example: '{ "providers": { "lmstudio": { "baseUrl": "http://localhost:1234/v1", "api": "openai-completions", "apiKey": "lmstudio", "models": [{ "id": "qwen2.5-coder-32b-instruct" }] } } }'
    notes: 'Routes through LM Studio''s OpenAI-compatible local server.'
  - runner: llamacpp
    supported: openai_compatible
    example: '{ "providers": { "llamacpp": { "baseUrl": "http://localhost:8080/v1", "api": "openai-completions", "apiKey": "llamacpp", "models": [{ "id": "llama-3-70b" }] } } }'
    notes: 'Works via llama.cpp''s OpenAI-compatible HTTP server. Size/quantization tags are part of the model id string.'
  - runner: vllm
    supported: openai_compatible
    example: '{ "providers": { "vllm": { "baseUrl": "http://localhost:8000/v1", "api": "openai-completions", "apiKey": "vllm", "compat": { "supportsDeveloperRole": false, "supportsReasoningEffort": false }, "models": [{ "id": "qwen2.5-72b" }] } } }'
    notes: 'Use the /v1 OpenAI-compatible endpoint. Compat flags are commonly required because vLLM does not support the developer role or reasoning_effort natively.'
  - runner: other
    supported: openai_compatible
    notes: 'Any runner exposing an OpenAI-compatible, Anthropic-compatible, or Google Generative AI endpoint can be registered in models.json.'

default_model_site: defaultProvider and defaultModel keys in ~/.pi/agent/settings.json (or .pi/settings.json); session override via --provider and --model flags

env_vars:
  - name: PI_CODING_AGENT_DIR
    effect: 'Relocates the config directory where models.json, settings.json, and auth.json are loaded. Default is ~/.pi/agent.'
  - name: ANTHROPIC_API_KEY
    effect: 'Provides the API key for the built-in anthropic provider. Resolution order: --api-key > auth.json > env var > custom provider apiKey in models.json.'
  - name: OPENAI_API_KEY
    effect: 'Provides the API key for the built-in openai provider and OpenAI-compatible endpoints.'
  - name: GEMINI_API_KEY
    effect: 'Provides the API key for the built-in google provider and google-generative-ai custom endpoints.'
  - name: AZURE_OPENAI_BASE_URL
    effect: 'Redirects Azure OpenAI requests to a specific resource endpoint. Alternative to AZURE_OPENAI_RESOURCE_NAME.'
  - name: AZURE_OPENAI_DEPLOYMENT_NAME_MAP
    effect: 'Maps OpenAI model IDs to Azure deployment names, e.g. gpt-4=my-gpt4,gpt-4o=my-gpt4o.'
  - name: AWS_ENDPOINT_URL_BEDROCK_RUNTIME
    effect: 'Redirects Amazon Bedrock runtime requests to a proxy endpoint.'
  - name: GOOGLE_CLOUD_PROJECT
    effect: 'Selects the Google Cloud project for Vertex AI built-in provider.'
  - name: GOOGLE_CLOUD_LOCATION
    effect: 'Selects the Google Cloud region for Vertex AI built-in provider.'
  - name: PI_OFFLINE
    effect: 'Disables startup network operations (update checks, package checks, telemetry). Does not change endpoint selection.'

changes: []

requires_claudine_update: true
reason: 'Pi is not currently one of Claudine''s wrapped providers. If Claudine adds Pi support, it will need a new provider adapter, models.json/catalog integration, and credential/env resolution matching Pi''s config layout.'
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
| `api` | provider/model | API standard: `openai-completions`, `openai-responses`, `anthropic-messages`, or `google-generative-ai` |
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

### Interaction with the built-in catalog

User-added models use **shadow** semantics within a provider:

- Built-in models are kept.
- Custom models are upserted by `id`.
- A custom model with the same `id` as a built-in model replaces the built-in entry.
- New `id`s are added alongside built-in models.

Because Pi's built-in catalog updates with each release, a manual block for a model that later becomes built-in should be removed to avoid stale overrides. Pi does not automate this cleanup; the user must delete the entry from `models.json`.

## Adding Local Models

Local model runners are supported through Pi's OpenAI-compatible shim. None are first-class native integrations; each is configured by pointing `baseUrl` at its OpenAI-compatible endpoint.

| Runner | Support path | Notes |
| :----- | :----------- | :---- |
| Ollama | `openai-completions` | `http://localhost:11434/v1`; use dummy `apiKey` because Ollama ignores it |
| oMLX | `openai-completions` | Observed host config uses `http://127.0.0.1:8000/v1` |
| LM Studio | `openai-completions` | `http://localhost:1234/v1` |
| llama.cpp | `openai-completions` | Any port the server exposes `/v1` on |
| vLLM | `openai-completions` | `http://localhost:8000/v1`; commonly needs `compat.supportsDeveloperRole: false` |

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
| `PI_CODING_AGENT_DIR` | Relocates the directory where `models.json`, `settings.json`, and `auth.json` are loaded |
| `ANTHROPIC_API_KEY` | API key for the built-in Anthropic provider |
| `OPENAI_API_KEY` | API key for the built-in OpenAI provider |
| `GEMINI_API_KEY` | API key for the built-in Google provider and `google-generative-ai` endpoints |
| `AZURE_OPENAI_BASE_URL` | Redirects Azure OpenAI requests to a specific resource |
| `AZURE_OPENAI_DEPLOYMENT_NAME_MAP` | Maps model IDs to Azure deployment names |
| `AWS_ENDPOINT_URL_BEDROCK_RUNTIME` | Redirects Bedrock runtime requests to a proxy |
| `GOOGLE_CLOUD_PROJECT` | Selects the Vertex AI project |
| `GOOGLE_CLOUD_LOCATION` | Selects the Vertex AI region |
| `PI_OFFLINE` | Disables startup network operations |

Variables can also be referenced inside `models.json` via `$VAR` or `${VAR}` interpolation in `apiKey`, `headers`, and credential `key` values.

## Sources

- [Pi project website](https://pi.dev/)
- [Pi GitHub repository](https://github.com/earendil-works/pi)
- [Pi documentation index](https://pi.dev/docs/latest)
- [Pi custom models documentation](https://pi.dev/docs/latest/models)
- [Pi custom providers documentation](https://pi.dev/docs/latest/custom-provider)
- [Pi settings documentation](https://pi.dev/docs/latest/settings)
- [Pi providers documentation](https://pi.dev/docs/latest/providers)
- [Host `~/.pi/agent/models.json` observed on this host](file:///Users/ken/.pi/agent/models.json)
- [Host `~/.pi/agent/settings.json` observed on this host](file:///Users/ken/.pi/agent/settings.json)

---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
has_official_schema: formal
schema_url: https://opencode.ai/config.json
model_config_paths:
  - scope: user
    path: ~/.config/opencode/opencode.json
    format: jsonc
    notes: 'Global user config. Also accepts opencode.jsonc. Observed on this host with a built-in Z.AI provider block and model metadata.'
  - scope: user
    path: ~/.config/opencode/opencode.jsonc
    format: jsonc
    notes: 'JSON-with-comments variant of the global config.'
  - scope: repo
    path: opencode.json
    format: jsonc
    notes: 'Project config in repository root. Also accepts opencode.jsonc. OpenCode walks up from cwd to the nearest Git root.'
  - scope: repo
    path: opencode.jsonc
    format: jsonc
    notes: 'JSON-with-comments variant of the project config.'
  - scope: env
    path: OPENCODE_CONFIG
    format: jsonc
    notes: 'Custom config file path loaded between global and project configs.'
  - scope: env
    path: OPENCODE_CONFIG_CONTENT
    format: json
    notes: 'Inline JSON config merged as a final local-scope override.'
  - scope: env
    path: "/Library/Application Support/opencode/opencode.json (macOS), /etc/opencode/opencode.json (Linux), %ProgramData%\\opencode\\opencode.json (Windows)"
    format: jsonc
    notes: 'Managed/organizational settings loaded at highest priority and not user-overridable.'
  - scope: env
    path: ai.opencode.managed plist / .mobileconfig (macOS MDM)
    format: jsonc
    notes: 'macOS managed preferences deployed via MDM; highest priority, not user-overridable.'
api_standards:
  - standard: openai_compatible
    base_url_site: provider.<id>.options.baseURL
    auth_site: provider.<id>.options.apiKey or {env:VARIABLE_NAME}
    adapter: '@ai-sdk/openai-compatible'
    notes: 'Officially documented path for user-added cloud/local models. Use @ai-sdk/openai-compatible for /v1/chat/completions endpoints and @ai-sdk/openai for /v1/responses endpoints.'
  - standard: bespoke
    base_url_site: provider.<id>.options.baseURL
    auth_site: provider.<id>.options.apiKey or {env:VARIABLE_NAME}
    adapter: 'npm key naming any AI SDK provider package (e.g. @ai-sdk/google, @ai-sdk/anthropic if installed)'
    notes: 'OpenCode can load arbitrary AI SDK packages via npm, but custom providers are officially documented only as OpenAI-compatible.'
metadata_overrides:
  - id
  - name
  - family
  - release_date
  - attachment
  - reasoning
  - temperature
  - tool_call
  - interleaved
  - cost
  - limit
  - modalities
  - experimental
  - status
  - provider
  - options
  - headers
  - variants
merge_semantics: merge
local_runners:
  - runner: ollama
    integration: first_class
    standard: openai_compatible
    example: '{ "provider": { "ollama": { "npm": "@ai-sdk/openai-compatible", "name": "Ollama (local)", "options": { "baseURL": "http://localhost:11434/v1" }, "models": { "gemma3:27b": { "name": "Gemma 3 27B", "limit": { "context": 128000, "output": 8192 } } } } }, "model": "ollama/gemma3:27b" }'
    notes: 'First-class `ollama launch opencode` integration. Manual setup uses the OpenAI-compatible server endpoint. Size/quantization tags are part of the model ID.'
  - runner: omlx
    integration: first_class
    standard: openai_compatible
    example: '{ "provider": { "omlx": { "npm": "@ai-sdk/openai-compatible", "name": "oMLX (local)", "options": { "baseURL": "http://localhost:8000/v1" }, "models": { "Qwen3.6-35B-A3B-oQ6": { "name": "Qwen3.6 35B A3B oQ6 (local)" } } } }, "model": "omlx/Qwen3.6-35B-A3B-oQ6" }'
    notes: 'First-class `omlx launch opencode` integration configures and launches OpenCode against the running oMLX server. Manual setup uses the OpenAI-compatible endpoint served by `omlx serve`; auth is optional — pass the key in options.apiKey or {env:VAR} if enabled.'
  - runner: lmstudio
    integration: base_url_override
    standard: openai_compatible
    example: '{ "provider": { "lmstudio": { "npm": "@ai-sdk/openai-compatible", "name": "LM Studio (local)", "options": { "baseURL": "http://127.0.0.1:1234/v1" }, "models": { "google/gemma-3n-e4b": { "name": "Gemma 3n-e4b (local)" } } } }, "model": "lmstudio/google/gemma-3n-e4b" }'
    notes: 'Uses the OpenAI-compatible server endpoint loaded by LM Studio. Set options.apiKey to the configured token when authentication is enabled.'
  - runner: llamacpp
    integration: base_url_override
    standard: openai_compatible
    example: '{ "provider": { "llamacpp": { "npm": "@ai-sdk/openai-compatible", "name": "llama-server (local)", "options": { "baseURL": "http://127.0.0.1:8080/v1" }, "models": { "qwen3-coder:a3b": { "name": "Qwen3-Coder a3b (local)", "limit": { "context": 128000, "output": 65536 } } } } }, "model": "llamacpp/qwen3-coder:a3b" }'
    notes: 'Uses llama.cpp llama-server OpenAI-compatible endpoint. Pass options.apiKey or {env:LLAMA_API_KEY} when --api-key is set.'
  - runner: vllm
    integration: base_url_override
    standard: openai_compatible
    example: '{ "provider": { "vllm": { "npm": "@ai-sdk/openai-compatible", "name": "vLLM (local)", "options": { "baseURL": "http://localhost:8000/v1" }, "models": { "qwen2.5-coder-32b-instruct": { "name": "Qwen2.5 Coder 32B Instruct (local)" } } } }, "model": "vllm/qwen2.5-coder-32b-instruct" }'
    notes: 'vLLM exposes an OpenAI-compatible API by default. Each vLLM process hosts one model; use --served-model-name aliases if needed.'
  - runner: other
    integration: base_url_override
    standard: openai_compatible
    example: '{ "provider": { "my-local": { "npm": "@ai-sdk/openai-compatible", "name": "My local runner", "options": { "baseURL": "http://localhost:<port>/v1" }, "models": { "<runner-model-id>": { "name": "My local model" } } } }, "model": "my-local/<runner-model-id>" }'
    notes: 'Any local server that exposes an OpenAI-compatible /v1 chat endpoint works with @ai-sdk/openai-compatible.'
cloud_bridge:
  supported: true
  mechanism: 'provider.<id>.options.baseURL with npm adapter @ai-sdk/openai-compatible (or @ai-sdk/openai for /v1/responses)'
  example: |
    // Direct: target cloud exposes an OpenAI-compatible API
    {
      "provider": {
        "openrouter": {
          "npm": "@ai-sdk/openai-compatible",
          "name": "OpenRouter",
          "options": { "baseURL": "https://openrouter.ai/api/v1", "apiKey": "{env:OPENROUTER_API_KEY}" },
          "models": { "anthropic/claude-sonnet-4": { "name": "Claude Sonnet 4 (via OpenRouter)" } }
        }
      },
      "model": "openrouter/anthropic/claude-sonnet-4"
    }

    // Proxy required: target cloud speaks only a non-OpenAI API (e.g. Anthropic Messages)
    // Run a LiteLLM proxy that translates Anthropic Messages <-> OpenAI /v1/chat/completions,
    // then point OpenCode at the proxy baseURL with @ai-sdk/openai-compatible.
default_model_site: 'Top-level model key in opencode.json/opencode.jsonc (user/project/managed); session override via --model / -m CLI flag.'
env_vars:
  - name: OPENCODE_CONFIG
    effect: 'Load an additional config file between global and project configs.'
  - name: OPENCODE_CONFIG_CONTENT
    effect: 'Inject inline JSON as a final local-scope config override.'
  - name: OPENCODE_MODELS_URL
    effect: 'Custom URL for fetching remote model configuration/catalog.'
  - name: OPENCODE_ENABLE_EXPERIMENTAL_MODELS
    effect: 'Surface experimental models in the model picker.'
  - name: OPENCODE_DISABLE_MODELS_FETCH
    effect: 'Prevent fetching models from remote sources.'
  - name: Provider API keys (e.g. OPENAI_API_KEY, ANTHROPIC_API_KEY, etc.)
    effect: 'Enable the corresponding built-in provider and its models. Can also be referenced via {env:VAR} in provider.<id>.options.apiKey.'
changes:
  - 'Removed the documented `anthropic_compatible` custom-provider standard; OpenCode official docs only document OpenAI-compatible adapters (@ai-sdk/openai-compatible for /v1/chat/completions, @ai-sdk/openai for /v1/responses) for user-added providers.'
  - 'Reclassified oMLX from unsupported to first_class (via `omlx launch opencode`), with manual OpenAI-compatible base-URL setup on http://localhost:8000/v1, consistent with local-runner ground truth.'
  - 'Confirmed Ollama first-class integration via `ollama launch opencode` and `ollama launch opencode --config`.'
  - 'Updated cloud-bridge guidance: OpenCode can be routed at any OpenAI-compatible cloud endpoint directly; non-OpenAI-compatible vendor APIs require a translation proxy such as LiteLLM.'
  - 'Added managed macOS MDM preferences (ai.opencode.managed) to model_config_paths; dropped OPENCODE_CONFIG_DIR because it does not accept model configuration directly.'
requires_claudine_update: true
reason: 'Claudine should treat Ollama and oMLX as first-class OpenCode integrations (`ollama launch opencode`, `omlx launch opencode`) and LM Studio, llama.cpp, and vLLM as valid OpenAI-compatible base-URL-override targets for OpenCode, rather than unsupported or Anthropic-compatible paths.'
---

# OpenCode CLI User-Side Model Configuration

## Introduction to OpenCode CLI Model Configuration

OpenCode stores runtime configuration in JSON or JSONC files. Model-related settings live in the same file as everything else; there is no separate model manifest.

| Scope | Path | Format | Notes |
| :---- | :--- | :----- | :---- |
| Remote / organizational | `.well-known/opencode` endpoint | JSON | Loaded first as a base layer; overridden by user/project config. Fetched automatically when authenticating with a provider that supports it. |
| Global user | `~/.config/opencode/opencode.json` or `opencode.jsonc` | JSONC | User-wide defaults. The inspected host uses `~/.config/opencode/opencode.jsonc`. |
| Project | `opencode.json`, `opencode.jsonc` in the repo root | JSONC | Walks up from the cwd to the nearest Git root. Overrides global config. |
| Custom file | `OPENCODE_CONFIG` env var | JSONC | Loaded between global and project configs. |
| Inline override | `OPENCODE_CONFIG_CONTENT` env var | JSON | Merged as a final local-scope override. |
| Managed / file | `/Library/Application Support/opencode/` (macOS), `/etc/opencode/` (Linux), `%ProgramData%\opencode` (Windows) | JSONC | Highest priority; not user-overridable. |
| Managed / MDM | macOS `.mobileconfig` with `ai.opencode.managed` payload | JSONC | Highest priority; not user-overridable. |

A formal JSON Schema is published at [`https://opencode.ai/config.json`](https://opencode.ai/config.json). OpenCode validates config strictly at startup, so declaring `"$schema": "https://opencode.ai/config.json"` is recommended.

## Adding Cloud Models

OpenCode ships with a large catalog of providers via the AI SDK and [Models.dev](https://models.dev). To add a model or provider that is not preloaded, define a custom provider block under `provider.<provider_id>`.

### Concrete example

Add a hypothetical cloud provider that exposes an OpenAI-compatible API:

```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "model": "acme-corp/qwen3-coder-480b",
  "provider": {
    "acme-corp": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "Acme Corp",
      "options": {
        "baseURL": "https://llm.acme.example.com/v1",
        "apiKey": "{env:ACME_API_KEY}"
      },
      "models": {
        "qwen3-coder-480b": {
          "name": "Qwen3 Coder 480B (Acme)",
          "reasoning": true,
          "modalities": { "input": ["text"], "output": ["text"] },
          "cost": { "input": 2.0, "output": 6.0 },
          "limit": { "context": 128000, "output": 32768 }
        }
      }
    }
  }
}
```

What each key means:

| Key | Meaning |
| :-- | :------ |
| `provider.<id>` | The provider namespace. The model ID becomes `<provider_id>/<model_id>`. |
| `npm` | The AI SDK provider package to load. This is the adapter mechanism. |
| `name` | Display name for the provider in the UI. |
| `options.baseURL` | Endpoint for this provider. |
| `options.apiKey` | API key. Can be a literal string or `{env:VAR}` substitution. |
| `options.headers` | Extra HTTP headers sent with every request. |
| `models.<id>` | A model entry. The full model reference is `<provider_id>/<id>`. |

### API standards and adapters

OpenCode's officially documented custom-provider path is **OpenAI-compatible**. The `npm` field selects the AI SDK package that speaks that standard.

| Standard | Adapter (`npm`) | Base URL | Auth |
| :------- | :-------------- | :------- | :--- |
| OpenAI-compatible (`/v1/chat/completions`) | `@ai-sdk/openai-compatible` | `provider.<id>.options.baseURL` | `provider.<id>.options.apiKey` or `{env:VAR}` |
| OpenAI-compatible (`/v1/responses`) | `@ai-sdk/openai` | `provider.<id>.options.baseURL` | `provider.<id>.options.apiKey` or `{env:VAR}` |
| Bespoke / provider-native | Any AI SDK package, e.g. `@ai-sdk/google`, `@ai-sdk/anthropic` if installed | `provider.<id>.options.baseURL` | Provider-specific, often via `apiKey` or env vars |

The observed host config does not set `npm` for its `zai-coding-plan` provider, which implies that provider is already known to OpenCode (built-in or from Models.dev). For providers OpenCode does not already know about, `npm` is required.

### Per-model metadata

The published schema and the inspected host config agree on the following override keys:

| Key | Purpose |
| :-- | :------ |
| `id` | Override the model ID sent on the wire. |
| `name` | Display name in the model picker. |
| `family` | Model family grouping. |
| `release_date` | Release date string. |
| `attachment` | Whether the model accepts file attachments. |
| `reasoning` | Whether the model supports reasoning/thinking output. |
| `temperature` | Whether temperature can be set. |
| `tool_call` | Whether the model supports tool calling. |
| `interleaved` | Reasoning field mapping for providers that stream reasoning separately. |
| `cost` | Per-token pricing: `input`, `output`, optional `cache_read`, `cache_write`, and `context_over_200k`. |
| `limit` | `context`, optional `input`, and `output` token limits. |
| `modalities` | `input` and `output` arrays of `text`, `audio`, `image`, `video`, `pdf`. |
| `experimental` | Flag the model as experimental. |
| `status` | `alpha`, `beta`, `deprecated`, or `active`. |
| `provider` | Per-model provider override (`npm`, `api`). |
| `options` | Provider-specific request options (e.g. `reasoningEffort`, `thinking`). |
| `headers` | Extra HTTP headers for this model. |
| `variants` | Variant-specific configuration and disabling. |

### Interaction with the built-in catalog

User-defined provider and model blocks **merge** with the built-in catalog and remote Models.dev data. Non-conflicting keys are preserved; conflicting keys are overridden by the user's config. This means:

- Adding a new model under a built-in provider (e.g. `provider.openrouter.models`) appends it to the picker.
- Defining a model with the same ID as a built-in entry shadows the built-in metadata.
- `blacklist` and `whitelist` on a provider hide built-in models without removing them from the catalog source.

Because OpenCode fetches model metadata from remote sources and updates its catalog over time, a manual block for a model that later becomes built-in should be removed to avoid stale overrides. OpenCode does not auto-remove or warn about these stale blocks.

### Cross-cloud bridging

OpenCode can be pointed at a different cloud vendor's API, but only through the **OpenAI-compatible** standard that its custom provider client speaks.

**Direct base URL — target exposes an OpenAI-compatible API:**

```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "model": "openrouter/anthropic/claude-sonnet-4",
  "provider": {
    "openrouter": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "OpenRouter",
      "options": {
        "baseURL": "https://openrouter.ai/api/v1",
        "apiKey": "{env:OPENROUTER_API_KEY}"
      },
      "models": {
        "anthropic/claude-sonnet-4": {
          "name": "Claude Sonnet 4 (via OpenRouter)"
        }
      }
    }
  }
}
```

**Translation proxy required — target speaks only a non-OpenAI API:**

If the target cloud's native API is not OpenAI-compatible (for example, Anthropic's Messages API), do not point `baseURL` directly at it. Run a proxy such as [LiteLLM](https://docs.litellm.ai/docs/) that translates between Anthropic Messages and OpenAI `/v1/chat/completions`, then configure OpenCode against the proxy:

```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "model": "litellm/claude-sonnet-4",
  "provider": {
    "litellm": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "LiteLLM Proxy",
      "options": {
        "baseURL": "https://litellm.example.com/v1",
        "apiKey": "{env:LITELLM_API_KEY}"
      },
      "models": {
        "claude-sonnet-4": {
          "name": "Claude Sonnet 4 (via LiteLLM)"
        }
      }
    }
  }
}
```

## Adding Local Models

Local model support is a property of **API-standard bridging**, not of OpenCode "knowing about" a runner. OpenCode's documented custom-provider path is OpenAI-compatible (`@ai-sdk/openai-compatible`), so any local runner that exposes an OpenAI-compatible `/v1` chat endpoint can be wired by setting `options.baseURL` to the runner's URL.

| Runner | Integration path | Notes |
| :----- | :--------------- | :---- |
| Ollama | First-class | `ollama launch opencode` configures and launches OpenCode automatically. Manual setup uses `http://localhost:11434/v1`. |
| oMLX | First-class | `omlx launch opencode` configures and launches OpenCode automatically. Manual setup uses `http://localhost:8000/v1`; auth is optional. |
| LM Studio | Base-URL override | Start the LM Studio server, then use `http://127.0.0.1:1234/v1`. |
| llama.cpp | Base-URL override | Start `llama-server`, then use `http://127.0.0.1:8080/v1`. |
| vLLM | Base-URL override | `vllm serve` exposes `http://localhost:8000/v1` by default. |
| Other | Base-URL override | Any local server with an OpenAI-compatible `/v1/chat/completions` endpoint works with `@ai-sdk/openai-compatible`. |

### Ollama example

```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "model": "ollama/gemma3:27b",
  "provider": {
    "ollama": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "Ollama (local)",
      "options": { "baseURL": "http://localhost:11434/v1" },
      "models": {
        "gemma3:27b": {
          "name": "Gemma 3 27B",
          "limit": { "context": 128000, "output": 8192 }
        }
      }
    }
  }
}
```

Model IDs include the Ollama tag (e.g. `:27b`), so the full reference is `ollama/gemma3:27b`.

### LM Studio example

```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "model": "lmstudio/google/gemma-3n-e4b",
  "provider": {
    "lmstudio": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "LM Studio (local)",
      "options": { "baseURL": "http://127.0.0.1:1234/v1" },
      "models": {
        "google/gemma-3n-e4b": {
          "name": "Gemma 3n-e4b (local)"
        }
      }
    }
  }
}
```

## Environment Overrides

OpenCode-specific environment variables that redirect model endpoints or selection:

| Variable | Effect |
| :------- | :----- |
| `OPENCODE_CONFIG` | Load an additional config file between global and project configs. |
| `OPENCODE_CONFIG_CONTENT` | Inject inline JSON as a final local-scope config override. |
| `OPENCODE_MODELS_URL` | Custom URL for fetching remote model configuration/catalog. |
| `OPENCODE_ENABLE_EXPERIMENTAL_MODELS` | Surface experimental models in the model picker. |
| `OPENCODE_DISABLE_MODELS_FETCH` | Prevent fetching models from remote sources. |

Provider credentials are usually picked up from provider-specific environment variables (e.g. `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`). These can also be referenced inside config with `{env:VAR}` substitutions, such as `"apiKey": "{env:ANTHROPIC_API_KEY}"`.

Session model selection follows this precedence (highest first):

1. `--model` / `-m` CLI flag.
2. `model` key in the effective config (project > custom > global > remote).
3. Last used model.
4. Internal default priority.

## Changelog

- **2026-07-02** — Removed the documented `anthropic_compatible` custom-provider standard. OpenCode officially documents only OpenAI-compatible adapters for user-added providers (`@ai-sdk/openai-compatible` for `/v1/chat/completions`, `@ai-sdk/openai` for `/v1/responses`).
- **2026-07-02** — Reclassified oMLX from unsupported to first-class (via `omlx launch opencode`), with manual OpenAI-compatible base-URL setup on `http://localhost:8000/v1`, consistent with local-runner ground truth.
- **2026-07-02** — Confirmed Ollama first-class integration via `ollama launch opencode` and `ollama launch opencode --config`.
- **2026-07-02** — Updated cloud-bridge guidance: OpenCode can be routed at any OpenAI-compatible cloud endpoint directly; non-OpenAI-compatible vendor APIs require a translation proxy such as LiteLLM.
- **2026-07-02** — Added managed macOS MDM preferences (`ai.opencode.managed`) to config files and dropped `OPENCODE_CONFIG_DIR` because it does not accept model configuration directly.

## Sources

- [OpenCode — Config](https://opencode.ai/docs/config)
- [OpenCode — Models](https://opencode.ai/docs/models)
- [OpenCode — Providers](https://opencode.ai/docs/providers)
- [OpenCode — CLI](https://opencode.ai/docs/cli)
- [OpenCode config JSON Schema](https://opencode.ai/config.json)
- [OpenCode repository](https://github.com/anomalyco/opencode)
- [Ollama OpenCode integration](https://docs.ollama.com/integrations/opencode)
- [AI SDK](https://ai-sdk.dev/)
- [Models.dev](https://models.dev)
- [LiteLLM proxy docs](https://docs.litellm.ai/docs/)

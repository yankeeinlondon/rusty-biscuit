---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
has_official_schema: formal
schema_url: https://app.kilo.ai/config.json
model_config_paths:
  - scope: user
    path: ~/.config/kilo/kilo.jsonc
    format: jsonc
    notes: 'Global user config. Observed on this host at this path and currently contains only a "$schema" reference to https://app.kilo.ai/config.json. Also accepts kilo.json.'
  - scope: user
    path: ~/.config/kilo/kilo.json
    format: json
    notes: 'JSON variant of the global config.'
  - scope: user
    path: ~/.config/kilo/tui.jsonc
    format: jsonc
    notes: 'TUI-specific settings (notifications, sounds, themes, keybindings) rather than model configuration.'
  - scope: repo
    path: ./kilo.jsonc
    format: jsonc
    notes: 'Project config in repository root. Overrides global config. Also accepts kilo.json.'
  - scope: repo
    path: ./.kilo/kilo.jsonc
    format: jsonc
    notes: 'Project config inside .kilo directory. Legacy .kilocode/ is also read.'
  - scope: repo
    path: ./.kilo/tui.jsonc
    format: jsonc
    notes: 'Project TUI settings.'
  - scope: env
    path: KILO_CONFIG
    format: jsonc
    notes: 'Custom config file path loaded between global and project configs (inherited OpenCode convention).'
  - scope: env
    path: KILO_CONFIG_CONTENT
    format: json
    notes: 'Inline JSON config merged as a final local-scope override (inherited OpenCode convention).'
  - scope: env
    path: KILO_CONFIG_DIR
    format: jsonc
    notes: 'Custom directory searched for agents, commands, modes, and plugins (inherited OpenCode convention).'
  - scope: env
    path: /Library/Application Support/kilo/ (macOS), /etc/kilo/ (Linux), %ProgramData%\kilo (Windows)
    format: jsonc
    notes: 'Managed/organizational settings loaded at highest priority and not user-overridable (inherited OpenCode convention).'
api_standards:
  - standard: openai_compatible
    base_url_site: provider.<id>.options.baseURL
    auth_site: provider.<id>.options.apiKey or {env:VARIABLE_NAME}
    adapter: '@ai-sdk/openai-compatible'
    notes: 'Primary path for custom/local models and OpenAI Chat Completions-compatible gateways. Kilo custom providers default to this unless another API is selected.'
  - standard: anthropic_compatible
    base_url_site: provider.<id>.options.baseURL
    auth_site: provider.<id>.options.apiKey or {env:VARIABLE_NAME}
    adapter: '@ai-sdk/anthropic'
    notes: 'Used for Anthropic Messages API providers (e.g. Anthropic, MiniMax). Set via the provider api/npm fields.'
  - standard: bespoke
    base_url_site: provider.<id>.options.baseURL
    auth_site: provider.<id>.options.apiKey or {env:VARIABLE_NAME}
    adapter: 'npm AI SDK provider package (e.g. @ai-sdk/openai for OpenAI Responses, @ai-sdk/google, @ai-sdk/azure)'
    notes: 'Provider-native protocols selected via the provider "api" or "npm" fields, such as OpenAI Responses for OpenAI/xAI models.'
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
    integration: base_url_override
    standard: openai_compatible
    example: '{"provider":{"ollama":{"npm":"@ai-sdk/openai-compatible","options":{"baseURL":"http://localhost:11434/v1"},"models":{"gemma3:27b":{"name":"Gemma 3 27B","limit":{"context":128000,"output":8192}}}}},"model":"ollama/gemma3:27b"}'
    notes: 'Ollama also exposes an Anthropic-compatible /v1/messages endpoint, but the documented Kilo path uses the OpenAI-compatible server. Size/quantization tags are part of the model ID.'
  - runner: lmstudio
    integration: base_url_override
    standard: openai_compatible
    example: '{"provider":{"lmstudio":{"npm":"@ai-sdk/openai-compatible","options":{"baseURL":"http://localhost:1234/v1"},"models":{"deepseek-r1-0528":{"name":"DeepSeek R1 0528"}}}},"model":"lmstudio/deepseek-r1-0528"}'
    notes: 'LM Studio also supports Anthropic Messages, but the documented Kilo path uses the OpenAI-compatible server loaded by LM Studio.'
  - runner: llamacpp
    integration: base_url_override
    standard: openai_compatible
    example: '{"provider":{"llamacpp":{"npm":"@ai-sdk/openai-compatible","options":{"baseURL":"http://127.0.0.1:8080/v1"},"models":{"qwen3-coder:a3b":{"name":"Qwen3-Coder a3b (local)","limit":{"context":128000,"output":65536}}}}},"model":"llamacpp/qwen3-coder:a3b"}'
    notes: 'llama-server also supports Anthropic Messages; the OpenAI-compatible endpoint is the standard Kilo example.'
  - runner: vllm
    integration: base_url_override
    standard: openai_compatible
    example: '{"provider":{"vllm":{"npm":"@ai-sdk/openai-compatible","options":{"baseURL":"http://localhost:8000/v1","apiKey":"EMPTY"},"models":{"Qwen/Qwen2.5-1.5B-Instruct":{"name":"Qwen2.5 1.5B Instruct (local vLLM)"}}}},"model":"vllm/Qwen/Qwen2.5-1.5B-Instruct"}'
    notes: 'vLLM also supports Anthropic Messages; the OpenAI-compatible endpoint is the standard Kilo example.'
  - runner: omlx
    integration: base_url_override
    standard: openai_compatible
    example: '{"provider":{"omlx":{"npm":"@ai-sdk/openai-compatible","options":{"baseURL":"http://localhost:8000/v1"},"models":{"Qwen3.6-35B-A3B-oQ6":{"name":"Qwen3.6-35B-A3B-oQ6 (local)","limit":{"context":262144,"output":98304}}}}},"model":"omlx/Qwen3.6-35B-A3B-oQ6"}'
    notes: 'oMLX also exposes an Anthropic-compatible /v1/messages endpoint; the OpenAI-compatible endpoint is the standard Kilo example.'
  - runner: other
    integration: base_url_override
    standard: openai_compatible
    example: '{"provider":{"my-local":{"npm":"@ai-sdk/openai-compatible","options":{"baseURL":"http://localhost:<port>/v1"},"models":{"<model-id>":{"name":"My local model"}}}},"model":"my-local/<model-id>"}'
    notes: 'Any local server exposing an OpenAI-compatible /v1 chat endpoint can be wired via @ai-sdk/openai-compatible. If the runner only speaks Anthropic Messages, use @ai-sdk/anthropic instead.'
cloud_bridge:
  supported: true
  mechanism: 'provider.<id>.options.baseURL plus the matching AI SDK adapter (the provider "npm" or "api" field).'
  example: |
    {
      "$schema": "https://app.kilo.ai/config.json",
      "provider": {
        "openai": {
          "options": {
            "baseURL": "https://litellm.example.com/v1",
            "apiKey": "{env:LITELLM_API_KEY}"
          }
        }
      },
      "model": "openai/gemini-2.5-pro"
    }
default_model_site: 'Top-level model key in kilo.jsonc/kilo.json (global/project/managed); session override via --model / -m CLI flag.'
env_vars:
  - name: KILO_PROVIDER
    effect: 'Override the active provider ID.'
  - name: KILO_<FIELD_NAME>
    effect: 'For non-kilocode providers, maps to provider options (e.g. KILO_API_KEY -> apiKey).'
  - name: KILOCODE_<FIELD_NAME>
    effect: 'For the kilocode provider, maps to provider options (e.g. KILOCODE_MODEL -> kilocodeModel).'
  - name: KILO_ORG_ID
    effect: 'Select the Kilo organization/team for non-interactive kilo run sessions.'
  - name: KILO_API_KEY
    effect: 'API key for the Kilo Gateway when using kilocode provider models.'
  - name: KILO_EXPERIMENTAL_OUTPUT_TOKEN_MAX
    effect: 'Overrides the default 32,000 fallback output-token limit when a custom/local model has output: 0.'
  - name: KILO_CONFIG
    effect: 'Load an additional config file between global and project configs.'
  - name: KILO_CONFIG_CONTENT
    effect: 'Inject inline JSON as a final local-scope config override.'
  - name: KILO_CONFIG_DIR
    effect: 'Use a custom directory for agents, commands, modes, and plugins.'
  - name: Provider API keys (e.g. OPENAI_API_KEY, ANTHROPIC_API_KEY, etc.)
    effect: 'Enable the corresponding built-in provider and its models. Can also be referenced via {env:VAR} in provider.<id>.options.apiKey.'
changes:
  - 'Confirmed Kilo CLI 1.0 is an OpenCode fork and uses the same kilo.jsonc/kilo.json config surface, precedence chain, and AI-SDK adapter model as OpenCode.'
  - 'User-added models live under provider.<provider_id>.models; the full model reference is provider_id/model_id.'
  - 'Reclassified local runners from "unsupported/first-class shim" to base-URL override paths: Ollama, LM Studio, llama.cpp, vLLM, and oMLX all work by pointing the OpenAI-compatible (or Anthropic-compatible) adapter at their local endpoints.'
  - 'oMLX is no longer unsupported; it serves OpenAI- and Anthropic-compatible endpoints and integrates the same way as other local runners.'
  - 'Kilo does not ship runner-native launch hooks (e.g. no "ollama launch kilo"); local integration is done through provider blocks.'
  - 'Documented the three API-standard paths for custom providers: OpenAI Chat Completions-compatible (default), Anthropic Messages-compatible, and provider-native/bespoke via the npm/api adapter fields.'
  - 'Updated metadata override keys and merge semantics: user config merges on top of the built-in catalog and hourly-refreshed models.dev data, shadowing same-ID entries.'
  - 'Added KILO_EXPERIMENTAL_OUTPUT_TOKEN_MAX and provider API-key env variables to the environment override list.'
requires_claudine_update: true
reason: 'Claudine should treat Kilo Code as a distinct provider with the OpenCode-derived config surface, support resolving provider.<id>.models blocks and the kilo.jsonc precedence chain, and surface local-runner base-URL overrides for Ollama, LM Studio, oMLX, llama.cpp, and vLLM rather than marking oMLX unsupported.'
---

# Kilo Code User-Side Model Configuration

## Introduction to Kilo Code Model Configuration

Kilo Code stores runtime configuration in JSON or JSONC files. Model-related settings live in the same file as everything else; there is no separate model manifest. The Kilo CLI is a fork of [OpenCode](https://opencode.ai), so the configuration shape, precedence, and AI-SDK adapter mechanism are inherited from OpenCode with Kilo-branded paths and schema URL.

| Scope | Path | Format | Notes |
| :---- | :--- | :----- | :---- |
| Global user | `~/.config/kilo/kilo.json` or `kilo.jsonc` | JSONC | User-wide defaults. The inspected host file is `~/.config/kilo/kilo.jsonc` and currently contains only a `"$schema"` reference to `https://app.kilo.ai/config.json`. |
| TUI user | `~/.config/kilo/tui.json` or `tui.jsonc` | JSONC | Terminal UI settings (notifications, sounds, themes, keybindings), not model configuration. |
| Project | `kilo.json`, `kilo.jsonc`, or `.kilo/kilo.json` in the repo root | JSONC | Project-specific settings that override global config. Legacy `.kilocode/` is also read. |
| Project TUI | `.kilo/tui.json` or `tui.jsonc` | JSONC | Project TUI overrides. |
| Custom file | `KILO_CONFIG` env var | JSONC | Loaded between global and project configs. |
| Custom directory | `KILO_CONFIG_DIR` env var | JSONC | Searched for agents, commands, modes, and plugins. |
| Inline override | `KILO_CONFIG_CONTENT` env var | JSON | Merged as a final local-scope override. |
| Managed / MDM | `/Library/Application Support/kilo/` (macOS), `/etc/kilo/` (Linux), `%ProgramData%\kilo` (Windows) | JSONC | Highest priority; not user-overridable. |

A formal JSON Schema is published at [`https://app.kilo.ai/config.json`](https://app.kilo.ai/config.json). Because the CLI is an OpenCode fork, the schema still references some OpenCode/opencode fields (for example `opencode serve` server settings and the `https://opencode.ai/config.json` fallback in older tooling), but Kilo documentation directs users to use `https://app.kilo.ai/config.json` for validation.

## Adding Cloud Models

Kilo ships with a large catalog of models via the AI SDK, [Models.dev](https://models.dev), and the [Kilo Gateway](https://kilo.ai/docs/gateway). To add a model or provider that is not preloaded, define a custom provider block under `provider.<provider_id>`.

### Concrete example

Add a hypothetical cloud provider that exposes an OpenAI-compatible API:

```jsonc
{
  "$schema": "https://app.kilo.ai/config.json",
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
| `models.<id>` | A model entry. The full model reference is `<provider_id>/<id>`. |

### API standards and adapters

Kilo does not expose a generic "OpenAI" or "Anthropic" toggle. Instead, the user names an AI SDK provider package in the `npm` field (or selects a protocol in the `api` field). That package determines the wire protocol.

| Standard | Adapter (`npm` / `api`) | Base URL | Auth |
| :------- | :---------------------- | :------- | :--- |
| OpenAI Chat Completions-compatible | `@ai-sdk/openai-compatible` | `provider.<id>.options.baseURL` | `provider.<id>.options.apiKey` or `{env:VAR}` |
| Anthropic Messages-compatible | `@ai-sdk/anthropic` | `provider.<id>.options.baseURL` | `provider.<id>.options.apiKey` or `{env:VAR}` |
| OpenAI Responses (provider-native) | `@ai-sdk/openai` | `provider.<id>.options.baseURL` | Provider-specific, often via `apiKey` or env vars |
| Bespoke / provider-native | Any AI SDK package, e.g. `@ai-sdk/google`, `@ai-sdk/azure` | `provider.<id>.options.baseURL` | Provider-specific |

For providers Kilo already knows about (built-in or from Models.dev), the `npm` adapter may be omitted. For providers Kilo does not already know about, `npm` (or a recognized `api` value) is required.

### Per-model metadata

The published schema and Kilo documentation agree on the following override keys:

| Key | Purpose |
| :-- | :------ |
| `id` | Override the model ID sent on the wire. |
| `name` | Display name in the model picker. |
| `family` | Model family grouping. |
| `release_date` | Release date string. |
| `attachment` | Whether the model accepts file attachments. |
| `reasoning` | Whether the model supports reasoning/thinking output. |
| `temperature` | Whether the model supports the temperature parameter. |
| `tool_call` | Whether the model supports tool calling. |
| `interleaved` | Reasoning field mapping for providers that stream reasoning separately. |
| `cost` | Per-million-token pricing: `input`, `output`, optional `cache_read`, `cache_write`, and `context_over_200k`. |
| `limit` | `context`, `input`, and `output` token limits. |
| `modalities` | `input` and `output` arrays of `text`, `audio`, `image`, `video`, `pdf`. |
| `experimental` | Flag the model as experimental. |
| `status` | `alpha`, `beta`, `deprecated`, or `active`. |
| `provider` | Per-model provider override (`npm`, `api`). |
| `options` | Provider-specific request options (e.g. `reasoningEffort`, `thinking`). |
| `headers` | Extra HTTP headers for this model. |
| `variants` | Variant-specific configuration and disabling. |

### Interaction with the built-in catalog

User-defined provider and model blocks **merge** with the built-in catalog and remote Models.dev data. Non-conflicting keys are preserved; conflicting keys are overridden by the user's config. This means:

- Adding a new model under a built-in provider appends it to the picker.
- Defining a model with the same ID as a built-in entry shadows the built-in metadata.
- `whitelist` and `blacklist` on a provider hide built-in models without removing them from the catalog source.

Because Kilo fetches model metadata from Models.dev and refreshes its catalog hourly, a manual block for a model that later becomes built-in should be removed to avoid stale overrides. Kilo does not auto-remove or warn about these stale blocks.

### Cross-cloud bridging

Kilo can be routed at a different cloud vendor's API by overriding `provider.<id>.options.baseURL` and choosing the AI SDK adapter that matches the target's protocol. If the target vendor's native API does not speak one of Kilo's supported standards, route through a translation proxy such as [LiteLLM](https://github.com/BerriAI/litellm) instead of pointing directly at a base URL that cannot work.

```jsonc
{
  "$schema": "https://app.kilo.ai/config.json",
  "model": "openai/gemini-2.5-pro",
  "provider": {
    "openai": {
      "options": {
        "baseURL": "https://litellm.example.com/v1",
        "apiKey": "{env:LITELLM_API_KEY}"
      }
    }
  }
}
```

Here the `openai` provider's client is redirected to a LiteLLM proxy that translates the OpenAI Chat Completions API to the upstream vendor's native format.

## Adding Local Models

Local model runners are wired the same way as cloud custom providers: an AI SDK adapter package plus a `baseURL` pointing at the local server. Kilo does not ship runner-native launch hooks (for example, there is no `ollama launch kilo`); integration is done through provider blocks.

| Runner | Integration path | Standard | Notes |
| :----- | :--------------- | :------- | :---- |
| Ollama | Base-URL override | OpenAI-compatible | Endpoint `http://localhost:11434/v1`. Also supports Anthropic Messages. |
| LM Studio | Base-URL override | OpenAI-compatible | Endpoint `http://localhost:1234/v1`. Also supports Anthropic Messages. |
| llama.cpp | Base-URL override | OpenAI-compatible | Endpoint `http://127.0.0.1:8080/v1`. Also supports Anthropic Messages. |
| vLLM | Base-URL override | OpenAI-compatible | Endpoint `http://localhost:8000/v1`. Also supports Anthropic Messages. |
| oMLX | Base-URL override | OpenAI-compatible | Endpoint `http://localhost:8000/v1`. Also supports Anthropic Messages. |
| Other | Base-URL override | OpenAI-compatible or Anthropic-compatible | Any local server exposing a matching `/v1` endpoint works with the corresponding adapter. |

### Ollama example

```jsonc
{
  "$schema": "https://app.kilo.ai/config.json",
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
  "$schema": "https://app.kilo.ai/config.json",
  "model": "lmstudio/deepseek-r1-0528",
  "provider": {
    "lmstudio": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "LM Studio (local)",
      "options": { "baseURL": "http://localhost:1234/v1" },
      "models": {
        "deepseek-r1-0528": {
          "name": "DeepSeek R1 0528"
        }
      }
    }
  }
}
```

## Environment Overrides

Kilo-specific environment variables that redirect model endpoints or selection:

| Variable | Effect |
| :------- | :----- |
| `KILO_PROVIDER` | Override the active provider ID. |
| `KILO_<FIELD_NAME>` | For non-kilocode providers, maps to provider options (e.g. `KILO_API_KEY` -> `apiKey`). |
| `KILOCODE_<FIELD_NAME>` | For the `kilocode` provider, maps to provider options (e.g. `KILOCODE_MODEL` -> `kilocodeModel`). |
| `KILO_ORG_ID` | Select the Kilo organization/team for non-interactive `kilo run` sessions. |
| `KILO_API_KEY` | API key for the Kilo Gateway when using `kilocode` provider models. |
| `KILO_EXPERIMENTAL_OUTPUT_TOKEN_MAX` | Overrides the default 32,000 fallback output-token limit when a custom/local model has `output: 0`. |
| `KILO_CONFIG` | Load an additional config file between global and project configs. |
| `KILO_CONFIG_CONTENT` | Inject inline JSON as a final local-scope config override. |
| `KILO_CONFIG_DIR` | Use a custom directory for agents, commands, modes, and plugins. |

Provider credentials are usually picked up from provider-specific environment variables (e.g. `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`). These can also be referenced inside config with `{env:VAR}` substitutions, such as `"apiKey": "{env:ANTHROPIC_API_KEY}"`.

Session model selection follows this precedence (highest first):

1. `--model` / `-m` CLI flag.
2. `model` key in the effective config (project > custom > global > managed).
3. Last used model.
4. Internal default priority.

## Changelog

- **2026-07-02** — Confirmed Kilo CLI 1.0 is an OpenCode fork and uses the same `kilo.jsonc` config surface, precedence chain, and AI-SDK adapter model.
- **2026-07-02** — Reclassified all listed local runners (Ollama, LM Studio, llama.cpp, vLLM, oMLX) as base-URL override paths; oMLX is no longer unsupported.
- **2026-07-02** — Documented the three API-standard paths for custom providers: OpenAI Chat Completions-compatible, Anthropic Messages-compatible, and provider-native/bespoke via `npm`/`api` adapter fields.
- **2026-07-02** — Updated per-model metadata keys, merge semantics, and environment variables to match the current published schema and docs.
- **2026-07-02** — Noted that Kilo does not ship runner-native launch hooks; local integration is done through provider blocks.

## Sources

- [Kilo Code — Custom Models](https://kilo.ai/docs/code-with-ai/agents/custom-models)
- [Kilo Code — Model Selection](https://kilo.ai/docs/code-with-ai/agents/model-selection)
- [Kilo Code — CLI](https://kilo.ai/docs/code-with-ai/platforms/cli)
- [Kilo Code — CLI Reference](https://kilo.ai/docs/code-with-ai/platforms/cli-reference)
- [Kilo Code — AI Gateway](https://kilo.ai/docs/gateway)
- [Kilo Code — Gateway Models & Providers](https://kilo.ai/docs/gateway/models-and-providers)
- [Kilo Code — Auto Model](https://kilo.ai/docs/code-with-ai/agents/auto-model)
- [Kilo Code config JSON Schema](https://app.kilo.ai/config.json)
- [Kilo Code repository](https://github.com/Kilo-Org/kilocode)
- [OpenCode — Config](https://opencode.ai/docs/config)
- [OpenCode — Models](https://opencode.ai/docs/models)
- [OpenCode — Providers](https://opencode.ai/docs/providers)
- [AI SDK](https://ai-sdk.dev/)
- [Models.dev](https://models.dev)

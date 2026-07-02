---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
has_official_schema: formal
schema_url: https://app.kilo.ai/config.json
config_files:
  - scope: user
    path: ~/.config/kilo/kilo.jsonc
    format: jsonc
    notes: 'Global user config. The host file exists at this path and currently contains only a "$schema" reference to https://app.kilo.ai/config.json. Also accepts kilo.json.'
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
    notes: 'Primary path for user-added cloud/local models and OpenAI-compatible gateways, including Kilo Gateway.'
  - standard: anthropic_compatible
    base_url_site: provider.<id>.options.baseURL
    auth_site: provider.<id>.options.apiKey or {env:ANTHROPIC_API_KEY}
    adapter: '@ai-sdk/anthropic'
    notes: 'Use the AI SDK Anthropic provider package for Anthropic-compatible endpoints.'
  - standard: bespoke
    base_url_site: provider.<id>.options.baseURL
    auth_site: provider.<id>.options.apiKey or {env:VARIABLE_NAME}
    adapter: 'npm key naming any AI SDK provider package (e.g. @ai-sdk/google, @ai-sdk/azure)'
    notes: 'Kilo loads the specified npm package to speak the provider native protocol.'
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
    supported: openai_compatible
    example: '{ "provider": { "ollama": { "npm": "@ai-sdk/openai-compatible", "name": "Ollama (local)", "options": { "baseURL": "http://localhost:11434/v1" }, "models": { "gemma3:27b": { "name": "Gemma 3 27B", "limit": { "context": 128000, "output": 8192 } } } } }, "model": "ollama/gemma3:27b" }'
    notes: 'Uses the Ollama OpenAI-compatible server endpoint. Size/quantization tags are part of the model ID.'
  - runner: lmstudio
    supported: openai_compatible
    example: '{ "provider": { "lmstudio": { "npm": "@ai-sdk/openai-compatible", "name": "LM Studio (local)", "options": { "baseURL": "http://localhost:1234/v1" }, "models": { "deepseek-r1-0528": { "name": "DeepSeek R1 0528" } } } }, "model": "lmstudio/deepseek-r1-0528" }'
    notes: 'Uses the LM Studio OpenAI-compatible server endpoint loaded by LM Studio.'
  - runner: llamacpp
    supported: openai_compatible
    example: '{ "provider": { "llamacpp": { "npm": "@ai-sdk/openai-compatible", "name": "llama-server (local)", "options": { "baseURL": "http://127.0.0.1:8080/v1" }, "models": { "qwen3-coder:a3b": { "name": "Qwen3-Coder a3b (local)", "limit": { "context": 128000, "output": 65536 } } } } }, "model": "llamacpp/qwen3-coder:a3b" }'
    notes: 'Uses llama.cpp llama-server OpenAI-compatible endpoint.'
  - runner: vllm
    supported: openai_compatible
    example: '{ "provider": { "vllm": { "npm": "@ai-sdk/openai-compatible", "name": "vLLM (local)", "options": { "baseURL": "http://localhost:8000/v1" }, "models": { "qwen2.5-coder-32b-instruct": { "name": "Qwen2.5 Coder 32B Instruct (local)" } } } }, "model": "vllm/qwen2.5-coder-32b-instruct" }'
    notes: 'vLLM exposes an OpenAI-compatible API by default.'
  - runner: omlx
    supported: unsupported
    notes: 'No official documentation or built-in integration. Would require a custom OpenAI-compatible shim if one exists.'
  - runner: other
    supported: openai_compatible
    notes: 'Any local server that exposes an OpenAI-compatible /v1 endpoint can be wired via the @ai-sdk/openai-compatible adapter.'
default_model_site: 'Top-level model key in kilo.jsonc/kilo.json (user/project/managed); session override via --model / -m CLI flag.'
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
changes: []
requires_claudine_update: false
---

# Kilo Code User-Side Model Configuration

## Introduction to Kilo Code Model Configuration

Kilo Code stores runtime configuration in JSON or JSONC files. Model-related settings live in the same file as everything else; there is no separate model manifest. The Kilo CLI is a fork of [OpenCode](https://opencode.ai), so the configuration shape, precedence, and adapter mechanism are inherited from OpenCode with Kilo-branded paths and schema URL.

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

Kilo does not expose a generic "OpenAI" or "Anthropic" toggle. Instead, the user names an AI SDK provider package in the `npm` field. That package determines the wire protocol.

| Standard | Adapter (`npm`) | Base URL | Auth |
| :------- | :-------------- | :------- | :--- |
| OpenAI-compatible | `@ai-sdk/openai-compatible` | `provider.<id>.options.baseURL` | `provider.<id>.options.apiKey` or `{env:VAR}` |
| Anthropic-compatible | `@ai-sdk/anthropic` | `provider.<id>.options.baseURL` | `provider.<id>.options.apiKey` or `{env:ANTHROPIC_API_KEY}` |
| Bespoke / provider-native | Any AI SDK package, e.g. `@ai-sdk/google`, `@ai-sdk/azure` | `provider.<id>.options.baseURL` | Provider-specific, often via `apiKey` or env vars |

For providers Kilo already knows about (built-in or from Models.dev), the `npm` adapter may be omitted. For providers Kilo does not already know about, `npm` is required.

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
| `temperature` | Whether temperature can be set. |
| `tool_call` | Whether the model supports tool calling. |
| `interleaved` | Reasoning field mapping for providers that stream reasoning separately. |
| `cost` | Per-token pricing: `input`, `output`, optional `cache_read`, `cache_write`, and `context_over_200k`. |
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

Because Kilo fetches model metadata from remote sources and updates its catalog over time, a manual block for a model that later becomes built-in should be removed to avoid stale overrides. Kilo does not auto-remove or warn about these stale blocks.

## Adding Local Models

Local model runners are wired the same way as cloud custom providers: an AI SDK adapter package plus a `baseURL` pointing at the local server.

| Runner | Support | Notes |
| :----- | :------ | :---- |
| Ollama | OpenAI-compatible shim | First-class docs example; endpoint `http://localhost:11434/v1`. |
| LM Studio | OpenAI-compatible shim | First-class docs example; endpoint `http://localhost:1234/v1`. |
| llama.cpp | OpenAI-compatible shim | Expected to work via `llama-server`; endpoint `http://127.0.0.1:8080/v1`. |
| vLLM | OpenAI-compatible shim | vLLM serves an OpenAI-compatible API by default. |
| oMLX | Unsupported | No official documentation or integration. |
| Other | OpenAI-compatible shim | Any local server exposing an OpenAI-compatible `/v1` chat endpoint works with `@ai-sdk/openai-compatible`. |

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
2. `model` key in the effective config (project > custom > global > remote).
3. Last used model.
4. Internal default priority.

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

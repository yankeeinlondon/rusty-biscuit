---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7
has_official_schema: none
model_config_paths:
  - scope: user
    path: ~/.config/goose/config.yaml
    format: yaml
    notes: 'Global user settings. Top-level GOOSE_PROVIDER and GOOSE_MODEL keys pin the default provider and model. This host does not have Goose CLI installed, so no local file was observed.'
  - scope: user
    path: ~/.config/goose/custom_providers/*.json
    format: json
    notes: 'One JSON file per custom provider. Each file adds a user-defined provider with its own model list and endpoint configuration.'
  - scope: user
    path: ~/.config/goose/secrets.yaml
    format: yaml
    notes: 'Fallback file-based secret store used when the system keyring is unavailable or disabled; holds API keys referenced by custom providers.'
  - scope: env
    path: Shell environment variables
    format: other
    notes: 'Session-scoped overrides such as GOOSE_PROVIDER, GOOSE_MODEL, GOOSE_PROVIDER__HOST, and provider-specific HOST variables.'
api_standards:
  - standard: openai_compatible
    base_url_site: base_url in custom provider JSON (or OPENAI_HOST for the built-in openai provider)
    auth_site: api_key_env in custom provider JSON (or OPENAI_API_KEY for the built-in openai provider)
    adapter: none
    notes: 'Custom providers with engine openai/openai_compatible speak the OpenAI Chat Completions API. The built-in OpenAI provider can also be pointed at any OpenAI-compatible endpoint.'
  - standard: anthropic_compatible
    base_url_site: base_url in custom provider JSON (or ANTHROPIC_HOST for the built-in anthropic provider)
    auth_site: api_key_env in custom provider JSON (or ANTHROPIC_API_KEY for the built-in anthropic provider)
    adapter: none
    notes: 'Custom providers with engine anthropic/anthropic_compatible speak the Anthropic Messages API.'
  - standard: bespoke
    base_url_site: base_url in custom provider JSON (or OLLAMA_HOST for the built-in ollama provider)
    auth_site: none (Ollama local servers typically require no key)
    adapter: none
    notes: 'Custom providers with engine ollama/ollama_compatible use the Ollama native API, which is neither OpenAI nor Anthropic compatible.'
metadata_overrides:
  - name
  - resolved_model
  - context_limit
  - input_token_cost
  - output_token_cost
  - currency
  - supports_cache_control
  - reasoning
merge_semantics: merge
local_runners:
  - runner: ollama
    integration: first_class
    standard: bespoke
    example: |
      GOOSE_PROVIDER: ollama
      GOOSE_MODEL: qwen2.5
      OLLAMA_HOST: http://localhost:11434
    notes: 'First-class built-in provider using the native Ollama API. Model IDs include the Ollama tag, e.g. qwen2.5 or michaelneale/deepseek-r1-goose. Can also be added as a custom OpenAI- or Anthropic-compatible provider.'
  - runner: lmstudio
    integration: first_class
    standard: openai_compatible
    example: |
      GOOSE_PROVIDER: lmstudio
      GOOSE_MODEL: qwen2.5-coder-14b-instruct
    notes: 'First-class built-in provider. Connects to localhost:1234 by default over LM Studios OpenAI-compatible endpoint. Use the model id reported by LM Studio.'
  - runner: omlx
    integration: base_url_override
    standard: openai_compatible
    example: |
      {
        "name": "local_omlx",
        "engine": "openai",
        "display_name": "oMLX",
        "base_url": "http://localhost:8000/v1",
        "requires_auth": false,
        "models": [{"name": "qwen2.5-7b", "context_limit": 32768}]
      }
    notes: 'No native provider; add via custom OpenAI-compatible provider. oMLX also exposes an Anthropic-compatible endpoint, so engine anthropic with base_url http://localhost:8000 works as well.'
  - runner: llamacpp
    integration: base_url_override
    standard: openai_compatible
    example: |
      {
        "name": "local_llamacpp",
        "engine": "openai",
        "display_name": "llama.cpp",
        "base_url": "http://localhost:8080/v1",
        "requires_auth": false,
        "models": [{"name": "qwen2.5-coder-14b", "context_limit": 32768}]
      }
    notes: 'llama.cpp server exposes an OpenAI-compatible Chat Completions API and, since build b7187, an Anthropic-compatible /v1/messages endpoint. Add it as a custom provider on either standard.'
  - runner: vllm
    integration: base_url_override
    standard: openai_compatible
    example: |
      {
        "name": "local_vllm",
        "engine": "openai",
        "display_name": "vLLM",
        "base_url": "http://localhost:8000/v1",
        "requires_auth": false,
        "models": [{"name": "qwen2.5-coder-32b-instruct", "context_limit": 32768}]
      }
    notes: 'vLLM serves both OpenAI-compatible and Anthropic-compatible APIs (anthropic since v0.11.1). Add it as a custom provider on either standard, or point the built-in openai provider at it with OPENAI_HOST.'
  - runner: other
    integration: base_url_override
    standard: openai_compatible
    notes: 'Any local runner that exposes an OpenAI-compatible, Anthropic-compatible, or Ollama-compatible endpoint can be added as a custom provider.'
cloud_bridge:
  supported: true
  mechanism: custom provider JSON with engine openai/anthropic and base_url pointing at a gateway, or OPENAI_HOST/ANTHROPIC_HOST on built-in providers
  example: |
    {
      "name": "litellm_bridge",
      "engine": "openai",
      "display_name": "LiteLLM Bridge",
      "base_url": "http://localhost:4000/v1",
      "api_key_env": "LITELLM_API_KEY",
      "requires_auth": true,
      "models": [{"name": "bedrock-claude-sonnet-4", "context_limit": 200000}]
    }
default_model_site: 'GOOSE_MODEL key in ~/.config/goose/config.yaml; session-scope override via GOOSE_MODEL env var or --model flag.'
env_vars:
  - name: GOOSE_PROVIDER
    effect: Selects the active provider (e.g. anthropic, openai, ollama, lmstudio, or a custom provider id).
  - name: GOOSE_MODEL
    effect: Selects the active model for the session. Overrides the config.yaml GOOSE_MODEL value.
  - name: GOOSE_FAST_MODEL
    effect: Overrides the providers default fast/auxiliary model.
  - name: GOOSE_PROVIDER__TYPE
    effect: Overrides the provider implementation type.
  - name: GOOSE_PROVIDER__HOST
    effect: Overrides the API endpoint host for the current provider.
  - name: GOOSE_PROVIDER__API_KEY
    effect: Overrides the API key for the current provider.
  - name: GOOSE_PLANNER_PROVIDER
    effect: Provider used for planning mode.
  - name: GOOSE_PLANNER_MODEL
    effect: Model used for planning mode.
  - name: GOOSE_CONTEXT_LIMIT
    effect: Overrides the context limit for the main model.
  - name: GOOSE_INPUT_LIMIT
    effect: Overrides the Ollama input prompt limit (maps to num_ctx).
  - name: GOOSE_PLANNER_CONTEXT_LIMIT
    effect: Overrides the context limit for the planner model.
  - name: GOOSE_TEMPERATURE
    effect: Overrides model temperature.
  - name: GOOSE_MAX_TOKENS
    effect: Overrides maximum tokens per response.
  - name: OLLAMA_HOST
    effect: Overrides the Ollama server endpoint.
  - name: OPENAI_HOST
    effect: Overrides the OpenAI-compatible endpoint used by the built-in openai provider.
  - name: OPENAI_API_KEY
    effect: API key for the built-in openai provider.
  - name: ANTHROPIC_HOST
    effect: Overrides the Anthropic endpoint used by the built-in anthropic provider.
  - name: ANTHROPIC_API_KEY
    effect: API key for the built-in anthropic provider.
  - name: CLAUDE_THINKING_TYPE
    effect: Controls Claude reasoning mode (adaptive/enabled/disabled) on Anthropic and Databricks providers.
  - name: GEMINI3_THINKING_LEVEL
    effect: Sets the thinking level for Gemini 3 models (low or high).
changes:
  - 'Goose has moved to the Agentic AI Foundation (AAIF). The repository is now https://github.com/aaif-goose/goose and the documentation site is https://goose-docs.ai (block.github.io/goose redirects).'
  - 'The built-in provider roster has expanded significantly to include Ollama Cloud, Ramalama, Atomic Chat, Docker Model Runner, ChatGPT Codex, GitHub Copilot, and numerous cloud providers (Avian, EmpirioLabs, FuturMix, Novita AI, Routstr, etc.).'
  - 'LM Studio is now a first-class built-in provider using its OpenAI-compatible endpoint, not only a custom-provider path.'
  - 'The custom provider JSON schema gained new optional fields: dynamic_models, skip_canonical_filtering, base_path, env_vars, timeout_seconds, catalog_provider_id, model_doc_link, setup_steps, fast_model, preserves_thinking, and resolved_model on ModelInfo.'
  - 'New environment variables were documented: GOOSE_CONTEXT_LIMIT, GOOSE_INPUT_LIMIT, GOOSE_PLANNER_CONTEXT_LIMIT, GOOSE_FAST_MODEL, CLAUDE_THINKING_TYPE, and GEMINI3_THINKING_LEVEL.'
  - 'The custom provider engine field accepts openai/openai_compatible, anthropic/anthropic_compatible, and ollama/ollama_compatible (case-insensitive).'
requires_claudine_update: true
reason: 'Claudines Goose provider adapter and model-catalog metadata should be updated for the AAIF repository move, the expanded built-in provider roster, and the new custom-provider schema fields (dynamic_models, env_vars, base_path, preserves_thinking, etc.) so that config parsing and provider detection remain accurate.'
---

# Goose CLI User-Side Model Configuration

## Introduction to Goose CLI Model Configuration

Goose CLI stores persistent model configuration in YAML files at user scope. The primary file is `~/.config/goose/config.yaml` (macOS/Linux) or `%APPDATA%\Block\goose\config\config.yaml` (Windows). API keys are normally stored in the system keyring; when the keyring is unavailable, Goose falls back to `~/.config/goose/secrets.yaml`.

| Scope | Path | Format | Effect |
| :---- | :--- | :----- | :----- |
| User | `~/.config/goose/config.yaml` | YAML | Sets `GOOSE_PROVIDER`, `GOOSE_MODEL`, temperature, token limits, planner model, and extensions. |
| User | `~/.config/goose/custom_providers/*.json` | JSON | One file per custom provider; adds user-defined providers and models. |
| User | `~/.config/goose/secrets.yaml` | YAML | File-based fallback for API keys when the keyring is disabled or unavailable. |
| Env | Shell environment variables | n/a | Session overrides; highest precedence. |

Precedence is: environment variables > `config.yaml` > defaults. Goose CLI was not installed on this host, so no actual config files were observed.

Goose does not publish a formal JSON Schema for `config.yaml` or for custom provider JSON files. The custom provider shape is documented in the Goose guides and is defined in the Rust source (`DeclarativeProviderConfig` and `ModelInfo`), but there is no published machine-readable schema URL.

## Adding Cloud Models

To use a cloud model that Goose does not ship out of the box, add a **custom provider** JSON file under `~/.config/goose/custom_providers/` and select it with `GOOSE_PROVIDER`.

### Concrete example

```json
{
  "name": "custom_corp_api",
  "engine": "openai",
  "display_name": "Corporate API",
  "description": "Custom corporate OpenAI-compatible API",
  "api_key_env": "CUSTOM_CORP_API_KEY",
  "base_url": "https://api.company.com/v1",
  "requires_auth": true,
  "supports_streaming": true,
  "models": [
    {
      "name": "gpt-4o",
      "context_limit": 128000,
      "input_token_cost": 2.5e-6,
      "output_token_cost": 1.0e-5,
      "supports_cache_control": false,
      "reasoning": false
    }
  ],
  "headers": {
    "x-origin-client-id": "my-client"
  }
}
```

Select the provider in `~/.config/goose/config.yaml`:

```yaml
GOOSE_PROVIDER: custom_corp_api
GOOSE_MODEL: gpt-4o
```

### What each key means

| Key | Effect |
| :-- | :----- |
| `name` | Unique provider id used in `GOOSE_PROVIDER`. |
| `engine` | API family: `openai`/`openai_compatible`, `anthropic`/`anthropic_compatible`, or `ollama`/`ollama_compatible`. |
| `display_name` | Human-readable label in the UI. |
| `description` | Optional short description of the provider. |
| `base_url` | API base URL; may contain `${VAR}` placeholders expanded from `env_vars`. |
| `base_path` | Optional URL path appended after `base_url`. |
| `api_key_env` | Environment variable or secret key name holding the API key; leave empty when auth is not required. |
| `requires_auth` | Whether the provider needs an API key; defaults to `true`. |
| `supports_streaming` | Whether the endpoint supports streaming responses. |
| `models` | Static list of models available from this provider. |
| `dynamic_models` | When `true` or omitted, fetch models from the providers `/v1/models` endpoint instead of using the static list. |
| `headers` | Extra headers to send on every request. |
| `timeout_seconds` | Optional request timeout in seconds. |
| `env_vars` | Optional templated variables for `base_url`, `base_path`, and `headers`. |
| `fast_model` | Optional cheaper/faster model for lightweight tasks. |
| `preserves_thinking` | Whether reasoning/thinking blocks are preserved; defaults to `true` for OpenAI engines, `false` otherwise. |
| `skip_canonical_filtering` | If `true`, bypass Gooses canonical model-registry filtering. |
| `catalog_provider_id` | Optional known-catalog mapping for provider-specific behavior. |
| `model_doc_link` | Optional URL to the providers model documentation. |
| `setup_steps` | Optional setup instructions shown to the user. |

### Supported API standards

| Standard | Base URL | Auth | Notes |
| :------- | :------- | :--- | :---- |
| OpenAI-compatible | `base_url` (or `OPENAI_HOST` for the built-in `openai` provider) | `api_key_env` (or `OPENAI_API_KEY`) | Most common; works with many gateways and proxies. |
| Anthropic-compatible | `base_url` (or `ANTHROPIC_HOST` for the built-in `anthropic` provider) | `api_key_env` (or `ANTHROPIC_API_KEY`) | For endpoints that speak the Anthropic Messages API. |
| Ollama (bespoke) | `base_url` (or `OLLAMA_HOST` for the built-in `ollama` provider) | Usually none | Native Ollama API, not OpenAI/Anthropic compatible. |

There is **no adapter/package mechanism** like an npm ai-sdk key. Goose expects the upstream to expose one of the supported API families directly.

### Per-model metadata

Inside each object in the `models` array, a user can declare:

| Key | Meaning |
| :-- | :------ |
| `name` | Model id sent to the provider. |
| `resolved_model` | Optional underlying model name when `name` is an alias or endpoint. |
| `context_limit` | Maximum context length in tokens. |
| `input_token_cost` | Per-input-token cost in USD. |
| `output_token_cost` | Per-output-token cost in USD. |
| `currency` | Currency symbol for the costs (default `$`). |
| `supports_cache_control` | Whether the model supports Anthropic-style cache control markers. |
| `reasoning` | Whether the model supports reasoning/thinking controls. |

### Interaction with the built-in catalog

Custom providers are **merged** into the provider list alongside built-in providers. If a custom provider file uses the same `name` as a built-in provider, the custom file is loaded later and **shadows** the built-in entry.

Goose does not auto-merge model lists between providers. A model that later ships in a built-in provider will appear there as a separate entry; the user should remove or disable the redundant custom provider file. Goose does not automate this cleanup.

### Cross-cloud bridging

Goose CLI can be routed at a different cloud vendors API by adding a custom provider whose `engine` matches the API standard the target speaks (OpenAI-compatible or Anthropic-compatible) and pointing `base_url` at a gateway or proxy.

If the target vendors native API does not serve either standard, place a translation proxy such as **LiteLLM** between Goose and the vendor:

```json
{
  "name": "litellm_bridge",
  "engine": "openai",
  "display_name": "LiteLLM Bridge",
  "base_url": "http://localhost:4000/v1",
  "api_key_env": "LITELLM_API_KEY",
  "requires_auth": true,
  "models": [
    {"name": "bedrock-claude-sonnet-4", "context_limit": 200000}
  ]
}
```

For the built-in `openai` provider, the same idea works through `OPENAI_HOST`:

```bash
export OPENAI_HOST="http://localhost:4000/v1"
export OPENAI_API_KEY="litellm-token"
export GOOSE_PROVIDER="openai"
export GOOSE_MODEL="bedrock-claude-sonnet-4"
```

## Adding Local Models

Goose has first-class providers for **Ollama** and **LM Studio**. Other local runners can be used if they expose an OpenAI-compatible, Anthropic-compatible, or Ollama-compatible endpoint.

| Runner | Integration path | Notes |
| :----- | :--------------- | :---- |
| Ollama | First-class | Built-in `ollama` provider using the native Ollama API. |
| LM Studio | First-class | Built-in `lmstudio` provider using LM Studios OpenAI-compatible endpoint. |
| oMLX | Base-URL override | No native provider; add as a custom OpenAI- or Anthropic-compatible provider. |
| llama.cpp | Base-URL override | Add `llama-server` as a custom OpenAI- or Anthropic-compatible provider. |
| vLLM | Base-URL override | Add `vllm serve` as a custom OpenAI- or Anthropic-compatible provider. |

### Ollama example

```yaml
GOOSE_PROVIDER: ollama
GOOSE_MODEL: qwen2.5
OLLAMA_HOST: http://localhost:11434
```

The model id includes the size/quantization tag (`:14b`) exactly as Ollama reports it.

### LM Studio example

```yaml
GOOSE_PROVIDER: lmstudio
GOOSE_MODEL: qwen2.5-coder-14b-instruct
```

LM Studio defaults to `localhost:1234`.

### vLLM example

```json
{
  "name": "local_vllm",
  "engine": "openai",
  "display_name": "vLLM",
  "base_url": "http://localhost:8000/v1",
  "requires_auth": false,
  "models": [
    {"name": "qwen2.5-coder-32b-instruct", "context_limit": 32768}
  ]
}
```

Select it with `GOOSE_PROVIDER: local_vllm`.

## Environment Overrides

Environment variables take precedence over the corresponding settings in `config.yaml`. Model-related overrides include:

| Variable | Effect |
| :------- | :----- |
| `GOOSE_PROVIDER` | Active provider id. |
| `GOOSE_MODEL` | Active model id. |
| `GOOSE_FAST_MODEL` | Fast/auxiliary model for lightweight calls. |
| `GOOSE_PROVIDER__TYPE` | Provider implementation type override. |
| `GOOSE_PROVIDER__HOST` | Endpoint host override. |
| `GOOSE_PROVIDER__API_KEY` | API key override. |
| `GOOSE_PLANNER_PROVIDER` | Provider for planning mode. |
| `GOOSE_PLANNER_MODEL` | Model for planning mode. |
| `GOOSE_CONTEXT_LIMIT` | Context limit for the main model. |
| `GOOSE_INPUT_LIMIT` | Ollama input limit (`num_ctx`). |
| `GOOSE_PLANNER_CONTEXT_LIMIT` | Context limit for the planner model. |
| `GOOSE_TEMPERATURE` | Model temperature. |
| `GOOSE_MAX_TOKENS` | Maximum tokens per response. |
| `OLLAMA_HOST` | Ollama server endpoint. |
| `OPENAI_HOST` | OpenAI-compatible endpoint for the built-in `openai` provider. |
| `OPENAI_API_KEY` | API key for the built-in `openai` provider. |
| `ANTHROPIC_HOST` | Anthropic endpoint for the built-in `anthropic` provider. |
| `ANTHROPIC_API_KEY` | API key for the built-in `anthropic` provider. |
| `CLAUDE_THINKING_TYPE` | Claude reasoning mode (`adaptive`, `enabled`, `disabled`). |
| `GEMINI3_THINKING_LEVEL` | Gemini 3 thinking level (`low`, `high`). |

## Changelog

- **2026-07-02** — Updated for Goose move to the Agentic AI Foundation (AAIF): repository is now `https://github.com/aaif-goose/goose` and docs are at `https://goose-docs.ai`.
- **2026-07-02** — Expanded built-in provider roster to include Ollama Cloud, Ramalama, Atomic Chat, Docker Model Runner, ChatGPT Codex, GitHub Copilot, and additional cloud providers.
- **2026-07-02** — Reclassified LM Studio as a first-class built-in provider using its OpenAI-compatible endpoint.
- **2026-07-02** — Documented new custom-provider JSON fields: `dynamic_models`, `skip_canonical_filtering`, `base_path`, `env_vars`, `timeout_seconds`, `catalog_provider_id`, `model_doc_link`, `setup_steps`, `fast_model`, `preserves_thinking`, and `resolved_model` on model entries.
- **2026-07-02** — Added newly documented environment variables: `GOOSE_CONTEXT_LIMIT`, `GOOSE_INPUT_LIMIT`, `GOOSE_PLANNER_CONTEXT_LIMIT`, `GOOSE_FAST_MODEL`, `CLAUDE_THINKING_TYPE`, and `GEMINI3_THINKING_LEVEL`.

## Sources

- [Goose CLI documentation](https://goose-docs.ai/)
- [Goose — Supported LLM Providers](https://goose-docs.ai/docs/getting-started/providers)
- [Goose — Configure Custom Provider](https://goose-docs.ai/docs/getting-started/providers#configure-custom-provider)
- [Goose — Configuration Files](https://goose-docs.ai/docs/guides/config-files)
- [Goose — Environment Variables](https://goose-docs.ai/docs/guides/environment-variables)
- [Goose — Local LLMs](https://goose-docs.ai/docs/getting-started/providers#local-llms)
- [Goose source repository](https://github.com/aaif-goose/goose)
- [Goose declarative provider config source](https://github.com/aaif-goose/goose/blob/main/crates/goose-providers/src/declarative.rs)
- [Goose provider types — ModelInfo source](https://github.com/aaif-goose/goose/blob/main/crates/goose-provider-types/src/base.rs)

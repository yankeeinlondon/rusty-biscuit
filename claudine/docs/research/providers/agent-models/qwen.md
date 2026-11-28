---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-01
agent: open_code
model: kimi-for-coding/k2p7
has_official_schema: informal
schema_url: https://qwenlm.github.io/qwen-code-docs/en/users/configuration/model-providers/
default_models:
  - id: qwen3.5-plus
    is_default: true
    notes: Built-in default for the OpenAI-compatible protocol and the first model in the Alibaba Cloud Coding Plan auto-configuration.
  - id: qwen3.6-plus
    notes: Latest model with thinking enabled; Pro subscribers only.
  - id: qwen3.7-plus
    notes: Advanced model with thinking enabled.
  - id: qwen3-coder-plus
    notes: Optimized for coding tasks.
  - id: qwen3-coder-next
    notes: Experimental coding model.
  - id: qwen3-max-2026-01-23
    notes: Latest max model with thinking enabled.
  - id: glm-5
    notes: GLM model with thinking enabled.
  - id: glm-4.7
    notes: GLM model with thinking enabled.
  - id: kimi-k2.5
    notes: Kimi model with thinking and vision/video support.
  - id: MiniMax-M2.5
    notes: MiniMax model with thinking enabled.
model_selection:
  - method: interactive_command
    site: "/model"
    example: "/model qwen3-coder-plus"
    notes: Runtime model switcher; persists selection. Also supports /model --fast, /model --vision, /model --voice.
  - method: interactive_command
    site: "/auth"
    example: "/auth"
    notes: Interactive authentication and provider setup; choosing Alibaba Cloud Coding Plan auto-configures the default model catalog.
  - method: cli_flag
    site: "--model"
    example: "qwen --model qwen3-coder-plus"
    notes: Launch-time model override; alias -m.
  - method: cli_flag
    site: "--auth-type"
    example: "qwen --auth-type openai --model qwen3-coder-plus"
    notes: Selects protocol/auth type at launch.
  - method: env_var
    site: "OPENAI_MODEL"
    example: "OPENAI_MODEL=qwen3-coder-plus qwen"
    notes: Alias QWEN_MODEL. Provider-specific model env var for the OpenAI-compatible protocol.
  - method: env_var
    site: "ANTHROPIC_MODEL"
    example: "ANTHROPIC_MODEL=claude-sonnet-4-20250514 qwen"
    notes: Provider-specific model env var for Anthropic protocol.
  - method: env_var
    site: "GEMINI_MODEL / GOOGLE_MODEL"
    example: "GEMINI_MODEL=gemini-2.5-pro qwen"
    notes: Provider-specific model env var for Gemini/Vertex AI protocol.
  - method: config_file
    site: "model.name"
    example: '"model": { "name": "qwen3-coder-plus" }'
    notes: Default model in settings.json.
  - method: config_file
    site: "modelProviders"
    example: '"modelProviders": { "openai": { "protocol": "openai", "models": [{ "id": "qwen3-coder-plus", "envKey": "DASHSCOPE_API_KEY" }] } }'
    notes: Sealed provider catalog; selecting a provider model applies its generationConfig atomically.
  - method: wire_envelope
    site: "request body model field"
    example: '{ "model": "qwen3-coder-plus" }'
    notes: OpenAI Chat Completions, Anthropic Messages, or Gemini request body after alias/provider resolution.
precedence: "interactive_command (/model runtime, /auth setup) > cli_flag (--model, --auth-type) > env_var (OPENAI_MODEL, ANTHROPIC_MODEL, GEMINI_MODEL/GOOGLE_MODEL) > config_file (model.name, modelProviders) > built-in default (qwen3.5-plus for OpenAI-compatible protocol)"
dynamic_listing:
  available: false
  method: "none — no qwen models / list-models subcommand or API"
  example: "Interactive only: run qwen then /model. Non-interactive consumers read the resolved model from stream-json result events."
changes: []
requires_claudine_update: true
reason: "Claudine's Qwen provider wrapper currently reports Unsupported. Qwen Code's model surface is materially different from Claude/Codex: it is multi-protocol (openai/anthropic/gemini/vertex-ai), uses a modelProviders catalog with sealed generationConfig, supports runtime /model selection with --fast/--vision/--voice variants, and relies on provider-specific env vars (OPENAI_MODEL, ANTHROPIC_MODEL, GEMINI_MODEL) and CLI flags (--auth-type, --model). Claudine needs a Qwen adapter, model catalog ingestion for modelProviders, auth-type/protocol passthrough, and environment-variable wiring before it can wrap Qwen accurately."
---

# Qwen Code (Qwen CLI) Model Support

Qwen Code is Alibaba/QwenLM's open-source agentic CLI. It is a multi-protocol agent: it can speak the OpenAI, Anthropic, Gemini, and Qwen/DashScope API shapes, and it can route to local OpenAI-compatible servers. This document focuses on how models are discovered, configured, and selected.

## Models Available

Qwen Code **does not ship with a usable default model out of the box**. The original Qwen OAuth free tier was discontinued on 2026-04-15, so on first launch the CLI prompts you to run `/auth` and connect a provider. Once a provider is configured, the model catalog becomes available.

### Built-in fallback default

When no explicit model is configured and the active auth type resolves to the OpenAI-compatible protocol, the built-in default model ID is:

| Model ID | Notes |
|----------|-------|
| `qwen3.5-plus` | Hard-coded default for the OpenAI-compatible protocol resolver. |

### Alibaba Cloud Coding Plan (auto-configured via `/auth`)

Choosing **Alibaba ModelStudio → Coding Plan** in the `/auth` flow auto-configures the following models and adds them to the `/model` picker:

| Model ID | Notes |
|----------|-------|
| `qwen3.5-plus` | Advanced model with thinking enabled. |
| `qwen3.6-plus` | Latest model with thinking enabled; Pro subscribers only. |
| `qwen3.7-plus` | Advanced model with thinking enabled. |
| `qwen3-coder-plus` | Optimized for coding tasks. |
| `qwen3-coder-next` | Experimental coding model. |
| `qwen3-max-2026-01-23` | Latest max model with thinking enabled. |
| `glm-5` | GLM model with thinking enabled. |
| `glm-4.7` | GLM model with thinking enabled. |
| `kimi-k2.5` | Kimi model with thinking and vision/video support. |
| `MiniMax-M2.5` | MiniMax model with thinking enabled. |

### Third-party providers

The `/auth` → **Third-party Providers** menu offers built-in setup for DeepSeek, MiniMax, Z.AI, Idealab, ModelScope, OpenRouter, and Requesty. These are registered as `openai`-protocol entries in `modelProviders`.

### Local / self-hosted models

Any local inference server with an OpenAI-compatible endpoint (Ollama, vLLM, LM Studio, SGLang, etc.) is registered under the `openai` auth type with a local `baseUrl`.

## Model Configuration Details

### Schema — informal

Qwen Code publishes **no formal schema artifact** (no JSON Schema, OpenAPI, or protobuf) for model configuration. The authoritative informal schema is the prose-and-examples [Model Providers](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/model-providers/) page. The configuration surface consists of:

- `modelProviders` in `settings.json` — a per-auth-type catalog of model entries.
- Each entry requires `id` and accepts optional `name`, `description`, `envKey`, `baseUrl`, and `generationConfig`.
- `generationConfig` is treated as an **impermeable, atomic layer** when a provider model is selected: every field in the provider entry wins and missing fields are set to `undefined`, not inherited from top-level `model.generationConfig`.
- Runtime/ad-hoc models are created when `--model`, env vars, or `model.name` are used without a matching `modelProviders` entry.

### How a model is selected

Models can be chosen through five mechanisms:

| Mechanism | Site | Example |
|-----------|------|---------|
| Interactive command | `/model` | `/model qwen3-coder-plus` |
| Interactive command | `/auth` | `/auth` → select Coding Plan |
| CLI flag | `--model` / `-m` | `qwen --model qwen3-coder-plus` |
| CLI flag | `--auth-type` | `qwen --auth-type openai` |
| Environment variable | `OPENAI_MODEL` / `QWEN_MODEL` | `OPENAI_MODEL=qwen3-coder-plus qwen` |
| Environment variable | `ANTHROPIC_MODEL` | `ANTHROPIC_MODEL=claude-sonnet-4-20250514 qwen` |
| Environment variable | `GEMINI_MODEL` / `GOOGLE_MODEL` | `GEMINI_MODEL=gemini-2.5-pro qwen` |
| Config file | `model.name` | `"model": { "name": "qwen3-coder-plus" }` |
| Config file | `modelProviders` | `"modelProviders": { "openai": { "models": [...] } }` |
| Wire envelope | Request body `model` | `{ "model": "qwen3-coder-plus" }` |

**Precedence (highest wins):**

1. **Interactive commands** — `/model` at runtime, `/auth` at setup.
2. **CLI flags** — `--model`, `--auth-type`, plus provider-specific flags like `--openai-api-key` / `--openai-base-url`.
3. **Environment variables** — provider-specific mappings (`OPENAI_MODEL`, `ANTHROPIC_MODEL`, `GEMINI_MODEL`, `GOOGLE_MODEL`).
4. **Config files** — `model.name`, `modelProviders`, `security.auth.selectedType` in `settings.json`.
5. **Built-in default** — `qwen3.5-plus` for the OpenAI-compatible protocol.

```mermaid
flowchart TD
    A[Launch qwen] --> B{/model used in running session?}
    B -- yes --> C[Use /model selection]
    B -- no --> D{--model / --auth-type set?}
    D -- yes --> E[Use CLI flag selection]
    D -- no --> F{Provider model env var set?}
    F -- yes --> G[Use env var model]
    F -- no --> H{model.name or modelProviders configured?}
    H -- yes --> I[Use settings.json selection]
    H -- no --> J[Fall back to built-in default qwen3.5-plus]
    C --> K[Resolve credentials via envKey / env / CLI flags]
    E --> K
    G --> K
    I --> K
    J --> K
    K --> L[Send request with resolved model ID]
```

When a model is selected from `modelProviders`, its `generationConfig` is applied as a sealed package; lower layers do not merge into it.

### Programmatic model enumeration — not available

Qwen Code **cannot** enumerate its model catalog programmatically:

- There is **no `qwen models` / `qwen list-models` subcommand**.
- There is **no `--list-models` flag** or model-catalog API.
- There is **no config dump** that emits the resolved catalog.

The `/model` picker is the sole native catalog view and is interactive. Non-interactive consumers must read the resolved model from the `--output-format json` / `stream-json` result events.

## Sources

- [Qwen Code overview](https://qwenlm.github.io/qwen-code-docs/en/users/overview/) *(installation, first-run /auth flow, multi-protocol support)*
- [Qwen Code — Model Providers](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/model-providers/) *(modelProviders schema, auth types, generationConfig layering, precedence, RuntimeModelSnapshot)*
- [Qwen Code — Authentication](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/auth/) *(Qwen OAuth discontinued, Coding Plan setup, API-key providers, env vars, CLI flags)*
- [Qwen Code — Settings](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/) *(settings.json layers, model.name, CLI arguments, env vars)*
- [Qwen Code — Commands](https://qwenlm.github.io/qwen-code-docs/en/users/features/commands/) *(slash commands including /model and /auth)*
- [Qwen Code — Headless Mode](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/) *(non-interactive execution, output formats, resolved model in result events)*
- [Qwen Code repository](https://github.com/QwenLM/qwen-code)

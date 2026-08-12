---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-01
agent: open_code
model: kimi-for-coding/k2p7
has_official_schema: informal
schema_url: https://moonshotai.github.io/kimi-cli/en/configuration/config-files.md
default_models:
  - id: kimi-for-coding
    alias: kimi-code
    context_window: 262144
    is_default: true
    notes: Default model for the Kimi Code platform after OAuth login. The managed config key written by /login is `kimi-code/kimi-for-coding`; the API model identifier is `kimi-for-coding`.
  - id: kimi-k2.7-code
    context_window: 262144
    is_default: false
    notes: Available via Moonshot AI Open Platform (moonshot.cn / moonshot.ai) after API-key login. Coding-optimized model.
  - id: kimi-k2.7-code-highspeed
    context_window: 262144
    is_default: false
    notes: High-speed variant of K2.7 Code on Moonshot Open Platform.
  - id: kimi-k2.6
    context_window: 262144
    is_default: false
    notes: General multimodal model on Moonshot Open Platform; supports thinking toggle, image/video input.
  - id: kimi-k2.5
    context_window: 262144
    is_default: false
    notes: General multimodal model on Moonshot Open Platform.
  - id: moonshot-v1-8k
    context_window: 8192
    is_default: false
    notes: Moonshot V1 generation model on Moonshot Open Platform.
  - id: moonshot-v1-32k
    context_window: 32768
    is_default: false
    notes: Moonshot V1 generation model on Moonshot Open Platform.
  - id: moonshot-v1-128k
    context_window: 131072
    is_default: false
    notes: Moonshot V1 generation model on Moonshot Open Platform.
  - id: moonshot-v1-8k-vision-preview
    context_window: 8192
    is_default: false
    notes: Vision preview variant on Moonshot Open Platform.
  - id: moonshot-v1-32k-vision-preview
    context_window: 32768
    is_default: false
    notes: Vision preview variant on Moonshot Open Platform.
  - id: moonshot-v1-128k-vision-preview
    context_window: 131072
    is_default: false
    notes: Vision preview variant on Moonshot Open Platform.
model_selection:
  - method: cli_flag
    site: --model
    example: kimi --model kimi-for-coding
    notes: Must match a key in the `models` table of the active configuration file.
  - method: env_var
    site: KIMI_MODEL_NAME
    example: KIMI_MODEL_NAME=kimi-k2.7-code kimi
    notes: Overrides the `model` field of the current provider's model config for `kimi`-type providers.
  - method: config_file
    site: default_model
    example: default_model = "kimi-for-coding"
    notes: Top-level config key pointing to a model key defined in the `models` table.
  - method: interactive_command
    site: /model
    example: /model
    notes: Refreshes available models from the provider API and opens an interactive picker; writes the chosen model back to config.toml and reloads.
  - method: wire_envelope
    site: ACP session model state
    example: models.current_model_id
    notes: ACP `initialize`/`load_session` responses expose current and available models; model selection happens at session creation, not per-turn.
precedence: "interactive_command (/model, runtime) > env_var (KIMI_MODEL_NAME) > cli_flag (--model) > config_file (default_model)"
dynamic_listing:
  available: true
  method: "Provider GET /v1/models endpoint (queried at /login and when running /model); ACP initialize/load_session exposes available_models"
  example: "GET https://api.kimi.com/coding/v1/models; ACP response field `models.available_models`"
changes: []
requires_claudine_update: true
reason: "Claudine's model_catalog module currently expects a static provider model list. Kimi Code CLI ships with no built-in models and populates its catalog dynamically from the provider's /models endpoint at /login, using managed provider keys (e.g., `managed:kimi-code`) and platform-prefixed model keys (e.g., `kimi-code/kimi-for-coding`). It also supports six provider back-ends (kimi, openai_legacy, openai_responses, anthropic, gemini, vertexai) and env-var-based model override (KIMI_MODEL_NAME). Claudine needs to handle dynamic model discovery, managed key prefixes, and per-provider backend typing for Kimi."
---

# Kimi Code CLI Model Support

## Models Available

Kimi Code CLI does **not** ship with a hard-coded model catalog. On a fresh install the configuration file (`~/.kimi/config.toml`) is created with an empty `models` table, an empty `providers` table, and `default_model = ""`. Models are introduced into the CLI in one of two ways:

1. **OAuth/API-key login** — running `/login` (or `kimi login`) selects a platform and fetches the current model list from that platform's `GET /v1/models` endpoint.
2. **Manual configuration** — editing `config.toml` (or passing `--config` / `--config-file`) to add `providers` and `models` entries.

### Built-in platform support

`/login` currently supports three platforms out of the box:

| Platform | Provider `type` | Base URL | Model ID prefix |
| --- | --- | --- | --- |
| Kimi Code | `kimi` | `https://api.kimi.com/coding/v1` | n/a (uses `kimi-for-coding`) |
| Moonshot AI Open Platform (CN) | `kimi` | `https://api.moonshot.cn/v1` | `kimi-k` |
| Moonshot AI Open Platform (Global) | `kimi` | `https://api.moonshot.ai/v1` | `kimi-k` |

When OAuth login completes for the **Kimi Code** platform, the CLI writes a managed provider (`managed:kimi-code`) and managed models whose config keys are prefixed with the platform id, e.g. `kimi-code/kimi-for-coding`. The first model returned by the API is selected as the default.

### Models typically available after login

The table below combines the models documented by Moonshot/Kimi with the context-window sizes published in the platform pricing docs. Because the CLI reads the live `/models` endpoint, the exact set may change.

| Model ID | Platform | Context window | Notes |
| --- | --- | --- | --- |
| `kimi-for-coding` | Kimi Code | 262,144 | Default model after Kimi Code OAuth login; also aliased as `kimi-code` in display logic. |
| `kimi-k2.7-code` | Moonshot Open Platform | 262,144 | Coding-optimized model; supports thinking, image and video input. |
| `kimi-k2.7-code-highspeed` | Moonshot Open Platform | 262,144 | Faster variant of `kimi-k2.7-code`. |
| `kimi-k2.6` | Moonshot Open Platform | 262,144 | General multimodal model; supports thinking toggle. |
| `kimi-k2.5` | Moonshot Open Platform | 262,144 | General multimodal model. |
| `moonshot-v1-8k` | Moonshot Open Platform | 8,192 | V1 generation model. |
| `moonshot-v1-32k` | Moonshot Open Platform | 32,768 | V1 generation model. |
| `moonshot-v1-128k` | Moonshot Open Platform | 131,072 | V1 generation model. |
| `moonshot-v1-8k-vision-preview` | Moonshot Open Platform | 8,192 | Vision preview model. |
| `moonshot-v1-32k-vision-preview` | Moonshot Open Platform | 32,768 | Vision preview model. |
| `moonshot-v1-128k-vision-preview` | Moonshot Open Platform | 131,072 | Vision preview model. |

### Adding bespoke or local models

Bespoke models are registered by adding new entries to the `providers` and `models` tables in `~/.kimi/config.toml`. The CLI supports six provider `type` values, so a local model can be exposed through any compatible protocol:

- **OpenAI-compatible local server** (`openai_legacy` or `openai_responses`): point `base_url` at a local endpoint such as Ollama, vLLM, or llama.cpp.
- **Anthropic-compatible gateway** (`anthropic`): route through a Claude Messages API proxy.
- **Google Gemini / Vertex AI** (`gemini` / `vertexai`).

There is no separate "model marketplace", "plugin", or first-class "local model" concept — every model is just a provider + model pair in the config file.

## Model Configuration Details

### Schema — informal

Kimi Code CLI does not publish a formal schema artifact (JSON Schema, OpenAPI, or protobuf) for its model configuration. Validation is performed by Pydantic models in [`src/kimi_cli/config.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/config.py) and [`src/kimi_cli/auth/platforms.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/auth/platforms.py). The user-facing contract is documented informally in the [Config Files](https://moonshotai.github.io/kimi-cli/en/configuration/config-files.md) and [Providers and Models](https://moonshotai.github.io/kimi-cli/en/configuration/providers.md) pages.

The relevant configuration shapes are:

- Top-level: `default_model` (string, must be a key in `models`).
- `[providers.<name>]`: `type`, `base_url`, `api_key`, optional `env`, `custom_headers`, `reasoning_key`, `oauth`.
- `[models.<name>]`: `provider`, `model`, `max_context_size`, optional `capabilities`, `display_name`.

### How a model is selected

Kimi Code CLI exposes four model-selection surfaces:

1. **CLI flag at startup** — `kimi --model <model-key>` (short `-m`). The value must be a key in the current config's `models` table.
2. **Environment variable** — `KIMI_MODEL_NAME` overrides the `model` field inside the active model config for `kimi`-type providers.
3. **Configuration file** — `default_model` names the initial model.
4. **Interactive slash command** — `/model` (no arg opens a picker, or pass a model key) refreshes the catalog from the provider API, switches the active model, and writes the choice back to `config.toml`.
5. **ACP/Wire envelope** — ACP session creation returns the current model and available models in `models.current_model_id` / `models.available_models`. Wire mode's `initialize` returns slash commands but does not expose the model catalog; model selection in Wire happens through the config that started the server.

**Precedence:**

```text
interactive_command (/model at runtime) > env_var (KIMI_MODEL_NAME) > cli_flag (--model) > config_file (default_model)
```

Note that this precedence differs from some other agentic CLIs: `KIMI_MODEL_NAME` wins over `--model` at launch time.

### Programmatic model enumeration

There is **no** `kimi models` subcommand or `--list-models` flag. However, the catalog can be enumerated programmatically through the underlying provider API and through ACP:

| Mechanism | Method | Example |
| --- | --- | --- |
| Provider API | `GET /v1/models` on the configured `base_url` | `curl https://api.kimi.com/coding/v1/models -H "Authorization: Bearer $TOKEN"` |
| Interactive CLI | `/model` slash command | `kimi` then type `/model` |
| ACP session setup | `models.available_models` in `initialize`/`load_session` response | Returned automatically when an ACP client connects |

The `/model` picker and the ACP response both refresh from the provider's `/models` endpoint, so the catalog is dynamic rather than static.

### Related model-behavior configuration

| Concern | Mechanism | Notes |
| --- | --- | --- |
| **Thinking mode** | `--thinking` / `--no-thinking`, `/model` picker, `default_thinking`, `capabilities = ["thinking"]` or `["always_thinking"]` | Models with `"thinking"` in the API id automatically enable `always_thinking`. |
| **Generation parameters** | `KIMI_MODEL_TEMPERATURE`, `KIMI_MODEL_TOP_P`, `KIMI_MODEL_MAX_TOKENS` | Passed through to the Kimi provider only. |
| **Preserved thinking** | `KIMI_MODEL_THINKING_KEEP` | Forwarded as `thinking.keep` on Moonshot thinking models. |
| **Max context size** | `max_context_size` in model config, or `KIMI_MODEL_MAX_CONTEXT_SIZE` | Drives compaction; does not constrain the API unless the provider honors it. |

## Sources

- [Kimi Code CLI Docs — Providers and Models](https://moonshotai.github.io/kimi-cli/en/configuration/providers.md)
- [Kimi Code CLI Docs — Config Files](https://moonshotai.github.io/kimi-cli/en/configuration/config-files.md)
- [Kimi Code CLI Docs — Config Overrides](https://moonshotai.github.io/kimi-cli/en/configuration/overrides.md)
- [Kimi Code CLI Docs — Environment Variables](https://moonshotai.github.io/kimi-cli/en/configuration/env-vars.md)
- [Kimi Code CLI Docs — `kimi` Command Reference](https://moonshotai.github.io/kimi-cli/en/reference/kimi-command.md)
- [Kimi Code CLI Docs — Slash Commands](https://moonshotai.github.io/kimi-cli/en/reference/slash-commands.md)
- [Kimi Code CLI Docs — Wire Mode](https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.md)
- [Kimi Code CLI Source — `src/kimi_cli/config.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/config.py)
- [Kimi Code CLI Source — `src/kimi_cli/auth/platforms.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/auth/platforms.py)
- [Kimi Code CLI Source — `src/kimi_cli/llm.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/llm.py)
- [Kimi API Docs — Model List](https://platform.kimi.com/docs/models.md)
- [Kimi API Docs — List Models API](https://platform.kimi.com/docs/api/list-models.md)
- [Kimi API Docs — Kimi K2.7 Code Pricing](https://platform.kimi.com/docs/pricing/chat-k27-code.md)
- [Kimi API Docs — Kimi K2.6 Pricing](https://platform.kimi.com/docs/pricing/chat-k26.md)
- [Kimi API Docs — Kimi K2.5 Pricing](https://platform.kimi.com/docs/pricing/chat-k25.md)

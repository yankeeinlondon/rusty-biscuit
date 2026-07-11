---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-01
agent: open_code
model: kimi-for-coding/k2p7
has_official_schema: formal
schema_url: https://opencode.ai/config.json
default_models:
  - id: opencode/big-pickle
    context_window: 200000
    is_default: false
  - id: opencode/claude-fable-5
    context_window: 1000000
    is_default: false
  - id: opencode/claude-haiku-4-5
    context_window: 200000
    is_default: false
  - id: opencode/claude-opus-4-1
    context_window: 200000
    is_default: false
  - id: opencode/claude-opus-4-5
    context_window: 200000
    is_default: false
  - id: opencode/claude-opus-4-6
    context_window: 1000000
    is_default: false
  - id: opencode/claude-opus-4-7
    context_window: 1000000
    is_default: false
  - id: opencode/claude-opus-4-8
    context_window: 1000000
    is_default: false
  - id: opencode/claude-sonnet-4
    context_window: 1000000
    is_default: false
  - id: opencode/claude-sonnet-4-5
    context_window: 1000000
    is_default: false
  - id: opencode/claude-sonnet-4-6
    context_window: 1000000
    is_default: false
  - id: opencode/claude-sonnet-5
    context_window: 1000000
    is_default: false
  - id: opencode/deepseek-v4-flash
    context_window: 1000000
    is_default: false
  - id: opencode/deepseek-v4-flash-free
    context_window: 200000
    is_default: false
  - id: opencode/deepseek-v4-pro
    context_window: 1000000
    is_default: false
  - id: opencode/gemini-3-flash
    context_window: 1048576
    is_default: false
  - id: opencode/gemini-3.1-pro
    context_window: 1048576
    is_default: false
  - id: opencode/gemini-3.5-flash
    context_window: 1048576
    is_default: false
  - id: opencode/glm-5
    context_window: 204800
    is_default: false
  - id: opencode/glm-5.1
    context_window: 204800
    is_default: false
  - id: opencode/glm-5.2
    context_window: 1000000
    is_default: false
  - id: opencode/gpt-5
    context_window: 400000
    is_default: false
  - id: opencode/gpt-5-codex
    context_window: 400000
    is_default: false
  - id: opencode/gpt-5-nano
    context_window: 400000
    is_default: false
  - id: opencode/gpt-5.1
    context_window: 400000
    is_default: false
  - id: opencode/gpt-5.1-codex
    context_window: 400000
    is_default: false
  - id: opencode/gpt-5.1-codex-max
    context_window: 400000
    is_default: false
  - id: opencode/gpt-5.1-codex-mini
    context_window: 400000
    is_default: false
  - id: opencode/gpt-5.2
    context_window: 400000
    is_default: false
  - id: opencode/gpt-5.2-codex
    context_window: 400000
    is_default: false
  - id: opencode/gpt-5.3-codex
    context_window: 400000
    is_default: false
  - id: opencode/gpt-5.3-codex-spark
    context_window: 128000
    is_default: false
  - id: opencode/gpt-5.4
    context_window: 1050000
    is_default: false
  - id: opencode/gpt-5.4-mini
    context_window: 400000
    is_default: false
  - id: opencode/gpt-5.4-nano
    context_window: 400000
    is_default: false
  - id: opencode/gpt-5.4-pro
    context_window: 1050000
    is_default: false
  - id: opencode/gpt-5.5
    context_window: 1050000
    is_default: false
  - id: opencode/gpt-5.5-pro
    context_window: 1050000
    is_default: false
  - id: opencode/grok-build-0.1
    context_window: 256000
    is_default: false
  - id: opencode/kimi-k2.5
    context_window: 262144
    is_default: false
  - id: opencode/kimi-k2.6
    context_window: 262144
    is_default: false
  - id: opencode/kimi-k2.7-code
    context_window: 262144
    is_default: false
  - id: opencode/mimo-v2.5-free
    context_window: 200000
    is_default: false
  - id: opencode/minimax-m2.5
    context_window: 204800
    is_default: false
  - id: opencode/minimax-m2.7
    context_window: 204800
    is_default: false
  - id: opencode/minimax-m3
    context_window: 512000
    is_default: false
  - id: opencode/nemotron-3-ultra-free
    context_window: 1000000
    is_default: false
  - id: opencode/north-mini-code-free
    context_window: 256000
    is_default: false
  - id: opencode/qwen3.5-plus
    context_window: 262144
    is_default: false
  - id: opencode/qwen3.6-plus
    context_window: 262144
    is_default: false
model_selection:
  - method: cli_flag
    site: "--model / -m"
    example: "opencode run -m opencode/gpt-5.5 \"Explain async/await\""
    notes: "Sets the model for the launched command or session only. The value is a provider/model ID."
  - method: config_file
    site: "model"
    example: "{ \"model\": \"opencode/gpt-5.5\" }"
    notes: "Top-level key in opencode.json. A separate small_model key is used for lightweight tasks such as title generation."
  - method: env_var
    site: "OPENCODE_CONFIG_CONTENT"
    example: "OPENCODE_CONFIG_CONTENT='{\"model\":\"opencode/gpt-5.5\"}' opencode run \"Hello\""
    notes: "Delivers an inline JSON config override through an environment variable. OPENCODE_CONFIG can also point to a custom config file path."
  - method: interactive_command
    site: "/models"
    example: "/models"
    notes: "TUI slash command that opens the interactive model picker. Use /connect to add provider credentials first."
  - method: wire_envelope
    site: "AI SDK request body `model` field"
    example: "{ \"model\": \"opencode/gpt-5.5\" }"
    notes: "Resolved provider/model ID sent on each inference request after alias and variant expansion."
precedence: "interactive_command (/models, runtime) > cli_flag (--model / -m) > env_var (OPENCODE_CONFIG_CONTENT inline config; OPENCODE_CONFIG custom path is loaded as a config_file layer) > config_file (model key; layered remote < global < OPENCODE_CONFIG custom path < project < .opencode dirs < OPENCODE_CONFIG_CONTENT < managed settings) > last used model > internal default priority"
dynamic_listing:
  available: true
  method: "CLI subcommand `opencode models [provider]` with optional `--refresh` and `--verbose`"
  example: "opencode models --refresh"
changes: []
requires_claudine_update: true
reason: "Claudine's OpenCode adapter currently focuses on event streaming and lifecycle hooks. It does not yet model the formal opencode.json schema, the provider/model ID format, the opencode models dynamic catalog, model variants, or the small_model default. To normalize model selection and merge the OpenCode catalog with Claudine's model_catalog, these surfaces need to be added."

---

# OpenCode CLI Model Support

## Model's Available

OpenCode is a provider-agnostic agentic CLI. It uses the [AI SDK](https://ai-sdk.dev/) and [Models.dev](https://models.dev/) to support 75+ LLM providers, and it can run local models through OpenAI-compatible endpoints.

### Out-of-the-box catalog

On installation the CLI ships with built-in provider definitions, but a model is not usable until the corresponding provider credentials are configured (for example with `/connect` or `opencode auth login`). The primary curated provider that is available out of the box is **OpenCode Zen**, which exposes tested models under the `opencode/<model-id>` namespace. The full Zen catalog can be fetched from `https://opencode.ai/zen/v1/models` and listed locally with `opencode models opencode`.

| Model ID | Context window | Notes |
|----------|----------------|-------|
| `opencode/big-pickle` | 200K | Free limited-time stealth model |
| `opencode/claude-fable-5` | 1M | Highest-capability Claude model |
| `opencode/claude-haiku-4-5` | 200K | Fast/efficient Claude model |
| `opencode/claude-opus-4-1` | 200K | Deprecated in docs but still listed |
| `opencode/claude-opus-4-5` | 200K | Legacy Opus |
| `opencode/claude-opus-4-6` | 1M | Opus 4.6 |
| `opencode/claude-opus-4-7` | 1M | Opus 4.7 |
| `opencode/claude-opus-4-8` | 1M | Latest Opus at time of research |
| `opencode/claude-sonnet-4` | 1M | Deprecated in docs but still listed |
| `opencode/claude-sonnet-4-5` | 1M | Sonnet 4.5 |
| `opencode/claude-sonnet-4-6` | 1M | Sonnet 4.6 |
| `opencode/claude-sonnet-5` | 1M | Latest Sonnet at time of research |
| `opencode/deepseek-v4-flash` | 1M | Fast DeepSeek model |
| `opencode/deepseek-v4-flash-free` | 200K | Free tier |
| `opencode/deepseek-v4-pro` | 1M | DeepSeek V4 Pro |
| `opencode/gemini-3-flash` | ~1M | Google Gemini 3 Flash |
| `opencode/gemini-3.1-pro` | ~1M | Google Gemini 3.1 Pro Preview |
| `opencode/gemini-3.5-flash` | ~1M | Google Gemini 3.5 Flash |
| `opencode/glm-5` | 200K | Z.AI GLM 5 |
| `opencode/glm-5.1` | 200K | Z.AI GLM 5.1 |
| `opencode/glm-5.2` | 1M | Z.AI GLM 5.2 |
| `opencode/gpt-5` | 400K | OpenAI GPT 5 |
| `opencode/gpt-5-codex` | 400K | GPT 5 Codex |
| `opencode/gpt-5-nano` | 400K | Default small_model for title generation |
| `opencode/gpt-5.1` | 400K | GPT 5.1 |
| `opencode/gpt-5.1-codex` | 400K | GPT 5.1 Codex |
| `opencode/gpt-5.1-codex-max` | 400K | GPT 5.1 Codex Max |
| `opencode/gpt-5.1-codex-mini` | 400K | GPT 5.1 Codex Mini |
| `opencode/gpt-5.2` | 400K | GPT 5.2 |
| `opencode/gpt-5.2-codex` | 400K | GPT 5.2 Codex |
| `opencode/gpt-5.3-codex` | 400K | GPT 5.3 Codex |
| `opencode/gpt-5.3-codex-spark` | 128K | GPT 5.3 Codex Spark |
| `opencode/gpt-5.4` | 1.05M | GPT 5.4 |
| `opencode/gpt-5.4-mini` | 400K | GPT 5.4 Mini |
| `opencode/gpt-5.4-nano` | 400K | GPT 5.4 Nano |
| `opencode/gpt-5.4-pro` | 1.05M | GPT 5.4 Pro |
| `opencode/gpt-5.5` | 1.05M | GPT 5.5 |
| `opencode/gpt-5.5-pro` | 1.05M | GPT 5.5 Pro |
| `opencode/grok-build-0.1` | 256K | xAI Grok Build |
| `opencode/kimi-k2.5` | 262K | Moonshot Kimi K2.5 |
| `opencode/kimi-k2.6` | 262K | Moonshot Kimi K2.6 |
| `opencode/kimi-k2.7-code` | 262K | Moonshot Kimi K2.7 Code |
| `opencode/mimo-v2.5-free` | 200K | Xiaomi MiMo free tier |
| `opencode/minimax-m2.5` | 200K | MiniMax M2.5 |
| `opencode/minimax-m2.7` | 200K | MiniMax M2.7 |
| `opencode/minimax-m3` | 512K | MiniMax M3 |
| `opencode/nemotron-3-ultra-free` | 1M | NVIDIA Nemotron free tier |
| `opencode/north-mini-code-free` | 256K | North Mini Code free tier |
| `opencode/qwen3.5-plus` | 262K | Alibaba Qwen3.5 Plus |
| `opencode/qwen3.6-plus` | 262K | Alibaba Qwen3.6 Plus |

Other providers become available as soon as their credentials are added; their model IDs follow the same `provider/model` format and can be discovered with `opencode models <provider>`.

### Adding bespoke and local models

OpenCode supports bespoke models in several ways:

1. **Local OpenAI-compatible servers** — configure `provider.<id>` with `npm: @ai-sdk/openai-compatible`, a `baseURL`, and a `models` map. Documented examples include [Ollama](https://opencode.ai/docs/providers/#ollama), [LM Studio](https://opencode.ai/docs/providers/#lm-studio), [llama.cpp](https://opencode.ai/docs/providers/#llamacpp), and [Atomic Chat](https://opencode.ai/docs/providers/#atomic-chat).
2. **Custom OpenAI-compatible provider** — use the same `@ai-sdk/openai-compatible` package for any proxy or endpoint that speaks the OpenAI Chat Completions API.
3. **Extend a built-in provider** — add entries under `provider.<built-in>.models` (for example OpenRouter, LLM Gateway, Helicone, or a custom Amazon Bedrock inference profile).
4. **Proxy or BYOK for a built-in provider** — set `provider.<built-in>.options.baseURL` to route requests through a gateway, or use your own OpenAI/Anthropic key inside OpenCode Zen.

## Model Configuration Details

### Schema

OpenCode publishes a **formal JSON Schema** for its runtime config at [`https://opencode.ai/config.json`](https://opencode.ai/config.json). The schema references [`https://models.dev/model-schema.json`](https://models.dev/model-schema.json) for model identifiers. Config files may be written in JSON or JSONC.

Key model-related schema locations:

- Top-level `model` and `small_model` strings.
- `provider.<id>.models` map, where each model may carry `id`, `name`, `family`, `limit`, `cost`, `capabilities`, `variants`, `options`, etc.
- `agent.<id>.model` and `command.<id>.model` for per-agent or per-command model overrides.

### Selecting a model

| Mechanism | Site | Example |
|-----------|------|---------|
| CLI flag | `--model` / `-m` | `opencode run -m opencode/gpt-5.5 "Explain closures"` |
| Config file | `model` key | `{ "model": "opencode/gpt-5.5" }` |
| Environment variable | `OPENCODE_CONFIG_CONTENT` | `OPENCODE_CONFIG_CONTENT='{"model":"opencode/gpt-5.5"}' opencode run ...` |
| Interactive command | `/models` | `/models` in the TUI |
| Wire envelope | AI SDK `model` field | `{ "model": "opencode/gpt-5.5" }` |

### Precedence

OpenCode resolves the active model in the following order:

1. **Runtime interactive selection** — `/models` inside an open TUI session.
2. **CLI flag** — `--model` / `-m` at launch.
3. **Environment variable** — `OPENCODE_CONFIG_CONTENT` (inline JSON config override). `OPENCODE_CONFIG` is treated as a custom config file layer.
4. **Config file** — the `model` key, with layered merging: remote (`.well-known/opencode`) < global (`~/.config/opencode/opencode.json`) < custom path (`OPENCODE_CONFIG`) < project (`opencode.json`) < `.opencode` directories < inline (`OPENCODE_CONFIG_CONTENT`) < managed settings / MDM.
5. **Last used model** — persisted session state.
6. **Internal default priority** — first model from the built-in priority list when nothing else is set.

### Programmatic model enumeration

OpenCode **can** enumerate its model catalog programmatically:

- `opencode models` — list all available models from configured providers.
- `opencode models <provider>` — filter to one provider.
- `opencode models --refresh` — refresh the cached model list from models.dev.
- `opencode models --verbose` — include metadata such as context limits, costs, and capabilities.

The interactive TUI equivalent is the `/models` slash command.

## Sources

- [OpenCode website](https://opencode.ai)
- [OpenCode GitHub repository](https://github.com/anomalyco/opencode)
- [OpenCode documentation home](https://opencode.ai/docs)
- [OpenCode Models documentation](https://opencode.ai/docs/models)
- [OpenCode Providers documentation](https://opencode.ai/docs/providers)
- [OpenCode Config documentation](https://opencode.ai/docs/config)
- [OpenCode CLI documentation](https://opencode.ai/docs/cli)
- [OpenCode Zen documentation](https://opencode.ai/docs/zen)
- [OpenCode runtime config JSON Schema](https://opencode.ai/config.json)

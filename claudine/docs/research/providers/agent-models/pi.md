---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-01
agent: open_code
model: kimi-for-coding/k2p7
has_official_schema: formal
schema_url: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/model-registry.ts

default_models:
  - id: claude-opus-4-8
    context_window: 1000000
    is_default: true
    notes: "Practical default when Anthropic auth is configured. Pi's real fallback is auth-dependent: it scans a provider-preference table and picks the first available model with valid credentials."
  - id: us.anthropic.claude-opus-4-6-v1
    alias: "amazon-bedrock default"
    context_window: 1000000
    notes: "Default for Amazon Bedrock provider."
  - id: Ring-2.6-1T
    alias: "ant-ling default"
    notes: "Default for Ant Ling provider."
  - id: gpt-5.5
    alias: "openai default"
    notes: "Default for the OpenAI provider and the OpenAI Codex (ChatGPT Plus/Pro) provider (both resolve to the same model id)."
  - id: gpt-5.4
    alias: "azure-openai default"
    notes: "Default for the Azure OpenAI Responses provider and the GitHub Copilot provider (both resolve to the same model id)."
  - id: nvidia/nemotron-3-super-120b-a12b
    alias: "nvidia default"
    notes: "Default for NVIDIA NIM provider."
  - id: deepseek-v4-pro
    alias: "deepseek default"
    notes: "Default for DeepSeek provider."
  - id: gemini-3.1-pro-preview
    alias: "google default"
    notes: "Default for Google Gemini and Google Vertex providers."
  - id: moonshotai/kimi-k2.6
    alias: "openrouter default"
    notes: "Default for OpenRouter provider."
  - id: zai/glm-5.1
    alias: "vercel-ai-gateway default"
    notes: "Default for Vercel AI Gateway provider."
  - id: grok-4.20-0309-reasoning
    alias: "xai default"
    notes: "Default for xAI provider."
  - id: openai/gpt-oss-120b
    alias: "groq default"
    notes: "Default for Groq provider."
  - id: zai-glm-4.7
    alias: "cerebras default"
    notes: "Default for Cerebras provider."
  - id: glm-5.1
    alias: "zai default"
    notes: "Default for ZAI Coding Plan (Global and China) providers."
  - id: devstral-medium-latest
    alias: "mistral default"
    notes: "Default for Mistral provider."
  - id: MiniMax-M2.7
    alias: "minimax default"
    notes: "Default for MiniMax providers."
  - id: kimi-k2.6
    alias: "moonshotai default"
    notes: "Default for the Moonshot AI providers and the OpenCode Zen / OpenCode Go providers (both resolve to the same model id)."
  - id: moonshotai/Kimi-K2.6
    alias: "huggingface default"
    notes: "Default for the Hugging Face provider and the Together AI provider (both resolve to the same model id)."
  - id: accounts/fireworks/models/kimi-k2p6
    alias: "fireworks default"
    notes: "Default for Fireworks provider."
  - id: kimi-for-coding
    alias: "kimi-coding default"
    notes: "Default for Kimi For Coding provider."
  - id: "@cf/moonshotai/kimi-k2.6"
    alias: "cloudflare-workers-ai default"
    notes: "Default for Cloudflare Workers AI provider."
  - id: workers-ai/@cf/moonshotai/kimi-k2.6
    alias: "cloudflare-ai-gateway default"
    notes: "Default for Cloudflare AI Gateway provider."
  - id: mimo-v2.5-pro
    alias: "xiaomi default"
    notes: "Default for Xiaomi MiMo providers."

model_selection:
  - method: cli_flag
    site: "--model"
    example: 'pi --model claude-sonnet-4-5'
    notes: "Select a model at launch. Accepts a bare model id, provider/id, or pattern with optional :<thinking> suffix (e.g. sonnet:high). Highest-precedence model-selection surface."
  - method: cli_flag
    site: "--provider"
    example: 'pi --provider anthropic --model claude-opus-4-8'
    notes: "Used together with --model to disambiguate. --provider alone does not select a model."
  - method: cli_flag
    site: "--models"
    example: 'pi --models "claude-sonnet-4-5,gpt-5.5"'
    notes: "Comma-separated glob/fuzzy patterns that populate the Ctrl+P cycle list and scope the initial model choice."
  - method: cli_flag
    site: "--thinking"
    example: 'pi --thinking high'
    notes: "Sets the thinking level (off/minimal/low/medium/high/xhigh) for models that support it."
  - method: cli_flag
    site: "--api-key"
    example: 'pi --api-key sk-ant-... --model claude-opus-4-8'
    notes: "Supplies a one-time API key; must be paired with an explicit --model."
  - method: env_var
    site: "ANTHROPIC_API_KEY / OPENAI_API_KEY / etc."
    example: "ANTHROPIC_API_KEY=sk-ant-... pi"
    notes: "Provider API keys unlock the corresponding built-in models. See providers.md for the full env-var table."
  - method: env_var
    site: "PI_OFFLINE"
    example: "PI_OFFLINE=1 pi --list-models"
    notes: "Disables startup network operations but does not change model selection."
  - method: config_file
    site: "defaultProvider / defaultModel"
    example: '{ "defaultProvider": "anthropic", "defaultModel": "claude-opus-4-8" }'
    notes: "Global (~/.pi/agent/settings.json) or project (.pi/settings.json) defaults."
  - method: config_file
    site: "enabledModels"
    example: '{ "enabledModels": ["claude-sonnet-4-5", "gpt-5.5"] }'
    notes: "Patterns for Ctrl+P cycling; also used as scoped models at startup when --models is not provided."
  - method: config_file
    site: "defaultThinkingLevel"
    example: '{ "defaultThinkingLevel": "medium" }'
    notes: "Default thinking level applied to compatible models."
  - method: interactive_command
    site: "/model"
    example: "/model claude-sonnet-4-5"
    notes: "Switch model at runtime. Highest precedence within an open interactive session."
  - method: interactive_command
    site: "/scoped-models"
    example: "/scoped-models"
    notes: "Enable/disable models for Ctrl+P cycling."
  - method: wire_envelope
    site: "RPC set_model command"
    example: '{"type":"set_model","provider":"anthropic","modelId":"claude-opus-4-8"}'
    notes: "Runtime model switch via the JSON-RPC stdin protocol (--mode rpc)."

precedence: "cli_flag (--model/--provider/--models/--thinking) > interactive_command (/model at runtime) > config_file (defaultProvider/defaultModel/enabledModels/defaultThinkingLevel; project overrides global) > auth-based fallback (first available model from defaultModelPerProvider lookup)"

dynamic_listing:
  available: true
  method: "pi --list-models [search]"
  example: "pi --list-models claude"

changes: []
requires_claudine_update: true
reason: "Claudine's provider matrix and model_catalog module do not yet include Pi. Adding Pi support would require a new provider adapter, mapping Pi's model-selection surfaces (CLI flags, settings keys, RPC envelope, /model command), and handling Pi's auth-gated dynamic default model resolution."
---

# Pi Coding Agent — Model Support

## Model's Available

Pi is a **multi-provider** agent harness. It does not ship with a single fixed default model; instead it bundles built-in model catalogs for 15+ providers and selects from the subset that the user has authenticated.

### Built-in providers

Out of the box Pi knows how to talk to:

| Provider category | Providers |
|-------------------|-----------|
| Subscriptions | Claude Pro/Max, ChatGPT Plus/Pro (Codex), GitHub Copilot |
| API keys | Anthropic, Ant Ling, OpenAI, Azure OpenAI, DeepSeek, NVIDIA NIM, Google Gemini, Google Vertex, Mistral, Groq, Cerebras, Cloudflare AI Gateway, Cloudflare Workers AI, xAI, OpenRouter, Vercel AI Gateway, ZAI, OpenCode Zen/Go, Hugging Face, Fireworks, Together AI, Kimi For Coding, MiniMax, Xiaomi MiMo |
| Cloud (IAM) | Amazon Bedrock |

Each provider has its own generated `*.models.ts` catalog (e.g. [`anthropic.models.ts`](https://github.com/earendil-works/pi/blob/main/packages/ai/src/providers/anthropic.models.ts)). The full set of built-in models numbers in the hundreds and is updated with each Pi release.

### Default model resolution

When no explicit model is requested, Pi resolves an initial model in this order:

1. CLI `--model` / `--provider --model`.
2. First entry from the scoped model list (`--models` CLI flag or `enabledModels` setting).
3. Saved settings default (`defaultProvider` + `defaultModel`).
4. First available model with configured auth, preferring the `defaultModelPerProvider` lookup table in [`model-resolver.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/model-resolver.ts).

Because step 4 depends on which API keys or OAuth tokens are present, the "default model" is **auth-dependent**. In practice, a user with only `ANTHROPIC_API_KEY` set will start on `claude-opus-4-8`.

## Adding Bespoke Models

Pi supports custom models through two mechanisms:

### `models.json`

Add providers and models to `~/.pi/agent/models.json`. This is the recommended path for local models and OpenAI/Anthropic/Google-compatible endpoints.

```json
{
  "providers": {
    "ollama": {
      "baseUrl": "http://localhost:11434/v1",
      "api": "openai-completions",
      "apiKey": "ollama",
      "models": [
        { "id": "llama3.1:8b" },
        { "id": "qwen2.5-coder:7b" }
      ]
    }
  }
}
```

Supported `api` values: `openai-completions`, `openai-responses`, `anthropic-messages`, `google-generative-ai`, plus provider-specific APIs such as `mistral-conversations`, `google-vertex`, `bedrock-converse-stream`, etc. See [docs/models.md](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/models.md) for the full config reference.

### Extensions

For non-standard APIs, OAuth flows, or dynamic model discovery, a TypeScript extension can call `pi.registerProvider()`:

```ts
pi.registerProvider("my-provider", {
  baseUrl: "https://api.example.com",
  apiKey: "$MY_API_KEY",
  api: "openai-completions",
  models: [{ id: "my-model", name: "My Model", reasoning: false, input: ["text"], cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 }, contextWindow: 128000, maxTokens: 4096 }]
});
```

See [docs/custom-provider.md](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/custom-provider.md).

## Model Configuration Details

### Schema

Pi provides a **formal schema** for `models.json` using TypeBox in [`packages/coding-agent/src/core/model-registry.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/model-registry.ts). It validates the `providers` object, model definitions, per-model overrides, and compatibility flags. There is also an **informal schema** documented in prose in [docs/models.md](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/models.md).

### Selection mechanisms and precedence

| Surface | Mechanism | Example |
|---------|-----------|---------|
| Launch flag | `--model <pattern>` | `pi --model claude-sonnet-4-5` |
| Launch flag | `--provider <name> --model <id>` | `pi --provider anthropic --model claude-opus-4-8` |
| Launch flag | `--models <patterns>` | `pi --models "claude-*,gpt-5.5"` |
| Launch flag | `--thinking <level>` | `pi --thinking high` |
| Launch flag | `--api-key <key>` (must pair with `--model`) | `pi --api-key sk-... --model gpt-5.5` |
| Env var | Provider API keys | `ANTHROPIC_API_KEY=... pi` |
| Config file | `defaultProvider` / `defaultModel` | `~/.pi/agent/settings.json` |
| Config file | `enabledModels` | `~/.pi/agent/settings.json` |
| Config file | `defaultThinkingLevel` | `~/.pi/agent/settings.json` |
| Runtime command | `/model` | `/model claude-opus-4-8` |
| Runtime command | `/scoped-models` | `/scoped-models` |
| Wire envelope | RPC `set_model` | `{"type":"set_model","provider":"anthropic","modelId":"claude-opus-4-8"}` |

**Precedence:**

```text
CLI flags (--model, --provider, --models, --thinking)
  > interactive runtime command (/model)
  > config-file defaults (project .pi/settings.json overrides global ~/.pi/agent/settings.json)
  > auth-based fallback (first available model from the built-in defaultModelPerProvider table)
```

Note that `--model` can carry a thinking-level shorthand (`sonnet:high`), which is overridden by an explicit `--thinking` flag. Provider API keys do not select a model directly; they unlock the corresponding provider's models for the fallback and scoping logic.

### Programmatic catalog enumeration

Pi **can** enumerate its model catalog programmatically:

- CLI: `pi --list-models [search]` prints a table of available models (filtered by configured auth).
- JSON mode: `pi --mode json --list-models` is not a dedicated API, but the RPC protocol exposes `get_available_models`.
- RPC: send `{"type":"get_available_models"}` to receive the full list of `Model` objects.

Example:

```bash
pi --list-models claude
```

Output columns: provider, model id, context window, max-out, thinking support, image support.

## Sources

- [Pi homepage](https://pi.dev/)
- [Pi GitHub repository](https://github.com/earendil-works/pi)
- [Pi coding-agent README — CLI reference](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/README.md)
- [docs/models.md — Custom Models / models.json reference](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/models.md)
- [docs/custom-provider.md — Extension-based custom providers](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/custom-provider.md)
- [docs/providers.md — Provider auth and env vars](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/providers.md)
- [docs/settings.md — Settings reference](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/settings.md)
- [docs/rpc.md — RPC protocol including set_model / get_available_models](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/rpc.md)
- [packages/ai/src/providers/anthropic.models.ts — Anthropic built-in catalog](https://github.com/earendil-works/pi/blob/main/packages/ai/src/providers/anthropic.models.ts)
- [packages/coding-agent/src/core/model-resolver.ts — defaultModelPerProvider and model resolution](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/model-resolver.ts)
- [packages/coding-agent/src/core/model-registry.ts — TypeBox models.json schema](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/model-registry.ts)
- [packages/coding-agent/src/cli/list-models.ts — --list-models implementation](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/cli/list-models.ts)
- [packages/coding-agent/src/cli/args.ts — CLI flag parsing](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/cli/args.ts)
- [packages/coding-agent/src/main.ts — launch-time model selection orchestration](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/main.ts)

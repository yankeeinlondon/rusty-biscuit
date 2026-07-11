# User-Side Model Configuration Across Agentic CLIs

User-side model extension matters because agentic CLIs are long-running process coordinators, not just chat clients. The selected model determines latency, tool-call reliability, context limits, reasoning style, privacy posture, cost, and failure behavior across an entire task loop. A static built-in catalog is never enough.

There are three practical reasons users need extension paths:

1. **Local models**: Teams want private, offline, low-cost, or hardware-proximate inference through runners such as Ollama, LM Studio, oMLX, llama.cpp, and vLLM.
2. **Early cloud-model access**: CLI catalogs lag model launches. Users often need a new provider model before the CLI has absorbed it into its picker, aliases, metadata, or defaults.
3. **Aggregator and gateway routing**: Many users standardize on OpenRouter, LiteLLM, internal gateways, or vendor-neutral routers for credentials, audit, throttling, failover, and policy.

Across providers, model extension is less about whether a CLI “supports Ollama” and more about which API standard it can speak, where base URLs are delivered, and whether user config can define durable model identity and metadata.

## Provider Comparison

| Provider    | User Config                                                                                          | Format         | API Standards For User Models                                                             | Base-URL Delivery                                                          | Adapter Mechanism                                                                      | Catalog Semantics                                                              |
|-------------|------------------------------------------------------------------------------------------------------|----------------|-------------------------------------------------------------------------------------------|----------------------------------------------------------------------------|----------------------------------------------------------------------------------------|--------------------------------------------------------------------------------|
| Claude Code | `~/.claude/settings.json`, `.claude/settings.json`, `.claude/settings.local.json`, env               | JSON plus env  | Anthropic Messages                                                                        | `ANTHROPIC_BASE_URL`                                                       | None; endpoint must speak Anthropic-compatible API                                     | Merge                                                                          |
| Codex CLI   | `~/.codex/config.toml`, profiles, trusted `.codex/config.toml`                                       | TOML           | OpenAI Responses API                                                                      | `openai_base_url` or `[model_providers.<id>].base_url`                     | None; endpoint must serve `/v1/responses`                                              | Catalog replacement for `model_catalog_json`; provider entries are constrained |
| Gemini CLI  | `~/.gemini/settings.json`, `.gemini/settings.json`, dotenv                                           | JSON plus env  | Gemini / Vertex API only                                                                  | `GOOGLE_GEMINI_BASE_URL`, `GOOGLE_VERTEX_BASE_URL`                         | None; gateway must speak Gemini-compatible API                                         | Merge                                                                          |
| Goose       | `~/.config/goose/config.yaml`, `custom_providers/*.json`, `secrets.yaml`                             | YAML plus JSON | OpenAI-compatible, Anthropic-compatible, Ollama-native                                    | Custom provider `base_url`, `OPENAI_HOST`, `ANTHROPIC_HOST`, `OLLAMA_HOST` | Custom provider `engine`: `openai`, `anthropic`, `ollama`                              | Merge                                                                          |
| Kilo Code   | `~/.config/kilo/kilo.json(c)`, project `kilo.json(c)`, `.kilo/kilo.json(c)`                          | JSONC          | OpenAI-compatible, Anthropic-compatible, provider-native via AI SDK                       | `provider.<id>.options.baseURL`, `KILO_CONFIG_CONTENT`                     | `provider.<id>.npm` or `api`, commonly `@ai-sdk/openai-compatible`                     | Merge with same-ID shadowing                                                   |
| Kimi Code   | `~/.kimi/config.toml`                                                                                | TOML           | OpenAI Chat, OpenAI Responses, Anthropic, Kimi/Gemini/Vertex bespoke                      | `[providers.<name>].base_url`, selected env overrides                      | Provider `type` field                                                                  | Shadow                                                                         |
| OpenCode    | `~/.config/opencode/opencode.json(c)`, project config, env-injected config                           | JSONC          | OpenAI-compatible, provider-native via AI SDK                                             | `provider.<id>.options.baseURL`, `OPENCODE_CONFIG_CONTENT`                 | `provider.<id>.npm`, commonly `@ai-sdk/openai-compatible` or `@ai-sdk/openai`          | Merge                                                                          |
| Pi          | `~/.pi/agent/models.json`, `~/.pi/agent/settings.json`, `.pi/settings.json`, `~/.pi/agent/auth.json` | JSON           | OpenAI-compatible, Anthropic-compatible, Gemini, Vertex, Mistral, Bedrock, extension APIs | Provider/model `baseUrl` in `models.json`                                  | `api` selects Pi’s internal protocol; `compat` and `streamSimple` extensions fill gaps | Shadow by model `id` within provider                                           |
| Qwen Code   | `~/.qwen/settings.json`, `.qwen/settings.json`, dotenv, system settings                              | JSON           | OpenAI, Anthropic, Gemini, Vertex                                                         | Per-model `baseUrl` under `modelProviders.<type>`                          | Protocol-specific provider blocks                                                      | Merge, but settings scopes can replace `modelProviders`                        |

## API Standards

The near-universal extension standards are OpenAI-compatible and Anthropic-compatible APIs. Gemini is the main exception.

**Claude Code** is the clean Anthropic-compatible case. It emits Anthropic Messages requests and redirects them with `ANTHROPIC_BASE_URL`. There is no provider plug-in layer. If the endpoint implements `/v1/messages`, Claude Code can use it; otherwise a gateway must translate.

**Codex CLI** is the strict OpenAI Responses case. Custom providers are TOML entries under `[model_providers.<id>]`, but the wire protocol is Responses-only. Chat-Completions-only providers need a translating proxy.

**Gemini CLI** speaks Gemini or Vertex through the Google GenAI SDK. It can be pointed at a gateway with `GOOGLE_GEMINI_BASE_URL`, but that gateway must be Gemini-compatible. A direct OpenAI-compatible or Anthropic-compatible local endpoint is not enough.

**Goose, Kilo, Kimi, Pi, and Qwen** are multi-provider by design. They let users declare provider/model blocks and choose the protocol per provider or model. These are the cleanest surfaces for aggregator routing because user config can express “this model belongs to this provider namespace and uses this base URL.”

**OpenCode** is also adapter-shaped, but its officially documented custom-provider path is OpenAI-compatible through AI SDK adapters. Other AI SDK packages may be loaded by `npm`, but the durable documented bridge for user-added local and cloud models is OpenAI-compatible.

## Base URLs And Adapters

Base URL delivery is the most important practical difference.

Claude Code relies on environment variables for arbitrary endpoint redirection. That makes one-off routing simple, but durable model identity mostly comes from model-selection variables and picker metadata rather than rich provider blocks.

Gemini CLI uses settings for model aliases and metadata, but endpoint redirection is env-driven. `GOOGLE_GEMINI_BASE_URL` and `GOOGLE_VERTEX_BASE_URL` are useful gateway hooks, not OpenAI/Anthropic compatibility layers.

Codex uses TOML provider blocks. This is explicit and durable, but constrained by the Responses API requirement and reserved provider IDs.

Goose uses custom provider JSON files. It has a broad provider model: OpenAI-compatible, Anthropic-compatible, and native Ollama engines can all be expressed.

Kilo and OpenCode use the clearest adapter-shaped mechanism. A provider block selects an AI SDK adapter with `provider.<id>.npm` or `api`, supplies the endpoint at `provider.<id>.options.baseURL`, and declares models under `provider.<id>.models`. Kilo uses the same OpenCode-derived shape with Kilo-branded paths, schema URL, and env vars, and it explicitly documents Anthropic-compatible adapters as well as OpenAI-compatible ones.

Pi uses JSON provider blocks in `~/.pi/agent/models.json`. There is no npm-style adapter key; the `api` field selects Pi’s internal protocol implementation, while `compat` handles provider quirks. Unsupported protocols can be bridged through an extension `streamSimple` function.

Kimi uses TOML provider and model tables. A model key points at a provider key, and the provider declares its type and base URL.

Qwen uses per-protocol `modelProviders` blocks and per-model `baseUrl`. It bridges cleanly across OpenAI, Anthropic, Gemini, and Vertex, though settings-scope replacement behavior can surprise users when project settings replace user-level provider definitions.

## Merge, Shadow, And Replace Semantics

Most providers merge user-added models with their built-in catalog. That is the friendliest behavior for early access: the user can add one missing model without losing the built-ins.

Claude Code, Gemini CLI, Goose, Kilo, OpenCode, and Qwen are broadly merge-oriented. Kilo and OpenCode also allow same-ID entries to shadow built-in metadata while preserving non-conflicting catalog data. Kilo’s catalog is fed by built-ins plus hourly Models.dev refreshes, so stale manual overrides are a real maintenance risk.

Kimi and Pi are shadow-oriented. In Pi, custom models are upserted by `id` within a provider: built-ins remain, new IDs are added, and a custom entry with the same ID replaces the built-in entry. This gives strong local control but creates stale-config risk when a manually added model later becomes built-in.

Codex is the sharpest case. Custom providers can be added, but `model_catalog_json` is replacement-shaped: if users take over the catalog, they own the whole catalog. Reserved provider IDs such as `openai`, `ollama`, and `lmstudio` also limit shadowing.

The operational rule is the same everywhere: user model blocks are static, while CLI catalogs self-update. Once the CLI natively supports a manually added model, the user should remove the manual block unless they intentionally want to keep overriding metadata or routing.

## Environment Overrides

Environment variables fall into three buckets.

**Model selection overrides** choose the active model for a session. Examples include `ANTHROPIC_MODEL`, `GEMINI_MODEL`, `GOOSE_MODEL`, `OPENAI_MODEL`, Qwen’s provider-specific model vars, Kimi model vars, and Kilo’s `KILO_PROVIDER` plus config `model` selection.

**Endpoint overrides** redirect traffic. Examples include `ANTHROPIC_BASE_URL`, `GOOGLE_GEMINI_BASE_URL`, `GOOGLE_VERTEX_BASE_URL`, `OPENAI_BASE_URL`, `OPENAI_HOST`, `ANTHROPIC_HOST`, `OLLAMA_HOST`, `CODEX_OSS_BASE_URL`, and provider-block `base_url` / `baseURL` equivalents.

**Credential and config overrides** supply keys or inject settings. Examples include `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GEMINI_API_KEY`, `GOOSE_PROVIDER__API_KEY`, provider-specific API-key env vars, and local-runner placeholder keys such as `VLLM_API_KEY`. Kilo adds `KILO_CONFIG`, `KILO_CONFIG_CONTENT`, `KILO_CONFIG_DIR`, `KILO_API_KEY`, `KILOCODE_<FIELD_NAME>`, and `KILO_EXPERIMENTAL_OUTPUT_TOKEN_MAX`. Pi adds `PI_CODING_AGENT_DIR`, `PI_CODING_AGENT_SESSION_DIR`, `PI_OFFLINE`, and `PI_CACHE_RETENTION`.

Env vars are best for session-scoped routing and local experimentation. Config files are better for durable named models, metadata, and team-repeatable setup.

## Local Runner Fit

The cleanest local-runner integrations are the providers whose client speaks a standard the runner already serves.

Claude Code bridges surprisingly well because current local runners commonly expose Anthropic-compatible `/v1/messages` endpoints. Ollama and oMLX provide first-class launch hooks, while LM Studio, llama.cpp, and vLLM work through `ANTHROPIC_BASE_URL`.

Codex bridges well when the runner serves OpenAI Responses. Ollama and LM Studio are first-class OSS providers; oMLX, llama.cpp, and vLLM are custom provider/base-url paths. The main trap is assuming any OpenAI-compatible Chat Completions endpoint is enough. For Codex, it must be Responses-compatible or proxied.

Goose is one of the strongest local-runner providers. Ollama and LM Studio are first-class, and custom providers can target OpenAI-compatible or Anthropic-compatible endpoints. It also has an Ollama-native path.

OpenCode is strong. Ollama and oMLX have first-class launch integrations, and the rest are straightforward OpenAI-compatible `baseURL` providers through AI SDK adapters.

Kilo is similarly strong for configuration, but more manual operationally. It does not ship runner-native launch hooks; local integration is done through provider blocks. Ollama, LM Studio, llama.cpp, vLLM, oMLX, and similar servers fit cleanly by selecting `@ai-sdk/openai-compatible` and pointing `options.baseURL` at the local `/v1` endpoint. Anthropic-compatible local endpoints can use the Anthropic adapter instead.

Pi is strong for local runners. oMLX has a first-class `omlx launch pi` path, while Ollama, LM Studio, llama.cpp, and vLLM fit through `models.json` provider blocks using OpenAI-compatible or Anthropic-compatible endpoints. Dummy API keys are acceptable for runners that ignore auth.

Kimi and Qwen are flexible but more manual. Both can express local OpenAI-compatible endpoints cleanly in config, and both can also target other standards through their provider models, but they do not have the same first-class runner-launch story as Claude, Codex, Goose, OpenCode, or Pi.

Gemini CLI is the weakest local-runner bridge. Local runners generally serve OpenAI-compatible and sometimes Anthropic-compatible APIs, while Gemini CLI speaks Gemini. Normal local-runner use therefore needs a Gemini-compatible translating proxy.

## Point Of View

The best provider model is not the one with the largest built-in catalog. It is the one that makes API-standard bridging explicit, durable, and inspectable.

Kilo, OpenCode, Goose, Pi, Kimi, and Qwen have the richest user-side configuration surfaces because they let users declare providers and models directly. Kilo and OpenCode’s AI SDK adapter field is especially expressive; Goose’s custom provider files are pragmatic and easy to reason about; Pi’s `api` plus `compat` model is less pluggable but very practical for local runners and OpenAI-compatible gateways.

Claude Code is narrower but clean: Anthropic-compatible base-url routing is enough for many local and gateway setups, and the lack of an adapter layer keeps the mental model simple.

Codex is clean but strict. The Responses-only requirement is technically coherent, but it makes proxies more important when targeting providers or local servers that only implement Chat Completions.

Gemini CLI is the outlier. Its gateway mode is useful for enterprise routing, but because it is Gemini-protocol-only, it does not bridge cleanly to the local-runner ecosystem without a translator.

Across the fleet, the durable abstraction for Claudine should be: provider config maps model identity to API standard, base URL, auth source, adapter, metadata, and catalog merge behavior. Local-runner support should be modeled as standard compatibility plus base-url delivery, not as a hardcoded yes/no list per runner.

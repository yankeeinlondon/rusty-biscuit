---
sequence:
- name: draft
- name: iterate
- name: finalize
prompt: |-
  Every agentic CLI ships an out-of-box model set, and each chooses its own exact accepted model strings, selection mechanisms (flags, config, interactive picker), precedence rules, and dynamic-listing behavior. The strings a CLI accepts are not the same as canonical model identities, so the mapping between them is a first-class concern for Claudine's model catalog.

  ## Task

  Your task is to report on out-of-box model offerings and selection across the Agentic CLI providers Claudine supports.

  - your report should start by outlining why out-of-box model knowledge matters to a wrapper like Claudine (model selection, catalog mapping, fallback and substitution detection)
  - and then shift its focus to how providers differ: default offerings and the exact strings they accept, selection mechanisms and their precedence, rolling aliases versus pinned ids, and dynamic listing behavior
  - close with a point of view on how these offerings map onto Claudine's model catalog and where the mapping is lossy or many-to-many

  As background material we have agent-models research documents for each provider that Claudine supports. They can be found at `@claudine/docs/research/agent-models/*.md`.

  Important: your final response is saved verbatim as the body of this summary document, so it must be the complete document text and nothing else — no preamble, no commentary. Never write to this document yourself.

  ::block when="state.name == 'draft'"
  - Iterate over the first three research documents to develop a point of view on how to write this document and then produce an initial draft of the document
  ::end-block
  ::block when="state.name == 'iterate'"

  - Note: the initial draft has already been created — it is the body of `@claudine/docs/research/summary/agent-models.md` (everything below the frontmatter); read it from there
  - Act as an orchestrator and iterate over each remaining provider's research document:
      - provide the subagent the current draft and ask them to return an improved draft based on the research document they've been assigned
  - Once every remaining provider has been incorporated, your final response is the fully updated draft
  ::end-block

  ::block when="state.name == 'finalize'"

  The document has now gone through several rounds of improvement and your task is just to make sure the document is consistent in tone and detail and that nothing looks incorrect or incomplete. The current draft is the body of `@claudine/docs/research/summary/agent-models.md` (everything below the frontmatter); read it from there, make any adjustments, and your final response will be considered the finalized summary document.
  ::end-block
hash: 74de132d13e7af97-d6e030415cc44cd9
last_updated: 2026-07-03
---
# Agentic CLI Model Offerings and Selection

Out-of-box model knowledge matters to Claudine because a wrapper cannot treat `model` as a provider-neutral string. Each agentic CLI accepts its own selectors: launch flags, config keys, environment variables, aliases, router names, provider/model pairs, provider-prefixed IDs, interactive picker values, and runtime envelopes. Those selectors are user-facing inputs, not always canonical model identities.

For Claudine, this affects three core jobs:

1. **Model selection** — Claudine must pass the right selector through the right surface for the target CLI.
2. **Catalog mapping** — Claudine's model catalog must preserve both the provider-native selector and the underlying canonical model where that can be known.
3. **Fallback and substitution detection** — CLIs can silently resolve aliases, route by account tier, use fallback chains, restore session models, filter by configured auth, or switch internal auxiliary models. Claudine needs to distinguish requested model, resolved model, and auxiliary models, and report substitutions where they can be observed.

The provider research shows several distinct shapes. Claude Code is alias-heavy and account-dependent, with no programmatic model listing. Codex is ID-heavy, has a JSON catalog listing command, and lacks a model-selection environment variable. Gemini CLI is router-heavy: `auto`, `pro`, `flash`, and `flash-lite` are entries into a formal model configuration system with context-based resolution and fallback chains. Goose, Kimi, OpenCode, and Qwen are multi-provider in different ways: some require a provider/model pair, some use provider-prefixed IDs, some populate catalogs dynamically, and some route through protocol-specific config. Pi and Kilo are researched ahead of current Claudine code support, but their documents are useful because they stress the same catalog design problem.

## Provider Differences

### Default Offerings and Accepted Strings

The CLIs do not agree on what "the model string" is.

Claude Code accepts family aliases such as `default`, `best`, `fable`, `opus`, `sonnet`, `haiku`, `sonnet[1m]`, `opus[1m]`, and `opusplan`, plus full Anthropic model IDs such as `claude-opus-4-8`, `claude-sonnet-5`, and `claude-haiku-4-5`. The same alias can resolve differently by account type and backend. `default` is especially contextual: it can mean Opus, Sonnet, or a provider-specific default depending on API, subscription, Bedrock, Vertex, Foundry, or AWS context.

Codex accepts direct OpenAI model IDs such as `gpt-5.5`, `gpt-5.4`, `gpt-5.4-mini`, and `gpt-5.3-codex-spark`. It does not expose a Claude-style family alias layer. Its model identity is closer to the canonical API model string, but routing can still be affected by `model_provider`, `--oss`, custom providers, profiles, and `model_catalog_json`.

Gemini CLI accepts aliases and concrete IDs. Its default is `auto`, a router rather than a concrete model. It also accepts `pro`, `flash`, and `flash-lite`, which resolve through `modelConfigs.modelIdResolutions`, `classifierIdResolutions`, feature flags, account access, and fallback chains. Concrete IDs include `gemini-3-pro-preview`, `gemini-3-flash-preview`, `gemini-3.1-pro-preview`, `gemini-3.1-flash-lite`, `gemini-3.5-flash`, `gemini-2.5-pro`, `gemini-2.5-flash`, and Gemma entries.

Goose requires a provider/model pair. `GOOSE_PROVIDER=anthropic` plus `GOOSE_MODEL=claude-sonnet-4-5` means something different from the same model-looking string under another provider. Goose has no usable universal default: `GOOSE_PROVIDER` and `GOOSE_MODEL` default to `None`, and first run prompts for provider, credentials, and model. It does have per-provider picker defaults and recommendations, such as Anthropic `claude-sonnet-4-5`, first-run Tetrate/OpenAI `gpt-5`, and local provider IDs such as Ollama `qwen2.5`.

Kimi Code does not ship with a hard-coded local model catalog. A fresh config has empty `providers` and `models` tables and `default_model = ""`. Models arrive through `/login` or `kimi login`, which choose a platform and fetch the live model list from that platform's `GET /v1/models` endpoint, or through manual entries in `~/.kimi/config.toml`. Kimi's managed platform model is `kimi-for-coding`; after Kimi Code OAuth login the config key is typically `kimi-code/kimi-for-coding`, backed by a managed provider such as `managed:kimi-code`. Moonshot API-key setups expose IDs such as `kimi-k2.7-code`, `kimi-k2.7-code-highspeed`, `kimi-k2.6`, `kimi-k2.5`, and `moonshot-v1-*`. The accepted `--model` value is a config model key, not necessarily the raw API model ID.

OpenCode uses provider/model IDs such as `opencode/gpt-5.5`, `opencode/claude-sonnet-5`, `opencode/gemini-3.1-pro`, `opencode/kimi-k2.7-code`, and many others. The primary curated out-of-box provider is OpenCode Zen, whose tested models live under the `opencode/` namespace. The prefix is part of the accepted selector and is not the canonical upstream provider. A model also is not necessarily usable just because it appears in OpenCode's provider definitions; most providers still require credentials.

Qwen Code is multi-protocol rather than single-provider. On first launch it does not have a usable model until `/auth` configures a provider; the old Qwen OAuth free tier has been discontinued. Once configured, the OpenAI-compatible resolver has a built-in fallback default of `qwen3.5-plus`, and the Alibaba Cloud Coding Plan also starts with `qwen3.5-plus` while adding `qwen3.6-plus`, `qwen3.7-plus`, `qwen3-coder-plus`, `qwen3-coder-next`, `qwen3-max-2026-01-23`, `glm-5`, `glm-4.7`, `kimi-k2.5`, and `MiniMax-M2.5`. The same CLI can also select Anthropic, Gemini, Vertex, third-party OpenAI-compatible providers, or local OpenAI-compatible servers through `modelProviders`.

Kilo Code, researched ahead of current Claudine support, uses `provider_id/model_id` selectors. Its default is the virtual router `kilo-auto/free`, not a canonical upstream model. Other Auto Model tiers include `kilo-auto/frontier`, `kilo-auto/balanced`, `kilo-auto/efficient`, and `kilo-auto/small`. Representative hosted IDs include `anthropic/claude-opus-4.7`, `anthropic/claude-sonnet-4.6`, `openai/gpt-5.4`, `google/gemini-3.1-pro-preview`, `x-ai/grok-4`, `deepseek/deepseek-v3.2`, `moonshotai/kimi-k2.5`, and `minimax/minimax-m2.7`.

Pi, also researched ahead of current Claudine support, is an auth-gated multi-provider harness. It bundles generated model catalogs for subscription providers, API-key providers, cloud IAM providers, and local/custom provider definitions. Its practical default is selected from configured credentials: with Anthropic auth, the practical default is `claude-opus-4-8`; other provider defaults include `gpt-5.5`, `gpt-5.4`, `gemini-3.1-pro-preview`, `moonshotai/kimi-k2.6`, `kimi-for-coding`, `MiniMax-M2.7`, and provider-specific Bedrock or Cloudflare model paths. Pi accepts bare model IDs, provider-qualified IDs, fuzzy or glob patterns, and optional thinking suffixes such as `sonnet:high`.

### Selection Mechanisms and Precedence

Most providers separate launch-time selection from runtime switching, and those two phases should not be flattened.

Claude Code's effective order is runtime `/model`, then `--model`, then `ANTHROPIC_MODEL`, then settings. Settings are layered managed > project > user. Organization restrictions and `availableModels` apply on top. Resumed sessions can restore the transcript model unless launch-time overrides or alias-pinning variables intervene.

Codex's order is runtime `/model`, then launch flags such as `--model`, `-c model=...`, `--oss`, and `--profile`, then trusted project config, profile config, user config, system config, and built-in defaults. Codex notably has no dedicated model-selection environment variable; provider environment variables carry credentials, not model choices.

Gemini CLI's launch-time order is `--model` / `-m`, then `GEMINI_MODEL`, then `model.name` in settings, then the experimental local Gemma router, then default `auto`. Runtime `/model` sits above those for the current session and can persist to settings. After a selector is chosen, Gemini still resolves through model config rules and fallback chains.

Goose uses flat launch-time selection: `goose run --provider` / `--model` overrides `GOOSE_PROVIDER` / `GOOSE_MODEL`, which override `config.yaml`, which otherwise falls through to no usable default. There is no in-session `/model` slash command and no runtime model switch; `/mode` controls tool-permission mode, not model selection. A running session is fixed to the provider/model selected at launch. Goose also has auxiliary model surfaces: `GOOSE_FAST_MODEL`, `GOOSE_PLANNER_PROVIDER`, `GOOSE_PLANNER_MODEL`, `GOOSE_PLANNER_CONTEXT_LIMIT`, `GOOSE_EDITOR_MODEL`, and `GOOSE_TOOLSHIM`.

Kimi Code uses runtime `/model` as the highest-precedence selector. At launch time, `KIMI_MODEL_NAME` wins over `--model`, and `--model` wins over config `default_model`. That means Kimi's order is `/model` > `KIMI_MODEL_NAME` > `--model` > `default_model`, which is unusual because the environment variable can override the explicit launch flag. `/model` refreshes available models from the provider API, switches the active model, writes the selection back to `config.toml`, and reloads. ACP session creation exposes `models.current_model_id` and `models.available_models`.

OpenCode uses runtime `/models`, then `--model` / `-m`, then inline config through `OPENCODE_CONFIG_CONTENT`, then config files, last-used model, and internal default priority. `OPENCODE_CONFIG` points at a custom config file layer. OpenCode's formal JSON/JSONC config supports top-level `model`, top-level `small_model`, and per-agent or per-command overrides through `agent.<id>.model` and `command.<id>.model`.

Qwen Code uses runtime `/model` and setup-time `/auth`, then `--model` / `-m` and `--auth-type`, then provider-specific environment variables such as `OPENAI_MODEL` / `QWEN_MODEL`, `ANTHROPIC_MODEL`, `GEMINI_MODEL`, and `GOOGLE_MODEL`, then `settings.json` fields such as `model.name`, `modelProviders`, and `security.auth.selectedType`. Provider-specific launch flags, such as OpenAI API key or base URL flags, participate in the same launch-time layer. When a selected model matches a `modelProviders` entry, that entry's `generationConfig` is applied atomically instead of being merged with lower-priority generation settings.

Kilo Code uses runtime `/models` and `/variant`, then `--model` / `-m` and `--variant`, then environment-backed config selection through `KILO_PROVIDER`, `KILO_CONFIG`, or `KILO_CONFIG_CONTENT`, then merged project/global config, last-used session model, and finally `kilo-auto/free`. It also supports top-level `model` config and per-agent overrides such as `agent.plan.model`.

Pi's launch-time resolution starts with `--model`, optionally paired with `--provider`, plus `--models` for scoped model patterns and `--thinking` for supported reasoning levels. If no explicit launch model is provided, Pi uses the scoped model list from `--models` or `enabledModels`, then saved `defaultProvider` / `defaultModel`, then auth-based fallback through its provider default table. Project `.pi/settings.json` overrides global `~/.pi/agent/settings.json`. Inside an open session, `/model` can switch the current model, `/scoped-models` can change the cycle list, and RPC mode exposes `set_model`. Provider API-key environment variables unlock providers for fallback and listing; they do not directly select a model.

### Rolling Aliases Versus Pinned IDs

Rolling aliases are the most lossy part of catalog mapping.

Claude's `opus`, `sonnet`, `haiku`, `fable`, `best`, and `default` are user-facing selectors whose target can move over time or by backend. `default` is not a model identity; it means "clear the override and use the account/provider recommendation."

Gemini's `auto`, `pro`, `flash`, and `flash-lite` are also not stable canonical models. They are tier/router selectors. `auto` can route by preview access, feature flags, prompt complexity, Plan Mode, and fallback availability.

Kilo's `kilo-auto/*` entries are virtual model tiers rather than pinned upstream models. `kilo-auto/free` routes to available free models, `kilo-auto/frontier` targets highest-capability paid routing, `kilo-auto/balanced` chooses cost-effective routing, `kilo-auto/efficient` uses request-difficulty classification, and `kilo-auto/small` handles lightweight background tasks.

Codex is comparatively pinned: users normally choose concrete IDs such as `gpt-5.5`. Even there, "current recommended default" is a moving concept, and the dynamic catalog may be refreshed remotely.

Goose, OpenCode, Kilo, and Pi all embed routing context in selectors. `opencode/gpt-5.5` maps to an upstream GPT model through OpenCode's namespace. `GOOSE_MODEL=claude-sonnet-4-5` is only meaningful with the selected `GOOSE_PROVIDER`. `anthropic/claude-sonnet-4.6` in Kilo is a gateway selector. Pi `--model sonnet`, `--models "claude-*,gpt-5.5"`, or `sonnet:high` is a resolver pattern or convenience selector, not a canonical upstream ID.

Kimi and Qwen add provider-managed catalogs and platform prefixes. Kimi may select `kimi-code/kimi-for-coding` while the underlying API model is `kimi-for-coding`, or a manually authored config key whose `model` field points to another provider-specific ID. Qwen selectors can be built-in fallback IDs, Coding Plan managed IDs, third-party OpenAI-compatible model IDs, Anthropic or Gemini protocol models, or local-server IDs. Qwen also exposes runtime variants through `/model --fast`, `/model --vision`, and `/model --voice`, which should not be flattened into the same meaning as the main model selector.

### Dynamic Listing Behavior

Dynamic listing is inconsistent and should be modeled explicitly.

Codex has the cleanest programmatic path: `codex debug models` emits the model catalog as JSON, and `--bundled` can restrict it to the compiled-in catalog. This makes Codex a strong candidate for a dynamic `ModelCatalogSource`.

OpenCode exposes dynamic listing through `opencode models`, `opencode models <provider>`, `opencode models --refresh`, and `opencode models --verbose`. The command can enumerate configured provider catalogs, filter to one provider, refresh cached data from models.dev, and include metadata such as context limits, costs, and capabilities. Its interactive equivalent is `/models`.

Kilo Code exposes dynamic listing through `kilo models [provider] [--verbose] [--refresh]` and the gateway REST endpoint `GET https://api.kilo.ai/api/gateway/models`.

Kimi Code has no `kimi models` subcommand or `--list-models` flag, but its catalog is still dynamic. `/login` and `/model` query the configured provider's `GET /v1/models` endpoint, and ACP `initialize` / `load_session` responses expose `models.available_models`. For Kimi, dynamic discovery is how the normal catalog is populated.

Pi supports programmatic catalog enumeration. `pi --list-models [search]` prints an auth-filtered table with provider, model ID, context window, max output, thinking support, and image support. RPC mode also exposes `get_available_models`, returning full `Model` objects. The visible list depends on configured credentials, custom `models.json`, and extensions.

Claude Code does not provide a non-interactive listing command. The `/model` picker is interactive-only. Claudine must rely on static knowledge, settings, gateway cache where applicable, and stream-json `result.modelUsage` to observe the resolved model.

Gemini CLI also lacks a model listing command. Its catalog is machine-readable through formal `settings.schema.json` and `modelConfigs.modelDefinitions`, but the CLI does not dump a runtime catalog. Resolved model observation comes from stream-json events, `/chat debug`, or model hooks.

Goose has no programmatic catalog listing command. There is no `goose models`, `goose list-models`, `--list-models`, or model-catalog API. `goose configure` fetches provider model lists interactively at configure time, and `goose info -v` reports the current resolved provider/model configuration, not the available menu. Goose does ship per-provider declarative JSON catalogs and custom-provider JSON, but those are config/research inputs rather than a stable non-interactive CLI listing surface.

Qwen Code has no `qwen models`, `qwen list-models`, `--list-models`, model catalog API, or resolved-catalog dump. The `/model` picker is the native catalog view and is interactive. Non-interactive consumers need to parse configured `modelProviders` where available and observe the resolved model from JSON or stream-json result events.

## Implications for Claudine's Model Catalog

Claudine's catalog should treat provider-accepted selectors and canonical model identities as separate fields.

A useful model entry needs at least:

- `provider`: the CLI being wrapped or researched, such as `claude`, `codex`, `gemini`, `goose`, `kimi`, `opencode`, `qwen`, `kilo`, or `pi`.
- `accepted_selector`: the exact string or structured selector Claudine can pass to that CLI.
- `canonical_model_id`: the best-known upstream model identity, when one exists.
- `selector_kind`: pinned ID, rolling alias, router, provider/model pair, provider-qualified ID, managed platform key, config model key, Auto Model tier, fuzzy pattern, scoped model list, variant selector, or internal auxiliary model.
- `resolution_scope`: account, backend, provider config, credentials, protocol/auth type, feature flag, session resume, fallback chain, dynamic catalog, custom model file, extension, runtime picker state, or launch-time provider/model pair.
- `listing_source`: static research, formal schema, provider config file, CLI dynamic listing, provider `/models` endpoint, gateway endpoint, RPC method, ACP response, provider declarative catalog, custom-provider JSON, interactive-only picker, or observed runtime output.
- `confidence`: whether the mapping is exact, inferred, context-dependent, auth-dependent, pattern-dependent, or only observable after launch.

The mapping is exact only in easy cases: Codex `gpt-5.5` to OpenAI `gpt-5.5`, Gemini `gemini-2.5-pro` to the concrete Gemini model, or Pi `--provider anthropic --model claude-opus-4-8` to the Anthropic model, assuming no gateway rewrites. It becomes lossy when the selector is an alias (`sonnet`), router (`auto`), account default (`default`), provider-prefixed catalog ID (`opencode/gpt-5.5`), provider/model pair (`GOOSE_PROVIDER` + `GOOSE_MODEL`), managed key (`kimi-code/kimi-for-coding`), Auto Model tier (`kilo-auto/free`), Qwen protocol-local `modelProviders` entry, auth-gated fallback, or fuzzy/scoped Pi pattern.

It is also many-to-one and one-to-many:

- Many selectors can point to one canonical model. Claude `sonnet`, a full Claude model ID, a provider override, and a managed setting may all resolve to the same Sonnet generation.
- One selector can point to many canonical models. Claude `default`, Gemini `auto`, Gemini `flash`, Kilo `kilo-auto/frontier`, Kilo `kilo-auto/efficient`, Qwen provider/protocol-selected names, and Pi auth-based fallback can resolve differently by account, flags, protocol, credentials, provider catalog, mode, or fallback state.
- One CLI model can represent another provider's model behind a wrapper namespace, as with OpenCode's `opencode/gpt-5.5`, Goose's provider modes, Qwen's Anthropic/Gemini/OpenAI-compatible auth types, Kilo's gateway IDs, or Pi's multi-provider catalog.
- One run can use multiple models even when the user selected one main model, because providers expose fast models, planner models, editor models, toolshim interpreter models, subagent models, per-command models, advisor models, review models, memory models, classifiers, summarizers, background models, thinking levels, vision or voice variants, and fallback chains.

For Claudine, the practical stance should be:

1. Preserve the exact user/provider selector.
2. Preserve the provider, protocol, auth, config-key, and variant context needed to make that selector meaningful.
3. Resolve to a canonical model only when the mapping is stable enough to defend.
4. Record alias, router, fallback, pattern, and virtual-tier metadata instead of flattening it away.
5. Prefer dynamic catalog sources where the CLI exposes them, especially Codex, OpenCode, Kimi, Kilo, and Pi.
6. For Claude, Gemini, Goose, and Qwen, combine static research/config parsing with runtime observation of the resolved model.
7. Report both requested and resolved models when they differ or when resolution is context-dependent.

The catalog should therefore not be a flat table of "provider -> models." It should be a resolution graph: selectors, config surfaces, provider/protocol context, dynamic catalog sources, canonical model candidates, auxiliary model roles, and runtime observations. That graph is the only way Claudine can make model selection predictable while still respecting the very different model semantics each agentic CLI ships out of the box.

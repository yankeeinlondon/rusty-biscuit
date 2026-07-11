---
$schema: ./_schema.yaml
created: 2026-07-01
last_updated: 2026-07-01
agent: open_code
model: default
has_official_schema: formal
schema_url: https://raw.githubusercontent.com/google-gemini/gemini-cli/main/schemas/settings.schema.json

default_models:
  - id: auto
    alias: auto
    context_window: 1000000
    is_default: true
    notes: "Special router alias, NOT a concrete model. The documented default when no --model/GEMINI_MODEL/model.name is set. Resolves via modelConfigs.modelIdResolutions.auto: default gemini-3-pro-preview when preview access is available, else gemini-2.5-pro; with useGemini3_1 (+useCustomTools) -> gemini-3.1-pro-preview(-customtools). Offered in /model as 'Auto (Gemini 3)' and 'Auto (Gemini 2.5)'."
  - id: gemini-3-pro-preview
    alias: pro
    context_window: 1000000
    is_default: false
    notes: "Gemini 3 Pro (preview). Pro tier, gemini-3 family. Supports thinking + multimodal tool use. Concrete target of the `pro` and `auto` aliases when preview access is available. Highest-capability mainstream model."
  - id: gemini-3-flash-preview
    alias: flash
    context_window: 1000000
    is_default: false
    notes: "Gemini 3 Flash (preview). Flash tier, gemini-3 family. thinking=false, multimodal tool use=true. Used by Auto (Gemini 3) routing for simple prompts and as the last-resort fallback in the `preview`/`auto-preview` model chains."
  - id: gemini-3.1-pro-preview
    context_window: 1000000
    is_default: false
    notes: "Gemini 3.1 Pro (preview, rolling out). Pro tier, access-gated. Appears in /model Manual and Auto (Gemini 3) when available; also reachable directly via `gemini -m gemini-3.1-pro-preview`. The `gemini-3.1-pro-preview-customtools` variant is registered but hidden (isVisible:false)."
  - id: gemini-3.1-flash-lite
    alias: flash-lite
    context_window: 1000000
    is_default: false
    notes: "Gemini 3.1 Flash-Lite. Flash-lite tier, gemini-3 family, NOT preview. thinking=false, multimodal tool use=true. Target of the `flash-lite` alias per modelIdResolutions."
  - id: gemini-3.5-flash
    context_window: 1000000
    is_default: false
    notes: "Gemini 3.5 Flash. Flash tier, NOT preview. Selected when the useGemini3_5Flash context flag is true; otherwise gemini-3-flash-preview / gemini-2.5-flash are used."
  - id: gemini-2.5-pro
    context_window: 1000000
    is_default: false
    notes: "Gemini 2.5 Pro. Pro tier, gemini-2.5 family. thinking=false, multimodal tool use=false. GA fallback when preview access is unavailable; first entry of the `default` availability chain. Concrete target of `pro`/`auto` for accounts without preview."
  - id: gemini-2.5-flash
    alias: flash
    context_window: 1000000
    is_default: false
    notes: "Gemini 2.5 Flash. Flash tier, gemini-2.5 family. thinking=false, multimodal tool use=false. Legacy concrete target of the `flash` alias (cheatsheet); resolves here when preview access is off. Second entry of the `default` chain."
  - id: gemini-2.5-flash-lite
    alias: flash-lite
    context_window: 1000000
    is_default: false
    notes: "Gemini 2.5 Flash-Lite. Flash-lite tier. Background/utility model used by internal aliases (classifier, summarizer-default/shell, prompt-completion, fast-ack-helper, edit-corrector). Subject to a silent fallback chain (flash-lite -> flash -> pro) for internal calls."
  - id: gemma-4-31b-it
    is_default: false
    notes: "Gemma 4 31B IT (open model) served via the Gemini API. Gated by experimental.gemma (default true). Custom tier, gemma-4 family, displayName set. thinking=true, multimodal tool use=false. Context window not documented."
  - id: gemma-4-26b-a4b-it
    is_default: false
    notes: "Gemma 4 26B A4B IT (open model) via the Gemini API. Gated by experimental.gemma. Custom tier, gemma-4 family. thinking=true, multimodal tool use=false. Context window not documented."

model_selection:
  - method: interactive_command
    site: "/model"
    example: "/model set gemini-3-pro-preview --persist"
    notes: "Primary runtime selection. `/model` (no arg) opens the picker dialog (Auto Gemini 3 / Auto Gemini 2.5 / Manual); `/model manage` opens the configure dialog; `/model set <name> [--persist]` switches immediately and, with --persist, writes model.name for future sessions. Does NOT override sub-agent models."
  - method: interactive_command
    site: "/settings"
    example: "/settings"
    notes: "Settings editor: edits model.name and general.plan.modelRouting (Auto Pro/Flash switching by Plan Mode phase) and experimental.* flags with validation. Equivalent to editing settings.json."
  - method: cli_flag
    site: "--model / -m"
    example: "gemini -m gemini-3.1-pro-preview"
    notes: "Highest-precedence model selector at launch. Accepts an alias (auto/pro/flash/flash-lite) or a concrete model name. Default value `auto`. Applies to the launched session only."
  - method: env_var
    site: "GEMINI_MODEL"
    example: "GEMINI_MODEL=gemini-3-flash-preview gemini"
    notes: "Session model override; explicitly 'Overrides the hardcoded default.' Lower precedence than --model/-m, higher than model.name. Not a tier flag; takes a model name/alias."
  - method: config_file
    site: "model.name (settings.json)"
    example: '"model": { "name": "gemini-2.5-pro" }'
    notes: "Persistent initial model in settings.json. Accepts a concrete ID or an alias (resolved by modelIdResolutions). Layered system-defaults < user (~/.gemini/settings.json) < project (.gemini/settings.json) < system-settings."
  - method: config_file
    site: "modelConfigs.* (settings.json)"
    example: '"modelConfigs": { "customAliases": { "my-pro": { "extends": "chat-base-3", "modelConfig": { "model": "gemini-3-pro-preview" } } } }'
    notes: "The resolution machinery: aliases (named presets with `extends` inheritance), customAliases/customOverrides (merged over built-ins), overrides (per-model/alias match), modelDefinitions (tier/family/features registry), modelIdResolutions + classifierIdResolutions (context-based name->ID rules), modelChains (availability/fallback). Gated for runtime edits by experimental.dynamicModelConfiguration."
  - method: env_var
    site: "GEMINI_API_KEY / GOOGLE_API_KEY / GOOGLE_CLOUD_PROJECT / GOOGLE_GENAI_USE_VERTEXAI"
    example: "GOOGLE_GENAI_USE_VERTEXAI=true GOOGLE_CLOUD_PROJECT=pid gemini"
    notes: "Auth/backend selectors, not model names, but they determine which provider-form model IDs and endpoint the resolved model is sent to: Gemini API (GEMINI_API_KEY), Vertex AI (GOOGLE_API_KEY/ADC + GOOGLE_CLOUD_PROJECT/GOOGLE_CLOUD_LOCATION, or GOOGLE_GENAI_USE_VERTEXAI=true per the README/auth docs). Config-reference describes the backend via security.auth.selectedType/enforcedType."
  - method: env_var
    site: "GOOGLE_GEMINI_BASE_URL / GOOGLE_VERTEX_BASE_URL / GOOGLE_GENAI_API_VERSION"
    example: "GOOGLE_GEMINI_BASE_URL=https://my-gateway gemini"
    notes: "Redirect Gemini-API / Vertex requests to an alternate endpoint (gateway/proxy) and/or API version. Changes WHERE the resolved model is served, not which model is selected. Endpoint must be Gemini- or Vertex-API-compatible."
  - method: wire_envelope
    site: "request `model` field (Gemini generateContent / Vertex)"
    example: '"model": "gemini-3-pro-preview"'
    notes: "The concrete model ID emitted on each inference request after alias/context resolution. Observable via the BeforeModel/AfterModel hook LLMRequest.model (stable SDK-agnostic shape), `/chat debug` (dumps the latest API request JSON), and `--output-format stream-json` events."

precedence: "interactive_command (/model, runtime) > cli_flag (--model / -m) > env_var (GEMINI_MODEL) > config_file (model.name) > local_router (experimental Gemma model router) > default (auto). settings.json layering (per general config precedence): system-defaults < user (~/.gemini) < project (.gemini) < system-settings; then env vars; then CLI args. Resolved ID is then chosen by modelConfigs.modelIdResolutions / classifierIdResolutions based on context (hasAccessToPreview, useGemini3_1, useGemini3_5Flash, useCustomTools, requestedModels) and availability/fallback is governed by modelConfigs.modelChains. /model does not override sub-agent models."

dynamic_listing:
  available: false
  method: "none — no `gemini models` / `--list-models` subcommand or model-catalog API. `/model` opens an interactive picker/dialog (requires a TTY) and is the sole runtime catalog view. The catalog itself is statically declared in `modelConfigs.modelDefinitions` and formally described by the JSON Schema at schemas/settings.schema.json — both machine-readable, but neither is dumped by a CLI command. `/stats model` reports per-model usage statistics and `/chat debug` dumps the most recent API request (including the resolved model); neither enumerates the available catalog."
  example: "Interactive only: run `gemini` then `/model` (or `/model set <name> [--persist]`). Non-interactive consumers must read the resolved model from `--output-format stream-json` events or `/chat debug`, or read the static catalog from `modelConfigs.modelDefinitions` in settings.json / the formal schema."

changes: []

requires_claudine_update: true
reason: "Gemini CLI is a first-party Google/Gemini-only provider whose model surface is materially richer than a flat name list and is not yet represented in Claudine's model_catalog for the Gemini provider. (1) A formal JSON Schema at schemas/settings.schema.json covers `model.*` and the extensive `modelConfigs.*` block — aliases with `extends` inheritance, customAliases/customOverrides, a modelDefinitions tier/family/features registry, context-based modelIdResolutions/classifierIdResolutions, and availability modelChains — which Claudine should use to validate/parse Gemini settings and to resolve aliases. (2) The current default lineup and tier aliases need capturing: default router `auto` -> gemini-3-pro-preview (gemini-2.5-pro fallback), plus `pro`/`flash`/`flash-lite` and the gemini-3.1-pro-preview / gemini-3.5-flash / gemma-4-* entries. (3) The documented precedence (`--model`/`-m` > GEMINI_MODEL > model.name > experimental local Gemma router > default `auto`) and the Gemini-specific auth/backend routing env vars (GEMINI_API_KEY, GOOGLE_API_KEY, GOOGLE_CLOUD_PROJECT, GOOGLE_GENAI_USE_VERTEXAI, GOOGLE_GEMINI_BASE_URL, GOOGLE_VERTEX_BASE_URL) must be modeled for accurate wrapper selection/reporting. (4) Programmatic model enumeration is NOT available (interactive `/model` only), so Claudine's wrapper must resolve/read the Gemini model from stream-json events or the static modelDefinitions rather than enumerating a runtime catalog."

---

# Gemini CLI Model Support

## Model's Available

Gemini CLI (Google's open-source agentic CLI, observed against the **v0.49.0** release line) is a **first-party Google model provider**: out of the box it talks only to Google's own Gemini / Gemma families over the Gemini API (and the Vertex AI / Code Assist dialects of the same). It does **not** ship any OpenAI-compatible, Anthropic-compatible, or generic local-model backend.

### Tiers, aliases, and the `auto` router (the primary interface)

Users rarely type a concrete model ID. Gemini CLI exposes a small set of **tier aliases** that resolve to the appropriate model for the account and feature flags, plus an **`auto` router** that is the documented default:

| Alias | Tier | Default concrete model | Notes |
|-------|------|------------------------|-------|
| `auto` | auto | `gemini-3-pro-preview` (preview) / `gemini-2.5-pro` (no preview) | **Default when nothing is specified.** With `useGemini3_1` → `gemini-3.1-pro-preview`. Router picks Pro for complex prompts and Flash for simple ones. |
| `pro` | pro | `gemini-3-pro-preview` (preview) / `gemini-2.5-pro` | Highest-reasoning tier. |
| `flash` | flash | `gemini-3-flash-preview` (preview) / `gemini-3.5-flash` (`useGemini3_5Flash`) / `gemini-2.5-flash` | Fast/balanced tier. |
| `flash-lite` | flash-lite | `gemini-3.1-flash-lite` / `gemini-2.5-flash-lite` | Fastest tier; also the background/utility model. |

The concrete target of each alias is chosen by the **`modelConfigs.modelIdResolutions`** / **`classifierIdResolutions`** rules based on context flags (`hasAccessToPreview`, `useGemini3_1`, `useGemini3_5Flash`, `useCustomTools`, `requestedModels`). To pin a specific version, pass the full model ID (e.g. `gemini-3.1-pro-preview`, `gemini-2.5-flash`) to `--model`, `GEMINI_MODEL`, or `model.name`.

### Models available out of the box

The built-in `modelConfigs.modelDefinitions` registry (visible in the formal schema) declares the catalog. Visible (`isVisible: true`) entries:

| Model ID (exact) | Alias | Tier / Family | Preview | Context | Default on |
|------------------|-------|---------------|---------|---------|------------|
| `gemini-3-pro-preview` | `pro` | pro / gemini-3 | yes | 1M | `auto` / `pro` (preview accounts) |
| `gemini-3-flash-preview` | `flash` | flash / gemini-3 | yes | 1M | Auto (Gemini 3) simple prompts; `preview`/`auto-preview` last-resort fallback |
| `gemini-3.1-pro-preview` | — | pro / gemini-3 | yes | 1M | — (rolling out; access-gated) |
| `gemini-3.1-flash-lite` | `flash-lite` | flash-lite / gemini-3 | no | 1M | `flash-lite` alias (current resolution) |
| `gemini-3.5-flash` | — | flash / gemini-3 | no | 1M | — (selected when `useGemini3_5Flash`) |
| `gemini-2.5-pro` | `pro` | pro / gemini-2.5 | no | 1M | `auto` / `pro` (no preview); GA fallback |
| `gemini-2.5-flash` | `flash` | flash / gemini-2.5 | no | 1M | `flash` alias (legacy/cheatsheet resolution) |
| `gemini-2.5-flash-lite` | `flash-lite` | flash-lite / gemini-2.5 | no | 1M | background/utility (classifier, summarizer, prompt-completion) |
| `gemma-4-31b-it` | — | custom / gemma-4 | no | — | — (open model via Gemini API; `experimental.gemma`) |
| `gemma-4-26b-a4b-it` | — | custom / gemma-4 | no | — | — (open model via Gemini API) |

Hidden registry entries: `gemini-3.1-pro-preview-customtools` (`isVisible:false`, selected via `useCustomTools`), the tier router aliases `auto` / `pro` / `flash` / `flash-lite`, and the family-scoped routers `auto-gemini-3` / `auto-gemini-2.5`.

> 📝 All Gemini-family models document a **1M-token context window**. Gemma context windows are not documented in the configuration reference.

### Adding bespoke models

Gemini CLI has no native OpenAI-compatible / Anthropic-compatible / Ollama backend. Bespoke and local models are registered through four channels:

1. **Catalog entries & named presets** (`other`) — declare model metadata (tier/family/features) in `modelConfigs.modelDefinitions` and named presets in `modelConfigs.customAliases` / `customOverrides`. This adds *catalog* entries; the underlying wire backend must still be a Gemini-/Vertex-compatible endpoint. Editing `modelDefinitions` requires a restart; runtime edits of the resolution machinery require `experimental.dynamicModelConfiguration: true`.
2. **Local Gemma router** (`local`) — `experimental.gemmaModelRouter.*` (or the `gemini gemma setup` command) runs a local Gemma model via a LiteRT-LM shim to make *routing decisions* locally. This routes to a hosted Gemini model using local classification, rather than exposing a local model as the chat model.
3. **Vertex AI** (`provider_plugin`) — the first-party cloud backend. Set `GOOGLE_CLOUD_PROJECT` (+ `GOOGLE_CLOUD_LOCATION`) and authenticate via ADC, a service-account key, or `GOOGLE_API_KEY`; or set `GOOGLE_GENAI_USE_VERTEXAI=true` per the README/auth docs. Same model IDs as the Gemini API path.
4. **Compatible gateway** (`other`) — redirect Gemini-API or Vertex requests at a gateway/proxy via `GOOGLE_GEMINI_BASE_URL`, `GOOGLE_VERTEX_BASE_URL`, or `GOOGLE_GENAI_API_VERSION`. The endpoint must speak the Gemini or Vertex API — Gemini CLI never emits OpenAI Chat Completions or Anthropic Messages.

> ⚠️ A raw local OpenAI-compatible endpoint (Ollama, LM Studio, llama.cpp server) is **not** directly usable. You must front it with a gateway that exposes the Gemini or Vertex API.

## Model Configuration Details

### Schema — formal

Unlike Claude Code, Gemini CLI publishes a **formal, machine-readable schema**: a JSON Schema at [`schemas/settings.schema.json`](https://raw.githubusercontent.com/google-gemini/gemini-cli/main/schemas/settings.schema.json) in the repo (hosted at the raw GitHub URL). It is generated from source and is intended for editor autocomplete/validation of the entire `settings.json` — including the `model.*` block and the extensive `modelConfigs.*` block. This is the authoritative contract for model configuration; the configuration reference page describes the same keys in prose with their default JSON values.

### How a model is selected — mechanisms and precedence

The [Model routing](https://geminicli.com/docs/cli/model-routing/) page documents the model-selection order explicitly:

1. **`--model` / `-m` CLI flag** — a model specified at launch is always used. Accepts an alias (`auto`/`pro`/`flash`/`flash-lite`) or a concrete name. Default value is `auto`.
2. **`GEMINI_MODEL` environment variable** — used if `--model` is not passed; "overrides the hardcoded default."
3. **`model.name` in `settings.json`** — the persistent initial model. Settings files layer **system-defaults < user (`~/.gemini/settings.json`) < project (`.gemini/settings.json`) < system-settings**, then environment variables, then CLI arguments (general config precedence).
4. **Local model (experimental)** — if the Gemma local router (`experimental.gemmaModelRouter`) is enabled, it routes the request locally instead of resolving a hosted Gemini model.
5. **Default** — if none of the above is set, the default is `auto`.

At runtime, **`/model`** (and `/model set <name> [--persist]`, `/model manage`) switches the session model; `/model set --persist` also writes `model.name` for future sessions. Note: `/model` does **not** override sub-agent models, so model-usage reports may show other models.

After a name is chosen, the concrete model ID is resolved by `modelConfigs.modelIdResolutions` / `classifierIdResolutions` (context-aware), and availability/fallback is governed by `modelConfigs.modelChains`.

**Precedence summary:** `interactive_command (/model) > cli_flag (--model/-m) > env_var (GEMINI_MODEL) > config_file (model.name) > local_router (experimental Gemma) > default (auto)`, with settings.json layered system-defaults < user < project < system-settings.

```mermaid
flowchart TD
    A[Launch gemini] --> B{--model / -m set?}
    B -- yes --> C[Use launch-time model]
    B -- no --> D{GEMINI_MODEL env set?}
    D -- yes --> E[Use GEMINI_MODEL]
    D -- no --> F{model.name in settings.json?}
    F -- yes --> G[Use model.name \n layered system-defaults < user < project < system-settings]
    F -- no --> H{experimental Gemma local router enabled?}
    H -- yes --> I[Route via local Gemma]
    H -- no --> J[Default: auto]
    C --> K[Resolve concrete ID via modelIdResolutions / classifierIdResolutions \n context: hasAccessToPreview, useGemini3_1, useGemini3_5Flash, useCustomTools]
    E --> K
    G --> K
    I --> K
    J --> K
    K --> L[Apply availability/fallback via modelChains]
    L --> M[Session runs]
    M -->|/model set name --persist| N[Runtime switch + optional persist]
```

### Programmatic model enumeration — not available

Gemini CLI **cannot** enumerate its model catalog programmatically. There is:

- **No `gemini models` / `gemini list-models` subcommand** and **no `--list-models` flag** (the CLI surface is `gemini`, `gemini update`, `gemini extensions`, `gemini mcp`, `gemini skills`, `gemini gemma`).
- **No model-catalog API** exposed by the CLI.

The `/model` command opens an **interactive** picker/dialog (requires a TTY) and is the sole runtime catalog view. The catalog itself is *statically* declared in `modelConfigs.modelDefinitions` and formally described by the JSON Schema — both machine-readable, but neither is dumped by a CLI command. For the *resolved* model non-interactively, read `--output-format stream-json` events, `/chat debug` (dumps the latest API request JSON), or the `BeforeModel`/`AfterModel` hook `LLMRequest.model` field.

### Related model-behavior configuration

| Concern | Mechanism | Notes |
|---------|-----------|-------|
| **Plan-Mode routing** | `general.plan.modelRouting` (default `true`) | Auto-switches Pro (planning) / Flash (implementation) by Plan Mode phase. |
| **Model steering (hints)** | `experimental.modelSteering` (default `false`) | User hints to guide the model during tool execution. Experimental. |
| **Availability / fallback chains** | `modelConfigs.modelChains` | Per-chain (`preview`, `auto-preview`, `default`) ordered models with retry/`isLastResort` and `actions`/`stateTransitions` for terminal/transient/not_found/unknown failures. |
| **Context compression** | `model.compressionThreshold` (default `0.5`) | Fraction of context at which compression triggers. |
| **Loop detection** | `model.disableLoopDetection` (default `false`) | Toggle infinite-loop detection. |
| **Retry budget** | `general.maxAttempts` (default `10`, max 10) | Max attempts for the main chat model. |
| **Quota / overage** | `billing.overageStrategy` (`ask`/`always`/`never`) | How quota exhaustion is handled when AI credits are available. |
| **Sub-agent model** | agent frontmatter `model` field | Per-subagent override; `/model` does not affect it. |
| **Gemma (open models)** | `experimental.gemma` (default `true`) | Enables Gemma 4 models via the Gemini API. |
| **Dynamic config** | `experimental.dynamicModelConfiguration` (default `false`) | Master switch for runtime edits of definitions/resolutions/chains. |

## Sources

- [Gemini CLI — GitHub repository](https://github.com/google-gemini/gemini-cli) *(README: Gemini 3 / 1M context, install, `-m` example, Vertex AI env vars)*
- [Gemini CLI — `schemas/settings.schema.json` (formal JSON Schema)](https://raw.githubusercontent.com/google-gemini/gemini-cli/main/schemas/settings.schema.json) *(authoritative contract for `model.*` and `modelConfigs.*`)*
- [Gemini CLI — Configuration reference](https://geminicli.com/docs/reference/configuration/) *(settings layers, `model.*`, full `modelConfigs.*` defaults: aliases, modelDefinitions, modelIdResolutions, classifierIdResolutions, modelChains; env vars `GEMINI_MODEL`, `GOOGLE_GEMINI_BASE_URL`, `GOOGLE_VERTEX_BASE_URL`, `GOOGLE_GENAI_API_VERSION`; schema tip)*
- [Gemini CLI — Model selection (`/model`)](https://geminicli.com/docs/cli/model/) *(Auto/Pro/Manual options, `/model` does not override sub-agents)*
- [Gemini CLI — Model routing](https://geminicli.com/docs/cli/model-routing/) *(ModelAvailabilityService, fallback, Local Gemma routing, **selection precedence**)*
- [Gemini CLI — Model steering (experimental)](https://geminicli.com/docs/cli/model-steering/)
- [Gemini CLI — Gemini 3 on Gemini CLI](https://geminicli.com/docs/get-started/gemini-3/) *(Gemini 3 Pro/Flash, Gemini 3.1 Pro Preview rollout, Auto vs Pro routing, capacity fallback, `gemini -m gemini-3.1-pro-preview`)*
- [Gemini CLI — CLI cheatsheet](https://geminicli.com/docs/cli/cli-reference/) *(`--model`/`-m` default `auto`, model aliases `auto`/`pro`/`flash`/`flash-lite`, `--output-format stream-json`)*
- [Gemini CLI — Command reference](https://geminicli.com/docs/reference/commands/) *(`/model manage`, `/model set <name> [--persist]`, `/settings`, `/stats model`, `/chat debug`)*
- [Gemini CLI — Authentication setup](https://geminicli.com/docs/get-started/authentication/) *(OAuth, `GEMINI_API_KEY`, Vertex AI ADC/service-account/`GOOGLE_API_KEY`, `GOOGLE_CLOUD_PROJECT`)*
- [Gemini CLI — Model configuration (generation settings)](https://geminicli.com/docs/cli/generation-settings/)
- [Gemini CLI — Hooks reference (Stable Model API)](https://geminicli.com/docs/hooks/reference) *(BeforeModel/AfterModel `LLMRequest.model` / `LLMResponse` — the stable wire shape for the resolved model)*
- [Gemini CLI — Subagents](https://geminicli.com/docs/core/subagents/) *(per-agent `model` frontmatter override)*
- [Gemini CLI — Releases (v0.49.0 latest stable)](https://github.com/google-gemini/gemini-cli/releases)

## Changelog

- **2026-07-01** — Initial research for `gemini.md` (this file), observed against the Gemini CLI `v0.49.0` release line. Established Gemini CLI as a first-party Google/Gemini-only provider (Gemini API + Vertex AI/Code Assist dialects) with no native OpenAI- or Anthropic-compatible backend. Classified the schema as **formal** (machine-readable JSON Schema at `schemas/settings.schema.json` covering `model.*` and `modelConfigs.*`). Documented the tier-alias system (`auto` default router, `pro`/`flash`/`flash-lite`) and the context-based resolution machinery (`modelIdResolutions` / `classifierIdResolutions` keyed on `hasAccessToPreview`/`useGemini3_1`/`useGemini3_5Flash`/`useCustomTools`); enumerated the out-of-the-box catalog (gemini-3-pro-preview, gemini-3-flash-preview, gemini-3.1-pro-preview, gemini-3.1-flash-lite, gemini-3.5-flash, gemini-2.5-pro/flash/flash-lite, gemma-4-31b-it, gemma-4-26b-a4b-it) plus hidden entries. Captured the full selection surface — `/model` runtime command (`/model set [--persist]`, `/model manage`), `--model`/`-m` flag, `GEMINI_MODEL` env var, `model.name` setting, experimental local Gemma router — with the documented precedence (`--model` > `GEMINI_MODEL` > `model.name` > local router > default `auto`) and settings-file layering. Recorded the rich `modelConfigs.*` machinery (aliases with `extends`, customAliases/customOverrides, modelDefinitions, modelIdResolutions/classifierIdResolutions, availability modelChains) and the `experimental.dynamicModelConfiguration` gate. Documented availability/fallback via `ModelAvailabilityService` + `modelChains`, Plan-Mode Pro/Flash routing, and the Gemini-specific auth/backend routing env vars. Recorded that there is **no** programmatic model catalog (no `gemini models` subcommand; `/model` is interactive-only) and that non-interactive consumers must read the resolved model from stream-json events / `/chat debug` / the BeforeModel hook. Documented the four bespoke-model channels (catalog entries/customAliases, local Gemma router, Vertex AI provider plugin, Gemini/Vertex-compatible gateway via base-URL overrides). Set `requires_claudine_update: true` against Claudine's `model_catalog` module.

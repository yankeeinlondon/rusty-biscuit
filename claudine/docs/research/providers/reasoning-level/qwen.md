---
$schema: ./_schema.yaml
schema_revision: 2
provider: qwen
created: 2026-09-29
last_updated: 2026-09-29
agent: opencode
model: zai-coding-plan/glm-5.3
reasoning_effort: provider_default
versions_examined:
- 0.19.8
evidence:
- claim: The installed Qwen CLI reports version 0.19.8.
  id: qwen-version
  limitations: Version string only; describes the Homebrew install on this host.
  location: claudine/docs/research/reasoning-level/_artifacts/qwen-cli-version.txt
  method: local_inspection
  observed_on: 2026-09-29
  version: 0.19.8
- claim: Neither `qwen --help` nor the full global-options list behind `qwen sessions --help` contains any effort, thinking, or reasoning launch flag or environment variable.
  id: qwen-help
  limitations: Help text can omit undocumented flags; source was also searched for QWEN_*THINK*/REASON*/EFFORT* variable names and none exist.
  location: claudine/docs/research/reasoning-level/_artifacts/qwen-help.txt
  method: local_inspection
  observed_on: 2026-09-29
  version: 0.19.8
- claim: The effort scale is low, medium, high, xhigh, max in that order (ranks 20/30/40/60/70); input normalization also accepts the aliases med, extrahigh, and maximum; a tier a model lacks is clamped to the lowest supported tier of at least the requested rank.
  id: tiers-source
  limitations: Minified bundle chunk; the file is identified by its exported symbols rather than a repo path.
  location: /opt/homebrew/Cellar/qwen-code/0.19.8/libexec/lib/node_modules/@qwen-code/qwen-code/chunks/chunk-SIUQ3YYX.js (REASONING_EFFORT_TIERS, REASONING_EFFORT_RANKS, normalizeReasoningEffort, clampReasoningEffort)
  method: source_code
  observed_on: 2026-09-29
  version: 0.19.8
- claim: '`/effort` is a built-in command with supportedModes interactive, non_interactive, and acp; it sets the runtime effort, persists it under the settings key model.reasoningEffort, and answers an unknown token with the error `Unknown reasoning effort "<value>". Choose one of: low, medium, high, xhigh, max.`'
  id: effort-command-source
  limitations: Source shows the command reaches both launch modes; per-provider effect is applied later in the request pipeline.
  location: /opt/homebrew/Cellar/qwen-code/0.19.8/libexec/lib/node_modules/@qwen-code/qwen-code/chunks/chunk-2EM7ECZD.js (effortCommand, packages/cli/src/ui/commands/effort-command.ts)
  method: source_code
  observed_on: 2026-09-29
  version: 0.19.8
- claim: The interactive picker offers exactly the five tiers with the tier descriptions quoted in the Levels section, and with nothing configured it shows `No effort configured - using the model/provider default.`
  id: effort-dialog-source
  limitations: Descriptions are UI strings; they carry no wire-level guarantee.
  location: /opt/homebrew/Cellar/qwen-code/0.19.8/libexec/lib/node_modules/@qwen-code/qwen-code/chunks/startInteractiveUI-JII36UGN.js (EffortDialog, use-effort-command.ts)
  method: source_code
  observed_on: 2026-09-29
  version: 0.19.8
- claim: 'Effort is read from the live content-generator config on every request; `/effort` mutates that config so the tier takes effect on the next turn, and it is a no-op while `reasoning: false` is set, so effort cannot silently re-enable thinking.'
  id: set-effort-source
  limitations: Runtime behavior of one CLI build; no server confirmation.
  location: /opt/homebrew/Cellar/qwen-code/0.19.8/libexec/lib/node_modules/@qwen-code/qwen-code/chunks/chunk-3GNOQZDC.js (Config.getReasoningEffort / Config.setReasoningEffort)
  method: source_code
  observed_on: 2026-09-29
  version: 0.19.8
- claim: 'At startup the settings key model.reasoningEffort is normalized and written over the resolved model config''s reasoning.effort unless `reasoning` is false; an invalid value produces the warning `Ignoring invalid model.reasoningEffort "<value>"; expected one of: low, medium, high, xhigh, max.` and the run continues on the model/provider default.'
  id: resolve-config-source
  limitations: Resolution order is established for the startup path only; mid-session hand edits were not exercised.
  location: /opt/homebrew/Cellar/qwen-code/0.19.8/libexec/lib/node_modules/@qwen-code/qwen-code/chunks/chunk-DIWVZ3VM.js (resolveCliGenerationConfig)
  method: source_code
  observed_on: 2026-09-29
  version: 0.19.8
- claim: 'For qwen-family wire models (model id starting with qwen, or the alias coder-model) any set effort collapses to the boolean `enable_thinking: true` and the reasoning field is deleted; extra_body is spread last and wins over pipeline-synthesized fields; `reasoning: false` or `request.config.thinkingConfig.includeThoughts === false` runs after extra_body and forces thinking off (enable_thinking false on DashScope, chat_template_kwargs.enable_thinking false elsewhere, plus thinking disabled on DeepSeek hostnames); on api.deepseek.com low and medium are rewritten to high and xhigh to max; GLM models on a Z.ai hostname get the nested effort flattened to a verbatim flat reasoning_effort.'
  id: openai-pipeline-source
  limitations: Wire shapes are read from the client; whether each server honors every verbatim token is not verified here.
  location: /opt/homebrew/Cellar/qwen-code/0.19.8/libexec/lib/node_modules/@qwen-code/qwen-code/chunks/chunk-I6WGFJJ3.js (DashScopeOpenAICompatibleProvider.buildRequest, shouldEnableThinkingFromEffort, reasoning-disabled branch)
  method: source_code
  observed_on: 2026-09-29
  version: 0.19.8
- claim: On the Anthropic protocol the base supported tiers are low, medium, high; xhigh is added for claude major version 5 or later and for opus 4.7 or later; max is added for major 5 or later and for opus and sonnet 4.6 or later; an unsupported tier is clamped with a debugLogger warning, and `request.config.thinkingConfig.includeThoughts === false` disables effort entirely.
  id: anthropic-generator-source
  limitations: Model gating is by name pattern in the client; no live Anthropic run was made from this host.
  location: /opt/homebrew/Cellar/qwen-code/0.19.8/libexec/lib/node_modules/@qwen-code/qwen-code/chunks/anthropicContentGenerator-CPF22MA7.js (anthropicSupportedEffortTiers, resolveEffectiveEffort)
  method: source_code
  observed_on: 2026-09-29
  version: 0.19.8
- claim: 'On the Gemini protocol low maps to LOW, medium to MEDIUM, high to HIGH, and xhigh and max are clamped to HIGH with a once-per-generator debug warning; `reasoning: false` maps to includeThoughts false.'
  id: gemini-generator-source
  limitations: Client-side mapping only; no live Gemini run was made from this host.
  location: /opt/homebrew/Cellar/qwen-code/0.19.8/libexec/lib/node_modules/@qwen-code/qwen-code/chunks/geminiContentGenerator-CDUNEJQE.js (buildThinkingConfig)
  method: source_code
  observed_on: 2026-09-29
  version: 0.19.8
- claim: The settings reference documents model.reasoningEffort with the tiers low, medium, high, xhigh, max, per-provider mapping and clamping, and `Leave unset to use the model/provider default`; it documents extra_body with an enable_thinking example, and model.enableOpenAILogging / model.openAILoggingDir writing API request and response JSON files.
  id: bundled-settings-doc
  limitations: Documentation shipped inside the package; prose can lag code.
  location: /opt/homebrew/Cellar/qwen-code/0.19.8/libexec/lib/node_modules/@qwen-code/qwen-code/bundled/qc-helper/docs/configuration/settings.md
  method: official_docs
  observed_on: 2026-09-29
  version: 0.19.8
- claim: 'The reasoning configuration section documents the per-provider wire shapes, `reasoning: false` disabling thinking on every provider, budget_tokens (honored as thinking.budget_tokens by Anthropic, ignored by OpenAI/DeepSeek servers), the samplingParams verbatim override that skips reasoning injection, and the coding-plan catalog where qwen3.5-plus and siblings ship with thinking enabled.'
  id: bundled-model-providers-doc
  limitations: Prose summary; exact code paths cited separately above.
  location: /opt/homebrew/Cellar/qwen-code/0.19.8/libexec/lib/node_modules/@qwen-code/qwen-code/bundled/qc-helper/docs/configuration/model-providers.md (Reasoning / thinking configuration)
  method: official_docs
  observed_on: 2026-09-29
  version: 0.19.8
- claim: The command table documents `/effort` as "Set reasoning effort for thinking-capable models" with usage `/effort` (opens picker) and `/effort high` over low/medium/high/xhigh/max, mapped and clamped per provider.
  id: bundled-commands-doc
  limitations: User-facing summary only.
  location: /opt/homebrew/Cellar/qwen-code/0.19.8/libexec/lib/node_modules/@qwen-code/qwen-code/bundled/qc-helper/docs/features/commands.md
  method: official_docs
  observed_on: 2026-09-29
  version: 0.19.8
- claim: The online settings page carries the same model.reasoningEffort row (tiers, per-provider clamping, unset default) and the extra_body notes, including that enable_thinking is translated to reasoning.effort on the openai-responses wire.
  id: online-settings-doc
  limitations: The live site tracks main and may describe a version newer than the installed 0.19.8.
  location: https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: 'Run non-interactively, `/effort` reports the effective configured tier: `Reasoning effort: not set (using the model/provider default).` when nothing is set and `Current reasoning effort: low` when model.reasoningEffort is low in project settings; `/effort maximum` is accepted as an alias for max and persists the tier to settings (the write was observed and reverted).'
  id: effort-noninteractive-test
  limitations: Single host, single model (qwen3.5-plus through DashScope compatible-mode); project-scope settings, user settings untouched.
  location: claudine/docs/research/reasoning-level/_artifacts/qwen-effort-command.txt
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.19.8
- claim: '`qwen -p "/effort bogus"` fails with `Unknown reasoning effort "bogus". Choose one of: low, medium, high, xhigh, max.` and exit code 1; no model request is made.'
  id: effort-invalid-test
  limitations: One invalid token tested; the message template is shared for all unknown tokens.
  location: claudine/docs/research/reasoning-level/_artifacts/qwen-effort-command.txt
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.19.8
- claim: 'An invalid model.reasoningEffort in settings prints `Ignoring invalid model.reasoningEffort "bogus"; expected one of: low, medium, high, xhigh, max.` and the run proceeds on the model/provider default with exit code 0.'
  id: effort-settings-invalid-test
  limitations: Project-scope settings; behavior of the same invalid value in user scope was not separately exercised.
  location: claudine/docs/research/reasoning-level/_artifacts/qwen-effort-command.txt
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.19.8
- claim: With --output-format stream-json the assistant events carry the model's full reasoning as a content part of type thinking, and neither the init nor the result event contains any reasoning-effort field.
  id: stream-json-run
  limitations: One prompt against qwen3.5-plus with no effort set.
  location: claudine/docs/research/reasoning-level/_artifacts/qwen-stream-json-run.jsonl
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.19.8
- claim: With -o json the final result event reports thought tokens under stats.models.<model>.tokens.thoughts alongside the thinking content parts, evidencing that thinking ran without stating a tier.
  id: json-stats-run
  limitations: One prompt; token accounting shape only.
  location: claudine/docs/research/reasoning-level/_artifacts/qwen-json-result-stats.json
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.19.8
- claim: With model.reasoningEffort low in project settings and --openai-logging, the logged request for qwen3.5-plus carries enable_thinking true and preserve_thinking true and no reasoning or reasoning_effort key, proving the tier collapse on qwen-family wire models.
  id: openai-logging-run
  limitations: One model and one tier; the collapse is asserted by code for every tier.
  location: claudine/docs/research/reasoning-level/_artifacts/qwen-openai-logging-request.json
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.19.8
- claim: The session chat record and the session debug log for the test run contain no reasoning-effort or thinking-level field; nothing records the tier a run used.
  id: session-record-check
  limitations: One session examined; only the fields present in that record can be ruled out.
  location: ~/.qwen/projects/-private-tmp-qwen-research-test/chats/<session>.jsonl (one session of the test run)
  method: local_inspection
  observed_on: 2026-09-29
  version: 0.19.8
support: every_model
levels:
- evidence_ids:
  - tiers-source
  - effort-invalid-test
  - effort-dialog-source
  meaning: Fastest and cheapest; least reasoning.
  native: low
  normalized: low
- evidence_ids:
  - tiers-source
  - effort-invalid-test
  - effort-dialog-source
  meaning: Balanced speed, cost, and reasoning.
  native: medium
  normalized: medium
- evidence_ids:
  - tiers-source
  - effort-invalid-test
  - effort-dialog-source
  meaning: Default - strong reasoning for hard tasks.
  native: high
  normalized: high
- evidence_ids:
  - tiers-source
  - effort-invalid-test
  - effort-dialog-source
  meaning: Extended reasoning for agentic/coding work.
  native: xhigh
  normalized: very_high
- evidence_ids:
  - tiers-source
  - effort-invalid-test
  - effort-dialog-source
  meaning: Maximum reasoning; highest cost and latency.
  native: max
  normalized: maximum
default_level:
  decided_by: model
  evidence_ids:
  - effort-noninteractive-test
  - effort-dialog-source
  - bundled-model-providers-doc
  - resolve-config-source
controls:
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - effort-command-source
  - effort-noninteractive-test
  - bundled-commands-doc
  id: session-effort
  kind: session_command
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: /effort
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - bundled-settings-doc
  - resolve-config-source
  - effort-noninteractive-test
  - online-settings-doc
  id: model-reasoning-effort
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: model.reasoningEffort
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - bundled-model-providers-doc
  - resolve-config-source
  id: provider-generation-config-reasoning-effort
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: modelProviders.models.generationConfig.reasoning.effort
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - bundled-model-providers-doc
  - resolve-config-source
  id: model-generation-config-reasoning-effort
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: model.generationConfig.reasoning.effort
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - bundled-model-providers-doc
  - openai-pipeline-source
  - set-effort-source
  id: generation-config-reasoning-false
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: generationConfig.reasoning
  value: on_or_off
- arguments: []
  changes_running_session: no
  evidence_ids:
  - bundled-model-providers-doc
  id: generation-config-sampling-params
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: generationConfig.samplingParams.reasoning_effort
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - bundled-model-providers-doc
  - openai-pipeline-source
  id: generation-config-extra-body
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: generationConfig.extra_body.reasoning_effort
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - bundled-settings-doc
  - openai-pipeline-source
  id: extra-body-enable-thinking
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: generationConfig.extra_body.enable_thinking
  value: on_or_off
- arguments: []
  changes_running_session: no
  evidence_ids:
  - bundled-model-providers-doc
  id: reasoning-budget-tokens
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: generationConfig.reasoning.budget_tokens
  value: token_budget
- arguments: []
  changes_running_session: no
  evidence_ids:
  - bundled-model-providers-doc
  - anthropic-generator-source
  - openai-pipeline-source
  id: request-thinking-config
  kind: request_field
  lasts: one_request
  launch_modes:
  - interactive
  - non_interactive
  name: request.config.thinkingConfig.includeThoughts
  value: on_or_off
precedence:
- generation-config-reasoning-false
- request-thinking-config
- generation-config-extra-body
- extra-body-enable-thinking
- generation-config-sampling-params
- session-effort
- model-reasoning-effort
- provider-generation-config-reasoning-effort
- model-generation-config-reasoning-effort
models:
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - openai-pipeline-source
  - openai-logging-run
  model: qwen*
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - anthropic-generator-source
  model: claude-5-*
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - anthropic-generator-source
  model: claude-opus-4-7*
- accepts:
  - low
  - medium
  - high
  - max
  evidence_ids:
  - anthropic-generator-source
  model: claude-opus-4-6*
- accepts:
  - low
  - medium
  - high
  - max
  evidence_ids:
  - anthropic-generator-source
  model: claude-sonnet-4-6*
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - anthropic-generator-source
  model: claude-*
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - gemini-generator-source
  - bundled-model-providers-doc
  model: gemini-*
- accepts:
  - high
  - max
  evidence_ids:
  - openai-pipeline-source
  - bundled-model-providers-doc
  model: deepseek-*
invalid_level:
  behavior: fails_the_request
  evidence_ids:
  - effort-invalid-test
  - effort-settings-invalid-test
  - effort-command-source
  - resolve-config-source
  message: 'Unknown reasoning effort "<value>". Choose one of: low, medium, high, xhigh, max.'
  warns: yes
reporting:
  command:
  - qwen
  - -p
  - /effort
  - -o
  - text
  evidence_ids:
  - effort-noninteractive-test
  - openai-logging-run
  - bundled-settings-doc
  - stream-json-run
  - session-record-check
  notes: 'The status query reports the requested tier (`Current reasoning effort: low`) or `not set (using the model/provider default)`; it does not report what a run used. The effective tier after per-model clamping surfaces only as a debug-log warning under -d. The exact wire field (--enable-- no; use --openai-logging, or model.enableOpenAILogging with model.openAILoggingDir, defaulting to logs/openai) is written per request as JSON whose body shows enable_thinking or reasoning_effort. In the stream itself, thinking content parts and result stats tokens.thoughts evidence that thinking ran, not which tier was chosen; session records under ~/.qwen/projects carry no effort field.'
  source: status_command
reasoning_output:
  control_id: generation-config-reasoning-false
  evidence_ids:
  - stream-json-run
  - json-stats-run
  reaches_caller: full_text
gaps:
- area: default_level
  detail: The thinking intensity each model applies server-side when no effort is sent is not observable from the client; the CLI sends nothing and the provider default applies.
  next_check: Compare thought-token counts in result stats across an unset run and each tier on one model, or read the ModelStudio per-model documentation for default thinking budgets.
- area: models
  detail: Whether the DashScope endpoint behind qwen-family models will accept a tiered reasoning_effort field (making the five tiers distinct on the wire) is unknown; 0.19.8 collapses every tier to enable_thinking on the client.
  entry: qwen*
  next_check: Re-read shouldEnableThinkingFromEffort in a later release and the DashScope compatible-mode API reference for a tiered thinking parameter.
- area: models
  detail: Which tiers a GLM model on a Z.ai hostname (glm-*) actually accepts verbatim on the flat reasoning_effort field is unverified; the client forwards the token without clamping.
  next_check: Drive a GLM model with each tier and inspect the --openai-logging request and response for rejections.
- area: reporting
  detail: No per-run record of the effective tier exists in stream events, session records, or the debug log (only clamp warnings); a caller cannot confirm after the fact which tier a completed run used without opt-in request logging.
  next_check: Watch for a reasoning-effort field in the typed daemon event schema or the stream result event in a later release.
changes:
- 'First version: effort scale (low/medium/high/xhigh/max), controls (`/effort`, model.reasoningEffort, generationConfig paths), per-model wire mapping and clamping, invalid-level behavior, reporting paths, and reasoning output for Qwen CLI 0.19.8.'
requires_claudine_update: true
reason: Qwen CLI exposes no launch flag and no environment variable for reasoning effort, so Claudine's flag- and env-based wrapper cannot set it today; the only non-interactive surfaces are the `/effort` session command (usable as a first prompt turn) and settings keys (model.reasoningEffort, or a scoped settings overlay), so offering one effort setting across providers needs a new injection mechanism for Qwen CLI.
contract_checked: 2026-09-29
---

Qwen Code (the `qwen` CLI) is a fork of Gemini CLI that speaks to several
backends: Qwen models on DashScope (its default), and OpenAI-compatible,
Anthropic-protocol, and Gemini-protocol providers configured through
`modelProviders`. Reasoning effort is one unified five-tier ladder held in the
client; each backend maps or clamps it to what the active model supports.

## Levels

The effort scale is exactly these five tokens, weakest first (the client ranks
them 20/30/40/60/70):

| Token | What Qwen Code says it does |
| --- | --- |
| `low` | Fastest and cheapest; least reasoning. |
| `medium` | Balanced speed, cost, and reasoning. |
| `high` | Default — strong reasoning for hard tasks. |
| `xhigh` | Extended reasoning for agentic/coding work. |
| `max` | Maximum reasoning; highest cost and latency. |

Input normalization also accepts the aliases `med` (`medium`), `extrahigh`
(`xhigh`), and `maximum` (`max`), case-insensitively and ignoring
spaces/underscores/hyphens; `/effort maximum` sets `max`, for example.

Two things are deliberately **not** tiers:

- **Unset.** When nothing is configured, Qwen Code sends no effort field at
  all and the model's own default applies (`No effort configured — using the
  model/provider default.`). The Coding Plan catalog ships `qwen3.5-plus`,
  `qwen3.6-plus`, `qwen3.7-plus`, `qwen3-max-2026-01-23`, `glm-5`, `kimi-k2.5`,
  and `MiniMax-M2.5` with thinking enabled by default.
- **`reasoning: false`.** A literal boolean at `generationConfig.reasoning`
  that turns thinking off on every provider. It is a switch, not a point on
  the scale, and while it is set, `/effort` cannot re-enable thinking.

## Choosing a Level

There is no launch flag and no environment variable. Every control is a
settings key, a session command, or a request field.

**`/effort` — the session command (works everywhere, persists).**

```sh
qwen -p "/effort high" -o text        # non-interactive: set and persist
qwen                                   # then /effort in the interactive picker
```

Bare `/effort` opens a picker in interactive mode and prints the current tier
in non-interactive mode; `/effort <tier>` sets the tier for the next turn and
persists it under `model.reasoningEffort` in the scope that owns model
selection (user scope by default). The reply is explicit that the request is
only a request: `Reasoning effort: high (requested; the effective tier depends
on the active provider/model).` If thinking is currently disabled it says so:
`Reasoning effort set to high, but thinking is currently disabled — it will
take effect when thinking is re-enabled.`

**`model.reasoningEffort` — the settings key.** In any `settings.json` scope
(`~/.qwen/settings.json`, or `.qwen/settings.json` in the project):

```json
{ "model": { "reasoningEffort": "high" } }
```

Read once at startup; it wins over a provider model's own
`generationConfig.reasoning.effort` but is blocked by `reasoning: false`.
An invalid value warns (`Ignoring invalid model.reasoningEffort "..."` /
`expected one of: low, medium, high, xhigh, max.`) and falls back to the
model/provider default.

**Per-model `generationConfig.reasoning.effort`.** A model entry under
`modelProviders` (provider models) or `model.generationConfig` (runtime
models) can carry its own effort:

```json
{ "modelProviders": { "openai": { "models": [ { "id": "my-model",
  "generationConfig": { "reasoning": { "effort": "max" } } } ] } } }
```

**`reasoning: false` — the off switch.**
`"reasoning": false` in a `generationConfig` disables thinking on every
provider; on `api.deepseek.com` it emits the explicit
`thinking: { type: "disabled" }` the server needs.

**Wire escape hatches (OpenAI-compatible pipeline only).** When
`generationConfig.samplingParams` is set, its keys ship verbatim and the
effort injection is skipped entirely — put the reasoning knob inside it
yourself (`samplingParams.reasoning_effort`). `generationConfig.extra_body`
is spread over the request last, so `extra_body.reasoning_effort` or
`extra_body.enable_thinking` override whatever the pipeline synthesized.
Beware the documented trap: `{ samplingParams: {...}, reasoning: { effort:
"max" } }` silently drops the reasoning field on OpenAI/DeepSeek requests.

**`budget_tokens`.** `"reasoning": { "effort": "high", "budget_tokens": 50000 }`
pins an exact thinking-token budget; only the Anthropic path honors it
(`thinking.budget_tokens`), OpenAI/DeepSeek servers ignore the field.

**Request field (SDK).** Programmatic callers can disable thinking for one
call via `request.config.thinkingConfig.includeThoughts: false`.

**Precedence** (strongest first), each step observed in source: a
`reasoning: false` config (or the per-request `includeThoughts: false`) strips
or overrides thinking fields *after* everything else the pipeline built,
including `extra_body`; then `extra_body.*` (spread last in the provider
request builder); then `samplingParams.*` (verbatim, skips the effort
injection); then the runtime effort (`/effort`, which mutates the live config
for the next turn); then the `model.reasoningEffort` settings key (applied
over the resolved model config at startup); then a provider model's or
runtime model's `generationConfig.reasoning.effort`.

## Models

The client accepts all five tokens everywhere and adapts per backend; the
differences below are in what each model actually receives.

- **`qwen*` (and the `coder-model` alias) on DashScope-compatible endpoints**
  — the effort ladder **collapses to on/off**: any set tier sends
  `enable_thinking: true` (with `preserve_thinking: true`) and drops the
  reasoning field; no tier at all leaves the server default in force. Verified
  on the wire with `low` against `qwen3.5-plus`. All five tokens are accepted
  and never error; they are simply equivalent.
- **Anthropic protocol (`claude-*`)** — tiers are clamped per model: 3.x, 4.5,
  and haiku models take `low`/`medium`/`high`; opus and sonnet 4.6+ add `max`;
  opus 4.7+ and anything 5+ add `xhigh`. An unsupported tier is clamped to the
  nearest supported tier of at least the requested rank with a debug-log
  warning (once per generator).
- **Gemini protocol (`gemini-*`)** — `low`→`LOW`, `medium`→`MEDIUM`,
  `high`→`HIGH`; `xhigh` and `max` clamp to `HIGH` with a debug warning.
- **DeepSeek hostnames (`api.deepseek.com`)** — `low` and `medium` are
  rewritten to `high`, `xhigh` to `max`; the distinct wire levels are `high`
  and `max`. On the Anthropic-compatible DeepSeek endpoint `xhigh`/`max` both
  pass as `max`.
- **GLM models on a Z.ai hostname (`glm-*`)** — the nested effort is flattened
  verbatim to a flat `reasoning_effort`; no client-side clamping (which tiers
  the server accepts is unverified — see gaps).

## Confirming the Level

- **Ask the CLI** (non-interactive status query — reports the *requested*
  tier, not what a run used):

  ```sh
  qwen -p "/effort" -o text
  # Current reasoning effort: low
  # Available: low, medium, high, xhigh, max
  ```

  Or `Reasoning effort: not set (using the model/provider default).` when
  nothing is configured.
- **Inspect the actual request.** `--openai-logging` (or
  `model.enableOpenAILogging`, output dir `model.openAILoggingDir`, default
  `logs/openai` under the working directory) writes one JSON file per request
  whose body shows `enable_thinking` or `reasoning_effort` exactly as sent.
  This is the only way to see the wire field a completed run carried.
- **Clamp warnings.** Per-model clamping logs one line
  (`reasoning.effort='xhigh' is not supported by '<model>'; using 'high'.`)
  through the debug logger, visible with `-d/--debug` — not in normal output.
- **Thought tokens.** The `result` event's
  `stats.models.<model>.tokens.thoughts` counts thinking tokens, evidence
  that thinking ran, not which tier was chosen. Session records
  (`~/.qwen/projects/.../chats/<session>.jsonl`) and the stream's `init` and
  `result` events carry no effort field at all.

**Reasoning output.** With `-o json` or `-o stream-json`, assistant events
carry the model's complete reasoning text as a content part of type `thinking`
(field `thinking`), before the `text` part — the full reasoning reaches a
non-interactive caller. Plain `-o text` prints only the final answer.
`reasoning: false` is the control that removes it; interactively, `Ctrl+O`
expands and collapses thinking blocks in the transcript.

## Sources

- Qwen CLI 0.19.8 install on this host — `qwen --version`, help output
  (`_artifacts/qwen-help.txt`, `_artifacts/qwen-cli-version.txt`).
- Bundled docs, shipped with 0.19.8:
  [settings.md](https://github.com/QwenLM/qwen-code/blob/main/docs/content/en/users/configuration/settings.md)
  and
  [model-providers.md](https://github.com/QwenLM/qwen-code/blob/main/docs/content/en/users/configuration/model-providers.md)
  (local copy: `<install>/bundled/qc-helper/docs/`), plus
  `bundled/qc-helper/docs/features/commands.md`.
- Online docs: <https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/>
  and <https://qwenlm.github.io/qwen-code-docs/en/users/configuration/model-providers/>.
- Minified bundle chunks read as source (symbols named in the evidence list):
  `chunk-SIUQ3YYX.js`, `chunk-2EM7ECZD.js`, `startInteractiveUI-JII36UGN.js`,
  `chunk-3GNOQZDC.js`, `chunk-DIWVZ3VM.js`, `chunk-I6WGFJJ3.js`,
  `anthropicContentGenerator-CPF22MA7.js`, `geminiContentGenerator-CDUNEJQE.js`
  under `<install>/libexec/lib/node_modules/@qwen-code/qwen-code/`.
- Disposable runs against `qwen3.5-plus` via DashScope compatible-mode, with
  project-scope settings only (user config restored after the persistence
  observation): `_artifacts/qwen-effort-command.txt`,
  `_artifacts/qwen-stream-json-run.jsonl`,
  `_artifacts/qwen-json-result-stats.json`,
  `_artifacts/qwen-openai-logging-request.json`.

## Changelog

- 2026-09-29 — first version, researched against Qwen CLI 0.19.8 on macOS:
  five-tier effort ladder, ten controls (no launch flag or env var),
  per-model wire mapping and clamping, invalid-level behavior, reporting
  paths, full reasoning text in json/stream-json output.
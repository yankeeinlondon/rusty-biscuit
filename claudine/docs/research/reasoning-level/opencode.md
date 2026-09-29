---
$schema: ./_schema.yaml
schema_revision: 2
provider: opencode
created: 2026-09-29
last_updated: 2026-09-29
agent: codex
model: gpt-6-luna
reasoning_effort: high
versions_examined:
- 1.18.33
evidence:
- claim: Installed version 1.18.33 exposes `--variant <string>` and `--thinking`; help shows no reasoning-specific config override flag.
  id: cli-help
  limitations: Help does not establish model acceptance or actual upstream effort.
  location: opencode --version; opencode --help; opencode run --help (2026-09-29)
  method: local_inspection
  observed_on: 2026-09-29
  version: 1.18.33
- claim: '"The active config contains `provider.zai-coding-plan.models.glm-5.2.reasoning: true`, but no effort token; the other inspected files have no reasoning/effort/thinking/variant keys."'
  id: local-config
  limitations: The boolean is not an effort choice; project, managed, and remote config were not inspected.
  location: ~/.config/opencode/opencode.json, opencode.jsonc, config.json (sanitized key inspection, 2026-09-29)
  method: local_inspection
  observed_on: 2026-09-29
  version: 1.18.33
- claim: Local MiniMax catalog has empty variant maps for `MiniMax-M2*` and variants `none` and `thinking` for `MiniMax-M3`.
  id: minimax-catalog
  limitations: Only local MiniMax catalog entries were checked; catalog entries do not establish upstream behavior.
  location: opencode models minimax --verbose (2026-09-29)
  method: local_inspection
  observed_on: 2026-09-29
  version: 1.18.33
- claim: Built-in variant examples are `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, and `max`; providers/models expose subsets. Anthropic describes `high` as default and `max` as maximum thinking budget.
  id: models-docs
  limitations: Docs say the list is not comprehensive and are not version-pinned to the installed CLI.
  location: https://opencode.ai/docs/models/
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: '`--variant` chooses provider-specific effort, `--thinking` shows thinking blocks, `--format json` emits raw events; documented environment variables include no direct effort variable.'
  id: cli-docs
  limitations: Rolling docs do not prove compatibility with installed 1.18.33 or upstream acceptance.
  location: https://opencode.ai/docs/cli/
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: '`OPENCODE_CONFIG`, `OPENCODE_CONFIG_DIR`, and `OPENCODE_CONFIG_CONTENT` choose or supply configuration, not a direct effort value.'
  id: config-docs
  limitations: Configuration source precedence does not establish reasoning-control precedence.
  location: https://opencode.ai/docs/config/
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: Provider-specific options include `reasoningEffort` and `thinking`; agent options override global model options; variants can map names to option values.
  id: model-config-docs
  limitations: Options and variant names depend on provider and model.
  location: https://opencode.ai/docs/models/#configure-models
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: Current docs show `--model openai/gpt-5.2#high` and document `#variant` selection for a run, session, agent, or command.
  id: suffix-docs
  limitations: Not version-pinned; suffix compatibility with installed 1.18.33 was not verified.
  location: https://opencode.ai/v2/docs/models
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: '`run` passes `args.variant` into prompt/command requests and uses `args.thinking` for reasoning presentation.'
  id: run-source
  limitations: Passing a requested value does not prove the model used it.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts
  method: source_code
  observed_on: 2026-09-29
  version: 1.18.33
- claim: Prompt requests take optional `variant`, fall back to a valid agent variant, and store the selection in session message model metadata.
  id: prompt-source
  limitations: Stored value is the request selection, not an upstream receipt.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/session/prompt.ts
  method: source_code
  observed_on: 2026-09-29
  version: 1.18.33
- claim: '`opencode run` resolves explicit `--variant`, then valid session-history variant, then saved per-model variant in `~/.local/state/opencode/model.json`.'
  id: variant-source
  limitations: Does not establish ordering against model/agent options, managed config, or model suffix.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run/variant.shared.ts
  method: source_code
  observed_on: 2026-09-29
  version: 1.18.33
- claim: Built-in variant maps depend on provider/model metadata and map common names to provider-specific request options.
  id: transform-source
  limitations: Does not enumerate external/custom variants or confirm upstream behavior.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/provider/transform.ts
  method: source_code
  observed_on: 2026-09-29
  version: 1.18.33
- claim: Unsupported catalog name completed with exit code 0 and no CLI warning; export retained the requested name.
  id: invalid-run
  limitations: Cannot determine whether upstream ignored it, defaulted, or accepted it as undocumented.
  location: opencode run --pure --model minimax/MiniMax-M3 --variant not-a-real-variant --format json (sanitized output, 2026-09-29)
  method: disposable_test
  observed_on: 2026-09-29
  version: 1.18.33
- claim: The noninteractive run emitted a `reasoning` JSON event when `--thinking` was enabled.
  id: reasoning-run
  limitations: One model does not establish output type across providers.
  location: opencode run --pure --model minimax/MiniMax-M3 --thinking --format json (sanitized output, 2026-09-29)
  method: disposable_test
  observed_on: 2026-09-29
  version: 1.18.33
- claim: Exported session/message model metadata contains the requested variant but no distinct effective-level field.
  id: export-run
  limitations: Does not exclude provider-specific telemetry elsewhere.
  location: opencode export ses_f12014ce9ffe91rprp0FJQ1SXo (sanitized output, 2026-09-29)
  method: disposable_test
  observed_on: 2026-09-29
  version: 1.18.33
support: some_models
levels:
- evidence_ids:
  - models-docs
  meaning: OpenAI built-in variant for no reasoning.
  native: none
  normalized: off
- evidence_ids:
  - models-docs
  meaning: OpenAI built-in variant for minimal reasoning effort.
  native: minimal
  normalized: minimal
- evidence_ids:
  - models-docs
  meaning: OpenAI built-in variant for low effort; some providers describe a lower effort or token budget.
  native: low
  normalized: low
- evidence_ids:
  - models-docs
  meaning: OpenAI built-in variant for medium reasoning effort.
  native: medium
  normalized: medium
- evidence_ids:
  - models-docs
  meaning: OpenAI built-in variant for high effort; Anthropic describes a high thinking budget.
  native: high
  normalized: high
- evidence_ids:
  - models-docs
  meaning: OpenAI built-in variant for extra-high reasoning effort.
  native: xhigh
  normalized: very_high
- evidence_ids:
  - models-docs
  meaning: Anthropic built-in variant for maximum thinking budget.
  native: max
  normalized: maximum
- evidence_ids:
  - minimax-catalog
  - transform-source
  meaning: MiniMax M3 variant enabling adaptive thinking rather than naming an ordinal effort point.
  native: thinking
  normalized: outside_scale
default_level:
  decided_by: unknown
  evidence_ids:
  - models-docs
  - prompt-source
controls:
- arguments:
  - --variant
  - <level>
  changes_running_session: no
  evidence_ids:
  - cli-help
  - run-source
  id: run-variant
  kind: launch_flag
  lasts: one_request
  launch_modes:
  - interactive
  - non_interactive
  name: --variant
  value: level_token
- arguments:
  - --model
  - openai/gpt-5.2#<level>
  changes_running_session: no
  evidence_ids:
  - suffix-docs
  id: model-suffix
  kind: model_suffix
  lasts: one_request
  launch_modes:
  - non_interactive
  name: --model
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - prompt-source
  id: request-variant
  kind: request_field
  lasts: one_request
  launch_modes:
  - interactive
  - non_interactive
  name: variant
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - model-config-docs
  id: model-options-config
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: provider.openai.models.gpt-5.options.reasoningEffort
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - model-config-docs
  id: agent-options-config
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: agent.deep-thinker.reasoningEffort
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - prompt-source
  id: agent-variant-config
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: agent.build.variant
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - variant-source
  id: saved-variant
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - non_interactive
  name: model.json
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - models-docs
  - variant-source
  id: variant-cycle
  kind: session_command
  lasts: until_changed
  launch_modes:
  - interactive
  name: variant_cycle
  value: level_token
- arguments:
  - --thinking
  changes_running_session: no
  evidence_ids:
  - cli-help
  - run-source
  - reasoning-run
  id: thinking-display
  kind: launch_flag
  lasts: one_session
  launch_modes:
  - non_interactive
  name: --thinking
  value: on_or_off
precedence:
- run-variant
- variant-cycle
- saved-variant
- agent-variant-config
- agent-options-config
- model-options-config
models:
- accepts: []
  evidence_ids:
  - minimax-catalog
  model: minimax/MiniMax-M2*
- accepts:
  - none
  - thinking
  evidence_ids:
  - minimax-catalog
  model: minimax/MiniMax-M3
invalid_level:
  behavior: unknown
  evidence_ids:
  - invalid-run
  - run-source
  warns: no
reporting:
  evidence_ids:
  - export-run
  - prompt-source
  notes: Session export retains the requested variant, and JSON `step_finish` reports reasoning-token counts, but neither confirms the upstream's effective level.
  source: nowhere
reasoning_output:
  control_id: thinking-display
  evidence_ids:
  - run-source
  - reasoning-run
  reaches_caller: hidden
gaps:
- area: levels
  detail: Built-in variants differ by provider/model, docs list is non-exhaustive, and custom variants can add arbitrary names; these examples are not a complete global token set.
  next_check: Enumerate the exact enabled model catalogs and custom variants for each Claudine deployment.
- area: default_level
  detail: There is no established OpenCode-wide default; docs state Anthropic's built-in `high` default, while other defaults are model/provider-defined and may depend on upstream account behavior.
  next_check: Run no-variant requests for supported provider/model and account classes and inspect request plus provider telemetry.
- area: controls
  detail: The rolling v2 docs describe `#variant`, but installed 1.18.33 suffix support was not tested.
  entry: model-suffix
  next_check: Test `--model provider/model#variant` with installed 1.18.33 against a disposable endpoint.
- area: precedence
  detail: Tagged source establishes explicit flag over session history over saved preference for `opencode run`; precedence against model/agent options, request fields, and `#variant` is not fully established.
  next_check: Send conflicting values through a disposable mock provider via TUI, request API, and `opencode run` and inspect final requests.
- area: models
  detail: Only the local MiniMax catalog was inspected; variant sets vary across the broader provider/model catalog.
  next_check: Inspect `opencode models <provider> --verbose` for each model Claudine offers and group only models whose sets differ.
- area: invalid_level
  detail: The tested unsupported name completed without a CLI warning, but it is unknown whether MiniMax ignored it, used a default, or accepted an undocumented value.
  next_check: Send an unsupported name to a disposable request-capturing endpoint with a fixed variant catalog; inspect request, response, exit, and warning.
- area: reporting
  detail: Session metadata records the requested variant but no inspected surface confirms what effort the upstream actually applied.
  next_check: Inspect provider telemetry or response metadata for an explicit effective-effort field per integration.
- area: reasoning_output
  detail: Default noninteractive presentation hides reasoning; with `--thinking`, full reasoning versus summaries depends on provider/model.
  next_check: Compare `--thinking --format json` outputs for representative providers/models.
changes:
- Updated to schema revision 2 and refreshed against installed 1.18.33, official docs/source, local config/catalog, and disposable runs.
- Corrected resolver precedence and MiniMax catalog findings; distinguished requested variant metadata from confirmed effective effort.
- Added config-option controls and documented remaining unknowns.
requires_claudine_update: true
reason: Claudine needs model-aware variant selection; OpenCode exposes no established provider-confirmed effective-level receipt, so Claudine cannot confirm the requested setting took effect.
contract_checked: 2026-09-29
---

## Levels

OpenCode calls reasoning settings **variants**. It has no closed token set: names come from provider/model metadata and users may define custom variants. Its documented OpenAI examples, weakest first, are `none`, `minimal`, `low`, `medium`, `high`, and `xhigh`. Anthropic documents `high` and `max`, where `max` is its maximum thinking budget; Google documents `low` and `high`. These variants are provider-specific, and the official list is incomplete.

The local MiniMax catalog lists `none` and `thinking` for `minimax/MiniMax-M3`. `thinking` enables adaptive thinking rather than naming an ordinal effort point, so it is outside the shared scale. Locally listed MiniMax M2 models have no catalog-defined variants.

## Choosing a Level

For one request, use the installed CLI's `--variant` flag:

```sh
opencode run --model openai/gpt-5 --variant high "Review this change"
```

The session prompt API accepts a `variant` field. Current rolling docs also show a variant suffix, such as `--model openai/gpt-5.2#high`; compatibility with installed 1.18.33 was not verified.

Configuration can set provider-specific options directly. For example, `provider.openai.models.gpt-5.options.reasoningEffort` sets OpenAI effort, and Anthropic uses a `thinking` option. Agent provider options override global model options; an agent can also select a variant with `agent.<name>.variant`. `variant_cycle` changes the active variant in an interactive session. OpenCode persists a per-model choice in `~/.local/state/opencode/model.json`.

No direct effort environment variable or reasoning-specific config override flag appeared in installed help or the official environment-variable list. `OPENCODE_CONFIG`, `OPENCODE_CONFIG_DIR`, and `OPENCODE_CONFIG_CONTENT` choose/supply config rather than select effort.

For `opencode run`, source establishes explicit `--variant`, then valid session history, then saved per-model preference. If those do not supply a value, a valid agent `variant` may be used. Precedence against all model/agent options and the newer documented `#variant` form remains unresolved. `--thinking` controls reasoning display, not effort.

## Models

The local catalog showed these differences from the documented built-in variants:

| Model pattern | Selectable variants |
| --- | --- |
| `minimax/MiniMax-M2*` | None |
| `minimax/MiniMax-M3` | `none`, `thinking` |

Use `opencode models <provider> --verbose` to inspect available variants. An empty variant map means no catalog-defined selectable variant, even when model metadata says reasoning is supported.

## Confirming the Level

No established OpenCode record states the level the upstream actually applied. `opencode export <sessionID>` retains the requested variant in session metadata. JSON `step_finish` events report reasoning token counts, but neither confirms the upstream honored the selected level.

By default, noninteractive output hides reasoning parts. `--thinking` emits reasoning blocks when the model returns them; the tested MiniMax M3 request produced a `reasoning` JSON event. Whether other providers return full reasoning, a summary, or none depends on the model.

## Sources

- [OpenCode models and variants](https://opencode.ai/docs/models/)
- [OpenCode CLI](https://opencode.ai/docs/cli/)
- [OpenCode configuration](https://opencode.ai/docs/config/)
- [OpenCode v2 model suffix docs](https://opencode.ai/v2/docs/models)
- [OpenCode v1.18.33 run command](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts)
- [OpenCode v1.18.33 prompt handling](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/session/prompt.ts)
- [OpenCode v1.18.33 run variant resolver](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run/variant.shared.ts)
- [OpenCode v1.18.33 provider transforms](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/provider/transform.ts)
- Local inspection on 2026-09-29: installed help, MiniMax catalog, and sanitized `~/.config/opencode` key inspection.
- Disposable tests on 2026-09-29: unsupported variant request, `--thinking --format json`, and session export.

## Changelog

- 2026-09-29: Updated initial research to schema revision 2. Corrected variant precedence and MiniMax catalog data, added config-option controls and current documented `#variant` syntax, and separated requested variant metadata from a provider-confirmed effective level. Recorded remaining cross-provider gaps.
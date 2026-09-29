---
$schema: ./_schema.yaml
schema_revision: 1
provider: opencode
created: 2026-09-29
last_updated: 2026-09-29
agent: codex
model: gpt-6-luna
reasoning_effort: high
versions_examined:
- 1.18.33
evidence:
- claim: The installed CLI is 1.18.33; `opencode run` accepts `--variant <string>` and `--thinking`, and has no reasoning-specific config override flag.
  id: local-version-help
  limitations: Help does not establish model support, invalid-value behavior, or the upstream's effective API payload.
  location: opencode --version; opencode --help; opencode run --help (host terminal, 2026-09-29)
  method: local_inspection
  observed_on: 2026-09-29
  version: 1.18.33
- claim: The inspected user configs contain no reasoning variant or effort selection.
  id: local-config
  limitations: Project, managed, and remote configs were not exhaustively searched; these user files do not establish provider defaults.
  location: ~/.config/opencode/opencode.json and ~/.config/opencode/config.json (host inspection, 2026-09-29)
  method: local_inspection
  observed_on: 2026-09-29
  version: 1.18.33
- claim: The catalog reports `minimax/MiniMax-M2.7-highspeed` with reasoning capability and no variants, while `minimax/MiniMax-M3` exposes `none` and `thinking`.
  id: local-model-catalog
  limitations: This authenticated/local snapshot covers one provider, not all OpenCode providers or custom models.
  location: '`opencode models minimax --verbose` (host terminal, 2026-09-29)'
  method: local_inspection
  observed_on: 2026-09-29
  version: 1.18.33
- claim: Built-in variants include Anthropic `high` and `max`, OpenAI `none`, `minimal`, `low`, `medium`, `high`, `xhigh` with per-model variation, and Google `low` and `high`; the list is non-exhaustive.
  id: official-models-docs
  limitations: The page does not enumerate every model's variants or establish every variant's request payload.
  location: https://opencode.ai/docs/models/
  method: official_docs
  observed_on: 2026-09-29
  version: 1.18.33
- claim: The CLI documents `opencode run --variant` for provider-specific reasoning variants, `--thinking` to show thinking blocks, and `--format json` for raw events.
  id: official-cli-docs
  limitations: The page does not confirm that an upstream model accepted the selected variant.
  location: https://opencode.ai/docs/cli/
  method: official_docs
  observed_on: 2026-09-29
  version: 1.18.33
- claim: Config layers include global, `OPENCODE_CONFIG`, project, `.opencode`/`OPENCODE_CONFIG_DIR`, `OPENCODE_CONFIG_CONTENT`, then managed settings.
  id: official-config-docs
  limitations: Config source precedence does not establish precedence between chosen variants, model options, and provider defaults.
  location: https://opencode.ai/docs/config/
  method: official_docs
  observed_on: 2026-09-29
  version: 1.18.33
- claim: Model options can set provider-specific values such as `reasoningEffort` and `thinking`; agent options override global model options, and models can define named variants.
  id: official-model-options
  limitations: Examples do not define one key/value vocabulary shared by all providers.
  location: https://opencode.ai/docs/models/#configure-models
  method: official_docs
  observed_on: 2026-09-29
  version: 1.18.33
- claim: Documented environment variables include config-path/content variables but no direct reasoning-effort variable.
  id: official-cli-environment
  limitations: Config environment variables can supply custom config containing a model or agent setting.
  location: https://opencode.ai/docs/cli/#environment-variables
  method: official_docs
  observed_on: 2026-09-29
  version: 1.18.33
- claim: The run command passes `args.variant` into the session prompt or command request; absent `--thinking`, non-interactive mode hides reasoning parts.
  id: source-run-variant
  limitations: Passing the requested string does not show that the model accepts or uses it.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts
  method: source_code
  observed_on: 2026-09-29
  version: 1.18.33
- claim: The session prompt schema has an optional top-level `variant` string, and the requested variant is recorded in the user message model reference.
  id: source-prompt-request
  limitations: The stored value is the request, not an upstream provider receipt.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/session/prompt.ts
  method: source_code
  observed_on: 2026-09-29
  version: 1.18.33
- claim: The source merges the custom config file after global config and inline `OPENCODE_CONFIG_CONTENT` after project and `.opencode` config.
  id: source-config-loading
  limitations: This establishes config-source merge order only.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/config/config.ts
  method: source_code
  observed_on: 2026-09-29
  version: 1.18.33
- claim: Built-in reasoning variant construction depends on model capabilities and provider/model family.
  id: source-transform
  limitations: The transform does not enumerate variants loaded from external model metadata or user config.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/provider/transform.ts
  method: source_code
  observed_on: 2026-09-29
  version: 1.18.33
- claim: The run variant resolver orders its inputs as explicit `--variant`, saved per-model preference, then session history; invalid saved/session values are filtered against model variants, but an explicit CLI value is passed through.
  id: source-variant-priority
  limitations: This resolver's precedence does not establish precedence against config-defined model options or managed config, and does not itself prove how an unknown explicit variant affects the provider request.
  location: https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run/variant.shared.ts
  method: source_code
  observed_on: 2026-09-29
  version: 1.18.33
levels:
- evidence_ids:
  - official-models-docs
  meaning: No reasoning, in the OpenAI built-in variant description.
  native: none
  normalized: off
- evidence_ids:
  - official-models-docs
  meaning: Minimal reasoning effort, in the OpenAI built-in variant description.
  native: minimal
  normalized: minimal
- evidence_ids:
  - official-models-docs
  meaning: Low effort; some providers describe this as a lower reasoning or token budget.
  native: low
  normalized: low
- evidence_ids:
  - official-models-docs
  meaning: Medium reasoning effort, in the OpenAI built-in variant description.
  native: medium
  normalized: medium
- evidence_ids:
  - official-models-docs
  meaning: High effort or thinking budget, depending on provider/model mapping.
  native: high
  normalized: high
- evidence_ids:
  - official-models-docs
  meaning: Extra-high reasoning effort in the OpenAI built-in variant description.
  native: xhigh
  normalized: very_high
- evidence_ids:
  - official-models-docs
  meaning: Maximum thinking budget in the Anthropic built-in variant description.
  native: max
  normalized: maximum
- evidence_ids:
  - local-model-catalog
  - source-transform
  meaning: A model-specific thinking-enabled variant; the catalog does not expose an effort budget or ordinal for it.
  native: thinking
  normalized: high
support: some_models
default_level:
  decided_by: unknown
  evidence_ids:
  - local-config
  - official-models-docs
controls:
- arguments:
  - --variant
  - <level>
  changes_running_session: no
  evidence_ids:
  - local-version-help
  - source-run-variant
  id: run-variant
  kind: launch_flag
  lasts: one_request
  launch_modes:
  - interactive
  - non_interactive
  name: --variant
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - source-prompt-request
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
  - official-models-docs
  - source-transform
  id: model-variant-config
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: provider.openai.models.gpt-5.variants
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - official-model-options
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
  - official-model-options
  - source-prompt-request
  id: agent-variant-config
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: agent.build.variant
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - official-models-docs
  id: variant-cycle
  kind: session_command
  lasts: until_changed
  launch_modes:
  - interactive
  name: variant_cycle
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - source-variant-priority
  id: saved-variant
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: model.json
  value: level_token
- arguments:
  - --thinking
  changes_running_session: no
  evidence_ids:
  - local-version-help
  - source-run-variant
  id: thinking-display
  kind: launch_flag
  lasts: one_session
  launch_modes:
  - non_interactive
  name: --thinking
  value: on_or_off
models:
- accepts: []
  evidence_ids:
  - local-model-catalog
  model: minimax/MiniMax-M2.7-highspeed
- accepts:
  - none
  - thinking
  evidence_ids:
  - local-model-catalog
  model: minimax/MiniMax-M3
precedence:
- run-variant
- saved-variant
- variant-cycle
invalid_level:
  behavior: unknown
  evidence_ids:
  - local-version-help
  - source-run-variant
  warns: unknown
reporting:
  evidence_ids:
  - source-prompt-request
  - official-cli-docs
  notes: Session message model.variant records the requested variant, and JSON step_finish events can report reasoning token counts. Neither confirms the upstream provider accepted or actually used the named level.
  source: nowhere
reasoning_output:
  control_id: thinking-display
  evidence_ids:
  - source-run-variant
  - official-cli-docs
  reaches_caller: unknown
gaps:
- detail: 'The complete provider-wide token set is not fixed: model catalogs and custom variants vary, and the docs say their built-in list is not comprehensive.'
  next_check: Enumerate variants from each exact models.dev and authenticated-provider catalog used by the Claudine deployment, including configured custom models.
  subject: levels
- detail: No universal native default is stated; without an OpenCode variant, behavior is delegated to the selected model/provider and may also depend on its account service.
  next_check: Capture upstream requests and responses with no variant for each supported provider/model and representative account configuration.
  subject: default-level
- detail: The CLI forwards a variant string, but no disposable request used a value unsupported by the selected model.
  next_check: Use a disposable mock OpenAI-compatible endpoint and a model with declared variants; request an undeclared value and inspect exit status, logs, and outbound request body.
  subject: invalid-level
- detail: OpenCode records the requested variant, but no inspected record proves the upstream used that level.
  next_check: Inspect provider-side telemetry or an upstream response field that explicitly reports the effective reasoning effort.
  subject: reporting
- detail: '`--thinking` displays returned reasoning parts, but whether their text is full reasoning or a summary depends on provider/model.'
  next_check: Run non-interactive requests with `--thinking --format json` on representative providers and classify the returned reasoning parts.
  subject: reasoning-output
- detail: Source establishes `--variant` over saved preference over session history for the run variant resolver, but does not establish precedence against model/agent config options, direct request fields, or managed settings across CLI modes.
  next_check: Use a disposable local provider with conflicting values at each config/request layer and inspect final requests from TUI and `opencode run`.
  subject: precedence
changes:
- Initial research document.
requires_claudine_update: true
reason: Claudine needs model-aware variant support because OpenCode uses provider-specific, model-specific names and some reasoning-capable models expose no selectable variant.
contract_checked: 2026-09-29
---

## Levels

OpenCode's reasoning setting is a **model variant**, not a universal effort enum. `opencode run --variant` passes a variant name, and the selected provider maps it into its own request format. The built-in names documented across popular providers include `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, and `max`. OpenCode says this list is not comprehensive, and models may support only a subset.

The documented descriptions order the OpenAI-style values from no reasoning through extra-high. Anthropic documents `high` and `max` as its thinking-budget variants. One installed MiniMax model exposes the non-scalar token `thinking`; its catalog entry does not state an effort budget. Custom variants may use any configured name, so there is no closed CLI token list.

## Choosing a Level

For a one-shot run, pass the model's variant by flag:

```sh
opencode run --model openai/gpt-5 --variant high "Review this change"
```

The local 1.18.33 help shows `--variant <string>`. `--thinking` controls whether returned reasoning blocks are displayed; it does not select effort. `--format json` emits raw events.

The session request API has a top-level `variant` string. In model configuration, `provider.<id>.models.<id>.variants` defines named variants; provider-specific options such as `options.reasoningEffort` can set request behavior directly. Agent options override global model options. Agent config can also select a variant with `agent.<name>.variant`.

In the TUI, use the `variant_cycle` keybind to cycle the current model's variants. The model picker is `/models`. OpenCode stores a per-model variant preference in `~/.local/state/opencode/model.json`; the run resolver prioritizes an explicit `--variant`, then that saved value, then session history. No reasoning-setting environment variable or config override flag appears in the installed help or documented CLI environment-variable list. `OPENCODE_CONFIG`, `OPENCODE_CONFIG_DIR`, and `OPENCODE_CONFIG_CONTENT` select or supply config sources; they are not direct effort variables.

The inspected files under `~/.config/opencode` had no reasoning level set. Config-source precedence is remote, global, `OPENCODE_CONFIG`, project, `.opencode`/`OPENCODE_CONFIG_DIR`, inline `OPENCODE_CONFIG_CONTENT`, then managed settings. That order does not establish whether a request variant, saved TUI choice, agent setting, or model option wins; see the precedence gap.

## Models

The choice is model-specific. OpenCode's docs give examples of provider-level defaults, while the installed catalog provides these concrete exceptions:

| Model | Selectable variants observed |
| --- | --- |
| `minimax/MiniMax-M2.7-highspeed` | None (`variants: {}`), despite reasoning capability |
| `minimax/MiniMax-M3` | `none`, `thinking` |

The catalog snapshot covers the configured MiniMax provider only. The official built-in variant list is partial; query the exact catalog/model entry before offering a value to a caller.

## Confirming the Level

There is no provider-confirmed effective-level report established here. Session messages record the requested `model.variant`; raw JSON `step_finish` events can include reasoning token counts, which indicate reasoning activity but do not identify the accepted effort. Neither value confirms that the upstream service honored the requested variant.

## Sources

- [OpenCode CLI documentation](https://opencode.ai/docs/cli/)
- [OpenCode model and variant documentation](https://opencode.ai/docs/models/)
- [OpenCode configuration documentation](https://opencode.ai/docs/config/)
- [OpenCode v1.18.33 `run.ts`](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/cli/cmd/run.ts)
- [OpenCode v1.18.33 session prompt schema](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/session/prompt.ts)
- [OpenCode v1.18.33 config loading](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/config/config.ts)
- [OpenCode v1.18.33 provider transforms](https://github.com/anomalyco/opencode/blob/v1.18.33/packages/opencode/src/provider/transform.ts)
- Local checks: `opencode --version`, `opencode --help`, `opencode run --help`, `opencode models minimax --verbose`, and read-only inspection of `~/.config/opencode/opencode.json` and `config.json` on 2026-09-29.

## Changelog

- 2026-09-29: Initial research for OpenCode CLI 1.18.33.
---
$schema: ./_schema.yaml
schema_revision: 2
provider: kilo
created: 2026-09-29
last_updated: 2026-09-29
agent: codex
model: gpt-6-luna
reasoning_effort: high
versions_examined:
- 7.3.45
evidence:
- claim: Installed kilo run --help exposes --variant <string> and --thinking, described as showing thinking blocks.
  id: cli-help
  limitations: Help does not establish precedence, a default level, or whether an upstream model honored a selected variant.
  location: claudine/docs/research/reasoning-level/_artifacts/kilo-run-help-7.3.45.txt
  method: local_inspection
  observed_on: 2026-09-29
  version: 7.3.45
- claim: ~/.kilo was absent and ~/.config/kilo/kilo.jsonc contained only the schema URL, with no reasoning setting.
  id: config-inspection
  limitations: This describes the inspected account on this host only.
  location: claudine/docs/research/reasoning-level/_artifacts/kilo-config-inspection.txt
  method: local_inspection
  observed_on: 2026-09-29
  version: 7.3.45
- claim: kilo models --verbose listed 606 model entries with their variant keys; the catalog's distinct native keys are instant, none, minimal, low, medium, high, thinking, xhigh, and max.
  id: model-catalog
  limitations: Catalog presence records Kilo's declared variants, not successful upstream requests. The list can change as providers update.
  location: claudine/docs/research/reasoning-level/_artifacts/kilo-7.3.45-model-variants.json
  method: local_inspection
  observed_on: 2026-09-29
  version: 7.3.45
- claim: Kilo documents /variant, Shift+Tab cycling, per-agent and global defaults, picker precedence, and per-task invalid selections failing rather than silently falling back.
  id: model-selection-docs
  limitations: The page does not give the exact global variant configuration key or order --variant and API request fields against remembered selections.
  location: https://kilo.ai/docs/code-with-ai/agents/model-selection
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: Agent configuration has a variant property; configuration merge order is built-in, global, project, .kilo/.kilocode config and agent files, then KILO_CONFIG_CONTENT.
  id: custom-modes-docs
  limitations: This order is documented for agent configuration merges, not for an explicit run flag or request field.
  location: https://kilo.ai/docs/customize/custom-modes
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: Named model variants are provider-specific configurations merged into the request, and custom models may define their own variant names.
  id: custom-models-docs
  limitations: Custom variant names are user-defined, so they do not form a finite provider-wide token list.
  location: https://kilo.ai/docs/code-with-ai/agents/custom-models
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: Workflow frontmatter and the command section of kilo.jsonc accept a variant override for a workflow.
  id: workflow-docs
  limitations: The page does not establish precedence against run flags, request fields, or agent defaults.
  location: https://kilo.ai/docs/customize/workflows
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: Kilo documents --variant as provider-specific reasoning effort and --thinking as showing thinking blocks.
  id: cli-reference
  limitations: The reference does not list model support or prove a model honored a choice.
  location: https://kilo.ai/docs/code-with-ai/platforms/cli-reference
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: The config schema declares AgentConfig.variant, command entries' variant field, custom model variants, and reasoning_display values.
  id: config-schema
  limitations: Schema fields establish configuration shape, not precedence or output from a particular run.
  location: https://app.kilo.ai/config.json
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: 'Session prompt, promptAsync, and command API request types each expose a variant?: string field.'
  id: sdk-request-types
  limitations: The generated SDK types do not establish how each endpoint resolves conflicts with configuration.
  location: ~/.config/kilo/node_modules/@kilocode/sdk/dist/v2/gen/sdk.gen.d.ts
  method: local_inspection
  observed_on: 2026-09-29
  version: 7.3.45
- claim: AssistantMessage has an optional variant field and UserMessage.model has an optional variant field.
  id: sdk-message-types
  limitations: These types do not prove an upstream provider honored the value.
  location: ~/.config/kilo/node_modules/@kilocode/sdk/dist/v2/gen/types.gen.d.ts
  method: local_inspection
  observed_on: 2026-09-29
  version: 7.3.45
- claim: reasoning_display controls how reasoning blocks appear in the VS Code chat UI.
  id: settings-docs
  limitations: UI display settings do not establish what reasoning content a non-interactive CLI caller receives.
  location: https://kilo.ai/docs/getting-started/settings
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: A run with --variant definitely-not-a-variant stopped at an API authentication error (401) before invalid-variant handling could be observed.
  id: invalid-variant-attempt
  limitations: Authentication prevented the test from establishing invalid-variant behavior.
  location: claudine/docs/research/reasoning-level/_artifacts/kilo-invalid-variant-session.txt
  method: disposable_test
  observed_on: 2026-09-29
  version: 7.3.45
- claim: Kilo's exported assistant message record includes the selected value at /info/variant.
  id: session-export
  limitations: This records Kilo's selected variant and does not prove that the upstream model accepted or used it.
  location: claudine/docs/research/reasoning-level/_artifacts/kilo-invalid-variant-session.txt
  method: local_inspection
  observed_on: 2026-09-29
  version: 7.3.45
support: some_models
levels:
- evidence_ids:
  - model-catalog
  - custom-models-docs
  meaning: Fast, non-thinking choice on models whose catalog offers the instant/thinking pair; exact behavior is model-specific.
  native: instant
  normalized: off
- evidence_ids:
  - model-catalog
  - custom-models-docs
  meaning: No-reasoning choice where the model catalog offers this token.
  native: none
  normalized: off
- evidence_ids:
  - model-catalog
  - custom-models-docs
  meaning: Lowest named effort choice in variants that expose minimal.
  native: minimal
  normalized: minimal
- evidence_ids:
  - model-catalog
  - custom-models-docs
  meaning: Low reasoning effort where listed for the model.
  native: low
  normalized: low
- evidence_ids:
  - model-catalog
  - custom-models-docs
  meaning: Medium reasoning effort where listed for the model.
  native: medium
  normalized: medium
- evidence_ids:
  - model-catalog
  - custom-models-docs
  meaning: High reasoning effort where listed for the model.
  native: high
  normalized: high
- evidence_ids:
  - model-catalog
  - custom-models-docs
  meaning: Thinking-enabled choice on models that expose the instant/thinking pair; exact behavior is model-specific.
  native: thinking
  normalized: high
- evidence_ids:
  - model-catalog
  - custom-models-docs
  meaning: Extra-high effort, ordered above high where offered.
  native: xhigh
  normalized: very_high
- evidence_ids:
  - model-catalog
  - custom-models-docs
  meaning: Maximum named effort, ordered above the other numeric effort choices where offered.
  native: max
  normalized: maximum
default_level:
  decided_by: unknown
  evidence_ids:
  - model-catalog
  - config-inspection
  - model-selection-docs
controls:
- arguments:
  - --variant
  - <level>
  changes_running_session: no
  evidence_ids:
  - cli-help
  - cli-reference
  id: run-variant
  kind: launch_flag
  lasts: one_session
  launch_modes:
  - interactive
  - non_interactive
  name: --variant
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - custom-modes-docs
  - config-schema
  id: agent-variant
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: agent.code.variant
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - custom-modes-docs
  id: agent-file-variant
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: variant
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - workflow-docs
  - config-schema
  id: workflow-config-variant
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: command.submit-pr.variant
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - workflow-docs
  id: workflow-file-variant
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: variant
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - custom-modes-docs
  id: config-content
  kind: environment_variable
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: KILO_CONFIG_CONTENT
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - model-selection-docs
  id: session-variant
  kind: session_command
  lasts: until_changed
  launch_modes:
  - interactive
  name: /variant
  value: level_token
- arguments:
  - --thinking
  changes_running_session: no
  evidence_ids:
  - cli-help
  - cli-reference
  id: show-thinking
  kind: launch_flag
  lasts: one_session
  launch_modes:
  - non_interactive
  name: --thinking
  value: on_or_off
- arguments: []
  changes_running_session: no
  evidence_ids:
  - sdk-request-types
  id: request-variant
  kind: request_field
  lasts: one_request
  launch_modes:
  - non_interactive
  name: variant
  value: level_token
precedence:
- session-variant
- config-content
- agent-file-variant
- agent-variant
models:
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-6-luna*
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-6-astra*
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/openai/gpt-6-sol*
- accepts:
  - none
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-opus-4.6
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-opus-4.7
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-opus-4.8
- accepts:
  - none
  - low
  - medium
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-sonnet-4.6
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-sonnet-5
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - model-catalog
  model: kilo/anthropic/claude-sonnet-5.5
- accepts:
  - none
  - low
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/deepseek/deepseek-v4-flash
- accepts:
  - none
  - low
  - high
  - max
  evidence_ids:
  - model-catalog
  model: kilo/deepseek/deepseek-v4-pro
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/google/gemini-3.1-pro*
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/google/gemini-3.1-flash-lite
- accepts:
  - none
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - model-catalog
  model: kilo/google/gemini-3.1-flash-lite-image
invalid_level:
  behavior: unknown
  evidence_ids:
  - invalid-variant-attempt
  warns: unknown
reporting:
  evidence_ids:
  - session-export
  - sdk-message-types
  field: /info/variant
  locator: session-export
  notes: The exported assistant message records Kilo's selected variant. This confirms Kilo's recorded run choice, not that the upstream model honored that variant.
  source: session_record
reasoning_output:
  control_id: show-thinking
  evidence_ids:
  - cli-help
  - cli-reference
  - settings-docs
  reaches_caller: unknown
gaps:
- area: default_level
  detail: The inspected config has no variant setting, and the reviewed documentation does not state one universal default or establish whether the default varies by model or account.
  next_check: Run authenticated disposable requests without a variant against models from different providers and inspect both the Kilo request and exported assistant record.
- area: controls
  detail: The documented Shift+Tab shortcut cycles variants in the active prompt, but the contract's control-name pattern cannot store the literal plus sign.
  next_check: Extend the control name pattern to allow + and record Shift+Tab as a session_command.
- area: controls
  detail: The model-selection page mentions a global default variant but does not specify its config key or storage location; only the per-agent variant key is established.
  next_check: Inspect the current Settings → Models config write or use a disposable profile to save a global variant and inspect the resulting kilo.jsonc.
- area: precedence
  detail: Documentation orders session and configuration selections, but does not establish where --variant, the variant request field, or workflow overrides rank when they conflict.
  next_check: With a disposable authenticated profile, set conflicting values at each control and inspect the request payload and exported session record.
- area: models
  detail: The catalog has 606 models and many distinct accepted-variant subsets. This document records representative groups, including the requested gpt-6-luna, but the contract's 40-entry maximum and model identifier pattern's exclusion of ~ prevent complete representation of the catalog and its aliases.
  next_check: Extend the model-pattern contract for Kilo's exact identifier grammar, then generate and validate a compact, exhaustive grouping of all 606 model entries.
- area: invalid_level
  detail: The disposable invalid-token run stopped at a 401 authentication error before Kilo or the upstream provider could validate the token.
  next_check: Repeat the same invalid --variant request in an authenticated disposable profile against a model with catalogued variants.
- area: reasoning_output
  detail: Help says --thinking shows thinking blocks, but no authenticated non-interactive response was available to determine whether reasoning text, a summary, or nothing reaches the caller.
  next_check: Run authenticated non-interactive JSON and default-format sessions with and without --thinking, then inspect emitted output for reasoning content.
changes:
- Raised the document to reasoning-level schema revision 2 and refreshed the installed Kilo version as 7.3.45.
- Replaced the previous 606-record model dump with representative grouped entries and documented the model-pattern and entry-limit gap.
- Added request-field, workflow, agent-file, and environment-overlay controls from current docs and installed SDK types.
- 'Corrected the invalid-level and reasoning-output conclusions: the available attempt was blocked by authentication, and no non-interactive reasoning output was verified.'
requires_claudine_update: true
reason: Claudine should expose Kilo's provider-specific variant setting and avoid assuming a universal default or that an exported selection proves the upstream model honored it.
contract_checked: 2026-09-29
---
## Levels

Kilo calls a reasoning choice a model variant. Its installed catalog for version 7.3.45 contains these built-in tokens: `instant`, `none`, `minimal`, `low`, `medium`, `high`, `thinking`, `xhigh`, and `max`. Which tokens appear depends on the selected model. Kilo also lets custom model definitions add named variants, so custom names are not a fixed part of this list.

The `instant`/`thinking` pair is model-specific. Kilo does not define one universal meaning for those names, though `instant` is the faster non-thinking option and `thinking` enables thinking on models that expose the pair. The approximate normalized ordering puts `none` and `instant` at the off end, then `minimal`, `low`, `medium`, `high`, `xhigh`, and `max`.

## Choosing a Level

For one non-interactive run, pass the provider's exact token:

```sh
kilo run --model openai/gpt-6-luna --variant high --format json "Explain the change"
```

Other controls include:

- In an interactive session, use `/variant` to choose a variant. Kilo also documents `Shift+Tab` to cycle through variants in the prompt.
- Set an agent default with `agent.<agent>.variant` in global or project `kilo.jsonc`. For example: `{"agent":{"code":{"variant":"high"}}}`.
- In a `.kilo/agents/<agent>.md` file, set the YAML frontmatter key `variant: high`.
- Set the environment overlay with `KILO_CONFIG_CONTENT='{"agent":{"code":{"variant":"high"}}}' kilo run "Explain the change"`.
- Set a workflow-specific choice using `variant: high` in the workflow's Markdown frontmatter or `command.<workflow>.variant` in `kilo.jsonc`.
- Kilo's session prompt, promptAsync, and command API requests accept a `variant` field. This applies to callers using the Kilo SDK/API.
- `--thinking` only asks the CLI to show thinking blocks; it does not select an effort level.

The documented configuration merge order is built-in defaults, global config, project config, `.kilo/` and `.kilocode/` config and agent files, then `KILO_CONFIG_CONTENT`. Kilo's picker docs put a session override above remembered per-agent selections, then per-agent config and global config. The docs do not settle conflicts between those choices and `--variant`, request fields, or workflow overrides.

## Models

The installed catalog lists 606 models. The frontmatter records representative model groups whose variant sets differ from the union of catalog tokens, including `kilo/openai/gpt-6-luna`. The contract's current 40-entry limit and model pattern syntax cannot capture every group or identifiers containing `~`; consult the versioned catalog artifact for the full observed list.

## Confirming the Level

Use `kilo export <sessionID>` and inspect the assistant message's `/info/variant`. This is Kilo's recorded selection for the run. It does not prove that an upstream provider accepted or applied the selected reasoning effort.

## Sources

- [Kilo Code model selection](https://kilo.ai/docs/code-with-ai/agents/model-selection)
- [Kilo Code custom modes and configuration precedence](https://kilo.ai/docs/customize/custom-modes)
- [Kilo Code custom models and variants](https://kilo.ai/docs/code-with-ai/agents/custom-models)
- [Kilo Code workflow variants](https://kilo.ai/docs/customize/workflows)
- [Kilo Code CLI reference](https://kilo.ai/docs/code-with-ai/platforms/cli-reference)
- [Kilo Code configuration schema](https://app.kilo.ai/config.json)
- Local CLI help, installed configuration inspection, model catalog, SDK type declarations, and sanitized session export listed in the evidence above.

## Changelog

The previous revision used schema revision 1, listed all 606 catalog entries, and treated a prior unauthenticated attempt as an unresolved check without distinguishing the 401 blocker. This revision upgrades the schema, adds the current controls and SDK request field, groups representative model families, documents the limits preventing exhaustive model coverage, and records invalid handling and non-interactive reasoning output as unknown until authenticated runs can settle them.
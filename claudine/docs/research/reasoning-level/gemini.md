---
$schema: ./_schema.yaml
schema_revision: 2
provider: gemini
created: 2026-09-29
last_updated: 2026-09-29
agent: codex
model: gpt-6-luna
reasoning_effort: high
versions_examined:
- 0.61.0
evidence:
- claim: The installed CLI is 0.61.0. Help lists --model/-m, prompt and output flags, but no reasoning-level flag or configuration-override flag.
  id: local-version-help
  limitations: This describes only the installed macOS CLI's command-line interface.
  location: gemini --version; gemini --help (Gemini CLI 0.61.0)
  method: local_inspection
  observed_on: 2026-09-29
  version: 0.61.0
- claim: The inspected user settings have no reasoning-level or thinking-budget value; the separate config has only remoteControlHostname, and this workspace has no project .gemini/settings.json.
  id: local-settings
  limitations: This does not inspect other users' settings or system-wide Gemini CLI settings.
  location: ~/.gemini/settings.json; ~/.gemini/config/config.json; project .gemini/settings.json
  method: local_inspection
  observed_on: 2026-09-29
  version: 0.61.0
- claim: The bundled SDK ThinkingLevel enum exposes THINKING_LEVEL_UNSPECIFIED, LOW, and HIGH; chat defaults use thinkingBudget 8192 for Gemini 2.5 aliases and HIGH for Gemini 3 aliases.
  id: local-thinking-enum-defaults
  limitations: The enum is version-pinned to the installed bundle; API models may accept additional values that this CLI's bundled SDK does not expose.
  location: /Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/chunk-JDPZ4CE3.js:258400 and :360449 (ThinkingLevel; DEFAULT_MODEL_CONFIGS)
  method: source_code
  observed_on: 2026-09-29
  version: 0.61.0
- claim: Model configuration resolves the selected alias and its inheritance chain first, then applies matching overrides; overrides are ordered by match level, specificity, and configuration order, with later equally specific entries applied last.
  id: local-model-config-resolver
  limitations: This establishes in-memory model-config resolution, not the merge precedence of conflicting modelConfigs values across every settings-file layer.
  location: /Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/chunk-JDPZ4CE3.js:340430-340500 (ModelConfigService.internalGetResolvedConfig)
  method: source_code
  observed_on: 2026-09-29
  version: 0.61.0
- claim: Non-interactive response text filters out thought-marked parts, and its output formatters emit message/result data without a thought record; usage may include thoughts_token_count.
  id: local-output-filter
  limitations: Interactive display has separate thought rendering settings; token counts do not identify the selected effort level.
  location: /Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/chunk-JDPZ4CE3.js:349276 and non-interactive JSON/text formatters
  method: source_code
  observed_on: 2026-09-29
  version: 0.61.0
- claim: The installed CLI exposes no direct reasoning flag, config-override flag, reasoning environment variable, model-name effort suffix, or in-session command dedicated to selecting a thinking level.
  id: local-no-level-cli-control
  limitations: A future CLI release may add a direct control; model and alias selectors can still select a preset that contains effort configuration.
  location: gemini --help; installed Gemini CLI 0.61.0 settings schema, command docs, and environment-variable docs/source
  method: local_inspection
  observed_on: 2026-09-29
  version: 0.61.0
- claim: Gemini CLI settings precedence is defaults, system defaults, user settings, project settings, system settings, environment variables, then command-line arguments; settings strings can interpolate environment variables.
  id: docs-cli-settings
  limitations: This generic order does not make any environment variable a direct thinking-level setting.
  location: https://geminicli.com/docs/reference/configuration/
  method: official_docs
  observed_on: 2026-09-29
  version: current documentation read 2026-09-29
- claim: Gemini CLI configures thinking through modelConfigs aliases and conditional overrides, passing generateContentConfig values to the provider with minimal validation.
  id: docs-model-generation
  limitations: The page does not establish which values each model accepts; installed version evidence is used for the local enum.
  location: https://geminicli.com/docs/cli/generation-settings/
  method: official_docs
  observed_on: 2026-09-29
  version: current documentation read 2026-09-29
- claim: Model selection precedence is --model, GEMINI_MODEL, model.name in settings.json, local model routing, then the default model; /model set can change the selected model in a session.
  id: docs-cli-model-routing
  limitations: Model selection is an indirect way to choose a preset, not a direct level value.
  location: https://geminicli.com/docs/cli/model-routing/
  method: official_docs
  observed_on: 2026-09-29
  version: current documentation read 2026-09-29
- claim: /model manage opens a model configuration dialog and /model set selects a model; the documented command list has no dedicated reasoning-level command.
  id: docs-cli-commands
  limitations: Current hosted documentation may be newer than CLI 0.61.0; local help/source was also inspected.
  location: https://geminicli.com/docs/reference/commands/
  method: official_docs
  observed_on: 2026-09-29
  version: current documentation read 2026-09-29
- claim: Gemini API documents LOW and HIGH as thinking-level tokens, ordered from less to more effort, and documents Gemini 2.5 controls through thinkingBudget rather than named thinking levels.
  id: docs-api-thinking-levels
  limitations: The online API supports newer model versions and level values that are not all exposed by the installed CLI SDK enum.
  location: https://ai.google.dev/gemini-api/docs/generate-content/thinking
  method: official_docs
  observed_on: 2026-09-29
  version: current documentation read 2026-09-29
- claim: The API's model matrix shows model-specific accepted levels and defaults; Gemini 2.5 models use numeric budgets in the CLI's GenerateContent configuration while Gemini 3 chat aliases use HIGH by default.
  id: docs-api-thinking-models
  limitations: API model support does not prove that every newer API level is exposed by CLI 0.61.0.
  location: https://ai.google.dev/gemini-api/docs/thinking
  method: official_docs
  observed_on: 2026-09-29
  version: current documentation read 2026-09-29
- claim: A request configured with thinkingLevel BOGUS received HTTP 400 INVALID_ARGUMENT naming generation_config.thinking_config.thinking_level and the invalid value BOGUS.
  id: test-invalid-thinking-level
  limitations: The probe tested one invalid enum string against one Gemini 3 model and account; it does not test model-specific rejection of a valid enum value.
  location: /var/folders/l9/xdcp3xnn6s78_5l9w2_mnvtw0000gn/T/gemini-client-error-Turn.run-sendMessageStream-2026-09-29T09-42-41-629Z.json (captured API error from isolated Gemini CLI 0.61.0 probe)
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.61.0
support: some_models
levels:
- evidence_ids:
  - local-thinking-enum-defaults
  - docs-api-thinking-levels
  meaning: The lower of the named thinking-level choices; use less reasoning effort than HIGH.
  native: LOW
  normalized: low
- evidence_ids:
  - local-thinking-enum-defaults
  - docs-api-thinking-levels
  meaning: The higher named thinking-level choice; Gemini CLI's built-in Gemini 3 chat aliases select it.
  native: HIGH
  normalized: high
default_level:
  decided_by: model
  evidence_ids:
  - local-thinking-enum-defaults
  - docs-api-thinking-models
  - local-settings
controls:
- arguments: []
  changes_running_session: no
  evidence_ids:
  - local-thinking-enum-defaults
  - docs-model-generation
  id: thinking-level-setting
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: thinkingConfig.thinkingLevel
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - local-thinking-enum-defaults
  - docs-model-generation
  - docs-api-thinking-levels
  id: thinking-budget-setting
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: thinkingConfig.thinkingBudget
  value: token_budget
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-model-generation
  - local-model-config-resolver
  id: model-config-custom-aliases
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: modelConfigs.customAliases
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - local-thinking-enum-defaults
  - docs-model-generation
  id: model-config-aliases
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: modelConfigs.aliases
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-model-generation
  - local-model-config-resolver
  id: model-config-overrides
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: modelConfigs.overrides
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - local-thinking-enum-defaults
  - local-model-config-resolver
  id: model-config-custom-overrides
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: modelConfigs.customOverrides
  value: level_token
- arguments:
  - --model
  - <model>
  changes_running_session: no
  evidence_ids:
  - local-version-help
  - local-no-level-cli-control
  - docs-cli-model-routing
  id: model-launch-selector
  kind: launch_flag
  lasts: one_session
  launch_modes:
  - interactive
  - non_interactive
  name: --model
  value: level_token
- arguments:
  - -m
  - <model>
  changes_running_session: no
  evidence_ids:
  - local-version-help
  - docs-cli-model-routing
  id: model-short-launch-selector
  kind: launch_flag
  lasts: one_session
  launch_modes:
  - interactive
  - non_interactive
  name: -m
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-cli-model-routing
  id: model-environment-selector
  kind: environment_variable
  lasts: one_session
  launch_modes:
  - interactive
  - non_interactive
  name: GEMINI_MODEL
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-cli-model-routing
  - docs-cli-settings
  id: model-settings-selector
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: model.name
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - docs-cli-model-routing
  - docs-cli-commands
  - local-no-level-cli-control
  id: model-session-command
  kind: session_command
  lasts: until_changed
  launch_modes:
  - interactive
  name: /model
  value: level_token
precedence:
- model-config-custom-overrides
- model-config-overrides
- model-config-custom-aliases
- model-config-aliases
- thinking-level-setting
- thinking-budget-setting
- model-session-command
- model-launch-selector
- model-short-launch-selector
- model-environment-selector
- model-settings-selector
models:
- accepts: []
  evidence_ids:
  - local-thinking-enum-defaults
  - docs-api-thinking-levels
  model: gemini-2.5-*
invalid_level:
  behavior: fails_the_request
  evidence_ids:
  - test-invalid-thinking-level
  message: Invalid value at 'generation_config.thinking_config.thinking_level' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConfig.ThinkingLevel), "BOGUS"
  warns: yes
reporting:
  evidence_ids:
  - local-output-filter
  - local-thinking-enum-defaults
  notes: Non-interactive JSON, stream-json, and text output do not report the applied thinkingLevel or thinkingBudget. Usage may include thoughts_token_count, which counts generated thought tokens but cannot identify the selected level. The inspected session records also do not record the selected setting for a turn.
  source: nowhere
reasoning_output:
  evidence_ids:
  - local-output-filter
  reaches_caller: hidden
gaps:
- area: default_level
  detail: 'There is no single named default across model families: built-in Gemini 2.5 chat aliases use thinkingBudget 8192, while Gemini 3 chat aliases use HIGH. Automatic model selection can resolve differently according to enabled models and account access; the exact effective default for each account was not measured.'
  next_check: Run successful no-override requests through isolated CLI profiles for each available model and authentication type, and inspect the generated request configuration.
- area: precedence
  detail: Matching model overrides are ordered by hierarchy and specificity before declaration order, while config-file layers and model selectors form separate precedence systems; a single total order cannot describe every conflicting combination.
  next_check: Use an isolated settings profile with conflicting values at each file layer, model alias, and override match scope, then inspect the resolved request configuration.
- area: models
  detail: Gemini 2.5 chat aliases use a numeric budget instead of the CLI's named thinkingLevel field. Model-specific acceptance of named LOW or HIGH values through CLI 0.61.0 was not independently tested.
  entry: gemini-2.5-*
  next_check: Run isolated requests for LOW and HIGH against each Gemini 2.5 model and inspect whether the API accepts the named field or requires thinkingBudget.
- area: invalid_level
  detail: The captured invalid-level request confirms rejection of BOGUS, but no request tested a valid CLI token that is unsupported by the chosen model.
  next_check: Configure a valid enum token for a model documented not to support it and record whether Gemini API refuses, falls back, or ignores it.
changes:
- Updated the file to contract revision 2 and rechecked the installed CLI, settings, bundled SDK enum, model-config resolution, and non-interactive output handling.
- Clarified numeric Gemini 2.5 thinking budgets, account/model-dependent defaults, indirect model selectors, and unresolved precedence and model-specific behavior.
requires_claudine_update: true
reason: Gemini CLI has no provider-wide effort flag. Claudine must project its normalized effort onto modelConfigs.thinkingLevel for named Gemini 3 levels or a model-specific thinkingBudget for Gemini 2.5, and it cannot confirm the applied value from non-interactive output.
contract_checked: 2026-09-29
---

## Levels

Gemini CLI 0.61.0's bundled `ThinkingLevel` enum has two usable named choices, `LOW` and `HIGH`, ordered from weaker to stronger reasoning. The enum also contains `THINKING_LEVEL_UNSPECIFIED`, a sentinel meaning no value was selected; it is not an effort level. The API currently documents other tokens for newer model versions, but this installed CLI bundle does not expose those tokens in its enum. Gemini 2.5 uses a numeric `thinkingBudget` rather than these named levels. (Installed enum: `local-thinking-enum-defaults`; [Gemini API thinking guide](https://ai.google.dev/gemini-api/docs/generate-content/thinking))

## Choosing a Level

The direct control lives under `modelConfigs` in Gemini CLI settings. A Gemini 3 alias can set `thinkingConfig.thinkingLevel`; a Gemini 2.5 alias can instead set a numeric `thinkingConfig.thinkingBudget`.

I inspected `~/.gemini/settings.json`, `~/.gemini/config/config.json`, and this workspace's `.gemini/settings.json`; none contains a custom reasoning level or thinking budget. The project settings file is absent. (Evidence: `local-settings`.)

```json
{
  "modelConfigs": {
    "customAliases": {
      "careful": {
        "extends": "chat-base-3",
        "modelConfig": {
          "model": "gemini-3-flash-preview",
          "generateContentConfig": {
            "thinkingConfig": { "thinkingLevel": "HIGH" }
          }
        }
      }
    }
  }
}
```

Select that preset at launch with `gemini --model careful --prompt "Review this change."`. The short form is `-m`. To set a Gemini 2.5 budget, replace `thinkingLevel` with `thinkingBudget`, for example `{ "thinkingConfig": { "thinkingBudget": 4096 } }`. `modelConfigs.overrides` and `modelConfigs.customOverrides` can set the same `generateContentConfig` conditionally by model or override scope. [Advanced model configuration](https://geminicli.com/docs/cli/generation-settings/)

Gemini CLI has no direct effort flag, config-override flag, reasoning environment variable, model-name suffix, or dedicated slash command. `--model` / `-m`, `GEMINI_MODEL`, and `model.name` in settings select a model or alias and therefore can select a preset indirectly. In an interactive session, `/model set careful` also selects one; it changes the model, not a standalone effort value. There is no separate caller-supplied effort field on a Gemini CLI request. [CLI help and inspected settings](local-version-help) · [Model selection](https://geminicli.com/docs/cli/model-routing/) · [Commands](https://geminicli.com/docs/reference/commands/)

For a configured alias, Gemini resolves its inheritance chain and then applies matching overrides. More specific matches outrank less specific matches; for ties, later entries win. Settings-file values follow the documented order defaults → system defaults → user → project → system settings → environment → CLI arguments. Model selection itself follows `/model` after launch, then `--model` / `-m`, `GEMINI_MODEL`, and `model.name`. These rules govern different layers, so a conflict spanning model selection, file layers, and matching overrides does not have one total precedence order. (Resolver: `local-model-config-resolver`; [settings precedence](https://geminicli.com/docs/reference/configuration/))

## Models

The CLI's built-in `gemini-2.5-*` chat aliases use `thinkingBudget: 8192` and do not use a named `thinkingLevel`; this includes `gemini-2.5-pro`, `gemini-2.5-flash`, and `gemini-2.5-flash-lite`. Other built-in Gemini 3 chat aliases use `HIGH`, which is the provider's default named setting for those aliases. A numeric budget is not interchangeable with one of the named level tokens. (Installed defaults: `local-thinking-enum-defaults`.)

The precise named-level acceptance for each current model, especially Gemini 2.5, remains unverified against this CLI version. API support varies by model, and the latest API matrix is not a version-pinned guarantee for CLI 0.61.0. ([API model matrix](https://ai.google.dev/gemini-api/docs/thinking))

## Confirming the Level

Gemini CLI does not report the actual `thinkingLevel` or `thinkingBudget` in non-interactive output, and the inspected session records contain no applied-level field. The usage statistic `thoughts_token_count` reports a count, not the setting that produced it. There is no confirmed post-run check for the applied level. (Evidence: `local-output-filter`.)

## Sources

- Installed Gemini CLI 0.61.0: `gemini --version` and `gemini --help`.
- Installed user settings: `~/.gemini/settings.json`, `~/.gemini/config/config.json`; this workspace's `.gemini/settings.json` is absent.
- Installed bundle: `@google/gemini-cli/bundle/chunk-JDPZ4CE3.js` (`ThinkingLevel`, `DEFAULT_MODEL_CONFIGS`, `ModelConfigService`, and non-interactive response formatters).
- [Gemini CLI configuration](https://geminicli.com/docs/reference/configuration/)
- [Gemini CLI advanced model configuration](https://geminicli.com/docs/cli/generation-settings/)
- [Gemini CLI model routing](https://geminicli.com/docs/cli/model-routing/)
- [Gemini CLI commands](https://geminicli.com/docs/reference/commands/)
- [Gemini API thinking for GenerateContent](https://ai.google.dev/gemini-api/docs/generate-content/thinking)
- [Gemini API thinking model matrix](https://ai.google.dev/gemini-api/docs/thinking)
- Invalid-level response from the disposable CLI probe: `/var/folders/l9/xdcp3xnn6s78_5l9w2_mnvtw0000gn/T/gemini-client-error-Turn.run-sendMessageStream-2026-09-29T09-42-41-629Z.json`.

## Changelog

- Replaced the revision 1 note with revision 2 contract fields and updated the source checks against installed Gemini CLI 0.61.0.
- Separated named thinking levels from numeric Gemini 2.5 budgets, documented indirect model selection and precedence layers, and recorded the remaining model/default verification gaps.
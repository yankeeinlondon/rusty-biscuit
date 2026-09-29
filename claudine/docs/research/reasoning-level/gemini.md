---
$schema: ./_schema.yaml
schema_revision: 1
provider: gemini
created: 2026-09-29
last_updated: 2026-09-29
agent: codex
model: gpt-6-luna
reasoning_effort: high
versions_examined:
- 0.61.0
evidence:
- claim: The installed CLI reports version 0.61.0. Its help has --model, -p/--prompt, -o/--output-format, and the other listed flags, but no reasoning-effort flag or settings-override flag.
  id: local-version-help
  limitations: This establishes the installed macOS CLI help only.
  location: gemini --version; gemini --help
  method: local_inspection
  observed_on: 2026-09-29
  version: 0.61.0
- claim: The inspected user settings contain no reasoning or thinking-level setting; the separate config.json contains only remoteControlHostname. The workspace has no project .gemini/settings.json.
  id: local-settings
  limitations: Other users, projects, or system-level settings files can set model configuration.
  location: ~/.gemini/settings.json; ~/.gemini/config/config.json; project .gemini/settings.json absent in this workspace
  method: local_inspection
  observed_on: 2026-09-29
  version: 0.61.0
- claim: There is no dedicated CLI flag, environment variable, model-name suffix, slash command, or request option for setting reasoning effort; model config is the available control surface.
  id: local-control-search
  limitations: A future Gemini CLI version may add controls. This search concerns 0.61.0.
  location: gemini --help; installed Gemini CLI settings definitions and slash-command documentation; environment-variable name search in installed CLI docs/source
  method: local_inspection
  observed_on: 2026-09-29
  version: 0.61.0
- claim: 'Built-in chat aliases set includeThoughts: true, use thinkingBudget: 8192 for Gemini 2.5 chat aliases, and use thinkingLevel: HIGH for Gemini 3 chat aliases, including gemini-3.8-flash. The bundled ThinkingLevel enum contains THINKING_LEVEL_UNSPECIFIED, LOW, and HIGH.'
  id: local-default-model-configs
  limitations: This is the shipped default alias configuration and SDK enum, not a successful live request for every model.
  location: /Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/chunk-5FZXKDXH.js (DEFAULT_MODEL_CONFIGS)
  method: source_code
  observed_on: 2026-09-29
  version: 0.61.0
- claim: Gemini CLI model generation is configured under modelConfigs using aliases and per-request overrides; generateContentConfig is passed to the provider with minimal validation, including thinkingConfig parameters.
  id: docs-generation-settings
  limitations: The page explains configuration mechanics but does not define the accepted thinking-level set per model.
  location: https://geminicli.com/docs/cli/generation-settings/
  method: official_docs
  observed_on: 2026-09-29
  version: 0.61.0
- claim: Settings-file precedence is defaults, system defaults, user, project, system settings, environment variables, and command-line arguments; system settings override other files. General environment and command-line precedence does not describe a dedicated thinking-level parameter.
  id: docs-config-precedence
  limitations: This is the current online documentation; local CLI help is the installed-version check.
  location: https://geminicli.com/docs/reference/configuration/
  method: official_docs
  observed_on: 2026-09-29
  version: 0.61.0
- claim: Built-in slash commands include /settings for editing settings and /model for selecting a model; neither is a dedicated reasoning-level command.
  id: docs-cli-commands
  limitations: This is current online documentation and may be newer than the installed CLI.
  location: https://geminicli.com/docs/reference/commands/
  method: official_docs
  observed_on: 2026-09-29
  version: 0.61.0
- claim: Gemini 3 generation accepts thinking levels LOW and HIGH through ThinkingLevel.LOW and ThinkingLevel.HIGH; Gemini 2.5 models use thinkingBudget rather than thinkingLevel. The documented ordering is low before high.
  id: docs-thinking-levels
  limitations: The documentation's model support and newer level names may change; 0.61.0's bundled enum is the version-specific token evidence used here.
  location: https://ai.google.dev/gemini-api/docs/generate-content/thinking
  method: official_docs
  observed_on: 2026-09-29
  version: 0.61.0
- claim: The current Gemini API model matrix gives model-specific level support and defaults, including exceptions for minimal and for low; Gemini 2.5 has a separate thinkingBudget control.
  id: docs-thinking-model-matrix
  limitations: This page also documents the newer Interactions API. It does not prove that Gemini CLI 0.61.0's generateContent integration forwards every newer level.
  location: https://ai.google.dev/gemini-api/docs/thinking
  method: official_docs
  observed_on: 2026-09-29
  version: 0.61.0
- claim: The documented built-in chat model configs use thinkingBudget 8192 for Gemini 2.5 and thinkingLevel HIGH for Gemini 3.
  id: docs-cli-model-config-defaults
  limitations: The API itself may apply model defaults outside Gemini CLI's named aliases.
  location: https://geminicli.com/docs/reference/configuration/
  method: official_docs
  observed_on: 2026-09-29
  version: 0.61.0
- claim: With an isolated project settings file, the CLI sent BOGUS as generation_config.thinking_config.thinking_level; the API returned HTTP 400 INVALID_ARGUMENT ('Invalid value at ... thinking_level ... BOGUS'), and the process exited 144.
  id: test-invalid-thinking-level
  limitations: This tests an invalid enum value on gemini-3.8-flash with the current API-key account; it does not test a valid value or a valid level unsupported by a particular model.
  location: 'gemini -p ''Reply only with OK.'' -m reasoning-probe -o json; disposable project alias configured thinkingLevel: BOGUS'
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.61.0
- claim: The non-interactive response text helper filters parts marked as thought, and its JSON/stream-json formatter emits content and result records without a thought record. Thought-token counts may appear in usage statistics.
  id: source-noninteractive-output
  limitations: Interactive UI has separate inline-thinking display behavior; this claim concerns non-interactive CLI output.
  location: /Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/chunk-JDPZ4CE3.js and bundle/gemini-3HFX2LNT.js (response processing and non-interactive formatters)
  method: source_code
  observed_on: 2026-09-29
  version: 0.61.0
support: some_models
levels:
- evidence_ids:
  - local-default-model-configs
  - docs-thinking-levels
  meaning: The lowest bundled Gemini 3 thinking level; reduces reasoning effort relative to HIGH.
  native: LOW
  normalized: low
- evidence_ids:
  - local-default-model-configs
  - docs-thinking-levels
  meaning: The highest bundled Gemini 3 thinking level; the built-in Gemini 3 chat aliases request it.
  native: HIGH
  normalized: high
default_level:
  decided_by: model
  evidence_ids:
  - local-default-model-configs
  - docs-cli-model-config-defaults
  native: HIGH
controls:
- arguments: []
  changes_running_session: no
  evidence_ids:
  - local-default-model-configs
  - docs-generation-settings
  id: thinking-level
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: thinkingLevel
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - local-default-model-configs
  - docs-thinking-levels
  - docs-generation-settings
  id: thinking-budget
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: thinkingBudget
  value: token_budget
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-generation-settings
  - local-default-model-configs
  id: model-config-alias
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
  - docs-generation-settings
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
  - local-default-model-configs
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
  - docs-generation-settings
  id: model-alias-launch
  kind: launch_flag
  lasts: one_session
  launch_modes:
  - interactive
  - non_interactive
  name: --model
  value: level_token
precedence:
- model-config-overrides
- model-config-custom-overrides
- model-config-alias
models:
- accepts: []
  evidence_ids:
  - local-default-model-configs
  - docs-thinking-levels
  model: gemini-2.5-pro
- accepts: []
  evidence_ids:
  - local-default-model-configs
  - docs-thinking-levels
  model: gemini-2.5-flash
- accepts: []
  evidence_ids:
  - local-default-model-configs
  - docs-thinking-levels
  model: gemini-2.5-flash-lite
invalid_level:
  behavior: fails_the_request
  evidence_ids:
  - test-invalid-thinking-level
  message: Invalid value at 'generation_config.thinking_config.thinking_level' (type.googleapis.com/google.ai.generativelanguage.v1beta.ThinkingConfig.ThinkingLevel), "BOGUS"
  warns: yes
reporting:
  evidence_ids:
  - source-noninteractive-output
  - local-default-model-configs
  notes: The non-interactive output and inspected response/session fields do not report the thinkingLevel or thinkingBudget applied to a turn. Usage can report thoughts_token_count, which measures generated thought tokens but does not identify the selected level.
  source: nowhere
reasoning_output:
  evidence_ids:
  - local-default-model-configs
  - source-noninteractive-output
  reaches_caller: hidden
gaps:
- detail: Requests using valid LOW and HIGH values did not produce a response in the disposable session before its timeout, so live acceptance was not checked. The latest API documentation also lists levels beyond the enum bundled with CLI 0.61.0.
  next_check: Retry isolated gemini-3.8-flash requests with LOW and HIGH in a disposable session, then inspect the sent request or a successful response; check whether other documented levels are accepted by this CLI version.
  subject: levels
- detail: The built-in Gemini 2.5 aliases use thinkingBudget 8192, while Gemini 3 aliases use HIGH; there is no single native default level across both model families. The account/provider default outside these aliases was not independently measured.
  next_check: Run one successful no-override request for a Gemini 2.5 model and one Gemini 3 model on a disposable profile, inspect each recorded request, and compare across an account with different model access.
  subject: default-level
- detail: Documentation establishes alias resolution before conditional overrides, but precedence among all competing user, project, system, extension, and model override values was not exercised for thinkingConfig.
  next_check: Create a disposable configuration with conflicting thinkingConfig values at each layer and matching modelConfigs.overrides, then inspect the final request payload.
  subject: precedence
- detail: No per-turn applied-level field was found in the inspected non-interactive formatters. Whether Gemini CLI exposes the full request through its debug response file or telemetry is not confirmed.
  next_check: Inspect a successful disposable run's debug response and telemetry records for a request field that records the applied thinking level.
  subject: reporting
changes:
- Initial research for Gemini CLI 0.61.0.
requires_claudine_update: true
reason: Gemini CLI selects reasoning through model configuration and aliases, has no dedicated effort flag or environment variable, has model-family-specific controls and defaults, and does not report the applied level in non-interactive output.
contract_checked: 2026-09-29
---

## Levels

Gemini CLI 0.61.0 bundles two named Gemini 3 thinking-level enum values: `LOW` and `HIGH`. `LOW` is the lower-effort setting; `HIGH` requests deeper reasoning. The API describes these as minimizing latency and cost versus maximizing reasoning depth.

The Gemini 2.5 built-in chat aliases use a numeric `thinkingBudget` instead of a named level. The shipped alias sets it to `8192`; Gemini 2.5's API supports values within model-specific ranges, `0` to disable thinking on most 2.5 models, and `-1` for dynamic thinking. Gemini 2.5 Pro cannot disable thinking.

The newer online Gemini API documentation lists `minimal` and `medium` for some models, but the enum bundled in this CLI version only exposes `LOW` and `HIGH`. Their acceptance through a raw CLI model config was not confirmed.

## Choosing a Level

There is no direct `--effort` or `--thinking-level` flag, no dedicated configuration override flag, no reasoning environment variable, and no model-name suffix for effort. `gemini --help` lists no such launch option. The CLI supports effort indirectly through its model configuration:

```json
{
  "modelConfigs": {
    "customAliases": {
      "careful": {
        "extends": "gemini-3.8-flash",
        "modelConfig": {
          "generateContentConfig": {
            "thinkingConfig": { "thinkingLevel": "HIGH" }
          }
        }
      }
    }
  }
}
```

Put this in a Gemini CLI settings file, then select the alias when launching:

```sh
gemini --model careful --prompt "Review this change for edge cases."
```

`--model` (or `-m`) selects a model or configured alias; it does not take a reasoning level directly. `modelConfigs.aliases` / `modelConfigs.customAliases` define reusable model configurations. `modelConfigs.overrides` applies `generateContentConfig` settings conditionally to matching model requests. Within a model config, use `thinkingConfig.thinkingLevel` for Gemini 3 or `thinkingConfig.thinkingBudget` for Gemini 2.5. For example, set the latter to `4096` to request a bounded Gemini 2.5 budget.

The CLI documents ordinary settings-file layers as defaults, user, project, then system settings, with environment variables and command-line arguments above files for settings they control. For model configs specifically, it resolves the selected alias first, then applies matching overrides; more specific overrides take priority, and the last equally specific matching override wins. No environment variable directly sets a thinking level. `/settings` opens a settings editor, but there is no dedicated in-session reasoning command; `/model` changes the selected model. No request field can be supplied separately from model configuration through the CLI interface.

## Models

The built-in Gemini 3 chat aliases inherit `thinkingLevel: HIGH`; Gemini 2.5 chat aliases inherit `thinkingBudget: 8192`. Gemini 2.5 does not accept the named `thinkingLevel` field. The following built-in models therefore differ from the named-level set above:

| Model | Named levels | Built-in setting |
| --- | --- | --- |
| `gemini-2.5-pro` | None | `thinkingBudget: 8192` |
| `gemini-2.5-flash` | None | `thinkingBudget: 8192` |
| `gemini-2.5-flash-lite` | None | `thinkingBudget: 8192` |

Gemini API support can vary by model and is updated independently. The current API model matrix documents additional level variants, but this research does not treat those as confirmed CLI 0.61.0 settings values.

## Confirming the Level

Gemini CLI does not report the applied `thinkingLevel` or `thinkingBudget` in its non-interactive `json` or `stream-json` output, and the local response/session structure inspected does not include the selected level. Usage statistics may include `thoughts_token_count`; this tells how many thinking tokens were generated, not which level was requested or accepted. There is no confirmed way to verify the actual level after a run.

## Sources

- Installed CLI version and help: `gemini --version`, `gemini --help` (Gemini CLI 0.61.0).
- Installed configuration inspection: `~/.gemini/settings.json`, `~/.gemini/config/config.json`, and this workspace's absent `.gemini/settings.json`.
- Installed package defaults and SDK enum: `@google/gemini-cli/bundle/chunk-5FZXKDXH.js`, `DEFAULT_MODEL_CONFIGS` and `ThinkingLevel`.
- Installed non-interactive response handling: `@google/gemini-cli/bundle/chunk-JDPZ4CE3.js`, `getResponseText`, thought event handling; `bundle/gemini-3HFX2LNT.js`, JSON and stream JSON formatting.
- [Gemini CLI Advanced Model Configuration](https://geminicli.com/docs/cli/generation-settings/)
- [Gemini CLI Configuration](https://geminicli.com/docs/reference/configuration/)
- [Gemini CLI Commands](https://geminicli.com/docs/reference/commands/)
- [Gemini API Thinking for Generate Content](https://ai.google.dev/gemini-api/docs/generate-content/thinking)
- [Gemini API Thinking model matrix](https://ai.google.dev/gemini-api/docs/thinking)
- Disposable invalid-level request: isolated project alias with `thinkingLevel: BOGUS`; command and exact API error are recorded in `test-invalid-thinking-level`.

## Changelog

- Initial research, 2026-09-29.
---
$schema: ./_schema.yaml
schema_revision: 1
provider: codex
created: 2026-09-29
last_updated: 2026-09-29
agent: claude
model: sonnet
reasoning_effort: high
versions_examined:
- 0.157.1
evidence:
- claim: codex --version prints codex-cli 0.157.1. Neither codex nor codex exec has a dedicated reasoning flag; -c/--config <key=value> overrides any config.toml key with a TOML-parsed value, and -p/--profile layers $CODEX_HOME/<name>.config.toml over the base user config.
  id: local-help
  limitations: Help text does not list the config keys that -c accepts.
  location: codex --version, codex --help, codex exec --help on this host (npm global install under ~/.nvm/versions/node/v22.20.0)
  method: local_inspection
  observed_on: 2026-09-29
  version: 0.157.1
- claim: Each model entry carries supported_reasoning_levels (effort tokens low, medium, high, xhigh, max, ultra with provider descriptions) and default_reasoning_level. Live catalog on this account lists gpt-6-astra, gpt-6-sol (low to ultra, default medium), gpt-6-luna and gpt-reserve (low to max, default medium), gpt-5.6-sol (low to ultra, default low), gpt-5.6-terra (low to ultra, default medium), gpt-5.6-luna and codex-auto-review (low to max, default medium), gpt-5.5 (low to xhigh, default medium).
  id: local-catalog
  limitations: The live catalog is served per account and version and changed between the bundled and live dumps (gpt-6-astra default is low bundled, medium live), so it is a snapshot, not a stable table.
  location: codex debug models (live catalog) and codex debug models --bundled (catalog compiled into the binary), run on this host
  method: local_inspection
  observed_on: 2026-09-29
  version: 0.157.1
- claim: 'An unknown token is echoed in the startup banner as ''reasoning effort: bogus'', sent to the API, and refused with HTTP 400 invalid_request_error: [ReasoningEffortParam] [reasoning.effort] [invalid_enum_value] Invalid value: ''bogus''. Supported values are: ''none'', ''minimal'', ''low'', ''medium'', ''high'', ''xhigh'', and ''max''. The process exits 1 and the model never runs.'
  id: test-invalid-token
  limitations: Only one unknown token was tried, and only against gpt-6-luna.
  location: codex exec --ephemeral --skip-git-repo-check -c model_reasoning_effort=bogus -m gpt-6-luna "say hi" (run from /tmp, exit code 1)
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.157.1
- claim: 'A valid token the model does not accept is not clamped locally. The event stream ends with turn.failed carrying HTTP 400 unsupported_value: Unsupported value: ''max'' is not supported with the ''gpt-5.5'' model. Supported values are: ''none'', ''low'', ''medium'', ''high'', and ''xhigh''.'
  id: test-unsupported-for-model
  limitations: Shows the server list for gpt-5.5 only; the server-side list for other models was not probed.
  location: codex exec --ephemeral --skip-git-repo-check --ignore-user-config --json -c model_reasoning_effort=max -m gpt-5.5 "say hi"
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.157.1
- claim: Each session's turn_context record holds the requested level at /payload/effort ('ultra', 'xhigh', 'none') and the same value under /payload/collaboration_mode/settings/reasoning_effort. With no level chosen, /payload/effort is absent and collaboration_mode.settings.reasoning_effort is null. All four runs completed. 'ultra' and 'none' are accepted on gpt-6-luna even though the catalog lists neither.
  id: test-session-records
  limitations: 'The record holds the client-side selection, not the value on the wire: ultra is recorded as ultra although source resolves it to max for this model. The unset case does not record the catalog default that was applied.'
  location: Session files ~/.codex/sessions/2026/09/29/rollout-2026-09-29T02-42-55-*.jsonl through rollout-2026-09-29T02-43-03-*.jsonl, produced by codex exec --skip-git-repo-check --ignore-user-config -m gpt-6-luna with no level, -c model_reasoning_effort=ultra, =xhigh, and =none
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.157.1
- claim: 'The user''s config.toml sets model_reasoning_effort = "medium"; without -c the startup banner prints ''reasoning effort: medium'', and with -c model_reasoning_effort=<x> it prints <x>, so the -c override outranks the user config file.'
  id: test-precedence
  limitations: Only the flag-versus-user-file pair was exercised. Project and profile layers are known from source only.
  location: codex exec --ephemeral --skip-git-repo-check -c model_reasoning_effort=bogus -m gpt-6-luna (user config.toml loaded), versus codex exec --skip-git-repo-check "say hi" (no -c)
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.157.1
- claim: 'With no level configured the banner prints ''reasoning effort: none'' for all three models. That is the banner''s rendering of an unset option, not the level none: the session record for the same case has no effort, and passing -c model_reasoning_effort=none produces the identical banner.'
  id: test-default-banner
  limitations: The banner cannot distinguish an unset level from the level none, so it is not a reliable report.
  location: codex exec --ephemeral --skip-git-repo-check --ignore-user-config -m gpt-6-luna|gpt-6-sol|gpt-5.5 "say hi"
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.157.1
- claim: On gpt-5.5 the JSONL stream contains {"type":"item.completed","item":{"type":"reasoning","text":"**Planning simple arithmetic solution**"}}, a summary, and turn.completed.usage reports reasoning_output_tokens. On gpt-6-luna no reasoning item appeared and only the token count (29) was reported. The --json stream carries no field naming the effort used.
  id: test-reasoning-output
  limitations: One prompt per model; whether gpt-6-luna can ever emit a summary was not established.
  location: codex exec --ephemeral --skip-git-repo-check --ignore-user-config --json -c model_reasoning_effort=high -c model_reasoning_summary=detailed -m gpt-5.5 and -m gpt-6-luna, prompt "What is 17*23? Think it through."
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.157.1
- claim: The client recognizes the tokens none, minimal, low, medium, high, xhigh, max, ultra, and persistent; any other non-empty string is kept as a model-defined Custom value and sent as is; only the empty string is rejected locally.
  id: source-effort-enum
  limitations: Source is upstream main on 2026-09-29, newer than the installed 0.157.1; the observed banner and refusal behavior match it.
  location: https://github.com/openai/codex/blob/c248f6d48b97eb4a2aa56147a0b11b7d763278b9/codex-rs/protocol/src/openai_models.rs (enum ReasoningEffort and its FromStr)
  method: source_code
  observed_on: 2026-09-29
  version: 0.157.1
- claim: The effort used is the configured level, else the model's default_reasoning_level from the catalog. Before an ordinary request, ultra is resolved to the model's multi_agent_reasoning_effort when that is a supported non-ultra level, else to max when the model lists it, else the model's highest non-ultra level, else medium. persistent is sent on the wire as disabled.
  id: source-effort-resolution
  limitations: Read from upstream main; the wire value was not captured for the ultra run.
  location: https://github.com/openai/codex/blob/c248f6d48b97eb4a2aa56147a0b11b7d763278b9/codex-rs/protocol/src/openai_models/reasoning_effort.rs and codex-rs/core/src/session/step_settings.rs (effective_reasoning_effort)
  method: source_code
  observed_on: 2026-09-29
  version: 0.157.1
- claim: 'A layer with a higher number overrides a lower one: packaged defaults -10, MDM 0, system 10, enterprise-managed 15, user config.toml 20, user profile file 21, project config 25, session flags (-c) 30, legacy managed config file 40, legacy managed config from MDM 50.'
  id: source-config-layers
  limitations: Upstream main, not the installed 0.157.1; only the flag-over-user-file pair was confirmed by running the binary.
  location: https://github.com/openai/codex/blob/c248f6d48b97eb4a2aa56147a0b11b7d763278b9/codex-rs/config/src/config_layer_source.rs (ConfigLayerSource::precedence)
  method: source_code
  observed_on: 2026-09-29
  version: 0.157.1
- claim: The app-server turn/start request takes an optional effort field documented as 'Override the reasoning effort for this turn and subsequent turns'; an experimental collaborationMode takes precedence over it.
  id: source-app-server-effort
  limitations: Not exercised against a running app server.
  location: https://github.com/openai/codex/blob/c248f6d48b97eb4a2aa56147a0b11b7d763278b9/codex-rs/app-server-protocol/src/protocol/v2/turn.rs (TurnStartParams.effort)
  method: source_code
  observed_on: 2026-09-29
  version: 0.157.1
- claim: The /model slash command is described as 'choose what model and reasoning effort to use'; its picker text reads 'Select Reasoning Level for <model>' and offers to 'Set the global default reasoning level and Plan mode override'. The TUI keymap has configurable actions tui.keymap.chat.increase_reasoning_effort and tui.keymap.chat.decrease_reasoning_effort.
  id: source-tui-controls
  limitations: The picker and key actions were not driven interactively, so default key bindings and whether a pick persists to config.toml are not confirmed.
  location: https://github.com/openai/codex/blob/c248f6d48b97eb4a2aa56147a0b11b7d763278b9/codex-rs/tui/src/slash_command.rs and codex-rs/tui/src/keymap.rs; strings in the installed 0.157.1 binary
  method: source_code
  observed_on: 2026-09-29
  version: 0.157.1
- claim: 'model_reasoning_effort is a string: ''Reasoning effort advertised by the selected model, such as low, medium, high, xhigh, max, or ultra. Available levels depend on the model and client.'' plan_mode_reasoning_effort overrides it for Plan mode, agents.default_subagent_reasoning_effort sets spawned agents'' effort, model_reasoning_summary takes auto, concise, detailed, or none, and hide_agent_reasoning suppresses reasoning events in the TUI and codex exec output.'
  id: docs-config-reference
  limitations: Page was read through a summarizing fetch and is not versioned; it omits none and minimal, which the client and API accept.
  location: https://developers.openai.com/codex/config-reference (redirects to https://learn.chatgpt.com/docs/config-file/config-reference)
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: The documentation describes /model as switching models and documents no dedicated reasoning-effort flag, slash command, or shortcut.
  id: docs-developer-commands
  limitations: The fetched content was truncated, so absence of detail is not proof of absence in the product.
  location: https://developers.openai.com/codex/cli/slash-commands (redirects to https://learn.chatgpt.com/docs/developer-commands?surface=cli)
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: No environment variable naming effort or reasoning exists in the binary.
  id: local-env-search
  limitations: A string search cannot rule out a variable assembled at run time.
  location: strings on the installed 0.157.1 native binary, searched for CODEX_ variables containing EFFORT or REASONING
  method: local_inspection
  observed_on: 2026-09-29
  version: 0.157.1
support: some_models
levels:
- evidence_ids:
  - test-invalid-token
  - test-session-records
  - test-unsupported-for-model
  meaning: Accepted by the API and by the client, and not listed in the model catalog. The runs on gpt-6-luna and the gpt-5.5 error list confirm it; the provider gives no description.
  native: none
  normalized: off
- evidence_ids:
  - test-invalid-token
  - source-effort-enum
  meaning: Accepted by the client enum and named in the API's supported-values message; not in any catalog entry and not run to completion here.
  native: minimal
  normalized: minimal
- evidence_ids:
  - local-catalog
  - docs-config-reference
  meaning: Fast responses with lighter reasoning
  native: low
  normalized: low
- evidence_ids:
  - local-catalog
  - docs-config-reference
  meaning: Balances speed and reasoning depth for everyday tasks
  native: medium
  normalized: medium
- evidence_ids:
  - local-catalog
  - docs-config-reference
  meaning: Greater reasoning depth for complex problems
  native: high
  normalized: high
- evidence_ids:
  - local-catalog
  - test-session-records
  meaning: Extra high reasoning depth for complex problems
  native: xhigh
  normalized: very_high
- evidence_ids:
  - local-catalog
  - test-invalid-token
  meaning: Maximum reasoning depth for the hardest problems
  native: max
  normalized: maximum
- evidence_ids:
  - local-catalog
  - source-effort-resolution
  - test-session-records
  meaning: 'Maximum reasoning with automatic task delegation. A client-side level: the API does not accept it, so the client sends the model''s max (or highest non-ultra) level instead.'
  native: ultra
  normalized: maximum
default_level:
  decided_by: model
  evidence_ids:
  - local-catalog
  - source-effort-resolution
  - test-session-records
controls:
- arguments:
  - -c
  - model_reasoning_effort=<level>
  changes_running_session: no
  evidence_ids:
  - local-help
  - test-precedence
  - test-session-records
  id: config-override-flag
  kind: config_override_flag
  lasts: one_session
  launch_modes:
  - interactive
  - non_interactive
  name: --config
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-config-reference
  - test-precedence
  - source-config-layers
  id: config-file-key
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: model_reasoning_effort
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-config-reference
  id: plan-mode-config-key
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  name: plan_mode_reasoning_effort
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-config-reference
  id: subagent-config-key
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: agents.default_subagent_reasoning_effort
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - source-tui-controls
  id: model-command
  kind: session_command
  lasts: unknown
  launch_modes:
  - interactive
  name: /model
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - source-tui-controls
  id: reasoning-keymap-actions
  kind: session_command
  lasts: unknown
  launch_modes:
  - interactive
  name: tui.keymap.chat.increase_reasoning_effort
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - source-app-server-effort
  id: app-server-turn-effort
  kind: request_field
  lasts: until_changed
  launch_modes:
  - non_interactive
  name: turn/start.effort
  value: level_token
precedence:
- config-override-flag
- config-file-key
models:
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  default: medium
  evidence_ids:
  - local-catalog
  - test-session-records
  model: gpt-6-luna
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  - ultra
  default: medium
  evidence_ids:
  - local-catalog
  model: gpt-6-sol
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  - ultra
  default: medium
  evidence_ids:
  - local-catalog
  model: gpt-6-astra
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  default: medium
  evidence_ids:
  - local-catalog
  model: gpt-reserve
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  - ultra
  default: low
  evidence_ids:
  - local-catalog
  model: gpt-5.6-sol
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  - ultra
  default: medium
  evidence_ids:
  - local-catalog
  model: gpt-5.6-terra
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  default: medium
  evidence_ids:
  - local-catalog
  model: gpt-5.6-luna
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  default: medium
  evidence_ids:
  - local-catalog
  - test-unsupported-for-model
  model: gpt-5.5
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  default: medium
  evidence_ids:
  - local-catalog
  model: codex-auto-review
invalid_level:
  behavior: fails_the_request
  evidence_ids:
  - test-invalid-token
  - test-unsupported-for-model
  - source-effort-enum
  message: '[ReasoningEffortParam] [reasoning.effort] [invalid_enum_value] Invalid value: ''bogus''. Supported values are: ''none'', ''minimal'', ''low'', ''medium'', ''high'', ''xhigh'', and ''max''.'
  warns: yes
reporting:
  evidence_ids:
  - test-session-records
  - test-default-banner
  - test-reasoning-output
  field: /payload/effort
  locator: ~/.codex/sessions/<yyyy>/<mm>/<dd>/rollout-<timestamp>-<session-id>.jsonl
  notes: 'Read the record whose type is turn_context; the same value is under /payload/collaboration_mode/settings/reasoning_effort. The field is absent when no level was chosen, and it holds the level as requested, so ultra appears as ultra though the wire value is max. The startup banner line ''reasoning effort: <level>'' on stderr is a second source but prints ''none'' for an unset level. The codex exec --json stream does not name the effort.'
  source: session_record
reasoning_output:
  evidence_ids:
  - test-reasoning-output
  - docs-config-reference
  reaches_caller: summary
gaps:
- detail: 'The default is the model''s catalog default_reasoning_level, and the live catalog served to this account differs from the bundled one (gpt-6-astra: low bundled, medium live). Whether the difference comes from the account, a plan, or a server-side rollout is not known.'
  next_check: Run codex debug models on a second account or plan tier and compare default_reasoning_level per model.
  subject: default-level
- detail: The session record stores the requested level, not the wire value. For ultra the resolved value (expected max) was not captured, and an unset level records nothing, so the catalog default that was applied cannot be read back.
  next_check: Point a local HTTP proxy at the provider base URL (config key chatgpt_base_url or model_providers) and read reasoning.effort in the request body for the ultra and unset cases.
  subject: reporting
- detail: Whether a /model pick applies only to the session or is written to config.toml as the global default was not established; the picker text offers to set the global default.
  next_check: Drive codex in a scratch CODEX_HOME through tmux, pick a level with /model, and diff config.toml before and after.
  subject: model-command
- detail: The default key bindings for the increase and decrease reasoning actions were not established, and the actions were not exercised.
  next_check: Read the default keymap table in codex-rs/tui/src/keymap.rs, or run /keymap in an interactive session.
  subject: reasoning-keymap-actions
- detail: The order among project config, profile file, and user config comes from source on upstream main, not the installed binary. Where /model and the keymap actions sit in the order was not tested.
  next_check: Set model_reasoning_effort differently in a scratch CODEX_HOME config.toml, its <name>.config.toml profile, and a project .codex/config.toml, and read the turn_context record for each combination.
  subject: precedence
- detail: minimal is accepted by the client and named by the API, but no run with it was completed, and no catalog entry advertises it.
  next_check: Run codex exec -c model_reasoning_effort=minimal against gpt-5.5 and gpt-6-luna and read the exit code and turn_context record.
  subject: minimal
- detail: The client also recognizes persistent, which it sends as disabled. Its meaning and which models accept it are not documented.
  next_check: Run codex exec -c model_reasoning_effort=persistent against a model and read the outcome; look for the token in the catalog's supported_reasoning_levels.
  subject: persistent
- detail: Summaries appeared on gpt-5.5 but not on gpt-6-luna, whose default_reasoning_summary is none. Whether luna-family models can emit a summary under model_reasoning_summary=detailed is not established.
  next_check: Repeat the JSON run with several prompts on gpt-6-luna and gpt-6-sol and look for item type reasoning.
  subject: reasoning-output
- detail: No model-name suffix for effort is documented or present in the binary strings; this was checked in docs and source but no suffixed model name was submitted.
  next_check: Run codex exec -m gpt-6-luna:high and read whether the model is refused.
  subject: model-suffix
changes:
- First version of the reasoning-level research for Codex CLI 0.157.1.
requires_claudine_update: true
reason: Claudine passes Codex a one-off -c model_reasoning_effort=<level> in its contract adapter but has no shared effort setting. The server refuses a level the model does not list instead of clamping, ultra is a client alias for max, the default differs by model, and the level a run used is readable only from the session record, so a provider-neutral mapping needs per-model level tables and a session-record check.
contract_checked: 2026-09-29
---

## Levels

Codex CLI names a reasoning level an **effort**. The client understands `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max`, and `ultra`. Which of them a run can use depends on the model, not on the CLI.

| Level | Claudine scale | What it does |
| ----- | -------------- | ------------ |
| `none` | off | No reasoning. The API accepts it, but no catalog entry lists it. |
| `minimal` | minimal | Accepted by the client and the API. Not listed for any model. |
| `low` | low | Fast responses with lighter reasoning. |
| `medium` | medium | Balances speed and reasoning depth for everyday tasks. |
| `high` | high | Greater reasoning depth for complex problems. |
| `xhigh` | very_high | Extra high reasoning depth for complex problems. |
| `max` | maximum | Maximum reasoning depth for the hardest problems. |
| `ultra` | maximum | Maximum reasoning with automatic task delegation. |

`ultra` is not an API value. Before sending a request the client turns it into the model's `max` level (or its highest level below `ultra`), so on a model that never lists `ultra`, such as `gpt-6-luna`, it still runs and behaves like `max`.

The client also recognizes `persistent`, which it sends as `disabled`. It is not documented (see `gaps`).

When no level is chosen, the model's own default applies, and that differs per model (see Models).

## Choosing a Level

Codex CLI has no dedicated effort flag. Use the generic configuration override, which takes a TOML value:

```bash
# One non-interactive run
codex exec -c model_reasoning_effort=high "summarize this repository"

# Interactive session
codex -c model_reasoning_effort=xhigh
```

Every control, from the highest precedence to the lowest, is listed here.

1. **`-c model_reasoning_effort=<level>`** overrides the config file for one launch. It works in interactive and `exec` modes. It cannot change a session that has already started.
2. **`model_reasoning_effort`** in `~/.codex/config.toml` (also read from a project `.codex/config.toml` and from a profile file chosen with `-p <name>`, which loads `$CODEX_HOME/<name>.config.toml`). It lasts until edited.
3. **`plan_mode_reasoning_effort`** sets the level for Plan mode only. Unset, Plan mode uses its built-in preset.
4. **`agents.default_subagent_reasoning_effort`** sets the default level for spawned agents.
5. **`/model`** in an interactive session opens a picker titled "Select Reasoning Level for <model>". The TUI also has key actions `tui.keymap.chat.increase_reasoning_effort` and `tui.keymap.chat.decrease_reasoning_effort`. Both change the running session; whether the pick is saved is unconfirmed.
6. **`turn/start.effort`** on the app-server protocol overrides the level for that turn and later turns.

There is no environment variable and no model-name suffix for effort.

### Precedence

Config layers are ranked by number, and the higher one wins. Session flags (`-c`, 30) beat project config (25), which beats a profile file (21), which beats the user `config.toml` (20). Managed and system layers sit either side of that (system 10, enterprise-managed 15, legacy managed files 40 and 50). Only the flag-over-user-file pair was confirmed by running the binary; the rest is read from source. Session commands change a running session afterward, and their place in the order was not tested.

## Models

The catalog on this account (`codex debug models`) lists these models. Each model also has a default when nothing is chosen:

| Model | Levels listed | Default |
| ----- | ------------- | ------- |
| `gpt-6-sol` | low, medium, high, xhigh, max, ultra | medium |
| `gpt-6-astra` | low, medium, high, xhigh, max, ultra | medium (low in the bundled catalog) |
| `gpt-6-luna` | low, medium, high, xhigh, max; none also worked | medium |
| `gpt-reserve` | low, medium, high, xhigh, max | medium |
| `gpt-5.6-sol` | low, medium, high, xhigh, max, ultra | low |
| `gpt-5.6-terra` | low, medium, high, xhigh, max, ultra | medium |
| `gpt-5.6-luna` | low, medium, high, xhigh, max | medium |
| `gpt-5.5` | low, medium, high, xhigh; none also accepted | medium |
| `codex-auto-review` | low, medium, high, xhigh, max | medium |

The catalog is fetched per account and version, so re-read it with `codex debug models` rather than treating this table as fixed.

## Confirming the Level

The most reliable source is the session file that Codex writes for each run: `~/.codex/sessions/<yyyy>/<mm>/<dd>/rollout-<timestamp>-<session-id>.jsonl`. In the record whose `type` is `turn_context`, `/payload/effort` holds the level that was requested.

```bash
jq -c 'select(.type=="turn_context") | .payload.effort' ~/.codex/sessions/2026/09/29/rollout-*.jsonl
```

Three limits apply:

- With no level chosen the field is absent. The default that was applied is not recorded.
- `ultra` is recorded as `ultra`, although the request carries `max`.
- `codex exec --ephemeral` writes no session file.

The startup banner line `reasoning effort: <level>` on stderr also echoes the setting. It prints `none` when nothing was chosen, exactly as it does for the level `none`, so it cannot confirm anything. The `codex exec --json` stream does not name the effort.

### An invalid level

Codex does not check tokens locally. It sends whatever it is given and the API decides:

- An unknown token such as `bogus` is refused with HTTP 400 (`invalid_request_error`) and the process exits 1. The message lists `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max` as valid.
- A valid token the model does not accept, such as `max` on `gpt-5.5`, fails the same way with `unsupported_value` and names that model's list. There is no silent fallback.
- With `--json` the failure arrives as an `error` event followed by `turn.failed`.

### Reasoning text

A non-interactive caller can receive a **summary**, never the raw reasoning. With `-c model_reasoning_summary=detailed`, `gpt-5.5` emitted `{"type":"item.completed","item":{"type":"reasoning","text":"..."}}` in the `--json` stream. `gpt-6-luna` emitted none, and only `reasoning_output_tokens` showed in `turn.completed.usage`. `hide_agent_reasoning = true` suppresses reasoning events entirely.

## Sources

- [Codex configuration reference](https://developers.openai.com/codex/config-reference) (redirects to `learn.chatgpt.com`)
- [Codex developer commands](https://developers.openai.com/codex/cli/slash-commands)
- [`ReasoningEffort` enum and parser](https://github.com/openai/codex/blob/c248f6d48b97eb4a2aa56147a0b11b7d763278b9/codex-rs/protocol/src/openai_models.rs)
- [Effort resolution, including `ultra`](https://github.com/openai/codex/blob/c248f6d48b97eb4a2aa56147a0b11b7d763278b9/codex-rs/protocol/src/openai_models/reasoning_effort.rs)
- [Config layer precedence](https://github.com/openai/codex/blob/c248f6d48b97eb4a2aa56147a0b11b7d763278b9/codex-rs/config/src/config_layer_source.rs)
- [App-server `turn/start` parameters](https://github.com/openai/codex/blob/c248f6d48b97eb4a2aa56147a0b11b7d763278b9/codex-rs/app-server-protocol/src/protocol/v2/turn.rs)
- Local observation on codex-cli 0.157.1: `codex --help`, `codex exec --help`, `codex debug models`, and disposable `codex exec` runs on 2026-09-29.

## Changelog

- 2026-09-29: first version, written against codex-cli 0.157.1.
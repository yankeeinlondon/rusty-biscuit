---
$schema: ./_schema.yaml
schema_revision: 2
provider: codex
created: 2026-09-29
last_updated: 2026-09-29
agent: claude
model: sonnet
reasoning_effort: high
versions_examined:
- 0.157.1
evidence:
- claim: codex --version prints codex-cli 0.157.1. Neither codex nor codex exec has a dedicated reasoning flag. -c/--config <key=value> overrides any config.toml key with a TOML-parsed value (raw string when it is not valid TOML), and -p/--profile layers $CODEX_HOME/<name>.config.toml on top of the base user config.
  id: local-help
  limitations: Help text does not list the configuration keys that -c accepts.
  location: codex --version, codex --help, codex exec --help on this host (npm global install under ~/.nvm/versions/node/v22.20.0)
  method: local_inspection
  observed_on: 2026-09-29
  version: 0.157.1
- claim: Each model entry carries supported_reasoning_levels (tokens low, medium, high, xhigh, max, ultra with descriptions) and default_reasoning_level. The live catalog lists gpt-6-astra and gpt-6-sol (low to ultra, default medium), gpt-6-luna, gpt-reserve and codex-auto-review (low to max, default medium), gpt-5.6-sol (low to ultra, default low), gpt-5.6-terra (low to ultra, default medium), gpt-5.6-luna (low to max, default medium), and gpt-5.5 (low to xhigh, default medium). Descriptions are low "Fast responses with lighter reasoning", medium "Balances speed and reasoning depth for everyday tasks", high "Greater reasoning depth for complex problems", xhigh "Extra high reasoning depth for complex problems", max "Maximum reasoning depth for the hardest problems", ultra "Maximum reasoning with automatic task delegation". No catalog entry lists none or minimal.
  id: local-catalog
  limitations: The catalog is served per account and version and differs from the bundled one (gpt-6-astra default is low bundled, medium live; the bundled list also has hidden gpt-5.4 and two gpt-daybreak entries), so it is a snapshot, not a stable table.
  location: codex debug models (live catalog) and codex debug models --bundled (catalog compiled into the binary), run on this host
  method: local_inspection
  observed_on: 2026-09-29
  version: 0.157.1
- claim: 'An unknown token is sent to the API and refused with HTTP 400 invalid_request_error: [ReasoningEffortParam] [reasoning.effort] [invalid_enum_value] Invalid value: ''bogus''. Supported values are: ''none'', ''minimal'', ''low'', ''medium'', ''high'', ''xhigh'', and ''max''. With --json the stream is thread.started, turn.started, error, turn.failed; the model never runs. The session record is still written and holds effort bogus. An empty string is refused locally at config load with "Error loading config.toml: reasoning_effort must not be empty".'
  id: test-invalid-token
  limitations: Only one unknown token was tried, and only against gpt-6-luna.
  location: CODEX_HOME=<scratch dir with symlinked auth.json> codex exec --skip-git-repo-check --json -m gpt-6-luna -c model_reasoning_effort=bogus "say hi"; also -c 'model_reasoning_effort=""'
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.157.1
- claim: 'A recognized token the model does not accept is not clamped locally. The stream ends with error then turn.failed carrying HTTP 400 unsupported_value: Unsupported value: ''max'' is not supported with the ''gpt-5.5'' model. Supported values are: ''none'', ''low'', ''medium'', ''high'', and ''xhigh''.'
  id: test-unsupported-for-model
  limitations: Shows the refusal text for the levels tried, not for every model and level pair.
  location: same scratch CODEX_HOME; codex exec --skip-git-repo-check --json -m gpt-5.5 -c model_reasoning_effort=max "say hi"
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.157.1
- claim: 'none completes on gpt-6-sol, gpt-6-luna, gpt-5.6-sol, gpt-5.6-terra, gpt-5.6-luna, gpt-5.5, gpt-reserve, and codex-auto-review, and is refused on gpt-6-astra (Supported values are: low, medium, high, xhigh, and max). minimal is refused on all nine models with unsupported_value, even though the generic enum message names it. Without -c web_search=disabled, minimal on gpt-5.5 fails on a different message: "The following tools cannot be used with reasoning.effort ''minimal'': web_search."'
  id: test-none-minimal-matrix
  limitations: One trivial prompt per pair. Hidden and non-catalog models were not tried.
  location: same scratch CODEX_HOME; for each of the nine catalog models, codex exec --skip-git-repo-check --json -m <model> -c model_reasoning_effort=<none|minimal> -c web_search=disabled "say hi"
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.157.1
- claim: Each session's turn_context record holds the requested level at /payload/effort and the same value at /payload/collaboration_mode/settings/reasoning_effort. With no level chosen /payload/effort is absent and the collaboration_mode value is null. The record is written even when the request is later refused. ultra completes on gpt-6-luna, gpt-6-astra and gpt-5.5 although none of them lists it in the catalog. codex exec --ephemeral writes no session file. The --json stream events (thread.started, turn.started, item.completed, turn.completed) never name the effort.
  id: test-session-records
  limitations: The record holds the client-side selection, not the wire value; ultra is recorded as ultra although source resolves it to another level. The unset case does not record the catalog default that was applied.
  location: Session files under <scratch CODEX_HOME>/sessions/2026/09/29/rollout-*.jsonl produced by codex exec --skip-git-repo-check --json -m gpt-6-luna with no level, then -c model_reasoning_effort=ultra, xhigh, none, minimal, persistent, bogus, high
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.157.1
- claim: 'User file alone gives low. -p p (profile file) over the user file gives high. A project .codex/config.toml gives xhigh over the user file and over the profile file, but only once the trust entry was keyed to the canonical path (/private/tmp/... on macOS); keyed to /tmp/... the project layer was ignored. -c model_reasoning_effort=medium beats all three. Order observed: -c, then trusted project file, then profile file, then user file.'
  id: test-precedence
  limitations: Enterprise-managed, system, and MDM layers were not exercised. Session commands were not tested against the layers.
  location: Scratch CODEX_HOME with config.toml (model_reasoning_effort = "low" plus a trusted-project entry), p.config.toml (high), and <project>/.codex/config.toml (xhigh); codex exec --json -m gpt-6-luna with and without -p p and -c model_reasoning_effort=medium; result read from the turn_context record
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.157.1
- claim: 'The stderr banner prints "reasoning effort: none" both when no level is chosen and when none is chosen, and "reasoning effort: high" when high is chosen. It cannot tell an unset level from none.'
  id: test-default-banner
  limitations: Plain codex exec only; the --json stream has no banner.
  location: codex exec --skip-git-repo-check -m gpt-6-luna with no -c, and with -c model_reasoning_effort=none, in a scratch CODEX_HOME with no config.toml (stderr banner)
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.157.1
- claim: With model_reasoning_summary=detailed all three models emit {"type":"item.completed","item":{"type":"reasoning","text":"**...**"}} in the --json stream, a short bold heading and not raw reasoning. With model_reasoning_summary=none, and with no summary setting on gpt-6-luna, no reasoning item appears. hide_agent_reasoning=true removes the item even with detailed. turn.completed.usage.reasoning_output_tokens reports a count in every case. In plain codex exec the summary goes to stderr and the answer to stdout.
  id: test-reasoning-output
  limitations: One prompt per model. show_raw_agent_reasoning was not tried.
  location: same scratch CODEX_HOME; codex exec --skip-git-repo-check --json -m gpt-5.5|gpt-6-luna|gpt-6-sol -c model_reasoning_effort=high -c model_reasoning_summary=detailed|none "What is 17*23? Think it through step by step."; plus -c hide_agent_reasoning=true; plus plain codex exec on gpt-6-luna
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.157.1
- claim: A suffixed model name is treated as a different model id and refused ("The 'gpt-6-luna:high' model is not supported when using Codex with a ChatGPT account."), so no model-name suffix selects an effort.
  id: test-model-suffix
  limitations: Tried only on a ChatGPT-account login.
  location: codex exec --skip-git-repo-check --json -m gpt-6-luna:high "say hi" in the scratch CODEX_HOME
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.157.1
- claim: 'persistent is accepted by the client and recorded in the session, then sent as disabled and refused: Unsupported value: ''disabled'' is not supported for ''reasoning.effort''.'
  id: test-persistent
  limitations: Two models only.
  location: codex exec --skip-git-repo-check --json -m gpt-6-luna|gpt-6-sol -c model_reasoning_effort=persistent "say hi" in the scratch CODEX_HOME
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.157.1
- claim: ModelInfo::resolve_reasoning_effort turns ultra into the model's multi_agent_reasoning_effort when that is a listed non-ultra level, else max when the model lists it, else its highest listed non-ultra level, else medium. persistent becomes the custom value disabled ("the Responses API calls it disabled").
  id: source-effort-resolution
  limitations: Upstream commit c248f6d is newer than the installed 0.157.1; the wire value for ultra was not captured.
  location: https://github.com/openai/codex/blob/c248f6d48b97eb4a2aa56147a0b11b7d763278b9/codex-rs/protocol/src/openai_models/reasoning_effort.rs
  method: source_code
  observed_on: 2026-09-29
  version: 0.157.1
- claim: TurnStartParams has an optional effort property, "Override the reasoning effort for this turn and subsequent turns.", typed as a non-empty string ("A non-empty reasoning effort value advertised by the model.").
  id: local-app-server-schema
  limitations: Not exercised against a running app server.
  location: codex app-server generate-json-schema --out <dir>, file v2/TurnStartParams.json
  method: local_inspection
  observed_on: 2026-09-29
  version: 0.157.1
- claim: The binary contains "choose what model and reasoning effort to use" (the /model description), "Select Reasoning Level for", "Set the global default reasoning level and Plan mode override", the keymap actions tui.keymap.chat.increase_reasoning_effort and tui.keymap.chat.decrease_reasoning_effort, and the keys plan_mode_reasoning_effort and default_subagent_reasoning_effort. It contains no CODEX_ variable whose name includes EFFORT or REASONING.
  id: local-strings-tui
  limitations: A string search shows presence, not behavior; the picker and key actions were not driven, and a variable assembled at run time would not appear.
  location: strings on the installed 0.157.1 native binary (codex-darwin-arm64 vendor/aarch64-apple-darwin/bin/codex)
  method: local_inspection
  observed_on: 2026-09-29
  version: 0.157.1
- claim: model_reasoning_effort is "Reasoning effort advertised by the selected model, such as low, medium, high, xhigh, max, or ultra." plan_mode_reasoning_effort is a "Plan-mode-specific reasoning override using a level supported by the selected model." agents.default_subagent_reasoning_effort is the "Default reasoning effort for spawned agents. An explicit spawn effort takes precedence." model_reasoning_summary takes auto, concise, detailed, or none. hide_agent_reasoning suppresses reasoning events in the TUI and codex exec output, and show_raw_agent_reasoning surfaces raw reasoning when the model emits it. The page names no environment variable or command-line flag for effort.
  id: docs-config-reference
  limitations: Read through a summarizing fetch and not versioned. It omits none, which the API accepts.
  location: https://learn.chatgpt.com/docs/config-file/config-reference (permanent redirect from https://developers.openai.com/codex/config-reference)
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
support: some_models
levels:
- evidence_ids:
  - test-none-minimal-matrix
  - test-session-records
  meaning: No reasoning. The API accepts it on every catalog model except gpt-6-astra, though no catalog entry lists it. The provider gives no description.
  native: none
  normalized: off
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
  - test-unsupported-for-model
  meaning: Maximum reasoning depth for the hardest problems
  native: max
  normalized: maximum
- evidence_ids:
  - local-catalog
  - source-effort-resolution
  - test-session-records
  meaning: Maximum reasoning with automatic task delegation. A client-side alias, never sent as ultra; the client substitutes the model's max level (or its highest non-ultra level), so it runs on models that do not list it. Whether delegation actually happens was not tested.
  native: ultra
  normalized: outside_scale
default_level:
  decided_by: model
  evidence_ids:
  - local-catalog
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
  - test-precedence
  id: project-config-key
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
  - local-help
  - test-precedence
  id: profile-config-key
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
  - test-precedence
  id: user-config-key
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
  - local-strings-tui
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
  - local-strings-tui
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
  - local-strings-tui
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
  - local-strings-tui
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
  - local-app-server-schema
  id: app-server-turn-effort
  kind: request_field
  lasts: until_changed
  launch_modes:
  - non_interactive
  name: turn/start.effort
  value: level_token
precedence:
- config-override-flag
- project-config-key
- profile-config-key
- user-config-key
models:
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
  - test-none-minimal-matrix
  - test-session-records
  model: gpt-6-astra
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  - ultra
  default: medium
  evidence_ids:
  - local-catalog
  - test-none-minimal-matrix
  model: gpt-6-sol
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  - ultra
  default: medium
  evidence_ids:
  - local-catalog
  - test-none-minimal-matrix
  model: gpt-5.6-terra
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  - ultra
  default: low
  evidence_ids:
  - local-catalog
  - test-none-minimal-matrix
  model: gpt-5.6-sol
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - max
  - ultra
  default: medium
  evidence_ids:
  - local-catalog
  - test-none-minimal-matrix
  - test-session-records
  model: gpt-6-luna
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
  - test-none-minimal-matrix
  model: gpt-5.6-luna
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
  - test-none-minimal-matrix
  model: gpt-reserve
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
  - test-none-minimal-matrix
  model: codex-auto-review
- accepts:
  - none
  - low
  - medium
  - high
  - xhigh
  - ultra
  default: medium
  evidence_ids:
  - local-catalog
  - test-unsupported-for-model
  - test-none-minimal-matrix
  - test-session-records
  model: gpt-5.5
invalid_level:
  behavior: fails_the_request
  evidence_ids:
  - test-invalid-token
  - test-unsupported-for-model
  - test-none-minimal-matrix
  - test-persistent
  - test-model-suffix
  message: '[ReasoningEffortParam] [reasoning.effort] [invalid_enum_value] Invalid value: ''bogus''. Supported values are: ''none'', ''minimal'', ''low'', ''medium'', ''high'', ''xhigh'', and ''max''.'
  warns: yes
reporting:
  evidence_ids:
  - test-session-records
  - test-default-banner
  field: /payload/effort
  locator: ~/.codex/sessions/<yyyy>/<mm>/<dd>/rollout-<timestamp>-<session-id>.jsonl
  notes: 'Read the record whose type is turn_context; the same value is at /payload/collaboration_mode/settings/reasoning_effort. The value is the level as requested: ultra is recorded as ultra, a refused level is recorded too, and an unset level leaves the field absent (null under collaboration_mode), so the applied catalog default cannot be read back. codex exec --ephemeral writes no file. The stderr banner line "reasoning effort: <level>" of plain codex exec is a second source but prints none for an unset level, and the --json stream never names the effort.'
  source: session_record
reasoning_output:
  evidence_ids:
  - test-reasoning-output
  - docs-config-reference
  reaches_caller: summary
gaps:
- area: controls
  detail: Whether a level picked with /model lasts only for the session or is written to config.toml as the global default is not established; the picker text offers to "Set the global default reasoning level and Plan mode override".
  entry: model-command
  next_check: Drive codex through tmux in a scratch CODEX_HOME, pick a level with /model, and diff config.toml and the next session's turn_context record.
- area: controls
  detail: The default key bindings for the increase and decrease reasoning actions, and whether they persist, were not established; the actions were not exercised.
  entry: reasoning-keymap-actions
  next_check: Read the default keymap table in codex-rs/tui/src/keymap.rs, or run /keymap in an interactive session, then press the bound keys and read the turn_context record.
- area: controls
  detail: The turn/start effort field is known from the generated schema only; whether it is accepted, refused, or clamped on a model that does not list the level was not run.
  entry: app-server-turn-effort
  next_check: Start codex app-server in a scratch CODEX_HOME, send turn/start with effort set to a level the model refuses, and read the response.
- area: precedence
  detail: Only the four layers -c, trusted project file, profile file, and user file were run. Enterprise-managed, system, MDM, and legacy managed layers are not exercised, and the rank of the session commands against the layers is unknown.
  next_check: Read ConfigLayerSource::precedence at the installed version's tag, then place model_reasoning_effort in a system config.toml and a managed_config.toml and read the turn_context record.
- area: levels
  detail: The catalog describes ultra as adding automatic task delegation, but whether a run with ultra delegates was not tested, and ultra was run to completion only on gpt-6-luna, gpt-6-astra, and gpt-5.5. Which level it resolves to on the wire was not captured.
  entry: ultra
  next_check: Point a local HTTP proxy at the provider base URL (chatgpt_base_url or a model_providers entry), run -c model_reasoning_effort=ultra, and read reasoning.effort in the request body and whether spawn-agent tool calls occur.
- area: default_level
  detail: 'The default is the model''s catalog default_reasoning_level, and the live catalog differs from the bundled one (gpt-6-astra: low bundled, medium live). Whether the difference comes from the account, the plan, or a server rollout is not known, and the applied default is not recorded in the session.'
  next_check: Run codex debug models on a second account or plan tier and compare default_reasoning_level per model, then capture the request body for an unset run through a local proxy.
- area: models
  detail: ultra was not run against gpt-5.6-luna, gpt-reserve, codex-auto-review, gpt-6-sol, gpt-5.6-sol, or gpt-5.6-terra by an explicit test on the alias path, and levels the catalog lists were taken from the catalog and the API's supported-values messages, not run one by one.
  entry: gpt-5.6-luna
  next_check: Loop codex exec -c model_reasoning_effort=<level> over every catalog model and level and record completion or the unsupported_value message.
- area: reasoning_output
  detail: show_raw_agent_reasoning is documented as surfacing raw reasoning when the model emits it, but was not tried, and the effect of model_reasoning_summary=auto and concise was not compared with detailed.
  next_check: Run codex exec --json -c show_raw_agent_reasoning=true and -c model_reasoning_summary=auto on gpt-6-sol and inspect the reasoning items.
changes:
- Rewritten for schema_revision 2; the gap and precedence structures follow the new contract, and contract_checked is left to the fleet.
- minimal and persistent are no longer listed as accepted levels. The client parses both, but the API refused minimal on all nine catalog models and refused persistent (sent as disabled) on gpt-6-luna and gpt-6-sol.
- none is now recorded per model. It completes on eight catalog models and is refused only on gpt-6-astra, so gpt-6-astra no longer shares a row with the models that accept it.
- ultra is now outside_scale, following the catalog's "automatic task delegation" description; the earlier draft mapped it to maximum.
- Precedence now lists the project file and profile file, both confirmed by running the binary (order -c, trusted project, profile, user). A project file applies only when its directory is trusted under the canonical path.
- Corrected the reasoning-output finding. gpt-6-luna does emit a reasoning summary under model_reasoning_summary=detailed; the earlier draft found none because the summary setting defaults to none.
- Added the model-name-suffix check (refused), the empty-string check (refused at config load), and the confirmation that --ephemeral writes no session record.
- Added gpt-reserve and codex-auto-review to the model table from the live catalog and dropped the hidden gpt-5.4 and gpt-daybreak entries.
requires_claudine_update: true
reason: Claudine already passes Codex a one-off -c model_reasoning_effort=<level> through its contract adapter but has no shared effort setting. A provider-neutral mapping needs per-model level tables, because the API refuses (rather than clamps) a level a model does not accept and none differs by model. It also needs the session-record check, since ultra is a client alias for max and is not an ordinary point on the scale, and no other place states the level a run used.
contract_checked: 2026-09-29
---

## Levels

Codex CLI calls a reasoning level an **effort**. The tokens the API accepts are `none`, `low`, `medium`, `high`, `xhigh`, and `max`, plus the client-side alias `ultra`. Which of them a run can use depends on the model, not on the CLI.

| Level | Claudine scale | What it does |
| ----- | -------------- | ------------ |
| `none` | off | No reasoning. Accepted by every catalog model except `gpt-6-astra`, though no catalog entry lists it. |
| `low` | low | Fast responses with lighter reasoning. |
| `medium` | medium | Balances speed and reasoning depth for everyday tasks. |
| `high` | high | Greater reasoning depth for complex problems. |
| `xhigh` | very_high | Extra high reasoning depth for complex problems. |
| `max` | maximum | Maximum reasoning depth for the hardest problems. |
| `ultra` | outside_scale | Maximum reasoning with automatic task delegation. |

`ultra` is not an API value. Before sending a request the client turns it into the model's `max` level (or its highest level below `ultra`), so it runs even on a model that does not list it, such as `gpt-6-luna` and `gpt-5.5`. The catalog describes delegation, but this research did not test whether delegation happens.

Two more tokens are parsed by the client and still fail: `minimal` is named in the API's generic error text but was refused on all nine catalog models, and `persistent` is sent as `disabled` and refused. Do not offer either.

When no level is chosen, the model's own catalog default applies, and that differs per model (see Models).

## Choosing a Level

Codex CLI has no dedicated effort flag and no environment variable. Use the generic configuration override, which takes a TOML value:

```bash
# One non-interactive run
codex exec -c model_reasoning_effort=high "summarize this repository"

# Interactive session
codex -c model_reasoning_effort=xhigh
```

The controls:

1. **`-c model_reasoning_effort=<level>`** (`--config`) applies to one launch, in interactive and `exec` modes. It cannot change a session that has already started.
2. **`model_reasoning_effort`** in a project `.codex/config.toml`. It is read only when the project directory is trusted, and the trust entry must use the canonical path (on macOS `/private/tmp/...`, not `/tmp/...`).
3. **`model_reasoning_effort`** in a profile file, selected with `-p <name>`, which layers `$CODEX_HOME/<name>.config.toml` over the user config:

    ```bash
    codex exec -p fast "summarize this repository"
    ```

4. **`model_reasoning_effort`** in `~/.codex/config.toml`. It lasts until edited.
5. **`plan_mode_reasoning_effort`** overrides the level for Plan mode only.
6. **`agents.default_subagent_reasoning_effort`** sets the default level for spawned agents. An explicit spawn effort wins.
7. **`/model`** in an interactive session opens a picker titled "Select Reasoning Level for <model>". The TUI also has key actions `tui.keymap.chat.increase_reasoning_effort` and `tui.keymap.chat.decrease_reasoning_effort`. Both act on the running session. Whether a pick is saved as the global default was not established.
8. **`turn/start.effort`** on the app-server protocol overrides the level for that turn and later turns.

There is no model-name suffix: `-m gpt-6-luna:high` is refused as an unknown model.

### Precedence

For the four file and flag controls, the order was observed by running the binary with a different level in each: `-c` beats a trusted project file, which beats a profile file, which beats the user `config.toml`. A project file whose directory is not trusted is ignored. Managed and system layers, and the rank of the session commands, were not tested.

## Models

Every catalog model accepts the levels in its row, and the API refuses the rest. The live catalog (`codex debug models`) lists these models; each row is what was observed on 2026-09-29:

| Model | Accepts | Default |
| ----- | ------- | ------- |
| `gpt-6-astra` | low, medium, high, xhigh, max, ultra (not `none`) | medium (low in the bundled catalog) |
| `gpt-6-sol` | none, low, medium, high, xhigh, max, ultra | medium |
| `gpt-5.6-terra` | none, low, medium, high, xhigh, max, ultra | medium |
| `gpt-5.6-sol` | none, low, medium, high, xhigh, max, ultra | low |
| `gpt-6-luna` | none, low, medium, high, xhigh, max, ultra | medium |
| `gpt-5.6-luna` | none, low, medium, high, xhigh, max | medium |
| `gpt-reserve` | none, low, medium, high, xhigh, max | medium |
| `codex-auto-review` | none, low, medium, high, xhigh, max | medium |
| `gpt-5.5` | none, low, medium, high, xhigh, ultra (not `max`) | medium |

`ultra` on `gpt-5.6-luna`, `gpt-reserve`, and `codex-auto-review` was not run. The catalog is fetched per account and version, so re-read it with `codex debug models` rather than treating this table as fixed.

## Confirming the Level

The most reliable source is the session file Codex writes for each run: `~/.codex/sessions/<yyyy>/<mm>/<dd>/rollout-<timestamp>-<session-id>.jsonl` (under `$CODEX_HOME` when that is set). In the record whose `type` is `turn_context`, `/payload/effort` holds the level that was requested.

```bash
jq -c 'select(.type=="turn_context") | .payload.effort' ~/.codex/sessions/2026/09/29/rollout-*.jsonl
```

Four limits apply:

- With no level chosen the field is absent. The default that was applied is not recorded.
- `ultra` is recorded as `ultra`, although the request carries a different level.
- A refused level is recorded too, so a record proves what was requested, not that the request succeeded.
- `codex exec --ephemeral` writes no session file.

The `reasoning effort: <level>` line in the stderr banner of plain `codex exec` also echoes the setting. It prints `none` when nothing was chosen, exactly as for the level `none`, so it cannot confirm an unset level. The `codex exec --json` stream does not name the effort.

### An invalid level

Codex does not check tokens locally, except that an empty string is rejected at config load. It sends whatever it is given and the API decides:

- An unknown token such as `bogus` is refused with HTTP 400 (`invalid_request_error`), and the message lists `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, and `max`.
- A recognized token the model does not accept, such as `max` on `gpt-5.5` or `none` on `gpt-6-astra`, fails the same way with `unsupported_value` and names that model's list. There is no silent fallback.
- With `--json` the failure arrives as an `error` event followed by `turn.failed`.

### Reasoning text

A non-interactive caller can receive a **summary**, never the raw reasoning. It appears only when `model_reasoning_summary` is set to something other than `none` (the catalog default is `none`). With `-c model_reasoning_summary=detailed`, `gpt-5.5`, `gpt-6-luna`, and `gpt-6-sol` each emitted one `{"type":"item.completed","item":{"type":"reasoning","text":"**...**"}}` event in the `--json` stream, a short bold heading. In plain `codex exec` the summary goes to stderr and the answer to stdout. `hide_agent_reasoning = true` suppresses the event, and `turn.completed.usage.reasoning_output_tokens` reports the count either way.

## Sources

- [Codex configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference) (redirected from `developers.openai.com/codex/config-reference`)
- [Effort resolution, including `ultra` and `persistent`](https://github.com/openai/codex/blob/c248f6d48b97eb4a2aa56147a0b11b7d763278b9/codex-rs/protocol/src/openai_models/reasoning_effort.rs)
- Local observation on codex-cli 0.157.1 on 2026-09-29, in a scratch `CODEX_HOME`: `codex --help`, `codex exec --help`, `codex debug models` (live and `--bundled`), `codex app-server generate-json-schema`, `strings` on the native binary, and disposable `codex exec` runs.

## Changelog

- 2026-09-29: rewritten for contract revision 2 against codex-cli 0.157.1.
    - `minimal` and `persistent` removed from the accepted levels: the API refused both.
    - `none` recorded per model: accepted everywhere except `gpt-6-astra`.
    - `ultra` moved outside the scale.
    - Project and profile precedence layers confirmed by running the binary.
    - Reasoning-summary finding corrected: `gpt-6-luna` emits a summary when `model_reasoning_summary=detailed`.
    - Model-name suffix, empty-string, and `--ephemeral` checks added.
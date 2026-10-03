---
$schema: ./_schema.yaml
schema_revision: 2
provider: kimi
created: 2026-09-29
last_updated: 2026-09-29
agent: claude
model: sonnet
reasoning_effort: high
versions_examined:
- 2.0.2
- 2.1.1
evidence:
- claim: 'kimi --version printed 2.0.2. The help output lists no flag for reasoning, thinking, or effort; the only model-related launch flag is -m/--model <model>. kimi --thinking exits 1 with "error: unknown option ''--thinking''" and an --effort argument exits 1 with "error: unknown option" before any model call.'
  id: local-help-2-0-2
  limitations: Covers 2.0.2 only. The host binary auto-upgraded to 2.1.1 during this run (see local-help-2-1-1).
  location: kimi --version and kimi --help on this host (binary at /Users/ken/.kimi-code/bin/kimi)
  method: local_inspection
  observed_on: 2026-09-29
  version: 2.0.2
- claim: 'kimi --version printed 2.1.1. A search of the help text for thinking, effort, and reason found nothing. kimi --thinking exits 1 with "error: unknown option ''--thinking''" and kimi --effort=high exits 1 with "error: unknown option ''--effort=high''". The 2.1.1 help lists the same flags as 2.0.2 for this topic.'
  id: local-help-2-1-1
  limitations: Absence of a flag in help and in a rejected invocation; it does not rule out an undocumented subcommand.
  location: kimi --version and kimi --help on this host after the binary upgraded itself (binary at /Users/ken/.kimi-code/bin/kimi)
  method: local_inspection
  observed_on: 2026-09-29
  version: 2.1.1
- claim: The user config carries a [thinking] table (enabled = true, effort = "high") and four managed model aliases. kimi-code/k3, kimi-code/k3-256k, and kimi-code/kimi-for-coding declare support_efforts = ["low", "high", "max"] and default_effort ("high", "high", "max"); kimi-code/kimi-for-coding-highspeed declares no support_efforts. All four carry the capabilities thinking and always_thinking. default_model is kimi-code/k3. ~/.kimi-code/migrations-effort.json records a "thinking-effort-max-to-high" migration dated 2026-09-22.
  id: local-config-toml
  limitations: The managed model entries are refreshed from the service, so the lists reflect this account on this date. The file was last modified 2026-09-21, before the 2.1.1 upgrade.
  location: /Users/ken/.kimi-code/config.toml (credentials redacted; read only, never modified)
  method: local_inspection
  observed_on: 2026-09-29
  version: 2.0.2
- claim: Documents [thinking] enabled (boolean, default true, "set to false to force Thinking off"), effort (low / medium / high / xhigh / max, "falls back to the model default when not in its supported list"), and keep; the model fields support_efforts ("unsupported values fall back to default_effort, and out-of-list values fail"), default_effort, and off_effort; and [secondary_model].default_effort, which outranks the bound model's default_effort for subagents while for main agents a global [thinking].effort overrides the variant's default_effort.
  id: docs-config-files
  limitations: Prose. Its statement that enabled = false forces thinking off does not hold for always-thinking models (see test-config-key), and its fallback and failure wording does not describe the environment variable route (see test-env-invalid-levels).
  location: https://moonshotai.github.io/kimi-code/en/configuration/config-files.html
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.1
- claim: KIMI_MODEL_THINKING_EFFORT has two roles. As a global override it forces a thinking effort (thinking.effort), "bypassing the model's declared support_efforts; kimi provider only". Inside the KIMI_MODEL_* family for a synthesized temporary model it takes low/medium/high/xhigh/max. Environment variables take priority over config.toml, and the -m <alias> option has the highest priority for model choice.
  id: docs-env-vars
  limitations: The documented value list is broader than what the managed Kimi service accepts.
  location: https://moonshotai.github.io/kimi-code/en/configuration/env-vars.html
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.1
- claim: For ordinary runtime parameters, command-line options outrank the user config file, and "a small number of environment variables explicitly override specific config file fields"; ordinary runtime parameters do not fall back to shell environment variables. It names no command-line option for thinking or effort.
  id: docs-overrides
  limitations: Its example override is a different variable, and it does not rank KIMI_MODEL_THINKING_EFFORT against thinking.effort; that was established by test-precedence.
  location: https://moonshotai.github.io/kimi-code/en/configuration/overrides.html
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.1
- claim: The option table has no thinking or effort option. In -p mode "Thinking content and Assistant text are both prefixed with a bullet"; with --output-format stream-json "Thinking content is not written to JSONL".
  id: docs-kimi-command
  limitations: Prose; the output behavior was also observed locally in test-output-modes.
  location: https://moonshotai.github.io/kimi-code/en/reference/kimi-command.html
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.1
- claim: The slash-command reference lists /model ("Switch the LLM model used in the current session") and /reload but no /effort or /thinking entry; a text search of the page for "effort" found no match.
  id: docs-slash-commands
  limitations: A finding of absence. The command exists in the binary (source-effort-command), so the documentation is incomplete rather than the command missing.
  location: https://moonshotai.github.io/kimi-code/en/reference/slash-commands.html
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.1
- claim: 'Latest release is 2.1.1 (2026-09-24). Effort history: 2.1.0 (2026-09-23) "Upgrade the default thinking effort to the recommended level for eligible users"; 0.29.0 (2026-07-22) "Support selecting a thinking effort level from ACP clients"; 0.26.0 "Warn in the /model and /effort pickers that switching invalidates the existing prompt cache"; 0.24.1 "web: Align thinking-level handling with the CLI: submit the selected level verbatim"; 0.31.0 added /secondary_model for subagents.'
  id: docs-changelog
  limitations: Release notes are terse and do not state the current rules.
  location: https://moonshotai.github.io/kimi-code/en/release-notes/changelog.html
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.1
- claim: POST /api/v1/sessions/{session_id}/prompts takes a string field thinking ("Thinking-mode effort level"), and POST /api/v1/sessions/{session_id}/profile takes the same field. GET /api/v1/sessions/{session_id}/status reports thinking_level, and GET /api/v1/models returns support_efforts and default_effort per alias.
  id: docs-server-api
  limitations: Documentation only. The local server was not started because kimi web opens a browser window, so accepted values, lifetime, and rank against the environment variable are unverified.
  location: https://moonshotai.github.io/kimi-code/en/reference/server-api.html
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.1
- claim: session/set_config_option is the "Unified model / thinking / mode picker dispatcher" of kimi acp; the page names configId values 'mode' and 'model' but no thinking config id or value.
  id: docs-acp
  limitations: Documentation only; the option id and value format for thinking are unverified.
  location: https://moonshotai.github.io/kimi-code/en/reference/kimi-acp.html
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.1
- claim: ThinkingConfigSchema has a forcedEffort field bound to KIMI_MODEL_THINKING_EFFORT. resolveThinkingEffortForModel picks the requested effort, else off when enabled is false, else the configured effort, else the model default; an off on an always-thinking model is replaced by the configured or default effort. resolveThinkingEffort rejects off with a ThinkingConfigError for an always-thinking model without an off_effort, and rejects a level outside support_efforts only under strict validation.
  id: source-thinking-resolution
  limitations: Read from strings embedded in the installed binary, not from a commit-pinned upstream repository; function names and line positions can change between releases.
  location: /Users/ken/.kimi-code/bin/kimi (2.1.1 single-file build embeds readable JavaScript; ThinkingConfigSchema, resolveThinkingEffortForModel, resolveThinkingEffort)
  method: source_code
  observed_on: 2026-09-29
  version: 2.1.1
- claim: '/effort (alias thinking, description "Switch thinking effort", availability always) opens a picker, or takes one level. The selectable list is the model''s support_efforts, with off added only when the model is not always-on. An argument outside the list is refused with "Unsupported thinking effort ... Available: ..." except on anthropic-protocol models, where it is sent unchanged with a warning. thinkingEffortToConfig writes enabled = false for off and does not write a level ranked above the model''s default_effort as effort. The picker''s Alt+S selects a session-only choice.'
  id: source-effort-command
  limitations: Read from embedded source; the persistence rule was not exercised on a level above the default.
  location: /Users/ken/.kimi-code/bin/kimi (2.1.1 embedded source; TUI slash-command table, handleEffortCommand, segmentsFor, thinkingEffortToConfig, performModelSwitch)
  method: source_code
  observed_on: 2026-09-29
  version: 2.1.1
- claim: The default run used kimi-code/k3 at thinkingEffort high in both the profile.bind and llm.request records. With the effort key removed from [thinking], -m kimi-code/k3 and -m kimi-code/k3-256k ran at high, -m kimi-code/kimi-for-coding ran at max, and -m kimi-code/kimi-for-coding-highspeed ran at the boolean value on, matching each model's default_effort. With effort = "high" present, kimi-for-coding ran at high (the global key outranks the model default) and highspeed still ran at on.
  id: test-default-and-models
  limitations: One account (managed Kimi Code login). A tiny prompt was used, so nothing was learned about how much reasoning each level produces.
  location: kimi -p "Reply with exactly the word ok." under an isolated KIMI_CODE_HOME (copy of config.toml, symlinked credentials); sessions read from <KIMI_CODE_HOME>/sessions/*/*/agents/main/wire.jsonl
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.0.2
- claim: With KIMI_MODEL_THINKING_EFFORT set to max, low, and off, the llm.request record carried thinkingEffort max, low, and off (thinkingKeep absent for off), each run exited 0, and the profile.bind record still carried the configured high. The off run produced no reasoning line on stderr. On kimi-code/kimi-for-coding-highspeed, which declares no levels, low and off were sent as written and both runs exited 0.
  id: test-env-forced-levels
  limitations: Shows what was requested and accepted, not how many reasoning tokens the service spent.
  location: KIMI_MODEL_THINKING_EFFORT=<level> kimi -p "Reply with exactly the word ok." under an isolated KIMI_CODE_HOME; wire.jsonl read afterwards
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.0.2
- claim: 'Each of bogus, medium, and xhigh was sent unchanged (llm.request thinkingEffort recorded the literal token) and the service answered 400. The command printed "error: failed to run prompt: provider.api_error: 400 Invalid request Error", exited 1, and no fallback occurred. No message named the level.'
  id: test-env-invalid-levels
  limitations: Only the managed Kimi Code endpoint was tested; other provider protocols pass the token to their own services.
  location: KIMI_MODEL_THINKING_EFFORT=<level> kimi -p "Reply with exactly the word ok." under an isolated KIMI_CODE_HOME, levels bogus, medium, and xhigh on kimi-code/k3
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.0.2
- claim: effort = "low" and effort = "max" were used as written (profile.bind and llm.request both recorded the value). effort = "medium" and effort = "bogus" were silently replaced by the model default high in both records; the runs exited 0 and printed no warning. With [thinking] enabled = false, kimi-code/k3 still ran at high, so enabled = false does not turn thinking off on an always-thinking model.
  id: test-config-key
  limitations: The user's real config.toml was never edited. Other providers' fallback behavior was not tested.
  location: Edited copy of config.toml under an isolated KIMI_CODE_HOME; [thinking] effort set to low, max, medium, and bogus in turn, then enabled = false
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.0.2
- claim: With effort = "low" in the config and KIMI_MODEL_THINKING_EFFORT=max, profile.bind recorded low and llm.request recorded max, so the environment variable outranks [thinking].effort. The [thinking].effort key (high) outranks a model's default_effort (max) for kimi-code/kimi-for-coding, and removing it restores max.
  id: test-precedence
  limitations: The rank of the server-API thinking field and the ACP option against the environment variable was not tested.
  location: config.toml [thinking] effort = "low" together with KIMI_MODEL_THINKING_EFFORT=max under an isolated KIMI_CODE_HOME
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.0.2
- claim: On 2.1.1 with kimi-code/k3, KIMI_MODEL_THINKING_EFFORT of max, low, and off reached llm.request unchanged and exited 0, while medium and bogus reached llm.request unchanged and exited 1 with the same 400 message. A config effort of max was used and medium was replaced by high with exit 0. The results are identical to 2.0.2.
  id: test-rerun-2-1-1
  limitations: The per-model runs and the highspeed model were not repeated on 2.1.1.
  location: kimi -p "Reply with exactly the word ok." under an isolated KIMI_CODE_HOME after the host binary upgraded to 2.1.1; wire.jsonl read afterwards
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.1.1
- claim: '/effort opened a picker titled "Select thinking effort" listing Low, High, and Max with "Alt+S session-only". /effort medium printed ''Error: Unsupported thinking effort "medium" for kimi-code/k3. Available: low, high, max''. /effort low printed "Thinking set to low.", changed the status bar to "thinking: low", and wrote effort = "low" into the scratch config, but the next llm.request still carried thinkingEffort max while profile.bind carried low, so the environment variable outranks /effort. Before any /effort the status bar showed the configured high although the request went out at max.'
  id: test-slash-effort
  limitations: One model and one account. The session-only path (Alt+S) was not exercised.
  location: Interactive kimi started in a detached tmux session with KIMI_MODEL_THINKING_EFFORT=max under an isolated KIMI_CODE_HOME; /effort typed into the prompt; wire.jsonl read afterwards
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.1.1
- claim: '/reload printed "Session reloaded." on an active session, the status bar stayed at "thinking: max", and both llm.request records carried thinkingEffort max, so editing thinking.effort does not change a running session. A /reload before any session existed printed "Runtime and TUI config reloaded; no active session." and the status bar did change to the new value, which applies to the session created next.'
  id: test-reload
  limitations: Only thinking.effort was edited; thinking.enabled and a model's default_effort were not tested.
  location: Interactive kimi in a detached tmux session with [thinking] effort = "max"; after the first prompt the config was edited to effort = "low", /reload was typed, and a second prompt sent; wire.jsonl read afterwards
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.1.1
- claim: In text mode stdout held only the answer while stderr held the version line, the reasoning text as a bullet-prefixed line, and the resume hint. With KIMI_MODEL_THINKING_EFFORT=off stderr held no reasoning line. In stream-json mode stdout held three JSON lines (system.version, the assistant content, and session.resume_hint with session_id), stderr was empty, no reasoning appeared, and no line stated the effort.
  id: test-output-modes
  limitations: One trivial prompt, so the reasoning text was one short line; whether longer reasoning is complete or abridged by the service was not established.
  location: kimi -p "Reply with exactly the word ok." with stdout and stderr captured separately, in text mode and with --output-format stream-json
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.0.2
- claim: Every run wrote a wire.jsonl. The profile.bind record (once per session) holds modelAlias and thinkingEffort as configured; each llm.request record holds modelAlias, model, thinkingKeep, and thinkingEffort for one request and matched the level each test had asked for. kimi-code.log carries an equivalent "llm config" line with thinkingEffort=<level>. The same two record types were present on 2.1.1.
  id: test-session-record
  limitations: The record is a local file whose format is not documented as stable, and no option that suppresses it was searched for.
  location: /Users/ken/.kimi-code/sessions/wd_<workspace>_<hash>/session_<session-id>/agents/main/wire.jsonl and .../logs/kimi-code.log
  method: local_inspection
  observed_on: 2026-09-29
  version: 2.0.2
support: some_models
levels:
- evidence_ids:
  - test-env-forced-levels
  - test-rerun-2-1-1
  - source-thinking-resolution
  - source-effort-command
  meaning: No reasoning. Not offered by /effort for always-thinking models, and thinking.enabled = false is replaced by the configured or default effort for them, but KIMI_MODEL_THINKING_EFFORT=off was sent, accepted, and produced no reasoning text on kimi-code/k3.
  native: off
  normalized: off
- evidence_ids:
  - local-config-toml
  - test-env-forced-levels
  - test-config-key
  - test-rerun-2-1-1
  meaning: Weakest level in support_efforts, which the CLI treats as ordered by strength. Kimi documents no further semantics.
  native: low
  normalized: low
- evidence_ids:
  - docs-config-files
  - test-env-invalid-levels
  - test-config-key
  - test-slash-effort
  meaning: Documented value for thinking.effort. The managed Kimi models do not list it in support_efforts; the service rejects it with a 400 when forced, /effort refuses it, and a config value of medium silently falls back to the model default. It may work on non-Kimi provider protocols that receive the token unchanged.
  native: medium
  normalized: medium
- evidence_ids:
  - local-config-toml
  - test-default-and-models
  meaning: The default on kimi-code/k3 and kimi-code/k3-256k, and through the global thinking.effort on kimi-code/kimi-for-coding.
  native: high
  normalized: high
- evidence_ids:
  - docs-config-files
  - test-env-invalid-levels
  meaning: Documented value for thinking.effort. The managed Kimi models do not list it and the service rejects it with a 400 when forced. Documented as passed through to providers that support it.
  native: xhigh
  normalized: very_high
- evidence_ids:
  - local-config-toml
  - test-env-forced-levels
  - test-config-key
  - test-slash-effort
  - source-effort-command
  meaning: Strongest level in support_efforts. Accepted through the environment variable, the config key, and /effort. As a /effort choice on a model whose default is lower it is not written to the config as effort.
  native: max
  normalized: maximum
default_level:
  decided_by: model
  evidence_ids:
  - local-config-toml
  - test-default-and-models
  - source-thinking-resolution
  - docs-changelog
  native: high
controls:
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-env-vars
  - source-thinking-resolution
  - test-env-forced-levels
  - test-precedence
  - test-slash-effort
  - docs-overrides
  - local-help-2-0-2
  - local-help-2-1-1
  id: effort-env-var
  kind: environment_variable
  lasts: one_session
  launch_modes:
  - interactive
  - non_interactive
  name: KIMI_MODEL_THINKING_EFFORT
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - source-effort-command
  - test-slash-effort
  - docs-slash-commands
  id: effort-session-command
  kind: session_command
  lasts: until_changed
  launch_modes:
  - interactive
  name: /effort
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-config-files
  - local-config-toml
  - test-config-key
  - test-reload
  id: effort-config-key
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: thinking.effort
  value: level_token
- arguments: []
  changes_running_session: unknown
  evidence_ids:
  - docs-config-files
  - test-config-key
  id: thinking-enabled-config-key
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: thinking.enabled
  value: on_or_off
- arguments: []
  changes_running_session: unknown
  evidence_ids:
  - docs-config-files
  - local-config-toml
  - test-default-and-models
  id: model-default-effort-config-key
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: models.default_effort
  value: level_token
- arguments: []
  changes_running_session: unknown
  evidence_ids:
  - docs-config-files
  id: subagent-default-effort-config-key
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: secondary_model.default_effort
  value: level_token
- arguments: []
  changes_running_session: unknown
  evidence_ids:
  - docs-server-api
  id: prompt-thinking-request-field
  kind: request_field
  lasts: unknown
  launch_modes:
  - non_interactive
  name: thinking
  value: level_token
- arguments: []
  changes_running_session: unknown
  evidence_ids:
  - docs-acp
  - docs-changelog
  id: acp-set-config-option
  kind: request_field
  lasts: unknown
  launch_modes:
  - non_interactive
  name: session/set_config_option
  value: level_token
precedence:
- effort-env-var
- effort-session-command
- effort-config-key
- model-default-effort-config-key
models:
- accepts:
  - low
  - high
  - max
  default: high
  evidence_ids:
  - local-config-toml
  - test-default-and-models
  model: kimi-code/k3*
- accepts:
  - low
  - high
  - max
  default: max
  evidence_ids:
  - local-config-toml
  - test-default-and-models
  model: kimi-code/kimi-for-coding
- accepts: []
  default: on
  evidence_ids:
  - local-config-toml
  - test-default-and-models
  - test-env-forced-levels
  model: kimi-code/kimi-for-coding-highspeed
invalid_level:
  behavior: fails_the_request
  evidence_ids:
  - test-env-invalid-levels
  - test-rerun-2-1-1
  - test-config-key
  - test-slash-effort
  - source-thinking-resolution
  message: 'error: failed to run prompt: provider.api_error: 400 Invalid request Error'
  warns: yes
reporting:
  evidence_ids:
  - test-session-record
  - test-env-forced-levels
  - test-precedence
  - test-slash-effort
  field: /thinkingEffort
  locator: ~/.kimi-code/sessions/wd_*/session_<session-id>/agents/main/wire.jsonl
  notes: Read the llm.request records (one per model request), not profile.bind and not the TUI status bar. profile.bind and the status bar hold the level as configured or chosen with /effort; with KIMI_MODEL_THINKING_EFFORT=max they stayed at the configured level while llm.request held max. The directory is <KIMI_CODE_HOME>/sessions (default ~/.kimi-code). Find the session with the session_id in the session.resume_hint line of --output-format stream-json output, or the "kimi -r <session-id>" line printed on stderr in text mode. kimi-code.log in the session logs directory carries the same value as thinkingEffort=<level> on an "llm config" line. Neither -p output mode prints the level.
  source: session_record
reasoning_output:
  evidence_ids:
  - docs-kimi-command
  - test-output-modes
  reaches_caller: full_text
gaps:
- area: controls
  detail: Whether editing thinking.enabled and running /reload changes a session that has already started was not tested; only thinking.effort was (test-reload). Documentation claims enabled = false forces thinking off, which is false for always-thinking models.
  entry: thinking-enabled-config-key
  next_check: In a tmux-hosted session under a scratch KIMI_CODE_HOME, edit [thinking] enabled, run /reload, send a prompt, and read the newest llm.request thinkingEffort in wire.jsonl.
- area: controls
  detail: Whether a changed default_effort on a model alias reaches a running session was not tested, and managed aliases are rewritten by service refreshes.
  entry: model-default-effort-config-key
  next_check: Edit default_effort on the active alias in a scratch config, run /reload in a live session, send a prompt, and read the newest llm.request thinkingEffort.
- area: controls
  detail: Only documentation describes secondary_model.default_effort; no subagent run was made, so neither its effect nor its reach into a running session is known.
  entry: subagent-default-effort-config-key
  next_check: Configure [secondary_model] default_effort in a scratch config, run a prompt that starts a subagent, and read the subagent's llm.request thinkingEffort under agents/*/wire.jsonl.
- area: controls
  detail: The thinking field of the server prompt and profile routes is documented but never called, because kimi web opens a browser window. Accepted values, how long the choice lasts, and whether it changes a running session are unknown.
  entry: prompt-thinking-request-field
  next_check: Start the local server without opening a browser, POST a prompt with thinking set, then read the newest llm.request thinkingEffort and GET the session status thinking_level.
- area: controls
  detail: The config id and value format that session/set_config_option uses for thinking are undocumented, and the option was not exercised.
  entry: acp-set-config-option
  next_check: Run kimi acp over stdio in a scratch KIMI_CODE_HOME, list config options from session/new, call set_config_option with the thinking option, and read llm.request thinkingEffort.
- area: precedence
  detail: The rank of the server-API thinking field, the ACP option, thinking.enabled, and secondary_model.default_effort against KIMI_MODEL_THINKING_EFFORT and /effort is not established, so those controls are absent from the precedence list.
  next_check: With KIMI_MODEL_THINKING_EFFORT=max set, apply each control at a different level and compare the newest llm.request thinkingEffort.
- area: default_level
  detail: The changelog says the default effort was upgraded "for eligible users" in 2.1.0. Whether the default depends on the account and not only on the model catalog entry is unknown; only one account was examined.
  next_check: Read default_effort per alias from GET /api/v1/models on a second account, or compare a fresh KIMI_CODE_HOME login that has no [thinking] table.
- area: models
  detail: Kimi Code can also drive OpenAI-, Anthropic-, and Gemini-protocol providers, which receive effort strings without client-side mapping. Only the managed Kimi Code provider was tested, and its catalog is refreshed from the service.
  next_check: Configure a disposable provider alias for each protocol under a scratch KIMI_CODE_HOME and record which tokens each accepts.
- area: reasoning_output
  detail: In text mode the reasoning reaches stderr, but whether it is the raw reasoning or a service-side summary was not established, and stream-json output omits it entirely.
  next_check: Run a long reasoning prompt in text mode and compare the stderr reasoning length with the reasoning token count in the session record.
- area: other
  detail: Only the level requested on the wire was verified. Whether low, high, max, or off change the reasoning the service performs (token counts, latency) was not measured.
  next_check: Run a multi-step reasoning prompt at each level with stream-json and compare usage in the session record and wall-clock time.
changes:
- 'Rewritten for schema revision 2: gaps now carry an area and entry, the two documented models that share a level list are one pattern, and every value recorded as unknown has a gap.'
- Re-verified every finding against a scratch KIMI_CODE_HOME. The host binary upgraded itself from 2.0.2 to 2.1.1 mid-run, so the core checks (forced levels, invalid levels, config key, missing flags) were repeated on 2.1.1 with identical results, and the newer-release gap is closed.
- 'New finding: /effort loses to KIMI_MODEL_THINKING_EFFORT, and the TUI status bar and profile.bind then show the chosen level while llm.request holds the forced one. /effort is added to the precedence list.'
- 'New finding: /reload on a running session does not apply an edited thinking.effort, so the config key does not change a session that has started.'
- 'Changelog citations corrected to what the page states: the default upgrade for eligible users is 2.1.0, not 0.42.0. The Server API profile route also takes thinking, and the slash-command reference omits /effort.'
requires_claudine_update: true
reason: The generated provider metadata is wrong. claudine/docs/providers/facts/kimi.yaml still feeds claudine/lib/src/provider/kimi/data.rs a binary_toggle with --thinking and --no-thinking, but Kimi Code CLI 2.0.2 and 2.1.1 have no such flags; each exits with unknown option. The real per-launch control is the KIMI_MODEL_THINKING_EFFORT environment variable, which beats /effort and the thinking.effort config key. Levels are model dependent (low, high, max on the managed k3 family and kimi-for-coding, none on kimi-for-coding-highspeed). Claudine also needs a reader for the level actually sent (llm.request thinkingEffort in the session wire.jsonl), because neither -p output mode nor the status bar states it, and its neutral scale must account for a rejected level failing the request through the environment variable but silently falling back through the config key. The facts file reasoning key graduates to this research topic, so it must be deleted and the catalog regenerated.
contract_checked: 2026-09-29
---

# Reasoning Level Support in Kimi Code CLI

Kimi Code CLI calls its reasoning control the **thinking effort**. Every managed Kimi model
reasons ("always thinking"), so the setting picks a depth rather than turning reasoning on
or off. There is **no command-line flag** for it: on 2.0.2 and 2.1.1, `--thinking` and
`--effort` both exit with `unknown option`. A caller chooses a level with an environment
variable, a config key, or a command typed inside a session.

## Levels

Weakest to strongest as the CLI orders them (`support_efforts` is ordered by strength):

| Level    | Normalized  | Notes                                                                                             |
| -------- | ----------- | ------------------------------------------------------------------------------------------------- |
| `off`    | `off`       | Only reachable through `KIMI_MODEL_THINKING_EFFORT=off` on always-thinking models; `/effort` does not offer it and `thinking.enabled = false` is replaced by the configured or default level |
| `low`    | `low`       | Accepted by the managed k3 family and `kimi-for-coding`                                           |
| `medium` | `medium`    | Documented, but rejected by the managed Kimi models (400)                                         |
| `high`   | `high`      | The default on `k3`, `k3-256k`                                                                    |
| `xhigh`  | `very_high` | Documented, but rejected by the managed Kimi models (400)                                         |
| `max`    | `maximum`   | Strongest managed level                                                                           |

`kimi-code/kimi-for-coding-highspeed` has no adjustable levels; it reports the boolean `on`.
Kimi publishes no description of what each level does beyond the ordering. No level falls
outside the scale.

The default is the model's `default_effort` (`high` for `k3`, `max` for `kimi-for-coding`),
unless `[thinking] effort` is set, which then applies to every model that lists it. A stock
config on this host carries `effort = "high"`, written by a 2026-09-22 migration
(`thinking-effort-max-to-high`). The 2.1.0 changelog says the default was raised "for
eligible users", so it may also depend on the account (see the gaps).

## Choosing a Level

Precedence, strongest first, as observed: `KIMI_MODEL_THINKING_EFFORT`, then `/effort`, then
`[thinking] effort`, then the model's `default_effort`. The server-API `thinking` field, the
ACP option, and `thinking.enabled` were not ranked (see the gaps).

**Environment variable, per launch** (best fit for a wrapper):

```sh
KIMI_MODEL_THINKING_EFFORT=low kimi -p "Summarize this repository"
```

The value is sent to the service unchanged and bypasses the model's `support_efforts`. It
works in interactive and `-p` runs, and it wins over everything else, including a later
`/effort`: after `/effort low` under `KIMI_MODEL_THINKING_EFFORT=max` the status bar read
`thinking: low` while the request went out at `max`.

**Config file, persistent** (`~/.kimi-code/config.toml`):

```toml
[thinking]
enabled = true
effort = "low"   # low | medium | high | xhigh | max; unsupported values fall back silently
```

The key is read when a session starts. Editing it and running `/reload` inside a running
session prints "Session reloaded." but does not change that session's level. A per-model
default goes under the alias table (`default_effort`, and `support_efforts` to declare which
levels the model takes). `[secondary_model] default_effort` sets the level for subagents.

**Inside a session:** `/effort` (alias `/thinking`) opens a picker, or takes a level:

```text
/effort high
```

The picker lists the model's levels and takes `Alt+S` for a session-only choice. A level
ranked above the model's default is not written back to the config as `effort`. An unlisted
level is refused with `Unsupported thinking effort "medium" for kimi-code/k3. Available: low,
high, max`. The reference page for slash commands does not list `/effort`; the command comes
from the binary.

**Requests** (not exercised): the local server's prompt and profile routes accept a
`thinking` string, and `kimi acp` offers `session/set_config_option` for a thinking option.

Picking the model (`-m kimi-code/k3`) changes which default applies but is not itself an
effort control.

An unsupported level does not behave the same on every route. Through
`KIMI_MODEL_THINKING_EFFORT` the token is sent as-is and the service answers `400`, so the run
prints `error: failed to run prompt: provider.api_error: 400 Invalid request Error` and exits
1. Through `[thinking] effort` it falls back to the model default with no warning. `/effort`
refuses an unlisted level before sending anything.

## Models

The managed catalog on this account (refreshed from the service, so it can change):

| Model alias                            | Levels               | Default |
| -------------------------------------- | -------------------- | ------- |
| `kimi-code/k3`, `kimi-code/k3-256k`    | `low`, `high`, `max` | `high`  |
| `kimi-code/kimi-for-coding`            | `low`, `high`, `max` | `max`   |
| `kimi-code/kimi-for-coding-highspeed`  | none (always on)     | `on`    |

`KIMI_MODEL_THINKING_EFFORT` bypasses these lists: `low` and `off` were sent and accepted on
the highspeed model. Models from other providers configured in Kimi Code receive effort
strings without client-side mapping; they were not tested.

## Confirming the Level

Neither `-p` output mode states the level, and neither does the TUI status bar. Read the
session record instead:

```text
~/.kimi-code/sessions/wd_<workspace>_<hash>/session_<session-id>/agents/main/wire.jsonl
```

Take the newest `"type":"llm.request"` line and read `thinkingEffort`. That is the level sent
with the request. Do not use the `profile.bind` line or the status bar for this: they record
the level as configured or chosen, and stayed at the configured level in a run where
`KIMI_MODEL_THINKING_EFFORT=max` made the request go out at `max`. The session id is in the
`session.resume_hint` line of `--output-format stream-json` output, and `KIMI_CODE_HOME` moves
the whole tree. The same value appears in `logs/kimi-code.log` as `thinkingEffort=<level>`.

## Reasoning Output

In `-p` text mode the reasoning arrives on **stderr** as bullet-prefixed lines, and the
answer on stdout; with `off` no reasoning line appears. In `--output-format stream-json` the
reasoning is not emitted at all. A wrapper that needs the reasoning text must use text mode.

## Sources

- [Configuration files](https://moonshotai.github.io/kimi-code/en/configuration/config-files.html)
- [Environment variables](https://moonshotai.github.io/kimi-code/en/configuration/env-vars.html)
- [Config overrides](https://moonshotai.github.io/kimi-code/en/configuration/overrides.html)
- [`kimi` command](https://moonshotai.github.io/kimi-code/en/reference/kimi-command.html)
- [Slash commands](https://moonshotai.github.io/kimi-code/en/reference/slash-commands.html)
- [Changelog](https://moonshotai.github.io/kimi-code/en/release-notes/changelog.html)
- [Server API](https://moonshotai.github.io/kimi-code/en/reference/server-api.html)
- [`kimi acp`](https://moonshotai.github.io/kimi-code/en/reference/kimi-acp.html)
- Local: `kimi --help` (2.0.2 and 2.1.1), `~/.kimi-code/config.toml`, the source embedded in
  the 2.1.1 binary, and disposable `kimi -p` and tmux-hosted interactive runs under a scratch
  `KIMI_CODE_HOME`.

## Changelog

- Rewritten for schema revision 2: gaps now carry an area and entry, the two documented
  models with the same levels are one pattern, and every unknown has a gap.
- Every finding was re-verified. The host binary upgraded itself from 2.0.2 to 2.1.1 during
  the run, so the core checks were repeated on 2.1.1 with identical results.
- New: `/effort` loses to `KIMI_MODEL_THINKING_EFFORT`, and the status bar and `profile.bind`
  then show the wrong level; `/reload` does not apply an edited `thinking.effort` to a running
  session.
- Corrected: the default upgrade "for eligible users" is 2.1.0, not 0.42.0. The server API's
  profile route also takes `thinking`, and the slash-command reference omits `/effort`.
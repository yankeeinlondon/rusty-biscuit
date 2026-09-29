---
$schema: ./_schema.yaml
schema_revision: 1
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
- claim: 'kimi --version prints 2.0.2. The help output lists no flag for reasoning, thinking, or effort; the only model-related launch flag is -m/--model <model>. Passing --thinking, --no-thinking, or --effort each fails immediately with "error: unknown option" (no model call is made).'
  id: local-help-2-0-2
  limitations: Covers the installed 2.0.2 only; the documentation describes 2.1.1, which was not installed, so a newer release could add a flag.
  location: kimi --version and kimi --help on this host (binary at /Users/ken/.kimi-code/bin/kimi)
  method: local_inspection
  observed_on: 2026-09-29
  version: 2.0.2
- claim: The user config carries a [thinking] table (enabled = true, effort = "high") and four managed model aliases. kimi-code/k3, kimi-code/k3-256k, and kimi-code/kimi-for-coding declare support_efforts = ["low", "high", "max"] and default_effort ("high", "high", "max"); kimi-code/kimi-for-coding-highspeed declares no support_efforts. All four carry the capabilities thinking and always_thinking. default_model is kimi-code/k3. ~/.kimi-code/migrations-effort.json records a "thinking-effort-max-to-high" migration dated 2026-09-22.
  id: local-config-toml
  limitations: The managed model entries are refreshed from the service ("managed refreshes may rewrite it"), so the lists reflect this account on this date.
  location: /Users/ken/.kimi-code/config.toml (credentials redacted; read only, never modified)
  method: local_inspection
  observed_on: 2026-09-29
  version: 2.0.2
- claim: Documents the [thinking] table (enabled, default true; effort, one of low/medium/high/xhigh/max, falling back to the model default when not in its supported list; keep) and the [models.<alias>] fields support_efforts ("unsupported values fall back to default_effort, out-of-list values fail"), default_effort, off_effort ("the only way to actually stop reasoning on models that reason by default"), and the [secondary_model] default_effort used by subagents. For the main agent a configured global [thinking].effort overrides a model variant's default_effort; for pool-bound subagents the variant's default_effort wins.
  id: docs-config-files
  limitations: Prose; the fallback and failure wording contradicts observed behavior for the environment variable route (see test-env-invalid-levels).
  location: https://moonshotai.github.io/kimi-code/en/configuration/config-files.html
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.1
- claim: KIMI_MODEL_THINKING_EFFORT "forces a thinking effort (thinking.effort), bypassing the model's declared support_efforts; kimi provider only" (example value max). The KIMI_MODEL_* channel is documented as the model-definition set that also lists KIMI_MODEL_THINKING_EFFORT as low/medium/high/xhigh/max. KIMI_CODE_HOME relocates the data root (default ~/.kimi-code).
  id: docs-env-vars
  limitations: The documented value list is broader than what the managed Kimi service accepts.
  location: https://moonshotai.github.io/kimi-code/en/configuration/env-vars.html
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.1
- claim: States that command-line options outrank the user config file, that environment variables are not a general fallback for config fields, and that a small named set of environment variables explicitly override specific config fields. Lists no command-line option for thinking or effort.
  id: docs-overrides
  limitations: Does not name KIMI_MODEL_THINKING_EFFORT's rank against thinking.effort; that was established by test.
  location: https://moonshotai.github.io/kimi-code/en/configuration/overrides.html
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.1
- claim: kimi -p runs one prompt non-interactively under the auto permission policy. In text output thinking content and assistant text are both prefixed with "• "; assistant text goes to stdout and thinking goes to stderr. With --output-format stream-json "Thinking content is not written to JSONL". The flag list has no thinking or effort option.
  id: docs-kimi-command
  limitations: Prose; the output behavior was also observed locally.
  location: https://moonshotai.github.io/kimi-code/en/reference/kimi-command.html
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.1
- claim: 'Latest documented release is 2.1.1 (2026-09-24). Effort history: 0.21.0 "Refactor the thinking effort system"; 0.29.0 "Support selecting a thinking effort level from ACP clients"; 0.24.2 "Fix Thinking effort routing: non-Kimi providers now preserve configured values, while Kimi models validate runtime selections and fall back safely"; 0.42.0 "Upgrade the default thinking effort to the recommended level for eligible users"; and a fix that thinking effort persists only levels below the model''s top tier (max).'
  id: docs-changelog
  limitations: Release notes are terse; they do not state the current rules.
  location: https://moonshotai.github.io/kimi-code/en/release-notes/changelog.html
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.1
- claim: The prompt-submission route takes an optional string field thinking ("Thinking-mode effort level") applied to the target agent together with profile and model overrides; session state reports thinking_level; GET /api/v1/models returns support_efforts and default_effort per alias.
  id: docs-server-api
  limitations: Documentation only; the route was not called, so the accepted values and how long the override lasts are unverified.
  location: https://moonshotai.github.io/kimi-code/en/reference/server-api.html
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.1
- claim: session/set_config_option is the "Unified model / thinking / mode picker dispatcher" of kimi acp.
  id: docs-acp
  limitations: Documentation only; not exercised, so the option id and value format are unverified.
  location: https://moonshotai.github.io/kimi-code/en/reference/kimi-acp.html
  method: official_docs
  observed_on: 2026-09-29
  version: 2.1.1
- claim: ThinkingConfigSchema has enabled, effort, forcedEffort, and keep, with forcedEffort bound to the environment variable KIMI_MODEL_THINKING_EFFORT. resolveThinkingEffortForModel picks the requested effort, else off when enabled is false, else the configured effort, else the model default, then falls back to the model default when the level is not in support_efforts (strict validation). resolveThinkingEffort sends the token unchanged as reasoning_effort, and rejects off with a ThinkingConfigError only when the model is always_thinking and declares no off_effort. A 400/422 whose message matches reasoning/thinking effort patterns gets a hint appended pointing at the config docs.
  id: source-thinking-resolution
  limitations: Read from strings embedded in the installed binary, not from a commit-pinned upstream repository; function names and line positions can change between releases.
  location: /Users/ken/.kimi-code/bin/kimi (2.0.2 single-file build embeds readable JavaScript source; modules agent-core-v2 llm-adapter thinking.ts, model-config thinking resolution, and the ThinkingConfigSchema section)
  method: source_code
  observed_on: 2026-09-29
  version: 2.0.2
- claim: 'The slash command /effort (alias /thinking, "Switch thinking effort", always available) opens a picker, or takes one level as its argument. An argument outside the model''s selectable list is an error ("Unsupported thinking effort ... Available: ...") except on anthropic-protocol models, where it is sent unchanged with a warning. A pick persists to [thinking] in config.toml except a level ranked above the model''s default_effort, which stays session-only; off persists enabled = false.'
  id: source-effort-command
  limitations: Read from embedded source; the interactive command was not run.
  location: /Users/ken/.kimi-code/bin/kimi (2.0.2 embedded source; TUI slash-command table, handleEffortCommand, thinkingEffortToConfig)
  method: source_code
  observed_on: 2026-09-29
  version: 2.0.2
- claim: With the user's config, the default run used kimi-code/k3 at thinkingEffort high in both the profile.bind and llm.request records. -m kimi-code/k3-256k ran at high, -m kimi-code/kimi-for-coding ran at high (the global [thinking].effort = "high" outranks that model's default_effort of max), and -m kimi-code/kimi-for-coding-highspeed ran at the boolean value on. With the effort key removed from [thinking], k3 ran at high and kimi-for-coding ran at max, matching each model's default_effort.
  id: test-default-and-models
  limitations: One account (managed Kimi Code login); a different account or a refreshed model catalog could differ. A tiny prompt was used, so nothing was learned about how much reasoning each level produces.
  location: kimi -p "Reply with exactly the word ok." run under an isolated KIMI_CODE_HOME (copy of config.toml, symlinked credentials); sessions read from <KIMI_CODE_HOME>/sessions/*/*/agents/main/wire.jsonl
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.0.2
- claim: With KIMI_MODEL_THINKING_EFFORT set to max, low, and off, the llm.request record carried thinkingEffort max, low, and off respectively (and thinkingKeep absent for off), each run exited 0, while the profile.bind record still carried the configured high. The off run produced no thinking text. On kimi-code/kimi-for-coding-highspeed, which declares no levels, KIMI_MODEL_THINKING_EFFORT=low was sent as low and the run exited 0.
  id: test-env-forced-levels
  limitations: Shows what was requested and accepted, not how many reasoning tokens the service spent; the effect of low or off on reasoning depth was not measured.
  location: KIMI_MODEL_THINKING_EFFORT=<level> kimi -p "Reply with exactly the word ok." under an isolated KIMI_CODE_HOME; wire.jsonl read afterwards
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.0.2
- claim: 'Each of bogus, medium, and xhigh was sent unchanged (llm.request thinkingEffort recorded the literal token) and the service answered 400. The command printed "error: failed to run prompt: provider.api_error: 400 Invalid request Error", exited 1, and the turn ended with outcome failed and retryable false. No fallback occurred and no other message named the level.'
  id: test-env-invalid-levels
  limitations: Only the managed Kimi Code endpoint was tested; other provider protocols pass the token through to their own services.
  location: KIMI_MODEL_THINKING_EFFORT=<level> kimi -p "Reply with exactly the word ok." under an isolated KIMI_CODE_HOME, levels bogus, medium, and xhigh on kimi-code/k3
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.0.2
- claim: effort = "low" and effort = "max" were used as written (profile.bind and llm.request both recorded the value). effort = "medium" and effort = "bogus" were silently replaced by the model default high in both records; the runs exited 0 and printed no warning. With [thinking] enabled = false, kimi-code/k3 still ran at high and kimi-code/kimi-for-coding-highspeed still ran at on, so enabled = false does not turn thinking off on these always-thinking models.
  id: test-config-key
  limitations: The user's real config.toml was never edited. Other providers' fallback behavior was not tested.
  location: Edited copy of config.toml under an isolated KIMI_CODE_HOME; [thinking] effort set to low, max, medium, and bogus in turn
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.0.2
- claim: With effort = "low" in the config and KIMI_MODEL_THINKING_EFFORT=max, profile.bind recorded low and llm.request recorded max, so the environment variable wins over [thinking].effort. The -m kimi-code/kimi-for-coding run shows [thinking].effort (high) winning over that model's default_effort (max), and removing [thinking].effort restores max.
  id: test-precedence
  limitations: The rank of /effort, the server-API thinking field, and the ACP option against the environment variable was not tested.
  location: config.toml [thinking] effort = "low" together with KIMI_MODEL_THINKING_EFFORT=max under an isolated KIMI_CODE_HOME; and the -m runs in test-default-and-models
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.0.2
- claim: In text mode stdout held only "• ok" while stderr held the version line, the reasoning text as a "• "-prefixed line, and the resume hint. In stream-json mode stdout held three JSON lines (system.version, the assistant content, and session.resume_hint with session_id) and stderr was empty; no reasoning appeared and no line stated the effort.
  id: test-output-modes
  limitations: One trivial prompt, so the reasoning text was one short line; whether longer reasoning is complete or abridged by the service was not established.
  location: kimi -p "Reply with exactly the word ok." with stdout and stderr captured separately, once in text mode and once with --output-format stream-json
  method: disposable_test
  observed_on: 2026-09-29
  version: 2.0.2
- claim: Every run wrote a wire.jsonl. The profile.bind record (once per session) holds modelAlias and thinkingEffort as configured; each llm.request record holds modelAlias, model, provider, thinkingEffort, and thinkingKeep for one request and matched the level the test had asked for. kimi-code.log carries an equivalent "llm config ... thinkingEffort=<level>" line. stream-json output includes session.resume_hint with the session_id for finding the directory.
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
  - source-thinking-resolution
  meaning: No reasoning. Not selectable in the picker for always-thinking models, and the config key thinking.enabled = false is ignored for them, but KIMI_MODEL_THINKING_EFFORT=off was sent, accepted, and produced no thinking text on kimi-code/k3.
  native: off
  normalized: off
- evidence_ids:
  - local-config-toml
  - test-env-forced-levels
  - test-config-key
  meaning: Weakest level in support_efforts, which the CLI treats as ordered by strength. Kimi documents no further semantics.
  native: low
  normalized: low
- evidence_ids:
  - docs-config-files
  - test-env-invalid-levels
  - test-config-key
  meaning: Documented value for thinking.effort. The managed Kimi models do not list it in support_efforts and the service rejects it with a 400 when forced; a config value of medium silently falls back to the model default. It may work on non-Kimi provider protocols that receive the token unchanged.
  native: medium
  normalized: medium
- evidence_ids:
  - local-config-toml
  - test-default-and-models
  meaning: The default on kimi-code/k3, kimi-code/k3-256k, and (through the global thinking.effort) kimi-code/kimi-for-coding.
  native: high
  normalized: high
- evidence_ids:
  - docs-config-files
  - test-env-invalid-levels
  meaning: Documented value for thinking.effort. The managed Kimi models do not list it and the service rejects it with a 400 when forced. Documented as passed through to OpenAI-compatible providers that support it.
  native: xhigh
  normalized: very_high
- evidence_ids:
  - local-config-toml
  - test-env-forced-levels
  - test-config-key
  meaning: Strongest level in support_efforts. Session-only when chosen with /effort on a model whose default is lower; usable as a config value and through the environment variable.
  native: max
  normalized: maximum
default_level:
  decided_by: model
  evidence_ids:
  - local-config-toml
  - test-default-and-models
  - source-thinking-resolution
  native: high
controls:
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-env-vars
  - source-thinking-resolution
  - test-env-forced-levels
  - test-precedence
  id: effort-env-var
  kind: environment_variable
  lasts: one_session
  launch_modes:
  - interactive
  - non_interactive
  name: KIMI_MODEL_THINKING_EFFORT
  value: level_token
- arguments: []
  changes_running_session: unknown
  evidence_ids:
  - docs-config-files
  - local-config-toml
  - test-config-key
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
  changes_running_session: yes
  evidence_ids:
  - source-effort-command
  id: effort-session-command
  kind: session_command
  lasts: until_changed
  launch_modes:
  - interactive
  name: /effort
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
  model: kimi-code/k3
- accepts:
  - low
  - high
  - max
  default: high
  evidence_ids:
  - local-config-toml
  - test-default-and-models
  model: kimi-code/k3-256k
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
  - test-config-key
  - source-thinking-resolution
  message: 'error: failed to run prompt: provider.api_error: 400 Invalid request Error'
  warns: yes
reporting:
  evidence_ids:
  - test-session-record
  - test-env-forced-levels
  - test-precedence
  field: /thinkingEffort
  locator: ~/.kimi-code/sessions/wd_*/session_<session-id>/agents/main/wire.jsonl
  notes: 'Read the llm.request records (one per model request), not profile.bind: profile.bind holds the level as configured at session start and does not reflect KIMI_MODEL_THINKING_EFFORT, while llm.request holds the level actually sent (max under KIMI_MODEL_THINKING_EFFORT=max when the config said low). The directory is <KIMI_CODE_HOME>/sessions (default ~/.kimi-code). Find the session with the session_id in the session.resume_hint line of --output-format stream-json output, or the "kimi -r <session-id>" line printed on stderr in text mode. kimi-code.log in the session logs directory carries the same value as thinkingEffort=<level> on an "llm config" line. Neither -p output mode prints the level.'
  source: session_record
reasoning_output:
  evidence_ids:
  - docs-kimi-command
  - test-output-modes
  reaches_caller: full_text
gaps:
- detail: The installed CLI is 2.0.2 while the documentation describes 2.1.1 (2026-09-24). A flag or rule added after 2.0.2 would not have been observed.
  next_check: After a deliberate upgrade of the host CLI (not done here because it changes the user's install), rerun kimi --help and the disposable level tests and compare with this document.
  subject: newer-release
- detail: Whether /effort, the server-API thinking field, and the ACP set_config_option outrank KIMI_MODEL_THINKING_EFFORT on a running session was not tested, so they are absent from the precedence list. The accepted value format of the request-field controls is also unverified.
  next_check: In a tmux-hosted interactive session started with KIMI_MODEL_THINKING_EFFORT=max, run /effort low, send a prompt, and read the newest llm.request thinkingEffort in wire.jsonl; then call the local kimi web prompt route with thinking set and read the same field.
  subject: effort-session-command
- detail: Whether editing thinking.effort or thinking.enabled reaches an already-started session through /reload (or a config watcher) was not tested.
  next_check: Start an interactive session, edit a copy of config.toml under KIMI_CODE_HOME, run /reload, send a prompt, and read the newest llm.request thinkingEffort.
  subject: config-reload
- detail: The changelog says the default effort was upgraded "for eligible users". Whether the default depends on the account, and not only on the model catalog entry, is unknown; only one account was examined.
  next_check: Read default_effort per alias from GET /api/v1/models on a second account, or compare a fresh KIMI_CODE_HOME login without a [thinking] table.
  subject: default-level
- detail: Only the level requested on the wire was verified. Whether low, high, max, or off change the reasoning the service actually performs (token counts, latency) was not measured, and the reasoning text length was one line.
  next_check: Run a multi-step reasoning prompt at each level with stream-json and compare usage in the session record and wall-clock time.
  subject: level-effect
- detail: In text mode the reasoning text reaches stderr, but it was not established whether it is the raw reasoning or a service-side summary, and stream-json output omits it entirely.
  next_check: Run a long reasoning prompt in text mode and compare the stderr reasoning length with the reasoning token count in the session record.
  subject: reasoning-output
- detail: Kimi Code can also drive OpenAI-, Anthropic-, and Gemini-protocol providers, which receive effort strings without client-side mapping. Only the managed Kimi Code provider was tested.
  next_check: Configure a disposable provider alias for each protocol under a scratch KIMI_CODE_HOME and record which tokens each accepts.
  subject: other-providers
changes:
- Initial document for Kimi Code CLI 2.0.2 (documentation read at 2.1.1); nothing to compare with.
requires_claudine_update: true
reason: The generated provider metadata is wrong. claudine/docs/providers/facts/kimi.yaml feeds claudine/lib/src/provider/kimi/data.rs a ReasoningSupport::BinaryToggle with --thinking, --no-thinking, but Kimi Code CLI 2.0.2 has no such flags (each exits with "unknown option"). The real controls are the KIMI_MODEL_THINKING_EFFORT environment variable (per launch, wins over config), the [thinking] effort config key, and the /effort session command, with model-dependent levels low/high/max on the managed k3 family and no adjustable level on kimi-for-coding-highspeed. Claudine also needs a reader for the level actually sent (llm.request thinkingEffort in the session wire.jsonl), and its neutral scale must account for a rejected level failing the request via the environment variable but silently falling back via the config key. The facts file reasoning key graduates to this research topic, so it must be deleted and the catalog regenerated.
contract_checked: 2026-09-29
---

# Reasoning Level Support in Kimi Code CLI

Kimi Code CLI calls its reasoning control the **thinking effort**. Every managed Kimi model
reasons ("always thinking"), so the setting picks a depth rather than turning reasoning on
or off. There is **no command-line flag** for it: on 2.0.2 `--thinking`, `--no-thinking`,
and `--effort` all exit with `unknown option`. A caller chooses a level with an environment
variable, a config key, or a command typed inside a session.

## Levels

Weakest to strongest as the CLI orders them (`support_efforts` is ordered by strength):

| Level    | Normalized  | Notes                                                                                             |
| -------- | ----------- | ------------------------------------------------------------------------------------------------- |
| `off`    | `off`       | Only reachable through `KIMI_MODEL_THINKING_EFFORT=off` on always-thinking models; `thinking.enabled = false` is ignored for them |
| `low`    | `low`       | Accepted by the managed k3 family and `kimi-for-coding`                                           |
| `medium` | `medium`    | Documented, but rejected by the managed Kimi models (400)                                         |
| `high`   | `high`      | The default on `k3`, `k3-256k`                                                                    |
| `xhigh`  | `very_high` | Documented, but rejected by the managed Kimi models (400)                                         |
| `max`    | `maximum`   | Strongest managed level                                                                           |

`kimi-code/kimi-for-coding-highspeed` has no adjustable levels; it reports the boolean `on`.
Kimi publishes no description of what each level does beyond the ordering.

The default is the model's `default_effort` (`high` for `k3`, `max` for `kimi-for-coding`),
unless `[thinking] effort` is set, which then applies to every model that lists it.
A stock config on this host carries `effort = "high"`, written by a 2026-09-22 migration
(`thinking-effort-max-to-high`).

## Choosing a Level

Precedence, strongest first, as observed: `KIMI_MODEL_THINKING_EFFORT`, then
`[thinking] effort`, then the model's `default_effort`. The rank of `/effort` and the
request fields against the environment variable is not established (see the gaps).

**Environment variable, per launch** (best fit for a wrapper):

```sh
KIMI_MODEL_THINKING_EFFORT=low kimi -p "Summarize this repository"
```

The value is sent to the service unchanged and bypasses the model's `support_efforts`.
It works in interactive and `-p` runs.

**Config file, persistent** (`~/.kimi-code/config.toml`):

```toml
[thinking]
enabled = true
effort = "low"   # low | medium | high | xhigh | max; unsupported values fall back silently
```

A per-model default goes under the alias table (`default_effort`, and `support_efforts` to
declare which levels the model takes). `[secondary_model] default_effort` sets the level for
subagents.

**Inside a session:** `/effort` (alias `/thinking`) opens a picker, or takes a level:

```text
/effort high
```

A level ranked above the model's default stays for the session only; anything else is also
written to `[thinking]`. `/model` offers the same choice next to the model.

**Requests** (not exercised): the local server's prompt route accepts a `thinking` string,
and `kimi acp` offers `session/set_config_option` for a thinking option.

Picking the model (`-m kimi-code/k3`) changes which default applies but is not itself an
effort control.

An unsupported level does not behave the same on every route. Through
`KIMI_MODEL_THINKING_EFFORT` the token is sent as-is and the service answers `400`, so the run prints
`error: failed to run prompt: provider.api_error: 400 Invalid request Error` and exits 1.
Through `[thinking] effort` it falls back to the model default with no warning. `/effort` refuses
an unlisted level with an `Unsupported thinking effort` error before sending anything.

## Models

The managed catalog on this account (refreshed from the service, so it can change):

| Model alias                            | Levels              | Default |
| -------------------------------------- | ------------------- | ------- |
| `kimi-code/k3`                         | `low`, `high`, `max` | `high`  |
| `kimi-code/k3-256k`                    | `low`, `high`, `max` | `high`  |
| `kimi-code/kimi-for-coding`            | `low`, `high`, `max` | `max`   |
| `kimi-code/kimi-for-coding-highspeed`  | none (always on)    | `on`    |

Models from other providers configured in Kimi Code receive effort strings without client-side
mapping; they were not tested.

## Confirming the Level

Neither `-p` output mode states the level. Read the session record instead:

```text
~/.kimi-code/sessions/wd_<workspace>_<hash>/session_<session-id>/agents/main/wire.jsonl
```

Take the newest `"type":"llm.request"` line and read `thinkingEffort`. That is the level
sent with the request. Do not use the `profile.bind` line for this: it records the level as
configured, and it stayed `low` in a run where `KIMI_MODEL_THINKING_EFFORT=max` made the
request go out at `max`. The session id is in the `session.resume_hint` line of
`--output-format stream-json` output, and `KIMI_CODE_HOME` moves the whole tree. The same
value appears in `logs/kimi-code.log` as `thinkingEffort=<level>`.

## Reasoning Output

In `-p` text mode the reasoning arrives on **stderr** as `• `-prefixed lines, and the answer
on stdout. In `--output-format stream-json` the reasoning is not emitted at all. A wrapper that
needs the reasoning text must use text mode.

## Sources

- [Configuration files](https://moonshotai.github.io/kimi-code/en/configuration/config-files.html)
- [Environment variables](https://moonshotai.github.io/kimi-code/en/configuration/env-vars.html)
- [Config overrides](https://moonshotai.github.io/kimi-code/en/configuration/overrides.html)
- [`kimi` command](https://moonshotai.github.io/kimi-code/en/reference/kimi-command.html)
- [Changelog](https://moonshotai.github.io/kimi-code/en/release-notes/changelog.html)
- [Server API](https://moonshotai.github.io/kimi-code/en/reference/server-api.html)
- [`kimi acp`](https://moonshotai.github.io/kimi-code/en/reference/kimi-acp.html)
- Local: `kimi --help`, `~/.kimi-code/config.toml`, the source embedded in the 2.0.2 binary,
  and disposable `kimi -p` runs under a scratch `KIMI_CODE_HOME`.

## Changelog

- Initial document for Kimi Code CLI 2.0.2 (documentation read at 2.1.1). The provider
  facts file for Kimi lists `--thinking`/`--no-thinking` flags that this version does not have.
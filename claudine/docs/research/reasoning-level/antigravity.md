---
$schema: ./_schema.yaml
schema_revision: 2
provider: antigravity
created: 2026-09-29
last_updated: 2026-09-29
agent: opencode
model: zai-coding-plan/glm-5.3
reasoning_effort: provider_default
versions_examined:
- 1.2.12
evidence:
- claim: the launch flag `--effort` is documented as "Reasoning effort for the current CLI session (low|medium|high|max)"
  id: agy-help-effort
  limitations: help text does not say which models accept which level, nor what the default is
  location: 'host: `agy --help` (Antigravity CLI 1.2.12, ~/.local/bin/agy)'
  method: local_inspection
  observed_on: 2026-09-29
  version: 1.2.12
- claim: the catalog lists effort-suffixed slugs — gemini-3.8/3.7/3.6-flash each as -low/-medium/-high, gemini-3.1-pro as -high/-low, gpt-oss-120b-medium, and claude-sonnet-4-6 and claude-opus-4-6-thinking with no effort variants
  id: models-list
  limitations: a flat list; it does not label defaults and does not say whether unlisted levels are rejected
  location: 'host: `agy models` (Antigravity CLI 1.2.12)'
  method: local_inspection
  observed_on: 2026-09-29
  version: 1.2.12
- claim: 'an unknown effort token is refused before any turn with `invalid model selection (--model "" --effort "bogus"): invalid --effort "bogus" (valid: low, medium, high, max)`, a JSON ERROR envelope, and exit code 1'
  id: invalid-effort-run
  limitations: tested once against the default model; the flag parser rejects the token before per-model checks run
  location: 'disposable temp workspace: `agy -p x --effort bogus --output-format json`'
  method: disposable_test
  observed_on: 2026-09-29
  version: 1.2.12
- claim: an effort-bearing model slug and a disagreeing `--effort` are refused with `--model gemini-3.8-flash-low conflicts with --effort=high` and exit code 1
  id: conflict-run
  limitations: one disagreeing pair exercised; the agreeing pair is covered by gpt-oss-run
  location: 'disposable temp workspace: `agy -p x --model gemini-3.8-flash-low --effort high --output-format json`'
  method: disposable_test
  observed_on: 2026-09-29
  version: 1.2.12
- claim: a family slug without an effort suffix combines with a compatible `--effort` — the run succeeded (status SUCCESS, exit 0)
  id: compose-run
  limitations: one family exercised; the resolved slug is not echoed in the envelope
  location: 'disposable temp workspace: `agy -p "Reply with exactly: apple" --model gemini-3.1-pro --effort low --output-format json`'
  method: disposable_test
  observed_on: 2026-09-29
  version: 1.2.12
- claim: 'gpt-oss-120b answers `has no "high" effort (available: medium)`, and the agreeing pair gpt-oss-120b-medium + --effort medium runs successfully — gpt-oss accepts only medium'
  id: gpt-oss-run
  limitations: high and medium probed; other tokens were not tried against this model
  location: 'disposable temp workspace: `agy -p x --model gpt-oss-120b --effort high` then `--model gpt-oss-120b-medium --effort medium`'
  method: disposable_test
  observed_on: 2026-09-29
  version: 1.2.12
- claim: '`max` parses as a valid token but every probed model rejects it — `gemini-3.1-pro has no "max" effort (available: low, high)` and `gemini-3.8-flash has no "max" effort (available: low, medium, high)`, both exit 1'
  id: max-rejected-run
  limitations: only this account's catalog was probed; another plan tier or a future model may accept it
  location: 'disposable temp workspace: `agy -p x --effort max` and `agy -p x --model gemini-3.8-flash --effort max`'
  method: disposable_test
  observed_on: 2026-09-29
  version: 1.2.12
- claim: models without adjustable reasoning answer `--effort is not supported for model "claude-sonnet-4-6"` with exit code 1
  id: claude-no-effort-run
  limitations: claude-sonnet-4-6 probed; claude-opus-4-6-thinking assumed to match because it appears without effort variants in the catalog
  location: 'disposable temp workspace: `agy -p x --model claude-sonnet-4-6 --effort low --output-format json`'
  method: disposable_test
  observed_on: 2026-09-29
  version: 1.2.12
- claim: '`/effort` as a print-mode prompt is answered by the CLI itself and prints the resolved effort token alone (`high` by default, `low` when launched with `--effort low`), exit 0'
  id: effort-status-run
  limitations: the no-argument form only reports; it does not establish what the interactive gauge persists
  location: 'disposable temp workspace: `agy -p /effort` and `agy -p /effort --effort low`'
  method: disposable_test
  observed_on: 2026-09-29
  version: 1.2.12
- claim: '`/model` as a print-mode prompt prints the resolved slug and display name (`gemini-3.1-pro-high<TAB>Gemini 3.1 Pro (High)`); `--effort low` on the same command line resolves it to `gemini-3.1-pro-low`, proving the flag overrides the effort embedded in the persisted model'
  id: model-status-run
  limitations: reports the invocation's resolution; it does not read back what an earlier, different run used
  location: 'disposable temp workspace: `agy -p /model`, `agy -p /model --effort low`, `agy -p /model --model gemini-3.8-flash --effort medium`'
  method: disposable_test
  observed_on: 2026-09-29
  version: 1.2.12
- claim: the stream `init` event carries `model` only when `--model` was passed and echoes it verbatim (`gemini-3.1-pro` for the family+effort launch); no `effort` field ever appears
  id: stream-init-run
  limitations: text and json output formats were not checked for a model field because their documented envelopes do not contain one
  location: 'disposable temp workspace: three `--output-format stream-json` runs (no model flags; `--model gemini-3.8-flash-high`; `--model gemini-3.1-pro --effort low`)'
  method: disposable_test
  observed_on: 2026-09-29
  version: 1.2.12
- claim: a run with real thinking (usage.thinking_tokens 202) emitted only user_input, agent_response, and checkpoint steps — no event carries reasoning text, which reaches the caller only as `thinking_tokens` counts
  id: stream-no-thought-run
  limitations: one short turn on the default model; a longer tool-using turn follows the same documented event vocabulary
  location: 'disposable temp workspace: `agy -p "Reply with exactly: apple" --output-format stream-json`'
  method: disposable_test
  observed_on: 2026-09-29
  version: 1.2.12
- claim: the persisted selection lives in the `model` key in display-name form ("Gemini 3.1 Pro (High)"), and the file was unchanged after the reporting runs
  id: settings-model-key
  limitations: read-only inspection; whether the interactive `/effort` gauge writes back to this key was not tested
  location: ~/.gemini/antigravity-cli/settings.json (read-only)
  method: local_inspection
  observed_on: 2026-09-29
  version: 1.2.12
- claim: each conversation record embeds the model slug of its run (gemini-3.8-flash-high found in the protobuf metadata blob of the conversation started with that slug)
  id: conversation-db-model
  limitations: the slug sits inside an opaque protobuf blob with no documented stable field, and it echoes the `--model` spelling rather than a resolved effort token
  location: ~/.gemini/antigravity-cli/conversations/<conversation-id>.db (SQLite, read-only)
  method: local_inspection
  observed_on: 2026-09-29
  version: 1.2.12
- claim: the model selector shows Gemini 3.8/3.7/3.6 Flash with Low/Medium/High (Medium selected by default), Gemini 3.1 Pro with Low/High (High default), a "Fast" option beside the flash effort levels, Claude Sonnet 4.6 (Thinking) and Claude Opus 4.6 (Thinking) with no level options, and GPT-OSS 120B fixed at Medium; the selection is sticky within a conversation turn
  id: docs-models-page
  limitations: describes the web-app selector; CLI slugs and rejection behavior had to be confirmed locally
  location: https://antigravity.google/docs/models/
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: the /model command "Choose your preferred reasoning model (persists across sessions)", /fast "Enable fast mode (bypass reasoning plans) for quick actions", and the settings.json keys table documents no effort key
  id: docs-cli-reference
  limitations: the page does not list /effort, which the 1.2.11 changelog and the binary add
  location: https://antigravity.google/docs/cli/reference/
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: the headless reference documents `--effort` (listing only low, medium, high — behind the CLI's low|medium|high|max), unknown `--model` exiting 1 with an ERROR envelope, /model runnable as `agy -p /model`, the init payload's `model` field appearing only when overridden, and `usage.thinking_tokens`
  id: docs-headless
  limitations: the page omits `max` and every model-specific rejection message, which were confirmed by disposable runs
  location: https://antigravity.google/docs/cli/headless/
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: '"Improved reasoning effort level for models with different support, selectable with `--effort` or from the effort gauge in `/effort` and `/model`."'
  id: changelog-effort
  limitations: one changelog line; it does not describe persistence or per-model level sets
  location: 'host: `agy changelog` (bundled release notes), 1.2.11 entry'
  method: local_inspection
  observed_on: 2026-09-29
  version: 1.2.11
support: some_models
levels:
- evidence_ids:
  - agy-help-effort
  - models-list
  - docs-models-page
  meaning: the weak end of the per-model effort scale; accepted by the Gemini flash and Pro families
  native: low
  normalized: low
- evidence_ids:
  - agy-help-effort
  - models-list
  - docs-models-page
  - gpt-oss-run
  meaning: the middle effort point; the default for the flash families and the only level GPT-OSS 120B runs at
  native: medium
  normalized: medium
- evidence_ids:
  - agy-help-effort
  - models-list
  - docs-models-page
  meaning: the strong end of every per-model scale offered today; the Gemini 3.1 Pro default
  native: high
  normalized: high
- evidence_ids:
  - agy-help-effort
  - invalid-effort-run
  - max-rejected-run
  meaning: advertised in the `--effort` flag help and in the invalid-token error's valid set, but no model in the examined catalog accepts it
  native: max
  normalized: maximum
- evidence_ids:
  - docs-cli-reference
  - docs-models-page
  meaning: fast mode bypasses reasoning plans for quick actions; offered beside the effort levels in the model selector and through the /fast command, and not accepted by `--effort`
  native: fast
  normalized: outside_scale
default_level:
  decided_by: model
  evidence_ids:
  - docs-models-page
  - models-list
  - settings-model-key
  - model-status-run
controls:
- arguments:
  - --effort
  - <level>
  changes_running_session: no
  evidence_ids:
  - agy-help-effort
  - changelog-effort
  - effort-status-run
  id: effort-flag
  kind: launch_flag
  lasts: one_session
  launch_modes:
  - interactive
  - non_interactive
  name: --effort
  value: level_token
- arguments:
  - --model
  - <model>
  changes_running_session: no
  evidence_ids:
  - models-list
  - docs-headless
  - compose-run
  - conflict-run
  id: model-flag
  kind: model_suffix
  lasts: one_session
  launch_modes:
  - interactive
  - non_interactive
  name: --model
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - settings-model-key
  - docs-cli-reference
  - model-status-run
  id: settings-model-key
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: model
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - changelog-effort
  - effort-status-run
  id: effort-session-command
  kind: session_command
  lasts: unknown
  launch_modes:
  - interactive
  name: /effort
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - docs-cli-reference
  - docs-models-page
  - model-status-run
  id: model-session-command
  kind: session_command
  lasts: until_changed
  launch_modes:
  - interactive
  name: /model
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - docs-cli-reference
  - docs-models-page
  id: fast-session-command
  kind: session_command
  lasts: unknown
  launch_modes:
  - interactive
  name: /fast
  value: on_or_off
precedence:
- effort-session-command
- model-session-command
- effort-flag
- model-flag
- settings-model-key
models:
- accepts:
  - low
  - medium
  - high
  default: medium
  evidence_ids:
  - docs-models-page
  - models-list
  - max-rejected-run
  model: gemini-*-flash-*
- accepts:
  - low
  - high
  default: high
  evidence_ids:
  - docs-models-page
  - models-list
  - max-rejected-run
  model: gemini-3.1-pro-*
- accepts: []
  evidence_ids:
  - models-list
  - claude-no-effort-run
  model: claude-*
- accepts:
  - medium
  default: medium
  evidence_ids:
  - models-list
  - docs-models-page
  - gpt-oss-run
  model: gpt-oss-120b*
invalid_level:
  behavior: refuses_to_start
  evidence_ids:
  - invalid-effort-run
  - conflict-run
  - max-rejected-run
  - claude-no-effort-run
  message: 'invalid model selection (--model "" --effort "bogus"): invalid --effort "bogus" (valid: low, medium, high, max)'
  warns: yes
reporting:
  command:
  - agy
  - -p
  - /effort
  evidence_ids:
  - effort-status-run
  - model-status-run
  - stream-init-run
  - conversation-db-model
  - docs-headless
  notes: '`agy -p /effort` is answered by the CLI itself and prints the resolved effort token for that invocation (`high` on defaults, `low` when launched with `--effort low`); pass the same `--model`/`--effort` flags a launch used to confirm what that launch resolves to. `agy -p /model` prints the resolved slug with the effort suffix. In stream-json the init event echoes `--model` verbatim (unresolved, absent without the flag) and never reports effort; the per-conversation SQLite record at ~/.gemini/antigravity-cli/conversations/<id>.db embeds the model slug in a protobuf metadata blob; usage.thinking_tokens reports thinking volume, not the level.'
  source: status_command
reasoning_output:
  evidence_ids:
  - stream-no-thought-run
  - docs-headless
  reaches_caller: hidden
gaps:
- area: default_level
  detail: the out-of-box default model and effort on a fresh account is not established; this host's settings.json pins "Gemini 3.1 Pro (High)", so every default observed here is the persisted one
  next_check: sign in on a fresh profile with a pristine ~/.gemini/antigravity-cli and run `agy -p /model`
- area: controls
  detail: how long an interactive `/effort` change stays in force (one session, the conversation, or persisted into settings.json) is not established; only the no-argument report form was exercised
  entry: effort-session-command
  next_check: in an interactive TUI, set a level with /effort, exit, then run `agy -p /effort` and diff settings.json
- area: controls
  detail: how long /fast stays in effect and whether it maps to a model level or only changes agent behavior is not established
  entry: fast-session-command
  next_check: toggle /fast in an interactive TUI, watch the effort gauge in /effort and /model, and restart the session to see whether it persists
- area: levels
  detail: no model in the examined catalog accepts `max`; whether any plan tier, enterprise model, or future release accepts it is unknown
  entry: max
  next_check: run `agy -p x --effort max` against every `agy models` slug on other account tiers
- area: models
  detail: whether custom models (settings.json custom models or the AGY_LLM_GATEWAY_MODELS gateway) accept `--effort` levels is not established
  next_check: configure a custom model per the gateway documentation and run `agy -p hi --model <custom> --effort high`
changes:
- First version of this document; initial research run for the reasoning-level topic.
requires_claudine_update: false
reason: The reasoning-level topic is not yet consumed by Claudine's provider metadata, so nothing here forces a code change today; when the unified effort setting lands, the recorded per-model level sets and the rule that an effort-bearing --model slug must agree with --effort (a conflict refuses to start) are what the antigravity wrapper must honor.
contract_checked: 2026-09-29
---

# Reasoning Level on Antigravity

Antigravity (`agy`, Google's terminal agent CLI) exposes reasoning effort as a
four-token scale on one launch flag, `--effort`, whose meaning depends on the
model family selected. Effort is also baked into the model slugs themselves
(`gemini-3.8-flash-low`), persisted in `settings.json`, adjustable in an
interactive session with `/effort` and `/model`, and complemented by an
off-scale `fast` mode that bypasses reasoning plans.

## Levels

`--effort` advertises `low|medium|high|max`. Which of those a model accepts is
per-family:

| Token   | Normalized   | What it does |
| ------- | ------------ | ------------ |
| `low`   | low          | weakest reasoning on the model's scale (flash and Pro families) |
| `medium`| medium       | flash default; the only level GPT-OSS 120B runs at |
| `high`  | high         | Gemini 3.1 Pro default; top of every scale offered today |
| `max`   | maximum      | accepted by the flag parser but rejected by every catalog model on the examined account |
| `fast`  | outside_scale | bypasses reasoning plans entirely; a mode beside the levels in the selector, reachable via `/fast`, not via `--effort` |

## Choosing a Level

**Launch flag `--effort`** (interactive and headless, lasts one session):

```sh
agy -p "Outline a plan to add caching to this service." --effort high
```

**Model suffix via `--model`** — every Gemini slug carries its effort; the
level is the suffix:

```sh
agy -p "Reverse the string antigravity." --model gemini-3.8-flash-low
```

A family slug without a suffix combines with `--effort`
(`--model gemini-3.1-pro --effort low` runs), but an effort-bearing slug and a
disagreeing `--effort` refuse to start:
`--model gemini-3.8-flash-low conflicts with --effort=high`.

**Configuration key `model`** in `~/.gemini/antigravity-cli/settings.json`,
written by the interactive `/model` picker and read at every launch; it stores
the display-name form (for example `"Gemini 3.1 Pro (High)"`). A launch-time
`--effort` overrides the effort embedded here.

**Session commands** (interactive): `/effort` opens the effort gauge for the
current model, `/model` picks model and effort together and persists the
choice across sessions, and `/fast` toggles fast mode. In headless mode
`/effort` and `/model` degrade to CLI-answered reports (`agy -p /effort`
prints the level), and a `stream-json` input session rejects slash commands
outright.

There is no environment variable for effort and no request field in the
stream-json input protocol; the only non-interactive controls are the two
launch flags and the persisted settings key.

## Models

| Pattern            | Accepts          | Default |
| ------------------ | ---------------- | ------- |
| `gemini-*-flash-*` | low, medium, high | medium  |
| `gemini-3.1-pro-*` | low, high        | high    |
| `claude-*`         | (none — `--effort is not supported for model`) | fixed thinking |
| `gpt-oss-120b*`    | medium           | medium  |

`max` is accepted by none of them on the examined account. A model asked for a
level it does not list answers `has no "<level>" effort (available: …)` and
exits 1 before running a turn.

## Confirming the Level

```sh
agy -p /effort                          # prints e.g. high
agy -p /model --effort low              # prints gemini-3.1-pro-low <TAB> Gemini 3.1 Pro (Low)
```

Both are answered by the CLI itself and report the invocation's resolved
level, so pass the same flags the real launch used. The `stream-json` `init`
event is not sufficient: it echoes `--model` verbatim (so a family slug shows
unresolved) and never carries an effort field. Each conversation's SQLite
record (`~/.gemini/antigravity-cli/conversations/<id>.db`) embeds the model
slug in a protobuf metadata blob. `usage.thinking_tokens` in every output
format reports thinking volume, not the chosen level.

## Sources

- `agy --help`, `agy models`, `agy changelog` — local inspection of agy 1.2.12 on this host
- Disposable headless runs in a temp workspace (invalid token, model/effort conflict, family+effort compose, `max` rejection, Claude rejection, `/effort` and `/model` reports, stream-json `init` and thought events)
- `~/.gemini/antigravity-cli/settings.json` and `conversations/<id>.db` — read-only inspection
- [Models](https://antigravity.google/docs/models/) — per-model level sets, defaults, selector options, stickiness
- [Headless mode](https://antigravity.google/docs/cli/headless/) — `--effort`/`--model` reference, error envelopes, `init` fields, `thinking_tokens`
- [CLI reference](https://antigravity.google/docs/cli/reference/) — `/model` persistence, `/fast`, settings keys
- [Antigravity CLI product page](https://antigravity.google/product/antigravity-cli)

## Changelog

- 2026-09-29 — first version: levels (`low`/`medium`/`high`/`max` plus
  off-scale `fast`), the `--effort` and `--model` launch controls, the
  persisted `model` settings key, the `/effort`, `/model`, and `/fast` session
  commands, per-family level sets and defaults, refusal behavior for invalid
  or unsupported levels, the `/effort` and `/model` status reports, and
  reasoning output confirmed hidden for non-interactive callers.
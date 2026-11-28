---
$schema: ./_schema.yaml
schema_revision: 2
provider: pi
created: 2026-09-29
last_updated: 2026-09-29
agent: claude
model: sonnet
reasoning_effort: high
versions_examined:
- 0.87.1
evidence:
- claim: The launch flag `--thinking <level>` (off, minimal, low, medium, high, xhigh, max), `--model <pattern>` with an optional `:<thinking>` suffix, `--models <patterns>`, and `-p` (non-interactive) exist.
  id: pi-help
  limitations: Help text lists the flags but not clamping, precedence, or per-model support.
  location: '`pi --help` and `pi --version`, run on macOS'
  method: local_inspection
  observed_on: 2026-09-29
  version: 0.87.1
- claim: '`--thinking` sets one of the seven tokens, overrides a `--model` suffix, and is clamped to the model''s capabilities; `--model` and `--models` accept an optional `:<thinking>` suffix.'
  id: docs-cli
  limitations: The hosted page does not say which direction clamping moves, what an unrecognized token does, or which models support which levels.
  location: https://pi.dev/docs/latest/cli
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: '`/thinking` selects the level for the current model, Pi limits the choices to levels the model supports, Ctrl+S in the selector saves the startup level, and a session records thinking-level changes and restores them on resume.'
  id: docs-models
  limitations: The hosted page does not state the default level and does not mention `thinkingLevelMap`; it says nothing about non-interactive use.
  location: https://pi.dev/docs/latest/models
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: The settings `defaultThinkingLevel` (default "medium"), `modelThinkingLevels` (keyed by exact `provider/modelId`), and `thinkingBudgets` (token budgets for minimal, low, medium, high) exist.
  id: docs-settings
  limitations: Package documentation read from the installed npm package, not the hosted site; says nothing about when settings are re-read.
  location: '@earendil-works/pi-coding-agent@0.87.1 docs/settings.md'
  method: official_docs
  observed_on: 2026-09-29
  version: 0.87.1
- claim: '`PI_REASONING_LEVEL` is exported to commands run by the bash tool and holds the current effective reasoning level.'
  id: docs-env
  limitations: Describes an output for tool commands only; it says nothing about the variable being read at startup.
  location: '@earendil-works/pi-coding-agent@0.87.1 docs/environment-variables.md'
  method: official_docs
  observed_on: 2026-09-29
  version: 0.87.1
- claim: RPC mode has `set_thinking_level`, `cycle_thinking_level`, and `get_available_thinking_levels`; the last returns ["off"] for a model without reasoning support; `get_state` carries `thinkingLevel`.
  id: docs-rpc
  limitations: Does not specify what `set_thinking_level` does with an unsupported or unrecognized level.
  location: '@earendil-works/pi-coding-agent@0.87.1 docs/rpc-commands.md'
  method: official_docs
  observed_on: 2026-09-29
  version: 0.87.1
- claim: '`--mode json` emits `thinking_start`, `thinking_delta`, and `thinking_end` events, and a `thinking_level_changed` event carrying `level` when the active level changes.'
  id: docs-json
  limitations: Does not say whether `thinking_level_changed` is emitted at startup.
  location: '@earendil-works/pi-coding-agent@0.87.1 docs/json.md'
  method: official_docs
  observed_on: 2026-09-29
  version: 0.87.1
- claim: A `thinking_level_change` session entry with a `thinkingLevel` field records the level.
  id: docs-session
  limitations: Says the entry is emitted when the user changes the level; does not say one is written at session start.
  location: '@earendil-works/pi-coding-agent@0.87.1 docs/session-format.md'
  method: official_docs
  observed_on: 2026-09-29
  version: 0.87.1
- claim: Sessions are stored under `~/.pi/agent/sessions/` grouped by working directory, unless `--no-session` is given; `--session-dir`, `PI_CODING_AGENT_SESSION_DIR`, or the `sessionDir` setting relocate them.
  id: docs-sessions
  limitations: Does not describe the directory-name encoding or the file-name pattern.
  location: '@earendil-works/pi-coding-agent@0.87.1 docs/sessions.md'
  method: official_docs
  observed_on: 2026-09-29
  version: 0.87.1
- claim: '`app.thinking.cycle` (default shift+tab) cycles the thinking level and `app.thinking.save` (default ctrl+s) saves the current level to settings.'
  id: docs-keybindings
  limitations: Keybindings are user-configurable; the defaults may differ on a host with a `keybindings.json`.
  location: '@earendil-works/pi-coding-agent@0.87.1 docs/keybindings.md'
  method: official_docs
  observed_on: 2026-09-29
  version: 0.87.1
- claim: The seven valid tokens are exactly off, minimal, low, medium, high, xhigh, max, matched with `Array.includes` and so case-sensitively; an unrecognized `--thinking` value pushes a warning diagnostic and is dropped.
  id: src-args
  limitations: Source only; the warning behavior was confirmed separately in test-invalid-flag, case sensitivity was not run.
  location: '@earendil-works/pi-coding-agent@0.87.1 dist/cli/args.js (VALID_THINKING_LEVELS, --thinking parsing)'
  method: source_code
  observed_on: 2026-09-29
  version: 0.87.1
- claim: '`--thinking` overrides a `--model` suffix; a `--model` suffix and scoped-model (`--models`) levels apply only when `--thinking` is absent; scoped-model levels are used for the initial model when `--model` is absent.'
  id: src-main
  limitations: Source only; the runtime outcome was confirmed in test-defaults.
  location: '@earendil-works/pi-coding-agent@0.87.1 dist/main.js (buildSessionOptions)'
  method: source_code
  observed_on: 2026-09-29
  version: 0.87.1
- claim: Startup level resolution is explicit option, then the level restored from a session that already has messages, then `modelThinkingLevels`, then `defaultThinkingLevel`, then the built-in `medium`; the result is clamped to the model, and is `off` when no model resolves.
  id: src-sdk
  limitations: Source only; the settings chain was run in test-defaults, the resumed-session branch was not run.
  location: '@earendil-works/pi-coding-agent@0.87.1 dist/core/sdk.js (createAgentSession)'
  method: source_code
  observed_on: 2026-09-29
  version: 0.87.1
- claim: A model without `reasoning` accepts only off; otherwise a level is dropped when its `thinkingLevelMap` entry is null, and xhigh and max are accepted only when the map names them. An unsupported level moves to the nearest supported level above it, and only when none exists, to the nearest below it.
  id: src-model-clamp
  limitations: Source only; the upward movement was also observed in test-clamp, the downward movement was not run.
  location: '@earendil-works/pi-ai@0.87.1 dist/models.js (getSupportedThinkingLevels, clampThinkingLevel)'
  method: source_code
  observed_on: 2026-09-29
  version: 0.87.1
- claim: For APIs that take a token budget Pi maps minimal, low, medium, high to 1024, 2048, 8192, 16384 tokens by default, and treats xhigh and max as high.
  id: src-budgets
  limitations: Applies only to budget-based APIs; other APIs use the model''s `thinkingLevelMap` values, which were not inspected.
  location: '@earendil-works/pi-ai@0.87.1 dist/api/simple-options.js (DEFAULT_THINKING_BUDGETS, clampReasoning)'
  method: source_code
  observed_on: 2026-09-29
  version: 0.87.1
- claim: '`/thinking <level>` matches case-insensitively against the model''s available levels, `/thinking` with no argument opens a selector, and an unknown argument shows `Unknown thinking level "<text>". Available levels: <list>.`'
  id: src-interactive
  limitations: Not run in a terminal session.
  location: '@earendil-works/pi-coding-agent@0.87.1 dist/modes/interactive/interactive-mode.js (handleThinkingCommand)'
  method: source_code
  observed_on: 2026-09-29
  version: 0.87.1
- claim: Pi deletes any inherited `PI_REASONING_LEVEL` before running a tool command and sets it from the session's level, so it is an output and never an input; no other `PI_` variable in the installed packages selects a level.
  id: src-bash-env
  limitations: Source only; the negative result rests on a search of `process.env.PI_*` reads in the coding-agent, pi-ai, and pi-agent-core dist directories.
  location: '@earendil-works/pi-coding-agent@0.87.1 dist/core/tools/bash.js (resolveSpawnContext)'
  method: source_code
  observed_on: 2026-09-29
  version: 0.87.1
- claim: Per-model `reasoning` flags and `thinkingLevelMap` entries, from which the accepted levels in `models` were computed with the rule in src-model-clamp and grouped into patterns.
  id: pi-model-catalog
  limitations: A cache last modified 2026-09-24 covering only the providers this host has catalogs for; catalogs change with `pi update` and differ per host, and the accepted sets are computed, not asked of each model.
  location: ~/.pi/agent/models-store.json (cached model catalog, `reasoning` and `thinkingLevelMap` fields only)
  method: local_inspection
  observed_on: 2026-09-29
  version: 0.87.1
- claim: 'An unrecognized `--thinking` value prints `Warning: Invalid thinking level "bogus". Valid values: off, minimal, low, medium, high, xhigh, max` to stderr, is ignored, exits 0, and the session records the settings default (here high).'
  id: test-invalid-flag
  limitations: One model on this host, whose settings set `defaultThinkingLevel` to high.
  location: '`pi --offline --session-dir <tmp> --mode json -p --tools read --model kimi-coding/k3 --thinking bogus "Reply with the single word ok." < /dev/null`'
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.87.1
- claim: On k3 (accepts low, high, max) `--thinking off` recorded `low` and `k3:medium` recorded `high`, both with empty stderr and exit 0; an accepted `--thinking low` recorded `low`; `--thinking max` beat the `:low` suffix and recorded `max`.
  id: test-clamp
  limitations: One model; nothing was observed on the wire, so what the API received is inferred from the recorded level and the model catalog.
  location: '`pi --offline --session-dir <tmp> --mode json -p --tools read` with `--model kimi-coding/k3 --thinking off`, `--model kimi-coding/k3:medium`, `--model kimi-coding/k3 --thinking low`, and `--model kimi-coding/k3:low --thinking max`; result read from the `thinking_level_change` entry of each session file'
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.87.1
- claim: 'An unqualified `--model k3:bogus` exits 1 with `Error: Model "k3:bogus" not found. Use --list-models to see available models.`; the provider-qualified `--model kimi-coding/k3:bogus` exits 0 with `Warning: Model "k3:bogus" not found for provider "kimi-coding". Using custom model id.`, sends `k3:bogus` as the model id, and the request fails with a 401 authentication_error (stopReason error) while the session records level high.'
  id: test-invalid-suffix
  limitations: One model; an invalid suffix on a model that does resolve was not tested.
  location: '`pi --offline --mode json -p --tools read --model k3:bogus` and `--model kimi-coding/k3:bogus`, stdin from /dev/null'
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.87.1
- claim: '`get_state` returns `data.thinkingLevel` (high at start); `get_available_thinking_levels` returned [low, high, max]; `set_thinking_level off` succeeded, emitted `thinking_level_changed` with level low, and the level became low; `set_thinking_level bogus` returned success and left the level low.'
  id: test-rpc
  limitations: One model; RPC gives no warning for a clamped or ignored request.
  location: '`pi --offline --mode rpc --no-session --model kimi-coding/k3` sent get_state, get_available_thinking_levels, set_thinking_level off, get_state, set_thinking_level bogus, get_state'
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.87.1
- claim: With no settings a reasoning model starts at medium and a non-reasoning model at off; `defaultThinkingLevel` low gave low; `modelThinkingLevels` high beat it; a `:minimal` suffix beat both; `--thinking off` beat the suffix; `--models fake/m1:low` without `--model` gave low; a project `.pi/settings.json` (minimal) applied with `-a` and was ignored with `-na` and with neither flag; an exported `PI_REASONING_LEVEL=max` had no effect.
  id: test-defaults
  limitations: Custom-provider model with no `thinkingLevelMap`; the request never reached a network because `get_state` does not call the model.
  location: '`PI_CODING_AGENT_DIR=<tmp> pi --offline --mode rpc --no-session --model fake/m1` with a temporary models.json (custom provider, one reasoning and one non-reasoning model) and temporary settings.json variants; `get_state` read back'
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.87.1
- claim: Rewriting `defaultThinkingLevel` and `modelThinkingLevels` in settings.json after startup did not change the running level (low), and a later `set_model` applied the `modelThinkingLevels` value read at startup (high), not the rewritten one (minimal).
  id: test-settings-cache
  limitations: One run; timing relies on the rewrite landing after Pi read its settings, which the unchanged level confirms.
  location: '`PI_CODING_AGENT_DIR=<tmp> pi --offline --mode rpc --no-session --model fake/m1`; settings.json rewritten after startup, then get_state, set_model to fake/m3, get_state'
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.87.1
- claim: The stream carried one thinking_start, 14 thinking_delta, and one thinking_end event and a `usage.reasoning` token count (37); no `thinking_level_changed` event was emitted at startup; every session file written in these runs held a `thinking_level_change` entry.
  id: test-json-reasoning
  limitations: One model (kimi-coding/k3); other providers may return summaries or redacted blocks. Plain `-p` output was not re-run; the earlier finding that it prints only the final answer rests on docs-json.
  location: '`pi --offline --no-session --mode json -p --tools read --model kimi-coding/k3:high "What is 17*23? Reply with number only." < /dev/null`'
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.87.1
support: some_models
levels:
- evidence_ids:
  - src-args
  - src-model-clamp
  - test-defaults
  - test-rpc
  meaning: Disables reasoning. Only models whose `thinkingLevelMap` does not null it accept it; a model without reasoning accepts only this level.
  native: off
  normalized: off
- evidence_ids:
  - src-args
  - src-budgets
  - pi-model-catalog
  meaning: Smallest reasoning setting; a 1024-token budget on budget-based APIs.
  native: minimal
  normalized: minimal
- evidence_ids:
  - src-args
  - src-budgets
  - test-clamp
  meaning: Light reasoning; a 2048-token budget on budget-based APIs.
  native: low
  normalized: low
- evidence_ids:
  - src-args
  - src-budgets
  - test-defaults
  meaning: Pi's built-in default; an 8192-token budget on budget-based APIs.
  native: medium
  normalized: medium
- evidence_ids:
  - src-args
  - src-budgets
  - test-clamp
  meaning: Heavy reasoning; a 16384-token budget on budget-based APIs.
  native: high
  normalized: high
- evidence_ids:
  - src-args
  - src-model-clamp
  - src-budgets
  - pi-model-catalog
  meaning: Above high, accepted only where the model's `thinkingLevelMap` names it; treated as high on budget-based APIs.
  native: xhigh
  normalized: very_high
- evidence_ids:
  - src-args
  - src-model-clamp
  - src-budgets
  - test-clamp
  meaning: Strongest setting, accepted only where the model's `thinkingLevelMap` names it; treated as high on budget-based APIs.
  native: max
  normalized: maximum
default_level:
  decided_by: model
  evidence_ids:
  - src-sdk
  - docs-settings
  - test-defaults
  - test-clamp
  native: medium
controls:
- arguments:
  - --thinking
  - <level>
  changes_running_session: no
  evidence_ids:
  - pi-help
  - docs-cli
  - src-args
  - src-main
  - test-clamp
  - test-defaults
  id: thinking-flag
  kind: launch_flag
  lasts: one_session
  launch_modes:
  - interactive
  - non_interactive
  name: --thinking
  value: level_token
- arguments:
  - --model
  - <model>:<level>
  changes_running_session: no
  evidence_ids:
  - pi-help
  - docs-cli
  - src-main
  - test-clamp
  - test-invalid-suffix
  - test-defaults
  id: model-suffix
  kind: model_suffix
  lasts: one_session
  launch_modes:
  - interactive
  - non_interactive
  name: --model
  value: level_token
- arguments:
  - --models
  - <model>:<level>
  changes_running_session: unknown
  evidence_ids:
  - pi-help
  - docs-cli
  - src-main
  - test-defaults
  id: models-suffix
  kind: model_suffix
  lasts: one_session
  launch_modes:
  - interactive
  - non_interactive
  name: --models
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-settings
  - src-sdk
  - test-defaults
  - test-settings-cache
  id: model-thinking-levels
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: modelThinkingLevels
  value: level_token
- arguments: []
  changes_running_session: no
  evidence_ids:
  - docs-settings
  - src-sdk
  - test-defaults
  - test-settings-cache
  id: default-thinking-level
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: defaultThinkingLevel
  value: level_token
- arguments: []
  changes_running_session: unknown
  evidence_ids:
  - docs-settings
  - src-budgets
  id: thinking-budgets
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: thinkingBudgets
  value: token_budget
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - docs-models
  - docs-keybindings
  - src-interactive
  id: thinking-command
  kind: session_command
  lasts: one_session
  launch_modes:
  - interactive
  name: /thinking
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - docs-rpc
  - test-rpc
  id: rpc-set-thinking-level
  kind: request_field
  lasts: one_session
  launch_modes:
  - non_interactive
  name: set_thinking_level
  value: level_token
precedence:
- rpc-set-thinking-level
- thinking-command
- thinking-flag
- model-suffix
- models-suffix
- model-thinking-levels
- default-thinking-level
models:
- accepts:
  - minimal
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: anthropic/claude-fable-5*
- accepts:
  - minimal
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: anthropic/claude-opus-5
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: anthropic/claude-opus-5-5
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: anthropic/claude-haiku-4-5*
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: anthropic/claude-opus-4-5*
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: anthropic/claude-sonnet-4-5*
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  - max
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: anthropic/claude-opus-4-6
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  - max
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: anthropic/claude-sonnet-4-6
- accepts:
  - off
  - high
  - max
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: deepseek/deepseek-v4-pro
- accepts:
  - off
  - low
  - high
  - max
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: deepseek/deepseek-flash
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-2.5-*
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-3.1-pro-preview*
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-3.5-flash*
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-3.7-flash
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-3.8-flash
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: groq/openai/gpt-oss-*
- accepts:
  - off
  default: off
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: groq/llama-*
- accepts:
  - low
  - high
  - max
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  - test-clamp
  model: kimi-coding/k3*
- accepts:
  - low
  - high
  - max
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: kimi-coding/kimi-for-coding
- accepts:
  - low
  - high
  - max
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: moonshotai*/kimi-k3
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: moonshotai*/kimi-k2.7-code*
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: mistral/magistral-*
- accepts:
  - off
  default: off
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: mistral/codestral-*
- accepts:
  - off
  default: off
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-4*
- accepts:
  - high
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5-pro
- accepts:
  - medium
  - high
  - xhigh
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5.*-pro
- accepts:
  - off
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5.5
- accepts:
  - off
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5.6-*
- accepts:
  - off
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-6-luna
- accepts:
  - off
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-6-sol
- accepts:
  - low
  - medium
  - high
  - xhigh
  - max
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-6-astra
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/o1*
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/o3*
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai-codex/gpt-5.3-codex-spark
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai-codex/gpt-5.4*
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai-codex/gpt-5.5
- accepts:
  - low
  - high
  - max
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: zai/glm-5.3*
- accepts:
  - off
  - high
  - max
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: zai/glm-5.2*
invalid_level:
  behavior: uses_nearest_level
  evidence_ids:
  - test-clamp
  - test-rpc
  - test-invalid-flag
  - test-invalid-suffix
  - src-model-clamp
  warns: no
reporting:
  evidence_ids:
  - test-clamp
  - test-rpc
  - test-json-reasoning
  - docs-session
  - docs-sessions
  - docs-env
  - src-bash-env
  field: /thinkingLevel
  locator: ~/.pi/agent/sessions/<cwd-slug>/<timestamp>_<session-id>.jsonl
  notes: The record is the `thinking_level_change` entry (`type` field); the last one in the file is the effective level, and one was present in every session file these runs wrote, including a launch-time level. Pi emits the `thinking_level_changed` stream event only when the level changes, not at startup, so a launch-time level is visible only in the file. `--no-session` writes no file; `--session-dir`, PI_CODING_AGENT_SESSION_DIR, or the `sessionDir` setting moves it. In RPC mode `get_state` returns the level at `/data/thinkingLevel`, and the bash tool sees it in PI_REASONING_LEVEL.
  source: session_record
reasoning_output:
  evidence_ids:
  - test-json-reasoning
  - docs-json
  reaches_caller: full_text
gaps:
- area: controls
  detail: The launch path of a `--models` suffix was confirmed (it sets the startup level when `--model` is absent), but whether Ctrl+P cycling applies the scoped level to the already-running session was not run.
  entry: models-suffix
  next_check: Start pi in a detached tmux session with `--models kimi-coding/k3:low,kimi-coding/k3-256k:max`, press Ctrl+P, and read the last `thinking_level_change` entry in the session file.
- area: controls
  detail: Whether editing `thinkingBudgets` reaches a running session was not tested; `modelThinkingLevels` and `defaultThinkingLevel` were tested and are read once at startup, but the budget setting may be read per request.
  entry: thinking-budgets
  next_check: Start an RPC session against a local echo server, rewrite `thinkingBudgets` in the temporary agent directory, send a second prompt, and compare the request bodies.
- area: reporting
  detail: '`AssistantMessage.providerThinkingLevel` is documented as preserving a provider detail but was absent in every observed run, so nothing observed confirms what level or budget actually reached the provider''s API.'
  next_check: Route a model through a local echo server with a `models.json` `baseUrl` and compare the request body''s effort or budget field with the recorded session level.
- area: reasoning_output
  detail: Full reasoning text was observed only for kimi-coding/k3; `ThinkingContent` can carry redacted blocks, so OpenAI, Codex, and Anthropic models may return a summary or nothing.
  next_check: Run `pi --mode json -p --thinking high` against an anthropic and an openai-codex model and compare the `thinking_end` content.
- area: models
  detail: The accepted sets are computed from the `reasoning` flag and `thinkingLevelMap` in this host's cached catalog and grouped by pattern; they were not asked of each model, and other hosts or later `pi update` runs may list different models. Only kimi-coding/k3 was confirmed at run time (`get_available_thinking_levels`).
  next_check: Run RPC `get_available_thinking_levels` for one model per pattern after `pi update`, and diff the result against this list.
- area: other
  detail: Only macOS with pi 0.87.1 was examined; Linux and Windows behavior, and hosted-docs pages other than cli and models, were not checked.
  next_check: Rerun the RPC `get_available_thinking_levels` and `--thinking bogus` checks on Linux and Windows.
changes:
- 'Migrated from schema revision 1 to revision 2: gaps now use `area` and `entry` instead of `subject`, every evidence entry carries `location`, `version`, and `observed_on`, and `normalized` and `changes_running_session` values are quoted where YAML would read them as booleans.'
- Re-ran every disposable test on 0.87.1 (invalid flag, clamp, suffix precedence, RPC, settings chain, JSON reasoning) and reproduced the earlier findings.
- 'Added a finding: a provider-qualified model with an unrecognized suffix (`kimi-coding/k3:bogus`) does not fail at startup; Pi treats `k3:bogus` as a custom model id, exits 0 with a warning, and the request then fails at the API.'
- 'Confirmed by run rather than source: a `--models` suffix sets the startup level when `--model` is absent, and a project `.pi/settings.json` is not applied without `-a`.'
- 'Closed a gap: `modelThinkingLevels` and `defaultThinkingLevel` are read once at startup; editing settings.json mid-session changes nothing, including on a later model switch.'
- Added the `thinkingBudgets` setting as a control with a token-budget value, and Ctrl+S / Shift+Tab as documented `/thinking` companions.
- Reduced `models` from 96 catalog entries to 38 patterns, listing only models whose levels differ from all seven tokens, and removed the unverified claim that some providers map `minimal` to their `low`.
- Fetched the hosted pi.dev cli and models pages; they confirm the flags and `/thinking` but state neither the default level nor `thinkingLevelMap`.
requires_claudine_update: true
reason: Pi accepts seven tokens with per-model support and silently clamps a level the model does not accept to the nearest one above it, so Claudine must map its scale to `--thinking <level>` using the model's own accepted list (RPC `get_available_thinking_levels`), treat the stderr warning for an unrecognized token as an error because Pi still exits 0, and confirm the outcome from the session record rather than the exit code.
contract_checked: 2026-09-29
---

## Levels

Pi calls the setting the *thinking level*; the environment variable it exports to
tools calls it the *reasoning level*. It accepts seven tokens, weakest first:
`off`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max`. Matching is exact
and, per source, case-sensitive.

| Token | Claudine scale | Meaning |
| --- | --- | --- |
| `off` | off | No reasoning; only where the model allows it |
| `minimal` | minimal | 1024-token budget on budget-based APIs |
| `low` | low | 2048 tokens |
| `medium` | medium | Pi's built-in default; 8192 tokens |
| `high` | high | 16384 tokens |
| `xhigh` | very_high | Only where the model names it; budget APIs treat it as `high` |
| `max` | maximum | Only where the model names it; budget APIs treat it as `high` |

No mode falls outside the scale. The budgets are defaults; the
`thinkingBudgets` setting overrides them for `minimal` through `high`.

Not every model accepts every token. Each catalog model declares a `reasoning`
flag and a `thinkingLevelMap`. Pi computes the accepted set from them: a model
without `reasoning` accepts only `off`; a level mapped to `null` is dropped;
`xhigh` and `max` are accepted only when the map names them. A level the model
does not accept is **clamped**, not refused: Pi moves to the nearest accepted
level above the request, and only when none exists, below it.

The default is `medium` (`defaultThinkingLevel` overrides it), then clamped to
the model. On a model that accepts only `low`, `high`, `max`, such as k3, the
default therefore lands on `high`; on a non-reasoning model it is `off`.

## Choosing a Level

Controls, strongest first. The first two only exist inside a running session.

| Control | Where | Example |
| --- | --- | --- |
| `set_thinking_level` | RPC request (`pi --mode rpc`) | `{"type":"set_thinking_level","level":"high"}` |
| `/thinking [level]` | Interactive session | `/thinking high` (no argument opens a selector; Shift+Tab cycles; Ctrl+S in the selector saves it) |
| `--thinking <level>` | Launch flag | `pi -p --thinking high "prompt"` |
| `:<level>` suffix | `--model` | `pi -p --model anthropic/claude-sonnet-5:high "prompt"` |
| `:<level>` suffix | `--models` | `pi --models sonnet:high,haiku:low` |
| `modelThinkingLevels` | `settings.json`, keyed `provider/modelId` | `{"modelThinkingLevels":{"anthropic/claude-sonnet-5":"high"}}` |
| `defaultThinkingLevel` | `settings.json` | `{"defaultThinkingLevel":"low"}` |
| `thinkingBudgets` | `settings.json`, token counts | `{"thinkingBudgets":{"high":20000}}` (adjusts a level's budget; does not choose a level) |

Non-interactive example, with stdin redirected:

```sh
pi --offline --no-session -p --thinking high "Summarize README.md" < /dev/null
```

Precedence was measured, not assumed. `--thinking` beat a `--model` suffix,
which beat `modelThinkingLevels`, which beat `defaultThinkingLevel`, which beats
the built-in `medium`. A `--models` suffix set the startup level when `--model`
was absent. Between the flag and the settings, a resumed session restores its
recorded level (source only). A session command or RPC request changes the level
after launch.

- A project `.pi/settings.json` overrides `~/.pi/agent/settings.json`
  (`PI_CODING_AGENT_DIR` relocates the latter) only when the project is trusted
  with `-a`; `-na` and no flag both ignored it in a non-interactive run.
- Settings are read once at startup. Editing `settings.json` mid-session changed
  nothing, even after a later model switch.
- No environment variable sets the level. `PI_REASONING_LEVEL` is exported to
  commands the bash tool runs; Pi deletes any inherited value first, and
  exporting `max` before launch had no effect.

An unrecognized token is handled differently from a valid token the model does
not accept:

| Request | Result |
| --- | --- |
| `--thinking bogus` | stderr `Warning: Invalid thinking level "bogus". Valid values: off, minimal, low, medium, high, xhigh, max`; token ignored; exit 0 at the configured default |
| `--model k3:bogus` (unqualified) | stderr `Error: Model "k3:bogus" not found. ...`; exit 1 |
| `--model kimi-coding/k3:bogus` (qualified) | stderr `Warning: ... Using custom model id.`; exit 0; the request fails at the API with a 401 |
| `--thinking off` on a model that lacks `off` | clamped upward, no message, exit 0 |

## Models

The accepted set comes from the model catalog, so it varies by provider and
version. The `models` frontmatter lists 38 patterns, only for models that do not
accept all seven tokens; `claude-opus-4-7`, `claude-opus-4-8`, and
`claude-sonnet-5` accept all seven. Highlights:

- Anthropic 4.5 models: `off` through `high`; 4.6 adds `max`; the Fable models,
  `claude-opus-5`, and `claude-opus-5-5` lack `off` (and the last lacks `minimal`).
- OpenAI GPT-5.x: `off`, `low`, `medium`, `high`, `xhigh`, no `minimal`; 5.6 and
  6.x add `max`; `-pro` models accept only `medium`, `high`, `xhigh`.
- OpenAI Codex provider: the same generation accepts `minimal` too.
- Google Gemini 2.5: `off` through `high`; Gemini 3.x drops `off`, and some drop
  `minimal`.
- Kimi k3 and GLM 5.3: only `low`, `high`, `max`; GLM 5.2 and DeepSeek V4 Pro
  accept `off`, `high`, `max`.
- Non-reasoning models (GPT-4 family, Llama, Codestral): `off` only.
- A custom `models.json` model with `reasoning: true` and no `thinkingLevelMap`
  accepts `off` through `high` and starts at `medium`.

The list was computed from this host's cached catalog. Ask Pi for the truth at
run time: RPC `get_available_thinking_levels` returns the current model's list
(`["low","high","max"]` on k3).

## Confirming the Level

Clamping is silent, so confirm the effective level after the request.

- **Session file.** The last `thinking_level_change` entry in the session JSONL
  (`thinkingLevel` field) is the effective level, and one was present in every
  session these runs wrote. Sessions live under
  `~/.pi/agent/sessions/<cwd-slug>/`; `--no-session` writes none, so use
  `--session-dir <tmp>` for a disposable run.
- **RPC.** `get_state` returns `data.thinkingLevel`; a change also emits a
  `thinking_level_changed` event.
- **Tool commands.** The bash tool sees `PI_REASONING_LEVEL`.
- **Not reported.** The JSON stream emits `thinking_level_changed` only on a change,
  not at startup, so a launch-time level does not appear in it. Nothing observed
  shows what reached the provider's API.

Observed on kimi-coding/k3, which accepts `low`, `high`, `max`:

| Request | Recorded level | stderr |
| --- | --- | --- |
| `--thinking off` | `low` | empty |
| `--model kimi-coding/k3:medium` | `high` | empty |
| `--thinking low` | `low` | empty |
| `--model kimi-coding/k3:low --thinking max` | `max` | empty |
| `--thinking bogus` | `high` (settings default) | invalid-level warning |

Reasoning reaches a non-interactive caller only with `--mode json`, as
`thinking_start`, `thinking_delta`, and `thinking_end` events and a
`usage.reasoning` token count. Observed as full text for k3; other providers may
return summaries or redacted blocks.

## Sources

- [Pi CLI reference](https://pi.dev/docs/latest/cli)
- [Pi models and thinking](https://pi.dev/docs/latest/models)
- `pi --help` and `pi --version` (0.87.1), run locally
- Installed package `@earendil-works/pi-coding-agent@0.87.1`: `docs/settings.md`,
  `docs/environment-variables.md`, `docs/rpc-commands.md`, `docs/json.md`,
  `docs/session-format.md`, `docs/sessions.md`, `docs/keybindings.md`, and
  `dist/cli/args.js`, `dist/main.js`, `dist/core/sdk.js`,
  `dist/core/tools/bash.js`, `dist/modes/interactive/interactive-mode.js`
- Installed package `@earendil-works/pi-ai@0.87.1`: `dist/models.js`,
  `dist/api/simple-options.js`
- Local model catalog cache `~/.pi/agent/models-store.json` (read for
  `reasoning` and `thinkingLevelMap` only)
- Disposable runs with a temporary session directory and a temporary
  `PI_CODING_AGENT_DIR`; the user's configuration was not changed

## Changelog

- Migrated to schema revision 2 (gaps use `area` and `entry`; evidence entries
  carry `location`, `version`, and `observed_on`).
- Re-ran every disposable test on 0.87.1 and reproduced the earlier findings.
- New: a provider-qualified model with a bad suffix (`kimi-coding/k3:bogus`)
  exits 0 with a warning and fails at the API; the unqualified form still exits 1.
- New: a `--models` suffix sets the startup level without `--model` (confirmed by
  run); a project `.pi/settings.json` needs `-a`.
- Closed a gap: settings are read once at startup and a mid-session edit changes
  nothing, even after a model switch.
- Added `thinkingBudgets` as a control and documented the Shift+Tab and Ctrl+S
  companions of `/thinking`.
- `models` shrank from 96 entries to 38 patterns; the unverified claim that some
  providers map `minimal` to `low` was removed.
- The hosted pi.dev pages confirm the flags and `/thinking` but not the default
  level or `thinkingLevelMap`.
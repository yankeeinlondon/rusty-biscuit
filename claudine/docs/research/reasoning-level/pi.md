---
$schema: ./_schema.yaml
schema_revision: 1
provider: pi
created: 2026-09-29
last_updated: 2026-09-29
agent: claude
model: sonnet
reasoning_effort: high
versions_examined:
- 0.87.1
evidence:
- claim: The launch flags `--thinking <level>` (off, minimal, low, medium, high, xhigh, max) and `--model <pattern>` with an optional `:<thinking>` suffix exist, and `pi -p` is the non-interactive mode.
  id: pi-help
  limitations: Help text lists the flags but not clamping, precedence, or per-model support.
  location: '`pi --help` output, run on macOS'
  method: local_inspection
  observed_on: 2026-09-29
  version: 0.87.1
- claim: '`--thinking` sets one of the seven tokens, overrides a `--model` suffix, and is clamped to the model''s capabilities; `--models` also accepts `:<thinking>` suffixes.'
  id: docs-cli
  limitations: The page does not say which direction clamping moves or what happens to an unrecognized token.
  location: https://pi.dev/docs/latest/cli
  method: official_docs
  observed_on: 2026-09-29
  version: unknown
- claim: Settings `defaultThinkingLevel` (default "medium"), `modelThinkingLevels` (keyed by exact provider/modelId), and `thinkingBudgets` exist; project settings override agent-directory settings.
  id: docs-settings
  limitations: Package documentation read from the installed npm package, not the hosted site.
  location: '@earendil-works/pi-coding-agent@0.87.1 docs/settings.md'
  method: official_docs
  observed_on: 2026-09-29
  version: 0.87.1
- claim: '`/thinking` selects the level for the current model, Pi limits the choices to levels the model supports, and a session records thinking-level changes.'
  id: docs-models
  limitations: Does not describe non-interactive use.
  location: '@earendil-works/pi-coding-agent@0.87.1 docs/models.md'
  method: official_docs
  observed_on: 2026-09-29
  version: 0.87.1
- claim: '`PI_REASONING_LEVEL` is exported to commands run by the bash tool and holds the current effective reasoning level.'
  id: docs-env
  limitations: Describes an output for tool commands only; says nothing about it being read at startup.
  location: '@earendil-works/pi-coding-agent@0.87.1 docs/environment-variables.md'
  method: official_docs
  observed_on: 2026-09-29
  version: 0.87.1
- claim: RPC mode has `set_thinking_level`, `cycle_thinking_level`, and `get_available_thinking_levels`; the last returns ["off"] for a model without reasoning support.
  id: docs-rpc
  limitations: Does not specify what set_thinking_level does with an unsupported level.
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
- claim: A `thinking_level_change` session entry with a `thinkingLevel` field is written when the level changes.
  id: docs-session
  limitations: Does not say an entry is written at session start.
  location: '@earendil-works/pi-coding-agent@0.87.1 docs/session-format.md'
  method: official_docs
  observed_on: 2026-09-29
  version: 0.87.1
- claim: The seven valid tokens are exactly off, minimal, low, medium, high, xhigh, max, matched case-sensitively; an invalid `--thinking` value pushes a warning diagnostic and is dropped.
  id: src-args
  limitations: Source only; behavior confirmed separately by disposable tests.
  location: '@earendil-works/pi-coding-agent@0.87.1 dist/cli/args.js (VALID_THINKING_LEVELS, --thinking parsing)'
  method: source_code
  observed_on: 2026-09-29
  version: 0.87.1
- claim: '`--thinking` overrides a `--model` suffix, which overrides a scoped-model (`--models`) level; scoped-model levels apply only when `--model` is absent.'
  id: src-main
  limitations: Source only; the `--models` branch was not run.
  location: '@earendil-works/pi-coding-agent@0.87.1 dist/main.js (buildSessionOptions)'
  method: source_code
  observed_on: 2026-09-29
  version: 0.87.1
- claim: Startup level resolution is explicit option, then the restored session level, then `modelThinkingLevels`, then `defaultThinkingLevel`, then the built-in `medium`; the result is clamped to the model, and is `off` when no model resolves.
  id: src-sdk
  limitations: Source only; the settings chain was also run (see test-defaults).
  location: '@earendil-works/pi-coding-agent@0.87.1 dist/core/sdk.js (createAgentSession)'
  method: source_code
  observed_on: 2026-09-29
  version: 0.87.1
- claim: '`setThinkingLevel` clamps to model capabilities, writes a `thinking_level_change` entry only when the effective level changes, and emits a `thinking_level_changed` event.'
  id: src-session
  limitations: Source only.
  location: '@earendil-works/pi-coding-agent@0.87.1 dist/core/agent-session.js (setThinkingLevel, getAvailableThinkingLevels)'
  method: source_code
  observed_on: 2026-09-29
  version: 0.87.1
- claim: A model without `reasoning` accepts only off; otherwise a level is dropped when `thinkingLevelMap[level]` is null, and xhigh and max are accepted only when the map names them. An unsupported level moves to the nearest supported level above it, then below it.
  id: src-model-clamp
  limitations: Source only; the upward movement was also observed (see test-clamp).
  location: '@earendil-works/pi-ai@0.87.1 dist/models.js (getSupportedThinkingLevels, clampThinkingLevel)'
  method: source_code
  observed_on: 2026-09-29
  version: 0.87.1
- claim: For token-budget providers Pi maps minimal, low, medium, high to 1024, 2048, 8192, 16384 tokens by default, and treats xhigh and max as high.
  id: src-budgets
  limitations: Applies only to APIs that take a budget; other APIs use `thinkingLevelMap` values instead.
  location: '@earendil-works/pi-ai@0.87.1 dist/api/simple-options.js (DEFAULT_THINKING_BUDGETS, clampReasoning)'
  method: source_code
  observed_on: 2026-09-29
  version: 0.87.1
- claim: '`/thinking <level>` matches case-insensitively against the model''s available levels and otherwise shows the error `Unknown thinking level "<text>". Available levels: <list>.`'
  id: src-interactive
  limitations: Not run in a terminal session.
  location: '@earendil-works/pi-coding-agent@0.87.1 dist/modes/interactive/interactive-mode.js (handleThinkingCommand)'
  method: source_code
  observed_on: 2026-09-29
  version: 0.87.1
- claim: Pi deletes any inherited `PI_REASONING_LEVEL` before running a tool command and sets it from the session's level, so it is an output and never an input.
  id: src-bash-env
  limitations: Source only; no other source reads the variable.
  location: '@earendil-works/pi-coding-agent@0.87.1 dist/core/tools/bash.js (resolveSpawnContext)'
  method: source_code
  observed_on: 2026-09-29
  version: 0.87.1
- claim: Per-model `reasoning` flags and `thinkingLevelMap` entries, from which the accepted levels in `models` were computed with the rule in src-model-clamp.
  id: pi-model-catalog
  limitations: A cache last refreshed 2026-09-24 for the providers this host has catalogs for; catalogs change with `pi update` and differ per host.
  location: ~/.pi/agent/models-store.json (cached model catalog, `reasoning` and `thinkingLevelMap` fields only)
  method: local_inspection
  observed_on: 2026-09-29
  version: 0.87.1
- claim: 'An unrecognized `--thinking` value prints `Warning: Invalid thinking level "bogus". Valid values: off, minimal, low, medium, high, xhigh, max` to stderr, is ignored, the run proceeds with exit 0 on the settings default (here high), and uppercase `XHIGH` is also rejected.'
  id: test-invalid-flag
  limitations: Run on one model with this host's settings (`defaultThinkingLevel` high).
  location: '`pi --offline --session-dir <tmp> --mode json -p --thinking bogus` and `--thinking XHIGH`, model kimi-coding/k3, one trivial prompt each, no tools but read'
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.87.1
- claim: On k3 (accepts low, high, max) `--thinking off` recorded `low` and `k3:medium` recorded `high` with an empty stderr; an accepted `--thinking low` recorded `low`; `--thinking max` beat the `:low` suffix and recorded `max`; usage.reasoning tokens were nonzero in each run.
  id: test-clamp
  limitations: One model; nothing observed on the wire, so what the API received is inferred from the session entry and the model's `thinkingLevelMap`.
  location: '`pi --offline --session-dir <tmp> --mode json -p` with `--thinking off`, `--model k3:medium`, `--thinking low`, `--model k3:low --thinking max`, model kimi-coding/k3; result read from the session JSONL'
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.87.1
- claim: 'An unrecognized suffix makes the whole model pattern unresolvable: stderr `Error: Model "k3:bogus" not found. Use --list-models to see available models.`, exit 1, no session file.'
  id: test-invalid-suffix
  limitations: Single model pattern.
  location: '`pi --offline --mode json -p --model k3:bogus`'
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.87.1
- claim: '`get_state` returns `data.thinkingLevel`; `get_available_thinking_levels` returned [low, high, max]; `set_thinking_level off` succeeded and the level became low (with a `thinking_level_changed` event, level low); `set_thinking_level bogus` returned success and left the level unchanged.'
  id: test-rpc
  limitations: One model; RPC does not report a warning for the clamped or ignored request.
  location: '`pi --offline --mode rpc --no-session` sent get_state, get_available_thinking_levels, set_thinking_level off, set_thinking_level bogus, model kimi-coding/k3'
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.87.1
- claim: With no settings a reasoning model starts at medium (levels off, minimal, low, medium, high) and a non-reasoning model at off; `defaultThinkingLevel` low gave low; `modelThinkingLevels` high beat it; a `:minimal` suffix beat both; `--thinking` beat the suffix and `modelThinkingLevels`; a project `.pi/settings.json` applied with `-a` and was ignored with `-na`; an exported `PI_REASONING_LEVEL=high` had no effect.
  id: test-defaults
  limitations: Custom-provider model with no `thinkingLevelMap`; the request never reached a network because get_state does not call the model.
  location: '`PI_CODING_AGENT_DIR=<tmp> pi --offline --mode rpc --no-session --model fake/m1` with a temporary models.json (custom provider, one reasoning and one non-reasoning model) and temporary settings.json variants; `get_state` read back'
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.87.1
- claim: In `--mode json` the stream carried thinking_start, 21 thinking_delta, and thinking_end events with readable reasoning text and a `reasoning` token count in usage; plain `-p` printed only the final answer; no `thinking_level_changed` event was emitted at startup.
  id: test-json-reasoning
  limitations: One model (kimi-coding/k3); other providers may return summaries or redacted blocks.
  location: '`pi --offline --mode json -p --model k3:high` and the same run without `--mode json`'
  method: disposable_test
  observed_on: 2026-09-29
  version: 0.87.1
support: some_models
levels:
- evidence_ids:
  - src-args
  - src-model-clamp
  - test-defaults
  meaning: Disables reasoning. Only models whose `thinkingLevelMap` does not null it accept it; a model without reasoning accepts only this level.
  native: off
  normalized: off
- evidence_ids:
  - src-args
  - src-budgets
  - pi-model-catalog
  meaning: Smallest reasoning setting; a 1024-token budget on budget-based APIs. Some providers map it to their `low`.
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
  meaning: Above high, accepted only where the model's `thinkingLevelMap` names it; treated as high on budget-based APIs.
  native: xhigh
  normalized: very_high
- evidence_ids:
  - src-args
  - src-model-clamp
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
  - docs-cli
  - src-main
  id: models-suffix
  kind: model_suffix
  lasts: one_session
  launch_modes:
  - interactive
  - non_interactive
  name: --models
  value: level_token
- arguments: []
  changes_running_session: unknown
  evidence_ids:
  - docs-settings
  - src-sdk
  - test-defaults
  id: model-thinking-levels
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: modelThinkingLevels
  value: level_token
- arguments: []
  changes_running_session: unknown
  evidence_ids:
  - docs-settings
  - src-sdk
  - test-defaults
  id: default-thinking-level
  kind: config_file_key
  lasts: until_changed
  launch_modes:
  - interactive
  - non_interactive
  name: defaultThinkingLevel
  value: level_token
- arguments: []
  changes_running_session: yes
  evidence_ids:
  - docs-models
  - src-interactive
  - src-session
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
  model: anthropic/claude-fable-5
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
  model: anthropic/claude-fable-5-1
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: anthropic/claude-haiku-4-5
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: anthropic/claude-haiku-4-5-20251001
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: anthropic/claude-opus-4-5
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: anthropic/claude-opus-4-5-20251101
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
  model: anthropic/claude-sonnet-4-5
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: anthropic/claude-sonnet-4-5-20250929
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
  default: off
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-4
- accepts:
  - off
  default: off
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-4-turbo
- accepts:
  - off
  default: off
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-4.1
- accepts:
  - off
  default: off
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-4.1-mini
- accepts:
  - off
  default: off
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-4.1-nano
- accepts:
  - off
  default: off
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-4o
- accepts:
  - off
  default: off
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-4o-2024-05-13
- accepts:
  - off
  default: off
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-4o-2024-08-06
- accepts:
  - off
  default: off
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-4o-2024-11-20
- accepts:
  - off
  default: off
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-4o-mini
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5
- accepts:
  - off
  default: off
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5-chat-latest
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5-mini
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5-nano
- accepts:
  - high
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5-pro
- accepts:
  - off
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5.1
- accepts:
  - off
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5.2
- accepts:
  - medium
  - xhigh
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5.2-chat-latest
- accepts:
  - medium
  - high
  - xhigh
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5.2-pro
- accepts:
  - off
  default: off
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5.3-chat-latest
- accepts:
  - off
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5.3-codex
- accepts:
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5.3-codex-spark
- accepts:
  - off
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5.4
- accepts:
  - off
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5.4-mini
- accepts:
  - off
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5.4-nano
- accepts:
  - medium
  - high
  - xhigh
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5.4-pro
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
  - medium
  - high
  - xhigh
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-5.5-pro
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
  model: openai/gpt-5.6-luna
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
  model: openai/gpt-5.6-sol
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
  model: openai/gpt-5.6-terra
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
  - minimal
  - low
  - medium
  - high
  - xhigh
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/gpt-realtime-2.1
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/o1
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/o1-pro
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/o3
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/o3-mini
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/o3-pro
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: openai/o4-mini
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
  model: openai-codex/gpt-5.4
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
  model: openai-codex/gpt-5.4-mini
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
  - off
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/deep-research-max-preview-04-2026
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/deep-research-preview-04-2026
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-2.5-computer-use-preview-10-2025
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-2.5-flash
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-2.5-flash-lite
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-2.5-pro
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-3-flash-preview
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-3.1-flash-lite
- accepts:
  - minimal
  - high
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-3.1-flash-lite-image
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-3.1-flash-lite-preview
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-3.1-flash-live-preview
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-3.1-pro-preview
- accepts:
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-3.1-pro-preview-customtools
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-3.5-flash
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-3.5-flash-lite
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-3.6-flash
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
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-flash-latest
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemini-flash-lite-latest
- accepts:
  - minimal
  - high
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemma-4-26b-a4b-it
- accepts:
  - minimal
  - high
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: google/gemma-4-31b-it
- accepts:
  - low
  - high
  - max
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: kimi-coding/k3
- accepts:
  - low
  - high
  - max
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: kimi-coding/k3-256k
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
  - off
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: kimi-coding/kimi-for-coding-highspeed
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: moonshotai/kimi-k2.6
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: moonshotai/kimi-k2.7-code
- accepts:
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: moonshotai/kimi-k2.7-code-highspeed
- accepts:
  - low
  - high
  - max
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: moonshotai/kimi-k3
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: zai/glm-4.7
- accepts:
  - off
  - minimal
  - low
  - medium
  - high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: zai/glm-5-turbo
- accepts:
  - off
  - high
  - max
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: zai/glm-5.2
- accepts:
  - off
  - high
  - max
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: zai/glm-5.2-highspeed
- accepts:
  - low
  - high
  - max
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: zai/glm-5.3
- accepts:
  - low
  - high
  - max
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: zai/glm-5.3-flash
- accepts:
  - low
  - high
  - max
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: zai/glm-5.3-highspeed
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
  - high
  - max
  default: high
  evidence_ids:
  - pi-model-catalog
  - src-model-clamp
  model: deepseek/deepseek-v4-pro
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
  - docs-session
  - src-session
  - docs-env
  - src-bash-env
  field: /thinkingLevel
  locator: ~/.pi/agent/sessions/<cwd-slug>/<timestamp>_<session-id>.jsonl
  notes: The record is the `thinking_level_change` entry (`type` field); the last one in the file is the effective level and one is written when the session starts. Pi emits the `thinking_level_changed` stream event only when the level changes, not at startup, so a launch-time level is visible only in the file. `--no-session` writes no file; `--session-dir` or PI_CODING_AGENT_SESSION_DIR moves it. In RPC mode `get_state` returns the level at `/data/thinkingLevel`.
  source: session_record
reasoning_output:
  evidence_ids:
  - test-json-reasoning
  - docs-json
  reaches_caller: full_text
gaps:
- detail: '`--models <model>:<level>` and the Ctrl+P cycle apply a scoped level, but the launch path was read in source only and whether the level changes the running session on cycling is unconfirmed.'
  next_check: Start pi in a detached tmux session with `--models kimi-coding/k3:low,kimi-coding/k3-256k:max`, press Ctrl+P, and read the last `thinking_level_change` entry.
  subject: models-suffix
- detail: Whether editing settings.json while a session runs changes the running level, or only the next model switch or launch, was not tested.
  next_check: Start an RPC session, rewrite `defaultThinkingLevel` in a temporary agent directory, call `get_state`, then `set_model` and call `get_state` again.
  subject: model-thinking-levels
- detail: Only the clamp for a token the model does not accept was recorded as `invalid_level`; an unrecognized token behaves differently (a stderr warning and the default level, or exit 1 for a bad `--model` suffix). Whether Claudine should treat that warning as an error is undecided.
  next_check: Check in a Claudine wrapper test that an invalid `--thinking` token is rejected before launch, since Pi exits 0 for it and only stderr shows the warning.
  subject: invalid-level
- detail: Full reasoning text was observed only for kimi-coding/k3; `ThinkingContent` can carry redacted blocks, so OpenAI, Codex, and Anthropic models may return a summary or nothing.
  next_check: Run `pi --mode json -p --thinking high` against an anthropic and an openai-codex model and compare the thinking_end content.
  subject: reasoning-output
- detail: '`AssistantMessage.providerThinkingLevel` is documented as preserving a provider detail but was absent in every observed run; only one Anthropic compat path sets it in source, so it may not confirm what reached the wire.'
  next_check: Route a model through a local echo server with `models.json` `baseUrl` and compare the request body's effort or budget field with the session level.
  subject: provider-thinking-level
- detail: Only macOS with pi 0.87.1 was examined; the model catalog is a per-host cache and the hosted docs at pi.dev were fetched for the CLI page only.
  next_check: Rerun the RPC get_available_thinking_levels check on Linux and Windows after `pi update`, and diff the catalog.
changes:
- 'Initial document: first research of reasoning level on Pi 0.87.1.'
requires_claudine_update: true
reason: Pi accepts seven tokens with per-model support and silently clamps an unsupported one upward, so Claudine must map its scale to `--thinking <level>` using the model's own accepted list and confirm the outcome from the session record, not the exit code.
contract_checked: 2026-09-29
---

## Levels

Pi accepts seven level tokens, weakest first: `off`, `minimal`, `low`, `medium`,
`high`, `xhigh`, `max`. Matching is exact and case-sensitive (`XHIGH` is
rejected). Pi calls the setting the *thinking level*; the environment variable
it exports to tools calls it the *reasoning level*.

Not every model accepts every level. Each model in Pi's catalog declares a
`reasoning` flag and a `thinkingLevelMap`, and Pi computes the accepted set from
them: a model without `reasoning` accepts only `off`; a level mapped to `null` is
dropped; `xhigh` and `max` are accepted only when the map names them. A level a
model does not accept is **clamped**, not refused: Pi moves to the nearest
accepted level above the request, and only if none exists, below it.

| Token | Claudine scale | Notes |
| --- | --- | --- |
| `off` | off | Not available on models that cannot disable reasoning (for example k3) |
| `minimal` | minimal | 1024-token budget on budget-based APIs; some providers map it to their `low` |
| `low` | low | 2048 tokens |
| `medium` | medium | Pi's default; 8192 tokens |
| `high` | high | 16384 tokens |
| `xhigh` | very_high | Only where the model names it; budget APIs treat it as `high` |
| `max` | maximum | Only where the model names it; budget APIs treat it as `high` |

Budgets are the defaults; the `thinkingBudgets` setting overrides them for
`minimal`, `low`, `medium`, and `high`.

## Choosing a Level

Controls, strongest first. The first two only exist inside a running session.

| Control | Where | Example |
| --- | --- | --- |
| `set_thinking_level` | RPC request (`pi --mode rpc`) | `{"type":"set_thinking_level","level":"high"}` |
| `/thinking [level]` | Interactive session | `/thinking high` (no argument opens a picker; `Shift+Tab` cycles) |
| `--thinking <level>` | Launch flag | `pi -p --thinking high "prompt"` |
| `:<level>` suffix | `--model` | `pi -p --model sonnet:high "prompt"` |
| `:<level>` suffix | `--models` | `pi --models sonnet:high,haiku:low` |
| `modelThinkingLevels` | `settings.json`, keyed `provider/modelId` | `{"modelThinkingLevels":{"anthropic/claude-sonnet-5":"high"}}` |
| `defaultThinkingLevel` | `settings.json` | `{"defaultThinkingLevel":"low"}` |

A project `.pi/settings.json` overrides the agent-directory
`~/.pi/agent/settings.json` (`PI_CODING_AGENT_DIR` relocates the latter) once the
project is trusted (`-a`); `-na` ignores it. There is no environment variable
that sets the level. `PI_REASONING_LEVEL` is exported to commands the bash tool
runs and Pi deletes any inherited value first, so setting it does nothing.

Precedence was measured, not assumed: `--thinking` beat a `--model` suffix,
which beat `modelThinkingLevels`, which beat `defaultThinkingLevel`, which beats
the built-in `medium`. A `--models` suffix applies only when `--model` is absent
(source only). A session command or RPC request changes the level after launch.
Resuming a session restores its recorded level unless a flag overrides it.

Non-interactive example, with stdin closed:

```sh
pi --offline --no-session -p --thinking high "Summarize README.md" < /dev/null
```

Redirect stdin: with a piped, never-closed stdin Pi waited and the run hung
until killed.

An unrecognized `--thinking` token prints
`Warning: Invalid thinking level "bogus". Valid values: ...` to stderr, is
ignored, and the run continues at the configured default with exit 0. An
unrecognized `--model` suffix (`k3:bogus`) instead fails: `Error: Model
"k3:bogus" not found.` and exit 1. A valid token the model does not accept is
clamped with no message at all.

## Models

The accepted set comes from the model catalog, so it varies by provider and
version; the `models` frontmatter lists the 96 catalog entries on this host
whose levels differ from the full list. Patterns:

- Anthropic Fable/Opus 5 and 5.5: no `off` on `claude-opus-5`, `claude-opus-5-5`,
  and the Fable models; `minimal` is also missing on `claude-opus-5-5`.
- Anthropic 4.5 models: `off` to `high`; 4.6 adds `max`; `claude-opus-4-7`,
  `claude-opus-4-8`, and `claude-sonnet-5` accept all seven.
- OpenAI GPT-5.x: `off`, `low`, `medium`, `high`, `xhigh` (no `minimal`); the
  5.6/6 families add `max`. `-pro` models drop `off` and `low`.
- OpenAI Codex provider: the same models accept `minimal` (sent as `low`).
- Google Gemini 3.x: `minimal` to `high`, some without `minimal`; Gemini 2.5
  accepts `off` to `high`.
- Kimi k3 (`kimi-coding/k3`, `moonshotai/kimi-k3`), GLM 5.3: only `low`, `high`,
  `max`. On these, Pi's default `medium` becomes `high`, and `off` becomes `low`.
- DeepSeek V4 Pro, GLM 5.2: `off`, `high`, `max`.
- Non-reasoning models (for example `openai/gpt-4o`, Mistral Codestral): `off`.
- A custom `models.json` model with `reasoning: true` and no `thinkingLevelMap`
  accepts `off` through `high`.

Ask Pi for the truth at run time: RPC `get_available_thinking_levels` returns the
current model's list.

## Confirming the Level

Clamping is silent, so confirm the effective level after the request.

- **Session file.** The last `thinking_level_change` entry in the session JSONL
  (`thinkingLevel` field) is the effective level. One is written when the session
  starts. `--no-session` writes none; use `--session-dir <tmp>` for a disposable
  run.
- **RPC.** `get_state` returns `data.thinkingLevel`; a change also emits a
  `thinking_level_changed` event.
- **Tool commands.** The bash tool sees `PI_REASONING_LEVEL`.
- **Not reported.** The JSON stream emits `thinking_level_changed` only on a change,
  not at startup, so a launch-time level does not appear in it; per-message
  `providerThinkingLevel` was absent in every run.

Observed on kimi-coding/k3, which accepts `low`, `high`, `max`:

| Request | Recorded level | stderr |
| --- | --- | --- |
| `--thinking off` | `low` | empty |
| `--model k3:medium` | `high` | empty |
| `--thinking low` | `low` | empty |
| `--model k3:low --thinking max` | `max` | empty |
| `--thinking bogus` | `high` (settings default) | invalid-level warning |

Reasoning reaches a non-interactive caller only with `--mode json`, as
`thinking_start`, `thinking_delta`, and `thinking_end` events; plain `-p` prints
just the answer. Observed for k3 as full text; other providers may return summaries
or redacted blocks.

## Sources

- [Pi CLI reference](https://pi.dev/docs/latest/cli)
- `pi --help` and `pi --version` (0.87.1), run locally
- Installed package `@earendil-works/pi-coding-agent@0.87.1`: `docs/settings.md`,
  `docs/models.md`, `docs/environment-variables.md`, `docs/rpc-commands.md`,
  `docs/json.md`, `docs/session-format.md`, and `dist/cli/args.js`, `dist/main.js`,
  `dist/core/sdk.js`, `dist/core/agent-session.js`,
  `dist/core/tools/bash.js`, `dist/modes/interactive/interactive-mode.js`
- Installed package `@earendil-works/pi-ai@0.87.1`: `dist/models.js`,
  `dist/api/simple-options.js`
- Local model catalog cache `~/.pi/agent/models-store.json` (read for
  `reasoning` and `thinkingLevelMap` only)
- Disposable runs with a temporary session directory and a temporary
  `PI_CODING_AGENT_DIR`; the user's configuration was not changed

## Changelog

- 2026-09-29: first version, examined against Pi 0.87.1 on macOS.
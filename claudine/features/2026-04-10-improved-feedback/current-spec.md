# Improved Feedback: Current Spec

Validated on 2026-04-10 against the current Claudine codebase, the provider research under `claudine/docs/research/non-interactive-sessions/`, and current provider-authored docs/source where available.

## Goal

Improve non-interactive feedback by fixing correctness gaps in the structured stream pipeline first, then adding small reporting improvements that fit the current wrapper and sink model.

This document intentionally excludes larger protocol migrations and UI-heavy ideas that are better handled as follow-on work.

## What We Should Optimize For Now

1. Correctness before presentation. If Claudine parses the wrong event names or misses important fields, no amount of UI polish helps.
2. Provider-authored stream contracts over reverse-engineered guesses.
3. Existing wrapper surfaces first. Prefer work that fits the current `StreamParser`, `StreamEventSink`, wrapper dispatch, and stderr formatting model.
4. Shared semantics over provider-specific field sprawl. Only add new first-class summary fields when multiple providers need the same concept.

## Scope

This slice should focus on:

- parsers already wired into `claudine::stream::create_parser`
- providers Claudine already exposes through wrappers but does not yet parse correctly
- summary/logging fixes that immediately improve operator feedback
- regression tests that lock current provider contracts in place

This slice should not try to:

- migrate Kimi from print-mode streaming to Wire mode
- add Roo stream support
- merge hook streams with stdout streams
- build pricing-table-driven cost estimation
- introduce a large new terminal UI model for checklists, plan panels, or nested session trees

## Priority 1: Fix Confirmed Parser Contract Bugs

### Claude

Current code treats any `type: "system"` message as init-like metadata, but current Claude structured output uses `system` subtypes such as `init` and `api_retry`, and emits rate-limit data under `rate_limit_info`.

Required changes:

- Parse `system` by `subtype`, not as a generic init event.
- Handle `system/api_retry` and map the classified error enum into actionable warnings or turn errors.
- Update rate-limit parsing to read `rate_limit_event.rate_limit_info.status`, `resetsAt`, and related fields instead of only `is_throttled` / `retry_after_ms`.
- Preserve verbose-only init metadata such as `apiKeySource`, `claude_code_version`, and `permissionMode` in stream metadata for logging and reporting.
- Surface `files_persisted` as an execution-side notification rather than dropping it.
- Keep `thinking_delta` support intact.

Why now:

- These are confirmed mismatches between current Claudine code and current Claude stream contracts.
- They directly affect billing-error reporting, throttle visibility, and session metadata quality.

Acceptance criteria:

- Claude billing failures are distinguishable from auth failures and generic turn errors.
- Claude subscription throttling uses the modern nested rate-limit shape.
- Verbose init metadata is preserved into reporting metadata.
- Regression tests cover `api_retry`, nested `rate_limit_info`, and `files_persisted`.

### Codex

Current Claudine parsing still assumes legacy item names such as `command_exec` and `patch_apply`, while current upstream `codex exec --json` emits `command_execution`, `file_change`, `collab_tool_call`, and `todo_list`.

Required changes:

- Align item parsing with the current upstream `exec_events.rs` contract.
- Recognize `command_execution`, `file_change`, `mcp_tool_call`, `collab_tool_call`, `web_search`, `todo_list`, `reasoning`, and `error`.
- Handle `item.updated` for `todo_list` instead of ignoring it.
- Map `collab_tool_call` start/completion to the existing subagent sink events.
- Treat `turn.failed` as a first-class error path.
- Keep `cached_input_tokens` separate from the total calculation and avoid speculative usage math beyond what the provider guarantees.

Why now:

- This is a confirmed parser-schema mismatch in a provider Claudine already supports.
- Missing these events hides tool progress, checklists, and subagent activity entirely.

Acceptance criteria:

- Codex tool items are no longer silently dropped because of stale item names.
- `todo_list` updates are observable in the stream path.
- `collab_tool_call` emits subagent lifecycle events.
- Regression tests use current upstream item names and `item.updated`.

### Goose

Claudine exposes Goose as a wrapped provider but does not currently provide a dedicated structured parser. `create_parser` falls back to the Claude parser, and `stream_protocol_for` returns `None` for Goose.

Required changes:

- Add `GooseStreamParser`.
- Register Goose in `create_parser`.
- Return the correct structured protocol from `stream_protocol_for`.
- Parse the current Goose top-level envelope: `message`, `notification`, `error`, and `complete`.
- Handle flattened `notification` payloads correctly.
- Extract nested `toolRequest` / `toolResponse` content from `message.content[]`.
- Emit subagent start/stop from `notification_type` values such as `subagent_tool_request` and `tasks_complete`.
- Capture `complete.total_tokens` into the final summary.

Why now:

- This is a functionality gap, not just a nice-to-have.
- Goose already has enough structured output to support basic parity with the other wrappers.

Acceptance criteria:

- Goose no longer falls back to Claude parsing.
- Tool activity, notifications, and final token totals are available in structured summaries.
- Regression tests cover each top-level Goose event type.

### Qwen

Current Qwen parsing handles only the coarse envelope and ignores the richer partial-stream events available when partial messages are enabled.

Required changes:

- Parse `stream_event` payloads, especially `content_block_delta`, `message_start`, `message_stop`, and `tool_progress`.
- Support `thinking` deltas and other partial assistant updates without regressing normal assistant text.
- Preserve final `permission_denials` from `result` for diagnostics and reporting.
- Keep expectations realistic: the richer `result.stats` object is currently a buffered-JSON feature, not something the stream wrapper should depend on.

Why now:

- Qwen already exposes these partial signals in the stream contract.
- This is incremental, low-risk work inside an existing parser.

Acceptance criteria:

- Qwen streams can surface thinking/tool-progress updates when partial messages are enabled.
- Denied tool calls are visible in the final provider summary/logging.
- Regression tests cover `stream_event` handling and `permission_denials`.

## Priority 2: Make Reporting Better Without Reworking The Whole UI

The current wrapper already supports:

- session start
- warnings
- before/after tool events
- permission requests
- subagent start/stop
- completion summaries

Near-term reporting work should use those existing hooks before inventing new renderer abstractions.

Recommended changes:

- Improve start-summary metadata where the parser can already provide it, especially model identity and provider session IDs.
- Improve warning quality for classified provider errors such as billing failures, auth failures, and hard throttles.
- Make tool-result formatting more informative for file-change and command-execution style items by including stable status and compact result context.
- Prefer subagent start/stop emission over provider-specific prose whenever the upstream stream has a dedicated signal.

Not for this slice:

- live checklist widgets
- multi-line plan panels
- nested subagent trees
- persistent status bars

Those features are valuable, but they require a deliberate normalized UI contract rather than ad hoc provider-specific formatting.

## Priority 3: Tighten Summary And Reporting Semantics

`StreamExecutionSummary` should stay compact and cross-provider. The current design is broadly right, but the immediate work should tighten how provider-specific details are stored.

Required guidance:

- Keep provider-specific extras in `raw_summary` / `provider_summary` unless the concept is clearly cross-provider.
- Avoid adding first-class fields for one provider only.
- Prefer normalized fields only for concepts that help multiple providers now, such as token usage, cost, duration, tool-call counts, and coarse error classification.

Immediate summary/reporting tasks:

- Ensure Claude rate-limit and Qwen permission-denial details survive into provider-summary logging.
- Ensure Goose token totals and Codex updated usage snapshots survive into final summary logging.
- Preserve enough session-start metadata for reporting queries and postmortems.

## Priority 4: Add Contract-Focused Regression Tests

The parser layer is where the current drift is happening. The best near-term investment after parser fixes is fixture-driven tests that lock each provider to its current documented contract.

Required test strategy:

- Add provider-specific fixtures based on current research examples and official source-backed shapes.
- Cover both happy-path and failure-path records.
- Cover the event names Claudine has historically drifted on:
    - Claude: `system/api_retry`, nested `rate_limit_info`
    - Codex: `command_execution`, `file_change`, `collab_tool_call`, `todo_list`, `item.updated`
    - Goose: `message`, flattened `notification`, `complete`
    - Qwen: `stream_event`, `tool_progress`, `permission_denials`
- Treat malformed-line behavior as a separate concern from schema drift.

## Recommended Delivery Order

1. Claude parser fixes
2. Codex parser fixes
3. Goose parser implementation and wiring
4. Qwen partial-stream support
5. Summary/reporting cleanup
6. Fixture-based regression tests

This order keeps the work biased toward confirmed correctness defects first, then closes the most obvious provider support gap, then improves richer feedback.

## Ideas Reviewed But Not Promoted Into This Slice

Several ideas from the original draft are good, but they are not the best "do now" work once the current code and provider contracts are checked side by side.

- Gemini per-model attribution is useful, but it is less urgent than the confirmed Claude, Codex, Goose, and Qwen gaps. Gemini already has a functioning parser and captures the core final stats path today.
- OpenCode permission and question visibility is a real need, but current stdout JSON intentionally filters out much of that information. That belongs in a hook-plus-stream fusion project rather than a parser-only slice.
- Kimi plan displays, structured questions, and rich subagent reporting are best handled through Kimi Wire mode, not by stretching the lighter print-mode parser.
- Roo support is valuable, but it is a separate provider-enablement decision rather than a small feedback polish task.

## Sources Reviewed

Internal:

- `claudine/features/2026-04-10-improved-feedback/spec.md`
- `claudine/docs/research/non-interactive-sessions/claude.md`
- `claudine/docs/research/non-interactive-sessions/codex.md`
- `claudine/docs/research/non-interactive-sessions/gemini.md`
- `claudine/docs/research/non-interactive-sessions/goose.md`
- `claudine/docs/research/non-interactive-sessions/kimi.md`
- `claudine/docs/research/non-interactive-sessions/opencode.md`
- `claudine/docs/research/non-interactive-sessions/qwen-cli.md`
- `claudine/docs/research/non-interactive-sessions/roo-code.md`
- `claudine/lib/src/stream/*.rs`
- `claudine/cli/src/commands/wrap/mod.rs`

Provider-authored references checked during this review:

- Anthropic Claude Code headless / Agent SDK docs and current stream type references
- OpenAI Codex non-interactive docs and current `codex-rs/exec/src/exec_events.rs`
- Google Gemini CLI headless docs and current `packages/core/src/output/types.ts`
- Goose running-tasks docs and current `crates/goose-cli/src/session/mod.rs`
- Qwen headless docs and current protocol types
- Kimi Wire mode docs and current wire protocol source

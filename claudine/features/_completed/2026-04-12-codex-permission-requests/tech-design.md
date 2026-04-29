# Codex Permission Request Summary Tech Design

This document turns `claudine/features/_unscheduled/codex-permission-requests/spec.md` into an implementation-ready design for Claudine's existing structured stream, wrapper summary, and synthetic JSONL summary pipeline.

Primary inputs:

- `claudine/features/_unscheduled/codex-permission-requests/spec.md`
- current Codex typed protocol in `claudine/lib/src/stream/protocol/codex.rs`
- current Codex parser in `claudine/lib/src/stream/codex.rs`
- current stream summary model in `claudine/lib/src/stream/summary.rs`
- current wrapper summary rendering in `claudine/cli/src/commands/wrap/mod.rs`
- current synthetic summary-event mapping in `claudine/lib/src/stream/reporting.rs`

The core design decision is to treat Codex permission activity as session-summary telemetry derived from the existing typed stream parser, not as a new cross-provider permissions abstraction and not as a reporting-schema expansion.

## Summary

This feature adds Codex-only session counters for permission-gate activity and exposes them in the same places that already consume `StreamExecutionSummary`:

1. the in-memory summary returned by `CodexStreamParser::finish(...)`
2. the wrapper completion summary rendered to stderr
3. the synthetic JSONL `SessionEnd` summary event

The feature remains intentionally narrow:

1. `PermissionRequest` and `ApprovalRequest` are counted together as `permission_prompts`
2. `UserInputRequest` is counted separately as `user_input_prompts`
3. no denial/approval resolution field is added in v1 because the current Codex typed stream model does not expose a trustworthy outcome signal
4. other providers leave the new fields unset

The result is an honest, provider-specific improvement that gives users immediate visibility into "the session paused because Codex needed me" without pretending Claudine already has a normalized permission-resolution model.

## Goals

1. Surface Codex permission-gate counts in `StreamExecutionSummary`.
2. Show those counts in the wrapper's end-of-run summary prose.
3. Serialize the same counts into the synthetic JSONL summary event.
4. Keep the field names provider-neutral enough for future providers to reuse.
5. Preserve backward-compatible JSON behavior by omitting absent fields.

## Non-Goals

1. No cross-provider normalization of permission models in v1.
2. No attempt to infer approvals or denials from missing or ambiguous Codex signals.
3. No reporting SQLite schema migration.
4. No change to provider capability tables such as `Provider::Codex.supports_event(...)`.
5. No redesign of badges, hook dispatch, or the permissions engine.

## Current Baseline

Today the relevant path already exists:

1. `claudine/lib/src/stream/protocol/codex.rs` has typed `CodexItem` variants for:
   - `PermissionRequest`
   - `ApprovalRequest`
   - `UserInputRequest`
2. `claudine/lib/src/stream/codex.rs` already detects those items in `handle_item_started(...)` and emits `sink.on_permission_request(&meta)`.
3. `claudine/lib/src/stream/summary.rs` has no fields for permission activity, so the parser drops that information once the live callback fires.
4. `claudine/cli/src/commands/wrap/mod.rs::format_summary_prose(...)` renders the final user-facing completion summary from `StreamExecutionSummary`.
5. `claudine/lib/src/stream/reporting.rs::summary_to_event_meta_with_context(...)` maps selected summary fields into the synthetic JSONL `SessionEnd` event.

This means Claudine can observe permission prompts live, but it cannot currently answer a basic session-level question:

> "How many times did Codex stop to ask me for permission during this run?"

## Why This Is Still Worth Doing

Claudine already logs live `PermissionRequest` events through the stream sink, so this feature is intentionally denormalized.

That duplication is useful, not accidental:

1. stderr completion summaries only have access to `StreamExecutionSummary`, not the replayed event stream
2. synthetic summary events are consumed as one-row session summaries by downstream tooling
3. users often care about the aggregate fact that a session repeatedly stopped for approval, not each individual prompt in isolation

This is the same pattern already used for `tool_calls`, `token_usage`, `cost_usd`, and duration: keep the raw event stream, but also carry the rollup in the final summary object.

## Spec Clarifications

### 1. Count shape

The summary will expose two counters:

1. `permission_prompts`
2. `user_input_prompts`

`permission_prompts` counts:

1. `CodexItem::PermissionRequest`
2. `CodexItem::ApprovalRequest`

`user_input_prompts` counts:

1. `CodexItem::UserInputRequest`

This keeps the end-user question simple:

1. "Did Codex need approval?" -> `permission_prompts`
2. "Did Codex ask me a blocking question?" -> `user_input_prompts`

### 2. No denial field in v1

The current typed Codex permission payload is:

```rust
pub struct CodexPermissionItem {
    pub id: Option<String>,
    pub name: Option<String>,
}
```

There is no resolved outcome, no explicit allow/deny status, and no typed completion event with a trustworthy decision payload.

Because of that, v1 must not add:

1. `permission_denials`
2. `permission_approvals`
3. any derived "blocked by denial" metric

If Codex later emits a real resolution event, that can be added as a follow-up field without renaming the prompt counters.

### 3. Field placement

The new counters belong as first-class optional fields on `StreamExecutionSummary`, not under `provider_summary`.

Reasoning:

1. they are derived rollups like `tool_calls`, not opaque provider blobs like `raw_summary`
2. they are intentionally named so other providers can reuse them later
3. top-level placement makes stderr formatting and summary-event serialization simpler

### 4. Provider scope

The fields are neutral, but the implementation is Codex-only in v1.

Behavior by provider:

1. Codex sets the counters when non-zero
2. all other providers leave them as `None`
3. serde omits the keys entirely when absent

### 5. Summary prose semantics

Permission counts are informational operational signals, not warning badges.

So the wrapper should:

1. append them to the primary completion summary line
2. not create a new badge category for them
3. preserve existing badge behavior for actual rate-limit, billing, quota, or permission-error conditions

## Data Model

Add two optional fields to `claudine/lib/src/stream/summary.rs`:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub permission_prompts: Option<u32>,

#[serde(skip_serializing_if = "Option::is_none")]
pub user_input_prompts: Option<u32>,
```

Recommended placement is immediately after `tool_calls`, because these fields are session activity counts with similar semantics.

`Default` behavior:

1. both fields default to `None`
2. parsers set `Some(n)` only when `n > 0`

This preserves today's serialized shape for all unaffected providers and sessions.

## Parser Design

### Codex parser state

Extend `CodexStreamParser` in `claudine/lib/src/stream/codex.rs` with:

```rust
permission_prompts: u32,
user_input_prompts: u32,
```

### Variant-sensitive counting

The current `handle_item_started(...)` path uses `item.as_permission()` and therefore loses the distinction between approval prompts and user-input prompts.

For this feature, change that branch to match the actual variant:

```rust
match item {
    CodexItem::PermissionRequest(perm) | CodexItem::ApprovalRequest(perm) => {
        self.permission_prompts += 1;
        let meta = self.permission_meta(&perm, "permission_prompt");
        self.sink.on_permission_request(&meta);
        return;
    }
    CodexItem::UserInputRequest(perm) => {
        self.user_input_prompts += 1;
        let meta = self.permission_meta(&perm, "user_input_prompt");
        self.sink.on_permission_request(&meta);
        return;
    }
    _ => {}
}
```

The parser should continue sending all three variants through `on_permission_request(...)` so existing live dispatch behavior stays unchanged.

### Enabling metadata

Update `permission_meta(...)` to include an explicit discriminator:

1. `extra["permission_kind"] = "permission_prompt"` for `PermissionRequest` and `ApprovalRequest`
2. `extra["permission_kind"] = "user_input_prompt"` for `UserInputRequest`

This metadata is not required for the final summary counts, but it makes the live event more truthful and gives downstream consumers a way to distinguish the classes without parser internals.

### Finish behavior

When `CodexStreamParser::finish(...)` constructs `StreamExecutionSummary`, set:

1. `permission_prompts: Some(self.permission_prompts)` when `> 0`
2. `user_input_prompts: Some(self.user_input_prompts)` when `> 0`

No other provider parser changes are required.

## Wrapper Summary Rendering

The active user-facing wrapper summary is rendered in:

- `claudine/cli/src/commands/wrap/mod.rs::format_summary_prose(...)`

That function currently builds one primary line from:

1. duration
2. token usage
3. cost
4. tool calls

The design is to append permission activity after tool calls:

1. `3 permission prompts`
2. `1 user input prompt`

Examples:

```txt
✓ 18s · 7K input tokens · 1K output tokens · 4 tool calls · 3 permission prompts
```

```txt
✓ 41s · 12 tool calls · 2 permission prompts · 1 user input prompt
```

Pluralization rules:

1. `1 permission prompt`, otherwise `N permission prompts`
2. `1 user input prompt`, otherwise `N user input prompts`

Important scope boundary:

1. these counts stay in the main prose line
2. they do not create new warning lines
3. they do not affect `format_verbose_summary_details_prose(...)` in v1

### Secondary formatter parity

`claudine/lib/src/stream/stderr.rs` still contains library-level completion formatters and unit tests for summary rendering conventions.

Even though the wrapper now uses `format_summary_prose(...)`, the same new fields should be added there too so:

1. future consumers of those helpers stay aligned
2. formatting behavior is consistent across the two summary surfaces
3. test coverage exists at both the prose-rendering layer and the library formatter layer

## Synthetic JSONL Summary Event

Extend `claudine/lib/src/stream/reporting.rs::summary_to_event_meta_with_context(...)` to map the new fields into `meta.extra`:

1. `permission_prompts`
2. `user_input_prompts`

Example synthetic summary event payload shape:

```json
{
  "event": "session_end",
  "extra": {
    "synthetic": true,
    "synthetic_kind": "stream_wrapper_summary",
    "stream_protocol": "jsonl",
    "tool_calls": 12,
    "permission_prompts": 2,
    "user_input_prompts": 1
  }
}
```

This is not a schema break:

1. `extra` is already an open-ended JSON object
2. absent keys remain absent
3. reporting ingestion already persists `extra_json` without needing a new column

## Reporting Impact

No SQLite schema change is needed for this feature.

Why:

1. the synthetic summary event already stores all unmapped extra fields inside `extra_json`
2. the new counters are primarily for wrapper stderr and machine consumers of summary events
3. current reporting queries do not aggregate synthetic summary permission counts

Also important:

1. this feature does not replace live `PermissionRequest` event logging
2. `DailySummary.total_permission_requests` continues to be computed from real event rows, not from the synthetic summary rollup

If later work wants SQL-level session aggregates for these new summary fields, that should be a separate reporting feature with an explicit migration.

## Recommended File-Level Changes

### `claudine/lib/src/stream/summary.rs`

1. Add `permission_prompts: Option<u32>`.
2. Add `user_input_prompts: Option<u32>`.
3. Update `Default`.
4. Expand serde round-trip and skip-none tests.

### `claudine/lib/src/stream/codex.rs`

1. Add parser counters.
2. Match permission-like item variants explicitly.
3. Add `permission_kind` to the emitted meta.
4. Populate the new summary fields in `finish(...)`.
5. Add parser tests for:
   - `PermissionRequest` increments `permission_prompts`
   - `ApprovalRequest` increments `permission_prompts`
   - `UserInputRequest` increments `user_input_prompts`
   - mixed sessions roll up both counts correctly

### `claudine/cli/src/commands/wrap/mod.rs`

1. Update `format_summary_prose(...)` to append permission-count clauses.
2. Add summary-rendering tests for:
   - permission prompts only
   - user input prompts only
   - both counters together
   - singular/plural formatting

### `claudine/lib/src/stream/stderr.rs`

1. Update `format_completion_summary(...)`.
2. Update `format_compact_completion(...)` only if compact mode should surface at least one of the new counters.

Recommended v1 choice:

1. include `permission_prompts` in normal completion formatting
2. omit both new counters from compact mode to keep `--quiet` terse

That keeps the main wrapper summary informative without making the compact one-line path noisy.

### `claudine/lib/src/stream/reporting.rs`

1. Map the new fields into `extra`.
2. Add tests that assert those keys appear when populated and are omitted when absent.

## Test Plan

Minimum test coverage:

1. `stream/summary.rs`
   - serde round-trip with the new fields populated
   - skip-none serialization omits the fields
2. `stream/codex.rs`
   - `PermissionRequest` increments `permission_prompts`
   - `ApprovalRequest` increments `permission_prompts`
   - `UserInputRequest` increments `user_input_prompts`
   - mixed sequence produces both counters in `finish(...)`
3. `commands/wrap/mod.rs`
   - rendered prose includes permission counts with correct pluralization
4. `stream/stderr.rs`
   - normal completion formatting includes permission counts
   - compact formatting remains unchanged if we intentionally omit them there
5. `stream/reporting.rs`
   - synthetic summary event includes both fields when present
   - missing fields are omitted

## Risks and Edge Cases

### 1. Duplicate counting from started/completed item pairs

The feature must count only the ask, not every lifecycle frame.

Current recommendation:

1. count only in `handle_item_started(...)`
2. ignore `item.completed` for permission and user-input items in v1

That matches the current parser shape and avoids accidental double counts.

### 2. Ambiguous future Codex variants

If Codex adds more permission-related item types later, typed deserialization will fail into the parser's unknown-event fallback and the session will simply undercount them.

That is acceptable for v1 because:

1. it preserves forward compatibility
2. typed protocol tests are already the safety net for new variants
3. future variants can be added intentionally once observed

### 3. Misleading denial language

This is the main product risk.

The design avoids it by:

1. using `permission_prompts`
2. keeping `user_input_prompts` separate
3. refusing to claim approval/denial outcomes without a real signal

## Rollout

This feature is small enough to ship in one change set:

1. summary model
2. Codex parser counters
3. wrapper summary rendering
4. synthetic summary-event serialization
5. focused tests

No config migration, docs migration, or reporting backfill is required.

## Deferred Follow-Ups

These are explicitly out of scope for this document:

1. adding `permission_denials` if Codex later emits resolution events
2. extending the same summary fields to Kimi, OpenCode, or other providers
3. promoting `permission_kind` into first-class reporting queries
4. adding badge or warning behavior for long-running "waiting on user" stalls
5. exposing a richer structured permission activity block instead of flat counters

---
created: 2026-06-10
provider: OpenCode
severity: regression
related_fixes:
  - fixes/2026-05-12-opencode-stderr-returns
---

# OpenCode: Parser Does Not Recognize New `timestamp=... level=...` Stderr Log Format

## The Problem We Are Solving

When running `claudine compose` with OpenCode as the agent, users see raw
structured log lines that should have been consumed by the stderr bridge
but instead pass through to the terminal:

```
timestamp=2026-06-10T16:11:27.352Z level=INFO run=df5a9474 message=tracking hash=86a6603a...
timestamp=2026-06-10T16:11:27.460Z level=INFO run=df5a9474 message=loop session.id=ses_14db... step=1
timestamp=2026-06-10T16:11:27.559Z level=INFO run=df5a9474 message=tracking hash=86a6603a...
timestamp=2026-06-10T16:11:27.574Z level=INFO run=df5a9474 message=process session.id=ses_14db...
timestamp=2026-06-10T16:11:27.574Z level=INFO run=df5a9474 message=stream providerID=zai-coding-plan...
timestamp=2026-06-10T16:11:27.575Z level=INFO run=df5a9474 message="llm runtime selected"...
timestamp=2026-06-10T16:11:31.461Z level=INFO run=df5a9474 message=evaluated permission=glob...
```

These lines carry exactly the same semantic information the bridge was built to
classify and promote — session lifecycle, LLM calls, step loops, permissions —
but they are invisible to the parser because OpenCode changed its stderr log
format.

## Root Cause

The header parser in
[`events.rs:163`](../../../claudine/lib/src/stream/logs/opencode/events.rs)
(`HEADER_RE`) expects the format:

```
LEVEL TIMESTAMP +DELTAms body...
```

Example (old format, still supported):

```
INFO  2026-05-12T20:00:12 +20ms service=session id=ses_abc title=New session created
```

OpenCode has started emitting a **new format** where every field is a
`key=value` pair and the fixed header is gone:

```
timestamp=2026-06-10T16:11:27.352Z level=INFO run=df5a9474 message=tracking hash=86a6603a...
```

Because `HEADER_RE` cannot match this new shape, `parse_line` returns
`ParsedOpenCodeStderrLine::RawText(...)`. The bridge's `handle_raw` method
calls `classify_raw`, which only matches `Error:` / `error:` prefixes. The
result is `StderrIngestOutcome::NotConsumed`, and the raw line passes through
to the user's terminal.

### Consequences

1. **Noisy terminal output.** Every INFO-level lifecycle log line leaks
   through as raw text during `compose`, `inline-compose`, and `sequence`.
2. **Silence watchdog blind spot.** The bridge never promotes these lines
   into `SemanticEvent`s, so `last_event_at` is not refreshed. During long
   NDJSON silences the watchdog relies on byte heartbeats only, not semantic
   activity — the exact problem the stderr bridge was built to solve.
3. **Summary enrichment loss.** Model identity, provider attribution, and
   subagent lineage carried in the new-format lines are not merged into
   `StreamExecutionSummary`.
4. **Permission and rate-limit signals missed.** `permission=... evaluated`
   and provider-error records in the new format are not classified.

## Format Comparison

### Old format (still valid)

```
LEVEL  TIMESTAMP +DELTAms key=value... [trailing message]
INFO  2026-05-12T20:00:12 +20ms service=session id=ses_abc title=New session created
ERROR 2026-04-15T19:26:02 +3054ms service=llm error={...json...}
```

Characteristics:
- Fixed header: level, timestamp (without timezone suffix), `+NNNms` delta
- Body is `key=value` pairs with optional trailing bare message
- One or more spaces between header fields

### New format (unrecognized)

```
timestamp=YYYY-MM-DDTHH:MM:SS.sssZ level=LEVEL key=value... [message=...]
timestamp=2026-06-10T16:11:27.352Z level=INFO run=df5a9474 message=tracking hash=86a6603a...
timestamp=2026-06-10T16:11:27.574Z level=INFO run=df5a9474 message=stream providerID=zai-coding-plan modelID=glm-5.1 session.id=ses_14db... small=false agent=build mode=primary
```

Characteristics:
- No fixed header; every field is `key=value`
- `timestamp=` carries a full ISO 8601 with millisecond precision and `Z` suffix
- `level=` carries one of `DEBUG|INFO|WARN|ERROR`
- `message=` carries the trailing log message (sometimes the only bare-ish
  token after all `key=value` pairs)
- `run=` carries a run/session identifier
- No `+DELTAms` delta field observed in the new format
- Body follows the same `key=value` syntax as the old format

### Tag-equivalence map

The new format carries the same semantic tags the classifiers already match
against, just in a different envelope:

| Old format tag | New format tag | Notes |
|---|---|---|
| (from header) `LEVEL` | `level=LEVEL` | Same four levels |
| (from header) `TIMESTAMP` | `timestamp=YYYY-MM-DDTHH:MM:SS.sssZ` | New has millis + Z suffix |
| (from header) `+DELTAms` | absent | Delta is not present in new format |
| (from body) `service=session` | `service=session` | Identical |
| (from body) `message tracking` | `message=tracking` | Trailing message becomes a tag value |
| (from body) `providerID=...` | `providerID=...` | Identical |
| (from body) `session.id=...` | `session.id=...` | Identical |
| (from body) `message=...` | `message=...` | Identical |

The key insight: the **body parser** (`parse_body`) already handles `key=value`
extraction correctly. The problem is purely in the **header detection** step
that decides whether a line is `Structured` or `RawText`.

## What We Are Building

### Goal

Teach `parse_line` to recognize the new `timestamp=... level=...` format so
that the stderr bridge can classify and consume these records exactly as it
does for the old format. No downstream classifier changes should be needed.

### Non-Goals

- **Do not change the bridge, classifiers, or `LogClassification` enum.** The
  fix is in the parser only. The bridge (`reasoning.rs`) and classifiers
  (`errors.rs`) operate on `OpenCodeLogRecord` which is format-agnostic.
- **Do not drop support for the old format.** Both formats must parse
  correctly. Old-format lines appear in existing fixtures and may still be
  emitted by older OpenCode builds.
- **Do not change the `--print-logs` / `--log-level` flags.** The
  `apply_structured_stream` configuration in `profile/opencode.rs` is correct.
- **Do not change `parse_body`.** The body extraction logic already handles
  `key=value` pairs correctly for both formats.

### Required Behavior Changes

1. **Add a second header regex** (or extend the existing one) in
   `events.rs` to match lines starting with
   `timestamp=YYYY-MM-DDTHH:MM:SS(.sss)?Z level=(DEBUG|INFO|WARN|ERROR)`.

2. **Update `parse_line`** to try the new-format header when the old-format
   header does not match. On match:
   - Extract `timestamp=` value → parse as UTC (support both
     `%Y-%m-%dT%H:%M:%S` and `%Y-%m-%dT%H:%M:%S%.3fZ`).
   - Extract `level=` value → `LogLevel` enum.
   - `delta_ms` → `0` (the field is absent in the new format).
   - Parse the remainder as `key=value` body using the existing `parse_body`.
   - Return `ParsedOpenCodeStderrLine::Structured(...)`.

3. **Timestamp parsing.** Extend `parse_timestamp` (or add a second parser)
   to handle:
   - Old: `2026-06-10T16:11:27` (no subseconds, no timezone suffix)
   - New: `2026-06-10T16:11:27.352Z` (millisecond subseconds, `Z` suffix)

4. **Message extraction nuance.** In the new format, the trailing log
   message is carried as `message=<value>` in the body — not as a bare token
   after the last `key=value` pair. This means:
   - `parse_body` will capture `message=tracking` as a tag
   - The `OpenCodeLogRecord.message` field may be empty
   - Classifiers that check `record.message` for keywords like `"stream"`,
     `"loop"`, `"exiting loop"`, `"created"`, `"evaluated"`,
     `"Sent HTTP response"`, `"opencode"` need the message from the
     `message=` tag

   The existing `has_trailing_keyword` helper checks both `record.message`
   AND tag values for trailing keywords, so most classifications should work
   without changes. **Verify each lifecycle classifier against new-format
   samples** (see test plan below).

5. **Preserve `delta_ms` handling.** When the delta is absent (new format),
   store `0`. No downstream consumer uses `delta_ms` for classification — it
   is informational only.

### Files to Modify

| File | Change |
|---|---|
| `claudine/lib/src/stream/logs/opencode/events.rs` | Add new-format header regex; update `parse_line` to try both formats; extend `parse_timestamp` |

### Files That Must NOT Change

| File | Reason |
|---|---|
| `claudine/lib/src/stream/logs/opencode/reasoning.rs` | Bridge operates on `OpenCodeLogRecord` — format-agnostic |
| `claudine/lib/src/stream/logs/opencode/errors.rs` | Classifiers operate on `OpenCodeLogRecord` — format-agnostic |
| `claudine/cli/src/commands/wrap/profile/opencode.rs` | `--print-logs --log-level INFO` is correct |
| `claudine/lib/src/stream/logs/opencode/mod.rs` | Re-exports only — no logic |

## Test Plan

### New unit tests in `events.rs`

Each test follows the existing pattern in the `tests` module and covers one
parsing dimension of the new format:

1. **`new_format_parses_info_level`** — a simple new-format INFO line parses
   as `Structured` with the correct `level`, `timestamp`, `tags`, and
   `delta_ms == 0`.

2. **`new_format_parses_all_levels`** — one line each for `DEBUG`, `INFO`,
   `WARN`, `ERROR` all parse with the correct `LogLevel`.

3. **`new_format_timestamp_includes_millis`** — verify that
   `2026-06-10T16:11:27.352Z` parses to a UTC `DateTime` with sub-millisecond
   accuracy (the `.352` part).

4. **`new_format_preserves_raw_line`** — `record.raw` equals the original
   input string (same invariant as the old format).

5. **`new_format_extracts_tags`** — a line with `run=abc service=session
   id=ses_123 parentID=ses_parent title=My task` extracts all tag pairs
   correctly.

6. **`new_format_message_tag_captured`** — `message=tracking hash=abc123`
   results in `tags["message"] == "tracking"` (the value before the next
   `key=` boundary).

7. **`new_format_rejects_non_matching`** — lines that do not start with
   `timestamp=YYYY-...` (e.g. `"not a log line"`, `"INFO old format line"`)
   still fall through to `RawText` when they also don't match the old header.

8. **`new_format_without_message_tag`** — a line with only structural tags
   (`timestamp=... level=INFO run=abc service=default`) parses with
   `tags["service"] == "default"` and no `message` tag.

### New classifier round-trip tests in `errors.rs`

These verify that the *classifiers* produce correct results when fed records
parsed from the new format. Each test provides a representative new-format
line and asserts the expected `LogClassification` variant:

9. **`new_format_classifies_session_created`** — maps to `SessionCreated`.

10. **`new_format_classifies_session_created_subagent`** — with `parentID=`
    tag → `SessionCreated { parent_id: Some(...) }`.

11. **`new_format_classifies_llm_call`** — `service=llm ... mode=primary`
    → `LlmCall`.

12. **`new_format_classifies_step_loop`** — `service=session.prompt ...
    step=N ... message=loop` → `StepLoop`.

13. **`new_format_classifies_step_exit`** — `service=session.prompt ...
    message="exiting loop"` → `StepExit`.

14. **`new_format_classifies_permission_evaluated`** — `service=permission
    ... message=evaluated` → `PermissionEvaluated`.

15. **`new_format_classifies_tracking_as_unclassified`** — `message=tracking`
    lines with `hash=` and `cwd=` tags should parse but classify as
    `Unclassified` (they are still `Consumed` by the bridge, suppressing raw
    passthrough).

### Bridge integration test

16. **`new_format_bridge_consumes_lifecycle_lines`** — replay a multi-line
    new-format fixture (covering boot, session, LLM call, step loop, step
    exit, permission) through `OpenCodeLogBridge::ingest` and verify:
    - Every line returns `StderrIngestOutcome::Consumed`
    - The expected `SemanticEvent` sequence is emitted
    - No raw text leaks through

### Existing regression tests

All existing tests in `events.rs`, `errors.rs`, and
`opencode_stderr_lifecycle.rs` must continue to pass without modification.
The old format is still valid and must remain supported.

### Fixture files

Create one new fixture file:

- `claudine/lib/tests/fixtures/logs/opencode-new-format-lifecycle.txt`

Containing representative new-format lines covering every lifecycle
classification (boot banner alternative, session created, subagent session,
LLM call, step loop, step exit, permission evaluated, tracking/snapshot).

## Edge Cases

- **Quoted `message=` values.** Observed in the wild:
  `message="llm runtime selected"`. The existing `parse_body` extracts
  `llm runtime selected` as the tag value (the `=` value parser reads to the
  next `key=` boundary; quotes are preserved but the body parser does not
  treat them specially). Verify classifiers tolerate the quotes.

- **Mixed-format streams.** OpenCode could plausibly emit old-format and
  new-format lines in the same session (e.g. during a version transition).
  Both formats must parse independently within the same stream.

- **`delta_ms` absent.** The new format has no `+NNNms` field. Store `0` and
   ensure no downstream codepath branches on `delta_ms == 0` as a sentinel.

- **New `run=` tag.** This tag appears in every new-format line but has no
  equivalent in the old format. It is a run/session correlation id. Store it
  in `tags` (like any unknown tag) but do not classify on it.

## Verification Checklist

- [ ] All existing tests pass without modification
- [ ] New-format `INFO` lines parse as `Structured`
- [ ] New-format `ERROR` lines with inline JSON in `error=` parse correctly
- [ ] Bridge returns `Consumed` for every new-format line observed in the wild
- [ ] No raw `timestamp=` lines appear in terminal output during `compose`
- [ ] Watchdog sees semantic activity from new-format stderr events
- [ ] Summary enrichment (model, provider, subagent lineage) works for new-format records

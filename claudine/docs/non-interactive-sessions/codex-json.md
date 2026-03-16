---
prompt: "The Codex CLI can output a stream of JSONL output when the `exec` command is paired with the `--json` flag. In non-interactive sessions which claudine wraps this is much more valuable than just text as it provides metadata we wouldn't get otherwise.\n\n  Here's an example of the JSONL data you might get in a request:\n\n  ```json\n  {\"type\":\"thread.started\",\"thread_id\":\"019cf582-ae5f-71f1-af52-8a6e62c1bc22\"}\n  {\"type\":\"turn.started\"}\n  {\"type\":\"item.completed\",\"item\":{\"id\":\"item_0\",\"type\":\"agent_message\",\"text\":\"Hi Ken. What do you want to work on?\"}}\n  {\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":28903,\"cached_input_tokens\":4480,\"output_tokens\":61}}\n  ```\n\n  - This metadata can be used to present metadata to the user on STDERR when they are executing a non-interactive command.\n  - This metadata can be used to enhance the data we're providing to our logging platform\n\n  Your task is to:\n  \n  - research other examples online and fill in any other missing details not self-evident from the example data\n  - determine how best to feed the metadata to logging and non-interactive sessions."
last_update: 2026-03-16
last_updated: 2026-03-16
---
# Plan: Use Codex JSONL As The Wrapper Control Plane

## Goal

Make `claudine codex --non-interactive` consume Codex's JSONL stream as the authoritative runtime surface, so Claudine can:

- show useful progress and usage metadata on `stderr`
- enrich its own JSONL and reporting pipeline with real Codex session metadata
- preserve a stable user-facing `stdout` contract for non-interactive runs

This plan is design work only. It does not implement the wrapper changes.

## Sources

Online:

- OpenAI Codex CLI help from the installed `codex-cli 0.114.0`
- OpenAI Codex SDK package page: `https://pypi.org/project/openai-codex-sdk/`
- OpenAI Codex issue about session logs: `https://github.com/openai/codex/issues/2288`
- OpenAI Codex issue discussing streamed event shapes: `https://github.com/openai/codex/issues/5773`

Local codebase:

- [mod.rs](/Volumes/coding/personal/rusty-biscuit/claudine/cli/src/commands/wrap/mod.rs)
- [exec.rs](/Volumes/coding/personal/rusty-biscuit/claudine/cli/src/commands/wrap/exec.rs)
- [profile.rs](/Volumes/coding/personal/rusty-biscuit/claudine/cli/src/commands/wrap/profile.rs)
- [codex.rs](/Volumes/coding/personal/rusty-biscuit/claudine/lib/src/adapters/codex.rs)
- [dispatch/mod.rs](/Volumes/coding/personal/rusty-biscuit/claudine/lib/src/dispatch/mod.rs)
- [ingest.rs](/Volumes/coding/personal/rusty-biscuit/claudine/lib/src/reporting/ingest.rs)

Local runtime inspection:

- installed wrapper script: `/Users/ken/.bun/install/global/node_modules/@openai/codex/bin/codex.js`
- installed native binary: `/Users/ken/.bun/install/global/node_modules/@openai/codex-darwin-arm64/vendor/aarch64-apple-darwin/codex/codex`

## Confirmed Upstream Facts

### 1. Codex `exec` already exposes the right primitives

`codex exec --help` on this machine reports:

- `--json` prints events to stdout as JSONL
- `--output-last-message <FILE>` writes the last assistant message to a file
- `--output-schema <FILE>` validates the final response shape
- `--ephemeral` disables session persistence

That combination is enough to separate:

- machine-facing event transport
- user-facing final response text
- optional structured final-output validation

### 2. Codex persists session JSONL unless run ephemerally

OpenAI's own issue tracker confirms that Codex writes per-session JSONL logs under `~/.codex/sessions/...` by default, and that `--ephemeral` disables persistence.

That matters because it gives Claudine two ingestion choices:

- live ingestion from child stdout
- deferred ingestion from Codex-owned session files

### 3. The public automation surface is broader than the four-line example

The user's sample:

- `thread.started`
- `turn.started`
- `item.completed`
- `turn.completed`

is valid, but it is not the whole surface.

OpenAI's SDK docs describe a streamed event model over stdin/stdout, and local binary inspection of `codex-cli 0.114.0` shows a broader event vocabulary including:

- thread and turn lifecycle events
- item start and item completion events
- agent message and agent message delta events
- reasoning and reasoning delta events
- plan update events
- web search begin and end events
- MCP tool call begin and end events
- command exec begin, output delta, and end events
- patch apply begin and end events
- image generation and view-image events
- permission and user-input request events
- turn abort and stream error style events

Not every one of those names is guaranteed to appear on `codex exec --json`, but they are present in the installed binary's event vocabulary. Claudine should therefore design for a richer stream than just "turn started / turn completed".

### 4. Claudine is already structurally close

Current Claudine state:

- the Codex adapter already normalizes `turn.completed.usage` into `meta.extra.token_usage`
- reporting ingestion already reads normalized `token_usage` from `EventMeta.extra`
- wrapper execution currently forwards child stdout and stderr but does not parse Codex JSONL inline

So the gap is transport and routing, not schema design.

## Design Decision

Use Codex JSONL as the wrapper's internal control plane for non-interactive Codex runs.

More concretely:

- Claudine should launch Codex with `exec --json`
- Claudine should also set `--output-last-message <tempfile>` unless the caller explicitly asked for raw JSONL passthrough
- Claudine should parse JSONL line-by-line as the child runs
- Claudine should emit selected progress and usage summaries to `stderr`
- Claudine should forward normalized events into its existing dispatch/logging pipeline
- Claudine should print the final assistant message to `stdout` from the temp file in default text mode

This gives Claudine the metadata benefits of `--json` without forcing raw JSONL onto callers who expect plain output.

## Why This Is Better Than The Alternatives

### Not recommended: continue using plain-text stdout only

That throws away:

- thread IDs
- turn boundaries
- tool lifecycle metadata
- token usage
- structured failure states

This is the current blind spot.

### Not recommended: switch default wrapper stdout to raw Codex JSONL

That would break the current wrapper expectation that non-interactive runs produce a final assistant response on `stdout`.

Raw JSONL should remain available, but it should be explicit.

### Not recommended: ingest only the persisted session files after the run

Deferred file ingestion is useful as a fallback and audit path, but it is too late for:

- live `stderr` status
- immediate hook dispatch
- real-time token / tool telemetry

It also fails under `--ephemeral`.

## Recommended Output Contract

### Default non-interactive wrapper mode

Command behavior:

- child runs with hidden Codex JSONL transport
- `stdout` receives the final assistant message only
- `stderr` receives Claudine status and metadata lines
- Claudine logs receive normalized `EventMeta`

This should be the default because it preserves current wrapper ergonomics.

### Explicit raw JSON mode

When the caller explicitly requests raw JSON output, Claudine should preserve Codex JSONL on `stdout` and avoid converting it back to text.

Practical trigger options:

- explicit wrapper `--output json`
- or explicit passthrough `--json`

In this mode:

- `stdout` is provider-native JSONL
- `stderr` may still carry short Claudine metadata summaries
- Claudine should still parse and log the stream if it can do so without mutating stdout

## What To Feed To `stderr`

Do not mirror every raw event. The stream is too noisy.

Recommended `stderr` summaries:

- session start: short thread ID and cwd
- turn start: prompt accepted / turn number if available
- tool begin/end: tool name, short target, success/failure, duration when available
- plan update: short rendered summary if the payload is concise
- web search begin/end: query count or completion note
- turn complete: input/output/cache token counts and final duration if available
- turn failure: concise error message

Do not emit deltas to `stderr`:

- agent message deltas
- reasoning deltas
- command output deltas
- raw response item events

Those belong in provider-native JSONL or deep logs, not in wrapper UX.

## What To Feed To Logging

### Log all coarse lifecycle events

Good candidates for `EventMeta` emission:

- `thread.started`
- `turn.started`
- `turn.completed`
- `turn.failed` or equivalent error events
- tool begin/end events
- plan update events
- web search begin/end events
- MCP tool begin/end events

### Do not log high-volume deltas as first-class Claudine events

Skip or collapse:

- message deltas
- reasoning deltas
- command output chunks
- raw response item fragments

Otherwise Claudine's JSONL and SQLite index will bloat quickly and become harder to query.

### Preserve raw provider payloads in `meta.extra`

For logged events, keep the original Codex event body or important provider-native fragments in `meta.extra` where reasonable. Claudine already follows this pattern for usage data, and it keeps future reporting options open.

## Mapping Direction

Recommended normalized mapping for live Codex JSONL:

- session lifecycle -> existing `SessionStart`
- turn start -> existing `BeforePrompt`
- turn complete -> existing `TurnComplete`
- turn failure / stream error -> existing `TurnError`
- tool begin -> existing `BeforeTool`
- tool end -> existing `AfterTool`
- plan or reasoning summaries worth surfacing -> existing `Notification`

If newer Codex events do not map cleanly, prefer preserving them in `meta.extra` over inventing new normalized events too early.

## Implementation Shape

### 1. Add a Codex-aware live JSONL path in the wrapper

Most likely touch points:

- [mod.rs](/Volumes/coding/personal/rusty-biscuit/claudine/cli/src/commands/wrap/mod.rs)
- [exec.rs](/Volumes/coding/personal/rusty-biscuit/claudine/cli/src/commands/wrap/exec.rs)
- [profile.rs](/Volumes/coding/personal/rusty-biscuit/claudine/cli/src/commands/wrap/profile.rs)

Needed behavior:

- spawn child with piped stdout in non-interactive Codex mode
- parse each line as JSON if possible
- keep raw passthrough available when explicitly requested
- collect `output-last-message` from a temp file for default text mode

### 2. Route parsed events into existing dispatch code

Prefer calling Claudine's library dispatch path directly rather than shelling out to `claudine handle` for each line.

Relevant code:

- [dispatch/mod.rs](/Volumes/coding/personal/rusty-biscuit/claudine/lib/src/dispatch/mod.rs)
- [codex.rs](/Volumes/coding/personal/rusty-biscuit/claudine/lib/src/adapters/codex.rs)

This avoids process churn and guarantees the same normalization path used elsewhere.

### 3. Expand the Codex adapter conservatively

Current adapter support is adequate for the user's sample, but live wrapper ingestion will likely expose additional event shapes. Extend the adapter only for event families that are useful for:

- `stderr` summaries
- hook dispatch
- reporting

Do not chase every delta event in the first pass.

### 4. Keep persisted session-file ingestion as a fallback

Even after live parsing exists, persisted Codex session JSONL should still be treated as:

- an audit trail
- a recovery path if live parsing fails
- a possible backfill source for missed sessions

But it should not be the primary real-time integration path.

## Testing

Minimum tests for the implementation phase:

- wrapper default non-interactive mode prints final assistant text to `stdout`
- wrapper default non-interactive mode emits token and session summaries to `stderr`
- explicit raw JSON mode preserves provider JSONL on `stdout`
- `turn.completed.usage` reaches Claudine reporting through normalized `token_usage`
- high-volume delta events are ignored or collapsed for Claudine logging
- `--ephemeral` runs still produce live logs even though no Codex session file exists

## Recommendation Summary

The right design is:

1. Treat Codex JSONL as the internal transport for wrapped non-interactive Codex runs.
2. Keep plain final assistant text as the default `stdout` behavior by pairing `--json` with `--output-last-message`.
3. Emit concise live metadata to `stderr`.
4. Feed parsed coarse-grained events into Claudine's existing dispatch and reporting pipeline.
5. Reserve raw provider JSONL on `stdout` for explicit JSON-output mode.

That gives Claudine better observability without regressing the current non-interactive UX.


Sources:

- [OpenAI SDK package page](https://pypi.org/project/openai-codex-sdk)
- [OpenAI issue on persisted session logs](https://github.com/openai/codex/issues/2288)
- [OpenAI issue on streamed event shapes](https://github.com/openai/codex/issues/5773)
- Relevant Claudine files: [mod.rs](/Volumes/coding/personal/rusty-biscuit/claudine/cli/src/commands/wrap/mod.rs), [exec.rs](/Volumes/coding/personal/rusty-biscuit/claudine/cli/src/commands/wrap/exec.rs), [codex.rs](/Volumes/coding/personal/rusty-biscuit/claudine/lib/src/adapters/codex.rs)

# Strategy for Parsing Gemini CLI Stream-JSON Output

## Background

Gemini CLI supports `--output-format stream-json` in headless mode. Instead of
returning only a final text blob, it emits newline-delimited JSON events on
stdout for session start, user/assistant messages, tool calls, tool results,
warnings, and the final session summary.

For wrapped non-interactive runs, this is materially better than plain text:

- it gives Claudine the provider session ID
- it exposes the resolved model
- it gives aggregate token usage, cache usage, duration, and tool-call counts
- it provides a typed event stream we can summarize on stderr without corrupting
  the assistant response on stdout

Primary sources used here:

- Gemini headless-mode docs: <https://geminicli.com/docs/cli/headless-mode/>
- Gemini telemetry docs: <https://geminicli.com/docs/telemetry/>
- Local Gemini CLI implementation inspected from `@google/gemini-cli` v0.33.1:
  [nonInteractiveCli.js](/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/dist/src/nonInteractiveCli.js)
- Local stream-json type definitions:
  [types.d.ts](/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/node_modules/@google/gemini-cli-core/dist/src/output/types.d.ts)
- Local stream formatter implementation:
  [stream-json-formatter.js](/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/node_modules/@google/gemini-cli-core/dist/src/output/stream-json-formatter.js)

## Confirmed Event Schema

As implemented in Gemini CLI v0.33.1, the stream emits these top-level event
types:

| Type | When | Confirmed fields |
|------|------|------------------|
| `init` | First event | `timestamp`, `session_id`, `model` |
| `message` | User input and assistant output | `timestamp`, `role`, `content`, optional `delta` |
| `tool_use` | Model requested a tool | `timestamp`, `tool_name`, `tool_id`, `parameters` |
| `tool_result` | Tool finished | `timestamp`, `tool_id`, `status`, optional `output`, optional `error` |
| `error` | Non-fatal warning/error event | `timestamp`, `severity`, `message` |
| `result` | Final event | `timestamp`, `status`, optional `error`, optional `stats` |

### Important Details Not Obvious From A Single Example

#### `message`

- Gemini emits a `message` event for the original user prompt.
- Assistant text is emitted as repeated `message` events with `role:
  "assistant"` and `delta: true`.
- Claudine should concatenate assistant `content` in arrival order.

#### `tool_use`

- Tool calls carry both a stable `tool_id` and the invoked `tool_name`.
- Arguments are emitted as `parameters`.
- This gives Claudine a real tool timeline for non-interactive Gemini runs
  without waiting for hook payloads.

#### `tool_result`

- Tool results only carry `tool_id`, not `tool_name`; correlation must be done
  by joining against the earlier `tool_use`.
- `status` is only `success` or `error`.
- When present, `error` has `{ type, message }`.
- Gemini currently reports cancelled tool calls as `success` in stream-json mode
  for legacy parity; Claudine should treat that as provider behavior, not as a
  true success guarantee.

#### `error`

- This is for non-fatal runtime conditions such as loop detection warnings.
- Fatal failures do not necessarily emit a standalone `error` event; Gemini can
  terminate with a final `result` event whose `status` is `error`.

#### `result.stats`

The current implementation emits an aggregated stats object:

| Field | Meaning |
|------|---------|
| `total_tokens` | Aggregate total across all models used in the run |
| `input_tokens` | Prompt/input tokens before cache subtraction |
| `output_tokens` | Candidate/output tokens |
| `cached` | Cached prompt tokens |
| `input` | Non-cached input tokens |
| `duration_ms` | Whole session wall-clock duration |
| `tool_calls` | Total completed tool calls |

Two nuances matter:

- `input_tokens` and `input` are not duplicates. `input_tokens` is the full
  prompt-side token count; `input` is the non-cached portion.
- The public headless docs mention per-model token usage breakdowns, but the
  installed v0.33.1 stream-json type and formatter only emit the simplified
  aggregate `stats` object. Claudine should code to the observed stream, not to
  the broader doc wording.

## What To Surface On STDERR

For wrapped non-interactive Gemini runs, stderr should show compact session
metadata while stdout remains the assistant response.

### On `init`

Recommended compact line:

```text
  Session: b5b53246 | Model: auto-gemini-3
```

This gives the user a provider session ID they can correlate with Gemini's own
logs and Claudine's reporting.

### On `tool_use` / `tool_result`

Do not print every tool event by default. That would turn stderr into a noisy
trace for normal runs. Instead:

- collect tool events in memory
- optionally print them only in verbose mode
- always feed them into logging metadata

### On `error`

Render warnings immediately on stderr:

```text
  Warning: Loop detected, stopping execution
```

These are operationally important and are not represented well by exit code
alone.

### On `result`

Recommended summary line:

```text
  Duration: 15.5s | Tokens: 32,983 in -> 87 out | Cache: 0 | Tools: 0
```

On failures:

```text
  Gemini failed [FatalTurnLimitedError]: Reached max session turns...
```

## What To Feed Into Claudine Logging

The existing reporting pipeline already knows how to ingest:

- `extra.model`
- `extra.token_usage`
- `extra.cost_usd`
- `session_id`
- `interactive`

That means we do not need a separate reporting system for Gemini stream-json.
We should synthesize one wrapper-side event in the existing Claudine JSONL
format and let the normal reporting sync ingest it.

## Recommended Synthetic Event Shape

After the wrapped Gemini session finishes, write one synthetic Claudine event
with:

- `provider = gemini`
- `event = session_end` or `turn_complete`
- `session_id = init.session_id`
- `extra.model = init.model`
- `extra.stream_status = result.status`
- `extra.stream_error = result.error` when present
- `extra.stream_stats = result.stats` as the raw provider summary
- `extra.token_usage = { input, output, total, cache_read }`
- `extra.tool_calls = result.stats.tool_calls`
- `extra.gemini_tools = [...]` summarized from `tool_use`/`tool_result`

Recommended normalized token mapping:

| Claudine key | Gemini source |
|-------------|---------------|
| `token_usage.input` | `stats.input_tokens` |
| `token_usage.output` | `stats.output_tokens` |
| `token_usage.total` | `stats.total_tokens` |
| `token_usage.cache_read` | `stats.cached` |

Also preserve Gemini's distinct non-cached input number under a provider-native
field:

- `extra.stream_stats.input`

That avoids losing the difference between total prompt tokens and non-cached
prompt tokens.

## Best Integration Point

This belongs in the wrapper layer, not in the Gemini hook adapter.

Reason:

- Gemini hook adapters parse hook payloads from `settings.json` hooks.
- `stream-json` is a separate headless stdout protocol.
- The wrapper already owns child-process execution, stdout/stderr routing, and
  non-interactive UX.

The clean design is:

1. When Claudine wraps a Gemini non-interactive run without an explicit raw
   output request, invoke Gemini with `--output-format stream-json`.
2. Parse stdout line-by-line in the wrapper.
3. Reconstruct assistant text to stdout.
4. Emit concise metadata to stderr.
5. Persist one synthetic Claudine JSONL event for reporting.

## Output-Mode Rules

Recommended behavior by mode:

| User intent | Wrapper behavior |
|------------|------------------|
| Default wrapped non-interactive Gemini run | Force internal `stream-json`, parse it, print assistant text to stdout |
| Explicit `--output text` | Respect request, skip forced stream parsing |
| Explicit `--output json` | Respect request, skip forced stream parsing |
| Explicit `--output stream` | Pass through raw JSONL unchanged |

This avoids surprising machine consumers who explicitly asked for raw stream
events, while still upgrading the default wrapped experience.

## Logging And Telemetry Strategy

Gemini itself already supports OpenTelemetry for logs, metrics, and traces. That
is useful upstream, but Claudine should still persist a normalized wrapper-side
summary because:

- Claudine reporting is JSONL-first today
- Claudine queries operate on its own SQLite index derived from those JSONL logs
- users want cross-provider reporting with one schema, not Gemini-only OTEL

The practical split should be:

- Gemini OTEL remains provider-native observability
- Claudine writes a normalized synthetic summary event for cross-provider
  session reporting
- Claudine may later forward the same normalized fields to its logging platform
  or OTEL exporter without re-parsing raw Gemini JSONL again

## Recommended Implementation Order

1. Fix Gemini wrapper flag mapping so universal `--output stream` correctly maps
   to `--output-format stream-json`.
2. Add a streaming parser in `claudine/cli/src/commands/wrap/exec.rs` for
   line-by-line Gemini stdout interception.
3. Add a small Gemini-specific parser struct in the wrapper layer for:
   - assistant text accumulation
   - tool timeline correlation by `tool_id`
   - final stats extraction
4. Write one synthetic Claudine JSONL event at session end using the normalized
   token fields above.
5. Add stderr summaries for `init`, `error`, and `result`.

## Summary

Gemini's `stream-json` output gives Claudine exactly the metadata that plain
text mode hides: provider session ID, resolved model, aggregate token usage,
cache usage, duration, warnings, and tool-call activity.

The right approach is not to expose raw JSONL by default. The right approach is
to consume it inside the wrapper, keep stdout clean for the assistant response,
surface compact diagnostics on stderr, and persist one normalized synthetic
event into Claudine's existing logging/reporting pipeline.

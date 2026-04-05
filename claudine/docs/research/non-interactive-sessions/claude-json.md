# Strategy for Parsing Claude Code Stream-JSON Output

## Background

When Claude Code runs with `--verbose --output-format stream-json`, it emits a
rich sequence of newline-delimited JSON objects to **stdout**. This structured
output replaces the plain-text response and gives Claudine full visibility into
session lifecycle, errors, token usage, costs, and model metadata — none of
which are available in the default `--print` text mode.

This document analyzes the stream-json format and proposes how Claudine should
parse it, what to surface to the user, and how to feed it into our reporting
pipeline.

---

## Message Types

A Claude Code stream-json session emits these top-level `type` values, in order:

| Type | Subtype | When | Key Fields |
|------|---------|------|------------|
| `system` | `hook_started` | Before each hook runs | `hook_name`, `hook_event` |
| `system` | `hook_response` | After each hook completes | `exit_code`, `outcome`, `stdout`, `stderr` |
| `system` | `init` | After hooks, before model call | `model`, `apiKeySource`, `session_id`, `tools`, `mcp_servers`, `permissionMode`, `claude_code_version` |
| `assistant` | — | Model response (may repeat per turn) | `message.content[]`, `message.model`, `message.usage`, optional `error` |
| `rate_limit_event` | — | After model response (subscription only) | `rate_limit_info.status`, `resetsAt`, `rateLimitType`, `overageStatus` |
| `result` | `success` | End of session | `is_error`, `result`, `total_cost_usd`, `duration_ms`, `duration_api_ms`, `usage`, `modelUsage`, `stop_reason` |

### Notable Differences by Auth Mode

| Field | API Key (`ANTHROPIC_API_KEY`) | Subscription (`apiKeySource: "none"`) |
|-------|-------------------------------|---------------------------------------|
| `init.model` | `claude-sonnet-4-6` (may default differently) | `claude-opus-4-6[1m]` |
| `init.apiKeySource` | `"ANTHROPIC_API_KEY"` | `"none"` |
| `assistant.error` | `"billing_error"` when balance is low | absent on success |
| `assistant.message.model` | `"<synthetic>"` on error | actual model id |
| `rate_limit_event` | absent | present with reset times |
| `result.total_cost_usd` | `0` on error | real cost |

---

## What to Display to the User

### STDOUT: The Response

STDOUT should contain **only the assistant's text response**, exactly as today's
`parse_captured_output` works — extract `content[].text` from `assistant`
messages. This keeps compose pipelines clean: the captured output is the
document body, nothing else.

```
# Extraction rule:
type == "assistant" AND message.content[].type == "text"
→ concatenate all text values
```

For error cases where `assistant.error` is set, the content text is the error
message (e.g. "Credit balance is too low"). This still goes to stdout for
capture, but the caller checks `result.is_error` to decide whether to treat it
as a successful composition.

### STDERR: Session Metadata & Diagnostics

STDERR is where Claudine should surface operational information. This is
displayed to the user (not captured by compose) and provides immediate
diagnostic value.

#### On Session Start (from `init`)

```
  Session: 8341ed81 | Claude Code 2.1.76
  Model: claude-opus-4-6[1m] | Auth: subscription
  Permission: default | Fast mode: off
```

Key fields to surface:

- **`model`** — which model is actually being used
- **`apiKeySource`** — `"ANTHROPIC_API_KEY"` vs `"none"` (subscription) — critical
  for diagnosing billing errors
- **`session_id`** — abbreviated, for cross-referencing logs
- **`claude_code_version`** — helps diagnose version-specific issues
- **`permissionMode`** — `default` vs others
- **`mcp_servers`** — count of enabled/disabled (not the full list)

Display should be compact (1-2 lines) and only shown when `--quiet` is not set.

#### On Error (from `assistant` with `error` field)

```
  Error [billing_error]: Credit balance is too low
  Auth: ANTHROPIC_API_KEY — switch to subscription or add credits
```

The `error` field on `assistant` messages is the primary error classifier.
Known values observed: `"billing_error"`. We should render these prominently
on stderr with actionable guidance.

#### On Completion (from `result`)

```
  Duration: 8.3s (API: 4.0s) | Turns: 1
  Tokens: 36,398 in → 14 out (cache: 36,395 created)
  Cost: $0.23 | Model: claude-opus-4-6[1m]
```

Key fields to surface:

- **`duration_ms`** / **`duration_api_ms`** — total vs API time shows hook/overhead
- **`num_turns`** — how many model turns the session used
- **`total_cost_usd`** — the cost (0 on error, real cost on success)
- **`usage`** — input/output/cache token breakdown
- **`stop_reason`** — `"end_turn"` (normal), `"stop_sequence"` (may indicate error)
- **`is_error`** — whether the session failed

Display only when not `--silent`. When `--quiet`, show a single summary line.

#### Rate Limit Info (from `rate_limit_event`)

```
  Rate limit: allowed | Resets: 2026-03-14 03:00 | Overage: allowed
```

Only present for subscription users. Show when `status != "allowed"` or when
verbose. The `resetsAt` is a Unix timestamp.

---

## Enhancing the Logging Pipeline

### Current State

Claudine's reporting pipeline (`claudine/lib/src/reporting/`) ingests JSONL
event logs written by hooks during provider sessions. It tracks sessions,
tools, turns, tokens, and costs via SQLite. However, for **non-interactive /
compose** sessions, the hooks fire but the rich stream-json data is never
captured — we only get the hook events (SessionStart, SessionEnd) without
the model-level detail.

### What Stream-JSON Adds

The `result` and `assistant` messages contain data that is currently
**unavailable** to the reporting pipeline for non-interactive sessions:

| Data | Current Source | Stream-JSON Source |
|------|---------------|-------------------|
| Token usage | Hook events (if provider emits them) | `result.usage` — always present |
| Cost | Not available for wrapped sessions | `result.total_cost_usd` |
| Model used | `init` event via hooks (if present) | `result.modelUsage` keys |
| API latency | Not available | `result.duration_api_ms` |
| Error classification | Exit code only | `assistant.error` field |
| Cache efficiency | Not available | `result.usage.cache_read_input_tokens` / `cache_creation_input_tokens` |
| Permission denials | Not available | `result.permission_denials` array |
| Stop reason | Not available | `result.stop_reason` |
| Per-model breakdown | Not available | `result.modelUsage` (multi-model sessions) |
| Context window used | Not available | `modelUsage[model].contextWindow` / `maxOutputTokens` |
| Service tier | Not available | `result.usage.service_tier` |
| Rate limit status | Not available | `rate_limit_event.rate_limit_info` |

### Proposed Integration

#### 1. Parse Result into a Structured Type

Define a `ClaudeSessionResult` struct (in `claudine/lib`) that captures the
fields from the `result` message:

```rust
pub struct ClaudeSessionResult {
    pub session_id: String,
    pub is_error: bool,
    pub error_type: Option<String>,       // from assistant.error
    pub result_text: String,
    pub stop_reason: String,
    pub duration_ms: u64,
    pub duration_api_ms: u64,
    pub num_turns: u32,
    pub total_cost_usd: f64,
    pub usage: TokenUsage,
    pub model_usage: HashMap<String, ModelUsage>,
    pub permission_denials: Vec<String>,
    pub api_key_source: String,           // from init
    pub model: String,                    // from init
    pub claude_code_version: String,      // from init
    pub rate_limit: Option<RateLimitInfo>,
}
```

#### 2. Return Parsed Result from `parse_captured_output`

Currently `parse_captured_output` returns `String` (just the response text).
For providers that emit structured output, we need richer return types. Two
approaches:

**Option A — Keep the trait simple, parse twice:**
`parse_captured_output` continues to return only the response text.
A separate method `parse_session_metadata(&self, raw: &str) -> Option<Value>`
returns the full parsed data for logging. Both operate on the same captured
stdout.

**Option B — Return a richer struct:**
Change `parse_captured_output` to return a struct with both the response text
and optional metadata. This avoids parsing twice but changes the trait
signature.

**Recommendation:** Option A. Parsing JSON lines twice is cheap, and it keeps
the existing trait contract stable. The metadata parsing method is only called
by the logging path, not the compose path.

#### 3. Feed into Reporting Pipeline

After a captured session completes, extract the `result` message and write
a synthetic JSONL event to the session log with the cost/token/model data.
This enriches the existing `SessionInfo` with:

- `total_cost_usd` (currently 0 for all wrapped sessions)
- `total_input_tokens`, `total_output_tokens` (from `result.usage`)
- `total_cache_read_tokens` (from `result.usage.cache_read_input_tokens`)
- `model` (from `result.modelUsage` keys)
- Per-model token breakdown for multi-model sessions

#### 4. Error Classification for Retry Logic

The `assistant.error` field enables smart retry behavior in compose pipelines:

| Error | Action |
|-------|--------|
| `billing_error` | Fail immediately with actionable message — no retry will help |
| `rate_limit` (if it exists) | Retry after `rate_limit_info.resetsAt` |
| `overloaded` (if it exists) | Backoff and retry |
| Network/timeout | Retry with backoff |

Currently, compose only checks `exit_code`. With stream-json, we can
distinguish "ran out of money" from "model overloaded" from "prompt was
rejected" and give precise feedback.

---

## Implementation Notes

### Claude Profile Changes

Claude's `prepare_captured_output` should inject:

```
--verbose --output-format stream-json
```

The `--verbose` flag is necessary for the full `init` block with `apiKeySource`,
version, and plugin details.

### Gemini Comparison

Gemini's stream-json is simpler:

```json
{"type":"message","role":"assistant","content":"...","delta":true}
{"type":"result","status":"success","stats":{"total_tokens":33646,...}}
```

Claude's format is richer (hooks, init, rate limits, per-model usage) but
follows the same principle: structured JSON lines where we filter by type.
The `parse_captured_output` / `parse_session_metadata` pattern works for both.

### What NOT to Parse

The `init` message contains large arrays (`tools`, `slash_commands`, `skills`,
`agents`) that are useful for debugging but should NOT be stored in the
reporting database. They would bloat the SQLite store. Instead:

- Log the full `init` to JSONL (already happens via hooks)
- Extract only the scalar fields (`model`, `apiKeySource`, `version`, etc.)
  for the session metadata struct
- The arrays are available in the raw log files if needed for debugging

---

## Summary

Stream-json output from Claude Code transforms Claudine's wrapped sessions from
opaque "run and hope" to fully observable pipelines. The key wins:

1. **Error diagnosis** — `billing_error` vs network vs model errors, surfaced
   immediately on stderr instead of a silent empty response
2. **Cost visibility** — per-session and per-model cost tracking for compose
   operations that currently show $0.00
3. **Cache efficiency** — see how much of the prompt was cached vs freshly
   created, enabling prompt optimization
4. **API vs total latency** — distinguish model thinking time from hook/startup
   overhead
5. **Rate limit awareness** — subscription users see when they're approaching
   limits, enabling preemptive throttling in batch compose operations

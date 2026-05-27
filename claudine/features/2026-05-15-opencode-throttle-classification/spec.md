# OpenCode 429 Classification — Overload vs. Rate-Limit vs. Usage Cap

## Problem

When claudine wraps OpenCode running `kimi-for-coding/kimi-k2.6`, the live
stream repeatedly reports `Usage limit reached for k2p6 (kimi-for-coding)`
while the session keeps making progress normally.

A live session log (`~/.claudine/logs/2026-05-15.jsonl`, 166 rate-limit
classifications) showed every one is identical:

```
error_name : AI_APICallError
is_fatal    : false
status_code : 429
responseBody: {"error":{"type":"rate_limit_error",
               "message":"The engine is currently overloaded, please try again later"}}
isRetryable : true
```

- **0** entries carry a usage-cap code (`"code":"1308"`)
- **0** entries carry `maxRetriesExceeded`
- **166/166** are HTTP 429 whose message is `The engine is currently
  overloaded, please try again later` — transient **server overload**, and
  intermittent (the session recovers and progresses throughout).

claudine's classifier flags any 429 as `RateLimit` and renders it with the
hardcoded wording `Usage limit reached for <model>` — a flatly wrong,
alarming label for "the provider's servers are busy."

## Root Cause

HTTP 429 ("Too Many Requests") is reused by Kimi and other
Anthropic-style APIs for conditions on **two unrelated axes**, and
`classify_llm_failure` collapses them all into a single classification:

```rust
let is_rate_limit = status_code == Some(429)
    || haystack.contains("\"code\":\"1308\"")
    || haystack.contains("Usage limit reached")
    || (is_fatal && status_code == Some(429));
```

The 429 status code alone is ambiguous. The real signal is the response
body — `error.type` and the message text.

## Conceptual Model — three separate conditions

| Condition | Axis | Nature | Retryable |
|---|---|---|---|
| **Server / engine overload** | provider capacity | the provider's infrastructure is busy — nothing to do with the account | yes, transient |
| **Rate limiting** | account frequency | this account sent requests too fast (concurrency / RPM / TPM / TPD) | yes, transient |
| **Usage cap** | account allowance | the account's cumulative quota is spent | no — terminal until reset |

Server overload is the *provider's* capacity; the other two are the
*account's* consumption. They are not flavors of one another. Any of the
three can arrive over HTTP 429.

## Goal

Classify an OpenCode 429 (and retry-exhaustion failure) into four distinct
kinds so server overload reads as overload, rate-limiting reads as
rate-limiting, and only a genuine cap or an exhausted-retry failure
terminates the run.

| Kind | Outcome | Message |
|---|---|---|
| `Overloaded` | non-terminal `Warning`, session continues | `server overloaded; will retry` |
| `RateLimited` | non-terminal `Warning`, session continues | `request throttled; will retry` |
| `UsageCap` | terminal `Error`, kill child | `usage limit reached for <model>[; resets at <t>]` |
| `RetriesExhausted` | terminal `Error`, kill child | `provider 429s did not clear after retries` |

## Error-Vocabulary Layers

Classification must respect which layer a signal lives in.

### Layer 1 — OpenCode-common (every provider)

OpenCode routes every provider through the Vercel AI SDK. The envelope —
`AI_APICallError`, `AI_RetryError`, `maxRetriesExceeded`, `statusCode`,
`isRetryable`, `responseBody`, `url` — and HTTP status codes are identical
across providers. Safe to key off for any provider.

### Layer 2 — provider-specific (`responseBody.error`)

| Provider | Vocabulary | Coverage |
|---|---|---|
| **ZAI / Zhipu** (`zai-coding-plan`, `api.z.ai`) | numeric codes `12xx`/`13xx`; `1308` = usage cap | known — `get_provider_code_description`, commit `79f7cce0` ("ZAI error code lookup"); **ZAI-only**, not universal |
| **Kimi / Moonshot standard API** (`api.moonshot.ai`) | string `error.type`: `engine_overloaded_error`, `rate_limit_reached_error` (transient); `exceeded_current_quota_error` (cap) | known — official `platform.kimi.ai/docs/api/errors` |
| **Kimi coding endpoint** (`kimi-for-coding`, `api.kimi.com/coding/v1/messages`) | Anthropic-compatible: `rate_limit_error`, `overloaded_error`, … — differs from the standard Kimi API | **partial** — transient overload confirmed; **no confirmed hard-cap type** (see Open Questions) |
| OpenAI / Gemini / Codex | own vocabularies | unknown |

The coding endpoint emits the generic `error.type: "rate_limit_error"` for
*both* overload and rate-limiting, so `error.type` alone is insufficient
there — the **message text** is the discriminator.

## Design

### 1. `events.rs` — `ProviderLimitKind` enum

Add to [`claudine/lib/src/stream/logs/opencode/events.rs`](../../lib/src/stream/logs/opencode/events.rs):

```rust
/// Classification of an OpenCode 429 / retry-exhaustion signal.
///
/// Server overload and rate-limiting are distinct conditions on unrelated
/// axes (provider capacity vs. account consumption); see the spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderLimitKind {
    /// The provider's servers are busy. Transient, retryable, not a cap.
    Overloaded,
    /// This account sent requests too fast. Transient, retryable.
    RateLimited,
    /// The account's usage allowance is exhausted. Terminal.
    UsageCap,
    /// A 429 wrapped in `AI_RetryError` / `maxRetriesExceeded` — the call
    /// failed after exhausting retries. Terminal.
    RetriesExhausted,
}
```

Rename `LogClassification::RateLimit` to `LogClassification::ProviderLimit`,
replacing the `is_fatal: bool` field with `kind: ProviderLimitKind`. The
`error_name` field is **dropped** — it is now redundant with `kind` (the
old values `"AI_APICallError"` / `"AI_RetryError"` were a proxy for the
fatal/non-fatal distinction that `kind` now encodes directly). The
remaining fields (`status_code`, `reset_at`, `provider_id`, `model_id`,
`provider_error`) are retained.

### 2. `errors.rs` — kind computation

`classify_llm_failure` replaces the `is_rate_limit` / `is_fatal` booleans
with kind computation from the raw line (`haystack`):

- **Error context** is present when **`record.tags.get("error").is_some()`**.
  An extractable HTTP status or a known error name is *not* sufficient on
  its own — only the presence of an `error` tag proves the line came from
  an OpenCode error envelope rather than from echoed tool output or quoted
  text. A terminal classification is only ever produced from an actual
  provider rejection.
- `has_cap` = `haystack.contains("\"code\":\"1308\"")` *(ZAI)*
  **or** `haystack.contains("exceeded_current_quota_error")` *(Kimi std)*
  **or** `haystack.contains("Usage limit reached")` *(legacy phrase)*.
- `has_429` = `extract_status_code(haystack) == Some(429)`.
- `exhausted` = `haystack.contains("AI_RetryError")`
  **or** `haystack.contains("maxRetriesExceeded")`.
- `is_overload` = `contains_any_ci(haystack, &["overload", "engine_overloaded_error"])`.
  (Use the existing `contains_any_ci` helper — `str::contains` is
  case-sensitive in Rust and would miss `"Overloaded"` / `"OVERLOAD"`.)

Resolution order (first match wins):

1. `has_cap` **and** error context → `ProviderLimit { kind: UsageCap }`.
   **Cap-with-context wins over retries-exhausted on purpose**: a 429 that
   exhausts retries *and* carries a 1308 / `exceeded_current_quota_error`
   signal is fundamentally a cap, not a network failure. The precise
   message (`usage limit reached for <model>`) is the actionable one;
   demoting it to `provider 429s did not clear after retries` would undo
   the distinction this feature exists to create.
2. `has_429` **and** `exhausted` → `ProviderLimit { kind: RetriesExhausted }`.
   (Retries exhausted = the call failed and no cap signal was present.)
3. `has_cap` **without** error context → not a `ProviderLimit`; emit a
   non-fatal `ApiFailure` carrying the provider's own message (advisory
   path, §4). Information preserved, severity not escalated. This is the
   primary defense against false-positive termination from echoed tool
   output or quoted error text that happens to mention a usage limit.
4. `has_429` **and** `is_overload` → `ProviderLimit { kind: Overloaded }`.
5. `has_429` → `ProviderLimit { kind: RateLimited }`.
6. otherwise → existing `ApiFailure` / `AuthFailure` paths, unchanged.

The literal `Usage limit reached` phrase is **kept** as a cap signal —
never discarded — but graded by the error-context gate so prose mentioning
a usage limit cannot, on its own, terminate a healthy run.

### 3. `reasoning.rs` — `on_provider_limit` branches on kind

The handler (renamed from `on_rate_limit`) in
[`claudine/lib/src/stream/logs/opencode/reasoning.rs`](../../lib/src/stream/logs/opencode/reasoning.rs)
branches on `ProviderLimitKind`:

| Kind | Semantic event | `state.rate_limit` / `rate_limit_events` | Early termination |
|---|---|---|---|
| `Overloaded` | `Warning { "server overloaded; will retry" }` | **untouched** | none |
| `RateLimited` | `Warning { "request throttled; will retry" }` | **untouched** | none |
| `UsageCap` | `Error { terminal, ApiRemote, "usage limit reached for <model>; resets at <t>" }` | set | **always** fire |
| `RetriesExhausted` | `Error { terminal, ApiRemote, "provider 429s did not clear after retries" }` | set (no `reset_at`) | **always** fire |

- The two transient kinds emit one `Warning` per occurrence — **no
  deduplication** — and do **not** set `state.rate_limit` or bump
  `rate_limit_events`. Consequence: the compose-loop's
  `on_rate_limit: pause|abort` logic (which reads `summary.rate_limit`) no
  longer pauses on transient overload or throttling — a correctness win
  with no extra code.
- The current `stdout_seen` guard on early termination is **removed** for
  the terminal kinds: a genuine cap or exhausted-retry failure terminates
  regardless of prior stdout activity. `fire_early_termination` keeps its
  existing once-only guard (`early_terminate_fired`).

### 4. Advisory path — surface the provider message

When a cap phrase appears **without** error context (resolution step 2),
the non-fatal `ApiFailure` message is the provider's own text, extracted
via the existing `extract_provider_message` (`errors.rs:186`); if
extraction yields nothing, fall back to the record's trailing message. The
user sees the actual provider wording, not a claudine paraphrase.

### 5. `render_rate_limit_message`

Retained only for the `UsageCap` / `RetriesExhausted` terminal messages.
The two transient messages are fixed strings built directly in the handler.

## Open Questions / Follow-ups

- **Kimi coding-endpoint hard cap is unknown.** The coding endpoint
  (`api.kimi.com/coding/v1/messages`) is Anthropic-compatible and has not
  been observed emitting a distinct cap type — it may reuse
  `rate_limit_error`. Until a real coding-endpoint cap sample is captured,
  a `kimi-for-coding` run can only terminate via `RetriesExhausted`
  (`maxRetriesExceeded`); a lone coding-endpoint 429 is always
  `Overloaded` or `RateLimited`. `exceeded_current_quota_error` →
  `UsageCap` covers the **standard** Kimi API only.
- **User-Agent deprioritization** — a report on a different tool
  (`earendil-works/pi#3585`) claims `api.kimi.com` deprioritizes
  non-whitelisted User-Agents. Unverified for OpenCode and inconsistent
  with the observed intermittent recovery; **not pursued in this feature**.

## Out of Scope

- `ApiFailure` / `AuthFailure` classification for non-429 errors.
- Other providers (Claude, Codex, Gemini, Qwen).
- The compose-loop `on_rate_limit` knob — no change needed; it benefits
  automatically from the transient kinds not setting `state.rate_limit`.
- Transient-warning deduplication / coalescing — declined; every 429 emits
  its own line.
- Any User-Agent injection / OpenCode-request modification.

## Behavior Changes

1. Transient Kimi "engine overloaded" 429s render as
   `server overloaded; will retry` instead of `Usage limit reached`; the
   session is no longer mislabeled as capped.
2. **Intentional:** a real usage cap seen *after* stdout activity now
   terminates the run (kills the OpenCode child) instead of warning and
   continuing. The error-context gate (`record.tags.error.is_some()`,
   §2) is the safety net — only an actual OpenCode error envelope can
   trigger termination; quoted/echoed cap text in tool output cannot.
3. **Semantic regression for `kimi-for-coding`:** because the coding
   endpoint has no confirmed hard-cap error type (Open Questions), a
   user who actually exhausts their Kimi coding allowance will see
   `provider 429s did not clear after retries`, never
   `usage limit reached for k2p6`. This is strictly less specific than
   today's (incorrect-but-precise) wording. Acceptable until a real
   coding-endpoint cap sample is captured and added to `has_cap`.

## Test Plan

Fixtures live in
[`claudine/lib/tests/fixtures/logs/`](../../lib/tests/fixtures/logs/).

- **New fixture** `opencode-429-overload.txt` — the real Kimi line from the
  2026-05-15 log (HTTP 429, `rate_limit_error`, message "engine is
  currently overloaded", `isRetryable:true`). Asserts
  `ProviderLimitKind::Overloaded`.
- **New tests:**
  - `Overloaded` → non-terminal `Warning` `server overloaded; will retry`,
    `state.rate_limit` stays `None`, no early termination.
  - `RateLimited` (a 429 with no overload wording) → non-terminal `Warning`
    `request throttled; will retry`, no early termination.
  - `429` + `maxRetriesExceeded` → `RetriesExhausted` → terminal `Error` +
    early termination.
  - `exceeded_current_quota_error` (standard Kimi API) with error context →
    `UsageCap` → terminal `Error` + early termination.
  - cap phrase with no error context → non-fatal `ApiFailure` carrying the
    provider message; no termination.
- **Updated tests** — the existing `1308` fixture and tests now classify
  `1308` as `UsageCap` and terminate regardless of stdout activity.
  Assertions, the `on_rate_limit` → `on_provider_limit` rename, and the
  following test renames are applied together (the old names assert the
  opposite of the new behavior and must not survive):

  | Old name | New name |
  |---|---|
  | `classifies_rate_limit_with_reset_time` | `classifies_usage_cap_with_reset_time` |
  | `fixture_rate_limit_classifies` | `fixture_usage_cap_classifies` |
  | `rate_limit_after_stdout_emits_warning_no_early_terminate` | `usage_cap_after_stdout_emits_terminal_error_and_early_terminate` |
  | `rate_limit_without_retry_error_is_warning_even_before_stdout` | `usage_cap_without_retry_error_still_terminates` |
  | `rate_limit_before_stdout_emits_terminal_error_and_early_terminate` | `usage_cap_before_stdout_emits_terminal_error_and_early_terminate` |
  | `rate_limit_fires_early_termination_only_once` | `provider_limit_fires_early_termination_only_once` |
- `cargo test -p claudine` and `cargo clippy -p claudine` pass clean.

## Affected Files

- `claudine/lib/src/stream/logs/opencode/events.rs` — `ProviderLimitKind`
  enum; `LogClassification::RateLimit` → `ProviderLimit`.
- `claudine/lib/src/stream/logs/opencode/errors.rs` — four-kind
  computation, error-context gate, overload detection, advisory path.
- `claudine/lib/src/stream/logs/opencode/reasoning.rs` —
  `on_rate_limit` → `on_provider_limit`, per-kind branching, removal of the
  `stdout_seen` termination guard for terminal kinds.
- `claudine/lib/tests/fixtures/logs/opencode-429-overload.txt` — new fixture.
- `.claude/skills/claudine/opencode-event-sources.md` — **repo-root**
  `.claude/`, not under `claudine/`. Update the rate-limit row to
  describe the four kinds and the two-axis model.

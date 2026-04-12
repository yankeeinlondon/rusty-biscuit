# Auth / Quota / Billing Badges — Detailed Design

## Problem

When a wrapped agentic CLI session fails or degrades, the root cause is often
one of:

1. **Authentication failure** — expired or missing API key / OAuth token
2. **Billing / credit exhaustion** — plan credits depleted, card declined
3. **Quota / rate-limit pressure** — throughput throttled, retries needed

Today Claudine already detects some of these signals in isolation but never
wires them into a **single, structured, human-readable badge** on the stderr
summary line. This design fills that gap.

## Existing Infrastructure

| Piece | Location | What it provides |
|---|---|---|
| Error kind extraction | `claude.rs:139-154`, `codex.rs:145-169`, `gemini.rs:164-192`, etc. | `error_kind` + `error_message` from stream |
| Rate-limit parsing | `claude.rs:215-228` (`RateLimitInfo`) | `is_throttled`, `retry_after_ms`, `message` |
| Auth error detection | `output.rs:872-876` | `authentication_error` → styled hint |
| Billing error detection | `claude.rs` test: `billing_error` kind | Already in `StreamExecutionSummary.error_kind` |
| Dashboard URLs | `provider.rs:514-526` (`usage_dashboard_url`) | Per-provider billing console deep link |
| Billing model metadata | `agents/*.rs` (`BillingCapabilities`) | Subscription / per-token / prepaid / provider-only |
| Context pressure | `kimi.rs:156-184` (`ContextUsage`) | Context window fill percentage |
| Stderr formatters | `stderr.rs` | `format_start_summary`, `format_completion_summary` |

## Proposed Type

```rust
/// Structured diagnostic badge emitted after a session ends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionBadge {
    /// Machine-readable category.
    pub category: BadgeCategory,
    /// How severe: informational, warning, or error.
    pub severity: BadgeSeverity,
    /// Short label (e.g. "Auth", "Billing", "Quota").
    pub label: &'static str,
    /// Human-readable message describing the condition.
    pub message: String,
    /// Optional deep link to the provider's remediation page.
    pub remediation_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BadgeCategory {
    Auth,
    Billing,
    Quota,
    RateLimit,
    ContextPressure,
    Permission,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BadgeSeverity {
    Info,
    Warning,
    Error,
}
```

## Badge Derivation Algorithm

```
fn derive_badges(summary: &StreamExecutionSummary, provider: Provider) -> Vec<SessionBadge>
```

The function inspects the summary fields that **already exist** on
`StreamExecutionSummary` plus the provider's `usage_dashboard_url()` and
`BillingCapabilities`. It produces zero or more badges.

### Decision tree

1. **Auth badge** — if `error_kind` matches an auth failure pattern
2. **Billing badge** — if `error_kind` matches a billing failure pattern
3. **Rate-limit badge** — if `rate_limit.is_throttled == Some(true)` or
   `error_kind` matches a rate-limit pattern
4. **Quota badge** — if `error_kind` matches a quota/credit-exhaustion pattern
   (distinct from generic billing)
5. **Context pressure badge** — if `context_usage.percent >= 80`
6. **Permission badge** — if `error_kind` matches a permission/forbidden pattern

Each badge maps `Provider::usage_dashboard_url()` into `remediation_url` where
applicable.

### Error-kind Pattern Table

```rust
const AUTH_KINDS: &[&str] = &[
    "authentication_error",
    "authentication_failed",
    "auth_error",
    "invalid_api_key",
    "unauthorized",
];

const BILLING_KINDS: &[&str] = &[
    "billing_error",
    "payment_required",
    "insufficient_quota",
    "credit_balance_tool_low",
    "account_suspended",
];

const RATE_LIMIT_KINDS: &[&str] = &[
    "rate_limit_error",
    "rate_limit",
    "rate_limit_exceeded",
    "too_many_requests",
    "resource_exhausted",
];

const QUOTA_KINDS: &[&str] = &[
    "quota_exceeded",
    "insufficient_quota",
    "usage_limit_reached",
    "capacity_limit",
];

const PERMISSION_KINDS: &[&str] = &[
    "permission_error",
    "forbidden_error",
    "access_denied",
    "forbidden",
];
```

Some error kinds overlap (e.g. `insufficient_quota` is both billing and quota).
The derivation function uses the first matching category in the priority order:
Auth > Billing > Quota > RateLimit > Permission.

## Provider-by-Provider Analysis

### Tier 1: Full Badge Support (implement first)

These providers have **stable structured error signals** in their stream output
and a **billing dashboard URL**.

#### Claude Code (Anthropic)

| Signal | Available | Source |
|---|---|---|
| Auth errors | Yes | `error.type = "authentication_error"` |
| Billing errors | Yes | `error.type = "billing_error"` |
| Rate-limit events | Yes | `rate_limit_event` with `is_throttled`, `retry_after_ms` |
| Quota/credit | Yes | `billing_error` with message "Insufficient credits" |
| Dashboard URL | Yes | `https://console.anthropic.com/settings/billing` |
| Billing model | Subscription + per-token | `BillingCapabilities` |
| Error contract | Structured JSON in `stream-json` | `{"type":"error","error":{"type":"...","message":"..."}}` |

**Badge examples:**

```
⚠ billing_error — Insufficient credits
  → https://console.anthropic.com/settings/billing

⚠ authentication_error — Invalid API key
  → https://console.anthropic.com/settings/billing

⏳ rate_limit — Rate limit exceeded (retry in 5.0s)
  → https://console.anthropic.com/settings/billing
```

**Implementation notes:**
- `ClaudeStreamParser` already parses `error_kind` and `error_message`
  (`claude.rs:139-154`)
- `RateLimitInfo` is already populated from `rate_limit_event`
  (`claude.rs:215-228`)
- `Provider::Claude.usage_dashboard_url()` already returns the billing console
- No parser changes needed — purely a badge derivation from existing summary

#### Codex CLI (OpenAI)

| Signal | Available | Source |
|---|---|---|
| Auth errors | Yes | `error_type = "authentication_error"` or inferred from status 401 |
| Billing errors | Yes | `error_type = "billing_error"` or status 402 |
| Rate-limit errors | Yes | `error_type = "rate_limit"` (already tested in `codex.rs:468`) |
| Quota/credit | Yes | `insufficient_quota` error kind |
| Dashboard URL | Yes | `https://platform.openai.com/usage` |
| Billing model | Subscription + per-token | `BillingCapabilities` |
| Error contract | Structured JSONL | `{"type":"error","error_type":"...","error_message":"..."}` |

**Badge examples:**

```
⚠ rate_limit — Too many requests
  → https://platform.openai.com/usage

⚠ insufficient_quota — You exceeded your current quota
  → https://platform.openai.com/usage
```

**Implementation notes:**
- `CodexStreamParser` already parses `error_type` and `error_message`
  (`codex.rs:145-169`)
- Error kinds tested: `rate_limit` (`codex.rs:468`)
- OpenAI returns well-documented HTTP-mapped error types
- No parser changes needed

### Tier 2: Partial Badge Support (implement second)

These providers expose **some** structured signals but may have gaps in
auth-specific or billing-specific error classification.

#### Gemini CLI (Google)

| Signal | Available | Source |
|---|---|---|
| Auth errors | Partial | Errors surface as generic `error` events with severity; auth-specific classification is not guaranteed |
| Billing errors | Partial | `result.status = "error"` with error object, but `type` is often `FatalTurnLimitedError` rather than `billing_error` |
| Rate-limit errors | Partial | Not explicitly parsed; Gemini may surface as retry headers, not stream events |
| Quota/credit | Partial | Google AI Studio quota errors are possible but classification varies |
| Dashboard URL | Yes | `https://aistudio.google.com/billing` |
| Billing model | Subscription + per-token | `BillingCapabilities` |
| Error contract | Semi-structured | `{"type":"error","severity":"warning","message":"..."}` and `{"type":"result","status":"error","error":{}}` |

**Badge examples:**

```
⚠ error — Loop detected
  → https://aistudio.google.com/billing

⚠ error — Reached max turns
  → https://aistudio.google.com/billing
```

**Implementation notes:**
- `GeminiStreamParser.handle_error()` (`gemini.rs:164-192`) uses `severity`
  as `error_kind` and `message` as `error_message`
- The `severity` field is `"warning"` or `"error"`, not a domain-specific
  error type
- Auth/billing discrimination must rely on **message substring matching**
  (e.g. contains "API key", "quota", "billing") rather than structured kind
- This is less reliable than Claude/Codex but still actionable
- May need to inspect the `result.error.type` field for more specific
  classification (e.g. `FatalTurnLimitedError` → quota badge)

**Gap:** Gemini's `handle_error` maps `severity` into `error_kind` rather than
extracting a domain-specific error type. A future improvement would add a
secondary classification pass that inspects `error_message` content for known
auth/billing substrings.

#### Kimi Code (Moonshot AI)

| Signal | Available | Source |
|---|---|---|
| Auth errors | Partial | Wire-mode errors may include auth failures, classified under `error.type` |
| Billing errors | Partial | Possible via Moonshot API error responses |
| Rate-limit errors | Partial | Possible but not explicitly tested |
| Quota/credit | Partial | Kimi membership quota is subscription-style |
| Context pressure | Yes | `context_usage` with threshold-based warning (`kimi.rs:156-184`) |
| Dashboard URL | Yes | `https://platform.moonshot.cn/console/account` |
| Billing model | Subscription + per-token | `BillingCapabilities` |
| Error contract | Structured JSON | `{"type":"error","error":{"type":"...","message":"..."}}` |

**Badge examples:**

```
⚠ context_pressure — Context window pressure: 86% used (110000/128000 tokens)
  → https://platform.moonshot.cn/console/account

⚠ billing_error — Subscription quota exceeded
  → https://platform.moonshot.cn/console/account
```

**Implementation notes:**
- `KimiStreamParser` has the richest **context pressure** support of any
  provider, with `ContextUsage` already populated (`kimi.rs:156-184`)
- Error classification follows the same pattern as Claude (nested `error.type`)
- Auth/billing errors are plausible from Moonshot's API but the exact error
  kinds are not documented; badge derivation will use substring matching as
  a fallback

#### Qwen Code (Alibaba)

| Signal | Available | Source |
|---|---|---|
| Auth errors | Partial | Possible via Bailian API errors |
| Billing errors | Partial | Qwen OAuth free quota exhaustion is possible |
| Rate-limit errors | Partial | Not explicitly parsed |
| Quota/credit | Partial | Prepaid credits model; exhaustion errors possible |
| Dashboard URL | Yes | `https://bailian.console.aliyun.com/` |
| Billing model | Prepaid + subscription + per-token | `BillingCapabilities` |
| Error contract | Structured JSON | `{"type":"error","error":{"type":"...","message":"..."}}` |

**Badge examples:**

```
⚠ quota_exceeded — OAuth free quota depleted
  → https://bailian.console.aliyun.com/

⚠ auth_error — Invalid access token
  → https://bailian.console.aliyun.com/
```

**Implementation notes:**
- `QwenStreamParser` uses the same error extraction pattern as Gemini
- Qwen's free OAuth quotas are a common failure mode; substring matching for
  "quota" and "free" in error messages would help classify these
- The Bailian console URL is available for remediation

### Tier 3: Delegated Billing (limited badge support)

These providers **delegate billing to upstream model providers**. They do not
have their own billing dashboards. Auth/billing errors that surface come from
the upstream API (e.g. OpenAI, Anthropic, Google) and are proxied through.

#### OpenCode

| Signal | Available | Source |
|---|---|---|
| Auth errors | Via upstream | Error messages propagated from the configured model provider |
| Billing errors | Via upstream | Error messages propagated from the configured model provider |
| Rate-limit errors | Via upstream | Error messages propagated from the configured model provider |
| Dashboard URL | **None** | `Provider::OpenCode.usage_dashboard_url()` returns `None` |
| Billing model | Provider-only | `BillingCapabilities` — "OpenCode delegates cost and billing to configured model providers" |

**Badge examples:**

```
⚠ authentication_error — Invalid API key for configured provider
  → (no dashboard URL available — operator must check their upstream provider)

⚠ error — API timeout
  → (no dashboard URL available)
```

**Implementation notes:**
- OpenCode's error classification is opaque: the `error_kind` and
  `error_message` come from the upstream provider but are passed through
  without enrichment
- Badge derivation can still match error kinds against the pattern table
  (since upstream providers like OpenAI and Anthropic emit structured errors)
- The **missing dashboard URL** is the main limitation — badges will show
  the error but cannot offer a deep remediation link
- A future enhancement could attempt to detect which upstream provider
  OpenCode is using (from model name or config) and map to the appropriate
  dashboard URL

#### Goose (Block)

| Signal | Available | Source |
|---|---|---|
| Auth errors | Via upstream | Errors from configured LLM provider |
| Billing errors | Via upstream | Errors from configured LLM provider |
| Rate-limit errors | Via upstream | Errors from configured LLM provider |
| Dashboard URL | **None** | `Provider::Goose.usage_dashboard_url()` returns `None` |
| Billing model | Provider-only | `BillingCapabilities` — "Goose itself is free; all cost is provider API usage" |

**Implementation notes:**
- Same situation as OpenCode: Goose is a pass-through for provider errors
- Goose has no native stream parser for structured errors (it uses MCP stream)
- Badge support depends entirely on whether error messages from the upstream
  provider are parseable
- **Recommendation:** Only provide generic badges without remediation URLs;
  consider Goose "best effort" tier

#### Roo Code

| Signal | Available | Source |
|---|---|---|
| Auth errors | Via upstream | Errors from Roo Cloud or BYOK provider |
| Billing errors | Partial | Roo Cloud uses prepaid credits; exhaustion errors are Roo-specific |
| Rate-limit errors | Via upstream | Errors from configured provider |
| Dashboard URL | **None** | `Provider::RooCode.usage_dashboard_url()` returns `None` |
| Billing model | Prepaid credits + per-token | `BillingCapabilities` — "Roo Cloud uses prepaid credits, BYOK providers bill per token" |

**Implementation notes:**
- Roo Code has a unique prepaid-credit model that could produce Roo-specific
  billing/credit exhaustion errors
- The stream parser does emit `TaskToolFailed` and `Error` events
- Roo does not expose a billing dashboard URL today
- A future enhancement could add a Roo-specific dashboard URL if one becomes
  available

## Summary Matrix

| Provider | Auth | Billing | Rate-Limit | Quota | Context | Dashboard URL | Tier |
|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| Claude | Structured | Structured | Structured | Structured | — | Yes | 1 |
| Codex | Structured | Structured | Structured | Structured | — | Yes | 1 |
| Gemini | Substring | Substring | — | Substring | — | Yes | 2 |
| Kimi Code | Substring | Substring | Substring | Substring | Structured | Yes | 2 |
| Qwen Code | Substring | Substring | Substring | Substring | — | Yes | 2 |
| OpenCode | Via upstream | Via upstream | Via upstream | Via upstream | — | No | 3 |
| Goose | Via upstream | Via upstream | Via upstream | Via upstream | — | No | 3 |
| Roo Code | Via upstream | Partial (prepaid) | Via upstream | Partial | — | No | 3 |

**Structured** = provider emits domain-specific error types (e.g. `billing_error`)
**Substring** = classification relies on matching substrings in error messages
**Via upstream** = errors proxied from the configured LLM provider
**Partial** = some provider-specific signals exist but coverage is incomplete

## Implementation Plan

### Phase 1: Core types + Tier 1 providers

1. **Define `SessionBadge`, `BadgeCategory`, `BadgeSeverity`** in a new module
   `claudine/lib/src/stream/badges.rs`
2. **Define error-kind pattern tables** as constants in the same module
3. **Implement `derive_badges(summary, provider)`** function
4. **Wire into `format_completion_summary`** in `stderr.rs` — append badges
   after the existing summary line
5. **Wire into `format_compact_completion`** for `--quiet` mode
6. **Add badges to `StreamExecutionSummary`** as `pub badges: Vec<SessionBadge>`
   (computed on `finish()`, not parsed from stream)
7. **Write tests** for:
   - Claude billing_error → billing badge with dashboard URL
   - Claude authentication_error → auth badge with dashboard URL
   - Claude rate_limit_event → rate-limit badge with retry hint
   - Codex rate_limit → rate-limit badge with dashboard URL
   - Codex insufficient_quota → quota badge with dashboard URL

### Phase 2: Tier 2 providers

1. **Add substring-based classification** for Gemini, Kimi, Qwen
2. **Wire Kimi context pressure** into a context-pressure badge
3. **Add per-provider substring tables** for auth/billing messages
4. **Write tests** for:
   - Gemini "Loop detected" → warning badge
   - Gemini "Reached max turns" → quota badge
   - Kimi context_usage > 80% → context-pressure badge
   - Qwen "free quota" substring → quota badge

### Phase 3: Tier 3 providers (best-effort)

1. **Enable badge derivation for delegated providers** using upstream error
   kinds where available
2. **Suppress remediation URL** when `usage_dashboard_url()` returns `None`
3. **Write tests** for:
   - OpenCode error with upstream auth kind → auth badge (no URL)
   - Goose error with upstream rate-limit kind → rate-limit badge (no URL)
   - Roo Code credit exhaustion → billing badge (no URL)

## Rendering

### Normal mode

```
✓ 12s · 1K in / 567 out · 89 cache · $0.0042 · 3 tools
⚠ billing_error — Insufficient credits
  → https://console.anthropic.com/settings/billing
```

### Quiet mode

```
✓ 12s · 1K→567 tokens · $0.0042 | ⚠ billing
```

### Silent mode

No output (existing behavior).

## File Changes

| File | Change |
|---|---|
| `claudine/lib/src/stream/badges.rs` | **New** — `SessionBadge` type, `derive_badges()`, pattern tables |
| `claudine/lib/src/stream/summary.rs` | Add `badges: Vec<SessionBadge>` to `StreamExecutionSummary` |
| `claudine/lib/src/stream/stderr.rs` | Render badges in `format_completion_summary`, `format_compact_completion` |
| `claudine/lib/src/stream/mod.rs` | Export `badges` module |
| `claudine/lib/src/stream/reporting.rs` | Include badges in `summary_to_event_meta` JSONL output |
| `claudine/cli/src/output.rs` | Use badge rendering in Prose output (styled badges) |

## Open Questions

1. **Should badges be computed eagerly in `finish()` or lazily on render?**
   Eager computation in `finish()` keeps the `StreamExecutionSummary` as the
   single source of truth and makes JSONL logging trivial. Recommended.

2. **Should substring matching for Tier 2 providers be configurable?**
   Initial implementation should use hardcoded tables. If users report
   misclassification, the tables can be updated. A future enhancement could
   allow user-configured patterns via config.

3. **Should Tier 3 providers attempt upstream provider detection?**
   OpenCode and Goose both support multiple upstream providers. Detecting
   which provider is in use (from model name or config) would enable
   provider-specific dashboard URLs. This is deferred to Phase 3+.

4. **Badge deduplication?**
   If both `error_kind = "billing_error"` and a rate-limit event fire in the
   same session, both badges should appear. No deduplication needed.

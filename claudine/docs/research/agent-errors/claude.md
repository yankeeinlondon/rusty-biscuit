---
$schema: ./_schema.yaml
created: 2026-07-12
last_updated: 2026-07-12
agent: claude-code
model: claude-opus-4-8
docs: https://code.claude.com/docs/en/agent-sdk/typescript
# Ordered buckets checked against the structured error-kind discriminator.
# Sequence order IS the cascade order (first substring hit wins). Every needle
# is a preserved Phase-A seed (evidence: seed); this fleet run proposes no
# kind-branch delta.
kind_buckets:
  - kind: api_remote
    needles:
      - text: billing
        evidence: seed
      - text: rate_limit
        evidence: seed
      - text: ratelimit
        evidence: seed
      - text: quota
        evidence: seed
      - text: overload
        evidence: seed
      - text: api_error
        evidence: seed
      - text: upstream
        evidence: seed
      - text: server
        evidence: seed
  - kind: configuration
    needles:
      - text: auth
        evidence: seed
      - text: permission
        evidence: seed
      - text: config
        evidence: seed
  - kind: interrupted
    needles:
      - text: interrupt
        evidence: seed
      - text: cancel
        evidence: seed
      - text: abort
        evidence: seed
# Ordered buckets checked against the free-form error message. All Phase-A seeds
# are preserved; no addition is proposed — the seeded `overloaded` needle
# already covers the capacity/overload motivating class.
msg_buckets:
  - kind: api_remote
    needles:
      - text: rate limit
        evidence: seed
      - text: quota
        evidence: seed
      - text: billing
        evidence: seed
      - text: credit
        evidence: seed
      - text: api error
        evidence: seed
      - text: overloaded
        evidence: seed
  - kind: configuration
    needles:
      - text: api key
        evidence: seed
      - text: authentication
        evidence: seed
      - text: not authorized
        evidence: seed
      - text: permission denied
        evidence: seed
  - kind: interrupted
    needles:
      - text: interrupt
        evidence: seed
      - text: cancel
        evidence: seed
      - text: aborted
        evidence: seed
gaps:
  - area: numeric-http-codes
    notes: >-
      Claude surfaces HTTP status numbers (429 rate limit, 529 overloaded, 401
      auth) inside message prose rather than as a discrete numeric wire code.
      They are deliberately NOT proposed as substring needles: as raw substrings
      they collide with token counts, timestamps, and IDs in non-error frames.
      The named phrasings (`rate limit`, `overloaded`) already classify the same
      families safely, so the numeric surface is recorded as a gap for a future
      exact-match / `code_buckets`-style channel rather than the substring
      cascade.
changes: []
requires_claudine_update: false
reason: >-
  All Phase-A seeds are preserved verbatim and the seeded `overload` (kind) /
  `overloaded` (msg) needles already cover the capacity/overload motivating
  class, so this fleet run proposes no runtime vocabulary delta. Research does
  not change classification behavior; any future capacity/numeric-code work is
  recorded as a gap for Phase C adjudication.
---

# Error Vocabulary Research on Claude Code

## Overview

Claude Code (Anthropic's coding CLI, `@anthropic-ai/claude-code`, closed
source; the TypeScript Agent SDK types are published) surfaces errors in
non-interactive mode through its `--output-format stream-json` JSONL event
stream. Errors arrive as free-form message prose on `assistant` message frames
carrying an `error` field, and — for account/usage pressure — as structured
`rate_limit_event` frames whose typed `rate_limit_info` shape the `signals/`
topic owns for detection. For the `SemanticErrorKind` rendering layer researched
here, the classifier keys primarily off message text, with a thinner
kind-discriminator branch. Because Claude Code is a thin client over the
Anthropic API, much of its error prose is pass-through of API-level errors
(rate limits, 429/529 overload, auth failures).

## Error Surfaces

### Structured Error Kinds

The SDK's message stream carries a short kind/type token on some error frames.
The seeded `kind_buckets` classify from that token; it is a diagnostic
side-channel rather than a first-class stable enum, so the kind branch is
best-effort and message text does the primary work.

### Message Text

The primary error surface. `assistant` frames with an `error` field carry
human-facing prose formatted from the API/SDK error types. All message-branch
seeds classify from this surface.

### Rate-Limit / Usage Events

`rate_limit_event` frames with a typed `rate_limit_info` (`rateLimitType`,
`status`) are the account-wide usage-pressure channel. These are **detection**
records owned by `signals/claude.md` (`usage_cap_approaching`, etc.); this topic
cites them and does not duplicate them as rendering needles.

### Numeric Codes

Claude Code exposes no Kimi-style JSON-RPC numeric wire codes. HTTP status
numbers (429, 529, 401) appear inside message prose only and are not modeled as
`code_buckets` — see `## Collisions and Precedence` and the `numeric-http-codes`
gap.

## Rate Limit, Quota, and Billing

Claude Code passes through Anthropic rate-limit and billing errors. The seeded
kind needles `billing`, `rate_limit`, `ratelimit`, `quota` and the message
needles `rate limit`, `quota`, `billing`, `credit` all classify to `api_remote`
and are preserved. The `signals/` topic separately owns the account usage-cap
*detection* records (`usage_cap_approaching` on `rate_limit_event`); per the D9
boundary those stay in `signals/claude.md`, cited rather than duplicated.

## Authentication, Permission, and Configuration

Seeded needles `auth`, `permission`, `config` (kind) and `api key`,
`authentication`, `not authorized`, `permission denied` (message) classify to
`configuration` and are preserved. Claude auth failures ("invalid x-api-key",
"authentication_error", `401`) surface as message prose matched by `api key` /
`authentication`. No new auth-family needle is proposed.

## Interruption and Cancellation

Seeded needles `interrupt`, `cancel`, `abort` (kind) and `interrupt`, `cancel`,
`aborted` (message) classify to `interrupted` and are preserved. Note the
existing seed asymmetry preserved from Phase A: the kind branch matches `abort`
while the message branch requires `aborted`.

## Upstream and Server

Seeded kind needles `api_error`, `upstream`, `server` classify to `api_remote`,
covering pass-through provider-infrastructure failures (5xx, upstream gateway
errors). Preserved unchanged.

## Capacity and Overload

The capacity/overload motivating class is already covered by preserved seeds:
`overload` (kind branch) and `overloaded` (message branch) both classify to
`api_remote`. Anthropic documents the `overloaded_error` type / HTTP 529
"Overloaded" response, which Claude Code passes through into message prose, so
these substrings are documented, safe, capacity-family classifiers. No new
capacity needle is required and no gap is recorded for this class.

## Collisions and Precedence

- **`overloaded` / `overload`** (seeds) — narrow, unambiguous capacity markers;
  they do not appear in Claude success/progress prose. Precedence unchanged.
- **`credit`** (seed, message branch) — matches "credit"/"credits"; acceptable
  in the `api_remote` billing family. Sticky seed, untouched.
- **`api_error` / `api error`** (seeds) — broad but scoped to the `api_remote`
  bucket. Sticky; untouched.
- **Bare HTTP numbers (429/529/401)** — rejected as substring needles: they
  collide with token counts and IDs in non-error frames. Recorded as the
  `numeric-http-codes` gap.

## Quirks and Gaps

- **Numeric HTTP codes are unsafe substrings** — need an exact-match surface,
  not the substring cascade. (`gaps`: `numeric-http-codes`.)
- **`abort` vs `aborted` seed asymmetry** — preserved from Phase A, not a delta.
- **Usage-cap events are a detection concern** — owned by `signals/claude.md`,
  not rendering vocabulary here.

## Sources

- [Claude Code Agent SDK (TypeScript) reference](https://code.claude.com/docs/en/agent-sdk/typescript)
  — `stream-json` message/error frame shapes, `SDKRateLimitInfo`.
- [Anthropic API errors](https://docs.claude.com/en/api/errors)
  — documented `overloaded_error` (HTTP 529) and `rate_limit_error` (HTTP 429)
  types that Claude Code passes through into message prose.
- `claudine/docs/research/signals/claude.md` — the `usage_cap_approaching`
  **detection** records for the same rate-limit surface (D9 cross-citation;
  detection, not rendering vocabulary).
- `claudine/docs/providers/facts/claude.yaml` (`error_vocabulary:`) — the
  Phase-A seed transcribed verbatim from `lib/src/stream/providers/claude.rs`.

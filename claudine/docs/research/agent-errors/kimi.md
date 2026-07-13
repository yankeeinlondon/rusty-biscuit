---
$schema: ./_schema.yaml
created: 2026-07-12
last_updated: 2026-07-12
agent: claude-code
model: claude-opus-4-8
docs: https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html
# Kimi is the roster's only NUMERIC-CODE classifier: its wire mode is JSON-RPC,
# so error frames carry an exact `error.code` that is matched FIRST (before any
# substring branch), then falls through to the message branch on an unknown
# code. There is no kind-discriminator branch. Sequence order IS the cascade
# order (first substring hit wins) on the message branch. Every needle and code
# is a preserved Phase-A seed (evidence: seed).
msg_buckets:
  - kind: api_remote
    needles:
      - text: rate limit
        evidence: seed
      - text: quota
        evidence: seed
      - text: billing
        evidence: seed
      - text: api error
        evidence: seed
      - text: upstream
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
      - text: auth
        evidence: seed
      - text: config
        evidence: seed
  - kind: interrupted
    needles:
      - text: interrupt
        evidence: seed
      - text: cancel
        evidence: seed
      - text: aborted
        evidence: seed
# Ordered numeric wire-code buckets, matched exactly as the FIRST cascade step.
# `name` links each code to its `protocol/kimi.rs` constant. Kimi extension codes
# classify to their family; standard JSON-RPC codes are explicitly mapped to
# agent_native so a standard code carrying auth/rate-limit prose does not fall
# through to the message branch and get re-classified. All codes are preserved
# Phase-A seeds.
code_buckets:
  - kind: configuration
    codes:
      - code: -32004
        name: AUTH_EXPIRED
        evidence: seed
  - kind: api_remote
    codes:
      - code: -32005
        name: CHAT_PROVIDER_ERROR
        evidence: seed
  - kind: agent_native
    codes:
      - code: -32700
        name: PARSE_ERROR
        evidence: seed
  - kind: agent_native
    codes:
      - code: -32600
        name: INVALID_REQUEST
        evidence: seed
  - kind: agent_native
    codes:
      - code: -32601
        name: METHOD_NOT_FOUND
        evidence: seed
  - kind: agent_native
    codes:
      - code: -32602
        name: INVALID_PARAMS
        evidence: seed
  - kind: agent_native
    codes:
      - code: -32603
        name: INTERNAL_ERROR
        evidence: seed
gaps:
  - area: capacity-overload-phrasing
    notes: >-
      No seeded Kimi needle or code covers the generic capacity/overload
      motivating class. Kimi wire mode surfaces provider-side failures through
      the `-32005 CHAT_PROVIDER_ERROR` extension code and, at the retry layer, an
      HTTP-429 `StepRetry` (a `rate_limited` DETECTION record in
      `signals/kimi.md`) — not a dedicated `overloaded`/`503` string or code. The
      wire protocol defines no `RESOURCE_EXHAUSTED`-style capacity code. No
      capacity substring needle or code is graduated without a source-pinned
      string; recorded as a gap for Phase C adjudication.
changes: []
requires_claudine_update: false
reason: >-
  All Phase-A seed needles and numeric codes are preserved verbatim; no runtime
  vocabulary delta is proposed. The capacity/overload class is recorded as an
  explicit gap (Kimi's overload surface is a detection-layer 429 `StepRetry`, not
  a rendering string/code). Research does not change classification behavior.
---

# Error Vocabulary Research on Kimi Code

## Overview

Kimi Code (Moonshot AI's coding CLI, `moonshotai/kimi-cli`) is the roster's only
provider whose non-interactive protocol is **JSON-RPC** ("wire mode"). Error
frames therefore carry an exact numeric `error.code`, which Claudine's Kimi
classifier matches as the **first** cascade step — before any substring branch —
and only falls through to the free-form message branch when the code is unknown.
This is why Kimi is the sole provider with a `code_buckets` branch, and why it
has no kind-discriminator branch: the numeric code *is* the structured
discriminator.

## Error Surfaces

### Numeric Codes

The primary, most reliable surface. Kimi's JSON-RPC `error.code` is exact-matched
first. Two families exist: Kimi-specific extension codes (`-32004 AUTH_EXPIRED`,
`-32005 CHAT_PROVIDER_ERROR`) and the five standard JSON-RPC codes
(`-32700`/`-32600`/`-32601`/`-32602`/`-32603`). The `name` field on each code
links it to its `protocol/kimi.rs` wire constant; graduation drops the name and
keeps only the numeric code.

### Message Text

The fallback surface, walked only when the numeric code is unknown. Free-form
`error.message` prose carries the remaining classifier work. All message-branch
seeds classify from this surface.

### Structured Error Kinds

None separate from the numeric code — the JSON-RPC code subsumes the
kind-discriminator role, so there is no `kind_buckets`.

## Numeric Code Mapping

All seven seeded codes are preserved with their protocol-constant names:

- **`-32004 AUTH_EXPIRED` → `configuration`** — the OAuth re-login requirement;
  narrower than any auth substring and matched first.
- **`-32005 CHAT_PROVIDER_ERROR` → `api_remote`** — upstream/provider failure
  pass-through.
- **`-32700 PARSE_ERROR`, `-32600 INVALID_REQUEST`, `-32601 METHOD_NOT_FOUND`,
  `-32602 INVALID_PARAMS`, `-32603 INTERNAL_ERROR` → `agent_native`** — the five
  standard JSON-RPC codes are explicitly mapped to `agent_native`. This mapping
  is load-bearing: without it, a standard code whose message happened to carry
  auth/rate-limit prose would fall through to the message branch and be
  re-classified. Pinning them to `agent_native` keeps a protocol-level framing
  error a protocol error.

## Rate Limit, Quota, and Billing

Seeded message needles `rate limit`, `quota`, `billing`, `api error`, `upstream`
classify to `api_remote` and are preserved. At the wire layer, Kimi's rate-limit
surface is an HTTP-429 `StepRetry` frame — a `rate_limited` **detection** record
owned by `signals/kimi.md` (which must win over the broader `generation_retried`
`StepRetry` record). Per D9 that detection record stays in `signals/`; this
document renders the same family via the seeded `rate limit` message needle when
the failure reaches the terminal error frame.

## Authentication, Permission, and Configuration

Seeded message needles `api key`, `authentication`, `not authorized`,
`permission denied`, `auth`, `config` classify to `configuration` and are
preserved — but note the numeric `-32004 AUTH_EXPIRED` code matches auth failures
*first*, so these message needles only fire on auth prose that arrived without
the extension code. The `signals/kimi.md` `auth_invalid` record keys off
`-32004` directly.

## Interruption and Cancellation

Seeded message needles `interrupt`, `cancel`, `aborted` classify to `interrupted`
and are preserved.

## Capacity and Overload

No seeded Kimi needle or code covers the generic capacity/overload class. Kimi's
provider-side failure is the `-32005 CHAT_PROVIDER_ERROR` extension code and, at
the retry layer, the HTTP-429 `StepRetry` (a detection record). The wire protocol
defines no `RESOURCE_EXHAUSTED`-style capacity code and no dedicated
`overloaded`/`503` string was source-pinned in this run. Recorded as the
`capacity-overload-phrasing` gap rather than graduated.

## Collisions and Precedence

- **Code-first precedence** — the exact numeric match runs before every
  substring branch, so a coded error never risks substring misclassification.
- **`auth` / `config`** (seeds, message branch) — broad substrings, but they
  only fire on uncoded auth/config prose because `-32004` matches coded auth
  first. Sticky seeds, untouched.
- **`upstream`** (seed, message branch, `api_remote`) — scoped; matches upstream
  gateway prose.
- **Standard-code → `agent_native` mapping** — prevents a standard JSON-RPC code
  from leaking into the substring branch; the key precedence invariant here.

## Quirks and Gaps

- **Only numeric-code provider** — code buckets are matched first; message
  branch is the unknown-code fallback.
- **Standard JSON-RPC codes are pinned to `agent_native`** — deliberate, to stop
  message-branch re-classification.
- **Capacity class uncovered** — no capacity code/string; overload is a
  detection-layer 429 `StepRetry`. (`gaps`: `capacity-overload-phrasing`.)

## Sources

- [Kimi CLI wire-mode docs](https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html)
  — JSON-RPC wire protocol, `error.code` shape, initialize handshake.
- `claudine/docs/research/signals/kimi.md` — the `auth_invalid` (`-32004`),
  `rate_limited` (429 `StepRetry`), and `generation_retried` (`StepRetry`)
  **detection** records for the same wire stream (D9 cross-citation; detection,
  not rendering vocabulary).
- `claudine/docs/providers/facts/kimi.yaml` (`error_vocabulary:`) — the Phase-A
  seed transcribed verbatim from `lib/src/stream/providers/kimi.rs`, including
  the numeric code mapping and its `protocol/kimi.rs` constant comments.

---
$schema: ./_schema.yaml
created: 2026-07-12
last_updated: 2026-07-12
agent: claude-code
model: claude-opus-4-8
docs: https://geminicli.com/docs/cli/headless/
# Ordered buckets checked against the structured error-kind discriminator.
# Sequence order IS the cascade order (first substring hit wins). Note the
# configuration bucket is checked FIRST and the repeated api_remote bucket is a
# "late ApiRemote" second pass after interrupted. Every needle is a preserved
# Phase-A seed (evidence: seed).
kind_buckets:
  - kind: configuration
    needles:
      - text: auth
        evidence: seed
      - text: permission
        evidence: seed
      - text: config
        evidence: seed
      - text: denied
        evidence: seed
  - kind: api_remote
    needles:
      - text: rate
        evidence: seed
      - text: quota
        evidence: seed
      - text: billing
        evidence: seed
  - kind: interrupted
    needles:
      - text: interrupt
        evidence: seed
      - text: cancel
        evidence: seed
      - text: abort
        evidence: seed
  - kind: api_remote
    needles:
      - text: api
        evidence: seed
      - text: upstream
        evidence: seed
      - text: server
        evidence: seed
# Ordered buckets checked against the free-form error message. All Phase-A seeds
# are preserved; no addition is proposed.
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
  - area: capacity-overload-phrasing
    notes: >-
      No seeded Gemini needle covers the capacity/overload motivating class.
      Gemini CLI surfaces resource pressure as Google-API `RESOURCE_EXHAUSTED`
      status and HTTP 429/503 responses, but the exact CLI-rendered string on
      the `stream-json` error surface could not be commit-pinned to a
      `google-gemini/gemini-cli` version-tagged source permalink in this
      non-interactive fleet run. A capacity substring needle (`resource_exhausted`
      / `overloaded` / `503`) is therefore deliberately NOT graduated — proposing
      it without a pinned citation would be guessing. Recorded as a gap for a
      live research run and Phase C adjudication.
  - area: numeric-http-codes
    notes: >-
      HTTP status numbers (429, 503) appear in Gemini message prose but are not
      proposed as substring needles: as raw substrings they collide with token
      counts and IDs in non-error frames. They belong behind an exact-match
      surface, not the case-insensitive substring cascade.
changes: []
requires_claudine_update: false
reason: >-
  All Phase-A seeds are preserved verbatim; no runtime vocabulary delta is
  proposed. The capacity/overload class is recorded as an explicit gap rather
  than graduated, because its exact CLI phrasing could not be source-pinned in
  this fleet run. Research does not change classification behavior.
---

# Error Vocabulary Research on Gemini CLI

## Overview

Gemini CLI (Google's coding CLI, `google-gemini/gemini-cli`, open source)
surfaces errors in non-interactive mode through its `--output-format stream-json`
JSONL event stream. A run emits an `init` metadata frame, `message` chunks, and
a terminal `result` frame; failures surface either as a `result` with
`status: error` carrying a nested `error.type` / `error.message`, or as
non-fatal `error` frames mid-stream. Claudine's Gemini classifier keys off both
a short kind/type token (the `kind_buckets` branch) and the free-form message
text (the `msg_buckets` branch). Because Gemini CLI fronts the Google Generative
Language API, much of its error prose is pass-through of API-level errors.

## Error Surfaces

### Structured Error Kinds

The `result`/`error` frames carry a short `error.type` token (e.g.
`FatalTurnLimitedError`). The seeded `kind_buckets` classify from this token.
It is a real but thin contract — the fatal-turn-limit type is stable, but the
broader failure taxonomy is expressed in message prose.

### Message Text

The primary error surface. `result.error.message` and mid-stream `error` frame
prose formatted from the CLI's error types and API pass-through. All
message-branch seeds classify from this surface.

### Numeric Codes

Gemini CLI exposes no JSON-RPC numeric wire codes. HTTP status numbers (429,
503) appear inside message prose only and are not modeled as `code_buckets` —
see the `numeric-http-codes` gap.

## Rate Limit, Quota, and Billing

Seeded kind needles `rate`, `quota`, `billing` and message needles `rate limit`,
`quota`, `billing`, `api error` classify to `api_remote` and are preserved. The
`signals/` topic separately owns Gemini's `turn_limit_reached` (nested
`error.type = FatalTurnLimitedError`) **detection** record — that stays in
`signals/gemini.md` and is cited here rather than duplicated as a rendering
needle.

## Authentication, Permission, and Configuration

Checked first in the kind branch. Seeded kind needles `auth`, `permission`,
`config`, `denied` and message needles `api key`, `authentication`,
`not authorized`, `permission denied` classify to `configuration` and are
preserved. Gemini auth failures (OAuth / API-key / `PERMISSION_DENIED`) surface
as message prose matched by `authentication` / `permission denied`.

## Interruption and Cancellation

Seeded needles `interrupt`, `cancel`, `abort` (kind) and `interrupt`, `cancel`,
`aborted` (message) classify to `interrupted` and are preserved. Note the
preserved seed asymmetry: the kind branch matches `abort` while the message
branch requires `aborted`.

## Upstream and Server (late ApiRemote)

The repeated `api_remote` kind bucket `api`, `upstream`, `server` is the
"late ApiRemote" second pass, checked *after* `interrupted`. This ordering is a
preserved precedence quirk: a broad `api` substring must not shadow an
interruption classification, so it runs last. Preserved unchanged.

## Capacity and Overload

No seeded Gemini needle covers the capacity/overload motivating class. Gemini
fronts the Google Generative Language API, whose resource-pressure surface is
`RESOURCE_EXHAUSTED` (gRPC status) and HTTP 429/503. Gemini CLI passes these
through, but the exact CLI-rendered capacity string on the `stream-json` error
surface could not be commit-pinned to a version-tagged source permalink in this
non-interactive run. Rather than guess a substring needle, the class is recorded
as the `capacity-overload-phrasing` gap for a live research run and Phase C
adjudication.

## Collisions and Precedence

- **`rate`** (seed, kind branch) — broad substring, but scoped to `api_remote`
  and it matches "rate limit"/"rate_limit" prose; acceptable. Sticky, untouched.
- **`api`** (seed, kind branch, late pass) — the broadest seed; ordered last so
  it cannot shadow `configuration` or `interrupted`. Flagged for Phase C
  awareness; not touched.
- **`denied`** (seed, configuration) — matches "denied"/"permission denied";
  scoped to `configuration`, checked first. Sticky.
- **Bare HTTP numbers (429/503)** and **`resource_exhausted`** — deliberately
  withheld (no pinned citation / collision risk); recorded as gaps.

## Quirks and Gaps

- **Capacity phrasing unpinned** — `RESOURCE_EXHAUSTED`/503 not source-pinned in
  this run. (`gaps`: `capacity-overload-phrasing`.)
- **Numeric HTTP codes are unsafe substrings** — need an exact-match surface.
  (`gaps`: `numeric-http-codes`.)
- **Late-ApiRemote ordering** — the second `api`/`upstream`/`server` bucket runs
  after `interrupted` by design.
- **`abort` vs `aborted` seed asymmetry** — preserved from Phase A, not a delta.

## Sources

- [Gemini CLI headless docs](https://geminicli.com/docs/cli/headless/)
  — `--output-format stream-json`, `init`/`message`/`result` frames, `error`
  shape.
- `claudine/docs/research/signals/gemini.md` — the `turn_limit_reached` /
  `tokens_consumed` / `model_resolved` **detection** records for the same stream
  (D9 cross-citation; detection, not rendering vocabulary).
- `claudine/docs/providers/facts/gemini.yaml` (`error_vocabulary:`) — the
  Phase-A seed transcribed verbatim from `lib/src/stream/providers/gemini.rs`.

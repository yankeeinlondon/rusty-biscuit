---
$schema: ./_schema.yaml
created: 2026-07-12
last_updated: 2026-07-12
agent: claude-code
model: claude-opus-4-8
docs: https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/
# Ordered buckets checked against the structured error-kind discriminator.
# Sequence order IS the cascade order (first substring hit wins). The repeated
# api_remote bucket is a "late ApiRemote" second pass after interrupted. Every
# needle is a preserved Phase-A seed (evidence: seed).
kind_buckets:
  - kind: api_remote
    needles:
      - text: rate
        evidence: seed
      - text: quota
        evidence: seed
      - text: billing
        evidence: seed
  - kind: configuration
    needles:
      - text: auth
        evidence: seed
      - text: config
        evidence: seed
      - text: permission
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
      No seeded Qwen needle covers the capacity/overload motivating class. Qwen
      Code is a Gemini-CLI fork fronting DashScope / OpenAI-compatible endpoints,
      whose resource-pressure surface is HTTP 429/503 and provider-specific
      throttle prose. The exact CLI-rendered capacity string on the
      `stream-json` error surface could not be commit-pinned to a
      `QwenLM/qwen-code` version-tagged source permalink in this non-interactive
      fleet run, so a capacity substring needle is deliberately NOT graduated.
      Recorded as a gap for a live research run and Phase C adjudication.
  - area: numeric-http-codes
    notes: >-
      HTTP status numbers (429, 503) appear in Qwen message prose but are not
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

# Error Vocabulary Research on Qwen Code

## Overview

Qwen Code (Alibaba's coding CLI, `QwenLM/qwen-code`, open source, forked from
Gemini CLI) surfaces errors in non-interactive mode through its
`--output-format stream-json` JSONL event stream. Current headless output opens
with a `system` frame (`subtype: init`) carrying the resolved model and session
metadata, then streams `assistant`/`message` chunks and a terminal `result`;
failures surface as `error` frames with a structured `error.type` or as terminal
`result` errors and stderr diagnostics. Because Qwen Code inherits the Gemini CLI
architecture, its classifier shape mirrors Gemini's — a thin kind-discriminator
branch plus the primary message branch.

## Error Surfaces

### Structured Error Kinds

`error` frames carry an `error.type` token (the `signals/` topic observed a
`rate_limit` discriminator). The seeded `kind_buckets` classify from this token;
it is a real but thin contract inherited from the Gemini fork.

### Message Text

The primary error surface. `result.error.message` and `error` frame prose,
formatted from the CLI's error types and DashScope/OpenAI-compatible API
pass-through. All message-branch seeds classify from this surface.

### Numeric Codes

Qwen Code exposes no JSON-RPC numeric wire codes. HTTP status numbers appear in
message prose only and are not modeled as `code_buckets` — see the
`numeric-http-codes` gap.

## Rate Limit, Quota, and Billing

Seeded kind needles `rate`, `quota`, `billing` and message needles `rate limit`,
`quota`, `billing`, `api error` classify to `api_remote` and are preserved. The
`signals/` topic separately owns Qwen's `rate_limited` (`error.type = rate_limit`)
**detection** record — that stays in `signals/qwen.md` and is cited here rather
than duplicated as a rendering needle.

## Authentication, Permission, and Configuration

Seeded kind needles `auth`, `config`, `permission` and message needles `api key`,
`authentication`, `not authorized`, `permission denied` classify to
`configuration` and are preserved. Qwen auth failures (API-key / OAuth /
DashScope credential errors) surface as message prose matched by `api key` /
`authentication`.

## Interruption and Cancellation

Seeded needles `interrupt`, `cancel`, `abort` (kind) and `interrupt`, `cancel`,
`aborted` (message) classify to `interrupted` and are preserved. Note the
preserved seed asymmetry: the kind branch matches `abort` while the message
branch requires `aborted`.

## Upstream and Server (late ApiRemote)

The repeated `api_remote` kind bucket `api`, `upstream`, `server` is the
"late ApiRemote" second pass, checked *after* `interrupted` so a broad `api`
substring cannot shadow an interruption classification. Preserved unchanged.

## Capacity and Overload

No seeded Qwen needle covers the capacity/overload motivating class. Qwen Code
fronts DashScope / OpenAI-compatible endpoints whose resource-pressure surface is
HTTP 429/503 and provider throttle prose. The exact CLI-rendered capacity string
could not be commit-pinned to a version-tagged source permalink in this
non-interactive run, so no capacity substring needle is graduated. Recorded as
the `capacity-overload-phrasing` gap for a live research run and Phase C
adjudication.

## Collisions and Precedence

- **`rate`** (seed, kind branch) — broad but scoped to `api_remote`; matches
  "rate limit"/"rate_limit" prose. Sticky, untouched.
- **`api`** (seed, kind branch, late pass) — the broadest seed; ordered last so
  it cannot shadow `configuration` or `interrupted`. Flagged for Phase C
  awareness; not touched.
- **Bare HTTP numbers (429/503)** — deliberately withheld (collision risk);
  recorded as a gap.

## Quirks and Gaps

- **Capacity phrasing unpinned** — 429/503 CLI string not source-pinned in this
  run. (`gaps`: `capacity-overload-phrasing`.)
- **Numeric HTTP codes are unsafe substrings** — need an exact-match surface.
  (`gaps`: `numeric-http-codes`.)
- **Gemini-fork lineage** — Qwen's stream shape and error taxonomy inherit
  Gemini CLI's; its own error strings still require independent citation.
- **`abort` vs `aborted` seed asymmetry** — preserved from Phase A, not a delta.

## Sources

- [Qwen Code headless docs](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/)
  — `--output-format stream-json`, `system`/`init`/`result`/`error` frames.
- `claudine/docs/research/signals/qwen.md` — the `rate_limited` /
  `model_resolved` **detection** records for the same stream (D9 cross-citation;
  detection, not rendering vocabulary).
- `claudine/docs/providers/facts/qwen.yaml` (`error_vocabulary:`) — the Phase-A
  seed transcribed verbatim from `lib/src/stream/providers/qwen.rs`.

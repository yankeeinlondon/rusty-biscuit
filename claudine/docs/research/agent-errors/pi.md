---
$schema: ./_schema.yaml
created: 2026-07-12
last_updated: 2026-07-12
agent: claude-code
model: claude-opus-4-8
docs: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/json.md
# Pi is a message-only classifier: its JSONL stream surfaces failures as
# free-form assistant `errorMessage` prose on a `message_end` frame with
# `stopReason: error`, with no structured error-kind discriminator, so there is
# no `kind_buckets`. Sequence order IS the cascade order (first substring hit
# wins). Every needle is a preserved Phase-A seed (evidence: seed).
msg_buckets:
  - kind: api_remote
    needles:
      - text: rate limit
        evidence: seed
      - text: quota
        evidence: seed
      - text: billing
        evidence: seed
      - text: out of credits
        evidence: seed
      - text: overloaded
        evidence: seed
      - text: "503"
        evidence: seed
      - text: api error
        evidence: seed
      - text: api timeout
        evidence: seed
  - kind: configuration
    needles:
      - text: api key
        evidence: seed
      - text: authentication
        evidence: seed
      - text: no api key
        evidence: seed
      - text: not authorized
        evidence: seed
      - text: no models available
        evidence: seed
      - text: model not found
        evidence: seed
      - text: invalid model
        evidence: seed
  - kind: interrupted
    needles:
      - text: abort
        evidence: seed
      - text: cancel
        evidence: seed
      - text: interrupt
        evidence: seed
gaps:
  - area: structured-error-kind-discriminator
    notes: >-
      Pi's stock JSONL stream (`packages/coding-agent/docs/json.md`) carries no
      machine error-kind enum: failures arrive as a `message_end` frame with
      `message.stopReason: error` and free-form `message.errorMessage` prose.
      Classification therefore leans entirely on the message branch; there is no
      kind-discriminator surface to seed a `kind_buckets` from, so that branch
      is intentionally absent.
changes: []
requires_claudine_update: false
reason: >-
  All Phase-A seeds are preserved verbatim and the seeded `overloaded` / `503`
  needles already cover the capacity/overload motivating class, so this fleet
  run proposes no runtime vocabulary delta. Research does not change
  classification behavior.
---

# Error Vocabulary Research on Pi

## Overview

Pi (Earendil Works' coding agent, `earendil-works/pi`, open source) surfaces
errors in non-interactive JSON mode through its JSONL event stream. A failing
turn ends with a `message_end` frame whose `message.role` is `assistant`,
`message.stopReason` is `error`, and `message.errorMessage` carries the
human-facing failure prose. There is no structured machine error-kind enum on
the stream, so Claudine's Pi classifier keys entirely off that free-form message
text — the document therefore has a `msg_buckets` branch and no `kind_buckets`.

## Error Surfaces

### Message Text

The sole error surface. `message_end` (`message.errorMessage`) prose formatted
from Pi's provider-error handling. Every seeded needle classifies from this
surface.

### Structured Error Kinds

None. Pi's documented JSONL schema exposes `stopReason: error` but no typed
error-kind discriminator (recorded as a gap). The `signals/` topic keys its Pi
`usage_capped` / `no_funds` / `auth_invalid` **detection** records off the same
`errorMessage` substring surface — those stay in `signals/pi.md`.

### Numeric Codes

Pi exposes no JSON-RPC numeric wire codes. The `503` needle below is a substring
of the HTTP-status phrasing Pi renders in `errorMessage`, not a discrete wire
code, and lives in the message branch.

## Rate Limit, Quota, and Billing

Seeded needles `rate limit`, `quota`, `billing`, `out of credits`, `api error`,
`api timeout` classify to `api_remote` and are preserved. Pi passes provider
rate-limit and funding failures through into `errorMessage`; `out of credits`
and `billing` are Pi's account-funding phrasings. The `signals/` topic owns the
`usage_capped` (`quota exceeded`) and `no_funds` (`billing`) **detection**
records for the same surface (D9); this document renders the same family for the
`SemanticErrorKind` summary layer and cites rather than duplicates.

## Authentication, Permission, and Configuration

Seeded needles `api key`, `authentication`, `no api key`, `not authorized`,
`no models available`, `model not found`, `invalid model` classify to
`configuration` and are preserved. Pi's pre-request auth guidance
("No API key found") and model-selection failures surface as `errorMessage`
prose matched by `no api key` / `model not found` / `invalid model`.

## Interruption and Cancellation

Seeded needles `abort`, `cancel`, `interrupt` classify to `interrupted` and are
preserved. Note Pi's seeded ordering lists `abort` first, unlike the
alphabetical-looking order of other providers — this is a preserved precedence
detail, not a delta.

## Capacity and Overload

The capacity/overload motivating class is already covered by preserved seeds:
`overloaded` and `503` both classify to `api_remote` in the message branch. Pi
passes API-level 503 overload responses through into `errorMessage`, so these
substrings are documented capacity-family classifiers. No new capacity needle is
required and no gap is recorded for this class.

## Collisions and Precedence

- **`overloaded` / `503`** (seeds) — narrow capacity markers; `503` is a digit
  substring that could in principle appear in an ID, but Pi only emits it inside
  an error `stopReason` frame, so the collision surface is small. Sticky seeds,
  untouched.
- **`api error` / `api timeout`** (seeds) — scoped to the `api_remote` bucket;
  broad but preserved from Phase A.
- **`model not found` / `invalid model`** (seeds) — configuration-family;
  narrower than a bare `model` substring (deliberately not seeded).

## Quirks and Gaps

- **No structured error-kind enum** — classification leans entirely on message
  text. (`gaps`: `structured-error-kind-discriminator`.)
- **`503` as a substring** — safe only because it appears solely in error
  frames; would be unsafe as a general-purpose numeric needle.

## Sources

- [Pi JSON output docs](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/json.md)
  — `message_end` frame shape, `stopReason`, `errorMessage`.
- `claudine/docs/research/signals/pi.md` — the `usage_capped` / `no_funds` /
  `auth_invalid` **detection** records for the same `errorMessage` surface
  (D9 cross-citation; detection, not rendering vocabulary).
- `claudine/docs/research/agent-errors/_seeds/pi.yaml` — the immutable Phase-A
  seed transcribed verbatim from `lib/src/stream/providers/pi.rs`.

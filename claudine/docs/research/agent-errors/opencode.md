---
$schema: ./_schema.yaml
created: 2026-07-12
last_updated: 2026-07-12
agent: claude-code
model: claude-opus-4-8
docs: https://opencode.ai/docs/cli/
# Ordered buckets checked against the structured error-kind discriminator.
# Sequence order IS the cascade order (first substring hit wins). The repeated
# api_remote bucket is a "late ApiRemote" second pass after interrupted. Every
# needle is a preserved Phase-A seed (evidence: seed). Kilo reuses this same
# parser but selects its OWN distinct vocabulary (see kilo.md).
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
      - text: provider
        evidence: seed
      - text: model
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
      - text: api timeout
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
      - text: model not found
        evidence: seed
      - text: invalid model
        evidence: seed
      - text: providermodelnotfound
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
      No seeded OpenCode needle covers the capacity/overload motivating class in
      the RENDERING vocabulary. OpenCode's overload/usage-cap surface is real but
      lives in its promoted-stderr log stream as typed
      `LogClassification::ProviderLimit` kinds (`Overloaded`, `UsageCap`,
      `RetriesExhausted`, `RateLimited`) — a wire-level DETECTION concern owned
      by `signals/opencode.md`, not the `SemanticErrorKind` substring cascade.
      The stdout NDJSON stream this topic classifies does not carry a confirmed,
      source-pinned `overloaded`/`503` rendering string, so a capacity substring
      needle is deliberately NOT graduated here. Recorded as a gap; the overload
      family is handled by the signals detection layer today.
  - area: numeric-http-codes
    notes: >-
      HTTP status numbers (429, 503) appear in OpenCode's promoted-stderr log
      lines but are consumed by the signals detection classifier, not the
      rendering substring cascade. As raw substrings on the message branch they
      would collide with token counts and IDs, so they are not proposed as
      needles.
changes: []
requires_claudine_update: false
reason: >-
  All Phase-A seeds are preserved verbatim; no runtime vocabulary delta is
  proposed. The capacity/overload class is covered by the signals DETECTION layer
  (`LogClassification::ProviderLimit`) rather than this rendering vocabulary, and
  is recorded as an explicit gap here. Research does not change classification
  behavior.
---

# Error Vocabulary Research on OpenCode

## Overview

OpenCode (`sst/opencode`, open source) surfaces errors across two channels: its
stdout NDJSON event stream (parsed by `lib/src/stream/providers/opencode.rs`,
the subject of this rendering-vocabulary topic) and its stderr log stream, whose
provider-limit lines Claudine *promotes* and classifies with a dedicated typed
`LogClassification` engine. The `SemanticErrorKind` rendering vocabulary here
keys off the stdout stream's short kind token (`kind_buckets`) and free-form
message text (`msg_buckets`); OpenCode's richest error signal — usage caps,
retry exhaustion, provider overload — arrives on the stderr channel and is owned
by the `signals/` topic as *detection*, not by this rendering cascade.

## Error Surfaces

### Structured Error Kinds

The stdout NDJSON stream carries a short kind/type token on error frames. The
seeded `kind_buckets` classify from it. This is a diagnostic side-channel; the
message branch does the primary rendering work.

### Message Text

The primary rendering surface. Error frame prose formatted from OpenCode's error
types and the AI-SDK envelope. All message-branch seeds classify from this
surface — including OpenCode-specific model-selection strings
(`ProviderModelNotFoundError` → the `providermodelnotfound` needle).

### Promoted Stderr Logs (detection, not rendering)

OpenCode's stderr `service=llm` / stream-error lines carry the provider-limit
detail. Claudine's promoted-stderr classifier maps these to
`LogClassification::ProviderLimit(kind = UsageCap | RetriesExhausted | Overloaded
| RateLimited)`. This is the `signals/opencode.md` territory (multiple wire
formats across `v1.17.7`/`v1.17.8`); it fires `SignalKind` detection events and
is deliberately **not** duplicated as rendering needles here.

### Numeric Codes

No JSON-RPC wire codes on the stdout stream; HTTP 429/503 appear in the promoted
stderr logs and are consumed by the detection classifier. See the
`numeric-http-codes` gap.

## Rate Limit, Quota, and Billing

Seeded kind needles `rate`, `quota`, `billing` and message needles `rate limit`,
`quota`, `billing`, `api error`, `api timeout` classify to `api_remote` and are
preserved. The account usage-cap / rate-limit *detection* records live in
`signals/opencode.md` (`usage_capped`, `retries_exhausted`, `rate_limited`);
cited, not duplicated (D9).

## Authentication, Permission, and Configuration

Seeded kind needles `auth`, `config`, `permission`, `provider`, `model` and
message needles `api key`, `authentication`, `not authorized`,
`permission denied`, `model not found`, `invalid model`, `providermodelnotfound`
classify to `configuration` and are preserved. OpenCode's model-resolution
failures (`ProviderModelNotFoundError`, invalid model id) are a notable
configuration surface unique to its multi-provider routing.

## Interruption and Cancellation

Seeded needles `interrupt`, `cancel`, `abort` (kind) and `interrupt`, `cancel`,
`aborted` (message) classify to `interrupted` and are preserved.

## Upstream and Server (late ApiRemote)

The repeated `api_remote` kind bucket `api`, `upstream`, `server` is the
"late ApiRemote" second pass, checked *after* `interrupted` and after the broad
`provider`/`model` configuration needles, so it cannot shadow those. Preserved
unchanged.

## Capacity and Overload

No seeded needle covers the capacity/overload class in this rendering vocabulary.
OpenCode's overload surface (`LogClassification::ProviderLimit(kind =
Overloaded)`) is real but lives on the promoted-stderr detection channel owned by
`signals/opencode.md`, not the stdout substring cascade. The stdout NDJSON stream
carries no confirmed, source-pinned `overloaded`/`503` rendering string, so no
capacity substring needle is graduated. Recorded as the
`capacity-overload-phrasing` gap; the class is handled by detection today.

## Collisions and Precedence

- **`provider`** (seed, kind branch) — broad substring in the `configuration`
  bucket; matches OpenCode's provider-routing error prose. Ordered before the
  late `api_remote` pass. Sticky, untouched.
- **`model`** (seed, kind branch) — very broad; it would match any model-name
  mention. Scoped to `configuration` and preserved from Phase A. Flagged for
  Phase C awareness — this is OpenCode's broadest seed and its safety depends on
  the stdout error frame not carrying arbitrary model prose.
- **`api`** (seed, kind branch, late pass) — broad; ordered last. Flagged.
- **`providermodelnotfound`** (seed) — the collapsed
  `ProviderModelNotFoundError` type name; narrow and safe.
- **Bare HTTP numbers (429/503)** — consumed by the detection layer, not
  proposed as rendering needles.

## Quirks and Gaps

- **Overload lives in detection, not rendering** — owned by
  `signals/opencode.md`. (`gaps`: `capacity-overload-phrasing`.)
- **Numeric HTTP codes** — detection-channel concern, unsafe as rendering
  substrings. (`gaps`: `numeric-http-codes`.)
- **`model` is the broadest seed** — safe only while the stdout error frame does
  not carry general model prose; flagged for Phase C.
- **Shared parser, distinct vocabulary** — Kilo reuses this parser but selects
  its own table; see `kilo.md`.

## Sources

- [OpenCode CLI docs](https://opencode.ai/docs/cli/) — stdout NDJSON event
  stream, error frame shapes.
- `claudine/docs/research/signals/opencode.md` — the `usage_capped` /
  `retries_exhausted` / `provider_overloaded` / `rate_limited`
  `LogClassification::ProviderLimit` **detection** records for the promoted
  stderr channel (D9 cross-citation; detection, not rendering vocabulary).
- `claudine/docs/providers/facts/opencode.yaml` (`error_vocabulary:`) — the
  Phase-A seed transcribed verbatim from `lib/src/stream/providers/opencode.rs`.

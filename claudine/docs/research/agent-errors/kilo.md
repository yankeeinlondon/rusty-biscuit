---
$schema: ./_schema.yaml
created: 2026-07-12
last_updated: 2026-07-12
agent: claude-code
model: claude-opus-4-8
docs: https://kilocode.ai/docs/
# Kilo reuses OpenCode's wire PARSER but is a DISTINCT provider record with its
# own selectable vocabulary. Its Phase-A seed was transcribed as an ordered copy
# of OpenCode's table (both front the same OpenCode-compatible stream shape), so
# the seeded buckets below are intentionally identical to opencode.md — a
# JUSTIFIED similarity, not copy-paste research (see `## Shared Parser, Distinct
# Vocabulary`). Every needle is a preserved Phase-A seed (evidence: seed).
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
  - area: kilo-native-error-strings
    notes: >-
      Kilo's provider-specific failure payloads — `PROMOTION_MODEL_LIMIT_REACHED`
      (non-retryable promotion cap), `PAID_MODEL_AUTH_REQUIRED`, and gateway
      `insufficient balance` / HTTP 402 — are documented in `signals/kilo.md` as
      wire-level DETECTION records on the OpenCode-shaped SSE `session.error`
      payload. They are NOT yet reflected as distinct rendering needles here: the
      seed is an ordered copy of OpenCode's table, and this fleet run does not
      graduate a Kilo-only substring without a source-pinned rendering string.
      Recorded as a gap so Phase C can decide whether Kilo's promotion-limit /
      paid-model-auth phrasing should diverge from the shared OpenCode seed.
  - area: capacity-overload-phrasing
    notes: >-
      No seeded Kilo needle covers the generic capacity/overload motivating
      class in the rendering vocabulary. Kilo's closest surface is the
      non-retryable `PROMOTION_MODEL_LIMIT_REACHED` promotion cap (a
      usage-cap-family DETECTION record in `signals/kilo.md`), not a provider
      `overloaded`/`503` string. No capacity substring needle is graduated
      without a pinned citation; recorded as a gap for Phase C adjudication.
changes: []
requires_claudine_update: false
reason: >-
  All Phase-A seeds are preserved verbatim; no runtime vocabulary delta is
  proposed. Kilo's seed is intentionally identical to OpenCode's (shared wire
  parser, ordered-copy seed) — a justified similarity — and its native
  promotion-limit / paid-model-auth phrasings are recorded as gaps rather than
  graduated. Research does not change classification behavior.
---

# Error Vocabulary Research on Kilo

## Overview

Kilo (Kilo Code's coding CLI) speaks an OpenCode-compatible stream shape and
therefore reuses Claudine's OpenCode wire *parser* — but it is a **distinct
provider record** with its own selectable error vocabulary. In Phase A its
seeded table was transcribed as an ordered copy of OpenCode's (both front the
same wire shape), which is why the frontmatter buckets here are identical to
`opencode.md`. That identity is a *justified* similarity, not lazily copied
research: Kilo's genuinely distinct error surfaces are its provider-specific SSE
`session.error` payloads (promotion limits, paid-model auth, gateway funding
failures), which this document researches separately below.

## Shared Parser, Distinct Vocabulary

Kilo flows through `for_provider(Kilo)` into the OpenCode wire parser, which is
constructed with a fixed `Kilo` identity so it selects `vocabulary::
error_keywords(Kilo)` — Kilo's own table — never OpenCode's. Reusing a parser
does **not** imply reusing a vocabulary (Phase A proved this with an end-to-end
Kilo fixture whose winning classification differs from OpenCode). The tables
happen to be byte-identical today because Kilo was seeded from OpenCode's; a
future Kilo-only delta (see gaps) would diverge them without touching the shared
parser. This is exactly the case the cross-provider copy-paste smell check is
meant to surface, and the similarity is recorded here as justified.

## Error Surfaces

### Structured Error Kinds

The OpenCode-shaped stream carries a short kind token on error frames; the seeded
`kind_buckets` classify from it. Diagnostic side-channel; message branch does the
primary work.

### Message Text

The primary rendering surface. Error frame prose formatted from the
OpenCode-compatible envelope plus Kilo's gateway responses. All message-branch
seeds classify here.

### SSE `session.error` Payloads (detection, not rendering)

Kilo's richest, provider-specific error signal is the global SSE `/event`
`session.error` payload with a nested `error.data.responseBody`. It carries
`PROMOTION_MODEL_LIMIT_REACHED`, `PAID_MODEL_AUTH_REQUIRED`, and gateway
`insufficient balance` (HTTP 402). These are wire-level **detection** records
owned by `signals/kilo.md` (`usage_capped`, `no_funds`, `auth_invalid`); they
fire `SignalKind` events and are cited, not duplicated as rendering needles here
(D9).

### Numeric Codes

No JSON-RPC wire codes; Kilo's HTTP 402 (funding) surfaces in the SSE payload
consumed by the detection classifier.

## Rate Limit, Quota, and Billing

Seeded kind needles `rate`, `quota`, `billing` and message needles `rate limit`,
`quota`, `billing`, `api error`, `api timeout` classify to `api_remote` and are
preserved. Kilo's account-funding failure (gateway `insufficient balance` / 402)
is a `no_funds` *detection* record in `signals/kilo.md`, not a rendering needle
here.

## Authentication, Permission, and Configuration

Seeded kind needles `auth`, `config`, `permission`, `provider`, `model` and
message needles `api key`, `authentication`, `not authorized`,
`permission denied`, `model not found`, `invalid model`, `providermodelnotfound`
classify to `configuration` and are preserved. Kilo's paid-model auth gate
(`PAID_MODEL_AUTH_REQUIRED`) is an `auth_invalid` *detection* record in
`signals/kilo.md` — noted as a gap for possible rendering divergence.

## Interruption and Cancellation

Seeded needles `interrupt`, `cancel`, `abort` (kind) and `interrupt`, `cancel`,
`aborted` (message) classify to `interrupted` and are preserved.

## Upstream and Server (late ApiRemote)

The repeated `api_remote` kind bucket `api`, `upstream`, `server` is the
"late ApiRemote" second pass, checked after `interrupted` and the broad
`provider`/`model` configuration needles. Preserved unchanged.

## Capacity and Overload

No seeded Kilo needle covers the generic capacity/overload class. Kilo's closest
surface is the non-retryable `PROMOTION_MODEL_LIMIT_REACHED` promotion cap — a
usage-cap-family *detection* record in `signals/kilo.md`, not a provider
`overloaded`/`503` rendering string. No capacity substring needle is graduated
without a pinned citation; recorded as the `capacity-overload-phrasing` gap.

## Collisions and Precedence

- **Identical table to OpenCode** — justified (shared wire parser, ordered-copy
  seed); the copy-paste smell is expected and documented, not accidental.
- **`model` / `provider`** (seeds, kind branch) — the broadest configuration
  seeds, inherited from OpenCode; same caveat as `opencode.md`. Flagged for
  Phase C.
- **Kilo-native strings withheld** — `PROMOTION_MODEL_LIMIT_REACHED` /
  `PAID_MODEL_AUTH_REQUIRED` are detection records, not proposed as rendering
  needles here; recorded as the `kilo-native-error-strings` gap.

## Quirks and Gaps

- **Justified table identity with OpenCode** — shared parser, ordered-copy seed;
  a future Kilo-only delta would diverge them.
- **Kilo-native error strings live in detection** — owned by `signals/kilo.md`.
  (`gaps`: `kilo-native-error-strings`.)
- **Capacity class uncovered in rendering vocabulary** — closest surface is a
  detection promotion-cap. (`gaps`: `capacity-overload-phrasing`.)

## Sources

- [Kilo Code docs](https://kilocode.ai/docs/) — CLI overview and provider
  gateway behavior.
- `claudine/docs/research/signals/kilo.md` — the `usage_capped`
  (`PROMOTION_MODEL_LIMIT_REACHED`), `no_funds` (gateway 402 `insufficient
  balance`), and `auth_invalid` (`PAID_MODEL_AUTH_REQUIRED`) **detection**
  records for Kilo's SSE `session.error` payload (D9 cross-citation; detection,
  not rendering vocabulary).
- `claudine/docs/providers/facts/kilo.yaml` (`error_vocabulary:`) — the Phase-A
  seed, an ordered copy of OpenCode's table, transcribed verbatim; Kilo flows
  through the shared OpenCode wire parser with a fixed `Kilo` identity.

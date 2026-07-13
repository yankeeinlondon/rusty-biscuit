---
$schema: ./_schema.yaml
created: 2026-07-12
last_updated: 2026-07-12
agent: claude-code
model: claude-opus-4-8
docs: https://developers.openai.com/codex/noninteractive
# Ordered buckets checked against the structured error-kind discriminator.
# Sequence order IS the cascade order (first substring hit wins). The repeated
# api_remote bucket is the "late ApiRemote" second pass after Interrupted.
# Every needle is a preserved Phase-A seed (evidence: seed) — the pilot proposes
# no kind-branch delta; all additions land in the message branch below.
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
      - text: denied
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
# are preserved; the pilot appends one documented capacity needle (`overloaded`)
# to the end of the first api_remote bucket so it is checked after the existing
# seeds without reordering them (R3 append-default within the same branch).
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
      - text: overloaded
        evidence: documented
        source: https://platform.openai.com/docs/guides/error-codes
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
gaps:
  - area: capacity-exact-phrasing
    notes: >-
      The motivating incident string, Codex's documented "Selected model is at
      capacity", could not be commit-pinned to a `openai/codex` source
      permalink or code path in this pilot session. The documented OpenAI 503
      `overloaded` message is proposed as a needle (it is the API-level surface
      Codex passes through), but the exact CLI-rendered capacity/at-capacity
      phrasing needs a version-pinned source citation from a live research run
      before an `at capacity` / `capacity` substring needle is graduated.
  - area: numeric-http-codes
    notes: >-
      Bare HTTP status substrings (429, 503, 401, 403) are documented Codex
      failure modes but are deliberately NOT proposed as needles: as raw
      substrings they collide with token counts, timestamps, and IDs in
      non-error prose. They belong in a numeric `code_buckets`-style surface or
      behind an exact-match matcher, not the current case-insensitive substring
      cascade. Recorded as a precedence gap for Phase C adjudication.
  - area: structured-error-kind-discriminator
    notes: >-
      Codex `exec --json` surfaces errors as free-form `error.message` /
      `error_message` prose on `turn.failed` and `error` frames rather than a
      stable machine error-kind enum. The kind_buckets branch therefore has a
      thinner contract than the message branch; classification leans on message
      text. No stable discriminator taxonomy could be confirmed for the kind
      branch beyond the seeded needles.
changes: []
requires_claudine_update: true
reason: >-
  Pilot proposes one message-branch addition — `overloaded` (documented, OpenAI
  503 overloaded surface) — to the first api_remote bucket, closing the
  capacity/overload motivating-class gap that no seeded Codex needle covered.
  This is a Phase C delta flagged for the C1 delta report and adjudication; it
  does not change runtime behavior by itself. All Phase-A seeds are preserved
  unchanged. The exact Codex-CLI "at capacity" phrasing and bare HTTP-code
  matching remain open gaps (see `gaps`) and are intentionally not graduated.
---

# Error Vocabulary Research on Codex CLI

> **Pilot document (spec `2026-07-11-provider-errors-as-data`, increment B2).**
> This is the `agent-errors` topic's Codex pilot: one provider researched first
> so the schema, fleet prompt, and deterministic gate can be hardened before the
> fleet runs the remaining roster. Pilot telemetry, the broad-substring review,
> and the signals-topic coordination are recorded in
> [`_pilot-codex.md`](./_pilot-codex.md) and [`_signals-overlap.md`](./_signals-overlap.md).

## Overview

Codex (OpenAI's coding CLI, `openai/codex`, Rust, open source) surfaces errors
in non-interactive mode through its `codex exec --json` JSONL event stream.
Errors arrive as free-form message prose carried on two event shapes: a
`turn.failed` event with a nested `error.message`, and a top-level `error`
event with an `error_message` field. There is no first-class, stable
machine-readable *error-kind enum* on the exec stream — the failure taxonomy is
expressed in human-facing message text formatted from Rust error types
(`UsageLimitReachedError` and friends). Claudine's Codex parser therefore
classifies primarily from message text; the `kind_buckets` discriminator is a
thinner, best-effort surface (see the gap entry).

Because Codex is a thin client over the OpenAI API, a large share of its error
prose is *pass-through* of API-level errors (rate limits, 429/503 overload,
auth failures) rather than CLI-native strings. Documented, stable surfaces are
strongest for the usage-limit / rate-limit family; capacity/overload phrasing
and the exact numeric-code surface are less firmly pinned (recorded as gaps).

## Error Surfaces

### Structured Error Kinds

The `exec --json` stream does not emit a stable error-kind discriminator that
Claudine can key on exactly; the seeded `kind_buckets` classify from whatever
short kind/type token the stream carries. This is a diagnostic side-channel, not
a first-class contract — see `## Quirks and Gaps`. All kind-branch needles are
preserved Phase-A seeds; the pilot proposes no kind-branch change.

### Message Text

The primary error surface. `turn.failed` (`error.message`) and `error`
(`error_message`) frames carry human-facing prose formatted from Rust error
types. This is where the classifier does its real work, and where the pilot's
one proposed addition (`overloaded`) lands.

### Numeric Codes

Codex does not expose Kimi-style JSON-RPC numeric wire codes on its exec stream;
HTTP status numbers (429, 503, 401, 403) appear inside message prose. They are
**not** modeled as `code_buckets` and are not proposed as substring needles
(collision risk — see `## Collisions and Precedence` and the `numeric-http-codes`
gap).

## Rate Limit, Quota, and Billing

Codex passes through OpenAI rate-limit errors. The seeded needles `rate`,
`quota`, `billing` (kind branch) and `rate limit`, `quota`, `billing`,
`api error` (message branch) all classify to `api_remote` and are preserved.

The `signals/` topic separately researched Codex's usage-cap surface
(`you've hit your usage limit`, from the current `UsageLimitReachedError`
formatting) as a `usage_capped` **detection** record — see `signals/codex.md`.
Per the D9 boundary, that string is a detection concern (it fires a
`SignalKind`), not a rendering-vocabulary needle here; this document cites it
rather than duplicating it. The seeded `rate limit` needle already renders the
same family for the `SemanticErrorKind` summary layer.

## Authentication, Permission, and Configuration

Seeded needles `auth`, `config`, `permission`, `denied` (kind) and `api key`,
`authentication`, `not authorized`, `permission denied`, `config` (message)
classify to `configuration` and are preserved. Codex auth failures ("Not logged
in", "Missing OpenAI API key", `401 Unauthorized`) surface as message prose and
are matched by `api key` / `authentication` / `not authorized`. No new
auth-family needle is proposed (the seeds cover the documented surfaces).

## Interruption and Cancellation

Seeded needles `interrupt`, `cancel`, `abort` (kind) and `interrupt`, `cancel`,
`aborted` (message) classify to `interrupted` and are preserved. Codex renders
turn interruption / cancellation on user or signal abort. Note the seed asymmetry
preserved from Phase A: the kind branch matches `abort` while the message branch
requires `aborted` — this is an existing precedence quirk, not a delta.

## Capacity and Overload

The motivating incident: Codex's documented **"Selected model is at capacity"**
matched no seeded Codex needle. Closing this class with provenance is the reason
the topic exists.

Two distinct surfaces:

1. **OpenAI 503 `overloaded`** — the OpenAI API documents the 503 overloaded
   response ("the engine is currently overloaded"). Codex passes API errors
   through into its `error`/`turn.failed` message prose, so the `overloaded`
   substring is a documented, safe, capacity-family classifier. **Proposed** as
   an `api_remote` message needle (`evidence: documented`), closing the
   motivating class.
2. **CLI-native "Selected model is at capacity"** — documented behaviorally
   (the signal-assurance incident) but **not commit-pinned** to a source
   permalink in this pilot. Recorded as the `capacity-exact-phrasing` gap. An
   `at capacity` / `capacity` substring needle is deliberately withheld until a
   live research run can cite the exact rendered string against a Codex version
   tag — proposing it now would be guessing.

## Collisions and Precedence

- **`overloaded`** (proposed) — narrow, unambiguous; it does not appear in Codex
  success/progress prose (`turn.completed`, `agent_message`, tool item frames).
  Appended after the existing seeds in the first `api_remote` message bucket, so
  precedence is unchanged: the seeded `rate limit` / `quota` / `billing` /
  `api error` needles still match first, and `overloaded` only catches the
  otherwise-unclassified overload prose. Safe substring.
- **`api`** (seed, kind branch) — very broad; it matches `api error`, `api key`,
  and any prose containing "api". This is an *existing* seed and its precedence
  is fixed by Phase A; the pilot does not touch it. Flagged here only so the
  Phase C review is aware the broadest seed already lives in the kind branch.
- **Bare HTTP numbers (429/503/401/403)** — rejected as substring needles: they
  collide with token counts and IDs in non-error frames. See the
  `numeric-http-codes` gap.

## Quirks and Gaps

- **No stable error-kind enum on the exec stream** — classification leans on
  message text; the `kind_buckets` branch is best-effort. (`gaps`:
  `structured-error-kind-discriminator`.)
- **Exact capacity phrasing unpinned** — "Selected model is at capacity" is
  documented behaviorally but not source-pinned here. (`gaps`:
  `capacity-exact-phrasing`.)
- **Numeric HTTP codes are unsafe substrings** — need an exact-match surface,
  not the substring cascade. (`gaps`: `numeric-http-codes`.)
- **`abort` vs `aborted` seed asymmetry** — preserved from Phase A, not a delta.

## Sources

- [Codex non-interactive docs](https://developers.openai.com/codex/noninteractive)
  — `codex exec --json` event stream, error frames.
- [OpenAI API error codes](https://platform.openai.com/docs/guides/error-codes)
  — documented 429 rate-limit and 503 `overloaded` responses (source for the
  proposed `overloaded` needle).
- `openai/codex` (Rust CLI, open source) — error-type formatting
  (`UsageLimitReachedError` et al.). A version-pinned permalink to the capacity
  string is the outstanding evidence gap.
- `claudine/docs/research/signals/codex.md` — the `usage_capped` /
  `rate_limited` **detection** records for the same Codex surface (D9
  cross-citation; detection, not rendering vocabulary).
- `claudine/docs/providers/facts/codex.yaml` (`error_vocabulary:`) — the Phase-A
  seed transcribed verbatim from `lib/src/stream/providers/codex.rs`.

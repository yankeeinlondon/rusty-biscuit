---
$schema: ./_schema.yaml
created: 2026-07-12
last_updated: 2026-07-12
agent: claude-code
model: claude-opus-4-8
docs: https://antigravity.google/docs/cli-statusline
# Antigravity is a message-only classifier: its buffered-JSON output surfaces
# failures as free-form message prose, with no structured error-kind
# discriminator, so there is no `kind_buckets`. Sequence order IS the cascade
# order (first substring hit wins). Every needle is a preserved Phase-A seed
# (evidence: seed). Note the configuration bucket is checked FIRST — Antigravity
# is an OAuth-first CLI whose dominant failure surface is sign-in.
msg_buckets:
  - kind: configuration
    needles:
      - text: sign in
        evidence: seed
      - text: sign-in
        evidence: seed
      - text: not logged in
        evidence: seed
      - text: authentication failed
        evidence: seed
      - text: authentication
        evidence: seed
      - text: unauthorized
        evidence: seed
      - text: "401"
        evidence: seed
      - text: "403"
        evidence: seed
  - kind: api_remote
    needles:
      - text: rate limit
        evidence: seed
      - text: quota
        evidence: seed
      - text: exhausted
        evidence: seed
      - text: out of credits
        evidence: seed
      - text: overloaded
        evidence: seed
      - text: "503"
        evidence: seed
      - text: resource_exhausted
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
      Antigravity (`agy`) has no published implementation source at tag 1.1.0
      (the public `google-antigravity/antigravity-cli` repo carries only README,
      changelog, and examples) and no official machine-readable stream/event
      contract. Its buffered-JSON output surfaces failures as free-form message
      prose with no typed error-kind enum, so classification is message-only and
      there is no `kind_buckets`. The seeded auth/sign-in phrasings were captured
      empirically from installed `agy` 1.1.0 (see `signals/antigravity.md`);
      firmer source citations are unavailable until Google publishes source.
changes: []
requires_claudine_update: false
reason: >-
  All Phase-A seeds are preserved verbatim and the seeded `overloaded` / `503` /
  `resource_exhausted` / `exhausted` needles already cover the capacity/overload
  motivating class, so this fleet run proposes no runtime vocabulary delta.
  Research does not change classification behavior.
---

# Error Vocabulary Research on Antigravity

## Overview

Antigravity (Google's coding CLI, binary `agy`) is the roster's tenth provider
and its first buffered-JSON stream: it emits a single JSON document at
`finish()` rather than line-delimited events. The public
`google-antigravity/antigravity-cli` repository at tag 1.1.0 contains only
README, changelog, and consumer examples — no implementation source — so error
surfaces were characterized empirically from the installed `agy` 1.1.0 binary
and its `--log-file` output rather than from source code. Antigravity is an
OAuth-first CLI (no API key), so its dominant failure surface is authentication
/ sign-in, which is why the seeded `configuration` bucket is checked before
`api_remote`.

## Error Surfaces

### Message Text

The primary error surface. Buffered-JSON message prose plus stdout-tail failure
lines carry human-facing failure text. Every seeded needle classifies from this
surface.

### Structured Error Kinds

None confirmed. No typed error-kind enum is exposed and no source is published,
so the classifier is message-only (recorded as a gap). The `signals/` topic
keys its Antigravity `auth_invalid` **detection** records off exit code + stdout
tail ("Please sign in to view available models", "authentication failed or timed
out"); those stay in `signals/antigravity.md`.

### App-Log Lines

`agy --log-file` produces glog-style lines ("You are not logged into
Antigravity", "Language server version:"). `signals/antigravity.md` records
these as documented but *uncompiled* detection gaps (Claudine has no runtime
app-log ingestion path). They are not rendering-vocabulary needles here.

### Numeric Codes

No JSON-RPC wire codes. `401`, `403`, `503` below are substrings of HTTP-status
phrasing rendered in message prose, not discrete wire codes, and live in the
message branch.

## Authentication, Permission, and Configuration

Checked first. Seeded needles `sign in`, `sign-in`, `not logged in`,
`authentication failed`, `authentication`, `unauthorized`, `401`, `403` classify
to `configuration` and are preserved. These are Antigravity's OAuth-preflight
failure phrasings, captured empirically from the installed binary. Ordering this
bucket ahead of `api_remote` encodes the real precedence: an unauthenticated
`agy` fails at sign-in before any API call.

## Rate Limit, Quota, and Capacity

Seeded needles `rate limit`, `quota`, `exhausted`, `out of credits`,
`overloaded`, `503`, `resource_exhausted` classify to `api_remote` and are
preserved. Because Antigravity fronts Google model infrastructure, its
capacity/overload family is unusually well-seeded — `overloaded`, `503`,
`resource_exhausted` (the Google-style `RESOURCE_EXHAUSTED` status phrasing), and
`exhausted` all appear here.

## Interruption and Cancellation

Seeded needles `abort`, `cancel`, `interrupt` classify to `interrupted` and are
preserved.

## Capacity and Overload

The capacity/overload motivating class is already covered by preserved seeds:
`overloaded`, `503`, `resource_exhausted`, and `exhausted` all classify to
`api_remote`. Antigravity's Google-backed infrastructure surfaces
`RESOURCE_EXHAUSTED` and 503 overload prose, so these are the strongest-seeded
capacity vocabulary of the roster. No new capacity needle is required and no gap
is recorded for this class.

## Collisions and Precedence

- **`authentication`** (seed) — broad substring, but scoped to the
  `configuration` bucket which is intentionally checked first for this
  OAuth-first CLI. Sticky seed, untouched.
- **`exhausted`** (seed) — matches "exhausted"/"resource_exhausted"; acceptable
  in the `api_remote` capacity family. Broader than `resource_exhausted` but
  both are seeded and preserved.
- **`401` / `403` / `503`** (seeds) — digit substrings; safe here because
  Antigravity only emits them inside error/auth-failure prose. Sticky, untouched.
- **`sign in` vs `sign-in`** (seeds) — both spellings preserved to catch either
  rendering.

## Quirks and Gaps

- **No published source** — surfaces are empirical (installed `agy` 1.1.0), not
  source-pinned. (`gaps`: `structured-error-kind-discriminator`.)
- **Buffered-JSON stream** — errors arrive in one document at `finish()`, unlike
  the line-delimited providers.
- **App-log detection is uncompiled** — owned as gaps by `signals/antigravity.md`,
  not rendering vocabulary here.

## Sources

- [Antigravity CLI statusline docs](https://antigravity.google/docs/cli-statusline)
  — the closest official documentation surface (client-rendered; no verbatim
  payload contract).
- `claudine/docs/research/signals/antigravity.md` — the `auth_invalid`
  **detection** records and the uncompiled app-log gaps for the same surface
  (D9 cross-citation; detection, not rendering vocabulary), reconfirmed against
  installed `agy` 1.1.0.
- `claudine/docs/providers/facts/antigravity.yaml` (`error_vocabulary:`) — the
  Phase-A seed transcribed verbatim from
  `lib/src/stream/providers/antigravity.rs`.

---
$schema: ./_schema.yaml
created: 2026-07-12
last_updated: 2026-07-12
agent: claude-code
model: claude-opus-4-8
docs: https://goose-docs.ai/docs/guides/running-tasks/
# Goose is RESEARCH-ONLY for this topic: it has no structured stream PARSER in
# `lib/src/stream/providers/` and no Phase-A `error_vocabulary` facts seed, so it
# is NOT a candidate for executable vocabulary and there are no seeds to
# preserve. The runtime buckets are intentionally EMPTY (Goose is explicitly
# empty at runtime, spec acceptance criteria). Goose's typed error catalog is
# documented in the body as a research-only reference; if Goose ever gains a
# stream parser, that catalog is the starting point for a seed.
kind_buckets: []
msg_buckets: []
gaps:
  - area: no-stream-parser
    notes: >-
      Goose has no structured stream parser in `lib/src/stream/providers/` and
      no `error_vocabulary` facts seed. It is retained as research-only and is
      explicitly empty at runtime; no executable vocabulary is proposed. Goose
      also has no line-delimited error-kind enum on a claudine-consumed stream —
      it catches most provider errors inside its agent loop and re-emits them as
      assistant `message` text ("Ran into this error: <TypedError>: …"), plus a
      dedicated `creditsExhausted` system notification. Those are wire-level
      DETECTION records owned by `signals/goose.md`, not rendering vocabulary.
  - area: capacity-overload-phrasing
    notes: >-
      Goose exposes no overload-specific error variant. It normalizes provider
      5xx-style failures to the typed `ProviderError::ServerError` ("Ran into
      this error: Server error:") — a coarse transient-provider signal — with no
      distinct `overloaded`/`at capacity`/`503` string. The
      `provider_overloaded` DETECTION record in `signals/goose.md` keys off that
      same `Server error:` prefix. There is therefore no capacity/overload
      rendering needle to propose; recorded as a gap.
changes: []
requires_claudine_update: false
reason: >-
  Goose is research-only: no stream parser, no facts seed, no runtime vocabulary.
  The buckets are intentionally empty and no delta is proposed. Research does not
  change classification behavior.
---

# Error Vocabulary Research on Goose

## Overview

Goose (Block's open-source coding agent, `block/goose`, Rust) is a **research-only**
provider for the `agent-errors` topic: it has no structured stream parser in
`lib/src/stream/providers/` and no Phase-A `error_vocabulary` facts seed, so it
contributes no executable `SemanticErrorKind` vocabulary and is explicitly empty
at runtime. This document exists to characterize Goose's error surfaces for a
future maintainer — if Goose ever gains a claudine stream parser, the typed error
catalog below is the starting point for a seed.

Goose's defining trait is that it **catches most provider errors inside its agent
loop** and re-emits them as assistant `message` text with a fixed wrapper prefix
("Ran into this error: <TypedError>: …"), rather than surfacing a raw provider
error frame. It also emits a dedicated `creditsExhausted` system notification for
funding exhaustion. Both are wire-level **detection** surfaces owned by
`signals/goose.md`; the rendering-vocabulary layer this topic owns has nothing to
classify because there is no parser consuming Goose's stream.

## Error Surfaces

### Message Text (agent-loop wrapped, detection-owned)

Goose's `type: message` frames carry assistant text; on a caught provider error
the text is Goose's fixed wrapper around a typed `ProviderError` display string.
Because there is no claudine parser, these strings are not classified into a
rendering vocabulary here — `signals/goose.md` keys detection records off the
wrapper prefixes (`Authentication error:`, `Rate limit exceeded:`,
`Server error:`).

### System Notifications (detection-owned)

`message.content[0].type: systemNotification` with
`notificationType: creditsExhausted` is Goose's explicit funding-exhaustion
signal, produced from `ProviderError::CreditsExhausted`. Stronger than matching
billing/quota prose — and, again, a `signals/goose.md` **detection** record
(`no_funds`), not a rendering needle.

### Structured Error Kinds / Numeric Codes

None consumed by claudine. Goose has no line-delimited error-kind enum on a
claudine-parsed stream and no JSON-RPC numeric wire codes.

## Typed Error Catalog (research-only reference)

Goose's `ProviderError` enum (as documented by `signals/goose.md`, source_code
confidence) renders these display strings. They are listed here as the reference
a future Goose parser would seed from — **not** as executable needles today:

- `Authentication error:` → would map to `configuration`
- `Rate limit exceeded:` → `api_remote`
- `Server error:` → `api_remote` (Goose's only overload-adjacent surface; coarse)
- `Network error:`, `Request failed`, `Endpoint not found (404)` → `api_remote`
- `Context length exceeded` → `configuration` / `agent_native` (context limit)
- `Credits exhausted` → funding; already a `creditsExhausted` detection record
- `Execution error`, `Usage data error`, `Unsupported operation`,
  `Provider refused request` → `agent_native` / `api_remote` depending on cause

This catalog is intentionally left out of the frontmatter buckets: with no parser
to consume it, encoding it as vocabulary would imply a runtime behavior that does
not exist.

## Rate Limit, Quota, and Billing

Goose surfaces these as `ProviderError::RateLimitExceeded` ("Rate limit
exceeded:") and the dedicated `creditsExhausted` notification. Both are
`signals/goose.md` detection records (`rate_limited`, `no_funds`); no rendering
needle is proposed (no parser).

## Authentication, Permission, and Configuration

`ProviderError::Authentication` ("Authentication error:") — a `signals/goose.md`
`auth_invalid` detection record keyed off Goose's fixed catch-all wrapper prefix.
No rendering needle proposed.

## Capacity and Overload

Goose has no overload-specific variant. Provider 5xx failures normalize to
`ProviderError::ServerError` ("Server error:") — a coarse transient signal, not a
distinct capacity string. The `provider_overloaded` detection record in
`signals/goose.md` keys off that same prefix. No capacity/overload rendering
needle exists to propose; recorded as the `capacity-overload-phrasing` gap.

## Collisions and Precedence

Not applicable to a rendering vocabulary — Goose has no runtime buckets. The
precedence that matters for Goose lives in `signals/goose.md`, where the
`creditsExhausted` notification and the typed wrapper prefixes are ordered so the
specific funding/auth records win over the coarse `Server error:` catch-all.

## Quirks and Gaps

- **Research-only, empty at runtime** — no parser, no facts seed, no vocabulary.
  (`gaps`: `no-stream-parser`.)
- **Errors are agent-loop-wrapped** — Goose re-emits typed provider errors as
  assistant text, so its error surface is detection-shaped, not a raw error
  frame.
- **No overload variant** — 5xx collapses to `ServerError`. (`gaps`:
  `capacity-overload-phrasing`.)

## Sources

- [Goose running-tasks docs](https://goose-docs.ai/docs/guides/running-tasks/)
  — non-interactive/headless run behavior.
- `claudine/docs/research/signals/goose.md` — the `no_funds` (`creditsExhausted`),
  `auth_invalid` (`Authentication error:`), `rate_limited` (`Rate limit
  exceeded:`), and `provider_overloaded` (`Server error:`) **detection** records
  and the `ProviderError` typed catalog (source_code confidence) that this
  research-only reference draws from (D9 cross-citation; detection, not rendering
  vocabulary).
- No `docs/research/agent-errors/_seeds/goose.yaml` exists — Goose is parser-less
  and has no Phase-A seed baseline.

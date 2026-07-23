---
$schema: ./_schema.yaml
created: 2026-07-14
last_updated: 2026-07-14
agent: codex
model: default
docs: https://code.claude.com/docs/en/errors
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
      - text: oauth_org_not_allowed
        evidence: documented
        source: https://code.claude.com/docs/en/agent-sdk/typescript
      - text: invalid_request
        evidence: documented
        source: https://code.claude.com/docs/en/agent-sdk/typescript
      - text: model_not_found
        evidence: documented
        source: https://code.claude.com/docs/en/agent-sdk/typescript
  - kind: interrupted
    needles:
      - text: interrupt
        evidence: seed
      - text: cancel
        evidence: seed
      - text: abort
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
      - text: credit
        evidence: seed
      - text: api error
        evidence: seed
      - text: overloaded
        evidence: seed
      - text: server is temporarily limiting requests
        evidence: documented
        source: https://code.claude.com/docs/en/errors
      - text: request rejected (429)
        evidence: documented
        source: https://code.claude.com/docs/en/errors
      - text: is temporarily unavailable, so auto mode cannot determine
        evidence: documented
        source: https://code.claude.com/docs/en/errors
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
      - text: not logged in
        evidence: documented
        source: https://code.claude.com/docs/en/errors
      - text: invalid api key
        evidence: documented
        source: https://code.claude.com/docs/en/errors
      - text: could not resolve authentication method
        evidence: documented
        source: https://code.claude.com/docs/en/errors
      - text: oauth token revoked
        evidence: documented
        source: https://code.claude.com/docs/en/errors
      - text: oauth token has expired
        evidence: documented
        source: https://code.claude.com/docs/en/errors
      - text: login expired
        evidence: documented
        source: https://code.claude.com/docs/en/errors
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
      Claude Code documents HTTP 401, 403, 429, 500, 504, and 529 failures,
      but its structured result exposes API status as a diagnostic field rather
      than a Kimi-style numeric error-code classifier input. Bare numbers are
      unsafe substring needles because they collide with IDs, counts, and
      ordinary assistant prose. The exact 429 message is proposed instead.
  - area: interruption-payloads
    notes: >-
      The Agent SDK documents AbortController cancellation and result terminal
      reasons such as aborted_streaming and aborted_tools, but the current
      classifier receives only an assistant error discriminator and message.
      No official structured assistant-error discriminator for cancellation or
      interruption was confirmed.
  - area: top-level-error-event-contract
    notes: >-
      Claudine accepts top-level `error` and `assistant.error` envelopes seen in
      captures, but the current public Agent SDK message union documents the
      assistant `error` discriminator rather than a stable standalone error
      event contract. Those envelope variants remain an observed compatibility
      surface, not a public first-class contract.
changes: []
requires_claudine_update: true
reason: >-
  Research preserves all Phase-A rows and proposes exact, documented additions:
  three SDK assistant-error discriminators that the seeds do not match, three
  remote/capacity messages, and six authentication messages. These additions
  close documented false-negative paths without broad `model`, `auth`, `rate`,
  or bare-status-code message needles.
---

# Claude Code Error-Classification Vocabulary

## Overview

Claude Code is a closed-source CLI; Anthropic publishes its non-interactive
contract through the official Claude Code and Agent SDK documentation rather
than a source repository. In print mode, `--output-format stream-json` emits a
JSONL stream compatible with the Agent SDK message union. The strongest
classification surface is the optional `SDKAssistantMessage.error`
discriminator, backed by human-readable assistant or result text. A terminal
result also carries `is_error`, `result`, and optional `api_error_status`, but
the numeric status is diagnostic metadata and is not passed to Claudine's
numeric-code branch.

The public contract is unusually useful for a closed CLI: Anthropic documents
the complete assistant error enum and exact user-facing runtime messages. Some
standalone `error` and `assistant.error` envelopes are present in Claudine's
captured compatibility corpus, but the current public SDK union centers the
documented `assistant` envelope and its optional error field. This vocabulary
therefore prefers official enum values and exact documented phrases over broad
guesses.

## Error Surfaces

### Structured Error Kinds

`SDKAssistantMessage.error` is a first-class enum with
`authentication_failed`, `oauth_org_not_allowed`, `billing_error`,
`rate_limit`, `overloaded`, `invalid_request`, `model_not_found`,
`server_error`, `max_output_tokens`, and `unknown`. Anthropic explicitly
distinguishes `overloaded` as API 529 capacity from `rate_limit` as API 429
quota pressure. Claudine checks this discriminator before message text, so a
kind match wins even if the accompanying prose contains a later bucket's
needle.

The existing first `api_remote` bucket covers all documented remote enum values
except neutral fallthrough values. The exact configuration values
`oauth_org_not_allowed`, `invalid_request`, and `model_not_found` contain none
of the seeded configuration substrings, so they are appended to that bucket.
`max_output_tokens` and `unknown` correctly retain the classifier's
`agent_native` fallthrough and need no needles.

### Message Text

Assistant content and `SDKResultMessage.result` carry human-readable failure
text. This is a documented but less stable surface than the enum: runtime copy
can vary by deployment, gateway, and Claude Code version. The error reference
nevertheless publishes exact messages, making narrow full-clause substrings
appropriate where the seeds miss a family.

### Result Metadata and Numeric Status

`SDKResultMessage` exposes `is_error`, the error text, and optional
`api_error_status`, described as the HTTP status that terminated the
conversation. This status is a diagnostic side-channel, not a numeric wire-code
bucket in Claudine: Claude's parser calls the shared classifier without a code.
Anthropic's API reference maps 401 to authentication, 402 to billing, 403 to
permission, 429 to rate limit, 500 to API error, 504 to timeout, and 529 to
overload. Bare numbers are not proposed as message substrings.

### Rate-Limit Events

The SDK message union also includes `SDKRateLimitEvent`, and observed streams
carry account-window status, reset times, and throttling state. Those records
fire normalized usage-cap and rate-limit signals; detection and extraction are
owned by [`signals/claude.md`](../signals/claude.md). This document only
classifies an error's kind and summary after an error surface has been selected.

## Rate Limit, Quota, and Billing

The structured enum values `billing_error` and `rate_limit` are already caught
by the seeded `billing` and `rate_limit` kind needles. Seeded message needles
cover `rate limit`, `quota`, `billing`, `credit`, and `api error`, including the
documented `Credit balance is too low` message.

Two documented throttle messages contain none of those seeded phrases:
`Server is temporarily limiting requests (not your usage limit)` and `Request
rejected (429)`. Their narrow leading clauses are appended to the first
`api_remote` message bucket. The former is temporary server throttling and the
latter is an account or deployment rate limit; both render as `api_remote`.
Whether either payload should fire `rate_limited` is a detection question
covered by [`signals/claude.md`](../signals/claude.md), not a frontmatter record
here.

## Authentication, Permission, and Configuration

The seeded kind needles `auth`, `permission`, and `config` classify most
configuration failures. However, `oauth_org_not_allowed`, `invalid_request`,
and `model_not_found` match none of them. The SDK defines all three as assistant
error discriminators; exact additions classify them as `configuration` without
introducing broad needles such as `oauth`, `request`, or `model`.

The message branch retains `api key`, `authentication`, `not authorized`, and
`permission denied`. Exact documented additions cover `Not logged in`, `Invalid
API key`, `Could not resolve authentication method`, `OAuth token revoked`,
`OAuth token has expired`, and `Login expired`. These all represent credentials
or account configuration and therefore classify as `configuration`. They are
appended after every seeded item, preserving Phase-A order.

## Interruption, Cancellation, and Abort

The preserved kind needles `interrupt`, `cancel`, and `abort`, and message
needles `interrupt`, `cancel`, and `aborted`, classify to `interrupted`. The SDK
also supports caller cancellation through `AbortController`, while result
metadata may report `aborted_streaming` or `aborted_tools`. Those result fields
are not inputs to the current keyword classifier, and no official
`SDKAssistantMessageError` cancellation value was found. The seed asymmetry
between kind `abort` and message `aborted` remains unchanged.

## Upstream, Server, and Provider Errors

Seeded kind needles `api_error`, `upstream`, and `server` classify remote
infrastructure failures as `api_remote`; `server_error` therefore matches the
seeded `server`. The documented messages `API Error: 500 Internal server error`,
mid-response server errors, and `Agent terminated early due to an API error`
are covered by existing `api error` or the structured `server_error` kind.
Request timeouts are described as potentially caused by server load or network
conditions, so the broad word `timeout` is not added to either family.

## Capacity and Overload

Claude's capacity vocabulary is confirmed. The first-class `overloaded`
assistant discriminator means the API returned HTTP 529 because the server is
at capacity, while the error reference publishes `API Error: Repeated 529
Overloaded errors. The API is at capacity`. Seeded kind `overload` and message
`overloaded` already classify both forms as `api_remote`.

Claude Code also documents an auto-mode failure of the form `<model> is
temporarily unavailable, so auto mode cannot determine the safety of <tool>`
when its classifier model is overloaded. That message contains neither seeded
capacity needle, so the narrow invariant clause `is temporarily unavailable,
so auto mode cannot determine` is appended as `api_remote`. The broader
`temporarily unavailable`, `at capacity`, `capacity`, and `model` fragments are
withheld to avoid ordinary prose collisions.

## Collisions and Precedence

The kind branch always runs before the message branch. Consequently,
`billing_error` wins `api_remote` even if its message says authentication is
needed, and `invalid_request` now wins `configuration` before a message such as
`API error` could classify remotely. That precedence follows the provider's
explicit discriminator.

| Candidate | Decision | Collision or precedence result |
|---|---|---|
| `overload` / `overloaded` | Preserve seeds | Narrow capacity terms; first `api_remote` bucket wins. |
| `rate` | Reject | Appears inside ordinary words and metadata; exact `rate_limit` and `rate limit` are available. |
| `model` | Reject | Common in success output, initialization metadata, and ordinary assistant prose. |
| `auth` as a message needle | Reject | Broad fragment; exact documented credential messages are safer. The seeded kind needle remains scoped to enum-like input. |
| `401`, `403`, `429`, `500`, `503`, `504`, `529` | Reject bare numbers | Can occur in IDs, token counts, timestamps, code, and successful prose. `503` is not Claude's canonical overload status; 529 is. |
| `at capacity` | Reject | The documented overload message already contains seeded `overloaded`; the generic phrase can occur in planning prose. |
| `is temporarily unavailable, so auto mode cannot determine` | Add | Narrow documented clause; catches the auto-mode capacity case without matching normal model discussion. |
| `invalid_request` | Add after seeded configuration items | Exact enum value; kind branch wins before any message bucket. |

Representative non-error frames include model names in `system/init`, token and
cost numbers in `result`, and arbitrary assistant prose. None of the proposed
message additions is expected in those records unless the provider is actually
describing the documented failure. Seed positions and bucket sequence are left
unchanged; additions occur only after the final seed in their existing bucket.

## Quirks and Gaps

- Claude Code is closed source. Official SDK types and error documentation are
  the strongest available contract; there is no provider source permalink to
  inspect.
- The public SDK documents `assistant.error`, while standalone top-level
  `error` and `assistant.error` envelopes remain compatibility observations.
- HTTP status is available on result metadata but is not passed into the
  classifier's numeric-code input. Bare-number substring matching is unsafe.
- No official structured assistant-error discriminator for interruption was
  confirmed. Abort terminal reasons live on a different result surface.
- Rate-limit-event detection, reset extraction, and usage-cap signaling belong
  to [`signals/claude.md`](../signals/claude.md).

## Changelog

This is the initial research document for the 2026-07-14 fleet run; `changes`
is empty by contract.

## Sources

- [Claude Code error reference](https://code.claude.com/docs/en/errors) — exact
  runtime messages, retry behavior, authentication failures, 429 throttling,
  and 529 capacity behavior.
- [Claude Agent SDK TypeScript reference](https://code.claude.com/docs/en/agent-sdk/typescript)
  — `SDKAssistantMessageError`, `SDKResultMessage`, `api_error_status`, terminal
  reasons, and the structured message union.
- [Claude Platform API errors](https://platform.claude.com/docs/en/api/errors) —
  HTTP status-to-error-type mapping and mid-stream error caveat.
- [`signals/claude.md`](../signals/claude.md) — detection records for rate-limit
  and usage-cap wire payloads that overlap this rendering surface.
- [`_seeds/claude.yaml`](./_seeds/claude.yaml) — immutable Phase-A vocabulary
  preserved in frontmatter.

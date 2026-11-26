---
$schema: ./_schema.yaml
created: 2026-07-14
last_updated: 2026-07-14
agent: codex
model: default
docs: https://kilo.ai/docs/code-with-ai/platforms/cli
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
      - text: server error
        evidence: source_code
        source: https://github.com/Kilo-Org/kilocode/blob/v7.2.48/packages/opencode/src/provider/error.ts#L123-L166
      - text: response decompression failed
        evidence: source_code
        source: https://github.com/Kilo-Org/kilocode/blob/v7.2.48/packages/opencode/src/session/message-v2.ts#L1256-L1270
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
      - text: please reauthenticate with the copilot provider
        evidence: source_code
        source: https://github.com/Kilo-Org/kilocode/blob/v7.2.48/packages/opencode/src/provider/error.ts#L48-L54
      - text: 'unauthorized:'
        evidence: source_code
        source: https://github.com/Kilo-Org/kilocode/blob/v7.2.48/packages/opencode/src/provider/error.ts#L78-L86
      - text: 'forbidden:'
        evidence: source_code
        source: https://github.com/Kilo-Org/kilocode/blob/v7.2.48/packages/opencode/src/provider/error.ts#L78-L86
  - kind: interrupted
    needles:
      - text: interrupt
        evidence: seed
      - text: cancel
        evidence: seed
      - text: aborted
        evidence: seed
gaps:
  - area: provider-specific-capacity-copy
    notes: >-
      Kilo's retry status can carry provider-supplied overload wording, and the
      signals research has an overloaded retry-status record. No exact tagged
      source establishes that overloaded, at capacity, or an equivalent phrase
      reaches the terminal error record's free-form message, so none is guessed.
  - area: resource-exhausted-spelling
    notes: >-
      No resource_exhausted rendering was confirmed in Kilo v7.2.48's
      structured run error path.
  - area: numeric-code-contract
    notes: >-
      APIError may carry an HTTP statusCode and responseBody, but Claudine's
      Kilo/OpenCode classifier has no numeric-code input. Bare 401, 402, 403,
      429, and 503 message needles are deliberately rejected.
  - area: kilo-response-body-codes
    notes: >-
      PAID_MODEL_AUTH_REQUIRED and PROMOTION_MODEL_LIMIT_REACHED are stable
      Kilo response-body codes used by signal detection, but the run command
      classifies and displays error.data.message rather than promoting those
      responseBody tokens into the rendering message.
changes: []
requires_claudine_update: true
reason: >-
  The proposal preserves every seeded bucket and item position, then appends
  source-attested Kilo-normalized server, transport, reauthentication,
  authentication, and authorization message phrases absent from the seeded
  runtime table.
---

# Kilo Code CLI Error-Classification Vocabulary

## Overview

Kilo Code CLI is open source and is derived from OpenCode. Its non-interactive
`kilo run --format json` mode writes newline-delimited JSON records. At tag
`v7.2.48`, terminal session failures appear as `error` records containing a
structured named-error object. The object has a `name` discriminator and
usually `data.message`; API failures can additionally retain an HTTP
`statusCode`, response headers, a serialized response body, and metadata.

The JSON output option and event envelope are source-defined contracts, but
Kilo's official CLI documentation does not publish an error taxonomy. Some
messages are normalized by Kilo, while ordinary provider API messages pass
through upstream text and are less stable. Claudine reuses its OpenCode stream
parser for Kilo but selects Kilo's independently ordered vocabulary.

## Error Surfaces

### Structured Error Kinds

The JSON error object's `name` is the structured discriminator. Kilo v7.2.48
defines `ProviderAuthError`, `MessageAbortedError`, `MessageOutputLengthError`,
`APIError`, `ContextOverflowError`, `StructuredOutputError`, and `UnknownError`
for assistant failures. These names are a first-class schema rather than a
diagnostic side-channel.

The seeded cascade already maps `ProviderAuthError` to `configuration`,
`MessageAbortedError` to `interrupted`, and `APIError` to the late
`api_remote` bucket. Output-length, context-overflow, structured-output, and
unknown errors deliberately fall through to `agent_native`; none of their
names alone proves a configuration or remote-service failure.

### Message Text

In JSON mode, `kilo run` emits the complete named-error object. In formatted
mode, it displays `error.data.message` when present and otherwise displays the
error name. Kilo normalizes a few messages itself: a Copilot reauthentication
instruction, HTML gateway/proxy authorization prefixes, a stream server-error
fallback, and response-decompression failure. Those locally controlled strings
are safe substring needles. Arbitrary provider response prose is not promoted
without separate evidence.

### Numeric Codes

`APIError.data.statusCode` is an optional non-negative HTTP status and
`responseBody` can contain symbolic provider or Kilo codes. This is structured
diagnostic metadata, but Claudine's Kilo parser does not supply a numeric code
to `classify_error_by_keywords`; consequently this document has no
`code_buckets`.

Records that should fire usage-cap, no-funds, rate-limit, overload, or
authentication signals are detection policy. They are documented in
[`signals/kilo.md`](../signals/kilo.md), not duplicated in this rendering
vocabulary.

## Rate Limit, Quota, and Billing

The first kind bucket retains `rate`, `quota`, and `billing`; the first message
bucket retains `rate limit`, `quota`, `billing`, `api error`, and `api timeout`.
Every item remains in its seeded position and maps to `api_remote`.

Kilo's stream-error normalizer recognizes `insufficient_quota` and renders the
exact message `Quota exceeded. Check your plan and billing details.` The
existing `quota` needle wins before `billing`, so no additional row is needed.
Kilo-specific promotion limits and insufficient-balance responses belong to
signal detection because they are carried in `responseBody`, not necessarily
the message supplied to this classifier.

## Authentication, Permission, and Configuration

The structured configuration bucket checks `auth`, `config`, `permission`,
`provider`, then `model`. `ProviderAuthError` therefore classifies as
`configuration` before the later broad `api_remote` bucket. The message bucket
retains all seven sticky configuration phrases in their seeded order.

Three source-controlled Kilo messages are appended. GitHub Copilot HTTP 403
errors receive `Please reauthenticate with the copilot provider...`; its full
prefix avoids a broad `copilot` match. HTML 401 and 403 gateway responses begin
with `Unauthorized:` and `Forbidden:` respectively. These prefixes cover
truncated records as well as the complete explanations, while avoiding unsafe
bare status numbers.

Kilo also declares the response-body code `PAID_MODEL_AUTH_REQUIRED`, but the
run command emits the assistant error object and chooses `data.message` for
formatted output. The response-body code is therefore retained in the Kilo
signals research and not guessed as a message needle here.

## Interruption, Cancellation, and Abort

The third kind bucket retains `interrupt`, `cancel`, and `abort`; the third
message bucket retains `interrupt`, `cancel`, and `aborted`. Kilo converts a DOM
`AbortError` to `MessageAbortedError`, so the structured kind classifies it as
`interrupted` before message matching. No additional interruption phrase is
needed.

Manual termination and the decision that a streamed record constitutes a
terminal interruption remain signal/wrapper concerns. The related detection
record is in [`signals/kilo.md`](../signals/kilo.md).

## Upstream, Server, and Provider Errors

The late seeded kind bucket checks `api`, `upstream`, then `server`. Its
position after configuration and interruption is deliberate: narrower
`ProviderAuthError` and `MessageAbortedError` discriminators win before broad
remote terms. A generic `APIError` reaches `api_remote` through this late
bucket unless an earlier rate, quota, or billing term already matched.

Two exact Kilo-controlled messages are appended to the first `api_remote`
message bucket. `Server error` is the fallback when the stream error code is
`server_error` and the provider supplied no message. `Response decompression
failed` represents a failed compressed response and is marked retryable.
Full phrases avoid unsafe message needles such as bare `server`, `response`,
`failed`, or `error`.

## Capacity and Overload

Kilo exposes a generic retry-status message surface, and the signals research
records an observed status whose message contains `overloaded`. That record is
appropriate for `provider_overloaded` detection, but it is not evidence that
the same literal reaches `error.data.message`, which is the rendering input
owned by this document.

The tagged v7.2.48 stream normalizer has a `server_error` fallback of `Server
error.`, which the proposed `server error` needle safely covers. It does not
define an exact `overloaded`, `at capacity`, `resource_exhausted`, 429, or 503
rendering in the inspected run error path. Those provider-specific capacity
phrases remain gaps rather than being inferred from Codex or OpenCode. See
[`signals/kilo.md`](../signals/kilo.md) for the retry-status overload record.

## Collisions and Precedence

| Candidate | Decision | Winning behavior or collision |
| --- | --- | --- |
| `server error` | Append to the first `api_remote` message bucket | Exact fallback; avoids matching ordinary prose containing only `server` or `error`. |
| `response decompression failed` | Append to the first `api_remote` message bucket | Exact transport failure; avoids broad `response` and `failed`. |
| `please reauthenticate with the copilot provider` | Append to `configuration` | Exact Kilo prefix; does not misclassify ordinary Copilot/model discussion. |
| `unauthorized:` | Append to `configuration` | Colon-bound gateway prefix; safer than `unauthorized`, `auth`, or bare `401`. |
| `forbidden:` | Append to `configuration` | Colon-bound gateway prefix; safer than `forbidden`, `permission`, or bare `403`. |
| `overloaded` / `at capacity` | Reject pending rendering evidence | Confirmed only on the separate retry-status detection surface. |
| `resource_exhausted` | Reject pending evidence | Not found in the tagged run error path. |
| `401`, `402`, `403`, `429`, `503` | Reject | Bare numbers can match identifiers, counts, timestamps, tool output, or successful assistant prose. |
| `rate`, `model`, `auth` | Preserve only in seeded kind buckets | These broad terms inspect a structured discriminator, not success prose; moving them into message matching would create collisions. |

Kind matching runs before message matching. Thus `ProviderAuthError` wins
`configuration` even if its message also mentions an API, and
`MessageAbortedError` wins `interrupted` before message inspection. Within the
message branch, the early `api_remote` bucket retains precedence over
configuration and interruption exactly as seeded; none of the appended exact
phrases overlaps an earlier seeded phrase.

## Quirks and Gaps

- Kilo's CLI and IDE extensions have distinct historical lineages. This
  document covers the current OpenCode-derived `kilo run --format json`
  surface, not legacy Roo-derived extension messages.
- `APIError` can retain useful `statusCode` and `responseBody` fields, but the
  shared Kilo/OpenCode classifier receives only kind and message. Treating bare
  numeric text as a substitute would be unsafe.
- `PAID_MODEL_AUTH_REQUIRED` and `PROMOTION_MODEL_LIMIT_REACHED` are stable
  response-body codes, not proven rendering messages. Their detection use is
  documented in [`signals/kilo.md`](../signals/kilo.md).
- No official Kilo error taxonomy was found. The official CLI page documents
  operation, while the discriminators and exact normalized strings come from
  the tagged source.
- No provider-specific capacity sentence equivalent to `Selected model is at
  capacity` was confirmed on the rendering surface. `overloaded`, `at
  capacity`, and `resource_exhausted` remain explicit gaps.

## Sources

- [Kilo CLI documentation](https://kilo.ai/docs/code-with-ai/platforms/cli)
- [Kilo v7.2.48 JSON output option and raw event emission](https://github.com/Kilo-Org/kilocode/blob/v7.2.48/packages/opencode/src/cli/cmd/run.ts#L241-L255)
- [Kilo v7.2.48 terminal error-record handling](https://github.com/Kilo-Org/kilocode/blob/v7.2.48/packages/opencode/src/cli/cmd/run.ts#L542-L551)
- [Kilo v7.2.48 assistant error discriminators](https://github.com/Kilo-Org/kilocode/blob/v7.2.48/packages/opencode/src/session/message-v2.ts#L42-L63)
- [Kilo v7.2.48 assistant error union](https://github.com/Kilo-Org/kilocode/blob/v7.2.48/packages/opencode/src/session/message-v2.ts#L463-L486)
- [Kilo v7.2.48 abort, authentication, decompression, and API error normalization](https://github.com/Kilo-Org/kilocode/blob/v7.2.48/packages/opencode/src/session/message-v2.ts#L1221-L1299)
- [Kilo v7.2.48 gateway/proxy messages and API-call normalization](https://github.com/Kilo-Org/kilocode/blob/v7.2.48/packages/opencode/src/provider/error.ts#L48-L91)
- [Kilo v7.2.48 stream-error normalization](https://github.com/Kilo-Org/kilocode/blob/v7.2.48/packages/opencode/src/provider/error.ts#L110-L166)
- [Kilo v7.2.48 Kilo-specific response-body codes](https://github.com/Kilo-Org/kilocode/blob/v7.2.48/packages/opencode/src/kilocode/kilo-errors.ts#L3-L40)
- [Kilo signal-detection research](../signals/kilo.md)

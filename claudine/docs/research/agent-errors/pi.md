---
$schema: ./_schema.yaml
created: 2026-07-14
last_updated: 2026-07-14
agent: codex
model: default
docs: https://github.com/earendil-works/pi/blob/v0.80.7/packages/coding-agent/docs/json.md
msg_buckets:
  - kind: api_remote
    needles:
      - text: rate limit
        evidence: seed
      - text: quota
        evidence: seed
      - text: billing
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L7-L24
      - text: out of credits
        evidence: seed
      - text: overloaded
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L26-L40
      - text: '503'
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L26-L40
      - text: api error
        evidence: seed
      - text: api timeout
        evidence: seed
      - text: insufficient_quota
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L7-L24
      - text: out of budget
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L7-L24
      - text: quota exceeded
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L7-L24
      - text: too many requests
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L26-L40
      - text: service unavailable
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L26-L40
      - text: server error
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L26-L40
      - text: internal error
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L26-L40
      - text: provider returned error
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L41-L44
      - text: network error
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L45-L60
      - text: connection refused
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L45-L60
      - text: fetch failed
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L45-L60
      - text: reset before headers
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L45-L60
      - text: socket hang up
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L45-L60
      - text: websocket closed
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L62-L65
      - text: websocket error
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L62-L65
      - text: stream ended before message_stop
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L66-L71
      - text: http2 request did not get a response
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L66-L71
      - text: resourceexhausted
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L83-L85
  - kind: configuration
    needles:
      - text: api key
        evidence: seed
      - text: authentication
        evidence: seed
      - text: no api key
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/api/anthropic-messages.ts#L283-L290
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
        evidence: source_code
        source: https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/types.ts#L456-L476
      - text: cancel
        evidence: seed
      - text: interrupt
        evidence: seed
gaps:
  - area: provider-authentication-and-permission-copy
    notes: >-
      Pi passes through many provider SDK messages. Beyond its stable No API key
      setup error, no provider-independent authorization or permission phrase was
      confirmed, so the remaining sticky configuration needles are not expanded.
  - area: at-capacity-phrase
    notes: >-
      Pi v0.80.7 explicitly recognizes overloaded, HTTP 503, service-unavailable,
      and ResourceExhausted forms, but no exact at capacity or Selected model is
      at capacity sentence was found. That provider-authored copy is not guessed.
  - area: numeric-code-contract
    notes: >-
      Provider error formatting can embed HTTP status numbers in errorMessage,
      but Pi's JSON protocol has no distinct numeric error-code field and the
      Claudine Pi classifier is message-only. No code_buckets are proposed.
  - area: regex-to-substring-loss
    notes: >-
      Pi's retry classifier uses regex forms such as rate.?limit and
      service.?unavailable. Claudine accepts literal substrings only, so this
      vocabulary records safe space-separated forms and cannot reproduce every
      punctuation variant without adding risky broad needles.
changes: []
requires_claudine_update: true
reason: >-
  The proposal preserves the seeded bucket and item ordering, upgrades four
  seeded rows with tagged-source evidence, and appends Pi-maintained quota,
  capacity, upstream, transport, and resource-exhaustion phrases absent from
  the seeded runtime table.
---

# Pi Error-Classification Vocabulary

## Overview

Pi is open source and provides a documented non-interactive JSON event-stream
mode through `pi --mode json`. At tag `v0.80.7`, stdout is newline-delimited
JSON: a session header is followed by agent, turn, message, tool, compaction,
and automatic-retry records. Request failures are represented inside assistant
messages by `stopReason: "error" | "aborted"` and a free-form `errorMessage`;
there is no provider-wide structured error-kind discriminator or numeric wire
code.

The JSON event shapes and assistant-message types are first-class source
contracts. Exact upstream message copy is less stable because Pi supports many
providers and preserves SDK or gateway diagnostics. Pi's own retry classifier
is the strongest common vocabulary: it classifies the same `errorMessage`
surface with maintained patterns for limits, overload, server, transport, and
resource-exhaustion failures.

## Error Surfaces

### Assistant Message Errors

The streaming API requires request, model, and runtime failures to end in an
`AssistantMessage` carrying `stopReason` of `error` or `aborted` and an optional
`errorMessage`. JSON mode emits those messages in `message_update`,
`message_end`, and terminal event records. The nested assistant stream also has
an `error` event whose `reason` is `error` or `aborted` and whose `error` value
is the final assistant message. `stopReason` is a lifecycle discriminator, not
a detailed error-kind taxonomy; Claudine therefore classifies the free-form
message only.

### Automatic-Retry Records

JSON mode exposes `auto_retry_start.errorMessage` and
`auto_retry_end.finalError`. Pi decides whether a failed assistant message is
retryable by testing `errorMessage` against ordered non-retryable limit and
retryable provider patterns. These records are diagnostic side-channels over
the same message vocabulary, not independent error kinds.

Whether an automatic-retry record should fire a rate-limit, usage-cap, or
retry signal is detection policy and belongs to [`signals/pi.md`](../signals/pi.md).
This document uses the maintained phrases only to render an already-selected
error's semantic kind and summary.

### Tool and Compaction Errors

Tool completion has an `isError` boolean and arbitrary tool result content.
Compaction completion can carry `errorMessage`, `aborted`, and `willRetry`.
Neither is automatically equivalent to a terminal provider request failure;
they are first-class operation records whose selection as signals remains
outside this rendering vocabulary.

### Numeric Codes

Pi's shared error formatter can include an extracted HTTP status in the free
text, including provider-prefixed forms such as `OpenAI API error (503): ...`.
The JSON protocol does not expose a separate numeric error-code field, so the
Pi cascade has no `code_buckets`. The seeded `503` remains a message substring,
with its collision risk documented below.

## Rate Limit, Quota, and Billing

The first `api_remote` bucket retains the seeded `rate limit`, `quota`,
`billing`, and `out of credits` positions. Pi's non-retryable provider-limit
pattern explicitly recognizes `insufficient_quota`, `out of budget`, `quota
exceeded`, and `billing`; the first three are appended as source-attested
rendering phrases. The existing `quota` seed shadows `insufficient_quota` and
`quota exceeded`, but retaining the exact forms records provenance and protects
classification if the broader seed is later adjudicated.

Pi also recognizes `too many requests` as a transient remote error. It is safer
than adding bare `rate`, and it classifies as `api_remote`. Account-limit
records such as monthly usage limits may represent a usage-cap signal rather
than transient throttling; that detection distinction is documented in the Pi
signals research.

## Authentication, Permission, and Configuration

The configuration bucket preserves `api key`, `authentication`, `no api key`,
`not authorized`, `no models available`, `model not found`, and `invalid model`
in seeded order. Pi's provider adapters emit the exact setup form `No API key
for provider: ...`, upgrading the existing `no api key` row to source-code
evidence. The earlier `api key` needle shadows it, but both sticky positions
remain intact.

No common Pi-authored 401/403 sentence was established across providers.
Provider SDK bodies can still match the sticky authentication and permission
phrases, but broad additions such as `auth`, `401`, or `403` would be unsafe in
a substring cascade.

## Interruption, Cancellation, and Abort

The final bucket preserves `abort`, `cancel`, then `interrupt`. Pi's stream
contract names `aborted` as a `StopReason` and as the error-event reason, so
`abort` is upgraded to tagged source evidence. Because `abort` is a substring
of `aborted`, it covers both the discriminator-derived fallback and messages
such as `Request was aborted`.

This bucket intentionally follows remote and configuration. A provider message
that says a request was aborted because of a remote `503` remains
`api_remote`: the earlier, more actionable remote condition wins. Plain user
cancellation without an earlier remote phrase resolves to `interrupted`.

## Upstream, Server, and Provider Errors

Pi's maintained transient-error pattern recognizes server, provider, network,
connection, fetch, socket, WebSocket, and premature-stream failures. Safe exact
literal phrases are appended to the first bucket: `server error`, `internal
error`, `provider returned error`, `network error`, `connection refused`,
`fetch failed`, `reset before headers`, `socket hang up`, `websocket closed`,
`websocket error`, `stream ended before message_stop`, and `http2 request did
not get a response`.

These phrases classify as `api_remote` because they describe an upstream API or
transport failure. Broader regex fragments used internally by Pi, including
bare `timeout`, `terminated`, `connection`, and `ended without`, are omitted:
as literal substring needles they would capture local-agent and ordinary prose
too readily. The seeded `api error` and `api timeout` remain as narrower forms.

## Capacity and Overload

Pi v0.80.7 explicitly treats `overloaded`, HTTP `503`, service-unavailable
forms, and gRPC `ResourceExhausted` as transient provider failures. The seeded
`overloaded` and `503` rows therefore receive source-code evidence, while
`service unavailable` and lowercase `resourceexhausted` are appended. All map
to `api_remote` and precede configuration and interruption.

No exact Pi-controlled `at capacity` or `Selected model is at capacity` message
was found. Because Pi passes through provider text, such a sentence might still
occur, but it is recorded as a gap rather than promoted without provenance.
The existing `overloaded` and new source-attested forms close only the capacity
spellings Pi itself documents in code.

## Collisions and Precedence

| Needle or family | Collision assessment | Winning behavior |
|---|---|---|
| `quota` | Broad enough to match both `insufficient_quota` and ordinary quota diagnostics, but unlikely in success output selected as an error | First `api_remote` bucket; shadows the appended exact quota forms |
| `billing` | Can appear in explanatory account prose, but Pi explicitly treats it as a provider-limit failure | First `api_remote` bucket |
| `503` | Bare numeric substring can occur in request IDs, token counts, or dates | Seeded behavior is retained; when present in selected error text it wins before every later bucket |
| `api key` | Shadows the more exact `no api key` seed | Configuration bucket; no semantic disagreement |
| `abort` | Matches both `abort` and `aborted` | Interrupted only if no earlier remote or configuration needle matched |
| `resourceexhausted` | Exact gRPC class spelling after ASCII lowercasing; does not match `resource_exhausted` | First `api_remote` bucket; underscore spelling remains unconfirmed |
| bare `429`, `500`, `502`, `504`, `524` | Pi recognizes them for retry, but substring matching would collide with unrelated numbers | Rejected from the proposed additions; detection belongs in signals |
| bare `model`, `auth`, `rate`, `server` | Common in successful or contextual prose | Rejected in favor of seeded or source-attested multiword phrases |

The cascade runs only on text already selected as an error, which reduces but
does not eliminate collisions. Bucket order remains the decisive contract:
remote phrases win over configuration phrases, and both win over interruption.

## Quirks and Gaps

Pi's internal patterns are regular expressions, while Claudine performs plain
case-insensitive substring matching. A Pi pattern such as `service.?unavailable`
accepts punctuation variants that the proposed literal `service unavailable`
does not. Expanding that to bare `service` or `unavailable` would create more
false positives than it resolves.

The exact `ResourceExhausted` spelling is attested, but `resource_exhausted` is
not present in Pi's maintained retry vocabulary. Likewise, no exact `at
capacity` sentence was confirmed. Both omissions are explicit rather than
cross-provider guesses.

Pi can preserve arbitrary provider response bodies, so no finite shared list
captures every authentication, billing, or capacity sentence. Numeric HTTP
statuses remain embedded diagnostic text rather than a typed numeric surface.

## Sources

- [Pi JSON event-stream documentation at v0.80.7](https://github.com/earendil-works/pi/blob/v0.80.7/packages/coding-agent/docs/json.md)
- [Assistant-message and stream error contracts at v0.80.7](https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/types.ts#L301-L313)
- [Assistant message fields and error-event union at v0.80.7](https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/types.ts#L380-L400)
- [Pi provider-limit and transient-error vocabulary at v0.80.7](https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/retry.ts#L7-L100)
- [Pi provider error normalization and formatting at v0.80.7](https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/utils/error-body.ts#L18-L112)
- [Anthropic adapter's missing-key error at v0.80.7](https://github.com/earendil-works/pi/blob/v0.80.7/packages/ai/src/api/anthropic-messages.ts#L283-L290)
- [Pi signals research](../signals/pi.md)

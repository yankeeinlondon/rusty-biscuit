---
$schema: ./_schema.yaml
created: 2026-07-14
last_updated: 2026-07-14
agent: codex
model: default
docs: https://opencode.ai/docs/cli/
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
        source: https://github.com/anomalyco/opencode/blob/v1.17.7/packages/opencode/src/provider/error.ts#L102-L147
      - text: connection reset by server
        evidence: source_code
        source: https://github.com/anomalyco/opencode/blob/v1.17.7/packages/opencode/src/session/message-v2.ts#L614-L648
      - text: provider response headers timed out
        evidence: source_code
        source: https://github.com/anomalyco/opencode/blob/v1.17.7/packages/opencode/src/provider/error.ts#L7-L13
      - text: response decompression failed
        evidence: source_code
        source: https://github.com/anomalyco/opencode/blob/v1.17.7/packages/opencode/src/session/message-v2.ts#L649-L663
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
      - text: 'unauthorized:'
        evidence: source_code
        source: https://github.com/anomalyco/opencode/blob/v1.17.7/packages/opencode/src/provider/error.ts#L57-L65
      - text: 'forbidden:'
        evidence: source_code
        source: https://github.com/anomalyco/opencode/blob/v1.17.7/packages/opencode/src/provider/error.ts#L57-L65
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
      OpenCode v1.17.7 recognizes the provider code server_is_overloaded but
      preserves a provider-supplied message when present. No single exact
      provider-authored capacity sentence such as at capacity was confirmed;
      the proposed server error needle covers only OpenCode's own fallback.
  - area: resource-exhausted-spelling
    notes: >-
      No resource_exhausted spelling was found in the v1.17.7 structured run
      error path, so it is not guessed from another provider's protocol.
  - area: numeric-code-contract
    notes: >-
      APIError can carry an HTTP statusCode and responseBody codes, but the
      shared Claudine OpenCode classifier has no numeric-code input. Bare 401,
      403, 429, and 503 message substrings are deliberately rejected.
  - area: billing-and-rate-limit-native-copy
    notes: >-
      OpenCode normalizes insufficient_quota, retry metadata, and arbitrary
      provider response text, but no provider-independent exact billing or
      rate-limit message beyond the sticky seeds was established.
changes: []
requires_claudine_update: true
reason: >-
  The proposal preserves every seeded bucket and item position, then appends
  source-attested OpenCode-normalized remote, authentication, and authorization
  message phrases that are absent from the seeded runtime table.
---

# OpenCode CLI Error-Classification Vocabulary

## Overview

OpenCode CLI is open source. Its documented non-interactive command,
`opencode run`, accepts `--format json` for raw newline-delimited JSON events.
At tag `v1.17.7`, the CLI projects a `session.error` event to an `error` record
whose `error` value is a structured named-error object. That object carries a
`name` discriminator and usually a free-form `data.message`; API failures can
also carry `statusCode`, response headers, and a serialized response body.

The event envelope and named-error schemas are first-class source contracts,
but the official CLI page documents only that JSON events exist, not their
error taxonomy. Provider message copy remains partly diagnostic and can pass
through arbitrary upstream text. OpenCode also normalizes several failures to
stable local messages, which are the strongest safe additions to Claudine's
substring vocabulary. There is no OpenCode numeric-code bucket in the shared
classifier.

## Error Surfaces

### Structured Error Kinds

The JSON error object's `name` is the structured discriminator. OpenCode's
shared session schema defines `ProviderAuthError`, `MessageAbortedError`,
`MessageOutputLengthError`, `APIError`, `ContextOverflowError`, and content or
structured-output errors. Provider selection separately defines
`ProviderModelNotFoundError` and `ProviderInitError`.

The seeded kind cascade already covers the important families. `APIError`
matches the late fourth bucket's broad `api` needle; `ProviderAuthError`,
`ProviderModelNotFoundError`, and `ProviderInitError` match the earlier
configuration bucket; `MessageAbortedError` matches interruption. Output
length, context overflow, content filtering, and unknown errors intentionally
fall through to `agent_native` because they are not necessarily remote service
failures.

### Message Text

For non-interactive JSON, OpenCode emits the entire named-error object. In
formatted mode it instead displays `data.message` when available and otherwise
the error name. Claudine resolves the structured kind and free-form message
separately, so the message vocabulary matters when a generic `APIError` or an
older/partial envelope lacks a useful kind.

OpenCode preserves arbitrary provider messages for ordinary API-call errors,
but it supplies stable text for stream error codes, connection resets,
decompression failures, header timeouts, and HTML gateway failures. Those
locally controlled strings are suitable source-attested needles; arbitrary
provider prose is not promoted without independent evidence.

### Numeric Codes

`APIError.data.statusCode` is an optional non-negative HTTP status, while
`responseBody` can retain provider codes such as `insufficient_quota`,
`server_is_overloaded`, and `server_error`. OpenCode's `run --format json`
therefore carries numeric and symbolic diagnostic metadata, but Claudine's
OpenCode parser does not pass a numeric code into the shared classifier. This
document consequently has no `code_buckets`.

Status/code records that should fire usage-cap, rate-limit, overload, or
authentication signals are detection policy and belong to
[`signals/opencode.md`](../signals/opencode.md), not this rendering vocabulary.

## Rate Limit, Quota, and Billing

The first seeded kind bucket preserves `rate`, `quota`, and `billing`; the
first message bucket preserves `rate limit`, `quota`, `billing`, `api error`,
and `api timeout`. All classify as `api_remote`, and every seeded position is
unchanged.

OpenCode explicitly maps the stream code `insufficient_quota` to `Quota
exceeded. Check your plan and billing details.` The existing `quota` message
needle wins before `billing`, so no new row is required. Retry handling also
recognizes free-tier and account usage-limit records, but deciding when those
records fire `usage_capped`, `rate_limited`, or `retries_exhausted` is covered
by the OpenCode signals research rather than duplicated here.

## Authentication, Permission, and Configuration

The structured configuration bucket checks `auth`, `config`, `permission`,
`provider`, then `model`. It therefore classifies `ProviderAuthError`,
`ProviderInitError`, and `ProviderModelNotFoundError` without additions. The
message bucket retains all seven sticky configuration phrases in their seeded
order.

Two exact source-controlled gateway prefixes are appended: `unauthorized:` and
`forbidden:`. OpenCode emits the former for an HTML 401 response and explains
that authentication may be missing or expired. It emits the latter for an HTML
403 response and explains that the account may lack permission. The existing
`authentication` seed would happen to match the full 401 explanation, but the
prefix remains useful for truncated copy; the 403 explanation does not contain
the seeded phrase `permission denied`.

## Interruption, Cancellation, and Abort

The third kind bucket retains `interrupt`, `cancel`, and `abort`; the third
message bucket retains `interrupt`, `cancel`, and `aborted`. OpenCode converts a
DOM `AbortError` to the discriminator `MessageAbortedError`, so the kind branch
classifies it as `interrupted` before message inspection. Tool-level text such
as `Tool execution aborted` is not necessarily a terminal session error;
selecting the event that constitutes termination remains detection policy.

No additional interruption phrase is proposed. Process exit and Ctrl+C remain
wrapper/signal concerns unless OpenCode emits a terminal error record selected
by the stream parser.

## Upstream, Server, and Provider Errors

The late seeded kind bucket checks `api`, `upstream`, and `server`. Its position
after configuration and interruption is deliberate: `ProviderAuthError` and
`MessageAbortedError` resolve to their narrower families before a broad remote
term could win. `APIError` resolves to `api_remote` through the earlier first
kind bucket's `rate`/`quota`/`billing` only when those appear, otherwise through
this late `api` needle.

Four locally normalized messages are appended to the first `api_remote`
message bucket: `server error`, `connection reset by server`, `provider response
headers timed out`, and `response decompression failed`. The strings represent
remote server failure, transport reset, response-header timeout, and corrupt
provider response compression respectively. Their full phrases avoid unsafe
message needles such as bare `server`, `timeout`, `connection`, or `failed`.

## Capacity and Overload

OpenCode v1.17.7 recognizes stream error codes `server_is_overloaded` and
`server_error` as retryable `APIError`. When the provider supplies message text,
OpenCode preserves it. When it does not, both codes normalize to the exact
fallback `Server error.` The appended `server error` needle therefore closes
the OpenCode-controlled capacity fallback while also correctly covering the
broader server-error family.

No exact provider-authored phrase equivalent to `Selected model is at
capacity` was confirmed for OpenCode. In particular, source recognition of
`server_is_overloaded` does not prove that the literal token `overloaded`
survives into `data.message`. That unconfirmed surface is recorded as a gap
rather than guessed. The promoted-stderr detection layer separately recognizes
overload vocabulary and distinguishes overloaded 429 responses from generic
rate limits; see [`signals/opencode.md`](../signals/opencode.md).

Bare `429` and `503` are not proposed. Although the optional `statusCode` can
carry them, the message matcher is substring-based and cannot distinguish an
HTTP status from an identifier, count, timestamp, tool output, or ordinary
assistant prose. `resource_exhausted` was not found in the tagged error path.

## Collisions and Precedence

| Candidate | Decision | Winning behavior or collision |
| --- | --- | --- |
| `server error` | Append to first `api_remote` message bucket | Exact OpenCode fallback; earlier seeded quota/billing/API phrases retain priority. |
| `connection reset by server` | Append to first `api_remote` message bucket | Full normalized transport message avoids broad `connection` and `server`. |
| `provider response headers timed out` | Append to first `api_remote` message bucket | Exact timeout wording avoids matching ordinary timing prose. |
| `response decompression failed` | Append to first `api_remote` message bucket | Exact response failure avoids broad `response` or `failed`. |
| `unauthorized:` / `forbidden:` | Append to configuration message bucket | Exact gateway prefixes; the first `api_remote` bucket still wins if earlier remote needles occur in the same message. |
| `rate` | Preserve only in structured kinds | Sticky seed; unsafe in messages because success prose can discuss rates. |
| `auth` | Preserve only in structured kinds | Sticky seed; the configuration kind bucket wins before late `api`/`server`. |
| `model` | Preserve only in structured kinds | Sticky seed; unsafe in messages because every successful run can mention its model. |
| `overloaded` / `at capacity` | Reject pending exact OpenCode output | Recognition below the message projection is not proof that either literal reaches `data.message`. |
| `401`, `403`, `429`, `503` | Reject | Bare numeric substrings collide with counts, IDs, timestamps, tool output, and assistant text. |

Representative successful JSON records include `text`, `reasoning`,
`tool_use`, `step_start`, and `step_finish`. Their payloads can contain arbitrary
assistant or tool text, including models, servers, rates, and numbers. The
proposed additions are full OpenCode-controlled failure phrases; normal prose
such as `Server capacity planning completed`, `Model 429 processed`, or
`Response decompression benchmark passed` matches none of them.

## Quirks and Gaps

- The official CLI documentation calls JSON output “raw JSON events” but does
  not document the error envelope or discriminator union; tagged source is the
  contract used here.
- `APIError` is deliberately broad. Its message can represent quota, overload,
  server failure, transport failure, or arbitrary upstream copy, so message
  precedence still matters even when the kind already resolves to
  `api_remote`.
- OpenCode can carry `statusCode`, response headers, and `responseBody`, but
  Claudine's OpenCode vocabulary has no numeric-code branch. Detection of
  status-bearing operational signals belongs to the signals topic.
- `ContextOverflowError`, `ContentFilterError`, `StructuredOutputError`, and
  `MessageOutputLengthError` remain `agent_native`; broad `model`, `error`, or
  `output` message needles would misclassify ordinary failures and prose.
- Exact provider-authored capacity copy, a `resource_exhausted` spelling, and
  provider-independent rate-limit/billing copy beyond the sticky seeds could
  not be confirmed and are retained as frontmatter gaps.

## Sources

- [OpenCode CLI `run` documentation at `v1.17.7`](https://github.com/anomalyco/opencode/blob/v1.17.7/packages/web/src/content/docs/cli.mdx#L338-L383)
- [OpenCode JSON event emission and error projection at `v1.17.7`](https://github.com/anomalyco/opencode/blob/v1.17.7/packages/opencode/src/cli/cmd/run.ts#L605-L725)
- [OpenCode session named-error schemas at `v1.17.7`](https://github.com/anomalyco/opencode/blob/v1.17.7/packages/core/src/v1/session.ts#L30-L57)
- [OpenCode `session.error` event schema at `v1.17.7`](https://github.com/anomalyco/opencode/blob/v1.17.7/packages/opencode/src/session/session.ts#L355-L374)
- [OpenCode error conversion at `v1.17.7`](https://github.com/anomalyco/opencode/blob/v1.17.7/packages/opencode/src/session/message-v2.ts#L614-L740)
- [OpenCode stream-code and API-error normalization at `v1.17.7`](https://github.com/anomalyco/opencode/blob/v1.17.7/packages/opencode/src/provider/error.ts#L23-L185)
- [OpenCode provider model/init discriminators at `v1.17.7`](https://github.com/anomalyco/opencode/blob/v1.17.7/packages/opencode/src/provider/provider.ts#L1075-L1090)
- [OpenCode retry and usage-limit handling at `v1.17.7`](https://github.com/anomalyco/opencode/blob/v1.17.7/packages/opencode/src/session/retry.ts#L35-L110)
- [OpenCode error-normalization tests at `v1.17.7`](https://github.com/anomalyco/opencode/blob/v1.17.7/packages/opencode/test/session/message-v2.test.ts#L1366-L1504)
- [Claudine OpenCode signal detection research](../signals/opencode.md)

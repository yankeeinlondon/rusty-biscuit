---
$schema: ./_schema.yaml
created: 2026-07-14
last_updated: 2026-07-14
agent: codex
model: default
docs: https://geminicli.com/docs/cli/headless/
kind_buckets:
  - kind: configuration
    needles:
      - text: auth
        evidence: seed
      - text: permission
        evidence: seed
      - text: config
        evidence: seed
      - text: denied
        evidence: seed
      - text: forbidden
        evidence: source_code
        source: https://github.com/google-gemini/gemini-cli/blob/v0.50.0/packages/core/src/utils/errors.ts#L137-L156
      - text: unauthorized
        evidence: source_code
        source: https://github.com/google-gemini/gemini-cli/blob/v0.50.0/packages/core/src/utils/errors.ts#L137-L159
  - kind: api_remote
    needles:
      - text: rate
        evidence: seed
      - text: quota
        evidence: seed
      - text: billing
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
  - kind: agent_native
    needles:
      - text: fatalturnlimitederror
        evidence: source_code
        source: https://github.com/google-gemini/gemini-cli/blob/v0.50.0/packages/core/src/utils/errors.ts#L105-L109
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
        evidence: issue_tracker
        source: https://github.com/google-gemini/gemini-cli/issues/7227
      - text: resource_exhausted
        evidence: issue_tracker
        source: https://github.com/google-gemini/gemini-cli/issues/23362
      - text: no capacity available for model
        evidence: issue_tracker
        source: https://github.com/google-gemini/gemini-cli/issues/23362
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
  - kind: interrupted
    needles:
      - text: interrupt
        evidence: seed
      - text: cancel
        evidence: seed
      - text: aborted
        evidence: seed
gaps:
  - area: numeric-wire-codes
    notes: >-
      Gemini stream-json result errors have type and message but no numeric code
      field. HTTP 429 and 503 occur inside provider-formatted messages and lower
      API errors, but bare decimal substrings are unsafe and do not justify
      code_buckets.
  - area: capacity-structured-discriminator
    notes: >-
      RESOURCE_EXHAUSTED and MODEL_CAPACITY_EXHAUSTED are attested inside reported
      Google API payloads, while the ADK stream projection strips its internal
      RESOURCE_EXHAUSTED status and emits only severity plus message. No stable
      capacity-specific result.error.type was confirmed at v0.50.0.
  - area: authentication-message-copy
    notes: >-
      FatalAuthenticationError is a stable structured type, but exact current
      authentication and permission message copy varies by auth backend. No new
      message needle beyond the sticky seed vocabulary is proposed.
  - area: upstream-error-specificity
    notes: >-
      Generic Google API failures preserve provider-formatted message text, but
      no narrower version-stable upstream/server phrase beyond the seeded api
      error vocabulary and the attested capacity phrases was confirmed.
changes: []
requires_claudine_update: true
reason: >-
  The proposal preserves the Phase-A cascade and adds structured forbidden,
  unauthorized, and fatal turn-limit kinds plus attested overloaded,
  resource_exhausted, and no-capacity message terms.
---

# Gemini CLI Error-Classification Vocabulary

## Overview

Gemini CLI is open source. Its documented headless `--output-format stream-json`
mode writes newline-delimited JSON events to stdout. Errors appear either as an
`error` event carrying `severity` and free-form `message`, or as a terminal
`result` with `status: "error"` and a nested `error.type` and `error.message`.
The public headless documentation names both event classes, while the tagged
TypeScript interfaces define their exact shapes.

The structured type is derived from the JavaScript error name; the message is
formatted from the underlying CLI or Google API error. That makes built-in
fatal class names comparatively stable and remote API copy less stable. The
stream result does not carry a numeric error code, even when the nested message
contains a Google API payload with an HTTP code and RPC status.

## Error Surfaces

### Structured Error Kinds

Terminal stream results carry `error.type`. The common error handler obtains it
from the error's name or constructor name, and dedicated handlers emit names
such as `FatalCancellationError` and `FatalTurnLimitedError`. These names are a
first-class machine field, not a diagnostic side-channel. An `error` event is
different: its `severity` is only `warning` or `error`, so it normally supplies
no family-specific kind vocabulary.

The proposed `kind_buckets` retain all four seeded buckets and their item order.
`forbidden` and `unauthorized` are appended to the leading configuration bucket
because Gemini defines `ForbiddenError` and `UnauthorizedError`, and the first
bucket intentionally gives configuration failures precedence over remote API
terms. A final `agent_native` bucket recognizes `FatalTurnLimitedError` only
after every seeded bucket has had its original opportunity to match.

### Message Text

Both error surfaces carry a human-readable `message`. The terminal handler runs
arbitrary failures through `parseAndFormatApiError`; consequently a message can
contain Gemini CLI copy, Google API copy, or a serialized nested API error. The
field is contractually present, but most remote wording is not an enum-like
contract.

The first message bucket remains `api_remote`, followed by `configuration` and
then `interrupted`. Capacity additions belong in the first bucket so a message
such as `API Error: No capacity available for model ... on the server` resolves
to `api_remote` before the later buckets are considered.

### Numeric Codes

The non-stream JSON schema permits `error.code`, and the process has documented
exit codes, but stream JSON's terminal `ResultEvent.error` contains only `type`
and `message`. `handleError` uses an extracted numeric value to choose the
process exit status without adding it to the emitted stream event. Gemini
therefore has no `code_buckets` proposal.

HTTP `429` and `503` remain useful evidence about the cause of an error. Their
wire-level interpretation and exit-code mapping are detection concerns covered
by [Gemini signal detection research](../signals/gemini.md), not numeric
rendering vocabulary.

## Rate Limit, Quota, and Billing

Gemini's tagged quota classifier converts Google API `429`, `499`, and `503`
failures into `TerminalQuotaError` or `RetryableQuotaError`. It recognizes
`RATE_LIMIT_EXCEEDED`, `QUOTA_EXHAUSTED`, daily and per-minute quotas, retry
delays, and insufficient-credit metadata. The structured class names already
contain the seeded `quota` substring, so the second kind bucket classifies both
as `api_remote` without another needle.

The first message bucket likewise retains `rate limit`, `quota`, `billing`, and
`api error`. Source-defined daily-quota copy contains `quota`; provider-returned
rate-limit copy commonly contains either `rate limit` or one of the capacity
terms added below. A terminal usage-cap or rate-limit record may also warrant a
normalized signal, but record detection belongs to the
[signals document](../signals/gemini.md).

## Authentication, Permission, and Configuration

Gemini defines `FatalAuthenticationError`, `FatalConfigError`, `ForbiddenError`,
and `UnauthorizedError`. The first two already match the seeded structured
needles `auth` and `config`. `ForbiddenError` and `UnauthorizedError` do not, so
the proposal appends `forbidden` and `unauthorized` to that same leading
`configuration` bucket.

The seeded message terms `api key`, `authentication`, `not authorized`, and
`permission denied` remain unchanged. Exact copy varies across Google login,
Gemini API key, Vertex AI, and Code Assist paths, so no broader `key`, `auth`,
`permission`, `401`, or `403` message substring is added. Authentication can
also fail before the main noninteractive loop; deciding whether stderr or an
exit status fires `auth_invalid` is signal detection and remains documented as
a gap in [Gemini signal detection research](../signals/gemini.md).

## Interruption, Cancellation, and Abort

Ctrl+C in headless mode creates `FatalCancellationError` with the exact message
`Operation cancelled.` and emits it as a terminal stream result. Both values
already match sticky seed terms: the structured `cancel` needle and message
`cancel` needle classify them as `interrupted`. `CanceledError` uses the
single-l spelling `canceled`, which is also covered by the same substring.

Interruption remains after configuration and the first remote-API pass in the
kind cascade, and after configuration in the message cascade. That order is
preserved even though a contrived mixed message could therefore classify as a
different family before reaching `cancel`.

## Upstream, Server, and Provider Errors

The late structured `api_remote` bucket retains `api`, `upstream`, and `server`.
Its position is deliberate: a kind containing both a configuration term and
`api` is configuration, while rate/quota/billing kinds win in the earlier
remote bucket. Generic errors can have the unhelpful type `Error`; their useful
classification usually comes from the formatted message instead.

No additional generic message substring such as `server`, `model`, `error`, or
`unavailable` is proposed. Those words are common in successful assistant text,
tool output, model names, and local diagnostics. The narrower capacity phrases
below cover verified provider failures without turning every server or model
mention into an API error.

## Agent-Native Turn Limits

Gemini emits `FatalTurnLimitedError` with a message explaining that the maximum
session turns were reached. It is a CLI policy failure rather than an API,
configuration, or interruption failure, so the proposal appends an
`agent_native` kind bucket containing `fatalturnlimitederror`. Appending the
bucket preserves every seeded bucket's position and precedence.

The same terminal condition is already a detection record for
`turn_limit_reached` in [Gemini signal detection research](../signals/gemini.md).
That record decides when to fire a signal; this needle only controls the error
kind and summary after an error envelope has been selected.

## Capacity and Overload

Capacity failures are confirmed on Gemini CLI's noninteractive surface. A
Gemini CLI 0.2.1 issue captures a structured error whose nested Google payload
has code `503`, status `UNAVAILABLE`, and message `The model is overloaded.
Please try again later.` A later Gemini CLI 0.34.0 issue captures code `429`,
status `RESOURCE_EXHAUSTED`, reason `MODEL_CAPACITY_EXHAUSTED`, and message `No
capacity available for model gemini-3.1-pro-preview on the server`.

The proposal adds three lowercase message needles to the first `api_remote`
bucket: `overloaded`, `resource_exhausted`, and `no capacity available for
model`. The phrase needle is intentionally model-agnostic. The status needle
handles formatted messages that preserve the RPC discriminator, while the
natural-language needle handles messages where formatting drops the status.

Gemini v0.50.0 source classifies `503` as retryable quota/capacity behavior and
internally recognizes `RESOURCE_EXHAUSTED` on the ADK event path. That projection
emits only `severity` and stripped `message`, however, so neither internal status
is added to `kind_buckets`. Bare `429` and `503` are not proposed as message
needles or numeric-code buckets.

## Collisions and Precedence

| Candidate | Decision | Winning behavior or collision |
| --- | --- | --- |
| `rate` | Retained only in seeded structured kinds | Broad in prose (`rate of change`, token rate); the classifier sees it only after an error record has been selected. In a kind containing `auth` and `rate`, leading configuration wins. |
| `model` | Rejected | Appears in successful answers, model metadata, and every model-specific failure; it does not identify capacity. |
| `auth` | Retained only in the seeded leading kind bucket | Correctly catches `FatalAuthenticationError`; a broader message needle would also match explanatory prose and configuration instructions. |
| `401`, `403` | Rejected | Bare numbers can occur in timestamps, identifiers, tool output, and prose; stream JSON has no numeric error-code field. |
| `429`, `503` | Rejected | These are HTTP evidence and signal-detection inputs, not safe message substrings or native stream code fields. |
| `server` | Retained only in the seeded late kind bucket | Too broad for messages. `No capacity available ... on the server` is caught earlier by its full capacity clause. |
| `overloaded` | Added to first message bucket | Distinctive in an error-selected message. It wins before configuration and interruption terms. |
| `resource_exhausted` | Added to first message bucket | Exact RPC spelling is distinctive; the underscore avoids the broader word `exhausted`. |
| `no capacity available for model` | Added to first message bucket | Narrower than `capacity` or `model`; it wins before the late structured `server` concept can matter. |

Representative successful stream events contain model identifiers and token
statistics, and assistant messages can discuss APIs, servers, HTTP codes, or
rate calculations. Claudine classifies only selected error kind/message fields,
which limits—but does not eliminate—the risk from broad substrings. The new
needles therefore use attested failure clauses rather than isolated `capacity`,
`model`, or decimal codes.

## Quirks and Gaps

- Stream `error.severity` is only `warning` or `error`; it is not the Google RPC
  status. In the ADK path, `RESOURCE_EXHAUSTED` merely changes severity to
  `error`, and the status itself is discarded.

- `result.error.type` reflects a JavaScript error name. Built-in fatal and quota
  classes classify well, but an ordinary `Error` depends entirely on its
  formatted message.

- `FatalAuthenticationError` can occur before the main stream loop. The exact
  structured-output behavior varies by output format and authentication path,
  so the frontmatter adds no speculative authentication message copy.

- No numeric code reaches Gemini's stream result error. Process exit codes and
  HTTP codes must not be conflated with Kimi-style exact wire-code buckets.

- Capacity wording has strong issue-tracker provenance but is service-returned
  copy rather than a CLI-owned constant. The three additions are deliberately
  narrower than `capacity`, `model`, `server`, or bare HTTP numbers.

## Changelog

- 2026-07-14: Replaced the capacity gap with issue-attested overload and
  resource-exhaustion message terms; added source-attested structured kinds for
  forbidden, unauthorized, and fatal turn-limit failures; and refreshed the
  surface analysis against Gemini CLI v0.50.0.

## Sources

- [Gemini CLI headless mode reference](https://geminicli.com/docs/cli/headless/)
- [Gemini CLI v0.50.0 stream JSON event contracts](https://github.com/google-gemini/gemini-cli/blob/v0.50.0/packages/core/src/output/types.ts#L29-L117)
- [Gemini CLI v0.50.0 terminal stream error emission](https://github.com/google-gemini/gemini-cli/blob/v0.50.0/packages/cli/src/utils/errors.ts#L60-L110)
- [Gemini CLI v0.50.0 cancellation and turn-limit result handlers](https://github.com/google-gemini/gemini-cli/blob/v0.50.0/packages/cli/src/utils/errors.ts#L167-L228)
- [Gemini CLI v0.50.0 error type and fatal error definitions](https://github.com/google-gemini/gemini-cli/blob/v0.50.0/packages/core/src/utils/errors.ts#L57-L159)
- [Gemini CLI v0.50.0 Google quota classification](https://github.com/google-gemini/gemini-cli/blob/v0.50.0/packages/core/src/utils/googleQuotaErrors.ts#L201-L385)
- [Gemini CLI v0.50.0 ADK resource-exhausted stream projection](https://github.com/google-gemini/gemini-cli/blob/v0.50.0/packages/cli/src/nonInteractiveCliAgentSession.ts#L570-L581)
- [Gemini CLI 0.2.1 overload report](https://github.com/google-gemini/gemini-cli/issues/7227)
- [Gemini CLI 0.34.0 model-capacity report](https://github.com/google-gemini/gemini-cli/issues/23362)
- [Claudine Gemini signal detection research](../signals/gemini.md)

---
$schema: ./_schema.yaml
created: 2026-07-14
last_updated: 2026-07-14
agent: codex
model: default
docs: https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/
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
      - text: no auth type is selected
        evidence: source_code
        source: https://github.com/QwenLM/qwen-code/blob/v0.19.6/packages/cli/src/validateNonInterActiveAuth.ts#L20-L31
  - kind: interrupted
    needles:
      - text: interrupt
        evidence: seed
      - text: cancel
        evidence: seed
      - text: aborted
        evidence: seed
  - kind: agent_native
    needles:
      - text: loop detection halted the run
        evidence: source_code
        source: https://github.com/QwenLM/qwen-code/blob/v0.19.6/packages/cli/src/nonInteractiveCli.ts#L143-L163
gaps:
  - area: capacity-overload-message
    notes: >-
      Qwen v0.19.6 internally classifies HTTP 529 with the diagnostic reason
      capacity-overload and treats HTTP 503 as rate limiting, but the current
      stream-json result builder emits only the provider or CLI message. No
      stable headless phrasing containing overloaded, at capacity,
      resource_exhausted, or a capacity-specific 429/503 clause was confirmed,
      so none is guessed as a substring needle.
  - area: structured-error-kind-projection
    notes: >-
      CLIResultMessageError permits optional error.type, but the v0.19.6 shared
      result builder emits error.message only. Claudine's sticky kind buckets
      remain compatibility vocabulary; no new current kind discriminator was
      confirmed on the primary headless stream.
  - area: numeric-wire-codes
    notes: >-
      Qwen's internal classifiers inspect HTTP 401, 403, 429, 503, 529, 1302,
      and 1305, but stream-json has no dedicated numeric error-code field.
      Numeric status embedded in free-form text is not a code_buckets surface.
  - area: upstream-provider-copy
    notes: >-
      Qwen supports multiple OpenAI-compatible providers and preserves their
      formatted error messages. Exact server, authentication, billing, and
      throttling copy therefore varies by configured backend; no additional
      provider-independent phrase was confirmed beyond the sticky seeds.
changes: []
requires_claudine_update: true
reason: >-
  The proposal preserves the Phase-A cascade and adds exact source-attested
  message needles for Qwen's missing-auth preflight and provider-native loop
  detector terminal error.
---

# Qwen CLI Error-Classification Vocabulary

## Overview

Qwen Code is open source. Its documented headless mode supports JSON and
newline-delimited `stream-json` output for automation. In `stream-json`, a
terminal failure is a `type: "result"` record with `is_error: true`, a result
subtype, and nested free-form `error.message`. The tagged TypeScript contract
also permits optional `error.type`, but the shared result builder at `v0.19.6`
does not populate it.

The stable classification input is therefore message text. Qwen has richer
internal retry diagnostics—including HTTP status, provider code, provider
message, transport code, and reasons such as `rate-limit`, `auth-error`, and
`capacity-overload`—but those are diagnostic side channels rather than fields
in the primary headless result envelope. Claudine retains its seeded structured
kind vocabulary for compatibility with accepted legacy or observed envelopes.

## Error Surfaces

### Structured Error Kinds

`CLIResultMessageError` allows `error.type?: string`, while its terminal subtype
is limited to `error_max_turns` or `error_during_execution`. The current shared
builder emits `error: { message }` and omits `error.type`; the subtype describes
execution disposition, not the underlying family. Consequently, no new
structured kind needle is proposed. All four seeded kind buckets and their
needles retain their original order.

The seeded kind cascade is still meaningful for compatibility records: remote
rate, quota, and billing terms win first; configuration terms win next;
interruption terms follow; and broad API, upstream, or server terms run last.

### Message Text

The terminal result's `error.message` is a first-class headless field. Auth
preflight failures are explicitly converted to an error result in JSON and
`stream-json` modes. Main-loop failures also reach this field, including
provider-formatted API errors and Qwen's provider-native loop detector message.

Provider API wording is less stable than Qwen-owned CLI copy because Qwen can
target multiple OpenAI-compatible backends. The proposal therefore adds only
two exact Qwen-owned phrases and preserves every seeded message needle in its
original position.

### Numeric Codes

There is no numeric error-code field in the `stream-json` terminal result.
Internally, Qwen recognizes HTTP or provider codes `401`, `403`, `429`, `503`,
`529`, `1302`, and `1305`, plus string codes such as
`Throttling.AllocationQuota`. These values drive retry diagnosis below the
stream projection; they do not justify `code_buckets` or bare-number message
needles.

Wire-level rate-limit and exit-code detection is documented separately in
[Qwen signal detection research](../signals/qwen.md).

## Rate Limit, Quota, and Billing

The first structured-kind bucket classifies `rate`, `quota`, and `billing` as
`api_remote`. The first message bucket classifies `rate limit`, `quota`,
`billing`, and `api error` the same way. Their order is unchanged.

Qwen's internal `isRateLimitError` recognizes `429`, `503`, `1302`, and `1305`.
The retry classifier separately recognizes Qwen OAuth's `free allocated quota
exceeded` condition and DashScope's `Throttling.AllocationQuota`. Both quota
phrases already contain the earlier seeded `quota` message needle. Detection
of a typed rate-limit record, quota signal, or usage cap belongs to
[the signals topic](../signals/qwen.md), not this rendering vocabulary.

## Authentication, Permission, and Configuration

The seeded configuration kind bucket checks `auth`, `config`, and `permission`.
The seeded message bucket checks `api key`, `authentication`, `not authorized`,
and `permission denied`. These positions remain fixed behind the first remote
message bucket.

Before the main loop, `validateNonInteractiveAuth` raises the exact message
`No auth type is selected...` when configuration resolves no authentication
method, then emits it as a terminal result in structured modes. This phrase
does not contain any seeded message needle, so `no auth type is selected` is
appended to the existing configuration bucket. The longer word
`authentication` is not substituted because the source string specifically
uses the abbreviated `auth` form.

## Interruption, Cancellation, and Abort

The third kind bucket preserves `interrupt`, `cancel`, and `abort`; the third
message bucket preserves `interrupt`, `cancel`, and `aborted`. Qwen's
non-interactive runner throws `Operation cancelled.` and its retry classifier
uses the reason `aborted`, so the sticky terms already cover those forms.

Cancellation also has exit-code behavior and does not consistently produce a
terminal stream record. Mapping process exit 130 to an interruption signal is
therefore owned by [Qwen signal detection research](../signals/qwen.md); this
document only classifies text after Claudine has selected an error surface.

## Upstream, Server, and Provider Errors

The final seeded kind bucket classifies `api`, `upstream`, and `server` as
`api_remote`. Its late position is intentional. A structured discriminator
containing both an authentication or interruption term and a broad remote term
must resolve through the earlier, more specific family.

Qwen's internal retry classifier distinguishes transport errors, client
errors, server errors, rate limits, provider-business failures, and unknown
errors. Current headless results flatten these diagnoses to message text. No
broad message needles such as `server`, `provider`, `http`, `failed`, or
`error` are proposed because they occur routinely in local diagnostics, tool
output, and successful assistant prose.

## Agent-Native Loop Detection

Qwen's non-interactive runner formats native loop-guard failures with the exact
leading clause `Loop detection halted the run`. This is a Qwen agent policy
failure, not a remote API or user interruption, so a final `agent_native`
message bucket is appended after all seeded buckets.

The late position preserves the Phase-A cascade. In a contrived mixed message,
a seeded rate-limit, configuration, or interruption phrase still wins before
the native-loop marker. Firing Claudine's normalized `runaway_repetition`
signal from this record is detection behavior covered by
[Qwen signal detection research](../signals/qwen.md).

## Capacity and Overload

Qwen `v0.19.6` explicitly classifies HTTP 529 as `capacity-overload` and treats
HTTP 503 as a retryable throttling or overload status. That establishes an
internal capacity family, but not a safe headless message vocabulary. The
terminal result builder emits the provider's or CLI's message without the
internal diagnosis reason, and no pinned source or fixture established a
stable `overloaded`, `at capacity`, `resource_exhausted`, or capacity-specific
429/503 phrase on this surface.

No capacity needle is proposed. The absence is recorded in `gaps` rather than
borrowing Codex's `Selected model is at capacity` wording or guessing that an
internal Qwen diagnostic reaches stdout. The known HTTP status classifiers and
their retry meaning remain documented in [the signals topic](../signals/qwen.md).

## Collisions and Precedence

| Candidate | Decision | Winning behavior or collision |
|---|---|---|
| `no auth type is selected` | Add after the seeded configuration messages | Exact Qwen preflight copy; an earlier remote phrase still wins in mixed text. |
| `loop detection halted the run` | Add in a final `agent_native` bucket | Exact native-guard marker; every sticky bucket retains precedence. |
| `rate` | Preserve only in structured kinds | The sticky kind needle is broad; normal prose commonly discusses rates. |
| `auth` | Preserve only in structured kinds | Too broad for messages: success text includes authentication status and setup instructions. |
| `model` | Reject | Model selection and initialization records use it during successful runs. |
| `overloaded` / `at capacity` | Gap | Internal capacity diagnosis exists, but no stable headless phrase was confirmed. |
| `401`, `403`, `429`, `503`, `529` | Reject | Bare substring numbers collide with counts, IDs, tool output, and ordinary prose. |
| `api` / `server` | Preserve only in the late structured-kind bucket | As message needles they would match configuration help, code, and successful discussion. |

Representative successful surfaces include `system` initialization records
with model metadata, assistant text, tool results, and successful terminal
results. Authentication setup prose and model/provider discussions make broad
`auth`, `model`, `provider`, and `server` message terms unsafe. Neither proposed
exact phrase appears in those normal contracts.

## Quirks and Gaps

- The stream type permits `error.type`, but the current result builder does not
  populate it. Seeded kind buckets are compatibility behavior, not evidence of
  a current first-class discriminator.
- Result subtype `error_during_execution` is too generic for a semantic family;
  `error_max_turns` describes an agent-native condition but is not passed as
  the nested error-kind discriminator by the current builder.
- Qwen's internal retry classifier is richer than its headless projection.
  Diagnostic reasons and numeric statuses must not be treated as emitted
  vocabulary without a version-pinned record path.
- Provider error copy varies across configured OpenAI-compatible services.
  Broad cross-provider guesses are deliberately excluded.
- Capacity is confirmed internally but unconfirmed as stable headless copy;
  this is the principal unresolved rendering gap.

## Changelog

- 2026-07-14: Reverified Qwen against tagged `v0.19.6` source, replaced the
  earlier unpinned surface description with source-attested stream contracts,
  and proposed exact auth-preflight and native loop-detector message needles.

## Sources

- [Qwen Code headless mode documentation](https://qwenlm.github.io/qwen-code-docs/en/users/features/headless/)
- [Qwen `v0.19.6` non-interactive result types](https://github.com/QwenLM/qwen-code/blob/v0.19.6/packages/cli/src/nonInteractive/types.ts#L126-L158)
- [Qwen `v0.19.6` terminal result builder](https://github.com/QwenLM/qwen-code/blob/v0.19.6/packages/cli/src/nonInteractive/io/BaseJsonOutputAdapter.ts#L1157-L1205)
- [Qwen `v0.19.6` auth-preflight structured error path](https://github.com/QwenLM/qwen-code/blob/v0.19.6/packages/cli/src/validateNonInterActiveAuth.ts#L16-L82)
- [Qwen `v0.19.6` native loop-detector message](https://github.com/QwenLM/qwen-code/blob/v0.19.6/packages/cli/src/nonInteractiveCli.ts#L143-L163)
- [Qwen `v0.19.6` retry error classifier](https://github.com/QwenLM/qwen-code/blob/v0.19.6/packages/core/src/utils/retryErrorClassification.ts#L12-L192)
- [Qwen `v0.19.6` rate-limit classifier](https://github.com/QwenLM/qwen-code/blob/v0.19.6/packages/core/src/utils/rateLimit.ts#L11-L170)
- [Qwen `v0.19.6` quota detection helpers](https://github.com/QwenLM/qwen-code/blob/v0.19.6/packages/core/src/utils/quotaErrorDetection.ts#L72-L119)
- [Qwen signal detection research](../signals/qwen.md)

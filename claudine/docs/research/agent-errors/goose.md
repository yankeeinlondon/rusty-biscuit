---
$schema: ./_schema.yaml
created: 2026-07-14
last_updated: 2026-07-14
agent: codex
model: default
docs: https://goose-docs.ai/docs/guides/running-tasks/
kind_buckets: []
msg_buckets: []
gaps:
  - area: no-stream-parser
    notes: >-
      Claudine has no Goose parser in lib/src/stream/providers and no immutable
      Phase-A seed file for Goose. Goose is therefore research-only for this
      topic and explicitly empty at runtime. Its stream-json message and system
      notification records are detection surfaces owned by signals/goose.md.
  - area: capacity-overload-phrasing
    notes: >-
      Goose v1.43.0 defines no overload- or capacity-specific ProviderError.
      Provider 5xx failures can normalize to the broad ServerError display
      prefix "Server error:", but that does not establish overloaded, at
      capacity, or resource_exhausted wording. The coarse detection mapping is
      documented in signals/goose.md; no rendering needle is guessed here.
  - area: interruption-error-frame
    notes: >-
      Goose cancellation is internal control flow and v1.43.0 exposes no stable
      interrupted or canceled terminal error discriminator in stream-json.
  - area: numeric-wire-codes
    notes: >-
      Provider adapters inspect HTTP status codes including 401, 403, 429, and
      503, but Goose does not expose them as a dedicated numeric error-code
      field consumed by Claudine. Bare numbers are unsafe substring needles.
changes: []
requires_claudine_update: false
---

# Goose CLI Error-Classification Vocabulary

## Overview

Goose is an open-source Rust agent. Its non-interactive `goose run` command
supports aggregate JSON and newline-delimited `stream-json`. The latter carries
tagged `message`, `notification`, `error`, and `complete` records, but most LLM
provider failures are caught inside the agent loop and converted to assistant
message content. Goose's first-party `ProviderError` enum supplies stable display
prefixes; provider response details remain free-form.

Claudine does not currently parse Goose structured output: there is no Goose
module in `lib/src/stream/providers`, no Phase-A Goose seed, and the generated
runtime vocabulary is intentionally empty. Consequently this is a research-only
catalog for a future parser, not an executable vocabulary proposal. Goose's
wire-record detection rules belong to [`signals/goose.md`](../signals/goose.md).

## Error Surfaces

### Structured Stream Records

`goose run --output-format stream-json` is the structured automation surface.
It serializes tagged stream records, but it does not expose `ProviderError` as a
stable error-kind discriminator. Caught provider failures normally appear inside
a `message` record; a separate `error` record represents errors escaping the
agent event stream. Choosing which record taints a run is detection policy, not
`SemanticErrorKind` vocabulary.

### Message Text

The generic agent-loop branch renders `Ran into this error: {provider_err}.`
followed by retry advice. `NetworkError` is rendered without that wrapper, while
credits exhaustion and context recovery use system notifications. This message
surface is source-attested, but no Claudine Goose parser currently passes it to
`classify_error_by_keywords`.

### System Notifications

`ProviderError::CreditsExhausted` becomes a camel-case
`notificationType: creditsExhausted` system notification with a user-facing
message and optional `top_up_url`. This is a first-class structured detection
surface. It is owned by the `no_funds` rule in
[`signals/goose.md`](../signals/goose.md), not by this rendering vocabulary.

### Numeric Codes

Goose provider adapters interpret HTTP statuses, and its Google status enum
names 400, 401, 403, 404, 429, 500, and 503. Those values are lower-layer
inputs, not a dedicated numeric code in the stream error contract. Goose has no
`code_buckets` proposal.

## Rate Limit, Quota, and Billing

`ProviderError::RateLimitExceeded` has the exact display prefix `Rate limit
exceeded:` and carries details plus an optional retry delay. The generic agent
branch retains only its display text in the assistant message, so `retry_delay`
is not available there. A future parser could safely classify the exact prefix
as `api_remote` before broader remote-error terms.

Credits exhaustion is stronger and structurally distinct: Goose emits the
`creditsExhausted` system notification instead of the generic error wrapper.
That record fires `no_funds` detection in
[`signals/goose.md`](../signals/goose.md); it is not duplicated in frontmatter.
No separate Goose-owned quota or billing message was confirmed.

## Authentication, Permission, and Configuration

`ProviderError::Authentication` displays `Authentication error:` and would map
to `configuration`. In the generic branch it becomes `Ran into this error:
Authentication error: ...`. Goose does not define a separate provider permission
or configuration error variant in this catalog. Tool-policy denials and
non-interactive approval failures are agent-control surfaces, not evidence for a
provider authentication needle.

## Interruption, Cancellation, and Abort

No stable `ProviderError` variant or stream terminal discriminator for
interruption, cancellation, or abort was confirmed at v1.43.0. Ctrl+C and
cancellation-token handling are detection and process-control concerns. This
family remains a gap rather than inheriting another provider's vocabulary.

## Upstream, Server, and Provider Errors

Goose defines the exact prefixes `Server error:`, `Network error:`, `Request
failed:`, and `Endpoint not found (404):`. These describe remote or transport
failures and would generally map to `api_remote`; `Provider refused request:` is
rendered through its own terminal refusal prose. `Execution error:`, `Usage data
error:`, and `Unsupported operation:` are not safely remote: their details can
describe local or agent-native failures.

The typed prefixes are useful seed candidates only after a Goose parser lands.
Adding them now would not alter any consumed classification path and would
misrepresent the generated empty-runtime contract.

## Capacity and Overload

The motivating capacity class is not source-confirmed for Goose. At v1.43.0 the
typed catalog has `ServerError`, but no overload, capacity, or resource-exhausted
variant or display constant. HTTP 503 exists in a provider-specific status enum,
yet it does not prove that `overloaded`, `at capacity`, or `resource_exhausted`
will reach structured CLI output.

The existing [`signals/goose.md`](../signals/goose.md) rule treats the fixed
`Server error:` wrapper as a deliberately coarse `provider_overloaded`
detection proxy. This document does not turn that proxy into a rendering needle.
The absence of exact capacity wording is recorded in `gaps`.

## Collisions and Precedence

| Candidate | Decision | Collision or precedence consequence |
|---|---|---|
| `rate limit exceeded:` | Future parser candidate | Exact typed prefix; should precede broad server/remote terms and map to `api_remote`. |
| `authentication error:` | Future parser candidate | Exact typed prefix; should map to `configuration` before any broad provider term. |
| `server error:` | Do not add now | Source-attested but coarse; can represent validation and other non-capacity failures. |
| `rate` | Reject | Ordinary success prose can discuss rates, and the substring is much broader than Goose's typed prefix. |
| `auth` | Reject | Can occur in configuration instructions, tool output, and identifiers. |
| `model` | Reject | Routine model-selection and success prose uses it. |
| `provider` | Reject | Goose routinely discusses provider configuration and identity outside errors. |
| `401`, `403`, `429`, `503` | Reject | Bare numeric substring matches can collide with counts, IDs, ports, timestamps, and tool output. |
| `overloaded`, `at capacity`, `resource_exhausted` | Gap | None is confirmed in Goose v1.43.0 CLI-rendered error copy. |

With empty runtime buckets there is no current first-hit order. If a parser is
added, narrow typed prefixes should be ordered authentication, rate limit, then
coarse remote families so broad text cannot shadow the specific classifications.

## Quirks and Gaps

- The user-specified seed path does not exist in this repository or `main`.
  Goose is the documented parser-less exception, so there are no sticky rows to
  preserve, re-kind, or reorder.
- Structured `stream-json` does not mean structured provider error kinds: most
  provider errors are converted to assistant message text.
- `CreditsExhausted` is promoted to a system notification and therefore bypasses
  the generic `ProviderError` text wrapper.
- `ContextLengthExceeded` first drives compaction and only later emits an inline
  inability-to-continue notification; classifying it solely from enum display
  copy would not reflect the observable stream.
- No exact capacity wording, interruption discriminator, or numeric stream code
  was confirmed. Each remains explicit in frontmatter `gaps`.

## Sources

- [Goose running-tasks documentation](https://goose-docs.ai/docs/guides/running-tasks/)
- [Goose headless-mode documentation at v1.43.0](https://github.com/aaif-goose/goose/blob/v1.43.0/documentation/docs/tutorials/headless-goose.md#what-is-headless-mode)
- [`ProviderError` display strings and HTTP status enum at v1.43.0](https://github.com/aaif-goose/goose/blob/v1.43.0/crates/goose-provider-types/src/errors.rs#L7-L53)
- [Goose agent-loop error projection at v1.43.0](https://github.com/aaif-goose/goose/blob/v1.43.0/crates/goose/src/agents/agent.rs#L2480-L2603)
- [System-notification wire types at v1.43.0](https://github.com/aaif-goose/goose/blob/v1.43.0/crates/goose-provider-types/src/conversation/message.rs#L252-L279)
- [Claudine Goose signal-detection research](../signals/goose.md)

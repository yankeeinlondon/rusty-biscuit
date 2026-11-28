---
$schema: ./_schema.yaml
created: 2026-07-14
last_updated: 2026-07-14
agent: codex
model: default
docs: https://developers.openai.com/codex/noninteractive
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
        evidence: source_code
        source: https://github.com/openai/codex/blob/rust-v0.139.0/codex-rs/protocol/src/error.rs#L68-L141
      - text: selected model is at capacity
        evidence: source_code
        source: https://github.com/openai/codex/blob/rust-v0.139.0/codex-rs/protocol/src/error.rs#L108-L118
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
  - area: structured-exec-error-kinds
    notes: >-
      Codex core and app-server expose CodexErrorInfo discriminators, but the
      rust-v0.139.0 codex exec --json projection reduces fatal errors and failed
      turns to a message-only ThreadErrorEvent. Claudine retains seeded kind
      compatibility for older or observed envelopes, but no additional current
      exec discriminator was confirmed.
  - area: numeric-http-codes
    notes: >-
      Core and app-server layers distinguish HTTP-derived overload and expose a
      JSON-RPC ingress overload code, but codex exec --json provides no numeric
      error-code field. Bare 429, 503, 401, and 403 substrings are unsafe because
      they can occur in token counts, identifiers, timestamps, tool output, and
      ordinary assistant prose.
  - area: resource-exhausted-spelling
    notes: >-
      No resource_exhausted spelling was confirmed in the rust-v0.139.0 Codex
      exec error path. The source uses ServerOverloaded and the selected-model
      capacity message instead, so resource_exhausted is not guessed as a
      needle.
  - area: authentication-exec-fixtures
    notes: >-
      Source defines Unauthorized and refresh-token failures below the exec
      projection, but no version-pinned exec JSONL fixture established exact
      current authentication or permission message copy beyond the sticky
      seeds.
changes: []
requires_claudine_update: true
reason: >-
  The proposal preserves the Phase-A cascade and appends two source-attested
  capacity terms to the first api_remote message bucket: overloaded and the
  exact selected model is at capacity clause.
---

# Codex CLI Error-Classification Vocabulary

## Overview

Codex CLI is open source. Its documented non-interactive mode, `codex exec
--json`, writes JSON Lines to stdout. The public stream has top-level `error`
and `turn.failed` records, plus non-fatal `item.completed` error items. At
`rust-v0.139.0`, fatal and failed-turn records carry a free-form `message`; the
richer internal and app-server error discriminators are flattened out of the
exec projection.

The message surface is therefore the current first-class classification input.
Claudine also accepts older or observed structured error-kind envelopes and
must preserve their seeded vocabulary, but current Codex source does not make
that discriminator part of the `exec --json` contract. No Kimi-style numeric
wire code reaches this classifier.

## Error Surfaces

### Structured Error Kinds

Codex core defines `CodexErr` and maps it to `CodexErrorInfo`, including
`UsageLimitExceeded`, `ServerOverloaded`, `Unauthorized`,
`ResponseStreamDisconnected`, and `ResponseTooManyFailedAttempts`. App-server
`TurnError` retains that optional structured information. These are genuine
provider contracts for app-server clients, but `codex exec --json` does not
preserve them: its `ThreadErrorEvent` contains only `message`.

Claudine's `kind_buckets` remain because its compatibility envelope can resolve
older or observed `error_type` and nested `error.type` fields. No new kind
needle is proposed. The four seeded buckets and every item retain their exact
positions.

### Message Text

The `exec --json` `error` event is documented in source as an unrecoverable
stream error, while `turn.failed.error` uses the same message-only structure.
The projection copies `TurnError.message` and may append non-empty
`additionalDetails`. Message substring classification is consequently the
only current exec surface that can distinguish capacity, quota, authentication,
and interruption families.

An `item.completed` item with `type: error` is explicitly non-fatal. Its message
can still resemble a failure, so selecting which wire record constitutes the
terminal error is detection policy rather than vocabulary policy. This document
classifies text only after Claudine has selected an error surface.

### Numeric Codes

No numeric code field exists on `ThreadErrorEvent`. Codex app-server uses JSON-RPC
code `-32001` with `Server overloaded; retry later.` when its own request ingress
is saturated, and lower layers interpret HTTP status and response-body codes.
Those codes do not reach `codex exec --json`, so this document has no
`code_buckets`. Bare HTTP numbers are not safe substring needles.

## Rate Limit, Quota, and Billing

The first seeded message bucket classifies `rate limit`, `quota`, `billing`, and
`api error` as `api_remote`. Core source has `UsageLimitReached`,
`QuotaExceeded`, and `UsageNotIncluded` variants; `QuotaExceeded` renders as
`Quota exceeded. Check your plan and billing details.`, which already matches
the first bucket's seeded `quota` before `billing` is considered.

Codex can emit usage-cap records such as `You've hit your usage limit` and
coarse `rate_limit` kinds. Their wire-level recognition and precedence as
`usage_capped` versus `rate_limited` signals belong to
[`signals/codex.md`](../signals/codex.md). They are not duplicated as detection
records here.

## Authentication, Permission, and Configuration

The seeded structured-kind bucket checks `auth`, `config`, `permission`, then
`denied`; the seeded message bucket checks `api key`, `authentication`, `not
authorized`, `permission denied`, then `config`. All classify as
`configuration`, and their order is unchanged.

Core and app-server source expose `Unauthorized` and refresh-token failure
information, but current exec output flattens these to message copy. Without a
version-pinned exec fixture establishing additional exact copy, narrower
authentication additions would be guesses. Broad additions such as `401`,
`403`, `token`, or `key` are rejected.

## Interruption, Cancellation, and Abort

The third kind bucket preserves `interrupt`, `cancel`, and `abort`; the third
message bucket preserves `interrupt`, `cancel`, and `aborted`. They classify as
`interrupted`. Core defines `TurnAborted` and `Interrupted`, while app-server
turn state can be `Interrupted`; the current exec projection initiates shutdown
for that state without emitting a `turn.failed` error record. Process exit and
Ctrl+C detection remain wrapper/signal concerns rather than additions here.

## Upstream, Server, and Provider Errors

The late seeded kind bucket checks `api`, `upstream`, and `server` and classifies
them as `api_remote`. Its late position is intentional: an error kind containing
an earlier rate/quota/billing, configuration, or interruption term is classified
by that earlier family before the broad remote-service terms are considered.

Codex core also defines request timeout, stream disconnection, connection
failure, unexpected HTTP status, internal server error, and retry-limit errors.
Some map to structured `CodexErrorInfo` internally, but their exec representation
is message-only. Existing seeded text covers explicit `api error` copy; no broad
`timeout`, `connection`, `failed`, or `internal` message needle is proposed
because those terms also describe agent-native and local-tool failures.

## Capacity and Overload

The capacity gap is source-confirmed. `CodexErr::ServerOverloaded` renders the
exact sentence `Selected model is at capacity. Please try a different model.`
and maps internally to `CodexErrorInfo::ServerOverloaded`. The Responses bridge
also maps the API response codes `server_is_overloaded` and `slow_down` to the
same overload family.

Two append-only `api_remote` message needles are proposed after every seeded
item: `overloaded`, followed by `selected model is at capacity`. The first
captures Codex's server-overload wording; the second closes the motivating
message-only `turn.failed` false negative without using broad `capacity` or `at
capacity`. Both belong in the first message bucket because capacity is a remote
service condition, distinct from account quota exhaustion.

HTTP 503 is associated with overload below the exec layer, but bare `503` is not
proposed. Likewise, neither `429` nor `resource_exhausted` is inferred from a
different provider's vocabulary. The unconfirmed `resource_exhausted` spelling
is recorded as a gap.

## Collisions and Precedence

| Candidate | Decision | Winning behavior or collision |
|---|---|---|
| `overloaded` | Add after seeded `api error` | Narrow overload term; earlier seeded message matches still win. |
| `selected model is at capacity` | Add after `overloaded` | Exact clause avoids matching ordinary capacity-planning prose. |
| `rate` | Preserve only in structured kinds | Sticky first-bucket seed; unsafe as a new message needle because normal prose uses “rate.” |
| `auth` | Preserve only in structured kinds | Sticky configuration seed; a kind containing it is classified before the late `api`/`server` bucket. |
| `model` | Reject | Model names and model-selection success prose are routine non-errors. |
| `capacity` / `at capacity` | Reject | Broad phrases can describe planning, storage, or non-error status; the exact Codex clause is used instead. |
| `401`, `403`, `429`, `503` | Reject | Substring matching would collide with counts, IDs, timestamps, tool output, and assistant prose. |
| `api` | Preserve in the late structured-kind bucket | Sticky broad seed; earlier configuration and interruption buckets retain precedence. |

Representative success records such as `turn.completed`, `agent_message`,
reasoning, command execution, and token usage do not contain either proposed
capacity needle. `Capacity planning completed for the selected model` contains
neither exact addition and therefore falls through rather than becoming
`api_remote`.

## Quirks and Gaps

- Current `exec --json` source is message-only even though Codex's app-server
  protocol has structured `CodexErrorInfo`; consumers must not assume the
  richer discriminator survives projection.
- A non-fatal `item.completed` error item and a fatal top-level `error` both
  carry message text. Event selection is detection policy, not something a
  keyword can safely repair.
- `turn.failed` falls back to the generic message `turn failed` if neither its
  turn payload nor a preceding critical error supplies copy. That intentionally
  remains `agent_native` rather than being classified by the unsafe word
  `failed`.
- Numeric HTTP statuses and app-server JSON-RPC `-32001` are real lower-layer
  facts but are absent from the exec classifier input.
- Exact current exec authentication copy and a `resource_exhausted` spelling
  could not be confirmed; both remain explicit frontmatter gaps.

## Sources

- [Codex non-interactive mode](https://developers.openai.com/codex/noninteractive)
- [Codex `exec --json` event contract at `rust-v0.139.0`](https://github.com/openai/codex/blob/rust-v0.139.0/codex-rs/exec/src/exec_events.rs#L9-L92)
- [Codex JSONL error projection at `rust-v0.139.0`](https://github.com/openai/codex/blob/rust-v0.139.0/codex-rs/exec/src/event_processor_with_jsonl_output.rs#L430-L545)
- [Core `CodexErr` variants and capacity message at `rust-v0.139.0`](https://github.com/openai/codex/blob/rust-v0.139.0/codex-rs/protocol/src/error.rs#L68-L141)
- [`CodexErr` to `CodexErrorInfo` mapping at `rust-v0.139.0`](https://github.com/openai/codex/blob/rust-v0.139.0/codex-rs/protocol/src/error.rs#L215-L246)
- [App-server `CodexErrorInfo` contract at `rust-v0.139.0`](https://github.com/openai/codex/blob/rust-v0.139.0/codex-rs/app-server-protocol/src/protocol/v2/shared.rs#L67-L127)
- [App-server protocol and overload code at `rust-v0.139.0`](https://github.com/openai/codex/blob/rust-v0.139.0/codex-rs/app-server/README.md#transport)
- [Responses overload mapping at `rust-v0.139.0`](https://github.com/openai/codex/blob/rust-v0.139.0/codex-rs/codex-api/src/api_bridge.rs#L38-L64)
- [Codex CLI 0.118.0 capacity incident](https://github.com/openai/codex/issues/17014)
- [Claudine Codex signal detection research](../signals/codex.md)

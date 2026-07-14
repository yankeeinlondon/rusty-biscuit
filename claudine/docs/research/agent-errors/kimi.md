---
$schema: ./_schema.yaml
created: 2026-07-14
last_updated: 2026-07-14
agent: codex
model: default
docs: https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html
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
      - text: upstream
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
      - text: auth
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
code_buckets:
  - kind: configuration
    codes:
      - code: -32004
        name: AUTH_EXPIRED
        evidence: seed
  - kind: api_remote
    codes:
      - code: -32005
        name: CHAT_PROVIDER_ERROR
        evidence: seed
  - kind: agent_native
    codes:
      - code: -32700
        name: PARSE_ERROR
        evidence: seed
  - kind: agent_native
    codes:
      - code: -32600
        name: INVALID_REQUEST
        evidence: seed
  - kind: agent_native
    codes:
      - code: -32601
        name: METHOD_NOT_FOUND
        evidence: seed
  - kind: agent_native
    codes:
      - code: -32602
        name: INVALID_PARAMS
        evidence: seed
  - kind: agent_native
    codes:
      - code: -32603
        name: INTERNAL_ERROR
        evidence: seed
  - kind: agent_native
    codes:
      - code: -32000
        name: INVALID_STATE
        evidence: source_code
        source: https://github.com/MoonshotAI/kimi-cli/blob/1.42.0/src/kimi_cli/wire/jsonrpc.py#L230-L252
  - kind: configuration
    codes:
      - code: -32001
        name: LLM_NOT_SET
        evidence: source_code
        source: https://github.com/MoonshotAI/kimi-cli/blob/1.42.0/src/kimi_cli/wire/jsonrpc.py#L230-L252
  - kind: configuration
    codes:
      - code: -32002
        name: LLM_NOT_SUPPORTED
        evidence: source_code
        source: https://github.com/MoonshotAI/kimi-cli/blob/1.42.0/src/kimi_cli/wire/jsonrpc.py#L230-L252
  - kind: api_remote
    codes:
      - code: -32003
        name: CHAT_PROVIDER_ERROR
        evidence: source_code
        source: https://github.com/MoonshotAI/kimi-cli/blob/1.42.0/src/kimi_cli/wire/jsonrpc.py#L230-L252
gaps:
  - area: capacity-overload-message
    notes: >-
      Kimi CLI 1.42.0 retries HTTP 429 and 503 and exposes the status through
      StepRetry, but no provider-owned overloaded, at capacity, or
      resource_exhausted message is defined for the terminal JSON-RPC error
      response. Those status records belong to signals/kimi.md; no message
      needle is guessed here.
  - area: provider-error-message-contract
    notes: >-
      CHAT_PROVIDER_ERROR forwards str(e) from the configured chat provider,
      so exact rate-limit, quota, billing, upstream, and capacity wording is
      provider-dependent rather than a stable Kimi-authored contract. The
      sticky seed vocabulary is retained without adding broad HTTP-number
      substrings.
  - area: print-stream-json-error-record
    notes: >-
      Print mode documents JSONL Message output, but source prints terminal
      exception text directly rather than defining an error-record schema.
      Kimi wire mode is therefore the first-class structured error surface.
  - area: permission-denial-wire-code
    notes: >-
      No dedicated wire error code for filesystem or tool permission denial
      was found in Kimi CLI 1.42.0. Approval rejection is a separate control
      flow and is not treated as a provider error-classification code.
changes: []
requires_claudine_update: true
reason: >-
  The proposal preserves every Phase-A message needle and code in place, then
  adds the source-defined -32000 INVALID_STATE, -32001 LLM_NOT_SET, -32002
  LLM_NOT_SUPPORTED, and current -32003 CHAT_PROVIDER_ERROR wire codes.
---

# Kimi Code CLI Error-Classification Vocabulary

## Overview

Kimi Code CLI is open source. Its strongest structured non-interactive error
surface is `kimi --wire`, a JSON-RPC 2.0 line protocol with error responses of
the form `error.code` plus `error.message`. The official wire-mode reference
documents the transport, while the tagged source defines the numeric error
constants and the prompt handler that selects them. This makes numeric codes
the most stable classification input; free-form provider messages are less
stable because Kimi forwards exception text from whichever chat provider is
configured.

Kimi also offers print mode with `--output-format=stream-json`. That format is
documented as JSONL `Message` records for assistant and tool output, not as a
structured terminal-error schema. In release `1.42.0`, terminal exceptions in
print mode are printed as text and classified coarsely by process exit code.
Consequently, Claudine's parser-backed classification vocabulary primarily
describes wire error responses, with the seeded message cascade retained for
provider exception text.

## Error Surfaces

### JSON-RPC Error Responses

Wire mode returns a JSON-RPC error object containing an integer `code`, a
string `message`, and optional `data`. Standard JSON-RPC failures use codes
`-32700` and `-32600` through `-32603`; Kimi-specific failures occupy the
`-32000` range. These are first-class protocol fields and exact code matching
should run before message substring matching.

The `1.42.0` source defines `INVALID_STATE`, `LLM_NOT_SET`,
`LLM_NOT_SUPPORTED`, `CHAT_PROVIDER_ERROR`, and `AUTH_EXPIRED` as `-32000`
through `-32004`. The immutable baseline records `CHAT_PROVIDER_ERROR` as
`-32005`; this research preserves that historical row and adds the tagged
release's current `-32003` value rather than rewriting provenance history.

### Message Text

Each JSON-RPC error also carries a message. Kimi authors stable copy for a few
local cases: an overlapping turn reports `An agent turn is already in
progress`, a missing LLM reports `LLM is not set`, and an expired OAuth session
reports `Authentication failed. Your login session may have expired. Please
run \"/login\" to sign in again.` The exact numeric code already classifies each
of these before the message cascade.

For non-OAuth `APIStatusError` and generic `ChatProviderError`, Kimi returns
`CHAT_PROVIDER_ERROR` with `str(e)`. The text can therefore contain vocabulary
chosen by an OpenAI-compatible, Anthropic-compatible, or other configured
backend. It is a diagnostic side-channel rather than a Kimi-owned message
contract. The seeded lowercase message needles remain useful compatibility
coverage, but this research does not manufacture new provider phrases.

### Print-Mode JSONL and Exit Codes

Print mode's `stream-json` writer emits assistant messages, tool results,
plans, and notifications. It ignores retry and interruption wire events for
JSON output coordination and does not define a JSON error frame. Its outer
exception handler prints terminal exception text and returns exit code `1` for
permanent failures or `75` for retryable connection, timeout, HTTP 429, and
HTTP 5xx failures.

Exit-code mapping and `StepRetry.status_code` recognition are detection
concerns owned by [`signals/kimi.md`](../signals/kimi.md). They can decide that
a record fires a rate-limit or retry signal; this document only classifies the
kind and summary after an error surface has been selected.

## Rate Limit, Quota, and Billing

The first message bucket checks `rate limit`, `quota`, `billing`, `api error`,
then `upstream`, and classifies the first match as `api_remote`. Its seeded
order is unchanged. In particular, quota or billing copy wins before later
configuration and interruption buckets.

Kimi's source independently classifies HTTP 429 as `rate_limit` for telemetry,
retries it, and includes its status code in `StepRetry`. The print-mode docs
also group quota exhaustion with permanent failures and 429 with retryable
failures. Those facts prove operational handling, not a stable terminal message
substring. The corresponding wire records and exit-code semantics are covered
by [`signals/kimi.md`](../signals/kimi.md), so they are not duplicated in the
frontmatter vocabulary.

## Authentication, Permission, and Configuration

`AUTH_EXPIRED` (`-32004`) maps to `configuration`. During a prompt, an OAuth
HTTP 401 produces that code and Kimi-authored re-login guidance. A non-OAuth
401 instead remains `CHAT_PROVIDER_ERROR`, because Kimi cannot prescribe its
own login flow for an external API-key provider. Exact code matching therefore
correctly distinguishes Kimi session expiry from a generic remote provider
failure.

The newly proposed `LLM_NOT_SET` (`-32001`) and `LLM_NOT_SUPPORTED` (`-32002`)
codes also map to `configuration`. The second message bucket retains `api key`,
`authentication`, `not authorized`, `permission denied`, `auth`, and `config`
in that order. `authentication` deliberately precedes the broader `auth`, and
the entire configuration bucket remains after `api_remote`, preserving the
seeded precedence.

No dedicated wire code for tool or filesystem permission denial was found.
Wire approval rejection is a human-policy response rather than proof of an
authentication or provider permission failure.

## Interruption, Cancellation, and Abort

The third message bucket checks `interrupt`, `cancel`, then `aborted`, mapping
all three to `interrupted`. The order is unchanged. In wire mode, cancellation
is normally a successful prompt response with `result.status: cancelled`, not
a JSON-RPC error response. Claudine's signal/parser layer owns recognition of
that record; the vocabulary remains relevant to free-form terminal exception
text such as print mode's `Interrupted by user`.

## Upstream, Server, and Provider Errors

`CHAT_PROVIDER_ERROR` maps to `api_remote`. Tagged source `1.42.0` assigns it
code `-32003` and uses it for non-OAuth `APIStatusError` and generic
`ChatProviderError`. The immutable seed's `-32005` mapping remains earlier in
the code cascade, while the current tagged value is appended as an additional
exact code. Because numeric matches are exact, the duplicate symbolic name
does not create substring shadowing.

Standard JSON-RPC parse, request, method, parameter, and internal errors map to
`agent_native`. `INVALID_STATE` (`-32000`) is also `agent_native`: its known
prompt-handler case describes overlapping agent turns, a Kimi protocol-state
failure rather than remote API behavior. These exact codes should win even if
their accompanying messages happen to contain seeded words such as `auth`,
`quota`, or `upstream`.

## Capacity and Overload

Kimi `1.42.0` treats HTTP 429, 500, 502, 503, and 504 as retryable. It emits a
`StepRetry` record with the exception class and optional HTTP status, so a 429
or 503 can be detected structurally. The source comments identify 429 as Too
Many Requests and 503 as Service Unavailable, but the terminal JSON-RPC
`CHAT_PROVIDER_ERROR` still forwards `str(e)` and defines no Kimi-owned
`overloaded`, `at capacity`, `resource_exhausted`, or equivalent phrase.

Accordingly, this proposal adds no capacity message needle. Treating every
503 as overload would collapse generic maintenance and upstream server
failures into capacity, while bare `429` or `503` substring matching would be
unsafe. The absence is recorded in `gaps`; structured retry/rate-limit
detection remains in [`signals/kimi.md`](../signals/kimi.md).

## Collisions and Precedence

The cascade's behavior is preserved exactly for seeded text. Representative
success prose demonstrates why no broader alternatives are added: `quota`
may appear in `/usage` output, `model` appears throughout normal configuration
and assistant prose, and `auth` appears in documentation or tool output.
Detection must first establish that the record is an error; the classifier
then applies these substrings only to the selected error message.

Within error text, the first `api_remote` bucket wins over configuration. Thus
`API error: authentication failed` classifies as `api_remote` because `api
error` is earlier, while `Authentication failed` alone classifies as
`configuration`. Interruption is last, so a provider message containing both
`rate limit` and `cancelled` remains `api_remote`. This may look surprising,
but it is the immutable seed's behavior contract.

Bare `rate`, `model`, `401`, `403`, `429`, and `503` are rejected as additions.
They can occur in successful usage displays, model identifiers, tool output,
ports, counters, and ordinary prose. Exact numeric matching is safe only in
the JSON-RPC `error.code` field, where negative protocol codes cannot collide
with HTTP status text.

## Quirks and Gaps

- The seed's `CHAT_PROVIDER_ERROR = -32005` differs from tagged Kimi CLI
  `1.42.0`, which defines `-32003`. Both are retained so research does not
  rewrite an observed baseline.

- Kimi's structured discriminator is the numeric JSON-RPC code, not a separate
  string error-kind field. This document therefore omits `kind_buckets`.

- Provider exception messages are intentionally passed through. Exact
  capacity, billing, static API-key, and permission-denial wording cannot be
  made universal across Kimi's configurable provider backends.

- Print-mode `stream-json` is structured for successful assistant/tool output
  but does not provide a source-defined terminal JSON error record in the
  inspected release.

## Changelog

Fresh first-run research document; `changes` is empty.

## Sources

- [Kimi wire-mode documentation](https://moonshotai.github.io/kimi-cli/en/customization/wire-mode.html)
- [Kimi print-mode documentation at `1.42.0`](https://github.com/MoonshotAI/kimi-cli/blob/1.42.0/docs/en/customization/print-mode.md#L1-L151)
- [Print-mode structured-output writer at `1.42.0`](https://github.com/MoonshotAI/kimi-cli/blob/1.42.0/src/kimi_cli/ui/print/visualize.py#L1-L188)
- [Print-mode terminal error and exit-code handling at `1.42.0`](https://github.com/MoonshotAI/kimi-cli/blob/1.42.0/src/kimi_cli/ui/print/__init__.py#L410-L449)
- [JSON-RPC error object and code definitions at `1.42.0`](https://github.com/MoonshotAI/kimi-cli/blob/1.42.0/src/kimi_cli/wire/jsonrpc.py#L31-L71)
- [Kimi-specific and standard error constants at `1.42.0`](https://github.com/MoonshotAI/kimi-cli/blob/1.42.0/src/kimi_cli/wire/jsonrpc.py#L230-L263)
- [Wire prompt-handler error projection at `1.42.0`](https://github.com/MoonshotAI/kimi-cli/blob/1.42.0/src/kimi_cli/wire/server.py#L645-L723)
- [API error classification at `1.42.0`](https://github.com/MoonshotAI/kimi-cli/blob/1.42.0/src/kimi_cli/soul/kimisoul.py#L100-L137)
- [Retryable status codes and `StepRetry` emission at `1.42.0`](https://github.com/MoonshotAI/kimi-cli/blob/1.42.0/src/kimi_cli/soul/kimisoul.py#L1255-L1267)
- [Kimi signal-detection research](../signals/kimi.md)

---
$schema: ./_schema.yaml
created: 2026-07-14
last_updated: 2026-07-14
agent: codex
model: default
msg_buckets:
  - kind: configuration
    needles:
      - text: sign in
        evidence: seed
      - text: sign-in
        evidence: seed
      - text: not logged in
        evidence: seed
      - text: authentication failed
        evidence: seed
      - text: authentication
        evidence: seed
      - text: unauthorized
        evidence: seed
      - text: '401'
        evidence: seed
      - text: '403'
        evidence: seed
  - kind: api_remote
    needles:
      - text: rate limit
        evidence: seed
      - text: quota
        evidence: seed
      - text: exhausted
        evidence: seed
      - text: out of credits
        evidence: seed
      - text: overloaded
        evidence: seed
      - text: '503'
        evidence: seed
      - text: resource_exhausted
        evidence: seed
  - kind: interrupted
    needles:
      - text: abort
        evidence: seed
      - text: cancel
        evidence: seed
      - text: interrupt
        evidence: seed
gaps:
  - area: published-error-contract
    notes: >-
      No official error-specific documentation was found. The public repository
      at tag 1.1.2 is not source-complete and exposes no implementation error
      enums, message constants, or structured error schema, so docs is omitted.
  - area: structured-error-kinds
    notes: >-
      Print-mode JSON has status and error fields, but no documented discriminator
      vocabulary was found. Antigravity remains a message-only classifier and has
      no kind_buckets.
  - area: numeric-code-contract
    notes: >-
      No distinct numeric wire-code field or published code enum was confirmed.
      Seeded 401, 403, and 503 values remain message substrings, not code_buckets.
  - area: capacity-and-overload-copy
    notes: >-
      The changelog confirms transient generation errors and retries, but no exact
      at capacity, Selected model is at capacity, overloaded, RESOURCE_EXHAUSTED,
      HTTP 429, or HTTP 503 provider message was published. The sticky overload
      needles are preserved without guessing additional copy.
  - area: rate-limit-quota-and-billing-copy
    notes: >-
      Official materials document quota and credits UI surfaces but do not publish
      terminal error payloads for throttling, exhausted quota, billing, or funds.
  - area: permission-error-copy
    notes: >-
      Official permission documentation defines allow, ask, deny, and force_ask
      decisions, but no stable non-interactive denial message was found.
changes: []
requires_claudine_update: false
---

# Antigravity CLI Error-Classification Vocabulary

## Overview

Antigravity CLI provides a non-interactive print mode and accepts a hidden
`--output-format json` option. The observed result is one JSON object with
`conversation_id`, `status`, `response`, `error`, `duration_seconds`,
`num_turns`, and `usage`. Failures are therefore available as free-form text in
`error`; no stream of JSON error frames, structured error-kind discriminator,
or separate numeric error code has been established. Plain print mode can also
write a human-readable error to stderr and return a nonzero exit status when a
server-side request fails.

The repository is public and tagged, but it is not an open-source implementation
of the CLI. Tag `1.1.2` contains a README, changelog, statusline/title examples,
and media, with no Go source, error enums, protocol definitions, or message
constants. Official documentation covers general operation, authentication,
permissions, and configuration, but no error contract was found. Consequently,
the frontmatter retains the immutable Phase-A seed as seed evidence and does not
promote plausible Google API vocabulary into source-attested additions.

## Error Surfaces

### Print-Mode JSON

`agy --print "<prompt>" --output-format json` returns a single result object.
The `status` field indicates overall outcome and `error` carries error text; the
available public material does not define allowed status values or the type and
format of `error`. This is the structured wrapper surface consumed by Claudine,
but its error payload is message-only rather than a first-class error taxonomy.

Because the result is a terminal object rather than an event stream, there is no
published JSON error-frame discriminator to populate `kind_buckets`. Treating
`status` as an error kind would invent semantics that the provider has not
documented.

### Plain Print Output and Exit Status

Plain `--print` mode emits human-readable output. The `1.1.2` changelog says a
server-side request failure now writes its error to stderr and returns a nonzero
exit code; earlier behavior could silently exit successfully with empty output.
This establishes the channel and failure status, but not any exact provider
message vocabulary.

Exit-code mapping and payload records that fire authentication or other signals
belong to [`signals/antigravity.md`](../signals/antigravity.md). In particular,
that document records captured unauthenticated print and model-listing exits;
those detection records are not duplicated as frontmatter needles here.

### Diagnostic App Logs

The CLI accepts `--log-file` and can emit glog-style diagnostic text. Existing
signals research captured authentication diagnostics there, but Claudine does
not ingest this side-channel as the print-mode error message. App-log record
detection remains owned by [`signals/antigravity.md`](../signals/antigravity.md),
not this rendering cascade.

### Numeric Codes

No distinct numeric code field or published error-code enum was found. The
seeded `401`, `403`, and `503` needles are ordinary substrings checked inside the
free-form message. `code_buckets` would incorrectly imply exact typed-code
matching and is therefore omitted.

## Rate Limit, Quota, and Billing

The `api_remote` bucket retains `rate limit`, `quota`, `exhausted`, and `out of
credits` in their seeded positions. They classify matched message text as
`api_remote`. Official materials establish quota and credit management as
product surfaces: the changelog describes Models & Quota, real-time quota
reload, G1 credits, and quota usage in the statusline. They do not publish the
exact terminal failure strings produced when those resources are depleted.

No `billing`, `insufficient funds`, bare `rate`, or HTTP `429` needle is added.
Those strings are either unattested on Antigravity's print-error surface or too
broad for safe substring matching. Whether a particular payload fires a
rate-limit, usage-cap, or no-funds signal is separately tracked in
[`signals/antigravity.md`](../signals/antigravity.md).

## Authentication, Permission, and Configuration

The first bucket preserves `sign in`, `sign-in`, `not logged in`,
`authentication failed`, `authentication`, `unauthorized`, `401`, and `403` in
that order, all mapping to `configuration`. The README documents system-keyring
authentication with Google Sign-In fallback, while the official permissions
guide defines `allow`, `deny`, `ask`, and `force_ask` decisions. Neither source
publishes a stable print-mode denial message or typed authentication error enum.

This bucket intentionally precedes remote failures. A mixed message such as an
authentication failure while refreshing quota resolves to `configuration`
rather than `api_remote`, preserving the established precedence quirk. The
specific captured unauthenticated exit records remain detection evidence in the
signals document and are not recast as new rendering vocabulary here.

## Interruption, Cancellation, and Abort

The final bucket preserves `abort`, `cancel`, then `interrupt`, classifying a
matching message as `interrupted`. The changelog documents Ctrl+C cancellation
of active streaming operations and separately mentions cancellation behavior,
but it does not publish the terminal error text or JSON `error` value produced
by an interrupted print run. The seed therefore remains the only vocabulary.

Interruption stays last so a message containing both a remote cause and
subsequent cancellation remains `api_remote`, while an otherwise unqualified
abort or cancellation resolves to `interrupted`.

## Upstream, Server, and Provider Errors

The seeded remote bucket covers `503` and the broader quota/exhaustion family.
The `1.1.2` changelog confirms that server-side request failures reach print-mode
stderr, and `1.0.16` documents automatic client-side retries for transient model
generation errors. Neither entry supplies exact message constants. Broad
needles such as `server`, `request failed`, `error`, `transient`, or `model` are
therefore excluded: each can occur in local diagnostics or ordinary successful
prose and would overclassify unrelated text.

No source-attested network, timeout, unavailable, internal-error, or provider
phrase could be added without a captured print-mode fixture or implementation
source.

## Capacity and Overload

The sticky remote bucket retains `overloaded`, `503`, and
`resource_exhausted`. These close common capacity-shaped matches at the seeded
behavior level, but the available Antigravity sources do not attest that the CLI
actually emits any of them. In particular, no exact `at capacity` or `Selected
model is at capacity` phrase was found.

The changelog's reference to retries for transient generation errors proves a
failure family exists, not its message spelling. Adding `at capacity`, `429`,
`service unavailable`, or another inferred Google API phrase would therefore
manufacture provenance. The missing provider-specific capacity copy is recorded
as a frontmatter gap.

## Collisions and Precedence

Classification ASCII-lowercases the message and walks buckets in order, so the
winning precedence is `configuration` before `api_remote` before `interrupted`.
All retained needles are lowercase. Several collision properties follow:

| Needle or family | Collision assessment | Winning behavior |
| --- | --- | --- |
| `authentication failed` / `authentication` | The shorter seed shadows the longer phrase only after the longer row has already been tested; both produce the same kind. | `configuration` |
| `sign in` / `sign-in` | Separate spellings avoid a broad `sign` match against ordinary prose. | `configuration` |
| `401`, `403`, `503` | Bare numbers can occur in filenames, counts, IDs, successful HTTP discussion, or tool output. They are retained only because seeds are immutable. | `401`/`403` are `configuration`; `503` is `api_remote` |
| `quota` / `exhausted` | These can appear in successful usage reports or discussion rather than an error. Error-surface selection must happen before this classifier. | `api_remote` |
| `abort`, `cancel`, `interrupt` | Broad stems can match explanatory prose, but their last position lets more actionable auth or remote causes win first. | `interrupted` if no earlier needle matches |
| `model`, `auth`, `rate` | These broad candidates commonly occur in successful status, setup, and model-selection text. | Omitted |

An error mentioning both sign-in and quota is classified as `configuration`;
one mentioning `503` and cancellation is `api_remote`. This ordering is part of
the vocabulary contract, not presentation preference.

## Quirks and Gaps

Antigravity's JSON output flag is hidden from ordinary help, and its schema is
not documented. The wrapper must select the error surface before keyword
classification; running this substring cascade over `response`, statusline
JSON, tool output, or successful quota prose would amplify the known broad-seed
collisions.

The public GitHub repository must not be described as implementation source.
Its tagged changelog is useful for proving channels and behavior changes, but it
cannot attest error constants or enums. No official error-specific documentation,
structured error kinds, numeric code contract, stable permission-denial copy,
or exact rate-limit/capacity copy was located. These unanswered surfaces are
retained in `gaps` rather than filled with Gemini API assumptions.

## Changelog

This is the initial research document; `changes` is empty.

## Sources

- [Antigravity CLI README at `1.1.2`](https://github.com/google-antigravity/antigravity-cli/blob/1.1.2/README.md#L1-L85)
- [Antigravity CLI changelog at `1.1.2`](https://github.com/google-antigravity/antigravity-cli/blob/1.1.2/CHANGELOG.md#L1-L265)
- [Antigravity CLI permissions documentation](https://antigravity.google/docs/cli-permissions)
- [Antigravity CLI settings and rendering documentation](https://antigravity.google/docs/cli-settings)
- [Antigravity CLI overview](https://antigravity.google/docs/cli-overview)
- [Antigravity CLI signal-detection research](../signals/antigravity.md)

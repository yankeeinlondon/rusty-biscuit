---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
status: draft-spec
reviewed: false
clarified: false
implemented: false
human_review: false
message_to_agent: ''
parent: ../../reviews/2026-07-31-macro-review/spec.md
depends_on:
    - 2026-10-11-schematic-generation-integrity
refresh_required: true
related: []
---

# Schematic runtime and response envelope

## Problem

Every generated Schematic client (`schematic-schema`, one module per API)
sends requests through code that the generator emits as template text and that
is too thin for the consumers that depend on it. Verified facts, from the
parent review (`schematic/reviews/2026-07-31-macro-review/spec.md`, called
"the parent" below) unless marked otherwise:

- **A caller can ask for the wrong response type.** The generic
  `request<T>` compiles with any `T`
  (`client.request::<WrongType>(ListModelsRequest::default())`). The shared
  trait `EndpointSpec` already has an associated `Response` type with 550
  generated impls, but no send method uses it (parent section 12, "Related
  fact for D3"). Verified in this tree: `EndpointSpec` is declared in
  `schematic/schema/src/shared.rs`.
- **Errors discard evidence.** `SchematicError::ApiError` carries only
  `status` and `body`; response headers (for example `Retry-After`,
  `x-ratelimit-remaining`) are lost on a non-2xx reply and are not exposed on
  success either. Pagination needs headers on success (gap A1).
- **No credential evidence and no credential policy.** The client resolves
  environment variables internally, never reports which one (or none) it
  used, and has no per-call credential (gaps A3, A4). sniff works around this
  with a hidden anonymous retry (gap A5).
- **No origin boundary.** Nothing checks that a request or a followed
  redirect stays on the configured origin before a credential is attached
  (gaps A6, A7). Default reqwest redirect behavior applies.
- **No shared deadline.** There is a fixed 30 s per-request timeout; an
  operation that spans several requests cannot share one budget (gap A8).
- **Client injection defeats any policy.** The generator emits `with_client`
  and `with_client_and_base_url`, so a caller-built `reqwest::Client` (whose
  redirect behavior is fixed at build time) can bypass any origin rule.
  Verified in this tree: `with_client_and_base_url` is called at
  `homelab/lib/src/eversolo.rs:62` and `homelab/lib/src/samsung_tv.rs:120`;
  the Samsung site also calls `danger_accept_invalid_certs(true)`
  (`samsung_tv.rs:116`) for a self-signed device certificate.
- **The logic is template text.** The runtime that must be correct is emitted
  by `schematic-gen` into `schematic-schema` and is therefore tested only
  through regenerated text.

This spec is the large breaking change of the whole body of work (parent D50,
spec 2 of 8): it moves the runtime into a hand-written package, introduces the
response envelope and typed send, and migrates **every** dependent crate in the
same run.

## Goal and non-goals

### Goal

When this spec's plan is complete and reviewed:

1. A new hand-written package `schematic-runtime` holds the shared runtime
   (envelope, `TransportError`, typed send, credentials, managed HTTP builder,
   cross-origin and redirect logic, shared deadline, redaction, `send_raw`).
2. Every generated request type declares its response type, and the only
   public send path is typed by it. The free generic `request<T>` is gone.
3. Every send returns the `Response<T, E>` envelope (or a `TransportError`
   when no HTTP response head arrived), with the metadata and redaction rules
   below.
4. Credentials follow the D11 precedence and are reported, never retried.
5. Generated clients have no injected HTTP client; a Schematic-managed
   builder replaces it; the cross-origin policy therefore always holds.
6. All 18 API definitions carry the `cross_origin` field (default
   `same_origin`).
7. All dependents (sniff, biscuit-speaks, model-citizen, and the five homelab
   crates; eight crates) build and pass their tests against the new surface.

### Ownership decided for this spec

Boundaries between this spec and its neighbors. They are assignments made
for the eight-spec split; the author confirms them at review 1.

- **Credentials, all of it, including `ApiKeyParam`.** This spec owns
  credential resolution, the credential type, and the `ApiKeyParam` handling.
  It **removes** spec 1's temporary `with_api_key_param` and the
  generator-local environment scan, and migrates their users (R33).
- **The hook mechanism and the `ResponseKind` guard disposition.** The
  mechanism is a marker trait implemented by the codecs that support hooks;
  spec 5 extends it to the stream and GraphQL codecs. `ErrorKind` is
  `#[non_exhaustive]`, so spec 5 adds `UnexpectedContentType` without a
  breaking change (PD17).
- **`request_id_header` and `secret_names`**: semantics and the API-level
  fields. Spec 4a owns only builder sugar and the per-parameter `.secret()`
  marker.
- **The area's justfile and its gates**: adding `schematic-runtime` to the
  recipes (R37) and naming `test-schema` as the gate for generated-crate
  mock-server tests.

### Non-goals (other specs own these)

- Pagination model, pagers, `collect_all`, `with_session` (spec 3). This spec
  supplies only the primitives spec 3 needs (page number slot in metadata,
  link-origin check, deadline handle, `InvalidArgument` kind).
- Builders, typed registration, per-endpoint cross-origin override, the
  `oauth_scopes` / `auth_policy` choice (specs 4a and 4b). **No endpoint literal
  (about 519; openai 265) is edited in this spec** (parent D42, D30).
- Response model v2 (success and error sets, selectors, stream end
  conditions, error mapping), GraphQL primitive and codec, SSE/NDJSON codecs
  (spec 5). The `Decode` trait shape defined here must *allow* them (R25), but
  this spec ships only the JSON, text, binary, and empty codecs.
- Generation integrity, partial generation, the isolated compile check, the
  query-parameter split, and request-side features (spec 1, a dependency).
- WebSocket (spec 6) and fidelity work (spec 7). The session-close grace
  after a deadline expires belongs to spec 3.
- sniff retiring `FocusedProviderClient` (tracked by sniff's own spec,
  starts after spec 3). This spec migrates sniff's *existing uses of the
  generated clients* to the new surface only.
- A generic middleware / hooks mechanism (parent D31 "later hooks
  mechanism"). Only the two existing hooks are preserved (R29).
- Per-endpoint wrapper methods. The parent rules (D53) that none are
  generated (an accepted gap); no later spec is promised them.

## Background: decisions this spec implements

Each decision is restated so the implementer need not open the parent. The
parent path is `schematic/reviews/2026-07-31-macro-review/spec.md`, section 15
for D-numbers. Where a later decision amended an earlier one, only the current
wording is given.

**D3 (typed request only; amended by D7).** Each generated request type
declares its response type; its primary send method is typed by it. The free
generic `request<T>` is removed from the public surface. The send returns the
`Response<T, E>` envelope (D7), not the bare type. The generator must preserve
concrete endpoint types rather than erase them.

**D7 (envelope).** Every send returns `Result<Response<T, E>, TransportError>`.
`Response<T, E>` is a struct with public metadata fields plus an inner enum
`Success(T)` or `Failure(E-bearing error)`, read through `data()` and
`error()` (both `Option`) plus `into_result()`. `T` is the endpoint's declared
success type and `E` its declared failure type. Metadata is present in both
outcomes. Any HTTP status, including a malformed or undeclared error body, is
reported inside the envelope with the raw body retained. Home crate: D39.

**D21 + D48 (envelope metadata).** The envelope carries: status; HTTP method;
full URL with secrets in the query redacted; request headers (redacted);
response headers; query parameters; auth method used; elapsed time; page
number (only when the call is part of a paginated operation); the provider's
request id (only when the definition names that header, via
`request_id_header("...")`; otherwise `None`).

**D48 (details).** An endpoint with no declared error body uses an
uninhabited placeholder error type, so a `Declared` error cannot exist; error
replies arrive as kind `Undeclared` with status, headers, and raw body.
`AuthMethod = PerCall | ClientLevel | EnvVar(name) | Anonymous`, never a
value. Built-in secret names: headers `Authorization`, `Proxy-Authorization`,
`Cookie`, `Set-Cookie`, `X-Api-Key`, `X-Auth-Token`, `Api-Key`; query
parameters `api_key`, `apikey`, `access_token`, `token`, `key`, `signature`.
Redacted values show `[redacted]`.

**D40 (secret marking).** Secrets are marked automatically from auth
strategies (for example an `ApiKeyParam`'s name), by an explicit `.secret()`
marker on any header or parameter in the definition, and by the built-in list.
The redaction set is the union of the three.

**D22 (narrowed promise).** Only Schematic's *stored metadata* and its
`Display` output are guaranteed free of credentials. A provider's decoded error
body is printed as received by `Debug`; a provider that echoes a credential in
an error message can leak it through `Debug` logging. The raw body stays in the
error as evidence and is not altered. The limitation must be stated in the
generated crate's docs for the error types and in the `schematic/docs/` topic
page for the envelope.

**D23 (URL in network-error text).** Network-error text keeps the URL. The
text is composed by Schematic from (1) its own redacted URL (secret query
values show `[redacted]`) and (2) the HTTP library's message with the
library's raw URL removed. The library's own error text is never stored or
shown. A sentinel-key test must prove a query-string key never appears in any
error text. (Requires the query/path split delivered by spec 1.)

**D24 (failures before anything is sent).** These are the outer
`TransportError` with specific kinds (for example `MissingCredential`,
`CrossOriginRejected`, `InvalidArgument`, `Timeout`) and still carry request
metadata. Named cases: missing credential, page size above the declared
maximum, origin rejected, invalid argument. sniff's "auth required" case moves
to the `TransportError` kind.

**D25 (undecodable 2xx).** A 2xx reply that cannot be decoded is a Failure
inside the envelope with the real status kept, kind `MalformedSuccess`, raw
body and headers kept, plus an `is_decode_failure()` helper. (A corporate
proxy's HTML login page returned with status 200 is the example.)

**D26 (cut-off body; governing rule).** A reply whose body is cut off after
the headers arrived is a Failure inside the envelope, kind `BodyInterrupted`,
with status, headers, and the partial bytes, plus `is_interrupted()`.
**Governing rule: an envelope exists if and only if an HTTP response head
(status line and headers) was received. The outer `TransportError` means no
response head arrived.**

**D32 (refused redirect or link).** One test: did this call receive an HTTP
response? A redirect refused by the policy (a 3xx was received) is a Failure
inside the envelope, kind `RedirectRefused`, with the redirect status and
`Location` visible. A refused next-page link (no request for it was started)
is the outer `TransportError`, kind `CrossOriginRejected`.

**D11 (credentials).** Callers may pass credentials programmatically at (1)
client level (set once, used by later requests) and (2) per call (overrides
client level). Precedence: per call, then client level, then environment
policy, then anonymous. Environment policy applies only when nothing was set
programmatically: the definition's ordered list of environment variable names
is scanned **once** before sending, the first non-empty variable is selected
(an empty variable counts as unset), and exactly that one credential is sent;
a caller may supply their own ordered list at client level, which replaces the
definition's order. Single attempt: no chain of keys, no retry, and no silent
anonymous retry if rejected; a consumer may make a separate anonymous call.
Every envelope reports the auth method (never the value). Credentials are
resolved once per request, so the report matches what was sent.

**D28 + D30 (cross-origin policy).** The policy for requests to a different
origin is authored in the API definition (a `schematic-define` primitive) and
governs both redirects and followed next-page links. Five values:

| Value | Meaning |
|---|---|
| `same_origin` | Follow only to the same origin (scheme, host, effective port). Anything else is refused. |
| `allow_origins([...])` | Follow to the listed exact origins; credentials are also sent to those. No wildcards, no domain matching. |
| `allow_without_credentials` | Follow to any origin; strip credentials when the origin differs. |
| `allow_any` | Follow anywhere with credentials. Risky; requires an explicit, clearly named opt-in. |
| `deny` | No redirects and no followed cross-links. |

Default when the definition says nothing: `same_origin`. A caller may tighten
the policy (for example to `deny`) but never loosen it beyond the definition.
Credentials attach only to allowed origins. Because policies require
Schematic to follow redirects itself, the HTTP client is built with redirects
disabled and Schematic follows them hop by hop: hop limit, per-hop origin
re-check, credential stripping when leaving the origin, and method/body rules
for 301, 302, 303, 307, 308. Landing 1 (specs 1-3) holds the runtime and the
API-level `cross_origin` field on the 18 API definitions; the per-endpoint
override is spec 4a, so **in this spec an endpoint cannot override the policy**.

**D31 (no injected HTTP client; amends D4, D12).** `with_client(reqwest::Client)`
and `with_client_and_base_url` are removed. A Schematic-managed builder covers
proxy, TLS root certificates, timeouts, user agent, default headers, a
shareable connection pool, and a base URL for tests. Author ruling after the
evidence note: the builder also gains an explicitly named, opt-in "accept
invalid certificates" setting for local devices (the Samsung TV with a
self-signed certificate on a home network). The two homelab call sites are
migrated in this spec.

**D13 (shared deadline).** Opt-in. A caller sets one overall deadline for an
operation (all requests it spans) on a monotonic clock. Each request gets the
remaining time. Once spent, no further request is sent and a `DeadlineExceeded`
error reports the completed requests and elapsed time. Dropping the call
cancels in-flight work and spawns no detached tasks. The 30 s per-request
default is unchanged for callers who do not opt in.

**D39 (runtime home).** A new hand-written package (working name
`schematic-runtime`) holds the envelope and metadata, `TransportError`,
redirect-following and cross-origin logic, the shared deadline, the pager (spec
3), and redaction. Generated clients depend on it and stay thin. Author-stated
cost: one more package and coordinated versions with the generator.

**D45 (`send_raw`).** A clearly named `send_raw` returns status, headers, and
bytes (with a JSON convenience) through the same credentials, cross-origin
rule, deadline, and redaction. Typed sends stay strict. Whether the erasing
request enum remains is left to planning.

**D49 item 5 (`ResponseKind` guard).** The generated client's `ResponseKind`
runtime guard follows the new response model in landing 1: kept, changed, or
removed as that model requires. The behavior on an unexpected response kind
must be covered by a test under the new model, or the guard is gone and the
test list says so.

**D50 (plan structure).** This is spec 2 of 8. Each plan is implemented end to
end without a human in the loop; the tree must be green (macOS, Linux, native
Windows, WSL2; every dependent builds and passes) when the plan ends; the
terminal state is "implementation complete, ready for review"; only the author
moves a spec to `_completed`.

**D53 (no per-endpoint wrappers).** The parent rules that no per-endpoint
wrapper methods are generated; this is an accepted gap.

**D20 item 2.** After each landing, every dependent crate builds and passes its
tests: sniff, biscuit-speaks, model-citizen, and the five homelab crates
(`homelab/lib`, `eversolo-integration`, `arcam-amp-integration`,
`sony-receiver-integration`, `unfolded-integration-helper`).

### Spike evidence (reported by the parent, not re-run by this spec)

Two throwaway prototypes under
`schematic/reviews/2026-07-31-macro-review/spikes/` informed these shapes:

- `typed-send/`: 28 fixture tests, 5 ported sniff call-site tests, 8
  compile-fail cases. One generic `send` covered every fixture including
  streaming and GraphQL (no `send_stream`). Wrong types failed to compile
  with readable errors. Compile cost: today's `EndpointSpec` x550 11.54 s;
  new trait (the spike named it `EndpointSpec`; here `EndpointSpec`) x550 11.70 s; x550 plus 550 `send` calls 18.54 s (about 7 s
  extra), so **do not generate a wrapper method per endpoint**. Incremental
  rebuild cost was not measured. macOS only; HTTP/2, Linux, Windows, WSL2, the
  generator, and real providers were untested.
- `pager-surface/`: deadline (150 ms pages, 400 ms budget: third request cut
  off, error reported 2 completed pages and elapsed, each request received the
  remaining budget, nothing sent afterwards); drop-cancel on HTTP/1.1 only
  (server saw the connection close, live tasks 1 to 0); host policy (link
  differing in port, scheme, or host rejected with zero requests to the other
  server; a declared extra origin followed). Pager content belongs to spec 3.

## Requirements

Each requirement is testable. "L1" means `just test` in the named package;
"L2" means `just test-l2` (must never take window focus; this spec expects no
L2 tests). Mock servers are local; nothing contacts a real provider. "Sentinel
token" means a distinctive fake credential value that must be absent where
stated.

### Package and typed send

**R1. Runtime package exists and owns the shared runtime (D39).** A hand-written
package `schematic-runtime` exists; `schematic-schema` depends on it; the
generator no longer emits the envelope, `TransportError`, redirect logic,
deadline, or redaction as template text into `schematic-schema` (a search of
generator output for those items finds only references to the runtime).
Validated by: L1 in `schematic-gen` (a generation test asserts the generated
`shared.rs` contains no definition of these types) and L1 in
`schematic-runtime` (the types exist and are tested there); workspace
package-list check (`sniff repo packages` lists the new package).

**R2. Typed send; no free `request<T>` (D3).** Each generated request type
implements the runtime's `EndpointSpec` trait (declared success type, failure type,
decoder, endpoint id). The public API of the generated crate and of
`schematic-runtime` exposes no generic `request<T>` that takes a caller-chosen
response type. A call that names a response type other than the declared one
fails to compile. Validated by: compile-fail (trybuild or `compile_fail`
doctests) in `schematic-runtime` covering a wrong success type, a foreign
request type, an impl with a missing associated type, and the absence of
`request<T>`; plus one compile-fail in `schematic-schema`'s test set (or the
isolated generated crate, whichever spec 1 established) showing `request::<T>`
is absent.

**R3. Envelope shape (D7).** `Response<T, E>` has public metadata and an inner
`Success(T)` / `Failure(ApiError<E>)` enum. `data()` returns `Some(&T)` on
success and `None` on failure; `error()` the reverse; `into_result()` agrees.
`into_parts()` returns the metadata with the outcome (so success metadata is
not lost). Validated by: L1 in `schematic-runtime` against a wiremock server
for one success and one failure request.

**R4. Metadata fields (D21, D48).** For a success, a failure, and a
transport-failure result, the metadata holds: status (not on transport
failure), method, redacted URL, redacted request headers, response headers
(not on transport failure), query parameters, `AuthMethod`, and `elapsed`.
`page` is `None` unless the call is marked part of a paginated operation (the
slot exists; spec 3 sets it). `request_id` is `Some(value)` when the
definition's `request_id_header` is present in the response, else `None`; with
no such definition field it is always `None`. Validated by: L1 in
`schematic-runtime` and an end-to-end L1 in `schematic-schema`'s tests for the
request-id case using a definition fixture that sets the field.

**R5. Redaction in stored metadata and `Display` (D21, D22, D40, D48).** With a
sentinel in a built-in secret header, a built-in secret query parameter, an
`ApiKeyParam` name, and a name marked secret in the definition, the sentinel is
absent from every stored envelope field, from the `Display` of the envelope,
the error, and the transport error, in the success, declared-failure,
undecodable-success, and transport-failure cases; the secret values show
`[redacted]`; a non-secret header or parameter (for example `page=2`) keeps
its real value. The built-in names are exactly the D48 lists. Real values are
never stored (inspect fields, not only printed text). Validated by: L1 in
`schematic-runtime` with wiremock; includes a table test over every built-in
name.

**R6. The `Debug` limitation is documented, not denied (D22).** A mock answers
401 with the sentinel in the body. The test asserts the raw body keeps it
(evidence preserved) and that stored metadata and `Display` do not contain it,
and makes no assertion about `Debug` of the decoded body. The limitation text
appears in the runtime's error-type rustdoc and in the `schematic/docs/` topic
page. Validated by: L1 in `schematic-runtime`; a grep-style L1 or doc check
that the text exists in both places.

**R7. Network-error text keeps a redacted URL (D23).** With a secret query
parameter whose value is the sentinel, (a) a connection to a closed port and
(b) a mock that stalls past the timeout each produce a `TransportError` whose
`Display` and `Debug` contain no sentinel, show the URL with the secret value
as `[redacted]`, and still show a non-secret parameter. The HTTP library's raw
URL appears nowhere in stored fields or text. Validated by: L1 in
`schematic-runtime` (closed-port and stalled-server cases).

### Failure placement and kinds

**R8. Governing rule (D26, D32).** If a call received an HTTP response head the
result is `Ok(Response)`; if not it is `Err(TransportError)`. Validated by: L1
table test in `schematic-runtime`: connection refused gives `TransportError`;
a 500 with an empty body, a 200 with an undecodable body, and a body cut off
after headers each give an envelope.

**R9. Pre-send failures are `TransportError` with request metadata (D24).**
Missing credential, an origin rejected by policy, and an invalid argument each
return `Err(TransportError)` with its specific kind, carrying redacted request
metadata and `elapsed`, and the mock records zero requests. Validated by: L1 in
`schematic-runtime`, one case per pre-send kind in PD7 (the page-size-above-max
kind is exercised through the `InvalidArgument` constructor spec 3 will call).

**R10. `MalformedSuccess` and `is_decode_failure()` (D25).** A mock answering a
JSON endpoint with status 200, `Content-Type: text/html`, and an HTML body
yields a Failure inside the envelope: status 200, kind `MalformedSuccess`, raw
body and headers kept, `is_decode_failure()` true, `into_result()` `Err`.
`is_decode_failure()` is false for a failure of any other kind and for a
success. Validated by: L1 in `schematic-runtime`.

**R11. `BodyInterrupted` and `is_interrupted()` (D26).** A raw-socket mock sends
a 200 with `Content-Length` larger than the bytes written, writes part of the
body, and closes. The result is an envelope Failure of kind `BodyInterrupted`
with status and headers present and partial bytes equal to what was sent;
`is_interrupted()` true and false for every other kind. Validated by: L1 in
`schematic-runtime` using a `tokio::net::TcpListener` mock (wiremock cannot
cut a body). Platform behavior of an abrupt close may differ on Windows
(unverified); see Risks.

**R12. `Undeclared` and the uninhabited placeholder (D48).** For an endpoint
with no declared error body, a `Declared` error value cannot be constructed
(compile-fail test on the placeholder type), and a 4xx or 5xx reply arrives as
kind `Undeclared` with status, headers, and raw body. Validated by:
compile-fail plus L1, both in `schematic-runtime`.

**R13. Declared failures decode on demand (D6 groundwork, D7).** For a
fixture endpoint whose failure type is a real struct, a declared 4xx body
decodes into `Declared(E)` with status and headers; an undecodable body for a
declared status yields kind `MalformedError` with the raw body kept. (No
shipped definition declares an error body until spec 5; this requirement
proves the trait path with a runtime-test fixture.) Validated by: L1 in
`schematic-runtime`.

### Credentials

**R14. Precedence (D11).** With sentinel tokens present at per-call, client
level, an environment variable, and nowhere, the mock sees exactly one
credential per request, in the order per-call, client-level, environment,
anonymous; removing the winner makes the next one win. Validated by: L1 in
`schematic-runtime` (generic) and one L1 through a generated client in
`schematic-schema`'s test set.

**R15. Environment scan (D11).** With several variables listed, the first
non-empty one is sent (an empty one is skipped); a caller-supplied ordered
list at client level replaces the definition's order. Validated by: L1 with
process-environment isolation (use the repo's serial-test pattern, since tests
mutate the environment).

**R16. Resolved once; single attempt; reporting (D11, D48).** Changing the
environment variable between resolution and the response does not change the
reported auth method; when the mock answers 401 it has received exactly one
request (no second request with another key or none); the reported auth method
is exactly one of `PerCall`, `ClientLevel`, `EnvVar(name)`, `Anonymous`, and
the sentinel is absent from envelope, error, and `Display`. Validated by: L1 in
`schematic-runtime`.

### Managed builder and client injection

**R17. No injected client (D31).** The generated clients have no constructor
that accepts a `reqwest::Client`; `with_client` and `with_client_and_base_url`
are absent from the public API of `schematic-schema` and `schematic-runtime`.
Validated by: compile-fail test; and a generation test asserting the generator
no longer emits those names.

**R18. Builder surface (D31).** The managed builder supports: base URL (for
tests), per-request timeout (default 30 s), connect timeout, proxy, extra TLS
root certificates, user agent, default headers, a shareable connection pool,
and a named opt-in accept-invalid-certificates setting. Each is observed by a
local mock: a proxy setting routes through a mock proxy; a custom root
certificate lets a local TLS mock be reached and its absence fails; the
timeouts, user agent, and default headers are observed by the mock; two
clients built from one pool share a connection (the mock counts accepted
connections); a test base URL redirects calls to the mock; accepting invalid
certificates reaches a self-signed local TLS mock and the default does not.
Validated by: L1 in `schematic-runtime` (local TLS and proxy mocks are
dev-dependencies; see PD33).

**R19. No path produces a client that follows redirects by itself.** For every
constructor and builder combination, the cross-origin tests (R22 to R26) pass,
and the underlying HTTP client is built with redirects disabled. Validated by:
L1 in `schematic-runtime` (a test builds clients through every public
constructor and asserts a 302 to another origin is not followed unless the
policy allows).

### Cross-origin

**R20. API-level field on all 18 definitions (D28, D30).** `RestApi` gains a
`cross_origin` field of a `schematic-define` type with the five values; all 18
API definitions in `schematic-definitions` set it; a definition that sets
nothing explicit behaves as `same_origin`; the generated client reads it from
the definition. In this spec an endpoint cannot override it. Validated by: L1 in
`schematic-define` and `schematic-definitions` (all 18 carry the field); L1 in
`schematic-gen` (generated code carries the policy); and a positive test of
the current public surface of `schematic-define` (the endpoint type exposes
no cross-origin setter, and no GraphQL primitive is exported). That surface
test is frozen to today's API: specs 4a and 5 retire or update it when they
add the per-endpoint override and the GraphQL primitive (see Refresh notes
and the Human review agenda). It is deliberately not a compile-fail test,
which would fail for the wrong reason once those specs land.

**R21. Allowed-origin rules (D28).** An origin is the exact (scheme, host,
effective port). `same_origin` allows only the base origin. `allow_origins`
allows the base plus the listed origins; a sibling subdomain and a different
port are not allowed. `deny` follows no redirect and no cross-origin link.
Validated by: L1 table test in `schematic-runtime` over base origin A and
origins B, B-sibling-subdomain, B-different-port, scheme change, default-port
spelling (`:443` explicit versus implicit).

**R22. Redirect matrix (D28).** Run once with two mock origins A (base) and B,
with B reached by a redirect from A:

- `same_origin`: B receives zero requests; the result is an envelope Failure
  kind `RedirectRefused` with the 3xx status and `Location` visible in the
  response headers (D32); a same-origin redirect is followed.
- `allow_origins([B])`: B is followed and receives the credential; an unlisted
  origin receives zero requests.
- `allow_without_credentials`: B is followed and receives no credential in any
  form (`Authorization` header, `Cookie`, and a query-string API key); a
  same-origin hop keeps its credential.
- `allow_any`: B is followed and receives the credential. The opt-in is
  explicitly and clearly named (compile-level check that no default or typo
  reaches it).
- `deny`: no redirect is followed (zero requests beyond the first).
- A chain A to A to B is re-checked at each hop.
- Default (nothing set) behaves exactly as `same_origin`.

Validated by: L1 in `schematic-runtime` with two wiremock servers.

**R23. Link check for spec 3 (D32, D28).** The runtime exposes a function that
checks a would-be next-page URL against the policy *before* any request for it
starts and returns `TransportError` kind `CrossOriginRejected` on refusal.
Under `same_origin` a link to origin B yields that error with zero requests to
B; a same-origin link passes under every value including `deny` (PD2).
Validated by: L1 in `schematic-runtime`. Spec 2 has no pager, so this is
tested on the function and through one generated-client call that follows an
absolute URL; the pager integration is spec 3's.

**R24. Redirect mechanics.** Hops are limited (PD3); exceeding the limit is an
envelope Failure kind `RedirectRefused` with the last 3xx visible. 301 and
302 on POST, and 303 on any method except HEAD, change the method to GET and
drop the body and body headers; 307 and 308 preserve method and body (PD4). A
redirect with no `Location`, or an unparsable one, is an envelope Failure
kind `RedirectRefused`. Each followed hop re-decides whether the
already-resolved credential is attached for the new URL (D11: credentials are
resolved once per request, not once per hop). Validated by: L1 in `schematic-runtime` (one case per status).

**R25. Tighten-only override (D28).** A caller may set a policy at least as
restrictive as the definition's (PD1); an attempt to loosen is rejected at
client construction with an error, and no request is sent. A caller override
to `deny` takes effect. Validated by: L1 in `schematic-runtime`.

### Deadline, raw, hooks, guard

**R26. Shared deadline (D13).** A deadline object created once can be passed to
several sends. Across N requests to a mock that delays each response, a
deadline smaller than the sum stops the operation: no request is sent after the
deadline and a `DeadlineExceeded` `TransportError` reports the completed
request count and elapsed time. Each request is given the remaining time (D13; the
mock records the effective timeout or the time it gave up). The alternative
`min(remaining, per-request timeout)` is on the Human review agenda, not
adopted here (PD23). Dropping the future
cancels the in-flight request (the mock sees the connection close) and no task
keeps running (a runtime shutdown completes promptly). Without a deadline the
30 s per-request default applies unchanged. Validated by: L1 in
`schematic-runtime`. The monotonic-clock property is validated by inspection of
the use of `Instant` (a wall-clock-moving test is not portable).

**R27. `send_raw` (D45).** `send_raw` returns status, headers, and bytes
(with a JSON convenience) through the same credential resolution, cross-origin
policy, deadline, and redaction as typed sends: sentinel absent from stored
metadata and `Display`; a cross-origin redirect refused under `same_origin`;
a deadline honored. Typed sends still fail to compile for a wrong response
type. Validated by: L1 in `schematic-runtime`.

**R28. `ResponseKind` guard disposition (D49).** The disposition (PD26) is
implemented and the behavior of a response of an unexpected content kind is
covered by a test under the new model: a JSON endpoint receiving non-JSON
content is `MalformedSuccess` (R10), a text or binary endpoint decodes any
content type, and the test list in the implementation log says whether the
guard was kept, changed, or removed. Validated by: L1 in `schematic-runtime`
and `schematic-schema`.

**R29. Existing hooks preserved (typed-send spike, Q2). Proposed: the parent
decides neither the hook types, the mechanism, `ErrorKind::Hook`, nor the
compile error below.** `pre_response_json`
(client-global) and `mutate_response::<R>` keep working for JSON endpoints
through the typed send, with a mutator typed by the endpoint's declared success
type. A hook registered on a text, binary, or empty endpoint is a compile
error (today it is silently never run); the mechanism is a marker trait
implemented only by the JSON codec, which spec 5 extends to the stream and
GraphQL codecs. A hook that fails yields kind `Hook`.
Validated by: L1 and compile-fail in `schematic-runtime`; existing generated
hook tests in `schematic-schema` still pass or are migrated and listed in the
log.

**R30. The `Decode` trait allows later codecs (spike Q1). Proposed: the
trait detail is the spike's recommendation, not a parent decision.** The `Decode` trait
receives the live response (not a pre-buffered body) and returns a success
value of an arbitrary associated type, so a stream of items and a generated
multi-shape enum are expressible. Validated by: L1 in `schematic-runtime` with
two test-only decoders (an incremental line stream where the first item is
observed before the mock finishes sending, and a two-shape enum). No SSE,
NDJSON, or GraphQL codec ships in this spec.

### Generator, migration, hygiene

**R31. Generated clients are typed and thin.** Per endpoint the generator emits
the request type, the `EndpointSpec` impl, and constants; it emits **no wrapper
method per endpoint** that forwards to `send` (compile-cost evidence above;
parent D53), with no exception. A fresh
generation compiles in the isolated crate established by spec 1, which now
includes `schematic-runtime`. Validated by: L1 in `schematic-gen` (generated
output assertions) and the spec-1 isolated compile check.

**R32. Runtime and generator versions are checked (D39, P4).** The generated
crate fails to compile, with a readable message, against a `schematic-runtime`
whose ABI marker differs from the one the generator was built for (PD11).
Validated by: L1 in `schematic-gen` (a test generates against a doctored
marker and asserts the isolated compile fails), and a second test that a
differing marker drives spec 1's partial-generation check to require full
generation (spec 1 defines that interface; this spec supplies the constant).
The constant and the check are Proposed (PD11); the parent does not decide
them.

**R33. Dependents migrate and pass.** sniff, biscuit-speaks, model-citizen,
`homelab/lib`, `eversolo-integration`, `arcam-amp-integration`,
`sony-receiver-integration`, and `unfolded-integration-helper` build and pass
their own tests, with no remaining use of `request<T>`, `with_client`,
`with_client_and_base_url`, the old `SchematicError` variants, or an injected
`reqwest::Client` for a generated client. `homelab/lib/src/eversolo.rs` and
`homelab/lib/src/samsung_tv.rs` use the managed builder (the Samsung site uses
the opt-in accept-invalid-certificates setting only when HTTPS is used, as
today; the 5 s connect timeout and 10 s request timeout are preserved).
The migration list also includes spec 1's temporary `with_api_key_param`
and generator-local environment scan: this spec removes both and migrates every
user of them onto the runtime's credential handling (R14 to R16). Behavior is
preserved: sniff's anonymous retry stays in sniff (PD31); sniff's
"auth required" branch uses `TransportError` kind `MissingCredential`.
Validated by: each package's own `just test {pkg}` and `just lint`, run from
the repo root (or the package area's recipes); a search for the removed names across the dependents.

**R34. Sentinel never leaks in the migrated consumers' logs.** No migrated
consumer formats a response header map or a request URL from the generated
client with a path that bypasses the redacted metadata. Validated by: L1 in
sniff (existing log/evidence tests still pass) and review of the call-site
inventory in the implementation log (not mechanically testable beyond that;
stated as such).

**R35. Generated WebSocket modules still compile (spec 6 groundwork).** The
generated WebSocket template modules compile after the shared runtime moves
into `schematic-runtime`. Validated by: the spec-1 isolated compile check
over a fresh generation that includes the WebSocket modules, plus `just
lint-schema`.

**R36. REST no-credential rule. Proposed (agenda entry).** For an API whose
definition requires authentication, when no credential resolves (per-call,
client level, environment all empty), the send fails before sending with
`TransportError` kind `MissingCredential`, unless the client was built with an
explicit anonymous setting. This makes REST and the WebSocket spec agree.
Validated by: L1 in `schematic-runtime` (zero requests at the mock; explicit
anonymous sends once with `AuthMethod::Anonymous`; an API that does not
require authentication is unaffected).

**R37. Area justfile and gates.** `schematic-runtime` is added to the
`schematic/justfile` recipes that enumerate packages (`sanity`, `test`, `lint`,
and the other per-package recipes there, for example `build`, `test-l2`,
`doctest`), and `test-schema` is named as the gate for the generated-crate
mock-server tests. Read-only verification, 2026-10-11: the CI planner
(`scripts/ci/affected_scope.py`) reads workspace members from
`cargo metadata` and derives the package area from the manifest directory, so a
workspace member at `schematic/runtime/` is discovered in area `schematic`
without a CI change. This was established by reading the planner, not by
running it; phase 1 confirms with `just ci-local --plan` and records the
result. CI is not changed by this spec. Validated by: the recipes run the new
package; `sniff repo packages` lists it.

## Design

Packages and key symbols. **Decided** means a parent decision covers it (cited).
**Proposed** means the spike recommended it or this spec proposes it; the
implementer adopts it unless a design problem is found and logs either way.

### `schematic-runtime` (new, hand-written, `schematic/runtime/`)

Decided: the package exists and holds the shared runtime (D39). Name and
location are Proposed (PD11).

| Symbol | What it does | Status |
|---|---|---|
| `EndpointSpec` trait | Links a request type to `Success`, `Failure`, `Decode`, `ENDPOINT_ID`, `DECLARED_ERROR_STATUSES` (default `400..=599`). Named `EndpointSpec` for continuity with the existing generated trait and to avoid colliding with the `schematic-define` struct `Endpoint`; a default flagged for review 1. | Proposed (typed-send spike): the name, the `Decode` detail, and `DECLARED_ERROR_STATUSES`; typed association Decided (D3) |
| `Decode<E>` trait | Async decode of the live response into `Output`; one impl per codec; a multi-shape success is a generated enum implementing it. | Proposed (not a parent decision) |
| Codecs `Json<T>`, `Text`, `Binary`, `Empty` | Built-in decoders. SSE, NDJSON, GraphQL codecs are spec 5. | Proposed |
| `Response<T, E>` { `meta`, `outcome` } | The envelope; `data()`, `error()`, `into_result()`, `into_parts()`, `is_decode_failure()`, `is_interrupted()`. | Decided (D7, D25, D26); `into_parts` Proposed |
| `Outcome<T, E>` = `Success(T)` / `Failure(ApiError<E>)` | Inner enum. | Decided (D7) |
| `ApiError<E>` { `kind`, `body: Bytes` } | Failure with raw body always kept. | Decided (raw body, D7); shape Proposed |
| `ErrorKind<E>` (`#[non_exhaustive]`) | `Declared(E)`, `Undeclared`, `MalformedError`, `MalformedSuccess`, `BodyInterrupted`, `RedirectRefused`, `Hook`. | `Declared`, `Undeclared`, `MalformedSuccess`, `BodyInterrupted`, `RedirectRefused` Decided (D25, D26, D32, D48); `MalformedError`, `Hook`, `#[non_exhaustive]` Proposed (S17 unratified); spec 5 adds `UnexpectedContentType` |
| `NoDeclaredError` | Uninhabited placeholder failure type. | Decided (D48); name Proposed |
| `ResponseMeta` / `RequestMeta` | Metadata fields of D21/D48, all redacted, plus `endpoint_id`. | Decided (field set); struct layout Proposed |
| `AuthMethod` | `PerCall`, `ClientLevel`, `EnvVar(name)`, `Anonymous`. | Decided (D48) |
| `TransportError` { `request`, `kind`, `elapsed` } | Outer error, no response head. Kinds: PD7. | Decided (D24 existence); kind list Proposed |
| Redaction module | Union of built-ins (D48), definition-marked names, and auth-strategy names; produces `[redacted]`; builds the Display text of D23. | Decided |
| `Credentials` and `CallOptions` | Client-level and per-call programmatic credentials, caller ordered env list, per-call deadline, per-call policy tightening. | Decided (D11, D13, D28); names Proposed (PD28) |
| `ClientBuilder` / `Client` | Managed transport; redirects disabled in the HTTP client; follows redirects itself. | Decided (D31); surface Proposed (PD8) |
| `OriginPolicy` | Runtime form of the five-value policy plus `check_link`. | Values Decided; runtime type and `check_link` Proposed (PD29, PD30) |
| `ABI` constant | `pub const ABI: u32` checked by generated code and read by spec 1's partial-generation check. | Proposed (PD11) |
| `Deadline` | Monotonic (`Instant`) shared deadline handle. | Decided (D13); form Proposed |
| `send`, `send_raw`, `RawResponse` | Typed and raw send. | `send_raw` Decided (D45); signature Proposed (PD14) |
| Hook types (`PreResponseJsonHook`, mutators) and the marker trait | Moved from generated text; the marker trait gates which codecs accept hooks. | Proposed (types, mechanism, and the compile error of R29); spec 5 extends the marker trait |

### `schematic-define`

- `CrossOrigin` (name Proposed, PD6/PD29): five-value enum; `allow_origins`
  holds exact origin strings. **Decided** (D28 values, default).
- `RestApi` gains `cross_origin: CrossOrigin`, `request_id_header:
  Option<String>`, and `secret_names: Vec<String>` (extra header and query
  names treated as secret). The first is **Decided** (D30); the second is
  **Decided** as to the facility (D48) with the field form **Proposed**; the
  third is **Proposed** (PD25: per-parameter `.secret()` markers wait for spec
  4a so that no endpoint-literal edit (about 519) happens here). This spec
  owns the semantics of `request_id_header` and `secret_names` and the
  API-level fields; spec 4a owns only builder sugar and the per-parameter
  `.secret()` marker.

### `schematic-definitions`

- Each of the 18 API definitions sets the new fields (`cross_origin`, and
  `request_id_header` where the provider is known to send one; absent
  otherwise). **Decided** (D30, D48). Which APIs set a request-id header and
  which set `allow_origins` (for example GitHub's upload host) is a content
  decision: Default in PD25.

### `schematic-gen`

- The client generator emits per-request `EndpointSpec` impls and constants that
  reference `schematic_runtime`; stops emitting the shared runtime text; stops
  emitting `with_client*`; emits the cross-origin and secret-name data from
  the definition; emits the ABI marker check; keeps or removes the erasing
  request enum per PD13. Package: `schematic-gen`. **Decided** (D3, D31, D39);
  details Proposed.
- Generated `Cargo.toml` of `schematic-schema` gains the runtime dependency
  (generator-owned file; regenerate, never hand-edit).

### `schematic-schema`

Regenerated only (parent rule: never hand-edited). It remains excluded from
the root workspace (`exclude` in the root `Cargo.toml`, verified); dependents
reach it by path.

### Dependents

Eight crates: `sniff/lib`, `biscuit-speaks/lib`, `model-citizen/lib` (a
manifest dependency only, no Rust references; confirm and log), and the five
homelab crates (`lib`, `eversolo-integration`, `arcam-amp-integration`,
`unfolded-integration-helper`, `sony-receiver-integration`) migrate call sites
mechanically (R33). Raw grep of `.request(` and `request::<` finds about 60
sites (not filtered to generated clients): sniff remote github 10, gitea 8,
bitbucket 8, gitlab 7; homelab `eversolo.rs` 24, `samsung_tv.rs` 2;
biscuit-speaks `elevenlabs.rs` 1. The phase 0 inventory filters these to
actual generated-client uses.

## Suggested phases

The plan may refine these. Every phase ends with the tree green: `just lint` and
`just test {pkg}` (from the repo root) for the touched packages, and the
dependents building. Because `schematic-schema` is regenerated wholesale and
dependents consume it, the schema regeneration and dependent migration are one
phase (phase 6); earlier phases add the runtime without changing the
generated crate.

0. **Inventory and baseline.** Record the call-site inventory (every use of
   `request<T>`, `with_client*`, `SchematicError`, `EndpointSpec`, hooks) in
   `implementation-log.md` for the eight dependent crates; run each dependent's
   `just test` for a baseline. Verify spec 1's outcomes (query split, isolated
   compile check, any runtime-compat hook) and log any difference from this
   spec's assumptions.
1. **Runtime skeleton.** Create `schematic-runtime` (workspace member): errors,
   metadata, redaction, `Response`/`ApiError`/`TransportError`, `AuthMethod`,
   with unit tests (R3 to R8, R5, R6). Add the package to the area justfile
   recipes and confirm CI planner discovery with `just ci-local --plan`
   (R37).
2. **Transport.** Managed `ClientBuilder`, credentials, single-attempt send,
   deadline, `Decode`/`EndpointSpec` and the four codecs, kinds (R9 to R16, R18,
   R26, R30), tests against wiremock and raw-socket mocks.
3. **Cross-origin runtime.** `OriginPolicy`, redirect follower, credential
   stripping, `check_link`, tighten-only (R19 to R25). Add `CrossOrigin` and
   the new `RestApi` fields to `schematic-define` and set them on the 18
   definitions (R20) in this phase, since the runtime tests drive from the
   definition type.
4. **`send_raw`, hooks, guard disposition** (R27 to R29).
5. **Generator.** Emit typed impls, remove `with_client*` and template text,
   add the ABI marker; extend the spec-1 isolated compile check to include the
   runtime (R1, R2, R17, R31, R32). Generator golden tests updated.
6. **Regenerate and migrate.** Regenerate `schematic-schema` with the
   generator; migrate sniff, biscuit-speaks, model-citizen, the five homelab
   crates (R33), including the two builder call sites; run every dependent's
   tests.
7. **Docs and skills.** READMEs, `schematic/docs/` topic page for the envelope
   (including the D22 limitation), `.claude/skills/schematic-define` and any
   other affected skill, `docs/dependencies.md` for new crates, comment/doc
   drift pass.
8. **Final gates.** Full `just test`, `just test-l2`, `just lint`, `just
   lint-schema` and `just test-schema` in `schematic/`; every dependent; `git status` clean of stray generated
   edits; log completion and the Human review agenda.

## Acceptance criteria and definition of done

The spec is done ("implementation complete, ready for review") when all hold:

1. Every requirement R1 to R34 passes its named validation, or the
   implementation log records it as an accepted gap with the reason (decidable
   by reading the log: no requirement is missing from it).
2. The build and tests pass on macOS, Linux, native Windows, and WSL2 per the
   repo's rules. Locally, run what the current host can and use the `os` skill
   to determine which hosts can produce evidence for the others; record in
   the log which OS evidence was produced and which was left to CI. A test
   that is red on exactly one OS is investigated with the `os` skill, not
   skipped.
3. `just test` and `just lint` pass in `schematic/` (all schematic packages,
   including `schematic-runtime`) and in each dependent's package area;
   `just lint-schema` passes (generated crate compiles and is clippy-clean
   with `-D warnings`) and `just test-schema` passes (generated-crate
   mock-server tests).
4. `schematic-schema` is regenerated by the generator (`just generate` or the
   area's recipe), never hand-edited; regeneration is deterministic
   (re-running produces no diff).
5. Every dependent crate builds and its tests pass: sniff, biscuit-speaks,
   model-citizen, `homelab/lib`, `eversolo-integration`,
   `arcam-amp-integration`, `sony-receiver-integration`,
   `unfolded-integration-helper`.
6. Documentation is updated for changed behavior: `schematic-runtime` README
   and crate docs; the affected package READMEs; `schematic/docs/` topic pages
   (a page describing the envelope, errors, credentials, cross-origin policy,
   builder, and deadline, written for a developer new to the repo with a
   compact example per rule and a Mermaid diagram for the
   envelope-versus-`TransportError` decision and for the redirect flow); the
   affected skills under `.claude/skills/` (at least `schematic-define`, plus
   any skill that documents the generated client or sniff's provider stack);
   `docs/dependencies.md` for new crates and per-area dependency docs. Docs
   state behavior in their own words and **never name or link a feature or fix
   spec**.
7. A comments-and-docs drift pass over every symbol whose behavior changed
   (`///`, `//!`, inline `//`); drift is resolved assuming the code is correct,
   and each instance is logged.
8. No commits by the agent unless the plan says otherwise. If the plan does
   commit: signed, author's name/email, no agent attribution, verified with
   `git verify-commit HEAD`, per repo rules.
9. `implementation-log.md` exists in this directory with the Human review
   agenda (below) filled in. The agent does not move the spec to `_completed`
   and does not run `just complete`.

## Pre-authorizations

Pre-authorizations the author must confirm before the unattended run (parent
D50 item 8, plus items specific to this spec). Until confirmed, none is
granted. Anything not listed that would require a human decision is logged and
the run continues with the default or stops at the nearest safe point if no
default exists.

- Regenerate `schematic-schema` (large tree change; generated output is never
  hand-edited).
- Create the `schematic-runtime` package at `schematic/runtime/`, add it to the
  root workspace and to the area justfile recipes (R37), and add it to the
  generated crate's dependencies.
- Edit the dependent crates: sniff, biscuit-speaks, model-citizen, and the five
  homelab crates, including their `Cargo.toml` files if a dependency edit is
  needed.
- Edit the 18 API definitions in `schematic-definitions` to set
  `cross_origin`, `request_id_header`, and `secret_names`; edit `RestApi` and
  its struct-literal construction sites (definitions, tests, doctests) for the
  three new fields. **Do not edit the endpoint literals (about 519; openai
  265).**
- Add dev-dependencies `trybuild` (compile-fail tests), `rcgen` and
  `tokio-rustls` (local TLS mocks, PD33), and any runtime dependency the
  transport needs, documenting each in `docs/dependencies.md`.
- Update `.claude/skills/` files affected by the changed behavior (at least
  `schematic-define`, plus any skill documenting the generated client or
  sniff's provider stack).

Deleting the stale git-tracked `schematic/gen/schematic/` tree is spec 1's
only; this spec does not delete it.

Not pre-authorized: committing, pushing, moving the spec to `_completed`,
changing CI scope or workflows, contacting real providers, or any
destructive action outside the files above.

## Dependencies and ordering

- **Requires** `2026-10-11-schematic-generation-integrity` landed. This spec
  assumes from it (unverified until phase 0): the generator separates query
  parameters from the path in `RequestParts` (needed by R5 and R7); a
  temporary `with_api_key_param` and generator-local environment scan exist
  (this spec removes them, R33); the
  isolated compile check of freshly generated output exists and runs on a
  normal test run; partial generation (D9) detects an incompatible
  shared-runtime change through an interface spec 1 defines (this spec
  supplies the ABI constant, R32); request-side features (text/binary bodies,
  `ApiKeyParam`, header and cookie parameters) are generated. If any is
  missing, log it, implement the minimum needed here only if it blocks a
  requirement, and flag it for review 1.
- **Blocks** the pagination spec (spec 3, which needs the envelope, deadline,
  `check_link`, `InvalidArgument`), and transitively specs 4a to 7.
- sniff's retirement of `FocusedProviderClient` is outside this spec and
  starts after spec 3.
- Order inside the run: runtime and tests before generator; generator before
  regeneration; regeneration and dependent migration together.

## Planning-time details assigned to this spec

These are the parent's P-items (P1 to P7 parts, plus P9 parts) that D50 assigns
here. Each has a default. **Default if not ruled before planning** means the
implementer adopts it and logs it in the Human review agenda; nothing here is
a decision of the author.

**PD1. Tighten-only comparison and rejection form (P1; R4, R25).**
Default if not ruled before planning: define each policy by what it reaches
(set of origins) and where credentials go. `deny`: reaches nothing, no
credentials. `same_origin`: reaches the base; credentials to the base.
`allow_origins(L)`: reaches base and L; credentials to those. `allow_without_credentials`:
reaches any origin; credentials to the base only. `allow_any`: reaches any
origin; credentials to any. A caller policy P' is permitted under definition
policy P only when reach(P') is a subset of reach(P) **and** credential-targets(P')
is a subset of credential-targets(P). So `allow_origins(L')` with L' a subset of
L is permitted; `allow_origins` over `allow_without_credentials` is **not**
comparable and is rejected. Rejection is a runtime error at client
construction (a typed `BuildError`), because the definition's policy is data;
a compile error is not feasible.

**PD2. `deny` and same-origin next-page links (P1; R23).** Default if not ruled
before planning: `deny` still follows same-origin next-page links (it forbids
redirects and cross-origin links only), so same-origin paging keeps working.

**PD3. Redirect hop limit (P1; R24).** Default if not ruled before planning:
10 hops (reqwest's own default), counting each followed redirect; the 11th
redirect response is refused as `RedirectRefused`. Constant lives in
`schematic-runtime`.

**PD4. Method and body rules (P1; R24).** Default if not ruled before planning:
301 and 302 turn POST into GET and drop the body and body-describing headers;
303 turns any method except HEAD into GET and drops the body; 307 and 308
preserve method and body. Request bodies are held as replayable bytes in the
typed path, so 307/308 replay is possible; a non-replayable body (a streamed
upload, if any exists) on a 307/308 is refused as `RedirectRefused`.

**PD5. Refusal at a later hop of a chain (P1; R22).** Default if not ruled
before planning: the envelope shows the 3xx response of the hop that was
refused (its status and headers including `Location`), with `meta.url` being
the URL of the request that received it; earlier hops are not listed. Log
whether callers need the chain (a `redirects` field is not added here).

**PD6. Cross-origin naming and public API (P1; R20).** Default if not ruled
before planning: type `CrossOrigin` in `schematic-define` with constructors
`CrossOrigin::same_origin()`, `allow_origins(iter)`,
`allow_without_credentials()`, `danger_allow_any()` (the `danger_` prefix is
the "clearly named opt-in"), `deny()`; field `RestApi::cross_origin`; generated
clients expose `ClientBuilder::cross_origin(policy)` as the tighten-only
override.

**PD7. `TransportError` kinds (P2; R9).** Default if not ruled before planning
(`#[non_exhaustive]`): `MissingCredential`, `CrossOriginRejected`,
`InvalidArgument` (also used for a page size above the declared maximum, set
by spec 3), `InvalidUrl`, `Connect` (DNS failure, connection refused or reset
before a response head), `Tls`, `Timeout` (per-request timeout fired before a
response head arrived), `DeadlineExceeded`, `Cancelled` is *not* a kind (drop
cancels silently), and `Other(String)` for unclassified library errors with
the library URL stripped. A timeout after a response head is `BodyInterrupted`
(an envelope), not `Timeout`.

**PD8. Managed builder surface and names (P3; R18).** Default if not ruled
before planning, on `schematic_runtime::ClientBuilder`: `base_url`, `timeout`,
`connect_timeout`, `proxy`, `add_root_certificate` (PEM or DER bytes),
`user_agent`, `default_header`, `pool(&SharedPool)` and `SharedPool::new()`,
`danger_accept_invalid_certificates()`, `credentials`, `cross_origin`,
`env_order`. Each generated client exposes `Client::builder()` pre-seeded from
the definition and `Client::new()` for the all-defaults case.

**PD9. Accept-invalid-certificates form (P3; R18).** Default if not ruled
before planning: a no-argument `danger_accept_invalid_certificates()` builder
method (reads as a deliberate opt-in; off by default). Samsung HTTPS call
sites call it; no other site does.

**PD10. Middleware / hooks mechanism (P3). Proposed.** Default if not ruled
before planning: no general middleware is built. Only `pre_response_json` and
`mutate_response` are preserved (R29), through a marker trait implemented by
the JSON codec only (spec 5 extends it to the stream and GraphQL codecs). A request for middleware found during migration is logged as
an open question.

**PD11. Runtime package name, location, versioning, compatibility (P4; R1,
R32). The ABI constant and its check are Proposed, not parent decisions.**
Default if not ruled before planning: package `schematic-runtime`, at
`schematic/runtime/`, workspace member, version `0.1.0`, path dependency.
Compatibility: the runtime exports `pub const ABI: u32`; the generator embeds
the value it was built against; generated `lib.rs` contains a `const _: () =
assert!(schematic_runtime::ABI == N)` with a readable message; and the
generator's partial-generation inventory check (spec 1) treats a differing
marker as "incompatible shared-runtime change: require full generation". The
generated `Cargo.toml` uses a path dependency, no version pin beyond the
workspace version.

**PD12. What lives in the runtime versus generated code (P4).** Default if not
ruled before planning: runtime holds `EndpointSpec`, `Decode`, the four codecs, all
envelope and error types, redaction, credentials, builder, redirect/origin
logic, deadline, hooks, `send`, `send_raw`. Generated code holds per-endpoint
request types, their `EndpointSpec` impls and constants, the per-API client struct
(thin: holds a runtime `Client`, definition data), and no per-endpoint
wrapper methods (D53).

**PD13. The erasing request enum (P5; S13).** Default if not ruled before
planning: keep the generated per-API request enum **only if** `into_parts` or
per-client scoping still needs it; it must not be reachable from any public
send path (typed send takes the concrete request type). If it is not needed,
delete it. Record which in the log.

**PD14. `send_raw` signature (P5; R27).** Default if not ruled before planning:
`client.send_raw(request, &CallOptions) -> Result<RawResponse, TransportError>`
where `request` is a generated typed request (so path, method, and body come
from the definition) and `RawResponse { meta, status, headers, body: Bytes }`
has `json::<T>()`. A raw send does no status-based failure classification: any
received response head, including 4xx and 5xx, is `Ok(RawResponse)`. Per the
governing rule a received head always yields a result, so a body cut off after
the head returns `RawResponse` with `body` holding the partial bytes and an
`interrupted: bool` flag, not a `TransportError`. (This choice is flagged for
review 1.)

**PD15. Client-global `pre_response_json` (P5; S14).** Default if not ruled
before planning: keep it client-global, run on every JSON-decoded response body
before decode, document that it sees the raw JSON of every endpoint, and note
that spec 5 must scope it for GraphQL wire bodies.

**PD16. Stream responses `Clone`/`Sync` (P5; S15).** Default if not ruled
before planning: accepted as not `Clone` and not `Sync`; documented.

**PD17. `ErrorKind` shape (P5; S17).** Default if not ruled before planning:
`#[non_exhaustive]`; variants as in Design; `UnexpectedContentType` is
**not added here; spec 5 adds it** (the enum is `#[non_exhaustive]`, so the
addition is not breaking). A wrong content type on a JSON endpoint is
`MalformedSuccess` per D25 in this spec.

**PD18. `into_result` versus `into_parts` (P5).** Default if not ruled before
planning: provide both. `into_result() -> Result<T, ApiError<E>>` discards
metadata; `into_parts() -> (ResponseMeta, Outcome<T, E>)` keeps it.

**PD19. `elapsed` for streams (P5).** Default if not ruled before planning:
time to the response head; no stream endpoint ships in this spec, so this is
documented on the `Decode` trait for spec 5.

**PD20. Value-only convenience (P5).** Default if not ruled before planning:
none is added (`data()` and `into_result()` cover it).

**PD21. Redaction of response headers beyond `Set-Cookie` (P5).** Default if
not ruled before planning: one redaction set applies by header name to request
and response headers alike (built-in list, auth-strategy names, and
`secret_names`).

**PD22. Placeholder error type name (P5).** Default if not ruled before
planning: `NoDeclaredError`.

**PD23. Deadline arithmetic (P6; R26).** Default if not ruled before planning:
each request under an opted-in deadline gets the remaining time (D13
literally), so the 30 s default does not bound a request once a deadline is
set. The alternative `min(remaining, per-request timeout)`, which keeps the
30 s bound, is on the Human review agenda. The deadline
is a `Deadline` handle created once (`Deadline::after(duration)`) and passed in
`CallOptions`. The session-close grace after expiry belongs to spec 3.

**PD24. HTTP/2 cancel wording (P6; S16).** Default if not ruled before
planning: the cancel criterion accepts "stream reset" as the HTTP/2 form of
"connection close". Tests are HTTP/1.1 only (HTTP/2 untested; stated in Risks).

**PD25. Request-id marker and secret marker forms; which APIs set them (P9
part; R4, R5).** Default if not ruled before planning: `RestApi::request_id_header:
Option<String>` set by the 18 definitions only where the provider's
documented request-id header is known from the existing definition files or the
provider's docs available offline (GitHub `x-github-request-id` is the
parent's example); all others `None`. `RestApi::secret_names: Vec<String>`
(case-insensitive for headers, exact for query names) is the only definition
secret marker in this spec; per-parameter `.secret()` waits for spec 4a. Each
API's `cross_origin` is `same_origin` except where an existing definition or
documented behavior requires another origin (for example GitHub's upload host
via `allow_origins`); if no evidence exists in the tree, leave `same_origin`
and log.

**PD26. `ResponseKind` guard (P5; D49; R28).** Default if not ruled before
planning: remove the runtime guard and its error. The typed `Decode`
implementations make a wrong-kind call unrepresentable for text and binary,
and a JSON endpoint receiving non-JSON becomes `MalformedSuccess`. The log
lists the tests that replace it.

**PD27. Status classification (R3, R10).** Default if not ruled before
planning: a final response with status 2xx is decoded as success; any other
final status (including a 304 or a 3xx that was not followed and not refused,
and 1xx never surfaces) is a Failure; a status inside
`DECLARED_ERROR_STATUSES` is decoded as `Declared` when `E` is declared,
otherwise `Undeclared`; a non-2xx status outside that range is `Undeclared`.

**PD28. Programmatic credential type (R14).** Default if not ruled before
planning: `Credential` wraps the secret value in a type whose `Debug` and
`Display` print `[redacted]` and is applied through the API definition's auth
strategy (bearer, basic, api-key header, api-key query, api-key cookie).
Per-call: `CallOptions::credential`; client level: `ClientBuilder::credentials`.
No label (see PD34).

**PD29. Where the policy type lives (R20).** Default if not ruled before
planning: `schematic-define` owns `CrossOrigin` (data); `schematic-runtime`
owns `OriginPolicy` (behavior) and does not depend on `schematic-define`; the
generator translates one to the other as generated constants.

**PD30. Link-check primitive for spec 3 (R23). Proposed.** Default if not ruled before
planning: `Client::check_link(&Url) -> Result<(), TransportError>`, usable
before building a request, applying the same comparison as redirects.

**PD31. Call-site migration policy (R33).** Default if not ruled before
planning: preserve behavior. sniff's anonymous retry (gap A5) is kept in sniff
as a separate second call; sniff's status policy (401, 403/429 rate limit,
404 as absent) is rebuilt over the envelope's status and headers; hand-written
`Retry-After` handling moves to reading the failure's headers. Users of spec
1's temporary `with_api_key_param` and environment scan move to the runtime's
credential handling.

**PD32. Withdrawn.** The parent rules (D53) that no per-endpoint wrapper
methods are generated, so there is nothing to default. The number is kept so
that PD references stay stable.

**PD33. Test infrastructure for TLS and proxy mocks (R18).** Default if not
ruled before planning: dev-dependencies for compile-fail tests (`trybuild`),
for generating a self-signed certificate (`rcgen`) and for a local TLS
acceptor (`tokio-rustls`), and a
hand-rolled TCP proxy mock; document each in `docs/dependencies.md`. If a
dependency cannot be resolved offline, log it and cover the case with a
loopback HTTP proxy test and a documented gap for TLS.

**PD34. Caller-supplied auth label (R16).** The parent's D7 text allows "a
caller-supplied label" for the auth method while D48 fixes `AuthMethod` to four
values with no label. Default if not ruled before planning: follow D48 (no
label).

**PD35. REST no-credential rule (R36). Proposed.** Default if not ruled before
planning: for an API that requires authentication, when no credential resolves
the send fails before sending with `TransportError` kind `MissingCredential`
unless the client was built with an explicit anonymous setting. It is the
rule the WebSocket spec assumes; the author confirms it at review 1.

## Risks and failure modes

- **Largest breaking change; no human in the loop.** Mitigation: phases keep
  the tree green; the dependents are migrated in one phase together with
  regeneration; the call-site inventory is recorded first.
- **Call-site counts are raw.** The parent's "about 27 call sites outside
  sniff" is not itemized. A raw grep finds about 60 `.request(` / `request::<`
  sites across eight dependent crates, not filtered to generated clients (see
  Design, Dependents); the phase 0 inventory establishes the real set and
  sniff's exhaustive matches. model-citizen holds a manifest dependency only
  (confirm in phase 0).
- **Compile-time regression.** Evidence (550 endpoint impls, synthetic; about 519 endpoint literals) is for the
  trait only; real-schema and incremental builds were not measured. Mitigation:
  no wrapper per endpoint (R31); log compile time of `schematic-schema` before
  and after on this host.
- **Redirect-following is new, security-sensitive code.** Risks: credential
  leakage on a hop, method/body mishandling, a loop. Mitigation: R22 to R24
  matrix with zero-request assertions on the refused origin; credential
  stripping asserted for all forms.
- **`Debug` credential leak through decoded error bodies** remains by D22;
  documented, not mitigated.
- **A 2xx on the failure side surprises callers** (D25: `into_result()` is
  `Err` for an undecodable 2xx); documented.
- **OS differences.** An abrupt connection close (R11) and connection-refused
  timing differ across macOS, Linux, native Windows, and WSL2; the spike ran on
  macOS only. Windows-specific path or process-environment handling in the
  environment-scan tests (R15) needs the serial-test pattern. HTTP/2 is
  untested.
- **Local TLS and proxy mock dependencies** may not resolve offline (PD33).
- **Version skew between generator and runtime** (R32) if the ABI marker is
  forgotten on a breaking runtime change; mitigated by the doctored-marker test.
- **Interaction with later specs.** Spec 3 consumes `check_link`,
  `InvalidArgument`, `Deadline`, and the `page` slot; spec 5 consumes `Decode`
  and `ErrorKind`; spec 4a needs the secret and request-id facilities to take a
  builder form. Changes here that break those are flagged in Refresh notes.
- **Landing size (P11).** Landing 1 is the largest landing (specs 1 to 3);
  this spec is the largest spec in it. If the run cannot finish, stop at the
  last green phase, record state in `implementation-log.md`, and set
  `message_to_agent` accordingly (the plan owns that mechanism).

## Process

Each spec has a 1:1 plan. The plan is implemented end to end, phase by phase,
**without a human in the loop**. Then **review 1** runs, with time set aside for
the author to review decisions made during implementation and any open design
questions. Then the next spec and plan are refreshed from the outcomes. Then
fix/review cycles repeat until the reviewer deems the spec production ready.
An agent's terminal state is "implementation complete, ready for review"; only
the author moves a spec to `_completed`.

Because no human is available while the plan runs, this spec is self-contained
(decisions and requirements are restated above), gives a default for every
remaining planning-time detail (PD1 to PD35), and lists in the Human review
agenda the entries the implementer must record. The implementer **logs each
entry in `implementation-log.md` as it occurs**, not at the end.

## Human review agenda

Record in `implementation-log.md`, for review 1, as they occur:

1. **Every default adopted** from "Planning-time details" (PD1 to PD35), with
   the PD number and whether the implementation matched it exactly.
2. **Every design choice not dictated by this spec**, with the alternatives
   considered (for example: struct layout of `ResponseMeta`, module layout of
   `schematic-runtime`, how `send` is generic over `EndpointSpec`, error `Display`
   wording, how the follower tracks credentials per hop).
3. **Every deviation** from a requirement or a Decided item, with the reason.
4. **Every open question found**, including: anything in the parent that
   contradicted this spec, any spec 1 outcome that differed from the
   assumptions in Dependencies, any case where middleware or a feature outside
   scope was needed by a dependent.
5. **Call-site inventory and migration notes**: counts per dependent, behavior
   preserved or changed (especially sniff's status policy and anonymous
   retry), and any dependent that needed more than a mechanical change.
6. **Evidence by OS**: which of macOS, Linux, native Windows, and WSL2 produced
   test evidence, and which tests were platform-sensitive.
7. **Measurements**: `schematic-schema` compile time before and after;
   any binary-size observation in a dependent.
8. **Departures from docs**: any `docs/` page corrected because the code
   departed from what was planned.
9. **Requirements not met** and accepted gaps (R-number and reason).
10. **Parent contradictions or gaps noticed** (see Refresh notes for the ones
    already known).
11. **Items for the author's ruling at review 1** (each is a default or a
    Proposed item, not a decision):
    - PD1: tighten-only comparison and rejection form.
    - PD14: `send_raw` signature and the interrupted-body result.
    - PD23: deadline arithmetic. Adopted: each request gets the remaining time
      (D13). Alternative: `min(remaining, per-request timeout)`.
    - PD26: removal of the `ResponseKind` guard.
    - The trait name `EndpointSpec` (default; alternative `Endpoint`, which
      collides with the `schematic-define` struct).
    - The `MissingCredential` default for an authenticated API with no
      resolved credential (R36, PD35).
    - The hook mechanism by name: a marker trait implemented by the JSON
      codec, extended by spec 5 (R29, PD10).
    - Proposed items that the parent does not decide: the hook types,
      `ErrorKind::Hook`, R29's compile error for hooks on non-JSON endpoints,
      the ABI constant, `check_link`, the `Decode` trait detail,
      `DECLARED_ERROR_STATUSES`.
    - The R20 surface test is frozen to today's API and must be retired or
      updated by specs 4a (endpoint-level override) and 5 (GraphQL primitive).

## Refresh notes

`refresh_required: true`. Before planning, refresh this spec from the outcomes
(review 1) of `2026-10-11-schematic-generation-integrity`. Specifically check:

- Whether spec 1 delivered the query-parameter split in `RequestParts` in the
  shape R5 and R7 assume (path and query separate, query as ordered key/value).
- The exact isolated compile-check harness, so R31 extends it rather than
  duplicating it, and whether it can resolve the new `schematic-runtime`
  dependency offline.
- Whether spec 1's partial-generation inventory check already has a hook for an
  "incompatible shared-runtime change" (PD11), and what marker it reads.
- Whether spec 1 removed the stale `schematic/gen/schematic/` tree (the
  pre-authorization above is conditional on it).
- Whether spec 1's request-side features changed `RequestParts` or the
  generated request types in ways that affect the `EndpointSpec` impl emission.
- The counts spec 1 established for D33 features (the "six" versus "about 29"
  figures are unreconciled in the parent, D49 item 6).

Also carry forward:

- The R20 surface test is frozen to the current public surface; specs 4a
  and 5 retire or update it when they add the per-endpoint override and the
  GraphQL primitive.
- The session-close grace after the deadline expires (parent S10) is spec 3's
  to decide; this spec's `Deadline` only supplies the handle.
- Whether the existing generated convenience methods for non-JSON endpoints
  disappear (D53: none are generated); the phase 0 inventory finds any use.
- Whether the temporary `with_api_key_param` and generator-local environment
  scan from spec 1 exist as this spec assumes, and who uses them (R33).

Known gaps and contradictions in the parent that the refresh should resolve or
keep flagged:

- D7 mentions "a caller-supplied label" for the auth method; D48 fixes four
  `AuthMethod` values with no label (PD34).
- D24 lists `Timeout` among failures "before anything is sent", but a timeout
  that fires after sending and before any response head is also "no head
  arrived" (PD7 treats it as `TransportError::Timeout`).
- D32 puts a refused next-page link in `TransportError`, but pagers are spec 3;
  this spec can only test the link check on a function (R23).
- D40 asks for a `.secret()` marker "on any header or parameter", which touches
  `ParamDef` and header tuple types, while D42 forbids touching the endpoint
  literals (about 519) in landing 1; PD25 resolves this with an API-level list
  and defers per-parameter markers to spec 4a (the parent leaves the owner of
  this P9 item unassigned).
- D49 item 5 puts the `ResponseKind` guard disposition in landing 1 "with the
  new response model", but the response model v2 arrives in spec 5; PD26
  decides it against the spec 2 model.
- The parent maps "middleware hooks" to P3 (spec 2) while D31 calls the hooks
  mechanism "later"; PD10 builds none.
- The parent does not say whether `schematic-runtime` is a workspace member
  (`schematic-schema` is excluded from the workspace); PD11 makes it a member.
- The "about 27 call sites" figure and sniff's count of exhaustive matches are
  not itemized in the parent (unverified).

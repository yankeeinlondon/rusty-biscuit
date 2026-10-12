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
    - 2026-10-11-schematic-response-model-graphql
refresh_required: true
related: []
---

# Schematic WebSocket

This is child spec 6 of 8 under the Schematic macro review (decision D50 of
the parent). It is a **draft**: it is refreshed from the outcomes of the
previous spec's review 1 before its plan runs (see "Refresh notes"). The order of the eight
specs is: 1 schematic-generation-integrity, 2 schematic-runtime-envelope,
3 schematic-pagination, 4a schematic-definition-api, 4b
schematic-definition-vocabulary, 5 schematic-response-model-graphql, 6 this
spec, 7 schematic-fidelity-polish. `depends_on` lists only spec 5; specs 1, 2,
4a and 4b are prerequisites through that chain.

## Problem

Schematic generates a Rust WebSocket client and a server-side "host" for
each of five WebSocket API definitions. The transport core is sound
(supervisor loop, request/response correlation, backoff), but the generated
clients are wrong or inert in six verified ways, and the definitions are
five dialects of the same vocabulary. Facts below come from the parent
(sections 8 and 10, ledger rows 3 and 9 to 12) and were re-checked against the
tree on 2026-10-11 where marked **verified**; everything else is the
parent's July evidence and is marked **parent**.

1. **Wrong URL (ledger row 3).** The Unfolded Circle Core definition sets
   `base_url: "ws://remote.local/ws"` and its first endpoint `CoreWs` has
   `path: "/ws"`, so the client connects to `ws://remote.local/ws/ws`
   (**verified** in `schematic/definitions/src/unfolded_circle/core_ws/mod.rs`,
   lines 144 and 182). The same definition's other endpoints use `/intg` and
   `/profiles` (**verified**), so they inherit the same base-URL error in a
   different form. Whether the real device paths are `/ws/intg` and
   `/ws/profiles` is **unverified**; the `unfolded-circle` skill and the
   Core API documentation settle it.
2. **Reconnect counter never resets; lifecycle open never re-sent (row 9).**
   In the generated supervisor the attempt counter is set to zero only for the
   first stream and is incremented after a *successful* redial, so the
   supervisor gives up after N lifetime reconnects, not N consecutive
   failures (**verified** in `schematic/gen/src/ws_codegen/client.rs`, around
   lines 636 to 682). The state is set to `Ready` straight after a redial and
   the lifecycle open message is not re-sent, so a "reconnected" stream is
   protocol-dead while reporting `Ready` (**parent**).
3. **Dormant `E0308` in the Bearer host template (row 10).** The Bearer branch
   of the generated host's auth check ends an `else` arm with a trailing `;`
   after `Err(...)`, so the arm yields `()` against a `Result` tail
   (**verified** at `schematic/gen/src/ws_codegen/shared.rs`, around lines 526
   to 534). It survives because `validate_code` is grammar-only and the unit
   test asserts substrings (**parent**). It is dormant because none of the five
   shipped definitions emits a Bearer host (**parent**).
4. **Env auth never applied (row 11).** The generator emits an `EnvMapping`
   (**verified**, `client.rs` around line 154) but connect-time header
   building never consults it, so no generated client sends an
   environment-derived credential, while the Dock module's docs advertise
   `UCR_DOCK_TOKEN` (**parent**).
5. **Diagnostics discarded (row 12).** `lower_to_plan` returns
   `Result<WsRuntimePlan, GeneratorError>` and the plan carries
   `WsDiagnostic { severity, path, message }` values with severities
   `Info`, `Warning`, `Error` (**verified**, `plan.rs`). The assembler does
   `Err(_) => return None` (**verified**, `schematic/gen/src/output/ws_modules.rs`
   around line 123), so a failing definition yields no file and no message, and
   the plan's diagnostics are never printed.
6. **`receive_timeout` accepted and ignored.** `WsClientOptions` exposes it;
   the supervisor reads it into an unused variable behind a `TODO`, and the
   planner pushes an `Info` diagnostic saying so (**verified**,
   `client.rs` around line 617 and `plan.rs` around line 471).
7. **Typed messages computed, never emitted.** The plan holds the client,
   server and bidirectional message lists, but the generated surface is
   `send(serde_json::Value)` and `next_event() -> Option<Result<Value, _>>`
   (**parent**).
8. **Name-matching instead of a declared link.** Correlation is inferred when
   an endpoint has messages literally named `RequestEnvelope` and
   `ResponseEnvelope`, with the field names `id` and `req_id` hard-coded
   (**verified**, `plan.rs` lines 188 to 206). `CorrelationHints` and
   `HeartbeatHints` have no users (**parent**), so Samsung's declared
   `RequestIdType::String` is dead.
9. **Forked vocabulary.** `WebSocketApi` already reuses the REST
   `AuthStrategy` (**verified**), but connection parameters are a
   WebSocket-only pair `ConnectionParam` / `ParamType` (**verified**,
   `schematic/define/src/websocket/params.rs`) instead of the REST `ParamDef`,
   the REST `EnvMapping` and header types are not used by the definition, and
   `env_auth: Vec<String>` is a second source of truth. The five definitions
   disagree with each other: identical auth handshakes are prose in one module
   and `AuthFlowHints` in its sibling, and `integration_ws` marks all six
   messages `Bidirectional` to trip a generator heuristic, with a test
   asserting the contortion (**parent**).
10. **Unfolded-Circle assumptions baked into the generic host.** The host
    generator emits a struct literally named `UnfoldedCircleEventHub` for
    every host and answers authentication with a fixed
    `authentication_success_response()` (**verified**,
    `schematic/gen/src/ws_codegen/host.rs`). The homelab consumers
    of these names are `homelab/lib` (`src/samsung_tv.rs`, the Samsung
    WebSocket client) and the four integration crates `eversolo-integration`,
    `arcam-amp-integration`, `sony-receiver-integration` and
    `unfolded-integration-helper` (the last consumes
    `unfolded_circle_integration_ws`); `homelab/cli` and `homelab/server` have
    none (**verified** by grep, listed in R15).

One parent claim did not hold when checked: the parent says ElevenLabs'
WebSocket types "do not use `schematic_define` at all". In the tree, the
ElevenLabs WebSocket *definition* **does** use `schematic_define::WebSocketApi`
(`schematic/definitions/src/elevenlabs/mod.rs`, line 189; **verified**). Only
the *message payload types* are standalone types that do not use
`schematic_define`. The implementer confirms the payload-type finding (see
R13) and logs it.

## Goal and non-goals

**Goal.** Make the generated WebSocket clients correct and honest: right URL,
a supervisor that recovers repeatedly and replays its opening message,
connect-time credentials resolved by the shared runtime rules, enforced
timeouts, loud diagnostics, typed message enums, an explicit declared
response link, and one shared vocabulary for parameters, auth, env mapping and
headers across the five definitions and the homelab consumers.

**Non-goals** (each is decided or deferred elsewhere):

- Full REST/WebSocket unification under one definition type is **rejected**
  (D8). `WebSocketApi` stays its own type with its own generator.
- The AsyncAPI **exporter** is spec 7 (D47). This spec must not break the
  existing AsyncAPI *importer* (`schematic/gen/src/asyncapi_import.rs`), but
  does not extend it.
- No contact with real devices or real providers. Every test uses a local mock
  WebSocket server.
- No change to REST definitions, the REST generator, pagination, or the
  response model (specs 1 to 5).
- No redesign of the WebSocket transport core (supervisor structure,
  correlation machinery, backoff curve) beyond the fixes listed.
- Hooking a WebSocket into HTTP-only features (shared deadline for a long-lived
  socket, pagination, redirect following) is out of scope unless a requirement
  below names it.

## Background: decisions this spec implements

Restated in full so the implementer does not need the parent. Provenance is
the parent spec named in the frontmatter, section 15 unless noted.

- **D8. WebSocket extent: fixes plus shared building blocks.** Fix the
  defects (wrong URL, reconnect counter reset, lifecycle replay, env auth,
  dropped diagnostics, the dormant Bearer-template compile error). Emit the
  typed message enums. Replace the `RequestEnvelope` string match with an
  explicit link. Make WebSocket reuse the REST parameter, auth-policy,
  env-mapping and header types, replacing `ConnectionParam`/`ParamType`.
  `WebSocketApi` stays its own type with its own generator, and the five
  definitions are normalized. Rejected: full REST/WS unification under one
  definition type, for this work.
- **D46. Stage order.** WebSocket is landing 3 and follows REST vocabulary
  (landings 1 and 2); fidelity and polish (spec 7) follows it.
- **D47 (boundary only).** The AsyncAPI exporter is in landing 4, after
  WebSocket reuses the REST types. It is not part of this spec.
- **D50 (spec 6 line).** "WebSocket. D8, D46. Independent of REST internals
  once spec 4 lands." Each spec leaves the whole repo green on macOS, Linux,
  native Windows and WSL2 and ends at "implementation complete, ready for
  review"; an agent never moves a spec to `_completed`.
- **D51.** The per-spec process, restated under "Process".
- **D11 (credentials), as it applies to connection setup.** Precedence is per
  call, then client level, then environment policy, then anonymous. The
  environment policy scans the definition's ordered list of variable names
  once before sending, selects the first non-empty variable, and sends exactly
  that one credential. There is a single attempt: no chain of different keys,
  no retry, no silent anonymous retry if the credential is rejected. Every
  result reports the auth method used (per call, client level, the variable's
  name, or anonymous), never the value.
- **D48 and D40 (redaction), as they apply to connection setup.**
  `AuthMethod = PerCall | ClientLevel | EnvVar(name) | Anonymous`, never a
  value. Built-in secret names: headers `Authorization`, `Proxy-Authorization`,
  `Cookie`, `Set-Cookie`, `X-Api-Key`, `X-Auth-Token`, `Api-Key`; query
  parameters `api_key`, `apikey`, `access_token`, `token`, `key`, `signature`.
  Secrets are marked from auth strategies (for example an `ApiKeyParam` name),
  by an explicit `.secret()` marker, and by the built-in list. Redacted values
  show `[redacted]`.
- **D23 (URL in error text), as it applies to connection setup.** Error text
  may include the URL, composed from Schematic's redacted URL, and never a raw
  query-string secret.
- **D28 and D31 (cross-origin and managed transport), as they apply to the
  handshake.** Credentials attach only to an allowed origin; the default is
  `same_origin`; Schematic follows redirects itself according to the policy.
  Whether and how this reaches a WebSocket handshake is **not decided** by the
  parent; this spec's default is R7 and is logged for review.
- **D39.** The shared runtime lives in a hand-written package (working name
  `schematic-runtime`; spec 2 fixes the final name, which this spec uses
  verbatim). Generated clients depend on it and stay thin.
- **D15 / D2 / D44 (vocabulary context).** REST definitions are built with
  builders and typed registration after spec 4. WebSocket reuse must follow
  whatever shapes spec 4 left (see "Refresh notes").

## Requirements

Each requirement is testable. "Mock server" means a local WebSocket server
bound to `127.0.0.1` on an ephemeral port, started inside the test (see
"Design" for the harness). None of these tests may open a window or take
focus; they use no terminal. Test levels follow the `rust-testing` skill: L1
runs under `just test`; L2 under `just test-l2` and must never take window
focus.

### Defect fixes

**R1. Core WebSocket URLs are correct.** After the change the generated
client for `unfolded_circle_core_ws` builds `ws://remote.local/ws` for the
`CoreWs` endpoint (not `.../ws/ws`), and each other endpoint's URL equals the
URL documented for the Unfolded Circle Core API. A generator validation
emits a Warning diagnostic only when a `base_url` path ends with a segment
that is exactly equal to the endpoint path's first segment (an exact
duplicated path segment, such as `/ws` plus `/ws`). It never rejects by
substring or prefix, and never produces an Error, because repeated segments
alone are not proof of a bad URL (a legitimate `/api/api` path exists; parent
D10 and generation-integrity R11).
Validated by: L1 unit test in `schematic-definitions` asserting
`base_url` and each endpoint `path`; L1 test in `schematic-gen` asserting the
composed URL for all endpoints of all five definitions; L1 test in
`schematic-gen` that a fixture definition with an exact duplicated segment emits
the Warning, and that a base path that merely contains or prefixes an
endpoint segment as a substring (for example `/api` and `/apikeys`) emits
nothing; L1 mock-server test in `schematic-schema` connecting the
generated client to the mock and asserting the request target the server saw.

**R2. The reconnect attempt counter counts consecutive failures.** The
supervisor resets its attempt counter to zero after every successful
connection. `max_attempts` therefore bounds consecutive failed attempts, not
lifetime reconnects. With `max_attempts = 2`, a mock server that accepts a
connection and then drops it five times in a row, and accepts each redial,
does not stop the supervisor; a mock server that refuses three consecutive
connections does stop it, and the connection state becomes `Closed`.
Validated by: L1 mock-server test in `schematic-schema`
(`schematic/schema/tests/ws_runtime.rs` or a sibling file).

**R3. The lifecycle open message is re-sent on every reconnect.** If an
endpoint declares a lifecycle open message, the supervisor sends it after each
successful redial and before publishing state `Ready`. If the open message
cannot be written, the attempt counts as a failed attempt (R2 semantics), the
state is not `Ready`, and an error event is delivered to the caller. An
endpoint with no open message behaves as today.
Validated by: L1 mock-server test: the server records the first text frame
after each of three connections and asserts it is the open message each time;
a second case where the server closes during the open write.

**R4. The Bearer host template compiles.** The generated host module for a
Bearer-authenticated WebSocket API compiles and its auth check returns
`Ok(response)` for a matching bearer token and an `Err` response with status
401 for a missing or wrong token. The existing substring-asserting unit test
is replaced or supplemented by a compile test.
Validated by: L1 test in `schematic-gen` that generates a host module from a
fixture `WebSocketApi` with `AuthStrategy::BearerToken` into a temporary
crate and compiles it (use the isolated fresh-output compile check from spec
1 if it is reusable; mark the test `slow_` per the testing guide if it is
slow); L1 mock-server test connecting an unauthenticated and an authenticated
client to the generated host.

**R5. Connect-time credentials follow the shared precedence.** The
runtime-envelope spec owns the precedence rules, redaction and the managed
builder; this requirement applies them to the handshake. Each generated
WebSocket client resolves its handshake credential once, before dialing, in
this order: per-connect-call value, client-level value, environment policy
(the first non-empty variable in the declared ordered list, from the
definition's `EnvMapping`), anonymous. Exactly one credential is sent. A
rejected handshake (HTTP 401 or 403 on upgrade) is not retried with another
credential or anonymously. The client reports the auth method used through an
accessor returning `AuthMethod`; it never exposes the value.
Placement follows the definition's `AuthStrategy`: Bearer and API-key to a
header, `ApiKeyParam` to a query parameter, Basic to the `Authorization`
header from the `EnvMapping` user and password lists.
Validated by: L1 mock-server tests in `schematic-schema` with sentinel
tokens set at every level (the server sees exactly one credential, in the
stated order; removing the winner makes the next level win); a 401 case
asserting the server received exactly one upgrade request; one test per auth
strategy asserting placement (header versus query).

**R6. A missing credential fails loudly before dialing.** This matches the
runtime-envelope spec's proposed REST default (that spec owns credential
precedence and the managed builder; if it rules otherwise, this rule follows
it). If the definition's auth strategy is not `None` and no credential
resolves at any level, `connect_*` fails before sending anything with a
`MissingCredential` error (mock server records zero connections), unless the
client was built anonymous. The explicit anonymous option exists and is
reported as `AuthMethod::Anonymous`.
Validated by: L1 mock-server test (zero connections recorded); L1 unit test
for the explicit anonymous option.

**R7. Secrets never leak from connection setup; the handshake never leaves
its origin.** (The never-follow redirect default is stricter than the
`same_origin` policy; see planning item 8 and the review agenda.) The sentinel credential value appears in no `Display` or
error text, no logged URL, and no stored metadata of the WebSocket client,
including for `ApiKeyParam` (query-string) placement, for a refused
connection, and for a stalled handshake. The handshake follows no redirect: a
3xx upgrade response is an error naming the status, no second request is
made, and no credential reaches any other origin (the mock "other origin"
records zero requests).
Validated by: L1 mock-server tests with a sentinel in the query string:
connection refused (closed port), handshake stall past the timeout, and 302
to a second mock server; assertions on error text, `Debug`/`Display` of
client options, and the second server's request count. Note the parent's D22
carve-out: only Schematic's stored metadata and `Display` are guaranteed
credential-free; a server-echoed value in a body is not covered.

**R8. `receive_timeout` is enforced.** When `WsClientOptions.receive_timeout`
is `Some(d)`, the supervisor treats `d` of inbound silence (no frame of any
kind, including ping and pong) as a lost connection: an error event
`ReceiveTimeout` is delivered and the reconnect policy applies. With `None`
nothing changes. The planner no longer emits the "not yet enforced"
diagnostic.
Validated by: L1 mock-server test with a server that sends one frame and goes
silent (assert the error event and a redial); a case where periodic pings keep
the connection alive past `d`.

**R9. Plan diagnostics are surfaced through the shared diagnostics
mechanism.** The generation-integrity spec owns the diagnostics format, the
severities, the two-phase writer and the per-output validation; this spec
defines none of its own. Every `WsDiagnostic` produced while lowering a
definition (including a `lower_to_plan` `Err`) is mapped onto that format
(API, path, message, severity) and reported through it. Whether an Error
blocks writes, and what happens to the other outputs and to existing files, is
decided by that spec's writer and its per-output validation, and the WebSocket
output reuses them unchanged; this spec adds no "fail before any write" rule of
its own. Info diagnostics follow that spec's verbosity rule.
Validated by: L1 test in `schematic-gen` with a fixture definition that
produces an `Error` (for example a duplicate message name or an unresolved
`responds_with`): asserts the diagnostic appears in the shared format with the
right severity and that the shared writer's behavior for an Error applies; L1
test that a Warning is reported and does not fail generation; L1 test that a
`lower_to_plan` failure is no longer swallowed (`Err(_) => return None` is
gone).

### Typed messages and the declared response link

**R10. Typed message enums are emitted and used.** For each endpoint the
generator emits an inbound enum (messages with direction `Server` or
`Bidirectional`) and an outbound enum (`Client` or `Bidirectional`), with one
variant per `MessageSchema` named after `MessageSchema.name` and carrying the
payload type that the message's `Schema` names. The generated client's primary
surface becomes typed: `send(outbound)` and `next_event()` yielding the
inbound enum (a decode failure yields an error event carrying the raw text;
the connection continues). The untyped surface remains as an explicit escape
hatch (`send_value(serde_json::Value)`, `next_value_event()`). The name
deliberately differs from REST `send_raw` (D45), which has different semantics
(it returns status, headers and bytes); the names are a Proposed default. Lifecycle messages (open, close, keepalive) remain
separate and are not variants.
Validated by: L1 generated-output assertions in `schematic-gen` (variant
names and directions); L1 compile-fail test (trybuild or the repo's existing
compile-fail pattern) that sending a value of the wrong type does not
compile; L1 mock-server test decoding every inbound message type of each of
the five definitions from a recorded fixture frame; L1 test that a malformed
frame yields the error event with raw text and does not close the connection.

**R11. An explicit `responds_with` link replaces the string match.**
`MessageSchema` gains `responds_with: Option<String>`: on a request message
(direction `Client` or `Bidirectional`), the name of the message, in the same
endpoint, that answers it. The generator pairs request and response only
through this link and the endpoint's `CorrelationHints`. No code compares a
message name to `"RequestEnvelope"` or `"ResponseEnvelope"`. The hint-less
inference of field names `id` and `req_id` is removed; the definitions that
relied on it declare `CorrelationHints` explicitly with the same values.
Where correlation is active, the declared `RequestIdType` is honored: with
`RequestIdType::String` the generated request ids are JSON strings, with
`U64` JSON numbers. Validation (Error): `responds_with` names no message in
the endpoint, names a message whose direction cannot be received, or is set
with no `CorrelationHints` on the endpoint or API. Validation (Warning): a
declared `RequestIdType` or `CorrelationHints` that nothing consumes.
Validated by: L1 test in `schematic-gen` that the source tree contains no
string literal `"RequestEnvelope"`/`"ResponseEnvelope"` in the planner (a
grep-style test over `plan.rs`, acceptable here because the property is
absence); L1 plan tests for each Error and Warning; L1 mock-server test of a
correlated request in which the server echoes the id and the test asserts the
JSON type (string versus number) the server received; L1 test that renaming
the envelope messages in a fixture does not change correlation.

### Shared vocabulary and normalization

**R12. WebSocket reuses the REST parameter, auth, env-mapping and header
types.** `schematic_define::websocket::ConnectionParam` and `ParamType` no
longer exist in the public API. `WebSocketEndpoint.connection_params` holds
the REST `ParamDef`. `WebSocketApi` declares authentication with the REST
`AuthStrategy`, `EnvMapping` (explicit field, not derived at generation time
from `env_auth`), and the REST header type for constant handshake headers.
It carries the REST `AuthPolicy` only if spec 4 kept it on `RestApi` (see
Planning-time details). `env_auth` follows whatever spec 4 did to the REST
equivalent (retirement of `env_auth[0]`-as-password).
Validated by: L1 compile-level check in `schematic-define` (a doc-test or
trybuild case showing the old names do not resolve through
`schematic_define::websocket`); L1 tests that the five definitions build and
that `lower_to_plan` succeeds on each; L1 serde round trip of a
`WebSocketApi` (the type derives `Serialize`/`Deserialize` today); a
compile-time or unit check asserting that `ParamDef`, `QueryParamType` and
`ParamStyle` implement `Serialize` and `Deserialize` (the derives are owned by
the definition-API spec, 4a; this spec asserts them and does not add them).

**R13. The five definitions are normalized onto the shared vocabulary.** All
five (`unfolded_circle` core and dock, Samsung remote, `integration_ws`,
ElevenLabs) declare: connection parameters as `ParamDef`; auth as
`AuthStrategy` plus `EnvMapping`; handshake challenge flows with
`AuthFlowHints` (never as prose only); message directions that are true to
the protocol (no `Bidirectional` used to trip a heuristic, and the test that
asserts that contortion is removed or rewritten to assert the true
directions); correlation with explicit `CorrelationHints` and `responds_with`
where the protocol correlates, and nothing where it does not; and heartbeat
with `HeartbeatHints` where the protocol has one. The ElevenLabs *definition* already
uses `schematic_define::WebSocketApi`; the implementer confirms that only its
message payload types are standalone and brings them onto the same footing if
so.
Validated by: L1 definition-conformance test in `schematic-definitions`
iterating all five and asserting the rules mechanically (no prose-only auth
flow; every `responds_with` resolves; no endpoint with both ends marked
`Bidirectional` solely to satisfy a heuristic is detectable, so assert true
direction against a per-definition expected table kept in the test).

**R14. The generated host carries no Unfolded Circle assumptions.** The host
module's event hub is named from the API (`{ApiName}EventHub`, where
`{ApiName}` is the PascalCase API name the generator already uses for client
and host struct names), not the literal `UnfoldedCircleEventHub`. The host no
longer sends a hard-coded Unfolded Circle authentication success envelope:
post-authentication behavior is a handler hook (see Design), implemented in
the Unfolded Circle integration crates, not in the generator.
Validated by: L1 generated-output test in `schematic-gen` over a non-Unfolded
fixture definition: no `UnfoldedCircle` string appears in its host module;
L1 mock-server test: a client authenticates to a generated host whose handler
returns a custom post-auth message and receives exactly it, and a handler that
returns none receives no extra frame.

### Dependents, harness and documentation

**R15. Homelab consumers are migrated and every dependent builds.** These
consume the generated WebSocket modules and are migrated in this spec:
`homelab/lib` (`src/samsung_tv.rs`, which imports
`schematic_schema::samsung_smart_tv_remote_ws`) and the four integration
crates `homelab/eversolo-integration`, `homelab/arcam-amp-integration`,
`homelab/sony-receiver-integration` and `homelab/unfolded-integration-helper`
(consumer of `unfolded_circle_integration_ws`). `homelab/cli` and
`homelab/server` have no WebSocket client consumers (**verified**) and are not
migrated. Each migrated package builds and its tests pass. `sniff`,
`biscuit-speaks` and `model-citizen` (path dependents of `schematic-schema`)
still build and pass their tests.
Validated by: `just test {pkg}` from the repo root for each named package
area on the current host; a grep-based L1 or script check that the removed
names (`UnfoldedCircleEventHub` as a generated name, `ConnectionParam`,
`ParamType`, untyped `send(Value)` as the primary surface of generated WS
clients) no longer appear in non-generated source.

**R16. The mock-server harness exists and is focus-safe.** A reusable test
helper starts a loopback WebSocket server on port 0 that can: record upgrade
requests (path, query, headers), script per-connection behavior (accept, drop
after N frames, refuse, stall, redirect, send frames), count connections, and
shut down deterministically. It is used by every mock-server requirement
above. It binds only to `127.0.0.1`, uses no fixed port, needs no terminal and
no network, and no test it supports takes window focus. Tests that need no
terminal are L1; there is no L2 test in this spec unless the implementer finds
one is required (then it follows the L2 focus rule and is logged).
Validated by: the harness's own L1 tests (records a request, scripted drop,
port 0 reuse safety); the CI parity rule that tests pass on macOS, Linux,
Windows and WSL2 (no Unix-only sockets, no fixed ports, no reliance on
timing under 100 ms margins without a tolerance; see the `os` skill).

**R17. Generated output, docs and skills are current.** `schematic-schema` is
regenerated with the generator, never hand-edited, and `just lint-schema`
passes. The `schematic` README, the relevant `schematic/docs` topic pages,
and the affected `.claude/skills` entries (`schematic-define`, `homelab`,
`unfolded-circle`, and any that describe the WebSocket surface) describe the
new behavior; docs do not link to or name this spec.
Validated by: `just lint-schema`; the repo's drift check
(`artifact_drift`, via `just check-drift` in `schematic/justfile`); a manual
pass recorded in the log listing each doc and skill touched.

## Design

Marked **Decided** only where the parent decided it; everything else is
**Proposed** and is the default adopted and logged by the implementer.

| Symbol | Package | What it does | Status |
|---|---|---|---|
| `WebSocketApi` (fields) | `schematic-define` | Gains `env_mapping: Option<EnvMapping>`, constant handshake `headers` (REST header type), and `auth_policy` only if REST keeps it; `env_auth` follows REST. Keeps `auth: AuthStrategy`. Stays its own type. | Own type: Decided (D8). Field set: Proposed |
| `WebSocketEndpoint::connection_params` | `schematic-define` | Becomes `Vec<ParamDef>` (REST type). | Decided (D8) |
| `ConnectionParam`, `ParamType` | `schematic-define` | Removed from the public API. | Decided (D8) |
| `ParamDef` serde | `schematic-define` | `ParamDef`, `QueryParamType`, `ParamStyle` derives are owned by spec 4a (they derived no serde traits when checked). This spec only asserts they exist (compile-time or unit check) because `WebSocketApi` derives `Serialize`/`Deserialize`; it does not add them. | Owned by 4a; assertion Proposed |
| `MessageSchema::responds_with` | `schematic-define` | Optional name of the answering message (R11). | Link: Decided (D8). Name and shape: Proposed |
| `CorrelationHints`, `RequestIdType`, `HeartbeatHints`, `AuthFlowHints` | `schematic-define` | Existing hint types, now consumed; unconsumed declarations warn. | Proposed |
| `WsDiagnostic`, `WsSeverity` | `schematic-gen` | Existing; gain an `api` field (API name) if needed, and are mapped onto the generation-integrity spec's diagnostics format. No WebSocket-specific format. | Proposed |
| `lower_to_plan` | `schematic-gen` | Collects all diagnostics (not only the first error), returns them with the plan or the failure. | Proposed |
| `ws_modules` assembler | `schematic-gen` | Propagates `Err`; reports diagnostics through the shared mechanism; reuses the generation-integrity spec's two-phase writer and per-output validation. | Proposed |
| Message enum emitters | `schematic-gen` (`ws_codegen/client.rs`) | Emit `{Endpoint}ServerMessage` and `{Endpoint}ClientMessage`, serde-decoded as untagged in declaration order, with the raw frame always retained for the error path. | Proposed |
| Supervisor loop template | `schematic-gen` (`ws_codegen/client.rs`) | Reset-on-success counter, open replay before `Ready`, idle receive timeout. | Behavior: Decided (D8 defect list). Details: Proposed |
| Handshake credential resolution | `schematic-gen` (`client.rs`) emitting calls into `schematic-runtime` | Resolve once with D11 precedence, place per `AuthStrategy`, report `AuthMethod`. | Rules: Decided (D11, D48). Reuse of runtime functions: Proposed |
| Bearer host template | `schematic-gen` (`ws_codegen/shared.rs`) | Remove the trailing `;` defect; add a compile test. | Decided (fix); test shape Proposed |
| Host module | `schematic-gen` (`ws_codegen/host.rs`) | Event hub named `{ApiName}EventHub`; `WsHandler` gains `on_authenticated` hook returning an optional outbound message; no fixed envelope. | Proposed |
| `WsClientOptions` | `schematic-schema` (generated `ws_shared`, from `ws_codegen/shared.rs`) | `receive_timeout` enforced; gains explicit anonymous option and credential setters. | Proposed |
| Mock WS server helper | `schematic-schema` tests (and `schematic-gen` tests that need it) | See R16. Built on `tokio-tungstenite`, already a dependency of `schematic-schema`. | Proposed |

Notes for the implementer:

- Generated clients keep depending on the shared runtime (D39). If the runtime
  exposes credential resolution and redaction only in HTTP-shaped functions,
  extract the transport-neutral part inside `schematic-runtime` rather than
  copying it into generated text, and log that you did so.
- Typed enum payload types come from each message's `Schema` name. Resolve
  names the way spec 4 resolves REST schema names (typed descriptors, D2 and
  D44) if spec 4 extended that to `MessageSchema`; otherwise keep today's
  name-based resolution and log the gap.
- Enum decoding has no declared discriminator today (the parent models none).
  The default is untagged decoding in declaration order. This can misdecode
  when two payload shapes overlap; the implementer adds a fixture test for
  each definition and logs any ambiguity found instead of inventing a
  discriminator field.

## Suggested phases

An outline; the plan may refine it. Each phase leaves the tree green. A phase
that changes generator output regenerates `schematic-schema` in that same
phase so `just lint-schema` and the drift test pass at the phase boundary.

1. **Harness and diagnostics (R16, R9).** Build the mock-server helper with
   its own tests. Surface `WsDiagnostic`s through the shared diagnostics
   mechanism.
   Remove the `receive_timeout` TODO diagnostic only in phase 3.
2. **Quick defects (R1, R4).** Correct the Core URLs (verify device paths
   first), add the base-path overlap Warning, fix the Bearer template, add the
   compile test. Regenerate.
3. **Supervisor (R2, R3, R8).** Counter reset, open replay, receive timeout;
   mock-server tests; regenerate.
4. **Shared vocabulary (R12, R13).** Change `schematic-define`, update the
   five definitions, the planner, the AsyncAPI importer's construction of
   `WebSocketApi`, and the README; remove `ConnectionParam`/`ParamType`. This
   is the widest compile change, so it is one phase with no behavior change
   beyond the vocabulary. Regenerate.
5. **Handshake auth (R5, R6, R7).** Credential resolution, placement, auth
   method accessor, redaction, no-redirect behavior. Regenerate.
6. **Declared link (R11).** `responds_with`, explicit correlation hints,
   removal of the name match, request-id type honored, validations. Update
   definitions that relied on inference. Regenerate.
7. **Typed surface and host (R10, R14).** Emit enums, typed `send`/`next_event`
   and raw escape hatch, `{ApiName}EventHub`, `on_authenticated`. Regenerate.
8. **Dependents (R15).** Migrate the homelab crates; build and test every
   dependent.
9. **Docs, skills, drift pass, matrix (R17).** README, topic pages, skills;
   comment-drift pass over every symbol whose behavior changed; run the full
   acceptance matrix; finish the implementation log.

## Acceptance criteria and definition of done

The spec is done ("implementation complete, ready for review") when all of
the following hold and are recorded in `implementation-log.md`:

- R1 to R17 each have a passing validating test or, for R17, a recorded manual
  pass.
- The workspace builds and its tests pass on macOS, Linux, native Windows and
  WSL2 per the repo rules. Evidence is gathered with the `os` skill (hosts and
  methods); where a host cannot be reached, the log says "not verified on X"
  and why instead of pausing. CI is not triggered speculatively (repo CI
  discipline); local runs may be as wide as useful.
- `just test` and `just lint` pass for the `schematic` area, and
  `just lint-schema` passes.
- `schematic-schema` was regenerated by the generator, never hand-edited, and
  the drift check is clean.
- Every dependent builds and passes its tests: `sniff`, `biscuit-speaks`,
  `model-citizen`, and the homelab crates in R15.
- README, `schematic/docs` topic pages and affected `.claude/skills` are
  updated for changed behavior; docs describe behavior and never name this
  spec; behavior decided but unbuilt is marked **planned**.
- A comments and docs drift pass covers every symbol whose behavior changed
  (code wins over comment; drift found and how it was resolved is logged).
- Ledger rows 3 and 9 to 12 are each recorded closed with the test that
  proves it, or recorded as an accepted gap with a reason (parent D20 requires
  this).
- No commit, push, pull request, `just complete`, or move to `_completed` is
  performed by the agent unless the plan says otherwise.

## Pre-authorizations

These steps would otherwise pause an unattended run. The author must confirm
them before the unattended run (D50 item 8); until then none is in force. The
specifics are listed below.

- Regenerating `schematic-schema` with the generator, repeatedly, in the
  phases listed.
- Editing dependent crates as R15 requires: the five homelab packages named
  there, and (only if a build break requires it) `sniff`, `biscuit-speaks`,
  `model-citizen`, with the minimal edit.
- Removing public API: `ConnectionParam`, `ParamType`, the untyped
  `send(Value)`/`next_event()` as primary surface, the generated name
  `UnfoldedCircleEventHub`.
- Adding a minimal transport-neutral function to `schematic-runtime` if
  needed (see Design notes). The serde derives on REST parameter types are
  owned by spec 4a and not added here; if they are missing, the log records an
  open question and the work stops short of adding them.
- Adding dev-dependencies only from crates already used in the workspace
  (for example `tokio-tungstenite`, `tokio`); no new third-party crate is
  pre-authorized. If one is truly needed the implementer logs it as an open
  question and proceeds without it where possible.
- Editing definitions in `schematic-definitions` for the five WebSocket APIs.
- Editing the README, `schematic/docs` and `.claude/skills` as R17 requires.
- Running builds and tests on other OS hosts reachable per the `os` skill
  (build, test and read only).
- Not pre-authorized: deleting stale files under `gen`/`schematic` output
  (stale-output deletion is not authorized by this spec); contacting a real device or provider; committing;
  pushing; running the full-scope CI; weakening or deleting a test other than
  the one that asserts the `Bidirectional` contortion (R13); bypassing signing.

## Dependencies and ordering

- **Frontmatter dependency:** `2026-10-11-schematic-response-model-graphql`
  must have landed and passed its review 1. It sits after the definition-API
  spec (builders, typed registration, `oauth_scopes`/`auth_policy` decision),
  so this spec needs the settled REST vocabulary. The parent's own D50 line
  says WebSocket needs only the definition-API spec; the stricter ordering
  here is the author's.
- **Transitive prerequisites** (landed through the chain, not listed in
  `depends_on`; specs 1, 2, 4a and 4b): the generation-integrity spec (its
  isolated fresh-output compile check, diagnostics format, two-phase writer and
  per-output validation, used by R4 and R9) and the runtime-and-envelope spec
  (the `schematic-runtime` package, credential precedence, the managed builder
  and its TLS settings, redaction, `AuthMethod`, used by R5 to R7). The
  definition-API and definition-vocabulary specs (4a, 4b) own the REST
  parameter, auth and header types this spec reuses.
- **Followed by** the fidelity-and-polish spec, whose AsyncAPI exporter
  exports the five definitions in the shape this spec leaves. Anything this
  spec renames or removes is that spec's input.
- Sniff's retirement of its hand-written provider client (D20) does not
  depend on this spec.

## Planning-time details assigned to this spec

Every item has a default; the implementer adopts it unless the author rules
otherwise before planning, and logs the adoption.

1. **Shape and name of the explicit response link (P10).** *Default if not
   ruled before planning:* `MessageSchema.responds_with: Option<String>` on the
   request message, naming the answering message in the same endpoint; no
   many-to-many form; one request has at most one declared response message.
2. **Correlation without hints.** *Default:* correlation is active only when
   `CorrelationHints` exist (endpoint-level, else API-level); `responds_with`
   without hints is an Error; hints without any `responds_with` pair
   correlate nothing and Warn.
3. **Diagnostics format and failure behavior (P10 item, parent question 14).**
   *Default:* the generation-integrity spec owns the format, severities,
   two-phase writer and per-output validation. Map `WsDiagnostic` onto that
   format and reuse the writer and validation; add no WebSocket-specific
   format or write rule. Log the mapping. If that spec's outcome is unavailable
   at planning time, stop and refresh this item rather than inventing a format.
4. **Which side of the double `/ws` is fixed.** *Default:* `base_url` becomes
   the origin (`ws://remote.local`) and each endpoint `path` is the full
   documented path; the other Core endpoints (`/intg`, `/profiles`) take the
   documented paths.
5. **Reconnect counter semantics.** *Default:* reset to zero on a successful
   dial and open-message write; incremented on each failed dial or failed open
   write; `max_attempts` bounds consecutive failures.
6. **Where the open message is replayed.** *Default:* after the dial succeeds
   and before `Ready`; replay of an `AuthFlowHints` challenge exchange after a
   reconnect is **not** added unless the existing client already handles
   challenges (**unverified**; check and log).
7. **Missing credential behavior (R6).** *Default:* match the
   runtime-envelope spec's proposed REST default: fail before sending with
   `MissingCredential` unless the client was built anonymous. That spec owns
   credential precedence and the managed builder. The parent's D11 puts
   anonymous last in the precedence; this default reads that as "anonymous only
   when chosen or when the definition declares no auth". If the runtime spec
   rules differently, follow it. Flag for review.
8. **Handshake and the cross-origin policy.** *Default:* no `cross_origin`
   field on `WebSocketApi`; the client connects only to its configured base
   origin plus endpoint path, never follows a redirect, and treats a 3xx
   upgrade response as an error. This is stricter than the `same_origin`
   default and than D31's "the cross-origin rule always holds"; it is kept
   deliberately and flagged in the review agenda as a candidate deviation.
9. **`receive_timeout` semantics (R8).** *Default:* idle inbound timeout
   across all frame types, including ping and pong, reset on every inbound
   frame; expiry is an error event then the reconnect policy.
10. **Typed enum naming and decoding.** *Default:* `{Endpoint}ServerMessage`
    and `{Endpoint}ClientMessage`; variants named by `MessageSchema.name`;
    `Bidirectional` messages appear in both; untagged decode in declaration
    order; failure yields an error event with the raw text.
11. **Untyped surface.** *Default:* keep `send_value(Value)` and
    `next_value_event()` as explicit escape hatches; the unsuffixed names become
    typed. `send_raw` is not used because D45 gives that name to the REST
    escape hatch with different semantics (status, headers, bytes).
12. **`WebSocketApi` field set after reuse.** *Default:* add `env_mapping` and
    `headers`; add `auth_policy` only if spec 4 kept `RestApi::auth_policy`
    (P12); if spec 4 removed it, WebSocket uses `AuthStrategy` plus
    `EnvMapping`; `env_auth` follows REST.
13. **Mapping the removed `ParamType` onto `QueryParamType`.** *Default:*
    `String`, `Integer`, `Boolean` map by name; `Float` maps to `Number`
    (**verified**: the REST type has `Number`, not `Float`). Path placeholders
    use `ParamDef` too unless spec 4 introduced a distinct typed path-parameter
    type, in which case use it.
14. **Host post-authentication behavior.** *Default:* `WsHandler` gains an
    `on_authenticated` hook returning `Option<outbound message>`, default
    `None`; the Unfolded Circle integration handlers in homelab implement it
    to send what the hard-coded envelope sent.
15. **Event hub name.** *Default:* `{ApiName}EventHub` using the PascalCase
    API name the generator already applies to client and host names.
16. **TLS acceptance for `wss://` to LAN devices.** *Default:* the
    accept-invalid-certificates option, if the Samsung migration needs it, comes
    from the runtime builder's opt-in setting (owned by the runtime-envelope
    spec) and is passed through; this spec adds no WebSocket-only switch. If the
    runtime builder has no such setting, do not add one, and log.
    (`homelab/lib/src/samsung_tv.rs` sets `danger_accept_invalid_certs(true)`
    on its REST client for HTTPS; whether its `wss://` path works today without
    an equivalent is **unverified**.)
17. **Samsung correlation.** *Default:* declare `CorrelationHints` and
    `responds_with` for Samsung only if the protocol notes in the repo
    (`schematic/docs/research/samsung-design.md`) show request and response
    ids; otherwise declare no correlation, and the dead `RequestIdType` then
    triggers the "unconsumed declaration" Warning, resolved by removing it.
18. **Test levels.** *Default:* all mock-server tests are L1 (loopback, no
    terminal); no L2 test is added.
19. **Mock server construction.** *Default:* build the helper on
    `tokio-tungstenite`'s server side, in `schematic-schema`'s test support,
    shared with `schematic-gen` tests by a dev-only module or small
    dev-dependency path; no new third-party crate.
20. **ElevenLabs normalization scope.** *Default:* normalize what the
    `WebSocketApi` definition declares (params, auth, env mapping) and the
    standalone message payload types, if they do not use `schematic_define`; do not change its protocol
    behavior.

## Risks and failure modes

- **Overlapping payload shapes break untagged decoding.** A message may decode
  as the wrong variant. Mitigation: per-definition fixture frames; log
  ambiguities; do not invent a discriminator.
- **Spec 4 or 5 may have moved the REST types.** `ParamDef`, `EnvMapping`,
  `AuthPolicy` names and serde support may differ from the tree today. See
  "Refresh notes".
- **Changing generated public names breaks homelab at compile time.** Intended;
  R15 migrates them. Order matters: regenerate and migrate in the same phase
  boundary where practical, or keep the tree green with a temporary shim that
  is removed in the same spec.
- **Timing flakiness on Windows and WSL2.** Reconnect, timeout and stall tests
  depend on timers. Use generous tolerances, injected short backoffs, and
  deterministic server scripts; follow the `os` skill.
- **A credential over cleartext `ws://`.** Local devices use `ws://`; the
  credential is sent in clear. Not changed here (D28 does not address it);
  documented in the topic page. **Unverified** whether the author wants a
  warning.
- **Unverified claims.** Whether device paths for `/intg` and `/profiles`
  carry a `/ws` prefix; whether the client already replays auth-flow
  challenges; whether `homelab/cli` consumes WebSocket modules; whether the
  generated Samsung `wss://` connection works without an
  accept-invalid-certificates option.
- **Silent skip of the compile test.** If the isolated compile check is slow
  and gated, R4 could pass by not running. The log records the command and
  result of the compile run.
- **Scope creep into the exporter.** Renames made here change what the
  AsyncAPI exporter (next spec) reads. Keep names stable once chosen.

## Process

This spec is built for the following process (D51, user-confirmed):

1. Each spec has a 1:1 plan. The plan is implemented end to end, phase by
   phase, **without a human in the loop**.
2. Then **review 1**: time is set aside for the author to review the decisions
   made during implementation and any open design questions.
3. Then the **next** spec and plan are refreshed from the outcomes of review 1.
4. Then fix and review cycles repeat until the reviewer deems the spec
   production ready.

An agent's terminal state is "implementation complete, ready for review". Only
the author moves a spec to `_completed`; an agent never does and never runs
`just complete`.

Because no human is available while the plan runs: this spec restates every
decision the implementer needs (above); every remaining planning-time detail
has a default flagged "Default if not ruled before planning", which the
implementer adopts and logs; and the implementer records the items in the
agenda below in `implementation-log.md` **as they occur**, not at the end. If
something is unclear, adopt the safest reading, log it as an open question,
and continue; do not stop to ask.

## Human review agenda

The implementer logs each of these in `implementation-log.md` for review 1:

1. Every default adopted from "Planning-time details assigned to this spec",
   with the number, the alternative considered, and why the default held.
2. Every design choice not dictated by this spec (names, field shapes,
   error kinds, file layout, helper placement).
3. Every deviation from this spec or the parent's decisions, and the reason.
4. Every open question found, with the evidence and the safest reading used.
5. Specifically: the missing-credential rule (item 7, R6), the redirect
   default (item 8; never follow, a 3xx upgrade is an error) as a candidate
   deviation from `same_origin` and D31's "always holds", the `responds_with`
   shape (item 1), the untagged typed-message decoding risk (item 10; no
   discriminator), the `send_value` / `next_value_event` naming (item 11), TLS
   for `wss://` (item 16), the `auth_policy` outcome (item 12), the Samsung
   correlation decision (item 17), the corrected device paths and their
   evidence (item 4), and the ElevenLabs finding (definition uses
   `WebSocketApi`; payload types standalone).
6. Any `schematic-runtime` change made on behalf of this spec.
7. Which OS hosts produced evidence and which did not.
8. Each ledger row (3, 9, 10, 11, 12) with its closing test or accepted gap.
9. Any test that could not be made deterministic, and what was done.
10. Doc and skill files touched, and any drift found and its resolution.

## Refresh notes

This spec depends on earlier specs' review 1 outcomes and must be refreshed
before its plan runs. Things that may change it:

- **From the response-model-and-GraphQL spec and the definition-API spec:**
  final names and serde support of `ParamDef`, header types, `EnvMapping`;
  whether `AuthPolicy` survived (P12); builder shapes (the five WebSocket
  definitions may need builders or a context parameter, D15 and D2); the
  `env_auth[0]` retirement; typed path parameters.
- **From the runtime-and-envelope spec:** final runtime package name, whether
  credential resolution, redaction and `AuthMethod` are reusable outside HTTP
  send, `TransportError` kinds (the missing-credential kind in R6), the
  managed-builder TLS option name (item 16), and any middleware hooks.
- **From the generation-integrity spec:** the validation-diagnostic format
  (item 3) and the isolated fresh-output compile check (R4).
- **From any review 1:** author rulings on items in the agenda above, which
  replace the defaults here.
- The parent's open items P1 (cross-origin details) and P10 may be ruled
  before planning; if so they replace items 3 and 8.

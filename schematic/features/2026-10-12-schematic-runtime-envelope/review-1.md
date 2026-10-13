---
$schema: feature-review.yaml
ready: false
findings: []
observations: []
human_in_the_loop:
    - |-
        Keep the generated API marker and client alias instead of the specified wrapper?

        The schematic-gen package now generates an API marker such as `GitHub` and a `GitHubClient` alias for the actual client. The spec's supplied default instead describes a per-API struct holding a runtime client. A consumer that stores a client now names `GitHubClient`; `GitHub::builder()` remains the entry point. The implementer also made construction fallible rather than silently falling back to another HTTP client.

        This meets test 1 (public names used across crates) and test 3 (later generation work inherits the representation). This is a departure from the supplied default, not a request to reaffirm that default.

        - Keep the marker and alias: avoids forwarding roughly fifteen builder methods for each of eighteen APIs and preserves compile-time API identity. Consumers must distinguish the API marker from its client type.
        - Restore wrappers: follows the original representation and keeps a single per-API instance type, but requires forwarding or additional conversion code and revisiting migrated consumers.

        Recommendation: keep the marker and alias, and keep construction fallible. If accepted, the repair records this representation in the current documentation and the next pipeline handoff. Until decided, consumer type names and later generator work risk being rewritten.
    - |-
        Keep the new decoder and hook extension contract for later codecs?

        The schematic-runtime package's [Decode](../../runtime/src/codec.rs) trait now has a type-level `decode(body)` function with no codec instance. Its input, `LiveBody`, owns a byte stream and timing bounds rather than exposing the HTTP library's response. A provided `decode_hooked` method delivers hooks; `SupportsHooks` controls registration. The separate [ErrorBody](../../runtime/src/endpoint.rs) trait decodes declared failures. The spec requires a live decoder that permits later streaming and multi-shape responses, but does not supply these exact signatures. The later response-model spec will implement against them.

        This meets test 1 (public extension interfaces) and test 3 (later codecs depend on the chosen interfaces).

        - Keep this contract: isolates codecs from the HTTP library and lets codecs without hooks use the default hook method. Stateful decoders must place state in their output or elsewhere rather than in a codec instance.
        - Use an instance-based decoder with an explicit context argument: makes per-decoder state and hook context more direct, but changes runtime codecs, generated associations, and the later implementation plan.

        Recommendation: keep the type-level decoder, owned live body, separate error-body trait, and provided hook method. If accepted, the repair makes these exact interfaces the later codec handoff. Until decided, streaming and GraphQL implementation could build on an interface the author later replaces.
    - |-
        Make connect timeout a shared-transport setting and reject all attached-client transport options?

        In schematic-runtime, a `SharedTransport` shares HTTP connections among clients. The supplied default lists `connect_timeout` on `ClientBuilder`; the spec expressly leaves its shared ownership for review. The implementer's dependency investigation found that the HTTP library sets connect timeout on its connection-owning client. Accordingly, a shared transport now owns it, and attaching a client while also specifying connect timeout, proxy, or TLS options fails even when the supplied setting matches.

        This meets test 1 (public builder/error behavior) and test 4 (routing and TLS configuration must have one unambiguous owner).

        - Keep shared ownership and reject every attached-client transport option: simple, avoids silently ignoring settings, and fits the dependency's model. Clients sharing connections cannot choose different connect timeouts.
        - Permit matching settings on attached clients: more permissive, but requires comparisons and an equality contract, especially for certificates.
        - Require separate transports when clients need different connect timeouts: preserves independent values at the cost of connection reuse; it can accompany the first option.

        Recommendation: keep shared ownership and unconditional rejection; use separate transports for different settings. If accepted, the repair documents this ownership as settled. Until decided, builder calls and shared-transport error behavior remain unstable.
    - |-
        Require the endpoint type when checking a pagination link?

        The supplied default is `Client::check_link(&Url)`, usable before constructing a request. The implementation log records [Client::check_link::<R>(&Url)](../../runtime/src/client.rs) in schematic-runtime instead. `R` identifies the endpoint so a refused link's error can report its real HTTP method and endpoint name. For example, a pager checks a URL using its listing-request type even though no request has been sent.

        This meets test 1 (public signature) and test 3 (the next pagination spec must adopt it).

        - Keep the endpoint-type argument: provides meaningful refusal metadata without another context object. Every caller must know the endpoint type.
        - Restore a type-free check: easier for general URL screening, but needs an explicit metadata argument or a different error contract to avoid inventing endpoint identity.

        Recommendation: keep the endpoint-type argument and the policy-checked absolute-URL request path described in the log. If accepted, the next pagination handoff uses its request type when checking links and when following absolute links. Until decided, pagination's link-check call sites and error mapping risk rework.
    - |-
        Allow the ElevenLabs endpoint-definition edit that was outside the approved scope?

        The spec explicitly says, "Do not edit the endpoint literals (547; openai 265)." The log nevertheless records adding `page_size` and `page_token` to the ElevenLabs `ListVoices` endpoint in schematic-definitions. These parameters let biscuit-speaks replace a hand-written voices request with generated typed sends while preserving its existing pagination. This changes what the generated request type can express.

        This meets test 2 (departure from an explicit scope restriction) and test 1 (generated public request methods).

        - Accept this narrow exception: removes the consumer's transport bypass and represents the parameters it already sends. It makes an exception to the endpoint freeze.
        - Revert and defer the endpoint edit: preserves the freeze, but needs another safe migration for the existing voices listing; retaining unmanaged HTTP access conflicts with this feature's transport goal.

        Recommendation: accept only these two existing query parameters as a scope exception. If accepted, retain the typed migration and record the departure in the implementation log; do not authorize unrelated endpoint changes. Until decided, the migrated voices listing depends on an unapproved definition change.
    - |-
        Retain shared-file compatibility checks alongside the new runtime ABI marker?

        The spec's dependency section says this feature "defines and persists the marker and replaces that comparison." The implementer retained the existing byte comparisons of generated `shared.rs` and `ws_shared.rs` beside the new ABI check in schematic-gen. The stated reason is that retained generated modules still call helpers in those files: matching the runtime's ABI number does not prove those helpers stayed compatible.

        This meets test 2 (departure from the stated replacement instruction). It also affects the workflow later generation changes use to decide whether a partial regeneration is safe.

        - Keep both checks: protects compatibility of retained generated modules as well as the hand-written runtime. Harmless shared-file edits may still require full generation.
        - Use only the runtime marker: follows the planned simplification, but needs evidence that every relevant shared-helper change is represented by that marker or another compatibility mechanism.

        Recommendation: keep both checks while generated modules depend on those helpers. If accepted, the repair records the two distinct compatibility questions and updates later planning assumptions. Until decided, partial generation can either remain unnecessarily restrictive or lose a protection its retained modules still need.
    - |-
        Change the required builder-test evidence, or repair the two proposed testing gaps?

        The spec requires each builder setting to be "observed by a local mock" and says a test failing on one OS is investigated, "not skipped." The log reports two changed premises: no portable loopback fixture was found that observes the connect timeout taking effect, and four TLS builder tests abort intermittently on native Windows with `0xc0000409`. Investigation reportedly reproduced the abort with plain reqwest; the tests are now ignored there and run on macOS, Linux, and WSL2. This is a request to change required evidence, not a claim that only a human can run tests.

        This meets test 2 (required validation is omitted) and test 4 (the missing cases concern network timeout and TLS behavior). Cross-OS receipts themselves are not a readiness gate in this review.

        - Accept both as bounded gaps: avoids a dependency investigation delaying the pipeline, but leaves the connect-timeout effect unobserved and native-Windows TLS behavior unproved under the failing conditions.
        - Accept only the connect-timeout gap and require repair of Windows TLS coverage: recognizes the loopback limitation without normalizing a permanent platform skip. Repair may require a better fixture or dependency diagnosis.
        - Keep both original evidence requirements: strongest assurance, but the next repair must find a portable effect test and resolve or safely replace the aborting TLS fixture.

        Recommendation: accept the narrowly documented connect-timeout effect gap; do not accept permanent Windows TLS exclusions. Require a repair that restores meaningful coverage, with the existing OS-skill investigation as its starting point. If accepted, the following review treats restored TLS coverage as a repair obligation and records the limited connect-timeout assumption explicitly. Until decided, no reviewer may turn a proposed gap into an accepted one.
human_resolved: true
human_review: true
human_review_items:
    - Decide the seven interface, scope, compatibility, and validation questions recorded in human_in_the_loop and Human rulings.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-12T16:05:21-07:00"
spec: "2026-10-12-schematic-runtime-envelope/spec.md"
implemented: false
description: "A **feature** review of `2026-10-12-schematic-runtime-envelope/spec.md`"
feature: "2026-10-12-schematic-runtime-envelope/review-1.md"
next_spec: "schematic/features/2026-10-12-schematic-endpoint-methods/spec.md"
next_plan: "schematic/features/2026-10-12-schematic-endpoint-methods/plan.md"
---

# Runtime envelope review 1

This review pauses for seven author decisions before comparing the implementation with the specification. **Production readiness has not been assessed.** `ready: false` and an empty finding list mean that the implementation review has not happened; they do not mean that the implementation failed review or has no defects. No readiness tests were run. The next pagination spec and plan were not refreshed.

Two agents independently read the exact specification and implementation-log paths for this feature. The classifications below use the log's recorded decisions and the spec's wording, rather than findings from implementation comparison. The questions are ranked by the reach of a later reversal. Adopted defaults and settled author decisions are informational, even when the log requests their confirmation.

## Decisions requiring the author

### Item 1: Keep the generated API marker and client alias instead of the specified wrapper?

The schematic-gen package now generates an API marker such as `GitHub` and a `GitHubClient` alias for the actual client. The spec's supplied default instead describes a per-API struct holding a runtime client. A consumer that stores a client now names `GitHubClient`; `GitHub::builder()` remains the entry point. The implementer also made construction fallible rather than silently falling back to another HTTP client.

This meets test 1 (public names used across crates) and test 3 (later generation work inherits the representation). This is a departure from the supplied default, not a request to reaffirm that default.

- Keep the marker and alias: avoids forwarding roughly fifteen builder methods for each of eighteen APIs and preserves compile-time API identity. Consumers must distinguish the API marker from its client type.
- Restore wrappers: follows the original representation and keeps a single per-API instance type, but requires forwarding or additional conversion code and revisiting migrated consumers.

Recommendation: keep the marker and alias, and keep construction fallible. If accepted, the repair records this representation in the current documentation and the next pipeline handoff. Until decided, consumer type names and later generator work risk being rewritten.

### Item 2: Keep the new decoder and hook extension contract for later codecs?

The schematic-runtime package's [Decode](../../runtime/src/codec.rs) trait now has a type-level `decode(body)` function with no codec instance. Its input, `LiveBody`, owns a byte stream and timing bounds rather than exposing the HTTP library's response. A provided `decode_hooked` method delivers hooks; `SupportsHooks` controls registration. The separate [ErrorBody](../../runtime/src/endpoint.rs) trait decodes declared failures. The spec requires a live decoder that permits later streaming and multi-shape responses, but does not supply these exact signatures. The later response-model spec will implement against them.

This meets test 1 (public extension interfaces) and test 3 (later codecs depend on the chosen interfaces).

- Keep this contract: isolates codecs from the HTTP library and lets codecs without hooks use the default hook method. Stateful decoders must place state in their output or elsewhere rather than in a codec instance.
- Use an instance-based decoder with an explicit context argument: makes per-decoder state and hook context more direct, but changes runtime codecs, generated associations, and the later implementation plan.

Recommendation: keep the type-level decoder, owned live body, separate error-body trait, and provided hook method. If accepted, the repair makes these exact interfaces the later codec handoff. Until decided, streaming and GraphQL implementation could build on an interface the author later replaces.

### Item 3: Make connect timeout a shared-transport setting and reject all attached-client transport options?

In schematic-runtime, a `SharedTransport` shares HTTP connections among clients. The supplied default lists `connect_timeout` on `ClientBuilder`; the spec expressly leaves its shared ownership for review. The implementer's dependency investigation found that the HTTP library sets connect timeout on its connection-owning client. Accordingly, a shared transport now owns it, and attaching a client while also specifying connect timeout, proxy, or TLS options fails even when the supplied setting matches.

This meets test 1 (public builder/error behavior) and test 4 (routing and TLS configuration must have one unambiguous owner).

- Keep shared ownership and reject every attached-client transport option: simple, avoids silently ignoring settings, and fits the dependency's model. Clients sharing connections cannot choose different connect timeouts.
- Permit matching settings on attached clients: more permissive, but requires comparisons and an equality contract, especially for certificates.
- Require separate transports when clients need different connect timeouts: preserves independent values at the cost of connection reuse; it can accompany the first option.

Recommendation: keep shared ownership and unconditional rejection; use separate transports for different settings. If accepted, the repair documents this ownership as settled. Until decided, builder calls and shared-transport error behavior remain unstable.

### Item 4: Require the endpoint type when checking a pagination link?

The supplied default is `Client::check_link(&Url)`, usable before constructing a request. The implementation log records [Client::check_link::<R>(&Url)](../../runtime/src/client.rs) in schematic-runtime instead. `R` identifies the endpoint so a refused link's error can report its real HTTP method and endpoint name. For example, a pager checks a URL using its listing-request type even though no request has been sent.

This meets test 1 (public signature) and test 3 (the next pagination spec must adopt it).

- Keep the endpoint-type argument: provides meaningful refusal metadata without another context object. Every caller must know the endpoint type.
- Restore a type-free check: easier for general URL screening, but needs an explicit metadata argument or a different error contract to avoid inventing endpoint identity.

Recommendation: keep the endpoint-type argument and the policy-checked absolute-URL request path described in the log. If accepted, the next pagination handoff uses its request type when checking links and when following absolute links. Until decided, pagination's link-check call sites and error mapping risk rework.

### Item 5: Allow the ElevenLabs endpoint-definition edit that was outside the approved scope?

The spec explicitly says, "Do not edit the endpoint literals (547; openai 265)." The log nevertheless records adding `page_size` and `page_token` to the ElevenLabs `ListVoices` endpoint in schematic-definitions. These parameters let biscuit-speaks replace a hand-written voices request with generated typed sends while preserving its existing pagination. This changes what the generated request type can express.

This meets test 2 (departure from an explicit scope restriction) and test 1 (generated public request methods).

- Accept this narrow exception: removes the consumer's transport bypass and represents the parameters it already sends. It makes an exception to the endpoint freeze.
- Revert and defer the endpoint edit: preserves the freeze, but needs another safe migration for the existing voices listing; retaining unmanaged HTTP access conflicts with this feature's transport goal.

Recommendation: accept only these two existing query parameters as a scope exception. If accepted, retain the typed migration and record the departure in the implementation log; do not authorize unrelated endpoint changes. Until decided, the migrated voices listing depends on an unapproved definition change.

### Item 6: Retain shared-file compatibility checks alongside the new runtime ABI marker?

The spec's dependency section says this feature "defines and persists the marker and replaces that comparison." The implementer retained the existing byte comparisons of generated `shared.rs` and `ws_shared.rs` beside the new ABI check in schematic-gen. The stated reason is that retained generated modules still call helpers in those files: matching the runtime's ABI number does not prove those helpers stayed compatible.

This meets test 2 (departure from the stated replacement instruction). It also affects the workflow later generation changes use to decide whether a partial regeneration is safe.

- Keep both checks: protects compatibility of retained generated modules as well as the hand-written runtime. Harmless shared-file edits may still require full generation.
- Use only the runtime marker: follows the planned simplification, but needs evidence that every relevant shared-helper change is represented by that marker or another compatibility mechanism.

Recommendation: keep both checks while generated modules depend on those helpers. If accepted, the repair records the two distinct compatibility questions and updates later planning assumptions. Until decided, partial generation can either remain unnecessarily restrictive or lose a protection its retained modules still need.

### Item 7: Change the required builder-test evidence, or repair the two proposed testing gaps?

The spec requires each builder setting to be "observed by a local mock" and says a test failing on one OS is investigated, "not skipped." The log reports two changed premises: no portable loopback fixture was found that observes the connect timeout taking effect, and four TLS builder tests abort intermittently on native Windows with `0xc0000409`. Investigation reportedly reproduced the abort with plain reqwest; the tests are now ignored there and run on macOS, Linux, and WSL2. This is a request to change required evidence, not a claim that only a human can run tests.

This meets test 2 (required validation is omitted) and test 4 (the missing cases concern network timeout and TLS behavior). Cross-OS receipts themselves are not a readiness gate in this review.

- Accept both as bounded gaps: avoids a dependency investigation delaying the pipeline, but leaves the connect-timeout effect unobserved and native-Windows TLS behavior unproved under the failing conditions.
- Accept only the connect-timeout gap and require repair of Windows TLS coverage: recognizes the loopback limitation without normalizing a permanent platform skip. Repair may require a better fixture or dependency diagnosis.
- Keep both original evidence requirements: strongest assurance, but the next repair must find a portable effect test and resolve or safely replace the aborting TLS fixture.

Recommendation: accept the narrowly documented connect-timeout effect gap; do not accept permanent Windows TLS exclusions. Require a repair that restores meaningful coverage, with the existing OS-skill investigation as its starting point. If accepted, the following review treats restored TLS coverage as a repair obligation and records the limited connect-timeout assumption explicitly. Until decided, no reviewer may turn a proposed gap into an accepted one.

## Implementation decisions (informational)

The following choices do not need the review cycle to stop:

- Supplied defaults adopted: same-origin links under `deny`; ten redirect hops; POST-to-GET rules and replayable 307/308 bodies; evidence from the refused hop without a whole-chain record; five named origin policies; no general middleware; the runtime package location/version; deletion of the unused request enum; typed raw sends and their completion states; non-exhaustive response-error kinds; `NoDeclaredError`; both metadata-preserving and value-only extraction; declared-error status range `400..=599`; one header redaction set; no additional value-only helper; HTTP/1.1 cancellation with HTTP/2 explicitly untested; and API-level secret/request-ID fields pending later builder work.
- Author decisions already settled: typed sends, shared proxy/TLS ownership, tighten-only origin rules including `deny`, anonymous sends when credentials are absent, client-global success-JSON hooks, decoding declared errors on arrival, removal of the content-type guard, shared deadline arithmetic, and stream idle bounds. None is reopened here.
- Credential precedence and explicit anonymous mode follow the spec. The spec explicitly requires rejecting injected query/cookie API-key collisions; although the log also calls collision refusal a departure from earlier behavior, this is an authorized change, not a new question. Environment proxies preserve earlier behavior.
- Internal or readily revised choices: module boundaries, paused-clock timing, saturating extreme deadline budgets, fragment and Content-Location redaction, no default user agent, diagnostic fixture placement, trybuild rather than doctests, ABI parsing with syn to avoid a generator transport dependency, generated prelude exports, percent encoding that preserves ordinary existing query bytes, and boxing public send futures to avoid the recorded consumer stack overflow.
- Required migration consequences and fixes recorded by the implementer: removed unreachable anonymous retries; preserved a separate workflow-runs retry; fixed GitLab double encoding; interpreted rate-limit headers; distinguished malformed and interrupted replies; preserved device wake behavior through fallible constructors; ended voices pagination when no continuation token exists; and removed model-citizen's unused generated-client dependency. These are not new author questions.
- Required or safe choices do not become questions merely because the log flags them: redacted raw headers use the required single metadata record; dropping secret query values on credential-free hops implements the stated stripping rule; built-in `key` redaction also redacts Eversolo remote-control key names; and no redirect-chain field is added under the supplied default.

## Additional public-contract choices

Beyond the seven questions above, the log records public metadata layout (`ResponseMeta` contains `RequestMeta`, whose `page` slot is reserved for pagination), public redacting capture constructors, boxed request metadata in transport errors, `EndpointSpec::METHOD`, opaque credentials, replayable request parts, redirect-refusal reasons, redacted hook metadata, unit `HookError`, `RawJsonError`, and exhaustive definition-side `CrossOrigin`. Public metadata layout and hook signatures also meet the cross-crate or later-spec test; the seven questions retain the widest representation, extension, policy, scope, and validation consequences. These additional contracts are named here as required by the seven-item limit. They must be considered when recording the interface rulings, without asking the author to reaffirm raw-header redaction or other supplied defaults. The decoder question includes the hook delivery mechanism and error-body contract.

## Deferred work and limits of the record

The log reports no required evidence that only a human with a real account, device, or credential can obtain. Real-provider contact was outside this implementation's authorization. The Windows abort is a technical investigation, not human-only verification.

Explicit later-spec work remains deferred: pagination must supply the page-number input and session-close grace; later codecs add SSE, NDJSON, and GraphQL; later definition builders add per-endpoint policy and per-parameter secret markers. Their changes must update the frozen surface test. Unsanitized URL queries inside `Link` response headers were flagged for pagination; no additional security guarantee is accepted here, and the normal review must compare that behavior with this spec's metadata promise before any handoff.

Runtime-only compile-fail tests reportedly run through the local recipe but not CI; the generated-client misuse tests use the existing fresh-output CI path. The proposed gap concerning connect timeout and native-Windows TLS is item 7. Final dependent Windows/WSL2 runs were left to CI after the boxing change. None of those recorded limitations has been silently accepted as a readiness finding or waiver.

The implementer left obsolete, uncompiled generator modules on disk after treating deletion as outside authorization. This is reversible maintenance, so it does not justify stopping for a human. The log also notes a remaining biscuit-speaks cache-path test gap, no dependent binary-size observation, and pre-existing defects or documentation drift involving sniff feature gating and listing stubs, OpenAI environment fallback, WebSocket environment mappings, unused `UpdateStrategy`, and generated examples. These remain information for the later implementation review or separately scoped maintenance; this decision-only review assigns them no findings.

## Human rulings

### Item 1

Ruling: **Rejected as built; replaced.** Neither the generated marker plus `GitHubClient` alias nor the spec's forwarding wrapper is accepted.

- **One user-facing name per API.** Each API gets a generated struct named after the API (`GitHub`, `OpenAI`, …), and that is the only name users write. An internal tag type (`GitHubApi`) keeps the compile-time guarantee that a client sends only its own API's requests; users never write it.
- **Client setup.** `GitHub::new()` returns a usable client with no `?`. Client-wide settings (`base_url`, `api_key`, `timeout`, `proxy`, …) are chained setters implemented once in the shared runtime, not forwarded per API.
- **Validation.** A bad setting is reported on the first call. An optional `.build()?` validates immediately and returns the same `GitHub` type, not a separate checked type. Nothing silently falls back to a default.
- **Credentials.** `.api_key()` places the secret wherever that API's definition says it goes; `.basic_auth(user, password)` exists where the API takes those; `.credentials(Credential::…)` remains the general form. With no key set, the API's environment variables are used, and with none present the request is sent anonymously.
- **Endpoint calls.** `gh.list_releases(owner, repo).per_page(10).await?`: required values are arguments, optional values are chained setters, per-call overrides (such as `.anonymous()`) chain the same way, the basic path needs one `?`, and the full reply is available through `.full()`. The same optional `.build()?` validates a call before it is sent.
- **Scheduling.** The work splits in two:
  - **This feature's repair** delivers the client shape: the one-name struct, chained client settings, the optional `.build()?`, and `.api_key()`.
  - **`2026-10-12-schematic-endpoint-methods`** delivers endpoint methods and call types as the primary call surface. It is scheduled after this feature and before `2026-10-13-schematic-pagination`, whose generic `pager(request)` entry-point default is superseded so that pagination builds on the endpoint call types.

Example:

```rust
let gh = GitHub::new()
    .base_url("https://github.example.com/api/v3")
    .api_key(key);

let releases = gh.list_releases("yankeeinlondon", "rusty-biscuit")
    .per_page(10)
    .await?;
```

### Item 2

Ruling: **Accepted as built**, primarily to keep scope contained. The type-level decoder (`Decode::decode(body)` with no codec instance), the owned `LiveBody`, the separate `ErrorBody` trait, and the provided `decode_hooked` method gated by `SupportsHooks` are the interfaces later codec work builds on.

- **Runtime vs. definition representation.** The definition-side `ApiResponse` enum (`Json(Schema)`, `Text`, `Binary`, `Empty`) remains the value-level description; each variant maps to a runtime decoder type so that a call's result keeps its concrete type (`Vec<Release>`, not a generic decoded value).
- **JsonL is high priority.** Newline-delimited JSON is already planned as the NDJSON stream shape in `2026-10-16-schematic-response-model-graphql` (R6); that work should not slip. `FormField` replies are low priority and unscheduled.
- **Hook obligation for the next hook-capable codec.** `decode_hooked` defaults to ignoring hooks, so a codec that implements `SupportsHooks` but does not override it would compile while silently never running registered hooks. Any codec that implements `SupportsHooks` (expected first: NDJSON) must ship a test proving registered hooks run on its output.
- **Schema checking.** Today, structural checking (types, required fields, enum values) happens inside the decoder through serde. JSON Schema value constraints (`minLength`, `pattern`, `maximum`, …) are not supported and are discarded by the OpenAPI importer. That gap is split out to `2026-10-20-type-constraints`: constraints matter primarily for what is sent, and it does not change the decoder contract.

### Item 3

Ruling: **Accepted as built.** A `SharedTransport` owns every connection-level setting: `proxy`, TLS (`add_root_certificate`, `danger_accept_invalid_certificates`), and `connect_timeout`. A client attached to a shared transport that also sets any of them is refused with `TransportOptionConflict`, even when the value matches. Under the Item 1 ruling the refusal surfaces at `.build()?` or on the first call. Clients that need different connection settings use separate transports.

- Matching-value acceptance (option B) is not adopted. Relaxing the refusal later would be non-breaking; tightening it later would not.
- The repair documents the ownership split (connection-level settings on the transport; `base_url`, `timeout`, credentials, and default headers on the client) and the separate-transport guidance as settled behavior in `schematic/docs/`.

### Item 4

Ruling: **Accepted as built.** `Client::check_link::<R>(&Url)` replaces the spec's type-free `check_link(&Url)`, and an endpoint path that is an absolute `http(s)://` URL is sent to that URL after the same origin check.

- The endpoint type lets a refusal report the real endpoint and method. It is also required for correctness once later definition work adds per-endpoint origin policies, at which point whether a link may be followed depends on the endpoint.
- Under the Item 1 ruling, the pager built on endpoint methods is the primary caller; ordinary callers are not expected to call `check_link` directly.
- The pagination handoff (`2026-10-13-schematic-pagination`) uses its request type when checking links and follows absolute links through the checked absolute-URL request path.

### Item 5

Ruling: **Accepted as a narrow scope exception.** The optional `page_size` (integer) and `page_token` (string) query parameters on the ElevenLabs `ListVoices` endpoint are allowed, so biscuit-speaks lists voices through generated typed sends instead of a hand-built request outside the runtime. The wire traffic is unchanged.

- The exception covers exactly these two parameters on this one endpoint. The endpoint-literal freeze otherwise stands, and this ruling authorizes no other endpoint change.
- The repair records the departure in the implementation log.
- `2026-10-14-schematic-definition-api` takes its definition baseline after this change, so the two parameters are part of that baseline. A later pagination strategy may adopt `page_token` as a pager-managed parameter.

### Item 6

Ruling: **Accepted as built, with a stated direction.** Partial generation keeps both checks, because they answer two distinct compatibility questions:

1. **Do retained modules match the hand-written runtime?** The `schematic-runtime` ABI marker answers this.
2. **Do retained modules match the generated shared helpers?** The byte comparison of `shared.rs` and `ws_shared.rs` answers this. The ABI marker does not cover those helpers.

Direction: shared helpers move into `schematic-runtime` over time, so the ABI marker covers them. The byte comparison is deleted once no generated shared helper file remains, which reaches the spec's intended single mechanism without a gap in protection.

- Moves ride along with features already touching that code. First candidates are the endpoint-method feature from Item 1, and `2026-10-17-schematic-websocket`, where moving `ws_shared` into the runtime is to be considered.
- The repair records the two compatibility questions in `schematic/docs/` and updates later planning that assumed the byte comparison had been replaced.

### Item 7

Ruling: **Connect-timeout gap accepted; Windows TLS gets a bounded repair, not a permanent exclusion.**

- **Connect timeout.** The effect is not observed by a mock, because a hanging connect cannot be produced portably or deterministically on loopback. Accepted, provided the ownership and delivery-to-the-HTTP-client tests remain and the reason is recorded beside them. The effect itself is the HTTP library's responsibility.
- **Windows TLS.** The `0xc0000409` abort reproduces with plain reqwest. Production clients share that reqwest configuration, so the skipped tests are the only place a possible native-Windows production exposure is visible. The next repair must:
  1. try having the runtime build its own rustls `ClientConfig` and hand it to reqwest (the preconfigured path: 0 aborts in 160 runs), and re-enable the four tests on native Windows if that path holds over a substantially larger parallel run (on the order of 1,000 runs with no abort); and
  2. if step 1 does not resolve it, capture the faulting stack with `cdbX64.exe` as described in the `os` skill.
- **If unresolved after both steps,** open a fix spec under `schematic/fixes/` recording the evidence, and change the `ignore` reason to name that fix. The skip is then tracked rather than permanent, and this feature does not wait on an upstream defect.
- The following review treats steps 1–2 (and, if needed, the fix spec) as a repair obligation.


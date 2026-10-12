---
total_phases: 8
created: 2026-10-11
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
---

# Schematic Runtime Envelope Implementation Plan

## Phase 1: Establish Scope and Contracts

Replace generated REST execution code with a hand-written `schematic-runtime`, then generate thin clients whose concrete requests determine their success type, failure type, codec, and API identity. The runtime owns response evidence, credential resolution, managed connections, origin enforcement, redirect following, deadlines, raw sends, and the two existing JSON hooks. Regeneration and migration of all eight dependent crates form one coordinated phase so the generated API never leaves consumers broken at a phase boundary.

Successful completion means every active requirement R1–R37 has observable passing evidence; R36 is recorded as withdrawn. Fresh generated REST and WebSocket modules compile, all affected packages and consumers pass their scoped gates, repeated generation produces no changes, and macOS, Linux, native Windows, and WSL2 have separately identified evidence. Documentation describes the shipped behavior, and `implementation-log.md` records adopted defaults, decisions, measurements, deviations, and review items. A proposed gap remains unmet until the author accepts it; missing evidence must never be called a pass. The implementation ends **implementation complete, ready for review**, without committing, pushing, or moving the feature directory.

Prerequisite refresh on 2026-10-12, from generation-integrity review 3: the prerequisite is implemented and ready. RequestParts is a five-element tuple with ordered, unencoded query pairs last; compiled fixture clients exercise request-side features. The existing four fresh-output checks run through gen/tests/common/fresh_output.rs and just test-fresh-output. Partial compatibility reads generated Rust inventory and compares shared.rs/ws_shared.rs bytes in output/inventory.rs::shared_runtime_compatibility; no serialized inventory or persisted ABI marker exists. The runtime workspace member still does not exist. Preserve sniff's migrated recursive=true GitHub requests and binary GetSource decoding for Bitbucket as the consumer baseline. The author withdrew REST selectors; later response-model work uses one endpoint with multiple wire-distinguishable responses.

### Necessary Rules

- **Authority and execution scope.** D56/D57 are settled author rulings. Adopt the specification's remaining Proposed items and defaults provisionally and log them, rather than reopening settled choices. This planning request authorizes creating this document only. At implementation entry, verify that the specification's pre-authorizations have been granted by the implementation instruction or existing session context. In a non-interactive run, missing required authorization is recorded as a blocker; do not wait for a prompt or treat `yolo: true` as authorization.
- **Dependency refresh.** Read the prerequisite's implementation/review outcomes before runtime work. Verify ordered query separation, request-side text/binary/header/cookie/API-key support, fresh-output compile checks, partial-generation compatibility interface, and stale-tree cleanup. Record differences and refresh this plan against actual interfaces. If the prerequisite is still unavailable, stop before implementation changes. A small blocking omission after it lands may be repaired here only under the spec's minimum-repair allowance, with the requirement, scope, and reason logged; do not absorb the entire prerequisite.
- **Numbering and phase boundaries.** Start at Phase 1; references to “phase 0” in the spec mean this phase's inventory. Waves below have globally unique numbers and run in order. Tasks within a wave may run concurrently only when their write sets are disjoint; a singleton wave is sequential. A wave is a scheduling step, not permission to bypass prerequisites. Earlier phases preserve checked-in schema; Phase 7 completes regeneration and every migration before its checkpoint.
- **Ownership and exclusions.** Runtime is a workspace member at `schematic/runtime/`, initially version `0.1.0`; schema remains workspace-excluded and generator-owned. Runtime does not depend on `schematic-define`. Edit API-level `RestApi` construction sites, never the 547 endpoint literals. Do not add endpoint wrappers, general middleware, pagination, SSE/NDJSON/GraphQL codecs, new serde derives on definition models, or endpoint-level origin overrides. Do not delete the stale generated tree owned by the prerequisite or retire sniff's `FocusedProviderClient`.
- **Credential contract.** Preserve `effective_auth_policy()`, accepted methods, legacy fallback, environment mapping, custom header names/prefixes, and Basic username/password behavior. Per-call → client → first non-empty configured environment value → anonymous; explicit anonymous mode bypasses and removes every credential source. No missing-credential error and no runtime authentication retry. Reject incompatible credential methods and auth-bearing default headers; reject injected query/cookie-key collisions. `AuthMethod` reports source only and is distinct from the definition's method-selection enum.
- **Origin contract.** Adopt PD1–PD6, PD29, and PD30. Base origin means parsed scheme, host, and effective port. Under `deny`, same-origin links pass, but redirects and cross-origin links fail. Overrides are only deny, same-origin, or subsets of a definition allowlist; a deny definition permits only deny. `allow_any` is definition-authored through the dangerous opt-in, never a permitted caller override. Use 10 followed hops, resolving each Location against its responding URL. Each target is compared to the configured base origin for credential forwarding, including A → B → B and B → A.
- **Head and evidence contract.** Any received head in a logical send requires an envelope. Failed later hops retain the last redirect's head as `RedirectFailed`; policy refusals use `RedirectRefused`. Metadata describes the last received hop, not an unreachable target. Raw body bytes and decoded provider values remain unchanged. Redact stored headers, URL/query data, Location URLs, secret-marked request IDs, redirect targets, and diagnostic context. Safe displays omit provider bodies; provider-body and raw-body Debug can expose provider echoes and must be documented. Never retain unchecked library source chains or arbitrary hook messages.
- **Decoder and timeout contract.** Adopt PD7, PD16–PD24, and PD27. Use one monotonic expiration for a buffered logical send, bounded by client timeout and shared deadline; redirects never reset it. A JSON parse determines JSON validity, even with a misleading Content-Type. Text/binary accept any content type; Empty drains and ignores bytes. Stream extension tests use an owned live body, an idle timeout between items, and an optional overall shared deadline. No production stream codec ships here. Deadline clones share an atomic completed-logical-send count, including classified HTTP failures, excluding redirect hops, pre-head failures, and dropped sends.
- **Shared connection settings.** D57 fixes proxy/TLS ownership on `SharedTransport` and `.transport(&t)`. Proposed implementation choice: place connect timeout on the shared transport too because it affects connection establishment; retain `.connect_timeout()` on standalone client builders, and reject conflicting settings when attached. Per-request timeout, credentials, hooks, metadata defaults, and policy remain client-owned. Confirm feasibility in Wave 2 and log this choice for author review; do not silently ignore client settings. Reject proxy/TLS conflicts per R18, including the required attached-client proxy case.
- **Remaining defaults.** Record a disposition for every PD1–PD35. PD1/15/23/26 are ruled; PD32/35 are withdrawn. Adopt PD8–PD12 and PD14/17/18/22/28 builder/type/raw/ABI defaults; PD13 removes the erasing enum unless private request construction still needs it, never admitting it to send. Adopt PD25's API-level secret list and offline-evidenced request IDs/origin exceptions; otherwise use None/same-origin. Adopt PD31's consumer behavior preservation, PD33's local test dependencies, and PD34's no auth label. PD19 measures stream elapsed to the head; PD20 adds no extra value-only shortcut; PD24 proves cancellation on HTTP/1.1 and records HTTP/2 as untested.
- **Tests and evidence.** Use Test Toolkit fixtures and the `rust-testing` skill, nextest-backed `just test`, and the existing fresh-output harness. Loopback HTTP/TLS/proxy/raw-socket tests are L1. Use fixture-owned ports, readiness signals, guaranteed shutdown, bounded waits, and `EnvGuard` with serial environment fixtures; no provider calls or fixed readiness sleeps. Compile-fail assertions must fail for the intended diagnostic, including a real generated request from another API. Existing L2 gates use the no-focus harness; this feature adds no L2 tests. Do not run `cargo fmt`.
- **CI discovery.** R37 requires planner inspection, not a workflow change. Run `just ci-local --plan` after the runtime becomes a member and record package/environment/gate discovery. Load `rust-devops` if the planner disagrees before considering any scope change. No new speculative CI cells or full-workspace test runs.

### Spike Decisions

The parent already establishes generic typed-send feasibility, synthetic trait compile cost, HTTP/1.1 cancellation, and origin checks. Do not repeat those spikes, add endpoint wrappers, or expand into a benchmark campaign. The real-schema compile samples below are the spec's required measurements, not new performance spikes. Any incremental-build, binary-size, HTTP/2, or broader platform-performance campaign is an author-review item, not scheduled work.

One bounded functional spike is justified: the production HTTP-library integration must expose live ownership and partial bytes while connection settings remain managed. Wave 2 runs it once before its dependent implementation. Turn useful cases into regression fixtures; do not schedule another spike in later phases.

### Wave 1 — Entry Inventory (parallel)

- [x] **Verify Prerequisite**
  - Read prerequisite implementation/review artifacts and inspect actual request parts, partial-generation inventory format, and fresh-output compile harness. Record readiness and exact integration points in `implementation-log.md`.
  - Confirm that retained inventory still reads generated Rust source. Record its module/export/ownership checks and the shared-file comparison; the runtime feature introduces the persisted ABI marker, not an inherited configuration field.
  - Record implementation authorization status and any blockers. This task is required before Wave 2.
- [x] **Inventory Consumers**
  - Inventory generated-client `request`, non-JSON convenience sends, constructors, variants/Headers authentication, hooks, error matches, logging, and `EndpointSpec` uses across all eight crates. Include indirect users and shared WebSocket types, filtering out ordinary reqwest calls.
  - Record file/symbol, API, response kind, old behavior, proposed replacement, and test owner for each call site. Confirm whether model-citizen is manifest-only; record exact package names from manifests rather than assuming directory names equal package names.
  - Ownership: write inventory notes in a separate temporary artifact; the phase coordinator merges them into the implementation log so agents do not concurrently edit it.

### Wave 2 — Contract Preparation (parallel after Wave 1)

- [x] **Capture Baseline**
  - Run affected Schematic and eight consumer L1 baselines through their canonical recipes; distinguish pre-existing failures. Record lint baseline where useful and build coverage for dependents.
  - Capture one `schematic-schema` compile-time sample on this host using its excluded-crate manifest. Record exact command, profile, feature set, toolchain, and cache state so Phase 7 repeats the same conditions without deleting shared caches.
  - Load `sniff` for package/host discovery and `os` for available build-host evidence routes. Record available environments and required OS-sensitive cases without launching a cross-platform performance study.
- [x] **Prove Decoder Boundary**
  - In an isolated throwaway fixture using the actual dependency versions, receive headers, incrementally retain body bytes, observe a truncated body, and return an owned test stream whose first item precedes body completion. Demonstrate drop cancellation without a detached worker.
  - Verify managed `SharedTransport` feasibility and connect-timeout ownership; record whether the proposed conflict rule can be enforced without raw-client injection or multiplying unrelated shared settings.
  - Produce a compact contract sketch for `EndpointSpec` API marker, live decoder input/error evidence, hook capability marker, expiration state, replayable request parts, and raw completion disposition. Delete throwaway artifacts or retain only intentional test fixtures.
  - If the live-decoder contract is infeasible, record the concrete blocker and alternatives before proceeding; do not replace it with unconditional prebuffering.

**Validation checkpoint:** prerequisite outcomes and authorizations are established; baseline and exact migration inventory exist; all PD defaults have a recorded planned disposition; decoder evidence and stable contracts unblock Phase 2. No production or generated API has changed.

### Compatibility input verification

The prerequisite reads owned generated Rust module files and parses lib.rs and prelude.rs with syn; it has no JSON/YAML/TOML inventory fields and no persisted ABI marker. The earlier plan's serialized-inventory matrix assumed a format that was not built. The configuration robustness matrix does not apply to this Rust grammar. Generated Cargo manifests are writers, and the lockfile helper delegates parsing to Cargo.

Implement the spec's proposed ABI assertion in generated lib.rs and read the persisted marker through the existing Rust inventory path. Starting from actual generated output, test a supported marker, a differing marker, a missing legacy marker, and malformed or conflicting marker declarations through public partial generation. Refusals leave output unchanged and identify the need for full generation. Preserve the existing module membership, export, ownership, and shared WebSocket checks; a matching ABI must not bypass them.

If implementation introduces a serialized configuration reader instead, record why and apply the full per-format, per-field configuration matrix before modifying that reader. This plan does not require introducing a separate inventory format.

## Phase 2: Build Runtime Foundations

Entry: Phase 1 contracts are fixed; checked-in schema and consumers stay on the old surface.

### Wave 3 — Package Setup (sequential)

- [ ] **Create Runtime Package**
  - Create the workspace member, manifest, crate root, test registration, and module boundaries for metadata/errors, redaction, transport, credentials, origin, deadline, codecs, and hooks. Introduce only dependencies required by these contracts and document them as added.
  - Define `EndpointSpec`/API marker, `Decode` boundary, request parts, call options, and ABI interfaces before delegating module implementation. Keep the runtime independent of `schematic-define`.
  - Add runtime to package-enumerating area recipes, including build, sanity, test, lint, doctest, coverage, and applicable tier/real recipes. Preserve existing L2 generator coverage; do not fabricate runtime L2 tests. Confirm discovery with `sniff repo packages` and `just ci-local --plan` (R1, R37).

### Wave 4 — Foundation Modules (parallel after Wave 3)

- [ ] **Define Response Types**
  - Implement metadata, `Response`/`Outcome`, `ApiError`, uninhabited `NoDeclaredError`, safe transport/build errors, and non-exhaustive failure kinds, including distinct redirect refusal/following failures.
  - Implement complementary data/error access, metadata-preserving `into_parts`, metadata-discarding `into_result`, and precise decode/interruption helpers. Add error-type rustdoc explaining the body Debug limitation now (R3, R4, R6, R8, R10–R13 groundwork).
  - Own response/error modules and their value/compile-fail tests; network classification checks follow in Phase 3.
- [ ] **Implement Redaction**
  - Implement the union of every D48 built-in name, effective-auth names, and API secret names. Headers match case-insensitively; query names exactly; ordered ordinary duplicate query values remain intact.
  - Produce safe stored URLs, query values, repeated header maps, URL-valued Location headers, request IDs, and diagnostic targets. Omit bodies/arbitrary error strings from Display; keep evidence bytes untouched.
  - Own redaction module and table tests; use all built-in names and distinguish non-secret values. Integrate sentinel checks across outcomes in Phase 3 (R5–R7).

**Validation checkpoint:** runtime compiles and its foundation L1/compile-fail tests pass; Schematic lint and touched-package gates pass, and dependents still build against unchanged schema. Planner discovers runtime in area `schematic` without CI edits. Crate docs and dependency records match this phase.

## Phase 3: Implement Managed Sends

Entry: metadata, redaction, live-decoder contracts, and request assembly interface are stable.

### Wave 5 — Request Infrastructure (parallel)

- [ ] **Build Managed Transport**
  - Implement standalone managed builder and `SharedTransport`, always with HTTP-library redirects disabled. Cover base URL, proxy, roots, invalid-certificate opt-in, connect/per-request timeouts, user agent, default headers, and transport attachment.
  - Validate HTTP(S) base URLs, reject user information and auth-bearing defaults, and enforce shared-settings conflict rules with typed safe `BuildError`.
  - Add local proxy/TLS fixtures, required-root/default-rejection tests, and sequential drained HTTP/1.1 connection-reuse evidence from two clients sharing one transport (R17–R19 groundwork, R18).
  - Own builder/transport modules and fixtures; raw client injection is never exposed.
- [ ] **Resolve Credentials**
  - Implement effective-policy validation and one-time source resolution, caller environment-order replacement, independent Basic fields, environment mappings, custom bearer headers/API-key prefixes, query/cookie keys, and explicit anonymous mode.
  - Separate resolved secrets from redacted metadata, reject duplicate key injection, and ensure default/header/variant migration cannot add a second credential source.
  - Own credential module and isolated environment tests. Tests observe selected credentials at the server and source evidence, including policy differing from legacy strategy (R14–R16).
- [ ] **Implement Deadlines**
  - Implement shared `Instant` expiration/counter, limiting-timeout cause, whole-send budget, and decoder-owned stream idle/deadline state. Count classified logical responses once; expose invalid-argument construction and page slot for later pagination.
  - Own deadline module and deterministic state tests; integrate short-delay, no-send-after-expiry, and cancellation server evidence in Wave 6 (R9, R26, R30).

### Wave 6 — Execution Integration (sequential)

- [ ] **Implement Typed Send**
  - Assemble replayable request bytes and ordered query/header/cookie inputs; resolve credentials once; capture redacted application request metadata; receive and classify response heads before handing live bodies to codecs.
  - Ship JSON, text, binary, and draining Empty decoders. Preserve original/partial bytes on read/decode failures; decode declared errors on arrival, distinguish undeclared statuses, and preserve malformed 2xx status. JSON parses valid JSON despite misleading Content-Type.
  - Cover no-head Connect/TLS/Timeout/DeadlineExceeded versus post-head BodyInterrupted, safe network diagnostics, pre-send InvalidArgument, and API-specific request bounds. Outer errors always retain request metadata.
  - Add mock success/failure envelope tests, request-ID/page defaults, declared/malformed/undeclared bodies, uninhabited-error compile failure, metadata/Display sentinel checks, and raw-socket truncation evidence (R2–R16, R26, R30).
  - Prove timeout leaves shared budget for another request, exhausted budget sends nothing, counted HTTP failures are completed sends, dropped futures cancel promptly, and test-only streams own their body and deliver before EOF. Test idle and overall stream expiry without shipping a protocol codec.

**Validation checkpoint:** runtime transport/credential/decoder/deadline L1 tests and compile-fail cases pass, including safe Display/Debug for outer errors and partial-byte retention. All changed packages lint; dependents still build against unchanged schema. New TLS/proxy/compile-fail dependencies are documented. No authentication retries exist.

## Phase 4: Enforce Origins and Redirects

### Wave 7 — Policy Ownership (parallel)

- [ ] **Define API Policy**
  - Add `CrossOrigin` with safe default and explicit dangerous opt-in, plus `RestApi::cross_origin`, `request_id_header`, and `secret_names`; update every API-level construction site, test fixture, importer-generated RestApi construction, and doctest required to compile.
  - Populate all 18 shipped definitions. Use only existing/offline provider evidence for request IDs and extra origins; log choices. Do not edit endpoint literals or add serde derives.
  - Own define/definitions and affected construction fixtures. Add the positive current-surface test for no endpoint override/no GraphQL primitive and document its later replacement (R20).
- [ ] **Implement Origin Rules**
  - Own runtime origin-policy module. Validate exact-origin allowlists and build the full tighten-only table, including deny → same-origin rejection naming both policies.
  - Implement pre-request origin enforcement and `check_link`, with no network work on refusal. Same-origin links pass under deny; disallowed destinations return CrossOriginRejected with safe request context.
  - Test effective-port normalization, scheme/host/port distinctions, sibling-host rejection, invalid URL/user-info schemes, allowlist paths/query/fragments, and clearly named dangerous opt-in (R9, R21, R23, R25).

### Wave 8 — Redirect Integration (sequential)

- [ ] **Follow Redirect Hops**
  - Integrate the follower into the single managed execution path. Resolve relative Location, remove fragments, enforce 10 followed hops, and reconstruct each hop from unauthenticated request parts before conditional credential attachment.
  - Cover 301/302 POST conversion; 303 conversion except HEAD; 307/308 replay; removal of body-specific headers; missing/invalid Location; explicit Host removal; no cookie jar or forwarded proxy authorization.
  - Exercise all five policies and defaults with two mock origins. Prove refused origins receive zero requests and foreign hops strip authorization, custom auth/secret headers, Cookie, and secret query keys. A → B → B stays anonymous; B → A may restore the resolved credential.
  - Preserve final/last-hop metadata and whole-send elapsed. Distinguish RedirectRefused from later-hop Connect/TLS/timeout/deadline RedirectFailed retaining the 3xx and sanitized Location/target. Redirect hops consume the original expiration, never restart it (R19, R22–R26).
  - Exercise every public constructor/attachment path against redirect policy so none enables library auto-follow.

**Validation checkpoint:** R19–R26 policy, link, redirect, failed-hop, and deadline tests pass with zero-request assertions and safe evidence. Definition changes compile across all construction sites; old schema and consumers still build. Generator translation is validated when emission changes in Phase 6.

## Phase 5: Complete Raw Sends and Hooks

### Wave 9 — Extension Surfaces (parallel)

- [ ] **Implement Raw Responses**
  - Add API-scoped typed-request `send_raw(request, &CallOptions)` through the common executor, returning metadata, bytes, and complete/interrupted/refused/failed completion disposition after any head.
  - Implement status/header convenience accessors from the single metadata record; complete HTTP errors remain complete raw responses. `json::<T>()` rejects incomplete disposition before parsing even when partial bytes are valid JSON.
  - Own raw module/tests; prove shared credentials, origin policy, deadline, redaction, and all incomplete evidence cases, with hooks bypassed (R27, PD14).
- [ ] **Preserve JSON Hooks**
  - Own hook module/tests. Implement JSON-codec capability marker and API-plus-endpoint keyed mutators; preserve successful JSON parse → global hook → success decode → typed mutator ordering.
  - Global hooks skip non-2xx and non-JSON endpoints; declared errors/raw sends bypass both hooks. Hook failures preserve original bytes and use safe Hook diagnostics, never arbitrary callback error text.
  - Compile-fail typed mutators on text/binary/empty endpoints; test two APIs sharing endpoint names cannot cross-dispatch hooks (R29, R30 extension contract).

### Wave 10 — Runtime Audit (sequential)

- [ ] **Audit Shared Execution**
  - Integrate both surfaces and rerun their boundary cases through the same execution core, including typed/raw API scoping, credentials, redirects, deadline/cancellation, and safe evidence.
  - Confirm runtime contains no ResponseKind guard or UnexpectedContentType kind. Record replacement mismatch tests and distinguish invalid JSON from a merely misleading Content-Type (R28).
  - Freeze runtime ABI/public contracts for generator integration; record PD14/hook/trait/enum choices for review and update runtime docs as behavior lands.

**Validation checkpoint:** raw and hook L1/compile-fail suites pass; all runtime public send paths enforce managed policy; no production stream/GraphQL codec or generic middleware has been added. Touched Schematic gates pass; checked-in consumers remain green.

## Phase 6: Generate Thin Typed Clients

Entry: Phase 5 runtime ABI is stable; generator changes target fixtures and isolated fresh output only.

### Wave 11 — Generator Changes (parallel with disjoint ownership)

- [ ] **Emit Typed Clients**
  - Own REST codegen and its golden tests: per-request runtime `EndpointSpec` impl with API marker, success/failure/codec/endpoint constants; translate effective auth policy, CrossOrigin, secret names, and request-ID configuration.
  - Emit thin per-API client wrappers with builder/new, typed send/raw, and scoped hook access. Remove generic request, injected-client constructors, shared REST-runtime definitions, generator-local environment scans, temporary API-key injection, non-JSON convenience send paths, and ResponseKind guard.
  - Keep erasing request enum only if private construction requires it. Emit no per-endpoint forwarding wrappers; retain request-side feature fidelity (R1, R2, R17, R20, R28, R31).
  - Handle generator fixture churn within this task; do not concurrently edit the checked-in schema.
- [ ] **Integrate Runtime Compatibility**
  - Own generated manifest/lib ABI emission, prerequisite inventory compatibility reader, and fresh-output harness; coordinate file boundaries with codegen task before edits.
  - Generate the path runtime dependency so both actual excluded schema and isolated output resolve correctly. Embed the ABI expected by this generator build; mismatched runtime fails compilation with readable assertion.
  - Make ABI mismatch refuse partial generation and require full generation before writing outputs. Exercise the generated Rust marker cases from Phase 1 through public generator-result tests, preserving inventory and no-mutation checks (R32).
  - Extend the prerequisite isolated compile harness instead of creating another: include runtime, shipped response kinds, request features, and generated WebSocket modules. Use its cache/offline resolution strategy (R31, R35).

### Wave 12 — Fresh Output Proof (sequential)

- [ ] **Validate Generated Surface**
  - Compile fresh complete output including WebSockets. Assert generated shared modules contain runtime references, not REST-runtime definitions; removed constructors/guard/free request and endpoint wrappers are absent.
  - Compile-fail wrong success type, missing associated type, arbitrary foreign type, actual request from API A sent through API B for typed/raw sends, forbidden hook registration, removed request API, and removed injected-client construction.
  - Test doctored ABI failure and incompatible-marker partial-generation refusal. Run generated-Rust marker refusal tests, verifying no partial output changes on invalid input.
  - Preserve WebSocket's current compile surface without performing spec 6's migration. Keep any shared legacy compatibility symbols narrowly scoped to WebSocket if necessary, documenting why; do not keep old REST send paths.

**Validation checkpoint:** generator L1/goldens, current schema tests, touched definition/runtime tests, and lint pass; fresh-output compile/negative cases pass. Checked-in schema is still unchanged. All consumers still build. ABI/configuration outcomes and WebSocket compatibility are proven before the breaking migration.

## Phase 7: Regenerate and Migrate Consumers

This is one coordinated phase: the temporary breaking state between regeneration and consumer edits is never presented as a completed phase. Wave 13 must precede migration; Wave 15 closes every dependent before continuing.

### Wave 13 — Schema Regeneration (sequential)

- [ ] **Regenerate Schema**
  - Regenerate all clients and generator-owned manifest/exports using the area generation recipe or its exact non-interactive generator invocation. Never hand-edit schema source or manifest. Preserve hand-maintained schema tests in their designated locations.
  - Record generated API differences and provide migration-ready examples for JSON/text/binary/empty sends, envelope extraction, status/header inspection, explicit anonymous calls, hooks, and managed builders.
  - Run fresh schema compilation before delegating consumers. Repeat the one compile-time sample under Phase 1's documented conditions; report the observation without an unsupported performance threshold.

### Wave 14 — Consumer Migration (parallel, four independent write sets)

- [ ] **Migrate Sniff Providers**
  - Own sniff generated-client uses/tests/docs. Rebuild 401 auth-required, 403/429/rate-limit, 404-absent, Retry-After, and request-ID evidence policy on envelopes, preserving undecodable/interrupted response distinctions.
  - Keep anonymous retry as an explicit second consumer call using forced anonymous options; assert the runtime made no retry and that the anonymous call cannot resend rejected client/env/header credentials.
  - Audit log/evidence paths to use redacted metadata, including redirect/header URL data. Migrate exhaustive old-error matches without losing classification (R33, R34).
- [ ] **Migrate Homelab Clients**
  - Own all five homelab crates and their related tests/docs. Migrate Eversolo and Samsung constructor injection to managed builders, preserving Samsung's 5-second connect and 10-second request timeouts and HTTPS-only invalid-certificate opt-in.
  - Migrate generic/convenience request calls and error handling; preserve device behavior with existing mock fixtures, never real-device requests. Cover all five crates even where changes are indirect (R33).
- [ ] **Migrate Model Consumers**
  - Own biscuit-speaks and model-citizen uses/tests/docs. Extract ElevenLabs binary output from the typed envelope and preserve relevant status/error handling.
  - Confirm model-citizen's actual usage; if manifest-only, verify dependency/build/test compatibility without unnecessary source edits. Document any required manifest/runtime imports (R33).
- [ ] **Migrate Schema Tests**
  - Own hand-maintained generated-crate tests and test wiring; do not edit generated files. Add generated-client integration for metadata/request IDs, credential precedence/anonymous 401, absolute-link enforcement, hooks, and malformed success evidence.
  - Register tests under `test-schema`; list migrated existing hook/guard tests. Demonstrate API-to-API rejection using actual generated types and verify WebSocket modules still compile (R2, R4, R14, R23, R28, R29, R35).

### Wave 15 — Migration Closure (sequential)

- [ ] **Close Consumer Inventory**
  - Match every Phase 1 inventory row to its migrated location and passing test or explicit review note. Search affected consumers/generated REST code for removed request/constructor/error/temp-auth paths, filtering unrelated reqwest and legitimately retained WebSocket symbols.
  - Run L1 and lint in all affected package areas, explicitly including all eight dependent crates and schema's excluded-manifest gates. Review migrated logging for secret bypasses; list behavior changes or non-mechanical migrations.
  - Rerun generation and compare before/after content diffs to prove no drift while preserving intentional schema test additions and existing unrelated work.

**Validation checkpoint:** the regenerated schema, all eight consumers, generated mock tests, and WebSocket modules compile/pass together; no inventoried legacy REST use remains. All migrations and safe-log reviews are accounted for, generation is deterministic, and compile measurements are logged. The whole phase is green before Phase 8 starts.

## Phase 8: Verify Documentation and Completion

### Wave 16 — Documentation Closure (parallel)

- [ ] **Complete Schematic Docs**
  - Own runtime/Schematic READMEs, affected topic pages, generated rustdoc templates, and Schematic skills. Describe typed/raw sends, envelope/transport boundary, declared/malformed/interrupted errors, credential sources/anonymous mode, builder/SharedTransport conflicts, origin/redirect behavior, request IDs/redaction, deadlines, and JSON hooks.
  - Give compact usable examples per rule and Mermaid diagrams for response placement and redirects. State into_result metadata loss, unchanged raw evidence, provider/raw Debug limitation, valid-JSON/content-type behavior, no runtime retries, and stream extension ownership/timeouts.
  - Remove stale injected-client/free-request guidance; document later-codec support as planned without claiming codecs ship. Current docs never name/link this feature or any other feature/fix snapshot.
- [ ] **Complete Supporting Records**
  - Own dependency records and consumer skills not owned by the docs task. Verify root and per-area dependency docs cover runtime and test crates; update sniff/provider and other affected skill guidance from actual behavior.
  - Audit changed symbols' `///`, `//!`, and inline comments, assigning corrections to owning files before the final gate. Record drift and resolutions in notes for the coordinator to merge into the log.
  - Complete R1–R37 traceability and PD1–PD35 dispositions, design decisions, author-review items, deviations, consumer inventory, OS evidence, performance observations, and unmet criteria. No newly discovered defect is dismissed as a documentation-only issue.

### Wave 17 — Final Evidence (sequential coordinator)

- [ ] **Run Final Gates**
  - In `schematic/`, run `just test`, `just lint`, `just doctest`, `just test-schema`, and `just lint-schema`; run existing `just test-l2` with the no-focus harness. Follow canonical nextest recipes; no consolidated `cargo test` or formatter invocation. Inspect recipe commands if legacy recipes still violate nextest policy before using them.
  - Run required scoped L1/lint gates for sniff, biscuit-speaks, model-citizen, and all five homelab crates. Reuse qualifying unchanged passing evidence instead of duplicating it solely for chronology; rerun after any fix that invalidates evidence.
  - Obtain/identify separate macOS, Linux, native Windows, and WSL2 build/test evidence using `os` skill routes and qualifying receipts. Match source state, package, environment, tier, features, and filters. Use authorized unattended build hosts and `BatchMode=yes`; no interactive authentication. Cross-compilation proves compilation only.
  - Prioritize truncation/closed-port timing, TLS/proxy/connection reuse, deadline cancellation, and environment isolation on each OS. Investigate OS-specific failures with the skill and update it when learning a new repo-specific fact. If evidence cannot be produced, report that environment unmet and the concrete route to complete it; never fabricate a four-OS pass or trigger unauthorized pushes.
  - Confirm runtime discovery with the resolved planner, without CI changes, and verify generation determinism after final generator/document-template edits. Do not duplicate successful measurements or run another spike.
- [ ] **Record Review Handoff**
  - Verify every active requirement has its named validation and location; R36 is withdrawn. Fill all human-review agenda entries as they occur and close the log with final commands/results, OS states, measurements, and remaining author choices.
  - Inspect changed/untracked files for scope and stray prototype artifacts, preserve user edits, and distinguish intended generated changes from drift. The working tree is expected to contain implementation changes; “clean of stray edits” does not mean discarding them.
  - Describe downstream pagination inputs (`Deadline`, completed-request accounting, page slot, InvalidArgument, check_link, redirect failures) and later codec/hook extension points. Flag updates needed for the frozen R20 surface test and any prerequisite departures.
  - If interrupted or truly blocked, log the last completed green phase, partial-wave state, blockers, and recovery actions; set the spec's `message_to_agent` for the next implementer. Do not mark unmet requirements or proposed gaps complete, commit/push, run `just complete`, or move the feature directory.

**Validation checkpoint:** final scoped evidence, deterministic generated output, docs/skills/dependency records, and complete implementation log support the review handoff. All four OS evidence states are explicit. Only passing required criteria permit **implementation complete, ready for review**; otherwise report precisely what remains.

### Requirement Coverage

| Requirements | Primary implementation and evidence |
| --- | --- |
| R1, R37 | Phase 2 package/recipes/discovery; Phase 6 runtime-reference emission; Phase 8 final discovery. |
| R2 | Phase 3 runtime bounds; Phase 6 generated API markers/compile failures; Phase 7 actual-client tests. |
| R3–R8 | Phase 2 shapes/redaction/docs; Phase 3 network evidence/classification; Phase 8 documentation check. |
| R9–R13 | Phase 3 argument/timeout/body/error tests; Phase 4 pre-request origin rejection. |
| R14–R16 | Phase 3 effective-policy/credential tests; Phase 7 generated-client and consumer behavior tests. |
| R17–R19 | Phase 3 managed construction/shared transport; Phase 4 constructor redirect matrix; Phase 6 removed-surface tests. |
| R20–R25 | Phase 4 definition/origin/redirect tests; Phase 6 configuration translation; Phase 7 generated link test. |
| R26 | Phase 3 deadline/ownership/cancellation; Phase 4 redirect budget/evidence; Phase 8 OS evidence. |
| R27–R30 | Phase 5 raw/hooks/shared-core audit; Phase 3 test decoders; Phase 6/7 generated surface and regression tests. |
| R31–R32 | Phase 6 thin output, isolated compile, ABI mismatch, partial-generation refusal, Rust marker input verification. |
| R33–R34 | Phase 7 all eight consumers, behavior preservation, inventory closure, safe-log review. |
| R35 | Phase 6 fresh WebSocket compile; Phase 7/8 generated schema compilation/lint. |
| R36 | Withdrawn; anonymous sends verified under R14/R16. |

Each implementation task includes its meaningful tests and relevant doc/comment updates. Phase checkpoints use touched-package tests/lint and dependent build checks, not whole-workspace runs. Parallel agents receive narrow write sets and the explicit instruction that the entire session tree is non-interactive; the coordinator owns shared manifests, cross-module integration, and implementation-log merges.

## Refresh log

### 2026-10-12 — generation-integrity review 3

- Replaced the pre-implementation dependency warning with verified tuple, authentication, compatibility, and fresh-output harness interfaces.
- Replaced the assumed serialized-inventory matrix with generated-Rust ABI-marker checks because no configuration inventory was created; retain the full matrix if a later implementation actually introduces such a reader.
- Updated the endpoint count to 547 and recorded sniff's GitHub/Bitbucket migration as the behavior-preservation baseline.
- Carried forward the author's REST-selector withdrawal; runtime scope, settled decisions, and pipeline ordering are unchanged.

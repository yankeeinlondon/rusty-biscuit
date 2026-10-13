---
total_phases: 8
created: 2026-10-11
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
---

# Schematic pagination implementation plan

## Phase 1: Reconcile contracts and establish the baseline

### Necessary Rules

This work replaces inert pagination metadata with an authored model, a shared runtime pager, thin generated capability implementations, and verified API defaults and exceptions. Success means callers can fetch one configured page or traverse a collection with exactly the authored controls; `collect_all` reports completion only after the provider signals the end. Failures preserve gathered items and response evidence, sessions attempt bounded cleanup, imported definitions retain pagination, and existing exports remain unchanged.

The implementation ends **implementation complete, ready for review**. This plan authorizes no implementation during the planning session, commits, spec edits, sibling-spec edits, or lifecycle moves.

Adopt the following defaults during implementation and record their provenance, alternatives, and reversal path in `implementation-log.md`. They remain planning choices where the specification labels them Proposed or Default.

| Topic | Implementation rule |
| --- | --- |
| Prerequisite gate | Require the reviewed outcomes of `2026-10-12-schematic-runtime-envelope` and the generation-integrity work before Phase 2. Confirm the actual runtime package, `EndpointSpec`, codecs, envelope, error kinds, page metadata, managed client, typed send, origin resolver, and shared deadline. Missing contracts block implementation at Phase 1; do not implement private replacements. Record reconciliation here and in the log rather than silently rewriting the snapshot spec. |
| Representation | Use named strategy structs inside the four-kind enum; owning define-side types and static runtime projections; one generic `Pager<E>` / `Pages<E>` over a non-endpoint-generic engine. Use method-level capability bounds and the diagnostic attributes recommended by the prototype. No generated per-endpoint pager method bodies. |
| Resolution | Endpoint exception first; otherwise only an explicit GET `Vec<T>` response can inherit the API default. Other methods and wrappers need entries. Do not infer through aliases or opaque descriptors. Retain resolution origin and caller knobs. |
| Exceptions and items | Use an ordered list of endpoint-id/`Collection` entries; reject duplicate and unknown ids, with a nearest-id hint. Align diagnostics with the landed generator format. Wrapper entries name a Rust item type path resolved by the existing resolver, including registered types, primitives, and qualified paths. |
| Managed inputs | Adopt a plain parameter only at the same destination and with an authored matching knob; omit its request field and constructor argument. Required managed inputs need a sendable strategy default. `server_default` is informational. Optional managed inputs are omitted on ordinary sends; required authored defaults are sent. Preserve request `Default` only when remaining fields permit it, including the typed-path and required non-pager rules. |
| Legacy metadata | Keep `PaginationStyle`, `PaginationResponse`, `with_pagination`, and the two legacy fields; document them as superseded without deprecation attributes. Their already-materialized query parameters can be adopted, but their pagination metadata cannot drive pager emission. Endpoint and EndpointParams literals gain no fields. |
| Entry points | **Superseded** by `2026-10-12-schematic-runtime-envelope` review 1, Item 1: the primary entry is a method on the endpoint's generated call type from `2026-10-12-schematic-endpoint-methods` (see the spec's inline review note); refresh this row before the plan runs. Was: generic `client.pager(request)`, configuration knobs, then `fetch_page()`, `pages()`, or `collect_all()` for non-session endpoints. `fetch_page()` returns `Page<Item>` and sends once. Session endpoints expose only `with_session`; iteration, collection, and an authored `renew()` are available inside the scope. |
| Continuation history | Detect any repeated exclusive continuation in forward traversal, including the starting continuation and nonconsecutive cycles. Store exact tuple keys/resolved URLs; use identity progress for inclusive boundaries. Histories are separate by direction and partition. Failure is terminal. |
| Collection errors | Add `InvalidContinuation`, `InvalidArgument`, and `TransportError` to the specified incomplete reasons. Default page limit: 100, readable/settable and positive, counting attempts within the helper. A final permitted page that proves completion succeeds. Preserve underlying cause/envelope and items from valid pages even if their continuation fails. |
| Alternates | Model and validate labeled alternates; emit no alternate-selection methods or constants influenced by alternates. |
| Origin policy | Check every next-page link with `Client::check_link::<R>(&url)`, where `R` is the listing request type, and send an absolute link through the runtime's checked absolute-URL request path (`2026-10-12-schematic-runtime-envelope` review 1, Item 4). Use the runtime policy and final response URL for relative-link resolution. Remove strategy-local trust and `InvalidTrustedOrigin`. Follow the landed ruling for `deny`; absent a ruling, use the spec's same-origin fallback and record it. |
| Session signature | Use a borrowed scoped pager and a boxed future tied to its borrow; log the final signature. Catch panics during closure construction and polling under unwinding, close once when a usable handle exists, then resume the original panic. No async drop or detached tasks. |
| Cleanup budget | Every close attempt uses a configurable 5-second grace, capped by the client's per-request timeout, independent of the page deadline. The preceding runtime review's ruling overrides this default. Successful close preserves the closure result; failed close returns `SessionError { outcome, close }`. Cancellation, abort, and termination rely on the lease. |
| Partition ownership | Caller-driven independent iterators, zero/background spawning. Session partitions share one open/close and the latest observed handle; do not serialize requests or reject rotating handles. Log the accepted out-of-order invalidation risk. |
| Provider evidence | Author only V-def/V-notes rows whose required/optional inputs, collection shape, continuation, and end signals agree. Keep U rows unpaginated unless separate verification authorization is recorded. No live credentialed calls. Keep six outside-pattern tests named `unverified_*` unless evidence changes. |
| Conditional edits | Treat the spec's pre-authorization list as pending until actual authorization is recorded. Missing authorization for network verification or the EMQX response fix skips those optional steps and records the gap; it does not block supported rows. Do not assume `yolo: true` grants those permissions. Other missing authorization for required implementation scope must be recorded before proceeding. |
| Measurement scope | Existing 56 model tests and the pager prototype's 31 runtime/21 compile cases answer representation and capability feasibility; do not repeat those spikes. The reported 1.77 MB size cost identifies an unresolved decode choice, not an adoption threshold. Run one quick decode comparison on one host before committing to the production decode path. No wider performance campaign, HTTP/2 study, incremental-build study, or rust-analyzer benchmark is required. |
| Test scope | All new behavioral and local mock-server tests are L1, using nextest and repository fixtures/toolkit. No L2/L3, browser, real-provider, or focus-taking test is needed. Never run `cargo fmt`. Load the OS skill before platform-specific work or claims about host limitations. |

### Execution conventions

Waves are numbered globally. Tasks in the same wave may be delegated concurrently **only** when their stated file ownership is disjoint and prerequisite contracts are frozen. Waves run in order; a single-task wave is intentionally sequential. Each task has a phase-local identifier for dependency references. If a delegate needs a shared integration file, its owner integrates the change after the wave. Every delegated brief must repeat that the whole session tree is non-interactive; no delegate waits on another delegate.

Each phase records changes, decisions, validation, deviations, and remaining risks as they occur. Run the relevant L1 and lint recipes for touched packages and ensure the generated crate still compiles at each boundary; reuse unchanged passing checks instead of regenerating or retesting unrelated code. The refreshed area recipes must include the runtime package. Phase 1 records existing failures rather than labeling them regressions.

### Wave 1 — Sequential preflight

- [ ] **1.1 Contract refresh**
  - Read the preceding runtime review outcomes and the current implementation. Produce a contract map covering every refresh item in the spec, including `RedirectFailed`, invalid-argument kinds, response decomposition, cleanup deadline precedence, and the final RestApi literal shape.
  - Confirm generation integrity supplied partial generation, the isolated compile check, query-parameter separation, and diagnostics. Record absent prerequisites and stop dependent work at this gate.
  - Review actual authorization for regeneration, dependent migrations, runtime/dev-dependency changes, definition authoring, and docs. Record the two optional authorization-dependent steps separately.
  - Inspect existing user changes before selecting files; create the implementation log following the worktree area's example. Copy the spec's entire 20-item Human review agenda into the log with an evidence/status slot per item.

### Wave 2 — Parallel baseline work

- [ ] **1.2 Coverage inventory**
  - Own the endpoint inventory in the log: enumerate all 18 APIs, every list candidate, explicit non-GET lists, GET arrays requiring opt-out, and declared legacy pagination. Recount current list/endpoint/legacy totals rather than hard-coding historical counts.
  - For each candidate record item type, selector, resolution origin, intended strategy/knobs, evidence category/source/date, and unpaginated reason. Reconcile the OpenAI S6/S7/S8 groups and the three special opt-outs.
  - Search every actual direct/relevant dependent for adopted-field construction and setters, including calls customizing one page. Build an exact field-removal-to-call-site migration ledger; do not begin sniff's broader client retirement.

- [ ] **1.3 Artifact baseline**
  - Own baseline artifacts in a temporary directory: capture generated sources, OpenAPI and Postman outputs, and existing request surfaces from the landed generator. Record the generated-crate build time and compiler/cache context.
  - Verify baseline generation/compilation, area L1/lint, schema tests/lint, and the isolated-output check using current non-interactive recipes. Preserve existing failures and expected differences without overwriting user changes.
  - Identify which supported serialized authoring inputs are actually read; finalize the input-matrix applicability and field inventory below before Phase 2.

### Input robustness scope

The generic importer overlay adds a CLI option and emitted Rust code, not a new configuration file or changed OpenAPI parsing policy. Rust API literals and provider response bodies are not configuration files. Do not broaden this feature into hardening every existing OpenAPI field.

The new pagination model nevertheless requires serde deserialization (R1). If exposed as serialized authoring input, it is load-bearing configuration: complete a matrix for **every concrete field** in that supported input before implementing its reader. Use JSON unless the refreshed public contract actually supports another format; create a separate matrix and fixture-walking test for each additional supported format. If serde is only a model round-trip facility with no configuration reader, record that boundary explicitly and retain strict model decode tests without inventing a loader. R12/R17 response-shape tests remain required in either case.

The following outcomes constrain that matrix; 1.3 expands each category into individual field columns using the final public type definitions (including nested dimensions), with no field left implicit. Absent/null may have equal outcomes only where the serialized format explicitly defines both as no optional dimension.

| Shape | Required discriminants, input destinations, selectors, item paths | Optional dimensions: size, total, follow-up, session, partition | Collections: cursor parts, constants, carry names, alternates, exception entries | Numeric bounds and sendable defaults |
| --- | --- | --- | --- | --- |
| Unedited control | Valid definition resolves to expected strategy and knobs | Authored dimension appears in resolution | All members affect expected resolution/validation | Expected bounds/default affect validation or sent request |
| Absent | Diagnostic; never an empty strategy | No dimension, explicitly defined | Required list: diagnostic; optional list: no entries only if format defines this | Required: diagnostic; optional: no declared bound/default |
| Explicit null | Diagnostic | Explicit no dimension only where defined; otherwise diagnostic | Diagnostic, distinct from absent/empty | Diagnostic, distinct from absent |
| Wrong whole type | Diagnostic | Diagnostic | Diagnostic | Diagnostic |
| Wrong one element | Diagnostic, where composite | Diagnostic for nested composite | Diagnostic for entire input; never discard member | Diagnostic for nested composite |
| Wrong all elements | Diagnostic, where composite | Diagnostic for nested composite | Diagnostic; never resolve to empty | Diagnostic for nested composite |
| Empty | Reject unusable destination/path; root selector requires explicit root form | Reject empty object lacking required dimension fields | Reject empty required cursor legs; explicit empty optional lists mean zero entries | Reject empty value; zero allowed only by field contract, never for positive bounds |
| Duplicate key | Reject duplicate object field | Reject duplicate object field | Reject object duplicates and duplicate exception ids/alternate labels/destinations by their validation rules | Reject duplicate object field |
| Trailing/invalid content | Reject entire document | Reject entire document | Reject entire document | Reject entire document |

For applicable configuration readers, one test per format starts with a real-tool fixture, proves the control, and walks every field/shape with one edit per cell. Assert public resolution/validation or generated/send behavior, not merely serde return values. Use structured edits for values and raw-text edits for duplicate/trailing-content cases. Reject trailing JSON and duplicate keys before a lossy Value/map can erase them. Inspect `#[serde(default)]`, `Option<T>`, `filter_map(..as_str())`, `unwrap_or_default()`, and `.ok()` on load-bearing reads. A review finding records the rest of that row and field column. Mark structurally impossible cells as such with a reason rather than silently skipping them.

**Checkpoint:** contracts and scope are reconciled; baseline artifacts and migrations are enumerated; configuration-reader applicability is explicit; all implementation defaults and optional gaps are logged. Phase 2 cannot begin on missing prerequisite runtime contracts.

## Phase 2: Establish the pagination model

### Wave 3 — Sequential model integration

- [ ] **2.1 Model contracts**
  - Implement the four named strategy variants, builders, serde support, open-enum attributes, selectors/paths, input/value sources, bases, polarity/counts, size/more/cursor/boundary/link dimensions, constants, follow-up/carry, sessions, partitions, collections, and alternates in `schematic/define/src/pagination.rs` and focused submodules.
  - Provide no public Default that creates an empty usable-looking strategy. Preserve legacy symbols, update their docs to superseded, and export the new types through existing public/prelude surfaces.
  - Add the two RestApi fields and initialize every literal, importer constructor, fixture, and definition emitter to preserve compilation. Do not add fields to Endpoint/EndpointParams or rewrite definition endpoint literals.
  - Freeze the resolution/static-projection contract consumed in Phase 3. If a configuration reader is applicable, publish its exhaustive field matrix before implementing deserialization.

### Wave 4 — Parallel model behavior and evidence

- [ ] **2.2 Resolution validation**
  - Own production resolution/validation: pure exception/default/none precedence, explicit GET Vec recognition, caller knobs, item type/path checks where schema is known, and runtime fallback for opaque schemas.
  - Implement every R2 rule with the adoption exception, required-managed-default rule, destination conflicts across constants/session/continuation/size/partition, unknown/duplicate table ids and nearest-id diagnostics. Remove only strategy-local trust validation.
  - Wire the validator into `schematic/gen/src/validation.rs`; record kept/renamed/dropped design rules. Do not claim opaque collection structure was checked.

- [ ] **2.3 Model evidence**
  - Own separate test files: port applicable assertions from the 56-test prototype, including the 16 inventory tests' eventual real-definition contract, without importing obsolete endpoint fields or LinkTrust behavior.
  - Test each rejection with its valid sibling, resolution precedence, explicit non-GET lists, wrapper types, duplicate ids, required defaults, adoption with/without knobs, and near-miss locations. Freeze current Endpoint/EndpointParams literal compilation for R3.
  - Implement the applicable serde/configuration matrix from Phase 1, alongside round-trip tests. Assert public effects and preserve absent/null/empty distinctions from the declared format.

**Checkpoint:** R1–R8 and the define-side R26 contract pass; empty defaults preserve baseline output; existing literals compile unchanged. Model/runtime projection and diagnostic contracts are stable enough for independent runtime and generator work.

## Phase 3: Build pager mechanics and generation

### Wave 5 — Parallel implementation on frozen contracts

- [ ] **3.1 Pager mechanics**
  - Own runtime pager production files: static projections, Pager/Pages, capability traits, Page/error shapes, four strategies, configuration versus iteration state, source-record counts, item streams, and readable page/item counters.
  - Implement explicit-signal precedence; missing/invalid continuation distinctions; null/empty/authored terminal values; nonterminal zero/false cursors; empty-page contradictions; missing/malformed totals; FullPage only with known effective size; checked arithmetic and both index bases.
  - Implement record-before-projection cursor extraction, nested array fan-outs in document order, no partial delivery from malformed pages, composite all-or-none cursors, inclusive identity filtering, exact loop histories, and terminal failed state. Keep decode access behind a stable internal seam for Phase 4's single measurement.
  - Implement optional/required size semantics, pre-send validation with request metadata, and authored adjustable size from the next request.

- [ ] **3.2 Pager emission**
  - Own generator codegen/assembly/request-struct files: replace the empty marker with static constants, item association, and only authored marker implementations. Emit nothing for non-lists/unpaginated lists, and no `using_*` or per-endpoint method bodies.
  - Remove adopted inputs from request fields and constructors using the same resolution/adoption contract as validation; retain ordinary parameters in definitions and preserve required non-pager/typed-path Default rules.
  - Resolve wrapper item types through the existing Rust type resolver. Add deterministic snapshots, alternate/legacy-metadata independence tests, and isolated generated compile fixtures using the runtime contract.
  - Update dependency/manifest assembly so emitted clients import the actual runtime package. The generator task integrates its own exports/imports; runtime module registration belongs to 3.1.

- [ ] **3.3 Core fixtures**
  - Own separate runtime test fixtures: port boundary/selector cases and prepare endpoint-shaped handwritten impls, local page servers, and positive/negative capability cases against the frozen contract.
  - Prepare the R30 mechanism matrix: GitHub page/Link, GitLab page/X-Next-Page and modeled keyset link, Gitea page/limit/total, Bitbucket next/previous/size, OpenAI S6/S7/S8 and item-id variants, Anthropic bidirectional ids, the three ElevenLabs strategies, nested EMQX metadata, and both Hugging Face strategies. Unverified provider fixtures exercise mechanisms without authoring those providers' real definitions.
  - Include AIP-158, Mastodon, SCIM, and Salesforce Bulk 2.0, plus `unverified_*` X API v2, WordPress, OData skiptoken, Cosmos DB, Salesforce REST query, and Reddit cases. Execute these fixtures through the integrated runtime in Phase 4, not only through extraction helpers.
  - Cover every R10 knob, forbidden post-pages knobs, no pager for an unpaginated endpoint, and the two positive cases. Keep fixture compilation separate from production edits; execute after 3.1 is integrated.
  - Use repository test helpers and temp isolation, with no host paths, fixed synchronization sleeps, leaked children, or external providers.

**Checkpoint:** core state tests and runtime trybuild pass; generator snapshots and isolated output compile; malformed collections cannot become empty successful pages. Provisional decode mechanics remain replaceable until Phase 4.

## Phase 4: Integrate typed transport and complete-list semantics

### Wave 6 — Single decode spike

- [ ] **4.1 Decode sample**
  - Before selecting the production decode path, compare provisional from_value decoding with shared from_slice decoding using the existing bloat scripts adapted to the landed runtime and current inventory count.
  - Keep the same representative endpoint set and release settings for one before/after sample on the implementation host, using scratch/temp build outputs. Preserve item projection, continuation access, envelope evidence, and one-pass item-stream behavior in the candidate.
  - Record binary sizes, host/compiler/cache context, correctness observations, and the chosen path. Prefer reuse when correct and simple; retain from_value with the concrete reason otherwise. No percentage gate, extra hosts, workloads, or later remeasurement.

### Wave 7 — Sequential runtime integration

- [ ] **4.2 Transport integration**
  - Implement the selected decoder and route every page through typed send with page-only RequestMeta, credential resolution, redaction, envelope preservation, and shared monotonic deadline. No retries or detached work.
  - Implement fetch_page's single send and original-wrapper ordinary-send behavior with required authored defaults. Expose next/prev/items/total only where supported.
  - Build follow-up method/path/carry rules, preserving repeated query values, case-insensitive header destinations and exact body segments. Managed inputs win; validate conflicts rather than silently overwriting them. Reject non-replayable bodies when reuse is required.
  - Parse all Link field values using the shared standards-aware parser; handle quoted commas, multi-token rels, identical versus ambiguous resolved next links, and invalid links. Resolve against the final response URL; apply carry before final runtime origin/security checks, including redirected followed links. Without a carry rule follow next links verbatim using GET.
  - Implement collect_all with retained cause/envelope/items and exact attempted-page limit semantics. Ordinary malformed/interrupted/refused envelope failures map to HttpError; RedirectFailed maps to TransportError with its prior 3xx. Map boundary stalls, missing/invalid continuation, invalid arguments, deadline, origin refusal, and other transport failures explicitly.

### Wave 8 — Parallel runtime acceptance tests

- [ ] **4.3 Traversal evidence**
  - Own traversal mock tests for all R11–R17/R21 rules: configured one-page send, required defaults and overrides, min/max zero-send checks, adjusted size, bases/overflow, total and explicit-more contradictions, empty cursor/link pages, composite boundaries, cyclic continuations, and failed-stays-failed.
  - Test every IncompleteReason with exact items/cause/envelope; retain valid-page items after bad continuation and exclude malformed-page items. Cover resumed starting position, zero limit, completion exactly at limit, and intermediate empty pages.
  - Exercise Shopify Only, AIP-158 AllExcept, and replacement request shapes including carry destinations and replay rejection.

- [ ] **4.4 Transport evidence**
  - Own two-origin link/policy/credential/redirect tests, relative `?page=2` and `../next`, scheme/host/port/userinfo cases, repeated/quoted/ambiguous Link fields, and verbatim versus authored query carry.
  - Test page-two 500, malformed 200, interrupted body, redirect refusal and failed hop; assert original status/headers/raw body evidence and page metadata only on paginated sends.
  - Test monotonic deadline exhaustion, completed-page/elapsed reporting, zero further sends and connection cancellation on drop. Use synchronized server events and at least 100 ms timing margins where real time is required, within the L1 time budget.
  - Add diagnostic sentinel tests for cursor/URL/auth secrets; raw response evidence retains the runtime's documented privacy limitations.

**Checkpoint:** R11–R21, R24, and relevant R36 behavior is proven through typed transport. Incomplete results cannot masquerade as success; refused origins receive neither requests nor credentials; the decode choice is final and logged.

## Phase 5: Add scoped sessions and partitions

### Wave 9 — Sequential lifecycle implementation

- [ ] **5.1 Scoped lifecycle**
  - Own runtime lifecycle files and final public signature: separate/open-on-first-page modes, session page target, in-band keep-alive, authored renew, and latest-handle replacement before item/continuation validation can fail.
  - Await one bounded close after closure success/error/construction panic/poll panic when a usable handle exists. Preserve the closure outcome in SessionError on close failure; resume the original panic after cleanup. No open/close surface or outside-scope pages.
  - Bound all close attempts by grace independently of page budget, including early return. Document lease reliance for cancellation/open failure/abort/termination and inability to return close evidence on resumed panic.
  - Implement caller-driven partition iterators, range checks, index/count on every request, independent histories, shared session ownership and latest-observed handle for subsequent page/close requests. Preserve concurrent polling without spawning or serialization.

### Wave 10 — Parallel lifecycle evidence

- [ ] **5.2 Session evidence**
  - Own scroll/PIT lifecycle tests: open/page/keep-alive/renew/close request records, each exit/panic path, expiry cleanup, early-return grace, slow/failed close, valid and failed closure outcomes, and latest handle after a malformed page.
  - Confirm no addressable close on open failure without a handle, and no detached work after cancellation. Add secret-handle and failed-close diagnostic sentinels.
  - Extend compile-fail cases for manual open/close, pages/fetch outside session scope and unauthored renew/keep-alive controls; verify permitted inside-scope collection.

- [ ] **5.3 Partition evidence**
  - Own four-partition PIT tests: one session open/close, index/count on all requests, independent continuations/stalls, zero/over-max rejection, and latest handle across partitions and cleanup.
  - Validate a rotating-handle/partition definition positively. Use coordinated out-of-order responses to prove latest-observed semantics without claiming provider safety. Log the accepted ordering/invalidation risk.

**Checkpoint:** R22/R22a/R23 and complete privacy coverage pass. Session cleanup preserves both results and failures; cancellation semantics and rotating-handle risk are documented rather than hidden.

## Phase 6: Author verified APIs and preserve imports

### Wave 11 — Parallel authoring with disjoint ownership

- [ ] **6.1 Forge strategies**
  - Own GitHub/GitLab/Gitea/Bitbucket modules: implement API-local strategy helpers and exceptions from the verified table, provider limits and informational server defaults, wrapper item paths/types and totals, and authored backward controls.
  - Model GitLab keyset alternates without emitting caller selection. Explicitly opt out unverified/default-inferred candidates such as Gitea tag/tree cases and GitLab participants; confirm inventory classifications rather than trusting name matches.
  - Prepare field-removal ledger entries and coverage expectations; do not modify endpoint literals or migrate shared dependents concurrently.

- [ ] **6.2 OpenAI overlay**
  - Own the hand-written `openai_pagination` module and its tests: S6/S7/S8 strategy defaults/overrides, data wrappers/item types, last-item-id cursor exceptions, supported before legs, limits and required sendable defaults from the vendored document.
  - Preserve opt-outs for ListModels and the two vector-store provider gaps. Assert each endpoint's controls rather than only group counts.
  - Keep overlay code outside importer-owned output; defer registration/imported factory edits to the integration owner in Wave 12.

- [ ] **6.3 Importer overlay**
  - Own generator CLI/import options, `import_pipeline.rs`, `definition_gen.rs`, and recipe plumbing: a generic validated overlay function-path option emits one call after API construction. No OpenAI-specific generator branch and no new config file.
  - Emit compiling RestApi literals for any API; no configured hook emits no call. Test temporary imported output compilation, unchanged overlay-file bytes, and before/after resolution through the same public factory used by clients and exporters.
  - Use a small vendored-derived fixture for routine L1 compilation and the actual vendored OpenAI input for the catalog re-import check; no network or additional performance sample.

- [ ] **6.4 Remaining coverage**
  - Own ElevenLabs verified strategy helpers, the all-API inventory test file, and gap records. Add empty fields/default opt-outs for the remaining APIs as required; do not apply recalled Anthropic/EMQX/Hugging Face strategies.
  - Explicitly enumerate every GET Vec endpoint under an API default and assert opt-outs as well as paginated entries. Include non-GET list recognition and non-list array writes.
  - If separately authorized evidence verifies U rows, record provider source/date and all required contract facts before authoring. Apply ListAuthUsers response correction only with both verification and explicit edit authorization; otherwise leave it unchanged and unpaginated. An inconclusive source stays a gap.

### Wave 12 — Sequential catalog integration

- [ ] **6.5 Catalog integration**
  - Integrate module exports and the configured OpenAI overlay invocation; run the updated import recipe and validate unknown exception ids against its output.
  - Merge authoring expectations into one R27 coverage test asserting endpoint id, resolved origin, strategy and exact knobs across all 18 APIs. Match every unverified endpoint to its verification-task entry.
  - Compare imported resolution before/after and prove the overlay survives a second import. Review every destination adoption and expected removed request field.

**Checkpoint:** R27/R28/R33/R35 pass against real definitions. Supported evidence is distinguished from mechanism-only tests; OpenAI re-import retains pagination without hand edits; literal changes remain within R3's explicit exception policy.

## Phase 7: Regenerate and migrate dependents

### Wave 13 — Sequential regeneration and migrations

- [ ] **7.1 Generated migration**
  - Regenerate schematic-schema and exports exclusively through the generator, using the existing non-interactive generation entry point. Migrate every ledger call site that uses removed adopted fields in the same wave: configured single-page callers use fetch_page and handle Page<Item>/metadata instead of the old wrapper.
  - Preserve unrelated ordinary typed sends and provider-specific clients; do not expand into sniff retirement. Discover actual schema dependents from manifests rather than assuming the illustrative list is exhaustive.
  - Keep test sources outside generator-owned paths unless the generator explicitly emits them; ensure repeated generation preserves generated-surface tests and their manifest registration.
  - Review schema diff statistics and a field-level removal list. Assert byte equality of OpenAPI/Postman against baseline, with only the exact separately approved response correction if any. Generate a second time and compare bytes for deterministic output.

### Wave 14 — Parallel end-to-end evidence

- [ ] **7.2 Generated evidence**
  - Own schema tests: local-server forge page-number, OpenAI cursor, and Bitbucket next-link traversals through actual generated code, including collect_all and configured one-page cursor/size sends.
  - Run the complete capability fail/pass suite against a real generated endpoint set, including session prohibitions and no alternate-selection API. Make snapshots portable across host/compiler path mappings; if needed assert custom diagnostic text rather than unstable path output and log the deviation.
  - Run D49 isolated generated compilation, `just test-schema`, and `just lint-schema`; record after-build time with the baseline context, without a new performance campaign.

- [ ] **7.3 Dependent evidence**
  - Own validation/log records only: build and run root `just test {pkg}` and relevant area lint for actual dependents, including sniff, biscuit-speaks, model-citizen, and homelab crates.
  - Confirm every removed-field use was migrated, no unrelated dependent behavior changed, and error/envelope handling still compiles with actual generated results.
  - Report failures to the migration owner for sequential fixes; do not have two workers edit the same dependent files.

**Checkpoint:** R25/R29/R31/R32 pass. Generated and isolated crates compile; exports and regeneration are deterministic; real generated controls match the model; dependents build and test successfully.

## Phase 8: Finish documentation and acceptance evidence

### Wave 15 — Parallel documentation and platform evidence

- [ ] **8.1 Documentation drift**
  - Own `schematic/docs/pagination.md`, its index link if present, define/gen/runtime READMEs, affected area/catalog/schema READMEs, dependency docs, and Schematic skills.
  - Explain model authoring, GET recognition/exception order, adoption/default rules, one-page result migration, compile-time controls, complete-traversal semantics, limits/error evidence, session cleanup/grace/cancellation, and partitions. Include compact examples and Mermaid resolution/lifecycle diagrams for a new developer.
  - List each unverified endpoint with provider, id, recalled strategy, and needed source; distinguish definition/provider gaps. Never link or name feature/fix directories from topic docs.
  - Review docs/comments for every changed symbol and remove the old nonexistent fetch-all claim. Log detected drift and its correction; do not add unrelated comment cleanup.

- [ ] **8.2 Platform evidence**
  - Load the OS skill and collect/reuse qualifying evidence for changed packages and schema on macOS, Linux, native Windows, and WSL2 according to repository policy and actual available hosts.
  - Prove portable trybuild paths and deadline tolerances, with no focus-taking windows. Report exact local/reused/CI-pending cells; absence of evidence is not a passing result. Required coverage without qualifying evidence must run when available, while respecting explicit execution constraints.
  - Use existing event-scheduled CI coverage; do not add speculative matrix cells or trigger a full-scope run to investigate behavior. If remote evidence is unavailable, record the blocker and remaining required validation honestly.

### Wave 16 — Sequential closure audit

- [ ] **8.3 Acceptance audit**
  - Run final scoped `just test`, `just lint`, `just test-schema`, `just lint-schema`, and the isolated generation check after the last behavior changes. Verify the runtime is included in area recipes. Avoid cargo test and automatic formatting; L2 remains unaffected.
  - Confirm R1–R36, including R2a/R11a/R22a, each points to a concrete test or reviewed artifact; the same acceptance checks cover either chosen decode path. R30 must include every inventoried mechanism and all ten outside patterns, with the six unverified names preserved.
  - Reconcile all 20 Human review agenda entries in the implementation log: defaults, prerequisite reconciliation, design choices, adopted/retained fields, provider coverage/sources, counts, single decode sample, build/diagnostic observations, platform evidence, deviations/open questions, drift, reversibility, proposed choices, cleanup grace, incomplete mappings, decided provenance, and rotating-handle risk.
  - Review endpoint-literal changes, schema diff size, export equality, import persistence, dependency docs and skill updates. Resolve missing evidence or report it as an outstanding requirement; never mark an unproven requirement complete.
  - Leave the feature in its active directory and report **implementation complete, ready for review** only when required work and evidence are complete. Otherwise report the precise blocking prerequisite or validation gap, with the next action needed. Do not commit or move the feature.

**Checkpoint:** the implementation is reviewable with deterministic artifacts, demonstrated public behavior, documented limitations, and a complete decision/evidence log. The author owns review closure and lifecycle movement.

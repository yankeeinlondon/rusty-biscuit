---
$schema: feature-review.yaml
ready: false
findings: []
observations: []
human_in_the_loop:
    - |-
        **Item 1 — Separate server sessions opened by partitions need a cleanup policy.**

        A partition divides a large listing into independent traversals. The author already decided that partitions may share one session whose handle changes. The specification says: “on a session, one session is opened for all and one close attempt is awaited when the scope ends” (requirement “Partitions”). That decision remains settled for a genuinely shared session.

        The premise changes in the implementation log: when the first page opens the session, each partition can open separate server state, while only the last observed handle is closed. Four partitions can therefore leave three sessions to expire even after normal completion. This is reported in the log, not independently verified in this review. The affected lifecycle is in schematic-runtime’s [session implementation](../../runtime/src/pager/session.rs); the evidence is [implementation log, Phase 5](implementation-log.md#phase-5), lines 2150 and 2168–2169.

        This meets **test 2, contradicts a decision**, because multiple separately opened sessions differ from the decided shared session. It also changes the cleanup premise: relying on expiry after ordinary success is broader than the already accepted cancellation and out-of-order-handle risks.

        Options:

        - **Reject partitioned first-page session opening until independent-session cleanup is designed.** This prevents knowingly abandoning separate sessions and preserves the shared-session design. It temporarily narrows the supported combinations, so it needs an explicit author ruling.
        - **Track and close every independently opened session.** This supports sliced searches correctly, but adds handle ownership, multiple close attempts, and a way to report several cleanup failures.
        - **Accept expiry as cleanup for the additional sessions.** This keeps the implementation small, but consumes server resources after successful calls and weakens the cleanup guarantee.

        **Recommendation:** choose the first option. No shipped definition uses sessions or partitions, according to the log, so building multiple-session ownership now would add maintenance without a current consumer. Do not reject ordinary shared sessions with rotating handles; their author ruling still applies.

        **If the answer is “go with your recommendation”:** record the narrower supported combination and have the repair cycle add focused validation, its test, and matching documentation. Leave shared-session handle rotation unchanged.

        **If undecided:** the review cannot establish which session combinations the feature promises to support, and later definition builders could make the ambiguous combination easy to author.
    - |-
        **Item 2 — Decide which response callbacks paging promises to execute.**

        schematic-runtime lets callers transform JSON responses with a global callback and change an endpoint’s decoded response with a callback registered for that endpoint. The log chooses to run the global callback before extracting page items and continuation values, while skipping the endpoint-specific typed callback. Ordinary sends can run both. This matters because changing an ordinary listing call to automatic paging can silently bypass a caller’s response transformation.

        The evidence is [implementation log, Phase 4](implementation-log.md#phase-4), line 1978. The public callback behavior is described by schematic-runtime’s [hooks module](../../runtime/src/hooks.rs), and page decoding lives in its [pager request module](../../runtime/src/pager/request.rs). The spec supplies no planning default for this difference.

        This meets **test 1, hard to reverse**: it fixes public callback behavior across the runtime and generated clients. Changing it later changes what existing callbacks see and can require another response-decoding path and revised generated-client tests.

        Options:

        - **Keep the logged behavior: global JSON callback runs; endpoint-specific typed callback does not.** It fits the page item result and avoids decoding the entire endpoint wrapper solely for a callback. Callers must know that endpoint-specific transformations are absent during paging.
        - **Run both before extracting the page.** This better preserves ordinary-send transformations, but requires a defined conversion from the mutated endpoint response back to items and continuation data, with extra decoding or conversion work.

        **Recommendation:** keep the logged behavior and make the omission an explicit public contract. Require a focused test of both callback paths through generated paging code. This avoids adding a second response conversion mechanism without a demonstrated consumer.

        **If the answer is “go with your recommendation”:** record that paging uses the global callback only; the repair/review cycle checks the documentation and adds or confirms a generated-client test that demonstrates the difference. The author is deciding callback semantics, not endorsing the current code’s correctness.

        **If undecided:** callers and later generated APIs cannot rely on a stable answer about which configured transformations paging executes.
    - |-
        **Item 3 — Decide what repeated calls return after a pager has failed.**

        The spec’s “Stall and loop detection” requirement says: “a pager that has failed stays failed (every later `next()` returns the same failure without a request).” The log instead records that the first call returns the original error and later calls return a separate `AlreadyFailed` error containing only the original error category. The original error can own response evidence that cannot be cloned. For example, the first failure can contain an HTTP status and response body; a later call reports the category without that evidence.

        The evidence is [implementation log, Phase 3](implementation-log.md#phase-3), line 1848. The affected public error type is schematic-runtime’s [PagerError](../../runtime/src/pager/error.rs). This is a declared design departure, not a code finding from this review.

        This meets **test 1, hard to reverse**: it changes the error variants and evidence that callers of every generated pager receive. The supplied terminal-failure rule does not specify this replacement error as a default.

        Options:

        - **Keep `AlreadyFailed` after the first failure.** This avoids forcing provider error bodies to be cloneable or introducing shared ownership throughout the error API. Callers must retain the first error if they need its evidence.
        - **Preserve the original failure and evidence on every later call.** This follows the spec literally, but requires an ownership change, such as sharing the stored error, and corresponding changes to consumers and tests.

        **Recommendation:** keep `AlreadyFailed`, explicitly narrowing the repeated-error guarantee to the original category and no further requests. Failure is terminal, and callers already receive the complete cause on its first occurrence. The extra ownership machinery buys little for this repository today.

        **If the answer is “go with your recommendation”:** record the accepted departure; the repair cycle aligns the public documentation and confirms tests cover the first error, the subsequent category-only error, and zero additional requests. Do not require cloning provider errors.

        **If undecided:** the final error contract remains unsettled, so downstream code may be written against an error variant or evidence guarantee that the author later rejects.
human_resolved: true
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: "codex/gpt-6.1-sol"
created: "2026-10-14T13:01:14-07:00"
spec: "2026-10-13-schematic-pagination/spec.md"
implemented: false
description: "A **feature** review of `2026-10-13-schematic-pagination/spec.md`"
feature: "2026-10-13-schematic-pagination/review-1.md"
next_spec: "schematic/features/2026-10-14-schematic-definition-api/spec.md"
next_plan: "schematic/features/2026-10-14-schematic-definition-api/plan.md"
---

# Review 1: Schematic pagination

Author decision update (2026-10-14): All three decisions below are resolved in **Human rulings**. The original decision descriptions and recommendations are retained as review history; the rulings govern the repair cycle. Implementation readiness remains unassessed and `ready: false` remains unchanged.

This review stops at the initial decision check. Three decisions need author rulings before the implementation is compared with the specification. **Production readiness was not assessed.** `ready: false` and `findings: []` mean “not reviewed,” rather than a failing code review or an implementation with no defects.

The two requested parallel readers examined the exact specification and implementation-log paths. The classification below uses those documents only. No implementation comparison, reproduction, or test execution was performed. Reported implementation behavior and past test results remain the implementer’s claims until the next review verifies them.

The current specification has no `message_to_agent` frontmatter property, although its refresh log mentions an implementer note. This metadata discrepancy does not require an author decision. Existing author rulings and supplied defaults are not being reopened.

## Decisions requiring author rulings

Items are ranked by the rework and contract risk of choosing incorrectly. They appear in full in `human_in_the_loop` above and below so the review loop and a human reader receive the same decision context.

**Item 1 — Separate server sessions opened by partitions need a cleanup policy.**

A partition divides a large listing into independent traversals. The author already decided that partitions may share one session whose handle changes. The specification says: “on a session, one session is opened for all and one close attempt is awaited when the scope ends” (requirement “Partitions”). That decision remains settled for a genuinely shared session.

The premise changes in the implementation log: when the first page opens the session, each partition can open separate server state, while only the last observed handle is closed. Four partitions can therefore leave three sessions to expire even after normal completion. This is reported in the log, not independently verified in this review. The affected lifecycle is in schematic-runtime’s [session implementation](../../runtime/src/pager/session.rs); the evidence is [implementation log, Phase 5](implementation-log.md#phase-5), lines 2150 and 2168–2169.

This meets **test 2, contradicts a decision**, because multiple separately opened sessions differ from the decided shared session. It also changes the cleanup premise: relying on expiry after ordinary success is broader than the already accepted cancellation and out-of-order-handle risks.

Options:

- **Reject partitioned first-page session opening until independent-session cleanup is designed.** This prevents knowingly abandoning separate sessions and preserves the shared-session design. It temporarily narrows the supported combinations, so it needs an explicit author ruling.
- **Track and close every independently opened session.** This supports sliced searches correctly, but adds handle ownership, multiple close attempts, and a way to report several cleanup failures.
- **Accept expiry as cleanup for the additional sessions.** This keeps the implementation small, but consumes server resources after successful calls and weakens the cleanup guarantee.

**Recommendation:** choose the first option. No shipped definition uses sessions or partitions, according to the log, so building multiple-session ownership now would add maintenance without a current consumer. Do not reject ordinary shared sessions with rotating handles; their author ruling still applies.

**If the answer is “go with your recommendation”:** record the narrower supported combination and have the repair cycle add focused validation, its test, and matching documentation. Leave shared-session handle rotation unchanged.

**If undecided:** the review cannot establish which session combinations the feature promises to support, and later definition builders could make the ambiguous combination easy to author.

**Item 2 — Decide which response callbacks paging promises to execute.**

schematic-runtime lets callers transform JSON responses with a global callback and change an endpoint’s decoded response with a callback registered for that endpoint. The log chooses to run the global callback before extracting page items and continuation values, while skipping the endpoint-specific typed callback. Ordinary sends can run both. This matters because changing an ordinary listing call to automatic paging can silently bypass a caller’s response transformation.

The evidence is [implementation log, Phase 4](implementation-log.md#phase-4), line 1978. The public callback behavior is described by schematic-runtime’s [hooks module](../../runtime/src/hooks.rs), and page decoding lives in its [pager request module](../../runtime/src/pager/request.rs). The spec supplies no planning default for this difference.

This meets **test 1, hard to reverse**: it fixes public callback behavior across the runtime and generated clients. Changing it later changes what existing callbacks see and can require another response-decoding path and revised generated-client tests.

Options:

- **Keep the logged behavior: global JSON callback runs; endpoint-specific typed callback does not.** It fits the page item result and avoids decoding the entire endpoint wrapper solely for a callback. Callers must know that endpoint-specific transformations are absent during paging.
- **Run both before extracting the page.** This better preserves ordinary-send transformations, but requires a defined conversion from the mutated endpoint response back to items and continuation data, with extra decoding or conversion work.

**Recommendation:** keep the logged behavior and make the omission an explicit public contract. Require a focused test of both callback paths through generated paging code. This avoids adding a second response conversion mechanism without a demonstrated consumer.

**If the answer is “go with your recommendation”:** record that paging uses the global callback only; the repair/review cycle checks the documentation and adds or confirms a generated-client test that demonstrates the difference. The author is deciding callback semantics, not endorsing the current code’s correctness.

**If undecided:** callers and later generated APIs cannot rely on a stable answer about which configured transformations paging executes.

**Item 3 — Decide what repeated calls return after a pager has failed.**

The spec’s “Stall and loop detection” requirement says: “a pager that has failed stays failed (every later `next()` returns the same failure without a request).” The log instead records that the first call returns the original error and later calls return a separate `AlreadyFailed` error containing only the original error category. The original error can own response evidence that cannot be cloned. For example, the first failure can contain an HTTP status and response body; a later call reports the category without that evidence.

The evidence is [implementation log, Phase 3](implementation-log.md#phase-3), line 1848. The affected public error type is schematic-runtime’s [PagerError](../../runtime/src/pager/error.rs). This is a declared design departure, not a code finding from this review.

This meets **test 1, hard to reverse**: it changes the error variants and evidence that callers of every generated pager receive. The supplied terminal-failure rule does not specify this replacement error as a default.

Options:

- **Keep `AlreadyFailed` after the first failure.** This avoids forcing provider error bodies to be cloneable or introducing shared ownership throughout the error API. Callers must retain the first error if they need its evidence.
- **Preserve the original failure and evidence on every later call.** This follows the spec literally, but requires an ownership change, such as sharing the stored error, and corresponding changes to consumers and tests.

**Recommendation:** keep `AlreadyFailed`, explicitly narrowing the repeated-error guarantee to the original category and no further requests. Failure is terminal, and callers already receive the complete cause on its first occurrence. The extra ownership machinery buys little for this repository today.

**If the answer is “go with your recommendation”:** record the accepted departure; the repair cycle aligns the public documentation and confirms tests cover the first error, the subsequent category-only error, and zero additional requests. Do not require cloning provider errors.

**If undecided:** the final error contract remains unsettled, so downstream code may be written against an error variant or evidence guarantee that the author later rejects.

## Implementation decisions (informational)

These decisions do not halt the review. Values below come from the implementation log; they have not been checked against the code. A “for review” or “awaiting ratification” label in that log is not, by itself, a reason to ask the author to approve a supplied default again.

### Supplied planning defaults

| Spec default | Logged outcome and classification |
| --- | --- |
| Collection failure reasons | Keep the five decided reasons and add invalid continuation, invalid argument, and transport failure; retain the original cause. Supplied default. |
| Page limit | 100 attempted pages, configurable and readable. Supplied default. |
| Exception table and diagnostics | Ordered entries; reject duplicate/unknown ids; nearest-id hint; reuse the existing structured diagnostic format. Supplied default, including permission to align that format. |
| Decode-path experiment | Retain `from_value`: the trial `from_slice` path added 55 KB, an additional body traversal, and a dependency feature at 94 endpoints. The spec expressly allows this outcome. |
| Strategy representation | Named structs, owning definition values, and static runtime projections. Supplied default; no separate ratification needed. |
| Stall detection | Remember exact continuations across an exclusive forward traversal; inclusive repeats require no new item identities before failure; histories are separate per partition/direction. Supplied default. The separate repeated-error decision is Item 3. |
| Alternate strategies | Model and validate now; emit no alternate-selection methods. Supplied default. |
| Six recalled provider patterns | Keep `unverified_` tests and do not promote them to provider evidence. Optional documentation verification was not authorized, so it was skipped. The spec supplies that fallback. |
| Page-size validation | Chainable setters; refuse invalid bounds before sending with the existing transport argument error. Supplied default. |
| Caller entry points | Thin forwards on generated endpoint calls; generic request-object paging remains available; endpoint setters precede pager controls and `.full()`. Supplied default and earlier author ruling. |
| Session closure | Mutable borrowed scope with a boxed future; no public manual open/close. The attempted unboxed alternative could not establish a `Send` future and was discarded. The supplied default delegates the exact signature. |
| Parameter adoption | Remove a plain request field only when the strategy supplies its matching control; use configured one-page fetching for migrated callers. Supplied default and earlier author ruling. |
| Wrapper item names | Rust type-path strings resolved by the existing registry/type rules until typed registration arrives. Supplied default. |
| Inferred root-array lists | GET plus explicit `Vec<T>` only; non-lists opt out; aliases and opaque responses require explicit entries. Supplied defaults. |
| Legacy pagination metadata | Retain without deprecation warnings, document as superseded, remove in the already assigned definition-API work. Supplied default. |
| Link trust | Remove pagination-specific trust; use the existing origin policy; same-origin next links work under `deny`. Supplied defaults and existing runtime behavior. |
| Partition interface | Return independent iterators; spawn nothing; caller drives them. Supplied default. The separate-session cleanup premise is Item 1. |
| Test time and documentation | Real time with at least 100 ms page-delay margins; topic page at `schematic/docs/pagination.md`. Supplied defaults. |
| Close grace | Five seconds, configurable, capped by the per-request timeout, separate from the page deadline, including early returns and expired deadlines. Supplied default; no overriding prior ruling was found. |

### Other choices delegated to implementation

- **Integration and organization:** bump runtime/generator compatibility from 3 to 4 together; reuse existing validator and error names; use internal seams for the final response URL, page metadata, and typed sends. Split model helpers into submodules and keep JSON selection in the runtime. These implement the existing compatibility rule or are internal structure, rather than new policy choices.
- **Model and diagnostics:** use a distinct “unauthored” resolution origin; share adoption calculation between validation and generation; keep a stable control order; group diagnostics under three feature names; use `max(2, id_length/3)` for nearest-id hints; keep the first duplicate entry for resolution while validation rejects the duplicate. These are local representation/diagnostic choices. The proposed generic pager and capability encoding implement the spec’s suggested surface; most controls use emitted marker implementations, with blanket implementations for kind/session controls.
- **Validation and serialization:** add a sendable required-parameter default separate from the provider’s documentation-only default; strengthen destination collisions for case-insensitive headers and body-path prefixes; reject empty selectors and unknown serialized fields. Missing lists/booleans fail, while optional values deliberately equate absence and null. No configuration loader was added, so the implementer classified the new serde support as model serialization rather than a file-reader robustness matrix. That applicability decision remains for the subsequent technical review.
- **Iteration semantics:** page metadata counts the one-based traversal position; valid page items are delivered before a continuation error is returned on the following iteration; direction changes reset the opposite history so deliberate backtracking is possible. Starting/default values, provider-issued next-link sizes, and end-signal precedence were recorded. These are local choices or applications of supplied rules; their correctness is not assessed here.
- **Carry and deadline mapping:** definition validation governs destination conflicts; managed values replace ordinary values at those destinations, including generated required defaults. Selective carry filters named form/multipart fields and refuses bodies without selectable fields. A deadline interrupting a received response body remains an interrupted-response failure mapped to HTTP failure, following the spec’s explicit interrupted-envelope mapping; this is not a request to reaffirm that mapping. Whether the implementation fulfills all deadline requirements is for technical review.
- **Session details:** open lazily on the first traversal request, report an open failure inside the closure, reuse the existing pager error for close failure, and make renewal before a known handle return false without sending. The scope can create traversals and item streams; close grace is forwarded onto generated session calls. Lazy opening and the exact closure/error representation are treated as delegated session API design; they are not independently grounds for stopping. Handling several independently opened sessions is the distinct changed premise in Item 1.
- **Catalog authoring:** author 94 paginated endpoints: GitHub 9, GitLab 9, Gitea 8, Bitbucket 8, OpenAI 58, ElevenLabs 2. The log recounts 18 REST APIs, 547 endpoint types, 511 production endpoint literals, and 156 collections. Bitbucket has eight wrapper lists because its source endpoint is now binary. Leave ElevenLabs voices unpaginated because the definition and research disagree on the token parameter name, as the spec’s contradiction rule requires.
- **Bounds and migration:** author OpenAI maxima only when valid for every request option; retain a positive model minimum even where provider documentation allows zero; omit documentation-only overrides; retain Gitea’s specified cap of 50; clamp sniff’s workflow-list request to 1–100. These bounds can be relaxed without a new API or format. They do not require author intervention here; the subsequent review still checks them against the contract. Remove 185 adopted fields and migrate the affected sniff/schema calls. Those removals and dependent edits are required by the existing author ruling.
- **Importer:** keep the hand-written OpenAI overlay outside the importer-owned directory; configure the generic hook with `--overlay`, requiring definition output and a Rust path. Re-import through the public factory, preserving exporters. This follows the decided overlay design; option spelling and file location are routine implementation details.
- **Testing and process:** add generated-crate trybuild support, move regeneration/migrations earlier to keep generation checks passing, run read-only preflight tasks concurrently, and collect background workers before dependent work. Reduce ten native-Windows path-bearing compile-fail snapshots because of a reported trybuild normalization defect; retain three cases and a positive control there, with all cases elsewhere. Raise the runtime trybuild timeout for cold builds. These are reversible test/process choices, not an author gate. The next review must still assess whether they leave required coverage missing.
- **Maintenance:** update stale comments, READMEs, docs, and skills alongside behavior; leave unrelated comment cleanup alone. No commit or lifecycle move is requested by this review.

### Deferred work and verification limits

- **Provider verification:** 32 endpoint entries remain on the verification list: Anthropic models; ElevenLabs voices; GitLab issue participants; two Gitea tree endpoints; eight endpoints in each EMQX variant; eleven Hugging Face endpoints. Network verification and the conditional EMQX authentication-user response correction were not authorized. The spec explicitly permits leaving these unpaginated and recording the source needed, so neither is a new blocking question.
- **Provider gaps and non-lists:** keep the two OpenAI vector-store responses without documented continuation inputs unpaginated; retain the existing model-list opt-outs and binary/non-list classification. Bitbucket page fields without an authored page-number control, OpenAI order fields, and ElevenLabs voice parameters stay available.
- **Later pipeline work:** endpoint builders, typed item registration, moving exception entries onto endpoints, and legacy-type removal already belong to the next definition-API work. Generated alternate selection, non-JSON pages, and unrelated response/query vocabulary remain outside this implementation. Manual retry of a failed page was originally excluded, but the Item 3 author ruling now requires this implementation to always offer it as a caller option. The caller is never required to retry. Optional automatic retry policies are desirable follow-up work, not a requirement for this landing. No new scope is assigned to a later spec by this review.
- **Unscheduled model limitations:** the log proposes an optional explicit end signal for next-link strategies when Salesforce needs one, possible future alternate-parameter collision enforcement, and special handling when a scroll handle repeats. These are proposals rather than an accepted expansion of the later vocabulary spec. Repeated scroll handles have no shipped definition; the current stall default is not being reopened solely for a hypothetical provider. Required partition defaults and header carry filtering also await a real use case.
- **Evidence not repeated:** the dedicated overlay-output compile was a one-off, with structural/re-import tests and compilation of actual imported definitions retained. No test covers cancellation during session opening/closing or extracting a session handle from a header, according to the log. No shipped endpoint exercises real session/partition providers. These testing claims need assessment in the next technical review; they are not proof of readiness here.
- **Performance and platforms:** the log reports a roughly 2.6 MB synthetic pager cost and noisy library-only cold-build growth from 35.5 to 49.9 seconds; further optimization/measurement was not required. It reports macOS, Linux, native Windows, and WSL2 runs, but no reusable receipts for the then-uncommitted tree. Cross-OS receipts and CI scheduling are not gates for this review. No HTTP/2, live-provider, rust-analyzer-latency, or incremental-build evidence is established by this decision check.
- **Polish:** correcting stale Bitbucket legacy defaults can accompany the already assigned legacy-type removal. Filing an upstream trybuild issue is optional work and has not been done. Neither requires this pipeline to stop.
- **Human-only evidence:** neither reader found a requirement that only a human-operated account, device, credential, or host can verify. Deferred provider documentation checks are authorization-limited work with an explicit safe fallback, not a human-only acceptance requirement.

## Findings

Not assessed. This review stopped before comparing implementation and specification. The empty finding list must not be used as evidence that implementation requirements or verification fidelity have passed.

## Observations

None from an implementation review; no such review was performed.

The next spec and plan were not refreshed because this review does not set `ready: true`. No `refresh` property is set. No code was changed and no tests were run. The author decisions originally carried by `human_in_the_loop` are now resolved in **Human rulings**; repair and technical review remain necessary. `human_review: false` means no separate later human-only verification task has been identified.

## Human rulings

### Item 1

Ruling: Approved option 1 on 2026-10-14. Reject definitions combining partitions with sessions opened by the first page until independent-session tracking and cleanup are designed. Keep first-page sessions without partitions and partitions sharing a session opened by a separate request supported, including the existing latest-observed-handle rule for rotating shared-session handles. The repair cycle must add focused validation, its test, and matching documentation. This ruling does not introduce caller-managed sessions.

### Item 2

Ruling: Approved option 1 on 2026-10-14. Automatic paging runs the client-wide JSON response callback before extracting items and continuation values, but does not run the endpoint-specific typed response callback. Make this difference from ordinary sends an explicit public contract. The repair/review cycle must confirm documentation and focused generated-client tests demonstrating both callback paths.

### Item 3

Ruling: Approved the revised recommendation on 2026-10-14; reject the original recommendation to keep `AlreadyFailed`.

- Remove `AlreadyFailed` from the public error design. Errors report the actual condition, such as pagination loop detection, rather than the pager's failure history.
- Manual retry must always be available as an option after a failure, regardless of error type. The caller receives the complete error and decides whether and when to explicitly retry; the caller is never required to retry. Availability is not conditional on the pager classifying the failure as temporary or safe to retry. Preserve the failed page's position without redelivering earlier pages.
- Retain the specific error and its supporting details. Later calls that do not explicitly retry return that error without sending requests; this does not remove the caller's option to explicitly retry, including after loop detection.
- Each failed retry returns the new attempt's specific error with its supporting details.
- Optional automatic retry policies are desirable follow-up work, not a requirement for this landing, and must not replace caller-controlled retry.

This ruling supersedes the specification's exclusion of retry-from-failed-page support and its unconditional failed-stays-failed rule. The repair cycle must implement the revised error and retry behavior, update the public documentation and implementation log, and verify that manual retry is always optional and available regardless of error type, that error evidence is retained, and that earlier pages are not redelivered. This ruling resolves the design decision; it does not approve the current implementation.

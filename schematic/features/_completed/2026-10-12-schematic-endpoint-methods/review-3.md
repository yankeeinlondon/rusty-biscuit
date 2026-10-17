---
$schema: feature-review.yaml
ready: true
findings: []
observations:
  - "Generated-schema behavior tests run locally but not in CI"
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-13T13:06:46-07:00"
spec: "2026-10-12-schematic-endpoint-methods/spec.md"
implemented: false
description: "A **feature** review of `2026-10-12-schematic-endpoint-methods/spec.md`"
feature: "2026-10-12-schematic-endpoint-methods/review-3.md"
previous: "2026-10-12-schematic-endpoint-methods/review-2.md"
refresh: refreshed
refresh_changes:
  - "Updated the pagination spec and plan to extend existing endpoint calls with thin pager forwards, applying the author's recorded primary-entry ruling."
  - "Added the completed endpoint-method prerequisite and verified runtime ABI, call signatures, error shapes, validation, and response-evidence handoffs."
  - "Recorded endpoint setters before full(), private call-state integration, collision reservations, and deadline request-count semantics."
  - "Corrected the REST surface count and the obsolete claim that generated clients cannot expose response headers."
  - "Added dated refresh logs to both pagination documents and set its refresh flag and implementer note."
---

# Review 3

**The feature is production ready.** Review 2's high finding is repaired, and the implementation satisfies the specification under the author's accepted interface and build-cost decisions. No high, medium, or low findings remain. No additional human decision is required.

## Previous findings and author rulings

The first check was review 2's frontmatter and body finding: call `.build()` omitted deterministic argument checks performed before sending. In schematic-runtime, [Call::build](../../runtime/src/call.rs) now delegates to the client's argument check, which uses the same first-hop preparation as typed and raw sends in [client.rs](../../runtime/src/client.rs). `FullCall::build` delegates to it; generated endpoint calls delegate to the runtime call. This checks the effective credential and its placement as well as request encoding and header assembly, without network work.

Review 2 has no human-stop property or rulings requiring further changes. Review 1 still has “pending” placeholders under `## Human rulings`, but its frontmatter records `human_resolved: true` and explicit author decisions for all three items. As review 2 did, this review treats those decisions as authority:

| Author decision | Current implementation and record | Result |
| --- | --- | --- |
| Review 1, item 1: retain named API-error fields with boxed response metadata | schematic-runtime's [CallError](../../runtime/src/call.rs) keeps `Api { error, meta: Box<ResponseMeta> }`, with request access and the inner error as its source. The implementation log records the accepted departure; consumer matches and documentation use this shape. | Implemented. This author ruling overrides the spec's decided tuple variant. |
| Review 1, item 2: retain generic full-response calls and document endpoint setters before `.full()` | Runtime rustdoc, generated call docs, the calling guide, READMEs, and generator skill describe the ordering. Generator assertions and compile-fail checks pin it. The pagination refresh carries it forward. | Implemented. |
| Review 1, item 3: retain homelab's fallback error and review conversions and documentation | homelab's [RestError](../../../homelab/lib/src/rest.rs) retains its transparent `Call` variant and existing transport/status/response mappings; its cause-preservation test and device documentation match. Consumer error folds remain consistent. | Implemented; review 2's unchanged consumer verification reused. |

The spec frontmatter also records the author's acceptance of chained setters without per-API features. Historical statements that this approval is pending do not reopen that decision.

Review 2's calling-guide observation is corrected: the guide now identifies path values followed by the body as constructor arguments and explains that query/header/cookie values remain setters. Its separate schema-suite scheduling observation remains informational below.

## Findings

None.

## Validation-class sweep

The repaired defect class was **early argument validation omitting stages used by send-time request preparation**. The sweep checked every call-build entry and each preparation stage within this feature's boundary. The runtime's [call_build tests](../../runtime/tests/l1/call_build.rs) exercise public call results against a recording loopback server. Each negative runtime case checks ordinary build, full-response build, and awaiting generic send; the generated GitHub test exercises endpoint and generic entry points. The following cases passed in this review:

| Site or stage | Shape tested | Observed and expected result |
| --- | --- | --- |
| Request encoding | Request's `into_parts` returns an argument error | Both build modes and await reject with the same argument error; zero requests. |
| Request headers | Invalid header name; header value ending in newline | Both build modes and await reject identically; zero requests. |
| Body content type | Raw-body content type ending in newline | Both build modes and await reject identically; zero requests. |
| Credential kind | Basic credential on a bearer-only API | Both build modes and await reject identically; zero requests. Generated GitHub endpoint/full calls also reject. |
| Credential header value | Access token ending in newline | Both build modes and await reject identically; zero requests. Generated GitHub generic send also rejects. |
| Header credential placement | Existing Authorization plus per-call bearer; separately, client bearer | Both build modes reject and await refuses before sending. The per-call case checks identical argument errors. |
| Query credential placement | Existing `key` query value plus credential in that slot | Both build modes and await reject identically; zero requests. |
| Cookie credential placement | Existing `key` cookie plus credential in that slot | Both build modes and await reject identically; zero requests. |
| Anonymous override | Conflicting Authorization, then `.anonymous()` | Both build modes accept; await sends once without Authorization. |
| Valid control | Ordinary request with an accepted bearer credential | Build sends nothing; await sends exactly once and decodes the reply. |
| Generated forwards and generic full call | Generated `build` forwards; `FullCall::build` delegation | All 547 generated forwards use the runtime check; shipped output compiles and matches fresh generation. |
| Other pre-send failures | Client settings, URL/origin refusal, expired deadline | These retain their separate error kinds and send-time behavior. The call-build contract and guide accurately identify its argument-error scope. |

Client and call validation remain separate: callers can validate client settings with the client's own `.build()`. Preparation is repeated at await, so a successful early check does not cache environment credentials or bypass later checks. Sharing preparation avoids maintaining a second validation implementation.

## Requirement and boundary evidence

This feature changes generated Rust calls and runtime adapters, rather than provider protocol definitions. A real Rust compiler consuming fresh output and a compiled client exercising a recording local HTTP server reach **F2** and are the appropriate boundary evidence. The server stimulus is synthetic; these tests do not establish live provider behavior or F3 evidence. No new configuration/file-format reader was added or changed, so the Input Robustness Matrix does not apply.

| Requirement | Boundary and strongest evidence | Assessment |
| --- | --- | --- |
| One inherent method per endpoint; constructor order; no immediate send | Rust compiler and HTTP dispatch, F2: fresh fixture compile/run, no-trait-import control, and shipped-output compile. Inventory: 547 request types and 547 call types. | Met. |
| Local endpoint call types, setters, shared settings, collisions | Rust compiler and HTTP dispatch, F2: fresh-output endpoint and renamed-setter behavior tests. F1 resolver tests cover reserved names, keywords, duplicate names, shared-module suffixes, and shipped rename inventory. | Met. |
| Await returns data or one combined error retaining evidence | Runtime/generated client against local HTTP replies, F2: success, declared failure, malformed success, interrupted body, and connection failure tests; error-source assertions. | Met under the accepted named-field shape. |
| Full reply retains status, headers, and request id | Compiled clients and local HTTP replies, F2: runtime full-response tests and generated GitHub metadata tests. Compiler rejection checks setter ordering. | Met under the accepted ordering. |
| Early argument checking, cloneable calls, repeated sends | Public build/await results and recording server, F2: validation sweep above, runtime repeated-call test, and generated cloned-call test. | Met. |
| Public request objects and equivalent generic sends | Rust compiler and HTTP dispatch, F2: generated endpoint/generic parity test, runtime call tests, and wrong-API compile rejection. | Met. |
| Endpoint rustdoc: description, route, arguments, available setters | Generated documentation content, F1: generator assertions cover required sections, empty sections, parameter descriptions, and full-response ordering. This requirement concerns text, not editor rendering. | Met. |
| Before/after build cost and output size, including OpenAI separately | Actual compiler measurements recorded in implementation-log.md, Tasks 1.4 and 4.5: clean/incremental builds, OpenAI probe, source and rlib sizes. Whole-command clean time increased 4.5%; crate-only time increased 34%. | Met under the author's approved build-cost branch; measurements reused. |
| In-repository consumer migration | Compiled consumers and local HTTP fixtures, F2: recorded biscuit-speaks/sniff/homelab migration evidence; current source audit confirms endpoint calls and distinguishes unrelated reqwest/WebSocket sends. | Met; unchanged consumer results reused. |
| Calling guide covers clients, arguments, overrides, full replies, early checks, and request objects | Written examples checked against actual signatures; behavior supported by the F2 tests above. GitHub examples use shipped read-only endpoints in place of the spec's illustrative unavailable issue creation. | Met. |

Fresh-crate compilation alone is F2, even where historical implementation-log entries call it F3. No new behavior requires terminal/browser focus or human-only hardware/provider access.

## Verification

Run against the current implementation on this host:

- `schematic/just test`: 1,937 ordinary tests passed, 12 existing tier tests skipped; six fresh-output checks and the compile-fail harness also passed.
- `schematic/just test-schema`: 40 tests passed, none skipped, including the new generated GitHub credential-validation case.
- `schematic/just lint` and `schematic/just lint-schema`: passed.
- `just check-tier-coverage schematic`: passed, zero stranded tests. The new runtime test module is declared by `tests/l1/main.rs` in its explicit Cargo test target and selected by L1.

Consumer suites were not rerun; their unchanged migration and error-conversion evidence from the implementation log and review 2 was reused. Cross-OS evidence is left to CI as requested and does not block this review. No application windows were opened. No production code was changed, formatted, or committed.

## Observations

### Generated-schema behavior tests run locally but not in CI

The generated schema crate remains excluded from the workspace, so its own 40 tests run through `just test-schema` locally. This predates the feature. CI already compiles shipped output and exercises freshly generated fixture clients, and the repaired validation cases live in the runtime's declared L1 binary. Scheduling the separate schema suite remains an author choice outside this repair cycle; it is not a readiness finding.

## Pagination handoff

The pagination spec and plan were refreshed after reading this feature's implementation and repair logs, both previous reviews and author decisions, the parent pagination decisions, and the next spec's Refresh notes. The refresh applies the already recorded endpoint-first ruling: pager methods extend existing local calls through thin forwards into the shared runtime. It also records the accepted full-response ordering, ABI 3, effective call settings, shared argument preparation, envelope preservation, private call-state integration, collision reservations, and deadline request-count semantics.

Both documents have dated refresh logs. The pagination spec has `refresh_required: false` and an implementer note. Its decided single-page/session/provider requirements and historical ruling text are preserved; generic examples are explained as the secondary request-object form. No contested requirement or new scope was introduced, so no refresh human items are needed.

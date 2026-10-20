---
$schema: feature-review.yaml
ready: false
findings:
  - "high: carry out ruling on item 1: keep CallError::Api { error, meta: Box<ResponseMeta> } and record it as an accepted departure"
  - "high: carry out ruling on item 2: keep the generic FullCall and document .full() as the final endpoint-specific step"
  - "high: carry out ruling on item 3: keep homelab RestError::Call and review its conversions and documentation"
observations: []
human_in_the_loop:
  - |-
    **Item 1 — Keep response metadata in a different combined-error shape?**

    The combined error returned by an awaited API call lives in schematic-runtime. The spec explicitly decides: “Shape: a `#[non_exhaustive]` enum with `Transport(TransportError)` and `Api(ApiError<E>)`.” The implementation log instead records `Api { error: ApiError<E>, meta: Box<ResponseMeta> }`. It explains that the existing `ApiError` has no metadata, although the spec also requires `request_meta()` to work for both variants. This meets **test 2 (contradicts a decided shape)** and **test 1 (public error format used across crates)**. The plan supplied this alternative; that does not override the spec's decided shape.

    - **Accept the implemented named fields.** Pros: retains response metadata without changing the existing `ApiError` used by response envelopes; boxed metadata keeps error values small. Cons: callers match `CallError::Api { error, meta }`, rather than the tuple pattern the spec promised.
    - **Preserve the decided tuple shape and revise `ApiError` to carry metadata.** Pros: preserves the decided match syntax. Cons: broadens a pre-existing public error type and requires reconciling envelope construction and other users of that type.

    **DECISION:** accept the named fields with boxed metadata. It preserves the required information while keeping the change concentrated in the new combined error. If you answer “go with your recommendation,” the next cycle should retain this shape, record the accepted departure, and review its implementation and consumers. Until ruled, review cannot determine which public error contract to enforce; reversing it later would require changing matches in generated-client consumers.
  - |-
    **Item 2 — Should `.full()` end access to endpoint-specific methods?**

    In schematic-schema, each endpoint has its own generated call type, which carries optional setters and is the planned home of pagination methods. The log records that `.full()` converts it to schematic-runtime's generic `FullCall`, which returns the complete response envelope. For example, `gh.list_releases(owner, repo).per_page(10).full().await` is supported, but `.full().per_page(10)` is not. The spec requires the envelope result but does not decide this return type or ordering restriction. This meets **test 1 (public return type across the runtime and generated crates)**. Pagination is not assessed here; the risk is that it builds on whichever call surface is accepted now.

    - **Keep generic `FullCall` and put `.full()` after endpoint setters.** Pros: shares one implementation and avoids generating another type per endpoint. Cons: conversion loses endpoint setters and future endpoint-specific methods.
    - **Keep an endpoint-specific type after `.full()`**, using a generated full-call wrapper or a mode parameter. Pros: endpoint setters can remain available and later endpoint methods can be designed for both response modes. Cons: adds generated API surface or more complex signatures and needs another compile-cost check.

    **DECISION:** keep generic `FullCall` and document `.full()` as the final endpoint-specific step. The approved examples already use that order, and the extra types buy convenience rather than required behavior. If you answer “go with your recommendation,” the next cycle should retain this restriction and check that the pagination documents use it consistently when the feature is ready for refresh. Until ruled, the public return-type boundary remains unsettled; changing it after pagination is implemented could require reworking both generators and callers.
  - |-
    **Item 3 — Keep homelab's new catch-all error variant?**

    homelab translates generated-client failures into its public `RestError`, used by its receiver and media-device REST clients. The log records a new `RestError::Call(CallError<NoDeclaredError>)` to handle future variants of schematic-runtime's non-exhaustive combined error. Existing transport, HTTP-status, and decoding failures retain their previous mappings; the new fallback has no producer today. The spec requires migrating consumers but does not decide this new consumer error variant. This meets **test 1 (a public enum that dependent crates can match)**.

    - **Keep `RestError::Call`.** Pros: preserves an otherwise unfamiliar future failure without forcing it into a misleading existing category; uses the primary `.await` call style. Cons: adds a public variant that callers must accommodate even before it is used.
    - **Use `.full().await` inside these consumer adapters and preserve the existing error enum.** Pros: retains the previous envelope-based mapping and adds no public fallback. Cons: these adapters use the more verbose envelope path, and the exception to the primary migration style must be recorded.

    **DECISION:** keep the new variant. It gives future combined-error variants an honest, information-preserving fallback, and the log reports no exhaustive in-repository matches to migrate. If you answer “go with your recommendation,” the next cycle should retain the variant and review the conversions and documentation. Until ruled, accepting this feature also accepts an unapproved public interface change in homelab; removing it after dependent code adopts it would require another migration.
human_resolved: true
human_review: true
human_review_items:
  - "Decide whether to accept the combined error's named fields instead of the spec's decided tuple shape; see Item 1."
  - "Decide whether converting to a full response ends access to endpoint-specific setters and methods; see Item 2."
  - "Decide whether to retain homelab's new public fallback error variant; see Item 3."
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-13T11:48:09-07:00"
spec: "2026-10-12-schematic-endpoint-methods/spec.md"
implemented: true
next: "2026-10-12-schematic-endpoint-methods/review-2.md"
implemented_by: claude/opus
log: schematic/features/2026-10-12-schematic-endpoint-methods/log.md
description: "A **feature** review of `2026-10-12-schematic-endpoint-methods/spec.md`"
feature: "2026-10-12-schematic-endpoint-methods/review-1.md"
next_spec: "schematic/features/2026-10-13-schematic-pagination/spec.md"
next_plan: "schematic/features/2026-10-13-schematic-pagination/plan.md"
---

# Review 1: author decisions required

The review stops at the decision-gathering stage. **Production readiness was not assessed.** The implementation has not been compared with the specification, and no tests were run for this review. `ready: false` and an empty finding list mean “not reviewed,” rather than a failed implementation or a clean bill of health. There are no blocked defect findings because defect review has not started.

Two agents independently read the specification and implementation log. The plan was consulted to distinguish implementation defaults from author decisions. The three items below are ranked by the likely rework if their public interface changes later. Their full options, trade-offs, and recommendations are also recorded in `human_in_the_loop` above.

## Decisions requiring an author ruling

### Item 1: combined-error representation

schematic-runtime's [CallError](../../runtime/src/call.rs) represents a failed awaited call. The spec's decided tuple `Api(ApiError<E>)` became named fields holding the original error and boxed response metadata. The log identifies missing metadata in the existing error as the reason. Accepting this departure is recommended, but it contradicts an explicit decision and determines how other crates match failures. Choose between accepting the named fields and retaining the tuple while revising the existing error type; see Item 1 in the frontmatter for the consequences.

### Item 2: the full-response return type

schematic-runtime's [FullCall](../../runtime/src/call.rs) returns response data together with status, headers, and request metadata. The generated endpoint call converts to this generic type, so endpoint setters must come before `.full()`. Keeping this ordering is recommended because it avoids additional generated types and matches the approved examples. The alternative keeps an endpoint-specific full-response type at the cost of more API surface. This public return type should be settled before pagination builds on the endpoint calls.

### Item 3: homelab's fallback error

homelab's [RestError](../../../homelab/lib/src/rest.rs) gained a `Call` variant for combined errors added in the future. Keeping it is recommended because it preserves those errors without misclassifying them. The alternative retains the old enum by converting from the full response envelope instead. This is a separate public interface change in a consumer crate, even though today's failures do not reach the new variant.

## Implementation decisions (informational)

The following do not require another author ruling:

- **Build-cost choice is already approved.** The spec frontmatter records “DECISION: A: keep changes as applied.” Chained setters remain, without per-API feature flags. The log and `message_to_agent` still call this unconfirmed; those statements are stale. The reported whole-build increase is 4.5%, versus 34% for the generated crate alone. No new premise is recorded that would justify reopening the approval.
- **Generic calls use `send(request)`.** Options chain on the returned call rather than being a second argument; raw sending retains its existing options argument. This follows the required shared call behavior.
- **Call names follow request names.** Replacing the trailing `Request` with `Call` retains API suffixes in shared modules and avoids 38 naming collisions. Requests without a constructor use `Default`. These are consistent mechanical choices, rather than a new semantic contract.
- **Internal implementation stays small.** Futures are boxed; `.build()` encodes a cloned request and discards the encoded parts, with encoding repeated when awaited; clone bounds apply only where needed. Generated setters reuse the existing request setters. These choices can be changed without changing the required user behavior.
- **The runtime exposes `Call::map_request` for generated setters.** The generated crate needs a callable request-update bridge into the runtime. Its documented visibility is an implementation choice; the log identifies no additional dependent use or changed later requirement requiring a separate ruling.
- **Naming checks reserve existing trait methods and future pagination names.** Duplicate endpoint method names fail generation instead of receiving numeric suffixes; warnings report permitted renames and errors report unresolved conflicts. Reserved-name constants have drift checks. Shipped definitions have an empty rename list.
- **Compile cost was reduced with `#[inline]` on generated future conversion.** The log reports that this avoids compiling unused send state machines inside the generated crate. This preserves call behavior.
- **Shared helpers stayed where they were.** The call work did not modify generated `shared.rs`, so the spec's candidate migration was not taken.
- **Validation and examples use equivalent practical substitutes.** The no-trait-import check uses a fresh-crate `cargo check` target rather than trybuild; documentation uses actual GitHub endpoints instead of the illustrative unavailable `create_issue`. Schema regeneration and test migration happened earlier than planned to keep drift checks passing. Existing splitting thresholds moved generated modules into directories while preserving public imports.
- **Error display separates context and cause.** Display describes the call and redacted response; `source()` exposes the inner error. Transparent display remains a cheap alternative, not an unresolved safety default.

## Deferred work and verification limits

Per-API feature flags and the options-struct fallback were not taken under the approved build-cost branch. Release-profile builds, editor latency, and dependent-consumer build measurements were outside the adopted measurement scope. Shared-helper migration, unused error accessors, dead generator-file cleanup, awkward OpenAI operation-name cleanup, and reducing oversized skill documents were left out because they were unnecessary or outside scope. Pagination, value constraints, and WebSocket work remain assigned to later features.

The log reports local tests of the generated crate, but CI only compiles that crate and does not run its 39 tests. It also reports manually checked documentation examples, unchanged tier or live-provider skips, and no applicable consumer L2 suite. These statements are recorded for the subsequent implementation review; their adequacy has not been assessed here. Cross-platform evidence is left to CI and is not a human decision gate. No required real account, credential, device, or other human-only verification was identified.

The log calls a fresh-crate compile test “F3.” That label has not been accepted as evidence by this review: compiling generated output is F2 unless an independent vendor schema or sample supplies the additional boundary required for F3. Verification fidelity must be evaluated when implementation review resumes.

No following-spec refresh was performed because readiness was not assessed. The next review should first read the author rulings below, check that the implementation reflects them, then perform the full specification and test review. The original spec remains a snapshot; the accepted departures belong in the implementation record and current documentation.

## Human rulings

### Item 1

Ruling: (pending)

### Item 2

Ruling: (pending)

### Item 3

Ruling: (pending)

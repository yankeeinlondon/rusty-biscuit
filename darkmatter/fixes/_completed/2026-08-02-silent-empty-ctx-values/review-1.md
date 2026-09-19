---
$schema: feature-review.yaml
ready: false
findings:
  - title: Persistent cache hits discard child partial-capture diagnostics
    priority: high
  - title: Body failures omit the available authored location
    priority: high
human_review: false
reviewed_by: codex/default
created: "2026-09-16T22:26:04-07:00"
spec: 2026-08-02-silent-empty-ctx-values/spec.md
implemented: true
implemented_by: claude/opus
next: 2026-08-02-silent-empty-ctx-values/review-2.md
log: darkmatter/fixes/2026-08-02-silent-empty-ctx-values/implementation-log.md
description: "A **fix** review of `2026-08-02-silent-empty-ctx-values/spec.md`"
fix: 2026-08-02-silent-empty-ctx-values/review-1.md
---

# Review 1: Silent Empty `ctx` Values

## Verdict

The fix is **not ready for production**. The checked lookup, fatal error
classification, request-epoch extension, frozen caller authority, and cache
context-closure design address the original silent-empty defect. However, two
explicit diagnostic contracts are incomplete: a warm persistent cache can
silence a child-only partial-capture warning, and a body interpolation failure
does not retain the authored line or byte span even though the scanner has it.

Human review is not required. Both findings are deterministic implementation
and Level 1 test gaps with direct fixes that do not require a new product or
design decision.

## Findings

### High — Persistent cache hits discard child partial-capture diagnostics

Requirement 3 says unavailable evidence remains owned by
`PartialRuntimeCapture`, and verification item 4 requires the diagnostic to be
retained. A cold child compose satisfies that contract because the child's
`EffectiveState` converts its context diagnostics into warnings and stores them
in `ComposeResult.report`. A persistent hit does not: both the fresh and stale
read paths construct `ComposeResult` with `ComposeReport::new()`
(`lib/src/markdown/compose/cache/runtime.rs:697-716`). The transclusion engine
then merges that empty report (`lib/src/markdown/compose/transclusion/engine.rs:1577-1583`).

This is observable when a root that needs no discovery-backed group transcludes
a child whose first `ctx.*` reference is captured from unavailable supplied
evidence. The cold run emits the child's `Partial runtime capture ...` warning;
the warm persistent-cache run returns the same typed empty/null output without
that warning. The cache therefore changes the diagnostic contract, even though
the context-closure hash correctly prevents stale output and missing-capture
bypass.

The claimed coverage does not test this boundary. `a_captured_group_without_evidence_renders_its_typed_projection`
checks that the context itself has diagnostics, but its report assertion only
rejects text containing `did not capture`; it never asserts that the
`PartialRuntimeCapture` warning reached the report
(`lib/tests/missing_ctx_capture.rs:273-303`). The persistent tests assert output,
hits, invalidation, and frozen-authority failure, but do not inspect warnings
(`lib/tests/request_context_epoch.rs:300-383`). The verification matrix's claim
that item 4 asserts the partial-capture diagnostic is therefore stronger than
the test.

Persist the diagnostic/report data needed to reconstruct child warnings, or
regenerate the captured-context warnings on a persistent hit without
duplicating them. Add a Level 1 cold/warm persistent-cache regression for a
child-only partial capture and assert the same warning count and content on
both runs.

### High — Body failures omit the available authored location

Requirement 2 requires a missing-capture error to include the source document
identity and an authored span or line when available. `interpolate_text`
already has each `ExpressionLocation`, including `loc.start` and `loc.end`, but
the fatal branch passes only the expression text and typed cause to
`interpolation_error` (`lib/src/markdown/compose/interpolation/rewrite.rs:123-168`).
`MarkdownError::Interpolation` has no range or line field
(`lib/src/markdown/types.rs:107-128`). At the pipeline boundary the error gains
the full on-disk `SourceContext`, but the missing-context renderer can only show
the linked file path; it has no location with which to render a focused body
excerpt (`lib/src/markdown/errors/blocks.rs:365-392`).

The Level 1 body test asserts `root.md`, `ctx.repo_root`, and `Repo`, but not a
line or span. Consequently, a document with multiple expressions reports which
file and expression failed but not where that authored expression occurs,
despite the required location being known at failure time.

Carry the original authored range (or a stable one-based line) through the
interpolation error wrapper and render it for body failures. Because
interpolation rescans replacement output, distinguish an authored location
from a generated-expression location rather than presenting a generated offset
as authored. Add Level 1 assertions for a body failure after several lines and
for a transcluded child, including the rendered diagnostic's file and line or
excerpt.

## Verification-Level Assessment

This fix concerns composition results, typed errors, warnings, cache behavior,
and CLI exit status. None of its requirements depend on a terminal emulator's
rendering, input encoder, OS keyboard events, paste/IME, mouse handling, or
scrolling. Level 1 is therefore the appropriate verification level throughout;
Level 2 and Level 3 are not required.

| Spec verification | Strongest verification present | Assessment |
| --- | --- | --- |
| 1. Typed fatal missing-capture error with variable, group, and source | Level 1 library and CLI process tests | Gap: file identity is tested, but the required available body line/span is not preserved. |
| 2. Frontmatter, body, conditions, and `$()` surfaces | Level 1 integration and evaluator tests | Sufficient for fatality and typed-cause propagation. |
| 3. Captured null/empty values remain values | Level 1 unit and integration tests | Sufficient. |
| 4. Unavailable evidence retains `PartialRuntimeCapture` | Level 1 direct-compose test | Gap: the report warning is not positively asserted and disappears on a warm persistent child-cache hit. |
| 5. Unknown keys retain the existing diagnostic only | Level 1 integration and checked-lookup tests | Sufficient. |
| 6. Missing projection key is an internal invariant failure | Level 1 unit and integration tests | Sufficient. |
| 7. Local, nested, sibling, and remote child-first capture uses one epoch | Level 1 filesystem, concurrency, and mocked-HTTP integration tests | Sufficient. |
| 8. Frozen child-first capture fails without ambient fallback | Level 1 integration and authority tests | Sufficient. |
| 9. Child-only cache identity changes and cache hits cannot bypass failure | Level 1 cold/warm persistent-cache tests in all freshness modes | Sufficient for output identity and fatal failure; warning preservation is the item 4 finding above. |
| 10. Root upgrade and child handoff regressions fail by name | Level 1 authority/epoch tests plus recorded mutation evidence | Sufficient for the functional boundary, though the handoff mutation is documented evidence rather than a permanently runnable mutation test. |
| 11. Context-free graphs avoid discovery and remain below the slow threshold | Level 1 graph and constructor tests; implementation log timing | Sufficient. |
| 12. Standalone conditions capture only reached groups and keep classification | Level 1 unit tests through the public entry point | Sufficient. |

## Implementation Assessment

The core architecture is sound. `ContextGroup::for_key` remains derived from
the capture projection authorities; `CtxLookupOutcome` keeps captured empty,
uncaptured, malformed projection, and unknown states distinct; and
`RequestContextEpoch` serializes monotonic group growth while handing each
source only its parent groups plus its own requirements. Persistent-cache
closure validation covers descendant-only groups before accepting a hit, and
the frozen authority cannot use a prior cache entry to evade the fatal lookup.

The implementation also updates the public context/cache documentation and
the Claudine callers that intentionally own a host-derived request context.
No performance or ergonomics issue beyond the two findings should block this
iteration.

## Validation Performed

- GitNexus index refreshed and current for `rusty-biscuit`; graph review traced
  the transclusion/context-authority path. `interpolate_text` has a HIGH
  upstream blast radius (47 impacted symbols across interpolation and inline
  flows), which reinforces the need for focused location tests when fixing the
  second finding.
- `git diff --check` reports one pre-existing trailing blank line at
  `claudine/lib/src/invocation_context.rs:1557`; it is outside the review
  artifacts and was not changed.
- GitNexus `detect-changes --scope all` reports CRITICAL aggregate risk for the
  implementation worktree: 208 changed symbols affect 36 execution flows.
  This review inspected the named composition, transclusion, and cache paths;
  the green package gates below provide the corresponding executable check.
- The first `darkmatter/just test` attempt could not write read-only shared
  Cargo artifacts under `target/debug/deps`. The suite was rerun with an
  isolated writable `CARGO_TARGET_DIR`: 7,898 tests passed, 7 were skipped,
  and one was classified slow.
- `darkmatter/just lint` passed for `darkmatter`, `darkmatter-cli`, `dmls`,
  `zed-dmls-cli`, and the `zed-dmls` `wasm32-wasip2` compile check using the
  same isolated target directory.
- `md schema validate --format json` reports this review valid against
  `feature-review.yaml` with no problems.

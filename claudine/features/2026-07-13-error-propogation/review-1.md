---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-17T04:24:59-07:00
spec: 2026-07-13-error-propogation/spec.md
implemented: true
description: A **feature** review of `2026-07-13-error-propogation/spec.md`
feature: 2026-07-13-error-propogation/review-1.md
next: 2026-07-13-error-propogation/review-2.md
previous: /
---

# Review 1: End-to-End Typed Error Propagation

## Verdict

The feature is **not ready for production**. The central discovery and
role-based selection work is present, and representative typed failures render
through real tmux panes, but four acceptance blockers remain. The source-aware
guard suppresses 64 known typed-error collapse findings, the versioned
`DiagnosticSnapshot` is not used by any production consumer, the two required
proxy routes intentionally retain different diagnostic identities and
renderings, and the required `just test` gate has a deterministic failing
`claudine-gen` test.

## Findings

### 1. Critical: the source-aware guard grandfathers 64 typed-provenance defects

The feature's Rust-aware scan is designed to prove that a binding is the error
side of a `Result` and that the typed value is not retained. However,
`claudine/cli/tests/error_guards/transport-allow.toml` contains 77 exceptions,
64 of which are tagged `error-propagation-followup`. Its own header states that
the scan found sites missed by the Phase 1 grep inventory and that those sites
are being frozen rather than fixed (`transport-allow.toml:18-28`). Examples
include lifecycle parsing, shell-expression resolution, provider policy loads,
composition preparation, dispatch, MCP, messaging, and wrapper orchestration.

This conflicts with the core scope and acceptance contract:

- D8 requires the implementation to inventory the complete in-scope production
  roots with a Rust-aware scan and classify every occurrence.
- The Goals require the audit to close every confirmed typed-error flattening
  site in the Claudine package area.
- Acceptance criterion 1 requires that no known typed Claudine, Darkmatter, or
  biscuit-file error be flattened at an in-process orchestration boundary
  (`spec.md:581-584`).

The allowlist's D10 rationale is not applicable. D10 requires a separate spec
for routing or retry-policy changes discovered during migration; preserving a
typed source at a newly discovered transport boundary is the work D1 and D8
explicitly require. A guard that passes by exempting the defects cannot prove
the absence of those defects.

**Required change:** review all 64 burn-down entries against their concrete
error types. Preserve each typed source through a concrete return, `#[from]`, a
typed `#[source]` wrapper, or a source-preserving report context. Keep only
genuinely unstructured or post-render conversions as narrowly reasoned
`retained` exceptions. The acceptance guard must fail while an in-scope typed
provenance defect remains.

### 2. High: `DiagnosticSnapshot` is a tested but unused production API

`claudine/lib/src/diagnostics/snapshot.rs:1-7` says every downstream consumer,
including `LifecycleErrorInfo`, machine output, and recovery records, reads one
versioned `DiagnosticSnapshot`. Production usage does not match that claim.
Outside the snapshot module and documentation, there is no production reference
to `DiagnosticSnapshot` or call to `DiagnosticSnapshot::from_diagnostic`.

Instead, `LifecycleErrorInfo` carries a separate `DiagnosticFacets` structure
with `&'static str` fields and independently calls
`select_effective_diagnostic`, `DiagnosticFacets::from_diagnostic`,
`next_registered_cause`, and `concise_message`
(`composition/lifecycle/context.rs:82-143`, `219-272`). This duplicates the
projection logic and omits the snapshot schema version and owned-string
forward-compatibility boundary. Standalone serialization round-trip tests for
`DiagnosticSnapshot` therefore do not verify any actual process, persistence,
recovery, or machine-output consumer.

This leaves D9 and acceptance criteria 1 and 3 incomplete. Rendering and
`err.*` currently share the selection algorithm, but the mandated single
serialized projection is not the source of truth and can drift.

**Required change:** project the selected diagnostic once into
`DiagnosticSnapshot` at the last in-process boundary. Make
`LifecycleErrorInfo`, machine output, and any recovery/persistence record
consume or embed that shared owned snapshot rather than rebuilding its fields.
Add integration tests that serialize through real production consumers and
prove unknown additive codes/detail fields survive their read/write path.

### 3. High: the required proxy-route parity is explicitly unimplemented

Acceptance criterion 5 requires the same proxy failure to have identical code,
headline, hint, and available typed resolution detail across supported
lifecycle routes (`spec.md:592-595`). The L2 test
`level2_proxy_routes_share_a_typed_surface_but_diverge_on_identity_in_tmux`
does the opposite: it asserts that initialize renders `Unresolvable file
reference`, while the terminal/recovery route renders `failed to load Markdown`
(`level2_typed_error_render_capture.rs:563-635`). The implementation plan also
states that AC5 is carried forward rather than met.

The test is valuable characterization, but it is evidence of an acceptance gap,
not evidence that the feature is complete. The terminal route still resolves a
missing target successfully and fails later at document read, so route-specific
resolver choice changes the public diagnostic identity and corrective guidance.

**Required change:** land the unified file-reference resolution dependency (or
the minimum shared proxy-resolution seam needed here), then promote the L2 test
to assert identical code, headline, hint, and typed resolution detail while
allowing only event/property context to differ. If that work is intentionally
deferred, this feature must remain not ready until the dependency lands and the
acceptance test passes.

### 4. High: the required `just test` gate is deterministically red

Acceptance criterion 9 requires `just test`, `just test-l2`, and `just lint` to
pass (`spec.md:604-606`). The implementation plan records one failing
`claudine-gen` test, and the source confirms it:

- `claudine/gen/tests/drift.rs:27-34` reads
  `reviews/2026-07-14-module-assessment/generated-artifact-baseline.json`.
- That path does not exist; the file now lives under
  `reviews/_completed/2026-07-14-module-assessment/`.
- The test uses `expect`, so the missing old path always fails.

Calling this pre-existing does not make the required package gate green. A
production-ready verdict cannot rely on a gate known to fail, especially when
the acceptance criteria name that gate explicitly.

**Required change:** update the drift test to the archived path (or move the
baseline to a stable non-lifecycle location), run the complete Claudine area
`just test`, and record a green result alongside `just test-l2` and `just lint`.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| No known typed error is flattened across in-process boundaries | Level 1 Rust-aware structural scan | **Gap:** 64 scanner findings are suppressed as follow-up debt; this does not satisfy D1, D8, or AC1. |
| Every Claudine diagnostic is discoverable through one registry | Level 1 source-parity and runtime downcast tests | Appropriate level for a Rust type/registry invariant. The separate boxed-reachability guard is also useful, subject to its recorded `AtomicWriteFailed` exception. |
| Owning/transparent selection, cycle/depth guards, and one-level cause | Level 1 unit and integration tests | Appropriate level for in-process selection semantics. |
| One versioned snapshot drives lifecycle, machine, recovery, and persistence consumers | Level 1 standalone snapshot round-trip tests | **Gap:** no production consumer uses the snapshot, so the required integration boundary is unverified and unimplemented. |
| Motivating initialize proxy failure renders an actionable component block | Level 1 process tests plus Level 2 tmux capture | Appropriate level. L2 verifies the block, actionable content, real-terminal SGR, OSC 8, and `NO_COLOR` behavior. |
| Initialize and terminal/recovery proxy routes share identity, headline, hint, and detail | Level 2 tmux capture | **Gap:** the strongest test explicitly asserts divergent headlines and failure stages. |
| Composition lookup, schema failure, transclusion failure, pre-flight failure, and unstructured fallback render correctly | Level 2 tmux capture | Appropriate level for the terminal rendering contract. |
| Frontmatter excerpt, plain output, exit code, lifecycle ordering, and exactly-once emission remain stable | Level 1 process characterization plus representative Level 2 captures | Appropriate levels for the exercised routes. |
| Catalog detail shape, present-null optionals, `err.msg` hygiene, and cause shape | Level 1 unit and structural parity tests | Appropriate level for structured data contracts, but it does not replace the missing production snapshot integration. |
| Required package gates pass | Recorded Level 1/L2/lint runs plus current static inspection | **Gap:** `just test` cannot pass while the `claudine-gen` drift test reads a nonexistent path. |

Level 3 is not applicable. This feature does not assert physical-key,
terminal-input encoding, paste, IME, mouse, or hotkey behavior. Level 2 is the
correct maximum tier for its real-terminal styling, hyperlink, plain-output,
and rendered-content requirements.

## Verification Performed

- Inspected the specification, implementation plan, decisions, inventory,
  diagnostic registry/selection, snapshot projection, lifecycle `err.*`
  projection, top-level walker, Rust-aware guards, L1 characterization, and L2
  tmux suite.
- Counted 77 transport allowlist entries: 64
  `error-propagation-followup` and 13 `retained`.
- Confirmed `DiagnosticSnapshot` has no production call site outside its own
  module/documentation.
- Confirmed the old generator baseline path is absent and the baseline exists
  under `reviews/_completed/`.
- Attempted `just test` from `claudine/`. With the warmed build, the `claudine`
  suite passed 3,485 tests (7 skipped; one retry) and `claudine-contract` passed
  47 tests (5 skipped). The area run then exceeded the non-interactive
  per-command ceiling while compiling `claudine-cli`, so it was terminated and
  did not provide a complete gate result. A narrowed feature-test run likewise
  exceeded the compilation ceiling. The deterministic generator-path failure
  above independently proves the full gate is not green.
- The implementation record reports `just test-l2` passing 131 tests and
  `just lint` passing, but those gates were not independently rerun to
  completion during this review.

## Closure Criteria

The feature can be reviewed again after all four findings are resolved. In
particular, the next review should see no burn-down exceptions for in-scope
typed provenance defects, production consumers wired through the shared
versioned snapshot, a Level 2 proxy-parity test that asserts equality rather
than divergence, and green complete results for `just test`, `just test-l2`,
and `just lint`.

---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T14:14:35-07:00
spec: 2026-07-13-error-propogation/spec.md
implemented: true
description: A **feature** review of `2026-07-13-error-propogation/spec.md`
feature: 2026-07-13-error-propogation/review-4.md
previous: 2026-07-13-error-propogation/review-3.md
next: 2026-07-13-error-propogation/review-5.md
---

# Review 4: End-to-End Typed Error Propagation

## Verdict

The feature is **not ready for production**. The typed-error architecture itself
is coherent: the central registry and role-based selector feed the shared
`DiagnosticSnapshot`, lifecycle `err.*`, persisted/reporting records, restored
diagnostics, and the top-level `BlockError` renderer. The focused diagnostic
suite passed 171 of 171 Level-1 tests, and the seven required real-terminal
failure classes have appropriately placed Level-2 tmux cases. Review 3's stale
terminal-proxy rationale is corrected, and the new WezTerm reachability probe
passes its 85-test harness suite and Clippy.

Production readiness remains blocked by Acceptance Criterion 9: there is still
no completed current green run of the full Claudine `just test`, `just test-l2`,
and `just lint` gates. The WezTerm change verifies the stale/unresponsive-socket
case at Level 1, but the canonical Level-2 suite has not demonstrated that the
original multi-test timeout is gone. Three other characterization comments also
still describe already-completed migration work as current or future behavior.

## Findings

### 1. High — The required full gate remains unverified; the WezTerm fix has only Level-1 closure evidence

Acceptance Criterion 9 explicitly requires completed green `just test`,
`just test-l2`, and `just lint` runs (`spec.md:622-624`). The new
`WezTermHarness::available` implementation now performs a bounded
`wezterm cli list --format json` probe (`biscuit-test-harness/src/wezterm.rs:201-254`),
and its manufactured stale-socket regression returns `false` inside three
seconds (`biscuit-test-harness/src/wezterm.rs:1085-1133`). That directly addresses
the stale/unresponsive endpoint failure mode and is green in the complete
biscuit-test-harness Level-1 suite.

It does not by itself prove that `just test-l2` completes. The previous failure
occurred after the WezTerm tests passed their availability gate and entered
`shared_or_spawn`; the new probe establishes that `list` responds, not that the
subsequent pane-spawn path can complete under the suite's parallel load. A
responsive-host smoke test also repeats `cli list` and early-returns when the
backend is unavailable (`wezterm.rs:1135-1166`); it does not create a pane or
exercise the canonical Claudine Level-2 recipe.

This review's focused Level-2 invocation was stopped during the cold
`claudine-cli`/DuckDB rebuild at the non-interactive subprocess ceiling, before
Nextest executed the proxy-parity case. Full Level-1 and lint gates likewise
have no new complete result. The focused 171-test diagnostic run is strong
feature evidence, but it is not the package-area acceptance gate.

**Required change:** run the complete current Claudine `just test`,
`just test-l2`, and `just lint` recipes to completion and record their results.
The Level-2 run must show that an unusable WezTerm backend skips cleanly and that
an available backend can complete `shared_or_spawn`; if pane creation can still
time out after `cli list` succeeds, make the availability/spawn contract agree
rather than accepting a green probe alone.

### 2. Medium — Three characterization blocks still document the pre-migration implementation

Review 3's terminal-proxy paragraph was fixed, but the same characterization
file retains three other stale descriptions:

- Route 1 says `pipeline.rs:1162` flattens the initialize proxy error with
  `eyre!` and that Phases 4/5 will replace it (`characterization_error_routes.rs:185-196`).
  The typed wrapper and `StatusBlock` rendering are already the current path.
- Route 3 says Phase 5 will migrate the post-start setup route to a typed
  diagnostic (`characterization_error_routes.rs:375-384`). The implementation
  now calls `LifecycleErrorInfo::from_error_or_action`, and the dedicated
  Level-2 lifecycle test asserts `config` / `config.invalid`.
- Route 4 says `HarnessError` is undiscoverable and only flattened harness text
  renders (`characterization_error_routes.rs:435-444`). `as_diagnostic` registers
  `HarnessError`, and the real-terminal pre-flight case asserts its structured
  block.

These comments are now the inverse of the code they accompany. That conflicts
with the repository's comment-quality rule and the spec's explicit maintenance
requirement to review rendering/propagation comments after behavior changes
(`spec.md:626-640`).

**Required change:** rewrite all three blocks in present tense around the
current typed routes. Preserve the useful D10 baseline explanation—exit code,
event order, and exactly-once emission—without retaining obsolete line numbers,
future-phase language, or claims that registered errors are flattened.

## Requirement Verification Levels

| Requirement | Strongest verification observed | Assessment |
|---|---|---|
| Preserve typed errors or a versioned snapshot across in-process and erased boundaries | Level 1 Rust-aware source guard, typed-chain tests, snapshot selection, and production record serialization | Appropriate. The single ratified D8 category-4 exception remains explicit; focused diagnostic/snapshot tests are green. |
| Discover every Claudine diagnostic and select one effective diagnostic | Level 1 registry parity, runtime downcast, semantic/transparent selection, cycle/depth, and boxed-reachability tests | Appropriate. Rendering and classification resolve through the same selected value. |
| Make terminal rendering, lifecycle `err.*`, and machine/persistence projection agree | Level 1 error-walker, `LifecycleErrorInfo`, `DiagnosticSnapshot`, restored-diagnostic, MCP import, reporting, loop, and sequence tests | Appropriate for the data/selection invariant; the focused diagnostic suite passed 171 of 171. |
| Render initialize proxy resolution as a source-aware component block | Level 1 process test plus Level 2 tmux capture | Appropriate. The Level-2 test checks block structure, actionable content, red SGR, OSC 8, `NO_COLOR`, and exit status. |
| Give initialize and terminal/recovery proxy routes the same identity, headline, hint, and available detail | Level 2 tmux parity capture, supported by Level 1 route characterization | Appropriate. The test permits only the documented event-context difference. It was green in Review 3; this review's rerun did not reach execution during the rebuild. |
| Cover composition lookup, schema/file reference, transclusion, harness pre-flight, and unstructured fallback rendering | Level 2 tmux captures for every required class | Appropriate placement. Typed cases reject the generic `Error:` line; the unstructured control requires it. |
| Preserve frontmatter excerpts, color/plain behavior, exit codes, lifecycle order, and exactly-once emission | Level 1 characterization plus representative Level 2 tmux captures | Appropriate for behavior. The assertions remain useful, but three accompanying comments are stale. |
| Keep detail catalog-shaped, present-null, forward-compatible, and one-cause deep | Level 1 catalog/corpus, round-trip, unknown-code/detail, and `err.*` projection tests | Appropriate and green in the focused run. |
| Pass all required Claudine gates | Complete biscuit-test-harness Level 1 + focused Claudine Level 1; attempted Claudine Level 2 | **Gap.** The current complete `just test`, `just test-l2`, and `just lint` area gates are not demonstrated. The WezTerm closure is Level 1 only. |

Level 3 is not applicable. This feature does not assert physical-key delivery,
terminal input encoding, paste, IME, mouse, or hotkey behavior. Level 2 is the
correct maximum tier for its real-terminal component rendering, SGR, OSC 8,
wrapping, and plain-output contracts.

## Verification Performed

- `just test biscuit-test-harness` passed 85 of 85 Level-1 tests, including the
  new missing-socket, missing-binary, stale-socket timeout, and responsive-host
  probe cases.
- `cargo clippy -p biscuit-test-harness --all-targets --all-features -- -D warnings`
  completed cleanly.
- A focused Claudine library run selected 171 diagnostic, snapshot, restored,
  lifecycle, MCP-import, and reporting tests; all 171 passed.
- Inspected the central discovery/selection walk, shared snapshot projection,
  lifecycle `err.*` conversion, top-level error walker, lossy-boundary guard,
  seven-route Level-2 capture suite, and D10 characterizations.
- Attempted the focused proxy-parity case through the canonical
  `just test-l2` recipe. The invocation was stopped during the cold dependency
  rebuild at the session's subprocess ceiling; no Level-2 test executed and no
  product assertion failed.
- The complete Claudine `just test`, `just test-l2`, and `just lint` gates were
  not completed in this review.
- Preserved the caller's existing unrelated modifications to
  `.claudine/memory/commits.md`, `CLAUDE.md`, and the in-scope uncommitted
  `biscuit-test-harness` implementation under review.

## Production Readiness Closure

No new architectural error-propagation defect was found. Production readiness
requires complete green current area gates and correction of the remaining
stale characterization prose. Until both are demonstrated, the feature remains
not ready.

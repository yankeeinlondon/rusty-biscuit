---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T17:21:59-07:00
spec: 2026-07-13-error-propogation/spec.md
implemented: false
description: A **feature** review of `2026-07-13-error-propogation/spec.md`
feature: 2026-07-13-error-propogation/review-7.md
previous: 2026-07-13-error-propogation/review-6.md
---

# Review 7: End-to-End Typed Error Propagation

## Verdict

The feature is **not ready for production**. Review 6's property-path and
documentation findings are closed, and the central typed diagnostic,
selection, snapshot, restoration, and component-rendering architecture remains
sound. Three high-severity gaps remain: terminal proxy failures discard their
known lifecycle event from structured detail, the new Level-2 automatic-TTY
test still inherits color-forcing variables from the tmux harness, and the
required current `just test`, `just test-l2`, and `just lint` gates have not
completed.

## Findings

### 1. High — Terminal proxy diagnostics discard the known lifecycle event

The specification requires lifecycle semantic wrappers to carry both the event
and action/property path (§D3), and requires event/property context to be
asserted separately across proxy routes (§Testing Strategy). The public
`FileReferenceContext` contract also reserves `None` for non-lifecycle surfaces
and documents `event` as the lifecycle event that was running
(`lib/src/composition/error/mod.rs:2140-2154`).

`dispatch_terminal_control` now derives the correct terminal event and uses it
in `failure.stack[*].proxy` (or the corresponding terminal signal), but then
stores `event: None` (`cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs:194-203`).
The detail projection serializes that value directly
(`lib/src/composition/error/render/mod.rs:380-383`), so
`err.detail.event`, snapshots, and machine/persistence projections receive
`null` even though the event is known. The human block also loses the explicit
“in the `failure` event” surface and only exposes the event incidentally inside
the property string.

The Level-2 parity test codifies the loss as intentional and asserts only that
the initialize route names its event
(`cli/tests/level2_typed_error_render_capture.rs:648-652,737-744`). That
contradicts the specification rather than verifying it.

**Required change:** store `Some(event.to_string())` in the terminal
`FileReferenceContext`. Add a live-route Level-1 assertion on the structured
`event` and `property` fields for both initialize and terminal/recovery proxy
failures, plus a Level-2 assertion specific to the rendered event label. Do not
allow the event embedded in the property path to satisfy the separate event
contract.

### 2. High — The “automatic TTY” Level-2 test still runs through forced color

Review 6 required an unforced real-pane case because the specification names
automatic TTY detection separately from `FORCE_COLOR` and `NO_COLOR`. The new
`TTY_AUTO` case claims that neither override is exported and that
`Terminal::new()` therefore probes the pane
(`cli/tests/level2_typed_error_render_capture.rs:398-404,527-547`).

That premise is false for `TmuxHarness`. Every spawned tmux shell calls
`apply_color_forcing_env` (`biscuit-test-harness/src/tmux.rs:305-308`), which
sets both `FORCE_COLOR=1` and `CLICOLOR_FORCE=1`
(`biscuit-test-harness/src/lib.rs:413-436`). `run_in_pane` unsets only
`NO_COLOR`; `TTY_AUTO` adds only `COLUMNS`
(`cli/tests/level2_typed_error_render_capture.rs:355-360,380-404`). Consequently
Claudine's `compute_terminal` still takes its `FORCE_COLOR` optimistic branch,
and the test can pass without exercising automatic TTY detection at all.

The strongest reliable evidence for automatic detection therefore remains
below the required Level 2. This is a verification-level gap and must be
treated as high severity under the review rubric.

**Required change:** explicitly unset `NO_COLOR`, `FORCE_COLOR`, and
`CLICOLOR_FORCE` in the spawned pane before the automatic case, or use a
harness mode that preserves natural capabilities. Keep the forced-color and
no-color cases because they exercise different branches. The automatic case
must still assert real-pane SGR and OSC 8 output after the forcing variables
are proven absent.

### 3. High — Acceptance Criterion 9 still lacks complete current gate evidence

Acceptance Criterion 9 explicitly requires the Claudine-area `just test`,
`just test-l2`, and `just lint` recipes to pass. The only post-Review-6
implementation commit is `091b092d4`; no complete gate record follows it.

This review ran `just test`. The catalog crate passed 21 tests, and the
Claudine library reached 772 passing tests with no observed failure, but the
command exceeded the non-interactive 60-second ceiling and was interrupted
with 3,057 library tests still unrun. A focused CLI test invocation also
exceeded the ceiling during compilation before executing tests. The fast
`lint-lifecycle-doc-facets` guard passed. These are useful partial signals, but
none is a completed canonical gate; `just test-l2` and the full `just lint`
were not started after the repeated compilation ceiling.

**Required change:** after Findings 1 and 2 are fixed, run `just test`,
`just test-l2`, and `just lint` to completion in the Claudine package area and
record their exact summaries. Partial compilation, focused tests, or historical
results do not satisfy the explicit acceptance criterion.

## Requirement Verification Levels

| Requirement | Strongest verification observed | Assessment |
|---|---|---|
| Preserve typed causes or a versioned snapshot across in-process and erased boundaries | Level 1 Rust-aware transport guards, typed-source tests, snapshot round trips, and restoration tests | Appropriate in design; no new competing transport path was found. The current full gate did not complete. |
| Discover every Claudine diagnostic through one registry and select one effective diagnostic | Level 1 registry parity, runtime downcast, semantic/transparent selection, repeated-chain, and depth-guard tests | Appropriate. The CLI walker and snapshot projection share the selector. |
| Keep terminal rendering, lifecycle `err.*`, and serialized projection on the same diagnostic | Level 1 error-walker, `LifecycleErrorInfo`, `DiagnosticSnapshot`, and restored-diagnostic tests | Appropriate for identity/selection. Terminal proxy detail still loses its known `event`. |
| Carry source document, lifecycle event, and authored property context through proxy wrapping | Live-route Level 1 rendering plus Level 2 tmux capture | **Gap.** Property paths are now stable, but terminal `err.detail.event` is still `null`; no live structured assertion catches it. |
| Render initialize and terminal/recovery proxy misses as the same typed component block | Level 2 tmux parity capture plus Level 1 process characterization | Appropriate for block identity, headline, hint, exit code, ordering, and emission count. Event-detail completeness remains open. |
| Cover composition lookup, schema/file-reference, transclusion, harness pre-flight, and unstructured fallback rendering | Level 2 tmux captures plus Level 1 process tests | Appropriate. Typed routes distinguish `StatusBlock` output from the generic fallback. |
| Preserve forced color, `NO_COLOR`, plain/piped output, and OSC 8 behavior | Level 2 tmux for forced color and `NO_COLOR`; Level 1 spawned-process coverage for piped stderr | Appropriate for those explicit branches. |
| Preserve automatic real-TTY detection | A nominal Level-2 tmux test whose shell inherits `FORCE_COLOR=1` and `CLICOLOR_FORCE=1` | **Gap.** The test exercises forced capability selection, not automatic detection. |
| Preserve exit codes, lifecycle ordering, retry/control decisions, and exactly-once emission | Level 1 route characterization plus representative Level 2 captures | Appropriate for the covered routes. |
| Keep diagnostic detail catalog-shaped, structured, and present-null only when genuinely unavailable | Level 1 catalog/corpus guards and snapshot tests | Structurally appropriate, but the terminal event is available and is incorrectly projected as `null`. |
| Pass all required Claudine gates | Historical focused evidence plus incomplete current test attempts | **Gap.** Complete current `just test`, `just test-l2`, and `just lint` results are absent. |

Level 3 is not applicable. This feature does not specify OS keyboard/mouse
delivery, terminal input encoding, paste, IME, or hotkey behavior. Its terminal
requirements concern rendered output, for which Level 2 is the correct tier.

## Verification Performed

- Read the specification, prior reviews, implementation plan/decisions,
  inventory, central discovery/selection implementation, snapshots,
  restoration, lifecycle projection, top-level error walker, proxy routes,
  rendering tests, and living error-architecture documentation.
- Used GitNexus against the current worktree to trace the central discovery and
  effective-selection symbols and their callers.
- Verified Review 6's property-path fix in all three production constructors
  and its exact-path Level-1/Level-2 assertions.
- Verified Review 6's documentation fix: the living docs now distinguish the
  typed probe and no-probe resolver arms from the legacy present-null arm.
- Attempted the current `just test` and a focused CLI test under the mandated
  non-interactive ceiling; both were interrupted as described in Finding 3.
- Ran `just lint-lifecycle-doc-facets`; it passed.
- Preserved the caller's unrelated existing changes to `CLAUDE.md` and
  `prompts/_implement/implement-suggestions.md`.

## Production Readiness Closure

Project the terminal event as structured diagnostic context, make the
automatic-TTY Level-2 case genuinely unforced, and complete all three canonical
Claudine gates. Until all three conditions are met, this feature is not
production ready.

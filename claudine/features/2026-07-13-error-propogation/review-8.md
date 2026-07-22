---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T18:18:20-07:00
spec: 2026-07-13-error-propogation/spec.md
implemented: true
description: A **feature** review of `2026-07-13-error-propogation/spec.md`
feature: 2026-07-13-error-propogation/review-8.md
previous: 2026-07-13-error-propogation/review-7.md
next: 2026-07-13-error-propogation/review-9.md
---

# Review 8: End-to-End Typed Error Propagation

## Verdict

The feature is **not ready for production**. Review 7's product-side defect is
closed: initialize and terminal proxy failures now project their lifecycle
event and authored property as independent structured fields, and focused
Level-1 tests pass. The required Level-2 gate remains red, however, because the
new automatic-TTY case asserts OSC 8 support that an unforced tmux environment
does not advertise, while the route-parity case uses a wrap-sensitive event
label assertion. The full `just test` and `just lint` gates also did not
complete within the non-interactive execution ceiling.

## Findings

### 1. High — The automatic-TTY Level-2 case reaches detection but requires an unsupported capability

The Review 7 fix correctly removes and proves the absence of `NO_COLOR`,
`FORCE_COLOR`, and `CLICOLOR_FORCE` before launching Claudine
(`cli/tests/level2_typed_error_render_capture.rs:410-444`). The captured process
therefore reaches `Terminal::new()` as intended. Its red SGR assertion passes,
which is valid Level-2 evidence that automatic TTY/color detection is active.

The same test then requires OSC 8 (`cli/tests/level2_typed_error_render_capture.rs:600-606`),
but an unforced tmux pane does not identify a hyperlink-capable outer terminal.
`osc8_link_support` deliberately enables links only for known terminal
identifiers (`biscuit-terminal/lib/src/discovery/detection/osc8.rs:21-49`). The
real capture consequently renders the documented Markdown fallback and the
test fails consistently after all retries. The forced-color test passes because
`Terminal::new_optimistic()` enables OSC 8; that does not prove the automatic
capability branch.

This is a Level-2 verification gap and also directly violates Acceptance
Criterion 9, which requires `just test-l2` to pass.

**Required change:** keep the unforced tmux case as the automatic TTY/color
test and assert its supported contract (StatusBlock, SGR, actionable content,
and hyperlink fallback). Verify automatic OSC 8 selection in an unforced,
known-capable backend such as WezTerm, after proving the same override variables
are absent. Do not make tmux itself imply that its unknown outer terminal
supports hyperlinks.

### 2. High — The event-label Level-2 assertion is invalid under legitimate wrapping

The terminal proxy capture visibly renders the new structured event separately:

```text
referenced by `failure.stack[*].proxy` in the `failure`
event of ...
```

That satisfies the product requirement and agrees with the passing Level-1
`err.detail.event` assertion. The Level-2 test nevertheless searches for the
single contiguous substring `` `failure` event of ``
(`cli/tests/level2_typed_error_render_capture.rs:779-790`). At the configured
width, `Prose` legitimately wraps between the styled event name and “event of,”
so the assertion fails on every observed attempt.

This is not a rendering defect, but it leaves the required Level-2 gate red and
makes the test sensitive to path length and pane width instead of the semantic
content it is meant to verify.

**Required change:** assert the event label with whitespace-normalized captured
text or a wrap-tolerant match such as `` `failure`\s+event of ``. Retain the
independent full-property assertion so an event name occurring only inside
`failure.stack[*].proxy` cannot satisfy the event contract.

### 3. High — Acceptance Criterion 9 still has no complete current gate pass

The canonical gate evidence from this review is:

- Focused Level 1: both new proxy-detail tests passed (2/2).
- `just test-l2`: failed. Before interruption, 145/147 tests reached a final
  result: 144 passed and the automatic-TTY test failed; the parity test also
  failed three observed attempts before the run was interrupted with two tests
  unfinished.
- `just test`: the catalog crate passed 21/21. The library reached 1,059/3,829
  observed passes with no failure, but the recipe did not complete within the
  non-interactive ceiling.
- `just lint`: all 18 error-transport guards and the lifecycle-doc-facets guard
  passed; Clippy completed for the catalog, library, and contract crates. The
  recipe exceeded the ceiling while checking the CLI/rendezvous dependency
  graph and was interrupted with exit 130.

Partial runs are useful diagnostics but cannot satisfy the specification's
explicit requirement that `just test`, `just test-l2`, and `just lint` pass.

**Required change:** repair Findings 1 and 2, then run all three canonical
Claudine recipes to completion and record their exact summaries.

## Requirement Verification Levels

| Requirement | Strongest verification observed | Assessment |
|---|---|---|
| Preserve typed causes or versioned snapshots across in-process and erased boundaries | Level 1 typed-chain, snapshot, restoration, and Rust-aware transport guards | Appropriate; all 18 transport guards passed in this review. |
| Discover every Claudine diagnostic through one registry and select one effective diagnostic | Level 1 registry parity, runtime downcast, and semantic/transparent selection tests | Appropriate; the registry parity guard passed. |
| Use the same diagnostic for terminal rendering, lifecycle `err.*`, and serialized output | Level 1 selector/snapshot/lifecycle tests plus Level 2 representative captures | Appropriate for identity and projection. |
| Carry source document, lifecycle event, and authored property through both proxy routes | Level 1 live-route structured assertions and Level 2 tmux captures | Product behavior is present. The Level-2 event assertion is wrap-fragile and fails. |
| Render initialize and terminal/recovery proxy misses as the same component diagnostic | Level 2 tmux route captures | The captures show the shared code/headline/hint and separate route context, but the parity test fails on wrapping. |
| Cover composition lookup, schema/file-reference, transclusion, harness pre-flight, and unstructured fallback | Level 2 tmux captures | Appropriate; the representative cases completed successfully in the observed gate run. |
| Preserve forced color, `NO_COLOR`, plain output, and OSC 8 component rendering | Level 2 forced-color and `NO_COLOR` tmux captures | Appropriate for those explicit modes. |
| Preserve automatic real-TTY color detection | Level 2 unforced tmux capture with all forcing variables proven absent | Appropriate and passing for SGR; the test incorrectly couples this to OSC 8 support. |
| Select OSC 8 automatically when the detected terminal supports it | Forced Level 2 capture plus failing unforced tmux assertion | **Gap.** Use an unforced, known-capable real backend for automatic capability evidence. |
| Preserve exit codes, lifecycle ordering, control decisions, and exactly-once emission | Level 1 characterization plus Level 2 route captures | Appropriate for the reviewed routes. |
| Keep diagnostic detail catalog-shaped and structured | Level 1 catalog/corpus guards and focused live-route assertions | Appropriate; the known terminal event now projects as `failure`, not `null`. |
| Pass every required Claudine gate | One failing Level-2 gate and incomplete full Level-1/lint gates | **Gap.** Acceptance Criterion 9 is not met. |

Level 3 is not applicable. The feature specifies terminal output, diagnostic
selection, and lifecycle data propagation; it does not depend on OS keyboard or
mouse injection, terminal input encoding, paste, IME, or hotkey behavior.

## Verification Performed

- Read the specification, Review 7, the Claudine error-architecture guidance,
  the test-tier contract, and the three files changed after Review 7.
- Used GitNexus on the current worktree to trace `dispatch_terminal_control`,
  `select_effective_diagnostic`, and the file-reference detail projection.
- Verified the terminal dispatch is shared by failure/finalize/recovery callers
  and that both proxy routes feed the same `LifecycleErrorInfo` projection.
- Ran the focused Level-1 proxy-detail tests: 2 passed.
- Ran `just test-l2`, `just test`, and `just lint` with the results recorded in
  Finding 3.
- Preserved the caller's unrelated changes to `CLAUDE.md` and
  `prompts/_implement/implement-suggestions.md`.

## Production Readiness Closure

Make the automatic OSC 8 case target a terminal that can advertise the
capability, make the event-label assertion wrap-tolerant, and complete all three
canonical gates. Until then, the feature is not production ready.

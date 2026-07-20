---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T14:42:53-07:00
spec: 2026-07-13-error-propogation/spec.md
implemented: true
description: A **feature** review of `2026-07-13-error-propogation/spec.md`
feature: 2026-07-13-error-propogation/review-5.md
previous: 2026-07-13-error-propogation/review-4.md
---

# Review 5: End-to-End Typed Error Propagation

## Verdict

The feature is **not ready for production**. No new typed-error propagation,
selection, rendering, or snapshot defect was found. Review 4's three stale
characterization blocks are corrected, the 18-test transport guard is green,
the proxy-route identity test is green in a real tmux terminal, and a live
WezTerm Level-2 case now completes `WezTermHarness::shared_or_spawn`. That is
the correct Level-2 evidence for the specific backend failure that blocked the
previous review.

Production readiness remains blocked by Acceptance Criterion 9. There is still
no completed current run of the full Claudine `just test`, `just test-l2`, and
`just lint` trio. This review's bounded full-gate attempts made progress without
an observed product failure but were stopped at the non-interactive subprocess
ceiling. Focused green tests establish the feature behavior; they do not replace
the explicitly required area gates.

## Findings

### 1. High — Acceptance Criterion 9 still lacks complete current gate evidence

The specification requires `just test`, `just test-l2`, and `just lint` to pass
in the Claudine package area (`spec.md:621-624`). No implementation or review
record after Review 4 contains a complete current result for those three
recipes.

The original WezTerm concern is materially improved and now has the right level
of evidence. `just test biscuit-test-harness` passed 85 of 85 Level-1 tests,
including the stale-socket timeout and responsive-host probe. More importantly,
`just test-l2 level2_perf_tree_renders_styled_in_wezterm` passed against the live
WezTerm backend after entering `shared_or_spawn`, and
`level2_proxy_routes_share_identity_across_routes_in_tmux` passed in tmux. The
new reachability probe therefore no longer rests on manufactured Level-1 input
alone.

The complete area gates remain unproven:

- The bounded `just test` run completed the catalog, core-library, and contract
  portions without an observed failure. It reached the CLI suite, where 300
  tests had passed before the run was interrupted at the session ceiling;
  1,869 CLI tests and the generator package did not complete in that attempt.
- The bounded `just lint` run completed the transport and lifecycle-document
  guards plus Clippy for catalog types, the library, and the contract. It was
  interrupted while checking `claudine-cli`; the remaining CLI and generator
  lint work did not complete.
- A complete `just test-l2` was not run. The two focused Level-2 checks above
  are green, including the formerly suspect WezTerm spawn boundary, but they do
  not execute the whole real-terminal matrix.

No failure from these interrupted runs is attributed to the product—their exit
status came from the deliberate SIGINT used to honor the non-interactive command
ceiling. They nevertheless cannot be recorded as passing gates.

**Required change:** run all three canonical recipes to completion in a session
that permits their full runtime and record the exact summaries. If they are
green, this review found no remaining feature-level defect that would prevent a
production-ready verdict.

## Requirement Verification Levels

| Requirement | Strongest verification observed | Assessment |
|---|---|---|
| Preserve typed errors or a versioned snapshot across in-process and erased boundaries | Level 1 Rust-aware transport guard, typed-source tests, snapshot selection, restoration, and serialization tests | Appropriate. `just lint-transport` passed 18 of 18, including typed-collapse, boxed-source, registry, catalog-shape, and snapshot re-erasure guards. |
| Discover every Claudine diagnostic and select one effective diagnostic | Level 1 source parity, runtime downcast, semantic/transparent selection, and renderer/lifecycle agreement tests | Appropriate. The focused diagnostic selection passed 22 of 22; the registry guard is also green in the transport suite. |
| Make terminal rendering, lifecycle `err.*`, and machine/persistence projection agree | Level 1 error-walker, `LifecycleErrorInfo`, `DiagnosticSnapshot`, restored-diagnostic, reporting, loop, sequence, and MCP tests | Appropriate for selection and data transport. No competing selection path was found in current source. |
| Render initialize proxy resolution as a source-aware component block | Level 1 process test plus Level 2 tmux capture | Appropriate. The real-terminal case asserts the component block rather than generic `Error:` fallback. |
| Give initialize and terminal/recovery proxy routes the same identity, headline, hint, and available detail | Level 2 tmux route-parity capture | Appropriate and green in this review. The test permits only the specified event/property context difference. |
| Cover composition lookup, schema/file reference, transclusion, harness pre-flight, and unstructured fallback rendering | Level 2 tmux captures, with Level 1 component and fallback tests | Appropriate placement for terminal rendering, SGR/OSC behavior, and generic-fallback discrimination. These cases were unchanged by the Review 4 closure commit. |
| Preserve frontmatter excerpts, color/plain behavior, exit codes, lifecycle order, and exactly-once emission | Level 1 route characterization plus representative Level 2 captures | Appropriate. Review 4's stale Route 1, Route 3, and Route 4 prose now describes the current typed behavior while retaining the D10 baselines. |
| Keep detail catalog-shaped, present-null, forward-compatible, and one-cause deep | Level 1 catalog/corpus guards and snapshot round-trip tests | Appropriate and green in the focused transport/diagnostic runs. |
| Reject an unusable WezTerm backend without hiding a broken usable-backend spawn path | Level 1 bounded stale-socket probe plus Level 2 live WezTerm `shared_or_spawn` capture | Appropriate. This closes Review 4's level mismatch for the backend fix. |
| Pass all required Claudine gates | Partial full-gate runs plus focused Level 1 and Level 2 runs | **Gap.** Complete current `just test`, `just test-l2`, and `just lint` results are still absent. |

Level 3 is not applicable. The feature does not assert OS keyboard or mouse
delivery, terminal input encoding, paste, IME, or hotkey behavior. Its terminal
requirements concern rendered pane content and styling, for which Level 2 is
the correct maximum tier.

## Verification Performed

- `just test biscuit-test-harness`: 85 passed, 0 failed.
- `just lint-transport`: 18 passed, 0 failed.
- `just test-cli diagnostic`: 22 passed, 0 failed.
- `just test-l2 level2_proxy_routes_share_identity_across_routes_in_tmux`: 1
  passed, 0 failed.
- `just test-l2 level2_perf_tree_renders_styled_in_wezterm`: 1 passed, 0
  failed, exercising live WezTerm `shared_or_spawn`.
- Inspected the central discovery/selection seam, snapshot boundary,
  lifecycle projection sites, top-level error walker, error-guard inventory,
  ten typed-render Level-2 cases, and corrected D10 characterization prose.
- Attempted the complete `just test` and `just lint` recipes. Both were
  deliberately interrupted when they exceeded the non-interactive subprocess
  ceiling; no completed assertion or Clippy failure was observed before the
  interruption.
- GitNexus's MCP transport was unavailable (`Transport closed`), so execution
  flow inspection fell back to the indexed symbol names documented by the
  feature plus direct source, test, history, and blame inspection.
- Preserved the caller's unrelated existing modification to `CLAUDE.md`.

## Production Readiness Closure

Review 4's implementation and documentation concerns are closed at the
appropriate verification levels. The only remaining blocker found in Review 5
is the specification's explicit full-gate requirement. Until complete current
green results exist for all three canonical recipes, the feature remains not
ready for production.

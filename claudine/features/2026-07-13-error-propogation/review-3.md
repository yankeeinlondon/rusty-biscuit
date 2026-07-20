---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T13:05:57-07:00
spec: 2026-07-13-error-propogation/spec.md
implemented: true
description: A **feature** review of `2026-07-13-error-propogation/spec.md`
feature: 2026-07-13-error-propogation/review-3.md
previous: 2026-07-13-error-propogation/review-2.md
---

# Review 3: End-to-End Typed Error Propagation

## Verdict

The feature is **not ready for production**. Review 2's propagation findings
are implemented: erased reports now project into `DiagnosticSnapshot`, sync
failures persist that snapshot beside their prose, `--repo` restores its
captured diagnostic instead of re-erasing `snapshot.message`, the lossy-source
guard detects snapshot re-erasure, and the two proxy routes now render the same
resolution-time diagnostic. The focused Level 1 and Level 2 checks for those
changes pass.

Production readiness remains blocked by Acceptance Criterion 9. The canonical
Level-2 gate still cannot complete reliably on an available WezTerm backend:
tests pass the `available()` gate and then repeatedly time out in
`shared_or_spawn()`. The complete Level-1 and lint gates also lack a current
green end-to-end result. There is additionally stale characterization prose
that still describes the proxy divergence the implementation has removed.

## Findings

### 1. High — The required Level-2 gate misclassifies an unusable WezTerm backend and does not complete

`just test-l2` is an explicit production-readiness criterion. The current
canonical run started 146 tests and cleared the former 140-column failure, but
multiple WezTerm tests passed `WezTermHarness::available()` and then failed to
attach or spawn a pane with `command timed out after 15s`. The failures reached
configured retries in:

- `level2_wezterm_file_property_uses_choose_one`;
- `level2_wezterm_file_array_property_uses_choose_many`;
- the WezTerm operation-file and sequence-YAML chooser cases; and
- the WezTerm dry-run hyperlink capture.

The run was stopped at the non-interactive command ceiling after 44 passes,
with 102 tests unexecuted. A focused run of
`level2_wezterm_file_property_uses_choose_one` reproduced the same timeout on
two complete attempts before the third attempt was stopped. This is not a
feature-output assertion failure, but it is still a gate defect: the testing
contract defines `available()` as runtime reachability, and an unusable backend
must skip cleanly rather than enter repeated 25-second failures.

The full Level-1 area run also exceeded the command ceiling after the catalog
crate passed 21 tests and the library passed 844 of 3,829 selected tests with no
observed failure. `just lint-transport` is green, but a complete `just lint`
result was not produced. Acceptance Criterion 9 requires completed green
`just test`, `just test-l2`, and `just lint` runs, not partial runs without an
observed product failure.

**Required change:** make WezTerm reachability and spawn behavior agree under
the canonical suite—skip when the remote-control endpoint cannot create a
pane, or serialize/resource-group the WezTerm cases if concurrent owned panes
are not reliable. Then record complete green `just test`, `just test-l2`, and
`just lint` runs from the Claudine package area.

### 2. Medium — The terminal-proxy characterization still documents the removed resolver divergence

`cli/tests/characterization_error_routes.rs:247-254` still says the terminal
route uses `harness::resolve_harness_path`, succeeds on a missing target, and
fails later during pre-flight. Its assertion message repeats that explanation.
The current implementation and
`level2_proxy_routes_share_identity_across_routes_in_tmux` establish the
opposite: terminal recovery now uses the same existence-checking proxy resolver
as `initialize`, and both routes fail at resolution with
`composition.invalid_file_reference`.

The event-order assertion can remain—the terminal route still runs its
`start`/`failure`/`finalize` lifecycle—but its rationale is stale. This is the
kind of behavior-changing comment drift the repository's authoring discipline
requires fixing in the same change.

**Required change:** rewrite the characterization documentation and assertion
message around the current resolution-time failure while retaining the valid
event-order and exactly-once assertions.

## Requirement Verification Levels

| Requirement | Strongest verification observed | Assessment |
|---|---|---|
| Preserve typed errors or a versioned snapshot across in-process and erased boundaries | Level 1 Rust-aware source guard, snapshot-selection tests, and record tests | Appropriate and green in focused runs. The guard now detects snapshot-facet re-erasure; 30 retained exceptions and the single ratified D8 category-4 exception remain live and structurally validated. |
| Discover every Claudine diagnostic and select one effective diagnostic | Level 1 registry parity, runtime downcast, cycle/depth, source-chain, and error-walker tests | Appropriate. `RestoredDiagnostic` is registered, discoverable, selectable, and fixed-point tested. |
| Make rendering, lifecycle `err.*`, and machine projection use the same diagnostic | Level 1 error-walker and `DiagnosticSnapshot` projection assertions | Appropriate for the facet/selection invariant. Focused tests verify code, category, disposition, origin, detail, message, and one-level cause. |
| Preserve snapshots in reporting, loop, sequence, MCP import, and prep/recovery records | Level 1 serialization and orchestration tests | Appropriate for data transport. Focused snapshot/reporting tests passed 54 of 54. |
| Restore the prep-time `--repo` failure without re-erasing its identity | Level 1 typed selection and component-render assertion | Appropriate for the restoration invariant; the test verifies the selected facets and rendered block rather than message substrings alone. |
| Render the motivating initialize proxy failure as a structured block | Level 1 process tests plus Level 2 tmux capture | Appropriate and green in the focused Level-2 run. |
| Give initialize and terminal proxy routes the same identity, headline, hint, and available detail | Level 2 tmux capture, supported by Level 1 resolver/diagnostic tests | Appropriate and green. The promoted parity test now asserts the shared resolution-time surface instead of the old divergence. |
| Preserve frontmatter excerpts, color/no-color behavior, exit codes, event ordering, and exactly-once emission | Level 1 characterization plus representative Level 2 tmux captures | Appropriate for the exercised routes. The terminal-proxy characterization explanation is stale even though its event-order assertions remain valid. |
| Pass all required Claudine gates | Focused Level 1/Level 2 runs and attempted full area runs | **Gap.** The full Level-2 gate encounters WezTerm spawn timeouts and does not complete; full Level-1 and lint results are also incomplete. |

Level 3 is not applicable. This feature does not assert physical-key, terminal
input-encoder, paste, IME, mouse, or hotkey behavior. Level 2 is the correct
maximum tier for its real-terminal component rendering, SGR, OSC 8, wrapping,
and plain-output requirements.

## Verification Performed

- `just lint-transport` passed all 18 structural, registry, catalog, boxed-source,
  and lossy-boundary guards.
- Focused library snapshot, restoration, reporting, ingest, and carried-record
  checks passed 54 of 54 tests.
- Focused CLI error-walker, `--repo` restoration, and loop characterization
  checks passed 18 of 18 tests.
- The focused Level-2 proxy-parity and 140-column context cases passed 2 of 2
  through the canonical `just test-l2` recipe.
- The complete Level-2 attempt passed 44 tests before repeated WezTerm
  attach/spawn timeouts and the non-interactive ceiling stopped the run; 102
  tests were not executed.
- A focused WezTerm chooser run reproduced two complete spawn-timeout failures
  before its third retry was stopped at the same ceiling.
- The complete Level-1 attempt passed 21 catalog tests and 844 Claudine library
  tests without an observed assertion failure before the ceiling; the remainder
  of the area gate did not run.
- Preserved the caller's unrelated modifications to
  `.claudine/memory/commits.md` and `CLAUDE.md`.

## Production Readiness Closure

The propagation architecture itself has no remaining blocker found in this
iteration. Production readiness requires a reliable, completed Level-2 gate,
complete green Level-1 and lint gates, and correction of the stale proxy-route
characterization prose. Until those closure conditions are demonstrated, the
feature remains not ready.

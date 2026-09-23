---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-07-20T20:25:16-07:00
spec: 2026-07-13-error-propogation/spec.md
implemented: false
description: A **feature** review of `2026-07-13-error-propogation/spec.md`
feature: 2026-07-13-error-propogation/review-9.md
previous: 2026-07-13-error-propogation/review-8.md
---

# Review 9: End-to-End Typed Error Propagation

## Verdict

The feature is **not ready for production**. Review 8's two implementation
defects are closed, and this review found no remaining functional defect or
verification-level mismatch in the feature-specific behavior. The automatic
TTY case now asserts tmux's supported Markdown-link fallback, a reachable
unforced WezTerm pane proves automatic OSC 8 selection, and the proxy-route
event assertions tolerate legitimate terminal wrapping. All of those cases
passed in real terminals.

Acceptance Criterion 9 is still not fully evidenced, however. `just lint`
completed successfully, but the canonical `just test-l2` run had to be
interrupted after 147/148 passes when an unrelated idle-flush test entered its
documented approximately 80-second wait, and `just test` exceeded the
non-interactive execution ceiling after 913 observed library passes. Neither
interruption is a product failure, but an interrupted gate is not a passing
gate.

## Findings

### 1. High — Acceptance Criterion 9 still lacks complete `just test` and `just test-l2` passes

The feature-specific closure evidence is green:

- `level2_initialize_proxy_block_auto_detects_tty_color_in_tmux` passed and now
  rejects OSC 8 while requiring the visible Markdown-link fallback.
- `level2_initialize_proxy_block_auto_detects_osc8_in_wezterm` passed in an
  unforced, reachable WezTerm pane after proving `NO_COLOR`, `FORCE_COLOR`, and
  `CLICOLOR_FORCE` were absent.
- `level2_proxy_routes_share_identity_across_routes_in_tmux` passed with
  wrap-tolerant event-label assertions.
- All other representative typed-error L2 cases passed: initialize and
  terminal proxy resolution, composition source lookup, schema failure,
  transclusion failure, harness pre-flight denial, plain/no-color rendering,
  and the deliberately unstructured fallback.
- Focused Level-1 suites passed: 126/126 diagnostic tests, 49/49 snapshot tests,
  5/5 effective-render tests, 2/2 public registry/discovery tests, 8/8 route
  characterization tests, and 2/2 live proxy-detail projection tests.
- `just lint` passed, including all 18 Rust-aware error-transport guards, the
  lifecycle documentation guard, and Clippy for the catalog, library, contract,
  CLI, and generator crates.

The canonical runs still did not reach successful summaries:

- `just test-l2` reached 147/148 passes. The only unfinished case was
  `level2_prompt_idle_flush_keeps_the_task_bar_in_tmux`, whose own documentation
  states that approximately 80 seconds is the cheapest honest run because it
  must cross two hardcoded 30-second ticks. Session policy required interruption
  after the process exceeded the non-interactive ceiling.
- `just test` completed the catalog at 21/21 and reached 913 observed library
  passes before interruption. Two slow library tests timed out on their first
  attempts and were in the configured nextest retry path; no final test failure
  had been reported.

The slow unfinished tests are outside this feature's changed behavior, so this
finding does not identify a new error-propagation defect. It remains a release
blocker because Acceptance Criterion 9 explicitly requires complete passing
summaries, and Review 8 required those summaries rather than partial evidence.

**Required change:** run `just test` and `just test-l2` to completion in CI or
another non-interactive environment whose command budget accommodates the
known slow tests, and record their exact successful summaries. No further
feature implementation change is indicated unless either gate reports a final
failure.

## Requirement Verification Levels

| Requirement | Strongest verification observed | Assessment |
|---|---|---|
| Preserve typed causes in-process and use versioned snapshots at erased/process/persistence boundaries | Level 1 typed-chain, selection, restoration, serialization, and Rust-aware transport guards | Appropriate; focused diagnostic and snapshot suites and all 18 guards passed. |
| Discover every Claudine diagnostic through one registry | Level 1 runtime downcast tests plus Rust-aware implementation/registry parity | Appropriate; both public registry tests and the parity guard passed. |
| Select the same diagnostic for rendering, `err.*`, and serialized output | Level 1 selection/snapshot/lifecycle tests plus Level 2 representative rendering | Appropriate; selection, cause, snapshot, and effective-render suites passed. |
| Carry source document, lifecycle event, and authored property through both proxy routes | Level 1 live-route structured assertions plus Level 2 tmux capture | Appropriate; both projection tests and the route-parity capture passed. |
| Render initialize and terminal/recovery proxy misses with identical code, headline, hint, and available typed detail | Level 2 tmux route-parity capture | Appropriate; the route-parity case passed after making only the layout-sensitive assertion wrap-tolerant. |
| Render composition lookup, schema/file-reference, transclusion, and harness pre-flight failures as component blocks; retain a generic fallback for genuinely unstructured errors | Level 2 tmux captures | Appropriate; every representative case passed. |
| Preserve frontmatter/source excerpts and actionable corrective context | Level 1 enrichment tests plus Level 2 invalid-reference and typed-error captures | Appropriate; source-aware content survives the real-terminal path. |
| Preserve forced color, `NO_COLOR`, and plain/piped output contracts | Level 2 forced-color and `NO_COLOR` tmux captures plus Level 1 spawned-process piped-stderr test | Appropriate; terminal-dependent styling is L2, while the non-TTY pipe branch is correctly verified with a real spawned process. |
| Automatically enable TTY color without forcing | Level 2 unforced tmux capture with all forcing variables proven absent | Appropriate; the red SGR assertion passed. |
| Automatically select OSC 8 only in a detected capable terminal | Level 2 unforced tmux negative/fallback capture plus unforced WezTerm positive capture | Appropriate; both capability branches passed in real terminals. |
| Preserve exit codes, lifecycle ordering, retry/proxy decisions, and exactly-once emission | Level 1 route characterization plus Level 2 proxy-route captures | Appropriate for the reviewed routes; all focused characterization and L2 assertions passed. |
| Keep `err.msg` notification-safe and preserve provider-message precedence | Level 1 message hygiene, snapshot, and lifecycle projection tests | Appropriate; no terminal encoder behavior is involved. |
| Keep registered detail catalog-shaped, structured, and present-null where unavailable | Level 1 catalog/corpus/snapshot guards and live proxy-detail assertions | Appropriate; all focused tests and static guards passed. |
| Pass every required Claudine gate | Complete lint pass; interrupted full Level-1 and Level-2 runs | **Gap.** Acceptance Criterion 9 remains unproven despite all feature-specific evidence passing. |

Level 3 is not applicable. This feature concerns error identity, data transport,
process behavior, and terminal output; it does not depend on OS keyboard or
mouse injection, terminal input encoding, paste, IME, or hotkey behavior.

## Ergonomics and Performance Assessment

No additional ergonomic or performance change is recommended. The public
discovery seam, role-based selector, and owned snapshot keep consumers on one
projection path. The selector's bounded chain walk favors clarity; its small
`Vec` identity set is capped at 64 entries and runs only on failure paths, so a
more elaborate allocation or lookup strategy would add complexity without a
meaningful performance benefit.

The new L2 helper generalization is also proportionate: it reuses the
`TerminalHarness` contract across tmux and WezTerm without introducing a new
abstraction into production code. WezTerm scrollback capture uses the harness's
escape-preserving API, clears prior scrollback first, and remains target-gated.

## Verification Performed

- Read the specification, Review 8, the Claudine error-architecture authority,
  the Rust and test-tier guidance, and the WezTerm harness guidance.
- Reviewed the post-Review-8 implementation diff and the final L2 helpers and
  assertions.
- Used GitNexus to inspect the registry, effective selector, snapshot, and
  lifecycle projection relationships on the current worktree.
- Ran the focused Level-1, Level-2, characterization, and transport checks
  summarized above.
- Ran `just lint` to successful completion.
- Attempted `just test-l2` and `just test`; interrupted them only when required
  by the non-interactive command ceiling, preserving their partial summaries.
- Preserved the caller's unrelated changes to `CLAUDE.md` and
  `prompts/_implement/implement-suggestions.md`.

## Production Readiness Closure

No product or test-level implementation gap remains from Review 8. Supply
complete successful `just test` and `just test-l2` summaries from an environment
that can accommodate the known slow tests. Until that explicit acceptance gate
is evidenced, the feature is not production ready.

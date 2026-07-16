---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-16T10:03:06-07:00
spec: 2026-07-15-reference-graph/spec.md
implemented: true
description: "A **feature** review of `2026-07-15-reference-graph/spec.md`"
feature: 2026-07-15-reference-graph/review-3.md
previous: 2026-07-15-reference-graph/review-2.md
---

# Review 3 — Reference Graph

## Verdict

Not ready for production. The reference-graph implementation now satisfies the functional,
compatibility, lifecycle, and performance requirements I reviewed. All four findings from review 2
are closed, and I found no remaining defect in the reference-graph implementation itself.

The required Darkmatter-area Level-1 test gate is nevertheless red on the reviewed revision. The
failure is in an apparently unrelated `darkmatter-cli` terminal-detection test, but AC12 explicitly
requires the full area test gate to pass. Production readiness therefore cannot be asserted until
that failure is resolved and `just test` passes.

## Findings

### High — The required Darkmatter Level-1 test gate is red

Running `just test` from the Darkmatter package area completed the 5,751-test library suite and all
focused reference-graph tests successfully, then failed in the CLI suite at
`darkmatter-cli::compose_terminal_detection compose_redirected_does_not_spawn_appearance_defaults`.
The test failed on all four nextest attempts because redirected `md compose` spawned the macOS
`defaults` appearance probe and created the sentinel that the test asserts must remain absent
(`cli/tests/compose_terminal_detection.rs:128`). Nextest stopped the remaining CLI tests after the
failure, reporting 350 passed, 1 failed, 71 skipped, and 208 not run in that suite.

This test is outside the reference-graph feature and does not invalidate the focused graph results.
It does invalidate AC12's required area-wide Level-1 gate on the production candidate. Fix or
otherwise resolve the terminal-detection regression, rerun `just test`, and require a clean result
before changing `ready` to `true`.

## Review 2 Closure

| Prior finding | Status | Evidence |
|---|---|---|
| Path-valued options used lossy `Path::display()` identity | Closed | `GraphIdentityEncoder::path` now encodes Unix path bytes and Windows wide units with platform tags (`lib/src/markdown/compose/context/options.rs:1457`). Graph and cache identities have Unix tests proving that non-UTF-8 paths with identical display text remain distinct (`options.rs:2362`, `2379`). |
| Performance evidence predated the final fixes | Closed | `results.md:236-270` records a back-to-back, same-session comparison of the final implementation against pre-opacity commit `db7e46792`. No fixture exceeds both the 5% and 100 µs construction-regression limits. `results.md:292-301` also preserves the prebuilt-validation speedup on every fixture. |
| Volatile-context coverage did not build and reuse real graphs | Closed | The interpolated-link and conditional-transclusion tests now construct real graphs, prove that output or membership changes, accept same-context reuse, and reject cross-context reuse through the public validator (`lib/src/markdown/reference/graph.rs:1501`, `1572`). |
| Remote/preflight lifecycle checks did not prove graph non-retention | Closed | Graph-level tests now build and drop real graphs while checking externally observable strong counts and weak drop probes for the remote fetch runtime and preflight graph (`graph.rs:1686`, `1745`). |

## Requirement-to-Verification Assessment

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1–AC3: opaque, immutable, builder-only graph with private construction invariants | Level 1: API/source checks and unit tests around `ReferenceGraph::from_build` | Appropriate and passing. The graph fields are private and construction is crate-private (`lib/src/markdown/reference/types.rs:452`, `601`). |
| AC4: reject incompatible roots, descendants, mode, and options before flattening | Level 1: typed mismatch tests, descendant mutation tests, and validation-order inspection | Appropriate and passing. `verify_graph_compatibility` runs before flattening (`lib/src/markdown/reference/validate.rs:563`). |
| AC5: exhaustive, deterministic option identity without runtime retention | Level 1: identity-classification, boundary, volatile-state, and non-UTF-8 path tests | Appropriate and passing. The lossless path gap from review 2 is closed. |
| AC6: no retained non-clone runtime state | Level 1: graph-construction strong-count and weak-drop-probe tests | Appropriate and passing for shell handlers, remote fetch state, and preflight graphs. |
| AC7: exactly one dependency child identity per graph child | Level 1: transclusion, TOC-link, prologue, and epilogue unit/integration cases | Appropriate and passing. |
| AC8: graph mode is the sole extraction-mode switch | Level 1: mode behavior and provenance mismatch tests | Appropriate and passing. |
| AC9: clone preserves provenance and validation behavior | Level 1: clone equivalence tests | Appropriate and passing. |
| AC10: callers migrate to accessors and borrowed views | Level 1: compile-time API use plus view/accessor tests | Appropriate and passing. |
| AC11: preserve CLI and JSON output | Level 1: serializer, fixture, and spawned-CLI assertions | Appropriate and passing in the focused graph cases. |
| AC12: focused tests plus Darkmatter build, test, lint, and hygiene gates | Level 1 plus build/lint tooling | **Fail.** `just build` passes, but `just test` fails as described above. The lint run was inconclusive because it exceeded the session's 60-second non-interactive command limit without output and was terminated. |
| AC13: preserve reuse win without material construction regression | Criterion benchmark evidence | Appropriate and passing under the specification's two-part budget. |

This feature defines deterministic library and CLI behavior; it does not define terminal rendering,
terminal input encoding, keyboard, paste, IME, mouse, or other OS-input behavior. Level 2 and Level 3
coverage are therefore not required for its user-observable requirements.

## Verification Performed

- Read the complete specification, review 2, implementation, focused tests, and final benchmark
  report.
- Used GitNexus to inspect the graph construction and compatibility-validation flows. Symbol-specific
  upstream impact is LOW for `ReferenceGraph`; the options-classification path is also LOW risk and
  reaches the compose pipeline. The branch-wide comparison against `main` is noisy because this
  long-lived branch contains hundreds of unrelated changes, so it was not used as feature-specific
  risk evidence.
- Used `sniff` to establish the Darkmatter package-area scope: `darkmatter`, `darkmatter-cli`, and
  `dmls`.
- `just build`: pass for all three scoped packages on macOS.
- `just test`: fail in the unrelated CLI test documented above; all reference-graph tests observed
  before that failure passed.
- `just lint`: inconclusive; terminated after the non-interactive 60-second limit with no output.
- `bf` parsed all requested frontmatter values successfully. `md schema validate` accepted the spec,
  but could not validate either review because the repository's existing
  `schemas/feature-review.yaml` uses unsupported standalone-schema keys (`description` and
  `$schema`).
- Verification was performed on macOS. Cross-platform path encoding was reviewed in source; Windows
  and Linux execution was not available in this session.

The prompt's `@prompts/./_reviews/.../review-2.md` reference does not resolve in this worktree. The
canonical previous review is the feature-local `review-2.md`, so its frontmatter was updated there.

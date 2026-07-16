---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-16T09:36:53-07:00
spec: 2026-07-15-performance-followup/spec.md
implemented: false
description: "A **feature** review of `2026-07-15-performance-followup/spec.md`"
feature: 2026-07-15-performance-followup/review-1.md
---

# Review 1 — Performance Follow-up

## Implementation status (2026-07-16)

Seven of the nine findings are closed. **Two remain open and cannot be closed by
implementation** — they need an owner decision and a Windows host respectively,
so `implemented` stays `false`.

| Finding | Status |
|---|---|
| High — F21 redirected-output test is red | **Closed.** Already fixed by `74e0fdc90` before this pass; verified green. |
| High — F2 has no Level-2 proof | **Closed.** Genuine L2 test added under WezTerm; the manufactured-PTY test reclassified as L1. |
| High — nine checkout-hostile fixture paths | **Closed.** Removed from the index and working tree; `results.md` provenance corrected. |
| High — raw benchmark samples not retained | **Closed with corrections.** 1,950 real observations retained; F33's headline corrected. |
| High — Windows behavior open / F17 tests not Windows-ready | **Partially closed.** Tests are now Windows-*ready* (Unix-utility deps removed) and Linux-verified on a real kernel. Windows **behavioral** run remains **OPEN** — no Windows host reachable. |
| High — F32 unapproved prompt-frequency change | **Closed.** Prior prompt behavior restored; clone removal retained. |
| High — F35.5 computes the same artifact twice | **Closed.** Also fixed an unreported `--save` defect (three artifacts, not two). |
| High — integrated compose regression gate fails | **OPEN — owner decision required.** Re-measured under a drift bracket in a quiet window: the regressions are **real**, not measurement noise. |
| Medium — F17 no-poll timing test ineffective | **Closed.** Replaced with a deterministic wait seam; mutation-tested. |

### What the owner must decide

1. **The compose regression gate (High).** A bracketed re-measurement at the
   current head confirms `compose_trivial` +34.0 %, `compose_schema_transclusion`
   +27.4 %, `compose_interpolation_heavy` +11.0 %, `compose_transclusion_heavy`
   +14.4 %, against a drift floor of ≤5.9 % and a flat `render_basic` control.
   The "it's noise" explanation is closed off. The cost is graph/identity
   construction on the **setup** path (Command Setup 5.3 → 8.5 ms), not the
   descendant re-read. Options: fix the ReferenceGraph setup cost, re-threshold
   with a recorded compatibility decision, or keep the feature blocked. No
   production code was changed to chase the benchmark.
   Evidence: `benchmarks/raw/f-cumulative-closeout/run-20260716T230028/`.
2. **Windows behavioral evidence (High).** Requires a Windows host. The F17 and
   F22 suites are now portable by construction and cross-compile clean, but
   "compiles" is not "behaves" and this review is right to insist on the
   distinction.

### Claims downgraded during this pass

Two previously reported measurements did not survive re-measurement and are
corrected in `results.md`:

- **F33's control regressions** (−19.3 % / −19.1 %) were **cross-run drift on a
  loaded host** and sit at parity (+0.1 % / +0.7 %) when re-measured quietly.
  The target win is **−77.5 %**, not −82.5 %. The disposition still passes its
  ≥30 % floor, on better evidence.
- **F2's "Verified (L2) on macOS and real Linux"** was **wrong** — that test was
  L1, exactly as this review found. Real L2 now exists on macOS/WezTerm only;
  L2 on Linux remains open.

Additionally, **F35.5/35.6/35.7, F23, and F25** rest on temporary harnesses that
were deleted after capture; their raw observations are unrecoverable and those
claims should be treated as **unverified** pending a rebuilt harness.

## Verdict

Not ready for production. Several of the implementation changes are sound and the focused
Darkmatter regressions passed, but the feature does not satisfy its release contract. One committed
Finding 21 regression test currently fails, Finding 2's claimed Level-2 proof is only a Level-1
pseudo-TTY test under the specification's required taxonomy, Windows behavior remains unverified,
and the retained benchmark artifacts are insufficient to reproduce the Criterion conclusions.
The closeout also ships Windows-incompatible tracked paths, leaves Finding 35.5 incomplete, records
an unapproved user-visible prompt change, and reports material integrated compose regressions.

## Findings

### High — Finding 21's redirected-output regression test is red

The focused `darkmatter-cli` nextest run failed
`compose_redirected_does_not_spawn_appearance_defaults` on all four nextest attempts. The sentinel
created by the `defaults` shim proves that redirected `md compose -vv --perf` invoked some
`defaults` command, contradicting the feature's claimed no-fork proof
(`darkmatter/cli/tests/compose_terminal_detection.rs:76-133`). Sixteen other tests in the same
focused run passed.

The current oracle records every `defaults` invocation, while its failure message attributes any
invocation specifically to `defaults read -g AppleInterfaceStyle`. That does not distinguish the
appearance probe from other macOS terminal-discovery probes. Instrument the shim to retain its
arguments, constrain the assertion to the appearance command, and then fix whichever non-TTY call
path the narrower test identifies. The appropriate verification level for this redirected child
process behavior is Level 1, but the release proof must be unambiguous and green.

### High — Finding 2 has no Level-2 terminal-emulator proof

`level2_terminal_osc_cache.rs` calls its tests Level 2 because they use `expectrl`, but the helper
spawns the probe in a pseudo-terminal and manufactures OSC 10 and OSC 11 response bytes
(`biscuit-terminal/lib/tests/level2_terminal_osc_cache.rs:1-15,65-115,123-176` and
`biscuit-terminal/lib/tests/common/pty.rs:1-5,147-215`). Under the review taxonomy, this is Level 1:
the test, rather than WezTerm, Kitty, or tmux, owns the response bytes. Renaming the test and running
it under a Linux kernel do not elevate its verification level.

The test is useful Level-1 coverage of the cache and request count, but it cannot satisfy the
specification's explicit Level-2 gate. Add an actual terminal-emulator or multiplexer test that
runs the probe in WezTerm, Kitty, or tmux and captures the real session. Keep the manufactured PTY
test as the fast Level-1 regression.

### High — The feature commits nine checkout-hostile absolute-path fixture duplicates

The index contains nine paths whose first component is `benchmarks` followed by a literal newline
and an embedded absolute macOS path. They duplicate the intended fixture names. These entries were
added by commit `093ea3dfb`; they are not untracked artifacts as `results.md` claims. Control
characters and the embedded host path make this repository state non-portable, including invalid
checkout names on Windows, directly violating the cross-platform acceptance criterion.

Remove the nine malformed tracked entries without deleting the intended files under
`benchmarks/fixtures`, and correct the closeout's provenance statement. Verify a clean clone or
checkout on Windows before release.

### High — The required raw benchmark samples were not retained

The specification requires raw samples to be retained so reported means, confidence intervals,
and regressions can be independently recomputed. The retained `criterion-*.json` artifacts contain
only derived fields such as `mean`, `median`, `median_abs_dev`, `slope`, and `std_dev`; no Criterion
sample vectors are present. The temporary harness profiles likewise retain summaries rather than
the individual observations. Hyperfine's recorded `times` arrays do not repair the missing
Criterion and temporary-harness evidence.

This leaves the measurement claims for Findings 13, 14, 23, 33, and 35, including Findings
35.5–35.7, below the specification's reproducibility contract. Retain the matching Criterion
`new/sample.json` data and the raw observations from every temporary harness, record the exact
commands and tool versions, and regenerate the summaries from those retained inputs.

### High — Windows behavior is open, and Finding 17's tests are not Windows-ready

The specification explicitly requires real non-macOS behavioral runs for Finding 17's wait
primitive and Finding 22's directory traversal. `results.md` acknowledges that Windows behavior
for both remains open; a Windows cross-compile is not a behavioral run.

The Finding 17 tests also invoke Unix utilities by name: `true` in the pipeline and fast-completion
tests, `sleep` in the timeout test, and `pgrep` in a Unix-only assertion
(`darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs:1238-1267,1310-1331,1334-1402`).
Several saturation tests silently return when Python is unavailable. Replace these dependencies
with a platform-neutral helper executable or test-binary child modes, make required prerequisites
fail or use the repository's explicit level-gating mechanism rather than silently passing, then run
the Finding 17 and Finding 22 behavior suites on a real Windows host.

### High — Finding 32 ships an unapproved user-visible prompt-frequency change

The closeout explicitly records that a rule persisted midway through a shell-expansion stage no
longer affects later directives in that stage, so a multi-directive flow may prompt twice where it
previously prompted once. That is a deliberate user-observable compatibility change still awaiting
owner acceptance, not an implementation detail that can be closed by the policy snapshot tests.

Obtain and record the required compatibility approval, or preserve the prior prompt behavior while
removing the repeated rule-set clones. Level 1 is sufficient to verify this state-machine behavior;
Level 3 is not required because the requirement does not concern the terminal's keyboard encoder.

### High — Finding 35.5 still computes the same hash artifact twice

`run_hash_diff` first calls `compare_hash` and then `explain_hash_diff`
(`darkmatter/cli/src/commands/hash.rs:141-158`). Each public operation computes the same effective
hash artifact (`darkmatter/lib/src/markdown/hash/compare.rs:91-98` and
`darkmatter/lib/src/markdown/hash/explain.rs:449-462`). The closeout acknowledges this residual, so
the requirement that each unique `(kind, effective options)` artifact be computed at most once per
operation is not implemented.

Provide a library operation that returns the comparison and explanation from one computed artifact,
or obtain an explicit scope/compatibility decision changing the requirement. A benchmark showing
that the duplicate is small does not satisfy the specified structural bound.

### High — The integrated compose regression gate does not pass

The final comparison reports regressions of 34.8% for `compose_trivial`, 23.1% for
`compose_schema_transclusion`, 18.3% for `compose_interpolation_heavy`, and 14.0% for
`compose_transclusion_heavy`. `results.md` attributes these to coordinated ReferenceGraph changes,
but this feature's dependency contract requires consuming those commits. Production readiness is a
property of the integrated feature head; an ownership boundary does not make a failed release gate
pass.

Resolve the integrated regressions, revise the predeclared thresholds through an explicit owner
decision with supporting raw evidence, or keep the feature blocked on the linked ReferenceGraph
work. Do not close the performance feature while its own final matrix reports these regressions.

### Medium — Finding 17's no-poll timing test cannot catch the stated regression

`fast_command_completion_is_not_delayed_by_a_poll_interval` says ten retired 10 ms poll delays can
add up to 100 ms, but asserts only that the run finishes within 500 ms
(`darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs:1310-1331`). The retired behavior
therefore fits comfortably inside the passing bound. Prefer a deterministic wait seam or
instrumented primitive; if elapsed time must be used, establish a measured platform allowance that
actually separates the old and new implementations.

## Requirement-to-verification assessment

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| F1 timezone compatibility and Darkmatter no-network seam | Level 1 unit/seam tests | Appropriate; focused selection passed |
| F2 OSC cache/request count and repeated construction | Level 1 pseudo-TTY with manufactured OSC replies | Wrong level; the specification requires Level 2 |
| F3 one terminal detection per CLI invocation | Level 1 spawned-binary counter test | Appropriate; focused CLI test passed |
| F21 redirected no-appearance fork | Level 1 spawned-binary shim test | Appropriate level, but currently failing and the oracle is overbroad |
| F4, F7, F11–F14, F16, F33, and F35 internal performance mechanisms | Level 1 unit/property/snapshot tests plus benchmark summaries | Appropriate test level for internal behavior, but raw performance evidence is incomplete |
| F17 wait, stream, timeout, and failure semantics | Level 1 in-process subprocess tests | Appropriate level, but Windows behavior is absent and the no-poll guard is ineffective |
| F22 directory membership and aggregate hash behavior | Level 1 unit and CLI tests | Appropriate level, but the required Windows behavioral run is absent |
| F23 code-theme terminal/browser presentation | Level 1 snapshot/unit tests, browser computed-style checks, and existing Level-2 real-terminal capture | Appropriate; no input-encoder behavior is specified |
| F32 prompt-frequency compatibility | Level 1 policy/state tests | Appropriate level, but they verify an unapproved UX change |
| F25 and measured no-win dispositions | Profile/benchmark summaries | Evidence gap; required raw observations are absent |

No requirement in this feature depends on physical keyboard, paste, IME, or mouse encoding, so
Level 3 is not applicable.

## Verification performed for this review

- Scoped Darkmatter nextest selection: 26 passed. This included graph-cycle and overlap cases,
  shell stream/timeout behavior, policy snapshot behavior, interpolation lookup, identity/cache
  tests, manifest checks, and transclusion cases.
- Focused `darkmatter-cli` nextest selection: 16 passed and 1 failed after all four configured
  attempts. All 15 directory-hash tests and the single-detection test passed; the redirected
  `defaults` test failed.
- Source and repository-index inspection confirmed the pseudo-TTY classification, duplicate hash
  computation, non-portable test commands, missing raw sample vectors, and nine malformed tracked
  fixture paths.
- `md schema validate` could not validate this review because the repository's
  `schemas/feature-review.yaml` is itself rejected as a standalone SimplifiedSchema: tagged schema
  documents currently reject its `$schema` and `description` keys.
- GitNexus impact analysis classified `apply_replacements` and `interpolate_frontmatter` as
  critical-blast-radius paths and `discover_remote_urls_from_expressions` as critical across three
  execution flows. The focused verification above exercises representative regressions, but the
  unresolved release gates prevent a production-ready verdict.

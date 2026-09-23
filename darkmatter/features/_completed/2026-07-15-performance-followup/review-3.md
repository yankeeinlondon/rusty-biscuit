---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-16T20:44:02-07:00
spec: 2026-07-15-performance-followup/spec.md
implemented: true
description: "A **feature** review of `2026-07-15-performance-followup/spec.md`"
feature: 2026-07-15-performance-followup/review-3.md
previous: 2026-07-15-performance-followup/review-2.md
next: 2026-07-15-performance-followup/review-4.md
---

# Review 3 — Performance Follow-up

## Verdict

This feature is **not ready for production**. The developer has closed the
previous review's Windows-host, retained-raw-sample, and theme-independent
real-terminal evidence gaps. The current WezTerm Level-2 proof is especially
strong: it configures a known foreground, observes that exact response, and
proves that repeated `Terminal` construction issues one OSC 10 query.

Three release blockers remain. The shell-reservation remediation does not have
a stable green Level-1 gate, two new public hash methods still violate the
explicit compatibility invariant, and the integrated compose regression has no
accepted reproducible disposition. GitNexus rates `prepare_directive` as
**CRITICAL** impact: 44 upstream symbols, three direct callers, two execution
processes (`run_stage` and `run_compose_pipeline_internal`), and five affected
modules. The narrower option-cache and OSC-query changes are low-risk.

## Findings

### High — Reservation cleanup is not verified by a stable Level-1 gate

The RAII `ReservationGuard` is the right implementation shape, but all five new
cleanup tests use a wall-clock assertion to infer whether a reservation was
released. `assert_command_is_still_approvable` fails whenever a normal second
`prepare_directive` takes five seconds, even though the reservation timeout is
30 seconds and the call successfully returns an approved directive.

The canonical `darkmatter/just test` run made the problem concrete:

- `whitelist_exact_write_failure_releases_the_rest_of_the_chain` and
  `blacklist_exact_write_failure_releases_the_rest_of_the_chain` failed all four
  attempts at 5.29–5.85 seconds for the measured second composition;
- the approval-handler, whitelist-prefix, and waiter-notification cases each
  failed once before passing on retry; and
- fail-fast stopped the area gate after 2,509 of 5,768 tests, leaving 3,259
  unrun.

This is the correct verification level for F32, but the oracle is not reliable:
it conflates unrelated preparation cost with a leaked reservation. Verify the
runtime's reservation state or use a synchronized waiter notification with a
timeout comfortably separated from normal preparation latency. The CRITICAL
compose path cannot be accepted while its regression suite is red and flaky.

### High — The no-new-public-API invariant remains knowingly violated

Compatibility invariant 2 permits no new public Rust API shape. The current
implementation still exposes `Markdown::diff_hash` at
`darkmatter/lib/src/markdown/hash/explain.rs:495` and
`Markdown::plan_hash_save_explained` at
`darkmatter/lib/src/markdown/hash/save.rs:88`. `results.md` explicitly records
this as an open review-2 finding requiring either a non-public seam or an owner
ruling; neither is present.

The OSC proof's public feature and query-counter API from review 2 have been
successfully removed in favor of a crate-private tracing event. The two hash
methods remain a release blocker even though their shared-artifact optimization
and Level-1 behavior tests are otherwise sound.

### High — The integrated compose regression has no reproducible accepted closeout

The new option-baseline cache and reference-manifest changes substantially
improve the previous +11–34% integrated regression, but the retained candidate
runs disagree on the release-critical `compose_trivial` case:

- `run-20260717T033000-trivial-conservative` reports **+4.91%**
  (**+0.543 ms**) against the audit base, outside its 1.13% identical-code drift;
  while
- `run-20260717T033745` reports **+0.76%** (**+0.091 ms**) against the audit
  base, with 0.36% identical-code drift.

The harness points readers to a `summary.md` for the detached-worktree pins and
build commands, but no such summary exists in the retained run directory. The
feature's `results.md` still says the compose regression is open and owned by
the Opaque Reference Graph feature; it does not mention or accept either new
run. Consequently there is no auditable choice of run, pin manifest,
recomputation command, threshold decision, or owner disposition. Acceptance
criteria 5 and 6 remain unmet. Retain one quiet-host bracketed run with all
required metadata, explain the rejected run, and record either a passing gate
or an explicit owner-approved re-threshold.

### Medium — Closeout documentation is no longer a trustworthy source of truth

The implementation and evidence now close several review-2 findings, but the
final audit still lists the old integrated regression and public hash API as
open while the spec's final table presents the feature as closed. The new
reference-graph setup remediation and its two runs are absent from
`results.md`. The run harness also references a missing `summary.md`.

Reconcile the spec audit, results, evidence index, and owner decisions after the
three blockers above are resolved. Until then acceptance criterion 7 is not
met.

### Medium — The new OSC latency regression test is load-sensitive

`biscuit-terminal/just test` ran all 2,770 tests. The feature's
`terminal_repeated_construction_latency` failed its first attempt and passed its
retry, so nextest classified it as flaky. The exact one-query functional test
and both retained WezTerm Level-2 tests passed, so this does not invalidate the
OSC cache behavior. It does mean the performance smoke test is not yet a stable
gate under parallel area-test load. Isolate the benchmark-like latency test or
replace its host-time threshold with an invariant that is robust under
contention.

The same area run ultimately failed an unrelated `Table__baseline` snapshot;
that pre-existing rendering drift is recorded here for gate completeness but is
not attributed to this feature.

## Requirement-to-verification assessment

| User-observable requirement | Strongest verification present | Assessment |
|---|---|---|
| F2: repeated terminal construction reuses one OSC 10 result | Level 1 manufactured PTY cross-checks request bytes against the internal event count; Level 2 real WezTerm on macOS and retained real Kitty on Linux | Appropriate. The Level-2 test pins and restores a known foreground and observes exactly one real-emulator response. No Level 3 is needed. |
| F3: one `md compose` invocation performs one terminal detection | Level 1 spawned CLI with debug-event count | Appropriate for process-local detection; no terminal rendering or input-encoder claim is involved. |
| F21: redirected compose avoids macOS appearance discovery | Level 1 spawned CLI with a PATH sentinel; F2 supplies the interactive Level-2 counterpart | Appropriate for the no-spawn assertion. |
| F17: fast completion, saturation, timeout, kill/reap, and failure selection | Level 1 real child-process tests on macOS and Linux plus retained Windows-host execution | Appropriate. Windows behavioral evidence now covers the OS-divergent implementation. |
| F22: directory membership, aggregate hash, and exit status | Level 1 library and spawned-CLI tests on macOS and Linux plus retained Windows-host execution | Appropriate. No real-terminal behavior is asserted. |
| F23: theme remains dynamic between renders and output remains stable | Level 1 unit/snapshot coverage, headless-browser rendering, and existing Level-2 terminal coverage | Appropriate for browser and terminal output. Retained raw performance vectors are now present. |
| F32: approval behavior remains compatible under concurrent and failing compositions | Level 1 unit/concurrency tests | **Gap.** The level is correct, but the new error-path cleanup tests are red/flaky and their timing oracle is inconclusive. |
| F35.5: hash explanation, persisted hash, and CLI exit behavior remain compatible | Level 1 library and spawned-CLI tests | Appropriate behavioral level; the separate public-API compatibility invariant still fails. |

No requirement in this feature depends on terminal keyboard encoding, paste,
IME, mouse input, or OS-injected key events. Level 3 is therefore not required.

## Prior-review closure assessment

- **Closed:** Windows-host F17/F22 behavior; F2's deterministic macOS and Linux
  Level-2 evidence; removal of the public OSC instrumentation surface; raw
  vectors for F23, F25, F35.3, F35.5, F35.6, and F35.7; and the apparent
  reservation-cleanup implementation defect through the RAII guard.
- **Not closed:** the two public hash APIs, a stable conclusive reservation
  cleanup test gate, the integrated regression disposition, and consistent
  closeout documentation.

## Verification performed for this review

- `darkmatter/just test`: **failed** after 2,507 passes, three flaky cleanup
  tests, and two cleanup-test failures; 3,259 tests were not run due to
  fail-fast;
- `darkmatter/just lint`: passed for `darkmatter`, `darkmatter-cli`, and `dmls`;
- `biscuit-terminal/just test`: 2,769 passed, the new OSC latency test was flaky,
  and one unrelated table snapshot failed;
- `biscuit-terminal/just lint`: passed for the library and CLI;
- `biscuit-terminal/just test-l2`: both feature-relevant WezTerm tests passed;
  the subsequent broader CLI tier stopped on an environment-permission failure
  while attempting to start another WezTerm mux server, leaving 70 unrelated
  CLI Level-2 tests unrun;
- both new reference-graph setup reports were recomputed from 96 retained
  observations per arm and produced the conflicting `compose_trivial` results
  described above; and
- GitNexus impact/change analysis and Sniff package/package-area/dependency
  discovery established the affected Darkmatter and Biscuit Terminal scope.

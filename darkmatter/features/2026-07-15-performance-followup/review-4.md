---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-17T06:28:19-07:00
spec: 2026-07-15-performance-followup/spec.md
implemented: false
description: "A **feature** review of `2026-07-15-performance-followup/spec.md`"
feature: 2026-07-15-performance-followup/review-4.md
previous: 2026-07-15-performance-followup/review-3.md
next: 2026-07-15-performance-followup/review-5.md
---

# Review 4 — Performance Follow-up

## Verdict

This feature is **not ready for production**. Review 3's flaky reservation-set
oracles, OSC wall-clock assertion, and evidence-documentation gaps have been
materially improved or closed. The current Darkmatter Level-1 gate passes, the
OSC test now asserts a deterministic one-query invariant, and the reference-graph
run record honestly explains why it cannot establish the release threshold.

Three release blockers remain. The integrated compose threshold is explicitly
not established, the feature-gated hash seam is still public Rust API when
enabled, and the waiter-notification test can pass without the waiter ever
parking. GitNexus rates the changed hash CLI orchestration as low risk (one
direct caller and two affected CLI processes for each entry point), but that
does not waive the specification's compatibility or evidence requirements.

## Findings

### High — The required integrated compose threshold remains unresolved

Acceptance criterion 5 requires reproducible same-byte benchmark artifacts to
meet their predeclared thresholds. The accepted run says the opposite at
`benchmarks/raw/f-refgraph-setup-fix/run-20260717T033745/summary.md:15`:
**“Threshold NOT ESTABLISHED — neither pass nor fail.”** Its measured
`compose_trivial` point estimate is +0.76%, but the same audit binary measured
11.049 ms and 11.957 ms in runs one minute apart, an 8.2% identical-code drift
that exceeds the 5% release gate. The run also records host load of 5.42–7.16
and reconstructs the `after` pin from a working tree sampled 33 minutes before
the corresponding commit existed.

The evidence is now candid and auditable, which closes review 3's documentation
finding, but it cannot support a passing verdict. Capture the already-specified
two admissible quiet-host runs from committed pins and apply the predeclared
gate. Until that produces a reproducible pass, acceptance criteria 5 and 6 are
unmet. This host-dependent measurement is handed off to
[`performance-compliance.md`](./performance-compliance.md); future attempts and
results belong there rather than in the next implementation review.

### High — The supposedly internal hash seam is externally callable Rust API

Review 3 required the two orchestration methods to move behind a non-public
seam. Their underlying `Markdown` methods are now `pub(crate)`, but
`darkmatter/lib/src/lib.rs:89` exposes `pub mod internal` whenever the public
Cargo feature `internal-hash-orchestration` is enabled, and
`darkmatter/lib/src/internal.rs:60` and `:82` expose public functions from that
module. `#[doc(hidden)]` only hides generated documentation; it does not change
visibility. A downstream crate can enable the feature and call
`darkmatter::internal::diff_hash` or
`darkmatter::internal::plan_hash_save_explained`.

Consequently the remediation changes public Rust API shape under a non-default
feature and does not satisfy compatibility invariant 2 as written. The docs'
claim that this is “not public API” is a policy statement, not compiler-enforced
privacy. Rust has no friend-crate visibility, so resolve the design tension
explicitly: restructure the crate boundary, remove the shared seam, or obtain
and record an owner-approved compatibility exception. Defining the invariant as
“default-feature API only” after implementation would also require an explicit
owner decision rather than a documentation assertion.

### High — The notification regression test can pass without testing notification

`handler_error_notifies_a_waiter_blocked_on_the_same_command` claims that its
waiter is provably parked before the approval handler errors, but its only
synchronization proves that the approver entered the handler. The handler then
sleeps for 200 ms and the main thread starts the waiter. There is no barrier or
test hook confirming that the waiter reached `reserve_allow_once` before the
approver releases its reservation.

The comment at
`darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs:3081` acknowledges
the false-positive path: a late waiter still passes by reserving the command
itself and the test “would just stop exercising the notification.” That directly
contradicts the test's stated proof. The implementation may notify correctly,
but this Level-1 oracle does not establish it and can silently lose coverage
under scheduler or host-load variation. Add deterministic synchronization at
the wait-entry point or another observable reservation-wait hook, then prove
release wakes an already parked peer. Because F32 covers user-visible approval
progress and deadlock avoidance, inconclusive Level-1 concurrency evidence is a
production readiness gap.

## Requirement-to-verification assessment

| User-observable requirement | Strongest verification present | Assessment |
|---|---|---|
| F2: repeated terminal construction reuses one OSC 10 result | Level 1 manufactured PTY asserts exactly one OSC 10 request across 53 constructions and cross-checks the internal event count; retained Level 2 real WezTerm on macOS and real Kitty on Linux | Appropriate. The former wall-clock test is now deterministic. Level 2 exercises the real emulator's query/response path; Level 3 is not needed. |
| F3: one `md compose` invocation performs one terminal detection | Level 1 spawned CLI with debug-event count | Appropriate. This is process-local behavior, not terminal rendering or input encoding. |
| F21: redirected compose avoids macOS appearance discovery | Level 1 spawned CLI with a PATH sentinel; F2 supplies the interactive Level-2 counterpart | Appropriate for the no-spawn claim. |
| F17: fast completion, saturation, timeout, kill/reap, and failure selection | Level 1 real child-process tests on macOS and Linux plus retained Windows-host execution | Appropriate for the OS-divergent process behavior. |
| F22: directory membership, aggregate hash, and exit status | Level 1 library and spawned-CLI tests on macOS and Linux plus retained Windows-host execution | Appropriate. No real-terminal behavior is asserted. |
| F23: theme remains dynamic between renders and output remains stable | Level 1 unit/snapshot coverage, headless-browser rendering, and retained Level-2 terminal coverage | Appropriate for browser and terminal output. |
| F32: approval behavior remains compatible under concurrent and failing compositions | Level 1 unit/concurrency tests | **Gap.** Exact reservation-set cleanup is now deterministic, but notification of an already parked waiter remains unproved because the test can pass through a late-reservation path. |
| F35.5: hash explanation, persisted hash, and CLI exit behavior remain compatible | Level 1 library and spawned-CLI tests | Appropriate behavioral level; the separate public-API compatibility invariant remains unsatisfied. |

No requirement in this feature depends on terminal keyboard encoding, paste,
IME, mouse input, or OS-injected key events. Level 3 is therefore not required.

## Prior-review closure assessment

- **Closed:** exact reservation-set cleanup after approval-handler and policy
  persistence failures; replacement of the OSC latency threshold with a
  deterministic query-count invariant; an auditable accepted/rejected
  reference-graph run disposition; and reconciliation of the spec and results
  with the unresolved benchmark verdict.
- **Not closed:** compiler-enforced non-public hash orchestration, deterministic
  proof that reservation release notifies an already parked waiter, and a
  reproducible integrated compose threshold pass.

## Verification performed for this review

- `darkmatter/just test`: 5,768 Darkmatter tests, 559 CLI tests, and 566 DMLS
  tests passed; one unrelated pre-existing Darkmatter test passed on retry and
  was reported flaky;
- `darkmatter/just lint`: passed for `darkmatter`, `darkmatter-cli`, and `dmls`;
- `biscuit-terminal/just test`: 2,769 of 2,770 tests passed, including the
  feature's OSC structural test; the same unrelated `Table__baseline` snapshot
  drift recorded in review 3 failed all retries;
- `biscuit-terminal/just lint`: passed for the library and CLI;
- the accepted and rejected reference-graph reports were recomputed from their
  retained raw vectors and reproduced +0.76% and +4.91%, respectively; and
- GitNexus change/impact analysis plus Sniff package, package-area, and
  dependency discovery established the affected Darkmatter and Biscuit
  Terminal scope. No current Level-2 or Level-3 run was needed: the remediation
  changed Level-1 test oracles and hash visibility, not terminal-emulator or
  keyboard-encoder behavior.

# Performance Compliance

## Purpose

This document owns performance work that cannot be completed honestly until the
host meets the measurement contract. Record every future attempt here,
including attempts declined before capture, links to retained raw artifacts,
admissibility decisions, and the final threshold verdict.

The originating evidence and predeclared contract remain in
[`results.md`](./results.md) under *Reference-graph setup remediation*. Review 4
records the release finding that created this handoff. Moving the work here does
not close acceptance criteria 5 or 6.

## Outstanding Tasks

### Integrated compose-regression threshold

The remaining measurement is a pair of independent, admissible quiet-host runs
of the committed `f-refgraph-setup-fix` harness. Complete these tasks:

- [ ] Confirm the 1-minute host load is below 2.0 before starting a capture.
- [ ] Build all three arms from their committed SHAs in detached worktrees with
  isolated target directories:
  - `base`: `51c1f16e10ffe825b56987573ba4eabc659c768e`
  - `before`: `e15b1cc22b113a9b24058207d760cd879fa62eb6`
  - `after`: `92a3d502eb65c30205a9a255dd13dd8dc6d0aabf`
- [ ] Capture the first run with all six harness cases and retain the complete
  5-second-interval `load.log`.
- [ ] Capture a second independent run under the same contract and retain its
  complete `load.log`.
- [ ] Verify that both runs satisfy every admissibility condition below.
- [ ] Recompute the reports from the retained raw vectors and record the
  artifacts, metrics, and threshold verdict in this file.

## Admissibility and Threshold Contract

A run contributes to compliance only when all of these conditions hold:

1. The 1-minute load remains below 2.0 for the full capture, with no retained
   5-second sample at or above 2.0.
2. Every case's identical-code `after_A`/`after_B` drift floor is below 1.0%,
   including both controls.
3. The two admissible runs' `base` means agree within 1.5%.
4. Each run includes all six committed harness cases, including `render_basic`
   and `help`; a compose-only capture is inadmissible.
5. All three binaries are built from the committed SHAs listed above, not from
   a working tree.
6. `compose_trivial` `after` versus `base` at or below 5% is a pass and closes
   without owner involvement. A result above 5% outside drift is a failure and
   must be escalated to Ken.

## Attempt and Result Log

### 2026-07-17 — Review 4 remediation

**Declined before capture; inadmissible host.** The host reported a 1-minute
load of 21.28 and a 5-minute load of 52.47, more than ten times the permitted
1-minute load. No benchmark was run and no threshold result was claimed.

The three required SHAs are now committed objects, so the committed-pin
condition can be met when a quiet host is available.

**Current verdict:** not established. Acceptance criteria 5 and 6 remain open.

### 2026-07-17 — Review 6 implementation cycle

**Declined before capture; inadmissible host.** During the review-6
implement cycle (see [`review-6.md`](./review-6.md), *Tracked
Production-Readiness Blocker*, and [`log.md`](./log.md) *Implementation of
Review Findings #6*), the host reported a 1-minute load average of **30.56**
(5-minute 49.57, 15-minute 42.72) on a 16-core machine — more than fifteen
times the admissibility ceiling of 2.0. Admissibility condition 1 (1-minute
load below 2.0 for the full capture) cannot be met, so no benchmark was run
and no threshold result was claimed.

Review 6 confirms this is a deferred **performance measurement**, not a code
finding: its only actionable finding (terminal-detection test doc drift) was
fixed in the cycle-6 implementation. The integrated compose-regression
threshold remains blocked solely on a quiet host, not on any owner ruling or
outstanding implementation work. The three required SHAs remain committed
objects, so the committed-pin condition can still be met when a quiet host is
available.

**Current verdict:** not established. Acceptance criteria 5 and 6 remain open.

### 2026-07-17 — Review 7 implementation cycle

**Declined before capture; inadmissible host.** During the review-7 implement
cycle (see [`review-7.md`](./review-7.md), *Tracked Production-Readiness
Blocker*, and [`log.md`](./log.md) *Implementation of Review Findings #7*), the
16-core host reported a 1-minute load average of **23.64** (5-minute 41.89,
15-minute 36.61) at assessment start, and **16.75** (5-minute 38.49, 15-minute
35.56) on recheck — more than eight times the admissibility ceiling of 2.0.
Admissibility condition 1 (1-minute load below 2.0 for the full capture) cannot
be met, so no benchmark was run and no threshold result was claimed.

Review 7 carries **no new implementation findings**; its *Findings* section
states so explicitly and classifies the integrated compose-regression threshold
as "the existing deferred measurement owned by `performance-compliance.md`, not
a new implementation finding." The three required build-arm SHAs
(`51c1f16e10ffe825b56987573ba4eabc659c768e`,
`e15b1cc22b113a9b24058207d760cd879fa62eb6`,
`92a3d502eb65c30205a9a255dd13dd8dc6d0aabf`) were re-verified as committed objects
via `git cat-file -t`, so the committed-pin condition (admissibility condition
5) can still be met when a quiet host is available.

**Current verdict:** not established. Acceptance criteria 5 and 6 remain open.

### 2026-07-17 — Review 7 assessment

**Declined before capture; inadmissible host.** The review-7 assessment checked
the host before attempting the committed three-arm harness. The host reported
load averages of **8.84** (1 minute), **19.21** (5 minutes), and **30.74**
(15 minutes), so admissibility condition 1 was already violated by more than
four times the 1-minute ceiling. No benchmark was run and no threshold result
was claimed.

The review found no new implementation or verification-level defect. The
review-6 documentation finding is closed, and the focused deterministic tests
remain green. The integrated compose-regression threshold is nevertheless
still not established, so acceptance criteria 5 and 6 remain open.

### 2026-07-17 — Review 8 assessment

**Declined before capture; inadmissible host.** The review-8 assessment checked
the host before attempting the committed three-arm harness. It reported load
averages of **12.18** (1 minute), **18.46** (5 minutes), and **26.43**
(15 minutes), exceeding the 2.0 ceiling by more than six times. No benchmark was
run and no threshold verdict was claimed from an inadmissible host.

### 2026-07-17 — Review 8 implementation cycle

**Declined before capture; inadmissible host.** The implementation cycle
rechecked admissibility on the 16-core host at 21:31:50 and again at 21:32:27:

| Reading | 1-minute | 5-minute | 15-minute |
| --- | ---: | ---: | ---: |
| 21:31:50 | 30.19 | 33.45 | 26.43 |
| 21:32:27 | 36.52 | 34.59 | 27.12 |

Both readings exceed the 2.0 ceiling by more than fifteen times, and the
1-minute figure *rose* between them, so admissibility condition 1 cannot be met
and no capture was started. No benchmark was run and no threshold result was
claimed.

Review 8 carries **no new implementation findings**; its *Findings* section
states so explicitly and records that the tracked release-evidence gap "is not
duplicated as a new implementation finding on every review iteration." The three
required build-arm SHAs (`51c1f16e10ffe825b56987573ba4eabc659c768e`,
`e15b1cc22b113a9b24058207d760cd879fa62eb6`,
`92a3d502eb65c30205a9a255dd13dd8dc6d0aabf`) were re-verified as committed objects
via `git cat-file -t`, so the committed-pin condition (admissibility condition 5)
can still be met when a quiet host is available.

**Current verdict:** not established. Acceptance criteria 5 and 6 remain open.

### 2026-07-17 — Review 9 assessment

**Declined before capture; inadmissible host.** The review-9 assessment checked
the host before attempting the committed three-arm harness. It reported load
averages of **55.05** (1 minute), **70.90** (5 minutes), and **56.41**
(15 minutes), exceeding the 2.0 ceiling by more than 27 times. No benchmark was
run and no threshold verdict was claimed from an inadmissible host.

The three required build-arm SHAs
(`51c1f16e10ffe825b56987573ba4eabc659c768e`,
`e15b1cc22b113a9b24058207d760cd879fa62eb6`, and
`92a3d502eb65c30205a9a255dd13dd8dc6d0aabf`) were re-verified as committed
objects. The threshold remains blocked solely on an admissible quiet-host
capture.

**Current verdict:** not established. Acceptance criteria 5 and 6 remain open.

### 2026-07-20 — Review 10 assessment

**Declined before capture; inadmissible host.** The review-10 assessment checked
the host before attempting the committed three-arm harness. It reported load
averages of **55.19** (1 minute), **131.94** (5 minutes), and **140.85**
(15 minutes). A later reading rose to **93.97** (1 minute), **105.16**
(5 minutes), and **125.72** (15 minutes). The host was also running another
Cargo build in this worktree, so admissibility condition 1 was already violated
by more than 27 times at the first reading. No benchmark was run and no
threshold verdict was claimed from an inadmissible host.

The three required build-arm SHAs
(`51c1f16e10ffe825b56987573ba4eabc659c768e`,
`e15b1cc22b113a9b24058207d760cd879fa62eb6`, and
`92a3d502eb65c30205a9a255dd13dd8dc6d0aabf`) remain committed objects. The
threshold remains blocked solely on two admissible quiet-host captures.

**Current verdict:** not established. Acceptance criteria 5 and 6 remain open.

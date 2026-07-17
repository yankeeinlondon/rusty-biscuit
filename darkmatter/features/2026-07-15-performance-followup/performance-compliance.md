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

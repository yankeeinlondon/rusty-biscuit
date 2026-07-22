---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-17T10:00:46-07:00
spec: 2026-07-15-performance-followup/spec.md
implemented: true
description: "A **feature** review of `2026-07-15-performance-followup/spec.md`"
feature: 2026-07-15-performance-followup/review-5.md
previous: 2026-07-15-performance-followup/review-4.md
next: 2026-07-15-performance-followup/review-6.md
---

# Review 5 — Performance Follow-up

## Scope

This review carries forward the two non-performance findings from Review 4.
The host-dependent integrated compose measurement is tracked separately in
[`performance-compliance.md`](./performance-compliance.md), where all future
attempts and results belong.

The implementation summary supplied after Review 4 claims that both findings
below are closed. Those claims have not been independently reviewed here and do
not change their status. Each finding remains open until its remediation and
required evidence are verified.

## Verdict

This feature is **not ready for production**. The two remaining implementation
findings require independent review. No performance-compliance result is part
of this review.

## Findings

### High — Verify that hash orchestration has no externally callable seam

Review 4 found that the `#[doc(hidden)]` public `internal` module behind the
non-default `internal-hash-orchestration` Cargo feature remained externally
callable Rust API. The reported remediation removes `internal.rs`, the feature,
and `plan_hash_save_explained`, returning `md hash --diff` and `--save` to the
public two-call path.

Do not close this finding from the report alone. Verify that no public or
feature-gated replacement seam remains, that the library and CLI manifests no
longer expose the feature, and that hash output, persisted values, and exit
statuses remain compatible. Any alternative shared seam still requires either
compiler-enforced non-public visibility or an explicit owner-approved
compatibility exception.

### High — Verify that the waiter test proves notification of a parked peer

Review 4 found that
`handler_error_notifies_a_waiter_blocked_on_the_same_command` could pass when
the waiter arrived late and reserved the command itself, never exercising the
notification path. The reported remediation adds a test-only parked-waiter
counter updated while holding the shared mutex immediately before the condition
variable wait.

Do not close this finding from the report alone. Verify that the synchronization
proves the waiter is parked before the approver releases its reservation, that
the test cannot pass through the late-reservation path, and that removing the
notification causes a bounded deterministic failure. Confirm that the test-only
hook does not alter production behavior.

## Required Verification

- Inspect the effective public API and Cargo feature surface after the hash-seam
  removal, then run the focused hash library and spawned-CLI behavior tests.
- Inspect the parked-waiter synchronization under the shared mutex and
  mutation-check the notification test by temporarily removing `notify_all`,
  restoring it immediately afterward.
- Run the affected Darkmatter package-area build, test, and lint gates selected
  by impact and package-scope analysis.
- Run GitNexus change detection against `main` before any eventual commit and
  confirm only the expected symbols and execution flows changed.

## Verification Performed for This Review

None. This document is a carry-forward review queue and does not accept the
implementation summary as evidence.

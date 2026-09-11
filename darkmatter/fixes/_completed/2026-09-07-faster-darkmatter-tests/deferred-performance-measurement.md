---
fix: 2026-09-07-faster-darkmatter-tests
review: 2026-09-07-faster-darkmatter-tests/review-1.md
status: deferred
created: 2026-09-09
owner: Ken Snyder
---

# Deferred performance measurement

Performance evidence this fix could not legitimately produce, why, and the
exact sequence that closes it. Everything here is *deferred*, not waived: the
measurement is still owed, it simply cannot be taken from the implementing
side.

## Deferral 1 — hosted candidate CI samples and ratified family budgets

**Maps back to:** [`review-1.md`](review-1.md) § Findings → *High — Required CI
comparison and ratified budgets do not exist*.

**Spec requirement:** [`spec.md`](spec.md) §5 requires three consecutive
candidate runs for every configured CI package/environment leg
(`spec.md:167-173`) and requires `results.md` to report ratified family budgets
against comparable evidence (`spec.md:193-196`).

**Operator ruling:** the finding carries an explicit annotation —
*"this bar was set to high and should not be considered a blocker being
production ready."* The measurement remains owed; its severity as a
merge/readiness gate does not stand.

### Why it is deferred

The requirement is not satisfiable by the implementing session. A *candidate*
sample is by definition a hosted CI run of the candidate revision, and hosted
CI does not run a revision that has not been pushed. The branch is unpushed —
[`log.md` § PR and CI handoff](log.md#pr-and-ci-handoff) records that the
remote branch was deleted and that the operator owns re-establishing it. So:

- there is no candidate CI source, therefore no candidate row on any leg;
- `test-audit attribute budgets` refuses every leg with `insufficient-runs`,
  because only **one** of the three required consecutive green *baseline*
  samples exists (run `34008778001`);
- a budget ratified from one sample, or from local timings, would be a fabricated
  ceiling. The rule
  `ceil(worst observed family summed duration × 1.25)` is fixed in advance
  precisely so the limit cannot be tuned after the candidate is observed.

This is **not** the ordinary "host CPU load made the measurement untrustworthy"
deferral. Local host load did produce two rejected samples this cycle, and those
are already retained as rejected evidence under
[`measurement/rejected-high-load/`](measurement/rejected-high-load/) and
[`measurement/rejected-http-regression/`](measurement/rejected-http-regression/).
The deferral recorded here is a structural access limitation: the runners are
unreachable until push.

### Exact missing artifacts

| Artifact | Legs | Present | Required |
|---|---|---:|---:|
| Compatible green baseline samples | Ubuntu, macOS, Windows, WSL2 | 1 (`34008778001`) | 3 consecutive |
| Candidate samples | Ubuntu, macOS, Windows, WSL2 | 0 | 3 consecutive |
| Ratified family budgets | all 65 families × 4 legs | 0 | 1 ceiling per family per leg |
| Matched-identity comparison | per environment | 0 | baseline ↔ candidate, added/removed/gated shown separately |
| Candidate `just zed-verify` execution | Ubuntu | 0 | 1 |
| Native Windows and WSL2 candidate runtime | Windows, WSL2 | 0 | 3 each |

### Closing sequence (operator-owned)

1. Reconcile branch history and push the candidate revision. The candidate
   commit set for this fix is identified in
   [`results.md` § Candidate identity](results.md#candidate-identity).
2. Collect two further compatible baseline samples, same workflow definition
   and runner image, with no intervening workflow edits.
3. Ratify each family budget as
   `ceil(worst observed family summed duration × 1.25)`, independently per
   environment, **before** looking at any candidate row.
4. Collect three consecutive green candidate runs per configured leg.
5. Compare matched identities within each environment; report added, removed,
   and gated identities separately and never net them across platforms.
6. Fill the `pending` cells in [`results.md` § CI legs](results.md#ci-legs) and
   flip AC7.

No retry policy, timeout limit, tier route, or feature selection may be changed
to make a leg fit. If a family exceeds its ratified ceiling, that is the result.

## Deferral 2 — Level-3 runtime evidence

**Maps back to:** [`review-1.md`](review-1.md) § Findings → *High — Level-3
keyboard, pointer, and image-paint behavior was not executed*.

Recorded separately in
[`results.md` § Level-3 synchronization and availability](results.md#level-3-synchronization-and-availability),
because its blocker is different in kind: L3 needs foreground window focus and
OS input injection on an attended macOS host. Running it from an unattended
session would steal focus from whoever is using the machine, which the repo's
L3 contract forbids. The finding's `**CRITICAL:**` annotation — the demand for a
clear explanation of the synchronization change — is answered in full in that
section and in the test modules themselves; only the runtime execution is
deferred.

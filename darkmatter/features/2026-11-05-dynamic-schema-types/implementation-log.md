---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-dynamic-schema-types/spec.md"
plan: "darkmatter/features/2026-11-05-dynamic-schema-types/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
---

# Implementation Log for 2026-11-05-dynamic-schema-types (9 phases)

## Baseline

Pending (Phase 1, Wave 1).

## Departures

The plan was reconciled on 2026-11-08 with the investigation outcomes and
Rulings Q8–Q20 (see the plan's "Reconciliation" section). The grant-store
location (now Q8) and the breadth staging (now Q16: value positions delivered,
whole types deferred) are rulings, no longer departures.

- **N35, mutation mechanism.** `DataRuntime::invalidate_memo` records no paths,
  so per-path invalidation of a pinned snapshot does not exist to reuse. The
  plan replaces it with a stricter check: a memo-generation counter plus a
  re-hash of the snapshot's recorded observations on each use after the counter
  moves. Grants decided earlier in the run are reused (Q17).

## Decisions applied

Phase rules N1–N42 (with N6a, N12a, N12b) are applied as written unless a later entry here says
otherwise.

## Measurements

None yet. Phase 9 measures the implementation once on one host against the S1
budgets (plan N1); the simulated baseline is in
`investigations/s1-s2-s3-s6-performance.md`.

## Phase 1

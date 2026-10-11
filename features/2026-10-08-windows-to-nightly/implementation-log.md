---
kind: implementation-log
feature: 2026-10-08-windows-to-nightly
---

# Implementation log

## Decisions taken before implementation (2026-10-10)

- Delivered in three phases on one branch, each committed for review:
  1. cadence correction, plus waiting for an unfinished PR run on a push;
  2. native Windows receipt publication and scheduled evidence consumption,
     including the all-zero-base bootstrap and the Windows migration baseline;
  3. resumable full-workspace rig seeding.
- The optional risk-directed Windows rig workflow is deferred.
- A push merged before its pull request's CI finished waits for that run
  (bounded at 90 minutes) instead of falling back to the full grid at once.
  This is an addition to the spec, which did not cover early merges.

## Phase 1 — cadence (2026-10-10)

- `environments.json`: `windows-latest` events are now `schedule` and
  `workflow_dispatch`. A PR-proven push plans no environment, cell, build, or
  preflight runner; a nightly plans native Windows and WSL2 cells, with Linux
  as WSL2's cell-less archive producer.
- `reuse_validation.py check --wait-minutes N` polls an unfinished PR run of
  the merged head once a minute; `ci.yml`'s `validation` job waits up to 90
  minutes (step timeout 95, job 100).
- **Departure:** `schema.py` refused a gating package with an empty gate list
  unless it carried an exclusion. A fully proven push is the first plan to
  select packages while scheduling no environment, so the rule now admits an
  empty gate list without an exclusion exactly when the plan schedules no
  environment. `scheduled_areas` already ignored such packages, so no area
  job launches.
- Docs and skills updated; seeding, receipt publication, and scheduled
  evidence consumption are marked **planned**.
- Not yet done from the spec: the contract check for conflicting cadence
  guidance, the Windows migration baseline, and the bootstrap/recovery
  diagnostics (phase 2).

### Validation

- `scripts/ci` Python suites: all pass (`test_inputs.py` runs no tests under
  `unittest`, unchanged from `main`).
- `ci_workflow_contracts`: 165 passed. `actionlint`: clean.

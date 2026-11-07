---
kind: feature
name: python-retirement
date: 2026-11-06
status: draft-spec
depends-on:
  - 2026-11-06-rust-ci-planner
  - 2026-11-06-os-failure-reports
  - 2026-10-27-release-process
next_spec: 2026-10-26-change-detection
reviewed: false
implemented: false
---

# Retire the remaining CI Python

## Why

Today `scripts/` holds about 21,000 lines of Python source and 34,000 of Python tests.
The evidence and planner ports remove much of it. This spec removes what is left, so
that Python is no longer a requirement for working in this repository.

## Outcome

- No CI workflow, `just` recipe, or git hook needs Python.
- `python3` and its version checks disappear from the setup steps.
- A check stops Python from returning to `scripts/`.

## Scope

For each remaining program, delete it if the new design made it unnecessary, otherwise
port it to Rust: `consolidation.py`, `completion.py`, `schema.py`, `cell_contract.py`,
`companion_suites.py`, `runner_loss.py`, `publish_gaps.py`, `constraints.py`,
`diff_scope.py`, `seed.py`, `test_inputs.py`, `workflow_reading.py`, and
`scripts/kache-config-merge.py`.

Also merge `ci-rollup` and the other `scripts/*.rs` tools into the same structure if
that reduces the number of concepts.

## Spike needed

Inventory what actually remains once the ports land. This list reflects today, and much
of it may be gone by then, so size this spec only at that point.

## Not in scope

One-off analysis scripts kept under completed `features/` or `fixes/` directories, which
nothing runs. Their Python is historical record.

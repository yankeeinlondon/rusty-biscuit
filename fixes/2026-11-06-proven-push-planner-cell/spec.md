---
kind: fix
name: proven-push-planner-cell
date: 2026-11-06
status: draft-spec
next_spec: 2026-11-06-release-path-repair
reviewed: false
implemented: false
---

# A proven merge must not plan a cell for an environment it removed

## Why

Seven of the eight most recent failing pushes to `main` failed in `Resolve this
cell's execution contract`, before compiling anything:

```text
cell_contract: cell-contract-mismatch: the plan knows no environment 'ubuntu-latest'
```

When a merge's pull request already proved the tree, the planner moves Linux and
macOS to `proven_environments` (`scripts/ci/affected_scope.py`,
`schedule_environments`). It still emits the `check` cell that compiles a
package's unchanged dependents on `ubuntu-latest` (`package_cells`), so that
cell names an environment the plan no longer holds.

Replaying the planner for `a816f1447` with `--event push --proven-event
pull_request` reproduced the exact failing cells: `messenger` and
`test-toolkit`. Planning the same tree as a pull request does not trigger it, so
the defect can only appear after merge.

## Outcome

- A push proven by its pull request plans no cells for a proven environment,
  including dependents checks, because the pull request compiled them on the
  same tree.
- A plan in which any cell names an unplanned environment is refused by the
  planner, rather than discovered later by the cell contract.
- The contract failure no longer leaves an empty artifact name that adds a
  second, misleading error in `Upload producer status`.

## Scope

- The planner fix, with a regression test built from the `a816f1447` shape.
- Correct `CLAUDE.md` and `docs/topics/ci-cd.md` where they say a dependents
  check "always runs".

## Not in scope

- The Oct 26 `coverage-audit` failure, which has a different cause. Investigate
  it only if it recurs.
- Porting the planner (`2026-11-06-rust-ci-planner`).

## Note on language

This is a small change to existing Python. Repairing current behavior is the
accepted exception to "no new Python" until the planner is ported.

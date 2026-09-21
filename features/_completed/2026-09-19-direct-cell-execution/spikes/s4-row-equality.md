---
kind: spike
name: s4-row-equality
feature: 2026-09-19-direct-cell-execution
phase: 3
date: 2026-09-20
---

# Migration step 1 evidence — the rows and the lists describe one plan

The specification's first migration step is to add the deterministic row
adapter **beside** the current projection and "prove identity equality **and
uniqueness**, dispatch fields, area membership, and complete joins back to the
plan". This is that proof.

## Method

`spikes/row-equality.py`, run from the repository root:

```sh
python3 features/2026-09-19-direct-cell-execution/spikes/row-equality.py
```

It plans the Phase 1 corpus shapes **live**, with the real planner on the real
workspace, rather than reading the frozen files under `spikes/plans/` — those
are version-4 documents and are now the previous generation. For each shape it
compares three descriptions of the same plan:

| Description | Derived from |
|---|---|
| `cells` | the plan's own `execution: execute` cells |
| `rows` | `affected_scope.row_sets`, the four per-area sets |
| `lists` | `matrix_record`'s environment lists, read back as the cells they schedule |

and additionally checks that no `{package, environment, gate}` key appears in
two rows, that every row's runner is the one the plan's environment table
gives, that every row is dispatched inside its package's area, and that every
selected area has a row document whether or not it executes anything.

## Result (2026-09-20, `fix/archive-path-sites` + this phase)

```
pr           cells=   4  rows=   4  lists=   4  areas=  1  OK
all          cells= 374  rows= 374  lists= 374  areas= 31  OK
all-reused   cells=  71  rows=  71  lists=  71  areas= 31  OK
mixed        cells= 308  rows= 308  lists= 308  areas= 31  OK
nightly      cells=  66  rows=  66  lists=  66  areas= 31  OK
prohibited   cells= 308  rows= 308  lists= 308  areas= 31  OK
gap-only     cells=   0  rows=   0  lists=  15  areas= 31  OK
             NOTE lists-vs-cells: the lists schedule 15 cell(s) the plan does
             not carry and omit 0
```

The six shapes a real planner emits agree exactly, in both directions, with no
duplicate key anywhere. The `all` shape's 374 executing cells match the Phase 1
baseline's count, so the adapter changed no selection.

## The one disagreement, and why it is the point

`gap-only` is the synthesized shape (`spikes/plans/README.md`): real gap cells,
the areas and packages that own them, no builds. No real plan is gap-only
today, because lint never reuses.

For that document the two descriptions **disagree**, and the row side is the
correct one:

- The **rows** read the cells. The document carries no executing cell, so it
  dispatches nothing — which is what an area with nothing to run means.
- The **lists** read a *package record's* declared `gates`. Those records still
  declare `lint`, so the projection would schedule fifteen lint jobs for cells
  the plan does not carry.

That is exactly the class of defect this feature exists to remove: a second
document, derived from a different field, that can disagree with what the
planner decided. It is reported here rather than asserted, because a planner
never emits a document whose package records and cells disagree — but a hand-
trimmed one, or a future planner bug, can, and only one of the two descriptions
notices.

## What this does not prove

Nothing hosted. The rows are not yet dispatched by any workflow — Phase 5 does
that — so this compares two descriptions of one plan, not two runs. The
workflow-boundary contract (the union of the four inputs equals the area's
executing cells, checked against the plan artifact) is Phase 5 Wave 2's task.

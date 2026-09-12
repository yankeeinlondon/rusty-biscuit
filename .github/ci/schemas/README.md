# CI contract schemas

Three documents carry every decision this repository's CI makes. All are
defined once in [`scripts/ci/schema.py`](../../../scripts/ci/schema.py), which
is also their validator.

| Document | Version | Written by | Read by |
|---|---|---|---|
| Resolved plan | 2 | `scripts/ci/affected_scope.py --resolved-plan` | `ci.yml`, `ci-rollup`, `just ci-local --plan`, the pre-push hook |
| Validation receipt | 2 | the pre-push hook, `scripts/cross-check.sh` | `scripts/ci/local_evidence.py`, the planner, `ci-rollup` |
| Scope receipt | 1 | the pre-push hook (`local_evidence.py scope-record`) | `ci.yml` through `local_evidence.py scope-verify` |

`contract.json` in this directory is the field contract dumped from that module
so Rust tooling can assert against it without running Python. Regenerate it with
`python3 scripts/ci/schema.py`; `scripts/ci/test_schema.py` fails when it drifts.

> **Status.** `affected_scope.py` emits the resolved plan with
> `--resolved-plan`, or with `--plan-out` alongside the legacy scope document.
> Both receipt producers — the pre-push hook and `scripts/cross-check.sh` —
> write version 2; version 1 is read-only and never upgraded in place.
> `ci-rollup rollup --plan` derives its expected cells from `cells[]`, and
> still reads the legacy *policy* document for the governance of a
> `gates = false` package, which owns no cells at all.
>
> **The legacy `scope.json` projection is still live.** `ci.yml` and
> `just/ci-local.just` read it, and `affected_scope.py::legacy_scope_document`
> projects it from the same cells — a projection, not a second calculation, so
> the two documents cannot disagree about what CI will run. It carries the
> area fan-out (`scheduled_areas`, `area_matrix`, `area_slugs`) as well.
> Deleting it means moving those consumers onto `cells[]` first; the governance
> metadata of a non-gating package has to move into the plan before the policy
> half can go.
>
> A third document, the rollup's own `ci-results.json`, is **not** defined here:
> it is Rust-owned by `scripts/ci-rollup.rs` and is at `schema_version: 3`,
> versioned independently of the plan's 2, the receipt's 2, and the baseline's
> 2. The plan fields that tool reads are asserted against `contract.json` by
> `plan_fields_match_the_frozen_contract`, so renaming one breaks a test rather
> than silently dropping a field serde never recognized.
>
> The version-1 field list was amended twice, both times additively and
> both times without a version bump because no consumer had read the document
> yet: `source_packages` and `reverse_dependencies` first (AC1 is stated over
> the difference between "changed" and "selected", and the OQ1 seam report needs
> the dependent names), then the optional `input_paths` (a package's build
> closure, which the gate-input identity is defined over) and
> `evidence_rejections`.
>
> Version 2 makes a carried plan self-sufficient: CI applies verified evidence
> to a matching scope receipt (`affected_scope.py --apply-to`) and re-projects
> `scope.json` from it without re-selecting, so the plan carries the
> `environments` table it was resolved against, each package's
> `l1_include_slow` and (for `gates = false`) its `exclusion`, and each cell's
> `reusable`. They are required, so a version-1 scope receipt misses as
> `scope-schema` and CI calculates scope itself — the miss the receipt
> contract permits — rather than hitting and then having to re-run selection.

## Resolved plan

One document describing everything a run will do, consumed identically by local
validation and hosted CI. It replaces the unversioned scope document, whose
`excluded_environment` field could suppress a *matrix* entry while leaving the
*policy* expecting a CI result for it — the PR #76 seven-cell regression.

Identity and grouping are deliberately separate. **Package is the stored
identity** of every cell, artifact, baseline entry, and receipt. **Area is a
derived grouping field** carried alongside it for presentation and outcome
ownership. A cell whose `area` disagrees with its package record is invalid.

- `areas[]` — one entry per selected area, with the reason it was selected and
  the packages contributing to it. Nested areas such as `claudine/rendezvous`
  are their own entries, never folded into a parent.
- `packages[]` — the package records: gates, explicit Cargo `targets`, tiers,
  features, backends, native dependencies, and an optional `dependent_seam`
  describing downstream packages compiled inside this package's own check.
  A `gates = false` package holds a record with an empty gate list and no
  cells: a governed absence stays visible without demanding a result.
- `source_packages[]` and `reverse_dependencies[]` — which packages *changed*,
  and which direct reverse dependents were reported but selected nowhere. An
  unchanged dependent receives no area, package record, or cell (AC1); whether
  its seam is compiled at all is Open Question 1.
- `environments[]` — the `.github/ci/environments.json` records the cells were
  resolved against, verbatim: the runner labels, native keys, and capabilities
  that evidence application and the legacy projection read.
- `l1_include_slow` on a package record, and `exclusion` on exactly the
  records whose `gates` list is empty — the policy facts the legacy matrix and
  rollup policy documents are projected from.
- `input_paths[]` on a package record — the manifest directories of its build
  closure, dev-dependencies included. The gate-input identity of a cell is
  computed over these plus that gate's global inputs; a plan that omits them
  makes its cells exact-tree-only for reuse. Optional, because only a verifier
  needs them.
- `evidence_rejections[]` — one coded reason per receipt cell that was refused,
  so a reviewer can tell a missing receipt from a rejected one. Optional.
- `cells[]` — one record per `{package, environment, gate}`, carrying
  `execution` (what will happen), `origin` (where the result comes from),
  `state` (what is true now), `reusable` (whether any local receipt may ever
  satisfy it — false for `check` and for the L1 host a companion suite needs),
  the target kinds covered, which gate supplied the compile coverage, and a
  selection reason.
- `accepted_evidence[]`, `policy_gaps[]`, `prohibited_cells[]` — the three
  reasons a cell is not executed, kept as separate lists so none can be
  silently read as another.

`execution`, `origin`, and `state` are independent axes. The cross-field rules
that make a combination meaningful — a reused cell must name its evidence, an
accepted gap must name its governing policy entry, a prohibited cell can never
be scheduled — are enforced by `validate_resolved_plan`.

## Validation receipt

Version 2 keys evidence per `{package, environment, gate}` instead of per
environment, so results from several hosts combine without overwriting one
another. Each cell carries its outcome, exit code, completion, counts, duration,
bounded failure detail, backend proof, report path, and its **gate-input
identity** — the hash of the build closure's tree entries plus the global inputs
that gate depends on. Two heads with equal gate-input identity for a cell may
share that cell's result; nothing else makes an older result reusable.

Verification *recomputes* that identity over both trees rather than trusting the
one the receipt stored: a stored identity is an unverified claim by the host
that wrote it, and both trees are present locally, so the comparison can be made
instead of believed. The stored value remains as provenance.

A gate that produced no report is recorded `partial`. It has an exit code and
nothing to attribute it to, so it is published (the run happened) and refused
for reuse (`incomplete-run`), rather than being made unpublishable or silently
credited as a tested cell.

A `complete` failing run is reusable evidence and stays a failure. An
`interrupted` run contributes nothing; an interrupted *cell* inside a complete
run is rejected alone and leaves its siblings reusable.

### Version-1 receipts

Version 1 has no per-cell outcome: presence in a package list *is* the pass
claim. It is accepted only on exact tree identity, only as pass-only
whole-environment evidence, never through gate-input equivalence, and is never
upgraded in place. Its measurements render as the exact string
`not recorded (v1 receipt)`.

## Scope receipt

What the planner selected for one committed `base..head`, published on
`refs/notes/ci-local/scope` — one ref for every host, since scope is
OS-independent — and bound to the exact `{base, head, tree}`. It carries two
documents: `plan`, the canonical resolved plan exactly as `--plan-out` writes
it, and `scope`, the legacy `scope.json` projection `ci.yml` still fans out
from; `plan_schema_version` names the plan's version so a plan migration is
refused as `scope-schema` rather than parsed. `validate_scope_receipt` checks
structure only — that the carried plan validates and names the receipt's base
and head, and that the projection carries every key the workflow reads
(`SCOPE_PROJECTION_FIELDS`) and the plan's packages. The identity comparison
is the verifier's, in R3's order: schema, head, tree, base, then structure.

CI reads it from the event head only and, on a hit, writes both documents out
in place of running the planner; verified validation evidence is then applied
to the carried plan (`affected_scope.py --apply-to`) and the projection is
re-derived from the result, still without selection. Its miss codes are
`SCOPE_REJECTIONS`
(`scope-missing`, `scope-schema`, `scope-head-mismatch`, `scope-tree-mismatch`,
`scope-base-mismatch`, `scope-malformed`), kept apart from the cell rejections
below because they refuse a whole document rather than one outcome.

## Rejection vocabulary

Every refusal to accept evidence names one code from `schema.REJECTIONS`, so the
planner, the verifier, and `just ci-local --plan` describe the same refusal the
same way. Adding a code is a contract change.

# CI schema versions

The version history of every versioned document the CI system writes or reads:
what each generation changed, why, and what happens to a document of an older
generation. It is the one changelog for these documents. Field contracts live
with their owners (listed below); this page records how they got there.

Specs are named by their `{date}-{name}` directory. Find them under `features/`
or `fixes/`, in whichever lifecycle directory holds them now.

## Current versions

| Document | Version | Owner (writer and validator) | Mirrors that must move with it |
|---|---|---|---|
| [Resolved plan](#resolved-plan) | 7 | `scripts/ci/schema.py::RESOLVED_PLAN_SCHEMA_VERSION` | `.github/ci/schemas/contract.json` (regenerated), `ci-rollup.rs::PLAN_SCHEMA_VERSION`, `test_toolkit::archive_guard::PLAN_SCHEMA_VERSION`, the `.githooks/tests/fixtures/plan-*.json` fixtures and `affected_scope_stub.py` |
| [Validation receipt](#validation-receipt) | 2 (reads 1) | `schema.py::RECEIPT_SCHEMA_VERSION`, `LEGACY_RECEIPT_SCHEMA_VERSION` | — |
| [Scope receipt](#scope-receipt) | 1 | `schema.py::SCOPE_RECEIPT_SCHEMA_VERSION` | — |
| [Expected-test manifest](#expected-test-manifest) | 2 | `schema.py::EXPECTED_MANIFEST_SCHEMA_VERSION` | `ci-rollup.rs::EXPECTED_MANIFEST_SCHEMA_VERSION` stays **1** on purpose (see below) |
| [Completion record](#completion-record) | 1 | `schema.py::COMPLETION_RECORD_SCHEMA_VERSION` | `ci-rollup.rs::COMPLETION_RECORD_SCHEMA_VERSION` |
| [Result document](#result-document) (`ci-results.json`) | 5 | `ci-rollup.rs::RESULT_SCHEMA_VERSION` | — |
| [Skip baseline](#skip-baseline) (`.github/ci/ci-baseline.toml`) | 3 | `ci-rollup.rs::BASELINE_SCHEMA_VERSION` | `affected_scope.py::BASELINE_SCHEMA_VERSION`, the file's own `schema_version` |
| [Environment table](#environment-table) (`.github/ci/environments.json`) | 3 | `affected_scope.py::ENVIRONMENTS_SCHEMA_VERSION` | `ci-rollup.rs::ENVIRONMENTS_SCHEMA_VERSION` |
| [Build archive documents](#build-archive-documents) | 3 (manifest); status, verdict, sidecar 1 | `ci-build-archive.rs` | sidecar: `affected_scope.py::SIDECAR_SCHEMA_VERSION` |
| [Build key and compiler-work documents](#build-key-and-compiler-work-documents) | 1 | `ci-build.rs` | key: `scripts/ci/build_key.py::KEY_SCHEMA_VERSION` |

## Rules for changing a version

- **Bump on any change to a required field.** Making a field optional, or
  adding an optional field that no reader relies on yet, doesn't need a bump.
  The plan's version-1 amendments below show where that line falls.
- **Readers check the version before the field set.** A document from another
  generation is refused as `unknown-schema-version` (or `scope-schema` for a
  scope receipt). Otherwise a reader reports a missing field, which looks like
  a corrupt document when the real answer is version skew.
- **Two branches never share a number.** If two lines of work both claim the
  next version, the merge gives their union a new number of its own. Otherwise
  one number names two incompatible shapes. It has happened twice: version 4
  and version 7 of the plan.
- **A plan bump never moves the receipt version.** A new plan shape isn't a
  reason to invalidate a cell that was already validated. The scope receipt
  carries the plan's version inside it (`plan_schema_version`), so an older
  scope receipt misses once as `scope-schema` and CI runs one fresh
  calculation. Nothing is ever upgraded in place.
- **Every mirror moves in the same change.** The mirrors are listed in the
  table above. Contract tests catch most drift
  (`the_plan_schema_version_matches_the_frozen_contract`,
  `plan_fields_match_the_frozen_contract`, `test_schema.py` against
  `contract.json`). When the environment table moved to 3 and
  `ci-rollup.rs` didn't, every area's coverage audit refused the shipped
  table for two runs.
- **Record the change here in the same commit.** Also regenerate
  `contract.json` with `python3 scripts/ci/schema.py`.

## Resolved plan

The document that describes everything one run will do. Field contract:
[`.github/ci/schemas/README.md`](../../.github/ci/schemas/README.md#resolved-plan).

| Version | Change | Source |
|---|---|---|
| 1 | First frozen contract. Amended twice without a bump, because no consumer had read the document yet: `source_packages` and `reverse_dependencies` (AC1 is stated over the difference between *changed* and *selected*, and the seam report needs the dependent names), then the optional `input_paths` and `evidence_rejections`. | `2026-09-11-cicd-cleanup` |
| 2 | The plan becomes self-sufficient for what CI does with a carried scope receipt: applying verified evidence and re-projecting `scope.json` without selecting again. Adds the plan-level `environments` table, per-package `l1_include_slow` and (non-gating) `exclusion`, and per-cell `reusable`. | `2026-09-11-cicd-cleanup` |
| 3 | Two branches, two incompatible shapes: build records (plan-level `builds[]`, per-cell `build`) on one, and the required `change_inventory` on the other. | `2026-09-12-single-os-compile` and the change-inventory work |
| 4 | The union of both version-3 shapes. Either version-3 document is refused by version. | — |
| 5 | On `main`: the required `archive_guard` scan scope and `change_inventory.deleted`. The planner is the only component that knows which files an event put in scope, so the guard reads its scope from the plan instead of choosing between an empty scan and a full one. | `2026-09-19-less-brittle` |
| 5, 6 (unreleased) | On the direct-cell-execution branch: one dispatch row per executing cell, so every execution input has to come from the plan. Adds `skip_policy`, area `execution_path`, per-cell `profile` and `requires_node` (5), then an executing L2 cell's `backends` (6). Never reached `main` under these numbers. | `2026-09-19-direct-cell-execution` |
| 7 | The union of `main`'s 5 and the branch's 6. The optional per-cell `companions_only` (a lint cell whose required work is its companions) arrives with it. | both of the above |
| 7 (amended) | The optional per-cell `test_filter`: an L1 cell narrowed to the tests that read a changed file. No bump, because absence means exactly what every version-7 plan already meant — the whole tier — and a plan is only ever read by the code at the head it was resolved for, so no reader meets the field without understanding it. | `2026-09-22-test-input-blind-spot` |

A scope receipt from any earlier generation misses once as `scope-schema`.

## Validation receipt

What the pre-push hook (and `scripts/cross-check.sh`) proved on one host, stored
under `refs/notes/ci-local/<environment>` and read by
`scripts/ci/local_evidence.py`.

| Version | Change | Source |
|---|---|---|
| 1 | Whole-environment receipt: exact-tree only, pass only, with no per-cell measurements and no gate-input equivalence. Still read, never written, never upgraded in place. Its missing measurements render as `not recorded (v1 receipt)`. | `2026-09-11-cicd-cleanup` |
| 2 | Keyed per `{package, environment, gate}`. `strict` and `warn` publish a complete run whether it passed or failed; only passing cells qualify for reuse. Live since 2026-09-12. | `2026-09-11-cicd-cleanup` |
| 2 (amended) | The optional per-cell `test_filter`: the narrowing a test-input run applied. Such a cell satisfies only the plan cell with the same filter, on the exact tree, from any host (`docs/cicd/test-inputs.md`). No bump: a reader that predates it would see an ordinary L1 cell on the host's environment, and the only such reader is the same head's code. | `2026-09-22-test-input-blind-spot` |

Every plan bump since has left this at 2 on purpose.

## Scope receipt

What the planner selected for one exact `{base, head, tree}`. The hook publishes
it under `refs/notes/ci-local/scope`, and CI adopts it as the plan on an exact
match.

| Version | Change | Source |
|---|---|---|
| 1 | Introduced. It carries the plan it selected, plus that plan's version as `plan_schema_version`, and that inner version is what makes it miss after a plan bump. Keys the plan has since retired (`matrix`, `area_matrix`) are ignored rather than refused, because the check requires keys and ignores extras. | `2026-09-10-local-affected-scope` |

## Expected-test manifest

A producer's listing of the tests its tier selects, written by
`just _expected_manifest` on the execution target.

| Version | Change | Source |
|---|---|---|
| 1 | The identities a tier selected. It can't tell a test that was never compiled for this target from one that stopped running. | — |
| 2 | Records `ignored` and `excluded` identities separately, plus the listing's provenance: environment, tier, target triple, resolved nextest version, archive mode, and the selection applied. | `2026-09-19-direct-cell-execution` |

`ci-rollup` deliberately reads version 1 only, and only for **legacy cells**
(reused, gap, or rolled up without a plan). It refuses a version-2 manifest and
names `scripts/ci/completion.py` as its reader: comparing a second time would
recompute what the completion contract already settled.

## Completion record

A producer's claim that it ran what the plan scheduled. `scripts/ci/completion.py`
writes it only after validation succeeds, and each area's coverage audit reads
it.

| Version | Change | Source |
|---|---|---|
| 1 | Introduced. It exists only after validation passes, so a missing record and a `complete: false` record mean the same thing. It binds the plan's head, the cell, its build, and the run. | `2026-09-19-direct-cell-execution` |

## Result document

`ci-results.json`, written by `ci-rollup rollup`. Each area uploads one slice
(`ci-results-<area>`).

| Version | Change |
|---|---|
| 1 | Keyed by area. |
| 2 | Every identity keyed by package. |
| 3 | The area-owned result model: each cell carries its derived `area`, its `origin`, the evidence behind a reused result, its measured duration, and its target coverage. The document carries the accepted evidence the run was scheduled against. |
| 4 | `counts` becomes an optional measurement alongside `duration_s`. An unmeasured cell omits it instead of reporting a zero, which would read as a suite that found nothing. |
| 5 | Each executing cell carries its `completion`. The meaning of an executing cell's skip set changes: it is only what that cell's reports observed, because the producer already compared them with the expected listing. |

A reader refuses a document from another generation before interpreting any
cell.

## Skip baseline

`.github/ci/ci-baseline.toml`, the hand-edited budget of approved exact skips.

| Version | Change |
|---|---|
| 2 | Its own version line, keyed on `{package, environment, tier}`, independent of the machine-generated result document. Earlier history is not recorded in the tree. |
| 3 | Removes `[[failure]]` known-failure entries. Producer jobs now fail visibly, so a downstream rollup can't pardon them consistently. The skip budget stays package-keyed. Since the direct-cell-execution work, producers and the audit read the plan's `skip_policy` snapshot of this file, never the file itself. |

## Environment table

`.github/ci/environments.json`, the capability table cells are resolved against.

| Version | Change | Source |
|---|---|---|
| 1 | Runner labels, native keys, and capabilities. | — |
| 2 | The per-environment `build` contract: the compile-affecting inputs a native producer declares, and the predicates an archive-only environment is checked against. | `2026-09-12-single-os-compile` |
| 3 | `events`: the GitHub events that schedule each environment. `ci-rollup` doesn't read this field but still checks the version. | `2026-09-18-ci-cadence` |

## Build archive documents

Written and verified by `scripts/ci-build-archive.rs`.

| Document | Version | Notes |
|---|---|---|
| Archive manifest | 3 | The constant first appears in the tree at 3 (2026-09-15), and no earlier generation shipped. A consumer refuses a generation it doesn't understand rather than checking only the fields it recognizes. |
| Build status | 1 | The only document a failed producer emits. |
| `verify --json` verdict | 1 | — |
| Sidecar | 1 | Mirrored by the planner. |

## Build key and compiler-work documents

Written by `scripts/ci-build.rs`. The event, report, and key documents are all
at version 1. The key's version is mirrored in `scripts/ci/build_key.py`, so
the planner and the builder derive the same key.

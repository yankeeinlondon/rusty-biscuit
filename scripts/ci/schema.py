#!/usr/bin/env python3
"""Canonical CI contracts: the resolved plan, the validation receipt, the scope
receipt, and the two documents a producer proves itself with — the expected-test
manifest and the completion record.

One module owns both documents so the planner, the evidence verifier, the hook,
and the Rust rollup cannot drift into private copies of the same vocabulary.
`fixes/2026-09-11-cicd-cleanup/spec.md` requires exactly this: "Determine the
smallest coherent schema/workflow changes... Avoid parallel planners, duplicate
policy stores".

The field contract is also dumped to `.github/ci/schemas/contract.json` so the
Rust side can assert its structs against the same source without spawning a
Python process. `test_schema.py` fails when that file drifts from this module.

## Notes

`affected_scope.py` emits the resolved plan, `local_evidence.py` verifies
receipts against it per cell, and the Rust rollup reads it through `--plan`.
All three are on this contract; `verified_environment` survives only as the
version-1 reader.

Phase 3 amended the frozen field list once, adding `source_packages` and
`reverse_dependencies`. Both are load-bearing rather than convenience: AC1 is
stated over the difference between "changed" and "selected", and OQ1's seam
report needs the dependent names. The version stayed 1 because no consumer had
read version 1 yet, so there was no document to migrate.

Phase 4 amended it a second time, adding the optional `input_paths` (a
package's build-closure manifest directories) and `evidence_rejections`. The
gate-input equivalence rule of spec section 3.3 is defined over the build
closure, and the planner is the only thing that knows it; without the field
there is nowhere for a verifier to learn which paths a cell's identity covers.
Both are optional, so every version-1 document already written stays valid and
the version again stayed 1.

Version 2 makes the plan self-sufficient for the two operations CI performs on
a carried scope receipt without re-selecting anything: applying verified
evidence (`affected_scope.apply_accepted_cells`) and projecting the legacy
`scope.json` (`legacy_scope_document`). It adds the plan-level `environments`
table the cells were resolved against, per-package `l1_include_slow` and the
non-gating `exclusion`, and per-cell `reusable`. They are required, and the
version moved, because a version-1 receipt lacks them: it now misses as
`scope-schema` and CI calculates scope itself, which the receipt contract
permits, rather than hitting and then having to re-run selection to finish.

Version 3 adds build records (`fixes/2026-09-12-single-os-compile/spec.md`):
the plan-level `builds` list and the per-cell `build` reference that names one
of them. A build record is plumbing keyed by `{package, producer environment,
build}` — never a result cell, never baseline-eligible — and it exists so an
executing test cell can say which immutable compile it consumes instead of
compiling its own. A version-2 receipt carries no build records at all, so it
misses as `scope-schema` and CI resolves the plan itself rather than being
partially upgraded into a document whose builds nothing derived.

Version 4 exists because two changes each called themselves version 3 on
separate branches: the build records above, and the `change_inventory` below.
The merged document requires both, so "3" named two incompatible shapes — a
document from either branch passed the version check and then failed on a
missing field, which reads as a corrupt document rather than a version skew.
4 names the union, and either older shape now misses as
`unknown-schema-version`, which is what it is.

`change_inventory` is the changed paths, normalized and bucketed once by the
calculator so every reader — the plan renderer, the local pre-push report,
`ci-reporting` — states the same thing about what changed. It is computed from
the paths alone and is deliberately allowed to disagree with `change_class`,
which is derived from the gating packages a change selects. A version-2 or
version-3 scope receipt therefore misses once as `scope-schema` and is never
upgraded in place; validation receipts are untouched, so
`RECEIPT_SCHEMA_VERSION` does not move.

Version 5 adds the required `archive_guard` scope
(`fixes/2026-09-19-less-brittle`) and the `deleted` half of the change
inventory it reads. The guard scans repository source for compile-time paths
that do not survive an archived run, and the planner is the only thing that
knows which files an event put in scope; a plan that carried no answer would
leave the guard choosing between an empty scan and a full one, and it refuses
to guess.

`2026-09-19-direct-cell-execution` numbered its own changes 5 and 6 on a
branch developed alongside it: the hosted workflows stop consuming environment
lists and expand one matrix row per executing cell, so every input a downstream
job used to receive as a workflow argument has to be answerable from the plan
alone. It adds the plan-level `skip_policy` snapshot of
`.github/ci/ci-baseline.toml` (ruling R8 — producers and the audit read the
plan, never the file), the area-level `execution_path` that says which dispatch
form that area is on (R9), the per-cell `profile` and `requires_node`
execution inputs, and an executing L2 cell's `backends`, the hostable subset
its producer must prove.

Version 7 is those two together, for the reason version 4 exists: "5" named two
incompatible shapes. A version-4, -5, or -6 scope receipt misses once as
`scope-schema`; `RECEIPT_SCHEMA_VERSION` again does not move, because neither a
scheduling representation nor a scan scope is a reason to invalidate a
validated cell.

The direct-cell-execution work adds two producer-side documents on their own
version lines. `EXPECTED_MANIFEST_SCHEMA_VERSION` moves 1 → 2: a v1 manifest
listed only the identities a tier selected, which cannot tell a test that was
never compiled on this target from one that stopped running, so v2 records the
ignored and excluded identities and the provenance of the listing.
`COMPLETION_RECORD_SCHEMA_VERSION` starts at 1 — the artifact a producer
publishes to say it ran what the plan scheduled. Neither touches the plan or
the receipt.
"""

from __future__ import annotations

import hashlib
import json
import re
from datetime import date
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = ROOT / ".github" / "ci" / "schemas" / "contract.json"

RESOLVED_PLAN_SCHEMA_VERSION = 7
RECEIPT_SCHEMA_VERSION = 2

#: The scope receipt: what the planner selected for one exact `{base, head,
#: tree}`, published by the hook under `refs/notes/ci-local/scope` and taken as
#: the plan by CI on an exact match (`fixes/2026-09-10-local-affected-scope`,
#: R3). Distinct from the validation receipt: it carries no outcome.
SCOPE_RECEIPT_SCHEMA_VERSION = 1

#: The last receipt version that predates per-cell outcomes. Accepted only on
#: exact tree identity, pass-only, whole-environment, and never upgraded in
#: place (spec section 3.6).
LEGACY_RECEIPT_SCHEMA_VERSION = 1

#: The producer's expected-test manifest (`just/devops.just::_expected_manifest`).
#: Version 2 records `ignored` and `excluded` identities explicitly instead of
#: dropping them, and carries the provenance — environment, tier, target,
#: resolved nextest version, archive mode, and the selection applied — without
#: which a listing proves nothing about the report it is compared to.
EXPECTED_MANIFEST_SCHEMA_VERSION = 2

#: The producer completion record (`scripts/ci/completion.py`). Written only
#: after validation succeeds, so `complete: true` cannot exist without the
#: evidence behind it (ruling R10).
COMPLETION_RECORD_SCHEMA_VERSION = 1

#: Rendered in place of counts and duration for a version-1 receipt. Verbatim
#: from spec section 3.6 — the text is part of the contract, not decoration.
UNRECORDED_MEASUREMENT = "not recorded (v1 receipt)"

ENVIRONMENTS = ("ubuntu-latest", "windows-latest", "macos-latest", "wsl2-ubuntu")

#: The GitHub events a plan can be resolved for, as `github.event_name` spells
#: them. `affected_scope.EVENT_NAMES` is this tuple.
EVENTS = ("pull_request", "push", "schedule", "workflow_dispatch")

#: A cell's gate. `check` and `lint` are compile-time gates; the rest are test
#: tiers. One flat vocabulary because a cell key is `{package, environment,
#: gate/tier}` and the two kinds are never both present for one key.
GATES = ("lint", "check", "L1", "L2", "browser")

#: The gates whose execution consumes a Nextest archive, and therefore the only
#: gates a cell may reference a build from. `check` and `lint` are deliberately
#: separate configurations (spec section 6): Clippy is another compiler driver,
#: and check-only target kinds may emit no executable at all.
BUILD_GATES = ("L1", "L2", "browser")

#: Cargo target kinds a check or test gate may be asked to cover. Replaces the
#: blanket `--all-targets` contract (spec section 1.6).
TARGET_KINDS = ("lib", "bin", "test", "example", "bench")

#: What the plan intends to happen to a cell.
EXECUTIONS = ("execute", "reuse", "omit")

#: Where a cell's result comes from. `prior-local` is a receipt from an older
#: head accepted through gate-input identity; `local` is the current head's.
ORIGINS = ("ci", "local", "prior-local", "none")

#: The plan-time state of a cell, before any result exists.
CELL_STATES = ("pending", "reused", "accepted-gap", "prohibited")

#: How an area's work reaches its hosted jobs: one matrix row per executing
#: cell. The environment-list form (`lists`) is not admitted, because no
#: shipped workflow can dispatch it; a plan that claimed it would describe work
#: nothing runs. Ruling R9's per-area switch was replaced by an all-areas
#: switch whose rollback is a revert (Phase 7 of
#: `2026-09-19-direct-cell-execution`); Phase 8 retires this field.
EXECUTION_PATHS = ("rows",)

#: One dispatch row's keys, in the order a matrix job's label renders them
#: (ruling R1). Dispatch identity only: everything else a job needs it resolves
#: from the plan by the `{package, environment, gate}` this row names.
ROW_FIELDS = ("package", "gate", "environment", "runner")

#: The four disjoint row sets one area dispatches. `wsl` takes every
#: archive-only guest cell, `check` and `lint` their gates, and `test` the
#: native test tiers; together they partition the area's executing cells.
ROW_SET_NAMES = ("test", "check", "lint", "wsl")

#: The nextest profile a hosted test cell runs under. One value today — the
#: workflows export `NEXTEST_PROFILE: ci` for every tier — carried on the cell
#: so the producer's expected-test listing and its gate command select from one
#: recorded answer rather than two independent conventions.
CI_PROFILE = "ci"

#: Buckets of the plan's change inventory, in report order. Exhaustive and
#: disjoint over the changed paths: `other` is the declared home for a path no
#: rule claims, so a reader never has to wonder whether a path was dropped.
CHANGE_BUCKETS = ("configuration", "documentation", "source", "other")

#: The machine-readable result state for a governed, unexpired policy gap.
#: Design Decision 10: never inferred from a GitHub conclusion, so retry and
#: rollup logic cannot mistake it for an interruption.
ACCEPTED_GAP_STATE = "ACCEPTED GAP"

#: Whether a local run finished the work it claimed. Only `complete` is
#: reusable evidence (spec section 3.4).
COMPLETIONS = ("complete", "interrupted", "partial")

OUTCOMES = ("pass", "fail")

#: Why a receipt, or one of its cells, was not accepted. Shared by the verifier,
#: the planner, and `just ci-local --plan` so a rejection reads the same
#: everywhere. Adding a code is a contract change.
REJECTIONS = (
    "missing-receipt",
    "malformed-receipt",
    "unknown-schema-version",
    "unknown-environment",
    "unknown-gate",
    "unknown-package",
    "tree-mismatch",
    "environment-mismatch",
    "revision-mismatch",
    "gate-inputs-changed",
    "incomplete-run",
    "failed-cell",
    "conflicting-evidence",
    "dirty-tree",
    "v1-requires-exact-tree",
    "v1-not-equivalence-eligible",
    "v1-is-pass-only",
    #: The plan's snapshot of the exact-skip policy (ruling R8). All three are
    #: planner-time errors: the snapshot travels in the artifact a carried scope
    #: receipt reuses, so an approval that cannot be bound to a cell, one whose
    #: expiry has passed, or a snapshot that cannot say what it read must be
    #: refused here rather than surprising a producer hours later.
    "skip-policy-cell",
    "skip-policy-expired",
    "skip-policy-provenance",
)

#: Why CI declined a scope receipt and calculated scope itself. Kept apart
#: from [`REJECTIONS`]: those refuse a *cell's outcome*; these refuse a whole
#: document, and the two are never reported in the same list.
SCOPE_REJECTIONS = (
    "scope-missing",
    "scope-schema",
    "scope-head-mismatch",
    "scope-tree-mismatch",
    "scope-base-mismatch",
    "scope-event-mismatch",
    "scope-malformed",
)

#: Why a consumer refused its build inputs. Mirrored by `ci-build`'s own
#: `REJECTIONS` constant, which `contract.json` is asserted against.
#:
#: Kept apart from both [`REJECTIONS`] and [`SCOPE_REJECTIONS`]: those refuse a
#: cell's outcome and a whole scope document respectively. These refuse the
#: *inputs* to a cell that has not run yet, and every one is an infrastructure
#: verdict a workflow reports — never a test result, and never something a
#: consumer may repair by compiling a replacement.
BUILD_REJECTIONS = (
    "build-manifest-missing",
    "build-manifest-malformed",
    "build-manifest-schema",
    "build-digest-mismatch",
    "build-key-mismatch",
    "build-source-mismatch",
    "build-environment-incompatible",
    "build-runtime-incompatible",
    "build-archive-missing",
    "build-archive-corrupt",
    "build-sidecar-missing",
    "build-sidecar-corrupt",
    "build-asset-missing",
    "build-inventory-incomplete",
    "build-inventory-unexpected",
)

#: Why a producer refused to certify its own cell. Kept apart from every other
#: rejection tuple: these are decided ON the producer, before any artifact is
#: published, and each one means "this job did not prove what it was asked to
#: prove" rather than "a test failed" or "an input was unreadable".
COMPLETION_REJECTIONS = (
    "completion-plan-unreadable",
    "completion-cell-unknown",
    "completion-cell-not-executing",
    "completion-manifest-missing",
    "completion-manifest-schema",
    "completion-manifest-provenance",
    "completion-manifest-target",
    "completion-manifest-selection",
    "completion-report-missing",
    "completion-report-malformed",
    "completion-report-duplicate",
    "completion-test-missing",
    "completion-test-unexpected",
    "completion-test-failed",
    "completion-skip-unapproved",
    "completion-expected-empty",
    "completion-companion-incomplete",
    "completion-companion-failed",
    "completion-backend-unproven",
)

#: Why a producer refused a dispatch row before running anything. A row carries
#: dispatch identity alone, so every one of these means the row and the plan
#: disagree about what this run scheduled — the class of defect direct cell
#: execution exists to make impossible to ignore.
CELL_CONTRACT_REJECTIONS = (
    "cell-contract-row",
    "cell-contract-head",
    "cell-contract-unknown",
    "cell-contract-duplicate",
    "cell-contract-not-executing",
    "cell-contract-mismatch",
    "cell-contract-package",
    "cell-contract-build",
)

#: The tier markers `_tier_filter` builds every canonical selection from. The
#: filter STRINGS stay in `just/devops.just` — copying them here is the drift
#: this feature exists to remove — but the vocabulary is what tells an ad hoc
#: `-E 'test(one_test)'` apart from a tier expression, which is the only way a
#: producer can notice a filter that shrank its expected and observed sets
#: together. `test_completion.py` checks the shipped filters against it.
TIER_MARKERS = ("level2_", "level3_", "browser_", "real_", "slow_", "perf_")

#: What a canonical selection must say for each tier, as data rather than as
#: the filter strings. `completion.py` binds a listing's recorded expression
#: to the cell's gate through this table, so a `_tier_filter` that drifted to
#: another tier's marker cannot certify itself: `nextest list` and `nextest
#: run` would agree with each other and disagree with this. A positive tier
#: selects exactly its own marker. An L1-shaped tier negates a union of
#: markers that MUST take every other tier out and MAY also take out the
#: L1-internal ones — `slow_` unless `l1-include-slow`, `perf_` for
#: `worktree-cli` — because those two vary by package and switch, not by gate.
CANONICAL_SELECTION: dict[str, Any] = {
    "selects": {"L2": "level2_", "L3": "level3_", "browser": "browser_", "real": "real_"},
    "excludes": {
        "tiers": ["L1", "sanity"],
        "must": ["level2_", "level3_", "browser_", "real_"],
        "may": ["slow_", "perf_"],
    },
}

#: Cap on `failed_tests` carried in a receipt cell. A failing L1 suite can name
#: thousands of tests; a Git note is not a report store. Past the cap the cell
#: sets `failure_detail_truncated` and the full list stays in the host report.
FAILURE_DETAIL_LIMIT = 20

_SHA = re.compile(r"[0-9a-f]{40}")
_IDENTITY = re.compile(r"[0-9a-f]{8,64}")

#: A Windows drive prefix. `C:rel.rs` is drive-relative rather than absolute,
#: so the leading-slash test does not cover it.
_DRIVE = re.compile(r"[A-Za-z]:")

#: A planned build key: sixteen lowercase hex digits of XXH64, as `ci-build
#: key` writes it. Narrower than [`_IDENTITY`] on purpose — a SHA-256 here
#: would mean something computed the key outside the one hashing boundary.
_BUILD_KEY = re.compile(r"[0-9a-f]{16}")


# ---------------------------------------------------------------------------
# Field contract
# ---------------------------------------------------------------------------

#: Required keys, in the order a reader should expect them. Value is `True` for
#: a required key and `False` for one that may be absent.
RESOLVED_PLAN_FIELDS: dict[str, bool] = {
    "schema_version": True,
    "base": True,
    "head": True,
    "change_class": True,
    #: What changed, bucketed once from the changed paths. A sibling of
    #: `change_class`, never a summary of it: `change_class` is derived from the
    #: gating packages a change selects, so the two are allowed to disagree and
    #: a reader is seeing the truth when they do.
    "change_inventory": True,
    #: The archive-path guard's scan scope: whether the planner selected it and,
    #: if so, whether the scan is the full eligible corpus or an explicit path
    #: list. Required because an absent field and "nothing eligible changed" are
    #: different claims, and the guard refuses to guess which one it is looking
    #: at (fixes/2026-09-19-less-brittle).
    "archive_guard": True,
    "full_scope": True,
    "full_scope_gates": True,
    "areas": True,
    "packages": True,
    #: Packages whose own source changed, as opposed to packages the plan
    #: selected. Kept distinct because AC1 is stated over the difference: a
    #: direct reverse dependent is reported and never selected.
    "source_packages": True,
    #: Direct reverse Cargo dependents of `source_packages` that the plan did
    #: NOT select. Reported so a reader can see the seam OQ1 governs; they
    #: receive no area, package record, or cell under any OQ1 option.
    "reverse_dependencies": True,
    #: The `.github/ci/environments.json` records the cells were resolved
    #: against, verbatim. Evidence application and the legacy projection need
    #: runner labels, native keys, and capabilities; carrying them is what
    #: lets CI act on a receipt without consulting the checkout's table.
    "environments": True,
    "cells": True,
    #: The run-scoped build records the executing test cells consume, one per
    #: distinct planned build key. Empty when nothing executes: evidence-
    #: satisfied and governed cells create no consumer demand, so an all-reused
    #: plan schedules no owner at all.
    "builds": True,
    "accepted_evidence": True,
    #: The snapshot of `.github/ci/ci-baseline.toml`'s approved exact-skip
    #: budget, with the entries that apply to this plan's cells and provenance
    #: naming the file and its content hash. Required: an optional snapshot
    #: would let a plan silently carry no policy at all, and the producer that
    #: decides whether an observed skip is approved reads only the plan.
    "skip_policy": True,
    #: The dispatch rows derived from the executing cells, `{area: {test,
    #: check, lint, wsl, has_*_rows}}`. Optional and derived: `row_sets` is the
    #: one function that computes it, every consumer re-derives rather than
    #: trusting a carried copy, and it rides in the document so the renderer
    #: and the workflow show the same dispatch.
    "rows": False,
    #: Why each rejected receipt cell was refused, one coded reason per entry.
    #: Optional because a plan resolved with no evidence at all has nothing to
    #: report; `just ci-local --plan` renders it when present (AC7).
    "evidence_rejections": False,
    "policy_gaps": True,
    "prohibited_cells": True,
    "job_estimate": True,
    "preflight_os": True,
    "preflight_reason": True,
    "flags": True,
    #: The GitHub event the plan was resolved for, and the environments that
    #: event does not schedule (`{name, events}`) or that an earlier run of
    #: another event already validated for this tree (`{name, event}`). All
    #: three are optional and absent rather than empty, so a plan resolved
    #: without an event is byte-identical to one from before they existed
    #: (fixes/2026-09-18-ci-cadence).
    "event": False,
    "deferred_environments": False,
    "proven_environments": False,
    #: Environments in the plan's table for their build records and preflight
    #: runner alone (`{name, for}`): the event did not schedule them, but a
    #: scheduled archive-only guest runs what they compile. Optional and absent
    #: rather than empty (fixes/2026-09-19-nightly-scope).
    "producing_environments": False,
}

#: `paths` and `counts` are present exactly when `diff_available` is true, and
#: `reason` exactly when it is false. A manual full-scope run consulted no diff;
#: recording empty buckets would read as "nothing changed", which is a different
#: and false claim.
CHANGE_INVENTORY_FIELDS: dict[str, bool] = {
    "diff_available": True,
    "paths": False,
    "counts": False,
    #: The subset of the diff's paths that were REMOVED. Gated exactly like
    #: `paths` and `counts`. Declared rather than inferred: `git diff
    #: --name-only` cannot tell a deletion from a path that is simply not there,
    #: and a consumer that treated every missing path as a deletion would stop
    #: checking a file the diff named.
    "deleted": False,
    "reason": False,
}

#: Exactly one of three shapes, each carrying a non-empty `reason`. `mode` and
#: `paths` are absent when `selected` is false: an unselected guard has no scope
#: to describe, and an empty list there would be indistinguishable from the real
#: "nothing eligible changed" state.
ARCHIVE_GUARD_FIELDS: dict[str, bool] = {
    "selected": True,
    "mode": False,
    "paths": False,
    "reason": True,
}

#: The scan scopes a selected guard can have. `changed` always carries `paths`,
#: even when empty; `full` never does.
ARCHIVE_GUARD_MODES = ("changed", "full")

AREA_FIELDS: dict[str, bool] = {
    "area": True,
    "selection_reason": True,
    "packages": True,
    #: Which dispatch form this area is on (ruling R9). Required, because
    #: "unstated" is the one answer a workflow branching on it cannot act on.
    "execution_path": True,
}

#: The plan's snapshot of the hand-edited exact-skip policy. `source` and
#: `content_hash` are the provenance: which file was read and what it hashed
#: to, so an audit reading the plan alone can say what policy it is applying.
SKIP_POLICY_FIELDS: dict[str, bool] = {
    "source": True,
    "content_hash": True,
    "entries": True,
}

#: One approved exact-test skip, keyed like every other stored result. `backend`
#: narrows it to one L2 terminal backend; `expiry` is optional but an expired
#: entry is refused rather than ignored.
SKIP_ENTRY_FIELDS: dict[str, bool] = {
    "package": True,
    "environment": True,
    "gate": True,
    "owner": True,
    "reason": True,
    "source_run": True,
    #: The exact `<testsuite>::<testcase>` identities approved, as the baseline
    #: file spells them. Optional: an entry that names none approves any
    #: observed skip in its cell, which is the widest an approval can be and
    #: still be owned, dated, and attributable.
    "tests": False,
    "backend": False,
    "expiry": False,
}

PACKAGE_FIELDS: dict[str, bool] = {
    "package": True,
    "area": True,
    "selection_reason": True,
    "gates": True,
    "targets": True,
    "tiers": True,
    "test_args": True,
    "check_args": True,
    "l2_backends": True,
    "runner_tools": True,
    "companion_suites": True,
    #: Build outputs the producer must add to this package's archive, relative
    #: to the profile output directory. Declared by the package because the
    #: package is what breaks when one is missing.
    "archive_includes": True,
    #: Named build sidecars from `.github/ci/sidecars.json` — another package's
    #: binaries, compiled by the producer and shipped beside the archive
    #: because a consumer with no Cargo cannot build one.
    "sidecars": True,
    #: The package's `l1-include-slow` policy, which `cell_contract.py` hands
    #: the test job; it is resolved from the plan, never re-read.
    "l1_include_slow": True,
    "native": True,
    #: The package's `requires-toolchain` policy: its L1 drives `cargo`/`rustc`
    #: itself, so the consumer provisions the pinned toolchain where the
    #: environment has `cargo_toolchain`. Resolved per cell by
    #: `cell_contract.py`, never re-read from the checkout. Optional (absent reads as false) so a plan
    #: written before the field existed still projects.
    "requires_toolchain": False,
    #: The governance of a `gates = false` package: exclusion class, owner,
    #: reason, expiry. Present exactly when `gates` is empty, because the
    #: rollup's policy document is projected from the plan and reads it there.
    "exclusion": False,
    #: Repository-relative manifest directories of the package's build closure,
    #: dev-dependencies included. The gate-input identity of spec section 3.3
    #: is computed over these plus the gate's global inputs. Optional: a plan
    #: that omits it simply makes its cells exact-tree-only for reuse.
    "input_paths": False,
    #: The unchanged direct reverse dependencies compiled inside this package's
    #: own `ubuntu-latest` check cell (Open Question 1, Option B): the sorted
    #: `dependents` names and the explicit `check_args` that compile them.
    #: Present exactly when at least one is attributed; never names a package
    #: the plan also selects.
    "dependent_seam": False,
}

CELL_FIELDS: dict[str, bool] = {
    "package": True,
    "area": True,
    "environment": True,
    "gate": True,
    "execution": True,
    "origin": True,
    "state": True,
    #: Whether a local receipt may ever satisfy this cell. False for `check`
    #: (no receipt records one) and for an L1 host a companion suite needs;
    #: decided by policy at resolution time so applying evidence later needs
    #: no policy.
    "reusable": True,
    "target_kinds": True,
    "compile_coverage_from": True,
    "selection_reason": True,
    "evidence": False,
    "gap": False,
    "prohibition": False,
    #: On a `check` cell only: the package record's `dependent_seam.dependents`,
    #: carried so a reader of the cell alone sees what it also compiled.
    "dependents": False,
    #: The registered companion suites THIS cell must run: name, canonical
    #: recipe, declared environment, and whether counts are expected. Present
    #: only on a cell a companion attaches to, which is the same cell R7 makes
    #: non-reusable.
    "companions": False,
    #: On a `lint` cell with companions only: the cell's required work is those
    #: companions and NOT the package's own Clippy, because nothing selected the
    #: package itself. Optional, so every cell written before it stays valid.
    "companions_only": False,
    #: The planned build key this cell executes. Present exactly on an
    #: executing [`BUILD_GATES`] cell: a reused, governed, or prohibited cell
    #: consumes no build, and lint and check compile their own configurations.
    "build": False,
    #: The nextest profile this cell's gate command and its expected-test
    #: listing both select. Present exactly where `build` is, and for the same
    #: reason: a lint or check gate drives no nextest selection at all, so a
    #: profile on one is a misbinding rather than a harmless extra.
    "profile": False,
    #: Whether this cell provisions Node and pnpm before its gate. Optional and
    #: present only when true, like `requires_toolchain`: absent reads as false,
    #: so a plan written before the field existed still projects.
    "requires_node": False,
    #: The terminal backends THIS cell must prove drove a test: the sorted,
    #: non-empty subset of the package's `l2_backends` that the cell's
    #: environment can host. Present exactly on an executing L2 cell. The
    #: producer requires these through `BISCUIT_TEST_REQUIRED_BACKENDS`, records
    #: them in its expected manifest, and `completion.py` demands a proof for
    #: each — reading the package-wide list instead is what refused every
    #: mixed-backend cell before schema version 6.
    "backends": False,
}

#: One immutable compile configuration, its native owner, and every result cell
#: that executes its outputs. Identity is `{package, producer, key}`; area is
#: never part of it, and `key` is the digest of [`BUILD_IDENTITY_FIELDS`].
BUILD_FIELDS: dict[str, bool] = {
    "key": True,
    "package": True,
    #: The native environment that compiles this record. Exactly one — a build
    #: key with two owners is what plan validation exists to refuse.
    "producer": True,
    #: `build-<package>-<producer>-<key>`. Package-keyed like every other
    #: store; `build` is an internal artifact tier, never a result-cell gate.
    "artifact": True,
    #: The execution environments the producer's contract permits, whether or
    #: not this run consumes them all.
    "compatible_environments": True,
    #: Why a consumer outside the producer's own environment is allowed. Human
    #: prose for `ci-plan`; the machine rule is the environment table's.
    "compatibility_reason": True,
    #: The `{environment, gate}` cells that execute these outputs, sorted. A
    #: record with none is removed rather than left unconsumed.
    "consumers": True,
    #: Every plan-known input the key was computed from, unhashed. Stored so a
    #: reader can see *why* two cells share or split a key without recomputing
    #: the digest, and so the producer's realized manifest has something to
    #: check itself against.
    "identity": True,
}

BUILD_CONSUMER_FIELDS: dict[str, bool] = {"environment": True, "gate": True}

#: The compile-affecting inputs the planner knows. Discovered inputs — the
#: actual linker and native-library versions, the archive checksums — belong to
#: the producer's realized manifest, not here.
BUILD_IDENTITY_FIELDS: dict[str, bool] = {
    #: The plan's head. One revision is the identity of both the source tree
    #: and, with `lockfile` below, the resolved dependency graph.
    "source_commit": True,
    "lockfile": True,
    "rust": True,
    "nextest": True,
    "host": True,
    "target": True,
    "profile": True,
    "rustflags": True,
    "cargo_config": True,
    "linker": True,
    "archive_format": True,
    "package": True,
    "target_kinds": True,
    #: The package's isolated feature arguments, verbatim. Two packages with
    #: different feature graphs split keys even in one owner's target tree.
    "features": True,
    "native": True,
    "archive_includes": True,
    "sidecars": True,
}

RECEIPT_FIELDS: dict[str, bool] = {
    "schema_version": True,
    "environment": True,
    "base": True,
    "head": True,
    "tree": True,
    "scope_identity": True,
    "completion": True,
    "host": True,
    "cells": True,
}

RECEIPT_CELL_FIELDS: dict[str, bool] = {
    "package": True,
    "gate": True,
    "outcome": True,
    "exit_code": True,
    "completion": True,
    "counts": True,
    "duration_s": True,
    "gate_input_identity": True,
    "backends": True,
    "report": True,
    "failed_tests": False,
    "failure_detail_truncated": False,
    #: `{key, digest}` of the build record this cell's tests actually ran from,
    #: when the run consumed a verified archive. Optional because a cell that
    #: compiled in place has no archive to name, and because every receipt
    #: written before archives existed must stay readable.
    "build": False,
}

#: What a receipt cell's `build` names: the PLANNED key and the producer's
#: REALIZED digest, read from the manifest the consumer verified.
RECEIPT_BUILD_FIELDS: dict[str, bool] = {"key": True, "digest": True}

HOST_FIELDS: dict[str, bool] = {"os": True, "kernel": True, "report_dir": True}

SCOPE_RECEIPT_FIELDS: dict[str, bool] = {
    "schema_version": True,
    "base": True,
    "head": True,
    "tree": True,
    "plan_schema_version": True,
    #: The canonical resolved plan, exactly as `--plan-out` writes it.
    "plan": True,
    #: Legacy wire projection retained for receipt-schema compatibility;
    #: consumers derive scheduling projections from the canonical plan.
    "scope": True,
}

#: Required keys of the retained legacy wire projection. Scheduling consumers
#: re-project from `plan`; validation preserves the receipt schema contract.
SCOPE_PROJECTION_FIELDS = (
    "packages",
    "areas",
    "scheduled_areas",
    #: The dispatch rows `ci.yml` hands each area, `{area: {test, check, lint,
    #: wsl, has_*_rows}}`. A scope receipt is re-projected from its carried
    #: plan rather than read back, so this is derived with everything else —
    #: but it is what the fan-out expands, so a projection without it would
    #: schedule nothing.
    "area_rows",
    "area_slugs",
    "source_packages",
    "reverse_dependencies",
    "full_scope",
    "full_scope_gates",
    "change_class",
    "preflight_os",
    "preflight_reason",
    "policy",
    #: The deterministic native build-owner matrix, one entry per producer
    #: environment that owns at least one record. Derived from the final plan,
    #: never re-planned.
    "build_owners",
    #: The same owners flattened one entry per planned record, plus the matrix
    #: vector and `runs-on` lookup `ci.yml`'s owner job expands. Present even
    #: when empty: an all-reused plan schedules no owner and must say so.
    "build_slices",
    "build_artifacts",
    "build_runners",
    "job_estimate",
    "flags",
)

COUNT_FIELDS = ("total", "passed", "failed", "skipped", "errored")

#: The producer's expected-test manifest. Every provenance field is required:
#: a listing that cannot say which environment, tier, target, nextest binary,
#: transport, and selection produced it is not comparable to any report, and
#: the one thing a v1 manifest could not express — why an identity is absent —
#: is exactly what the tightened skip rule now turns into a failure.
EXPECTED_MANIFEST_FIELDS: dict[str, bool] = {
    "schema_version": True,
    "environment": True,
    "tier": True,
    #: The target triple the listed binaries were COMPILED for, read from the
    #: listing itself. In archive mode that is the producer's target, not the
    #: consumer's host — which is the point: `cfg` decided what exists when the
    #: archive was built.
    "target": True,
    "nextest_version": True,
    "from_archive": True,
    "selection": True,
    "packages": True,
    #: The L2 terminal backends and companion suites the producer was told to
    #: require. Optional, and cross-checked against the plan rather than trusted:
    #: the plan is the source of truth, and a disagreement means the job was
    #: provisioned for different work than the plan scheduled.
    "backends": False,
    "companion_suites": False,
}

#: What the listing selected with. `filter` is `_tier_filter`'s expression for
#: this tier (package-scoped in archive mode); `profile` and `test_args` are the
#: cell's and the package's, so a producer that ran something else says so.
EXPECTED_SELECTION_FIELDS: dict[str, bool] = {
    "filter": True,
    "profile": True,
    "test_args": True,
}

#: One package's listing. Three disjoint identity sets: `tests` is what must
#: report a result, `ignored` what `#[ignore]` excused, and `excluded` what the
#: tier expression did not select. A test compiled out by `cfg` appears in none
#: of them — it does not exist on this target.
EXPECTED_PACKAGE_FIELDS: dict[str, bool] = {
    "tests": True,
    "ignored": True,
    "excluded": True,
}

#: The completion record, keyed like every other stored result. `complete` is
#: never written false by a successful validation — the record exists only when
#: the producer proved its cell, so an absent record and a false one are the
#: same verdict to the audit (ruling R10).
COMPLETION_RECORD_FIELDS: dict[str, bool] = {
    "schema_version": True,
    "package": True,
    "environment": True,
    "gate": True,
    "complete": True,
    #: The revision the plan was resolved for and this job tested.
    "head": True,
    "run": True,
    "attempt": True,
    #: The nextest binary that both listed and ran this cell's tests (R4).
    #: Present exactly when the cell ran tests: a check or lint gate drives no
    #: nextest at all, and a placeholder there would read as a version.
    "nextest_version": False,
    #: Every staged report the comparison read, relative to the staging tree.
    #: The audit's inventory check: a record naming a report nothing uploaded
    #: is unproven coverage.
    "reports": True,
    #: The planned build key this cell consumed, present exactly when the cell
    #: references one.
    "build": False,
    #: The package's `input_paths` — what this gate's identity is computed over.
    #: Optional because a plan may omit them, which simply makes the cell
    #: exact-tree-only for reuse.
    "gate_inputs": False,
    #: The companion suites and L2 backends this cell proved, when it owed any.
    "companions": False,
    "backends": False,
}


def contract() -> dict[str, Any]:
    """The whole field contract, for cross-language assertion."""
    return {
        "resolved_plan": {
            "schema_version": RESOLVED_PLAN_SCHEMA_VERSION,
            "document": RESOLVED_PLAN_FIELDS,
            "area": AREA_FIELDS,
            "package": PACKAGE_FIELDS,
            "cell": CELL_FIELDS,
            "build": BUILD_FIELDS,
            "build_consumer": BUILD_CONSUMER_FIELDS,
            "build_identity": BUILD_IDENTITY_FIELDS,
            "change_inventory": CHANGE_INVENTORY_FIELDS,
            "skip_policy": SKIP_POLICY_FIELDS,
            "skip_entry": SKIP_ENTRY_FIELDS,
            "row": list(ROW_FIELDS),
            "row_sets": list(ROW_SET_NAMES),
            "archive_guard": ARCHIVE_GUARD_FIELDS,
        },
        "receipt": {
            "schema_version": RECEIPT_SCHEMA_VERSION,
            "legacy_schema_version": LEGACY_RECEIPT_SCHEMA_VERSION,
            "document": RECEIPT_FIELDS,
            "cell": RECEIPT_CELL_FIELDS,
            "cell_build": RECEIPT_BUILD_FIELDS,
            "host": HOST_FIELDS,
            "counts": list(COUNT_FIELDS),
            "failure_detail_limit": FAILURE_DETAIL_LIMIT,
            "unrecorded_measurement": UNRECORDED_MEASUREMENT,
        },
        "expected_manifest": {
            "schema_version": EXPECTED_MANIFEST_SCHEMA_VERSION,
            "document": EXPECTED_MANIFEST_FIELDS,
            "selection": EXPECTED_SELECTION_FIELDS,
            "package": EXPECTED_PACKAGE_FIELDS,
        },
        "completion_record": {
            "schema_version": COMPLETION_RECORD_SCHEMA_VERSION,
            "document": COMPLETION_RECORD_FIELDS,
            "rejections": list(COMPLETION_REJECTIONS),
        },
        "cell_contract": {
            "rejections": list(CELL_CONTRACT_REJECTIONS),
        },
        "scope_receipt": {
            "schema_version": SCOPE_RECEIPT_SCHEMA_VERSION,
            "document": SCOPE_RECEIPT_FIELDS,
            "projection": list(SCOPE_PROJECTION_FIELDS),
            "rejections": list(SCOPE_REJECTIONS),
        },
        "vocabulary": {
            "environments": list(ENVIRONMENTS),
            "gates": list(GATES),
            "build_gates": list(BUILD_GATES),
            "target_kinds": list(TARGET_KINDS),
            "executions": list(EXECUTIONS),
            "origins": list(ORIGINS),
            "cell_states": list(CELL_STATES),
            "execution_paths": list(EXECUTION_PATHS),
            "ci_profile": CI_PROFILE,
            "change_buckets": list(CHANGE_BUCKETS),
            "archive_guard_modes": list(ARCHIVE_GUARD_MODES),
            "accepted_gap_state": ACCEPTED_GAP_STATE,
            "completions": list(COMPLETIONS),
            "outcomes": list(OUTCOMES),
            "rejections": list(REJECTIONS),
            "build_rejections": list(BUILD_REJECTIONS),
            "completion_rejections": list(COMPLETION_REJECTIONS),
            "tier_markers": list(TIER_MARKERS),
            "canonical_selection": CANONICAL_SELECTION,
        },
    }


def canonical(document: dict[str, Any]) -> str:
    """The byte form persisted to a Git note or a scope artifact.

    Sorted and separator-tight so a receipt round-trips to identical bytes;
    evidence comparison is a byte comparison, never a structural one.
    """
    return json.dumps(document, sort_keys=True, separators=(",", ":"))


def identity(text: str) -> str:
    """The hex identity of a canonical byte string.

    One function so a scope identity and a gate-input identity are comparable
    artifacts rather than two conventions. The width satisfies the `[0-9a-f]
    {8,64}` shape both receipt fields are validated against.
    """
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


# ---------------------------------------------------------------------------
# Validation
# ---------------------------------------------------------------------------


def _keys(
    where: str, value: Any, fields: dict[str, bool], code: str = "malformed-receipt"
) -> list[str]:
    if not isinstance(value, dict):
        return [f"{code}: {where} must be an object"]
    problems = [
        f"{code}: {where} is missing required field '{name}'"
        for name, required in fields.items()
        if required and name not in value
    ]
    problems += [
        f"{code}: {where} has unknown field '{name}'"
        for name in sorted(value)
        if name not in fields
    ]
    return problems


def _member(where: str, value: Any, allowed: tuple[str, ...], code: str) -> list[str]:
    if value in allowed:
        return []
    return [f"{code}: {where} is {value!r}, expected one of {list(allowed)}"]


def _sha(where: str, value: Any) -> list[str]:
    if isinstance(value, str) and _SHA.fullmatch(value):
        return []
    return [f"malformed-receipt: {where} is not a full Git object ID: {value!r}"]


def _str_list(where: str, value: Any, allowed: tuple[str, ...] | None = None) -> list[str]:
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        return [f"malformed-receipt: {where} must be a list of strings"]
    if allowed is None:
        return []
    return [
        f"malformed-receipt: {where} contains unknown value {item!r}"
        for item in value
        if item not in allowed
    ]


def _is_normalized_relative_path(entry: str) -> bool:
    """Whether `entry` is one POSIX-spelled path inside the repository.

    Containment is part of normalization here, not a separate rule: every
    consumer resolves a plan path against the checkout root, and `/etc/passwd`,
    `C:/windows`, or `pkg/../../outside.rs` each name a file nothing that read
    this plan agreed to read.
    """
    if not entry or entry != entry.strip():
        return False
    if "\\" in entry or entry.startswith("./"):
        return False
    if entry.startswith("/"):
        return False
    if _DRIVE.match(entry):
        return False
    return ".." not in entry.split("/")


def _unnormalized(where: str, entries: list[str]) -> list[str]:
    return [
        f"malformed-receipt: {where} carries {entry!r}, which is not a "
        "normalized repository-relative path"
        for entry in entries
        if not _is_normalized_relative_path(entry)
    ]


def _normalized_path_list(where: str, value: Any) -> list[str]:
    """Every reason `value` is not a sorted, de-duplicated, normalized path list.

    One spelling per path, so a Windows-spelled diff and a POSIX-spelled one
    produce the same document and two planners cannot disagree about whether a
    file is in scope.
    """
    problems = _str_list(where, value)
    if problems:
        return problems
    if list(value) != sorted(value):
        problems.append(f"malformed-receipt: {where} is not sorted")
    if len(set(value)) != len(value):
        problems.append(f"malformed-receipt: {where} repeats a path")
    return problems + _unnormalized(where, list(value))


def _archive_guard(value: Any) -> list[str]:
    """Every reason `value` is not a valid archive-path guard scope.

    The rules the guard itself refuses to guess at: an unselected guard
    describes no scope, a `changed` scan always names its paths (an explicitly
    empty list is the real "nothing eligible changed" state and is never an
    implicit full scan), and a `full` scan never carries a list a reader could
    mistake for the whole of what was checked.
    """
    where = "resolved plan archive_guard"
    problems = _keys(where, value, ARCHIVE_GUARD_FIELDS)
    if problems:
        return problems

    selected = value["selected"]
    if not isinstance(selected, bool):
        return [f"malformed-receipt: {where} selected must be a boolean"]
    if not isinstance(value["reason"], str) or not value["reason"].strip():
        problems.append(
            f"malformed-receipt: {where} must state why it holds the scope it does"
        )

    if not selected:
        problems += [
            f"malformed-receipt: {where} selected nothing and must not carry {name!r}"
            for name in ("mode", "paths")
            if name in value
        ]
        return problems

    if "mode" not in value:
        return problems + [
            f"malformed-receipt: {where} is selected and must name a scan mode"
        ]
    problems += _member(f"{where} mode", value["mode"], ARCHIVE_GUARD_MODES, "malformed-receipt")

    if value["mode"] == "changed":
        if "paths" not in value:
            problems.append(
                f"malformed-receipt: {where} is a changed scan and must name its "
                "paths; an empty scan is spelled as an explicit empty list"
            )
        else:
            problems += _normalized_path_list(f"{where} paths", value["paths"])
    elif value["mode"] == "full" and "paths" in value:
        problems.append(
            f"malformed-receipt: {where} is a full scan and must not carry paths; "
            "a listed subset would read as the whole of what was checked"
        )
    return problems


def _change_inventory(value: Any) -> list[str]:
    """Every reason `value` is not a valid change inventory, or an empty list.

    The invariants a reader depends on: one normalized spelling per path, one
    bucket per path, sorted buckets, and counts that were derived from the
    lists rather than reported alongside them.
    """
    where = "resolved plan change_inventory"
    problems = _keys(where, value, CHANGE_INVENTORY_FIELDS)
    if problems:
        return problems

    available = value["diff_available"]
    if not isinstance(available, bool):
        return [f"malformed-receipt: {where} diff_available must be a boolean"]

    if not available:
        problems = [
            f"malformed-receipt: {where} reports no diff and must not carry {name!r}"
            for name in ("paths", "counts", "deleted")
            if name in value
        ]
        if not isinstance(value.get("reason"), str) or not value.get("reason"):
            problems.append(
                f"malformed-receipt: {where} reports no diff and must state why"
            )
        return problems

    if "reason" in value:
        problems.append(
            f"malformed-receipt: {where} carries a diff inventory and must not "
            "also carry an absence reason"
        )
    for name in ("paths", "counts", "deleted"):
        if name not in value:
            problems.append(
                f"malformed-receipt: {where} carries a diff and is missing {name!r}"
            )
    if problems:
        return problems

    problems += _normalized_path_list(f"{where} deleted", value["deleted"])

    paths, counts = value["paths"], value["counts"]
    if not isinstance(paths, dict) or sorted(paths) != sorted(CHANGE_BUCKETS):
        return [
            f"malformed-receipt: {where} paths must name exactly "
            f"{list(CHANGE_BUCKETS)}"
        ]
    if not isinstance(counts, dict) or sorted(counts) != sorted(
        (*CHANGE_BUCKETS, "total")
    ):
        return [
            f"malformed-receipt: {where} counts must name exactly "
            f"{[*CHANGE_BUCKETS, 'total']}"
        ]

    seen: set[str] = set()
    for bucket in CHANGE_BUCKETS:
        entries = paths[bucket]
        problems += _str_list(f"{where} paths {bucket}", entries)
        if not isinstance(entries, list) or not all(
            isinstance(entry, str) for entry in entries
        ):
            continue
        if list(entries) != sorted(entries):
            problems.append(f"malformed-receipt: {where} {bucket} is not sorted")
        problems += _unnormalized(f"{where} {bucket}", entries)
        problems += [
            f"malformed-receipt: {where} places {entry!r} in more than one bucket"
            for entry in entries
            if entry in seen or entries.count(entry) > 1
        ]
        seen.update(entries)
        if counts[bucket] != len(entries):
            problems.append(
                f"malformed-receipt: {where} counts {bucket} is "
                f"{counts[bucket]!r} but the bucket holds {len(entries)}"
            )
    if counts["total"] != len(seen):
        problems.append(
            f"malformed-receipt: {where} counts total is {counts['total']!r} "
            f"but the buckets hold {len(seen)} paths"
        )
    return problems


def _as_date(value: Any) -> date | None:
    """`value` as a calendar date, or `None` when it is not one."""
    if isinstance(value, date):
        return value
    if not isinstance(value, str):
        return None
    try:
        return date.fromisoformat(value)
    except ValueError:
        return None


def _skip_policy(value: Any, cells: set[tuple[str, str, str]], today: date) -> list[str]:
    """Every reason `value` is not a valid exact-skip snapshot (ruling R8).

    The provenance code covers the snapshot's own shape, not only a malformed
    hash: a snapshot that cannot say which file it read and what that file
    hashed to excuses nothing, however well-formed its entries are.
    """
    problems = _keys(
        "resolved plan skip_policy", value, SKIP_POLICY_FIELDS, "skip-policy-provenance"
    )
    if problems:
        return problems

    if not isinstance(value["source"], str) or not value["source"]:
        problems.append(
            "skip-policy-provenance: resolved plan skip_policy names no source file"
        )
    content_hash = value["content_hash"]
    if not isinstance(content_hash, str) or not _IDENTITY.fullmatch(content_hash):
        problems.append(
            f"skip-policy-provenance: resolved plan skip_policy content_hash "
            f"{content_hash!r} is not a content digest"
        )

    entries = value["entries"]
    if not isinstance(entries, list):
        return problems + [
            "malformed-receipt: resolved plan skip_policy entries must be a list"
        ]
    for index, entry in enumerate(entries):
        label = f"resolved plan skip_policy entry {index}"
        entry_problems = _keys(label, entry, SKIP_ENTRY_FIELDS)
        problems += entry_problems
        if entry_problems or not isinstance(entry, dict):
            continue
        problems += _member(
            f"{label} environment", entry["environment"], ENVIRONMENTS, "unknown-environment"
        )
        problems += _member(f"{label} gate", entry["gate"], GATES, "unknown-gate")
        problems += [
            f"malformed-receipt: {label} has no {field}"
            for field in ("owner", "reason", "source_run")
            if not entry[field]
        ]
        if "tests" in entry:
            problems += _str_list(f"{label} tests", entry["tests"])
        if "backend" in entry and not (
            isinstance(entry["backend"], str) and entry["backend"]
        ):
            problems.append(f"malformed-receipt: {label} names an empty backend")
        key = (entry["package"], entry["environment"], entry["gate"])
        if key not in cells:
            problems.append(
                f"skip-policy-cell: {label} approves {key[0]}/{key[1]}/{key[2]}, "
                "which this plan does not carry; an approval nothing can bind to "
                "is a policy the audit would never apply"
            )
        if "expiry" in entry:
            expiry = _as_date(entry["expiry"])
            if expiry is None:
                problems.append(
                    f"skip-policy-provenance: {label} expiry {entry['expiry']!r} is "
                    "not a YYYY-MM-DD date"
                )
            elif expiry < today:
                problems.append(
                    f"skip-policy-expired: {label} expired on {entry['expiry']}; an "
                    "expired approval is refused at planning time, not discovered by "
                    "a producer"
                )
    return problems


def _rows(value: Any, areas: set[str]) -> list[str]:
    """Every reason `value` is not a valid dispatch-row document.

    Shape, key order, area membership, and global uniqueness only. The join
    back to the cells is deliberately not asserted here: `row_sets` derives the
    document from the plan's cells in one place, and re-deriving it inside
    validation would make the check agree with itself rather than with the
    scheduler.
    """
    if not isinstance(value, dict):
        return ["malformed-receipt: resolved plan rows must be an object"]
    problems: list[str] = []
    seen: dict[tuple[str, str, str], str] = {}
    for area in sorted(value):
        document = value[area]
        if area not in areas:
            problems.append(
                f"malformed-receipt: resolved plan rows name area {area!r}, which "
                "the plan does not select"
            )
        if not isinstance(document, dict):
            problems.append(f"malformed-receipt: resolved plan rows {area} must be an object")
            continue
        expected = {*ROW_SET_NAMES, *(f"has_{name}_rows" for name in ROW_SET_NAMES)}
        if set(document) != expected:
            problems.append(
                f"malformed-receipt: resolved plan rows {area} must carry exactly "
                f"{sorted(expected)}"
            )
            continue
        for name in ROW_SET_NAMES:
            rows = document[name]
            if not isinstance(rows, list):
                problems.append(
                    f"malformed-receipt: resolved plan rows {area} {name} must be a list"
                )
                continue
            if document[f"has_{name}_rows"] is not bool(rows):
                problems.append(
                    f"malformed-receipt: resolved plan rows {area} has_{name}_rows "
                    "must be true exactly when the row set is nonempty"
                )
            for row in rows:
                # The key SET, not the key order: [`canonical`] sorts keys, so
                # a plan that has been persisted once no longer spells a row in
                # R1's order. That order is the emitter's contract — it is what
                # GitHub renders a matrix job's label from, and `row_sets`
                # builds every row in it — and it survives the `scope.json`
                # projection the workflow actually expands.
                if not isinstance(row, dict) or set(row) != set(ROW_FIELDS):
                    problems.append(
                        f"malformed-receipt: resolved plan rows {area} {name} carries "
                        f"{row!r}, which is not a {list(ROW_FIELDS)} dispatch row"
                    )
                    continue
                problems += _member(
                    f"resolved plan rows {area} {name} environment",
                    row["environment"],
                    ENVIRONMENTS,
                    "unknown-environment",
                )
                problems += _member(
                    f"resolved plan rows {area} {name} gate", row["gate"], GATES, "unknown-gate"
                )
                key = (row["package"], row["environment"], row["gate"])
                if key in seen:
                    problems.append(
                        f"conflicting-evidence: resolved plan rows dispatch "
                        f"{key[0]}/{key[1]}/{key[2]} from both {seen[key]} and "
                        f"{area}/{name}; one executing cell is exactly one row"
                    )
                seen[key] = f"{area}/{name}"
    return problems


def validate_resolved_plan(document: Any, today: Any = None) -> list[str]:
    """Every reason `document` is not a valid resolved plan, or an empty list.

    `today` is the date an approval's expiry is judged against, defaulting to
    the current date. A caller that validates a stored document as of some
    other day passes it explicitly.

    ## Returns

    Problems in document order. Each begins with a code from [`REJECTIONS`] so
    a caller can classify without parsing prose.
    """
    judged = _as_date(today) or date.today()
    # Version before shape, deliberately. A document from an older schema
    # usually differs in BOTH, and the field check would then report the field
    # it lacks — sending the reader after a corrupt document when the answer is
    # that this tool moved on. Guarded on presence because the field check
    # below is what proves the key exists at all.
    if isinstance(document, dict) and "schema_version" in document:
        if document["schema_version"] != RESOLVED_PLAN_SCHEMA_VERSION:
            return [
                f"unknown-schema-version: resolved plan is version "
                f"{document['schema_version']!r}, this tool writes "
                f"{RESOLVED_PLAN_SCHEMA_VERSION}"
            ]

    problems = _keys("resolved plan", document, RESOLVED_PLAN_FIELDS)
    if problems:
        return problems

    for field in ("base", "head"):
        problems += _sha(f"resolved plan {field}", document[field])
    problems += _member(
        "resolved plan change_class",
        document["change_class"],
        ("full", "package", "documentation"),
        "malformed-receipt",
    )
    problems += _change_inventory(document["change_inventory"])
    problems += _archive_guard(document["archive_guard"])

    areas = {}
    for entry in document["areas"]:
        problems += _keys("area record", entry, AREA_FIELDS)
        if isinstance(entry, dict) and "area" in entry:
            areas[entry["area"]] = entry
            if not entry.get("selection_reason"):
                problems.append(
                    f"malformed-receipt: area {entry['area']!r} has no selection reason"
                )
            if "execution_path" in entry:
                problems += _member(
                    f"area {entry['area']} execution_path",
                    entry["execution_path"],
                    EXECUTION_PATHS,
                    "malformed-receipt",
                )

    packages = {}
    for entry in document["packages"]:
        problems += _keys("package record", entry, PACKAGE_FIELDS)
        if not isinstance(entry, dict) or "package" not in entry:
            continue
        packages[entry["package"]] = entry
        if entry.get("area") not in areas:
            problems.append(
                f"unknown-package: package {entry['package']!r} names area "
                f"{entry.get('area')!r}, which the plan does not select"
            )
        problems += _str_list(
            f"package {entry['package']} gates", entry.get("gates"), GATES
        )
        problems += _str_list(
            f"package {entry['package']} targets", entry.get("targets"), TARGET_KINDS
        )
        if not entry.get("selection_reason"):
            problems.append(
                f"malformed-receipt: package {entry['package']!r} has no selection reason"
            )
        if "input_paths" in entry:
            problems += _str_list(
                f"package {entry['package']} input_paths", entry["input_paths"]
            )
        if not isinstance(entry.get("l1_include_slow"), bool):
            problems.append(
                f"malformed-receipt: package {entry['package']} l1_include_slow must be a boolean"
            )
        if ("exclusion" in entry) != (entry.get("gates") == []):
            problems.append(
                f"malformed-receipt: package {entry['package']} carries an exclusion "
                "exactly when it gates nothing"
            )
        if "dependent_seam" in entry:
            problems += _dependent_seam(entry["package"], entry["dependent_seam"])

    if not isinstance(document["environments"], list) or not all(
        isinstance(environment, dict) and isinstance(environment.get("name"), str)
        for environment in document["environments"]
    ):
        problems.append(
            "malformed-receipt: resolved plan environments must be a list of named records"
        )

    problems += _str_list("resolved plan source_packages", document["source_packages"])
    problems += _str_list(
        "resolved plan reverse_dependencies", document["reverse_dependencies"]
    )
    if isinstance(document["reverse_dependencies"], list):
        # AC1, machine-checked: naming a dependent is reporting, never selecting.
        problems += [
            f"unknown-package: {name!r} is reported as an unchanged reverse "
            "dependency and must not also hold a package record"
            for name in document["reverse_dependencies"]
            if name in packages
        ]
    for name, entry in packages.items():
        seam = entry.get("dependent_seam")
        if isinstance(seam, dict) and isinstance(seam.get("dependents"), list):
            # The same rule for the compiled seam: a dependent is compiled
            # inside the changed package's cell BECAUSE it is not selected.
            problems += [
                f"unknown-package: {dependent!r} is compiled as a dependent of "
                f"{name!r} and must not also hold a package record"
                for dependent in seam["dependents"]
                if dependent in packages
            ]

    for entry in document["cells"]:
        problems += _keys("cell record", entry, CELL_FIELDS)
        if not isinstance(entry, dict) or "package" not in entry:
            continue
        label = f"cell {entry.get('package')}/{entry.get('environment')}/{entry.get('gate')}"
        problems += _member(f"{label} environment", entry.get("environment"), ENVIRONMENTS, "unknown-environment")
        problems += _member(f"{label} gate", entry.get("gate"), GATES, "unknown-gate")
        problems += _member(f"{label} execution", entry.get("execution"), EXECUTIONS, "malformed-receipt")
        problems += _member(f"{label} origin", entry.get("origin"), ORIGINS, "malformed-receipt")
        problems += _member(f"{label} state", entry.get("state"), CELL_STATES, "malformed-receipt")
        problems += _str_list(f"{label} target_kinds", entry.get("target_kinds"), TARGET_KINDS)
        if entry["package"] not in packages:
            problems.append(
                f"unknown-package: {label} has no package record; package is the "
                "stored identity and every cell must carry one"
            )
        elif entry.get("area") != packages[entry["package"]].get("area"):
            problems.append(
                f"malformed-receipt: {label} claims area {entry.get('area')!r} but its "
                f"package record says {packages[entry['package']].get('area')!r}; area is "
                "derived from package, never independently assigned"
            )
        if not entry.get("selection_reason"):
            problems.append(f"malformed-receipt: {label} has no selection reason")
        if "dependents" in entry:
            problems += _str_list(f"{label} dependents", entry["dependents"])
            if entry.get("gate") != "check":
                problems.append(
                    f"malformed-receipt: {label} carries dependents but only a check "
                    "cell compiles them"
                )
        problems += _cell_backends(label, entry, packages.get(entry["package"]))
        problems += _cell_consistency(label, entry)

    problems += _build_records(document, packages)
    problems += _skip_policy(
        document["skip_policy"],
        {
            (entry["package"], entry["environment"], entry["gate"])
            for entry in document["cells"]
            if isinstance(entry, dict)
            and {"package", "environment", "gate"} <= set(entry)
        },
        judged,
    )
    if "rows" in document:
        problems += _rows(document["rows"], set(areas))

    if "evidence_rejections" in document:
        problems += _str_list(
            "resolved plan evidence_rejections", document["evidence_rejections"]
        )
    if not isinstance(document["job_estimate"], int) or document["job_estimate"] < 0:
        problems.append("malformed-receipt: resolved plan job_estimate must be a non-negative integer")
    if "event" in document:
        problems += _member("resolved plan event", document["event"], EVENTS, "malformed-receipt")
    scheduled = {
        environment.get("name")
        for environment in document["environments"]
        if isinstance(environment, dict)
    }
    for field, key in (("deferred_environments", "events"), ("proven_environments", "event")):
        if field not in document:
            continue
        entries = document[field]
        if not isinstance(entries, list) or not entries:
            problems.append(f"malformed-receipt: resolved plan {field} must be a non-empty list")
            continue
        for entry in entries:
            if not isinstance(entry, dict) or set(entry) != {"name", key}:
                problems.append(
                    f"malformed-receipt: resolved plan {field} entries carry exactly "
                    f"'name' and '{key}'"
                )
                continue
            problems += _member(f"resolved plan {field} name", entry["name"], ENVIRONMENTS, "unknown-environment")
            if entry["name"] in scheduled:
                problems.append(
                    f"malformed-receipt: resolved plan {field} names {entry['name']!r}, "
                    "which the plan also schedules"
                )
            if key == "events":
                problems += _str_list(f"resolved plan {field} events", entry["events"], EVENTS)
            else:
                problems += _member(f"resolved plan {field} event", entry["event"], EVENTS, "malformed-receipt")
    if "producing_environments" in document:
        entries = document["producing_environments"]
        if not isinstance(entries, list) or not entries:
            problems.append(
                "malformed-receipt: resolved plan producing_environments must be a non-empty list"
            )
        else:
            for entry in entries:
                if not isinstance(entry, dict) or set(entry) != {"name", "for"}:
                    problems.append(
                        "malformed-receipt: resolved plan producing_environments entries carry "
                        "exactly 'name' and 'for'"
                    )
                    continue
                problems += _member(
                    "resolved plan producing_environments name", entry["name"], ENVIRONMENTS, "unknown-environment"
                )
                # A producer is in the table — its owner job and preflight
                # runner come from there — and its guests are scheduled ones.
                if entry["name"] not in scheduled:
                    problems.append(
                        f"malformed-receipt: resolved plan producing_environments names "
                        f"{entry['name']!r}, which is not in the plan's environment table"
                    )
                guests = entry["for"]
                if not isinstance(guests, list) or not guests or not all(
                    isinstance(guest, str) and guest in scheduled and guest != entry["name"]
                    for guest in guests
                ):
                    problems.append(
                        f"malformed-receipt: resolved plan producing_environments entry "
                        f"{entry['name']!r} must name scheduled guests it produces for"
                    )
    return problems


def _build_records(document: dict[str, Any], packages: dict[str, Any]) -> list[str]:
    """Every reason the plan's build ownership is invalid, or an empty list.

    The invariants are the specification's Design Decision 1, machine-checked
    before any workflow acts on the plan: one compatible owner per executing
    test cell, no build key with two owners, no dangling reference, no
    unconsumed build, and nothing at all attached to lint or check.
    """
    builds = document.get("builds")
    if not isinstance(builds, list):
        return ["malformed-receipt: resolved plan builds must be a list"]

    problems: list[str] = []
    by_key: dict[str, dict[str, Any]] = {}
    environment_names = {
        environment.get("name")
        for environment in document.get("environments", [])
        if isinstance(environment, dict)
    }

    for entry in builds:
        problems += _keys("build record", entry, BUILD_FIELDS)
        if not isinstance(entry, dict) or "key" not in entry:
            continue
        key = entry["key"]
        label = f"build {entry.get('package')}/{entry.get('producer')}/{key}"
        if not isinstance(key, str) or not _BUILD_KEY.fullmatch(key):
            problems.append(
                f"malformed-receipt: {label} key is not a 16-digit hex build key; the "
                "key is computed only through `ci-build key`"
            )
        if key in by_key:
            problems.append(
                f"malformed-receipt: build key {key} has two owners, "
                f"{by_key[key].get('producer')!r} and {entry.get('producer')!r}; one "
                "key is compiled exactly once"
            )
            continue
        by_key[key] = entry

        if entry.get("package") not in packages:
            problems.append(
                f"unknown-package: {label} names a package the plan does not select"
            )
        problems += _str_list(
            f"{label} compatible_environments",
            entry.get("compatible_environments"),
            ENVIRONMENTS,
        )
        compatible = entry.get("compatible_environments")
        if isinstance(compatible, list):
            if compatible != sorted(set(compatible)):
                problems.append(
                    f"malformed-receipt: {label} compatible_environments must be sorted "
                    "and free of duplicates"
                )
            if entry.get("producer") not in compatible:
                problems.append(
                    f"malformed-receipt: {label} producer is not among its own "
                    "compatible_environments"
                )
        if entry.get("producer") not in environment_names:
            problems.append(
                f"unknown-environment: {label} names producer "
                f"{entry.get('producer')!r}, which is not in the plan's environment table"
            )
        expected_artifact = (
            f"build-{entry.get('package')}-{entry.get('producer')}-{key}"
        )
        if entry.get("artifact") != expected_artifact:
            problems.append(
                f"malformed-receipt: {label} artifact is {entry.get('artifact')!r}, "
                f"expected {expected_artifact!r}"
            )
        if not isinstance(entry.get("compatibility_reason"), str) or not entry[
            "compatibility_reason"
        ]:
            problems.append(f"malformed-receipt: {label} has no compatibility reason")
        problems += _build_consumers(label, entry, compatible)
        problems += _build_identity(label, entry)

    problems += _build_references(document, by_key)
    return problems


def _build_consumers(
    label: str, entry: dict[str, Any], compatible: Any
) -> list[str]:
    consumers = entry.get("consumers")
    if not isinstance(consumers, list) or not consumers:
        return [
            f"malformed-receipt: {label} has no consumer; a build nothing executes is "
            "removed rather than scheduled"
        ]
    problems: list[str] = []
    seen = []
    for consumer in consumers:
        problems += _keys(f"{label} consumer", consumer, BUILD_CONSUMER_FIELDS)
        if not isinstance(consumer, dict):
            continue
        problems += _member(
            f"{label} consumer gate", consumer.get("gate"), BUILD_GATES, "unknown-gate"
        )
        problems += _member(
            f"{label} consumer environment",
            consumer.get("environment"),
            ENVIRONMENTS,
            "unknown-environment",
        )
        if isinstance(compatible, list) and consumer.get("environment") not in compatible:
            problems.append(
                f"malformed-receipt: {label} is consumed in "
                f"{consumer.get('environment')!r}, which its producer's contract does "
                "not declare compatible"
            )
        seen.append((consumer.get("environment"), consumer.get("gate")))
    if seen != sorted(set(seen), key=lambda pair: (str(pair[0]), str(pair[1]))):
        problems.append(
            f"malformed-receipt: {label} consumers must be sorted by "
            "{environment, gate} and free of duplicates"
        )
    return problems


def _build_identity(label: str, entry: dict[str, Any]) -> list[str]:
    identity = entry.get("identity")
    problems = _keys(f"{label} identity", identity, BUILD_IDENTITY_FIELDS)
    if problems or not isinstance(identity, dict):
        return problems
    if identity.get("package") != entry.get("package"):
        problems.append(
            f"malformed-receipt: {label} identity names package "
            f"{identity.get('package')!r}"
        )
    problems += _str_list(f"{label} identity target_kinds", identity.get("target_kinds"), TARGET_KINDS)
    for field in ("cargo_config", "native", "archive_includes", "sidecars"):
        problems += _str_list(f"{label} identity {field}", identity.get(field))
        value = identity.get(field)
        if isinstance(value, list) and value != sorted(value):
            problems.append(f"malformed-receipt: {label} identity {field} must be sorted")
    for field in ("source_commit", "lockfile", "rust", "nextest", "host", "target", "profile", "linker", "archive_format"):
        if not isinstance(identity.get(field), str) or not identity[field]:
            problems.append(
                f"malformed-receipt: {label} identity {field} must be a non-empty string"
            )
    if not isinstance(identity.get("rustflags"), str) or not isinstance(
        identity.get("features"), str
    ):
        problems.append(
            f"malformed-receipt: {label} identity rustflags and features must be "
            "strings; an absent value is the empty string"
        )
    return problems


def _build_references(
    document: dict[str, Any], by_key: dict[str, dict[str, Any]]
) -> list[str]:
    """The cell side of build ownership: every reference and every demand."""
    problems: list[str] = []
    demanded: dict[str, list[tuple[str, str]]] = {key: [] for key in by_key}
    for entry in document.get("cells", []):
        if not isinstance(entry, dict):
            continue
        label = f"cell {entry.get('package')}/{entry.get('environment')}/{entry.get('gate')}"
        reference = entry.get("build")
        executes_a_tier = (
            entry.get("execution") == "execute" and entry.get("gate") in BUILD_GATES
        )
        if reference is None:
            if executes_a_tier:
                problems.append(
                    f"malformed-receipt: {label} will execute but references no build; "
                    "a test execution without one would compile its own"
                )
            continue
        if not executes_a_tier:
            problems.append(
                f"malformed-receipt: {label} references build {reference!r}, but only an "
                "executing L1, L2, or browser cell consumes one"
            )
            continue
        record = by_key.get(reference)
        if record is None:
            problems.append(
                f"malformed-receipt: {label} references build {reference!r}, which the "
                "plan does not carry"
            )
            continue
        if record.get("package") != entry.get("package"):
            problems.append(
                f"malformed-receipt: {label} references build {reference!r}, which "
                f"compiles {record.get('package')!r}"
            )
        demanded[reference].append((entry.get("environment"), entry.get("gate")))

    for key, record in by_key.items():
        consumers = record.get("consumers")
        if not isinstance(consumers, list):
            continue
        listed = sorted(
            (consumer.get("environment"), consumer.get("gate"))
            for consumer in consumers
            if isinstance(consumer, dict)
        )
        if listed != sorted(demanded[key]):
            problems.append(
                f"malformed-receipt: build {record.get('package')}/{key} lists consumers "
                f"{listed} but the cells referencing it are {sorted(demanded[key])}"
            )
    return problems


def _dependent_seam(package: str, seam: Any) -> list[str]:
    where = f"package {package} dependent_seam"
    if not isinstance(seam, dict):
        return [f"malformed-receipt: {where} must be an object"]
    problems = _keys(where, seam, {"dependents": True, "check_args": True, "native": True})
    if problems:
        return problems
    problems += _str_list(f"{where} dependents", seam["dependents"])
    problems += _str_list(f"{where} native", seam["native"])
    if not seam["dependents"]:
        problems.append(
            f"malformed-receipt: {where} names no dependent; a record with none "
            "to compile carries no seam at all"
        )
    if not isinstance(seam["check_args"], str) or not seam["check_args"]:
        problems.append(f"malformed-receipt: {where} check_args must be a non-empty string")
    return problems


def _cell_backends(
    label: str, entry: dict[str, Any], package: dict[str, Any] | None
) -> list[str]:
    """Every reason a cell's `backends` disagrees with its gate or its package.

    An executing L2 cell with none would make `completion.py` guess what the
    producer had to prove; one on any other cell binds a proof to a gate that
    drives no backend.
    """
    problems: list[str] = []
    executing_l2 = entry.get("execution") == "execute" and entry.get("gate") == "L2"
    if "backends" not in entry:
        if executing_l2:
            problems.append(
                f"malformed-receipt: {label} will execute the L2 tier but names no "
                "required backend; the producer could not know what to prove"
            )
        return problems
    if not executing_l2:
        problems.append(
            f"malformed-receipt: {label} carries required backends but is not an "
            "executing L2 cell"
        )
    backends = entry["backends"]
    if not isinstance(backends, list) or not backends:
        problems.append(
            f"malformed-receipt: {label} backends must be a non-empty list of strings"
        )
        return problems
    problems += _str_list(f"{label} backends", backends)
    declared = package.get("l2_backends") if isinstance(package, dict) else None
    if isinstance(declared, list):
        problems += [
            f"malformed-receipt: {label} requires backend {backend!r}, which its "
            f"package does not declare in l2_backends"
            for backend in backends
            if isinstance(backend, str) and backend not in declared
        ]
    return problems


def _cell_consistency(label: str, entry: dict[str, Any]) -> list[str]:
    """The cross-field rules a reader relies on when acting on a cell."""
    problems: list[str] = []
    execution, origin, state = entry.get("execution"), entry.get("origin"), entry.get("state")
    if not isinstance(entry.get("reusable"), bool):
        problems.append(f"malformed-receipt: {label} reusable must be a boolean")
    if execution == "reuse" and entry.get("reusable") is False:
        problems.append(
            f"malformed-receipt: {label} is reused although no evidence may satisfy it"
        )
    if execution == "reuse" and origin not in ("local", "prior-local"):
        problems.append(
            f"malformed-receipt: {label} is reused but its origin is {origin!r}; "
            "a reused cell is satisfied by local or prior-local evidence"
        )
    if execution == "reuse" and not entry.get("evidence"):
        problems.append(
            f"missing-receipt: {label} is reused but carries no evidence; a reused "
            "cell must name the receipt that satisfied it"
        )
    if execution == "execute" and origin != "ci":
        problems.append(
            f"malformed-receipt: {label} will execute but its origin is {origin!r}"
        )
    if state == "accepted-gap" and not entry.get("gap"):
        problems.append(
            f"malformed-receipt: {label} is an accepted gap with no governing policy "
            "entry; an ungoverned gap can never excuse missing coverage"
        )
    if state == "prohibited" and not entry.get("prohibition"):
        problems.append(
            f"malformed-receipt: {label} is prohibited but names no constraint"
        )
    if state == "prohibited" and execution == "execute":
        problems.append(
            f"malformed-receipt: {label} is prohibited yet scheduled to execute"
        )
    if state == "reused" and execution != "reuse":
        problems.append(
            f"malformed-receipt: {label} is in state 'reused' but its execution is {execution!r}"
        )
    runs_nextest = execution == "execute" and entry.get("gate") in BUILD_GATES
    if runs_nextest and not entry.get("profile"):
        problems.append(
            f"malformed-receipt: {label} will execute but names no nextest profile; "
            "the consumer would have to guess the selection its expected-test "
            "listing and its gate command must share"
        )
    if not runs_nextest and "profile" in entry:
        problems.append(
            f"malformed-receipt: {label} carries a nextest profile but runs no "
            "nextest selection"
        )
    if "requires_node" in entry and not isinstance(entry["requires_node"], bool):
        problems.append(f"malformed-receipt: {label} requires_node must be a boolean")
    if "companions_only" in entry:
        if entry["companions_only"] is not True:
            problems.append(
                f"malformed-receipt: {label} carries companions_only, which is "
                "present only to assert the cell's work IS its companions"
            )
        if entry.get("gate") != "lint":
            problems.append(
                f"malformed-receipt: {label} is companions-only but only a lint "
                "cell has Clippy to stand down"
            )
        if not entry.get("companions"):
            problems.append(
                f"malformed-receipt: {label} is companions-only and names no "
                "companion, so it would run nothing at all"
            )
    return problems


def validate_receipt(document: Any) -> list[str]:
    """Every reason `document` is not a valid version-2 validation receipt.

    A version-1 note is *not* malformed — it is a legacy document with narrower
    reuse rules — so it is reported with the `v1-*` codes instead.
    """
    if isinstance(document, dict) and document.get("schema_version") == LEGACY_RECEIPT_SCHEMA_VERSION:
        return [
            "v1-requires-exact-tree: a version-1 receipt carries no per-cell outcome "
            "and is reusable only on exact tree identity, pass-only, whole-environment"
        ]

    problems = _keys("receipt", document, RECEIPT_FIELDS)
    if problems:
        return problems

    if document["schema_version"] != RECEIPT_SCHEMA_VERSION:
        return [
            f"unknown-schema-version: receipt is version {document['schema_version']!r}, "
            f"this tool reads {RECEIPT_SCHEMA_VERSION} and "
            f"{LEGACY_RECEIPT_SCHEMA_VERSION}"
        ]

    problems += _member(
        "receipt environment", document["environment"], ENVIRONMENTS, "unknown-environment"
    )
    for field in ("base", "head", "tree"):
        problems += _sha(f"receipt {field}", document[field])
    if not _IDENTITY.fullmatch(str(document["scope_identity"])):
        problems.append("malformed-receipt: receipt scope_identity is not a hex identity")
    problems += _member(
        "receipt completion", document["completion"], COMPLETIONS, "malformed-receipt"
    )
    problems += _keys("receipt host", document["host"], HOST_FIELDS)

    if not isinstance(document["cells"], list) or not document["cells"]:
        problems.append("malformed-receipt: receipt cells must be a non-empty list")
        return problems

    seen: dict[tuple[str, str], dict[str, Any]] = {}
    for entry in document["cells"]:
        problems += _keys("receipt cell", entry, RECEIPT_CELL_FIELDS)
        if not isinstance(entry, dict) or "package" not in entry or "gate" not in entry:
            continue
        label = f"receipt cell {entry['package']}/{document['environment']}/{entry['gate']}"
        problems += _member(f"{label} gate", entry["gate"], GATES, "unknown-gate")
        problems += _member(f"{label} outcome", entry.get("outcome"), OUTCOMES, "malformed-receipt")
        problems += _member(f"{label} completion", entry.get("completion"), COMPLETIONS, "malformed-receipt")
        problems += _counts(label, entry.get("counts"))
        if not _IDENTITY.fullmatch(str(entry.get("gate_input_identity"))):
            problems.append(f"malformed-receipt: {label} has no gate-input identity")
        if not isinstance(entry.get("duration_s"), (int, float)) or entry["duration_s"] < 0:
            problems.append(f"malformed-receipt: {label} duration_s must be a non-negative number")
        problems += _str_list(f"{label} backends", entry.get("backends"))
        build = entry.get("build")
        if build is not None:
            problems += _keys(f"{label} build", build, RECEIPT_BUILD_FIELDS)
            if isinstance(build, dict):
                for field in ("key", "digest"):
                    if not _IDENTITY.fullmatch(str(build.get(field))):
                        problems.append(
                            f"malformed-receipt: {label} build {field} is not a digest"
                        )
        failed = entry.get("failed_tests", [])
        problems += _str_list(f"{label} failed_tests", failed)
        if isinstance(failed, list) and len(failed) > FAILURE_DETAIL_LIMIT:
            problems.append(
                f"malformed-receipt: {label} names {len(failed)} failed tests, over the "
                f"{FAILURE_DETAIL_LIMIT} the receipt carries; truncate and set "
                "failure_detail_truncated"
            )
        if (
            entry.get("outcome") == "fail"
            and entry.get("completion") == "complete"
            and not failed
            and not entry.get("failure_detail_truncated")
        ):
            # Only a COMPLETE failure owes detail. An interrupted or partial
            # gate — a compile failure staging no report, say — has an exit
            # code and nothing to attribute it to, and is rejected for reuse on
            # its completion rather than made unpublishable.
            problems.append(
                f"malformed-receipt: {label} failed but names no failing test; a failure "
                "with no detail cannot be acted on"
            )
        key = (entry["package"], entry["gate"])
        if key in seen and canonical(seen[key]) != canonical(entry):
            problems.append(
                f"conflicting-evidence: {label} appears twice with different results"
            )
        seen.setdefault(key, entry)
    return problems


def validate_scope_receipt(document: Any) -> list[str]:
    """Every reason `document` is not a valid scope receipt, or an empty list.

    Structural validity only: the identity comparison against an event's
    `{base, head, tree}` is the verifier's, because only it knows the event.
    Each problem begins with a code from [`SCOPE_REJECTIONS`].
    """
    problems = _keys("scope receipt", document, SCOPE_RECEIPT_FIELDS, "scope-malformed")
    if problems:
        return problems
    if document["schema_version"] != SCOPE_RECEIPT_SCHEMA_VERSION:
        return [
            f"scope-schema: scope receipt is version {document['schema_version']!r}, "
            f"this tool reads {SCOPE_RECEIPT_SCHEMA_VERSION}"
        ]
    if document["plan_schema_version"] != RESOLVED_PLAN_SCHEMA_VERSION:
        return [
            f"scope-schema: scope receipt carries a version "
            f"{document['plan_schema_version']!r} plan, this tool reads "
            f"{RESOLVED_PLAN_SCHEMA_VERSION}"
        ]
    for field in ("base", "head", "tree"):
        problems += [
            problem.replace("malformed-receipt", "scope-malformed", 1)
            for problem in _sha(f"scope receipt {field}", document[field])
        ]
    plan = document["plan"]
    plan_problems = validate_resolved_plan(plan)
    if plan_problems:
        return problems + [f"scope-malformed: the carried plan is invalid: {plan_problems[0]}"]
    for field in ("base", "head"):
        if plan[field] != document[field]:
            problems.append(
                f"scope-malformed: the carried plan names {field} "
                f"{str(plan[field])[:9]}, the receipt {str(document[field])[:9]}"
            )
    scope = document["scope"]
    if not isinstance(scope, dict):
        return problems + ["scope-malformed: scope receipt scope must be an object"]
    problems += [
        f"scope-malformed: the carried scope projection lacks '{name}'"
        for name in SCOPE_PROJECTION_FIELDS
        if name not in scope
    ]
    if not problems and scope["packages"] != [entry["package"] for entry in plan["packages"]]:
        # One projection of one plan: a scope naming other packages than the
        # plan it travels with would fan out work the plan never resolved.
        problems.append(
            "scope-malformed: the carried scope projection names other packages "
            "than the carried plan"
        )
    return problems


def validate_completion_record(document: Any) -> list[str]:
    """Every reason `document` is not a valid completion record, or an empty list.

    Structural only, and deliberately so: whether the record is *true* is the
    producer's question (it wrote the record from the comparison it just ran)
    and whether it is *bound to this run* is the audit's. Each problem begins
    with a code from [`COMPLETION_REJECTIONS`].
    """
    problems = _keys(
        "completion record", document, COMPLETION_RECORD_FIELDS, "completion-plan-unreadable"
    )
    if problems:
        return problems
    if document["schema_version"] != COMPLETION_RECORD_SCHEMA_VERSION:
        return [
            f"completion-manifest-schema: completion record is version "
            f"{document['schema_version']!r}, this tool reads "
            f"{COMPLETION_RECORD_SCHEMA_VERSION}"
        ]
    problems += _member(
        "completion record environment",
        document["environment"],
        ENVIRONMENTS,
        "completion-cell-unknown",
    )
    problems += _member(
        "completion record gate", document["gate"], GATES, "completion-cell-unknown"
    )
    problems += [
        problem.replace("malformed-receipt", "completion-plan-unreadable", 1)
        for problem in _sha("completion record head", document["head"])
    ]
    if document["complete"] is not True:
        problems.append(
            "completion-plan-unreadable: a completion record exists only when "
            "validation succeeded, so `complete` must be true"
        )
    problems += [
        problem.replace("malformed-receipt", "completion-report-missing", 1)
        for problem in _str_list("completion record reports", document["reports"])
    ]
    return problems


def _counts(label: str, counts: Any) -> list[str]:
    if not isinstance(counts, dict):
        return [f"malformed-receipt: {label} counts must be an object"]
    problems = [
        f"malformed-receipt: {label} counts is missing '{field}'"
        for field in COUNT_FIELDS
        if field not in counts
    ]
    problems += [
        f"malformed-receipt: {label} counts.{field} must be a non-negative integer"
        for field in COUNT_FIELDS
        if isinstance(counts.get(field), int) and counts[field] < 0
    ]
    if not problems:
        parts = sum(counts[field] for field in ("passed", "failed", "skipped", "errored"))
        if parts != counts["total"]:
            problems.append(
                f"malformed-receipt: {label} counts.total is {counts['total']} but its "
                f"parts sum to {parts}"
            )
    return problems


def reusable_cells(receipt: dict[str, Any]) -> tuple[list[dict[str, Any]], list[str]]:
    """Split a valid receipt into the cells that may satisfy plan cells and the
    reasons the rest may not.

    An interrupted whole receipt contributes nothing: the run did not execute
    the tree it claims. An interrupted or failing *cell* inside a complete run
    is rejected alone, leaving its completed passing siblings reusable (spec
    section 3.4).
    """
    problems = validate_receipt(receipt)
    if problems:
        return [], problems
    if receipt["completion"] != "complete":
        return [], [
            f"incomplete-run: the {receipt['environment']} run is "
            f"{receipt['completion']}; no cell of it is reusable"
        ]
    accepted, rejected = [], []
    for cell in receipt["cells"]:
        if cell["completion"] == "complete" and cell["outcome"] == "pass":
            accepted.append(cell)
        elif cell["completion"] == "complete":
            rejected.append(
                f"failed-cell: {cell['package']}/{receipt['environment']}/"
                f"{cell['gate']} failed; failing evidence is diagnostic only"
            )
        else:
            rejected.append(
                f"incomplete-run: {cell['package']}/{receipt['environment']}/"
                f"{cell['gate']} is {cell['completion']}"
            )
    return accepted, rejected


def main() -> None:
    CONTRACT_PATH.parent.mkdir(parents=True, exist_ok=True)
    CONTRACT_PATH.write_text(json.dumps(contract(), indent=2) + "\n", encoding="utf-8")
    print(f"wrote {CONTRACT_PATH.relative_to(ROOT)}")


if __name__ == "__main__":
    main()

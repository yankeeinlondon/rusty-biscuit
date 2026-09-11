#!/usr/bin/env python3
"""Canonical CI contracts: the resolved plan and the validation receipt.

One module owns both documents so the planner, the evidence verifier, the hook,
and the Rust rollup cannot drift into private copies of the same vocabulary.
`fixes/2026-09-11-cicd-cleanup/spec.md` requires exactly this: "Determine the
smallest coherent schema/workflow changes... Avoid parallel planners, duplicate
policy stores".

The field contract is also dumped to `.github/ci/schemas/contract.json` so the
Rust side can assert its structs against the same source without spawning a
Python process. `test_schema.py` fails when that file drifts from this module.

## Notes

`affected_scope.py` emits the resolved plan as of Phase 3. `local_evidence.py`
and the Rust rollup still predate it; Phases 4 and 5 move them over.

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
"""

from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = ROOT / ".github" / "ci" / "schemas" / "contract.json"

RESOLVED_PLAN_SCHEMA_VERSION = 1
RECEIPT_SCHEMA_VERSION = 2

#: The last receipt version that predates per-cell outcomes. Accepted only on
#: exact tree identity, pass-only, whole-environment, and never upgraded in
#: place (spec section 3.6).
LEGACY_RECEIPT_SCHEMA_VERSION = 1

#: Rendered in place of counts and duration for a version-1 receipt. Verbatim
#: from spec section 3.6 — the text is part of the contract, not decoration.
UNRECORDED_MEASUREMENT = "not recorded (v1 receipt)"

ENVIRONMENTS = ("ubuntu-latest", "windows-latest", "macos-latest", "wsl2-ubuntu")

#: A cell's gate. `check` and `lint` are compile-time gates; the rest are test
#: tiers. One flat vocabulary because a cell key is `{package, environment,
#: gate/tier}` and the two kinds are never both present for one key.
GATES = ("lint", "check", "L1", "L2", "browser")

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
#: everywhere. Phase 4 consumes this; adding a code is a contract change.
REJECTIONS = (
    "missing-receipt",
    "malformed-receipt",
    "unknown-schema-version",
    "unknown-environment",
    "unknown-gate",
    "unknown-package",
    "tree-mismatch",
    "gate-inputs-changed",
    "incomplete-run",
    "conflicting-evidence",
    "dirty-tree",
    "v1-requires-exact-tree",
    "v1-not-equivalence-eligible",
    "v1-is-pass-only",
)

#: Cap on `failed_tests` carried in a receipt cell. A failing L1 suite can name
#: thousands of tests; a Git note is not a report store. Past the cap the cell
#: sets `failure_detail_truncated` and the full list stays in the host report.
FAILURE_DETAIL_LIMIT = 20

_SHA = re.compile(r"[0-9a-f]{40}")
_IDENTITY = re.compile(r"[0-9a-f]{8,64}")


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
    "cells": True,
    "accepted_evidence": True,
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
}

AREA_FIELDS: dict[str, bool] = {
    "area": True,
    "selection_reason": True,
    "packages": True,
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
    "native": True,
    #: Repository-relative manifest directories of the package's build closure,
    #: dev-dependencies included. The gate-input identity of spec section 3.3
    #: is computed over these plus the gate's global inputs. Optional: a plan
    #: that omits it simply makes its cells exact-tree-only for reuse.
    "input_paths": False,
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
    "target_kinds": True,
    "compile_coverage_from": True,
    "selection_reason": True,
    "evidence": False,
    "gap": False,
    "prohibition": False,
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
}

HOST_FIELDS: dict[str, bool] = {"os": True, "kernel": True, "report_dir": True}

COUNT_FIELDS = ("total", "passed", "failed", "skipped", "errored")


def contract() -> dict[str, Any]:
    """The whole field contract, for cross-language assertion."""
    return {
        "resolved_plan": {
            "schema_version": RESOLVED_PLAN_SCHEMA_VERSION,
            "document": RESOLVED_PLAN_FIELDS,
            "area": AREA_FIELDS,
            "package": PACKAGE_FIELDS,
            "cell": CELL_FIELDS,
        },
        "receipt": {
            "schema_version": RECEIPT_SCHEMA_VERSION,
            "legacy_schema_version": LEGACY_RECEIPT_SCHEMA_VERSION,
            "document": RECEIPT_FIELDS,
            "cell": RECEIPT_CELL_FIELDS,
            "host": HOST_FIELDS,
            "counts": list(COUNT_FIELDS),
            "failure_detail_limit": FAILURE_DETAIL_LIMIT,
            "unrecorded_measurement": UNRECORDED_MEASUREMENT,
        },
        "vocabulary": {
            "environments": list(ENVIRONMENTS),
            "gates": list(GATES),
            "target_kinds": list(TARGET_KINDS),
            "executions": list(EXECUTIONS),
            "origins": list(ORIGINS),
            "cell_states": list(CELL_STATES),
            "accepted_gap_state": ACCEPTED_GAP_STATE,
            "completions": list(COMPLETIONS),
            "outcomes": list(OUTCOMES),
            "rejections": list(REJECTIONS),
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


def _keys(where: str, value: Any, fields: dict[str, bool]) -> list[str]:
    if not isinstance(value, dict):
        return [f"malformed-receipt: {where} must be an object"]
    problems = [
        f"malformed-receipt: {where} is missing required field '{name}'"
        for name, required in fields.items()
        if required and name not in value
    ]
    problems += [
        f"malformed-receipt: {where} has unknown field '{name}'"
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


def validate_resolved_plan(document: Any) -> list[str]:
    """Every reason `document` is not a valid resolved plan, or an empty list.

    ## Returns

    Problems in document order. Each begins with a code from [`REJECTIONS`] so
    a caller can classify without parsing prose.
    """
    problems = _keys("resolved plan", document, RESOLVED_PLAN_FIELDS)
    if problems:
        return problems

    if document["schema_version"] != RESOLVED_PLAN_SCHEMA_VERSION:
        return [
            f"unknown-schema-version: resolved plan is version "
            f"{document['schema_version']!r}, this tool writes "
            f"{RESOLVED_PLAN_SCHEMA_VERSION}"
        ]

    for field in ("base", "head"):
        problems += _sha(f"resolved plan {field}", document[field])
    problems += _member(
        "resolved plan change_class",
        document["change_class"],
        ("full", "package", "documentation"),
        "malformed-receipt",
    )

    areas = {}
    for entry in document["areas"]:
        problems += _keys("area record", entry, AREA_FIELDS)
        if isinstance(entry, dict) and "area" in entry:
            areas[entry["area"]] = entry
            if not entry.get("selection_reason"):
                problems.append(
                    f"malformed-receipt: area {entry['area']!r} has no selection reason"
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
        problems += _cell_consistency(label, entry)

    if "evidence_rejections" in document:
        problems += _str_list(
            "resolved plan evidence_rejections", document["evidence_rejections"]
        )
    if not isinstance(document["job_estimate"], int) or document["job_estimate"] < 0:
        problems.append("malformed-receipt: resolved plan job_estimate must be a non-negative integer")
    return problems


def _cell_consistency(label: str, entry: dict[str, Any]) -> list[str]:
    """The cross-field rules a reader relies on when acting on a cell."""
    problems: list[str] = []
    execution, origin, state = entry.get("execution"), entry.get("origin"), entry.get("state")
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
    the tree it claims. An interrupted *cell* inside a complete run is rejected
    alone, leaving its completed siblings reusable (spec section 3.4).
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
        if cell["completion"] == "complete":
            accepted.append(cell)
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

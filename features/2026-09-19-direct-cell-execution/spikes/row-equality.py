#!/usr/bin/env python3
"""Migration step 1's evidence: the rows and the lists describe one plan.

The specification's first migration step is "add the deterministic row adapter
BESIDE the current projection. Prove identity equality **and uniqueness**,
dispatch fields, area membership, and complete joins back to the plan."

This is that proof, run over the Phase 1 corpus shapes
(`spikes/plans/README.md`), planned live by the real planner on the real
workspace rather than read from the frozen version-4 files. For each shape it
checks four things and prints one line per shape:

1. **Equality** — the executing-cell set the four row sets dispatch is exactly
   the executing-cell set the environment-list projection describes, which is
   exactly the plan's `execution: execute` cells.
2. **Uniqueness** — no `{package, environment, gate}` key appears in two rows.
   The list side cannot state this at all: an environment list has no key.
3. **Dispatch fields** — every row's runner is the one the plan's environment
   table gives that environment.
4. **Area membership** — every row is dispatched inside its package's area, and
   every selected area has a row document whether or not it executes anything.

Run from the repository root:

    python3 features/2026-09-19-direct-cell-execution/spikes/row-equality.py
"""

from __future__ import annotations

import sys
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "scripts" / "ci"))

import affected_scope  # noqa: E402
from affected_scope import (  # noqa: E402
    ENVIRONMENTS_CONFIG,
    LINT_ENVIRONMENT,
    calculate_scope,
    legacy_scope_document,
    load_environments,
    load_metadata,
    package_ci_policy,
    row_sets,
    workspace_packages,
)

TODAY = date(2026, 9, 20)


def plan(files: list[str], **kwargs) -> dict:
    metadata = load_metadata(ROOT)
    packages = workspace_packages(metadata)
    environments = load_environments(ENVIRONMENTS_CONFIG, today=TODAY)
    policy = package_ci_policy(
        packages,
        runner_labels={environment["runner"] for environment in environments},
        root=ROOT,
        today=TODAY,
    )
    return calculate_scope(files, ROOT, metadata, environments, policy, **kwargs)


def executing(document: dict) -> set[tuple[str, str, str]]:
    return {
        (cell["package"], cell["environment"], cell["gate"])
        for cell in document["cells"]
        if cell["execution"] == "execute"
    }


def from_lists(document: dict) -> set[tuple[str, str, str]]:
    """The executing cells the environment-list projection describes.

    The inverse of `matrix_record`: every list a workflow input carries today,
    read back as the cells it would schedule.
    """
    cells: set[tuple[str, str, str]] = set()
    for record in legacy_scope_document(document)["matrix"]:
        package = record["package"]
        for environment in record["native_environments"]:
            cells.add((package, environment, "L1"))
        for environment in record["check_os"]:
            cells.add((package, environment, "check"))
        for environment in record["l2_environments"]:
            cells.add((package, environment, "L2"))
        for environment in record["browser_environments"]:
            cells.add((package, environment, "browser"))
        if record["wsl"]:
            cells.add((package, "wsl2-ubuntu", "L1"))
        if "lint" in record["gates"]:
            cells.add((package, LINT_ENVIRONMENT, "lint"))
    return cells


def from_rows(document: dict) -> list[tuple[str, str, str]]:
    """Every row's cell key, as a list so duplicates survive to be counted."""
    return [
        (row["package"], row["environment"], row["gate"])
        for area in row_sets(document).values()
        for name in affected_scope.schema.ROW_SET_NAMES
        for row in area[name]
    ]


def check(name: str, document: dict, lists_track_cells: bool = True) -> list[str]:
    """Check one shape; print its line and return its problems.

    `lists_track_cells` is false for the synthesized gap-only shape alone. The
    list projection reads a package record's `gates`, not its cells, so a
    document whose cells were trimmed without its package records is one the
    lists describe wrongly and the rows describe correctly. The divergence is
    reported rather than asserted — it is the defect class this feature
    removes, demonstrated.
    """
    problems: list[str] = []
    notes: list[str] = []
    cells = executing(document)
    rows = from_rows(document)
    lists = from_lists(document)

    if len(rows) != len(set(rows)):
        duplicated = sorted({key for key in rows if rows.count(key) > 1})
        problems.append(f"duplicate row key(s): {duplicated}")
    if set(rows) != cells:
        problems.append(
            f"rows-vs-cells: +{sorted(set(rows) - cells)} -{sorted(cells - set(rows))}"
        )
    if lists != cells:
        message = (
            f"lists-vs-cells: the lists schedule {len(lists - cells)} cell(s) the "
            f"plan does not carry and omit {len(cells - lists)}"
        )
        (problems if lists_track_cells else notes).append(message)

    runners = {entry["name"]: entry["runner"] for entry in document["environments"]}
    areas = {entry["package"]: entry["area"] for entry in document["packages"]}
    documents = row_sets(document)
    for area, sets in documents.items():
        for set_name in affected_scope.schema.ROW_SET_NAMES:
            for row in sets[set_name]:
                if row["runner"] != runners[row["environment"]]:
                    problems.append(f"{row} carries a runner the table does not give")
                if areas[row["package"]] != area:
                    problems.append(f"{row} is dispatched outside its package's area")
    missing = {entry["area"] for entry in document["areas"]} - set(documents)
    if missing:
        problems.append(f"selected area(s) with no row document: {sorted(missing)}")

    verdict = "OK" if not problems else "FAILED"
    print(
        f"{name:<12} cells={len(cells):>4}  rows={len(rows):>4}  "
        f"lists={len(lists):>4}  areas={len(documents):>3}  {verdict}"
    )
    for problem in problems:
        print(f"             {problem}")
    for note in notes:
        print(f"             NOTE {note}")
    return problems


def main() -> int:
    everything = plan([], force_all=True)
    executing_cells = [
        cell for cell in everything["cells"] if cell["execution"] == "execute"
    ]

    def accepted(cells) -> list[dict]:
        return [
            {
                "package": cell["package"],
                "environment": cell["environment"],
                "gate": cell["gate"],
                "origin": "local",
                "outcome": "pass",
                "evidence": {"ref": f"refs/notes/ci-local/{cell['environment']}"},
            }
            for cell in cells
        ]

    corpus = {
        "pr": plan(["biscuit-hash/lib/src/lib.rs"], event="pull_request"),
        "all": everything,
        "all-reused": plan([], force_all=True, accepted_cells=accepted(executing_cells)),
        "mixed": plan(
            [],
            force_all=True,
            accepted_cells=accepted(
                cell
                for cell in executing_cells
                if cell["environment"] == "wsl2-ubuntu" and cell["gate"] == "L1"
            ),
        ),
        "nightly": plan([], force_all=True, event="schedule"),
        "prohibited": plan(
            [],
            force_all=True,
            prohibitions={
                "wsl2-ubuntu": {
                    "owner": "ken",
                    "reason": "spike: prohibit the WSL2 leg to shape the corpus",
                    "expiry": "2027-12-31",
                    "source": "current.json",
                }
            },
        ),
    }

    failed = False
    for name, document in corpus.items():
        failed = bool(check(name, document)) or failed

    # The gap-only shape, derived as `spikes/plans/README.md` records: real gap
    # cells, the areas and packages that own them, no builds. No real plan is
    # gap-only today, because lint never reuses.
    gaps = [cell for cell in everything["cells"] if cell["state"] == "accepted-gap"]
    owners = sorted({cell["package"] for cell in gaps})
    gap_only = {
        **everything,
        "cells": gaps,
        "packages": [
            entry for entry in everything["packages"] if entry["package"] in owners
        ],
        "builds": [],
        "job_estimate": 0,
    }
    gap_only["rows"] = affected_scope.attach_rows(gap_only)
    failed = bool(check("gap-only", gap_only, lists_track_cells=False)) or failed

    print()
    print("Six real shapes: the rows and the lists describe the same executing cells.")
    print("Only the row side can state uniqueness; it holds for all seven shapes.")
    print(
        "gap-only is synthesized, and the one shape where the two DISAGREE: the "
        "lists\nread a package record's declared gates, the rows read the cells."
    )
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())

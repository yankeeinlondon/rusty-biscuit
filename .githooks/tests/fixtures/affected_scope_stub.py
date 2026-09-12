#!/usr/bin/env python3
"""Stand-in for `scripts/ci/affected_scope.py` inside the hook test repositories.

The real planner reads `cargo metadata` from the repository it lives in, and
the fixture repositories hold no Cargo workspace, so the hook's scope
calculation is answered here instead: the same command line (options, then
`--`, then the committed path set) is accepted and logged to
`$TEST_PLANNER_LOG` — one line per call, followed by a `root <dir> cwd <dir>`
line naming the tree this copy of the planner lives in and ran from — and
`--plan-out` receives a schema-valid plan for the base and head it was handed,
with stdout carrying the legacy projection of that plan. The plan is the fixed
one-cell `alpha` plan below unless `TEST_PLANNER_PLAN` names a fixture, in
which case that fixture's plan is emitted with its `base` and `head` rewritten.
`TEST_PLANNER_FAIL=1` makes the calculation fail, as a real planner error would.

Like the real planner, this one reads policy from its OWN tree: the
`preflight_reason` of every plan comes from `.github/ci/policy.json` next to
this file's repository root when that file exists. A test that commits one
value and leaves another unstaged can therefore tell which tree planned.

`--apply-to PLAN` mirrors the real overlay's shape without its validation: the
carried plan is copied, every accepted cell is marked reused, and the evidence
rejections ride along.

`local_evidence.py` imports this module for the gate-input classification, so
the names it reads are present (and empty).
"""

import json
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
POLICY = ROOT / ".github" / "ci" / "policy.json"

GLOBAL_PATHS_ALL_GATES = ()
GLOBAL_PATHS_BY_GATE = {"lint": (), "check": (), "test": ()}
JUST_PATHS = ()
GLOBAL_PREFIXES_ALL_GATES = ()
JUST_PREFIXES = ()
LOCKFILE_PATH = "Cargo.lock"


def preflight_reason() -> str:
    if POLICY.is_file():
        return json.loads(POLICY.read_text(encoding="utf-8"))["preflight_reason"]
    return "package-local change"


def fixed_plan(base: str, head: str) -> dict:
    package = {
        "package": "alpha",
        "area": "pkg",
        "selection_reason": "source change",
        "gates": ["L1"],
        "targets": ["lib", "test"],
        "tiers": ["L1"],
        "test_args": "",
        "check_args": "-p alpha",
        "l2_backends": [],
        "runner_tools": [],
        "companion_suites": [],
        "l1_include_slow": False,
        "native": {},
    }
    return {
        "schema_version": 2,
        "base": base,
        "head": head,
        "change_class": "package",
        "full_scope": False,
        "full_scope_gates": [],
        "areas": [{"area": "pkg", "selection_reason": "source change", "packages": ["alpha"]}],
        "packages": [package],
        "source_packages": ["alpha"],
        "reverse_dependencies": [],
        "environments": [
            {
                "name": "macos-latest",
                "runner": "macos-latest",
                "native_key": "macos-latest",
                "capabilities": {
                    "tmux": True,
                    "headless_browser": False,
                    "node_pnpm": False,
                    "archive_only": False,
                },
            }
        ],
        "cells": [
            {
                "package": "alpha",
                "area": "pkg",
                "environment": "macos-latest",
                "gate": "L1",
                "execution": "execute",
                "origin": "ci",
                "state": "pending",
                "reusable": True,
                "target_kinds": ["lib", "test"],
                "compile_coverage_from": "L1",
                "selection_reason": "no evidence for this environment",
            }
        ],
        "accepted_evidence": [],
        "policy_gaps": [],
        "prohibited_cells": [],
        "job_estimate": 1,
        "preflight_os": ["macos-latest"],
        "preflight_reason": preflight_reason(),
        "flags": {"ci_tooling": False},
    }


def projection(plan: dict) -> dict:
    packages = [entry["package"] for entry in plan["packages"]]
    return {
        "packages": packages,
        "areas": plan["areas"],
        "scheduled_areas": [area["area"] for area in plan["areas"]],
        "area_matrix": {
            area["area"]: {
                "include": [p for p in plan["packages"] if p["area"] == area["area"]]
            }
            for area in plan["areas"]
        },
        "area_slugs": {area["area"]: area["area"] for area in plan["areas"]},
        "source_packages": plan["source_packages"],
        "reverse_dependencies": plan["reverse_dependencies"],
        "full_scope": plan["full_scope"],
        "full_scope_gates": plan["full_scope_gates"],
        "change_class": plan["change_class"],
        "preflight_os": plan["preflight_os"],
        "preflight_reason": plan["preflight_reason"],
        "matrix": plan["packages"],
        "policy": [],
        "job_estimate": plan["job_estimate"],
        "flags": plan["flags"],
    }


def apply(plan: dict, accepted: list, rejections: list) -> dict:
    applied = json.loads(json.dumps(plan))
    for cell in applied["cells"]:
        for entry in accepted:
            if (entry["package"], entry["environment"], entry["gate"]) == (
                cell["package"], cell["environment"], cell["gate"]
            ):
                cell.update(
                    execution="reuse",
                    origin=entry.get("origin", "local"),
                    state="reused",
                    evidence=entry,
                )
    applied["accepted_evidence"] = accepted
    applied["evidence_rejections"] = rejections
    return applied


def main() -> None:
    log = os.environ.get("TEST_PLANNER_LOG")
    if log:
        with open(log, "a", encoding="utf-8") as handle:
            handle.write(" ".join(sys.argv[1:]) + "\n")
            handle.write(f"root {ROOT} cwd {Path.cwd()}\n")
    if os.environ.get("TEST_PLANNER_FAIL") == "1":
        raise SystemExit("stub planner: calculation failed on request")
    options = sys.argv[1:]
    if "--" in options:
        options = options[: options.index("--")]
    values = {}
    flags = set()
    while options:
        option = options.pop(0)
        if option in ("--resolved-plan", "--all"):
            flags.add(option)
        else:
            values[option] = options.pop(0)
    if "--apply-to" in values:
        plan = json.loads(Path(values["--apply-to"]).read_text(encoding="utf-8"))
        accepted = []
        if "--accepted-cells" in values:
            accepted = json.loads(Path(values["--accepted-cells"]).read_text(encoding="utf-8"))
        rejections = []
        if "--evidence-rejections" in values:
            rejections = json.loads(
                Path(values["--evidence-rejections"]).read_text(encoding="utf-8")
            )
        plan = apply(plan, accepted, rejections)
    else:
        base = values.get("--base", "0" * 40)
        head = values.get("--head", "0" * 40)
        fixture = os.environ.get("TEST_PLANNER_PLAN")
        if fixture:
            text = Path(fixture).read_text(encoding="utf-8")
            try:
                plan = json.loads(text)
            except json.JSONDecodeError:
                # A planner that wrote something unreadable: hand it on verbatim.
                if "--plan-out" in values:
                    Path(values["--plan-out"]).write_text(text, encoding="utf-8")
                print(text)
                return
            plan.update(base=base, head=head, preflight_reason=preflight_reason())
        else:
            plan = fixed_plan(base, head)
    if "--plan-out" in values:
        with open(values["--plan-out"], "w", encoding="utf-8") as handle:
            handle.write(json.dumps(plan, sort_keys=True, separators=(",", ":")))
    document = plan if "--resolved-plan" in flags else projection(plan)
    print(json.dumps(document, separators=(",", ":")))


if __name__ == "__main__":
    main()

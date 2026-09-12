#!/usr/bin/env python3
"""Stand-in for `scripts/ci/affected_scope.py` inside the hook test repositories.

The real planner reads `cargo metadata` from the repository it lives in, and
the fixture repositories hold no Cargo workspace, so the hook's scope
calculation is answered here instead: the same command line (options, then
`--`, then the committed path set) is accepted and logged to
`$TEST_PLANNER_LOG`, `--plan-out` receives a schema-valid plan for the base and
head it was handed, and stdout carries the legacy projection of that plan.
`TEST_PLANNER_FAIL=1` makes the calculation fail, as a real planner error would.

`local_evidence.py` imports this module for the gate-input classification, so
the names it reads are present (and empty).
"""

import json
import os
import sys

GLOBAL_PATHS_ALL_GATES = ()
GLOBAL_PATHS_BY_GATE = {"lint": (), "check": (), "test": ()}
JUST_PATHS = ()
GLOBAL_PREFIXES_ALL_GATES = ()
JUST_PREFIXES = ()
LOCKFILE_PATH = "Cargo.lock"


def main() -> None:
    log = os.environ.get("TEST_PLANNER_LOG")
    if log:
        with open(log, "a", encoding="utf-8") as handle:
            handle.write(" ".join(sys.argv[1:]) + "\n")
    if os.environ.get("TEST_PLANNER_FAIL") == "1":
        raise SystemExit("stub planner: calculation failed on request")
    options = sys.argv[1:]
    if "--" in options:
        options = options[: options.index("--")]
    values = dict(zip(options[::2], options[1::2]))
    base = values.get("--base", "0" * 40)
    head = values.get("--head", "0" * 40)
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
        "native": {},
    }
    plan = {
        "schema_version": 1,
        "base": base,
        "head": head,
        "change_class": "package",
        "full_scope": False,
        "full_scope_gates": [],
        "areas": [{"area": "pkg", "selection_reason": "source change", "packages": ["alpha"]}],
        "packages": [package],
        "source_packages": ["alpha"],
        "reverse_dependencies": [],
        "cells": [
            {
                "package": "alpha",
                "area": "pkg",
                "environment": "macos-latest",
                "gate": "L1",
                "execution": "execute",
                "origin": "ci",
                "state": "pending",
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
        "preflight_reason": "package-local change",
        "flags": {"ci_tooling": False},
    }
    if "--plan-out" in values:
        with open(values["--plan-out"], "w", encoding="utf-8") as handle:
            handle.write(json.dumps(plan, sort_keys=True, separators=(",", ":")))
    projection = {
        "packages": ["alpha"],
        "areas": plan["areas"],
        "scheduled_areas": ["pkg"],
        "area_matrix": {"pkg": {"include": [package]}},
        "area_slugs": {"pkg": "pkg"},
        "source_packages": ["alpha"],
        "reverse_dependencies": [],
        "full_scope": False,
        "full_scope_gates": [],
        "change_class": "package",
        "preflight_os": ["macos-latest"],
        "preflight_reason": "package-local change",
        "matrix": [package],
        "policy": [],
        "job_estimate": 1,
        "flags": {"ci_tooling": False},
    }
    print(json.dumps(projection, separators=(",", ":")))


if __name__ == "__main__":
    main()

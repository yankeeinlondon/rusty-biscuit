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
An empty path set (a `--` with nothing after it and no `--all`) yields the
empty plan the real planner yields for it — no package, no cell — whatever
`TEST_PLANNER_PLAN` names, so a base that equals the head selects nothing.

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

#: The one build the fixed plan's single executing cell consumes. A literal,
#: not a digest: this stub computes no key, it stands in for one.
FIXED_BUILD_KEY = "a1b2c3d4e5f60718"

GLOBAL_PATHS_ALL_GATES = ()
GLOBAL_PATHS_BY_GATE = {"lint": (), "check": (), "test": ()}
GLOBAL_PREFIXES_ALL_GATES = ()
ORCHESTRATION_PATHS = ()
ORCHESTRATION_PREFIXES = ()
LOCKFILE_PATH = "Cargo.lock"
EVENT_NAMES = ("pull_request", "push", "schedule", "workflow_dispatch")

# The Just recipes a gate's identity covers (`local_evidence.just_gate_inputs`):
# no fixture repository holds a justfile, so no recipe is reachable and the
# parser sees nothing. Present so the identity can be computed at all.
CI_RECIPES_BY_GATE = {"lint": (), "check": (), "test": ()}
CI_RECIPES_ALL_GATES = ()


def parse_just_recipes(text: str) -> tuple[dict, list]:
    return {}, []


def just_recipe_closure(recipes: dict, entries: tuple) -> set:
    return set()


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
        "archive_includes": [],
        "sidecars": [],
        "companion_suites": [],
        "l1_include_slow": False,
        "native": {},
    }
    return {
        "schema_version": 5,
        "base": base,
        "head": head,
        "change_class": "package",
        # Fixed, like every other field of this plan. The real bucketing rule
        # is `affected_scope.change_bucket`; no hook fixture reads a bucket, so
        # the stub only has to emit something the schema accepts.
        "change_inventory": {
            "diff_available": True,
            "paths": {
                "configuration": [],
                "documentation": [],
                "source": ["alpha/src/lib.rs"],
                "other": [],
            },
            "counts": {
                "configuration": 0,
                "documentation": 0,
                "source": 1,
                "other": 0,
                "total": 1,
            },
        },
        "full_scope": False,
        "full_scope_gates": [],
        "areas": [
            {
                "area": "pkg",
                "selection_reason": "source change",
                "packages": ["alpha"],
                "execution_path": "rows",
            }
        ],
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
                "build": FIXED_BUILD_KEY,
                "profile": "ci",
            }
        ],
        "builds": [
            {
                "key": FIXED_BUILD_KEY,
                "package": "alpha",
                "producer": "macos-latest",
                "artifact": f"build-alpha-macos-latest-{FIXED_BUILD_KEY}",
                "compatible_environments": ["macos-latest"],
                "compatibility_reason": (
                    "aarch64-apple-darwin archive produced on macos-latest"
                ),
                "consumers": [{"environment": "macos-latest", "gate": "L1"}],
                "identity": {
                    "source_commit": head,
                    "lockfile": "aaaabbbbccccdddd",
                    "rust": "1.97.1",
                    "nextest": "latest",
                    "host": "aarch64-apple-darwin",
                    "target": "aarch64-apple-darwin",
                    "profile": "test",
                    "rustflags": "",
                    "cargo_config": [],
                    "linker": "cc",
                    "archive_format": "tar.zst",
                    "package": "alpha",
                    "target_kinds": ["lib", "test"],
                    "features": "",
                    "native": [],
                    "archive_includes": [],
                    "sidecars": [],
                },
            }
        ],
        "accepted_evidence": [],
        # The real planner snapshots `.github/ci/ci-baseline.toml`; no fixture
        # repository holds one, so the honest snapshot here is the empty budget
        # the shipped file carries.
        "skip_policy": {
            "source": ".github/ci/ci-baseline.toml",
            "content_hash": "aaaabbbbccccdddd",
            "entries": [],
        },
        "policy_gaps": [],
        "prohibited_cells": [],
        "job_estimate": 1,
        "preflight_os": ["macos-latest"],
        "preflight_reason": preflight_reason(),
        "flags": {"ci_tooling": False},
    }


def empty_plan(base: str, head: str) -> dict:
    plan = fixed_plan(base, head)
    plan.update(
        change_class="documentation",
        change_inventory={
            "diff_available": True,
            "paths": {name: [] for name in ("configuration", "documentation", "source", "other")},
            "counts": {
                name: 0
                for name in ("configuration", "documentation", "source", "other", "total")
            },
        },
        areas=[],
        packages=[],
        source_packages=[],
        cells=[],
        builds=[],
        job_estimate=0,
        preflight_os=["ubuntu-latest"],
        preflight_reason="no build/test packages affected; preflight runs on the scope host only",
    )
    return plan


def projection(plan: dict) -> dict:
    packages = [entry["package"] for entry in plan["packages"]]
    scheduled = sorted({entry["area"] for entry in plan["packages"] if entry["gates"]})
    owners = build_owners(plan)
    slices = build_slices(owners)
    return {
        "packages": packages,
        "areas": plan["areas"],
        "scheduled_areas": scheduled,
        # The dispatch rows `ci.yml` fans each area out from. Derived here the
        # same way the planner derives them — one row per executing cell,
        # partitioned into the four disjoint sets — so the stub's projection
        # has the shape a receipt reader validates against.
        "area_rows": row_sets(plan),
        "area_slugs": {area: area for area in scheduled},
        "source_packages": plan["source_packages"],
        "reverse_dependencies": plan["reverse_dependencies"],
        "full_scope": plan["full_scope"],
        "full_scope_gates": plan["full_scope_gates"],
        "change_class": plan["change_class"],
        "preflight_os": plan["preflight_os"],
        "preflight_reason": plan["preflight_reason"],
        "policy": [],
        "build_owners": owners,
        "build_slices": slices,
        "build_artifacts": [entry["artifact"] for entry in slices],
        "build_runners": {entry["artifact"]: entry["runner"] for entry in slices},
        "job_estimate": plan["job_estimate"],
        "flags": plan["flags"],
    }


def row_sets(plan: dict) -> dict:
    """`{area: {test, check, lint, wsl, has_*_rows}}`, as the planner emits it."""
    runners = {entry["name"]: entry["runner"] for entry in plan["environments"]}
    names = ("test", "check", "lint", "wsl")
    areas = {
        entry["area"]: {
            **{name: [] for name in names},
            **{f"has_{name}_rows": False for name in names},
        }
        for entry in plan["areas"]
    }
    for cell in plan["cells"]:
        if cell["execution"] != "execute":
            continue
        document = areas.get(cell["area"])
        if document is None:
            continue
        if cell["environment"] == "wsl2-ubuntu":
            name = "wsl"
        elif cell["gate"] in ("check", "lint"):
            name = cell["gate"]
        else:
            name = "test"
        document[name].append(
            {
                "package": cell["package"],
                "gate": cell["gate"],
                "environment": cell["environment"],
                "runner": runners.get(cell["environment"], cell["environment"]),
            }
        )
    for document in areas.values():
        for name in names:
            document[name].sort(
                key=lambda row: (row["package"], row["gate"], row["environment"])
            )
            document[f"has_{name}_rows"] = bool(document[name])
    return areas


def build_owners(plan: dict) -> list:
    """The owner projection, grouped exactly as the real one is."""
    owners: dict = {}
    for record in plan.get("builds", []):
        owner = owners.setdefault(
            record["producer"],
            {
                "environment": record["producer"],
                "runner": record["producer"],
                "packages": [],
                "builds": [],
            },
        )
        owner["builds"].append(record)
    for owner in owners.values():
        owner["packages"] = sorted({entry["package"] for entry in owner["builds"]})
    return [owners[name] for name in sorted(owners)]


def build_slices(owners: list) -> list:
    """The owner matrix flattened one entry per record, as `ci.yml` expands it."""
    slices = [
        {
            "artifact": entry["artifact"],
            "key": entry["key"],
            "package": entry["package"],
            "producer": owner["environment"],
            "runner": owner["runner"],
            "consumers": entry.get("consumers", []),
            "compatible_environments": entry.get("compatible_environments", []),
            "native": [],
        }
        for owner in owners
        for entry in owner["builds"]
    ]
    slices.sort(key=lambda entry: entry["artifact"])
    return slices


def prune_builds(plan: dict) -> None:
    """Drop the demand evidence removed, in place.

    The real overlay derives no key either: it only removes a reference the
    carried plan already computed.
    """
    demanded: dict = {}
    for cell in plan["cells"]:
        if cell["execution"] == "execute" and "build" in cell:
            demanded.setdefault(cell["build"], []).append(
                {"environment": cell["environment"], "gate": cell["gate"]}
            )
        else:
            cell.pop("build", None)
            cell.pop("profile", None)
    plan["builds"] = [
        {**record, "consumers": sorted(
            demanded[record["key"]], key=lambda item: (item["environment"], item["gate"])
        )}
        for record in plan.get("builds", [])
        if demanded.get(record["key"])
    ]


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
    prune_builds(applied)
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
    paths = None
    if "--" in options:
        paths = options[options.index("--") + 1 :]
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
        if paths == [] and "--all" not in flags:
            plan = empty_plan(base, head)
        elif fixture:
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
            for record in plan.get("builds", []):
                record["identity"]["source_commit"] = head
        else:
            plan = fixed_plan(base, head)
        if "--all" in flags:
            # A full-scope request consults no diff, so it records the absence
            # rather than buckets it did not compute.
            plan["change_inventory"] = {
                "diff_available": False,
                "reason": "explicit full-scope request; no diff was consulted",
            }
        if "--event" in values:
            # Like the real planner: the event the plan is for rides in it, so
            # CI's scope-verify can refuse a receipt planned for another.
            plan["event"] = values["--event"]
    if "--plan-out" in values:
        with open(values["--plan-out"], "w", encoding="utf-8") as handle:
            handle.write(json.dumps(plan, sort_keys=True, separators=(",", ":")))
    document = plan if "--resolved-plan" in flags else projection(plan)
    print(json.dumps(document, separators=(",", ":")))


if __name__ == "__main__":
    main()

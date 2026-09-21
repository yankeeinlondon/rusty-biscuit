#!/usr/bin/env python3
"""Phase 7's capacity evidence: the shipped adapter against GitHub's limits.

S3 (`spikes/s3-capacity.md`) modeled capacity from the Phase 1 corpus before
any workflow changed. This re-measures it from the shipped planner, row
adapter, scope projection, and workflows, for the plans that matter to
capacity: the full workspace with no event, a push to `main`, a manual
dispatch, and the nightly schedule. It reads nothing hosted.

For each plan it prints every individual matrix's cardinality, the serialized
sizes of the outputs that carry them, and a runner-job estimate that counts
area audits, gap publishers, and build owners. It then prints the reusable
workflow call depth and the unique reusable-workflow count, read from the
shipped YAML. It exits non-zero if the planner's own guard would refuse any of
these plans.

Run from the repository root, against this spec's `spikes/` directory
wherever its lifecycle currently places it (`features/` or
`features/_completed/`); the script finds the repository root itself.
"""

from __future__ import annotations

import json
import re
import sys
from datetime import date
from pathlib import Path

# Found by walking up, not by a fixed depth: closing a spec moves it between
# lifecycle directories, and a counted `parents[n]` then names the wrong root.
ROOT = next(
    parent
    for parent in Path(__file__).resolve().parents
    if (parent / "scripts" / "ci" / "affected_scope.py").is_file()
)
sys.path.insert(0, str(ROOT / "scripts" / "ci"))

import affected_scope  # noqa: E402
import schema  # noqa: E402
from affected_scope import (  # noqa: E402
    ENVIRONMENTS_CONFIG,
    calculate_scope,
    legacy_scope_document,
    load_environments,
    load_metadata,
    package_ci_policy,
    workspace_packages,
)

WORKFLOWS = ROOT / ".github" / "workflows"
USES = re.compile(r"^\s*uses:\s*\./\.github/workflows/(\S+)\s*$", re.MULTILINE)

#: The runner jobs ci.yml schedules once per run, `ci-reporting` included
#: (it runs `always()`).
FIXED_JOBS = ("validation", "scope", "ci-gate", "ci-reporting")

SHAPES = {
    "all": {"force_all": True},
    "push": {"force_all": True, "event": "push"},
    "dispatch": {"force_all": True, "event": "workflow_dispatch"},
    "nightly": {"force_all": True, "event": "schedule"},
}


def plan_for(options: dict[str, object]) -> dict:
    metadata = load_metadata(ROOT)
    environments = load_environments(ENVIRONMENTS_CONFIG, today=date.today())
    policy = package_ci_policy(
        workspace_packages(metadata),
        runner_labels={entry["runner"] for entry in environments},
        root=ROOT,
        today=date.today(),
    )
    return calculate_scope([], ROOT, metadata, environments, policy, **options)


def compact(value: object) -> int:
    # `jq -c` form, which is what `ci.yml` writes to `$GITHUB_OUTPUT`.
    return len(json.dumps(value, separators=(",", ":")))


def measure(plan: dict) -> dict:
    scope = legacy_scope_document(plan)
    rows = scope["area_rows"]
    gap_areas = sorted({cell["area"] for cell in plan["cells"] if cell["state"] == "accepted-gap"})
    scheduled = scope["scheduled_areas"]
    counts = {
        name: sum(len(document[name]) for document in rows.values())
        for name in schema.ROW_SET_NAMES
    }
    largest = max(
        (
            (len(document[name]), area, name)
            for area, document in rows.items()
            for name in schema.ROW_SET_NAMES
        ),
        default=(0, "-", "-"),
    )
    payloads = {area: compact(document) for area, document in rows.items()}
    calling = [area for area, document in rows.items()
               if any(document[f"has_{name}_rows"] for name in schema.ROW_SET_NAMES)]
    # R3: the test job's L2 and browser rows now start beside L1 instead of
    # staging behind it, so each area's peak rises by that many runners.
    staged = {
        area: sum(1 for row in document["test"] if row["gate"] != "L1")
        for area, document in rows.items()
    }
    runner_jobs = {
        "fixed": len(FIXED_JOBS),
        "preflight": len(plan["preflight_os"]),
        "build owners": len(scope["build_owners"]),
        "area-drift": int(bool(plan["flags"].get("area_drift"))),
        "coverage-audit": len(scheduled),
        "accepted-gaps": len([area for area in gap_areas if area in scheduled]),
        "producer rows": sum(counts.values()),
    }
    call_jobs = {
        "area-ci": len(scheduled),
        "package-ci": len([area for area in calling if area in scheduled]),
        "wsl2": counts["wsl"],
    }
    return {
        "executing": sum(1 for cell in plan["cells"] if cell["execution"] == "execute"),
        "areas": len(scheduled),
        "counts": counts,
        "largest": largest,
        "build_owners": len(scope["build_owners"]),
        "largest_payload": max(payloads.items(), key=lambda item: item[1], default=("-", 0)),
        "area_rows": compact(rows),
        "scope_outputs": {
            name: compact(scope[name])
            for name in ("area_rows", "build_owners", "build_slices", "policy",
                         "build_runners", "build_artifacts")
        },
        "scope_total": compact(scope),
        "runner_jobs": runner_jobs,
        "call_jobs": call_jobs,
        "peak_rise": max(staged.items(), key=lambda item: item[1], default=("-", 0)),
        "job_estimate": plan["job_estimate"],
    }


def reusable_chain() -> tuple[int, set[str]]:
    unique: set[str] = set()

    def depth(name: str, seen: tuple[str, ...]) -> int:
        if name in seen:
            raise SystemExit(f"reusable-workflow cycle through {name}")
        called = USES.findall((WORKFLOWS / name).read_text(encoding="utf-8"))
        unique.update(called)
        return 1 + max((depth(entry, seen + (name,)) for entry in called), default=0)

    return depth("ci.yml", ()), unique


def main() -> int:
    refused = 0
    for label, options in SHAPES.items():
        try:
            plan = plan_for(options)
        except RuntimeError as error:
            print(f"{label}: REFUSED by the planner: {error}")
            refused += 1
            continue
        result = measure(plan)
        print(f"== {label}")
        print(json.dumps(result, indent=2, sort_keys=True))
        print(
            f"   runner jobs: {sum(result['runner_jobs'].values())}; "
            f"call jobs: {sum(result['call_jobs'].values())}"
        )
    depth, unique = reusable_chain()
    print(f"== reusable workflows: depth {depth} (incl. caller); unique {sorted(unique)}")
    print(
        f"== ceilings: matrix {affected_scope.MATRIX_LIMIT}; per-area rows "
        f"{affected_scope.AREA_ROW_SET_BUDGET} B; all rows "
        f"{affected_scope.TOTAL_ROW_SET_BUDGET} B"
    )
    return 1 if refused else 0


if __name__ == "__main__":
    sys.exit(main())

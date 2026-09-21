#!/usr/bin/env python3
"""The derived halves of the plan documents the CI suites write by hand.

`test_evidence_reuse`, `test_ci_local`, `test_local_evidence`, and the
`just ci-local` stub planner all assemble a resolved plan literally, because
what they test is what happens *after* planning. Every generation of the schema
adds fields that follow mechanically from the cells — build records at version
3, the nextest profile and the skip-policy snapshot at version 5 — and
hand-writing them in four places would guarantee they drift from the cells they
claim to serve.

[`finalize_plan`] derives all of them from the cells, the same way
`affected_scope` does. It is deliberately a plain fixture helper: the real
derivation reads the environment table's build contracts, the shipped baseline
file, and digests through `ci-build`, none of which a document fixture has or
needs.
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent))

import schema  # noqa: E402


#: Which native environment compiles for each execution environment. Mirrors
#: `.github/ci/environments.json`: the WSL2 guest consumes the Linux producer's
#: archive and every other environment compiles for itself.
PRODUCER_OF = {
    "ubuntu-latest": "ubuntu-latest",
    "wsl2-ubuntu": "ubuntu-latest",
    "macos-latest": "macos-latest",
    "windows-latest": "windows-latest",
}

CONTRACTS: dict[str, dict[str, Any]] = {
    "ubuntu-latest": {
        "target": "x86_64-unknown-linux-gnu",
        "linker": "cc",
        "executes": ["ubuntu-latest", "wsl2-ubuntu"],
    },
    "macos-latest": {
        "target": "aarch64-apple-darwin",
        "linker": "cc",
        "executes": ["macos-latest"],
    },
    "windows-latest": {
        "target": "x86_64-pc-windows-msvc",
        "linker": "link.exe",
        "executes": ["windows-latest"],
    },
}


def _key(package: str, producer: str) -> str:
    """A stable stand-in for the digest `ci-build key` computes.

    Sixteen hex digits, as the contract requires, and a function of
    `{package, producer}` so two cells that share a producer share a key —
    which is the property every fixture here is written to exercise.
    """
    material = f"{package}\x1f{producer}".encode("utf-8")
    return f"{int.from_bytes(material[:8].ljust(8, b'0'), 'big') ^ len(material):016x}"


#: The snapshot every fixture plan carries. The shipped `ci-baseline.toml` is
#: empty, so the honest fixture approves nothing; a case that needs an approval
#: replaces `entries` after calling [`finalize_plan`].
EMPTY_SKIP_POLICY: dict[str, Any] = {
    "source": ".github/ci/ci-baseline.toml",
    "content_hash": "aaaabbbbccccdddd",
    "entries": [],
}


def finalize_plan(plan: dict[str, Any]) -> dict[str, Any]:
    """Give `plan` every field a valid current-generation plan derives.

    Build records and their per-cell references ([`attach_builds`]), the
    nextest profile on exactly the cells that run one, the area-level
    execution path, and the skip-policy snapshot. Mutates and returns `plan`,
    and is idempotent: a fixture that resolves a cell to reuse or prohibition
    calls it again and watches the cell's derived fields disappear with the
    execution they described.
    """
    attach_builds(plan)
    for cell in plan["cells"]:
        if cell["execution"] == "execute" and cell["gate"] in schema.BUILD_GATES:
            cell.setdefault("profile", schema.CI_PROFILE)
        else:
            cell.pop("profile", None)
            cell.pop("requires_node", None)
    for area in plan.get("areas", ()):
        area.setdefault("execution_path", "rows")
    plan.setdefault("skip_policy", dict(EMPTY_SKIP_POLICY))
    return plan


def attach_builds(plan: dict[str, Any]) -> dict[str, Any]:
    """Give `plan` the build records its executing test cells demand.

    Mutates the cells in place — an executing L1/L2/browser cell gains its
    `build` reference and every other cell loses one — and replaces `builds`.
    Idempotent, so a fixture that resolves a cell to reuse or prohibition can
    call it again and watch the record disappear.
    """
    records: dict[str, dict[str, Any]] = {}
    for cell in plan["cells"]:
        if cell["execution"] != "execute" or cell["gate"] not in schema.BUILD_GATES:
            cell.pop("build", None)
            continue
        producer = PRODUCER_OF[cell["environment"]]
        contract = CONTRACTS[producer]
        key = _key(cell["package"], producer)
        cell["build"] = key
        record = records.setdefault(
            key,
            {
                "key": key,
                "package": cell["package"],
                "producer": producer,
                "artifact": f"build-{cell['package']}-{producer}-{key}",
                "compatible_environments": sorted(contract["executes"]),
                "compatibility_reason": (
                    f"{contract['target']} archive produced on {producer}"
                ),
                "consumers": [],
                "identity": {
                    "source_commit": plan["head"],
                    "lockfile": "aaaabbbbccccdddd",
                    "rust": "1.97.1",
                    "nextest": "latest",
                    "host": contract["target"],
                    "target": contract["target"],
                    "profile": "test",
                    "rustflags": "",
                    "cargo_config": [],
                    "linker": contract["linker"],
                    "archive_format": "tar.zst",
                    "package": cell["package"],
                    "target_kinds": ["lib", "test"],
                    "features": "",
                    "native": [],
                    "archive_includes": [],
                    "sidecars": [],
                },
            },
        )
        record["consumers"].append(
            {"environment": cell["environment"], "gate": cell["gate"]}
        )

    for record in records.values():
        record["consumers"].sort(key=lambda entry: (entry["environment"], entry["gate"]))
    plan["builds"] = sorted(
        records.values(), key=lambda record: (record["package"], record["key"])
    )
    return plan

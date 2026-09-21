#!/usr/bin/env python3
"""Build records for the plan documents the CI suites write by hand.

`test_evidence_reuse`, `test_ci_local`, and the `just ci-local` stub planner all
assemble a resolved plan literally, because what they test is what happens
*after* planning. Since version 3 a valid plan also owns its build records, and
hand-writing them in three places would guarantee they drift from the cells they
claim to serve.

This derives them from the cells instead, the same way `affected_scope` does,
and is deliberately a plain fixture helper: the real derivation reads the
environment table's build contracts and digests through `ci-build`, neither of
which a document fixture has or needs.

[`archive_guard`] is here for the same reason: version 5 made the guard's scan
scope a required field, and it is a selection decision no post-planning fixture
makes.
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


def archive_guard(paths: list[str] | None = None) -> dict[str, Any]:
    """The archive-path guard scope a hand-written plan must carry.

    The default is the honest one for a fixture that performed no selection:
    the guard was not selected, so it describes no scope. Pass `paths` where a
    fixture needs a changed-file scan to be present.
    """
    if paths is None:
        return {
            "selected": False,
            "reason": "fixture plan; no scan scope was selected",
        }
    return {
        "selected": True,
        "mode": "changed",
        "paths": sorted(set(paths)),
        "reason": "fixture plan; changed-file scan",
    }


def _key(package: str, producer: str) -> str:
    """A stable stand-in for the digest `ci-build key` computes.

    Sixteen hex digits, as the contract requires, and a function of
    `{package, producer}` so two cells that share a producer share a key —
    which is the property every fixture here is written to exercise.
    """
    material = f"{package}\x1f{producer}".encode("utf-8")
    return f"{int.from_bytes(material[:8].ljust(8, b'0'), 'big') ^ len(material):016x}"


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

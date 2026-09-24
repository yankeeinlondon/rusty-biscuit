#!/usr/bin/env python3
"""Phase 1 test-input probe: which non-source files the ten packages' tests read.

Runs the planner's own index (`scripts/ci/test_inputs.py`) over every tracked
non-source path, restricted to the integration-test targets of the ten
packages, and prints each reference with the narrowed unit the planner would
schedule. Re-run it after a package's move and diff the `unit` column: the
identities must come from walking the new target roots, not from a stored
table.

Usage: test-input-probe.py [--json OUT]
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path, PurePosixPath

REPO = Path(subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip())
sys.path.insert(0, str(REPO / "scripts" / "ci"))

import affected_scope  # noqa: E402
import test_inputs  # noqa: E402

PACKAGES = (
    "tree-hugger", "claudine", "sniff", "biscuit-file", "schematic-gen",
    "biscuit-terminal-cli", "claudine-gen", "dmls", "sniff-cli", "biscuit-tui-cli",
)


def main() -> int:
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1", "--offline"], cwd=REPO, text=True))
    packages = [p for p in metadata["packages"] if p["name"] in PACKAGES]
    include_slow = frozenset(
        p["name"] for p in packages
        if ((p.get("metadata") or {}).get("ci") or {}).get("tests", {}).get("l1-include-slow"))
    targets = [t for t in test_inputs.targets_from_metadata(packages, REPO.as_posix()) if t.kind == "test"]
    tracked = subprocess.check_output(["git", "ls-files"], cwd=REPO, text=True).splitlines()
    candidates = [p for p in tracked if not affected_scope.is_package_source_path(PurePosixPath(p))]

    def read(path: str) -> str | None:
        try:
            return (REPO / path).read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            return None

    references = test_inputs.scan(targets, read, candidates, include_slow)
    rows = [
        {"package": r.package, "path": r.path, "source": r.source, "line": r.line,
         "embedded": r.embedded, "unit": r.unit}
        for r in references
    ]
    if len(sys.argv) == 3 and sys.argv[1] == "--json":
        Path(sys.argv[2]).write_text(json.dumps(rows, indent=2) + "\n")
    for row in rows:
        print(f"{row['package']}\t{row['path']}\t{row['source']}:{row['line']}\t{row['unit']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

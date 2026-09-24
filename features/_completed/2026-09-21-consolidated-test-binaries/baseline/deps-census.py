#!/usr/bin/env python3
"""Census of the four packages' test executables in `target/debug/deps`.

Counts every `<target>-<16 hex>` executable whose `<target>` is a current test
target of a package in scope, including stale copies from earlier feature
sets. A target name two packages share is counted under both, so the rows are
upper bounds. Run from the repository root.
"""

from __future__ import annotations

import collections
import json
import os
import re
import subprocess

PACKAGES = ("claudine-cli", "darkmatter", "darkmatter-cli", "biscuit-terminal")
EXECUTABLE = re.compile(r"^(.+)-([0-9a-f]{16})$")


def main() -> int:
    meta = json.loads(subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1", "--color=never"],
        check=True, capture_output=True, text=True,
    ).stdout)
    owners: dict[str, set[str]] = collections.defaultdict(set)
    for package in meta["packages"]:
        if package["name"] in PACKAGES:
            for target in package["targets"]:
                if "test" in target["kind"]:
                    owners[target["name"].replace("-", "_")].add(package["name"])

    deps = os.path.join(meta["target_directory"], "debug", "deps")
    stats = collections.defaultdict(lambda: [0, 0, set()])
    large = [0, 0]
    for entry in os.scandir(deps):
        if not entry.is_file(follow_symlinks=False) or not os.access(entry.path, os.X_OK):
            continue
        info = entry.stat(follow_symlinks=False)
        allocated = info.st_blocks * 512
        if info.st_size > 1 << 20 and "." not in entry.name:
            large[0] += 1
            large[1] += allocated
        match = EXECUTABLE.match(entry.name)
        if not match or match.group(1) not in owners:
            continue
        for package in owners[match.group(1)]:
            row = stats[package]
            row[0] += 1
            row[1] += allocated
            row[2].add(match.group(1))

    print("package\texecutables\tdistinct_targets\tallocated_bytes")
    for package, (count, size, names) in sorted(stats.items()):
        print(f"{package}\t{count}\t{len(names)}\t{size}")
    print(f"all_executables_over_1MB\t{large[0]}\t-\t{large[1]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

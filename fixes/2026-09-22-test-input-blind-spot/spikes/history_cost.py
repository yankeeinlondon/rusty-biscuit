#!/usr/bin/env python3
"""How often the test-input rule would have added a narrowed cell, over real history.

Run from the repository root: `python3 <this> merges` (main's first-parent
history, the unit CI plans) or `python3 <this> commits` (every non-merge commit,
the unit a pre-push hook sees at most). Since 2026-08-01.

An approximation, stated rather than hidden: references come from TODAY's tree,
not each commit's, and "already selected" models source and suite ownership but
not embedded-source selection. Both bias toward counting a cell that the real
planner, at that commit, might not have added.
"""
import collections
import json
import os
import subprocess
import sys
from pathlib import Path, PurePosixPath

sys.path.insert(0, "scripts/ci")
import affected_scope as planner  # noqa: E402
import test_inputs  # noqa: E402

root = Path(".").resolve()
metadata = json.loads(subprocess.run(
    ["cargo", "metadata", "--format-version", "1", "--no-deps"],
    capture_output=True, check=True, text=True,
).stdout)
packages = planner.workspace_packages(metadata)
directories = sorted(
    ((os.path.dirname(p["manifest_path"]).removeprefix(f"{root}/"), p["name"]) for p in packages.values()),
    key=lambda entry: -len(entry[0]),
)


def owner(path: str) -> str | None:
    return next((name for directory, name in directories if path.startswith(f"{directory}/")), None)


def suite_owners(path: str) -> set[str]:
    found = set(planner.SUITE_OWNER_PATHS.get(path, ()))
    for prefix, owners in planner.SUITE_OWNER_PREFIXES:
        if path.startswith(prefix):
            found |= set(owners)
    return found


def is_source(path: str) -> bool:
    return planner.is_package_source_path(PurePosixPath(path))


mode = sys.argv[1] if len(sys.argv) > 1 else "merges"
log = ["git", "log", "--since=2026-08-01", "--format=%H", "main"]
log.insert(2, "--first-parent" if mode == "merges" else "--no-merges")
revisions = subprocess.run(log, capture_output=True, check=True, text=True).stdout.split()

changes = {}
for revision in revisions:
    records = [r.decode() for r in subprocess.run(
        ["git", "diff", "--name-status", "-z", "-M", f"{revision}^", revision],
        capture_output=True, check=True,
    ).stdout.split(b"\0") if r]
    changed, gone, index = [], [], 0
    while index < len(records):
        status = records[index]
        if status[0] in "RC":
            source, destination = records[index + 1], records[index + 2]
            changed.append(destination)
            if status[0] == "R":
                gone.append(source)
            index += 3
        else:
            changed.append(records[index + 1])
            if status[0] == "D":
                gone.append(records[index + 1])
            index += 2
    changes[revision] = (changed, gone)

candidates = {p for changed, gone in changes.values() for p in changed + gone if not is_source(p)}
targets = test_inputs.targets_from_metadata(packages.values(), root.as_posix())
references = collections.defaultdict(list)
for reference in test_inputs.scan(targets, planner.worktree_reader(root), candidates):
    references[reference.path].append(reference)

stats, per_package = collections.Counter(), collections.Counter()
for changed, gone in changes.values():
    selected = {owner(p) for p in changed if is_source(p)} | {o for p in changed for o in suite_owners(p)}
    narrowed = {
        reference.package
        for path in {p for p in changed + gone if not is_source(p)}
        for reference in references.get(path, [])
        if not reference.product and reference.unit and reference.package not in selected
    }
    stats["revisions"] += 1
    stats["revisions adding narrowed cells"] += bool(narrowed)
    stats["narrowed cells"] += len(narrowed)
    per_package.update(narrowed)
print(mode, dict(stats))
print("per package:", per_package.most_common())

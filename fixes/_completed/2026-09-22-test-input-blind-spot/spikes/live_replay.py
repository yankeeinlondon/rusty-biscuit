#!/usr/bin/env python3
"""Replay the three docs-only commits through the current planner, before and after.

Run from the repository root with full history. Each commit is materialized in
a temporary worktree and planned from its own `cargo metadata`, exactly as the
pre-push hook plans a committed tree; "before" is the same planner with the
test-input search switched off.
"""
import json
import subprocess
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, "scripts/ci")
import affected_scope as planner  # noqa: E402
import diff_scope  # noqa: E402
import schema  # noqa: E402

COMMITS = ("869cbb225", "aa1f03c70", "330805a84")

environments = planner.load_environments(planner.ENVIRONMENTS_CONFIG)
# The historical manifests name a since-retired runner tool; selection reads none.
planner.validate_package_ci = lambda *args, **kwargs: None

for commit in COMMITS:
    with tempfile.TemporaryDirectory() as temporary:
        tree = Path(temporary) / "tree"
        subprocess.run(["git", "worktree", "add", "--detach", "-q", str(tree), commit], check=True)
        try:
            stream = subprocess.run(
                ["git", "diff", "--name-status", "-z", f"{commit}^", commit],
                capture_output=True, check=True,
            ).stdout
            changed, deleted, renamed = (
                [path.decode() for path in paths] for paths in diff_scope.parse_name_status(stream)
            )
            metadata = json.loads(subprocess.run(
                ["cargo", "metadata", "--format-version", "1", "--offline"],
                cwd=tree, capture_output=True, check=True, text=True,
            ).stdout)
            policy = planner.package_ci_policy(
                planner.workspace_packages(metadata), {e["runner"] for e in environments}, tree
            )

            def plan() -> tuple[dict, float]:
                started = time.time()
                document = planner.calculate_scope(
                    changed, tree, metadata, environments, policy, event="pull_request",
                    deleted=deleted, renamed_from=renamed,
                )
                return document, time.time() - started

            after, seconds = plan()
            search = planner.test_input_references
            planner.test_input_references = lambda *args, **kwargs: []
            before, _ = plan()
            planner.test_input_references = search

            print(f"== {commit}  changed={len(changed)} deleted={deleted} renamed_from={renamed}")
            print(f"   before: change_class={before['change_class']} "
                  f"packages={[entry['package'] for entry in before['packages']]} cells={len(before['cells'])}")
            print(f"   after:  change_class={after['change_class']} planning took {seconds:.1f}s; "
                  f"schema problems={schema.validate_resolved_plan(after)}")
            for cell in after["cells"]:
                print(f"     {cell['package']}/{cell['environment']}/{cell['gate']} {cell['execution']} "
                      f"filter={cell.get('test_filter')}")
        finally:
            subprocess.run(["git", "worktree", "remove", "--force", str(tree)], check=True)

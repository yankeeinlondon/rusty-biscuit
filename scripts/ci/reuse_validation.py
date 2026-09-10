#!/usr/bin/env python3
"""Reuse PR CI only for the same tested tree and the same integration base.

The receipt's artifact name is the versioned identity; no archive download is
needed. It records the actual checkout, not Actions' run-level head SHA (which
identifies the PR head rather than the synthetic merge that checkout tests).
Only a completed, successful run of this repository's ci.yml can supply it.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
from pathlib import Path
from typing import Any, Callable


def receipt_name(tree: str, base: str, head: str) -> str:
    if not all(re.fullmatch(r"[0-9a-f]{40}", sha) for sha in (tree, base, head)):
        raise ValueError("receipt requires full Git object IDs")
    return f"ci-validation-v1-{tree}-{base}-{head}"


def git_revision(ref: str) -> str:
    return subprocess.run(
        ["git", "rev-parse", "--verify", ref], check=True,
        capture_output=True, text=True, timeout=15,
    ).stdout.strip()


def record_receipt(event: dict[str, Any], sha: str) -> str:
    """Refuse evidence if checkout stops testing the event's synthetic merge."""
    pr = event["pull_request"]
    base, head = pr["base"]["sha"], pr["head"]["sha"]
    if (git_revision("HEAD") != sha
            or git_revision("HEAD^1") != base
            or git_revision("HEAD^2") != head):
        raise ValueError("PR checkout does not match its merge, base, and head")
    return receipt_name(git_revision("HEAD^{tree}"), base, head)


def github_api(endpoint: str) -> Any:
    return json.loads(subprocess.run(
        ["gh", "api", endpoint], check=True, capture_output=True,
        text=True, timeout=15,
    ).stdout)


def find_validation(
    repo: str, sha: str, before: str, tree: str,
    api: Callable[[str], Any] = github_api,
) -> tuple[str, str]:
    """Return a run ID and reason; bounded searches may miss reuse, never tests."""
    receipt_name(tree, before, sha)
    if before == "0" * 40:
        return "", "A new main branch requires normal CI."
    root = f"repos/{repo}"
    prs = api(f"{root}/commits/{sha}/pulls?per_page=100")
    for pr in prs[:3]:
        if (not pr.get("merged_at") or pr.get("merge_commit_sha") != sha
                or pr["base"]["ref"] != "main"
                or pr["base"]["repo"]["full_name"] != repo):
            continue
        head = pr["head"]["sha"]
        expected = receipt_name(tree, before, head)
        runs = api(
            f"{root}/actions/workflows/ci.yml/runs"
            f"?event=pull_request&head_sha={head}&per_page=20"
        )["workflow_runs"]
        candidates = [run for run in runs
                      if run.get("event") == "pull_request"
                      and run.get("path") == ".github/workflows/ci.yml"
                      and run.get("head_sha") == head
                      and run["repository"]["full_name"] == repo]
        if not candidates:
            continue
        # Do not resurrect an older green run after a newer failure or rerun.
        run = max(candidates, key=lambda item: int(item["id"]))
        if run.get("status") != "completed" or run.get("conclusion") != "success":
            return "", "The latest PR CI run has not completed successfully."
        run_id = str(int(run["id"]))
        artifacts = api(
            f"{root}/actions/runs/{run_id}/artifacts?name={expected}&per_page=100"
        )["artifacts"]
        if any(a.get("name") == expected and a.get("expired") is False
               for a in artifacts):
            return run_id, "The tested PR tree and integration base match this push."
    return "", "No successful PR validation receipt matches this push."


def check_validation(
    event_name: str, ref: str, event: dict[str, Any], repo: str, sha: str,
    api: Callable[[str], Any] = github_api,
) -> tuple[str, str]:
    if event_name != "push" or ref != "refs/heads/main":
        return "", "PR and manual runs always perform normal CI."
    try:
        if git_revision("HEAD") != sha:
            return "", "The checkout does not match the pushed commit."
        return find_validation(repo, sha, event["before"], git_revision("HEAD^{tree}"), api)
    except Exception:
        # Missing permissions, rate limits, timeouts, malformed API data, or
        # unavailable Git objects must schedule normal CI, never skip it.
        return "", "Validation evidence could not be verified; running normal CI."


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("record", "check"))
    args = parser.parse_args()
    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text(encoding="utf-8"))
    sha = os.environ["GITHUB_SHA"]
    if args.mode == "record":
        name = record_receipt(event, sha)
        Path("ci-validation.txt").write_text(name + "\n", encoding="utf-8")
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
            output.write(f"receipt={name}\n")
        return
    run_id, reason = check_validation(
        os.environ["GITHUB_EVENT_NAME"], os.environ["GITHUB_REF"], event,
        os.environ["GITHUB_REPOSITORY"], sha,
    )
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        output.write(f"reuse={'true' if run_id else 'false'}\nrun_id={run_id}\n")
    with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as summary:
        summary.write(f"## PR validation reuse\n\n{reason}\n")
        if run_id:
            url = f"{os.environ['GITHUB_SERVER_URL']}/{os.environ['GITHUB_REPOSITORY']}/actions/runs/{run_id}"
            summary.write(f"\n[Original validation]({url})\n\nGit tree: `{git_revision('HEAD^{tree}')}`\n")


if __name__ == "__main__":
    main()

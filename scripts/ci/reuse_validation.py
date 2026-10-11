#!/usr/bin/env python3
"""Reuse PR CI only for the same tested tree and the same integration base.

The receipt's artifact name is the versioned identity; no archive download is
needed. It records the tree of the actual checkout, which `ci.yml` pins to the
PR head (`TESTED_REVISION`: the revision the plan names and `ci-build` binds
every archive to), never GitHub's synthetic merge. A merge commit on `main`
matches only when its tree equals that head's tree, so a PR that was behind
`main` gets normal CI after the merge: the merged tree was never tested.
Only a completed, successful run of this repository's ci.yml can supply it.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time
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


def record_receipt(event: dict[str, Any]) -> str:
    """Refuse evidence if checkout stops testing the event's PR head."""
    pr = event["pull_request"]
    base, head = pr["base"]["sha"], pr["head"]["sha"]
    if git_revision("HEAD") != head:
        raise ValueError("PR checkout does not match the pull request head")
    return receipt_name(git_revision("HEAD^{tree}"), base, head)


def github_api(endpoint: str) -> Any:
    return json.loads(subprocess.run(
        ["gh", "api", endpoint], check=True, capture_output=True,
        text=True, timeout=15,
    ).stdout)


def pending_waiter(minutes: float, interval: float = 60.0) -> Callable[[], bool]:
    """Sleep one poll interval and return True until `minutes` have passed."""
    deadline = time.monotonic() + minutes * 60

    def wait() -> bool:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            return False
        time.sleep(min(interval, remaining))
        return True

    return wait


def find_validation(
    repo: str, sha: str, before: str, tree: str,
    api: Callable[[str], Any] = github_api,
    wait: Callable[[], bool] = lambda: False,
) -> tuple[str, str]:
    """Return a run ID and reason; bounded searches may miss reuse, never tests.

    A PR merged while its CI run is still queued or in progress would
    otherwise get the full grid again; `wait` lets the push hold until that
    run finishes, and returns False once its budget is spent.
    """
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
        while True:
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
                break
            # Do not resurrect an older green run after a newer failure or rerun.
            run = max(candidates, key=lambda item: int(item["id"]))
            if run.get("status") == "completed" or not wait():
                break
        if not candidates:
            continue
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
    wait: Callable[[], bool] = lambda: False,
) -> tuple[str, str]:
    if event_name != "push" or ref != "refs/heads/main":
        return "", "PR and manual runs always perform normal CI."
    try:
        if git_revision("HEAD") != sha:
            return "", "The checkout does not match the pushed commit."
        return find_validation(repo, sha, event["before"], git_revision("HEAD^{tree}"), api, wait)
    except Exception:
        # Missing permissions, rate limits, timeouts, malformed API data, or
        # unavailable Git objects must schedule normal CI, never skip it.
        return "", "Validation evidence could not be verified; running normal CI."


def git_is_ancestor(sha: str) -> bool:
    return subprocess.run(
        ["git", "merge-base", "--is-ancestor", sha, "HEAD"],
        capture_output=True, text=True, timeout=15,
    ).returncode == 0


def nightly_base(
    repo: str,
    api: Callable[[str], Any] = github_api,
    is_ancestor: Callable[[str], bool] = git_is_ancestor,
) -> tuple[str, str]:
    """The head of the newest successful nightly that this head descends from.

    The nightly plans what changed since the last nightly that proved the
    workspace (fixes/2026-09-19-nightly-scope), so the base is the newest
    completed, successful `schedule` run of `ci.yml` on `main` whose head is
    an ancestor of the current one. A failed nightly is never a base: the
    packages it failed are still in the diff from the last success. No such
    run — the first nightly, or a history rewrite — yields `""`, and the
    caller plans the full workspace.
    """
    runs = api(
        f"repos/{repo}/actions/workflows/ci.yml/runs"
        "?event=schedule&branch=main&status=success&per_page=20"
    )["workflow_runs"]
    candidates = [
        run for run in runs
        if run.get("event") == "schedule"
        and run.get("path") == ".github/workflows/ci.yml"
        and run.get("status") == "completed"
        and run.get("conclusion") == "success"
        and run["repository"]["full_name"] == repo
    ]
    for run in sorted(candidates, key=lambda item: int(item["id"]), reverse=True):
        sha = str(run.get("head_sha", ""))
        if re.fullmatch(r"[0-9a-f]{40}", sha) and is_ancestor(sha):
            return sha, f"Planning what changed since the last successful nightly, run {int(run['id'])} at {sha[:9]}."
    return "", "No successful nightly on main is an ancestor of this head; planning the full workspace."


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("record", "check", "nightly-base"))
    parser.add_argument(
        "--wait-minutes", type=float, default=0.0,
        help="check: how long to wait for an unfinished PR run of the merged head",
    )
    args = parser.parse_args()
    if args.mode == "nightly-base":
        try:
            sha, reason = nightly_base(os.environ["GITHUB_REPOSITORY"])
        except Exception:
            # An API or Git failure plans the full workspace, never nothing.
            sha, reason = "", "The last nightly could not be determined; planning the full workspace."
        print(sha)
        print(reason, file=sys.stderr)
        return
    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text(encoding="utf-8"))
    sha = os.environ["GITHUB_SHA"]
    if args.mode == "record":
        name = record_receipt(event)
        Path("ci-validation.txt").write_text(name + "\n", encoding="utf-8")
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
            output.write(f"receipt={name}\n")
        return
    run_id, reason = check_validation(
        os.environ["GITHUB_EVENT_NAME"], os.environ["GITHUB_REF"], event,
        os.environ["GITHUB_REPOSITORY"], sha,
        wait=pending_waiter(args.wait_minutes),
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

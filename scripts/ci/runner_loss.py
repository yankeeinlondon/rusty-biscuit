#!/usr/bin/env python3
"""Attribute jobs whose hosted runner died, and decide whether to retry them.

A job whose runner stops talking to GitHub ("The hosted runner lost
communication with the server") is terminated after roughly 45 minutes with
every step left `in_progress`/`null` and no log uploaded. Its producer status
step never runs, so `ci-rollup` can only render `MISSING` — the same cell a
genuinely absent report gets — and nothing distinguishes a dead machine from a
broken package. Seen three times on 2026-08-27/28, every time on the WSL2
guest-provisioning step, every time during Windows-runner saturation, and every
time green on a plain rerun.

Two consumers, one classification:

- `ci-verdict` runs `attribute` before the rollup. For every runner-lost job it
  synthesizes the `status-<package>-<job>[-<environment>]/status.json` the dead
  producer would have written, with `detail` naming the step that was running,
  so the grid reads `MISSING — … the hosted runner lost communication …`. A
  status a live producer did upload is never overwritten.
- `ci-infra-retry.yml` runs `decide` once the run has completed (the rerun
  endpoint refuses an in-progress run). It reruns the failed jobs only when
  this is attempt 1 AND every failed job is runner-lost: a real test failure
  is never retried, and a second loss is left for a human.

Both read the same GitHub data — the run's jobs and each failed job's check-run
annotations — through `gh api`, and the classification itself is pure so the
tests never touch the network.
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

RUNNER_LOST_MARKER = "lost communication with the server"

# `<package> / <job> (<package> on <environment>)`, with the WSL2 leg nested one
# level deeper. The producer status names come from `_package-ci.yml` and
# `_wsl-ci.yml`: L1 for `test`, L2 for `test-l2`, and the literal job name for
# `check`, `lint`, and `browser`. `lint` carries no environment (its artifact
# is `status-<package>-lint`), and the WSL2 `archive` job records no status of
# its own — its loss surfaces through the dependent `test` leg's upstream edge.
JOB_KINDS = {
    "test": "L1",
    "test-l2": "L2",
    "test-browser": "browser",
    "check": "check",
    "lint": "lint",
}
NATIVE_JOB = re.compile(
    r"^(?P<package>[A-Za-z0-9_.-]+) / (?P<kind>test|test-l2|test-browser|check|lint) "
    r"\((?P=package)(?: on (?P<environment>[A-Za-z0-9_.-]+))?\)$"
)
WSL_JOB = re.compile(
    r"^(?P<package>[A-Za-z0-9_.-]+) / wsl2 \((?P=package)\) / test "
    r"\((?P=package) on (?P<environment>[A-Za-z0-9_.-]+)\)$"
)

# Jobs that judge the run rather than produce evidence. Their failure follows
# from the producers' and must not veto a retry.
NON_PRODUCER_JOBS = {"ci-verdict", "Unrolled-up job summary"}


def parse_job_name(name: str) -> dict[str, str | None] | None:
    """Map a GitHub job name to the producer cell it would have reported."""
    match = WSL_JOB.match(name)
    if match:
        return {
            "package": match["package"],
            "job": "L1",
            "environment": match["environment"],
        }
    match = NATIVE_JOB.match(name)
    if match:
        return {
            "package": match["package"],
            "job": JOB_KINDS[match["kind"]],
            "environment": match["environment"],
        }
    return None


def is_runner_lost(annotations: list[dict[str, Any]]) -> bool:
    return any(
        RUNNER_LOST_MARKER in str(annotation.get("message", ""))
        for annotation in annotations
        if annotation.get("annotation_level") == "failure"
    )


def interrupted_step(job: dict[str, Any]) -> str | None:
    """The step that was running when the runner went silent, if any."""
    for step in job.get("steps", []):
        if step.get("status") == "in_progress" and step.get("conclusion") is None:
            return str(step.get("name"))
    return None


def classify(
    jobs: list[dict[str, Any]], annotations: dict[int, list[dict[str, Any]]]
) -> dict[str, Any]:
    """Split a run's jobs into runner-lost producers and other failures.

    `annotations` is keyed by job id and need only cover failed jobs.
    """
    lost: list[dict[str, Any]] = []
    other_failures: list[str] = []
    for job in jobs:
        if job.get("conclusion") != "failure":
            continue
        name = str(job.get("name"))
        if name in NON_PRODUCER_JOBS:
            continue
        if is_runner_lost(annotations.get(int(job["id"]), [])):
            cell = parse_job_name(name)
            lost.append(
                {
                    "id": int(job["id"]),
                    "name": name,
                    "step": interrupted_step(job),
                    "cell": cell,
                }
            )
        else:
            other_failures.append(name)
    return {"runner_lost": lost, "other_failures": other_failures}


def should_rerun(classification: dict[str, Any], attempt: int) -> tuple[bool, str]:
    lost = classification["runner_lost"]
    other = classification["other_failures"]
    if not lost:
        return False, "no job lost its runner"
    if attempt != 1:
        return False, f"attempt {attempt}: a runner was lost again; leaving it for a human"
    if other:
        return False, (
            f"{len(other)} job(s) failed for reasons other than a lost runner "
            f"({', '.join(other[:3])}{'…' if len(other) > 3 else ''}); "
            "a retry would only re-run the infrastructure failures"
        )
    return True, f"all {len(lost)} failed job(s) lost their runner; rerunning them once"


def status_directory(cell: dict[str, str | None]) -> str:
    name = f"status-{cell['package']}-{cell['job']}"
    if cell["environment"]:
        name += f"-{cell['environment']}"
    return name


def synthesize_status(records: list[dict[str, Any]], artifacts: Path) -> list[str]:
    """Write the producer status each runner-lost job could not; return paths."""
    written: list[str] = []
    for record in records:
        cell = record["cell"]
        if cell is None:
            continue
        directory = artifacts / status_directory(cell)
        target = directory / "status.json"
        if target.exists():
            # A live producer's own account always wins over this proxy.
            continue
        step = record["step"] or "before any step reported"
        status: dict[str, Any] = {
            "package": cell["package"],
            "job": cell["job"],
            "result": "failure",
            "detail": (
                f"the hosted runner lost communication with the server during "
                f"`{step}`; GitHub terminated the job and no log was uploaded "
                f"(job {record['id']})"
            ),
        }
        if cell["environment"]:
            status["environment"] = cell["environment"]
        directory.mkdir(parents=True, exist_ok=True)
        target.write_text(json.dumps(status, indent=2) + "\n", encoding="utf-8")
        written.append(str(target))
    return written


# ---------------------------------------------------------------------------
# GitHub access (kept behind one seam so everything above stays offline)
# ---------------------------------------------------------------------------


def gh_api(path: str, *args: str, attempts: int = 3) -> str:
    """`gh api` stdout, retried on the transient 5xx the API served all day."""
    last_error = ""
    for attempt in range(1, attempts + 1):
        result = subprocess.run(
            ["gh", "api", path, *args],
            capture_output=True,
            text=True,
            check=False,
        )
        if result.returncode == 0:
            return result.stdout
        last_error = result.stderr.strip()
        if attempt < attempts:
            time.sleep(5 * attempt)
    raise RuntimeError(f"gh api {path} failed after {attempts} attempts: {last_error}")


def fetch_jobs(repo: str, run_id: int, attempt: int) -> list[dict[str, Any]]:
    # `--paginate` follows the Link header, so a 400-job run needs no page
    # arithmetic here; `--jq` flattens each page's `jobs` into one line each.
    lines = gh_api(
        f"repos/{repo}/actions/runs/{run_id}/attempts/{attempt}/jobs?per_page=100",
        "--paginate",
        "--jq",
        ".jobs[]",
    )
    return [json.loads(line) for line in lines.splitlines() if line.strip()]


def fetch_annotations(repo: str, jobs: list[dict[str, Any]]) -> dict[int, list[dict[str, Any]]]:
    return {
        int(job["id"]): json.loads(gh_api(f"repos/{repo}/check-runs/{job['id']}/annotations"))
        for job in jobs
        if job.get("conclusion") == "failure"
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["attribute", "decide"])
    parser.add_argument("--repo", required=True, help="owner/name")
    parser.add_argument("--run-id", type=int, required=True)
    parser.add_argument("--attempt", type=int, required=True)
    parser.add_argument(
        "--artifacts",
        help="attribute: the ci-rollup artifacts root to synthesize status.json into",
    )
    parser.add_argument("--out", help="write the classification JSON here as well")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    jobs = fetch_jobs(args.repo, args.run_id, args.attempt)
    classification = classify(jobs, fetch_annotations(args.repo, jobs))
    rerun, reason = should_rerun(classification, args.attempt)
    report = {**classification, "attempt": args.attempt, "rerun": rerun, "reason": reason}

    if args.command == "attribute":
        if not args.artifacts:
            print("attribute requires --artifacts", file=sys.stderr)
            return 2
        report["synthesized"] = synthesize_status(
            classification["runner_lost"], Path(args.artifacts)
        )

    if args.out:
        Path(args.out).write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    sys.exit(main())

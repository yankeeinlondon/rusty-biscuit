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

- The rollup job runs `attribute` before rolling up. For every runner-lost job
  it synthesizes the `status-<package>-<job>[-<environment>]/status.json` the
  dead producer would have written, with `detail` naming the step that was
  running, so the grid reads `MISSING — … the hosted runner lost communication
  …`. A status a live producer did upload is never overwritten. `--package`
  narrows the synthesis to one area's packages, so an area-owned rollup
  attributes its own producers and nobody else's.
- `ci-infra-retry.yml` runs `decide` once the run has completed (the rerun
  endpoint refuses an in-progress run). It reruns the failed jobs only when
  this is attempt 1 AND every failed job is runner-lost: a real test failure
  is never retried, and a second loss is left for a human.

Every failure also carries the STEP that failed and the stage it belongs to.
A job whose tests passed and whose artifact upload then failed is a
`artifact-upload` failure, not a test failure: it establishes no test
regression and rerunning the suite would prove nothing (spec section 4, and the
repository's own classification rule).

A failed test, lint, or compile COMMAND is not a failed job at all: the
producers run their gate commands under `continue-on-error` and fold the
outcome into the status artifact, so such a job concludes `success` with a
red step and is classified nowhere here — it is neither a lost runner nor a
failure that vetoes the retry. That is deliberate. The failure is judged by
the area's rollup against its baseline; if it is accepted, a rerun of the
lost job is exactly what turns the run green, and if it is not, the rerun
re-executes only the lost job, never the failed suite.

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

# The producer status names come from `_package-ci.yml` and `_wsl-ci.yml`: L1
# for `test`, L2 for `test-l2`, and the job id for `check`, `lint`, and
# `test-browser`. `lint` carries no environment (its artifact is
# `status-<package>-lint`), and the WSL2 `archive` job records no status of its
# own — its loss surfaces through the dependent `test` leg's upstream edge.
JOB_KINDS = {
    "test": "L1",
    "test-l2": "L2",
    "test-browser": "browser",
    "check": "check",
    "lint": "lint",
}

# GitHub labels a called workflow's job `<caller job label> / <called job
# label>`, so the area restructure put every producer three or four segments
# deep:
#
#   area-ci (claudine) / claudine-cli / test (ubuntu-latest)
#   area-ci (claudine) / claudine-cli / wsl2 / test (wsl2-ubuntu)
#
# Parsed from the TAIL, never from a fixed segment count. The last segment
# names the gate and the environment; the segment that owns it is the package,
# which `_area-ci.yml` labels `name: ${{ matrix.package }}`. A caller chain
# that gains or loses a level therefore changes nothing here — which matters,
# because the chain is already at GitHub's four-level ceiling and the previous
# whole-name regexes stopped matching the moment the area level was inserted.
GATE_SEGMENT = re.compile(
    r"^(?P<kind>test|test-l2|test-browser|check|lint) \((?P<detail>[^()]+)\)$"
)
NAME_TOKEN = re.compile(r"[A-Za-z0-9_.-]+")

# `_package-ci.yml`'s delegating job, which owns no cell of its own: the WSL2
# cell belongs to the package one segment further up.
WSL_DELEGATION_SEGMENT = "wsl2"

# Jobs that judge the run rather than produce evidence. Their failure follows
# from the producers' and must not veto a retry — an area rollup fails BECAUSE
# its producer's runner died, so counting it as an unrelated failure would
# disable the one-shot retry entirely. `ci-gate` is the policy-free fold of
# every blocking job's result (`ci.yml`), so it fails whenever any producer
# did.
NON_PRODUCER_JOBS = {"ci-gate", "infrastructure summary (advisory)"}
AREA_ROLLUP_JOB = re.compile(r"^area-ci \(.+\) / rollup$")


def is_non_producer(name: str) -> bool:
    """Whether a job judges the run instead of producing a result cell."""
    return name in NON_PRODUCER_JOBS or AREA_ROLLUP_JOB.match(name) is not None


def parse_job_name(name: str) -> dict[str, str | None] | None:
    """Map a GitHub job name to the producer cell it would have reported."""
    segments = [segment.strip() for segment in name.split(" / ")]
    if len(segments) < 2:
        return None
    gate = GATE_SEGMENT.match(segments.pop())
    if gate is None:
        return None
    while segments and segments[-1] == WSL_DELEGATION_SEGMENT:
        segments.pop()
    if not segments or not NAME_TOKEN.fullmatch(segments[-1]):
        return None
    kind = gate["kind"]
    # `lint` runs on one environment and its artifact records none, so the
    # label's environment is presentation only and must not reach the key.
    environment = None if kind == "lint" else gate["detail"]
    if environment is not None and not NAME_TOKEN.fullmatch(environment):
        return None
    return {
        "package": segments[-1],
        "job": JOB_KINDS[kind],
        "environment": environment,
    }


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


def failed_step(job: dict[str, Any]) -> str | None:
    """The first step that concluded `failure`, if any."""
    for step in job.get("steps", []):
        if step.get("conclusion") == "failure":
            return str(step.get("name"))
    return None


#: Step-name markers, most specific first. A step matches the first stage whose
#: marker it contains. Ordering matters: "Upload the JUnit report" names both an
#: upload and a report, and it is an upload failure.
STAGE_MARKERS: tuple[tuple[str, tuple[str, ...]], ...] = (
    ("artifact-upload", ("upload", "artifact")),
    ("archive", ("archive",)),
    ("setup", ("set up", "setup", "checkout", "install", "provision", "toolchain")),
    ("test", ("test", "l1", "l2", "l3", "browser", "nextest")),
    ("lint", ("lint", "clippy", "fmt")),
    ("check", ("check", "compile", "build")),
)


def failure_stage(step: str | None) -> str:
    """Which stage a failed step belongs to.

    ## Notes

    Classifying by stage is what keeps "the tests failed" honest. Passing tests
    followed by a failed artifact upload are an infrastructure failure of the
    publication step; calling that a test regression sends someone to read a
    green report looking for a red test.
    """
    if not step:
        return "unknown"
    lowered = step.lower()
    for stage, markers in STAGE_MARKERS:
        if any(marker in lowered for marker in markers):
            return stage
    return "other"


def classify(
    jobs: list[dict[str, Any]], annotations: dict[int, list[dict[str, Any]]]
) -> dict[str, Any]:
    """Split a run's jobs into runner-lost producers and other failures.

    `annotations` is keyed by job id and need only cover failed jobs.
    """
    lost: list[dict[str, Any]] = []
    other_failures: list[str] = []
    failures: list[dict[str, Any]] = []
    for job in jobs:
        if job.get("conclusion") != "failure":
            continue
        name = str(job.get("name"))
        if is_non_producer(name):
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
            step = failed_step(job)
            other_failures.append(name)
            failures.append(
                {
                    "id": int(job["id"]),
                    "name": name,
                    "step": step,
                    "stage": failure_stage(step),
                    "cell": parse_job_name(name),
                }
            )
    return {
        "runner_lost": lost,
        "other_failures": other_failures,
        "failures": failures,
    }


def should_rerun(classification: dict[str, Any], attempt: int) -> tuple[bool, str]:
    lost = classification["runner_lost"]
    other = classification["other_failures"]
    if not lost:
        return False, "no job lost its runner"
    if attempt != 1:
        return False, f"attempt {attempt}: a runner was lost again; leaving it for a human"
    if other:
        # Named by the STAGE that failed, not by the job alone: a reader
        # deciding what to do next needs to know whether a test failed or a
        # publication step did.
        stages = {
            f"{failure['name']} ({failure['stage']})"
            for failure in classification.get("failures", [])
        } or set(other)
        listed = sorted(stages)
        return False, (
            f"{len(other)} job(s) failed for reasons other than a lost runner "
            f"({', '.join(listed[:3])}{'…' if len(listed) > 3 else ''}); "
            "a retry would only re-run the infrastructure failures"
        )
    return True, f"all {len(lost)} failed job(s) lost their runner; rerunning them once"


def status_directory(cell: dict[str, str | None]) -> str:
    name = f"status-{cell['package']}-{cell['job']}"
    if cell["environment"]:
        name += f"-{cell['environment']}"
    return name


def synthesize_status(
    records: list[dict[str, Any]],
    artifacts: Path,
    packages: set[str] | None = None,
) -> list[str]:
    """Write the producer status each runner-lost job could not; return paths.

    `packages`, when given, is the owning area's package set: an area-owned
    rollup attributes its own producers and leaves another area's to that
    area's rollup. Package, not area, because package is the stored identity of
    every cell (Design Decision 1) and a job name carries no area.
    """
    written: list[str] = []
    for record in records:
        cell = record["cell"]
        if cell is None:
            continue
        if packages is not None and cell["package"] not in packages:
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
    parser.add_argument(
        "--package",
        action="append",
        default=[],
        metavar="NAME",
        help=(
            "attribute: synthesize only these packages' statuses; repeatable and "
            "comma-separated. An area-owned rollup passes its own packages"
        ),
    )
    parser.add_argument("--out", help="write the classification JSON here as well")
    return parser.parse_args()


def selected_packages(raw: list[str]) -> set[str] | None:
    """The `--package` narrowing, or `None` for every package."""
    names = {name.strip() for entry in raw for name in entry.split(",") if name.strip()}
    return names or None


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
            classification["runner_lost"],
            Path(args.artifacts),
            selected_packages(args.package),
        )

    if args.out:
        Path(args.out).write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    sys.exit(main())

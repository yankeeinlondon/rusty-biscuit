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

- The coverage-audit job runs `attribute` before rolling up. For every runner-lost job
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

A failed test, lint, or compile command fails its producer visibly and is an
ordinary non-runner-loss failure here. If another job also loses its runner,
that real gate failure vetoes the automatic retry. The producer still uploads
its status and report when possible so the area's coverage audit can attribute the
cell precisely.

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

# The job ids a producer can carry, mapped to the gate they prove when the
# label does not say. Under the row-driven layout the gate IS in the label —
# a row is `{package, gate, environment, runner}` and GitHub renders every one
# of its values — so this table only decides the THREE-token form the
# workflows carried before, and the pre-row `test-l2`/`test-browser` job ids
# that historical runs still hold. `lint` carries no environment (its artifact
# is `status-<package>-lint`).
JOB_KINDS = {
    "test": "L1",
    "test-l2": "L2",
    "test-browser": "browser",
    "check": "check",
    "lint": "lint",
}

#: The gates a row's second token may name, as the plan spells them. `test` is
#: the job id, never a gate: a status under that name would manufacture a
#: phantom cell beside the real L1 one.
ROW_GATES = frozenset({"L1", "L2", "browser", "check", "lint"})

# GitHub labels a called workflow's job `<caller job label> / <called job
# label>`, and it builds a matrix job's own label from EVERY value in its row.
# Both label shapes therefore reach this parser (ruling R1):
#
#   area-ci (claudine) / package-ci / test (claudine-cli, L1, macos-latest, macos-latest)
#   area-ci (claudine) / package-ci / wsl2 (playa-cli, L1, wsl2-ubuntu, windows-latest) / test (wsl2-ubuntu)
#   area-ci (claudine) / claudine-cli / test (ubuntu-latest)                      <- pre-row
#   area-ci (claudine) / claudine-cli / wsl2 / test (wsl2-ubuntu)                 <- pre-row
#
# Both are parsed, and deliberately: reverting R1 must be a label-format change
# rather than a silent attribution loss, and a retry decision is routinely
# taken over a run created before the current layout.
#
# Parsed from the TAIL, never from a fixed segment count. A four-token
# parenthetical carries the whole cell; a three-token one carries only the
# environment, and the package is then the segment that owns it — which the
# pre-row `_area-ci.yml` labelled `name: ${{ matrix.package }}`. A caller chain
# that gains or loses a level changes nothing here, which matters because the
# chain is at GitHub's four-level ceiling and the previous whole-name regexes
# stopped matching the moment the area level was inserted.
GATE_SEGMENT = re.compile(
    r"^(?P<kind>test|test-l2|test-browser|check|lint) \((?P<detail>[^()]+)\)$"
)
NAME_TOKEN = re.compile(r"[A-Za-z0-9_.-]+")

# `ci.yml`'s build owner. A top-level matrix job, so its label is one segment
# and `parse_job_name` already answers `None` for it — a build owns no result
# cell. It still owes its consumers an account of itself: a lost runner leaves
# no build status either, and without one every dependent cell reads as an
# unexplained MISSING rather than as blocked by a named build.
BUILD_JOB_SEGMENT = re.compile(r"^build \((?P<artifact>[A-Za-z0-9_.-]+)\)$")


def parse_build_job_name(name: str) -> str | None:
    """The producer environment (or legacy artifact name) of a build job."""
    matched = BUILD_JOB_SEGMENT.match(name.strip())
    return matched["artifact"] if matched else None

# `_package-ci.yml`'s delegating job, which owns no cell of its own. Under the
# row layout it expands a matrix, so its label carries the row the delegated
# workflow's static job name cannot — which is exactly where the WSL2 cell's
# identity has to be read from.
WSL_DELEGATION_SEGMENT = "wsl2"
WSL_DELEGATION_ROW = re.compile(r"^wsl2 \((?P<detail>[^()]+)\)$")

# Jobs that only fold or summarize the run rather than produce evidence. Their
# failure follows from producers and must not veto a retry. The legacy `rollup`
# spelling remains recognized because runs created before the coverage-audit
# change could fail it for a lost producer. A new `coverage-audit` failure is
# not ignored: its enforcement runs only after every producer succeeded, so it
# represents an independent completeness or exception-policy failure.
# `infrastructure summary (advisory)` is the pre-`ci-reporting` spelling, kept
# for the same reason as the legacy rollup one: a retry decision may be taken
# over a run created before the rename.
NON_PRODUCER_JOBS = {
    "ci-gate",
    "ci-reporting (advisory)",
    "infrastructure summary (advisory)",
}
LEGACY_AREA_ROLLUP_JOB = re.compile(r"^area-ci \(.+\) / rollup$")


def is_non_producer(name: str) -> bool:
    """Whether a job judges the run instead of producing a result cell."""
    return name in NON_PRODUCER_JOBS or LEGACY_AREA_ROLLUP_JOB.match(name) is not None


def row_cell(detail: str) -> dict[str, str | None] | None:
    """The cell a four-token row parenthetical names, if it is one.

    `package, gate, environment, runner` — ruling R1's order, which is also
    the order GitHub renders the row's values in. The runner is dropped: it is
    dispatch, not identity, and `wsl2-ubuntu` on `windows-latest` is precisely
    the pair that must not collapse. `lint` keeps its environment out of the
    key, because its status artifact has never carried one.
    """
    tokens = [token.strip() for token in detail.split(",")]
    if len(tokens) != 4:
        return None
    package, gate, environment, runner = tokens
    if gate not in ROW_GATES:
        return None
    if not all(NAME_TOKEN.fullmatch(token) for token in (package, environment, runner)):
        return None
    return {
        "package": package,
        "job": gate,
        "environment": None if gate == "lint" else environment,
    }


def parse_job_name(name: str) -> dict[str, str | None] | None:
    """Map a GitHub job name to the producer cell it would have reported."""
    segments = [segment.strip() for segment in name.split(" / ")]
    if len(segments) < 2:
        return None
    gate = GATE_SEGMENT.match(segments.pop())
    if gate is None:
        return None
    # A row-driven label carries the whole cell in its own parenthetical, so
    # no enclosing segment is consulted at all.
    cell = row_cell(gate["detail"])
    if cell is not None:
        return cell
    # The delegated WSL2 workflow labels its job statically, so the row rides
    # one segment out — on the delegating matrix job that called it.
    while segments:
        delegation = WSL_DELEGATION_ROW.match(segments[-1])
        if delegation is not None:
            cell = row_cell(delegation["detail"])
            if cell is not None:
                return cell
            segments.pop()
            continue
        if segments[-1] == WSL_DELEGATION_SEGMENT:
            segments.pop()
            continue
        break
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
    area's coverage audit. Package, not area, because package is the stored identity of
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


def synthesize_build_status(
    records: list[dict[str, Any]],
    artifacts: Path,
    builds: list[dict[str, Any]],
    packages: set[str] | None = None,
) -> list[str]:
    """Write the build status each runner-lost owner leg could not; return paths.

    A build record is plumbing, never a result cell: this creates no cell, and
    the document it writes is only ever read to explain a cell that could not
    run. `builds` is the resolved plan's `builds` list, which is the only place
    the artifact name, package, producer, and key agree.
    """
    written: list[str] = []
    for record in records:
        artifact = parse_build_job_name(record["name"])
        if artifact is None:
            continue
        for build in builds:
            if artifact not in (build["artifact"], build["producer"]):
                continue
            if packages is not None and build["package"] not in packages:
                continue
            directory = artifacts / (
                f"build-status-{build['package']}-{build['producer']}-{build['key']}"
            )
            target = directory / "build-status.json"
            if target.exists():
                # The owner's own account always wins over this proxy.
                continue
            step = record["step"] or "before any step reported"
            directory.mkdir(parents=True, exist_ok=True)
            target.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "key": build["key"],
                        "package": build["package"],
                        "producer": build["producer"],
                        "artifact": build["artifact"],
                        "result": "failure",
                        "stage": "produce",
                        "consumers": build.get("consumers", []),
                        "detail": (
                            f"the hosted runner lost communication with the server during "
                            f"`{step}`; GitHub terminated the build owner and no archive "
                            f"was uploaded (job {record['id']})"
                        ),
                    },
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )
            written.append(str(target))
    return written


def plan_builds(path: str | None) -> list[dict[str, Any]]:
    """The resolved plan's build records, or an empty list without a plan."""
    if not path:
        return []
    document = json.loads(Path(path).read_text(encoding="utf-8"))
    builds = document.get("builds")
    return builds if isinstance(builds, list) else []


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
    parser.add_argument(
        "--plan",
        help=(
            "attribute: the resolved plan, so a runner-lost BUILD owner can be "
            "attributed to the build record its consumers name"
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
        selected = selected_packages(args.package)
        report["synthesized"] = synthesize_status(
            classification["runner_lost"], Path(args.artifacts), selected
        )
        report["synthesized_builds"] = synthesize_build_status(
            classification["runner_lost"],
            Path(args.artifacts),
            plan_builds(args.plan),
            selected,
        )

    if args.out:
        Path(args.out).write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    sys.exit(main())

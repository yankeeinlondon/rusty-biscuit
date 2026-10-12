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
import io
import json
import os
import re
import subprocess
import sys
import time
import zipfile
from pathlib import Path
from typing import Any, Callable

sys.path.insert(0, str(Path(__file__).resolve().parent))

import schema  # noqa: E402  (needs the path insert above when run as a script)

ROOT = Path(__file__).resolve().parents[2]
ENVIRONMENTS_PATH = ".github/ci/environments.json"
WORKFLOW_PATH = ".github/workflows/ci.yml"


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


#: Nightly runs one anchor search looks at: `NIGHTLY_PAGE_SIZE` per page, at
#: most `NIGHTLY_MAX_PAGES` pages. A search that ends with runs unexamined is
#: reported as `search-exhausted`, never as "no baseline".
NIGHTLY_PAGE_SIZE = 50
NIGHTLY_MAX_PAGES = 4

#: Jobs listed per page when a legacy run's audits are checked.
JOBS_PAGE_SIZE = 100
JOBS_MAX_PAGES = 6

AUDIT_JOB = re.compile(r"(^|/ )coverage-audit$")


def git_environments_at(sha: str) -> list[dict[str, Any]] | None:
    """The environment table as it was committed at `sha`, or None if unreadable."""
    result = subprocess.run(
        ["git", "show", f"{sha}:{ENVIRONMENTS_PATH}"],
        capture_output=True, text=True, timeout=30,
    )
    if result.returncode != 0:
        return None
    document = schema.loads_strict(result.stdout)
    environments = document.get("environments") if isinstance(document, dict) else None
    if not isinstance(environments, list) or not all(
        isinstance(entry, dict) and isinstance(entry.get("name"), str) for entry in environments
    ):
        return None
    return environments


def download_certification(repo: str, artifact_id: int) -> Any:
    """The single JSON document inside a certification artifact."""
    result = subprocess.run(
        ["gh", "api", f"repos/{repo}/actions/artifacts/{int(artifact_id)}/zip"],
        check=True, capture_output=True, timeout=60,
    )
    with zipfile.ZipFile(io.BytesIO(result.stdout)) as archive:
        names = [name for name in archive.namelist() if not name.endswith("/")]
        if names != ["certification.json"]:
            raise ValueError(f"certification artifact holds {names}, expected certification.json")
        return schema.loads_strict(archive.read("certification.json").decode("utf-8"))


def schedule_events(environment: dict[str, Any]) -> list[str]:
    return list(environment.get("events") or schema.EVENTS)


def scheduled_environments(table: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """The environments a `schedule` event plans cells for, in table order."""
    return [entry for entry in table if "schedule" in schedule_events(entry)]


def eligible_runs(runs: list[dict[str, Any]], repo: str) -> list[dict[str, Any]]:
    """Successful scheduled runs of this workflow on `main`, newest first.

    A foreign repository, another workflow path, another branch, or a run that
    did not complete successfully is never an anchor, and neither advances nor
    rolls back the successful history.
    """
    kept = [
        run for run in runs
        if run.get("event") == "schedule"
        and run.get("path") == WORKFLOW_PATH
        and run.get("head_branch") == "main"
        and run.get("status") == "completed"
        and run.get("conclusion") == "success"
        and isinstance(run.get("repository"), dict)
        and run["repository"].get("full_name") == repo
        and type(run.get("id")) is int
        and re.fullmatch(r"[0-9a-f]{40}", str(run.get("head_sha", "")))
    ]
    return sorted(kept, key=lambda run: run["id"], reverse=True)


def list_nightlies(
    repo: str, api: Callable[[str], Any]
) -> tuple[list[dict[str, Any]], bool]:
    """`(runs, exhausted)` from the bounded, paginated list of successful nightlies.

    `exhausted` is true when the page budget ended with more runs available.
    """
    runs: list[dict[str, Any]] = []
    total = 0
    for page in range(1, NIGHTLY_MAX_PAGES + 1):
        document = api(
            f"repos/{repo}/actions/workflows/ci.yml/runs"
            f"?event=schedule&branch=main&status=success&per_page={NIGHTLY_PAGE_SIZE}&page={page}"
        )
        batch = document["workflow_runs"]
        total = int(document.get("total_count", len(batch)))
        if not isinstance(batch, list):
            raise ValueError("workflow_runs is not a list")
        runs.extend(batch)
        if len(batch) < NIGHTLY_PAGE_SIZE or len(runs) >= total:
            return runs, False
    return runs, len(runs) < total


def run_certification(
    repo: str,
    run: dict[str, Any],
    environment: dict[str, Any],
    api: Callable[[str], Any],
    download: Callable[[str, int], Any],
) -> tuple[str, dict[str, Any] | None]:
    """`(state, document)` for one run's certification of one environment.

    `state` is `absent` (no artifact of that name), `invalid` (an artifact
    exists but is unreadable, expired, or does not describe this run,
    environment, and policy), or `valid`.
    """
    sha = run["head_sha"]
    digest = schema.policy_digest(environment)
    name = schema.certification_name(environment["name"], sha, digest)
    artifacts = api(
        f"repos/{repo}/actions/runs/{int(run['id'])}/artifacts?name={name}&per_page=100"
    )["artifacts"]
    present = [a for a in artifacts if a.get("name") == name]
    if not present:
        return "absent", None
    live = [a for a in present if a.get("expired") is False and type(a.get("id")) is int]
    if len(live) != 1:
        return "invalid", None
    try:
        document = download(repo, live[0]["id"])
    except Exception:
        return "invalid", None
    if schema.validate_certification(document):
        return "invalid", None
    if not (
        document["run_id"] == run["id"]
        and document["head"] == sha
        and document["environment"] == environment["name"]
        and document["event"] == "schedule"
        and document["workflow_path"] == WORKFLOW_PATH
        and document["environments_digest"] == digest
        and document["coverage_policy_version"] >= schema.COVERAGE_POLICY_VERSION
        and document["certified"] is True
        and document["audits"]["required"] == document["audits"]["passed"]
    ):
        return "invalid", None
    return "valid", document


def legacy_audits_passed(repo: str, run: dict[str, Any], api: Callable[[str], Any]) -> bool:
    """Whether the run's normal audits all ran and passed.

    A pre-certification run left no per-environment proof, so the run's own
    jobs are the evidence: at least one `coverage-audit` job, every one of
    them `success`. A skipped audit proves nothing about the cells it covers.
    """
    audits = []
    for page in range(1, JOBS_MAX_PAGES + 1):
        document = api(
            f"repos/{repo}/actions/runs/{int(run['id'])}/jobs"
            f"?filter=latest&per_page={JOBS_PAGE_SIZE}&page={page}"
        )
        jobs = document["jobs"]
        if not isinstance(jobs, list):
            raise ValueError("jobs is not a list")
        audits += [job for job in jobs if AUDIT_JOB.search(str(job.get("name", "")))]
        if len(jobs) < JOBS_PAGE_SIZE:
            return bool(audits) and all(job.get("conclusion") == "success" for job in audits)
    # More jobs than the budget: the audit set cannot be proven complete.
    return False


def resolve_anchors(
    repo: str,
    table: list[dict[str, Any]],
    api: Callable[[str], Any] = github_api,
    is_ancestor: Callable[[str], bool] = git_is_ancestor,
    environments_at: Callable[[str], list[dict[str, Any]] | None] = git_environments_at,
    download: Callable[[str, int], Any] = download_certification,
) -> list[dict[str, Any]]:
    """Each scheduled environment's comparison anchor, with the reason for it.

    One anchor per environment, never one for the run: the newest successful
    nightly that this head descends from AND that proved this environment. A
    run proves an environment when it published a valid certification for it
    (a policy-current, audited, fully accounted run), or, for a run from
    before certifications existed, when its own workflow scheduled the
    environment at that commit and every normal audit ran and passed. A run
    never credits an environment its schedule did not include, so a WSL2-only
    history anchors WSL2 and leaves Windows in full scope.

    ## Returns

    A document for the planner (`schema.ANCHOR_INPUT_*`): environments with a
    full-scope fallback carry the coded reason, so no bootstrap is inferred
    from silence. Lookup failures degrade to `api-failure` full scope for the
    affected environments; nothing here ever narrows scope on a guess.
    """
    scheduled = scheduled_environments(table)
    anchors: list[dict[str, Any]] = []

    def fallback(environment: dict[str, Any], reason: str) -> dict[str, Any]:
        return {
            "environment": environment["name"], "mode": "full", "anchor": None,
            "reason": reason,
            "source": {"kind": "none", "run_id": None, "head": None},
            "covered_cells": None,
        }

    try:
        runs, exhausted = list_nightlies(repo, api)
    except Exception:
        return _document([fallback(environment, "api-failure") for environment in scheduled])
    candidates = eligible_runs(runs, repo)
    if not candidates and not exhausted:
        return _document([fallback(environment, "bootstrap-no-run") for environment in scheduled])

    ancestors: dict[int, bool] = {}
    for environment in scheduled:
        found: dict[str, Any] | None = None
        uncertified = False
        try:
            for run in candidates:
                if run["id"] not in ancestors:
                    ancestors[run["id"]] = is_ancestor(run["head_sha"])
                if not ancestors[run["id"]]:
                    continue
                state, certification = run_certification(repo, run, environment, api, download)
                if state == "valid":
                    found = {
                        "environment": environment["name"], "mode": "incremental",
                        "anchor": run["head_sha"], "reason": "certified-ancestor",
                        "source": {"kind": "nightly-certified", "run_id": run["id"], "head": run["head_sha"]},
                        "covered_cells": certification["covered_cells"],
                    }
                    break
                if state == "invalid":
                    uncertified = True
                    continue
                # No certification for THIS environment: a legacy run is
                # credited only if it carries none for ANY scheduled one.
                if any(
                    run_certification(repo, run, other, api, download)[0] != "absent"
                    for other in scheduled if other["name"] != environment["name"]
                ):
                    uncertified = True
                    continue
                then = environments_at(run["head_sha"])
                record = next((e for e in then or [] if e["name"] == environment["name"]), None)
                if record is None or "schedule" not in schedule_events(record):
                    uncertified = True
                    continue
                if not legacy_audits_passed(repo, run, api):
                    uncertified = True
                    continue
                found = {
                    "environment": environment["name"], "mode": "incremental",
                    "anchor": run["head_sha"], "reason": "legacy-wsl2-ancestor",
                    "source": {"kind": "nightly-legacy", "run_id": run["id"], "head": run["head_sha"]},
                    "covered_cells": None,
                }
                break
        except Exception:
            anchors.append(fallback(environment, "api-failure"))
            continue
        if found is not None:
            anchors.append(found)
        elif exhausted:
            anchors.append(fallback(environment, "search-exhausted"))
        elif uncertified:
            anchors.append(fallback(environment, "policy-uncertified"))
        else:
            anchors.append(fallback(environment, "no-qualifying-ancestor"))
    return _document(anchors)


def _document(anchors: list[dict[str, Any]]) -> dict[str, Any]:
    return {"schema_version": schema.ANCHOR_INPUT_SCHEMA_VERSION, "anchors": anchors}


def nightly_anchors_summary(document: dict[str, Any]) -> str:
    """One line per environment for the run's step summary."""
    lines = []
    for entry in document["anchors"]:
        where = (
            f"since {entry['anchor'][:9]} (run {entry['source']['run_id']})"
            if entry["mode"] == "incremental" else "the full gated workspace"
        )
        lines.append(f"- `{entry['environment']}`: {where} — {entry['reason']}")
    return "\n".join(lines)


def certification_document(
    plan: dict[str, Any],
    environment: str,
    *,
    run_id: int,
    attempt: int,
    head: str,
    tree: str,
    table: list[dict[str, Any]],
    audits: tuple[int, int],
    gate_passed: bool,
) -> dict[str, Any]:
    """The certification one environment of a finished nightly earns.

    `covered_cells` is what the environment is proven for after this run: the
    cells it executed or reused (the gate and every audit passed, or the
    document is not `certified`) plus the cells its anchor already covered and
    this plan did not reselect. Each carries the coverage contract the plan's
    anchor records for it in `required_cells`, which is what a later nightly
    compares with its own policy. Accepted gaps are counted but cover nothing.
    A cell the plan reopened because later evidence contradicted its anchor
    (`reopened_cells`) is covered only as this run executed it, so a failure
    there leaves the document uncertified rather than inherited again.

    ## Errors

    Raises ``ValueError`` when the plan carries no anchor for `environment`,
    since a certification states which history it was built on, and when a
    covered cell has no recorded contract or is narrowed by a `test_filter`:
    such a cell proves less than the whole required cell, so nothing may
    certify it as covered, and when the anchor both inherits and reopens one
    cell.
    """
    anchors = plan.get("coverage_anchors") or []
    mine = next((a for a in anchors if a["environment"] == environment), None)
    if mine is None:
        raise ValueError(f"the plan carries no coverage anchor for {environment}")
    record = next((e for e in table if e["name"] == environment), None)
    if record is None:
        raise ValueError(f"environment {environment!r} is not in the table")
    contracts = {(c["package"], c["gate"]): c["contract"] for c in mine["required_cells"]}

    def contracted(package: str, gate: str, narrowed: bool = False) -> dict[str, str]:
        contract = contracts.get((package, gate))
        if contract is None or narrowed:
            raise ValueError(
                f"{package}/{gate} on {environment} has no whole-cell coverage contract "
                "in the plan; it cannot be certified as covered"
            )
        return {"package": package, "gate": gate, "contract": contract}

    executing, reused, gaps = [], [], 0
    for cell in plan["cells"]:
        if cell["environment"] != environment:
            continue
        if cell["state"] == "accepted-gap":
            gaps += 1
        elif cell["execution"] == "execute":
            executing.append(contracted(cell["package"], cell["gate"], bool(cell.get("test_filter"))))
        elif cell["state"] == "reused":
            reused.append(contracted(cell["package"], cell["gate"], bool(cell.get("test_filter"))))
    reopened = {(c["package"], c["gate"]) for c in mine["reopened_cells"]}
    if any((c["package"], c["gate"]) in reopened for c in mine["inherited_cells"]):
        raise ValueError(
            f"the plan both inherits and reopens a cell on {environment}; a reopened "
            "cell is covered only by this run's own result"
        )
    inherited = [contracted(c["package"], c["gate"]) for c in mine["inherited_cells"]]
    covered = {(c["package"], c["gate"]): c for c in [*executing, *reused, *inherited]}
    required, passed = audits
    return {
        "schema_version": schema.CERTIFICATION_SCHEMA_VERSION,
        "run_id": run_id,
        "attempt": attempt,
        "workflow_path": WORKFLOW_PATH,
        "event": "schedule",
        "head": head,
        "tree": tree,
        "environment": environment,
        "coverage_policy_version": schema.COVERAGE_POLICY_VERSION,
        "environments_digest": schema.policy_digest(record),
        "plan_identity": schema.identity(schema.canonical(plan)),
        "anchors_used": [
            {key: value for key, value in a.items() if key != "required_cells"}
            for a in sorted(anchors, key=lambda entry: entry["environment"])
        ],
        "certified": bool(gate_passed and required == passed),
        "cells": {
            "executing": len(executing), "reused": len(reused),
            "accepted_gap": gaps, "inherited": len(mine["inherited_cells"]),
        },
        "covered_cells": [covered[key] for key in sorted(covered)],
        "audits": {"required": required, "passed": passed},
    }


def count_audits(jobs: list[dict[str, Any]], required: int) -> tuple[int, int]:
    """`(required, passed)` for a finished run's coverage-audit jobs.

    `required` is the number of scheduled areas, each of which owns one audit;
    only audit jobs that concluded `success` count, so a skipped or cancelled
    audit leaves the run uncertified.
    """
    passed = sum(
        1 for job in jobs
        if AUDIT_JOB.search(str(job.get("name", ""))) and job.get("conclusion") == "success"
    )
    return required, passed


def run_jobs(repo: str, run_id: int, api: Callable[[str], Any] = github_api) -> list[dict[str, Any]]:
    jobs: list[dict[str, Any]] = []
    for page in range(1, JOBS_MAX_PAGES + 1):
        batch = api(
            f"repos/{repo}/actions/runs/{int(run_id)}/jobs"
            f"?filter=latest&per_page={JOBS_PAGE_SIZE}&page={page}"
        )["jobs"]
        jobs += batch
        if len(batch) < JOBS_PAGE_SIZE:
            return jobs
    raise ValueError("the run has more jobs than the audit count can prove complete")


def certify(args: argparse.Namespace) -> None:
    """Write one certification per anchored environment under `--out`.

    Each lands in `<out>/<artifact name>/certification.json`; the artifact
    name is printed as a JSON list with its environment so the workflow can
    upload each under that exact name.
    """
    try:
        plan = schema.loads_strict(Path(args.plan).read_text(encoding="utf-8"))
    except ValueError as error:
        raise SystemExit(f"certify: the resolved plan is unreadable: {error}") from None
    problems = schema.validate_resolved_plan(plan)
    if problems:
        raise SystemExit("certify: the resolved plan is invalid: " + "; ".join(problems))
    repo = os.environ["GITHUB_REPOSITORY"]
    run_id = int(os.environ["GITHUB_RUN_ID"])
    table = json.loads((ROOT / ENVIRONMENTS_PATH).read_text(encoding="utf-8"))["environments"]
    scheduled_areas = {entry["area"] for entry in plan["packages"] if entry["gates"]}
    audits = count_audits(run_jobs(repo, run_id), len(scheduled_areas))
    head = plan["head"]
    tree = git_revision(f"{head}^{{tree}}")
    written = []
    for anchor in plan.get("coverage_anchors", []):
        document = certification_document(
            plan, anchor["environment"], run_id=run_id,
            attempt=int(os.environ["GITHUB_RUN_ATTEMPT"]), head=head, tree=tree,
            table=table, audits=audits, gate_passed=args.gate_result == "success",
        )
        invalid = schema.validate_certification(document)
        if invalid:
            raise SystemExit("certify: built an invalid certification: " + "; ".join(invalid))
        name = schema.certification_name(
            anchor["environment"], head, document["environments_digest"]
        )
        directory = Path(args.out) / name
        directory.mkdir(parents=True, exist_ok=True)
        (directory / "certification.json").write_text(schema.canonical(document), encoding="utf-8")
        written.append({
            "environment": anchor["environment"], "name": name,
            "certified": document["certified"],
        })
    print(json.dumps(written, separators=(",", ":")))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("record", "check", "nightly-anchors", "certify"))
    parser.add_argument("--plan", help="certify: the run's resolved plan")
    parser.add_argument("--out", help="certify: directory the certification artifacts are written under")
    parser.add_argument("--gate-result", default="failure", help="certify: the final gate's result")
    parser.add_argument(
        "--wait-minutes", type=float, default=0.0,
        help="check: how long to wait for an unfinished PR run of the merged head",
    )
    args = parser.parse_args()
    if args.mode == "nightly-anchors":
        document = resolve_anchors(
            os.environ["GITHUB_REPOSITORY"],
            json.loads((ROOT / ENVIRONMENTS_PATH).read_text(encoding="utf-8"))["environments"],
        )
        print(json.dumps(document, separators=(",", ":")))
        summary = nightly_anchors_summary(document)
        print(summary, file=sys.stderr)
        if os.environ.get("GITHUB_STEP_SUMMARY"):
            with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as handle:
                handle.write(f"## Nightly coverage anchors\n\n{summary}\n")
        return
    if args.mode == "certify":
        certify(args)
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

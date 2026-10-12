#!/usr/bin/env python3
"""Record hosted producer cells as durable observations.

A scheduled hosted producer proves its own cell (`completion.py`). That proof
lives in workflow artifacts, which nothing outside the run can find, so a
nightly that failed *elsewhere* left the cells it had proved unreachable. This
module turns a finished cell into an observation (`schema.py`) that the same
verifier every other producer feeds can read:

* `emit` runs in the producer job, after its completion step, and writes ONE
  observation file for the cell. A pass needs the gate to have succeeded **and**
  a valid completion record bound to this run; an honest failure needs a real
  report that names a failing test. A cell with no usable report, an unfinished
  or cancelled gate, or a gate that failed outside its tests writes nothing and
  exits nonzero with the reason: interrupted work never earns proof, and nothing
  is fabricated.
* `publish` runs in a trusted scheduled job that holds `contents: write`. It
  validates every observation of one run, drops invalid ones loudly, groups the
  rest by `(environment, head)`, and hands each group to
  `observation_notes.publish` (local write, fetch, union, non-force push).
* `publish --recent` is recovery: it does the same for the newest
  `RECOVERY_RUNS` completed scheduled runs of `ci.yml` on `main`, so a cell
  whose run never reached (or never finished) its own `publish` is still
  discovered. The nightly's `scope` job runs it with `--no-push` before
  verification, which needs only `actions: read`; `publish-observations` runs
  it with a push so the recovery outlives the run. Republishing an observation
  a note already holds changes nothing.
* `gh_probe` is the discovery-time half: it fetches the cited artifact and
  compares bytes, so a pass earns credit only while its report is retrievable
  and unaltered.

## Conventions this module and the workflows share

* Artifact names are derived, never stored: the junit artifact is
  `junit-<package>-<gate>-<environment>` (`completion.JUNIT_DIRECTORY`, the
  `junit_artifact` output of `cell_contract.py`), the completion artifact is
  `completion-<package>-<gate>-<environment>`, and the observation artifact is
  `observation-<package>-<gate>-<environment>`. The run is the observation's
  `producer.run_id`.
* An observation's `producer.host.label` is the cell's **environment** name. The
  observation document carries no environment (the note it sits under does), so
  this is how a probe that is handed only `(report, observation)` recomputes the
  artifact name.
* A report reference is `{path, sha256, bytes, store: "artifact"}` where `path`
  is the member of the junit artifact (`L1/<package>.xml`). The proof's
  `expected_listing_digest` names the artifact's `expected-<gate>.json` and its
  `completion_record_digest` names `completion.json` in the completion
  artifact.
* An emitted file is an envelope `{schema_version, environment, head,
  observation}`: the observation itself cannot say where it belongs.

Python 3.9 stdlib plus this directory only: hosted runners, including Windows
(`PYTHONUTF8=1`), run it.

## Exit status

`emit`: `0` written, `1` refused (reason on stderr, nothing written).
`publish`: `0` everything valid is on the remote, or recorded locally under
`--no-push` (or there was nothing to publish), `3` an observation was rejected
or could not be recorded locally, or (`--recent`) one run's artifacts could not
be collected, `4` recorded locally but the remote refused it, `5` the run's
(or, `--recent`, the run list's) artifacts could not be listed. `4` outranks
`3`.
"""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
import platform
import re
import subprocess
import sys
import tempfile
import zipfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Optional

sys.path.insert(0, str(Path(__file__).resolve().parent))

import completion  # noqa: E402
import local_evidence  # noqa: E402
import observation_notes  # noqa: E402
import schema  # noqa: E402

ENVELOPE_VERSION = 1
OBSERVATION_FILE = "observation.json"
COMPLETION_FILE = "completion.json"

#: Largest member or envelope read from a downloaded artifact. Nothing this
#: module reads is near it; the cap exists so a hostile zip cannot exhaust memory.
MEMBER_LIMIT = 64 * 1024 * 1024
ENVELOPE_LIMIT = 1024 * 1024

#: Artifact listing bounds: `per_page` and the page cap together bound the API
#: calls one run costs.
PAGE_SIZE = 100
MAX_PAGES = 10

#: How many of the newest completed scheduled runs `publish --recent` collects.
#: Each nightly re-reads this window, so an orphaned observation gets this many
#: nightlies to be recovered before only a manual `--run-id` reaches it.
RECOVERY_RUNS = 5
WORKFLOW_FILE = "ci.yml"
WORKFLOW_PATH = f".github/workflows/{WORKFLOW_FILE}"

EXIT_REFUSED = 1
EXIT_REJECTED = 3
EXIT_LOCAL_ONLY = 4
EXIT_NO_ARTIFACTS = 5

LOCAL_ONLY_MESSAGE = "hosted observations are local to this runner; remote reuse unavailable"

#: Fetches one GitHub API path and returns the response body. Injected so tests
#: and the probe never need the network.
Fetch = Callable[[str], bytes]


class HostedError(Exception):
    """An observation could not be built, or an artifact could not be read."""


# ---------------------------------------------------------------------------
# Names
# ---------------------------------------------------------------------------


def junit_artifact(package: str, gate: str, environment: str) -> str:
    return completion.JUNIT_DIRECTORY.format(
        package=package, gate=gate, environment=environment
    )


def completion_artifact(package: str, gate: str, environment: str) -> str:
    return f"completion-{package}-{gate}-{environment}"


def observation_artifact(package: str, gate: str, environment: str) -> str:
    return f"observation-{package}-{gate}-{environment}"


def expected_member(gate: str) -> str:
    return f"expected-{gate}.json"


# ---------------------------------------------------------------------------
# emit
# ---------------------------------------------------------------------------


@dataclass
class RunContext:
    """What the workflow knows about the run that produced the cell."""

    run_id: str
    attempt: int
    workflow: str
    host_os: str = ""
    host_kernel: str = ""


def workflow_path(reference: str) -> str:
    """`.github/workflows/ci.yml` from `owner/repo/.github/workflows/ci.yml@ref`."""
    path = reference.split("@", 1)[0]
    marker = path.find(".github/")
    return path[marker:] if marker >= 0 else path


def _digest(path: Path) -> tuple[str, int]:
    content = path.read_bytes()
    return hashlib.sha256(content).hexdigest(), len(content)


def _combine(cells: list[dict[str, Any]]) -> dict[str, Any]:
    """One cell from several invocations of the same gate and package."""
    if len(cells) == 1:
        return cells[0]
    combined = dict(cells[0])
    combined["counts"] = {
        name: sum(cell["counts"][name] for cell in cells) for name in schema.COUNT_FIELDS
    }
    combined["duration_s"] = sum(cell["duration_s"] for cell in cells)
    combined["outcome"] = "fail" if any(cell["outcome"] == "fail" for cell in cells) else "pass"
    combined["exit_code"] = next((cell["exit_code"] for cell in cells if cell["exit_code"]), 0)
    failed = sorted({name for cell in cells for name in cell.get("failed_tests", [])})
    combined.pop("failed_tests", None)
    combined.pop("failure_detail_truncated", None)
    if len(failed) > schema.FAILURE_DETAIL_LIMIT:
        combined["failed_tests"] = failed[: schema.FAILURE_DETAIL_LIMIT]
        combined["failure_detail_truncated"] = True
    elif failed:
        combined["failed_tests"] = failed
    return combined


def _completion_record(
    path: Optional[Path], package: str, environment: str, gate: str, head: str, run: RunContext
) -> tuple[dict[str, Any], str]:
    """The record, validated and bound to this cell and run, with its digest."""
    if path is None or not path.is_file():
        raise HostedError(
            "the gate succeeded but its completion record is absent, so the cell is "
            "unproven and earns no observation"
        )
    content = path.read_bytes()
    try:
        record = schema.loads_strict(content.decode("utf-8"))
    except (UnicodeDecodeError, ValueError) as error:
        raise HostedError(f"the completion record is not valid JSON: {error}") from error
    problems = schema.validate_completion_record(record)
    if problems:
        raise HostedError(f"the completion record is invalid: {problems[0]}")
    expected = {
        "package": package,
        "environment": environment,
        "gate": gate,
        "head": head,
        "run": run.run_id,
        "attempt": run.attempt,
    }
    for name, value in expected.items():
        if record[name] != value:
            raise HostedError(
                f"the completion record's {name} is {record[name]!r}, this cell's is "
                f"{value!r}; a record from another cell or run proves nothing here"
            )
    if not isinstance(record.get("nextest_version"), str) or not record["nextest_version"]:
        raise HostedError("the completion record names no nextest version")
    return record, hashlib.sha256(content).hexdigest()


def _expected_manifest(stage: Path, gate: str) -> tuple[Optional[dict[str, Any]], Optional[str]]:
    path = stage / expected_member(gate)
    if not path.is_file():
        return None, None
    content = path.read_bytes()
    try:
        document = schema.loads_strict(content.decode("utf-8"))
    except (UnicodeDecodeError, ValueError):
        return None, None
    return (document if isinstance(document, dict) else None), hashlib.sha256(content).hexdigest()


def _proven_backends(stage: Path) -> set[str]:
    path = stage / local_evidence.BACKEND_PROOFS_FILE
    if not path.is_file():
        return set()
    try:
        document = schema.loads_strict(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return set()
    if not isinstance(document, dict):
        return set()
    return {
        name
        for name, entry in document.items()
        if isinstance(entry, dict) and entry.get("proven") is True
    }


def build_observation(
    plan: dict[str, Any],
    cell_id: str,
    stage: Path,
    completion_path: Optional[Path],
    gate_outcome: str,
    run: RunContext,
    *,
    checkout: Optional[str] = None,
) -> dict[str, Any]:
    """The envelope for one finished hosted cell.

    ## Errors

    Raises [`HostedError`] for every case that must produce no observation: an
    unknown or unplanned cell, a checkout that is not the tested revision, a
    gate that did not finish (`cancelled`, `skipped`, empty), a missing or
    unreadable report, a pass without a valid completion record, a failure the
    report cannot attribute, and an observation the schema would reject.
    """
    parts = cell_id.rsplit("/", 2)
    if len(parts) != 3:
        raise HostedError(f"{cell_id!r} is not package/environment/gate")
    package, environment, gate = parts
    planned = next(
        (
            cell
            for cell in plan["cells"]
            if (cell["package"], cell["environment"], cell["gate"]) == (package, environment, gate)
        ),
        None,
    )
    if planned is None:
        raise HostedError(f"{cell_id} is not a cell of the resolved plan")
    if gate not in local_evidence.RECORDABLE_GATES:
        raise HostedError(f"{gate} produces no report, so {cell_id} has nothing to record")
    if gate_outcome not in ("success", "failure"):
        raise HostedError(
            f"the gate step of {cell_id} ended {gate_outcome or 'unknown'}; interrupted "
            "work earns no observation"
        )
    head = plan["head"]
    try:
        actual = local_evidence.revision(checkout or "HEAD")
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        raise HostedError(f"the checkout does not resolve: {error}") from error
    if actual != head:
        raise HostedError(
            f"the checkout is {actual[:12]}, the plan tested {head[:12]}; an observation "
            "binds to the revision the plan names"
        )

    records = [
        record
        for record in local_evidence.staged_records(stage)
        if record.get("package") == package
        and record.get("tier") == gate
        and record.get("environment") in ("", None, environment)
    ]
    if not records:
        raise HostedError(f"{stage} stages no {gate} report for {package}")
    proven = _proven_backends(stage)
    pass_record: Optional[dict[str, Any]] = None
    completion_digest: Optional[str] = None
    if gate_outcome == "success":
        pass_record, completion_digest = _completion_record(
            completion_path, package, environment, gate, head, run
        )
        proven |= set(pass_record.get("backends", []))
    required = {(package, gate): list(planned.get("backends", []))}
    cells = []
    for staged in records:
        if staged.get("report_present") is False:
            raise HostedError(f"{package} {gate} ran but staged no report; nothing to cite")
        cell = local_evidence.receipt_cell(staged, stage, plan, head, proven, required)
        if cell["completion"] != "complete":
            raise HostedError(
                f"the {gate} report of {package} ({staged.get('xml')}) is missing, "
                "unreadable, or attributes no failing test"
                + (" or no backend drove a test" if gate in local_evidence.BACKEND_PROVEN_GATES else "")
                + "; the cell is partial and earns no observation"
            )
        cells.append(cell)
    cell = _combine(cells)
    if gate_outcome == "success" and (
        cell["outcome"] != "pass" or cell["counts"]["failed"] or cell["counts"]["errored"]
    ):
        raise HostedError("the gate succeeded but its report records a failure; contradictory")
    if gate_outcome == "failure" and cell["outcome"] != "fail":
        raise HostedError(
            "the gate failed but its report records no failing test, so the failure is "
            "outside the tests and not an observation"
        )
    if planned.get("test_filter"):
        cell["test_filter"] = planned["test_filter"]

    reports = []
    for staged in records:
        digest, size = _digest(stage / staged["xml"])
        reports.append({"path": staged["xml"], "sha256": digest, "bytes": size, "store": "artifact"})

    manifest, listing_digest = _expected_manifest(stage, gate)
    proof: dict[str, str] = {}
    if gate_outcome == "success":
        if manifest is None or listing_digest is None:
            raise HostedError(f"the expected listing {expected_member(gate)} is absent or unreadable")
        if not isinstance(manifest.get("target"), str) or not manifest["target"]:
            raise HostedError("the expected listing names no target")
        assert pass_record is not None and completion_digest is not None
        proof = {
            "nextest_version": pass_record["nextest_version"],
            "target": manifest["target"],
            "expected_listing_digest": listing_digest,
            "completion_record_digest": completion_digest,
        }
    elif (
        manifest is not None
        and listing_digest is not None
        and isinstance(manifest.get("target"), str)
        and manifest["target"]
        and isinstance(manifest.get("nextest_version"), str)
        and manifest["nextest_version"]
    ):
        # A failure needs no proof; what the listing does show is kept.
        proof = {
            "nextest_version": manifest["nextest_version"],
            "target": manifest["target"],
            "expected_listing_digest": listing_digest,
        }

    host = {
        "os": run.host_os or platform.system(),
        "kernel": run.host_kernel or f"{platform.system()} {platform.release()}",
        "label": environment,
    }
    body: dict[str, Any] = {
        "producer": {
            "kind": "hosted",
            "host": host,
            "workflow": run.workflow,
            "run_id": run.run_id,
            "attempt": run.attempt,
        },
        "base": plan["base"],
        "scope_identity": schema.identity(schema.canonical(plan)),
        "completion": "complete",
        "cell": cell,
        "reports": reports,
    }
    if proof:
        body["proof"] = proof
    features = local_evidence._planned_features(plan, planned)
    if features:
        body["features"] = features
    observation = observation_notes.with_identity(body)
    problems = schema.validate_observation(observation, environment)
    if problems:
        raise HostedError(f"the observation would be invalid: {problems[0]}")
    return {
        "schema_version": ENVELOPE_VERSION,
        "environment": environment,
        "head": head,
        "observation": observation,
    }


def write_atomic(path: Path, document: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(schema.canonical(document) + "\n", encoding="utf-8")
    os.replace(temporary, path)


# ---------------------------------------------------------------------------
# publish
# ---------------------------------------------------------------------------


@dataclass
class Loaded:
    path: Path
    environment: str
    head: str
    observation: dict[str, Any]


@dataclass
class PublishSummary:
    """What a publication did, for the exit status and the step log."""

    published: list[str] = field(default_factory=list)
    #: Recorded in the local notes because no remote was asked for.
    recorded: list[str] = field(default_factory=list)
    local_only: list[str] = field(default_factory=list)
    rejected: list[str] = field(default_factory=list)
    unrecorded: list[str] = field(default_factory=list)
    error: Optional[str] = None

    @property
    def exit_status(self) -> int:
        if self.local_only:
            return EXIT_LOCAL_ONLY
        if self.rejected or self.unrecorded:
            return EXIT_REJECTED
        return 0


_HEAD = re.compile(r"[0-9a-f]{40}")


def _commit_exists(head: str, repo: Optional[Path]) -> bool:
    result = subprocess.run(
        ["git", "cat-file", "-e", f"{head}^{{commit}}"],
        cwd=repo, capture_output=True, timeout=30,
    )
    return result.returncode == 0


def load_envelopes(
    root: Path, *, repo: Optional[Path] = None, run_id: Optional[str] = None
) -> tuple[list[Loaded], list[str]]:
    """Every valid envelope under `root`, and a reason for each file that is not.

    A file is rejected, never repaired: a malformed observation is never
    published. Duplicate observations (the same id) are kept once.
    """
    loaded: dict[str, Loaded] = {}
    rejected: list[str] = []
    for path in sorted(root.rglob("*.json")):
        label = str(path.relative_to(root))
        if path.is_symlink() or not path.is_file():
            rejected.append(f"{label}: not a regular file")
            continue
        if path.stat().st_size > ENVELOPE_LIMIT:
            rejected.append(f"{label}: larger than {ENVELOPE_LIMIT} bytes")
            continue
        try:
            document = schema.loads_strict(path.read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError, ValueError) as error:
            rejected.append(f"{label}: unreadable: {error}")
            continue
        reason = _envelope_problem(document, repo, run_id)
        if reason:
            rejected.append(f"{label}: {reason}")
            continue
        loaded.setdefault(
            document["observation"]["observation_id"],
            Loaded(path, document["environment"], document["head"], document["observation"]),
        )
    return list(loaded.values()), rejected


def _envelope_problem(document: Any, repo: Optional[Path], run_id: Optional[str]) -> Optional[str]:
    if not isinstance(document, dict) or set(document) != {
        "schema_version", "environment", "head", "observation",
    }:
        return "not an observation envelope"
    if type(document["schema_version"]) is not int or document["schema_version"] != ENVELOPE_VERSION:
        return f"envelope version {document['schema_version']!r} is not {ENVELOPE_VERSION}"
    environment = document["environment"]
    if environment not in schema.ENVIRONMENTS:
        return f"unknown environment {environment!r}"
    head = document["head"]
    if not isinstance(head, str) or not _HEAD.fullmatch(head):
        return "head is not a full Git object ID"
    observation = document["observation"]
    problems = schema.validate_observation(observation, environment)
    if problems:
        return problems[0]
    producer = observation["producer"]
    if producer["kind"] != "hosted":
        return f"producer kind {producer['kind']!r} is not hosted"
    if producer["host"].get("label") != environment:
        return "host label is not the cell's environment, so its artifact cannot be derived"
    if run_id is not None and producer["run_id"] != run_id:
        return f"belongs to run {producer['run_id']}, not {run_id}"
    if any(report["store"] != "artifact" for report in observation["reports"]):
        return "cites a report that is not an artifact"
    if not _commit_exists(head, repo):
        return f"head {head[:12]} is not in this checkout"
    return None


def publish_directory(
    root: Path,
    *,
    repo: Optional[Path] = None,
    remote: Optional[str] = "origin",
    run_id: Optional[str] = None,
    publisher: Callable[..., Any] = observation_notes.publish,
    **publish_options: Any,
) -> PublishSummary:
    """Publish every valid observation under `root`.

    Groups are published independently, so one group's remote failure does not
    withhold another's. Re-running over the same directory publishes nothing
    new: the collection union drops a republished observation.
    """
    summary = PublishSummary()
    loaded, rejected = load_envelopes(root, repo=repo, run_id=run_id)
    summary.rejected = rejected
    for reason in rejected:
        print(f"hosted_observations: rejected {reason}", file=sys.stderr)
    groups: dict[tuple[str, str], list[Loaded]] = {}
    for item in loaded:
        groups.setdefault((item.environment, item.head), []).append(item)
    for (environment, head), items in sorted(groups.items()):
        observations = [item.observation for item in items]
        try:
            result = publisher(
                environment, head, observations, remote=remote, repo=repo, **publish_options
            )
        except (observation_notes.PublicationError, subprocess.SubprocessError, OSError) as error:
            summary.unrecorded.append(f"{environment}@{head[:12]}: {error}")
            print(
                f"hosted_observations: nothing recorded for {environment}@{head[:12]}: {error}",
                file=sys.stderr,
            )
            continue
        ids = [observation["observation_id"] for observation in observations]
        if result.published:
            summary.published.extend(ids)
            print(f"hosted_observations: published {len(ids)} observation(s) for {environment}@{head[:12]}")
        elif remote is None:
            summary.recorded.extend(ids)
            print(f"hosted_observations: recorded {len(ids)} observation(s) locally for {environment}@{head[:12]}")
        else:
            summary.local_only.extend(ids)
            summary.error = result.error
            print(
                f"hosted_observations: {LOCAL_ONLY_MESSAGE} "
                f"({environment}@{head[:12]}): {result.error}",
                file=sys.stderr,
            )
    if not loaded and not rejected:
        print("hosted_observations: no observations to publish")
    return summary


# ---------------------------------------------------------------------------
# GitHub artifacts
# ---------------------------------------------------------------------------


def gh_fetch(path: str) -> bytes:
    """`gh api <path>`, as bytes. Needs `GH_TOKEN` on a runner."""
    try:
        result = subprocess.run(
            ["gh", "api", path], capture_output=True, timeout=180, check=False
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise HostedError(f"gh api {path} could not run: {error}") from error
    if result.returncode != 0:
        detail = result.stderr.decode("utf-8", "replace").strip().splitlines()
        raise HostedError(f"gh api {path} failed: {detail[-1] if detail else result.returncode}")
    return result.stdout


def list_artifacts(repository: str, run_id: str, fetch: Fetch) -> list[dict[str, Any]]:
    """Every artifact of a run, page by page, up to `MAX_PAGES`."""
    found: list[dict[str, Any]] = []
    for page in range(1, MAX_PAGES + 1):
        body = fetch(
            f"repos/{repository}/actions/runs/{run_id}/artifacts?per_page={PAGE_SIZE}&page={page}"
        )
        try:
            document = json.loads(body)
        except ValueError as error:
            raise HostedError(f"the artifact listing is not JSON: {error}") from error
        batch = document.get("artifacts") if isinstance(document, dict) else None
        if not isinstance(batch, list):
            raise HostedError("the artifact listing has no artifacts list")
        found.extend(entry for entry in batch if isinstance(entry, dict))
        if len(batch) < PAGE_SIZE:
            return found
    raise HostedError(f"run {run_id} has more than {MAX_PAGES * PAGE_SIZE} artifacts")


def _members(archive: bytes) -> dict[str, tuple[str, int]]:
    """`{member: (sha256, bytes)}` for every file in a zip, size-capped."""
    members: dict[str, tuple[str, int]] = {}
    with zipfile.ZipFile(io.BytesIO(archive)) as zipped:
        for info in zipped.infolist():
            if info.is_dir():
                continue
            if info.file_size > MEMBER_LIMIT:
                continue
            content = zipped.read(info)
            members[info.filename.replace("\\", "/")] = (
                hashlib.sha256(content).hexdigest(), len(content),
            )
    return members


def collect_run(
    repository: str, run_id: str, destination: Path, fetch: Fetch = gh_fetch
) -> list[str]:
    """Download this run's `observation-*` artifacts under `destination`.

    Each artifact gets its own subdirectory, and only top-level files of a
    bounded size are written: a downloaded zip is untrusted data. Expired
    artifacts are skipped with a message.
    """
    names = []
    for artifact in list_artifacts(repository, run_id, fetch):
        name = artifact.get("name")
        if not isinstance(name, str) or not name.startswith("observation-"):
            continue
        if artifact.get("expired") is True:
            print(f"hosted_observations: {name} has expired and cannot be published", file=sys.stderr)
            continue
        if type(artifact.get("id")) is not int:
            continue
        archive = fetch(f"repos/{repository}/actions/artifacts/{artifact['id']}/zip")
        target = destination / re.sub(r"[^A-Za-z0-9._-]", "_", name)
        target.mkdir(parents=True, exist_ok=True)
        with zipfile.ZipFile(io.BytesIO(archive)) as zipped:
            for info in zipped.infolist():
                member = info.filename
                if (
                    info.is_dir()
                    or "/" in member
                    or "\\" in member
                    or member.startswith(".")
                    or info.file_size > ENVELOPE_LIMIT
                ):
                    continue
                (target / member).write_bytes(zipped.read(info))
        names.append(name)
    return names


def recent_scheduled_runs(
    repository: str, fetch: Fetch = gh_fetch, limit: int = RECOVERY_RUNS
) -> list[str]:
    """The ids of the newest `limit` completed scheduled `ci.yml` runs on `main`.

    Only a scheduled run on `main` executes default-branch code and records
    observations, so the query's filters are re-checked on every entry: another
    event, workflow, branch, or repository, or a run still in progress, is never
    collected whatever the API returned. Its conclusion is deliberately not
    checked; a failed or cancelled nightly is exactly the run whose finished
    cells need recovering.
    """
    body = fetch(
        f"repos/{repository}/actions/workflows/{WORKFLOW_FILE}/runs"
        f"?event=schedule&branch=main&status=completed&per_page={limit}"
    )
    try:
        document = json.loads(body)
    except ValueError as error:
        raise HostedError(f"the run listing is not JSON: {error}") from error
    runs = document.get("workflow_runs") if isinstance(document, dict) else None
    if not isinstance(runs, list):
        raise HostedError("the run listing has no workflow_runs list")
    kept = [
        run for run in runs
        if isinstance(run, dict)
        and run.get("event") == "schedule"
        and run.get("path") == WORKFLOW_PATH
        and run.get("head_branch") == "main"
        and run.get("status") == "completed"
        and isinstance(run.get("repository"), dict)
        and run["repository"].get("full_name") == repository
        and type(run.get("id")) is int
    ]
    kept.sort(key=lambda run: run["id"], reverse=True)
    return [str(run["id"]) for run in kept[:limit]]


def recover_recent(
    repository: str,
    *,
    remote: Optional[str] = "origin",
    repo: Optional[Path] = None,
    fetch: Fetch = gh_fetch,
    limit: int = RECOVERY_RUNS,
) -> PublishSummary:
    """Publish the observations of every run `recent_scheduled_runs` names.

    Each run is collected and validated exactly as `publish --run-id` would,
    and one run that cannot be collected is reported without withholding the
    others.

    ## Errors

    Raises [`HostedError`] when the run list itself cannot be read.
    """
    summary = PublishSummary()
    runs = recent_scheduled_runs(repository, fetch, limit)
    print(f"hosted_observations: recovering {len(runs)} earlier scheduled run(s): {', '.join(runs) or 'none'}")
    for run_id in runs:
        with tempfile.TemporaryDirectory(prefix="hosted-observations-") as scratch:
            try:
                collect_run(repository, run_id, Path(scratch), fetch)
            except (HostedError, zipfile.BadZipFile, OSError) as error:
                summary.unrecorded.append(f"run {run_id}: {error}")
                print(f"hosted_observations: cannot collect run {run_id}: {error}", file=sys.stderr)
                continue
            part = publish_directory(Path(scratch), repo=repo, remote=remote, run_id=run_id)
        summary.published.extend(part.published)
        summary.recorded.extend(part.recorded)
        summary.local_only.extend(part.local_only)
        summary.rejected.extend(part.rejected)
        summary.unrecorded.extend(part.unrecorded)
        summary.error = part.error or summary.error
    return summary


def gh_probe(
    repository: str, *, fetch: Fetch = gh_fetch, log: Optional[Callable[[str], None]] = None
) -> Callable[[dict[str, Any], dict[str, Any]], bool]:
    """The `report_probe` for `local_evidence.verify_cells`.

    The returned callable answers whether an `artifact` report reference is
    still retrievable *and unaltered*: the producer run's artifact of the
    conventional name must exist and be unexpired, the cited member must hash to
    the reference, and the proof's documents must hash to the proof's digests.
    Listings and archives are cached per process. Any failure — API, network,
    malformed archive — answers `False` with a reason on stderr and never raises,
    so the cell simply executes.
    """
    emit_log = log or (lambda message: print(f"hosted_observations: {message}", file=sys.stderr))
    listings: dict[str, list[dict[str, Any]]] = {}
    archives: dict[int, dict[str, tuple[str, int]]] = {}

    def members_of(run_id: str, name: str) -> dict[str, tuple[str, int]]:
        if run_id not in listings:
            listings[run_id] = list_artifacts(repository, run_id, fetch)
        matches = [a for a in listings[run_id] if a.get("name") == name]
        if not matches:
            raise HostedError(f"run {run_id} has no artifact named {name}")
        live = [a for a in matches if a.get("expired") is not True and type(a.get("id")) is int]
        if not live:
            raise HostedError(f"artifact {name} of run {run_id} has expired")
        identifier = live[0]["id"]
        if identifier not in archives:
            archives[identifier] = _members(
                fetch(f"repos/{repository}/actions/artifacts/{identifier}/zip")
            )
        return archives[identifier]

    def check(report: dict[str, Any], observation: dict[str, Any]) -> bool:
        producer = observation["producer"]
        if producer.get("kind") != "hosted" or not isinstance(producer.get("run_id"), str):
            raise HostedError("the observation is not a hosted one with a run")
        environment = producer["host"].get("label")
        if environment not in schema.ENVIRONMENTS:
            raise HostedError(f"host label {environment!r} names no environment")
        cell = observation["cell"]
        package, gate = cell["package"], cell["gate"]
        run_id = producer["run_id"]
        junit = members_of(run_id, junit_artifact(package, gate, environment))
        cited = junit.get(report["path"])
        if cited is None:
            raise HostedError(f"{junit_artifact(package, gate, environment)} has no {report['path']}")
        if cited != (report["sha256"], report["bytes"]):
            raise HostedError(f"{report['path']} differs from the digest the observation recorded")
        proof = observation.get("proof")
        if proof is None:
            return True
        listing = junit.get(expected_member(gate))
        if listing is None or listing[0] != proof["expected_listing_digest"]:
            raise HostedError(f"{expected_member(gate)} is absent or differs from the proof")
        if "completion_record_digest" in proof:
            record = members_of(run_id, completion_artifact(package, gate, environment))
            held = record.get(COMPLETION_FILE)
            if held is None or held[0] != proof["completion_record_digest"]:
                raise HostedError(f"{COMPLETION_FILE} is absent or differs from the proof")
        return True

    def probe(report: dict[str, Any], observation: dict[str, Any]) -> bool:
        try:
            return check(report, observation)
        except Exception as error:  # noqa: BLE001 - discovery must fail safe, never crash
            emit_log(f"report not retrievable, the cell will execute: {error}")
            return False

    return probe


# ---------------------------------------------------------------------------
# command line
# ---------------------------------------------------------------------------


def _emit_command(args: argparse.Namespace, fetch: Fetch) -> int:
    try:
        plan = schema.loads_strict(Path(args.plan).read_text(encoding="utf-8"))
        problems = schema.validate_resolved_plan(plan)
        if problems:
            raise HostedError(f"the resolved plan is invalid: {problems[0]}")
        envelope = build_observation(
            plan,
            args.cell,
            Path(args.stage),
            Path(args.completion) if args.completion else None,
            args.gate_outcome,
            RunContext(
                run_id=args.run_id,
                attempt=args.attempt,
                workflow=workflow_path(args.workflow),
                host_os=args.host_os,
                host_kernel=args.host_kernel,
            ),
        )
    except (HostedError, OSError, ValueError) as error:
        out = Path(args.out)
        if out.exists():
            out.unlink()
        message = f"no observation for {args.cell}: {error}"
        print(f"hosted_observations: {message}", file=sys.stderr)
        if os.environ.get("GITHUB_ACTIONS"):
            print(f"::warning title=no observation recorded::{message}")
        return EXIT_REFUSED
    write_atomic(Path(args.out), envelope)
    print(
        f"hosted_observations: recorded {envelope['observation']['cell']['outcome']} "
        f"observation {envelope['observation']['observation_id'][:12]} for {args.cell}"
    )
    return 0


def _publish_command(args: argparse.Namespace, fetch: Fetch) -> int:
    remote = None if args.no_push else args.remote
    if args.directory:
        summary = publish_directory(
            Path(args.directory), remote=remote, run_id=args.expect_run_id
        )
        return summary.exit_status
    repository = args.repository or os.environ.get("GITHUB_REPOSITORY", "")
    if repository and args.recent and not args.run_id:
        try:
            return recover_recent(repository, remote=remote, fetch=fetch).exit_status
        except HostedError as error:
            print(f"hosted_observations: cannot list earlier scheduled runs: {error}",
                  file=sys.stderr)
            return EXIT_NO_ARTIFACTS
    if not repository or not args.run_id or args.recent:
        print("hosted_observations: publish needs --directory, or --repository and one of "
              "--run-id or --recent", file=sys.stderr)
        return EXIT_NO_ARTIFACTS
    with tempfile.TemporaryDirectory(prefix="hosted-observations-") as scratch:
        try:
            names = collect_run(repository, args.run_id, Path(scratch), fetch)
        except (HostedError, zipfile.BadZipFile, OSError) as error:
            print(f"hosted_observations: cannot collect run {args.run_id}: {error}", file=sys.stderr)
            return EXIT_NO_ARTIFACTS
        print(f"hosted_observations: collected {len(names)} observation artifact(s) of run {args.run_id}")
        summary = publish_directory(Path(scratch), remote=remote, run_id=args.run_id)
        return summary.exit_status


def main(argv: Optional[list[str]] = None, *, fetch: Fetch = gh_fetch) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)

    emit = commands.add_parser("emit", help="write the observation of one finished cell")
    emit.add_argument("--plan", required=True)
    emit.add_argument("--cell", required=True, help="package/environment/gate")
    emit.add_argument("--stage", required=True, help="the staged JUnit tree")
    emit.add_argument("--completion", default="", help="the cell's completion record, if any")
    emit.add_argument("--gate-outcome", required=True, help="the gate step's outcome")
    emit.add_argument("--run-id", required=True)
    emit.add_argument("--attempt", required=True, type=int)
    emit.add_argument("--workflow", required=True)
    emit.add_argument("--host-os", default="")
    emit.add_argument("--host-kernel", default="")
    emit.add_argument("--out", required=True)
    emit.set_defaults(handler=_emit_command)

    publish = commands.add_parser("publish", help="publish a run's observations to the notes refs")
    publish.add_argument("--directory", help="a directory of downloaded observation artifacts")
    publish.add_argument("--repository", help="owner/name; GITHUB_REPOSITORY by default")
    publish.add_argument("--run-id", help="collect this run's observation artifacts")
    publish.add_argument(
        "--recent", action="store_true",
        help=f"collect the newest {RECOVERY_RUNS} completed scheduled runs' observation artifacts",
    )
    publish.add_argument("--expect-run-id", help="with --directory, require this run id")
    publish.add_argument("--remote", default="origin")
    publish.add_argument("--no-push", action="store_true")
    publish.set_defaults(handler=_publish_command)

    args = parser.parse_args(argv)
    return args.handler(args, fetch)


if __name__ == "__main__":
    raise SystemExit(main())

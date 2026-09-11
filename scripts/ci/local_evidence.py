#!/usr/bin/env python3
"""Record and verify local CI evidence carried by Git notes.

Two generations live here on purpose.

`verify_cells` is the current contract: it reads **every**
`refs/notes/ci-local/<environment>` note reachable from the outgoing head,
returns the accepted `{package, environment, gate}` cells from all of them
together, and returns a coded reason for every cell it refused. Evidence from
macOS and from a prior WSL cross-check therefore combine in one run, which the
environment-at-a-time predecessor could never express.

`verified_environment` and `record` are that predecessor, kept because
`schema_version: 1` notes are already published against open branches. Spec
section 3.6 makes those notes exact-tree, pass-only, whole-environment, and
never upgraded in place, so they are read through the old comparison and expire
naturally as their heads do. Nothing writes a version-1 note any more:
`record_cells` writes version 2.
"""

from __future__ import annotations

import argparse
import json
import platform
import re
import subprocess
import sys
from pathlib import Path
from typing import Any
from xml.etree import ElementTree

sys.path.insert(0, str(Path(__file__).resolve().parent))

import affected_scope  # noqa: E402  (needs the path insert above)
import schema  # noqa: E402


#: The version this module still *reads* through the legacy comparison below.
#: `record_cells` writes `schema.RECEIPT_SCHEMA_VERSION`.
SCHEMA_VERSION = schema.LEGACY_RECEIPT_SCHEMA_VERSION

ENVIRONMENTS = schema.ENVIRONMENTS
NOTES_PREFIX = "refs/notes/ci-local"

#: Tiers the JUnit staging manifest records, and therefore the gates a local
#: run can publish a measured outcome for. `lint` and `check` produce no report
#: and are never reusable from a local receipt.
RECORDABLE_GATES = ("L1", "L2", "browser")


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], check=True, capture_output=True, text=True, timeout=20
    ).stdout.strip()


def git_optional(*args: str) -> str | None:
    """`git(*args)`, or `None` when Git refuses — never a raised exception.

    Every caller here treats "Git cannot answer" as "no evidence", which is the
    fail-safe direction: the cell stays scheduled.
    """
    try:
        return git(*args)
    except (OSError, subprocess.SubprocessError):
        return None


def revision(ref: str) -> str:
    value = git("rev-parse", "--verify", ref)
    if not re.fullmatch(r"[0-9a-f]{40}", value):
        raise ValueError(f"not a full Git object ID: {value}")
    return value


# ---------------------------------------------------------------------------
# Gate-input identity (spec section 3.3, Design Decision 4)
# ---------------------------------------------------------------------------


def gate_global_inputs(gate: str) -> list[str]:
    """The global inputs that belong to `gate`, as repository-relative paths.

    Derived from the planner's own per-gate classification rather than a second
    table: Design Decision 4 is "one classification, two consumers". The cell
    vocabulary (`L1`, `L2`, `browser`) collapses onto the planner's `test`
    gate, which is the gate those tiers are scheduled under.

    `Cargo.lock` is added here although the planner deliberately excludes it
    from *selection* (a lockfile edit must not schedule every package). For
    *equivalence* the opposite is true: a resolved dependency change makes an
    older result describe a different build, so the lockfile has to move the
    identity.
    """
    planner_gate = gate if gate in ("lint", "check") else "test"
    paths = set(affected_scope.GLOBAL_PATHS_ALL_GATES)
    paths |= set(affected_scope.GLOBAL_PATHS_BY_GATE[planner_gate])
    paths |= set(affected_scope.JUST_PATHS)
    paths |= {prefix.rstrip("/") for prefix in affected_scope.GLOBAL_PREFIXES_ALL_GATES}
    paths |= {prefix.rstrip("/") for prefix in affected_scope.JUST_PREFIXES}
    paths.add(affected_scope.LOCKFILE_PATH)
    return sorted(paths)


def gate_input_identity(paths: list[str] | tuple[str, ...], ref: str) -> str:
    """The identity of `paths` as they stand at `ref`.

    Git's own hashing boundary is the whole rule: the identity is over the
    `git ls-tree` entries — mode, type, object ID, path — for each input, so
    two trees agree exactly when Git says the content agrees. A path absent at
    `ref` contributes nothing, which is what makes an input that exists on
    neither side identical on both.

    ## Examples

    ```python
    before = gate_input_identity(["claudine/lib"], "HEAD~1")
    after = gate_input_identity(["claudine/lib"], "HEAD")
    ```
    """
    entries: list[str] = []
    for path in sorted({path for path in paths if path}):
        listing = git_optional("ls-tree", "-r", "--full-tree", ref, "--", path)
        if listing:
            entries.extend(line for line in listing.splitlines() if line.strip())
    return schema.identity("\n".join(sorted(set(entries))))


def cell_input_paths(plan: dict[str, Any], package: str, gate: str) -> list[str] | None:
    """Every path a cell's gate-input identity covers, or `None` if unknowable.

    `None` means the plan declared no build closure for the package, so no
    older receipt can be proven equivalent and only exact-tree reuse remains.
    """
    for entry in plan["packages"]:
        if entry["package"] != package:
            continue
        declared = entry.get("input_paths")
        if not declared:
            return None
        return [*declared, *gate_global_inputs(gate)]
    return None


# ---------------------------------------------------------------------------
# Version-2 verification
# ---------------------------------------------------------------------------


def load_plan(path: str) -> dict[str, Any]:
    document = json.loads(Path(path).read_text(encoding="utf-8"))
    if not isinstance(document, dict):
        raise ValueError("the resolved plan must be a JSON object")
    return document


def candidate_commits(base: str, head: str) -> list[str]:
    """Commits a receipt may be attached to, newest first.

    The outgoing head plus everything it added over the base. A receipt older
    than the merge base describes work that is already upstream and is never
    consulted.
    """
    listing = git_optional("rev-list", f"{base}..{head}")
    commits = listing.splitlines() if listing else []
    if head not in commits:
        commits.insert(0, head)
    return commits


def latest_note(environment: str, commits: list[str]) -> tuple[str, str] | None:
    """The newest note on `commits` for `environment`, with its commit."""
    for commit in commits:
        note = git_optional("notes", "--ref", f"{NOTES_PREFIX}/{environment}", "show", commit)
        if note:
            return commit, note
    return None


def verify_cells(
    plan_path: str, base: str, head: str
) -> tuple[list[dict[str, Any]], list[str]]:
    """Every plan cell satisfied by published evidence, and why the rest were not.

    ## Returns

    `(accepted, rejections)`. Each accepted entry is a receipt cell widened
    with `environment`, `origin` (`local` for the outgoing tree, `prior-local`
    for an older head accepted through gate-input equivalence), `evidence`, and
    a rendered `measurements` string. Each rejection begins with a code from
    [`schema.REJECTIONS`].

    Missing, malformed, incompatible, stale, conflicting, and incomplete input
    all fail safe: the cell is simply not accepted, so it stays scheduled.

    ## Notes

    Equivalence recomputes the identity over **both** trees rather than
    trusting the identity the receipt stored. A stored string is an unverified
    claim by the host that wrote it; both trees are present locally, so the
    comparison can be made rather than believed.
    """
    accepted: list[dict[str, Any]] = []
    rejections: list[str] = []
    try:
        plan = load_plan(plan_path)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        return [], [f"malformed-receipt: the resolved plan is unreadable: {error}"]

    problems = schema.validate_resolved_plan(plan)
    if problems:
        return [], [f"malformed-receipt: the resolved plan is invalid: {problems[0]}"]

    try:
        head_sha = revision(head)
        merge_base = git("merge-base", base, head_sha)
        head_tree = revision(f"{head_sha}^{{tree}}")
    except (OSError, ValueError, subprocess.SubprocessError):
        return [], ["missing-receipt: the base/head pair does not resolve in this repository"]

    wanted = {
        (cell["package"], cell["environment"], cell["gate"]) for cell in plan["cells"]
    }
    commits = candidate_commits(merge_base, head_sha)
    seen: set[tuple[str, str, str]] = set()

    for environment in ENVIRONMENTS:
        found = latest_note(environment, commits)
        if found is None:
            continue
        note_commit, note_text = found
        try:
            document = json.loads(note_text)
        except json.JSONDecodeError:
            rejections.append(
                f"malformed-receipt: the {environment} note on {note_commit[:9]} is not JSON"
            )
            continue
        if not isinstance(document, dict):
            rejections.append(
                f"malformed-receipt: the {environment} note on {note_commit[:9]} is not an object"
            )
            continue

        if document.get("schema_version") == schema.LEGACY_RECEIPT_SCHEMA_VERSION:
            legacy_accepted, legacy_rejections = _accept_legacy(
                document, plan, environment, head_tree, wanted, seen
            )
            accepted.extend(legacy_accepted)
            rejections.extend(legacy_rejections)
            continue

        cells, cell_rejections = schema.reusable_cells(document)
        rejections.extend(cell_rejections)
        if not cells:
            continue

        exact = document.get("tree") == head_tree
        for cell in cells:
            key = (cell["package"], environment, cell["gate"])
            if key not in wanted or key in seen:
                continue
            label = "/".join(key)
            if exact:
                origin = "local"
            else:
                paths = cell_input_paths(plan, cell["package"], cell["gate"])
                if paths is None:
                    rejections.append(
                        f"gate-inputs-changed: {label} was tested on {note_commit[:9]}, "
                        "and the plan declares no build closure for it, so equivalence "
                        "with this head cannot be established"
                    )
                    continue
                if gate_input_identity(paths, note_commit) != gate_input_identity(
                    paths, head_sha
                ):
                    rejections.append(
                        f"gate-inputs-changed: {label} was tested on {note_commit[:9]}, "
                        "whose gate inputs differ from this head's"
                    )
                    continue
                origin = "prior-local"
            seen.add(key)
            accepted.append(_accepted_cell(cell, environment, origin, note_commit, document))

    return accepted, rejections


def _accepted_cell(
    cell: dict[str, Any],
    environment: str,
    origin: str,
    note_commit: str,
    receipt: dict[str, Any],
) -> dict[str, Any]:
    counts = cell["counts"]
    return {
        **cell,
        "environment": environment,
        "origin": origin,
        "measurements": (
            f"{counts['total']} test(s), {counts['failed']} failed, "
            f"{cell['duration_s']}s"
        ),
        "evidence": {
            "ref": f"{NOTES_PREFIX}/{environment}",
            "commit": note_commit,
            "host": receipt["host"],
        },
    }


def _accept_legacy(
    document: dict[str, Any],
    plan: dict[str, Any],
    environment: str,
    head_tree: str,
    wanted: set[tuple[str, str, str]],
    seen: set[tuple[str, str, str]],
) -> tuple[list[dict[str, Any]], list[str]]:
    """Accept a version-1 note under the narrow rules of spec section 3.6.

    Exact tree identity only, pass-only, and whole-environment — the same
    breadth the environment-at-a-time verifier granted, so a legacy receipt
    neither gains reach nor is silently upgraded. Its measurements render as
    [`schema.UNRECORDED_MEASUREMENT`] because the document has none.
    """
    if document.get("tree") != head_tree:
        return [], [
            f"v1-not-equivalence-eligible: the {environment} version-1 receipt tested "
            f"tree {str(document.get('tree'))[:9]}, and a version-1 receipt is reusable "
            "only on exact tree identity"
        ]
    accepted = []
    for package, env, gate in sorted(wanted):
        if env != environment or (package, env, gate) in seen:
            continue
        seen.add((package, env, gate))
        accepted.append(
            {
                "package": package,
                "gate": gate,
                "environment": environment,
                "outcome": "pass",
                "completion": "complete",
                "origin": "local",
                "measurements": schema.UNRECORDED_MEASUREMENT,
                "evidence": {
                    "ref": f"{NOTES_PREFIX}/{environment}",
                    "schema_version": schema.LEGACY_RECEIPT_SCHEMA_VERSION,
                },
            }
        )
    return accepted, []


# ---------------------------------------------------------------------------
# Version-2 recording
# ---------------------------------------------------------------------------


def junit_counts(report: Path) -> tuple[dict[str, int], list[str]] | None:
    """Counts and failing test names from a staged JUnit report.

    ## Returns

    `None` when the report is absent or unparseable, which the caller turns
    into a non-reusable cell rather than into invented measurements.
    """
    try:
        root = ElementTree.parse(report).getroot()
    except (OSError, ElementTree.ParseError):
        return None
    suites = [root] if root.tag == "testsuite" else list(root.iter("testsuite"))
    totals = {"total": 0, "passed": 0, "failed": 0, "skipped": 0, "errored": 0}
    for suite in suites:
        total = int(suite.get("tests", 0))
        failed = int(suite.get("failures", 0))
        errored = int(suite.get("errors", 0))
        skipped = int(suite.get("skipped", 0))
        totals["total"] += total
        totals["failed"] += failed
        totals["errored"] += errored
        totals["skipped"] += skipped
        totals["passed"] += total - failed - errored - skipped
    failing = []
    for case in root.iter("testcase"):
        if case.find("failure") is not None or case.find("error") is not None:
            name = "::".join(filter(None, (case.get("classname"), case.get("name"))))
            failing.append(name or "<unnamed>")
    return totals, failing


def staged_records(stage: Path) -> list[dict[str, Any]]:
    """The JUnit staging manifest `just _stage_junit` appends to."""
    manifest = stage / "manifest.jsonl"
    if not manifest.is_file():
        return []
    records = []
    for line in manifest.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(record, dict) and record.get("tier") in RECORDABLE_GATES:
            records.append(record)
    return records


#: Gates whose result is meaningless without proof that a terminal backend
#: actually drove a test. A Level 2 suite whose backend is absent *skips*, and
#: nextest prints PASS in about 0.02 s — indistinguishable from evidence unless
#: the backend proof is read.
BACKEND_PROVEN_GATES = ("L2",)

#: Written by `test-toolkit` while `BISCUIT_TEST_REQUIRED_BACKENDS` is set: one
#: `{backend, test, decision}` record per gate decision.
BACKEND_EXECUTIONS_FILE = "backend-executions.jsonl"

#: Written by `just ci-local`: the backends it required for each invocation,
#: which is the only per-cell attribution available — the execution records
#: name a test, not a package.
GATE_BACKENDS_FILE = "gate-backends.jsonl"


def _jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.is_file():
        return []
    records = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(record, dict):
            records.append(record)
    return records


def executed_backends(stage: Path) -> set[str]:
    """The backends that actually ran at least one test in this staging directory."""
    return {
        record["backend"]
        for record in _jsonl(stage / BACKEND_EXECUTIONS_FILE)
        if record.get("decision") == "run" and record.get("backend")
    }


def required_backends(stage: Path) -> dict[tuple[str, str], list[str]]:
    """The backends each `{package, gate}` invocation required."""
    return {
        (record["package"], record["gate"]): list(record.get("backends", []))
        for record in _jsonl(stage / GATE_BACKENDS_FILE)
        if record.get("package") and record.get("gate")
    }


def receipt_cell(
    record: dict[str, Any],
    stage: Path,
    plan: dict[str, Any],
    head: str,
    proven: set[str] | None = None,
    required: dict[tuple[str, str], list[str]] | None = None,
) -> dict[str, Any]:
    """One measured cell of a local run.

    A gate whose report is missing or unreadable is recorded `partial`: the
    run has an exit code but no attributable outcome, so it must not stand in
    for a CI execution. The same applies to a failure whose report names no
    failing test — the receipt would assert a failure nothing can be acted on,
    and to a backend-proven gate no backend actually drove.
    """
    gate = record["tier"]
    package = record["package"]
    paths = cell_input_paths(plan, package, gate)
    measured = junit_counts(stage / record.get("xml", ""))
    counts = {field: 0 for field in schema.COUNT_FIELDS}
    failing: list[str] = []
    completion = "partial"
    if measured is not None:
        counts, failing = measured
        completion = "complete"
    outcome = "pass" if record.get("exit_code", 0) == 0 else "fail"
    if outcome == "fail" and not failing:
        completion = "partial"

    wanted = (required or {}).get((package, gate), [])
    backends = sorted(set(wanted) & (proven or set()))
    if gate in BACKEND_PROVEN_GATES and not backends:
        completion = "partial"

    cell = {
        "package": package,
        "gate": gate,
        "outcome": outcome,
        "exit_code": int(record.get("exit_code", 0)),
        "completion": completion,
        "counts": counts,
        "duration_s": record.get("duration_s", 0),
        "gate_input_identity": (
            gate_input_identity(paths, head) if paths else schema.identity(f"{package}/{gate}")
        ),
        "backends": backends,
        "report": record.get("xml", ""),
    }
    if len(failing) > schema.FAILURE_DETAIL_LIMIT:
        cell["failed_tests"] = sorted(failing)[: schema.FAILURE_DETAIL_LIMIT]
        cell["failure_detail_truncated"] = True
    elif failing:
        cell["failed_tests"] = sorted(failing)
    return cell


def record_cells(
    plan_path: str,
    stage_dir: str,
    environment: str,
    base: str,
    head: str,
    completion: str = "complete",
    report_dir: str = "",
) -> str:
    """The version-2 receipt for a finished local run, as canonical bytes.

    ## Errors

    Raises ``ValueError`` when the tested base is not an ancestor of the
    outgoing head, when the run staged no recordable gate, or when the
    assembled document would not validate — publishing an invalid receipt is
    worse than publishing none, because CI would reject it silently later.
    """
    head_sha = revision(head)
    if git("merge-base", base, head_sha) != revision(base):
        raise ValueError("the tested base must be an ancestor of the outgoing head")
    plan = load_plan(plan_path)
    problems = schema.validate_resolved_plan(plan)
    if problems:
        raise ValueError(f"the resolved plan is invalid: {problems[0]}")

    stage = Path(stage_dir)
    proven = executed_backends(stage)
    required = required_backends(stage)
    cells = [
        receipt_cell(record, stage, plan, head_sha, proven, required)
        for record in staged_records(stage)
    ]
    if not cells:
        raise ValueError(
            f"the run staged no L1/L2/browser report under {stage_dir}; there is "
            "nothing to publish"
        )
    return assemble_receipt(
        plan, environment, base, head_sha, cells, completion, report_dir
    )


def assemble_receipt(
    plan: dict[str, Any],
    environment: str,
    base: str,
    head_sha: str,
    cells: list[dict[str, Any]],
    completion: str,
    report_dir: str,
    host: dict[str, str] | None = None,
) -> str:
    """The receipt document, validated, as the bytes a Git note carries.

    ## Errors

    Raises ``ValueError`` when the document would not validate — publishing an
    invalid receipt is worse than publishing none, because CI would reject it
    silently later.
    """
    document = {
        "schema_version": schema.RECEIPT_SCHEMA_VERSION,
        "environment": environment,
        "base": revision(base),
        "head": head_sha,
        "tree": revision(f"{head_sha}^{{tree}}"),
        "scope_identity": schema.identity(schema.canonical(plan)),
        "completion": completion,
        "host": host
        or {
            "os": platform.system(),
            "kernel": f"{platform.system()} {platform.release()}",
            "report_dir": report_dir
            or f"~/.rusty-biscuit/ci-evidence/{head_sha[:9]}",
        },
        "cells": cells,
    }
    problems = schema.validate_receipt(document)
    if problems:
        raise ValueError(f"the assembled receipt is invalid: {problems[0]}")
    return schema.canonical(document)


# ---------------------------------------------------------------------------
# Cross-check publication (spec section 3.8, Design Decision 7)
# ---------------------------------------------------------------------------


def cross_check_rejection(
    tested_tree: str, head_tree: str, remote_dirty: bool, run_filters: str
) -> str | None:
    """Why a `cross-check` run may not become a receipt, or `None` if it may.

    `scripts/cross-check.sh` ships the developer's *local* tree — including
    uncommitted work — to a standing clone. That is the right thing for a smoke
    run and the wrong thing for evidence: only a run whose tested tree is the
    outgoing head's tree, on a clean remote worktree, with no test filter
    narrowing the cell, describes the cell CI would otherwise execute.
    """
    if not tested_tree or tested_tree != head_tree:
        return (
            f"the remote tested tree {tested_tree[:9] or '<unknown>'} is not this "
            f"head's tree {head_tree[:9]}; a patched or older tree is not evidence "
            "for this head"
        )
    if remote_dirty:
        return "the remote worktree was dirty, so the run did not test a known tree"
    if run_filters.strip():
        return (
            f"the run was narrowed by {run_filters.strip()!r}; a filtered run does "
            "not cover the cell it would be published as"
        )
    return None


def record_cross_check(
    plan_path: str,
    package: str,
    report: str,
    exit_code: int,
    duration_s: int,
    tested_tree: str,
    remote_dirty: bool,
    run_filters: str,
    base: str,
    head: str,
    environment: str = "wsl2-ubuntu",
    host_label: str = "",
) -> str:
    """The receipt for a qualifying `cross-check` run.

    ## Errors

    Raises ``ValueError`` with the exact publication reason when the run does
    not qualify, so the caller can print it rather than silently publishing
    nothing.
    """
    head_sha = revision(head)
    head_tree = revision(f"{head_sha}^{{tree}}")
    refusal = cross_check_rejection(tested_tree, head_tree, remote_dirty, run_filters)
    if refusal is not None:
        raise ValueError(refusal)

    plan = load_plan(plan_path)
    problems = schema.validate_resolved_plan(plan)
    if problems:
        raise ValueError(f"the resolved plan is invalid: {problems[0]}")
    if not any(
        cell["package"] == package
        and cell["environment"] == environment
        and cell["gate"] == "L1"
        for cell in plan["cells"]
    ):
        raise ValueError(
            f"the resolved plan has no {package}/{environment}/L1 cell for this "
            "head, so the run proves nothing the plan asked for"
        )

    stage = Path(report).parent
    cell = receipt_cell(
        {
            "tier": "L1",
            "package": package,
            "xml": Path(report).name,
            "exit_code": exit_code,
            "duration_s": duration_s,
        },
        stage,
        plan,
        head_sha,
    )
    if cell["completion"] != "complete":
        raise ValueError(
            f"the remote run produced no usable report for {package}; nothing is "
            "published"
        )
    return assemble_receipt(
        plan,
        environment,
        base,
        head_sha,
        [cell],
        "complete",
        "",
        host={
            "os": "Linux",
            "kernel": host_label or "WSL2 (cross-check archive mode)",
            "report_dir": str(stage),
        },
    )


# ---------------------------------------------------------------------------
# Version-1 compatibility (spec section 3.6)
# ---------------------------------------------------------------------------


def expected_evidence(
    scope: dict[str, Any], base: str, head: str, environment: str
) -> dict[str, Any]:
    source = sorted(scope.get("source_packages", []))
    reverse = sorted(scope.get("reverse_dependencies", []))
    matrix = {
        entry["package"]: entry
        for entry in scope.get("matrix", [])
        if entry["package"] in source
    }

    def has_l1(entry: dict[str, Any]) -> bool:
        if "test" not in entry.get("gates", []):
            return False
        if environment == "wsl2-ubuntu":
            return bool(entry.get("wsl"))
        return environment in entry.get("native_environments", [])

    l1 = sorted(name for name, entry in matrix.items() if has_l1(entry))
    l2 = sorted(
        name
        for name, entry in matrix.items()
        if "L2" in entry.get("tiers", [])
        and environment in entry.get("l2_environments", [])
    )
    return {
        "schema_version": SCHEMA_VERSION,
        "base": revision(base),
        "head": revision(head),
        "tree": revision(f"{head}^{{tree}}"),
        "environment": environment,
        "source_packages": source,
        "reverse_dependencies": reverse,
        "l1_packages": l1,
        "l2_packages": l2,
    }


def load_scope(path: str) -> dict[str, Any]:
    document = json.loads(Path(path).read_text(encoding="utf-8"))
    if not isinstance(document, dict):
        raise ValueError("scope must be a JSON object")
    return document


def record(scope_path: str, base: str, head: str, environment: str) -> str:
    """A version-1 receipt. Retained to read and test notes already in the wild."""
    if git("merge-base", base, head) != revision(base):
        raise ValueError("the tested base must be an ancestor of the outgoing head")
    return json.dumps(
        expected_evidence(load_scope(scope_path), base, head, environment),
        sort_keys=True,
        separators=(",", ":"),
    )


def verified_environment(scope_path: str, base: str, head: str) -> str:
    """The first environment whose version-1 note matches the legacy scope.

    Superseded by [`verify_cells`], which answers per cell across every
    environment at once. Kept because version-1 notes bind to exact heads and
    must keep verifying until those heads are gone.
    """
    scope = load_scope(scope_path)
    # PR base.sha may have advanced since the branch diverged. The hook tests
    # from that divergence point; CI still compares its independently computed
    # package/tier scope, so additional upstream changes cannot widen coverage.
    try:
        base = git("merge-base", base, head)
    except (OSError, subprocess.SubprocessError):
        return ""
    for environment in ENVIRONMENTS:
        try:
            note = git(
                "notes", "--ref", f"{NOTES_PREFIX}/{environment}", "show", head
            )
            actual = json.loads(note)
            expected = expected_evidence(scope, base, head, environment)
        except (OSError, subprocess.SubprocessError, ValueError, json.JSONDecodeError):
            continue
        if actual == expected:
            return environment
    return ""


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    recorder = subparsers.add_parser(
        "record", help="emit a version-1 receipt from a legacy scope document"
    )
    recorder.add_argument("--scope", required=True)
    recorder.add_argument("--base", required=True)
    recorder.add_argument("--head", required=True)
    recorder.add_argument("--environment", required=True, choices=ENVIRONMENTS)

    cells = subparsers.add_parser(
        "record-cells", help="emit a version-2 receipt from a finished local run"
    )
    cells.add_argument("--plan", required=True)
    cells.add_argument("--stage", required=True)
    cells.add_argument("--base", required=True)
    cells.add_argument("--head", required=True)
    cells.add_argument("--environment", required=True, choices=ENVIRONMENTS)
    cells.add_argument(
        "--completion", default="complete", choices=list(schema.COMPLETIONS)
    )
    cells.add_argument("--report-dir", default="")

    cross = subparsers.add_parser(
        "cross-check", help="emit a receipt for a qualifying cross-check run"
    )
    cross.add_argument("--plan", required=True)
    cross.add_argument("--package", required=True)
    cross.add_argument("--report", required=True)
    cross.add_argument("--exit-code", type=int, required=True)
    cross.add_argument("--duration", type=int, default=0)
    cross.add_argument("--tested-tree", required=True)
    cross.add_argument("--remote-dirty", action="store_true")
    cross.add_argument("--run-filters", default="")
    cross.add_argument("--base", required=True)
    cross.add_argument("--head", required=True)
    cross.add_argument("--environment", default="wsl2-ubuntu", choices=ENVIRONMENTS)
    cross.add_argument("--host-label", default="")

    verifier = subparsers.add_parser("verify")
    verifier.add_argument("--scope", help="legacy scope document (version-1 path)")
    verifier.add_argument("--plan", help="resolved plan (version-2 path)")
    verifier.add_argument("--base", required=True)
    verifier.add_argument("--head", required=True)
    verifier.add_argument(
        "--cells",
        action="store_true",
        help="emit the accepted per-cell set as JSON instead of one environment name",
    )
    verifier.add_argument(
        "--rejections",
        metavar="FILE",
        help="write the coded rejection reasons here",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if args.command == "record":
        print(record(args.scope, args.base, args.head, args.environment))
        return
    if args.command == "record-cells":
        print(
            record_cells(
                args.plan,
                args.stage,
                args.environment,
                args.base,
                args.head,
                args.completion,
                args.report_dir,
            )
        )
        return
    if args.command == "cross-check":
        try:
            print(
                record_cross_check(
                    args.plan,
                    args.package,
                    args.report,
                    args.exit_code,
                    args.duration,
                    args.tested_tree,
                    args.remote_dirty,
                    args.run_filters,
                    args.base,
                    args.head,
                    args.environment,
                    args.host_label,
                )
            )
        except (OSError, ValueError) as refusal:
            print(f"cross-check: publishing no receipt — {refusal}", file=sys.stderr)
            raise SystemExit(3) from refusal
        return
    if args.cells:
        if not args.plan:
            raise SystemExit("verify --cells requires --plan")
        accepted, rejections = verify_cells(args.plan, args.base, args.head)
        if args.rejections:
            Path(args.rejections).write_text(
                json.dumps(rejections, indent=0), encoding="utf-8"
            )
        print(json.dumps(accepted, separators=(",", ":")))
        return
    if not args.scope:
        raise SystemExit("verify without --cells requires --scope")
    print(verified_environment(args.scope, args.base, args.head))


if __name__ == "__main__":
    main()

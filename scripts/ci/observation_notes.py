#!/usr/bin/env python3
"""Publish observations to the `refs/notes/ci-local/<environment>` collections.

A note on a commit holds a version-3 collection of immutable observations
(`schema.py`). Several producers write to the same commit and environment —
sequential packages, a retry, a second host — so a note is never overwritten:
publication unions what is already there with what is new.

The order of operations is the contract:

1. The merged collection is written to the **local** notes ref first. From
   that point the proof survives anything that happens to the remote.
2. The remote ref is fetched into a scratch ref, its collection for the commit
   is unioned in, and the local ref is rebased onto the fetched tip by
   re-attaching local notes. Nothing is ever force-pushed.
3. The push is non-force. A rejected push means someone published in between,
   so the loop fetches and merges again, up to [`MAX_ATTEMPTS`] times with a
   jittered backoff.

A failure at any point leaves the local ref intact and is reported; the cells
the unpublished observations cover simply stay scheduled for everyone else.

Two producers call this module. The pre-push hook publishes a version-2
receipt's cells (`publish-receipt`, kind `hook`), with the plan it ran under so
each observation records its package's feature arguments ([`hook_features`]).
A rig run of
`scripts/cross-check.sh` publishes one cell per finished leg (`publish-receipt
--kind rig`, [`publish_rig_cell`]): the fetched staging directory is judged by
`completion.py`, the documents the proof cites are retained under the evidence
directory, and the observation is `rig` with that directory as its
`report_dir`. This runs on the orchestrating host over fetched files because a
native Windows rig has no usable `python3`.

A rig leg that re-runs a cell names its predecessor (`--retry-of`), which the
script asks `retry-predecessor` for before the leg starts. Lineage is never
derived from the order observations reach a collection.
"""

from __future__ import annotations

import argparse
import contextlib
import hashlib
import json
import platform
import random
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Iterator

sys.path.insert(0, str(Path(__file__).resolve().parent))

import completion  # noqa: E402
import local_evidence  # noqa: E402
import schema  # noqa: E402

NOTES_PREFIX = "refs/notes/ci-local"
SCRATCH_PREFIX = "refs/notes/ci-remote"

#: Push attempts before the observation is left local-only.
MAX_ATTEMPTS = 5


class PublicationError(ValueError):
    """An observation could not be merged into, or written to, a collection."""


@dataclass
class PublishResult:
    """What happened to one publication.

    `local` is whether the proof is on this host's notes ref; `published` is
    whether the remote carries it too. They differ exactly when the remote was
    unreachable, kept rejecting the push, or there was no remote to try.
    """

    local: bool
    published: bool
    attempts: int
    observation_ids: list[str]
    error: str | None = None


def _git(
    *args: str, repo: Path | None = None, text_input: str | None = None, check: bool = True
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=repo,
        input=text_input,
        capture_output=True,
        text=True,
        check=check,
        timeout=120,
    )


def ref_for(environment: str) -> str:
    return f"{NOTES_PREFIX}/{environment}"


def file_digest(path: Path) -> tuple[str, int]:
    content = path.read_bytes()
    return hashlib.sha256(content).hexdigest(), len(content)


def with_identity(observation: dict[str, Any]) -> dict[str, Any]:
    """`observation` carrying the id of its own content."""
    body = {name: value for name, value in observation.items() if name != "observation_id"}
    return {"observation_id": schema.observation_identity(body), **body}


def receipt_observations(
    receipt: dict[str, Any],
    *,
    kind: str = "hook",
    proof: dict[str, str] | None = None,
    features: str = "",
    package_features: dict[str, str] | None = None,
    run_id: str = "",
) -> list[dict[str, Any]]:
    """One observation per cell of a valid version-2 receipt.

    Each cell's report is digested from `receipt.host.report_dir`, where the
    recorder already verified it exists. `proof`, `features`, and `run_id` are
    carried onto every observation; a `hook` observation proves through its
    report counts and takes no proof. `package_features` gives a cell's
    feature arguments by package when they differ per cell, as a hook
    receipt's do; `features` overrides it.

    ## Errors

    Raises [`PublicationError`] for an invalid receipt or a retained report that
    cannot be read: an observation naming bytes nothing can show is the
    "claim, not evidence" the digest exists to prevent.
    """
    problems = schema.validate_receipt(receipt)
    if problems:
        raise PublicationError(f"the receipt is invalid: {problems[0]}")
    host = receipt["host"]
    directory = Path(host["report_dir"]).expanduser()
    observations = []
    for cell in receipt["cells"]:
        reports = []
        if cell["completion"] == "complete":
            try:
                digest, size = file_digest(directory / cell["report"])
            except OSError as error:
                raise PublicationError(
                    f"{directory} does not retain {cell['report']}: {error}"
                ) from error
            reports.append(
                {"path": cell["report"], "sha256": digest, "bytes": size, "store": "host-dir"}
            )
        producer: dict[str, Any] = {"kind": kind, "host": dict(host)}
        if run_id:
            producer["run_id"] = run_id
        observation: dict[str, Any] = {
            "producer": producer,
            "base": receipt["base"],
            "scope_identity": receipt["scope_identity"],
            "completion": receipt["completion"],
            "cell": cell,
            "reports": reports,
        }
        if proof is not None:
            observation["proof"] = dict(proof)
        cell_features = features or (package_features or {}).get(cell["package"], "")
        if cell_features:
            observation["features"] = cell_features
        observations.append(with_identity(observation))
    return sorted(observations, key=lambda observation: observation["observation_id"])


def hook_features(plan: dict[str, Any]) -> dict[str, str]:
    """Each package's feature arguments in a hook run under `plan`.

    `just ci-local` passes a package's plan `test_args` verbatim to `_test`
    and `_test_l2`, the only gates a hook receipt records, and the plan keys
    that package's builds on the same string, so this is both what ran and
    what the verifier compares.
    """
    return {package["package"]: package.get("test_args", "") for package in plan["packages"]}


def collection_of(
    environment: str, head: str, tree: str, observations: list[dict[str, Any]]
) -> dict[str, Any]:
    document = {
        "schema_version": schema.COLLECTION_SCHEMA_VERSION,
        "environment": environment,
        "head": head,
        "tree": tree,
        "observations": sorted(observations, key=lambda o: o["observation_id"]),
    }
    problems = schema.validate_collection(document)
    if problems:
        raise PublicationError(f"the collection would be invalid: {problems[0]}")
    return document


def _cell_identity(observation: dict[str, Any]) -> tuple[str, str, str]:
    cell = observation["cell"]
    return (cell["package"], cell["gate"], cell.get("test_filter", ""))


def _same_run(left: dict[str, Any], right: dict[str, Any]) -> bool:
    ignored = ("observation_id", "retry_of")
    strip = lambda o: {k: v for k, v in o.items() if k not in ignored}  # noqa: E731
    return strip(left) == strip(right)


def _hosted_predecessor(
    observation: dict[str, Any], merged: dict[str, dict[str, Any]]
) -> str | None:
    """The id of the earlier attempt of the same hosted run this one re-ran.

    GitHub starts a re-run attempt only after the previous attempt finished, so
    a lower attempt of the same workflow run and cell is an earlier execution
    whenever it is published. The highest such attempt is named.
    """
    producer = observation["producer"]
    if producer["kind"] != "hosted":
        return None
    cell = _cell_identity(observation)
    earlier = [
        other for other in merged.values()
        if other["producer"]["kind"] == "hosted"
        and _cell_identity(other) == cell
        and other["producer"].get("workflow") == producer.get("workflow")
        and other["producer"].get("run_id") == producer.get("run_id")
        and other["producer"].get("attempt", 0) < producer.get("attempt", 0)
    ]
    if not earlier:
        return None
    return max(earlier, key=lambda o: (o["producer"]["attempt"], o["observation_id"]))["observation_id"]


def merge_observations(
    existing: list[dict[str, Any]], new: list[dict[str, Any]]
) -> list[dict[str, Any]]:
    """The union of two observation lists.

    The same id twice is one entry; the same id with different content is
    corruption and is refused. A `new` observation identical to an existing one
    apart from its id/retry link is a republication and is dropped.

    Retry lineage is never inferred from the order observations arrive: a
    delayed pass published after a newer failure must stay unordered with it,
    so the verifier reports the conflict and the cell executes. The one link
    added here is execution identity: a hosted re-run attempt names the
    earlier attempt of the same run and cell ([`_hosted_predecessor`]). Any
    other retry carries the `retry_of` its producer recorded when it started.
    """
    merged = {o["observation_id"]: o for o in existing}
    ordered = sorted(new, key=lambda o: o["producer"].get("attempt", 0))
    for observation in ordered:
        known = merged.get(observation["observation_id"])
        if known is not None:
            if known != observation:
                raise PublicationError(
                    f"observation {observation['observation_id'][:12]} exists with "
                    "different content"
                )
            continue
        if any(_same_run(observation, other) for other in merged.values()):
            continue
        if "retry_of" not in observation:
            predecessor = _hosted_predecessor(observation, merged)
            if predecessor is not None:
                observation = with_retry_of(observation, predecessor)
        merged[observation["observation_id"]] = observation
    return sorted(merged.values(), key=lambda o: o["observation_id"])


def with_retry_of(observation: dict[str, Any], predecessor: str) -> dict[str, Any]:
    """`observation` re-identified as the retry of `predecessor`."""
    return with_identity({**observation, "retry_of": predecessor})


def retry_predecessor(
    environment: str, commit: str, package: str, gate: str, repo: Path | None = None
) -> str | None:
    """The observation a run of this cell starting now retries, or `None`.

    Called when a retry *starts*, before it executes: every observation already
    in the local collection then describes an earlier execution, so naming the
    cell's one unsuperseded observation records real lineage. Several
    unsuperseded observations are themselves unordered, and none is named.
    Calling this after the run finishes would reintroduce publication-order
    lineage.
    """
    try:
        commit = _git("rev-parse", "--verify", commit, repo=repo).stdout.strip()
    except subprocess.CalledProcessError:
        return None
    text = read_note(ref_for(environment), commit, repo)
    if not text:
        return None
    try:
        collection = _collection_from_note(text, environment, commit, repo)
    except PublicationError:
        return None
    peers = [o for o in collection["observations"] if _cell_identity(o) == (package, gate, "")]
    retried = {o.get("retry_of") for o in peers}
    heads = [o["observation_id"] for o in peers if o["observation_id"] not in retried]
    return heads[0] if len(heads) == 1 else None


def _collection_from_note(
    text: str, environment: str, commit: str, repo: Path | None
) -> dict[str, Any]:
    """A note of any supported generation, as a collection.

    ## Errors

    Raises [`PublicationError`] for a note this cannot carry forward: a
    version-1 receipt (no per-cell result to convert), a version-2 receipt
    whose reports are no longer retained here, or an invalid document.
    """
    try:
        document = schema.loads_strict(text)
    except ValueError as error:
        raise PublicationError(f"the note on {commit[:9]} is not valid JSON: {error}") from error
    if not isinstance(document, dict):
        raise PublicationError(f"the note on {commit[:9]} is not an object")
    version = document.get("schema_version")
    if schema.is_version(version, schema.COLLECTION_SCHEMA_VERSION):
        problems = schema.validate_collection(document)
        if problems:
            raise PublicationError(f"the collection on {commit[:9]} is invalid: {problems[0]}")
        return document
    if schema.is_version(version, schema.RECEIPT_SCHEMA_VERSION):
        return collection_of(
            environment, document["head"], document["tree"], receipt_observations(document)
        )
    raise PublicationError(
        f"the note on {commit[:9]} is version {version!r} and cannot be carried "
        "into a version-3 collection"
    )


def read_note(ref: str, commit: str, repo: Path | None = None) -> str | None:
    result = _git("notes", "--ref", ref, "show", commit, repo=repo, check=False)
    return result.stdout if result.returncode == 0 else None


def write_note(ref: str, commit: str, collection: dict[str, Any], repo: Path | None = None) -> None:
    text = schema.canonical(collection)
    if len(text.encode("utf-8")) > schema.COLLECTION_SIZE_LIMIT:
        raise PublicationError(
            f"the collection for {commit[:9]} is {len(text)} bytes, over the "
            f"{schema.COLLECTION_SIZE_LIMIT} a note may carry"
        )
    blob = _git("hash-object", "-w", "--stdin", repo=repo, text_input=text).stdout.strip()
    _git("notes", "--ref", ref, "add", "-f", "-C", blob, commit, repo=repo)


def _notes_map(ref: str, repo: Path | None) -> dict[str, str]:
    result = _git("notes", "--ref", ref, "list", repo=repo, check=False)
    mapping = {}
    for line in result.stdout.splitlines():
        fields = line.split()
        if len(fields) == 2:
            mapping[fields[1]] = fields[0]
    return mapping


def _blob_text(blob: str, repo: Path | None) -> str:
    return _git("cat-file", "-p", blob, repo=repo).stdout


def _remote_has(remote: str, ref: str, repo: Path | None) -> bool:
    result = _git("ls-remote", remote, ref, repo=repo)
    return bool(result.stdout.strip())


def _rebase_onto(
    ref: str, scratch: str, commit: str, merged: dict[str, Any], repo: Path | None,
) -> list[str]:
    """Move `ref` onto the fetched tip, re-attaching every local note.

    Returns the commits whose local note could not be unioned with a different
    remote note and so kept the remote's.
    """
    dropped: list[str] = []
    local = _notes_map(ref, repo)
    remote = _notes_map(scratch, repo)
    environment = ref.rsplit("/", 1)[-1]
    texts = {c: _blob_text(blob, repo) for c, blob in local.items() if remote.get(c) != blob}
    _git("update-ref", ref, scratch, repo=repo)
    for other, text in texts.items():
        if other == commit:
            write_note(ref, other, merged, repo)
            continue
        if other in remote:
            try:
                ours = _collection_from_note(text, environment, other, repo)
                theirs = _collection_from_note(_blob_text(remote[other], repo), environment, other, repo)
                union = {**ours, "observations": merge_observations(
                    theirs["observations"], ours["observations"])}
                write_note(ref, other, union, repo)
            except PublicationError:
                dropped.append(other)
            continue
        blob = _git("hash-object", "-w", "--stdin", repo=repo, text_input=text).stdout.strip()
        _git("notes", "--ref", ref, "add", "-f", "-C", blob, other, repo=repo)
    if commit not in texts:
        write_note(ref, commit, merged, repo)
    return dropped


def publish(
    environment: str,
    commit: str,
    new_observations: list[dict[str, Any]],
    *,
    tree: str | None = None,
    remote: str | None = "origin",
    repo: Path | None = None,
    attempts: int = MAX_ATTEMPTS,
    sleep: Callable[[float], None] = time.sleep,
    before_push: Callable[[int], None] | None = None,
) -> PublishResult:
    """Add `new_observations` to the collection on `commit`, locally then remotely.

    `before_push(attempt)` runs immediately before each push; it exists so a
    test can make another publisher win the race deterministically.

    ## Errors

    Raises [`PublicationError`] only when the *local* write cannot happen (an
    invalid observation, an uncarriable existing note, an oversized
    collection). Remote problems are returned in [`PublishResult.error`].
    """
    ref = ref_for(environment)
    commit = _git("rev-parse", "--verify", commit, repo=repo).stdout.strip()
    tree = tree or _git("rev-parse", "--verify", f"{commit}^{{tree}}", repo=repo).stdout.strip()
    existing = read_note(ref, commit, repo)
    current = _collection_from_note(existing, environment, commit, repo) if existing else None
    merged = collection_of(
        environment,
        commit,
        tree,
        merge_observations(
            current["observations"] if current else [],
            new_observations,
        ),
    )
    # Republishing what the note already holds must not move the notes ref.
    if current is None or schema.canonical(current) != schema.canonical(merged):
        write_note(ref, commit, merged, repo)
    ids = [o["observation_id"] for o in merged["observations"]]
    result = PublishResult(local=True, published=False, attempts=0, observation_ids=ids)
    if remote is None:
        result.error = "no remote was given; the observation is local only"
        return result

    scratch = f"{SCRATCH_PREFIX}/{environment}"
    for attempt in range(1, attempts + 1):
        result.attempts = attempt
        try:
            if _remote_has(remote, ref, repo):
                _git("fetch", "--no-tags", remote, f"+{ref}:{scratch}", repo=repo)
                theirs_text = read_note(scratch, commit, repo)
                if theirs_text:
                    theirs = _collection_from_note(theirs_text, environment, commit, repo)
                    unioned = collection_of(
                        environment, commit, tree,
                        merge_observations(theirs["observations"], merged["observations"]),
                    )
                    if schema.canonical(unioned) != schema.canonical(merged):
                        merged = unioned
                        write_note(ref, commit, merged, repo)
                tip = _git("rev-parse", "--verify", scratch, repo=repo).stdout.strip()
                descends = _git(
                    "merge-base", "--is-ancestor", tip, ref, repo=repo, check=False
                ).returncode == 0
                if not descends:
                    _rebase_onto(ref, scratch, commit, merged, repo)
            if before_push is not None:
                before_push(attempt)
            push = _git("push", "--no-verify", remote, f"{ref}:{ref}", repo=repo, check=False)
            if push.returncode == 0:
                result.published = True
                result.error = None
                result.observation_ids = [o["observation_id"] for o in merged["observations"]]
                return result
            lines = (push.stderr or push.stdout).strip().splitlines()
            result.error = lines[-1] if lines else f"git push exited {push.returncode}"
        except (subprocess.SubprocessError, OSError, PublicationError) as error:
            result.error = f"{type(error).__name__}: {error}"
        if attempt < attempts:
            sleep(random.uniform(0.2, 1.0) * attempt)
    result.error = (
        f"the remote kept rejecting the push after {attempts} attempt(s): {result.error}; "
        "the observation is local only"
    )
    return result


@contextlib.contextmanager
def _removed_on_failure(path: Path) -> Iterator[None]:
    """Remove `path` if the block does not finish: a declined leg leaves no litter."""
    try:
        yield
    except BaseException:
        shutil.rmtree(path, ignore_errors=True)
        raise


class RigDeclined(Exception):
    """A rig run that must publish nothing, with the reason to print."""


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _manifest_of(path: Path) -> dict[str, Any] | None:
    """The expected-test manifest if it is a readable JSON object, else `None`."""
    try:
        document = schema.loads_strict(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None
    return document if isinstance(document, dict) else None


def _nonempty(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def record_rig_cell(
    *,
    plan_path: str,
    package: str,
    environment: str,
    reports_dir: str,
    evidence_dir: str,
    run_id: str,
    head: str,
    base: str,
    tested_tree: str,
    remote_dirty: bool,
    run_filters: str,
    exit_code: int,
    duration_s: int,
    host_label: str = "",
    build_key: str = "",
    build_digest: str = "",
    root: str = ".",
    gate: str = "L1",
) -> tuple[str, list[dict[str, Any]]]:
    """The `rig` observation(s) for one finished leg, and the head they belong to.

    `reports_dir` is the leg's fetched staging directory: `manifest.jsonl`,
    `<gate>/<package>.xml`, and the producer's `expected-<gate>.json`. The
    documents the proof cites are copied into one directory per cell and run,
    `<evidence_dir>/<head>/<environment>/<package>-<gate>-<run_id>/`, which
    becomes the observation's `report_dir`.

    A cell that finished with failing tests is a completed failure and is
    recorded as such; `completion.py` refuses such a run, so no completion
    record exists and the failing observation carries at most the manifest-side
    proof. Every other refusal means the run is incomplete.

    ## Errors

    Raises [`RigDeclined`] with the printable reason when nothing may be
    published: an unqualified tree, a cell the plan did not schedule, an
    incomplete run, or an exit code that contradicts the report.
    """
    if environment not in local_evidence.CROSS_CHECK_HOSTS:
        raise RigDeclined(
            f"{environment} is not a rig environment; a WSL2 result is never filed "
            "as native Windows, nor the reverse"
        )
    try:
        head_sha = local_evidence.revision(head)
        head_tree = local_evidence.revision(f"{head_sha}^{{tree}}")
    except ValueError as error:
        raise RigDeclined(str(error)) from error
    refusal = local_evidence.cross_check_rejection(
        tested_tree, head_tree, remote_dirty, run_filters
    )
    if refusal is not None:
        raise RigDeclined(refusal)

    plan = local_evidence.load_plan(plan_path)
    problems = schema.validate_resolved_plan(plan)
    if problems:
        raise RigDeclined(f"the resolved plan is invalid: {problems[0]}")
    planned = next(
        (
            cell
            for cell in plan["cells"]
            if (cell["package"], cell["environment"], cell["gate"])
            == (package, environment, gate)
        ),
        None,
    )
    if planned is None:
        raise RigDeclined(
            f"the resolved plan has no {package}/{environment}/{gate} cell for this "
            "head, so the run proves nothing the plan asked for"
        )

    reports = Path(reports_dir)
    report_source = reports / gate / f"{package}.xml"
    if not report_source.is_file():
        raise RigDeclined("the remote run produced no JUnit report")

    retained = Path(evidence_dir).expanduser() / head_sha / environment / f"{package}-{gate}-{run_id}"
    # A run id names one run: replacing its directory never touches another run's
    # documents, which the digests of an earlier observation depend on.
    if retained.exists():
        shutil.rmtree(retained)
    retained.mkdir(parents=True)
    with _removed_on_failure(retained):
        report = retained / f"{package}-{gate}.xml"
        shutil.copyfile(report_source, report)
        expected = retained / schema.PROOF_EXPECTED_FILE.format(package=package, gate=gate)
        manifest: dict[str, Any] | None = None
        manifest_source = reports / f"expected-{gate}.json"
        if manifest_source.is_file():
            shutil.copyfile(manifest_source, expected)
            manifest = _manifest_of(expected)

        completion_file = retained / schema.PROOF_COMPLETION_FILE.format(package=package, gate=gate)
        backend_proofs = reports / "backend-proofs.json"
        target = manifest.get("target", "") if manifest is not None else ""
        arguments = argparse.Namespace(
            plan=Path(plan_path),
            cell=f"{package}/{environment}/{gate}",
            expected_manifest=expected if manifest is not None else None,
            artifacts=reports,
            companions=None,
            backend_proofs=backend_proofs if backend_proofs.is_file() else None,
            target=target if isinstance(target, str) else "",
            run=run_id,
            attempt=1,
            out=completion_file,
            root=Path(root),
        )
        proven = False
        codes: list[str] = []
        try:
            record = completion.validate(arguments)
        except completion.Refused as refused:
            codes = [problem.split(":", 1)[0] for problem in refused.problems]
            failed_only = (
                not refused.infrastructure
                and bool(codes)
                and set(codes) == {"completion-test-failed"}
            )
            if manifest is None and codes == ["completion-manifest-missing"]:
                # No usable listing: the only completed result left is a failure the
                # report itself shows, which `record_cross_check` confirms below.
                failed_only = exit_code != 0
            if not failed_only:
                raise RigDeclined(
                    "the run is incomplete and proves nothing: " + "; ".join(refused.problems)
                ) from refused
        else:
            proven = True
            completion_file.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")

        if proven and exit_code != 0:
            raise RigDeclined(
                f"the run exited {exit_code} although its report shows every expected "
                "test passing, so the outcome is not attributable"
            )
        if not proven and exit_code == 0:
            raise RigDeclined(
                "the run exited 0 although its report shows a failing test, so the "
                "outcome is not attributable"
            )

        try:
            receipt = json.loads(
                local_evidence.record_cross_check(
                    plan_path, package, str(report), exit_code, duration_s, tested_tree,
                    remote_dirty, run_filters, base, head_sha, environment, host_label,
                    build_key, build_digest,
                )
            )
        except (OSError, ValueError) as error:
            raise RigDeclined(str(error)) from error

        proof: dict[str, str] | None = None
        if manifest is not None and _nonempty(manifest.get("nextest_version")) and _nonempty(target):
            proof = {
                "nextest_version": manifest["nextest_version"],
                "target": target,
                "expected_listing_digest": _sha256(expected),
            }
            if proven:
                proof["completion_record_digest"] = _sha256(completion_file)
        if proven and proof is None:
            raise RigDeclined("the passing run's expected manifest names no nextest version or target")
        observations = receipt_observations(
            receipt,
            kind="rig",
            proof=proof,
            features=local_evidence._planned_features(plan, planned),
            run_id=run_id,
        )
        return head_sha, observations


def publish_rig_cell(*, remote: str | None, retry_of: str | None = None, **recorded: Any) -> int:
    """Record and publish one rig leg; the exit code the script maps.

    `retry_of` is the predecessor [`retry_predecessor`] named when the leg
    started; without it the observation retries nothing.

    `0` published, `4` recorded locally only, `3` nothing recorded. Every
    outcome prints its own line, so a leg that publishes nothing says why.
    """
    environment = recorded["environment"]
    package = recorded["package"]
    try:
        head, observations = record_rig_cell(**recorded)
        if retry_of:
            observations = [with_retry_of(o, retry_of) for o in observations]
    except RigDeclined as reason:
        print(f"cross-check: publishing no {environment} observation - {reason}", file=sys.stderr)
        return 3
    except (PublicationError, ValueError, OSError) as error:
        print(f"cross-check: publishing no {environment} observation - {error}", file=sys.stderr)
        return 3
    outcome = observations[0]["cell"]["outcome"]
    try:
        result = publish(environment, head, observations, remote=remote)
    except (PublicationError, ValueError) as error:
        shutil.rmtree(observations[0]["producer"]["host"]["report_dir"], ignore_errors=True)
        print(f"cross-check: publishing no {environment} observation - {error}", file=sys.stderr)
        return 3
    label = "a passing" if outcome == "pass" else "a FAILING"
    if result.published:
        print(
            f"cross-check: published {label} {environment} observation for "
            f"{package} at {head[:9]}."
        )
        return 0
    print(
        f"cross-check: recorded {label} {environment} observation for {package} "
        f"locally; it could not be pushed ({result.error})",
        file=sys.stderr,
    )
    return 4


def host_description(label: str = "") -> dict[str, str]:
    host = {"os": platform.system(), "kernel": f"{platform.system()} {platform.release()}"}
    if label:
        host["label"] = label
    return host


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    receipt = commands.add_parser(
        "publish-receipt",
        help="publish a version-2 receipt's cells (hook) or one finished rig leg (rig)",
    )
    receipt.add_argument("--kind", choices=("hook", "rig"), default="hook")
    receipt.add_argument("--receipt", help="file holding the receipt (kind hook)")
    receipt.add_argument("--head", required=True)
    receipt.add_argument("--environment", required=True, choices=schema.ENVIRONMENTS)
    receipt.add_argument("--remote", default="origin")
    receipt.add_argument("--no-push", action="store_true")
    receipt.add_argument("--json", action="store_true", help="print the result as JSON")
    rig = receipt.add_argument_group("kind rig")
    receipt.add_argument("--plan", help="the resolved plan the run or leg ran under")
    rig.add_argument("--package")
    rig.add_argument("--reports-dir", help="the leg's fetched staging directory")
    rig.add_argument("--evidence-dir", help="where the per-run proof directory is created")
    rig.add_argument("--run-id")
    rig.add_argument("--base")
    rig.add_argument("--tested-tree", default="")
    rig.add_argument("--remote-dirty", action="store_true")
    rig.add_argument("--run-filters", default="")
    rig.add_argument("--exit-code", type=int)
    rig.add_argument("--duration", type=int, default=0)
    rig.add_argument("--host-label", default="")
    rig.add_argument("--build-key", default="")
    rig.add_argument("--build-digest", default="")
    rig.add_argument(
        "--retry-of",
        help="the observation this leg retries, as `retry-predecessor` printed it when the leg started",
    )
    predecessor = commands.add_parser(
        "retry-predecessor",
        help="before a rig leg starts, print the one observation of its cell it would retry",
    )
    predecessor.add_argument("--head", required=True)
    predecessor.add_argument("--environment", required=True, choices=schema.ENVIRONMENTS)
    predecessor.add_argument("--package", required=True)
    predecessor.add_argument("--gate", default="L1")
    args = parser.parse_args()

    if args.command == "retry-predecessor":
        print(retry_predecessor(args.environment, args.head, args.package, args.gate) or "")
        return

    if args.kind == "rig":
        required = ("plan", "package", "reports_dir", "evidence_dir", "run_id", "base", "exit_code")
        missing = [name for name in required if getattr(args, name) is None]
        if missing:
            parser.error("--kind rig needs " + ", ".join("--" + n.replace("_", "-") for n in missing))
        raise SystemExit(
            publish_rig_cell(
                remote=None if args.no_push else args.remote,
                retry_of=args.retry_of or None,
                plan_path=args.plan,
                package=args.package,
                environment=args.environment,
                reports_dir=args.reports_dir,
                evidence_dir=args.evidence_dir,
                run_id=args.run_id,
                head=args.head,
                base=args.base,
                tested_tree=args.tested_tree,
                remote_dirty=args.remote_dirty,
                run_filters=args.run_filters,
                exit_code=args.exit_code,
                duration_s=args.duration,
                host_label=args.host_label,
                build_key=args.build_key,
                build_digest=args.build_digest,
            )
        )
    if not args.receipt or not args.plan:
        parser.error("--kind hook needs --receipt and --plan")
    if args.retry_of:
        parser.error("--retry-of names one cell's predecessor; a hook receipt retries nothing")

    document = schema.loads_strict(Path(args.receipt).read_text(encoding="utf-8"))
    try:
        result = publish(
            args.environment,
            args.head,
            receipt_observations(
                document, package_features=hook_features(local_evidence.load_plan(args.plan))
            ),
            remote=None if args.no_push else args.remote,
        )
    except (PublicationError, ValueError, OSError, KeyError) as error:
        print(f"publish-receipt: no observation recorded - {error}", file=sys.stderr)
        raise SystemExit(3) from error
    if args.json:
        print(json.dumps({"local": result.local, "published": result.published,
                          "attempts": result.attempts, "error": result.error}))
    if not result.published:
        print(f"publish-receipt: local only - {result.error}", file=sys.stderr)
        raise SystemExit(4)


if __name__ == "__main__":
    main()

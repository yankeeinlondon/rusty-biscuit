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
"""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import random
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable

sys.path.insert(0, str(Path(__file__).resolve().parent))

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
    receipt: dict[str, Any], *, kind: str = "hook"
) -> list[dict[str, Any]]:
    """One observation per cell of a valid version-2 receipt.

    Each cell's report is digested from `receipt.host.report_dir`, where the
    recorder already verified it exists.

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
        observations.append(
            with_identity(
                {
                    "producer": {"kind": kind, "host": dict(host)},
                    "base": receipt["base"],
                    "scope_identity": receipt["scope_identity"],
                    "completion": receipt["completion"],
                    "cell": cell,
                    "reports": reports,
                }
            )
        )
    return sorted(observations, key=lambda observation: observation["observation_id"])


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


def merge_observations(
    existing: list[dict[str, Any]], new: list[dict[str, Any]], *, link_retries: bool = False
) -> list[dict[str, Any]]:
    """The union of two observation lists.

    The same id twice is one entry; the same id with different content is
    corruption and is refused. A `new` observation identical to an existing one
    apart from its id/retry link is a republication and is dropped. With
    `link_retries`, a new observation of a cell that already has exactly one
    unretried observation becomes its retry, which is how a second run of one
    cell at one head supersedes the first; with several unordered ones it stays
    unlinked and the verifier treats a disagreement as a conflict.
    """
    merged = {o["observation_id"]: o for o in existing}
    for observation in new:
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
        if link_retries and "retry_of" not in observation:
            cell = _cell_identity(observation)
            peers = [o for o in merged.values() if _cell_identity(o) == cell]
            retried = {o.get("retry_of") for o in peers}
            heads = [o for o in peers if o["observation_id"] not in retried]
            if len(heads) == 1:
                observation = with_identity(
                    {**{k: v for k, v in observation.items() if k != "observation_id"},
                     "retry_of": heads[0]["observation_id"]}
                )
        merged[observation["observation_id"]] = observation
    return sorted(merged.values(), key=lambda o: o["observation_id"])


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
    link_retries: bool = True,
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
    base = (
        _collection_from_note(existing, environment, commit, repo)["observations"]
        if existing
        else []
    )
    merged = collection_of(
        environment,
        commit,
        tree,
        merge_observations(base, new_observations, link_retries=link_retries),
    )
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
                    merged = collection_of(
                        environment, commit, tree,
                        merge_observations(theirs["observations"], merged["observations"]),
                    )
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


def host_description(label: str = "") -> dict[str, str]:
    host = {"os": platform.system(), "kernel": f"{platform.system()} {platform.release()}"}
    if label:
        host["label"] = label
    return host


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    receipt = commands.add_parser(
        "publish-receipt", help="publish the cells of a version-2 receipt as observations"
    )
    receipt.add_argument("--receipt", required=True, help="file holding the receipt")
    receipt.add_argument("--head", required=True)
    receipt.add_argument("--environment", required=True, choices=schema.ENVIRONMENTS)
    receipt.add_argument("--remote", default="origin")
    receipt.add_argument("--no-push", action="store_true")
    receipt.add_argument("--json", action="store_true", help="print the result as JSON")
    args = parser.parse_args()

    document = schema.loads_strict(Path(args.receipt).read_text(encoding="utf-8"))
    try:
        result = publish(
            args.environment,
            args.head,
            receipt_observations(document),
            remote=None if args.no_push else args.remote,
        )
    except (PublicationError, ValueError) as error:
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

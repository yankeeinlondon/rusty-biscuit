#!/usr/bin/env python3
"""Persisted execution constraints, enforced at the trigger boundary only.

An instruction such as "WSL was already run; do not run it again" outlived the
session that gave it in PR #76: a later push from a fresh terminal scheduled WSL
anyway. A constraint recorded here survives that, and `just ci-local --plan` and
`.githooks/pre-push` consult it before anything can trigger a workflow.

CI never reads this store (Design Decision 8). A constraint without qualifying
evidence must block the push; it must never make CI silently skip required
coverage.

## Notes

A record's optional `repository` is a remote URL with its scheme, credentials,
and `.git` suffix removed — `github.com/yankeeinlondon/rusty-biscuit` for this
repository — so the SSH and HTTPS spellings of one remote, and every worktree
of one clone, share an identity. Both the record and the checked remote are
normalized by [`repository_identity`]. The hook checks the remote the push
actually targets; an unknown remote matches every repository-scoped record,
because an unknown identity cannot prove exemption.

A record's optional `branch` is matched against every branch name the caller
supplies (`--branch` may repeat): the hook names the REMOTE branch an update
writes and, when the refspec renames it, the local branch too, so a record
under either name binds.

The store lives at `<home>/.rusty-biscuit/ci-constraints/<repository>/`
(Open Question 2, ruled 2026-09-12: Option B), beside the evidence directory,
with `BISCUIT_CI_CONSTRAINTS_DIR` as an override; [`default_directory`] derives
`<repository>` from the same identity. `<home>` is `Path.home()` — `HOME` on
Unix, `USERPROFILE` on native Windows, which sets no `HOME` outside Git Bash —
so a test that relocates the home must set both. The default is resolved only
by this module's commands, which only the hook and `just ci-local` run; the
planner takes the store as an explicit `--constraints` argument and has no
default, which is what keeps CI from ever reading it.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from datetime import date
from pathlib import Path
from typing import Any, Sequence


#: A constraint record's fields. `environment` and `reason` are the refusal a
#: reader sees; `owner` and `expiry` are what stop it becoming an ambient trap.
REQUIRED_FIELDS = ("environment", "reason", "owner", "expiry")
OPTIONAL_FIELDS = ("gate", "repository", "branch")

ENVIRONMENT_VARIABLE = "BISCUIT_CI_CONSTRAINTS_DIR"


#: Characters a repository identity may carry that some filesystem refuses in a
#: path segment: a port (`host:2222`), a Windows drive (`C:`), and the rest of
#: the NTFS reserved set. Each becomes `_` so one identity is one directory on
#: every OS.
_UNPORTABLE_SEGMENT_CHARACTERS = '<>:"|?*'


def store_root() -> Path:
    """`<home>/.rusty-biscuit/ci-constraints`, beside the evidence directory."""
    return Path.home() / ".rusty-biscuit" / "ci-constraints"


def store_segments(identity: str) -> list[str]:
    """A repository identity as nested directory names under the store root.

    A filesystem remote's identity is an absolute path (`/tmp/x/origin`, or
    `C/Users/x/origin` once `repository_identity` has split the drive), so the
    segments are taken relative — joining an absolute identity onto the root
    would replace the root, and a record would land outside the store.
    """
    segments = []
    for segment in identity.replace("\\", "/").split("/"):
        cleaned = "".join(
            "_" if character in _UNPORTABLE_SEGMENT_CHARACTERS or ord(character) < 32 else character
            for character in segment
        )
        if cleaned in ("", ".", ".."):
            continue
        segments.append(cleaned)
    return segments


def default_directory(repository: str = "") -> str:
    """The store directory for `repository`, a remote URL in any spelling.

    An unknown repository (empty identity) resolves to the store ROOT, and
    `load` reads recursively, so every repository's records bind: an unknown
    identity cannot prove exemption from any of them.
    """
    return str(store_root().joinpath(*store_segments(repository_identity(repository))))


def resolve_directory(repository: str = "", explicit: str = "") -> str:
    """The store to read: `explicit`, else `BISCUIT_CI_CONSTRAINTS_DIR`, else the default."""
    return explicit or os.environ.get(ENVIRONMENT_VARIABLE) or default_directory(repository)


def repository_identity(remote: str) -> str:
    """`host/path` for a remote URL in any Git spelling; unchanged if it has neither."""
    remote = remote.strip()
    if "://" in remote:
        remote = remote.split("://", 1)[1]
    elif ":" in remote and "/" not in remote.split(":", 1)[0]:
        # scp-like `git@host:owner/name`
        remote = remote.replace(":", "/", 1)
    if "@" in remote.split("/", 1)[0]:
        remote = remote.split("@", 1)[1]
    remote = remote.rstrip("/")
    if remote.endswith(".git"):
        remote = remote[: -len(".git")]
    return remote


class Constraint:
    """One recorded prohibition, valid or not."""

    def __init__(self, path: Path, document: Any) -> None:
        self.path = path
        self.document = document
        self.problem: str | None = None
        if not isinstance(document, dict):
            self.problem = "must be a JSON object"
            return
        missing = [field for field in REQUIRED_FIELDS if not document.get(field)]
        if missing:
            self.problem = f"is missing {missing}"
            return
        unknown = sorted(set(document) - set(REQUIRED_FIELDS) - set(OPTIONAL_FIELDS))
        if unknown:
            self.problem = f"has unknown field(s) {unknown}"
            return
        try:
            self.expiry = date.fromisoformat(str(document["expiry"]))
        except ValueError:
            self.problem = f"has an unparseable expiry {document['expiry']!r}"

    @property
    def environment(self) -> str:
        return str(self.document.get("environment", "")) if isinstance(self.document, dict) else ""

    @property
    def gate(self) -> str:
        return str(self.document.get("gate", "")) if isinstance(self.document, dict) else ""

    def applies_to(self, repository: str, branches: Sequence[str]) -> bool:
        """Whether this constraint governs a push to `repository` under any of `branches`.

        An unset `repository` or `branch` means "everywhere", so the common
        case — one prohibition, one branch, one host — needs neither field. An
        empty `repository` or an empty `branches` is an unknown identity, which
        cannot prove exemption either.
        """
        if not isinstance(self.document, dict):
            return True
        declared_repository = repository_identity(str(self.document.get("repository") or ""))
        actual_repository = repository_identity(repository)
        if declared_repository and actual_repository and declared_repository != actual_repository:
            return False
        declared_branch = self.document.get("branch")
        names = [name for name in branches if name]
        if declared_branch and names and declared_branch not in names:
            return False
        return True

    def expired(self, today: date) -> bool:
        return self.problem is None and self.expiry < today

    def describe(self) -> str:
        if self.problem is not None:
            return f"{self.path.name}: {self.problem}"
        return (
            f"{self.environment}"
            + (f" {self.gate}" if self.gate else "")
            + f": {self.document['reason']} "
            f"(set by {self.document['owner']}, expires {self.document['expiry']})"
        )


def load(
    directory: str,
    repository: str = "",
    branches: Sequence[str] = (),
    today: date | None = None,
):
    """Split a constraint store into the prohibitions that bind and those that do not.

    ## Returns

    `(active, expired, malformed)`. A malformed record is reported separately
    and is treated as **binding** by callers: an instruction that cannot be read
    is not an instruction that can be ignored.

    Records are read recursively: the store root holds one directory per
    repository, and a caller that could not name its repository is handed the
    root so that all of them bind.
    """
    today = today or date.today()
    active: list[Constraint] = []
    expired: list[Constraint] = []
    malformed: list[Constraint] = []
    if not directory:
        return active, expired, malformed
    root = Path(directory)
    if not root.is_dir():
        return active, expired, malformed
    for path in sorted(root.rglob("*.json")):
        try:
            document = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            constraint = Constraint(path, None)
            constraint.problem = f"is unreadable: {error}"
            malformed.append(constraint)
            continue
        constraint = Constraint(path, document)
        if constraint.problem is not None:
            malformed.append(constraint)
        elif not constraint.applies_to(repository, branches):
            continue
        elif constraint.expired(today):
            expired.append(constraint)
        else:
            active.append(constraint)
    return active, expired, malformed


def unsatisfied(constraints: list[Constraint], plan: dict[str, Any] | None) -> list[Constraint]:
    """The constraints a plan does not already satisfy.

    A constraint is satisfied when the resolved plan schedules no execution in
    its environment — because the cells are reused, governed as accepted gaps,
    or absent. A cell the planner already resolved to `prohibited` counts as an
    execution: it is the coverage a constraint left unsatisfied, not a cell that
    was satisfied. Without a plan nothing is satisfied: the fail-safe direction
    at a trigger boundary is to stop and explain, never to assume.
    """
    if plan is None:
        return list(constraints)
    executing = {
        (cell["environment"], cell["gate"])
        for cell in plan.get("cells", [])
        if cell.get("execution") == "execute" or cell.get("state") == "prohibited"
    }
    return [
        constraint
        for constraint in constraints
        if any(
            environment == constraint.environment
            and (not constraint.gate or gate == constraint.gate)
            for environment, gate in executing
        )
    ]


def prohibited_environments(constraints: list[Constraint]) -> list[str]:
    """The environment names the planner must mark prohibited, sorted."""
    return sorted({constraint.environment for constraint in constraints})


def read_plan(path: str) -> dict[str, Any]:
    """The resolved plan at `path`, or a `ValueError` naming why it cannot be trusted.

    Only the cell fields the satisfaction rule reads are checked; the planner's
    own schema is `schema.validate_resolved_plan`.
    """
    try:
        document = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read the resolved plan at {path}: {error}") from None
    cells = document.get("cells") if isinstance(document, dict) else None
    if not isinstance(cells, list) or not all(
        isinstance(cell, dict)
        and all(isinstance(cell.get(field), str) for field in ("environment", "gate", "execution"))
        for cell in cells
    ):
        raise ValueError(f"the resolved plan at {path} has no readable 'cells' list")
    return document


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    for command in ("check", "environments", "directory"):
        subparser = subparsers.add_parser(command)
        subparser.add_argument(
            "--directory",
            default="",
            help=f"the store; default ${ENVIRONMENT_VARIABLE}, else the repository's directory",
        )
        subparser.add_argument("--plan", default="", help="resolved plan JSON; unreadable blocks")
        subparser.add_argument(
            "--repository", default="", help="URL of the remote being pushed to, any spelling"
        )
        subparser.add_argument(
            "--branch",
            action="append",
            default=None,
            help="branch the push writes; repeatable, a record under any given name applies",
        )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    directory = resolve_directory(args.repository, args.directory)

    if args.command == "directory":
        # `just ci-local` hands this to the planner's `--constraints` so the
        # planner itself never resolves a default.
        print(directory)
        return

    active, expired, malformed = load(directory, args.repository, args.branch or ())

    if args.command == "environments":
        # The planner's input: every environment a recorded constraint forbids,
        # whether or not this plan would have scheduled it.
        print("\n".join(prohibited_environments(active + malformed)))
        return

    for constraint in expired:
        print(f"ci-constraints: ignoring expired constraint — {constraint.describe()}")

    plan = None
    if args.plan:
        try:
            plan = read_plan(args.plan)
        except ValueError as error:
            # An unreadable plan proves nothing, whatever the store holds.
            print(f"ci-constraints: {error}", file=sys.stderr)
            raise SystemExit(1)
    blocking = unsatisfied(active, plan) + malformed
    if not blocking:
        return
    print("", file=sys.stderr)
    print("Push blocked by a recorded execution constraint:", file=sys.stderr)
    for constraint in blocking:
        print(f"  - {constraint.describe()}", file=sys.stderr)
    print("", file=sys.stderr)
    print(
        "Review the resolved plan with 'just ci-local --plan'. Reuse qualifying "
        "evidence for that environment, or remove the constraint file once the "
        "restriction no longer applies — never work around it by pushing with "
        "--no-verify.",
        file=sys.stderr,
    )
    raise SystemExit(1)


if __name__ == "__main__":
    main()

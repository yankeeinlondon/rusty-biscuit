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
import math
import os
import stat
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


class DocumentError(ValueError):
    """A document that cannot be trusted, and the JSON path of the problem.

    `location` is a path such as `cells[2].gate`; it is empty when the problem
    is the document as a whole. Locations and validation order match the Rust
    reader in `scripts/ci-push/documents.rs`, which the shared parity fixture
    checks, and so does the message.
    """

    def __init__(self, location: str, problem: str, document: str = "") -> None:
        if location:
            message = f"{document}: `{location}` {problem}" if document else f"`{location}` {problem}"
        else:
            message = f"{document} {problem}" if document else problem
        super().__init__(message)
        self.location = location
        self.problem = problem
        self.document = document


class StoreError(ValueError):
    """A store that exists but could not be walked or read. It blocks: the
    records it hides might bind."""

    def __init__(self, path: Path, error: OSError) -> None:
        super().__init__(f"cannot read the constraint store at {path}: {error}")
        self.path = path


def _refuse_constant(name: str) -> Any:
    raise ValueError(f"{name} is not a JSON number")


def _finite_float(text: str) -> float:
    value = float(text)
    if not math.isfinite(value):
        raise ValueError(f"number out of range: {text}")
    return value


def _finite_int(text: str) -> int:
    # The Rust reader holds an integer beyond 64 bits as a float, and refuses
    # one too large for that.
    if not math.isfinite(float(text)):
        raise ValueError(f"number out of range: {text}")
    return int(text)


def _unique(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    document: dict[str, Any] = {}
    for key, value in pairs:
        if key in document:
            raise ValueError(f"duplicate key {key!r}")
        document[key] = value
    return document


def _lone_surrogate(value: Any) -> bool:
    if isinstance(value, str):
        return any(0xD800 <= ord(character) <= 0xDFFF for character in value)
    if isinstance(value, dict):
        return any(_lone_surrogate(key) or _lone_surrogate(item) for key, item in value.items())
    if isinstance(value, list):
        return any(_lone_surrogate(item) for item in value)
    return False


def loads_strict(data: bytes) -> Any:
    """Exactly one JSON value from UTF-8 `data`, or a `DocumentError`.

    Refuses what `json.loads` quietly accepts and the Rust reader does not: a
    duplicate key (the last would otherwise win), `NaN`/`Infinity`, a number
    out of float range, and an unpaired surrogate escape. Empty documents,
    trailing content, comments, and trailing commas are already refused.
    """
    try:
        document = json.loads(
            data.decode("utf-8"),
            object_pairs_hook=_unique,
            parse_constant=_refuse_constant,
            parse_float=_finite_float,
            parse_int=_finite_int,
        )
    except ValueError as error:  # JSONDecodeError and UnicodeDecodeError are ValueErrors
        raise DocumentError("", f"is not valid JSON: {error}") from None
    if _lone_surrogate(document):
        raise DocumentError("", "is not valid JSON: a string holds an unpaired surrogate")
    return document


def _kind(value: Any) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "a boolean"
    if isinstance(value, (int, float)):
        return "a number"
    if isinstance(value, str):
        return "a string"
    if isinstance(value, list):
        return "an array"
    return "an object"


def _field_path(at: str, field: str) -> str:
    return f"{at}.{field}" if at else field


def _object(value: Any, at: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise DocumentError(at, f"must be an object, found {_kind(value)}")
    return value


def _required_text(document: dict[str, Any], at: str, field: str) -> str:
    location = _field_path(at, field)
    if field not in document:
        raise DocumentError(location, "is required")
    value = document[field]
    if value == "":
        raise DocumentError(location, 'must be a nonempty string, found ""')
    if not isinstance(value, str):
        raise DocumentError(location, f"must be a nonempty string, found {_kind(value)}")
    return value


def _optional_wildcard_text(document: dict[str, Any], at: str, field: str) -> str:
    """Absence and `""` both mean "every value"; explicit null is refused."""
    if field not in document:
        return ""
    value = document[field]
    if not isinstance(value, str):
        raise DocumentError(
            _field_path(at, field), f"must be a string when present, found {_kind(value)}"
        )
    return value


_ASCII_DIGITS = frozenset("0123456789")


def _calendar_date(document: dict[str, Any], at: str, field: str) -> date:
    """Exactly `YYYY-MM-DD` naming a real day. `date.fromisoformat` (3.11+)
    also reads basic (`20261105`) and week-date (`2026-W45-1`) spellings."""
    text = _required_text(document, at, field)
    shaped = (
        len(text) == 10
        and text[4] == "-"
        and text[7] == "-"
        and all(character in _ASCII_DIGITS for index, character in enumerate(text) if index not in (4, 7))
    )
    if shaped:
        try:
            return date(int(text[0:4]), int(text[5:7]), int(text[8:10]))
        except ValueError:
            pass
    raise DocumentError(
        _field_path(at, field), f"must be a YYYY-MM-DD calendar date, found {json.dumps(text)}"
    )


def validate_record(document: Any) -> date:
    """The expiry of a valid constraint record, or a `DocumentError`.

    Unknown fields are refused: the store is written by hand, and a misspelled
    `branchh` would otherwise widen the record to every branch.
    """
    record = _object(document, "")
    unknown = sorted(set(record) - set(REQUIRED_FIELDS) - set(OPTIONAL_FIELDS))
    if unknown:
        raise DocumentError(unknown[0], "is not a constraint field")
    for field in ("environment", "reason", "owner"):
        _required_text(record, "", field)
    expiry = _calendar_date(record, "", "expiry")
    for field in OPTIONAL_FIELDS:
        _optional_wildcard_text(record, "", field)
    return expiry


class Constraint:
    """One recorded prohibition, valid or not.

    `error` is a document that could not be parsed at all; otherwise
    `document` is validated here. `problem` and `location` are set for a
    malformed record.
    """

    def __init__(self, path: Path, document: Any, error: DocumentError | None = None) -> None:
        self.path = path
        self.document = document
        self.location = ""
        self.problem: str | None = None
        if error is None:
            try:
                self.expiry = validate_record(document)
            except DocumentError as invalid:
                error = invalid
        if error is not None:
            self.location = error.location
            self.problem = error.problem

    def _text(self, field: str) -> str:
        # A malformed record still names its environment to the planner when
        # it spells one; anything else is no environment.
        value = self.document.get(field) if isinstance(self.document, dict) else None
        return value if isinstance(value, str) else ""

    @property
    def environment(self) -> str:
        return self._text("environment")

    @property
    def gate(self) -> str:
        return self._text("gate")

    def applies_to(self, repository: str, branches: Sequence[str]) -> bool:
        """Whether this constraint governs a push to `repository` under any of `branches`.

        An unset `repository` or `branch` means "everywhere", so the common
        case — one prohibition, one branch, one host — needs neither field. An
        empty `repository` or an empty `branches` is an unknown identity, which
        cannot prove exemption either. A malformed record applies everywhere.
        """
        if self.problem is not None:
            return True
        declared_repository = repository_identity(self.document.get("repository", ""))
        actual_repository = repository_identity(repository)
        if declared_repository and actual_repository and declared_repository != actual_repository:
            return False
        declared_branch = self.document.get("branch", "")
        names = [name for name in branches if name]
        if declared_branch and names and declared_branch not in names:
            return False
        return True

    def expired(self, today: date) -> bool:
        return self.problem is None and self.expiry < today

    def describe(self) -> str:
        if self.problem is not None:
            if self.location:
                return f"{self.path}: `{self.location}` {self.problem}"
            return f"{self.path} {self.problem}"
        return (
            f"{self.environment}"
            + (f" {self.gate}" if self.gate else "")
            + f": {self.document['reason']} "
            f"(set by {self.document['owner']}, expires {self.document['expiry']})"
        )


def _is_record_name(name: str) -> bool:
    # Windows filesystems are case-insensitive, and its glob matched so.
    suffix = name[-5:]
    return (suffix.lower() if os.name == "nt" else suffix) == ".json"


def _record_paths(root: Path) -> list[Path]:
    """Every `*.json` entry under `root`, recursively, without descending
    symlinked directories, ordered by path component (lowercased on Windows),
    as the Rust reader orders them. `Path.rglob` is not used: it skips a
    directory it cannot read."""
    found: list[Path] = []

    def walk(directory: str) -> None:
        try:
            with os.scandir(directory) as entries:
                for entry in entries:
                    if _is_record_name(entry.name):
                        found.append(Path(entry.path))
                    if entry.is_dir(follow_symlinks=False):
                        walk(entry.path)
        except OSError as error:
            raise StoreError(Path(directory), error) from None

    walk(str(root))

    def key(path: Path) -> tuple[str, ...]:
        parts = path.relative_to(root).parts
        return tuple(part.lower() for part in parts) if os.name == "nt" else parts

    return sorted(found, key=key)


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
    root so that all of them bind. A missing store, or one that is not a
    directory, holds none.

    ## Errors

    `StoreError` when the store exists but a directory or record cannot be
    read; skipping it could hide a record that binds.
    """
    today = today or date.today()
    active: list[Constraint] = []
    expired: list[Constraint] = []
    malformed: list[Constraint] = []
    if not directory:
        return active, expired, malformed
    root = Path(directory)
    try:
        mode = os.stat(root).st_mode
    except (FileNotFoundError, NotADirectoryError):
        return active, expired, malformed
    except OSError as error:
        raise StoreError(root, error) from None
    if not stat.S_ISDIR(mode):
        return active, expired, malformed
    for path in _record_paths(root):
        try:
            data = path.read_bytes()
        except OSError as error:
            raise StoreError(path, error) from None
        try:
            constraint = Constraint(path, loads_strict(data))
        except DocumentError as error:
            constraint = Constraint(path, None, error)
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
    at a trigger boundary is to stop and explain, never to assume. A malformed
    constraint is never satisfied.

    `plan` is what `read_plan` returned: every cell has `environment`, `gate`,
    and `execution`, and `state` only when the plan carries one.
    """
    if plan is None:
        return list(constraints)
    executing = {
        (cell["environment"], cell["gate"])
        for cell in plan["cells"]
        # An absent `state` is no prohibited-state override; it is never
        # synthesized.
        if cell["execution"] == "execute" or cell.get("state") == "prohibited"
    }
    return [
        constraint
        for constraint in constraints
        if constraint.problem is not None
        or any(
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
    own schema is `schema.validate_resolved_plan`. `cells` is a required array
    (`[]` is no cells); each cell needs nonempty `environment`, `gate`, and
    `execution`, and a `state` that is absent or a nonempty string. Other
    fields are tolerated. A `DocumentError` names the JSON path.
    """
    try:
        data = Path(path).read_bytes()
    except OSError as error:
        raise ValueError(f"cannot read the resolved plan at {path}: {error}") from None
    try:
        document = loads_strict(data)
        root = _object(document, "")
        if "cells" not in root:
            raise DocumentError("cells", "is required")
        cells = root["cells"]
        if not isinstance(cells, list):
            raise DocumentError("cells", f"must be an array, found {_kind(cells)}")
        for index, cell in enumerate(cells):
            at = f"cells[{index}]"
            cell = _object(cell, at)
            # Checked first, in the Rust reader's order, so both name the
            # same field when a cell has several problems.
            if "state" in cell:
                _required_text(cell, at, "state")
            for field in ("environment", "gate", "execution"):
                _required_text(cell, at, field)
    except DocumentError as error:
        raise DocumentError(error.location, error.problem, f"the resolved plan at {path}") from None
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

    try:
        active, expired, malformed = load(directory, args.repository, args.branch or ())
    except StoreError as error:
        # A store that cannot be read may hide a record that binds.
        print(f"ci-constraints: {error}", file=sys.stderr)
        raise SystemExit(1)

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

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

Where the store LIVES is Open Question 2 and is not yet ruled (blocker B2 in
`fixes/2026-09-11-cicd-cleanup/open-questions-and-blockers.md`). This module
therefore reads the directory named by `BISCUIT_CI_CONSTRAINTS_DIR` and has no
default: every part of the interface that holds under all three options is
implemented, and the one line that encodes the choice is [`default_directory`].
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from datetime import date
from pathlib import Path
from typing import Any


#: A constraint record's fields. `environment` and `reason` are the refusal a
#: reader sees; `owner` and `expiry` are what stop it becoming an ambient trap.
REQUIRED_FIELDS = ("environment", "reason", "owner", "expiry")
OPTIONAL_FIELDS = ("gate", "repository", "branch")

ENVIRONMENT_VARIABLE = "BISCUIT_CI_CONSTRAINTS_DIR"


def default_directory() -> str:
    """The store's location when none is named explicitly.

    Empty until Open Question 2 is ruled. The recommendation on record is a
    per-branch file under `~/.rusty-biscuit/ci-constraints/<repo>/`, beside the
    evidence directory PR #74 established; Option A (an environment variable)
    and Option C (a Git note) would fill this differently. Nothing else in this
    module depends on the answer.
    """
    return ""


def directory_from_environment() -> str:
    return os.environ.get(ENVIRONMENT_VARIABLE) or default_directory()


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

    def applies_to(self, repository: str, branch: str) -> bool:
        """Whether this constraint governs the given checkout.

        An unset `repository` or `branch` means "everywhere", so the common
        case — one prohibition, one branch, one host — needs neither field.
        """
        if not isinstance(self.document, dict):
            return True
        for field, actual in (("repository", repository), ("branch", branch)):
            declared = self.document.get(field)
            if declared and actual and declared != actual:
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


def load(directory: str, repository: str = "", branch: str = "", today: date | None = None):
    """Split a constraint store into the prohibitions that bind and those that do not.

    ## Returns

    `(active, expired, malformed)`. A malformed record is reported separately
    and is treated as **binding** by callers: an instruction that cannot be read
    is not an instruction that can be ignored.
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
    for path in sorted(root.glob("*.json")):
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
        elif not constraint.applies_to(repository, branch):
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
    or absent. Without a plan nothing is satisfied: the fail-safe direction at a
    trigger boundary is to stop and explain, never to assume.
    """
    if plan is None:
        return list(constraints)
    executing = {
        (cell["environment"], cell["gate"])
        for cell in plan.get("cells", [])
        if cell.get("execution") == "execute"
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


def read_plan(path: str | None) -> dict[str, Any] | None:
    if not path:
        return None
    try:
        document = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    return document if isinstance(document, dict) else None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    for command in ("check", "environments"):
        subparser = subparsers.add_parser(command)
        subparser.add_argument("--directory", default=directory_from_environment())
        subparser.add_argument("--plan", default="")
        subparser.add_argument("--repository", default="")
        subparser.add_argument("--branch", default="")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    active, expired, malformed = load(args.directory, args.repository, args.branch)

    if args.command == "environments":
        # The planner's input: every environment a recorded constraint forbids,
        # whether or not this plan would have scheduled it.
        print("\n".join(prohibited_environments(active + malformed)))
        return

    for constraint in expired:
        print(f"ci-constraints: ignoring expired constraint — {constraint.describe()}")

    blocking = unsatisfied(active, read_plan(args.plan)) + malformed
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

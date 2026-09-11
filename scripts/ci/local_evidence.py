#!/usr/bin/env python3
"""Record and verify exact-tree local CI evidence carried by Git notes."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from pathlib import Path
from typing import Any


SCHEMA_VERSION = 1
ENVIRONMENTS = ("ubuntu-latest", "windows-latest", "macos-latest", "wsl2-ubuntu")
NOTES_PREFIX = "refs/notes/ci-local"


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], check=True, capture_output=True, text=True, timeout=20
    ).stdout.strip()


def revision(ref: str) -> str:
    value = git("rev-parse", "--verify", ref)
    if not re.fullmatch(r"[0-9a-f]{40}", value):
        raise ValueError(f"not a full Git object ID: {value}")
    return value


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
    if git("merge-base", base, head) != revision(base):
        raise ValueError("the tested base must be an ancestor of the outgoing head")
    return json.dumps(
        expected_evidence(load_scope(scope_path), base, head, environment),
        sort_keys=True,
        separators=(",", ":"),
    )


def verified_environment(scope_path: str, base: str, head: str) -> str:
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
    for command in ("record", "verify"):
        subparser = subparsers.add_parser(command)
        subparser.add_argument("--scope", required=True)
        subparser.add_argument("--base", required=True)
        subparser.add_argument("--head", required=True)
        if command == "record":
            subparser.add_argument("--environment", required=True, choices=ENVIRONMENTS)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if args.command == "record":
        print(record(args.scope, args.base, args.head, args.environment))
    else:
        print(verified_environment(args.scope, args.base, args.head))


if __name__ == "__main__":
    main()

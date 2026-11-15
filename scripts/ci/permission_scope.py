#!/usr/bin/env python3
"""Decide whether a change needs the permission-widening comparison, and read
what the trusted checker found.

CI runs this file from a checkout of the BASE commit only (the trusted side);
the candidate commit is fetched as Git objects and never checked out, built,
or run. Both commands print `key=value` lines for `$GITHUB_OUTPUT`.

* `scope` is the cheap path check every pull request and push pays. It prints
  `decision=skip` when no input listed in `.github/ci/permission-inputs.json`
  changed, `decision=compare` when the trusted checker must compare the two
  commits, and `decision=widening` when they cannot be compared at all (the
  base has no trusted checker, or there is no base commit). A policy file
  that is a committed symbolic link or whose `permissions` is a reference
  can read any file, so then any change at all is compared.
* `decide` reads the checker's JSON report and prints
  `potential_widening=true|false`. A failed checker or an unreadable report
  exits 1, which fails the job: an error never means "no change".

User-facing guide: `docs/topics/ci-cd.md` ("Permission-widening approval").

## Examples

```bash
python3 -I scripts/ci/permission_scope.py scope --repository . \\
    --inputs .github/ci/permission-inputs.json --base "$BASE" --head "$HEAD"
python3 -I scripts/ci/permission_scope.py decide --report report.json --status 0
```
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from collections.abc import Callable
from pathlib import Path
from typing import Any

# The trusted checker: absent from a base that predates it.
CHECKER_SOURCE = "darkmatter/lib/examples/permission_widening.rs"
FULL_ID = re.compile(r"[0-9a-f]{40}")
ZERO_ID = "0" * 40
# A top-level `permissions:` whose value starts on the same line and is not a
# flow mapping: a reference (or something only the checker can judge).
REFERENCE_LINE = re.compile(r"^permissions:[ \t]*(?![ \t]*(?:#|\{|$))\S", re.MULTILINE)

Git = Callable[..., bytes]


def make_git(repository: Path) -> Git:
    """Read-only Git plumbing in `repository`, with no hooks, pager, or
    external diff, textconv, or credential helper able to run."""

    def git(*args: str) -> bytes:
        return subprocess.run(
            ["git", "-C", str(repository), "-c", "core.hooksPath=/dev/null", "--no-pager", *args],
            check=True,
            capture_output=True,
            timeout=60,
            env={**os.environ, "GIT_TERMINAL_PROMPT": "0", "GIT_CONFIG_NOSYSTEM": "1"},
        ).stdout

    return git


def load_inputs(path: Path) -> dict[str, list[str]]:
    """The input paths, grouped. Malformed or empty lists are an error."""
    document = json.loads(path.read_text(encoding="utf-8"))
    groups = {
        "policy-files": [entry["path"] for entry in document["policy-files"]],
        "default-schemas": [entry["path"] for entry in document["default-schemas"]],
        "checker-inputs": list(document["checker-inputs"]),
    }
    for name, paths in groups.items():
        if not paths or not all(isinstance(path, str) and path for path in paths):
            raise ValueError(f"`{name}` in {path} must be a non-empty list of paths")
    return groups


def changed_paths(git: Git, base: str, head: str) -> list[str]:
    output = git("diff", "--name-only", "--no-renames", "--no-ext-diff", "--no-textconv", "-z", base, head, "--")
    return [path for path in output.decode("utf-8", "surrogateescape").split("\0") if path]


def matches(path: str, inputs: list[str]) -> bool:
    return any(path.startswith(entry) if entry.endswith("/") else path == entry for entry in inputs)


def reads_beyond_itself(git: Git, commit: str, path: str) -> bool:
    """Whether the policy file at `path` in `commit` can name other files: it
    (or a parent folder) is a committed link, or its `permissions` is a
    reference. Unreadable content counts, conservatively."""
    names = path.split("/")
    for depth in range(1, len(names) + 1):
        listing = git("ls-tree", "-z", commit, "--", "/".join(names[:depth]))
        if not listing:
            return False
        if listing.startswith(b"120000 "):
            return True
    try:
        text = git("cat-file", "blob", f"{commit}:{path}").decode("utf-8")
    except (subprocess.CalledProcessError, UnicodeDecodeError):
        return True
    return REFERENCE_LINE.search(text) is not None


def scope(repository: Path, inputs_path: Path, base: str, head: str, git: Git | None = None) -> dict[str, str]:
    """The scope decision for comparing `base` with `head`."""
    if not base or base == ZERO_ID:
        return {"decision": "widening", "reason": "there is no base commit to compare with"}
    for name, value in (("base", base), ("head", head)):
        if not FULL_ID.fullmatch(value):
            raise ValueError(f"the {name} commit must be a full 40-character id, got `{value}`")
    if base == head:
        return {"decision": "skip", "reason": "the base and head commits are the same"}
    if not (repository / CHECKER_SOURCE).is_file():
        return {"decision": "widening", "reason": "the base commit has no trusted checker (bootstrap)"}
    git = git or make_git(repository)
    groups = load_inputs(inputs_path)
    changed = changed_paths(git, base, head)
    listed = groups["policy-files"] + groups["default-schemas"] + groups["checker-inputs"]
    hits = [path for path in changed if matches(path, listed)]
    if hits:
        return {"decision": "compare", "reason": f"changed inputs: {', '.join(hits[:10])}"}
    if changed and any(
        reads_beyond_itself(git, commit, path) for commit in (base, head) for path in groups["policy-files"]
    ):
        return {"decision": "compare", "reason": "a policy file may read other files, so every change is compared"}
    return {"decision": "skip", "reason": "no permission input changed"}


def decide(report_text: str, status: int) -> tuple[dict[str, str], list[str]]:
    """The outputs and the summary lines for one checker run.

    ## Errors

    `ValueError` when the checker failed or its report cannot be read.
    """
    try:
        report: Any = json.loads(report_text)
    except json.JSONDecodeError as error:
        raise ValueError(f"the checker's report is not JSON: {error}") from error
    if not isinstance(report, dict):
        raise ValueError("the checker's report is not a JSON object")
    if status != 0 or "error" in report:
        raise ValueError(f"the checker failed (status {status}): {report.get('error', 'no error given')}")
    widening = report.get("potential_widening")
    reasons = report.get("reasons")
    if not isinstance(widening, bool) or not isinstance(reasons, list) or widening != bool(reasons):
        raise ValueError("the checker's report has no consistent `potential_widening` and `reasons`")
    lines = [f"Compared `{report.get('base')}` with `{report.get('head')}`."]
    changed = report.get("changed_inputs") or []
    lines.append("Changed inputs: " + (", ".join(f"`{path}`" for path in changed) if changed else "none") + ".")
    if widening:
        lines.append("This change may widen permissions, so it needs an approval of the `permission-widening` environment:")
        lines += [f"- `{reason.get('input')}` ({reason.get('kind')}): {reason.get('detail')}" for reason in reasons]
    else:
        lines.append("No permission widens; no approval is needed.")
    return {"potential_widening": "true" if widening else "false"}, lines


def emit(outputs: dict[str, str], out: Callable[[str], None] = print) -> None:
    for key, value in outputs.items():
        out(f"{key}={value}")


def main(argv: list[str] | None = None, out: Callable[[str], None] = print) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    commands = parser.add_subparsers(dest="command", required=True)
    scoping = commands.add_parser("scope")
    scoping.add_argument("--repository", type=Path, required=True)
    scoping.add_argument("--inputs", type=Path, required=True)
    scoping.add_argument("--base", default="")
    scoping.add_argument("--head", required=True)
    deciding = commands.add_parser("decide")
    deciding.add_argument("--report", type=Path, required=True)
    deciding.add_argument("--status", type=int, required=True)
    arguments = parser.parse_args(argv)
    try:
        if arguments.command == "scope":
            emit(scope(arguments.repository, arguments.inputs, arguments.base, arguments.head), out)
            return 0
        text = arguments.report.read_text(encoding="utf-8") if arguments.report.exists() else ""
        outputs, lines = decide(text, arguments.status)
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        print(f"permission-scope: {error}", file=sys.stderr)
        return 1
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as handle:
            handle.write("## Permission widening\n\n" + "\n".join(lines) + "\n")
    print("\n".join(lines), file=sys.stderr)
    emit(outputs, out)
    return 0


if __name__ == "__main__":
    sys.exit(main())

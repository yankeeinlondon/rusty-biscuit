#!/usr/bin/env python3
"""Decide whether a change needs the permission-widening comparison, and report
what the trusted checker found.

CI runs this file from a checkout of the BASE commit only (the trusted side);
the candidate commit is fetched as Git objects and never checked out, built,
or run. Both commands print `key=value` lines for `$GITHUB_OUTPUT`.

* `scope` is the cheap path check every pull request and push pays. It prints
  `decision=skip` when no input listed in `.github/ci/permission-inputs.json`
  changed, `decision=compare` when the trusted checker must compare the two
  commits, and `decision=uncompared` when they cannot be compared at all (the
  base has no trusted checker, or there is no base commit). A policy file
  that is a committed symbolic link or whose `permissions` is a reference
  can read any file, so then any change at all is compared.
* `decide` reads the checker's JSON report and prints
  `permission_change=widens|narrows|unknown|none` on stdout, and the report
  (a Markdown section naming each entry that changed, per policy) on stderr
  and in the job summary. CI posts that section as the pull-request comment
  whenever the change is not `none`. Only the resolved policies count; a
  change to the check's own files is code review and is not reported. A
  failed checker or an unreadable report exits 1, which fails the job: an
  error never means "no change".

User-facing guide: `docs/topics/ci-cd.md` ("Permission-change report").

## Examples

```bash
python3 -I scripts/ci/permission_scope.py scope --repository . \\
    --inputs .github/ci/permission-inputs.json --base "$BASE" --head "$HEAD"
python3 -I scripts/ci/permission_scope.py decide --report report.json --status 0
```
"""

from __future__ import annotations

import argparse
import base64
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
        return {"decision": "uncompared", "reason": "there is no base commit to compare with"}
    for name, value in (("base", base), ("head", head)):
        if not FULL_ID.fullmatch(value):
            raise ValueError(f"the {name} commit must be a full 40-character id, got `{value}`")
    if base == head:
        return {"decision": "skip", "reason": "the base and head commits are the same"}
    if not (repository / CHECKER_SOURCE).is_file():
        return {"decision": "uncompared", "reason": "the base commit has no trusted checker (bootstrap)"}
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


SCOPE_NOTES = {
    "user-block-default": "The default for a user whose configuration has no `permissions` block.",
    "user-entry-defaults": "The default for each setting a user's `permissions` block leaves out.",
    "repository-block-default": "The default for a repository whose configuration has no `permissions` block.",
    "repository-entry-defaults": "The default for each setting a repository's `permissions` block leaves out.",
}
UNREADABLE = "defaults-not-understood"
ENTRY_DEFAULTS = "repository-entry-defaults"
DATA_ENVIRONMENT = "PERMISSION_REPORT_DATA"


def literal(text: str) -> str:
    """`text` as a Darkmatter literal token, so the report template shows it
    verbatim and never reads it as `{{ … }}` or `$( … )` syntax."""
    payload = base64.urlsafe_b64encode(text.encode("utf-8")).decode("ascii").rstrip("=")
    return f"{{{{!data:v1:{payload}}}}}"


def code(text: str) -> str:
    """`text` as one Markdown inline code span, whatever backticks it holds."""
    fence = "`" * (max((len(run) for run in re.findall(r"`+", text)), default=0) + 1)
    padded = f" {text} " if text.startswith("`") or text.endswith("`") or not text.strip() else text
    return literal(f"{fence}{padded}{fence}")


def _split(entries: list[str]) -> tuple[set[str], set[str]]:
    """The allows and the exclusions (without their `!`) of one entry list."""
    return {entry for entry in entries if not entry.startswith("!")}, {
        entry[1:] for entry in entries if entry.startswith("!")
    }


def _rule(rule: str) -> dict[str, str]:
    form, _, command = rule.partition(" ")
    return {"form": form, "code": code(command)}


def _entries(before: list[str], after: list[str], default: list[str] | None, item: Callable[[str], Any]) -> dict[str, Any]:
    old_allows, old_exclusions = _split(before)
    new_allows, new_exclusions = _split(after)
    changes = {
        "added": [item(entry) for entry in sorted(new_allows - old_allows)],
        "removed": [item(entry) for entry in sorted(old_allows - new_allows)],
        "excluded": [item(entry) for entry in sorted(new_exclusions - old_exclusions)],
        "unexcluded": [item(entry) for entry in sorted(old_exclusions - new_exclusions)],
    }
    return {
        "changed": any(changes.values()),
        "before_count": len(before),
        "before_default": default is not None and sorted(before) == sorted(default),
        **changes,
    }


def policy_data(policy: dict[str, Any], defaults: dict[str, Any] | None) -> dict[str, Any]:
    """One policy's changes, as the report template reads them."""
    before, after = policy.get("before") or {}, policy.get("after") or {}
    shell = _entries(
        before.get("shell-commands") or [], after.get("shell-commands") or [],
        (defaults or {}).get("shell-commands") if defaults else None, _rule,
    )
    reads = _entries(
        before.get("file-read") or [], after.get("file-read") or [],
        (defaults or {}).get("file-read") if defaults else None, code,
    )
    network = {
        "changed": before.get("network") != after.get("network"),
        "before": before.get("network") or "deny",
        "after": after.get("network") or "deny",
    }
    return {
        "title": code(str(policy.get("input"))),
        "scope_note": SCOPE_NOTES.get(str(policy.get("scope")), ""),
        "shell": shell,
        "file_read": reads,
        "network": network,
    }


def _widens(policy: dict[str, Any]) -> bool:
    return bool(
        policy["shell"]["added"] or policy["shell"]["unexcluded"]
        or policy["file_read"]["added"] or policy["file_read"]["unexcluded"]
        or (policy["network"]["changed"] and policy["network"]["after"] == "allow")
    )


def decide(report_text: str, status: int) -> tuple[dict[str, str], dict[str, Any] | None]:
    """The verdict for one checker run, and the data the report template
    renders (`None` when no committed permission changed).

    `permission_change` is `widens`, `narrows`, `unknown` (a changed default
    the trusted code cannot read), or `none`. Only the resolved policies
    count: a change to the check's own files is code review, not a change in
    what the tools may do, and is not reported.

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
    policies = report.get("policies")
    if not isinstance(widening, bool) or not isinstance(reasons, list) or widening != bool(reasons):
        raise ValueError("the checker's report has no consistent `potential_widening` and `reasons`")
    if not isinstance(policies, list):
        raise ValueError("the checker's report has no `policies`")
    defaults = next((p.get("after") for p in policies if p.get("scope") == ENTRY_DEFAULTS), None)
    changed = []
    for policy in policies:
        data = policy_data(policy, defaults if policy.get("scope") == "repository" else None)
        if data["shell"]["changed"] or data["file_read"]["changed"] or data["network"]["changed"]:
            changed.append(data)
    unreadable = [
        {"input": code(str(reason.get("input"))), "detail": literal(str(reason.get("detail")))}
        for reason in reasons
        if reason.get("kind") == UNREADABLE
    ]
    if any(_widens(policy) for policy in changed):
        verdict = "widens"
    elif unreadable:
        verdict = "unknown"
    elif changed:
        verdict = "narrows"
    else:
        return {"permission_change": "none"}, None
    sources = sorted({str(p.get("input")) for p in policies if p.get("scope") == "repository"})
    return {"permission_change": verdict}, {
        "verdict": verdict,
        "sources": [code(source) for source in sources],
        "policies": changed,
        "unreadable": unreadable,
    }


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
        outputs, data = decide(text, arguments.status)
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        print(f"permission-scope: {error}", file=sys.stderr)
        return 1
    destination = os.environ.get(DATA_ENVIRONMENT)
    if destination and data is not None:
        Path(destination).write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    print(f"Committed permissions: {outputs['permission_change']}.", file=sys.stderr)
    emit(outputs, out)
    return 0


if __name__ == "__main__":
    sys.exit(main())

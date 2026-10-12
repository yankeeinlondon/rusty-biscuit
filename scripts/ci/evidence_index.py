#!/usr/bin/env python3
"""A readable listing of the local evidence store.

The pre-push hook retains each run's JUnit reports under
`$BISCUIT_CI_EVIDENCE_DIR/<head sha>/<environment>/` (root default
`~/.rusty-biscuit/ci-evidence`). The commit SHA is the store's key: the
receipt is a git note on that commit and its `host.report_dir` names the
directory, so the layout is never renamed. This module only reads it and
joins each directory with what Git knows about the commit.

A directory whose commit this clone does not have (garbage-collected, or
pushed from another clone) is still listed, with an unknown branch and
subject.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from datetime import datetime
from pathlib import Path


def evidence_root() -> Path:
    """The store root, resolved as the pre-push hook resolves it."""
    override = os.environ.get("BISCUIT_CI_EVIDENCE_DIR", "").strip()
    return Path(override) if override else Path.home() / ".rusty-biscuit" / "ci-evidence"


def read_manifest(path: Path) -> list[dict]:
    """Manifest lines that parse; a torn line is skipped, not fatal."""
    records = []
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(record, dict):
            records.append(record)
    return records


def summarize_environment(directory: Path) -> dict | None:
    manifest = directory / "manifest.jsonl"
    if not manifest.is_file():
        return None
    records = read_manifest(manifest)
    return {
        "environment": directory.name,
        "cells": len(records),
        "failed": sum(1 for r in records if r.get("exit_code", 0) != 0),
        "missing_reports": sum(1 for r in records if r.get("report_present") is False),
        "duration_s": sum(int(r.get("duration_s") or 0) for r in records),
        "recorded": manifest.stat().st_mtime,
    }


def git_lines(args: list[str], repo: Path) -> list[str]:
    result = subprocess.run(
        ["git", *args], cwd=repo, capture_output=True, text=True, encoding="utf-8", check=False
    )
    return result.stdout.splitlines() if result.returncode == 0 else []


def commit_details(shas: list[str], repo: Path) -> dict[str, dict]:
    """Subject, commit date and nearest branch for each SHA this clone has."""
    if not shas:
        return {}
    known = []
    checks = subprocess.run(
        ["git", "cat-file", "--batch-check=%(objectname) %(objecttype)"],
        cwd=repo, input="\n".join(shas) + "\n", capture_output=True, text=True, check=False,
    ).stdout.splitlines()
    for line in checks:
        parts = line.split()
        if len(parts) == 2 and parts[1] == "commit":
            known.append(parts[0])
    if not known:
        return {}

    details: dict[str, dict] = {}
    for line in git_lines(["log", "--no-walk=unsorted", "--format=%H%x00%cI%x00%s", *known], repo):
        sha, date, subject = line.split("\x00", 2)
        details[sha] = {"date": date, "subject": subject, "branch": None}
    # `name-rev` names a commit relative to the nearest ref (`feat/x~3`), so a
    # run on a commit that has since moved behind its branch tip stays readable.
    names = git_lines(
        ["name-rev", "--name-only", "--refs=refs/heads/*", "--refs=refs/remotes/*", *known], repo
    )
    for sha, name in zip(known, names):
        if sha in details and name != "undefined":
            details[sha]["branch"] = name.removeprefix("remotes/").removeprefix("origin/")
    return details


def collect(root: Path, repo: Path) -> list[dict]:
    """One entry per evidence directory, most recently recorded first."""
    if not root.is_dir():
        return []
    entries = []
    for directory in root.iterdir():
        if not directory.is_dir():
            continue
        environments = [
            summary
            for child in sorted(directory.iterdir())
            if child.is_dir() and (summary := summarize_environment(child)) is not None
        ]
        entries.append(
            {
                "sha": directory.name,
                "path": str(directory),
                "environments": environments,
                "recorded": max((e["recorded"] for e in environments), default=directory.stat().st_mtime),
            }
        )
    details = commit_details([e["sha"] for e in entries], repo)
    for entry in entries:
        found = details.get(entry["sha"], {})
        entry["branch"] = found.get("branch")
        entry["subject"] = found.get("subject")
        entry["commit_date"] = found.get("date")
        entry["cells"] = sum(e["cells"] for e in entry["environments"])
        entry["failed"] = sum(e["failed"] for e in entry["environments"])
        entry["duration_s"] = sum(e["duration_s"] for e in entry["environments"])
        if not entry["environments"]:
            entry["result"] = "empty"
        elif entry["failed"]:
            entry["result"] = "fail"
        elif any(e["missing_reports"] for e in entry["environments"]):
            entry["result"] = "partial"
        else:
            entry["result"] = "pass"
    entries.sort(key=lambda e: e["recorded"], reverse=True)
    return entries


def format_duration(seconds: int) -> str:
    if seconds < 60:
        return f"{seconds}s"
    if seconds < 3600:
        return f"{seconds // 60}m"
    return f"{seconds // 3600}h{seconds % 3600 // 60:02d}m"


def render(entries: list[dict], root: Path, width: int) -> str:
    lines = [f"Evidence root: {root}", ""]
    if not entries:
        lines.append("No evidence recorded.")
        return "\n".join(lines)
    rows = [
        (
            datetime.fromtimestamp(e["recorded"]).strftime("%Y-%m-%d %H:%M"),
            e["sha"][:10],
            e["branch"] or "?",
            ",".join(env["environment"] for env in e["environments"]) or "-",
            str(e["cells"]),
            e["result"],
            format_duration(e["duration_s"]),
            e["subject"] or "(commit not in this clone)",
        )
        for e in entries
    ]
    header = ("RECORDED", "COMMIT", "BRANCH", "ENV", "CELLS", "RESULT", "TIME", "SUBJECT")
    widths = [max(len(header[i]), *(len(r[i]) for r in rows)) for i in range(len(header) - 1)]

    def line(cells: tuple[str, ...]) -> str:
        fixed = "  ".join(c.ljust(w) for c, w in zip(cells, widths))
        room = max(width - len(fixed) - 2, 10)
        subject = cells[-1] if len(cells[-1]) <= room else cells[-1][: room - 1] + "…"
        return f"{fixed}  {subject}".rstrip()

    lines.append(line(header))
    lines.extend(line(r) for r in rows)
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--limit", type=int, default=0, help="show only the N most recent runs")
    parser.add_argument("--json", action="store_true", help="print the listing as JSON")
    parser.add_argument("--repo", default=".", help="the clone whose Git history names the commits")
    args = parser.parse_args(argv)

    root = evidence_root()
    entries = collect(root, Path(args.repo))
    if args.limit > 0:
        entries = entries[: args.limit]
    if args.json:
        print(json.dumps({"root": str(root), "runs": entries}, indent=2))
    else:
        print(render(entries, root, shutil.get_terminal_size((120, 24)).columns))
    return 0


if __name__ == "__main__":
    sys.exit(main())

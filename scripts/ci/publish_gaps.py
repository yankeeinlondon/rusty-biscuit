#!/usr/bin/env python3
"""Publish each accepted policy gap as a `neutral` check run on the tested head.

Specification section 6 wants an accepted gap to appear IMMEDIATELY, explained,
without a test or archive build starting for it. The area rollup renders the
gap too, but only after every producer in the area has finished. This tool
runs from a job that waits on nothing: it reads the resolved plan the scope job
already published, selects the area's `accepted-gap` cells, and creates one
check run per cell through the Checks API.

Presentation only (Design Decision 10). The machine-readable `ACCEPTED GAP`
state is decided by the planner and carried in the plan and the area's result
slice; nothing here is read back by the rollup, the gate, or the retry logic.
`neutral` is Open Question 4's ruling (2026-09-12): it leaves a pull request
CLEAN and never alters the run's conclusion, where `cancelled` would mark the
PR UNSTABLE and collide with the one meaning `cancelled` already has —
interruption.

Fail closed. A cell in state `accepted-gap` whose governance record is absent,
ungoverned, incomplete, or expired is refused with exit 2: publishing `neutral`
for it would present a cell the rollup is about to BLOCK on as harmless. An
unreadable plan is exit 1. Refusal is the tool's, so the workflow never spells
a conclusion of its own.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from datetime import date
from pathlib import Path
from typing import Any

ACCEPTED_GAP_STATE = "accepted-gap"
CONCLUSION = "neutral"
MARKER = "ACCEPTED GAP"

#: Where a governed gap is declared when the cell's record does not say.
POLICY_LINK = ".github/ci/environments.json"

#: The fields a gap must carry to be published as accepted. The planner
#: (`affected_scope.gap_record`) always emits them for a governed gap; their
#: absence means the record is not one the planner produced.
REQUIRED_GOVERNANCE = ("owner", "reason", "expiry")

#: GitHub caps a check run's `name`; a longer one is rejected at the API, and a
#: truncated one would no longer identify the cell.
NAME_LIMIT = 255


class Refusal(Exception):
    """A cell that must not be published as an accepted gap, with the reason."""


def cell_label(cell: dict[str, Any]) -> str:
    return f"{cell['package']}/{cell['environment']}/{cell['gate']}"


def accepted_gap_cells(plan: dict[str, Any], area: str) -> list[dict[str, Any]]:
    """This area's cells in state `accepted-gap`, in plan order."""
    return [
        cell
        for cell in plan.get("cells", [])
        if cell.get("area") == area and cell.get("state") == ACCEPTED_GAP_STATE
    ]


def governance(cell: dict[str, Any], today: date) -> dict[str, Any]:
    """The cell's governance record, or a `Refusal` naming why it cannot publish."""
    label = cell_label(cell)
    gap = cell.get("gap")
    if not isinstance(gap, dict):
        raise Refusal(f"{label} is an accepted gap with no governance record")
    if gap.get("governed") is not True:
        raise Refusal(f"{label} is an ungoverned gap; only a governed gap is accepted")
    missing = [field for field in REQUIRED_GOVERNANCE if not gap.get(field)]
    if missing:
        raise Refusal(f"{label} is governed but its record lacks {', '.join(missing)}")
    try:
        expiry = date.fromisoformat(str(gap["expiry"]))
    except ValueError as error:
        raise Refusal(f"{label} has an unreadable expiry {gap['expiry']!r}: {error}") from error
    if expiry < today:
        raise Refusal(
            f"{label} acceptance expired on {expiry.isoformat()}; the area blocks on it"
        )
    return gap


def policy_line(source: str, environment: str, capability: str) -> int | None:
    """The 1-based line of `environments.<environment>.capabilities.<capability>`.

    A textual walk rather than a parse: `json.loads` discards positions, and
    the file is hand-written with one key per line. The search stays inside
    the environment's block — the next `"name":` ends it — so a capability the
    environment does not declare anchors nothing rather than another
    environment's entry.
    """
    lines = source.splitlines()
    inside = False
    for number, line in enumerate(lines, start=1):
        stripped = line.strip()
        if stripped.startswith('"name":'):
            inside = stripped == f'"name": "{environment}",' or stripped == f'"name": "{environment}"'
            continue
        if inside and stripped.startswith(f'"{capability}":'):
            return number
    return None


def details_url(
    server: str, repo: str, head_sha: str, policy_path: str, line: int | None
) -> str:
    """A link to the policy entry at the tested head, anchored when the line is known."""
    url = f"{server.rstrip('/')}/{repo}/blob/{head_sha}/{policy_path}"
    return f"{url}#L{line}" if line is not None else url


def check_run(
    cell: dict[str, Any],
    gap: dict[str, Any],
    *,
    head_sha: str,
    repo: str,
    server: str,
    policy_source: str | None,
) -> dict[str, Any]:
    """One `POST /repos/{repo}/check-runs` payload for an accepted-gap cell."""
    area = cell["area"]
    package = cell["package"]
    environment = cell["environment"]
    gate = cell["gate"]
    capability = gap.get("capability") or "(unnamed capability)"
    policy_path = gap.get("policy") or POLICY_LINK
    line = (
        policy_line(policy_source, environment, capability)
        if policy_source is not None
        else None
    )
    link = details_url(server, repo, head_sha, policy_path, line)
    closes = gap.get("closes")
    # `closes` names a feature DIRECTORY by convention (`features/_unscheduled/
    # <name>`), so the link is a tree; GitHub redirects a tree link to the blob
    # when the path turns out to be a file.
    tree = f"{server.rstrip('/')}/{repo}/tree/{head_sha}"

    name = f"accepted gap: {area} / {package} / {gate} ({environment})"
    if len(name) > NAME_LIMIT:
        raise Refusal(f"{cell_label(cell)} produces a check name over {NAME_LIMIT} characters")

    summary = "\n".join(
        [
            f"**{MARKER}** — no test ran for this cell, by governed policy.",
            "",
            "| field | value |",
            "|---|---|",
            f"| area | `{area}` |",
            f"| package | `{package}` |",
            f"| gate / tier | `{gate}` |",
            f"| environment | `{environment}` |",
            f"| capability | `{capability}` |",
            f"| accepted by | {gap['owner']} |",
            f"| expires | {gap['expiry']} |",
            "",
            str(gap["reason"]),
        ]
    )
    closes_text = (
        f"[`{closes}`]({tree}/{closes}) is the tracked work that ends this gap. "
        "When it lands, the environment hosts the tier and the policy entry goes."
        if closes
        else (
            "The policy entry names no tracked work (`closes`). Closing this gap "
            f"means hosting `{capability}` on `{environment}` and removing the entry."
        )
    )
    text = "\n".join(
        [
            "## Policy entry",
            "",
            f"`environments.{environment}.capabilities.{capability}` in "
            f"[`{policy_path}`]({link}), accepted by {gap['owner']} until {gap['expiry']}.",
            "",
            "## Changing or revoking this acceptance",
            "",
            "Edit that entry to change its `owner`, `expiry`, `reason`, or `closes`. "
            "Remove the entry, or set `available: true` once the environment can host "
            "the tier, to revoke it. An absent, incomplete, or expired entry is a "
            "blocking `POLICY GAP`, not an accepted one: the area's rollup fails until "
            "the coverage exists or the acceptance is renewed.",
            "",
            "## Coverage that closes this gap",
            "",
            closes_text,
            "",
            "## What this check means",
            "",
            f"Presentation only. The `{MARKER}` state was decided by the planner from "
            f"`{policy_path}` before any job ran and is carried in the resolved plan "
            f"and this area's result slice. A `{CONCLUSION}` conclusion is neither a "
            "pass nor a test failure and never alters the run's conclusion; the "
            f"area's `rollup` job owns the outcome for `{area}`.",
        ]
    )
    return {
        "name": name,
        "head_sha": head_sha,
        "status": "completed",
        "conclusion": CONCLUSION,
        "details_url": link,
        "output": {
            "title": f"{MARKER}: {package} {gate} on {environment}",
            "summary": summary,
            "text": text,
        },
    }


def payloads(
    plan: dict[str, Any],
    area: str,
    *,
    head_sha: str,
    repo: str,
    server: str,
    today: date,
    policy_source: str | None,
) -> list[dict[str, Any]]:
    """Every check run this area publishes; raises `Refusal` on the first bad cell."""
    return [
        check_run(
            cell,
            governance(cell, today),
            head_sha=head_sha,
            repo=repo,
            server=server,
            policy_source=policy_source,
        )
        for cell in accepted_gap_cells(plan, area)
    ]


def post(repo: str, payload: dict[str, Any]) -> None:
    """Create one check run through `gh api`; `GH_TOKEN` comes from the environment."""
    result = subprocess.run(
        ["gh", "api", "--method", "POST", f"repos/{repo}/check-runs", "--input", "-"],
        input=json.dumps(payload),
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"gh api POST repos/{repo}/check-runs failed for {payload['name']!r}: "
            f"{result.stderr.strip()}"
        )


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--plan", required=True, help="the resolved plan JSON")
    parser.add_argument("--area", required=True, help="the area whose accepted gaps to publish")
    parser.add_argument(
        "--head-sha",
        required=True,
        help="the commit the check runs land on; the PR head, or it never shows in the PR",
    )
    parser.add_argument(
        "--repo",
        default=os.environ.get("GITHUB_REPOSITORY", ""),
        help="owner/name; defaults to GITHUB_REPOSITORY",
    )
    parser.add_argument(
        "--server",
        default=os.environ.get("GITHUB_SERVER_URL", "https://github.com"),
        help="defaults to GITHUB_SERVER_URL",
    )
    parser.add_argument(
        "--environments",
        default=POLICY_LINK,
        help="the policy file, read only to anchor the link to the entry's line",
    )
    parser.add_argument("--today", default="", help="ISO date; defaults to today (tests)")
    parser.add_argument("--out", default="", help="write the payloads as a JSON list here")
    parser.add_argument(
        "--dry-run", action="store_true", help="build and write the payloads; call nothing"
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    if not args.repo:
        print("publish-gaps: --repo or GITHUB_REPOSITORY is required", file=sys.stderr)
        return 1
    try:
        plan = json.loads(Path(args.plan).read_text(encoding="utf-8"))
        if not isinstance(plan, dict) or not isinstance(plan.get("cells"), list):
            raise ValueError("the document has no 'cells' list")
    except (OSError, ValueError) as error:
        print(f"publish-gaps: cannot read the resolved plan {args.plan}: {error}", file=sys.stderr)
        return 1
    today = date.fromisoformat(args.today) if args.today else date.today()
    try:
        policy_source: str | None = Path(args.environments).read_text(encoding="utf-8")
    except OSError:
        policy_source = None

    try:
        runs = payloads(
            plan,
            args.area,
            head_sha=args.head_sha,
            repo=args.repo,
            server=args.server,
            today=today,
            policy_source=policy_source,
        )
    except Refusal as refusal:
        print(
            f"publish-gaps: refusing to publish a {CONCLUSION} check for area "
            f"{args.area!r}: {refusal}",
            file=sys.stderr,
        )
        return 2

    if args.out:
        Path(args.out).write_text(json.dumps(runs, indent=2) + "\n", encoding="utf-8")
    if not runs:
        print(f"publish-gaps: area {args.area!r} carries no accepted gap; nothing to publish")
        return 0
    for run in runs:
        if args.dry_run:
            print(f"publish-gaps: would publish {CONCLUSION} check {run['name']!r}")
            continue
        try:
            post(args.repo, run)
        except RuntimeError as error:
            print(f"publish-gaps: {error}", file=sys.stderr)
            return 1
        print(f"publish-gaps: published {CONCLUSION} check {run['name']!r} on {args.head_sha}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

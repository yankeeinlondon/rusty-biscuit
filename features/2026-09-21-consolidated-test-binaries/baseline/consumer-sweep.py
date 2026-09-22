#!/usr/bin/env python3
"""Phase 1 sweep: every `--test <name>` selector that names a target of the
four packages in scope, classified active vs historical.

Run from the repository root. Writes Markdown to stdout. The classification
rules are listed in the output so a reviewer can audit each decision.

`--after` (Phase 7) re-runs the sweep once the packages are migrated. The old
per-file target names are no longer Cargo targets, so they are read from the
four committed migration manifests instead, and each historical hit is listed
with its record date.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

PACKAGES = {
    "claudine-cli": "claudine/cli/",
    "darkmatter": "darkmatter/lib/",
    "darkmatter-cli": "darkmatter/cli/",
    "biscuit-terminal": "biscuit-terminal/lib/",
}
SELECTOR = re.compile(r"--test[ =]+([A-Za-z0-9_-]+)")
PACKAGE_FLAG = re.compile(r"(?:-p|--package)[ =]+([A-Za-z0-9_-]+)")
TEXT_SUFFIXES = (".md", ".rs", ".py", ".sh", ".toml", ".yml", ".yaml", ".just", "justfile")

HISTORICAL = [
    (re.compile(r"(^|/)(features|fixes)/_complete(d)?/"), "completed spec record"),
    (re.compile(r"(^|/)reviews?/"), "review record"),
    (re.compile(r"(^|/)(implementation-log|review-log|CHANGELOG)[^/]*\.md$"), "dated log"),
    (re.compile(r"(^|/)(features|fixes)/[^/]+/(review-\d+|log|results|deferred-[^/]*|phase\d+-[^/]*)\.md$"), "spec-directory record (log, review round, results)"),
    (re.compile(r"(^|/)(features|fixes)/_unscheduled/"), "unscheduled spec (not active guidance; revisit when scheduled)"),
]
IN_FLIGHT = re.compile(r"(^|/)(features|fixes)/\d{4}-\d{2}-\d{2}-[^/]+/")
FEATURE_DIR = Path(__file__).resolve().parent.parent
DATED = re.compile(r"(\d{4}-\d{2}-\d{2})-[^/]+/")


def old_targets() -> dict[str, set[str]]:
    """Old per-file target name -> owning package, from the migration manifests."""
    owners: dict[str, set[str]] = defaultdict(set)
    for package in PACKAGES:
        manifest = json.loads((FEATURE_DIR / f"{package}-migration.json").read_text())
        for module in manifest["modules"]:
            owners[module["old_target"]].add(package)
    return owners


def record_date(path: str) -> str:
    """The date a historical record carries: its dated directory, else its last commit."""
    dated = DATED.findall(path)
    if dated:
        return dated[-1]
    out = subprocess.run(
        ["git", "log", "-1", "--format=%cs", "--", path], check=True, capture_output=True, text=True,
    ).stdout.strip()
    return out or "untracked"


def metadata() -> dict:
    out = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1", "--color=never"],
        check=True, capture_output=True, text=True,
    ).stdout
    return json.loads(out)


def classify(path: str, line: str) -> tuple[str, str]:
    for pattern, why in HISTORICAL:
        if pattern.search(path):
            return "historical", why
    if path.startswith("features/2026-09-21-consolidated-test-binaries/"):
        return "self", "this feature's own records"
    if IN_FLIGHT.search(path):
        return "in-flight-spec", "active dated spec/plan; its author owns the command at landing time"
    if path.endswith(".rs"):
        if line.lstrip().startswith("//"):
            return "active", "source comment or doc in a test/source file"
        # A literal the test prints or compares: rewriting it is a test-body
        # edit, which spec §3 forbids without a ruling (see rulings.md R9).
        return "active", "**string literal in test code** (R9)"
    if path.endswith("justfile") or path.endswith(".just"):
        return "active", "recipe"
    if "/skills/" in path or path.startswith(".claude/") or path.startswith(".opencode/"):
        return "active", "agent skill"
    return "active", "documentation"


def main() -> int:
    after = "--after" in sys.argv[1:]
    meta = metadata()
    members = set(meta["workspace_members"])
    owners: dict[str, set[str]] = defaultdict(set)
    for package in meta["packages"]:
        if package["id"] not in members:
            continue
        for target in package["targets"]:
            if "test" in target["kind"]:
                owners[target["name"]].add(package["name"])
    if after:
        # Migrated packages now expose only consolidated targets; their old
        # names come from the manifests, while other packages keep theirs so
        # same-named targets elsewhere still mark a hit ambiguous.
        for names in owners.values():
            names.difference_update(PACKAGES)
        for name, packages in old_targets().items():
            owners[name] |= packages

    files = subprocess.run(["git", "ls-files"], check=True, capture_output=True, text=True).stdout.split()
    rows = []
    for path in files:
        if not path.endswith(TEXT_SUFFIXES):
            continue
        try:
            lines = open(path, encoding="utf-8").read().splitlines()
        except (UnicodeDecodeError, FileNotFoundError, IsADirectoryError):
            continue
        for number, line in enumerate(lines, 1):
            for match in SELECTOR.finditer(line):
                name = match.group(1)
                in_scope = owners.get(name, set()) & set(PACKAGES)
                if not in_scope:
                    continue
                named = set(PACKAGE_FLAG.findall(line))
                if named and not (named & in_scope):
                    continue  # explicitly another package's same-named target
                inside_owner = any(path.startswith(PACKAGES[p]) for p in in_scope)
                ambiguous = not named and len(owners[name]) > 1 and not inside_owner
                kind, why = classify(path, line)
                rows.append((kind, path, number, name, ",".join(sorted(in_scope)), why, ambiguous, line.strip()))

    by_kind: dict[str, list] = defaultdict(list)
    for row in rows:
        by_kind[row[0]].append(row)

    print("---")
    print("kind: evidence")
    print("feature: 2026-09-21-consolidated-test-binaries")
    if after:
        print("plan_phase: 7")
        print("generator: baseline/consumer-sweep.py --after")
        print("---")
        print()
        print("# `--test` consumer re-sweep (after all four migrations)")
        print()
        print("Every `--test <name>` selector in git-tracked text files whose `<name>` is an")
        print("old per-file test target of `claudine-cli`, `darkmatter`, `darkmatter-cli`, or")
        print("`biscuit-terminal`, as recorded in the four `*-migration.json` manifests.")
        print("A line that names another package with `-p`/`--package`")
    else:
        print("created: 2026-09-21")
        print("plan_phase: 1")
        print("generator: baseline/consumer-sweep.py")
        print("---")
        print()
        print("# Active `--test` consumer sweep (before any migration)")
        print()
        print("Every `--test <name>` selector in git-tracked text files whose `<name>` is a")
        print("current test target of `claudine-cli`, `darkmatter`, `darkmatter-cli`, or")
        print("`biscuit-terminal`. A line that names another package with `-p`/`--package`")
    print("is dropped. A bare name that another workspace package also uses is kept")
    print("and marked **ambiguous** for the package phase to confirm.")
    print()
    print("Classification rules, first match wins:")
    print()
    for pattern, why in HISTORICAL:
        print(f"- `{pattern.pattern}` → **historical** ({why}); never rewritten")
    print("- this feature's own directory → **self**")
    print("- other dated `features/`/`fixes/` directories → **in-flight-spec**; reviewed individually in Phase 7, and rewritten only if the spec is still unimplemented guidance")
    print("- everything else (recipes, skills, docs, prompts, source comments) → **active**; the Phase 3–7 worklist")
    print()
    print("| Class | Hits | Files |")
    print("|---|---:|---:|")
    for kind in ("active", "in-flight-spec", "self", "historical"):
        items = by_kind.get(kind, [])
        print(f"| {kind} | {len(items)} | {len({r[1] for r in items})} |")
    print()
    for kind in ("active", "in-flight-spec"):
        items = by_kind.get(kind, [])
        print(f"## {kind} ({len(items)})")
        print()
        print("| File:line | Target | Package | Why | Line |")
        print("|---|---|---|---|---|")
        for _, path, number, name, pkg, why, ambiguous, text in sorted(items, key=lambda r: (r[1], r[2])):
            flag = " **ambiguous**" if ambiguous else ""
            text = text.replace("|", "\\|")
            if len(text) > 140:
                text = text[:137] + "…"
            print(f"| `{path}:{number}` | `{name}`{flag} | {pkg} | {why} | `{text}` |")
        print()
    print("## historical (by file; never rewritten)")
    print()
    counts: dict[str, int] = defaultdict(int)
    for row in by_kind.get("historical", []):
        counts[row[1]] += 1
    if after:
        print("| File | Hits | Record date |")
        print("|---|---:|---|")
        for path in sorted(counts):
            print(f"| `{path}` | {counts[path]} | {record_date(path)} |")
    else:
        print("| File | Hits |")
        print("|---|---:|")
        for path in sorted(counts):
            print(f"| `{path}` | {counts[path]} |")
    return 0


if __name__ == "__main__":
    sys.exit(main())

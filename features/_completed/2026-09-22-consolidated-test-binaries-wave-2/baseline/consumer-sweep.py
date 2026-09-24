#!/usr/bin/env python3
"""Wave-2 baseline sweep: every reference to a current integration-test
target of the ten packages in scope, classified active vs historical.

Adapted from `2026-09-21-consolidated-test-binaries`'s `baseline/consumer-sweep.py`.
That sweep matched `--test <name>` only; this one also matches nextest
`binary(<name>)` / `binary_id(<pkg>::<name>)` filters and prose that calls a
backticked name a test binary or target.

Run from the repository root. Writes Markdown to stdout.

`--after` (Phase 6) re-runs the sweep once the packages are migrated. The old
per-file target names are no longer Cargo targets, so they are read from the
ten committed migration manifests instead, and each historical hit is listed
with its record date.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

FEATURE = "2026-09-22-consolidated-test-binaries-wave-2"
PACKAGES = {
    "tree-hugger": "tree-hugger/lib/",
    "claudine": "claudine/lib/",
    "sniff": "sniff/lib/",
    "biscuit-file": "biscuit-file/lib/",
    "schematic-gen": "schematic/gen/",
    "biscuit-terminal-cli": "biscuit-terminal/cli/",
    "claudine-gen": "claudine/gen/",
    "dmls": "darkmatter/dmls/",
    "sniff-cli": "sniff/cli/",
    "biscuit-tui-cli": "biscuit-tui/cli/",
}
NAME = r"[A-Za-z0-9_-]+"
SELECTORS = [
    ("--test", re.compile(rf"--test[ =]+({NAME})")),
    ("binary()", re.compile(rf"\bbinary\(\s*[=~]?\s*({NAME})\s*\)")),
    ("binary_id()", re.compile(rf"\bbinary_id\(\s*[=~]?\s*({NAME})::({NAME})\s*\)")),
    # A bare nextest binary id in prose, `claudine-gen::steering_check`.
    ("pkg::target", re.compile(rf"(?<![\w(:=~-])({NAME})::({NAME})\b")),
]
# Prose: a backticked name on a line that talks about a test binary/target.
PROSE_CONTEXT = re.compile(r"\b(test[- ]binar(y|ies)|test[- ]targets?|integration[- ]tests?\b|binar(y|ies)\b)", re.I)
BACKTICKED = re.compile(rf"`({NAME})`")
TEST_PATH = re.compile(rf"(?<![\w/])((?:[\w.-]+/)*)tests/({NAME})\.rs\b")
PACKAGE_FLAG = re.compile(rf"(?:-p|--package)[ =]+({NAME})")
TEXT_SUFFIXES = (".md", ".rs", ".py", ".sh", ".toml", ".yml", ".yaml", ".just", "justfile", ".json")

HISTORICAL = [
    (re.compile(r"(^|/)(features|fixes)/_complete(d)?/"), "completed spec record"),
    (re.compile(r"(^|/)reviews?/"), "review record"),
    (re.compile(r"(^|/)(implementation-log|review-log|CHANGELOG)[^/]*\.md$"), "dated log / changelog"),
    (re.compile(r"(^|/)(features|fixes)/[^/]+/(review-\d+|log|[^/]*results|deferred-[^/]*|phase\d+-[^/]*)\.md$"), "spec-directory record (log, review, results)"),
    (re.compile(r"(^|/)(docs/(superpowers/)?(plans|specs)|\.ai/plans)/\d{4}-\d{2}-\d{2}[^/]*\.md$"), "dated plan/spec record"),
    (re.compile(r"^\.claudine/memory/"), "agent memory log"),
    (re.compile(r"(^|/)(features|fixes)/[^/]+/baseline/"), "another feature's baseline evidence"),
    (re.compile(r"(^|/)(features|fixes)/_unscheduled/"), "unscheduled spec"),
    (re.compile(r"^tools/test-audit/fixtures/claudine-compat/"), "frozen replay fixture (its README forbids live edits)"),
]
IN_FLIGHT = re.compile(r"(^|/)(features|fixes)/\d{4}-\d{2}-\d{2}-[^/]+/")
FEATURE_DIR = Path(__file__).resolve().parent.parent
# `--after` only: why each remaining non-historical hit stays, keyed by
# (file, old target). A hit with no entry is printed as "undispositioned".
AFTER_DISPOSITIONS = {
    (".claude/skills/os/wsl.md", "steering_check"):
        "historical: a dated account (\"Proven non-vacuous on 2026-09-09\") of the binary as it was then",
    (".github/workflows/sniff-performance.yml", "bench_ids_sync"):
        "accurate: names the test module (`l1::bench_ids_sync`), not a selector",
    ("claudine/docs/topics/performance-testing.md", "bench_ids_sync"):
        "accurate: names the test module (`sniff`'s `l1::bench_ids_sync`), not a selector; the step's "
        "claim that it covers claudine's benches is pre-existing drift, out of scope",
    ("claudine/lib/src/stream/protocol/kimi/tests.rs", "protocol_fixture_replay"):
        "accurate: rewritten in Phase 6 to name the `l1` binary's module",
    (".claude/skills/rust-testing/SKILL.md", "windows_captured_stdout"):
        "historical: a past-tense account of the file's old `#[ignore]` era (Phase 5)",
    ("sniff/docs/cli/repo_recent-commits.md", "bench_ids_sync"):
        "historical: sample command output quoting an old commit's file list",
    ("sniff/docs/cli/repo_recent-commits.md", "uv_with_install_plan"):
        "historical: sample command output quoting an old commit's file list",
    ("sniff/docs/cli/repo_source-code-changes.md", "bench_ids_sync"):
        "historical: sample command output quoting an old commit's file list",
    ("sniff/docs/cli/repo_source-code-changes.md", "uv_with_install_plan"):
        "historical: sample command output quoting an old commit's file list",
    ("claudine/fixes/2026-07-13-rendezvous-local-ipc/plan.md", "drift"):
        "in-flight spec record (2026-07-13); its author owns the command at landing time",
    ("fixes/2026-09-22-test-input-blind-spot/spec.md", "boundary_lint"):
        "in-flight spec record (2026-09-22): a measurement taken against the old binary",
}
DATED = re.compile(r"(\d{4}-\d{2}-\d{2})-[^/]+/")


def old_targets() -> dict[str, list[str]]:
    """Package -> its old per-file target names, from the migration manifests."""
    targets: dict[str, list[str]] = {}
    for package in PACKAGES:
        manifest = json.loads((FEATURE_DIR / f"{package}-migration.json").read_text())
        targets[package] = sorted({module["old_target"] for module in manifest["modules"]})
    return targets


def record_date(path: str) -> str:
    """The date a historical record carries: its dated directory, else its last commit."""
    dated = DATED.findall(path)
    if dated:
        return dated[-1]
    out = subprocess.run(
        ["git", "log", "-1", "--format=%cs", "--", path], check=True, capture_output=True, text=True,
    ).stdout.strip()
    return out or "untracked"


def classify(path: str, line: str) -> tuple[str, str]:
    if path.startswith(f"features/{FEATURE}/"):
        return "self", "this feature's own records"
    for pattern, why in HISTORICAL:
        if pattern.search(path):
            return "historical", why
    if IN_FLIGHT.search(path):
        return "in-flight-spec", "active dated spec/plan"
    if path.endswith(".rs"):
        if line.lstrip().startswith("//"):
            return "active", "source comment"
        return "active", "**string literal in code**"
    if path.endswith("justfile") or path.endswith(".just"):
        return "active", "recipe"
    if path.startswith(".config/"):
        return "active", "nextest config"
    if path.startswith(".github/"):
        return "active", "CI workflow/config"
    if path.startswith("scripts/") or "/scripts/" in path:
        return "active", "script"
    if "/skills/" in path or path.startswith(".claude/") or path.startswith(".opencode/"):
        return "active", "agent skill"
    if path.startswith("prompts/") or "/prompts/" in path:
        return "active", "prompt"
    return "active", "documentation"


def main() -> int:
    after = "--after" in sys.argv[1:]
    meta = json.loads(subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1", "--color=never"],
        check=True, capture_output=True, text=True,
    ).stdout)
    members = set(meta["workspace_members"])
    owners: dict[str, set[str]] = defaultdict(set)
    targets: dict[str, list[str]] = {}
    pkg_area: dict[str, str] = {}
    root = meta["workspace_root"].rstrip("/") + "/"
    for package in meta["packages"]:
        if package["id"] not in members:
            continue
        names = [t["name"] for t in package["targets"] if "test" in t["kind"]]
        for name in names:
            owners[name].add(package["name"])
        pkg_area[package["name"]] = package["manifest_path"].split(root, 1)[1].split("/", 1)[0] + "/"
        if package["name"] in PACKAGES:
            targets[package["name"]] = names
    if after:
        # Migrated packages now expose only consolidated targets; their old
        # names come from the manifests, while other packages keep theirs so
        # same-named targets elsewhere still mark a hit ambiguous.
        for packages in owners.values():
            packages.difference_update(PACKAGES)
        targets = old_targets()
        for package, names in targets.items():
            for name in names:
                owners[name].add(package)

    files = subprocess.run(["git", "ls-files"], check=True, capture_output=True, text=True).stdout.split("\n")
    rows = []
    path_rows = []
    for path in files:
        if not path.endswith(TEXT_SUFFIXES):
            continue
        try:
            lines = open(path, encoding="utf-8").read().splitlines()
        except (UnicodeDecodeError, FileNotFoundError, IsADirectoryError):
            continue
        area = path.split("/", 1)[0] + "/"
        # An area's skill (`.claude/skills/sniff/`) writes paths relative to that area.
        skill = re.match(r"\.claude/skills/([^/]+)/", path)
        home = skill.group(1) + "/" if skill else area
        for number, line in enumerate(lines, 1):
            # A `-p` may sit on the preceding line of a `\`-continued command.
            context = line
            if number > 1 and lines[number - 2].rstrip().endswith("\\"):
                context = lines[number - 2] + " " + line
            named = set(PACKAGE_FLAG.findall(context))
            for match in TEST_PATH.finditer(line):
                prefix, name = match.group(1), match.group(2)
                in_scope = owners.get(name, set()) & set(PACKAGES)
                if not in_scope:
                    continue
                # Attribute by where the written path points: a full manifest
                # path, a shorter tail written from inside that package's area,
                # or a bare `tests/...` inside the package itself.
                in_scope = {
                    p for p in in_scope
                    if (prefix and prefix.endswith(PACKAGES[p]))
                    or (prefix and PACKAGES[p].endswith(prefix) and home == pkg_area[p])
                    or (not prefix and path.startswith(PACKAGES[p]))
                }
                if len(in_scope) != 1:
                    continue
                kind, why = classify(path, line)
                if kind == "active":
                    path_rows.append((path, number, name, ",".join(sorted(in_scope)), why, line.strip()))
            found: list[tuple[str, str, set[str]]] = []
            for form, pattern in SELECTORS:
                for match in pattern.finditer(line):
                    if form in ("binary_id()", "pkg::target"):
                        pkg, name = match.group(1), match.group(2)
                        if pkg in PACKAGES and name in targets.get(pkg, []):
                            found.append((form, name, {pkg}))
                        continue
                    found.append((form, match.group(1), named))
            if PROSE_CONTEXT.search(line) and not any(f == "--test" for f, _, _ in found):
                for match in BACKTICKED.finditer(line):
                    # A name several packages share (`cli`, `integration`) cannot
                    # be attributed from prose alone; it is nearly always a word.
                    if len(owners.get(match.group(1), ())) == 1:
                        found.append(("prose", match.group(1), named))
            for form, name, flags in found:
                in_scope = owners.get(name, set()) & set(PACKAGES)
                if not in_scope:
                    continue
                if flags and not (flags & in_scope):
                    continue  # explicitly another package's same-named target
                if flags:
                    in_scope &= flags
                # Inside an owner's area, a bare name means that owner unless a
                # different same-named owner shares the area.
                inside_owner = area in {pkg_area[p] for p in in_scope} and len(
                    [o for o in owners[name] if pkg_area[o] == area]) == len(
                    [p for p in in_scope if pkg_area[p] == area])
                reasons = []
                if not flags and len(owners[name]) > 1 and not inside_owner:
                    reasons.append("name shared by " + ", ".join(sorted(owners[name])))
                if form == "prose" and "_" not in name:
                    # `drift`, `pipeline`, `fixtures`: ordinary words and module names.
                    reasons.append("single-word name in prose; confirm it names the target")
                if form == "pkg::target" and "-" not in next(iter(in_scope)):
                    # `sniff::fixtures` is also a Rust path spelling.
                    reasons.append("also a Rust path spelling; confirm it is a binary id")
                kind, why = classify(path, line)
                rows.append((kind, path, number, name, ",".join(sorted(in_scope)), form, why, "; ".join(reasons), line.strip()))

    by_kind: dict[str, list] = defaultdict(list)
    for row in rows:
        by_kind[row[0]].append(row)
    rev = subprocess.run(["git", "rev-parse", "HEAD"], check=True, capture_output=True, text=True).stdout.strip()

    print("---")
    print("kind: evidence" if after else "kind: baseline")
    print(f"feature: {FEATURE}")
    print("created: 2026-09-23")
    print(f"rev: {rev}")
    if after:
        print("plan_phase: 6")
    print("generator: baseline/consumer-sweep.py" + (" --after" if after else ""))
    print("---")
    print()
    if after:
        print("# Test-selector consumer re-sweep (after all ten migrations)")
        print()
        print("Every reference, in git-tracked text files, to an old per-file")
        print("integration-test target of the ten wave-2 packages, as recorded in the ten")
        print("`*-migration.json` manifests, in one of these forms:")
    else:
        print("# Test-selector consumer sweep (before any wave-2 migration)")
        print()
        print("Every reference, in git-tracked text files, to a current integration-test")
        print("target of the ten wave-2 packages, in one of these forms:")
    print()
    print("- `--test <name>` (cargo test, cargo nextest, cargo check, just pass-through)")
    print("- nextest `binary(<name>)` (also `=`/`~` matchers)")
    print("- nextest `binary_id(<package>::<name>)`")
    print("- nextest binary id in prose, `<package>::<name>`")
    print("- prose: a backticked `<name>` on a line that says test binary / test target / integration test / binary")
    print()
    print("A line whose `-p`/`--package` (on the line or the `\\`-continued line above)")
    print("names another package is dropped. A bare name that another workspace package")
    print("also uses is kept and listed under **ambiguous** unless the file sits in the")
    print("owning package's area. Prose naming a single-word target (`drift`,")
    print("`pipeline`, `fixtures`, `about`, `tty`, ...) and a bare `<package>::<name>`")
    print("for a dash-free package (`claudine::x` is also a Rust path) are listed under")
    print("**ambiguous**. Prose naming a name several packages share (`cli`,")
    print("`integration`) is not matched at all.")
    print()
    print("Classification rules, first match wins:")
    print()
    print("- this feature's own directory → **self**")
    for pattern, why in HISTORICAL:
        print(f"- `{pattern.pattern}` → **historical** ({why}); never rewritten")
    print("- other dated `features/`/`fixes/` directories → **in-flight-spec**; reviewed at landing")
    print("- everything else (recipes, skills, docs, prompts, config, scripts, source comments) → **active**; the migration worklist")
    print()
    print("## Old targets in scope (from the migration manifests)" if after else "## Targets in scope")
    print()
    print("| Package | Manifest | Targets | Names |")
    print("|---|---|---:|---|")
    for pkg, path in PACKAGES.items():
        names = targets.get(pkg, [])
        print(f"| `{pkg}` | `{path.rstrip('/')}` | {len(names)} | {', '.join(f'`{n}`' for n in names)} |")
    print()

    def is_ambiguous(row) -> bool:
        return bool(row[7])

    print("| Class | Confirmed hits | Ambiguous hits | Files |")
    print("|---|---:|---:|---:|")
    for kind in ("active", "in-flight-spec", "self", "historical"):
        items = by_kind.get(kind, [])
        amb = sum(1 for r in items if is_ambiguous(r))
        print(f"| {kind} | {len(items) - amb} | {amb} | {len({r[1] for r in items})} |")
    print()

    active = [r for r in by_kind.get("active", []) if not is_ambiguous(r)]
    print("### Active confirmed hits per package")
    print()
    print("| Package | Hits |")
    print("|---|---:|")
    per_pkg: dict[str, int] = defaultdict(int)
    for r in active:
        for p in r[4].split(","):
            per_pkg[p] += 1
    for pkg in PACKAGES:
        print(f"| `{pkg}` | {per_pkg.get(pkg, 0)} |")
    print()

    def disposition(path: str, name: str) -> str:
        return AFTER_DISPOSITIONS.get((path, name), "**undispositioned**")

    def table(title: str, items: list, with_note: bool) -> None:
        print(f"## {title} ({len(items)})")
        print()
        if not items:
            print("None.")
            print()
            return
        head = "| File:line | Target | Package | Form | Why |" + (" Note |" if with_note else "") + (" Disposition |" if after else "") + " Line |"
        print(head)
        print("|" + "---|" * (head.count("|") - 1))
        for _, path, number, name, pkg, form, why, note, text in sorted(items, key=lambda r: (r[1], r[2])):
            text = text.replace("|", "\\|")
            if len(text) > 140:
                text = text[:137] + "…"
            cells = [f"`{path}:{number}`", f"`{name}`", pkg, form, why] + ([note] if with_note else []) + ([disposition(path, name)] if after else []) + [f"`{text}`"]
            print("| " + " | ".join(cells) + " |")
        print()

    table("active — confirmed", active, False)
    table("active — ambiguous (confirm before rewriting)", [r for r in by_kind.get("active", []) if is_ambiguous(r)], True)
    table("in-flight-spec", by_kind.get("in-flight-spec", []), True)
    if after:
        # Old names in this feature's own evidence are the before-side record.
        print(f"## self ({len(by_kind.get('self', []))}, by file)")
        print()
        self_counts: dict[str, int] = defaultdict(int)
        for row in by_kind.get("self", []):
            self_counts[row[1]] += 1
        print("| File | Hits |")
        print("|---|---:|")
        for path in sorted(self_counts):
            print(f"| `{path}` | {self_counts[path]} |")
        print()
    else:
        table("self", by_kind.get("self", []), True)

    print(f"## active — file-path references (informational, {len(path_rows)})")
    print()
    print("Not selectors: `tests/<name>.rs` paths in active files. They go stale when")
    print("the file moves under a consolidated binary's directory, so they belong on the")
    print("same worklist, but they are not counted in the selector totals above.")
    print()
    if path_rows:
        print("| File:line | Target | Package | Why |" + (" Disposition |" if after else "") + " Line |")
        print("|---|---|---|---|" + ("---|" if after else "") + "---|")
        for path, number, name, pkg, why, text in sorted(path_rows):
            text = text.replace("|", "\\|")
            if len(text) > 140:
                text = text[:137] + "…"
            extra = f" {disposition(path, name)} |" if after else ""
            print(f"| `{path}:{number}` | `{name}` | {pkg} | {why} |{extra} `{text}` |")
    else:
        print("None.")
    print()
    if after:
        print("## historical (by file; never rewritten)")
        print()
        counts_after: dict[str, int] = defaultdict(int)
        for row in by_kind.get("historical", []):
            counts_after[row[1]] += 1
        if not counts_after:
            print("None.")
        else:
            print("| File | Hits | Record date |")
            print("|---|---:|---|")
            for path in sorted(counts_after):
                print(f"| `{path}` | {counts_after[path]} | {record_date(path)} |")
        return 0
    print("## Reviewer notes")
    print()
    print("- `tools/test-toolkit/tests/ci_workflow_contracts.rs` reads")
    print("  `biscuit-tui/cli/tests/windows_captured_stdout.rs` at run time; moving that")
    print("  file breaks a test in another package, not just a comment.")
    print("- `.claude/skills/rust-testing/SKILL.md:777` names `cli/tests/spawn_site_guard.rs`")
    print("  \"in sniff\"; the path rule cannot attribute a bare `cli/` prefix from a")
    print("  cross-area skill, so it is listed here instead of in the table above.")
    print("- `tools/test-audit/fixtures/claudine-compat/families.json` holds sixteen")
    print("  `claudine::<target>` suite ids. It is a frozen replay fixture whose README")
    print("  forbids live edits, so it is classified historical.")
    print("- Selector-free consumers were checked and hold no target names:")
    print("  `.config/nextest.toml` filters these packages only by `package(...)` and")
    print("  tier-prefix `test(/(^|::)level2_/)` patterns, which survive module nesting;")
    print("  no justfile under `just/` and no `.github/` workflow names a target with")
    print("  `binary(...)` or `binary_id(...)`.")
    print()
    print("## historical (by file; never rewritten)")
    print()
    counts: dict[str, int] = defaultdict(int)
    for row in by_kind.get("historical", []):
        counts[row[1]] += 1
    if not counts:
        print("None.")
    else:
        print("| File | Hits |")
        print("|---|---:|")
        for path in sorted(counts):
            print(f"| `{path}` | {counts[path]} |")
    return 0


if __name__ == "__main__":
    sys.exit(main())

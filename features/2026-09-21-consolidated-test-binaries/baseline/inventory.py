#!/usr/bin/env python3
"""Phase 1 baseline inventory of every integration-test target in scope.

Merges three sources, because none is sufficient alone (spec §1):

- `cargo metadata --no-deps`: the targets Cargo actually builds, with their
  source path, `required-features`, and edition;
- each package `Cargo.toml`: the `[[test]]` keys metadata does not report
  (`harness`, `test`, `doctest`, `bench`, and whether the entry exists at all);
- the before-listings in `baseline/listings/`: which tier(s) each target's
  tests are selected into today, per feature set.

Evidence tool only. Phase 2's `consolidation.py inventory` supersedes it.

Usage (repository root): inventory.py <listings-dir> <out.json> <out.md>
"""

from __future__ import annotations

import gzip
import json
import re
import subprocess
import sys
import tomllib
from collections import defaultdict
from pathlib import Path

PACKAGES = ("claudine-cli", "darkmatter", "darkmatter-cli", "biscuit-terminal")
MARKERS = ("level2_", "level3_", "browser_", "real_", "slow_")
# `_tier_filter` adds `perf_` to the L1 exclusion for worktree-cli only.
PACKAGE_MARKERS = {"worktree-cli": ("perf_",)}
TEST_FN = re.compile(r"#\[test\][^\n]*\n(?:\s*#\[[^\n]*\n)*\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)")
# Keys Cargo honors on a [[test]] entry. Any key other than name/path is a
# candidate target-wide setting (R3).
TARGET_WIDE_KEYS = ("harness", "required-features", "edition", "test", "doctest", "bench", "doc", "proc-macro", "crate-type")
TIERS = ("L1", "L2", "L3", "browser", "real")
INNER_CFG = re.compile(r"^\s*#!\[\s*cfg\s*\((.*)\)\s*\]\s*$")
MOD_COMMON = re.compile(r"^[ \t]*(pub(\([a-z]+\))?[ \t]+)?mod[ \t]+common[ \t]*;", re.M)


def run_metadata(repo: Path) -> dict:
    out = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1", "--color=never"],
        cwd=repo, check=True, capture_output=True, text=True,
    ).stdout
    return json.loads(out)


def name_marker(package: str, name: str) -> str | None:
    for marker in MARKERS + PACKAGE_MARKERS.get(package, ()):
        if name.startswith(marker):
            return marker
    return None


def load_listings(listings: Path) -> dict:
    """{package: {feature_set: {tier: {binary_id: {test: verdict}}}}}"""
    found: dict = defaultdict(lambda: defaultdict(dict))
    for path in sorted(listings.glob("*__*__*__*.json*")):
        stem = path.name.removesuffix(".gz").removesuffix(".json")
        _host, package, feature_set, tier = stem.split("__")
        text = gzip.decompress(path.read_bytes()).decode() if path.suffix == ".gz" else path.read_text()
        if not text.strip():
            continue
        document = json.loads(text)
        suites = {}
        for binary_id, suite in document["rust-suites"].items():
            if suite.get("kind") != "test":
                continue
            suites[binary_id] = {
                test: ("ignored" if case["ignored"] else case["filter-match"]["status"])
                for test, case in suite["testcases"].items()
            }
        found[package][feature_set][tier] = suites
    return found


def main() -> int:
    repo = Path.cwd()
    listings_dir, out_json, out_md = (Path(arg) for arg in sys.argv[1:4])
    meta = run_metadata(repo)
    members = set(meta["workspace_members"])
    listings = load_listings(listings_dir)

    workspace_test_targets = 0
    packages_with_tests = 0
    for package in meta["packages"]:
        if package["id"] not in members:
            continue
        count = sum(1 for target in package["targets"] if "test" in target["kind"])
        workspace_test_targets += count
        packages_with_tests += 1 if count else 0

    inventory: dict = {
        "generator": "features/2026-09-21-consolidated-test-binaries/baseline/inventory.py",
        "workspace": {
            "workspace_members": len(members),
            "packages_with_test_targets": packages_with_tests,
            "integration_test_targets": workspace_test_targets,
        },
        "packages": {},
    }

    for package in meta["packages"]:
        if package["name"] not in PACKAGES or package["id"] not in members:
            continue
        manifest_path = Path(package["manifest_path"])
        crate_dir = manifest_path.parent
        manifest = tomllib.loads(manifest_path.read_text())
        declared = {entry["name"]: entry for entry in manifest.get("test", [])}
        tests_dir = crate_dir / "tests"

        targets = []
        for target in sorted(package["targets"], key=lambda t: t["name"]):
            if "test" not in target["kind"]:
                continue
            source = Path(target["src_path"])
            text = source.read_text(encoding="utf-8", errors="replace")
            entry = declared.get(target["name"], {})
            extra_keys = sorted(key for key in entry if key not in ("name", "path"))
            meta_features = sorted(target.get("required-features", []))
            manifest_features = sorted(entry.get("required-features", []))
            inner_cfgs = [m.group(1) for line in text.splitlines() if (m := INNER_CFG.match(line))]
            binary_id = f"{package['name']}::{target['name']}"

            # Tier placement today: under every captured feature set that
            # built this target, which tier expressions select which tests.
            tiers_by_feature_set: dict[str, dict[str, int]] = {}
            test_names: set[str] = set()
            ignored: set[str] = set()
            for feature_set, by_tier in sorted(listings.get(package["name"], {}).items()):
                if binary_id not in by_tier.get("L1", {}):
                    continue
                counts = {}
                for tier in TIERS:
                    cases = by_tier.get(tier, {}).get(binary_id, {})
                    counts[tier] = sum(1 for verdict in cases.values() if verdict == "matches")
                    test_names.update(cases)
                    ignored.update(test for test, verdict in cases.items() if verdict == "ignored")
                tiers_by_feature_set[feature_set] = counts

            marker = name_marker(package["name"], target["name"])
            # A marker-carrying name is safe as a module name only if every
            # test in the target already carries the same marker (S1 §2);
            # otherwise the module segment would move tests between tiers.
            alias_required = None
            alias_basis = None
            if marker is not None:
                # A target this host cannot list (platform-absent) falls back
                # to its source's `#[test]` names; on-host listings decide.
                names = test_names or set(TEST_FN.findall(text))
                alias_basis = "macos-listing" if test_names else "source-scan (platform-absent on macOS; confirm from on-host listing)"
                if names:
                    alias_required = any(not re.search(rf"(^|::){marker}", test) for test in names)

            targets.append({
                "name": target["name"],
                "src_path": str(source.relative_to(repo)),
                "nested_root": source.parent != tests_dir,
                "declared_in_manifest": target["name"] in declared,
                "required_features": meta_features,
                "required_features_manifest": manifest_features,
                "required_features_agree": meta_features == manifest_features or not declared.get(target["name"]),
                "harness": entry.get("harness", True),
                "edition": target.get("edition"),
                "target_wide_keys": extra_keys,
                "name_marker": marker,
                "module_name_alias_required": alias_required,
                "module_name_alias_basis": alias_basis,
                "inner_cfg": inner_cfgs,
                "declares_mod_common": bool(MOD_COMMON.search(text)),
                "listed_tests": len(test_names),
                "ignored_tests": len(ignored),
                "selected_by_tier": tiers_by_feature_set,
            })

        # Undeclared crate roots: a `tests/<dir>/main.rs` Cargo would also
        # auto-discover, reported whether or not metadata lists it.
        nested_roots = sorted(str(p.relative_to(repo)) for p in tests_dir.glob("*/main.rs"))
        inventory["packages"][package["name"]] = {
            "manifest": str(manifest_path.relative_to(repo)),
            "autotests": manifest.get("package", {}).get("autotests", True),
            "features": sorted(manifest.get("features", {})),
            "ci_tests": manifest.get("package", {}).get("metadata", {}).get("ci", {}).get("tests", {}),
            "top_level_test_files": len(list(tests_dir.glob("*.rs"))),
            "nested_main_rs": nested_roots,
            "test_targets": targets,
            "bench_targets": [
                {"name": b["name"], "harness": b.get("harness", True)} for b in manifest.get("bench", [])
            ],
        }

    Path(out_json).write_text(json.dumps(inventory, indent=2, sort_keys=True) + "\n")
    Path(out_md).write_text(render_markdown(inventory))
    return 0


def contract_key(target: dict) -> str:
    """Execution-contract key per spec §1 (tier decided per test, not per target)."""
    features = ",".join(target["required_features"]) or "-"
    harness = "std" if target["harness"] else "custom"
    wide = ",".join(k for k in target["target_wide_keys"] if k != "required-features") or "-"
    return f"features={features} harness={harness} other-keys={wide}"


def render_markdown(inventory: dict) -> str:
    lines = [
        "---",
        "kind: evidence",
        "feature: 2026-09-21-consolidated-test-binaries",
        "created: 2026-09-21",
        "plan_phase: 1",
        "generator: baseline/inventory.py",
        "data: baseline/inventory.json",
        "---",
        "",
        "# Baseline test-target inventory (before any migration)",
        "",
        "Generated from `cargo metadata --no-deps`, each package `Cargo.toml`, and",
        "the macOS before-listings in `baseline/listings/`. `inventory.json` is the",
        "complete record and the future migration manifest's `before` side. This",
        "page summarizes it.",
        "",
        "## Workspace context",
        "",
        "| Measure | Value |",
        "|---|---:|",
    ]
    ws = inventory["workspace"]
    lines += [
        f"| Workspace members (`cargo metadata`) | {ws['workspace_members']} |",
        f"| Members with ≥1 integration-test target | {ws['packages_with_test_targets']} |",
        f"| Integration-test targets | {ws['integration_test_targets']} |",
        "",
        "## Per package",
        "",
        "| Package | Top-level `tests/*.rs` | Test targets | Nested `main.rs` roots | Declared `[[test]]` | `mod common;` files | Harness ≠ std | Target-wide keys beyond `required-features` | `[[bench]]` (harness=false) |",
        "|---|---:|---:|---|---:|---:|---:|---|---:|",
    ]
    for name, package in inventory["packages"].items():
        targets = package["test_targets"]
        custom = sum(1 for t in targets if not t["harness"])
        wide = sorted({k for t in targets for k in t["target_wide_keys"] if k != "required-features"})
        benches = sum(1 for b in package["bench_targets"] if not b["harness"])
        lines.append(
            f"| `{name}` | {package['top_level_test_files']} | {len(targets)} | "
            f"{', '.join(f'`{p}`' for p in package['nested_main_rs']) or '—'} | "
            f"{sum(1 for t in targets if t['declared_in_manifest'])} | "
            f"{sum(1 for t in targets if t['declares_mod_common'])} | {custom} | "
            f"{', '.join(wide) or 'none'} | {benches} |"
        )
    lines += ["", "## Execution contracts (required features × harness × other target-wide keys)", ""]
    for name, package in inventory["packages"].items():
        groups: dict[str, list[str]] = defaultdict(list)
        for target in package["test_targets"]:
            groups[contract_key(target)].append(target["name"])
        lines.append(f"### `{name}`")
        lines.append("")
        lines.append("| Contract | Targets | Name markers present |")
        lines.append("|---|---:|---|")
        for key, names in sorted(groups.items()):
            markers = defaultdict(int)
            for target in package["test_targets"]:
                if target["name"] in names:
                    markers[target["name_marker"] or "(none)"] += 1
            summary = ", ".join(f"`{m}`×{c}" for m, c in sorted(markers.items()))
            lines.append(f"| `{key}` | {len(names)} | {summary} |")
        lines.append("")

    lines += [
        "## Tier placement today (macOS listing, widest captured feature set per target)",
        "",
        "Counts of tests selected by each canonical tier expression. A target",
        "that appears in more than one tier column mixes tiers inside one file;",
        "consolidation keeps that because selection is per test.",
        "",
    ]
    for name, package in inventory["packages"].items():
        mixed = []
        for target in package["test_targets"]:
            by_fs = target["selected_by_tier"]
            if not by_fs:
                continue
            widest = max(by_fs.items(), key=lambda kv: (sum(kv[1].values()), kv[0]))[1]
            tiers = [t for t, c in widest.items() if c]
            if len(tiers) > 1:
                mixed.append(f"`{target['name']}` ({', '.join(f'{t}:{widest[t]}' for t in tiers)})")
        unlisted = [t["name"] for t in package["test_targets"] if not t["selected_by_tier"]]
        lines.append(f"- **`{name}`**: mixed-tier targets: {', '.join(mixed) or 'none'}. "
                     f"Targets never built by a captured feature set: {', '.join(f'`{u}`' for u in unlisted) or 'none'}.")
    lines += [
        "",
        "## Feature-gated targets: tiers their tests select into",
        "",
        "A consolidated target is keyed by `required-features`; tier selection",
        "stays per test. A gated target whose tests select into a tier its",
        "name does not announce keeps that placement under an alias (R2).",
        "",
        "| Package | Target | Required features | Selected per tier (feature set that builds it) |",
        "|---|---|---|---|",
    ]
    for name, package in inventory["packages"].items():
        for target in package["test_targets"]:
            if not target["required_features"]:
                continue
            by_fs = target["selected_by_tier"]
            widest = max(by_fs.items(), key=lambda kv: (sum(kv[1].values()), kv[0])) if by_fs else None
            placed = ", ".join(f"{t}:{c}" for t, c in widest[1].items() if c) if widest else "platform-absent on macOS"
            placed = placed or "none selected on macOS (tests cfg-absent here, or all ignored)"
            lines.append(f"| `{name}` | `{target['name']}` | `{','.join(target['required_features'])}` | {placed} |")
    lines += [
        "",
        "## Module-name hazard (R2 neutral-alias rule, S1 §2)",
        "",
        "A target whose **name** carries a tier marker keeps its tests' tiers as a",
        "module name only if every one of its tests already carries that marker",
        "in its own path. Targets whose name has no marker are always safe.",
        "",
        "| Package | Marker-named targets | Alias required (a test lacks the marker) | Undetermined |",
        "|---|---:|---|---:|",
    ]
    for name, package in inventory["packages"].items():
        marked = [t for t in package["test_targets"] if t["name_marker"]]
        needs = [t["name"] for t in marked if t["module_name_alias_required"]]
        unknown = sum(1 for t in marked if t["module_name_alias_required"] is None)
        provisional = [t["name"] for t in marked if (t["module_name_alias_basis"] or "").startswith("source-scan")]
        note = f" (source-scan only: {', '.join(f'`{n}`' for n in provisional)})" if provisional else ""
        lines.append(f"| `{name}` | {len(marked)} | {', '.join(f'`{n}`' for n in needs) or 'none'} | {unknown}{note} |")
    lines += [
        "",
        "## Inner `#![cfg]` crate conditions (must become module-declaration conditions)",
        "",
    ]
    for name, package in inventory["packages"].items():
        cfgs: dict[str, list[str]] = defaultdict(list)
        for target in package["test_targets"]:
            for cfg in target["inner_cfg"]:
                cfgs[cfg].append(target["name"])
        rendered = "; ".join(f"`cfg({c})`×{len(v)}" for c, v in sorted(cfgs.items())) or "none"
        lines.append(f"- **`{name}`**: {rendered}")
    lines.append("")
    return "\n".join(lines)


if __name__ == "__main__":
    raise SystemExit(main())

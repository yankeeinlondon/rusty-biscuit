#!/usr/bin/env python3
"""Calculate dependency-aware CI scope for the Rusty Biscuit workspace."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
AREA_CONFIG = ROOT / ".github" / "ci" / "areas.json"
GLOBAL_PATHS = {
    ".config/nextest.toml",
    ".github/ci/areas.json",
    ".github/workflows/_area-ci.yml",
    ".github/workflows/ci.yml",
    "Cargo.lock",
    "Cargo.toml",
    "justfile",
    "rust-toolchain.toml",
    "scripts/ci/affected_scope.py",
}
GLOBAL_PREFIXES = (".cargo/", "just/")


def load_metadata(root: Path) -> dict[str, Any]:
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


def load_area_config(path: Path) -> list[dict[str, Any]]:
    with path.open(encoding="utf-8") as config_file:
        areas = json.load(config_file)

    defaults = {
        "full_os": ["ubuntu-latest", "windows-latest"],
        "check_os": ["macos-latest"],
        "shards": ["1/1"],
        "soft_os": ["windows-latest"],
        "l2": False,
        "browser": False,
        "kache": True,
        "ai_provider_stubs": False,
    }
    return [{**defaults, **area} for area in areas]


def curated_areas(root: Path) -> list[str]:
    justfile = (root / "justfile").read_text(encoding="utf-8")
    match = re.search(r'^areas := "([^"]+)"$', justfile, re.MULTILINE)
    if match is None:
        raise RuntimeError("unable to read the curated area list from justfile")
    return match.group(1).split()


def validate_area_config(root: Path, areas: list[dict[str, Any]]) -> None:
    configured = [area["area"] for area in areas]
    expected = curated_areas(root)
    if configured != expected:
        raise RuntimeError(
            "CI area configuration differs from the root justfile:\n"
            f"configured={configured}\nexpected={expected}"
        )


def workspace_packages(metadata: dict[str, Any]) -> dict[str, dict[str, Any]]:
    members = set(metadata["workspace_members"])
    return {
        package["id"]: package
        for package in metadata["packages"]
        if package["id"] in members
    }


def package_directories(
    root: Path, packages: dict[str, dict[str, Any]]
) -> list[tuple[PurePosixPath, str]]:
    directories = []
    for package_id, package in packages.items():
        manifest = Path(package["manifest_path"]).resolve()
        relative = manifest.parent.relative_to(root.resolve())
        directories.append((PurePosixPath(relative.as_posix()), package_id))
    return sorted(directories, key=lambda item: len(item[0].parts), reverse=True)


def owner_area(package: dict[str, Any], root: Path, area_names: set[str]) -> str | None:
    manifest = Path(package["manifest_path"]).resolve()
    relative = manifest.relative_to(root.resolve())
    top_level = relative.parts[0]
    return top_level if top_level in area_names else None


def changed_package_ids(
    files: list[str],
    root: Path,
    packages: dict[str, dict[str, Any]],
    area_names: set[str],
) -> tuple[set[str], set[str], bool]:
    directories = package_directories(root, packages)
    seeds: set[str] = set()
    explicit_areas: set[str] = set()

    for raw_file in files:
        normalized = raw_file.replace("\\", "/").removeprefix("./")
        if normalized in GLOBAL_PATHS or normalized.startswith(GLOBAL_PREFIXES):
            return set(packages), area_names, True

        changed = PurePosixPath(normalized)
        matched_package = False
        for directory, package_id in directories:
            if changed == directory or directory in changed.parents:
                seeds.add(package_id)
                matched_package = True
                break

        if matched_package or not changed.parts:
            continue

        top_level = changed.parts[0]
        if top_level in area_names:
            explicit_areas.add(top_level)
            for package_id, package in packages.items():
                if owner_area(package, root, area_names) == top_level:
                    seeds.add(package_id)

    return seeds, explicit_areas, False


def dependent_closure(
    seeds: set[str], metadata: dict[str, Any], packages: dict[str, dict[str, Any]]
) -> set[str]:
    reverse_dependencies: dict[str, set[str]] = {
        package_id: set() for package_id in packages
    }
    for node in metadata["resolve"]["nodes"]:
        if node["id"] not in packages:
            continue
        for dependency in node.get("deps", []):
            dependency_id = dependency["pkg"]
            if dependency_id in reverse_dependencies:
                reverse_dependencies[dependency_id].add(node["id"])

    affected = set(seeds)
    pending = list(seeds)
    while pending:
        package_id = pending.pop()
        for dependent in reverse_dependencies.get(package_id, set()):
            if dependent not in affected:
                affected.add(dependent)
                pending.append(dependent)
    return affected


def calculate_scope(
    files: list[str],
    root: Path,
    metadata: dict[str, Any],
    area_config: list[dict[str, Any]],
    force_all: bool = False,
) -> dict[str, Any]:
    packages = workspace_packages(metadata)
    area_names = {area["area"] for area in area_config}

    if force_all:
        affected_ids = set(packages)
        affected_areas = area_names
        full_scope = True
    else:
        seeds, explicit_areas, full_scope = changed_package_ids(
            files, root, packages, area_names
        )
        affected_ids = dependent_closure(seeds, metadata, packages)
        affected_areas = set(explicit_areas)
        for package_id in affected_ids:
            area = owner_area(packages[package_id], root, area_names)
            if area is not None:
                affected_areas.add(area)

    selected_areas = [
        area for area in area_config if area["area"] in affected_areas
    ]
    selected_packages = sorted(packages[package_id]["name"] for package_id in affected_ids)
    return {
        "areas": selected_areas,
        "packages": selected_packages,
        "full_scope": full_scope,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--all", action="store_true", help="select the full workspace")
    parser.add_argument("files", nargs="*", help="changed repository-relative paths")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    area_config = load_area_config(AREA_CONFIG)
    validate_area_config(ROOT, area_config)
    metadata = load_metadata(ROOT)
    scope = calculate_scope(args.files, ROOT, metadata, area_config, args.all)
    print(json.dumps(scope, separators=(",", ":")))


if __name__ == "__main__":
    main()

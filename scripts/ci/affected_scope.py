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
EXEMPTIONS_CONFIG = ROOT / ".github" / "ci" / "exemptions.json"
GLOBAL_PATHS = {
    ".config/nextest.toml",
    ".github/ci/areas.json",
    ".github/ci/exemptions.json",
    ".github/kache-version",
    ".github/workflows/_area-ci.yml",
    ".github/workflows/ci.yml",
    "Cargo.lock",
    "Cargo.toml",
    "justfile",
    "rust-toolchain.toml",
    "scripts/ci/affected_scope.py",
}
GLOBAL_PREFIXES = (".cargo/", ".github/actions/", "just/")

# Bootstrap-preflight breadth (D3). A global CI/tooling change validates every
# runner OS before fan-out; a package-local change validates only the scope host
# plus the OSes its selected areas actually run on.
ALL_RUNNER_OS = ["macos-latest", "ubuntu-latest", "windows-latest"]
SCOPE_HOST_OS = "ubuntu-latest"

# Per-area policy defaults, shared by config loading and preflight OS derivation
# so a test that passes minimal area records still resolves OS policy.
AREA_DEFAULTS: dict[str, Any] = {
    "full_os": ["ubuntu-latest", "windows-latest"],
    "check_os": ["macos-latest"],
    "shards": ["1/1"],
    "soft_os": ["windows-latest"],
    "l2": False,
    "browser": False,
    "kache": True,
    "ai_provider_stubs": False,
    # Capability policy (D8/D9/D11). `backends`: L2 terminal backends this area's
    # tests require. `native`: OS -> system packages needed to build/test.
    # `canary`: whether this area is a global-change canary (Phase 4).
    "backends": [],
    "native": {},
    "canary": False,
}

# Single policy surface (D10). Every area record is validated against this
# schema so a typo, an unsupported runner OS, or an unknown field fails loudly
# instead of silently mis-scoping CI.
SUPPORTED_RUNNER_OS = {"ubuntu-latest", "windows-latest", "macos-latest"}
KNOWN_L2_BACKENDS = {"tmux", "wezterm", "kitty", "apple-terminal"}
REQUIRED_AREA_FIELDS = {"area", "check_args"}
OS_LIST_FIELDS = ("full_os", "check_os", "soft_os")
ALLOWED_AREA_FIELDS = REQUIRED_AREA_FIELDS | set(AREA_DEFAULTS)


def load_metadata(root: Path) -> dict[str, Any]:
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


def validate_area_schema(areas: list[dict[str, Any]]) -> None:
    """Validate each raw area record against the capability-policy schema (D10).

    ## Errors

    Raises ``RuntimeError`` naming the offending area and field for a missing
    required field, an unknown field, an unsupported runner OS, an unknown L2
    backend, or a mistyped value.
    """
    for index, area in enumerate(areas):
        label = area.get("area", f"<record {index}>")

        missing = REQUIRED_AREA_FIELDS - area.keys()
        if missing:
            raise RuntimeError(f"area '{label}' is missing required field(s): {sorted(missing)}")

        unknown = area.keys() - ALLOWED_AREA_FIELDS
        if unknown:
            raise RuntimeError(
                f"area '{label}' has unknown field(s): {sorted(unknown)}; "
                f"allowed fields are {sorted(ALLOWED_AREA_FIELDS)}"
            )

        for field in OS_LIST_FIELDS:
            if field in area:
                invalid = [os for os in area[field] if os not in SUPPORTED_RUNNER_OS]
                if invalid:
                    raise RuntimeError(
                        f"area '{label}' field '{field}' names unsupported runner OS(es): "
                        f"{invalid}; supported: {sorted(SUPPORTED_RUNNER_OS)}"
                    )

        for backend in area.get("backends", []):
            if backend not in KNOWN_L2_BACKENDS:
                raise RuntimeError(
                    f"area '{label}' requires unknown L2 backend '{backend}'; "
                    f"known backends: {sorted(KNOWN_L2_BACKENDS)}"
                )

        native = area.get("native", {})
        if not isinstance(native, dict):
            raise RuntimeError(f"area '{label}' field 'native' must be an OS->packages map")
        for os_name, packages in native.items():
            if os_name not in SUPPORTED_RUNNER_OS:
                raise RuntimeError(
                    f"area '{label}' field 'native' names unsupported OS '{os_name}'"
                )
            if not isinstance(packages, list) or not all(isinstance(p, str) for p in packages):
                raise RuntimeError(
                    f"area '{label}' field 'native.{os_name}' must be a list of package names"
                )

        for field in ("l2", "browser", "kache", "ai_provider_stubs", "canary"):
            if field in area and not isinstance(area[field], bool):
                raise RuntimeError(f"area '{label}' field '{field}' must be a boolean")


def load_area_config(path: Path) -> list[dict[str, Any]]:
    with path.open(encoding="utf-8") as config_file:
        areas = json.load(config_file)

    validate_area_schema(areas)
    return [{**AREA_DEFAULTS, **area} for area in areas]


def load_exemptions(path: Path) -> dict[str, str]:
    """Load the package -> reason CI-ownership exemption map.

    ## Errors

    Raises ``RuntimeError`` for a duplicate exemption or an empty reason.
    """
    if not path.exists():
        return {}
    with path.open(encoding="utf-8") as config_file:
        entries = json.load(config_file)
    exemptions: dict[str, str] = {}
    for entry in entries:
        package = entry["package"]
        if package in exemptions:
            raise RuntimeError(f"duplicate CI-ownership exemption for '{package}'")
        if not entry.get("reason", "").strip():
            raise RuntimeError(f"exemption for '{package}' must give a non-empty reason")
        exemptions[package] = entry["reason"]
    return exemptions


def validate_ownership(
    metadata: dict[str, Any],
    root: Path,
    area_config: list[dict[str, Any]],
    exemptions: dict[str, str],
) -> None:
    """Every Cargo workspace member must have exactly one CI owner (D10).

    A member is owned when it lives under a curated area directory or is listed
    in the exemption map. Cargo metadata is the source of truth for membership.

    ## Errors

    Raises ``RuntimeError`` naming the offending packages for an unmapped member,
    a package that is both owned and exempt, or an exemption for a package that
    no longer exists.
    """
    packages = workspace_packages(metadata)
    area_names = {area["area"] for area in area_config}
    all_names = {package["name"] for package in packages.values()}
    owned_names = {
        package["name"]
        for package in packages.values()
        if owner_area(package, root, area_names) is not None
    }

    unmapped = sorted(all_names - owned_names - exemptions.keys())
    if unmapped:
        raise RuntimeError(
            "workspace packages have no CI owner or exemption: "
            f"{unmapped}. Add each to a curated area in .github/ci/areas.json "
            "(the area's justfile must define all canonical recipes) or to "
            ".github/ci/exemptions.json with a reason."
        )

    contradictory = sorted(exemptions.keys() & owned_names)
    if contradictory:
        raise RuntimeError(
            f"packages are both owned by a curated area and exempted: {contradictory}"
        )

    stale = sorted(exemptions.keys() - all_names)
    if stale:
        raise RuntimeError(
            f"exemptions reference packages not in the workspace: {stale}"
        )


def global_trigger(files: list[str]) -> str | None:
    """Return the first changed path that forces full workspace scope, if any."""
    for raw_file in files:
        normalized = raw_file.replace("\\", "/").removeprefix("./")
        if normalized in GLOBAL_PATHS or normalized.startswith(GLOBAL_PREFIXES):
            return normalized
    return None


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

    change_class, preflight_os, preflight_reason = classify_preflight(
        files, selected_areas, full_scope, force_all
    )

    canaries = [
        area["area"] for area in selected_areas if area.get("canary", False)
    ]

    return {
        "areas": selected_areas,
        "packages": selected_packages,
        "full_scope": full_scope,
        "change_class": change_class,
        "preflight_os": preflight_os,
        "preflight_reason": preflight_reason,
        "canaries": canaries,
    }


def classify_preflight(
    files: list[str],
    selected_areas: list[dict[str, Any]],
    full_scope: bool,
    force_all: bool,
) -> tuple[str, list[str], str]:
    """Classify the change and derive its bootstrap-preflight OS matrix (D3).

    ## Returns

    A ``(change_class, preflight_os, reason)`` triple. ``change_class`` is one
    of ``"full"``, ``"package"``, or ``"documentation"``.
    """
    if full_scope:
        if force_all:
            reason = "explicit full-scope request selects every runner OS"
        else:
            trigger = global_trigger(files)
            reason = (
                f"global CI/tooling input changed ({trigger}); "
                "preflight runs on every runner OS before fan-out"
                if trigger is not None
                else "workspace-global change selects full scope"
            )
        return "full", list(ALL_RUNNER_OS), reason

    if selected_areas:
        os_set = {SCOPE_HOST_OS}
        for area in selected_areas:
            os_set.update(area.get("full_os", AREA_DEFAULTS["full_os"]))
            os_set.update(area.get("soft_os", AREA_DEFAULTS["soft_os"]))
        reason = (
            f"package-local change across {len(selected_areas)} area(s); "
            "preflight covers the scope host plus each area's required OSes"
        )
        return "package", sorted(os_set), reason

    return (
        "documentation",
        [SCOPE_HOST_OS],
        "no build/test areas affected; preflight runs on the scope host only",
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--all", action="store_true", help="select the full workspace")
    parser.add_argument("files", nargs="*", help="changed repository-relative paths")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    area_config = load_area_config(AREA_CONFIG)
    validate_area_config(ROOT, area_config)
    exemptions = load_exemptions(EXEMPTIONS_CONFIG)
    metadata = load_metadata(ROOT)
    validate_ownership(metadata, ROOT, area_config, exemptions)
    scope = calculate_scope(args.files, ROOT, metadata, area_config, args.all)
    print(json.dumps(scope, separators=(",", ":")))


if __name__ == "__main__":
    main()

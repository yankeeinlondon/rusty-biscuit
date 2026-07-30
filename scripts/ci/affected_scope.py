#!/usr/bin/env python3
"""Calculate dependency-aware CI scope for the Rusty Biscuit workspace."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from datetime import date
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
AREA_CONFIG = ROOT / ".github" / "ci" / "areas.json"
GLOBAL_PATHS = {
    ".config/nextest.toml",
    ".github/ci/areas.json",
    ".github/workflows/_area-ci.yml",
    ".github/workflows/_wsl-ci.yml",
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
# plus the runner OSes its selected areas' environments actually land on.
ALL_RUNNER_OS = ["macos-latest", "ubuntu-latest", "windows-latest"]
SCOPE_HOST_OS = "ubuntu-latest"

# `environment` is not `os`. Windows, macOS, and Linux are operating systems;
# WSL2 is a distinct supported Linux environment that a Windows runner HOSTS.
# Policy and every result identity are keyed by environment; only `runs-on` and
# the native-package lookup are keyed by runner OS. Conflating the two silently
# merges two environments into one rollup cell.
SUPPORTED_RUNNER_OS = {"ubuntu-latest", "windows-latest", "macos-latest"}
SUPPORTED_ENVIRONMENTS = SUPPORTED_RUNNER_OS | {"wsl2-ubuntu"}

# Which runner label hosts each environment. Only entries whose environment
# differs from its host need listing; the three native ones map to themselves.
ENVIRONMENT_RUNNER_OS: dict[str, str] = {"wsl2-ubuntu": "windows-latest"}

# Environments where a headless L2 terminal backend can be provisioned. tmux is
# the only one CI can host, and it installs on Linux (apt) and macOS (brew).
# Windows has no tmux port and no proven alternative (plan 2.3), so an L2 tier
# there is a POLICY GAP, never a green `0 run / N skipped` cell. `wsl2-ubuntu`
# is absent because the WSL leg runs from a `nextest archive` (plan 2.2), which
# carries no broker binary and hosts no tmux server.
L2_PROVISIONED_ENVIRONMENTS = {"ubuntu-latest", "macos-latest"}

# Environments where Node + pnpm are provisioned for an area whose canonical
# `just test` also drives a JavaScript suite (`"node": true`).
#
# Linux only, for the reason `lint` is Linux only: the one such suite today
# (`homelab/server/frontend`, Vue + Vitest) runs under jsdom with no native
# dependency, so four environments would execute identical code paths and buy
# four toolchain installs. This does NOT reduce tests within an area — the
# recipe is identical on every environment and self-gates on the capability the
# way `require_level!` self-gates on a terminal backend.
NODE_PROVISIONED_ENVIRONMENTS = {"ubuntu-latest"}

# Per-area policy defaults, shared by config loading and preflight OS derivation
# so a test that passes minimal area records still resolves environment policy.
AREA_DEFAULTS: dict[str, Any] = {
    "ci": True,
    # Every area runs the same canonical L1 recipe on all three native
    # environments. macOS used to be compile-check-only because "runner minutes
    # bill ~10x"; the repo is public, so standard runners are free and that
    # justification is void.
    "environments": ["ubuntu-latest", "windows-latest", "macos-latest", "wsl2-ubuntu"],
    # Compile-only gate, and the ONLY place `RUSTFLAGS: -D warnings` is enforced
    # outside the Linux `lint` job. Pointed at Windows: `lint` already denies
    # warnings on Linux, and macOS is the primary development host where warning
    # drift is caught locally. Windows is nobody's dev box, so it is where
    # warning drift actually hides.
    "check_os": ["windows-latest"],
    "shards": ["1/1"],
    "l2": False,
    "browser": False,
    "ai_provider_stubs": False,
    # Capability policy (D8/D9/D11). `backends`: L2 terminal backends this area's
    # tests require. `native`: runner OS -> system packages needed to build/test.
    # `node`: whether this area's canonical `just test` also drives a JavaScript
    # suite, so the leg needs Node + pnpm. `canary`: whether this area is a
    # global-change canary (Phase 4). `policy_gaps`: tiers this area owns tests
    # for that no environment can currently host — recorded so the rollup
    # renders POLICY GAP, never green.
    "backends": [],
    "native": {},
    "node": False,
    "canary": False,
    "policy_gaps": [],
}

# Single policy surface (D10). Every area record is validated against this
# schema so a typo, an unsupported environment, or an unknown field fails loudly
# instead of silently mis-scoping CI.
KNOWN_L2_BACKENDS = {"tmux", "wezterm", "kitty", "apple-terminal"}
KNOWN_TIERS = {"L1", "L2", "browser"}
EXCLUSION_CLASSES = {"capability", "promotion-pending", "time-bounded"}
REQUIRED_AREA_FIELDS = {"area"}
OPTIONAL_AREA_FIELDS = {"check_args", "reason", "owner", "expiry", "exclusion_class"}
ALLOWED_AREA_FIELDS = REQUIRED_AREA_FIELDS | OPTIONAL_AREA_FIELDS | set(AREA_DEFAULTS)
POLICY_GAP_FIELDS = {"tier", "environments", "reason", "owner", "expiry"}

# Retired policy fields, named so a stale record gets an actionable message
# instead of appearing in a generic unknown-field list.
RETIRED_AREA_FIELDS: dict[str, str] = {
    "soft_os": (
        "it made a test leg continue-on-error, which did not merely stop the leg "
        "from blocking — it removed the leg from the run's verdict entirely. "
        "Record a known failure in the results baseline instead, which keeps the "
        "signal"
    ),
    "full_os": (
        "it named a runner OS list, but WSL2 is an environment a Windows runner "
        "HOSTS rather than a runner label of its own. Use 'environments', whose "
        "values are ubuntu-latest, windows-latest, macos-latest, and wsl2-ubuntu"
    ),
}


def load_metadata(root: Path) -> dict[str, Any]:
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


def validate_native_map(label: str, native: Any) -> None:
    """Validate a `native` OS -> system-package declaration.

    Shared by curated areas and by exempt packages: a package's system libraries
    are a property of the package, so an exempt package declares them the same
    way an owned one does. `_ensure-native-libs` reads both files.

    ## Errors

    Raises ``RuntimeError`` naming the offending declaration.
    """
    if not isinstance(native, dict):
        raise RuntimeError(f"{label} field 'native' must be an OS->packages map")
    for os_name, packages in native.items():
        if os_name not in SUPPORTED_RUNNER_OS:
            raise RuntimeError(f"{label} field 'native' names unsupported OS '{os_name}'")
        if not isinstance(packages, list) or not all(isinstance(p, str) for p in packages):
            raise RuntimeError(
                f"{label} field 'native.{os_name}' must be a list of package names"
            )


def validate_expiry(label: str, field: str, value: Any, today: date) -> None:
    """Validate a time-bounded policy date.

    An exclusion or policy gap without an end date is a permanent one wearing a
    temporary label. A *past* end date must fail the scope calculation loudly
    rather than lapse quietly, which is the whole point of bounding it.

    ## Errors

    Raises ``RuntimeError`` naming the record and the date.
    """
    if not isinstance(value, str):
        raise RuntimeError(f"{label} field '{field}' must be an ISO date string (YYYY-MM-DD)")
    try:
        expiry = date.fromisoformat(value)
    except ValueError as error:
        raise RuntimeError(
            f"{label} field '{field}' must be an ISO date (YYYY-MM-DD), got '{value}'"
        ) from error
    if expiry < today:
        raise RuntimeError(
            f"{label} field '{field}' expired on {value}. Close the item, or move the "
            "date out with a fresh justification — a lapsed bound is a permanent "
            "exclusion wearing a temporary label."
        )


def validate_policy_gaps(label: str, area: dict[str, Any], today: date) -> None:
    """Validate an area's declared policy gaps and prove every gap is covered.

    A policy gap is a tier the area owns tests for that some declared
    environment cannot host. It exists so the rollup can render POLICY GAP for
    that cell instead of a green `0 run / N skipped`.

    ## Errors

    Raises ``RuntimeError`` naming the area, the gap, and the offending field.
    """
    gaps = area.get("policy_gaps", [])
    if not isinstance(gaps, list):
        raise RuntimeError(f"{label} field 'policy_gaps' must be a list of gap records")

    environments = area.get("environments", AREA_DEFAULTS["environments"])
    for index, gap in enumerate(gaps):
        gap_label = f"{label} policy_gaps[{index}]"
        if not isinstance(gap, dict):
            raise RuntimeError(f"{gap_label} must be an object")

        missing = POLICY_GAP_FIELDS - gap.keys()
        if missing:
            raise RuntimeError(f"{gap_label} is missing required field(s): {sorted(missing)}")
        unknown = gap.keys() - POLICY_GAP_FIELDS
        if unknown:
            raise RuntimeError(f"{gap_label} has unknown field(s): {sorted(unknown)}")

        if gap["tier"] not in KNOWN_TIERS:
            raise RuntimeError(
                f"{gap_label} names unknown tier '{gap['tier']}'; known tiers: "
                f"{sorted(KNOWN_TIERS)}"
            )
        for field in ("reason", "owner"):
            if not isinstance(gap[field], str) or not gap[field].strip():
                raise RuntimeError(f"{gap_label} must give a non-empty '{field}'")
        validate_expiry(gap_label, "expiry", gap["expiry"], today)

        gap_environments = gap["environments"]
        if not isinstance(gap_environments, list) or not gap_environments:
            raise RuntimeError(f"{gap_label} must list at least one environment")
        invalid = [env for env in gap_environments if env not in SUPPORTED_ENVIRONMENTS]
        if invalid:
            raise RuntimeError(
                f"{gap_label} names unsupported environment(s): {invalid}; supported: "
                f"{sorted(SUPPORTED_ENVIRONMENTS)}"
            )
        undeclared = [env for env in gap_environments if env not in environments]
        if undeclared:
            raise RuntimeError(
                f"{gap_label} names environment(s) the area does not run: {undeclared}. "
                "A gap describes a cell that IS scheduled but cannot execute; an "
                "environment the area never runs needs no gap record."
            )

    # An L2 tier with tests but no provisionable backend must be declared, or the
    # cell renders as an unremarkable green skip (plan 1.1/2.3).
    if area.get("ci", AREA_DEFAULTS["ci"]) and area.get("l2", AREA_DEFAULTS["l2"]):
        covered = {
            environment
            for gap in gaps
            if gap.get("tier") == "L2"
            for environment in gap.get("environments", [])
        }
        uncovered = [
            environment
            for environment in environments
            if environment not in L2_PROVISIONED_ENVIRONMENTS and environment not in covered
        ]
        if uncovered:
            raise RuntimeError(
                f"{label} sets \"l2\": true and runs on {uncovered}, where no L2 backend "
                "can be provisioned. Declare a 'policy_gaps' entry with tier \"L2\" for "
                "those environments so the rollup shows POLICY GAP instead of a green "
                "`0 run / N skipped` cell."
            )


def validate_area_schema(areas: list[dict[str, Any]], today: date | None = None) -> None:
    """Validate each raw area record against the capability-policy schema (D10).

    ## Errors

    Raises ``RuntimeError`` naming the offending area and field for a missing
    required field, a retired field, an unknown field, an unsupported
    environment or runner OS, an unknown L2 backend, an unowned or lapsed
    exclusion, an uncovered L2 policy gap, or a mistyped value.
    """
    today = today or date.today()

    for index, area in enumerate(areas):
        label = area.get("area", f"<record {index}>")

        missing = REQUIRED_AREA_FIELDS - area.keys()
        if missing:
            raise RuntimeError(f"area '{label}' is missing required field(s): {sorted(missing)}")

        # Types before semantics: every rule below reads these flags, so a
        # mistyped one must be reported as the typo it is rather than as the
        # policy violation its truthiness happens to produce.
        for field in ("l2", "browser", "ai_provider_stubs", "canary", "ci", "node"):
            if field in area and not isinstance(area[field], bool):
                raise RuntimeError(f"area '{label}' field '{field}' must be a boolean")

        # An area either gates in the fan-out matrix (and needs the compile-check
        # arguments) or explicitly does not (and must say why, who owns it, and
        # when the exclusion runs out). Both are declared here so every area
        # sniff discovers has exactly one record.
        in_matrix = area.get("ci", AREA_DEFAULTS["ci"])
        if in_matrix and not area.get("check_args", "").strip():
            raise RuntimeError(f"area '{label}' is in the CI matrix and must define 'check_args'")
        if not in_matrix:
            for field in ("reason", "owner"):
                if not area.get(field, "").strip():
                    raise RuntimeError(
                        f"area '{label}' sets \"ci\": false and must give a non-empty "
                        f"'{field}'"
                    )
            exclusion_class = area.get("exclusion_class", "")
            if exclusion_class not in EXCLUSION_CLASSES:
                raise RuntimeError(
                    f"area '{label}' sets \"ci\": false and must give an "
                    f"'exclusion_class' from {sorted(EXCLUSION_CLASSES)}"
                )
            # A capability exclusion (physical hardware, say) is permanent by
            # nature; everything else is backlog and must be time-bounded.
            if exclusion_class == "capability":
                if "expiry" in area:
                    raise RuntimeError(
                        f"area '{label}' is excluded on capability grounds, which is "
                        "permanent; drop 'expiry' or choose another exclusion_class"
                    )
            elif "expiry" not in area:
                raise RuntimeError(
                    f"area '{label}' sets \"ci\": false with exclusion_class "
                    f"'{exclusion_class}' and must give an 'expiry' date"
                )

        for field, guidance in RETIRED_AREA_FIELDS.items():
            if field in area:
                raise RuntimeError(f"area '{label}' uses retired field '{field}': {guidance}")

        unknown = area.keys() - ALLOWED_AREA_FIELDS
        if unknown:
            raise RuntimeError(
                f"area '{label}' has unknown field(s): {sorted(unknown)}; "
                f"allowed fields are {sorted(ALLOWED_AREA_FIELDS)}"
            )

        if "expiry" in area:
            validate_expiry(f"area '{label}'", "expiry", area["expiry"], today)

        if "environments" in area:
            invalid = [
                env for env in area["environments"] if env not in SUPPORTED_ENVIRONMENTS
            ]
            if invalid:
                raise RuntimeError(
                    f"area '{label}' field 'environments' names unsupported "
                    f"environment(s): {invalid}; supported: "
                    f"{sorted(SUPPORTED_ENVIRONMENTS)}"
                )

        # `check_os` really is a runner-OS list: a compile-check produces no test
        # results, so it has no environment identity to key a rollup cell to.
        if "check_os" in area:
            invalid = [os for os in area["check_os"] if os not in SUPPORTED_RUNNER_OS]
            if invalid:
                raise RuntimeError(
                    f"area '{label}' field 'check_os' names unsupported runner OS(es): "
                    f"{invalid}; supported: {sorted(SUPPORTED_RUNNER_OS)}"
                )

        for backend in area.get("backends", []):
            if backend not in KNOWN_L2_BACKENDS:
                raise RuntimeError(
                    f"area '{label}' requires unknown L2 backend '{backend}'; "
                    f"known backends: {sorted(KNOWN_L2_BACKENDS)}"
                )

        # A declared `node` capability that no declared environment can host is a
        # silent gap by construction: `node_environments` resolves empty, the
        # recipe self-skips on every leg, and the area reads green having run
        # none of its JavaScript tests. That is precisely the failure this flag
        # exists to prevent, so it fails at config time.
        if area.get("ci", AREA_DEFAULTS["ci"]) and area.get("node", AREA_DEFAULTS["node"]):
            environments = area.get("environments", AREA_DEFAULTS["environments"])
            if not any(
                environment in NODE_PROVISIONED_ENVIRONMENTS for environment in environments
            ):
                raise RuntimeError(
                    f"area '{label}' sets \"node\": true but runs on {environments}, none of "
                    f"which provisions pnpm ({sorted(NODE_PROVISIONED_ENVIRONMENTS)}). Its "
                    "JavaScript suite would skip on every leg while the area still reported "
                    "PASS. Declare an environment that can host it, or drop the capability."
                )

        # A WSL2 guest IS Linux, so it consumes the `ubuntu-latest` package list
        # (`_ensure-native-libs` keys off `uname -s`). `native` therefore stays a
        # runner-OS map and must not grow a `wsl2-ubuntu` key.
        validate_native_map(f"area '{label}'", area.get("native", {}))

        validate_policy_gaps(f"area '{label}'", area, today)


def environment_policy(area: dict[str, Any]) -> dict[str, Any]:
    """Split an area's declared environments into the shapes a workflow can use.

    ``_area-ci.yml`` puts ``native_environments`` straight into a ``runs-on``
    matrix, which is only sound because those three environment names ARE runner
    labels. ``wsl2-ubuntu`` is deliberately hoisted into its own boolean so it
    can never reach that matrix: a WSL job runs on `windows-latest` and executes
    through `wsl-bash`, so native dependency installation, paths, shells,
    caching, and artifact collection would otherwise take the Windows branch.

    ## Returns

    ``{"native_environments": [...], "l2_environments": [...],
    "node_environments": [...], "wsl": bool}``.
    """
    environments = list(area.get("environments", AREA_DEFAULTS["environments"]))
    return {
        "native_environments": [
            environment for environment in environments if environment in SUPPORTED_RUNNER_OS
        ],
        "l2_environments": (
            [
                environment
                for environment in environments
                if environment in L2_PROVISIONED_ENVIRONMENTS
            ]
            if area.get("l2", AREA_DEFAULTS["l2"])
            else []
        ),
        # Same derivation as `l2_environments`: the area declares a capability,
        # not a runner list, and the intersection with what CI can provision is
        # computed here rather than hard-coded in workflow YAML.
        "node_environments": (
            [
                environment
                for environment in environments
                if environment in NODE_PROVISIONED_ENVIRONMENTS
            ]
            if area.get("node", AREA_DEFAULTS["node"])
            else []
        ),
        "wsl": "wsl2-ubuntu" in environments,
    }


def load_area_config(path: Path) -> list[dict[str, Any]]:
    with path.open(encoding="utf-8") as config_file:
        areas = json.load(config_file)

    validate_area_schema(areas)
    return [{**AREA_DEFAULTS, **area} for area in areas]


def validate_ownership(
    metadata: dict[str, Any],
    root: Path,
    area_config: list[dict[str, Any]],
) -> None:
    """Every Cargo workspace member must fall under exactly one declared area (D10).

    Every area has a record in `areas.json`, whether or not it gates in the CI
    matrix — a `"ci": false` record declares the area and states why it is not
    gated. So ownership is simply: the package's directory maps to a declared
    area. A package in a brand-new directory fails here until someone decides
    what that area is and whether it should gate.

    ## Errors

    Raises ``RuntimeError`` naming the offending packages.
    """
    packages = workspace_packages(metadata)
    area_names = {area["area"] for area in area_config}

    unmapped = sorted(
        package["name"]
        for package in packages.values()
        if owner_area(package, root, area_names) is None
    )
    if unmapped:
        raise RuntimeError(
            "workspace packages fall under no declared area: "
            f"{unmapped}. Add the area to .github/ci/areas.json — with "
            'the full policy if it should gate, or with "ci": false and a '
            "reason if it should not."
        )


def validate_no_shadow_workspaces(metadata: dict[str, Any], root: Path) -> None:
    """No directory may re-declare a root workspace member in its own workspace.

    A nested ``[workspace]`` whose members are also root members makes the same
    package resolve into two different workspaces depending on the current
    directory. Cargo picks the nearest ancestor workspace root, so ``cd <area> &&
    cargo test`` then gets a different target directory, a different lockfile
    (hence different dependency versions than the root build ships), and
    different config discovery — which is how ``.config/nextest.toml``'s ``ci``
    profile became invisible to three areas.

    A genuinely standalone workspace is fine; it is only a shadow when its
    members overlap the root workspace.

    ## Errors

    Raises ``RuntimeError`` naming each shadowing manifest.
    """
    member_dirs = {
        Path(package["manifest_path"]).parent.resolve()
        for package in workspace_packages(metadata).values()
    }

    shadows: list[str] = []
    for manifest in sorted(root.glob("*/Cargo.toml")):
        if manifest.parent.resolve() == root.resolve():
            continue
        text = manifest.read_text(encoding="utf-8")
        if "[workspace]" not in text:
            continue
        nested_root = manifest.parent.resolve()
        if any(
            member == nested_root or nested_root in member.parents
            for member in member_dirs
        ):
            # POSIX separators so the message is identical on every runner
            # OS and stays copy-pasteable (`global_trigger` normalizes likewise).
            shadows.append(manifest.relative_to(root).as_posix())

    if shadows:
        raise RuntimeError(
            "nested [workspace] manifests shadow root workspace members: "
            f"{shadows}. Delete the nested [workspace] so the package resolves "
            "to one workspace, one target directory, one lockfile, and one "
            "`.config/nextest.toml`, regardless of the working directory."
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
    """Check the justfile's curated list against the area records.

    The two lists answer different questions and are deliberately not equal.
    The justfile's ``areas :=`` names every package area that owns tests, so
    ``check-canonical`` can verify its recipe set; ``ci: true`` names the areas
    the matrix actually fans out to. An area with a complete recipe set that is
    not yet promoted appears in the first and not the second — that gap is the
    promotion backlog, and collapsing it would force a choice between gating an
    area prematurely and leaving its recipes unverified.

    ## Errors

    Raises ``RuntimeError`` when a gating area is absent from the justfile (CI
    would fan out to recipes nothing validates) or when a justfile area has no
    record at all (its CI disposition was never decided).
    """
    declared = {area["area"] for area in areas}
    gating = [area["area"] for area in areas if area.get("ci", True)]
    curated = curated_areas(root)

    ungoverned = [area for area in gating if area not in curated]
    if ungoverned:
        raise RuntimeError(
            f"areas gate CI but are missing from the justfile's `areas :=` list: {ungoverned}. "
            "`just check-canonical` would never verify their recipe set."
        )

    undeclared = [area for area in curated if area not in declared]
    if undeclared:
        raise RuntimeError(
            f"areas are in the justfile's `areas :=` list but have no record in areas.json: "
            f"{undeclared}. Add a record with `ci: true`, or `ci: false` plus an owner and expiry."
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

    # `ci: false` areas are declared for ownership but never fan out. A change
    # inside one still selects its packages (so reverse-dependency closure and
    # coverage see it) -- it just launches no area job.
    selected_areas = [
        {**area, **environment_policy(area)}
        for area in area_config
        if area["area"] in affected_areas and area.get("ci", True)
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
        # Preflight runs on RUNNER labels, so each environment is resolved to the
        # runner that hosts it — `wsl2-ubuntu` preflights on `windows-latest`.
        os_set = {SCOPE_HOST_OS}
        for area in selected_areas:
            for environment in area.get("environments", AREA_DEFAULTS["environments"]):
                os_set.add(ENVIRONMENT_RUNNER_OS.get(environment, environment))
        reason = (
            f"package-local change across {len(selected_areas)} area(s); "
            "preflight covers the scope host plus the runner OS hosting each "
            "area's required environments"
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
    metadata = load_metadata(ROOT)
    validate_ownership(metadata, ROOT, area_config)
    validate_no_shadow_workspaces(metadata, ROOT)
    scope = calculate_scope(args.files, ROOT, metadata, area_config, args.all)
    print(json.dumps(scope, separators=(",", ":")))


if __name__ == "__main__":
    main()

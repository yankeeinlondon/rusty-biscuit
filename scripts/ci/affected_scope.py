#!/usr/bin/env python3
"""Calculate dependency-aware CI scope for the Rusty Biscuit workspace.

The package is the unit of selection, execution, and result identity. Package
policy lives in each package's own Cargo manifest under `[package.metadata.ci]`;
environment capabilities live in `.github/ci/environments.json`. Both are
validated here, loudly, before any scope is emitted.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from datetime import date
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
ENVIRONMENTS_CONFIG = ROOT / ".github" / "ci" / "environments.json"
GLOBAL_PATHS = {
    ".config/nextest.toml",
    ".github/ci/environments.json",
    ".github/workflows/_package-ci.yml",
    ".github/workflows/_wsl-ci.yml",
    ".github/workflows/ci.yml",
    "Cargo.toml",
    "justfile",
    "rust-toolchain.toml",
    "scripts/ci/affected_scope.py",
}
GLOBAL_PREFIXES = (".cargo/", ".github/actions/", "just/")

# Deliberately NOT in GLOBAL_PATHS. Every dependency add or removal rewrites the
# lockfile, so treating the filename as global escalated the most routine change
# in the repo to a full-workspace run: measured on PR #39, dropping one
# dev-dependency from one crate scheduled every package on every OS.
#
# The lockfile is the one global path whose blast radius is *derivable* — it
# names the resolved graph, so the packages a change can reach can be read out
# of the diff rather than assumed to be all of them. `Cargo.toml` stays global:
# it carries workspace membership and shared dependency tables whose effects are
# not recoverable from the file alone.
LOCKFILE_PATH = "Cargo.lock"

# CI's own tooling — the merge-gate binary (`scripts/ci-rollup*.rs`), the
# scope calculator (`scripts/ci/`), and the policy store (`.github/ci/`) — is
# not a Cargo package, so a change to it selects no package. It must still
# exercise something: `ci.yml` runs the rollup and scope test suites on the
# `ci_tooling` flag. (`scripts/ci/affected_scope.py` and
# `.github/ci/environments.json` are additionally GLOBAL_PATHS, since every
# package's scope depends on them.)
CI_TOOLING_PREFIXES = ("scripts/", ".github/ci/")

# Bootstrap-preflight breadth (D3). A global CI/tooling change validates every
# runner OS before fan-out; a package-local change validates only the scope host
# plus the runner OSes its selected packages' environments actually land on.
ALL_RUNNER_OS = ["macos-latest", "ubuntu-latest", "windows-latest"]
SCOPE_HOST_OS = "ubuntu-latest"

KNOWN_L2_BACKENDS = {"tmux", "wezterm", "kitty", "apple-terminal"}
KNOWN_TIERS = {"L1", "L2", "browser"}
EXCLUSION_CLASSES = {"capability", "promotion-pending", "time-bounded"}

# `runner-tools` is a CLOSED vocabulary implemented by the reusable workflow,
# not an arbitrary command surface.
KNOWN_RUNNER_TOOLS = {
    "ai-provider-stubs",
    "darkmatter-md-fixture",
    "messenger-desktop-stubs",
    "node-22",
    "pnpm-10",
    "l2-parallel-self-spawn",
    "neovim",
}

# Companion suites are non-Cargo test suites owned by a package. Each name maps
# to the justfile (by directory) and recipe that executes it; the recipe must
# exist, or the suite would be declared and silently never run.
COMPANION_SUITES = {"homelab-frontend": ("homelab", "test-frontend")}

# `[package.metadata.ci]` field vocabulary. Unknown fields fail loudly — a typo
# here silently mis-scopes CI otherwise.
CI_FIELDS = {"gates", "exclusion-class", "owner", "reason", "expiry", "native", "tests"}
CI_TEST_FIELDS = {
    "tiers",
    "l2-backends",
    "features",
    "all-features",
    "l1-include-slow",
    "runner-tools",
    "companion-suites",
}
EXCLUSION_FIELDS = {"exclusion-class", "owner", "reason", "expiry"}

# The compile-check OS was never overridden by any of the 31 area records — a
# constant, not configuration. It stays `--all-targets` on Windows: benches and
# examples compile there and nowhere else.
CHECK_OS = ["windows-latest"]

# GitHub Actions ceiling for a single matrix. The package matrix must stay
# under it even on a full-scope run.
MATRIX_LIMIT = 256


def load_metadata(root: Path) -> dict[str, Any]:
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
        # Cargo emits UTF-8; without this, Windows decodes with the locale
        # codec (cp1252), the reader thread dies on the first non-cp1252 byte,
        # and `result.stdout` is None.
        encoding="utf-8",
    )
    return json.loads(result.stdout)


def validate_expiry(label: str, field: str, value: Any, today: date) -> None:
    """Validate a time-bounded policy date.

    An exclusion or capability gap without an end date is a permanent one
    wearing a temporary label. A *past* end date must fail the scope calculation
    loudly rather than lapse quietly, which is the whole point of bounding it.

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


# ---------------------------------------------------------------------------
# Environment capabilities — .github/ci/environments.json
# ---------------------------------------------------------------------------

# Every L2 backend is a capability under its own name, so a package whose
# suite drives only GUI emulators renders a governed POLICY GAP instead of an
# ungoverned block.
KNOWN_CAPABILITIES = {
    "tmux",
    "wezterm",
    "kitty",
    "apple-terminal",
    "headless_browser",
    "node_pnpm",
    "archive_only",
}


def load_environments(path: Path, today: date | None = None) -> list[dict[str, Any]]:
    """Load and validate the environment capability table.

    One versioned, schema-validated table: runner labels, native-package
    installer keys, and whether an environment can host tmux, a headless
    browser, Node/pnpm, or archive-only execution. Capability only — package
    policy decides which tiers are expected, so an unsupported required tier is
    an explicit POLICY GAP downstream, never a silent absence.

    A capability value is either a boolean or, for a governed unavailability,
    an object carrying `available: false` plus `reason`, `owner`, and `expiry`
    — the two facts the eight per-area `policy_gaps` records used to restate
    (Windows has no tmux; the WSL2 leg is archive-only), declared once.

    ## Errors

    Raises ``RuntimeError`` naming the offending environment and field.
    """
    today = today or date.today()
    with path.open(encoding="utf-8") as config_file:
        document = json.load(config_file)

    if not isinstance(document, dict) or not isinstance(document.get("environments"), list):
        raise RuntimeError(f"{path}: must be an object with an 'environments' list")
    if document.get("schema_version") != 1:
        raise RuntimeError(f"{path}: schema_version must be 1")

    environments = document["environments"]
    names: set[str] = set()
    for index, environment in enumerate(environments):
        label = environment.get("name", f"<record {index}>") if isinstance(environment, dict) else f"<record {index}>"
        if not isinstance(environment, dict):
            raise RuntimeError(f"environments[{index}] must be an object")
        unknown = environment.keys() - {"name", "runner", "native_key", "capabilities"}
        if unknown:
            raise RuntimeError(f"environment '{label}' has unknown field(s): {sorted(unknown)}")
        for field in ("name", "runner", "native_key"):
            if not isinstance(environment.get(field), str) or not environment[field].strip():
                raise RuntimeError(f"environment '{label}' must give a non-empty '{field}'")
        if environment["name"] in names:
            raise RuntimeError(f"duplicate environment '{environment['name']}'")
        names.add(environment["name"])

        capabilities = environment.get("capabilities")
        if not isinstance(capabilities, dict):
            raise RuntimeError(f"environment '{label}' must define a 'capabilities' object")
        unknown_caps = capabilities.keys() - KNOWN_CAPABILITIES
        if unknown_caps:
            raise RuntimeError(
                f"environment '{label}' has unknown capabilities: {sorted(unknown_caps)}; "
                f"known: {sorted(KNOWN_CAPABILITIES)}"
            )
        missing_caps = KNOWN_CAPABILITIES - capabilities.keys()
        if missing_caps:
            raise RuntimeError(
                f"environment '{label}' must declare every capability; missing: "
                f"{sorted(missing_caps)}"
            )
        for capability, value in capabilities.items():
            cap_label = f"environment '{label}' capability '{capability}'"
            if isinstance(value, bool):
                continue
            if capability == "archive_only":
                raise RuntimeError(f"{cap_label} must be a boolean")
            if not isinstance(value, dict) or value.get("available") is not False:
                raise RuntimeError(
                    f"{cap_label} must be a boolean or an object with 'available': false"
                )
            unknown_gap = value.keys() - {"available", "reason", "owner", "expiry"}
            if unknown_gap:
                raise RuntimeError(f"{cap_label} has unknown field(s): {sorted(unknown_gap)}")
            for field in ("reason", "owner"):
                if not isinstance(value.get(field), str) or not value[field].strip():
                    raise RuntimeError(
                        f"{cap_label} is a governed unavailability and must give a "
                        f"non-empty '{field}'"
                    )
            validate_expiry(cap_label, "expiry", value.get("expiry"), today)

    runner_labels = {environment["runner"] for environment in environments}
    for environment in environments:
        if environment["native_key"] not in runner_labels:
            raise RuntimeError(
                f"environment '{environment['name']}' native_key "
                f"'{environment['native_key']}' is not a runner label "
                f"({sorted(runner_labels)})"
            )
        if environment["capabilities"]["archive_only"] is True and environment["runner"] == environment["name"]:
            raise RuntimeError(
                f"environment '{environment['name']}' is archive-only but names itself as "
                "its runner; an archive-only environment is hosted by another runner"
            )

    return environments


def capability(environment: dict[str, Any], name: str) -> bool:
    """Whether an environment can host a capability, boolean or governed object."""
    value = environment["capabilities"][name]
    if isinstance(value, bool):
        return value
    return bool(value.get("available"))


def capability_gap(environment: dict[str, Any], name: str) -> dict[str, str] | None:
    """The governance record for a governed unavailability, if one exists."""
    value = environment["capabilities"][name]
    if isinstance(value, dict) and value.get("available") is False:
        return {"owner": value["owner"], "reason": value["reason"], "expiry": value["expiry"]}
    return None


def backend_hostable(environment: dict[str, Any], backend: str) -> bool:
    """Whether an environment can host an L2 backend.

    The capability is looked up under the backend's own name (`tmux`,
    `wezterm`, `kitty`, `apple-terminal`), giving every declared backend an
    environment axis rather than hardcoding `tmux`. A backend with no
    capability entry is not hostable there.
    """
    value = environment["capabilities"].get(backend)
    if value is None:
        return False
    if isinstance(value, bool):
        return value
    return bool(value.get("available"))


# ---------------------------------------------------------------------------
# Package CI policy — [package.metadata.ci] in each Cargo manifest
# ---------------------------------------------------------------------------


def validate_native_map(label: str, native: Any, runner_labels: set[str]) -> None:
    """Validate a `native` OS -> system-package declaration.

    ## Errors

    Raises ``RuntimeError`` naming the offending declaration.
    """
    if not isinstance(native, dict):
        raise RuntimeError(f"{label} field 'native' must be an OS->packages map")
    for os_name, packages in native.items():
        if os_name not in runner_labels:
            raise RuntimeError(
                f"{label} field 'native' names unsupported OS '{os_name}'; "
                f"runner labels: {sorted(runner_labels)}"
            )
        if not isinstance(packages, list) or not all(isinstance(p, str) for p in packages):
            raise RuntimeError(
                f"{label} field 'native.{os_name}' must be a list of package names"
            )


def validate_package_ci(
    name: str,
    ci: dict[str, Any],
    runner_labels: set[str],
    root: Path,
    today: date,
) -> None:
    """Validate one package's `[package.metadata.ci]` block.

    ## Errors

    Raises ``RuntimeError`` on an unknown field, an invalid tier or tool name,
    conflicting `features`/`all-features`, an expired or unowned exclusion, an
    L2 tier without backends, or a companion suite with no registered canonical
    recipe.
    """
    label = f"package '{name}' [package.metadata.ci]"
    unknown = ci.keys() - CI_FIELDS
    if unknown:
        raise RuntimeError(
            f"{label} has unknown field(s): {sorted(unknown)}; allowed: {sorted(CI_FIELDS)}"
        )

    gates = ci.get("gates", True)
    if not isinstance(gates, bool):
        raise RuntimeError(f"{label} field 'gates' must be a boolean")

    declared_exclusion = EXCLUSION_FIELDS & ci.keys()
    if gates and declared_exclusion:
        raise RuntimeError(
            f"{label} sets exclusion field(s) {sorted(declared_exclusion)} while "
            "'gates' is true; exclusion governance only exists for a non-gating package"
        )
    if not gates:
        for field in ("reason", "owner"):
            if not isinstance(ci.get(field), str) or not ci[field].strip():
                raise RuntimeError(
                    f"{label} sets 'gates = false' and must give a non-empty '{field}'"
                )
        exclusion_class = ci.get("exclusion-class", "")
        if exclusion_class not in EXCLUSION_CLASSES:
            raise RuntimeError(
                f"{label} sets 'gates = false' and must give an 'exclusion-class' "
                f"from {sorted(EXCLUSION_CLASSES)}"
            )
        # A capability exclusion (physical hardware, say) is permanent by
        # nature; everything else is backlog and must be time-bounded.
        if exclusion_class == "capability":
            if "expiry" in ci:
                raise RuntimeError(
                    f"{label} is excluded on capability grounds, which is permanent; "
                    "drop 'expiry' or choose another exclusion-class"
                )
        elif "expiry" not in ci:
            raise RuntimeError(
                f"{label} sets 'gates = false' with exclusion-class '{exclusion_class}' "
                "and must give an 'expiry' date"
            )
    if "expiry" in ci:
        validate_expiry(label, "expiry", ci["expiry"], today)

    validate_native_map(label, ci.get("native", {}), runner_labels)

    tests = ci.get("tests", {})
    if not isinstance(tests, dict):
        raise RuntimeError(f"{label} field 'tests' must be an object")
    unknown_tests = tests.keys() - CI_TEST_FIELDS
    if unknown_tests:
        raise RuntimeError(
            f"{label}.tests has unknown field(s): {sorted(unknown_tests)}; "
            f"allowed: {sorted(CI_TEST_FIELDS)}"
        )

    tiers = tests.get("tiers", ["L1"])
    if not isinstance(tiers, list) or not all(isinstance(t, str) for t in tiers):
        raise RuntimeError(f"{label}.tests field 'tiers' must be a list of tier names")
    invalid_tiers = [tier for tier in tiers if tier not in KNOWN_TIERS]
    if invalid_tiers:
        raise RuntimeError(
            f"{label}.tests names unknown tier(s) {invalid_tiers}; known: "
            f"{sorted(KNOWN_TIERS)}. L3 and 'real' are opt-in local tiers, not CI tiers."
        )
    if len(set(tiers)) != len(tiers):
        raise RuntimeError(f"{label}.tests field 'tiers' has duplicates: {tiers}")
    if "L1" not in tiers:
        raise RuntimeError(
            f"{label}.tests declares {tiers} without L1; a gating package always runs "
            "L1 — 'gates = false' is how a package opts out"
        )

    backends = tests.get("l2-backends", [])
    if not isinstance(backends, list) or not all(isinstance(b, str) for b in backends):
        raise RuntimeError(f"{label}.tests field 'l2-backends' must be a list of backend names")
    invalid_backends = [b for b in backends if b not in KNOWN_L2_BACKENDS]
    if invalid_backends:
        raise RuntimeError(
            f"{label}.tests requires unknown L2 backend(s) {invalid_backends}; "
            f"known: {sorted(KNOWN_L2_BACKENDS)}"
        )
    if "L2" in tiers and not backends:
        raise RuntimeError(
            f"{label}.tests declares the L2 tier without 'l2-backends'; name the "
            "terminal backends its tests require"
        )
    if "L2" not in tiers and backends:
        raise RuntimeError(
            f"{label}.tests declares 'l2-backends' without the L2 tier"
        )

    features = tests.get("features", [])
    all_features = tests.get("all-features", False)
    if not isinstance(features, list) or not all(isinstance(f, str) for f in features):
        raise RuntimeError(f"{label}.tests field 'features' must be a list of feature names")
    if not isinstance(all_features, bool):
        raise RuntimeError(f"{label}.tests field 'all-features' must be a boolean")
    if features and all_features:
        raise RuntimeError(
            f"{label}.tests sets both 'features' and 'all-features'; they conflict — "
            "pick one feature contract for check, archive, and test"
        )
    if not isinstance(tests.get("l1-include-slow", False), bool):
        raise RuntimeError(f"{label}.tests field 'l1-include-slow' must be a boolean")

    tools = tests.get("runner-tools", [])
    if not isinstance(tools, list) or not all(isinstance(t, str) for t in tools):
        raise RuntimeError(f"{label}.tests field 'runner-tools' must be a list of tool names")
    invalid_tools = [t for t in tools if t not in KNOWN_RUNNER_TOOLS]
    if invalid_tools:
        raise RuntimeError(
            f"{label}.tests names unknown runner tool(s) {invalid_tools}; the "
            f"vocabulary is closed: {sorted(KNOWN_RUNNER_TOOLS)}"
        )

    suites = tests.get("companion-suites", [])
    if not isinstance(suites, list) or not all(isinstance(s, str) for s in suites):
        raise RuntimeError(
            f"{label}.tests field 'companion-suites' must be a list of suite names"
        )
    for suite in suites:
        registered = COMPANION_SUITES.get(suite)
        if registered is None:
            raise RuntimeError(
                f"{label}.tests names unknown companion suite '{suite}'; registered: "
                f"{sorted(COMPANION_SUITES)}"
            )
        directory, recipe = registered
        justfile = root / directory / "justfile"
        # A recipe DEFINITION, not a substring: `test-frontend-watch:` must not
        # satisfy the check for `test-frontend`.
        recipe_defined = justfile.is_file() and re.search(
            rf"^{re.escape(recipe)}(?::|\s)",
            justfile.read_text(encoding="utf-8"),
            re.MULTILINE,
        )
        if not recipe_defined:
            raise RuntimeError(
                f"companion suite '{suite}' expects the canonical recipe '{recipe}' in "
                f"{directory}/justfile, which does not exist"
            )


def package_ci_policy(
    packages: dict[str, dict[str, Any]],
    runner_labels: set[str],
    root: Path,
    today: date | None = None,
) -> dict[str, dict[str, Any]]:
    """Resolve every workspace package's CI policy from its own manifest.

    A package with no `[package.metadata.ci]` defaults to `gates = true` and
    the L1 tier. Non-gating is never inferred from zero observed tests.
    """
    today = today or date.today()
    policy: dict[str, dict[str, Any]] = {}
    for package in packages.values():
        name = package["name"]
        ci = (package.get("metadata") or {}).get("ci") or {}
        if not isinstance(ci, dict):
            raise RuntimeError(f"package '{name}' [package.metadata.ci] must be an object")
        validate_package_ci(name, ci, runner_labels, root, today)

        tests = ci.get("tests", {})
        record: dict[str, Any] = {
            "package": name,
            "gates": ci.get("gates", True),
            "tiers": tests.get("tiers", ["L1"]),
            "l2_backends": tests.get("l2-backends", []),
            "features": tests.get("features", []),
            "all_features": tests.get("all-features", False),
            "l1_include_slow": tests.get("l1-include-slow", False),
            "runner_tools": tests.get("runner-tools", []),
            "companion_suites": tests.get("companion-suites", []),
            "native": ci.get("native", {}),
        }
        if not record["gates"]:
            record["exclusion"] = {
                "exclusion_class": ci["exclusion-class"],
                "owner": ci["owner"],
                "reason": ci["reason"],
                "expiry": ci.get("expiry"),
            }
        policy[name] = record
    return policy


# ---------------------------------------------------------------------------
# Workspace graph
# ---------------------------------------------------------------------------


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


def direct_dependents(
    seeds: set[str], metadata: dict[str, Any], packages: dict[str, dict[str, Any]]
) -> set[str]:
    """The changed packages plus their DIRECT reverse Cargo dependencies.

    Deliberately not transitive (decided 2026-08-13, cost): a change two hops
    away is not re-tested here. The accepted trade-off is that a regression
    observable only through an intermediate package lands silently and
    surfaces the next time the intermediate itself is touched (or on a
    `workflow_dispatch` full run).
    """
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
    for package_id in seeds:
        affected.update(reverse_dependencies.get(package_id, set()))
    return affected


def build_closure(
    seed: str, metadata: dict[str, Any], packages: dict[str, dict[str, Any]]
) -> set[str]:
    """Forward dependency closure for build provisioning.

    Testing a package compiles its own dev-dependencies and, transitively, the
    normal and build dependencies of everything reached. Dev-dependencies of
    *dependencies* are never built, so they do not propagate. Target
    restrictions are deliberately not evaluated: a library needed on one
    platform is installed on all, which is the safe direction and matches how
    the declarations were consumed before.

    The closure comes from the workspace-unified `cargo metadata` resolve, not
    from any job's declared feature set: an OPTIONAL edge (e.g.
    biscuit-speaks -> playa) is present here only because some member enables
    it. If every enabling edge disappears, the union silently loses the
    optional package's native requirements. `test_affected_scope.py` pins the
    known instance (biscuit-speaks -> playa) against the real metadata so the
    coupling fails loudly instead of silently.
    """
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}

    def edges(package_id: str, include_dev: bool) -> list[str]:
        node = nodes.get(package_id)
        if node is None:
            return []
        reached = []
        for dependency in node.get("deps", []):
            kinds = {kind.get("kind") for kind in dependency.get("dep_kinds", [])}
            if include_dev or kinds != {"dev"}:
                reached.append(dependency["pkg"])
        return reached

    closure = {seed}
    pending = [seed]
    first = True
    while pending:
        current = pending.pop()
        for dependency in edges(current, include_dev=first and current == seed):
            if dependency in packages and dependency not in closure:
                closure.add(dependency)
                pending.append(dependency)
        if current == seed:
            first = False
    return closure


def validate_no_shadow_workspaces(metadata: dict[str, Any], root: Path) -> None:
    """No directory may re-declare a root workspace member in its own workspace.

    A nested ``[workspace]`` whose members are also root members makes the same
    package resolve into two different workspaces depending on the current
    directory. Cargo picks the nearest ancestor workspace root, so ``cd <dir> &&
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


def parse_lockfile(text: str) -> dict[tuple[str, str], frozenset[str]] | None:
    """Read `Cargo.lock` into `{(name, version): dependency crate names}`.

    Hand-parsed rather than run through a TOML library: the lockfile grammar is
    a fixed shape Cargo emits, and this runs before any dependency is available
    to install. Returns `None` when nothing parsed, which callers must treat as
    "undecidable" rather than "no dependencies changed".

    Dependency entries are reduced to the crate name, dropping the version and
    source that Cargo appends when a name is ambiguous. That merges two major
    versions of one crate into a single node, which over-approximates the
    impacted set — the safe direction.
    """
    entries: dict[tuple[str, str], frozenset[str]] = {}
    name: str | None = None
    version: str | None = None
    dependencies: set[str] = set()
    in_dependencies = False

    def flush() -> None:
        nonlocal name, version, dependencies
        if name is not None and version is not None:
            entries[(name, version)] = frozenset(dependencies)
        name = None
        version = None
        dependencies = set()

    for raw_line in text.splitlines():
        line = raw_line.strip()

        if line.startswith("["):
            flush()
            in_dependencies = False
            continue

        if in_dependencies:
            if line.startswith("]"):
                in_dependencies = False
                continue
            candidate = line.rstrip(",").strip('"').strip()
            if candidate:
                dependencies.add(candidate.split()[0])
            continue

        if line.startswith("dependencies = ["):
            # `dependencies = []` closes on its own line.
            in_dependencies = not line.rstrip().endswith("]")
            continue
        if line.startswith("name = "):
            name = line.split("=", 1)[1].strip().strip('"')
        elif line.startswith("version = "):
            version = line.split("=", 1)[1].strip().strip('"')

    flush()
    return entries or None


def lockfile_impacted_names(
    base_text: str | None, head_text: str | None, workspace_names: set[str]
) -> set[str] | None:
    """Workspace crates a lockfile change can reach, or `None` if undecidable.

    `None` means the caller must fall back to full scope. That is the outcome
    whenever the base revision's lockfile is unavailable or either side fails to
    parse: an unreadable diff is not evidence that nothing changed.
    """
    if base_text is None or head_text is None:
        return None

    base = parse_lockfile(base_text)
    head = parse_lockfile(head_text)
    if base is None or head is None:
        return None

    changed = {
        name
        for (name, version) in set(base) | set(head)
        if base.get((name, version)) != head.get((name, version))
    }

    # Reverse edges from BOTH revisions. A crate deleted outright has no
    # dependents left in the head graph, but the packages that depended on it in
    # the base are exactly the ones its removal affects.
    dependents: dict[str, set[str]] = {}
    for graph in (base, head):
        for (name, _version), dependencies in graph.items():
            for dependency in dependencies:
                dependents.setdefault(dependency, set()).add(name)

    impacted = set(changed)
    pending = list(changed)
    while pending:
        current = pending.pop()
        for dependent in dependents.get(current, ()):
            if dependent not in impacted:
                impacted.add(dependent)
                pending.append(dependent)

    return impacted & workspace_names


def changed_package_ids(
    files: list[str],
    root: Path,
    packages: dict[str, dict[str, Any]],
    lock_impacted: set[str] | None = None,
) -> tuple[set[str], bool]:
    """Map changed paths to the packages they can reach.

    A file inside a package's directory selects that package. A file that is
    under no package directory selects every package beneath its top-level
    directory (a shared justfile, a fixture at the directory root) — pure path
    scoping, not policy. Anything else (`docs/`, a root README) selects nothing.
    """
    directories = package_directories(root, packages)
    seeds: set[str] = set()

    for raw_file in files:
        normalized = raw_file.replace("\\", "/").removeprefix("./")
        if normalized in GLOBAL_PATHS or normalized.startswith(GLOBAL_PREFIXES):
            return set(packages), True

        if normalized == LOCKFILE_PATH:
            # Undecidable diff — no base lockfile, or one side did not parse.
            if lock_impacted is None:
                return set(packages), True
            for package_id, package in packages.items():
                if package["name"] in lock_impacted:
                    seeds.add(package_id)
            continue

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
        for directory, package_id in directories:
            if directory.parts and directory.parts[0] == top_level:
                seeds.add(package_id)

    return seeds, False


# ---------------------------------------------------------------------------
# Scope assembly
# ---------------------------------------------------------------------------


def feature_args(record: dict[str, Any], package: str) -> str:
    """The Cargo feature flags a package's check/test/archive invocations share.

    One feature contract, forwarded consistently to compile-check, archive
    construction, and the canonical test recipe — a package must not
    compile-check one feature set and test another.
    """
    if record["all_features"]:
        return "--all-features"
    if record["features"]:
        return f"--features {','.join(record['features'])}"
    return ""


def matrix_record(
    record: dict[str, Any],
    native: dict[str, list[str]],
    environments: list[dict[str, Any]],
) -> dict[str, Any]:
    """The workflow-facing shape of one gating package's policy."""
    features = feature_args(record, record["package"])
    check_args = f"-p {record['package']}" + (f" {features}" if features else "")

    native_environments = [
        environment["name"]
        for environment in environments
        if not capability(environment, "archive_only")
        and environment["runner"] == environment["name"]
    ]
    return {
        "package": record["package"],
        "check_args": check_args,
        "test_args": features,
        "l1_include_slow": record["l1_include_slow"],
        "tiers": record["tiers"],
        "l2_backends": record["l2_backends"],
        "runner_tools": record["runner_tools"],
        "companion_suites": record["companion_suites"],
        "native": native,
        "native_environments": native_environments,
        "check_os": CHECK_OS,
        "l2_environments": (
            [
                environment["name"]
                for environment in environments
                if any(
                    backend_hostable(environment, backend)
                    for backend in record["l2_backends"]
                )
            ]
            if "L2" in record["tiers"]
            else []
        ),
        "browser_environments": (
            [
                environment["name"]
                for environment in environments
                if capability(environment, "headless_browser")
            ]
            if "browser" in record["tiers"]
            else []
        ),
        "node_environments": (
            [
                environment["name"]
                for environment in environments
                if capability(environment, "node_pnpm")
            ]
            if record["companion_suites"]
            or {"node-22", "pnpm-10"} & set(record["runner_tools"])
            else []
        ),
        "wsl": any(
            capability(environment, "archive_only") for environment in environments
        ),
    }


def estimate_jobs(matrix: list[dict[str, Any]]) -> int:
    """The expanded job-count estimate for the package fan-out.

    An estimate, not an exact count: each `wsl` entry spawns two jobs (the
    Linux archive builder plus the Windows-hosted guest), and the reusable
    workflows may add legs this does not model. The enforced limit is the
    matrix length (checked against MATRIX_LIMIT), not this number.
    """
    return sum(
        len(entry["check_os"])
        + len(entry["native_environments"])
        + (2 if entry["wsl"] else 0)
        + 1  # lint
        + len(entry["l2_environments"])
        + len(entry["browser_environments"])
        for entry in matrix
    )


def policy_record(record: dict[str, Any]) -> dict[str, Any]:
    """The rollup-facing shape of one impacted package's policy."""
    shaped = {
        "package": record["package"],
        "gates": record["gates"],
        "tiers": record["tiers"],
        "l2_backends": record["l2_backends"],
        # The rollup needs to know a companion suite was DECLARED: a green
        # Rust JUnit report must not hide a companion that never ran (R12).
        "companion_suites": record["companion_suites"],
    }
    if not record["gates"]:
        shaped["exclusion"] = record["exclusion"]
    return shaped


def calculate_scope(
    files: list[str],
    root: Path,
    metadata: dict[str, Any],
    environments: list[dict[str, Any]],
    policy: dict[str, dict[str, Any]],
    force_all: bool = False,
    base_lockfile: str | None = None,
) -> dict[str, Any]:
    packages = workspace_packages(metadata)

    if force_all:
        affected_ids = set(packages)
        full_scope = True
    else:
        lock_impacted = None
        if any(
            raw.replace("\\", "/").removeprefix("./") == LOCKFILE_PATH for raw in files
        ):
            head_lockfile = None
            lock_path = root / LOCKFILE_PATH
            if lock_path.is_file():
                head_lockfile = lock_path.read_text(encoding="utf-8")
            lock_impacted = lockfile_impacted_names(
                base_lockfile,
                head_lockfile,
                {package["name"] for package in packages.values()},
            )

        seeds, full_scope = changed_package_ids(files, root, packages, lock_impacted)
        affected_ids = direct_dependents(seeds, metadata, packages)

    impacted = sorted(packages[package_id]["name"] for package_id in affected_ids)

    matrix = []
    for name in impacted:
        record = policy[name]
        if not record["gates"]:
            # A `gates = false` package is still selected (reverse-dependency
            # closure and coverage see it) — it just launches no jobs.
            continue
        package_id = next(
            package_id for package_id in affected_ids if packages[package_id]["name"] == name
        )
        closure = build_closure(package_id, metadata, packages)
        native: dict[str, list[str]] = {}
        for member_id in closure:
            member = policy[packages[member_id]["name"]]
            for os_name, declared in member["native"].items():
                bucket = native.setdefault(os_name, [])
                for entry in declared:
                    if entry not in bucket:
                        bucket.append(entry)
        # Sorted so scope.json is byte-stable across runs (set-iteration order
        # is PYTHONHASHSEED-dependent); nothing consumes the order, but noisy
        # diffs obscure real scope changes.
        native = {os_name: sorted(bucket) for os_name, bucket in native.items()}
        matrix.append(matrix_record(record, native, environments))

    job_estimate = estimate_jobs(matrix)
    if len(matrix) > MATRIX_LIMIT:
        raise RuntimeError(
            f"the package matrix has {len(matrix)} entries, over GitHub's "
            f"{MATRIX_LIMIT}-job matrix ceiling; the fan-out must be grouped"
        )

    change_class, preflight_os, preflight_reason = classify_preflight(
        files, matrix, full_scope, force_all, environments
    )

    top_dirs = {
        Path(packages[package_id]["manifest_path"]).parent.relative_to(root.resolve()).parts[0]
        for package_id in affected_ids
    }

    flags = {
        directory: directory in top_dirs
        for directory in ("claudine", "darkmatter", "sniff", "biscuit-tui", "playa")
    }
    flags["ci_tooling"] = any(
        raw.replace("\\", "/").removeprefix("./").startswith(CI_TOOLING_PREFIXES)
        for raw in files
    )

    return {
        "packages": impacted,
        "full_scope": full_scope,
        "change_class": change_class,
        "preflight_os": preflight_os,
        "preflight_reason": preflight_reason,
        "matrix": matrix,
        "policy": [policy_record(policy[name]) for name in impacted],
        "job_estimate": job_estimate,
        "flags": flags,
    }


def classify_preflight(
    files: list[str],
    matrix: list[dict[str, Any]],
    full_scope: bool,
    force_all: bool,
    environments: list[dict[str, Any]],
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

    if matrix:
        # Preflight runs on RUNNER labels, so each environment is resolved to
        # the runner that hosts it — `wsl2-ubuntu` preflights on
        # `windows-latest`.
        runner_of = {environment["name"]: environment["runner"] for environment in environments}
        os_set = {SCOPE_HOST_OS}
        for entry in matrix:
            for environment in entry["native_environments"]:
                os_set.add(runner_of.get(environment, environment))
            if entry["wsl"]:
                os_set.update(
                    runner_of[environment["name"]]
                    for environment in environments
                    if capability(environment, "archive_only")
                )
        reason = (
            f"package-local change across {len(matrix)} package(s); "
            "preflight covers the scope host plus the runner OS hosting each "
            "package's required environments"
        )
        return "package", sorted(os_set), reason

    return (
        "documentation",
        [SCOPE_HOST_OS],
        "no build/test packages affected; preflight runs on the scope host only",
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--all", action="store_true", help="select the full workspace")
    parser.add_argument(
        "--base-lockfile",
        help=(
            "the base revision's Cargo.lock. Without it a lockfile change is "
            "undecidable and selects the full workspace"
        ),
    )
    parser.add_argument("files", nargs="*", help="changed repository-relative paths")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    environments = load_environments(ENVIRONMENTS_CONFIG)
    runner_labels = {environment["runner"] for environment in environments}
    metadata = load_metadata(ROOT)
    validate_no_shadow_workspaces(metadata, ROOT)
    policy = package_ci_policy(workspace_packages(metadata), runner_labels, ROOT)
    base_lockfile = None
    if args.base_lockfile:
        candidate = Path(args.base_lockfile)
        if candidate.is_file():
            base_lockfile = candidate.read_text(encoding="utf-8")

    scope = calculate_scope(
        args.files, ROOT, metadata, environments, policy, args.all, base_lockfile
    )
    print(json.dumps(scope, separators=(",", ":")))


if __name__ == "__main__":
    main()

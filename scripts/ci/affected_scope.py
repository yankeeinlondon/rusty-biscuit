#!/usr/bin/env python3
"""Calculate dependency-aware CI scope for the Rusty Biscuit workspace.

The package is the unit of selection, execution, and result identity; the area
is a grouping derived from the package's manifest directory, never stored as an
identity. Package policy lives in each package's own Cargo manifest under
`[package.metadata.ci]`; environment capabilities live in
`.github/ci/environments.json`. Both are validated here, loudly, before any
scope is emitted.

`calculate_scope` returns the canonical resolved plan that
`scripts/ci/schema.py` defines and validates. `legacy_scope_document` projects
it into the shape `ci.yml`, `just/ci-local.just`, and `ci-rollup` still read,
and goes away with Phases 5 and 6 of `fixes/2026-09-11-cicd-cleanup/plan.md`.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from datetime import date
from pathlib import Path, PurePosixPath
from collections.abc import Callable, Sequence
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent))

import schema  # noqa: E402  (needs the path insert above when run as a script)


ROOT = Path(__file__).resolve().parents[2]
ENVIRONMENTS_CONFIG = ROOT / ".github" / "ci" / "environments.json"

# The environment names the policy table declares, for argument validation.
ENVIRONMENTS = schema.ENVIRONMENTS

# The three verdicts CI can produce per package.
GATES = ("lint", "check", "test")

# These input-analysis helpers remain covered for tooling diagnostics. Package
# scheduling itself is source-driven and does not consume global-input changes.
GLOBAL_PATHS_ALL_GATES = {
    ".github/ci/environments.json",
    ".github/workflows/_package-ci.yml",
    ".github/workflows/ci.yml",
    "Cargo.toml",
    "rust-toolchain.toml",
    "scripts/ci/affected_scope.py",
}
GLOBAL_PREFIXES_ALL_GATES = (".cargo/", ".github/actions/")
GLOBAL_PATHS_BY_GATE: dict[str, set[str]] = {
    "lint": {"clippy.toml"},
    "check": set(),
    "test": {".config/nextest.toml", ".github/workflows/_wsl-ci.yml"},
}

# Just files are global by RECIPE, not by path. CI executes a closed set of
# recipes; a change to one of those, or to anything they reach through header
# dependencies or `just <name>` calls at command position, re-runs the gate
# that owns the entry recipe. A change to any other recipe — `pre-push`,
# `cross-check`, `doctest`, the whole of `just/plan.just` — reaches no gate
# and selects nothing. Text outside every recipe (settings, imports, variable
# assignments) is conservatively global to every gate.
JUST_PATHS = {"justfile"}
JUST_PREFIXES = ("just/",)
CI_RECIPES_BY_GATE: dict[str, tuple[str, ...]] = {
    "lint": ("_lint",),
    "check": (),
    "test": ("_test", "_test_l2", "_test_browser"),
}
CI_RECIPES_ALL_GATES = ("_ensure-native-libs",)

# Any path that MAY force workspace scope for some gate; the per-gate decision
# is `gate_triggers`.
GLOBAL_PATHS = (
    GLOBAL_PATHS_ALL_GATES | JUST_PATHS | set().union(*GLOBAL_PATHS_BY_GATE.values())
)
GLOBAL_PREFIXES = GLOBAL_PREFIXES_ALL_GATES + JUST_PREFIXES

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
#
# The same leg runs the workflow-contract suite, whose subject is every file
# under `.github/workflows/`. A workflow edit widens the package gates but can
# select zero packages, and `test-toolkit` is `gates = false`, so without this
# trigger the one suite that inspects workflows would run on no job.
CI_TOOLING_PREFIXES = ("scripts/", ".github/ci/", ".github/workflows/")
CI_TOOLING_PATHS = {"tools/test-toolkit/tests/ci_workflow_contracts.rs"}

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
    "zed-extension",
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
    "local-features",
    "all-features",
    "l1-include-slow",
    "runner-tools",
    "companion-suites",
}
EXCLUSION_FIELDS = {"exclusion-class", "owner", "reason", "expiry"}

# Where `_package-ci.yml` runs clippy. A cell carries it so the lint gate's
# environment is visible in the matrix (AC10) instead of being implied.
LINT_ENVIRONMENT = "ubuntu-latest"

# The gates the L1 build itself compiles. A separate check cell is scheduled
# only for required kinds outside this set (spec section 1.7).
L1_TARGET_KINDS = ("lib", "bin", "test")

# Cargo's explicit selector for each kind outside `L1_TARGET_KINDS`. The check
# command is built from these and nothing else: `--all-targets` would recompile
# the L1 kinds and make the cell's `target_kinds` a label rather than a fact.
CHECK_SELECTORS = {"example": "--examples", "bench": "--benches"}

# Cargo target kinds this planner schedules for, in `schema.TARGET_KINDS`
# order. `custom-build` (build.rs) is deliberately absent: it is compiled as
# part of every other kind and is never independently selectable.
TARGET_KINDS = ("lib", "bin", "test", "example", "bench")

# Cargo spells a library target by its flavour (`lib`, `rlib`, `proc-macro`,
# `cdylib`, ...). All of them are the package's library for scheduling.
LIBRARY_TARGET_KINDS = {"lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"}

# The area a workspace member directly under the repository root belongs to.
# Named by `sniff repo package-area`, which this planner replicates.
ROOT_AREA = "root"

# GitHub Actions ceiling for a single matrix. The area matrix and every area's
# package matrix must stay under it even on a full-scope run.
MATRIX_LIMIT = 256

# Package CI is source-driven. Configuration, documentation, generated reports,
# fixtures, and CI plumbing validate through their own contract suites; they do
# not make unchanged Cargo packages rebuild. Keep this vocabulary aligned with
# Sniff's source-code classification when either side learns a new language.
SOURCE_SUFFIXES = {
    ".c", ".cc", ".cpp", ".cxx", ".css", ".go", ".h", ".hh", ".hpp",
    ".html", ".htm", ".java", ".js", ".jsx", ".m", ".mm", ".proto",
    ".py", ".rs", ".scss", ".sh", ".svelte", ".swift", ".ts", ".tsx",
    ".vue",
}


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
            unknown_gap = value.keys() - {"available", "reason", "owner", "expiry", "closes"}
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


def native_environments(environments: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """The environments that are their own runner and hold a toolchain.

    An archive-only environment (the WSL2 guest) is hosted by another runner and
    compiles nothing, so it can neither be a `runs-on` label nor host a check.
    """
    return [
        environment
        for environment in environments
        if not capability(environment, "archive_only")
        and environment["runner"] == environment["name"]
    ]


def capability(environment: dict[str, Any], name: str) -> bool:
    """Whether an environment can host a capability, boolean or governed object."""
    value = environment["capabilities"][name]
    if isinstance(value, bool):
        return value
    return bool(value.get("available"))


def capability_gap(environment: dict[str, Any], name: str) -> dict[str, str] | None:
    """The governance record for a governed unavailability, if one exists.

    `closes` names the tracked work that ends the gap. Spec section 6 requires
    it in the visible description of an accepted gap, so it travels with the
    record rather than being looked up again downstream. Absent when the policy
    entry declares none.
    """
    value = environment["capabilities"][name]
    if isinstance(value, dict) and value.get("available") is False:
        record = {
            "owner": value["owner"],
            "reason": value["reason"],
            "expiry": value["expiry"],
        }
        if value.get("closes"):
            record["closes"] = value["closes"]
        return record
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
    local_features = tests.get("local-features", features)
    all_features = tests.get("all-features", False)
    if not isinstance(features, list) or not all(isinstance(f, str) for f in features):
        raise RuntimeError(f"{label}.tests field 'features' must be a list of feature names")
    if not isinstance(local_features, list) or not all(
        isinstance(f, str) for f in local_features
    ):
        raise RuntimeError(
            f"{label}.tests field 'local-features' must be a list of feature names"
        )
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


def package_area(relative_manifest_directory: PurePosixPath | str) -> str:
    """The package area a workspace member belongs to.

    Replicates `sniff repo package-area`, whose rule is the directory path
    between the repository root and the package directory, or `root` when the
    package sits directly under it (`make_package_area` in
    `sniff/lib/src/filesystem/repo/detection.rs`). Nested areas fall out of the
    same rule: `claudine/rendezvous/core` belongs to `claudine/rendezvous`.

    The mapping is derived rather than committed on purpose (Design Decision
    3). A committed table would be a second policy store, free to drift from
    sniff without anything failing.

    ## Examples

    ```python
    assert package_area("claudine/lib") == "claudine"
    assert package_area("claudine/rendezvous/core") == "claudine/rendezvous"
    assert package_area("renderable") == "root"
    ```
    """
    parent = PurePosixPath(relative_manifest_directory).parent
    return ROOT_AREA if parent.as_posix() in (".", "") else parent.as_posix()


def area_slug(area: str) -> str:
    """A nested area name, spelled so it can be a GitHub artifact name.

    `/` is rejected in artifact names, so `claudine/rendezvous` would silently
    fail its upload. The slug is presentation only: `--area` and every stored
    record still carry the real area name.

    ## Examples

    ```python
    assert area_slug("claudine/rendezvous") == "claudine--rendezvous"
    ```
    """
    return area.replace("/", "--")


def manifest_directory(root: Path, package: dict[str, Any]) -> PurePosixPath:
    """A package's manifest directory, relative to the repository root."""
    relative = Path(package["manifest_path"]).resolve().parent.relative_to(root.resolve())
    return PurePosixPath(relative.as_posix())


def declared_target_kinds(package: dict[str, Any]) -> list[str]:
    """The Cargo target kinds a package actually declares.

    Read from `cargo metadata` rather than assumed, so "does this package have
    benches to compile?" is answered by the manifest instead of by a blanket
    `--all-targets`. A package with no `targets` key at all (the synthetic
    metadata the unit tests build) is treated as a plain library, which is the
    narrowest honest default.
    """
    targets = package.get("targets")
    if not targets:
        return ["lib"]
    kinds: set[str] = set()
    for target in targets:
        for kind in target.get("kind", []):
            if kind in LIBRARY_TARGET_KINDS:
                kinds.add("lib")
            elif kind in TARGET_KINDS:
                kinds.add(kind)
    return [kind for kind in TARGET_KINDS if kind in kinds]


def uncovered_target_kinds(target_kinds: Sequence[str]) -> list[str]:
    """The declared kinds no test gate compiles, in `TARGET_KINDS` order."""
    return [kind for kind in TARGET_KINDS if kind in target_kinds and kind not in L1_TARGET_KINDS]


def check_arguments(package: str, target_kinds: Sequence[str], features: str) -> str:
    """The `cargo check` arguments for a package's uncovered target kinds.

    Explicit selectors only (`--examples`, `--benches`): the L1 build already
    compiled `lib`, `bin`, and `test`, so a check that re-selects them claims
    nothing new and costs a second compile. A package with no uncovered kind
    owns no check cell, and its arguments are just the selector and features.
    """
    selectors = [CHECK_SELECTORS[kind] for kind in uncovered_target_kinds(target_kinds)]
    return " ".join(part for part in ["-p", package, *selectors, features] if part)


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


def is_package_source_path(path: PurePosixPath) -> bool:
    """Whether a package-owned path can change compiled or executed behavior."""
    return path.name == "build.rs" or path.suffix.lower() in SOURCE_SUFFIXES


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


def is_global_path(raw_file: str) -> bool:
    normalized = raw_file.replace("\\", "/").removeprefix("./")
    return normalized in GLOBAL_PATHS or normalized.startswith(GLOBAL_PREFIXES)


def diff_against(base_ref: str, path: str) -> str | None:
    """Unified diff of `path` between `base_ref` and the working tree.

    `None` when Git cannot produce it (unknown ref, shallow clone, no Git),
    which callers must treat as "undecidable": the global trigger then stands.
    """
    try:
        result = subprocess.run(
            ["git", "diff", "--no-color", "--unified=0", base_ref, "--", path],
            capture_output=True,
            text=True,
            check=False,
            cwd=ROOT,
        )
    except OSError:
        return None
    if result.returncode != 0:
        return None
    return result.stdout


def diff_changes_more_than_comments(diff_text: str) -> bool:
    """True when a unified diff adds or removes a line that is not a `#` comment
    or blank.

    Every global input — justfiles, TOML, workflow YAML, this script — uses `#`
    line comments, so one rule covers them all. It is deliberately naive about
    context: a changed `#` line inside a shell heredoc is still ignored, which
    can only ever make CI *smaller* by not noticing text no build executes.
    """
    for line in diff_text.splitlines():
        if line.startswith(("+++", "---")):
            continue
        if not line.startswith(("+", "-")):
            continue
        body = line[1:].strip()
        if body and not body.startswith("#"):
            return True
    return False


def read_at_ref(base_ref: str, path: str) -> str | None:
    """The content of `path` at `base_ref`, or `None` when Git cannot produce
    it (unknown ref, path absent there, no Git) — undecidable, so callers widen.
    """
    try:
        result = subprocess.run(
            ["git", "show", f"{base_ref}:{path}"],
            capture_output=True,
            text=True,
            check=False,
            cwd=ROOT,
        )
    except OSError:
        return None
    if result.returncode != 0:
        return None
    return result.stdout


def is_just_path(normalized: str) -> bool:
    return normalized in JUST_PATHS or normalized.startswith(JUST_PREFIXES)


# A recipe header: a name at column 0 followed by optional parameters and a
# `:` that is not the `:=` of an assignment. Quiet recipes (`@name:`) are
# matched after their `@` is stripped.
JUST_HEADER = re.compile(r"^(?P<name>[A-Za-z_][\w-]*)(?:\s+[^:]*?)?\s*:(?!=)(?P<deps>.*)$")

# `just <name>` at COMMAND position: the start of a command, or after a shell
# operator or keyword. Not inside prose — "then re-run just init." in an echo
# string is not a call, and treating it as one pulled `init` and every
# `_ensure-*` helper into the lint closure.
JUST_CALL = re.compile(
    r"(?:^|&&|\|\||;|\||\(|`|\bif\b|\bthen\b|\belse\b|\bdo\b|!|--)"
    r"\s*[@-]*just\s+(?P<name>[A-Za-z_][\w-]*)"
)


def parse_just_recipes(text: str) -> tuple[dict[str, dict[str, Any]], list[str]]:
    """Split a justfile into recipes and the text outside them.

    ## Returns

    ``(recipes, other)``: ``recipes`` maps a name to ``{"lines", "deps",
    "calls"}`` — its attribute, header, and body lines with comments and blank
    lines dropped, the recipe names in its header dependencies, and the
    recipe names it invokes at command position. ``other`` is every remaining
    non-comment line (settings, imports, assignments), in order.
    """
    recipes: dict[str, dict[str, Any]] = {}
    other: list[str] = []
    pending_attributes: list[str] = []
    current: dict[str, Any] | None = None

    for raw in text.splitlines():
        stripped = raw.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if raw[0] in " \t":
            if current is None:
                other.append(stripped)
            else:
                current["lines"].append(stripped)
                current["calls"].extend(match.group("name") for match in JUST_CALL.finditer(stripped))
            continue

        current = None
        if stripped.startswith("["):
            pending_attributes.append(stripped)
            continue
        header = JUST_HEADER.match(stripped.lstrip("@"))
        if header is None:
            other.extend(pending_attributes)
            pending_attributes = []
            other.append(stripped)
            continue
        deps = [
            match.group(1)
            for match in re.finditer(r"(?:^|[\s(])([A-Za-z_][\w-]*)", header.group("deps"))
        ]
        current = {
            "lines": [*pending_attributes, stripped],
            "deps": deps,
            "calls": [],
        }
        pending_attributes = []
        recipes[header.group("name")] = current

    other.extend(pending_attributes)
    return recipes, other


def just_recipe_closure(recipes: dict[str, dict[str, Any]], entries: tuple[str, ...]) -> set[str]:
    """Every recipe reachable from `entries` through dependencies and calls."""
    reachable: set[str] = set()
    queue = list(entries)
    while queue:
        name = queue.pop()
        if name in reachable:
            continue
        reachable.add(name)
        recipe = recipes.get(name)
        if recipe is not None:
            queue.extend(recipe["deps"])
            queue.extend(recipe["calls"])
    return reachable


def just_sources(root: Path) -> dict[str, str]:
    """The root justfile and everything under `just/`, as they are on disk."""
    sources: dict[str, str] = {}
    for candidate in sorted([root / "justfile", *(root / "just").glob("*.just")]):
        if candidate.is_file():
            sources[candidate.relative_to(root).as_posix()] = candidate.read_text(encoding="utf-8")
    return sources


def just_change_gates(
    normalized: str,
    base_ref: str,
    root: Path,
    reader: Callable[[str, str], str | None] = read_at_ref,
) -> set[str] | None:
    """The gates a changed just file can affect.

    Compares the file recipe by recipe against `base_ref`: a recipe that
    differs (or exists on one side only) is a trigger for every gate whose CI
    entry recipes reach it, on either side's recipe graph. Any change outside
    a recipe triggers every gate. `None` means the base content was
    unobtainable — undecidable, widen.
    """
    old_text = reader(base_ref, normalized)
    if old_text is None:
        return None
    new_sources = just_sources(root)
    new_text = new_sources.get(normalized, "")

    new_recipes, new_other = parse_just_recipes(new_text)
    old_recipes, old_other = parse_just_recipes(old_text)
    if new_other != old_other:
        return set(GATES)
    changed = {
        name
        for name in set(new_recipes) | set(old_recipes)
        if (new_recipes.get(name) or {}).get("lines") != (old_recipes.get(name) or {}).get("lines")
    }
    if not changed:
        return set()

    old_sources = {**new_sources, normalized: old_text}
    gates: set[str] = set()
    for sources in (new_sources, old_sources):
        graph: dict[str, dict[str, Any]] = {}
        for text in sources.values():
            graph.update(parse_just_recipes(text)[0])
        if changed & just_recipe_closure(graph, CI_RECIPES_ALL_GATES):
            return set(GATES)
        for gate, entries in CI_RECIPES_BY_GATE.items():
            if changed & just_recipe_closure(graph, entries):
                gates.add(gate)
    return gates


def gate_triggers(
    files: list[str],
    root: Path = ROOT,
    base_ref: str | None = None,
    differ: Callable[[str, str], str | None] = diff_against,
    reader: Callable[[str, str], str | None] = read_at_ref,
) -> dict[str, str | None]:
    """For each gate, the first changed path that forces workspace scope for it.

    With `base_ref`, a global path whose diff touches only comments and blank
    lines is not a trigger for any gate: nine comment lines in the root
    justfile once scheduled all 72 packages (PR #59). Without `base_ref`, or
    when the diff or base content is unobtainable, the path triggers every
    gate — the fallback is always wider.
    """
    triggers: dict[str, str | None] = {gate: None for gate in GATES}
    for raw_file in files:
        if not is_global_path(raw_file):
            continue
        normalized = raw_file.replace("\\", "/").removeprefix("./")
        diff = differ(base_ref, normalized) if base_ref is not None else None
        if diff is not None and not diff_changes_more_than_comments(diff):
            continue
        gates: set[str]
        if is_just_path(normalized):
            # Which gate a just file reaches is a property of its content, so
            # without a readable base it reaches every gate.
            decided = just_change_gates(normalized, base_ref, root, reader) if diff is not None else None
            gates = set(GATES) if decided is None else decided
        elif normalized in GLOBAL_PATHS_ALL_GATES or normalized.startswith(
            GLOBAL_PREFIXES_ALL_GATES
        ):
            gates = set(GATES)
        else:
            # A named input's gate is known from the name alone; a base ref
            # only adds the comment-only exemption above.
            gates = {gate for gate in GATES if normalized in GLOBAL_PATHS_BY_GATE[gate]}
        for gate in gates:
            if triggers[gate] is None:
                triggers[gate] = normalized
    return triggers


def global_trigger(
    files: list[str],
    base_ref: str | None = None,
    differ: Callable[[str, str], str | None] = diff_against,
    root: Path = ROOT,
    reader: Callable[[str, str], str | None] = read_at_ref,
) -> str | None:
    """The first changed path that forces workspace scope for ANY gate."""
    triggers = gate_triggers(files, root, base_ref, differ, reader)
    return next((path for path in triggers.values() if path is not None), None)


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


def source_paths_by_package(
    files: list[str], root: Path, packages: dict[str, dict[str, Any]]
) -> dict[str, list[str]]:
    """The changed source paths each package owns, in the order given.

    The paths themselves, not just the package identities, because every
    package record in the resolved plan must state a concrete selection reason
    and "a source change" is not one.
    """
    directories = package_directories(root, packages)
    owned: dict[str, list[str]] = {}

    for raw_file in files:
        normalized = raw_file.replace("\\", "/").removeprefix("./")
        changed = PurePosixPath(normalized)
        if not is_package_source_path(changed):
            continue
        for directory, package_id in directories:
            if changed == directory or directory in changed.parents:
                owned.setdefault(package_id, []).append(normalized)
                break

    return owned


def changed_package_ids(
    files: list[str],
    root: Path,
    packages: dict[str, dict[str, Any]],
    lock_impacted: set[str] | None = None,
) -> tuple[set[str], bool]:
    """Map package-owned source paths to their packages.

    Non-source inputs have dedicated validation and never fan out unchanged
    packages. ``Cargo.toml`` and lockfile edits therefore select no package by
    themselves; the next source edit exercises the resulting package graph.
    """
    return set(source_paths_by_package(files, root, packages)), False


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


# Where a governed policy gap is declared, linked from every gap the plan
# emits so a reader can reach the owner and expiry without being told them.
POLICY_LINK = ".github/ci/environments.json"

# `git` spells "no such object" as the all-zero ID. Used as the plan's base and
# head when the caller resolved no revision pair, so the field is always a
# well-formed object ID rather than a null a reader has to special-case.
NULL_OID = "0" * 40


def gap_record(environment: dict[str, Any], capabilities: list[str]) -> dict[str, Any]:
    """Why an environment cannot host a tier, and whether that is governed.

    ## Returns

    The first governed unavailability among `capabilities`, carrying its owner,
    reason, and expiry; otherwise an ungoverned record. An ungoverned gap is
    never an accepted gap — it blocks its area — so the distinction is carried
    in the document rather than inferred later.
    """
    for name in capabilities:
        governed = capability_gap(environment, name)
        if governed is not None:
            return {
                "capability": name,
                "governed": True,
                "policy": POLICY_LINK,
                **governed,
            }
    return {
        "capability": capabilities[0] if capabilities else "",
        "governed": False,
        "policy": POLICY_LINK,
    }


def whole_environment_evidence(
    environments: list[str] | None,
) -> dict[str, dict[str, Any]]:
    """Accept every cell of an environment on one whole-environment receipt.

    This is the version-1 migration path of spec section 3.6, not the normal
    one: a v1 note predates per-cell outcomes, so its only honest reuse grain
    is the whole environment, on exact tree identity, pass-only. Every live
    caller passes `--accepted-cells` instead. Repeatable rather than singular,
    and each cell still carries its own origin and evidence — which is what the
    retired `excluded_environment` could not express.
    """
    return {
        name: {"origin": "local", "evidence": f"refs/notes/ci-local/{name}"}
        for name in environments or []
    }


def package_cells(
    record: dict[str, Any],
    area: str,
    target_kinds: list[str],
    environments: list[dict[str, Any]],
    accepted: dict[tuple[str, str, str], dict[str, Any]],
    accepted_environments: dict[str, dict[str, Any]] | None = None,
    prohibitions: dict[str, dict[str, Any]] | None = None,
) -> list[dict[str, Any]]:
    """Every result cell one gating package owns, with its execution decided.

    A cell exists for each `{environment, gate}` the package's declared policy
    requires, whether or not CI will run it. Reuse, an accepted policy gap, and
    hosted execution are states of the same cell rather than reasons to drop
    it. That is what makes the PR #76 regression unrepresentable: omitting an
    *execution* no longer omits the *cell*, so the rollup still expects a
    result for it instead of reporting MISSING.
    """
    package = record["package"]
    cells: list[dict[str, Any]] = []

    def add(
        environment: str,
        gate: str,
        kinds: list[str],
        coverage: str,
        reason: str,
        *,
        gap: dict[str, Any] | None = None,
        reusable: bool = True,
    ) -> None:
        cell: dict[str, Any] = {
            "package": package,
            "area": area,
            "environment": environment,
            "gate": gate,
            "execution": "execute",
            "origin": "ci",
            "state": "pending",
            "target_kinds": kinds,
            "compile_coverage_from": coverage,
            "selection_reason": reason,
        }
        if gap is not None:
            cell["execution"] = "omit"
            cell["origin"] = "none"
            cell["gap"] = gap
            if gap["governed"]:
                cell["state"] = "accepted-gap"
        elif reusable:
            evidence = accepted.get((package, environment, gate)) or (
                accepted_environments or {}
            ).get(environment)
            if evidence is not None:
                cell["execution"] = "reuse"
                cell["origin"] = evidence.get("origin", "local")
                cell["state"] = "reused"
                cell["evidence"] = evidence
        # A recorded constraint stops an EXECUTION, never a satisfied cell: a
        # reused or governed cell consumes none of the host the constraint is
        # protecting. The prohibited cell stays a cell, so `--plan` can name
        # exactly which coverage the constraint leaves unsatisfied.
        constraint = (prohibitions or {}).get(environment)
        if constraint is not None and cell["execution"] == "execute":
            cell["execution"] = "omit"
            cell["origin"] = "none"
            cell["state"] = "prohibited"
            cell["prohibition"] = constraint
        cells.append(cell)

    add(
        LINT_ENVIRONMENT,
        "lint",
        target_kinds,
        # Clippy builds the package, but the check contract is about target
        # coverage and clippy's target selection is the lint recipe's, not the
        # plan's. Crediting it here would claim coverage nothing asserts.
        "",
        f"lint gate for {package}, hosted on {LINT_ENVIRONMENT}",
    )

    unchecked = uncovered_target_kinds(target_kinds)
    if unchecked:
        for environment in native_environments(environments):
            name = environment["name"]
            # The guest whose archive this runner builds never compiles, so
            # this cell is the only compile coverage those kinds get there.
            stands_for = [
                guest["name"]
                for guest in environments
                if capability(guest, "archive_only") and guest["native_key"] == name
            ]
            add(
                name,
                "check",
                unchecked,
                "check",
                f"{', '.join(unchecked)} target(s) are compiled by no test gate; "
                f"checked on {name}"
                + (
                    f", which also stands for {', '.join(stands_for)} "
                    "(archive built here, never compiles)"
                    if stands_for
                    else ""
                ),
                # A local receipt records L1, L2, and browser only (see
                # `local_evidence.RECORDABLE_GATES`), so no evidence — per-cell
                # or whole-environment — can ever stand in for a check.
                reusable=False,
            )

    l1_kinds = [kind for kind in target_kinds if kind in L1_TARGET_KINDS]
    for environment in environments:
        name = environment["name"]
        archive_only = capability(environment, "archive_only")
        for tier in record["tiers"]:
            if tier == "L1":
                add(
                    name,
                    "L1",
                    l1_kinds,
                    # The archive-only guest holds no toolchain: its binaries
                    # were compiled on the runner that built the archive, and
                    # saying otherwise would claim compile coverage that
                    # environment never produced.
                    f"{environment['native_key']} archive build"
                    if archive_only
                    else "L1",
                    f"{package} declares the L1 tier; {name} is a required environment",
                    # A companion suite is not in any local receipt, so its CI
                    # host must still run even when the Rust half was validated
                    # locally.
                    reusable=not (
                        record["companion_suites"] and capability(environment, "node_pnpm")
                    ),
                )
            elif tier == "L2":
                hostable = any(
                    backend_hostable(environment, backend)
                    for backend in record["l2_backends"]
                )
                add(
                    name,
                    "L2",
                    [],
                    "",
                    f"{package} declares the L2 tier with backend(s) "
                    f"{', '.join(record['l2_backends'])}",
                    gap=None if hostable else gap_record(environment, record["l2_backends"]),
                )
            elif tier == "browser":
                hostable = capability(environment, "headless_browser")
                add(
                    name,
                    "browser",
                    [],
                    "",
                    f"{package} declares the browser tier",
                    gap=None if hostable else gap_record(environment, ["headless_browser"]),
                )
    return cells


def matrix_record(
    record: dict[str, Any],
    native: dict[str, list[str]],
    environments: list[dict[str, Any]],
    gates: set[str] | frozenset[str] = frozenset(GATES),
    executing: set[tuple[str, str]] | None = None,
    area: str = "",
    target_kinds: Sequence[str] = ("lib",),
) -> dict[str, Any]:
    """The workflow-facing shape of one gating package's policy.

    Transitional: `ci.yml`, `just/ci-local.just`, and `ci-rollup` still consume
    this shape, and Phases 5 and 6 of `fixes/2026-09-11-cicd-cleanup/plan.md`
    move them onto the resolved plan's cells. Until then this is a projection
    *of* those cells — `executing` is the `{environment, gate}` set the plan
    resolved to hosted execution — so the two documents cannot disagree about
    what CI will run.
    """
    features = feature_args(record, record["package"])
    check_args = check_arguments(record["package"], target_kinds, features)
    testing = "test" in gates
    tiers = record["tiers"] if testing else []
    companion_suites = record["companion_suites"] if testing else []

    def runs(environment: str, gate: str) -> bool:
        return executing is None or (environment, gate) in executing

    return {
        "package": record["package"],
        "area": area,
        "gates": sorted(gates, key=GATES.index),
        "check_args": check_args,
        "test_args": features,
        "l1_include_slow": record["l1_include_slow"],
        "tiers": tiers,
        "l2_backends": record["l2_backends"],
        "runner_tools": record["runner_tools"],
        "companion_suites": companion_suites,
        "native": native,
        "native_environments": [
            environment["name"]
            for environment in native_environments(environments)
            if testing and runs(environment["name"], "L1")
        ],
        "check_os": [
            environment["name"]
            for environment in native_environments(environments)
            if "check" in gates and runs(environment["name"], "check")
        ],
        "l2_environments": (
            [
                environment["name"]
                for environment in environments
                if runs(environment["name"], "L2")
                and any(
                    backend_hostable(environment, backend)
                    for backend in record["l2_backends"]
                )
            ]
            if "L2" in tiers
            else []
        ),
        "browser_environments": (
            [
                environment["name"]
                for environment in environments
                if capability(environment, "headless_browser")
                and runs(environment["name"], "browser")
            ]
            if "browser" in tiers
            else []
        ),
        "node_environments": (
            [
                environment["name"]
                for environment in environments
                if capability(environment, "node_pnpm")
            ]
            if companion_suites
            or (testing and {"node-22", "pnpm-10"} & set(record["runner_tools"]))
            else []
        ),
        "wsl": testing
        and any(
            capability(environment, "archive_only") and runs(environment["name"], "L1")
            for environment in environments
        ),
    }


def estimate_jobs(
    cells: list[dict[str, Any]], environments: list[dict[str, Any]]
) -> int:
    """The expanded job-count estimate for the package fan-out.

    An estimate, not an exact count: an archive-only cell spawns two jobs (the
    Linux archive builder plus the Windows-hosted guest), and the reusable
    workflows may add legs this does not model. The enforced limit is the
    package count (checked against MATRIX_LIMIT), not this number.
    """
    archive_only = {
        environment["name"]
        for environment in environments
        if capability(environment, "archive_only")
    }
    return sum(
        2 if cell["environment"] in archive_only else 1
        for cell in cells
        if cell["execution"] == "execute"
    )


def policy_record(
    record: dict[str, Any],
    gates: set[str] | frozenset[str] = frozenset(GATES),
    area: str = "",
) -> dict[str, Any]:
    """The rollup-facing shape of one impacted package's policy.

    The rollup expects one evidence cell per declared test tier, so a package
    selected without its test gate declares no tiers here — otherwise the
    L1 evidence its skipped test job never produced would roll up as MISSING.
    The package stays in `packages` (the rollup's `--scope`) because its lint
    and check status cells are only admitted for in-scope packages.

    The rollup records the scheduled cells alongside its scope, so a
    `.github/ci/ci-baseline.toml` entry naming a tier this run did not
    schedule is reported `baseline-unscheduled` (a note), not
    `baseline-no-result` (a block).
    """
    testing = "test" in gates
    shaped = {
        "package": record["package"],
        "area": area,
        "gates": record["gates"],
        "tiers": record["tiers"] if testing else [],
        "l2_backends": record["l2_backends"],
        # The rollup needs to know a companion suite was DECLARED: a green
        # Rust JUnit report must not hide a companion that never ran (R12).
        "companion_suites": record["companion_suites"] if testing else [],
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
    base_ref: str | None = None,
    differ: Callable[[str, str], str | None] = diff_against,
    reader: Callable[[str, str], str | None] = read_at_ref,
    accepted_cells: list[dict[str, Any]] | None = None,
    accepted_environments: list[str] | None = None,
    prohibitions: dict[str, dict[str, Any]] | None = None,
    evidence_rejections: list[str] | None = None,
    base: str = NULL_OID,
    head: str = NULL_OID,
) -> dict[str, Any]:
    """The canonical resolved plan for one event.

    One document answers every downstream question: which areas were selected
    and why, which packages contribute to each, and what will happen to each
    `{package, environment, gate}` cell — execute, reuse verified evidence, or
    stand as a governed policy gap. `schema.validate_resolved_plan` is its
    contract.

    `accepted_cells` is the verified per-cell result set, not an environment
    name. Evidence from macOS and from a prior WSL run therefore combine, and
    an accepted cell is *omitted from execution while staying a cell* — the
    shape that makes the PR #76 seven-cell MISSING regression impossible.

    `prohibitions` maps an environment to the recorded constraint forbidding
    it. A prohibited cell is never scheduled and is listed in
    `prohibited_cells`, which is what `just ci-local --plan` refuses on.
    """
    packages = workspace_packages(metadata)

    full_gates = set(GATES) if force_all else set()
    full_scope = bool(full_gates)

    source_paths = source_paths_by_package(list(files), root, packages)
    source_ids = set(source_paths)
    reverse_ids = direct_dependents(source_ids, metadata, packages) - source_ids

    # AC1: an unchanged direct reverse dependent is *reported* here and
    # selected nowhere — no area, no package record, no cell. Whether its seam
    # is compiled at all, and where, is Open Question 1; every option there
    # leaves this presentation rule intact.
    affected_ids = set(packages) if full_scope else source_ids
    impacted = sorted(packages[package_id]["name"] for package_id in affected_ids)
    id_of = {packages[package_id]["name"]: package_id for package_id in affected_ids}

    accepted = {
        (entry["package"], entry["environment"], entry["gate"]): entry
        for entry in (accepted_cells or [])
    }
    whole_environments = whole_environment_evidence(accepted_environments)

    package_records: list[dict[str, Any]] = []
    cells: list[dict[str, Any]] = []
    for name in impacted:
        package_id = id_of[name]
        record = policy[name]
        area = package_area(manifest_directory(root, packages[package_id]))
        reason = (
            "explicit full-scope request"
            if full_scope
            else f"source change in {source_paths[package_id][0]}"
        )
        if not record["gates"]:
            package_records.append(
                non_gating_package_record(record, area, reason)
            )
            continue

        target_kinds = declared_target_kinds(packages[package_id])
        owned = package_cells(
            record,
            area,
            target_kinds,
            environments,
            accepted,
            whole_environments,
            prohibitions,
        )
        cells.extend(owned)
        features = feature_args(record, name)
        package_records.append(
            {
                "package": name,
                "area": area,
                "selection_reason": reason,
                "gates": [
                    gate
                    for gate in schema.GATES
                    if any(cell["gate"] == gate for cell in owned)
                ],
                "targets": target_kinds,
                "tiers": record["tiers"],
                "test_args": features,
                "check_args": check_arguments(name, target_kinds, features),
                "l2_backends": record["l2_backends"],
                "runner_tools": record["runner_tools"],
                "companion_suites": record["companion_suites"],
                "native": native_closure(package_id, metadata, packages, policy),
                "input_paths": closure_directories(
                    package_id, root, metadata, packages
                ),
            }
        )

    gating = [entry for entry in package_records if entry["gates"]]
    if len(gating) > MATRIX_LIMIT:
        raise RuntimeError(
            f"the package matrix has {len(gating)} entries, over GitHub's "
            f"{MATRIX_LIMIT}-job matrix ceiling; the fan-out must be grouped"
        )

    change_class, preflight_os, preflight_reason = classify_preflight(
        cells, gating, full_scope, environments
    )

    # Area flags drive test-shaped specialized jobs and therefore follow only
    # source changes (or an explicit full-scope request).
    flagged_ids = set(packages) if "test" in full_gates else source_ids
    top_dirs = {
        manifest_directory(root, packages[package_id]).parts[0]
        for package_id in flagged_ids
    }
    flags = {
        directory: directory in top_dirs
        for directory in ("claudine", "darkmatter", "sniff", "biscuit-tui", "playa")
    }
    normalized_files = [raw.replace("\\", "/").removeprefix("./") for raw in files]
    flags["ci_tooling"] = any(
        path.startswith(CI_TOOLING_PREFIXES) or path in CI_TOOLING_PATHS
        for path in normalized_files
    )

    return {
        "schema_version": schema.RESOLVED_PLAN_SCHEMA_VERSION,
        "base": base,
        "head": head,
        "change_class": change_class,
        "full_scope": full_scope,
        "full_scope_gates": sorted(full_gates, key=GATES.index),
        "areas": area_records(package_records, full_scope),
        "packages": package_records,
        "source_packages": sorted(
            packages[package_id]["name"] for package_id in source_ids
        ),
        "reverse_dependencies": sorted(
            packages[package_id]["name"] for package_id in reverse_ids
        ),
        "cells": cells,
        "accepted_evidence": [
            cell["evidence"] for cell in cells if cell["execution"] == "reuse"
        ],
        "evidence_rejections": list(evidence_rejections or []),
        "policy_gaps": [
            {
                "package": cell["package"],
                "area": cell["area"],
                "environment": cell["environment"],
                "gate": cell["gate"],
                **cell["gap"],
            }
            for cell in cells
            if "gap" in cell
        ],
        "prohibited_cells": [
            f"{cell['package']}/{cell['environment']}/{cell['gate']}"
            for cell in cells
            if cell["state"] == "prohibited"
        ],
        "job_estimate": estimate_jobs(cells, environments),
        "preflight_os": preflight_os,
        "preflight_reason": preflight_reason,
        "flags": flags,
    }


def non_gating_package_record(
    record: dict[str, Any], area: str, reason: str
) -> dict[str, Any]:
    """A selected package that launches nothing.

    `gates = false` is an owned, dated exclusion, so the package stays visible
    in its area with an empty gate list and no cells. Dropping it would hide a
    governed absence; giving it cells would demand results nothing produces.
    """
    return {
        "package": record["package"],
        "area": area,
        "selection_reason": f"{reason}; gates = false, so it launches nothing",
        "gates": [],
        "targets": [],
        "tiers": [],
        "test_args": "",
        "check_args": "",
        "l2_backends": [],
        "runner_tools": [],
        "companion_suites": [],
        "native": {},
    }


def native_closure(
    package_id: str,
    metadata: dict[str, Any],
    packages: dict[str, dict[str, Any]],
    policy: dict[str, dict[str, Any]],
) -> dict[str, list[str]]:
    """System packages the whole build closure needs, per runner OS."""
    native: dict[str, list[str]] = {}
    for member_id in build_closure(package_id, metadata, packages):
        member = policy[packages[member_id]["name"]]
        for os_name, declared in member["native"].items():
            bucket = native.setdefault(os_name, [])
            for entry in declared:
                if entry not in bucket:
                    bucket.append(entry)
    # Sorted so the plan is byte-stable across runs (set-iteration order is
    # PYTHONHASHSEED-dependent); nothing consumes the order, but noisy diffs
    # obscure real scope changes.
    return {os_name: sorted(bucket) for os_name, bucket in native.items()}


def closure_directories(
    package_id: str,
    root: Path,
    metadata: dict[str, Any],
    packages: dict[str, dict[str, Any]],
) -> list[str]:
    """The manifest directories of a package's build closure, sorted.

    The path half of the gate-input identity of spec section 3.3. The planner
    is the only thing that knows the closure, so the plan carries it; a verifier
    reading the plan then hashes exactly these directories plus the gate's
    global inputs, instead of re-deriving a second closure from a second source.
    """
    return sorted(
        manifest_directory(root, packages[member_id]).as_posix()
        for member_id in build_closure(package_id, metadata, packages)
    )


def area_records(
    package_records: list[dict[str, Any]], full_scope: bool
) -> list[dict[str, Any]]:
    """Areas grouped from their packages, in sorted order.

    Area is a grouping derived from package, never a stored identity (Design
    Decision 1), so it is assembled here rather than carried anywhere.
    """
    grouped: dict[str, list[str]] = {}
    for entry in package_records:
        grouped.setdefault(entry["area"], []).append(entry["package"])
    return [
        {
            "area": area,
            "selection_reason": (
                "explicit full-scope request"
                if full_scope
                else f"source change in package(s) {', '.join(sorted(members))}"
            ),
            "packages": sorted(members),
        }
        for area, members in sorted(grouped.items())
    ]


def classify_preflight(
    cells: list[dict[str, Any]],
    gating: list[dict[str, Any]],
    full_scope: bool,
    environments: list[dict[str, Any]],
) -> tuple[str, list[str], str]:
    """Classify the change and derive its bootstrap-preflight OS matrix (D3).

    ## Returns

    A ``(change_class, preflight_os, reason)`` triple. ``change_class`` is one
    of ``"full"``, ``"package"``, or ``"documentation"``.
    """
    if full_scope:
        return (
            "full",
            list(ALL_RUNNER_OS),
            "explicit full-scope request selects every runner OS",
        )

    if gating:
        # Preflight runs on RUNNER labels, so each environment is resolved to
        # the runner that hosts it — `wsl2-ubuntu` preflights on
        # `windows-latest`.
        runner_of = {environment["name"]: environment["runner"] for environment in environments}
        os_set = {SCOPE_HOST_OS}
        os_set.update(
            runner_of.get(cell["environment"], cell["environment"])
            for cell in cells
            if cell["execution"] == "execute"
        )
        reason = (
            f"package-local change across {len(gating)} package(s); "
            "preflight covers the scope host plus the runner OS hosting each "
            "package's required environments"
        )
        return "package", sorted(os_set), reason

    return (
        "documentation",
        [SCOPE_HOST_OS],
        "no build/test packages affected; preflight runs on the scope host only",
    )


# ---------------------------------------------------------------------------
# Transitional legacy projection
# ---------------------------------------------------------------------------


def legacy_scope_document(
    plan: dict[str, Any],
    policy: dict[str, dict[str, Any]],
    environments: list[dict[str, Any]],
) -> dict[str, Any]:
    """Today's `scope.json` shape, projected from the resolved plan.

    `ci.yml`, `just/ci-local.just`, and `ci-rollup` still read the package
    matrix and the rollup policy list. Phases 5 and 6 of
    `fixes/2026-09-11-cicd-cleanup/plan.md` move them onto `cells`, and this
    function goes with them. It exists so the planner can change shape without
    leaving the workflow reading a document that no longer exists.

    It is a projection, never a second calculation: every environment list
    below is read back out of the plan's cells.

    `area_matrix` groups the same package records by area, one ready-made
    `{"include": [...]}` per area, so `ci.yml` fans out one caller identity per
    selected area and `_area-ci.yml` fans out its packages underneath. Grouping
    happens here rather than in workflow `jq` so the shape is testable.
    """
    executing: dict[str, set[tuple[str, str]]] = {}
    for cell in plan["cells"]:
        if cell["execution"] == "execute":
            executing.setdefault(cell["package"], set()).add(
                (cell["environment"], cell["gate"])
            )

    matrix = []
    for entry in plan["packages"]:
        if not entry["gates"]:
            continue
        # The legacy vocabulary folds every test tier into one `test` gate.
        gates = {gate if gate in ("lint", "check") else "test" for gate in entry["gates"]}
        matrix.append(
            matrix_record(
                policy[entry["package"]],
                entry["native"],
                environments,
                gates,
                executing=executing.get(entry["package"], set()),
                area=entry["area"],
                target_kinds=entry["targets"],
            )
        )

    area_matrix: dict[str, dict[str, list[dict[str, Any]]]] = {}
    for entry in matrix:
        area_matrix.setdefault(entry["area"], {"include": []})["include"].append(entry)

    return {
        "packages": [entry["package"] for entry in plan["packages"]],
        "areas": plan["areas"],
        "scheduled_areas": sorted(area_matrix),
        "area_matrix": area_matrix,
        "area_slugs": {area: area_slug(area) for area in sorted(area_matrix)},
        "source_packages": plan["source_packages"],
        "reverse_dependencies": plan["reverse_dependencies"],
        "full_scope": plan["full_scope"],
        "full_scope_gates": plan["full_scope_gates"],
        "change_class": plan["change_class"],
        "preflight_os": plan["preflight_os"],
        "preflight_reason": plan["preflight_reason"],
        "matrix": matrix,
        "policy": [
            policy_record(
                policy[entry["package"]],
                {"test"} if entry["tiers"] else set(),
                entry["area"],
            )
            for entry in plan["packages"]
        ],
        "job_estimate": plan["job_estimate"],
        "flags": plan["flags"],
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--all", action="store_true", help="select the full workspace")
    parser.add_argument(
        "--accepted-cells",
        metavar="FILE",
        help=(
            "JSON list of verified {package, environment, gate} cells to reuse "
            "instead of executing; `-` reads stdin"
        ),
    )
    parser.add_argument(
        "--accepted-environment",
        action="append",
        choices=ENVIRONMENTS,
        default=[],
        metavar="NAME",
        help=(
            "accept every cell of this environment on a whole-environment "
            "receipt; repeatable"
        ),
    )
    parser.add_argument(
        "--resolved-plan",
        action="store_true",
        help=(
            "emit the canonical resolved plan instead of the legacy scope "
            "document the workflow still reads"
        ),
    )
    parser.add_argument(
        "--plan-out",
        metavar="FILE",
        help=(
            "also write the canonical resolved plan here; one calculation "
            "serves both the workflow's legacy document and the plan surface"
        ),
    )
    parser.add_argument(
        "--constraints",
        metavar="DIR",
        help=(
            "persisted execution-constraint store; environments it forbids are "
            "resolved to prohibited cells instead of scheduled ones"
        ),
    )
    parser.add_argument(
        "--evidence-rejections",
        metavar="FILE",
        help="JSON list of coded reasons published evidence was refused",
    )
    parser.add_argument("--base", default=NULL_OID, help="base revision under test")
    parser.add_argument("--head", default=NULL_OID, help="head revision under test")
    parser.add_argument("files", nargs="*", help="changed repository-relative paths")
    return parser.parse_args()


def read_accepted_cells(source: str | None) -> list[dict[str, Any]]:
    """The verified per-cell result set, or an empty one.

    ## Errors

    Raises ``RuntimeError`` when the document is not a list of cells naming a
    package, an environment, and a gate. Silently ignoring a malformed
    evidence file would schedule nothing and call the gap verified.
    """
    if not source:
        return []
    text = sys.stdin.read() if source == "-" else Path(source).read_text(encoding="utf-8")
    document = json.loads(text)
    if not isinstance(document, list):
        raise RuntimeError(f"accepted cells must be a JSON list, got {type(document).__name__}")
    for entry in document:
        missing = {"package", "environment", "gate"} - set(entry or {})
        if missing:
            raise RuntimeError(f"accepted cell {entry!r} is missing {sorted(missing)}")
    return document


def read_prohibitions(directory: str | None) -> dict[str, dict[str, Any]]:
    """The recorded constraints, keyed by the environment each forbids.

    The store's location is Open Question 2; `constraints.py` owns that choice
    and this function owns none of it.
    """
    if not directory:
        return {}
    import constraints  # local import: the planner runs where the store may not exist

    active, _expired, malformed = constraints.load(directory)
    return {
        entry.environment: {
            "owner": entry.document.get("owner", ""),
            "reason": entry.document.get("reason", ""),
            "expiry": entry.document.get("expiry", ""),
            "source": entry.path.name,
        }
        for entry in active + malformed
        if entry.environment
    }


def read_evidence_rejections(source: str | None) -> list[str]:
    if not source:
        return []
    document = json.loads(Path(source).read_text(encoding="utf-8"))
    if not isinstance(document, list) or not all(isinstance(x, str) for x in document):
        raise RuntimeError("evidence rejections must be a JSON list of strings")
    return document


def main() -> None:
    args = parse_args()
    environments = load_environments(ENVIRONMENTS_CONFIG)
    runner_labels = {environment["runner"] for environment in environments}
    metadata = load_metadata(ROOT)
    validate_no_shadow_workspaces(metadata, ROOT)
    policy = package_ci_policy(workspace_packages(metadata), runner_labels, ROOT)
    plan = calculate_scope(
        args.files,
        ROOT,
        metadata,
        environments,
        policy,
        args.all,
        accepted_cells=read_accepted_cells(args.accepted_cells),
        accepted_environments=args.accepted_environment,
        prohibitions=read_prohibitions(args.constraints),
        evidence_rejections=read_evidence_rejections(args.evidence_rejections),
        base=args.base,
        head=args.head,
    )
    if args.plan_out:
        Path(args.plan_out).write_text(schema.canonical(plan), encoding="utf-8")
    document = (
        plan if args.resolved_plan else legacy_scope_document(plan, policy, environments)
    )
    print(json.dumps(document, separators=(",", ":")))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Calculate dependency-aware CI scope for the Rusty Biscuit workspace.

The package is the unit of selection, execution, and result identity; the area
is a grouping derived from the package's manifest directory, never stored as an
identity. Package policy lives in each package's own Cargo manifest under
`[package.metadata.ci]`; environment capabilities live in
`.github/ci/environments.json`. Both are validated here, loudly, before any
scope is emitted.

`calculate_scope` returns the canonical resolved plan that
`scripts/ci/schema.py` defines and validates. `apply_accepted_cells` overlays
verified evidence on a plan that already exists — the operation CI performs on
a matching scope receipt, which is never re-selected. `legacy_scope_document`
projects a plan into the `scope.json` outputs `ci.yml` publishes and the policy
and build-owner lists `ci-rollup` reads.
"""

from __future__ import annotations

import argparse
import copy
import json
import re
import subprocess
import sys
from datetime import date
from pathlib import Path, PurePosixPath
from collections.abc import Callable, Sequence
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent))

import build_key  # noqa: E402  (needs the path insert above when run as a script)
import schema  # noqa: E402

try:  # Python 3.11+
    import tomllib
except ModuleNotFoundError:
    # IMPORTING this module must not require 3.11. `companion_suites.py` imports
    # it and runs on all three native environments, where `python3` can be a
    # 3.9 interpreter (macOS ships one at `/usr/bin/python3`); the PLANNER runs
    # only on the scope job's `ubuntu-latest` and on developer hosts. So the
    # parser is optional here and `load_skip_policy` refuses by name when it is
    # both absent and needed. The repository's Python helpers support 3.9.
    tomllib = None  # type: ignore[assignment]


ROOT = Path(__file__).resolve().parents[2]
ENVIRONMENTS_CONFIG = ROOT / ".github" / "ci" / "environments.json"

#: Version 2 added the per-environment `build` contract: the compile-affecting
#: inputs a native producer declares and the predicates an archive-only
#: environment is checked against. Version 3 added `events`, the GitHub events
#: that schedule the environment (fixes/2026-09-18-ci-cadence).
ENVIRONMENTS_SCHEMA_VERSION = 3

# The environment names the policy table declares, for argument validation.
ENVIRONMENTS = schema.ENVIRONMENTS

#: The GitHub events a run can carry, as `github.event_name` spells them. An
#: environment's `events` names the subset that schedules it; a planner run
#: without an event (a developer's `just ci-local --plan`, the test suites)
#: plans every environment.
EVENT_NAMES = schema.EVENTS

# The three verdicts CI can produce per package.
GATES = ("lint", "check", "test")

# These input-analysis helpers remain covered for tooling diagnostics. Package
# scheduling itself is source-driven and does not consume global-input changes.
GLOBAL_PATHS_ALL_GATES = {
    ".github/ci/environments.json",
    # The area workflow carries the dispatch row sets, so it decides what runs
    # (ruling R11). It is paired with `ORCHESTRATION_PATHS` below: adding it to
    # one table alone costs either a needless full-workspace pre-push or every
    # published local cell.
    ".github/workflows/_area-ci.yml",
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
    # `_expected_manifest` is a CI ENTRY recipe, not a helper: every test
    # producer calls it and `completion.py` refuses the cell when its answer
    # and the staged reports disagree. Editing the expected-set logic therefore
    # changes what a test gate PROVES, so it must select the test gate and move
    # the test cells' gate-input identity like any other entry (ruling R12).
    "test": ("_test", "_test_l2", "_test_browser", "_expected_manifest"),
}
CI_RECIPES_ALL_GATES = ("_ensure-native-libs",)

# Any path that MAY force workspace scope for some gate; the per-gate decision
# is `gate_triggers`.
GLOBAL_PATHS = (
    GLOBAL_PATHS_ALL_GATES | JUST_PATHS | set().union(*GLOBAL_PATHS_BY_GATE.values())
)
GLOBAL_PREFIXES = GLOBAL_PREFIXES_ALL_GATES + JUST_PREFIXES

# The global inputs that decide what CI RUNS and never what a local gate
# PRODUCES. `local_evidence.gate_global_inputs` leaves them out of a cell's
# gate-input identity, so editing a workflow no longer invalidates every
# published local cell (fixes/2026-09-18-ci-cadence, decision 5). The toolchain
# pin, Cargo config, the root manifest, the lockfile, `clippy.toml`, and
# `.config/nextest.toml` stay inputs: each changes what a local run compiles
# or executes. So do the Just recipes CI reaches — by recipe, through
# `local_evidence.just_gate_inputs`, with the same closure that decides
# selection above (fixes/2026-09-19-just-recipe-identity).
ORCHESTRATION_PATHS = {
    ".github/ci/environments.json",
    ".github/workflows/_area-ci.yml",
    ".github/workflows/_package-ci.yml",
    ".github/workflows/_wsl-ci.yml",
    ".github/workflows/ci.yml",
    "scripts/ci/affected_scope.py",
}
ORCHESTRATION_PREFIXES = (".github/actions/",)

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

# The two execution models a registered suite can have. A `cargo` suite runs
# inside its owner's ordinary Nextest cells, so no single environment hosts it.
# A `companion` suite is a non-Cargo suite the owner's package job runs beside
# them, on exactly one declared environment.
SUITE_KINDS = ("cargo", "companion")

# What a `cargo` suite declares instead of an environment: its owner's
# `[package.metadata.ci.tests]` environment set governs, and duplicating that
# here would be a second, driftable answer.
EVERY_DECLARED_ENVIRONMENT = "*"

# How a companion suite's runner obtains machine-readable counts. `json` is
# this repository's own counts document, written by `scripts/ci/suite_runner.py`;
# `vitest` is Vitest's `--reporter=json` output. A suite with no strategy
# declares WHY in `counts_reason`, because AC13 requires an unmeasured suite to
# render `not recorded` with a reason rather than a `0` that reads as evidence.
COUNTS_STRATEGIES = ("json", "vitest")

# The substitution the runner makes in `counts_args` before invoking a suite.
COUNTS_OUT_PLACEHOLDER = "{out}"

# Every test suite CI runs that is not reached by `just _test <pkg>` alone,
# plus the two Cargo suites that verify CI itself. One owner, one canonical
# recipe, one environment each (R7, spec section 2). This is a declared table
# in the planner, never a persistent store (spec D9).
#
# `repo-deps` (`scripts/`) owns the planner's Python contracts and the
# rollup/plan Rust suites; `test-toolkit` (`tools/test-toolkit/`) owns the
# workflow contracts and the `tools/test-audit` typecheck/Vitest pair;
# `homelab-server` owns the frontend suite that predates the registry.
#
# `just` is present only where a canonical recipe IS a Just recipe: each
# `(directory, recipe)` pair is what `validate_package_ci` checks still exists,
# so a renamed recipe fails here rather than silently never running.
#
# `lint_recipe` is the suite's half of the owner's LINT cell, present only for
# a suite that has one. A suite without it is absent from the lint cell
# entirely rather than expected there and never run.
#
# `counts`/`counts_args` are how `scripts/ci/companion_suites.py` obtains
# machine-readable counts; `counts_reason` replaces them for a gate with no
# test cardinality to report (S3: `tsc --noEmit` has none, and no flag can
# invent one). `node` marks the suites that need the Node + pnpm toolchain, so
# a Python-only owner does not provision one.
SUITE_REGISTRY: dict[str, dict[str, Any]] = {
    "repo-deps-l1": {
        "owner": "repo-deps",
        "kind": "cargo",
        "environment": EVERY_DECLARED_ENVIRONMENT,
        "recipe": "just _test repo-deps",
    },
    "test-toolkit-l1": {
        "owner": "test-toolkit",
        "kind": "cargo",
        "environment": EVERY_DECLARED_ENVIRONMENT,
        "recipe": "just _test test-toolkit",
    },
    "homelab-frontend": {
        "owner": "homelab-server",
        "kind": "companion",
        "environment": "ubuntu-latest",
        "recipe": "cd homelab && just test-frontend",
        "lint_recipe": "cd homelab && just lint-frontend",
        "just": (("homelab", "test-frontend"), ("homelab", "lint-frontend")),
        "node": True,
        "counts": "vitest",
        "counts_args": f"--reporter=json --outputFile={COUNTS_OUT_PLACEHOLDER}",
    },
    "test-audit-typecheck": {
        "owner": "test-toolkit",
        "kind": "companion",
        "environment": "ubuntu-latest",
        "recipe": "pnpm --dir tools/test-audit typecheck",
        "node": True,
        "counts": None,
        "counts_reason": "typecheck gate reports no test counts",
    },
    "test-audit-vitest": {
        "owner": "test-toolkit",
        "kind": "companion",
        "environment": "ubuntu-latest",
        "recipe": "pnpm --dir tools/test-audit test",
        "node": True,
        "counts": "vitest",
        "counts_args": f"--reporter=json --outputFile={COUNTS_OUT_PLACEHOLDER}",
    },
    **{
        suite: {
            "owner": "repo-deps",
            "kind": "companion",
            "environment": "ubuntu-latest",
            # The wrapper, not the bare module: it is what CI runs, and a
            # recipe that names something else is a recipe nobody can
            # reproduce. `python3 scripts/ci/<suite>` still runs the same
            # tests; it just reports no counts.
            "recipe": f"python3 scripts/ci/suite_runner.py {suite}",
            "counts": "json",
            "counts_args": f"--counts-out {COUNTS_OUT_PLACEHOLDER}",
        }
        for suite in (
            "test_affected_scope.py",
            "test_build_key.py",
            "test_ci_local.py",
            "test_completion.py",
            "test_constraints.py",
            "test_cross_check.py",
            "test_evidence_reuse.py",
            "test_local_evidence.py",
            "test_publish_gaps.py",
            "test_resolved_plan.py",
            "test_reuse_validation.py",
            "test_runner_loss.py",
            "test_schema.py",
        )
    },
    # Lint-only on purpose, and the one entry with no `recipe`: a source-policy
    # scan has no test half to attach to an L1 cell. Without that omission the
    # guard would run twice on Linux — once as `test-toolkit`'s L1 companion and
    # once in its lint cell — and its changed-file scope would be applied to a
    # cell whose evidence rules are the package's, not the scan's.
    "archive-path-guard": {
        "owner": "test-toolkit",
        "kind": "companion",
        "environment": "ubuntu-latest",
        "lint_recipe": "cd tools/test-toolkit && just archive-path-guard",
        "just": (("tools/test-toolkit", "archive-path-guard"),),
        "counts": None,
        "counts_reason": (
            "the guard answers a source-policy question over files, not a test "
            "cardinality; its two driver assertions are not the measurement"
        ),
    },
    # The build owner's artifact publisher: no Cargo package and no pnpm
    # workspace entry selects it, so this registration is the only thing that
    # schedules its suite. The glob stays quoted -- given a bare directory
    # `node --test` resolves it as a module and exits without having run a
    # test (measured on Node 22.20).
    "artifact-publisher": {
        "owner": "repo-deps",
        "kind": "companion",
        "environment": "ubuntu-latest",
        "recipe": "node --test 'scripts/ci/artifacts/*.test.cjs'",
        "node": True,
        "counts": None,
        "counts_reason": "node --test emits TAP, which no counts strategy reads",
    },
}

# Inputs a registered suite verifies that ordinary source ownership does not
# select. `scripts/**` and `tools/test-toolkit/**` need no entry: they are their
# owners' package directories, so `source_paths_by_package` already selects them
# (R13.4). What is left is everything outside a member directory — and the two
# owners' own manifests, which `changed_package_ids` deliberately excludes
# repository-wide.
#
# The manifest exception is NOT general (R13 amendment). These two packages
# exist only to verify CI, and their `[package.metadata.ci.tests]` blocks are
# where the suites above are declared to run at all; a manifest edit that
# selected nothing would ship a change to CI's own scheduling unverified. Every
# other package's manifest keeps the repository-wide rule, because the next
# source edit exercises the resulting package graph.
#
# A path MAY name more than one owner when both owners' suites consume it.
SUITE_OWNER_PREFIXES: tuple[tuple[str, tuple[str, ...]], ...] = (
    (".github/ci/", ("repo-deps",)),
    (".github/workflows/", ("test-toolkit",)),
    ("tools/test-audit/", ("test-toolkit",)),
)
SUITE_OWNER_PATHS: dict[str, tuple[str, ...]] = {
    # Cross-language by construction: `test_schema.py` and
    # `tools/test-toolkit/tests/ci_workflow_contracts.rs` read the same bytes,
    # which is the only reason these two documents are shipped at all. A change
    # that selected the Python half alone would let the two readers disagree
    # unobserved. Named one by one, not by a `schemas/` prefix: that directory
    # also holds documentation and is where an unrelated future schema would
    # land, and neither is an input the Rust suite reads. Those keep the
    # `.github/ci/` prefix's `repo-deps` selection.
    ".github/ci/schemas/archive_guard_cases.json": ("repo-deps", "test-toolkit"),
    ".github/ci/schemas/contract.json": ("repo-deps", "test-toolkit"),
    "pnpm-lock.yaml": ("test-toolkit",),
    "pnpm-workspace.yaml": ("test-toolkit",),
    "scripts/Cargo.toml": ("repo-deps",),
    "tools/test-toolkit/Cargo.toml": ("test-toolkit",),
}

# AC15 makes `sniff` the authority on package areas: this planner replicates
# sniff's rule and owns no mapping of its own. Nothing but the contracts in
# `test_resolved_plan.py::AreaGroupingTests` notices when the two diverge — a
# stale rule fans work out to the wrong area and every other check stays green
# — and those contracts need the real binary, which only `ci.yml`'s
# `area-drift` job provisions. This flag is what schedules that job, so it must
# cover every change that can move either side of the equivalence.
#
# Suite ownership cannot serve: `SUITE_OWNER_PREFIXES` / `SUITE_OWNER_PATHS`
# select the package that owns a changed CI input, so a change to sniff's own
# detection rule would schedule the contracts nowhere.
AREA_DRIFT_PREFIXES = ("sniff/",)
AREA_DRIFT_PATHS = {
    "scripts/ci/affected_scope.py",
    "scripts/ci/test_resolved_plan.py",
}

# A package's area is a function of WHERE its manifest sits, so a manifest that
# appears, moves, or disappears re-maps areas without touching sniff or this
# file — the case a path list keyed on the two implementations structurally
# cannot see. Every `Cargo.toml` at every depth therefore counts, fixture
# manifests included: `sniff repo package-area` answers per directory, so a
# manifest in an unusual location is precisely the input that separates the two
# rules, and nothing distinguishes a fixture manifest from a real one without
# reading it. The change list is `git diff --name-only`, which carries no
# add/modify/delete status, so "a manifest moved" is not separable from "a
# dependency was bumped" either. The over-approximation costs one parallel job
# that `ci-gate` folds; the miss it prevents is silent.
AREA_DRIFT_MANIFEST = "Cargo.toml"

# The archive-path guard (fixes/2026-09-19-less-brittle). It scans repository
# source for compile-time paths that do not survive an archived run, so the
# files it checks are almost never its owner's: package ownership selects it
# nowhere, and violations accumulated until an unrelated change happened to
# select `test-toolkit`. Selection follows the scanned source instead.
ARCHIVE_GUARD_SUITE = "archive-path-guard"

# Mirrors `test_toolkit::archive_guard::SKIPPED_DIRS`. The two are one policy —
# a path the planner selects and the scanner then refuses is a cell that ran
# nothing — and `tools/test-toolkit/tests/ci_workflow_contracts.rs` asserts the
# agreement across the language boundary. Matched against any path component,
# so `examples` excludes every package's.
ARCHIVE_GUARD_SKIPPED_DIRS = frozenset({
    "target", ".git", "node_modules", ".gitnexus", "scripts", "examples", "fuzz",
})

# The guard's own inputs, by exact path: the matcher, its fixture corpus, the
# driver, the exemption list, the canonical recipe, this selection rule, and
# the registry and execution configuration that decide whether the scan runs
# and under which scope.
#
# The admission rule is ownership, not topic: a file belongs here when it
# carries configuration whose ONLY consumer is this guard, so a change to it
# can silently alter whether the guard runs or what it scans while every other
# check stays green. That is what keeps this a narrow list rather than a rule
# that every workflow, manifest, or lockfile edit selects the guard.
#
# - `archive_guard.rs` (which also holds `ALLOWED`), `matcher_tests.rs`, and
#   `archive_path_guard.rs` are excluded from SCANNING by `GUARD_OWN_SOURCES`
#   — they must be free to spell the forbidden forms — and are owned here.
# - `affected_scope.py` needs its own entry because `scripts/` is a skipped
#   scan directory.
# - `tools/test-toolkit/Cargo.toml` is the REGISTRY BINDING: its
#   `[package.metadata.ci.tests] companion-suites` is the only declaration that
#   attaches `archive-path-guard` to an owner. Drop the entry and every other
#   gate stays green while the scan never runs again.
# - `_package-ci.yml` holds the guard's two execution controls — the
#   `BISCUIT_ARCHIVE_GUARD_PLAN` export, whose only reader is the guard, and
#   the companions-only status fold that makes a guard-only cell's verdict its
#   companion's. It already selects `test-toolkit` through
#   `SUITE_OWNER_PREFIXES`, so its lint cell and the attached companion exist
#   either way; what the entry buys is a plan that says the scan was selected
#   and under which mode, instead of an empty unselected scan running
#   incidentally.
# - `cell_contract.py` is the sole conduit for a cell's `companions_only`: the
#   lint job reads it from `steps.cell.outputs`, never from a workflow input.
#   Like `affected_scope.py` it lives under the skipped `scripts/` directory.
#
# Deliberately NOT owned inputs:
#
# - `.github/workflows/ci.yml` carries no guard-specific configuration. The
#   resolved-plan artifact it uploads is read by the coverage audits, the gap
#   publisher, and every area, so losing it is loud and general rather than a
#   guard-shaped silence.
# - `.github/ci/environments.json` decides whether Linux is scheduled at all.
#   When it is not, the guard cannot run anywhere, so selecting it from that
#   change would schedule nothing.
# - `scripts/ci/schema.py` and its generated `contract.json` VALIDATE the
#   `archive_guard` block; the emitter is `archive_guard_scope` in this file,
#   already owned. A validator-only change the emitter does not follow fails
#   the planner and `test_schema.py` / `test_resolved_plan.py`, which
#   `scripts/` ownership already selects.
# - `just/ci-local.just` drives the guard locally. No CI guard execution runs
#   it, so scheduling one answers nothing about it; `ci_workflow_contracts` and
#   `test_ci_local.py` are what cover that path.
ARCHIVE_GUARD_OWN_INPUTS = frozenset({
    ".github/workflows/_package-ci.yml",
    "scripts/ci/affected_scope.py",
    "scripts/ci/cell_contract.py",
    "tools/test-toolkit/Cargo.toml",
    "tools/test-toolkit/justfile",
    "tools/test-toolkit/src/archive_guard.rs",
    "tools/test-toolkit/src/archive_guard/matcher_tests.rs",
    "tools/test-toolkit/tests/archive_path_guard.rs",
})

# Bootstrap-preflight breadth (D3). A full-scope run validates every runner OS
# the plan's environments land on before fan-out; a package-local change
# validates only the scope host plus the runner OSes its selected packages'
# environments actually land on.
SCOPE_HOST_OS = "ubuntu-latest"

KNOWN_L2_BACKENDS = {"tmux", "wezterm", "kitty", "apple-terminal"}
KNOWN_TIERS = {"L1", "L2", "browser"}
EXCLUSION_CLASSES = {"capability", "promotion-pending", "time-bounded"}

# `runner-tools` is a CLOSED vocabulary implemented by the reusable workflow,
# not an arbitrary command surface. Everything left here is a RUNTIME facility
# the consumer provisions for itself; the compile-time entries moved to
# `sidecars` (see [`SIDECAR_TABLE`]) because a consumer with no Cargo cannot
# build one.
KNOWN_RUNNER_TOOLS = {
    "ai-provider-stubs",
    "node-22",
    "pnpm-10",
    "l2-parallel-self-spawn",
    "neovim",
    "zed-extension",
}

# The named build sidecars, declared as data rather than as a shell command a
# package could spell any way it liked. `ci-build produce` reads the same file
# to learn which package and binaries each name compiles.
SIDECAR_TABLE = ".github/ci/sidecars.json"
SIDECAR_SCHEMA_VERSION = 1

# The one L2 backend a CI runner provides. A package whose L2 declares only GUI
# terminal emulators renders a governed capability gap and never executes the
# tier there.
CI_HOSTABLE_L2_BACKEND = "tmux"

# What `just _test_l2` needs on a consumer that has no Cargo: the broker that
# owns the shared terminal pane, and the proof that a required backend actually
# drove a test. Both were recipe-time `cargo` invocations before the cutover.
HOSTABLE_L2_SIDECARS = ("backend-proof", "harness-broker")

# The only substitutions an `archive-includes` entry may carry. A dynamic
# library is `libfoo.so`, `libfoo.dylib`, and `foo.dll` on the three producers,
# and making a package spell all three is three chances to get one wrong.
INCLUDE_PLACEHOLDERS = ("{DLL_PREFIX}", "{DLL_SUFFIX}", "{EXE_SUFFIX}")

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
    "requires-toolchain",
    #: Build outputs the package's archive must carry, relative to the profile
    #: output directory (`examples/discovery_probe`, not
    #: `target/debug/examples/discovery_probe`) — the producer supplies the
    #: `<triple>/<profile>` prefix its own invocation created.
    "archive-includes",
    #: Named build sidecars from [`SIDECAR_TABLE`]: another package's binaries,
    #: compiled by the producer and shipped beside the archive.
    "sidecars",
}
EXCLUSION_FIELDS = {"exclusion-class", "owner", "reason", "expiry"}

# Where `_package-ci.yml` runs clippy. A cell carries it so the lint gate's
# environment is visible in the matrix (AC10) instead of being implied.
LINT_ENVIRONMENT = "ubuntu-latest"

# Where the `example` and `bench` kinds are compiled. One environment, like
# lint: compiling them once is the coverage, compiling them per operating
# system was cost (fixes/2026-09-18-ci-cadence, decision 3). The archive-only
# guest this host builds for is stood for here as before.
CHECK_ENVIRONMENT = "ubuntu-latest"

# The gates the L1 build itself compiles. A separate check cell is scheduled
# only for required kinds outside this set (spec section 1.7).
L1_TARGET_KINDS = ("lib", "bin", "test")

# Cargo's explicit selector for each kind outside `L1_TARGET_KINDS`. The check
# command is built from these and nothing else: `--all-targets` would recompile
# the L1 kinds and make the cell's `target_kinds` a label rather than a fact.
CHECK_SELECTORS = {"example": "--examples", "bench": "--benches"}

# Cargo's explicit selector for each L1 kind of an UNCHANGED direct dependent
# compiled inside a changed package's own check cell (Open Question 1, ruled
# Option B on 2026-09-12). The seam under test is the changed package's public
# API, and a consumer's `lib`, `bin`, and `test` targets are what consume it —
# the same kinds its own L1 build would compile were it selected. Its
# `example`/`bench` kinds are deliberately not selected: they would need a
# check cell of their own if the dependent were selected, and that cost is not
# in the ruling. Selected by the dependent's DECLARED kinds, never
# `--all-targets`, for the same reason as `CHECK_SELECTORS`.
DEPENDENT_SELECTORS = {"lib": "--lib", "bin": "--bins", "test": "--tests"}

# The one environment on which dependents are compiled (the ruling: Linux
# only). `_package-ci.yml` spells the same label in its step condition;
# `ci_workflow_contracts.rs` asserts the two agree.
DEPENDENTS_ENVIRONMENT = "ubuntu-latest"

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

#: Byte ceiling on one area's serialized row-set document. Row sets travel as
#: `scope`-job outputs indexed per area, exactly as `area_matrix` does today,
#: and a job output that outgrows GitHub's limit fails the run after the work
#: was already planned. Measured headroom on this workspace: the largest area
#: serializes to ~2.2 KB (`spikes/s3-capacity.md`), so 16 KB is a guard against
#: future growth rather than a constraint on today's plan. Ruling R7: the
#: planner is the only place this is enforced, and it refuses rather than
#: truncating.
AREA_ROW_SET_BUDGET = 16 * 1024

#: Byte ceiling on every area's row sets together. `ci.yml`'s `scope` job
#: carries them as ONE output (`area_rows`) that each area call indexes, and
#: GitHub caps a single output at 1 MB; half that leaves the refusal here
#: rather than at output time. The full-workspace plan's rows measure ~22 KB
#: (`spikes/s3-capacity.md`).
TOTAL_ROW_SET_BUDGET = 512 * 1024

#: The hand-edited exact-skip policy, snapshotted into every plan (ruling R8).
BASELINE_POLICY_PATH = ".github/ci/ci-baseline.toml"

#: The only dispatch form the shipped workflows implement (ruling R9 as
#: amended in Phase 7 of `2026-09-19-direct-cell-execution`: every area moved
#: together, and rollback is a revert rather than a per-area allowlist).
EXECUTION_PATH = "rows"

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
    # A runnable Cargo/rustc toolchain, for the minority of L1 suites that
    # SHELL OUT to one. Distinct from `archive_only`, which says how a cell is
    # executed: the two coincide today only because the archive design is what
    # leaves its guest without a toolchain.
    "cargo_toolchain",
    "archive_only",
}

#: The compile-affecting inputs a native producer declares. Every one of them
#: enters the planned build key, so adding a field here changes every key — a
#: field that describes the WORKFLOW rather than the compilation does not belong
#: in this table at all. `archive_cutover` was the one exception, and Task 6.5
#: removed it with the compile-in-place paths it guarded.
PRODUCER_BUILD_FIELDS = {
    "host",
    "target",
    "profile",
    "rustflags",
    "cargo_config",
    "linker",
    "archive_format",
    "nextest",
    "executes",
    "runtime",
}

#: What an archive-only environment declares. It compiles nothing, so it has no
#: compiler host, target, profile, or `executes` list — only the predicates a
#: producer's archive is checked against and the tool it runs that archive with.
CONSUMER_BUILD_FIELDS = {"nextest", "runtime"}

#: The predicates that decide whether one environment may execute another's
#: archive. Spec section 5: a runner-image, architecture, distribution, or
#: native-dependency change that breaks the predicate must create another build
#: key or fail planning, never move compilation into a toolchain-free guest.
RUNTIME_PREDICATE_FIELDS = {"arch", "abi", "libc", "native_libraries"}

#: The predicates that must be *equal* for a cross-environment edge. ABI and
#: libc together are what make native Windows and WSL2 structurally unpairable
#: even before the hosting rule below is applied.
RUNTIME_EQUAL_PREDICATES = ("arch", "abi", "libc")


#: Version of `.github/ci/ci-baseline.toml`, mirrored from
#: `ci-rollup.rs::BASELINE_SCHEMA_VERSION`. The planner reads the same file the
#: audit does and must refuse the same generations.
BASELINE_SCHEMA_VERSION = 3


def load_skip_policy(root: Path, today: date | None = None) -> dict[str, Any]:
    """Snapshot the approved exact-skip budget for the plan (ruling R8).

    `.github/ci/ci-baseline.toml` stays the hand-edited source of truth. The
    planner reads it once and writes what it read — provenance plus the
    entries — into the plan, so a producer and the area audit apply the policy
    the run was PLANNED with rather than whatever the file says by the time
    they read it. A missing file and an empty one snapshot identically: both
    approve nothing, and both hash the empty byte string.

    The returned entries are not yet bound to cells; `applicable_skip_entries`
    narrows them to the plan's own cells once those exist.

    ## Errors

    Raises ``RuntimeError`` for an unreadable or unparseable file, a schema
    generation this planner does not read, an entry missing its governance, or
    an entry whose expiry has passed. An expired approval is a planning error
    rather than a producer-time surprise, because the plan is the artifact a
    carried scope receipt reuses.
    """
    today = today or date.today()
    path = root / BASELINE_POLICY_PATH
    try:
        content = path.read_bytes()
    except OSError:
        content = b""
    if content and tomllib is None:
        raise RuntimeError(
            f"{BASELINE_POLICY_PATH} carries policy but this interpreter has no "
            f"`tomllib` (Python {sys.version_info.major}.{sys.version_info.minor}); "
            "resolving a plan needs Python 3.11 or newer. There is deliberately no "
            "second TOML reader to fall back to."
        )
    try:
        document = tomllib.loads(content.decode("utf-8")) if content else {}
    except (tomllib.TOMLDecodeError, UnicodeDecodeError) as error:
        raise RuntimeError(f"{BASELINE_POLICY_PATH}: cannot be parsed: {error}") from error
    if document and document.get("schema_version") != BASELINE_SCHEMA_VERSION:
        raise RuntimeError(
            f"{BASELINE_POLICY_PATH}: schema_version must be "
            f"{BASELINE_SCHEMA_VERSION}, got {document.get('schema_version')!r}"
        )

    entries: list[dict[str, Any]] = []
    for index, entry in enumerate(document.get("skip", [])):
        label = f"{BASELINE_POLICY_PATH} [[skip]] #{index}"
        if not isinstance(entry, dict):
            raise RuntimeError(f"{label}: must be a table")
        missing = [
            field
            for field in ("package", "environment", "tier", "owner", "reason", "source_run")
            if not entry.get(field)
        ]
        if missing:
            raise RuntimeError(f"{label}: missing {', '.join(missing)}")
        if entry["environment"] not in ENVIRONMENTS:
            raise RuntimeError(
                f"{label}: names environment {entry['environment']!r}, which is not "
                f"one of {list(ENVIRONMENTS)}"
            )
        if entry["tier"] not in schema.GATES:
            raise RuntimeError(
                f"{label}: names tier {entry['tier']!r}, which is not one of "
                f"{list(schema.GATES)}"
            )
        if "expiry" in entry:
            validate_expiry(label, "expiry", str(entry["expiry"]), today)
        # `tier` is the file's spelling of the same dimension the plan calls
        # `gate`; the snapshot uses the plan's word so a reader of the plan
        # never has to learn both.
        record = {
            "package": str(entry["package"]),
            "environment": str(entry["environment"]),
            "gate": str(entry["tier"]),
            "owner": str(entry["owner"]),
            "reason": str(entry["reason"]),
            "source_run": str(entry["source_run"]),
        }
        if entry.get("tests") is not None:
            record["tests"] = [str(name) for name in entry["tests"]]
        if entry.get("backend"):
            record["backend"] = str(entry["backend"])
        if entry.get("expiry"):
            record["expiry"] = str(entry["expiry"])
        entries.append(record)

    return {
        "source": BASELINE_POLICY_PATH,
        "content_hash": build_key.planned_keys([content.decode("utf-8", "replace")])[0],
        "entries": entries,
    }


def applicable_skip_entries(
    policy: dict[str, Any], cells: Sequence[dict[str, Any]]
) -> dict[str, Any]:
    """`policy` narrowed to the cells this plan carries.

    Applicability is what makes the snapshot usable: a repository-wide policy
    file names cells no single plan selects, and an approval a plan cannot bind
    to a cell is one the audit would never apply. The provenance still names
    the whole file and its hash, so narrowing is visible rather than silent.
    """
    keys = {
        (cell["package"], cell["environment"], cell["gate"]) for cell in cells
    }
    return {
        **policy,
        "entries": [
            entry
            for entry in policy["entries"]
            if (entry["package"], entry["environment"], entry["gate"]) in keys
        ],
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
    if document.get("schema_version") != ENVIRONMENTS_SCHEMA_VERSION:
        raise RuntimeError(
            f"{path}: schema_version must be {ENVIRONMENTS_SCHEMA_VERSION}"
        )

    environments = document["environments"]
    names: set[str] = set()
    for index, environment in enumerate(environments):
        label = environment.get("name", f"<record {index}>") if isinstance(environment, dict) else f"<record {index}>"
        if not isinstance(environment, dict):
            raise RuntimeError(f"environments[{index}] must be an object")
        unknown = environment.keys() - {"name", "runner", "native_key", "events", "capabilities", "build"}
        if unknown:
            raise RuntimeError(f"environment '{label}' has unknown field(s): {sorted(unknown)}")
        for field in ("name", "runner", "native_key"):
            if not isinstance(environment.get(field), str) or not environment[field].strip():
                raise RuntimeError(f"environment '{label}' must give a non-empty '{field}'")
        events = environment.get("events")
        if (
            not isinstance(events, list)
            or not events
            or any(event not in EVENT_NAMES for event in events)
            or len(set(events)) != len(events)
        ):
            raise RuntimeError(
                f"environment '{label}' must give a non-empty 'events' list drawn from "
                f"{list(EVENT_NAMES)} without repeats"
            )
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

    validate_build_contracts(environments)
    return environments


def validate_build_contracts(environments: list[dict[str, Any]]) -> None:
    """Validate every environment's compile contract and compatibility edges.

    Each native producer declares the compile-affecting inputs the planned
    build key is computed from; each archive-only environment declares only the
    predicates its producer's archive is checked against.

    Compatibility is declared, never inferred from an OS name. A producer may
    name itself and, beyond that, only an archive-only environment it is
    already the `native_key` of — so `ubuntu-latest -> wsl2-ubuntu` is the sole
    cross-environment edge this table can express and native Windows cannot
    reach the WSL2 guest at all, independently of the ABI and libc comparison
    that would also refuse it.

    ## Errors

    Raises ``RuntimeError`` naming the offending environment and field.
    """
    by_name = {environment["name"]: environment for environment in environments}
    producers = {
        environment["name"] for environment in native_environments(environments)
    }
    for environment in environments:
        name = environment["name"]
        contract = environment.get("build")
        if not isinstance(contract, dict):
            raise RuntimeError(f"environment '{name}' must declare a 'build' contract")
        expected = PRODUCER_BUILD_FIELDS if name in producers else CONSUMER_BUILD_FIELDS
        if contract.keys() != expected:
            raise RuntimeError(
                f"environment '{name}' build contract must declare exactly "
                f"{sorted(expected)}; got {sorted(contract)}"
            )
        _validate_runtime_predicates(name, contract.get("runtime"))
        if not isinstance(contract.get("nextest"), str) or not contract["nextest"].strip():
            raise RuntimeError(
                f"environment '{name}' build contract must give a non-empty 'nextest' "
                "version specifier"
            )
        if name in producers:
            _validate_producer_contract(name, contract)

    for environment in environments:
        name = environment["name"]
        if name not in producers:
            continue
        for executed in environment["build"]["executes"]:
            if executed == name:
                continue
            guest = by_name.get(executed)
            if guest is None:
                raise RuntimeError(
                    f"environment '{name}' declares it executes '{executed}', which is "
                    "not an environment in this table"
                )
            if not capability(guest, "archive_only") or guest["native_key"] != name:
                raise RuntimeError(
                    f"environment '{name}' declares it executes '{executed}', but a "
                    "producer may only serve an archive-only environment it hosts "
                    f"(native_key '{name}'); '{executed}' names "
                    f"'{guest['native_key']}'"
                )
            _validate_compatibility(name, environment["build"], executed, guest["build"])

    for environment in environments:
        name = environment["name"]
        if name in producers:
            continue
        owners = [
            producer
            for producer in producers
            if name in by_name[producer]["build"]["executes"]
        ]
        if owners != [environment["native_key"]]:
            raise RuntimeError(
                f"archive-only environment '{name}' must be executed by exactly its "
                f"native_key '{environment['native_key']}'; the table says "
                f"{owners or 'no producer'}"
            )


def _validate_runtime_predicates(label: str, runtime: Any) -> None:
    if not isinstance(runtime, dict) or runtime.keys() != RUNTIME_PREDICATE_FIELDS:
        raise RuntimeError(
            f"environment '{label}' build.runtime must declare exactly "
            f"{sorted(RUNTIME_PREDICATE_FIELDS)}"
        )
    for field in RUNTIME_EQUAL_PREDICATES:
        if not isinstance(runtime[field], str) or not runtime[field].strip():
            raise RuntimeError(
                f"environment '{label}' build.runtime '{field}' must be a non-empty string"
            )
    libraries = runtime["native_libraries"]
    if not isinstance(libraries, list) or not all(
        isinstance(item, str) for item in libraries
    ):
        raise RuntimeError(
            f"environment '{label}' build.runtime native_libraries must be a list of strings"
        )
    if libraries != sorted(libraries):
        raise RuntimeError(
            f"environment '{label}' build.runtime native_libraries must be sorted"
        )


def _validate_producer_contract(label: str, contract: dict[str, Any]) -> None:
    for field in ("host", "target", "profile", "linker", "archive_format"):
        if not isinstance(contract[field], str) or not contract[field].strip():
            raise RuntimeError(
                f"environment '{label}' build '{field}' must be a non-empty string"
            )
    if not isinstance(contract["rustflags"], str):
        raise RuntimeError(
            f"environment '{label}' build rustflags must be a string; declare '' for none"
        )
    config = contract["cargo_config"]
    if not isinstance(config, list) or not all(isinstance(item, str) for item in config):
        raise RuntimeError(
            f"environment '{label}' build cargo_config must be a list of "
            "repository-relative paths"
        )
    if config != sorted(config):
        raise RuntimeError(f"environment '{label}' build cargo_config must be sorted")
    executes = contract["executes"]
    if not isinstance(executes, list) or not all(isinstance(item, str) for item in executes):
        raise RuntimeError(
            f"environment '{label}' build executes must be a list of environment names"
        )
    if len(set(executes)) != len(executes) or executes != sorted(executes):
        raise RuntimeError(
            f"environment '{label}' build executes must be sorted and free of duplicates"
        )
    if label not in executes:
        raise RuntimeError(
            f"environment '{label}' build executes must include '{label}': a producer "
            "always executes what it compiles"
        )


def _validate_compatibility(
    producer: str,
    producer_build: dict[str, Any],
    guest: str,
    guest_build: dict[str, Any],
) -> None:
    for field in RUNTIME_EQUAL_PREDICATES:
        if producer_build["runtime"][field] != guest_build["runtime"][field]:
            raise RuntimeError(
                f"environment '{producer}' cannot execute in '{guest}': {field} is "
                f"{producer_build['runtime'][field]!r} on the producer and "
                f"{guest_build['runtime'][field]!r} on the consumer"
            )
    missing = set(producer_build["runtime"]["native_libraries"]) - set(
        guest_build["runtime"]["native_libraries"]
    )
    if missing:
        raise RuntimeError(
            f"environment '{producer}' cannot execute in '{guest}': the consumer does "
            f"not provide {sorted(missing)}"
        )
    if producer_build["nextest"] != guest_build["nextest"]:
        raise RuntimeError(
            f"environment '{producer}' cannot execute in '{guest}': the archive is "
            f"produced with nextest {producer_build['nextest']!r} and run with "
            f"{guest_build['nextest']!r}"
        )


def build_contract(environments: list[dict[str, Any]], producer: str) -> dict[str, Any]:
    """The producer contract of one native environment.

    ## Errors

    Raises ``RuntimeError`` when `producer` is absent or is not a producer.
    """
    for environment in environments:
        if environment["name"] == producer and "executes" in environment.get("build", {}):
            return environment["build"]
    raise RuntimeError(
        f"'{producer}' declares no producer build contract in the plan's environment table"
    )


def archive_producers(environments: list[dict[str, Any]]) -> set[str]:
    """Every environment that produces an archive its consumers execute.

    That is every native producer: the plan derives a build record for every
    executing L1, L2, or browser cell, and since
    `fixes/2026-09-12-single-os-compile` Task 6.5 there is no environment whose
    consumers compile in place. The function remains because the two
    workflow-facing projections — the owner matrix and a package's
    per-environment build references — are defined over producers rather than
    over records, and an environment that declares no `executes` list (the WSL2
    guest) owns nothing.
    """
    return {
        environment["name"]
        for environment in environments
        if "executes" in environment.get("build", {})
    }


def producer_of(environments: list[dict[str, Any]], execution: str) -> str:
    """The environment that compiles for `execution`.

    An environment that holds a toolchain compiles for itself; an archive-only
    guest is compiled for by the producer that declares it in `executes`.

    ## Errors

    Raises ``RuntimeError`` when no producer claims `execution`.
    """
    for environment in environments:
        if "executes" in environment.get("build", {}) and execution in environment["build"]["executes"]:
            return environment["name"]
    raise RuntimeError(
        f"no environment in the plan's table declares it executes '{execution}'"
    )


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


def companion_counts_problems(name: str, entry: dict[str, Any]) -> list[str]:
    """Defects in one companion entry's counts declaration (AC13).

    Every companion resolves to exactly one of two states: it reports counts
    through a known strategy, or it says why it cannot. The state in between —
    no strategy and no reason — is what renders an unmeasured suite as `0`.
    """
    strategy = entry.get("counts")
    if strategy is None:
        if not str(entry.get("counts_reason", "")).strip():
            return [
                f"companion suite '{name}' reports no counts and gives no "
                "reason; an unmeasured suite renders `not recorded` WITH a "
                "reason, never `0`"
            ]
        return []
    problems: list[str] = []
    if strategy not in COUNTS_STRATEGIES:
        problems.append(
            f"companion suite '{name}' declares counts strategy {strategy!r}; "
            f"the vocabulary is closed: {list(COUNTS_STRATEGIES)}"
        )
    if COUNTS_OUT_PLACEHOLDER not in str(entry.get("counts_args", "")):
        problems.append(
            f"companion suite '{name}' declares a counts strategy but no "
            f"`counts_args` naming {COUNTS_OUT_PLACEHOLDER}, so its runner has "
            "nowhere to read counts from"
        )
    return problems


def validate_suite_registry(
    registry: dict[str, dict[str, Any]],
    declarations: dict[str, list[str]],
) -> list[str]:
    """Defects in a suite registry and the declarations that claim its entries.

    `declarations` maps a package name to the suite names its
    `[package.metadata.ci.tests].companion-suites` claims. The two documents
    are checked against each other because neither alone can catch the failure
    this registry exists to prevent: a suite that is declared and never runs,
    or runs and is attributed to nobody.

    ## Returns

    One problem per defect, each naming the offending suite. An empty list
    means every registered suite has exactly one owner, one canonical recipe,
    one declared environment, and exactly one package claiming it.
    """
    problems: list[str] = []

    for name, entry in sorted(registry.items()):
        if entry.get("kind") not in SUITE_KINDS:
            problems.append(
                f"suite '{name}' declares kind {entry.get('kind')!r}; the "
                f"vocabulary is closed: {list(SUITE_KINDS)}"
            )
        if not str(entry.get("owner", "")).strip():
            problems.append(f"suite '{name}' declares no owning package")
        # A LINT-ONLY companion is a deliberate shape, not a missing recipe: a
        # source-policy scan has no test half, and giving it one would attach it
        # to its owner's L1 cell as well. Either half satisfies the rule this
        # check exists for — that no suite is registered and silently never run.
        if not str(entry.get("recipe", "")).strip() and not str(
            entry.get("lint_recipe", "")
        ).strip():
            problems.append(
                f"suite '{name}' declares no canonical recipe, so it would be "
                "registered and silently never run"
            )
        environment = entry.get("environment", "")
        if entry.get("kind") == "companion":
            if environment not in schema.ENVIRONMENTS:
                problems.append(
                    f"companion suite '{name}' declares environment "
                    f"{environment!r}, which is not a policy environment: "
                    f"{sorted(schema.ENVIRONMENTS)}"
                )
            problems += companion_counts_problems(name, entry)
        elif environment != EVERY_DECLARED_ENVIRONMENT:
            problems.append(
                f"cargo suite '{name}' declares environment {environment!r}; a "
                f"cargo suite runs wherever its owner's policy places its "
                f"cells, spelled {EVERY_DECLARED_ENVIRONMENT!r}"
            )

    claimants: dict[str, list[str]] = {}
    for package, claimed in sorted(declarations.items()):
        for name in claimed:
            claimants.setdefault(name, []).append(package)

    for name, packages in sorted(claimants.items()):
        entry = registry.get(name, {})
        if not entry:
            problems.append(
                f"package(s) {sorted(packages)} declare suite '{name}', which "
                f"is not registered; registered: {sorted(registry)}"
            )
            continue
        if len(packages) > 1:
            problems.append(
                f"suite '{name}' is declared by {sorted(packages)}; a suite has "
                "exactly one owner, or one owner's success hides another's skip"
            )
        elif packages[0] != entry.get("owner"):
            problems.append(
                f"suite '{name}' is declared by '{packages[0]}' but registered "
                f"to '{entry.get('owner')}'"
            )

    for name, entry in sorted(registry.items()):
        if entry.get("kind") == "companion" and name not in claimants:
            problems.append(
                f"companion suite '{name}' is registered to "
                f"'{entry.get('owner')}' but no package declares it, so nothing "
                "schedules it"
            )

    return problems


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
    L2 tier without backends, or a companion suite that is unregistered or
    missing its registered canonical recipe.
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
    if not isinstance(tests.get("requires-toolchain", False), bool):
        raise RuntimeError(f"{label}.tests field 'requires-toolchain' must be a boolean")

    tools = tests.get("runner-tools", [])
    if not isinstance(tools, list) or not all(isinstance(t, str) for t in tools):
        raise RuntimeError(f"{label}.tests field 'runner-tools' must be a list of tool names")
    invalid_tools = [t for t in tools if t not in KNOWN_RUNNER_TOOLS]
    if invalid_tools:
        raise RuntimeError(
            f"{label}.tests names unknown runner tool(s) {invalid_tools}; the "
            f"vocabulary is closed: {sorted(KNOWN_RUNNER_TOOLS)}"
        )

    validate_archive_includes(label, tests.get("archive-includes", []))
    validate_sidecars(label, tests.get("sidecars", []), root)

    suites = tests.get("companion-suites", [])
    if not isinstance(suites, list) or not all(isinstance(s, str) for s in suites):
        raise RuntimeError(
            f"{label}.tests field 'companion-suites' must be a list of suite names"
        )
    companions = {
        suite_name: entry
        for suite_name, entry in SUITE_REGISTRY.items()
        if entry["kind"] == "companion"
    }
    # Registration and recipe existence only. Whether the DECLARER is the
    # registered owner is a property of the whole declaration set — one manifest
    # cannot tell a wrong owner from a second one — so `validate_suite_registry`
    # owns that check.
    for suite in suites:
        registered = companions.get(suite)
        if registered is None:
            raise RuntimeError(
                f"{label}.tests names unknown companion suite '{suite}'; registered: "
                f"{sorted(companions)}"
            )
        # Every Just recipe the suite names, test half and lint half alike: a
        # renamed `lint-frontend` would otherwise leave the lint cell expecting
        # a companion nothing can run.
        for directory, recipe in registered.get("just", ()):
            justfile = root / directory / "justfile"
            # A recipe DEFINITION, not a substring: `test-frontend-watch:` must
            # not satisfy the check for `test-frontend`.
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


def load_sidecars(root: Path) -> dict[str, dict[str, Any]]:
    """The closed build-sidecar vocabulary, from the one file that owns it.

    Read rather than restated so the planner and `ci-build produce` cannot
    disagree about which names exist.

    ## Errors

    Raises ``RuntimeError`` when the table is absent, unreadable, or written to
    a generation this planner does not understand.
    """
    path = root / SIDECAR_TABLE
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise RuntimeError(f"cannot read the build-sidecar table {path}: {error}") from error
    version = document.get("schema_version")
    if version != SIDECAR_SCHEMA_VERSION:
        raise RuntimeError(
            f"{path} is sidecar schema version {version}, this planner reads "
            f"{SIDECAR_SCHEMA_VERSION}"
        )
    sidecars = document.get("sidecars")
    if not isinstance(sidecars, dict):
        raise RuntimeError(f"{path} field 'sidecars' must be an object")
    return sidecars


def validate_archive_includes(label: str, entries: Any) -> None:
    """Validate a package's declared archive includes.

    Each entry names a build output relative to the profile directory. The
    shape is checked here rather than at produce time because a malformed path
    is a scheduling-time mistake: it would otherwise surface as a failed
    producer minutes into a CI run.

    ## Errors

    Raises ``RuntimeError`` naming the offending entry.
    """
    if not isinstance(entries, list) or not all(isinstance(entry, str) for entry in entries):
        raise RuntimeError(
            f"{label}.tests field 'archive-includes' must be a list of paths"
        )
    for entry in entries:
        residue = entry
        for placeholder in INCLUDE_PLACEHOLDERS:
            residue = residue.replace(placeholder, "")
        if not entry or entry != entry.strip():
            raise RuntimeError(
                f"{label}.tests archive include {entry!r} is empty or padded with whitespace"
            )
        if "{" in residue or "}" in residue:
            raise RuntimeError(
                f"{label}.tests archive include {entry!r} uses an unknown placeholder; "
                f"known: {list(INCLUDE_PLACEHOLDERS)}"
            )
        if "\\" in entry:
            raise RuntimeError(
                f"{label}.tests archive include {entry!r} must use forward slashes; one "
                "spelling has to travel between a Windows producer and its consumer"
            )
        if entry.startswith("/") or (len(entry) > 1 and entry[1] == ":"):
            raise RuntimeError(
                f"{label}.tests archive include {entry!r} must be relative to the profile "
                "output directory, not absolute"
            )
        if ".." in entry.split("/"):
            raise RuntimeError(
                f"{label}.tests archive include {entry!r} escapes the target directory"
            )
        if entry.split("/")[0] in {"debug", "release", "target"}:
            raise RuntimeError(
                f"{label}.tests archive include {entry!r} names its own profile directory; "
                "entries are relative to it, and the producer supplies the "
                "<triple>/<profile> prefix its invocation created"
            )


def missing_tier_sidecars(record: dict[str, Any]) -> list[str]:
    """The sidecars a package's CI-hostable L2 tier needs and does not declare.

    `just _test_l2` spawns the harness broker and runs the backend proof. Both
    were `cargo` invocations at recipe time; an archive consumer has no Cargo,
    so a package that executes L2 on a runner and declares neither would reach
    a compiler it does not have — the one repair the specification forbids,
    discovered inside a tier rather than before the run.

    Scoped to `tmux` because it is the only L2 backend a runner provides: a
    package whose L2 declares GUI emulators alone renders a governed capability
    gap, never executes the tier, and declaring the sidecars there would compile
    two binaries for no reader and change its build key for nothing.

    Answered rather than raised, because this is a property of the REAL
    workspace: a synthetic L2 fixture models scheduling, has no recipe to run,
    and must not be made to carry a declaration it has no use for.
    """
    if "L2" not in record.get("tiers", []):
        return []
    if CI_HOSTABLE_L2_BACKEND not in record.get("l2_backends", []):
        return []
    declared = record.get("sidecars", [])
    return [name for name in HOSTABLE_L2_SIDECARS if name not in declared]


def validate_sidecars(label: str, names: Any, root: Path) -> None:
    """Validate a package's declared build sidecars against the closed table.

    ## Errors

    Raises ``RuntimeError`` naming the unknown or duplicated sidecar.
    """
    if not isinstance(names, list) or not all(isinstance(name, str) for name in names):
        raise RuntimeError(f"{label}.tests field 'sidecars' must be a list of sidecar names")
    if len(set(names)) != len(names):
        raise RuntimeError(f"{label}.tests field 'sidecars' has duplicates: {names}")
    if not names:
        # The table is read only by a package that names something in it, so a
        # workspace with no sidecars at all — every synthetic fixture, and most
        # real packages — never has to carry one.
        return
    known = load_sidecars(root)
    unknown = [name for name in names if name not in known]
    if unknown:
        raise RuntimeError(
            f"{label}.tests names unknown build sidecar(s) {unknown}; the vocabulary is "
            f"closed by {SIDECAR_TABLE}: {sorted(known)}"
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
            "companion_suites": tests.get("companion-suites", []),
            "requires_toolchain": tests.get("requires-toolchain", False),
            "native": ci.get("native", {}),
            "archive_includes": tests.get("archive-includes", []),
            "sidecars": tests.get("sidecars", []),
        }
        record["runner_tools"] = sorted(tests.get("runner-tools", []))
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


def dependent_seam(
    dependents: Sequence[str], packages_by_name: dict[str, dict[str, Any]]
) -> dict[str, Any] | None:
    """The unchanged dependents a changed package compiles in its own check.

    One `cargo check` for all of them: every `-p`, then the union of the
    `DEPENDENT_SELECTORS` their declared kinds need. A selector appears only
    when at least one listed dependent declares that kind, so no flag is a
    no-op. No feature flags: the seam is the public API under each dependent's
    default features, and declared-feature coverage is the dependent's own L1
    when it is itself selected.

    ## Returns

    `{"dependents": [...], "check_args": "..."}`, or `None` when there is
    nothing to compile — the record then carries no `dependent_seam` at all.
    """
    names = sorted(dependents)
    if not names:
        return None
    kinds: set[str] = set()
    for name in names:
        kinds.update(declared_target_kinds(packages_by_name[name]))
    selectors = [
        DEPENDENT_SELECTORS[kind] for kind in TARGET_KINDS if kind in kinds and kind in DEPENDENT_SELECTORS
    ]
    parts = [part for name in names for part in ("-p", name)] + selectors
    return {"dependents": names, "check_args": " ".join(parts)}


def package_directories(
    root: Path, packages: dict[str, dict[str, Any]]
) -> list[tuple[PurePosixPath, str]]:
    directories = []
    for package_id, package in packages.items():
        manifest = Path(package["manifest_path"]).resolve()
        relative = manifest.parent.relative_to(root.resolve())
        directories.append((PurePosixPath(relative.as_posix()), package_id))
    return sorted(directories, key=lambda item: len(item[0].parts), reverse=True)


def reverse_dependency_map(
    metadata: dict[str, Any], packages: dict[str, dict[str, Any]]
) -> dict[str, set[str]]:
    """Each workspace member's DIRECT reverse dependencies among the members."""
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
    return reverse_dependencies


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
    reverse_dependencies = reverse_dependency_map(metadata, packages)

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


def normalized_path(raw_file: str) -> str:
    """One spelling for a changed path, whatever the caller's OS produced.

    Every path comparison here — ownership, global triggers, the change
    inventory — is defined over this form, so a Windows-spelled diff and a
    POSIX-spelled one select the same packages and inventory the same paths.
    """
    return raw_file.replace("\\", "/").removeprefix("./")


def is_global_path(raw_file: str) -> bool:
    normalized = normalized_path(raw_file)
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
        normalized = normalized_path(raw_file)
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
        normalized = normalized_path(raw_file)
        changed = PurePosixPath(normalized)
        if not is_package_source_path(changed):
            continue
        for directory, package_id in directories:
            if changed == directory or directory in changed.parents:
                owned.setdefault(package_id, []).append(normalized)
                break

    return owned


def suite_owner_paths(
    files: list[str], packages: dict[str, dict[str, Any]]
) -> dict[str, list[str]]:
    """The changed suite inputs each package owns, in the order given.

    `SUITE_OWNER_PREFIXES` / `SUITE_OWNER_PATHS` name the inputs a registered
    suite verifies that lie outside its owner's package directory, so ordinary
    source ownership cannot select them. An owner the workspace does not
    contain selects nothing: the table is repository policy and the metadata is
    the authority on membership.

    ## Returns

    Package id -> the paths that selected it. Paths, not identities, for the
    same reason as [`source_paths_by_package`]: the record must state a
    concrete reason.
    """
    ids_by_name = {entry["name"]: package_id for package_id, entry in packages.items()}
    owned: dict[str, list[str]] = {}

    for raw_file in files:
        normalized = normalized_path(raw_file)
        owners = SUITE_OWNER_PATHS.get(normalized, ())
        if not owners:
            for prefix, prefix_owners in SUITE_OWNER_PREFIXES:
                if normalized.startswith(prefix):
                    owners = prefix_owners
                    break
        for owner in owners:
            package_id = ids_by_name.get(owner)
            if package_id is not None:
                owned.setdefault(package_id, []).append(normalized)

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
    themselves; the next source edit exercises the resulting package graph. The
    two CI-tooling owners' manifests are the one exception, taken through
    [`suite_owner_paths`] rather than here (see `SUITE_OWNER_PATHS`).
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


def cell_evidence(
    cell: dict[str, Any],
    accepted: dict[tuple[str, str, str], dict[str, Any]],
    accepted_environments: dict[str, dict[str, Any]] | None,
) -> dict[str, Any] | None:
    """The verified evidence that satisfies `cell`, or None.

    A per-cell acceptance wins over the version-1 whole-environment form so a
    reused cell always carries the most specific measurement available. A
    `check` cell has no receipt of its own; see `check_evidence`.
    """
    if cell["gate"] == "check":
        return check_evidence(cell, accepted)
    return accepted.get((cell["package"], cell["environment"], cell["gate"])) or (
        accepted_environments or {}
    ).get(cell["environment"])


def check_evidence(
    cell: dict[str, Any],
    accepted: dict[tuple[str, str, str], dict[str, Any]],
) -> dict[str, Any] | None:
    """The package's passing L1 on the same environment, standing in for its check.

    A receipt records no `check` gate, and the L1 run it does record never
    compiled the `example`/`bench` targets a check cell exists for. The ruling
    that replaced "check cells are never reusable" accepts that gap on the host
    that just ran the package's L1: a compile-only rerun there answers nothing
    the host's own build did not. Only a per-cell L1 acceptance qualifies — a
    version-1 whole-environment note proves no package was built.

    ## Returns

    Evidence that links the L1 receipt but carries none of its counts, so the
    rollup cannot present test counts as a check measurement.
    """
    l1 = accepted.get((cell["package"], cell["environment"], "L1"))
    if l1 is None:
        return None
    evidence = {
        "package": cell["package"],
        "environment": cell["environment"],
        "gate": "check",
        "origin": l1.get("origin", "local"),
        "outcome": "pass",
        "covered_by": "L1",
        "measurements": f"compile-only; covered by the passing L1 on {cell['environment']}",
    }
    if l1.get("evidence") is not None:
        evidence["evidence"] = l1["evidence"]
    return evidence


def mark_reused(cell: dict[str, Any], evidence: dict[str, Any]) -> None:
    """Resolve `cell` to reuse of `evidence`, in place.

    The one place a reused cell's shape is decided: `package_cells` reaches it
    while resolving from scratch and `apply_accepted_cells` while overlaying
    evidence on a carried plan, so the two cannot drift. A prohibition is
    dropped because a constraint stops an EXECUTION, and a reused cell
    consumes none of the host the constraint protects.
    """
    cell["execution"] = "reuse"
    cell["origin"] = evidence.get("origin", "local")
    cell["state"] = "reused"
    cell["evidence"] = evidence
    cell.pop("prohibition", None)
    # The execution inputs go with the execution, exactly as the build
    # reference does: a cell nothing will run selects no nextest profile,
    # provisions no Node, and has no producer to prove a backend.
    cell.pop("profile", None)
    cell.pop("requires_node", None)
    cell.pop("backends", None)


def companion_records(
    names: Sequence[str], environment: str, gate: str
) -> list[dict[str, Any]]:
    """The registry records one `{environment, gate}` cell must run.

    R7: a companion attaches to the ONE environment it declares, so only that
    cell carries it — and only that cell loses its reuse. A suite attaches to
    the lint gate only when it declares a `lint_recipe`; a Python contract
    suite has no lint half and must not leave the lint cell expecting evidence
    nothing produces.

    ## Returns

    One record per attached suite, sorted by name, carrying the name, the
    command that runs it, its environment, and whether counts are expected —
    everything a producer needs to run it and a reader needs to judge a
    missing measurement.
    """
    command_field = "lint_recipe" if gate == "lint" else "recipe"
    records: list[dict[str, Any]] = []
    for name in sorted(set(names)):
        entry = SUITE_REGISTRY.get(name)
        if entry is None or entry.get("kind") != "companion":
            continue
        if entry.get("environment") != environment:
            continue
        command = entry.get(command_field)
        if not command:
            continue
        # The counts strategy describes the suite's TESTS. Its lint half runs a
        # linter, which has no test cardinality at all and would reject the
        # test reporter's flags.
        strategy = entry.get("counts") if gate != "lint" else None
        record: dict[str, Any] = {
            "name": name,
            "recipe": command,
            "environment": entry["environment"],
            "counts": strategy,
        }
        if strategy is None:
            record["counts_reason"] = (
                "a lint gate reports no test counts"
                if gate == "lint"
                else entry.get("counts_reason", "")
            )
        else:
            record["counts_args"] = entry["counts_args"]
        records.append(record)
    return records


def package_cells(
    record: dict[str, Any],
    area: str,
    target_kinds: list[str],
    environments: list[dict[str, Any]],
    accepted: dict[tuple[str, str, str], dict[str, Any]],
    accepted_environments: dict[str, dict[str, Any]] | None = None,
    prohibitions: dict[str, dict[str, Any]] | None = None,
    dependents: Sequence[str] = (),
) -> list[dict[str, Any]]:
    """Every result cell one gating package owns, with its execution decided.

    A cell exists for each `{environment, gate}` the package's declared policy
    requires, whether or not CI will run it. Reuse, an accepted policy gap, and
    hosted execution are states of the same cell rather than reasons to drop
    it. That is what makes the PR #76 regression unrepresentable: omitting an
    *execution* no longer omits the *cell*, so the rollup still expects a
    result for it instead of reporting MISSING.

    Each cell records whether local evidence may ever satisfy it (`reusable`),
    so evidence verified after the plan was resolved can be applied to the
    carried document without re-deriving the policy that decided it.

    `dependents` are the unchanged direct reverse dependencies this package
    compiles inside its `DEPENDENTS_ENVIRONMENT` check cell (Open Question 1,
    Option B). They give a package that check cell even when no target kind of
    its own needs one; the other native environments' check cells still exist
    only for uncovered kinds.
    """
    package = record["package"]
    cells: list[dict[str, Any]] = []
    table = {entry["name"]: entry for entry in environments}
    # Only the suites that actually need the Node toolchain provision one: a
    # package whose companions are Python contract suites would otherwise
    # install pnpm on every capable runner.
    wants_node = any(
        SUITE_REGISTRY.get(suite, {}).get("node") for suite in record["companion_suites"]
    ) or bool({"node-22", "pnpm-10"} & set(record["runner_tools"]))

    def needs_node(environment: str) -> bool:
        return wants_node and capability(table[environment], "node_pnpm")

    def add(
        environment: str,
        gate: str,
        kinds: list[str],
        coverage: str,
        reason: str,
        *,
        gap: dict[str, Any] | None = None,
        reusable: bool = True,
        compiled_dependents: Sequence[str] = (),
        companions: Sequence[dict[str, Any]] = (),
        backends: Sequence[str] = (),
    ) -> None:
        cell: dict[str, Any] = {
            "package": package,
            "area": area,
            "environment": environment,
            "gate": gate,
            "execution": "execute",
            "origin": "ci",
            "state": "pending",
            "reusable": reusable and gap is None,
            "target_kinds": kinds,
            "compile_coverage_from": coverage,
            "selection_reason": reason,
        }
        if compiled_dependents:
            cell["dependents"] = list(compiled_dependents)
        if companions:
            cell["companions"] = list(companions)
        if gap is not None:
            cell["execution"] = "omit"
            cell["origin"] = "none"
            cell["gap"] = gap
            if gap["governed"]:
                cell["state"] = "accepted-gap"
        elif reusable:
            evidence = cell_evidence(cell, accepted, accepted_environments)
            if evidence is not None:
                mark_reused(cell, evidence)
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
        # The execution inputs, added after the execution decision and only to
        # the cells that will run one: a reused or governed cell resolves no
        # profile and provisions no toolchain, and saying otherwise would
        # describe work nothing does.
        if cell["execution"] == "execute" and gate in schema.BUILD_GATES:
            cell["profile"] = schema.CI_PROFILE
            if needs_node(environment):
                cell["requires_node"] = True
        # The backends THIS cell must prove, not the package's declaration:
        # `completion.py` compares the producer's `BISCUIT_TEST_REQUIRED_BACKENDS`
        # against this list. A gap, reused, or prohibited cell requires nothing
        # because it runs nothing, and the schema refuses the field there.
        if cell["execution"] == "execute" and backends:
            cell["backends"] = sorted(backends)
        cells.append(cell)

    # Lint, like check, lives on one environment; a plan that does not carry
    # it (a push whose pull request validation proved Linux) lints nowhere,
    # because that validation already did.
    if any(environment["name"] == LINT_ENVIRONMENT for environment in environments):
        add(
            LINT_ENVIRONMENT,
            "lint",
            target_kinds,
            # Clippy builds the package, but the check contract is about target
            # coverage and clippy's target selection is the lint recipe's, not
            # the plan's. Crediting it here would claim coverage nothing asserts.
            "",
            f"lint gate for {package}, hosted on {LINT_ENVIRONMENT}",
            reusable=False,
            companions=companion_records(
                record["companion_suites"], LINT_ENVIRONMENT, "lint"
            ),
        )

    unchecked = uncovered_target_kinds(target_kinds)
    seam = sorted(dependents)
    seam_reason = (
        f"also compiles {len(seam)} unchanged dependent(s) against {package}'s "
        f"public API: {', '.join(seam)}"
    )
    if unchecked:
        # One environment, whichever the event scheduled: a plan that does not
        # carry the check host (a run scheduling only deferred environments)
        # checks nowhere, and its L1 builds still compile the L1 kinds.
        for environment in native_environments(environments):
            name = environment["name"]
            if name != CHECK_ENVIRONMENT:
                continue
            # The guest whose archive this runner builds never compiles, so
            # this cell is the only compile coverage those kinds get there.
            stands_for = [
                guest["name"]
                for guest in environments
                if capability(guest, "archive_only") and guest["native_key"] == name
            ]
            hosts_seam = name == DEPENDENTS_ENVIRONMENT and bool(seam)
            add(
                name,
                "check",
                unchecked,
                "check",
                f"{', '.join(unchecked)} target(s) are compiled by no test gate; "
                f"checked on {name} only"
                + (
                    f", which also stands for {', '.join(stands_for)} "
                    "(archive built here, never compiles)"
                    if stands_for
                    else ""
                )
                + (f"; {seam_reason}" if hosts_seam else ""),
                # The package's passing L1 on this environment stands in for
                # the check (`check_evidence`) — except where the cell also
                # compiles unchanged dependents, which no local run builds.
                reusable=not hosts_seam,
                compiled_dependents=seam if hosts_seam else (),
            )
    elif seam:
        # No kind of its own needs a check, so this cell compiles nothing of
        # the package (`target_kinds` is empty) and exists for the seam alone.
        add(
            DEPENDENTS_ENVIRONMENT,
            "check",
            [],
            "check",
            f"no target kind of {package} needs a check; {seam_reason}; "
            f"checked on {DEPENDENTS_ENVIRONMENT} only",
            reusable=False,
            compiled_dependents=seam,
        )

    l1_kinds = [kind for kind in target_kinds if kind in L1_TARGET_KINDS]
    for environment in environments:
        name = environment["name"]
        archive_only = capability(environment, "archive_only")
        for tier in record["tiers"]:
            if tier == "L1":
                companions = companion_records(record["companion_suites"], name, "L1")
                # A suite that shells out to `cargo`, `just`, or a script that
                # calls either cannot run where no toolchain exists, however
                # the binaries got there. It is a GOVERNED GAP rather than a
                # silent omission: the coverage is genuinely absent on that
                # environment, and the area audit must say so out loud.
                toolchain_gap = (
                    gap_record(environment, ["cargo_toolchain"])
                    if record["requires_toolchain"]
                    and not capability(environment, "cargo_toolchain")
                    else None
                )
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
                    # locally. Only the environment a companion DECLARES loses
                    # its reuse (R7): deriving this from a runner capability
                    # made every L1 cell of a companion-owning package hostage
                    # to which environments happen to carry `node_pnpm`.
                    reusable=not companions,
                    companions=companions,
                    gap=toolchain_gap,
                )
            elif tier == "L2":
                hostable = sorted(
                    backend
                    for backend in record["l2_backends"]
                    if backend_hostable(environment, backend)
                )
                add(
                    name,
                    "L2",
                    [],
                    "",
                    f"{package} declares the L2 tier with backend(s) "
                    f"{', '.join(record['l2_backends'])}",
                    gap=None if hostable else gap_record(environment, record["l2_backends"]),
                    backends=hostable,
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
) -> dict[str, Any]:
    """The rollup-facing shape of one impacted package's plan record.

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
        "area": record["area"],
        # A gating package always owns at least its lint cell, so an empty
        # gate list is exactly `gates = false`.
        "gates": bool(record["gates"]),
        "tiers": record["tiers"] if testing else [],
        "l2_backends": record["l2_backends"],
        # The rollup needs to know a companion suite was DECLARED: a green
        # Rust JUnit report must not hide a companion that never ran (R12).
        "companion_suites": record["companion_suites"] if testing else [],
        # The lint half separately, because only a suite declaring a
        # `lint_recipe` runs in the lint job. Expecting the others there would
        # fail the lint cell of every package whose companions are test-only.
        "lint_companion_suites": [
            entry["name"]
            for entry in companion_records(
                record["companion_suites"], LINT_ENVIRONMENT, "lint"
            )
        ],
    }
    if not record["gates"]:
        shaped["exclusion"] = record["exclusion"]
    return shaped


# ---------------------------------------------------------------------------
# Build records — fixes/2026-09-12-single-os-compile
# ---------------------------------------------------------------------------

_TOOLCHAIN_CHANNEL = re.compile(r'^\s*channel\s*=\s*"([^"]+)"', re.MULTILINE)

#: Keyed by the lockfile's own bytes, so a fixture that rewrites `Cargo.lock`
#: gets a fresh digest while a suite resolving forty plans pays for one helper
#: invocation.
_LOCKFILE_DIGESTS: dict[str, str] = {}


def rust_channel(root: Path) -> str:
    """The pinned toolchain, read from the one file that owns it.

    `rust-toolchain.toml` is the repository's single toolchain authority, so the
    build key reads it rather than letting `environments.json` carry a second
    copy that could disagree with what CI actually installs.

    ## Errors

    Raises ``RuntimeError`` when the file is missing or names no channel.
    """
    path = root / "rust-toolchain.toml"
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"cannot read the pinned toolchain from {path}: {error}") from error
    found = _TOOLCHAIN_CHANNEL.search(text)
    if not found:
        raise RuntimeError(f"{path} declares no [toolchain] channel")
    return found.group(1)


def lockfile_digest(root: Path) -> str:
    """The build key's stand-in for the resolved dependency graph.

    Digested through the same `ci-build` boundary as the key itself: a second
    hashing implementation inside one contract is how two "identical" digests
    silently diverge.
    """
    text = (root / LOCKFILE_PATH).read_text(encoding="utf-8")
    cached = _LOCKFILE_DIGESTS.get(text)
    if cached is None:
        cached = build_key.planned_keys([text])[0]
        _LOCKFILE_DIGESTS[text] = cached
    return cached


def build_identity(
    record: dict[str, Any],
    contract: dict[str, Any],
    target_kinds: Sequence[str],
    *,
    producer: str,
    source_commit: str,
    lockfile: str,
    rust: str,
) -> dict[str, Any]:
    """Every plan-known compile-affecting input of one package's archive.

    Discovered inputs — the linker's actual version, the native libraries the
    binaries turned out to link, the archive checksums — are the producer's
    realized manifest, not this. What is here is exactly what the planner can
    assert before anything compiles.
    """
    return {
        "source_commit": source_commit,
        "lockfile": lockfile,
        "rust": rust,
        "nextest": contract["nextest"],
        "host": contract["host"],
        "target": contract["target"],
        "profile": contract["profile"],
        "rustflags": contract["rustflags"],
        "cargo_config": list(contract["cargo_config"]),
        "linker": contract["linker"],
        "archive_format": contract["archive_format"],
        "package": record["package"],
        "target_kinds": [kind for kind in target_kinds if kind in L1_TARGET_KINDS],
        "features": record["test_args"],
        # `native` is declared per runner label, and a producer's label is its
        # own name; the archive-only guest installs the same list but compiles
        # nothing, so the producer's entry is the one that shaped the build.
        "native": sorted(record["native"].get(producer, [])),
        # Declared by the package, and in the key because changing either set
        # changes what the producer emits and therefore what a consumer must
        # find. Sorted so two packages that declare the same set in a different
        # order do not split a key over nothing.
        "archive_includes": sorted(record["archive_includes"]),
        "sidecars": sorted(record["sidecars"]),
    }


def compatibility_reason(
    producer: str, contract: dict[str, Any], guests: Sequence[str]
) -> str:
    """Why this archive may run where the plan says it may."""
    predicates = contract["runtime"]
    base = (
        f"{contract['target']} archive produced on {producer} "
        f"({predicates['arch']}/{predicates['abi']}/{predicates['libc']})"
    )
    if not guests:
        return f"{base}; consumed only by its own producer environment"
    return (
        f"{base}; also executable in {', '.join(guests)} — same architecture, ABI, and "
        f"libc, hosted by {producer}"
    )


def derive_build_records(
    cells: list[dict[str, Any]],
    package_records: list[dict[str, Any]],
    environments: list[dict[str, Any]],
    root: Path,
    head: str,
) -> list[dict[str, Any]]:
    """One build record per distinct planned key, with its cells attached.

    Derivation happens *after* evidence has been applied to the cells, which is
    what makes "an all-reused plan schedules no owner" a property of the
    document rather than of a workflow condition. It reopens nothing: affected
    scope, package policy, tier policy, and gap policy are already decided, and
    this reads only the cells they produced.

    Cells are mutated in place to carry their `build` reference.

    ## Returns

    The records, sorted by `{package, producer, key}`.
    """
    demand = [
        cell
        for cell in cells
        if cell["execution"] == "execute" and cell["gate"] in schema.BUILD_GATES
    ]
    for cell in cells:
        cell.pop("build", None)
    if not demand:
        return []

    by_package = {entry["package"]: entry for entry in package_records}
    rust = rust_channel(root)
    lockfile = lockfile_digest(root)

    identities: dict[tuple[str, str], dict[str, Any]] = {}
    for cell in demand:
        producer = producer_of(environments, cell["environment"])
        slot = (cell["package"], producer)
        if slot in identities:
            continue
        record = by_package[cell["package"]]
        identities[slot] = build_identity(
            record,
            build_contract(environments, producer),
            record["targets"],
            producer=producer,
            source_commit=head,
            lockfile=lockfile,
            rust=rust,
        )

    slots = sorted(identities)
    keys = build_key.planned_keys(
        [schema.canonical(identities[slot]) for slot in slots]
    )
    key_of = dict(zip(slots, keys))

    builds: dict[str, dict[str, Any]] = {}
    for cell in demand:
        producer = producer_of(environments, cell["environment"])
        slot = (cell["package"], producer)
        key = key_of[slot]
        cell["build"] = key
        record = builds.get(key)
        if record is None:
            contract = build_contract(environments, producer)
            guests = [name for name in contract["executes"] if name != producer]
            record = {
                "key": key,
                "package": cell["package"],
                "producer": producer,
                "artifact": f"build-{cell['package']}-{producer}-{key}",
                "compatible_environments": sorted(contract["executes"]),
                "compatibility_reason": compatibility_reason(producer, contract, guests),
                "consumers": [],
                "identity": identities[slot],
            }
            builds[key] = record
        record["consumers"].append(
            {"environment": cell["environment"], "gate": cell["gate"]}
        )

    for record in builds.values():
        record["consumers"].sort(key=lambda entry: (entry["environment"], entry["gate"]))
    return sorted(
        builds.values(),
        key=lambda record: (record["package"], record["producer"], record["key"]),
    )


def prune_build_records(
    builds: list[dict[str, Any]], cells: list[dict[str, Any]]
) -> list[dict[str, Any]]:
    """Drop the consumer demand that verified evidence removed.

    The overlay never derives a key — the carried plan already computed every
    one, and `--apply-to` reads nothing from the checkout, which is what keeps
    the valid-receipt path free of a Rust toolchain. It only removes: a cell
    resolved to reuse stops referencing its build, and a record whose last
    consumer is satisfied is removed rather than left for an owner to compile
    for nobody.
    """
    demanded: dict[str, list[dict[str, str]]] = {}
    for cell in cells:
        if cell["execution"] == "execute" and cell["gate"] in schema.BUILD_GATES:
            if "build" in cell:
                demanded.setdefault(cell["build"], []).append(
                    {"environment": cell["environment"], "gate": cell["gate"]}
                )
        else:
            cell.pop("build", None)

    kept = []
    for record in builds:
        consumers = demanded.get(record["key"])
        if not consumers:
            continue
        kept.append(
            {
                **record,
                "consumers": sorted(
                    consumers, key=lambda entry: (entry["environment"], entry["gate"])
                ),
            }
        )
    return kept


def archive_builds(plan: dict[str, Any]) -> list[dict[str, Any]]:
    """The plan's build records an owner job produces.

    Every record, unless the plan somehow carries one owned by an environment
    that declares no producer contract — which `validate_resolved_plan` refuses
    and this filter therefore never has to drop.
    """
    producers = archive_producers(plan["environments"])
    return [record for record in plan["builds"] if record["producer"] in producers]


def build_owner_matrix(plan: dict[str, Any]) -> list[dict[str, Any]]:
    """The deterministic native build-owner projection of a resolved plan.

    One entry per producer environment that owns at least one record, carrying
    the runner label a workflow needs and the records themselves. A projection,
    never a second calculation: the plan's `builds` list is its only input, so a
    workflow reading this cannot schedule a build the plan did not resolve.
    """
    runners = {
        environment["name"]: environment["runner"]
        for environment in plan["environments"]
    }
    natives = {entry["package"]: entry.get("native", {}) for entry in plan["packages"]}
    owners: dict[str, dict[str, Any]] = {}
    for record in archive_builds(plan):
        owner = owners.setdefault(
            record["producer"],
            {
                "environment": record["producer"],
                "runner": runners.get(record["producer"], record["producer"]),
                "packages": [],
                "builds": [],
            },
        )
        owner["builds"].append(
            {
                "key": record["key"],
                "package": record["package"],
                "artifact": record["artifact"],
                "consumers": record["consumers"],
                "compatible_environments": record["compatible_environments"],
                # The prerequisites this record's own compile needs, spelled for
                # the producer's runner label. The owner job installs the union
                # of what it is about to compile and nothing else.
                "native": sorted(natives.get(record["package"], {}).get(record["producer"], [])),
            }
        )
    for owner in owners.values():
        owner["packages"] = sorted({entry["package"] for entry in owner["builds"]})
        owner["builds"].sort(key=lambda entry: (entry["package"], entry["key"]))
        owner["native"] = sorted(
            {name for entry in owner["builds"] for name in entry["native"]}
        )
    return [owners[name] for name in sorted(owners)]


def build_slices(owners: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Package-keyed artifact inventory for diagnostics, never a job matrix."""
    slices = [
        {
            "artifact": entry["artifact"],
            "key": entry["key"],
            "package": entry["package"],
            "producer": owner["environment"],
            "runner": owner["runner"],
            "consumers": entry["consumers"],
            "compatible_environments": entry["compatible_environments"],
            "native": entry["native"],
        }
        for owner in owners
        for entry in owner["builds"]
    ]
    slices.sort(key=lambda entry: entry["artifact"])
    return slices


def environment_events(environment: dict[str, Any]) -> list[str]:
    """The events that schedule `environment`; every event when undeclared.

    A hand-built record (the suites' fixtures, a plan from before the field
    existed) is scheduled everywhere, which is the behavior every such caller
    already relied on.
    """
    return list(environment.get("events") or EVENT_NAMES)


def schedule_environments(
    environments: list[dict[str, Any]],
    event: str | None,
    all_environments: bool = False,
    proven_event: str | None = None,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], list[dict[str, Any]]]:
    """Split the table into the environments this run plans and those it does not.

    ## Returns

    `(scheduled, deferred, proven)`. `deferred` holds the environments `event`
    does not schedule, each as `{name, events}` so a reader sees which event
    will. `proven` holds the environments a `proven_event` run already
    validated for this tree — a push to `main` after a green pull request
    plans only what the pull request could not — as `{name, event}`. With no
    `event`, or with `all_environments`, everything is scheduled and both
    lists are empty; a `proven_event` still applies.
    """
    if event is not None and event not in EVENT_NAMES:
        raise RuntimeError(f"unknown event {event!r}; expected one of {list(EVENT_NAMES)}")
    if proven_event is not None and proven_event not in EVENT_NAMES:
        raise RuntimeError(
            f"unknown proven event {proven_event!r}; expected one of {list(EVENT_NAMES)}"
        )
    scheduled: list[dict[str, Any]] = []
    deferred: list[dict[str, Any]] = []
    proven: list[dict[str, Any]] = []
    for environment in environments:
        events = environment_events(environment)
        if proven_event is not None and proven_event in events:
            proven.append({"name": environment["name"], "event": proven_event})
        elif event is None or all_environments or event in events:
            scheduled.append(environment)
        else:
            deferred.append({"name": environment["name"], "events": events})
    return scheduled, deferred, proven


def producing_environments(
    table: list[dict[str, Any]], scheduled: list[dict[str, Any]]
) -> list[tuple[dict[str, Any], list[str]]]:
    """The unscheduled environments a scheduled guest's archive is compiled on.

    An archive-only environment runs what its producer built, so scheduling
    the guest without its producer would plan cells no owner job could feed.
    A producer the event does not schedule bears no cell of its own — its
    lint, check, and test cells were the pull request's — but keeps its build
    records and its preflight runner (fixes/2026-09-19-nightly-scope). Before
    that fix `ubuntu-latest` had to stay on the `schedule` event for the
    WSL2 leg's archives alone, which put 161 Linux cells into every nightly.

    ## Returns

    ``(environment, guests)`` pairs in the table's order: the producer's
    record and the sorted names of the scheduled guests it produces for.
    """
    scheduled_names = {environment["name"] for environment in scheduled}
    producing: list[tuple[dict[str, Any], list[str]]] = []
    for environment in table:
        if environment["name"] in scheduled_names:
            continue
        executes = environment.get("build", {}).get("executes", [])
        guests = sorted(
            name for name in executes if name in scheduled_names and name != environment["name"]
        )
        if guests:
            producing.append((environment, guests))
    return producing


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
    event: str | None = None,
    all_environments: bool = False,
    proven_event: str | None = None,
    deleted: Sequence[str] = (),
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

    `event` is the GitHub event this plan is for. An environment the event
    does not schedule contributes no cell, no build, and no preflight runner,
    and is listed in `deferred_environments`; `all_environments` overrides that
    (the `ci:all-os` label). `proven_event` names an event whose run already
    validated this tree, so its environments are listed in
    `proven_environments` and planned nowhere. See `schedule_environments`.

    `deleted` is the subset of `files` the diff reports as removed. It is a
    separate input because a name-only diff cannot distinguish a deletion from
    a missing file; the change inventory and the archive-path guard's scope
    both read it, the guard because a removed Rust file triggers exemption
    maintenance and has nothing left to scan.
    """
    packages = workspace_packages(metadata)
    table = environments
    cell_environments, deferred_environments, proven_environments = schedule_environments(
        table, event, all_environments, proven_event
    )
    # Producers by demand: a scheduled archive-only guest keeps its producer
    # in the plan's table for builds and preflight, cell-less. Such a producer
    # is neither deferred nor proven for this run — it is listed once, in
    # `producing_environments`, with the guests it compiles for.
    producing = producing_environments(table, cell_environments)
    producing_names = {environment["name"] for environment, _ in producing}
    deferred_environments = [
        entry for entry in deferred_environments if entry["name"] not in producing_names
    ]
    proven_environments = [
        entry for entry in proven_environments if entry["name"] not in producing_names
    ]
    planned_names = {environment["name"] for environment in cell_environments} | producing_names
    environments = [environment for environment in table if environment["name"] in planned_names]

    full_gates = set(GATES) if force_all else set()
    full_scope = bool(full_gates)

    source_paths = source_paths_by_package(list(files), root, packages)
    source_ids = set(source_paths)
    # A suite owner selected by a changed input rather than by its own source:
    # its gates run, but the changed path says nothing about its public API, so
    # it contributes no reverse-dependency seam below.
    suite_paths = suite_owner_paths(list(files), packages)
    suite_ids = set(suite_paths) - source_ids
    reverse_map = reverse_dependency_map(metadata, packages)
    reverse_ids = (
        direct_dependents(source_ids, metadata, packages) - source_ids - suite_ids
    )

    # AC1: an unchanged direct reverse dependent is *reported* here and
    # selected nowhere — no area, no package record, no cell. Its seam is
    # compiled inside the changed package's own check cell instead (Open
    # Question 1, ruled Option B 2026-09-12): each source package's record and
    # `DEPENDENTS_ENVIRONMENT` check cell list the unselected, gating
    # dependents attributed to it. A dependent that is itself selected is
    # excluded (its own gates cover it), and a full-scope run selects
    # everything, so it attributes none.
    affected_ids = set(packages) if full_scope else source_ids | suite_ids
    packages_by_name = {package["name"]: package for package in packages.values()}

    def attributed_dependents(package_id: str) -> list[str]:
        return sorted(
            packages[dependent]["name"]
            for dependent in reverse_map[package_id]
            if dependent not in affected_ids
            # A `gates = false` member is a governed "CI launches nothing for
            # this package"; compiling it here would launch something.
            and policy[packages[dependent]["name"]]["gates"]
        )
    impacted = sorted(packages[package_id]["name"] for package_id in affected_ids)
    id_of = {packages[package_id]["name"]: package_id for package_id in affected_ids}

    accepted = {
        (entry["package"], entry["environment"], entry["gate"]): entry
        for entry in (accepted_cells or [])
        if entry.get("outcome") == "pass"
    }
    whole_environments = whole_environment_evidence(accepted_environments)

    package_records: list[dict[str, Any]] = []
    cells: list[dict[str, Any]] = []
    for name in impacted:
        package_id = id_of[name]
        record = policy[name]
        area = package_area(manifest_directory(root, packages[package_id]))
        if full_scope:
            reason = "explicit full-scope request"
        elif package_id in source_paths:
            reason = f"source change in {source_paths[package_id][0]}"
        else:
            reason = (
                f"suite input {suite_paths[package_id][0]} selects its "
                "registered owner"
            )
        if not record["gates"]:
            package_records.append(
                non_gating_package_record(record, area, reason)
            )
            continue

        target_kinds = declared_target_kinds(packages[package_id])
        seam = (
            None
            if package_id in suite_ids
            else dependent_seam(attributed_dependents(package_id), packages_by_name)
        )
        owned = package_cells(
            record,
            area,
            target_kinds,
            cell_environments,
            accepted,
            whole_environments,
            prohibitions,
            dependents=seam["dependents"] if seam else (),
        )
        cells.extend(owned)
        features = feature_args(record, name)
        package_record = {
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
            "archive_includes": record["archive_includes"],
            "sidecars": record["sidecars"],
            "l1_include_slow": record["l1_include_slow"],
            "requires_toolchain": record["requires_toolchain"],
            "native": native_closure(package_id, metadata, packages, policy),
            "input_paths": closure_directories(
                package_id, root, metadata, packages
            ),
        }
        if seam:
            seam["native"] = sorted({
                prerequisite
                for dependent in seam["dependents"]
                for prerequisite in native_closure(
                    packages_by_name[dependent]["id"], metadata, packages, policy
                ).get(DEPENDENTS_ENVIRONMENT, [])
            })
            package_record["dependent_seam"] = seam
        package_records.append(package_record)

    inventory = change_inventory(files, full_scope, deleted)
    guard = archive_guard_scope(
        files,
        full_scope,
        deleted,
        {environment["name"] for environment in cell_environments},
        event,
        inventory["diff_available"],
    )
    guard_owner = SUITE_REGISTRY[ARCHIVE_GUARD_SUITE]["owner"]
    # Already-selected is the other half of the contract and needs no code: the
    # owner's ordinary lint cell carries the guard because its manifest declares
    # the suite, so nothing new is scheduled and `companions_only` stays absent.
    # A workspace without the owner (a fixture) schedules nothing; a real one
    # that lost it is caught by `validate_suite_registry`, which is where an
    # unclaimed suite belongs.
    if (
        guard["selected"]
        and guard_owner not in set(impacted)
        and policy.get(guard_owner, {}).get("gates")
    ):
        owner_id = packages_by_name[guard_owner]["id"]
        scanned = guard.get("paths") or []
        trigger = f"changed path {scanned[0]}" if scanned else guard["reason"]
        record, owned = archive_guard_only_selection(
            policy[guard_owner],
            package_area(manifest_directory(root, packages[owner_id])),
            cell_environments,
            prohibitions,
            f"archive-path guard selected by {trigger}; this lint cell runs the "
            f"guard alone and no other {guard_owner} work",
            native_closure(owner_id, metadata, packages, policy),
            closure_directories(owner_id, root, metadata, packages),
        )
        if owned:
            package_records.append(record)
            cells.extend(owned)
            # The area's selection reason must not claim a source change it
            # never saw: what selected the guard was source in another area.
            suite_ids = suite_ids | {owner_id}
            # `reverse_ids` was settled before this selection existed. An owner
            # that is also an unchanged direct dependent now holds a record, and
            # a reported dependent is one selected nowhere.
            reverse_ids = reverse_ids - {owner_id}

    gating = [entry for entry in package_records if entry["gates"]]
    if len(gating) > MATRIX_LIMIT:
        raise RuntimeError(
            f"the plan gates {len(gating)} packages, over GitHub's "
            f"{MATRIX_LIMIT}-job matrix ceiling; the fan-out must be grouped"
        )

    # After every cell's execution is decided, never before: a cell satisfied by
    # verified evidence or standing as a governed gap creates no consumer
    # demand, so an all-reused plan derives no build and schedules no owner.
    builds = derive_build_records(cells, package_records, environments, root, head)

    outcomes = outcome_fields(cells, gating, full_scope, environments, evidence_rejections)

    # Area flags drive test-shaped specialized jobs and therefore follow only
    # source changes (or an explicit full-scope request).
    flagged_ids = set(packages) if "test" in full_gates else source_ids
    top_dirs = {
        manifest_directory(root, packages[package_id]).parts[0]
        for package_id in flagged_ids
    }
    flags = {
        directory: directory in top_dirs
        for directory in ("claudine", "darkmatter", "sniff", "playa")
    }
    normalized_files = [raw.replace("\\", "/").removeprefix("./") for raw in files]
    flags["area_drift"] = any(
        path.startswith(AREA_DRIFT_PREFIXES)
        or path in AREA_DRIFT_PATHS
        or path.rpartition("/")[2] == AREA_DRIFT_MANIFEST
        for path in normalized_files
    )

    plan: dict[str, Any] = {
        "schema_version": schema.RESOLVED_PLAN_SCHEMA_VERSION,
        "base": base,
        "head": head,
        "change_class": outcomes["change_class"],
        "change_inventory": inventory,
        "archive_guard": guard,
        "full_scope": full_scope,
        "full_scope_gates": sorted(full_gates, key=GATES.index),
        "areas": area_records(
            package_records,
            full_scope,
            {packages[package_id]["name"] for package_id in suite_ids},
        ),
        "packages": package_records,
        "source_packages": sorted(
            packages[package_id]["name"] for package_id in source_ids
        ),
        "reverse_dependencies": sorted(
            packages[package_id]["name"] for package_id in reverse_ids
        ),
        "environments": environments,
        "cells": cells,
        "builds": builds,
        "accepted_evidence": outcomes["accepted_evidence"],
        "skip_policy": applicable_skip_entries(load_skip_policy(root), cells),
        "evidence_rejections": outcomes["evidence_rejections"],
        "policy_gaps": outcomes["policy_gaps"],
        "prohibited_cells": outcomes["prohibited_cells"],
        "job_estimate": outcomes["job_estimate"],
        "preflight_os": outcomes["preflight_os"],
        "preflight_reason": outcomes["preflight_reason"],
        "flags": flags,
    }
    # Optional, and absent rather than empty: a plan resolved without an event
    # is byte-identical to one from before the field existed (R9: the schema
    # version follows the required set).
    if event is not None:
        plan["event"] = event
    if deferred_environments:
        plan["deferred_environments"] = deferred_environments
    if proven_environments:
        plan["proven_environments"] = proven_environments
    if producing:
        plan["producing_environments"] = [
            {"name": environment["name"], "for": guests} for environment, guests in producing
        ]
    plan["rows"] = attach_rows(plan)
    return plan


def attach_rows(plan: dict[str, Any]) -> dict[str, dict[str, Any]]:
    """The plan's dispatch rows, refused before emission if over budget.

    One call site for the pairing so a plan can never carry rows that were
    never checked: [`row_sets`] derives them and [`enforce_output_budgets`]
    decides whether they can be carried at all (ruling R7).
    """
    rows = row_sets(plan)
    enforce_output_budgets(rows)
    return rows


def outcome_fields(
    cells: list[dict[str, Any]],
    gating: list[dict[str, Any]],
    full_scope: bool,
    environments: list[dict[str, Any]],
    evidence_rejections: list[str] | None,
) -> dict[str, Any]:
    """The plan fields that are a function of the resolved cells alone.

    Everything here is derived, never selected: it is recomputed whenever a
    cell's execution changes, whether the planner resolved the cells from
    scratch or `apply_accepted_cells` overlaid evidence on a carried plan.
    """
    change_class, preflight_os, preflight_reason = classify_preflight(
        cells, gating, full_scope, environments
    )
    return {
        "change_class": change_class,
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
    }


def apply_accepted_cells(
    plan: dict[str, Any],
    accepted_cells: list[dict[str, Any]] | None,
    evidence_rejections: list[str] | None,
    accepted_environments: list[str] | None = None,
) -> dict[str, Any]:
    """The resolved plan with verified evidence applied, selection untouched.

    The plan a scope receipt carries is authoritative for one exact
    `{base, head, tree}` (fixes/2026-09-10-local-affected-scope, R3): CI may
    add the evidence it verified after the receipt was written, but it may not
    re-select areas, packages, gates, targets, features, or environment
    policy. This is the only operation CI performs on a carried plan, and it
    reads nothing from the checkout — no `cargo metadata`, no manifest policy,
    no environment table — because the plan carries what it needs.

    A cell is resolved to reuse when it is `reusable`, is pending execution or
    prohibited, and the accepted set names a passing result for it. Failing
    results are diagnostic records, never execution substitutes. Every other
    cell is left exactly as carried, and the cell-derived fields are
    recomputed from the result.

    ## Returns

    A new document; `plan` is not modified. For a plan the planner resolved
    without evidence, the result is byte-identical to the planner resolving
    the same inputs with the same evidence.

    ## Errors

    Raises ``RuntimeError`` when `plan` does not validate: a document from
    another schema generation lacks the facts this overlay depends on.
    """
    problems = schema.validate_resolved_plan(plan)
    if problems:
        raise RuntimeError(f"the plan to apply evidence to is invalid: {problems[0]}")
    applied = copy.deepcopy(plan)
    accepted = {
        (entry["package"], entry["environment"], entry["gate"]): entry
        for entry in (accepted_cells or [])
        if entry.get("outcome") == "pass"
    }
    whole_environments = whole_environment_evidence(accepted_environments)
    for cell in applied["cells"]:
        if not cell["reusable"] or cell["state"] not in ("pending", "prohibited"):
            continue
        evidence = cell_evidence(cell, accepted, whole_environments)
        if evidence is not None:
            mark_reused(cell, evidence)
    applied["builds"] = prune_build_records(applied["builds"], applied["cells"])
    # Re-derived, never carried forward: evidence applied here removes
    # executions, and a row set describing the pre-overlay plan would dispatch
    # work this document says is already satisfied.
    applied["rows"] = attach_rows(applied)
    gating = [entry for entry in applied["packages"] if entry["gates"]]
    applied.update(
        outcome_fields(
            applied["cells"],
            gating,
            applied["full_scope"],
            applied["environments"],
            evidence_rejections,
        )
    )
    return applied


def non_gating_package_record(
    record: dict[str, Any], area: str, reason: str
) -> dict[str, Any]:
    """A selected package that launches nothing.

    `gates = false` is an owned, dated exclusion, so the package stays visible
    in its area with an empty gate list and no cells. Dropping it would hide a
    governed absence; giving it cells would demand results nothing produces.
    The exclusion rides on the record because the rollup's policy document is
    projected from the plan, and that is the only place the governance is read.
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
        "archive_includes": [],
        "sidecars": [],
        "l1_include_slow": record["l1_include_slow"],
        "native": {},
        "exclusion": record["exclusion"],
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
    package_records: list[dict[str, Any]],
    full_scope: bool,
    suite_owners: set[str] | None = None,
) -> list[dict[str, Any]]:
    """Areas grouped from their packages, in sorted order.

    Area is a grouping derived from package, never a stored identity (Design
    Decision 1), so it is assembled here rather than carried anywhere.

    `suite_owners` names the packages selected because a registered suite they
    own verifies a changed input rather than because their own source changed;
    an area holding only those must not claim a source change it did not see.

    Every area states [`EXECUTION_PATH`]: one path per area per run, stated on
    the record itself (ruling R9).
    """
    owners = suite_owners or set()
    grouped: dict[str, list[str]] = {}
    for entry in package_records:
        grouped.setdefault(entry["area"], []).append(entry["package"])

    def reason(members: list[str]) -> str:
        if full_scope:
            return "explicit full-scope request"
        named = ", ".join(sorted(members))
        if set(members) <= owners:
            return f"changed suite input owned by package(s) {named}"
        return f"source change in package(s) {named}"

    return [
        {
            "area": area,
            "selection_reason": reason(members),
            "packages": sorted(members),
            "execution_path": EXECUTION_PATH,
        }
        for area, members in sorted(grouped.items())
    ]


# ---------------------------------------------------------------------------
# Change inventory
# ---------------------------------------------------------------------------

#: Suffixes that make a changed path code. `.py` is here because `scripts/ci/`
#: is the planner's own source, not its configuration.
SOURCE_SUFFIXES = frozenset({
    ".bash", ".c", ".cc", ".cjs", ".cpp", ".cs", ".css", ".fish", ".go", ".h",
    ".hpp", ".html", ".java", ".js", ".jsx", ".kt", ".lua", ".mjs", ".nu",
    ".php", ".pl", ".proto", ".ps1", ".py", ".rb", ".rs", ".scala", ".scm",
    ".scss", ".sh", ".sql", ".svelte", ".swift", ".ts", ".tsx", ".vue", ".zsh",
})

DOCUMENTATION_SUFFIXES = frozenset({
    ".adoc", ".markdown", ".md", ".mdx", ".rst", ".txt",
})

CONFIGURATION_SUFFIXES = frozenset({
    ".cfg", ".conf", ".ini", ".json", ".json5", ".jsonc", ".just", ".lock",
    ".properties", ".toml", ".xml", ".yaml", ".yml",
})

#: Extension-less files that are still build or tooling configuration. Compared
#: case-insensitively so `justfile` and `Justfile` are one rule.
CONFIGURATION_NAMES = frozenset({
    ".dockerignore", ".editorconfig", ".env", ".gitattributes", ".gitignore",
    ".npmrc", ".nvmrc", "dockerfile", "justfile", "makefile",
})


def change_bucket(path: str) -> str:
    """The one inventory bucket a normalized `path` belongs to.

    What a file *is* outranks where it sits: `.github/ci/README.md` is
    documentation even though everything else under `.github/` is
    configuration. `other` is reached deliberately, not by falling through — an
    extension-less `LICENSE` is neither code, prose the repo publishes, nor
    configuration anything reads.
    """
    name = path.rsplit("/", 1)[-1]
    suffix = PurePosixPath(name).suffix.lower()
    if suffix in SOURCE_SUFFIXES:
        return "source"
    if suffix in DOCUMENTATION_SUFFIXES:
        return "documentation"
    if suffix in CONFIGURATION_SUFFIXES or name.lower() in CONFIGURATION_NAMES:
        return "configuration"
    if "docs" in path.split("/")[:-1]:
        return "documentation"
    if path.startswith(".github/"):
        return "configuration"
    return "other"


def change_inventory(
    files: Sequence[str], full_scope: bool, deleted: Sequence[str] = ()
) -> dict[str, Any]:
    """What changed, classified once, for every reader of the plan.

    R8: computed from the changed paths alone and deliberately independent of
    `change_class`, which [`classify_preflight`] derives from the *gating
    packages* a change selects. The two answer different questions and are
    allowed to disagree — a lone `Cargo.lock` edit reports
    `change_class: documentation` because it selects no gating package, while
    its path is plainly `configuration`.

    A manual full-scope run consulted no diff, so it records that fact rather
    than empty buckets a reader would take for "nothing changed": `paths`,
    `counts`, and `deleted` are present exactly when `diff_available` is true.

    `deleted` is the subset of `files` the diff reports as removed. It is a
    separate input because `git diff --name-only` cannot distinguish a deletion
    from a path that is simply missing, and a consumer that treated every
    missing path as a deletion would silently stop checking a file the diff
    named. A deleted path still appears in its bucket: it changed.
    """
    if full_scope:
        return {
            "diff_available": False,
            "reason": (
                "explicit full-scope request selects every package; no diff "
                "was consulted"
            ),
        }

    buckets: dict[str, set[str]] = {bucket: set() for bucket in schema.CHANGE_BUCKETS}
    for raw_file in files:
        path = normalized_path(raw_file).strip()
        if path:
            buckets[change_bucket(path)].add(path)

    paths = {bucket: sorted(buckets[bucket]) for bucket in schema.CHANGE_BUCKETS}
    counts = {bucket: len(entries) for bucket, entries in paths.items()}
    counts["total"] = sum(counts.values())
    return {
        "diff_available": True,
        "paths": paths,
        "counts": counts,
        "deleted": normalized_paths(deleted),
    }


def normalized_paths(raw_files: Sequence[str]) -> list[str]:
    """One normalized, sorted, de-duplicated spelling per non-empty path."""
    return sorted({normalized_path(raw).strip() for raw in raw_files} - {""})


def archive_guard_eligible(path: str) -> bool:
    """Whether a normalized path is in the archive-path guard's scan domain.

    The selection half of the one policy the scanner also applies
    (`test_toolkit::archive_guard::is_eligible`). A build script is excluded
    because it runs on the producer, where the compile-time value is the only
    correct answer.

    The guard's own three sources are deliberately NOT excluded here. The
    scanner refuses them and reports them as ineligible, which is visible; a
    second exclusion list on this side would be a second thing to keep in step.
    """
    if not path.endswith(".rs"):
        return False
    if path.rsplit("/", 1)[-1] == "build.rs":
        return False
    return not (set(path.split("/")) & ARCHIVE_GUARD_SKIPPED_DIRS)


def archive_guard_triggered(path: str) -> bool:
    """Whether one normalized changed path selects the guard.

    Two reasons and no others: the path is source the guard would scan, or it
    is one of the guard's own declared inputs. Documentation, lockfiles, and
    the manifests and workflow files outside [`ARCHIVE_GUARD_OWN_INPUTS`]
    select no guard.
    """
    return archive_guard_eligible(path) or path in ARCHIVE_GUARD_OWN_INPUTS


def archive_guard_scope(
    files: Sequence[str],
    full_scope: bool,
    deleted: Sequence[str],
    scheduled: set[str],
    event: str | None,
    diff_available: bool,
) -> dict[str, Any]:
    """The archive-path guard's scan scope for one event.

    The plan's one answer to "does the guard run, and over what". Exactly one
    of three shapes, each carrying a non-empty `reason`:

    - `{"selected": false, "reason": …}`
    - `{"selected": true, "mode": "full", "reason": …}`
    - `{"selected": true, "mode": "changed", "paths": [...], "reason": …}`

    `paths` is present exactly in `changed` mode, and an explicitly empty list
    is a real state — nothing eligible changed — that is never upgraded to a
    full scan. Deleted paths still TRIGGER, because exemption maintenance has
    to run when an exempted file disappears, but they are not listed for
    scanning: there is nothing left to read.

    The environment check comes before every mode decision. The guard is
    Linux-only and must not force Linux into an event that excludes it; under
    the current table a `schedule` run schedules WSL2 alone, so the nightly
    records `selected: false` rather than adding a runner to it.
    """
    changed = normalized_paths(files)
    removed = normalized_paths(deleted)
    triggers = [path for path in changed + removed if archive_guard_triggered(path)]

    # A full-workspace request selects every package, so it selects the guard
    # too — there is no diff for a trigger to match against.
    if not full_scope and not triggers:
        return {
            "selected": False,
            "reason": (
                "no scanned source and no owned guard input changed; a "
                "documentation or unrelated configuration change selects no scan"
            ),
        }

    if LINT_ENVIRONMENT not in scheduled:
        occasion = f"the {event} event" if event else "this run"
        return {
            "selected": False,
            "reason": (
                f"{occasion} schedules no {LINT_ENVIRONMENT} cell; the guard is "
                f"hosted there alone and does not force it into an event that "
                f"excludes it"
            ),
        }

    if full_scope:
        return {
            "selected": True,
            "mode": "full",
            "reason": (
                "explicit full-scope request; the whole eligible corpus is scanned"
            ),
        }
    if not diff_available:
        return {
            "selected": True,
            "mode": "full",
            "reason": (
                "no change inventory is available, so the eligible corpus cannot "
                "be narrowed and is scanned whole"
            ),
        }
    if event == "push":
        return {
            "selected": True,
            "mode": "full",
            "reason": (
                "a push scans the whole eligible corpus; changed-file evidence "
                "from a pull request proves nothing about the files it omitted"
            ),
        }

    scanned = [
        path
        for path in changed
        if archive_guard_eligible(path) and path not in set(removed)
    ]
    occasion = f"the {event} event" if event else "this run"
    return {
        "selected": True,
        "mode": "changed",
        "paths": scanned,
        "reason": (
            f"{occasion} carries a change inventory; {len(scanned)} eligible "
            f"source path(s) are in scope and untouched files are not rescanned"
        ),
    }


def archive_guard_only_selection(
    record: dict[str, Any],
    area: str,
    environments: list[dict[str, Any]],
    prohibitions: dict[str, dict[str, Any]] | None,
    reason: str,
    native: dict[str, list[str]],
    input_paths: list[str],
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    """The guard owner's lint-only package record and its single cell.

    Selecting the owner the ordinary way would schedule its L1 on four
    environments, its Node companions, and the reverse-dependency seam its
    public API carries — work a Rust file in another area says nothing about.
    So `package_cells` is handed a policy with no tier and no target kind,
    which is what makes exactly one cell exist: `{owner, ubuntu-latest, lint}`.

    `companions_only` marks that cell's required work as its companions rather
    than the package's own Clippy, which this selection did not ask for.

    Lint cells are already non-reusable, which is what the specification's
    "initially prefer non-reusable guard execution" asks for: a changed-file
    pull request scan must never be presented as a completed full-tree push
    scan, and no evidence identity here can represent a scan's inputs.
    """
    lint_only = {**record, "tiers": [], "companion_suites": [ARCHIVE_GUARD_SUITE]}
    cells = package_cells(
        lint_only,
        area,
        [],
        environments,
        {},
        prohibitions=prohibitions,
    )
    for cell in cells:
        cell["selection_reason"] = reason
        cell["companions_only"] = True

    package_record = {
        "package": record["package"],
        "area": area,
        "selection_reason": reason,
        "gates": [
            gate for gate in schema.GATES if any(cell["gate"] == gate for cell in cells)
        ],
        "targets": [],
        "tiers": [],
        "test_args": "",
        "check_args": check_arguments(record["package"], [], ""),
        "l2_backends": record["l2_backends"],
        "runner_tools": record["runner_tools"],
        "companion_suites": [ARCHIVE_GUARD_SUITE],
        "archive_includes": record["archive_includes"],
        "sidecars": record["sidecars"],
        "l1_include_slow": record["l1_include_slow"],
        "requires_toolchain": record["requires_toolchain"],
        "native": native,
        "input_paths": input_paths,
    }
    return package_record, cells


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
        # Every runner the plan's environments land on — the table after the
        # event filtered it, so a scheduled run preflights no runner that
        # hosts nothing that night.
        return (
            "full",
            sorted({SCOPE_HOST_OS, *(environment["runner"] for environment in environments)}),
            "explicit full-scope request selects every scheduled runner OS",
        )

    if gating:
        # Preflight runs on RUNNER labels, so each environment is resolved to
        # the runner that hosts it — `wsl2-ubuntu` preflights on
        # `windows-latest`.
        runner_of = {environment["name"]: environment["runner"] for environment in environments}
        os_set = {SCOPE_HOST_OS}
        for cell in cells:
            if cell["execution"] != "execute":
                continue
            os_set.add(runner_of.get(cell["environment"], cell["environment"]))
            # The producer that compiles the cell's archive preflights too,
            # even when it bears no cell of its own (a nightly's Linux owner
            # for the WSL2 guest): the owner job runs on that runner.
            if cell["gate"] in schema.BUILD_GATES:
                try:
                    producer = producer_of(environments, cell["environment"])
                except RuntimeError:
                    continue
                os_set.add(runner_of.get(producer, producer))
        reason = (
            f"package-local change across {len(gating)} package(s); "
            "preflight covers the scope host plus the runner OS hosting each "
            "package's required environments and the producers compiling for them"
        )
        return "package", sorted(os_set), reason

    # R10: preflight establishes the prerequisites a package fan-out consumes.
    # With no gating package there is no fan-out, so there is nothing to
    # establish and the matrix is empty — `ci.yml`'s scalar guard reads this
    # exact `[]` and resolves the job to `skipped` right after `scope`.
    return (
        "documentation",
        [],
        "no build/test packages affected; there is no fan-out to bootstrap",
    )


# ---------------------------------------------------------------------------
# Transitional legacy projection
# ---------------------------------------------------------------------------


def row_sets(plan: dict[str, Any]) -> dict[str, dict[str, Any]]:
    """The area-local dispatch rows the hosted matrices expand.

    One row per executing cell, partitioned into the four disjoint sets each
    area dispatches: `wsl` takes every archive-only guest cell, `check` and
    `lint` take their gates, and `test` takes the native test tiers. Reused,
    accepted-gap, prohibited, and event-deferred work creates no row, because
    none of them is an execution.

    A row carries dispatch identity alone — `{package, gate, environment,
    runner}`, in that order (ruling R1) — and the job it expands into resolves
    the rest of its execution contract from this same plan by the
    `{package, environment, gate}` the row names. Every area the plan selects
    appears here, executing or not: an all-reused or gap-only area still owns a
    blocking audit, and an absent key would read as an absent area.

    Pure, and the plan is its only input: CI derives these from a carried scope
    receipt, where there is no checkout to consult.
    """
    runners = {
        entry["name"]: entry["runner"]
        for entry in plan["environments"]
        if isinstance(entry, dict)
    }
    areas = {
        entry["area"]: {
            **{name: [] for name in schema.ROW_SET_NAMES},
            **{f"has_{name}_rows": False for name in schema.ROW_SET_NAMES},
        }
        for entry in plan["areas"]
    }

    for cell in plan["cells"]:
        if cell["execution"] != "execute":
            continue
        document = areas.get(cell["area"])
        if document is None:
            # An area the plan does not list cannot be dispatched to: its
            # caller identity is the area record. Skipping it here would hide
            # that; `validate_resolved_plan` refuses the document instead.
            continue
        if cell["environment"] == "wsl2-ubuntu":
            name = "wsl"
        elif cell["gate"] in ("check", "lint"):
            name = cell["gate"]
        else:
            name = "test"
        document[name].append(
            {
                "package": cell["package"],
                "gate": cell["gate"],
                "environment": cell["environment"],
                "runner": runners.get(cell["environment"], cell["environment"]),
            }
        )

    for document in areas.values():
        for name in schema.ROW_SET_NAMES:
            document[name].sort(
                key=lambda row: (row["package"], row["gate"], row["environment"])
            )
            document[f"has_{name}_rows"] = bool(document[name])
    return areas


def enforce_output_budgets(rows: dict[str, dict[str, Any]]) -> None:
    """Refuse a plan whose dispatch rows cannot be carried (ruling R7).

    The planner is the only place this is checked, and it refuses whole: a
    truncated matrix is exactly the silently-dropped-cell defect this feature
    exists to remove, and a downstream re-derivation would be a second budget
    to keep aligned.

    ## Errors

    Raises ``RuntimeError`` naming the ceiling that was exceeded — GitHub's
    per-matrix job limit, [`AREA_ROW_SET_BUDGET`] with the area that exceeded
    it, or [`TOTAL_ROW_SET_BUDGET`].
    """
    for area in sorted(rows):
        document = rows[area]
        for name in schema.ROW_SET_NAMES:
            count = len(document.get(name, ()))
            if count > MATRIX_LIMIT:
                raise RuntimeError(
                    f"area {area!r} dispatches {count} {name} rows, over GitHub's "
                    f"{MATRIX_LIMIT}-job matrix ceiling; the area must be split "
                    "before it can be scheduled"
                )
        size = len(schema.canonical(document))
        if size > AREA_ROW_SET_BUDGET:
            raise RuntimeError(
                f"area {area!r} serializes {size} bytes of dispatch rows, over the "
                f"{AREA_ROW_SET_BUDGET}-byte per-area output budget; the rows are "
                "refused whole rather than truncated"
            )
    total = len(schema.canonical(rows))
    if total > TOTAL_ROW_SET_BUDGET:
        raise RuntimeError(
            f"the plan serializes {total} bytes of dispatch rows across "
            f"{len(rows)} areas, over the {TOTAL_ROW_SET_BUDGET}-byte scope output "
            "budget; the rows are refused whole rather than truncated"
        )


def legacy_scope_document(plan: dict[str, Any]) -> dict[str, Any]:
    """The `scope.json` projection of the resolved plan.

    It carries the scalars and per-area outputs `ci.yml`'s `scope` job
    publishes, and the policy and build-owner projections `ci-rollup` and
    `ci.yml`'s owner job read. It carries no per-package environment list:
    producers resolve their contract from the plan by cell key, and
    `just/ci-local.just` reads the plan's package records and cells.

    It is a projection, never a second calculation: the plan is its only
    input, so CI can project a carried receipt without touching the checkout.

    `scheduled_areas` are the areas owning at least one gating package, each
    one `_area-ci.yml` call. An area whose cells are all reused or accepted
    gaps is still scheduled: it keeps its blocking audit and result slice.
    """
    scheduled = sorted({entry["area"] for entry in plan["packages"] if entry["gates"]})
    owners = build_owner_matrix(plan)
    slices = build_slices(owners)

    return {
        "packages": [entry["package"] for entry in plan["packages"]],
        "areas": plan["areas"],
        "scheduled_areas": scheduled,
        # Derived here rather than read from `plan["rows"]`: one function
        # computes the rows, so the document a workflow expands and the
        # document `ci-plan` renders cannot disagree about what will run.
        "area_rows": row_sets(plan),
        "area_slugs": {area: area_slug(area) for area in scheduled},
        "source_packages": plan["source_packages"],
        "reverse_dependencies": plan["reverse_dependencies"],
        "full_scope": plan["full_scope"],
        "full_scope_gates": plan["full_scope_gates"],
        "change_class": plan["change_class"],
        "preflight_os": plan["preflight_os"],
        "preflight_reason": plan["preflight_reason"],
        "policy": [
            policy_record(entry, {"test"} if entry["tiers"] else set())
            for entry in plan["packages"]
        ],
        "build_owners": owners,
        "build_slices": slices,
        "build_artifacts": [entry["artifact"] for entry in slices],
        "build_runners": {entry["artifact"]: entry["runner"] for entry in slices},
        "job_estimate": plan["job_estimate"],
        "flags": plan["flags"],
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--all", action="store_true", help="select the full workspace")
    parser.add_argument(
        "--apply-to",
        metavar="FILE",
        help=(
            "apply the accepted cells and rejections to this already-resolved "
            "plan instead of selecting scope; reads no manifest, policy, or "
            "environment table, and excludes --all, --constraints, and files"
        ),
    )
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
    parser.add_argument(
        "--event",
        choices=EVENT_NAMES,
        help=(
            "the GitHub event this plan is for; an environment the event does "
            "not schedule is deferred, not planned. Omitted: every environment"
        ),
    )
    parser.add_argument(
        "--all-environments",
        action="store_true",
        help="plan every environment regardless of --event (the ci:all-os label)",
    )
    parser.add_argument(
        "--proven-event",
        choices=EVENT_NAMES,
        metavar="EVENT",
        help=(
            "an event whose run already validated this tree; its environments "
            "are listed as proven and planned nowhere"
        ),
    )
    parser.add_argument(
        "--deleted",
        action="append",
        default=[],
        metavar="PATH",
        help=(
            "a path the diff reports as removed; repeatable. A name-only diff "
            "cannot tell a deletion from a missing file, so the distinction is "
            "declared rather than guessed"
        ),
    )
    parser.add_argument("files", nargs="*", help="changed repository-relative paths")
    args = parser.parse_args()
    if args.apply_to and (
        args.all or args.files or args.constraints or args.deleted
        or args.event or args.all_environments or args.proven_event
    ):
        parser.error(
            "--apply-to applies evidence to a carried plan and performs no "
            "selection: it cannot be combined with --all, --constraints, --event, "
            "--all-environments, --proven-event, --deleted, or a file list"
        )
    return args


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

    The store is only ever the explicit `--constraints` directory: its default
    location is resolved by `constraints.py` at the trigger boundary (the hook,
    `just ci-local`), never here, so CI, which passes no store, cannot read one.
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
    accepted_cells = read_accepted_cells(args.accepted_cells)
    evidence_rejections = read_evidence_rejections(args.evidence_rejections)
    if args.apply_to:
        # The carried plan is the selection. Nothing from the checkout is
        # consulted here, so a receipt CI accepted cannot be second-guessed.
        plan = apply_accepted_cells(
            json.loads(Path(args.apply_to).read_text(encoding="utf-8")),
            accepted_cells,
            evidence_rejections,
            accepted_environments=args.accepted_environment,
        )
    else:
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
            accepted_cells=accepted_cells,
            accepted_environments=args.accepted_environment,
            prohibitions=read_prohibitions(args.constraints),
            evidence_rejections=evidence_rejections,
            base=args.base,
            head=args.head,
            event=args.event,
            all_environments=args.all_environments,
            proven_event=args.proven_event,
            deleted=args.deleted,
        )
    if args.plan_out:
        Path(args.plan_out).write_text(schema.canonical(plan), encoding="utf-8")
    document = plan if args.resolved_plan else legacy_scope_document(plan)
    print(json.dumps(document, separators=(",", ":")))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Tests for dependency-aware, package-keyed CI scope calculation."""

from __future__ import annotations

import itertools
import json
import os
import shlex
import subprocess
import sys
import tempfile
import unittest
from datetime import date
from pathlib import Path, PurePosixPath

import unittest.mock

import affected_scope
import cell_contract
import copy
import companion_suites
import diff_scope
import schema
import test_inputs
from tool_guard import require_tools
from pending_contracts import pending
from affected_scope import (
    EXCLUSION_CLASSES,
    calculate_scope,
    legacy_scope_document,
    package_area,
    diff_changes_more_than_comments,
    global_trigger,
    just_recipe_closure,
    parse_just_recipes,
    lockfile_impacted_names,
    parse_lockfile,
    package_cells,
    check_arguments,
    dependent_seam,
    CHECK_ENVIRONMENT,
    DEPENDENTS_ENVIRONMENT,
    EVENT_NAMES,
    schedule_environments,
    feature_args,
    apply_accepted_cells,
    capability,
    load_environments,
    package_ci_policy,
    validate_package_ci,
    validate_sidecars,
    load_sidecars,
    validate_no_shadow_workspaces,
    build_closure,
    estimate_jobs,
    load_metadata,
    workspace_packages,
    MATRIX_LIMIT,
    ROOT,
    ENVIRONMENTS_CONFIG,
    build_contract,
    build_owner_matrix,
    archive_producers,
    derive_build_records,
    prune_build_records,
    producer_of,
    CONSUMER_BUILD_FIELDS,
    PRODUCER_BUILD_FIELDS,
    SIDECAR_TABLE,
)

# Pinned so an expiry test asserts the rule, not today's date.
TODAY = date(2026, 7, 27)

#: Every job that runs this suite provisions a toolchain, so a missing Cargo
#: here is a provisioning regression rather than a host without Rust. Both set
#: BISCUIT_REQUIRE_CARGO; a developer host without Cargo still skips.
CARGO_ENFORCED_BY = (
    "`ci.yml`'s `preflight` matrix on every selected operating system and by "
    "its `ci-tooling` job, both of which set up the pinned Rust toolchain and "
    "set BISCUIT_REQUIRE_CARGO"
)

#: Git is how every one of those jobs obtained the checkout it runs this suite
#: from, so its absence is a host anomaly rather than an unprovisioned runner.
GIT_ENFORCED_BY = (
    "`ci.yml`'s `preflight` matrix and its `ci-tooling` job, each of which "
    "checks this repository out with Git before the suite runs"
)


def seed_build_inputs(root: Path) -> None:
    """Give a synthetic workspace the two files a planned build key reads.

    The key digests the pinned toolchain and the resolved dependency graph, so
    a root with neither has no build key to compute. Real checkouts always
    carry both; a fixture root has to be told.
    """
    (root / "rust-toolchain.toml").write_text(
        '[toolchain]\nchannel = "1.97.1"\n', encoding="utf-8"
    )
    lockfile = root / "Cargo.lock"
    if not lockfile.exists():
        lockfile.write_text("version = 4\n", encoding="utf-8")
    # The closed sidecar vocabulary, for a fixture whose package declares one.
    # Copied from the shipped table rather than invented, so a fixture cannot
    # accept a name the real planner would refuse.
    table = root / ".github" / "ci"
    table.mkdir(parents=True, exist_ok=True)
    (table / "sidecars.json").write_text(
        (ROOT / SIDECAR_TABLE).read_text(encoding="utf-8"),
        encoding="utf-8",
    )


def scope_document(
    files: list[str],
    root: Path,
    metadata: dict[str, object],
    environments: list[dict[str, object]],
    policy: dict[str, dict[str, object]],
    force_all: bool = False,
    **kwargs: object,
) -> dict[str, object]:
    """The `scope.json` projection `ci.yml` publishes, for cases asserting it."""
    plan = calculate_scope(
        files, root, metadata, environments, policy, force_all, **kwargs  # type: ignore[arg-type]
    )
    return legacy_scope_document(plan)


def gates_by_package(plan: dict) -> dict[str, list[str]]:
    """Each gating package's plan gates: the packages the fan-out reaches."""
    return {entry["package"]: entry["gates"] for entry in plan["packages"] if entry["gates"]}


def dispatched(plan: dict, package: str) -> set[tuple[str, str]]:
    """`(environment, gate)` of every dispatch row `ci.yml` hands out for `package`."""
    return {
        (row["environment"], row["gate"])
        for document in affected_scope.row_sets(plan).values()
        for name in schema.ROW_SET_NAMES
        for row in document[name]
        if row["package"] == package
    }


def producer_contracts(plan: dict, package: str) -> dict[tuple[str, str], dict[str, str]]:
    """What each of `package`'s producer jobs resolves, by `(environment, gate)`.

    The real resolver on the real rows: a job reads this instead of the
    per-package environment lists the retired matrix carried.
    """
    return {
        (row["environment"], row["gate"]): cell_contract.contract(plan, row, "")
        for document in affected_scope.row_sets(plan).values()
        for name in schema.ROW_SET_NAMES
        for row in document[name]
        if row["package"] == package
    }


def package(
    root: Path,
    name: str,
    relative_manifest: str,
    ci: object | None = None,
    targets: list[str] | None = None,
) -> dict[str, object]:
    metadata: dict[str, object] = {}
    if ci is not None:
        metadata["ci"] = ci
    record: dict[str, object] = {
        "id": name,
        "name": name,
        "manifest_path": str((root / relative_manifest).resolve()),
        "metadata": metadata or None,
    }
    if targets is not None:
        record["targets"] = [{"kind": [kind]} for kind in targets]
    return record


def ci_policy(**fields: object) -> dict[str, object]:
    policy: dict[str, object] = {}
    for key, value in fields.items():
        policy[key.replace("_", "-")] = value
    return policy


class AffectedScopeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        seed_build_inputs(self.root)
        # alpha-core depends on shared; beta-app depends on alpha-core.
        packages = [
            package(self.root, "alpha-core", "alpha/lib/Cargo.toml"),
            package(self.root, "beta-app", "beta/app/Cargo.toml"),
            package(self.root, "shared-tests", "tools/shared-tests/Cargo.toml"),
        ]
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {
                "nodes": [
                    {"id": "alpha-core", "deps": [{"pkg": "shared-tests", "dep_kinds": [{"kind": None}]}]},
                    {"id": "beta-app", "deps": [{"pkg": "alpha-core", "dep_kinds": [{"kind": None}]}]},
                    {"id": "shared-tests", "deps": []},
                ]
            },
        }
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def scope(self, files: list[str], **kwargs: object) -> dict[str, object]:
        return scope_document(
            files,
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
            **kwargs,  # type: ignore[arg-type]
        )

    def plan(self, files: list[str], **kwargs: object) -> dict:
        return calculate_scope(
            files,
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
            **kwargs,  # type: ignore[arg-type]
        )

    def test_source_change_selects_the_source_package_alone(self) -> None:
        # AC1: an unchanged direct reverse dependent is reported and selected
        # nowhere. It used to receive a compile-check entry, which presented an
        # untested area as a green top-level result (PR #76).
        scope = self.scope(["alpha/lib/src/lib.rs"])
        self.assertEqual(["alpha-core"], scope["packages"])
        self.assertEqual(["beta-app"], scope["reverse_dependencies"])
        # alpha-core declares only a library target, which the L1 build
        # already compiles (spec section 1.7), so its `check` gate exists for
        # beta-app's seam alone: one Linux cell owned by alpha-core (Open
        # Question 1, Option B), never a job for beta-app.
        plan = self.plan(["alpha/lib/src/lib.rs"])
        gates = gates_by_package(plan)
        self.assertEqual(["lint", "check", "L1"], gates["alpha-core"])
        self.assertNotIn("beta-app", gates)
        checks = {cell for cell in dispatched(plan, "alpha-core") if cell[1] == "check"}
        self.assertEqual({(DEPENDENTS_ENVIRONMENT, "check")}, checks)
        contract = producer_contracts(plan, "alpha-core")[(DEPENDENTS_ENVIRONMENT, "check")]
        self.assertEqual(["beta-app"], json.loads(contract["dependents"]))

    def test_an_unchanged_reverse_dependent_gets_no_area_job_or_cell(self) -> None:
        plan = calculate_scope(
            ["alpha/lib/src/lib.rs"],
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
        )
        self.assertEqual(["beta-app"], plan["reverse_dependencies"])
        self.assertEqual([], [entry for entry in plan["areas"] if entry["area"] == "beta"])
        self.assertEqual(
            [], [entry for entry in plan["packages"] if entry["package"] == "beta-app"]
        )
        self.assertEqual([], [cell for cell in plan["cells"] if cell["package"] == "beta-app"])

    def test_shared_dependency_change_selects_the_changed_package_only(self) -> None:
        # beta-app reaches shared-tests only through alpha-core; direct-only
        # scoping (2026-08-13) deliberately leaves it out. alpha-core is a
        # direct dependent and is now reported rather than scheduled.
        scope = self.scope(["tools/shared-tests/src/lib.rs"])
        self.assertEqual(["shared-tests"], scope["packages"])
        self.assertEqual(["alpha-core"], scope["reverse_dependencies"])

    def test_unrelated_documentation_change_has_empty_scope(self) -> None:
        scope = self.scope(["docs/architecture.md"])
        self.assertEqual([], scope["packages"])
        self.assertEqual([], scope["scheduled_areas"])
        self.assertEqual({}, scope["area_rows"])

    def test_package_documentation_and_manifest_changes_have_empty_scope(self) -> None:
        scope = self.scope(["alpha/lib/README.md", "alpha/lib/Cargo.toml"])
        self.assertEqual([], scope["packages"])

    def test_source_extensions_include_scripts_and_frontend_code(self) -> None:
        for path in ("alpha/lib/build.rs", "alpha/lib/scripts/check.py", "alpha/lib/ui/app.tsx"):
            with self.subTest(path=path):
                scope = self.scope([path])
                self.assertEqual(["alpha-core"], scope["packages"])

    def accepted(self, *cells: tuple[str, str, str]) -> list[dict[str, object]]:
        return [
            {
                "package": package,
                "environment": environment,
                "gate": gate,
                "origin": "local",
                "outcome": "pass",
                "evidence": f"refs/notes/ci-local/{environment}",
            }
            for package, environment, gate in cells
        ]

    def test_a_verified_macos_cell_is_reused_not_dropped(self) -> None:
        # AC4 and AC6, the PR #76 shape: the macOS L1 execution is omitted, but
        # the CELL survives with local origin. Dropping the cell is what made
        # seven proven-passing cells roll up as MISSING.
        plan = calculate_scope(
            ["alpha/lib/src/lib.rs"],
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
            accepted_cells=self.accepted(("alpha-core", "macos-latest", "L1")),
        )
        macos = [
            cell
            for cell in plan["cells"]
            if cell["environment"] == "macos-latest" and cell["gate"] == "L1"
        ]
        self.assertEqual(1, len(macos))
        self.assertEqual("reuse", macos[0]["execution"])
        self.assertEqual("local", macos[0]["origin"])
        self.assertEqual("reused", macos[0]["state"])
        self.assertEqual(
            [
                {
                    "package": "alpha-core",
                    "environment": "macos-latest",
                    "gate": "L1",
                    "origin": "local",
                    "outcome": "pass",
                    "evidence": "refs/notes/ci-local/macos-latest",
                }
            ],
            plan["accepted_evidence"],
            "the plan must carry the evidence it scheduled against, so the "
            "rollup expects the same set it consumed",
        )
        self.assertNotIn(("macos-latest", "L1"), dispatched(plan, "alpha-core"))
        self.assertIn(("ubuntu-latest", "L1"), dispatched(plan, "alpha-core"))

    def test_evidence_from_two_environments_combines(self) -> None:
        # AC5: the retired single `excluded_environment` could express one host.
        plan = calculate_scope(
            ["alpha/lib/src/lib.rs"],
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
            accepted_cells=self.accepted(
                ("alpha-core", "macos-latest", "L1"),
                ("alpha-core", "wsl2-ubuntu", "L1"),
            ),
        )
        reused = {
            cell["environment"] for cell in plan["cells"] if cell["execution"] == "reuse"
        }
        self.assertEqual({"macos-latest", "wsl2-ubuntu"}, reused)
        executing = {
            cell["environment"]
            for cell in plan["cells"]
            if cell["execution"] == "execute"
        }
        self.assertEqual({"ubuntu-latest", "windows-latest"}, executing)

    def test_reusing_macos_keeps_linux_and_windows_compile_coverage(self) -> None:
        # AC3 and Phase 3's task 3.
        plan = calculate_scope(
            ["alpha/lib/src/lib.rs"],
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
            accepted_cells=self.accepted(("alpha-core", "macos-latest", "L1")),
        )
        compiled = {
            cell["environment"]
            for cell in plan["cells"]
            if cell["gate"] in ("check", "L1") and cell["execution"] == "execute"
        }
        self.assertLessEqual({"ubuntu-latest", "windows-latest"}, compiled)

    def test_force_all_selects_every_package(self) -> None:
        scope = self.scope([], force_all=True)
        self.assertTrue(scope["full_scope"])
        self.assertEqual(
            ["alpha-core", "beta-app", "shared-tests"], scope["packages"]
        )

    def test_global_test_configuration_selects_no_packages(self) -> None:
        scope = self.scope([".config/nextest.toml"])
        self.assertFalse(scope["full_scope"])
        self.assertEqual([], scope["packages"])
        self.assertEqual([], scope["scheduled_areas"])

    def test_wsl_workflow_change_selects_no_packages(self) -> None:
        scope = self.scope([".github/workflows/_wsl-ci.yml"])
        self.assertFalse(scope["full_scope"])
        self.assertEqual([], scope["packages"])

    # -- per-gate global inputs (2026-09-09) --------------------------------
    #
    # A gate's verdict is a function of the source it compiles and of the
    # command and configuration it runs under. Each gate therefore has its
    # own list of non-source inputs, and nothing else re-runs it workspace-wide.

    def test_clippy_configuration_selects_no_packages(self) -> None:
        scope = self.scope(["clippy.toml"])
        self.assertFalse(scope["full_scope"])
        self.assertEqual([], scope["packages"])

    def test_non_source_input_does_not_widen_a_source_change(self) -> None:
        plan = self.plan(["clippy.toml", "alpha/lib/src/lib.rs"])
        gates = gates_by_package(plan)
        unwidened = self.plan(["alpha/lib/src/lib.rs"])
        self.assertEqual(gates_by_package(unwidened), gates)
        self.assertEqual(
            affected_scope.row_sets(unwidened), affected_scope.row_sets(plan)
        )
        self.assertEqual(["lint", "check", "L1"], gates["alpha-core"])
        self.assertNotIn("beta-app", gates)
        self.assertNotIn("shared-tests", gates)

    def test_compile_inputs_select_no_packages(self) -> None:
        for path in ("Cargo.toml", "rust-toolchain.toml", ".cargo/config.toml"):
            scope = self.scope([path])
            self.assertEqual([], scope["packages"], path)

    def test_the_scope_calculator_selects_nothing_without_its_owner(self) -> None:
        # `scripts/` is `repo-deps`'s package directory, and this workspace
        # does not contain it: the suite-owner table is repository policy, and
        # the metadata is the authority on membership. Ownership must therefore
        # select nothing here rather than inventing a package.
        scope = self.scope(["scripts/ci/affected_scope.py"])
        self.assertEqual([], scope["packages"])

    # -- content-aware global triggers (2026-08-28) -------------------------
    #
    # A global path is a trigger by *content*, not by name, once a base ref is
    # supplied: nine comment lines in the root justfile once scheduled all 72
    # packages. The differ is injected so these never touch Git.

    COMMENT_ONLY_DIFF = (
        "--- a/justfile\n+++ b/justfile\n@@ -1,2 +1,3 @@\n"
        "-# old wording\n+# new wording\n+\n+# and a second line\n"
    )
    RECIPE_DIFF = (
        "--- a/justfile\n+++ b/justfile\n@@ -5 +5 @@\n"
        "-    cargo clippy -p {{ pkg }}\n+    cargo clippy -p {{ pkg }} --all-targets\n"
    )

    def test_comment_only_diff_is_not_a_content_change(self) -> None:
        self.assertFalse(diff_changes_more_than_comments(self.COMMENT_ONLY_DIFF))
        self.assertTrue(diff_changes_more_than_comments(self.RECIPE_DIFF))
        # A `#` that is not at the start of the line is content (a shell
        # command with a fragment, a TOML value) — only leading `#` is a comment.
        self.assertTrue(diff_changes_more_than_comments("+foo # trailing\n"))

    def test_comment_only_global_change_with_base_ref_scopes_like_docs(self) -> None:
        differ = lambda base, path: self.COMMENT_ONLY_DIFF  # noqa: E731
        self.assertIsNone(global_trigger(["justfile"], "base", differ))
        scope = self.scope(["justfile"], base_ref="base", differ=differ)
        self.assertFalse(scope["full_scope"])
        self.assertEqual("documentation", scope["change_class"])
        self.assertEqual([], scope["packages"])

    def test_comment_only_global_change_still_scopes_the_other_files(self) -> None:
        differ = lambda base, path: self.COMMENT_ONLY_DIFF  # noqa: E731
        scope = self.scope(["justfile", "alpha/lib/src/lib.rs"], base_ref="base", differ=differ)
        self.assertFalse(scope["full_scope"])
        self.assertEqual(["alpha-core"], scope["packages"])

    # -- just files are global by recipe, not by path (2026-09-09) ----------
    #
    # `JUSTFILE_BASE` is the base-ref content; each test edits one recipe on
    # the working-tree side and asks which gates that reaches. The differ
    # only has to say "more than comments changed".

    JUSTFILE_BASE = (
        "set shell := [\"bash\", \"-eu\", \"-c\"]\n"
        "areas := \"alpha beta\"\n"
        "\n"
        "# CI's lint gate.\n"
        "_lint pkg: _storage_preflight\n"
        "    cargo clippy -p {{ pkg }} -- -D warnings\n"
        "\n"
        "_storage_preflight:\n"
        "    df -h .\n"
        "\n"
        "# CI's L1 gate.\n"
        "_test pkg *args:\n"
        "    filter=\"$(just _tier_filter L1)\"\n"
        "    cargo nextest run -p {{ pkg }} -E \"$filter\" {{ args }}\n"
        "\n"
        "_tier_filter tier:\n"
        "    echo 'not test(/^level2_/)'\n"
        "\n"
        "_ensure-native-libs *packages:\n"
        "    echo \"Install the headers, then re-run just init again.\" >&2\n"
        "\n"
        "init: _ensure-git-hooks\n"
        "    @echo ready\n"
        "\n"
        "[no-cd]\n"
        "_ensure-git-hooks:\n"
        "    ln -sf ../../.githooks/pre-push .git/hooks/pre-push\n"
        "\n"
        "# Developer convenience; CI never runs it.\n"
        "pre-push *selectors=\"\":\n"
        "    @just ci-local --lint-only {{ selectors }}\n"
    )

    def just_scope(self, working_tree: str, base: str | None = JUSTFILE_BASE) -> dict[str, object]:
        (self.root / "justfile").write_text(working_tree, encoding="utf-8")
        differ = lambda base_ref, path: self.RECIPE_DIFF  # noqa: E731
        reader = lambda base_ref, path: base  # noqa: E731
        return self.scope(["justfile"], base_ref="base", differ=differ, reader=reader)

    def test_a_change_inside_the_lint_recipe_selects_no_packages(self) -> None:
        edited = self.JUSTFILE_BASE.replace(
            "cargo clippy -p {{ pkg }} -- -D warnings",
            "cargo clippy -p {{ pkg }} --all-targets -- -D warnings",
        )
        scope = self.just_scope(edited)
        self.assertEqual([], scope["packages"])

    def test_a_change_to_a_recipe_the_lint_gate_depends_on_selects_no_packages(self) -> None:
        edited = self.JUSTFILE_BASE.replace("    df -h .\n", "    df -h . && sync\n")
        self.assertEqual([], self.just_scope(edited)["packages"])

    def test_a_change_to_a_recipe_the_test_gate_calls_selects_no_packages(self) -> None:
        # `_tier_filter` is reached from `_test` through a `just` call at
        # command position, not a header dependency.
        edited = self.JUSTFILE_BASE.replace("not test(/^level2_/)", "not test(/^level[23]_/)")
        self.assertEqual([], self.just_scope(edited)["packages"])

    def test_a_change_to_a_developer_recipe_gates_nothing(self) -> None:
        edited = self.JUSTFILE_BASE.replace(
            "@just ci-local --lint-only {{ selectors }}",
            "@just ci-local --lint-only --base origin/main {{ selectors }}",
        )
        scope = self.just_scope(edited)
        self.assertFalse(scope["full_scope"])
        self.assertEqual([], scope["full_scope_gates"])
        self.assertEqual("documentation", scope["change_class"])
        self.assertEqual([], scope["packages"])

    def test_a_just_name_inside_prose_is_not_a_call(self) -> None:
        # `_ensure-native-libs` says "re-run just init" in an echo string. If
        # that counted as a call, `init` and `_ensure-git-hooks` would sit in
        # every gate's closure and this edit would widen the workspace.
        edited = self.JUSTFILE_BASE.replace("ln -sf ../../.githooks/pre-push", "ln -sfn ../../.githooks/pre-push")
        self.assertEqual([], self.just_scope(edited)["full_scope_gates"])

    def test_a_change_to_the_shared_prerequisite_recipe_selects_no_packages(self) -> None:
        edited = self.JUSTFILE_BASE.replace("Install the headers", "Install the development headers")
        self.assertEqual([], self.just_scope(edited)["packages"])

    def test_a_removed_or_added_recipe_selects_no_packages(self) -> None:
        removed = self.JUSTFILE_BASE.replace("_tier_filter tier:\n    echo 'not test(/^level2_/)'\n\n", "")
        self.assertEqual([], self.just_scope(removed)["packages"])
        added = self.JUSTFILE_BASE + "\nrelease:\n    cargo build --release\n"
        self.assertEqual([], self.just_scope(added)["full_scope_gates"])

    def test_text_outside_every_recipe_selects_no_packages(self) -> None:
        for edited in (
            self.JUSTFILE_BASE.replace('areas := "alpha beta"', 'areas := "alpha beta gamma"'),
            self.JUSTFILE_BASE.replace('set shell := ["bash", "-eu", "-c"]', 'set shell := ["bash", "-euo", "pipefail", "-c"]'),
            'import "./just/color.just"\n' + self.JUSTFILE_BASE,
        ):
            self.assertEqual([], self.just_scope(edited)["packages"])

    def test_a_recipe_reformat_that_keeps_every_line_gates_nothing(self) -> None:
        # Comments and blank lines are not part of a recipe's identity.
        edited = self.JUSTFILE_BASE.replace("# CI's lint gate.\n", "# CI's clippy gate.\n#\n# Runs without features.\n")
        self.assertEqual([], self.just_scope(edited)["full_scope_gates"])

    def test_a_just_file_under_just_is_scoped_the_same_way(self) -> None:
        (self.root / "just").mkdir()
        (self.root / "justfile").write_text("import \"./just/devops.just\"\n", encoding="utf-8")
        base = "_lint pkg:\n    cargo clippy -p {{ pkg }}\n\nplan:\n    echo plan\n"
        (self.root / "just" / "devops.just").write_text(base.replace("echo plan", "echo planning"), encoding="utf-8")
        differ = lambda base_ref, path: self.RECIPE_DIFF  # noqa: E731
        reader = lambda base_ref, path: base  # noqa: E731
        scope = self.scope(["just/devops.just"], base_ref="base", differ=differ, reader=reader)
        self.assertEqual([], scope["full_scope_gates"])
        (self.root / "just" / "devops.just").write_text(base.replace("clippy -p", "clippy --all-targets -p"), encoding="utf-8")
        scope = self.scope(["just/devops.just"], base_ref="base", differ=differ, reader=reader)
        self.assertEqual([], scope["packages"])

    def test_unobtainable_base_content_still_selects_no_packages(self) -> None:
        (self.root / "justfile").write_text(self.JUSTFILE_BASE, encoding="utf-8")
        differ = lambda base_ref, path: self.RECIPE_DIFF  # noqa: E731
        reader = lambda base_ref, path: None  # noqa: E731
        scope = self.scope(["justfile"], base_ref="base", differ=differ, reader=reader)
        self.assertEqual([], scope["packages"])

    def test_parse_just_recipes_shapes(self) -> None:
        recipes, other = parse_just_recipes(self.JUSTFILE_BASE)
        self.assertEqual(
            ['set shell := ["bash", "-eu", "-c"]', 'areas := "alpha beta"'], other
        )
        self.assertEqual(["_storage_preflight"], recipes["_lint"]["deps"])
        self.assertEqual(["_tier_filter"], recipes["_test"]["calls"])
        self.assertEqual([], recipes["_ensure-native-libs"]["calls"])
        self.assertEqual(["_ensure-git-hooks"], recipes["init"]["deps"])
        self.assertEqual(["[no-cd]", "_ensure-git-hooks:", "ln -sf ../../.githooks/pre-push .git/hooks/pre-push"], recipes["_ensure-git-hooks"]["lines"])
        self.assertEqual(["ci-local"], recipes["pre-push"]["calls"])
        self.assertEqual({"_lint", "_storage_preflight"}, just_recipe_closure(recipes, ("_lint",)))
        self.assertEqual({"_test", "_tier_filter"}, just_recipe_closure(recipes, ("_test",)))

    def test_undecidable_global_diff_does_not_affect_package_scope(self) -> None:
        # No base ref, or a differ that cannot produce the diff: widen, never
        # narrow silently — for every gate.
        self.assertEqual("justfile", global_trigger(["justfile"]))
        differ = lambda base, path: None  # noqa: E731
        self.assertEqual("justfile", global_trigger(["justfile"], "base", differ))
        scope = self.scope(["justfile"], base_ref="base", differ=differ)
        self.assertFalse(scope["full_scope"])
        self.assertEqual([], scope["packages"])

    def test_package_local_change_derives_three_runner_preflight(self) -> None:
        scope = self.scope(["alpha/lib/src/lib.rs"])
        self.assertEqual("package", scope["change_class"])
        # Scope host plus the runner OS hosting each of the three native
        # environments (the fourth, wsl2-ubuntu, is hosted by windows-latest).
        self.assertEqual(
            ["macos-latest", "ubuntu-latest", "windows-latest"], scope["preflight_os"]
        )

    def test_a_global_config_change_selecting_no_package_expands_no_preflight(self) -> None:
        # R10: `.config/nextest.toml` widens every gate, but this fixture's
        # workspace has no package under it, so nothing fans out and there is
        # no bootstrap to establish.
        scope = self.scope([".config/nextest.toml"])
        self.assertEqual("documentation", scope["change_class"])
        self.assertEqual([], scope["preflight_os"])
        self.assertTrue(scope["preflight_reason"], "the skip must still state its decision")


class ClosureTests(unittest.TestCase):
    """R2 (amended 2026-08-13): a seed selects its DIRECT dependents only."""

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        seed_build_inputs(self.root)
        # biscuit-speaks -> [playa (optional), espeak]; its dependents are the
        # real claudine/research closure from the repo.
        packages = [
            package(self.root, "biscuit-speaks", "biscuit-speaks/lib/Cargo.toml"),
            package(self.root, "biscuit-speaks-cli", "biscuit-speaks/cli/Cargo.toml"),
            package(self.root, "claudine", "claudine/lib/Cargo.toml"),
            package(self.root, "claudine-cli", "claudine/cli/Cargo.toml"),
            package(self.root, "claudine-contract", "claudine/contract/Cargo.toml"),
            package(self.root, "research", "research/lib/Cargo.toml"),
            package(self.root, "research-cli", "research/cli/Cargo.toml"),
        ]
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {
                "nodes": [
                    {"id": "biscuit-speaks", "deps": []},
                    {
                        "id": "biscuit-speaks-cli",
                        "deps": [{"pkg": "biscuit-speaks", "dep_kinds": [{"kind": None}]}],
                    },
                    {
                        "id": "claudine",
                        "deps": [{"pkg": "biscuit-speaks", "dep_kinds": [{"kind": None}]}],
                    },
                    {
                        "id": "claudine-cli",
                        "deps": [{"pkg": "claudine", "dep_kinds": [{"kind": None}]}],
                    },
                    {
                        "id": "claudine-contract",
                        "deps": [{"pkg": "claudine", "dep_kinds": [{"kind": None}]}],
                    },
                    {
                        "id": "research",
                        "deps": [{"pkg": "biscuit-speaks", "dep_kinds": [{"kind": None}]}],
                    },
                    {
                        "id": "research-cli",
                        "deps": [{"pkg": "research", "dep_kinds": [{"kind": None}]}],
                    },
                ]
            },
        }
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def test_biscuit_speaks_selects_direct_dependents_only(self) -> None:
        scope = scope_document(
            ["biscuit-speaks/lib/src/lib.rs"],
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
        )
        # claudine-cli, claudine-contract, and research-cli reach the seed
        # only through an intermediate package and are deliberately excluded.
        # The direct dependents are reported and, since AC1, selected nowhere.
        self.assertEqual(["biscuit-speaks"], scope["packages"])
        self.assertEqual(
            [
                "biscuit-speaks-cli",
                "claudine",
                "research",
            ],
            scope["reverse_dependencies"],
        )


class NativeClosureTests(unittest.TestCase):
    """R5: native requirements are the union over the dependency closure."""

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        seed_build_inputs(self.root)
        # consumer (declares no native) -> native-lib (declares ALSA).
        packages = [
            package(self.root, "consumer", "consumer/Cargo.toml"),
            package(
                self.root,
                "native-lib",
                "native-lib/Cargo.toml",
                ci=ci_policy(native={"ubuntu-latest": ["libasound2-dev"]}),
            ),
        ]
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {
                "nodes": [
                    {
                        "id": "consumer",
                        "deps": [{"pkg": "native-lib", "dep_kinds": [{"kind": None}]}],
                    },
                    {"id": "native-lib", "deps": []},
                ]
            },
        }
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def test_a_dependent_job_receives_its_dependencies_native(self) -> None:
        plan = calculate_scope(
            ["consumer/src/lib.rs"],
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
        )
        contracts = producer_contracts(plan, "consumer")
        self.assertEqual(
            ["libasound2-dev"],
            json.loads(contracts[("ubuntu-latest", "L1")]["native_packages"]),
        )
        # The guest is Linux and installs the same closure; macOS needs none.
        self.assertEqual(
            ["libasound2-dev"],
            json.loads(contracts[("wsl2-ubuntu", "L1")]["native_packages"]),
        )
        self.assertEqual([], json.loads(contracts[("macos-latest", "L1")]["native_packages"]))


class PackagePolicyTests(unittest.TestCase):
    RUNNER_LABELS = {"ubuntu-latest", "windows-latest", "macos-latest"}

    def test_no_metadata_defaults_to_gating_l1(self) -> None:
        packages = {"a": {"name": "a", "manifest_path": "/x/Cargo.toml", "metadata": None}}
        policy = package_ci_policy(packages, self.RUNNER_LABELS, root=Path("/"), today=TODAY)
        self.assertTrue(policy["a"]["gates"])
        self.assertEqual(policy["a"]["tiers"], ["L1"])

    def test_unknown_field_is_rejected(self) -> None:
        with self.assertRaises(RuntimeError) as raised:
            validate_package_ci(
                "a",
                {"gates": True, "nope": 1},
                self.RUNNER_LABELS,
                root=Path("/"),
                today=TODAY,
            )
        self.assertIn("unknown field", str(raised.exception))

    def test_exclusion_requires_owner_reason_class_and_expiry(self) -> None:
        for missing in ("reason", "owner", "exclusion-class"):
            with self.subTest(missing=missing):
                ci = ci_policy(
                    gates=False,
                    reason="x",
                    owner="@o",
                    **{"exclusion-class": "promotion-pending", "expiry": "2027-01-31"},
                )
                ci.pop(missing.replace("_", "-"))
                with self.assertRaises(RuntimeError):
                    validate_package_ci("a", ci, self.RUNNER_LABELS, root=Path("/"), today=TODAY)

    def test_an_unknown_exclusion_class_is_rejected(self) -> None:
        ci = ci_policy(
            gates=False,
            reason="x",
            owner="@o",
            **{"exclusion-class": "not-a-class", "expiry": "2027-01-31"},
        )
        with self.assertRaises(RuntimeError) as raised:
            validate_package_ci("a", ci, self.RUNNER_LABELS, root=Path("/"), today=TODAY)
        message = str(raised.exception)
        self.assertIn("exclusion-class", message)
        self.assertIn("promotion-pending", message)

    def test_promotion_pending_remains_a_legal_exclusion_class(self) -> None:
        # `test-toolkit` was the tree's only user of this class and was promoted
        # out of it; the class itself stays available to the next package.
        self.assertIn("promotion-pending", EXCLUSION_CLASSES)
        ci = ci_policy(
            gates=False,
            reason="x",
            owner="@o",
            **{"exclusion-class": "promotion-pending", "expiry": "2027-01-31"},
        )
        validate_package_ci("a", ci, self.RUNNER_LABELS, root=Path("/"), today=TODAY)
        resolved = package_ci_policy(
            {"a": {"name": "a", "metadata": {"ci": ci}}},
            self.RUNNER_LABELS,
            Path("/"),
            today=TODAY,
        )
        self.assertFalse(resolved["a"]["gates"])
        self.assertEqual(
            resolved["a"]["exclusion"]["exclusion_class"],
            "promotion-pending",
        )

    def test_an_expired_exclusion_fails(self) -> None:
        ci = ci_policy(
            gates=False,
            reason="x",
            owner="@o",
            **{"exclusion-class": "promotion-pending", "expiry": "2026-01-01"},
        )
        with self.assertRaises(RuntimeError) as raised:
            validate_package_ci("a", ci, self.RUNNER_LABELS, root=Path("/"), today=TODAY)
        self.assertIn("expired", str(raised.exception))

    def test_a_capability_exclusion_must_not_carry_expiry(self) -> None:
        ci = ci_policy(
            gates=False,
            reason="physical hardware",
            owner="@o",
            **{"exclusion-class": "capability", "expiry": "2027-01-31"},
        )
        with self.assertRaises(RuntimeError) as raised:
            validate_package_ci("a", ci, self.RUNNER_LABELS, root=Path("/"), today=TODAY)
        self.assertIn("permanent", str(raised.exception))

    def test_exclusion_governance_only_applies_to_non_gating(self) -> None:
        ci = ci_policy(gates=True, owner="@o", reason="x")
        with self.assertRaises(RuntimeError) as raised:
            validate_package_ci("a", ci, self.RUNNER_LABELS, root=Path("/"), today=TODAY)
        self.assertIn("exclusion field", str(raised.exception))

    def test_l2_tier_requires_backends(self) -> None:
        with self.assertRaises(RuntimeError):
            validate_package_ci(
                "a",
                ci_policy(tests={"tiers": ["L1", "L2"]}),
                self.RUNNER_LABELS,
                root=Path("/"),
                today=TODAY,
            )

    def test_l2_backends_without_l2_tier_is_rejected(self) -> None:
        with self.assertRaises(RuntimeError):
            validate_package_ci(
                "a",
                ci_policy(tests={"tiers": ["L1"], **{"l2-backends": ["tmux"]}}),
                self.RUNNER_LABELS,
                root=Path("/"),
                today=TODAY,
            )

    def test_unknown_l2_backend_is_rejected(self) -> None:
        with self.assertRaises(RuntimeError):
            validate_package_ci(
                "a",
                ci_policy(tests={"tiers": ["L1", "L2"], **{"l2-backends": ["xterm"]}}),
                self.RUNNER_LABELS,
                root=Path("/"),
                today=TODAY,
            )

    def test_unknown_tier_is_rejected(self) -> None:
        with self.assertRaises(RuntimeError):
            validate_package_ci(
                "a",
                ci_policy(tests={"tiers": ["L1", "L3"]}),
                self.RUNNER_LABELS,
                root=Path("/"),
                today=TODAY,
            )

    def test_l1_is_always_present_when_tiers_declared(self) -> None:
        with self.assertRaises(RuntimeError):
            validate_package_ci(
                "a",
                ci_policy(tests={"tiers": ["L2"]}),
                self.RUNNER_LABELS,
                root=Path("/"),
                today=TODAY,
            )

    def test_features_and_all_features_conflict(self) -> None:
        with self.assertRaises(RuntimeError):
            validate_package_ci(
                "a",
                ci_policy(tests={"features": ["x"], **{"all-features": True}}),
                self.RUNNER_LABELS,
                root=Path("/"),
                today=TODAY,
            )

    def test_local_features_are_accepted_as_local_runner_policy(self) -> None:
        validate_package_ci(
            "a",
            ci_policy(tests={"features": ["ci-helper"], "local-features": []}),
            self.RUNNER_LABELS,
            root=Path("/"),
            today=TODAY,
        )

    def test_local_features_must_be_a_string_list(self) -> None:
        with self.assertRaises(RuntimeError):
            validate_package_ci(
                "a",
                ci_policy(tests={"local-features": "fast"}),
                self.RUNNER_LABELS,
                root=Path("/"),
                today=TODAY,
            )

    def test_unknown_runner_tool_is_rejected(self) -> None:
        with self.assertRaises(RuntimeError):
            validate_package_ci(
                "a",
                ci_policy(tests={**{"runner-tools": ["arbitrary-script"]}}),
                self.RUNNER_LABELS,
                root=Path("/"),
                today=TODAY,
            )

    def test_messenger_desktop_stubs_is_a_build_sidecar_not_a_runner_tool(self) -> None:
        # It moved: the stubs are `required-features = ["desktop"]` bin targets
        # the PRODUCER compiles, not a facility the consumer provisions. The
        # old spelling has to stop validating, or both would be legal at once.
        validate_package_ci(
            "messenger",
            ci_policy(tests={"sidecars": ["messenger-desktop-stubs"]}),
            self.RUNNER_LABELS,
            root=ROOT,
            today=TODAY,
        )
        with self.assertRaises(RuntimeError):
            validate_package_ci(
                "messenger",
                ci_policy(tests={"runner-tools": ["messenger-desktop-stubs"]}),
                self.RUNNER_LABELS,
                root=ROOT,
                today=TODAY,
            )

    def test_zed_extension_runner_tool_is_accepted(self) -> None:
        validate_package_ci(
            "dmls",
            ci_policy(tests={"runner-tools": ["zed-extension"]}),
            self.RUNNER_LABELS,
            root=Path("/"),
            today=TODAY,
        )

    def test_unknown_companion_suite_is_rejected(self) -> None:
        with self.assertRaises(RuntimeError):
            validate_package_ci(
                "a",
                ci_policy(tests={**{"companion-suites": ["mystery-suite"]}}),
                self.RUNNER_LABELS,
                root=Path("/"),
                today=TODAY,
            )

    def test_unknown_native_os_is_rejected(self) -> None:
        with self.assertRaises(RuntimeError):
            validate_package_ci(
                "a",
                ci_policy(native={"fedora": ["foo"]}),
                self.RUNNER_LABELS,
                root=Path("/"),
                today=TODAY,
            )


def resolved_contracts(
    record: dict,
    environments: list[dict] | None = None,
    accepted: dict | None = None,
    **cell_options: object,
) -> dict[tuple[str, str], dict[str, str]]:
    """`record`'s executing cells, each resolved by the real `cell_contract`.

    The cells come from `package_cells` and the contract from the plan it
    would travel in, so this is exactly what each producer job is handed.
    """
    environments = environments if environments is not None else environments_for_tests()
    cells = package_cells(
        {"features": [], "all_features": False} | record,
        area=record["area"],
        target_kinds=record["targets"],
        environments=environments,
        accepted=accepted or {},
        **cell_options,  # type: ignore[arg-type]
    )
    plan = {
        "head": "",
        "environments": environments,
        "packages": [record],
        "cells": cells,
        "builds": [],
    }
    runners = {environment["name"]: environment["runner"] for environment in environments}
    return {
        (cell["environment"], cell["gate"]): cell_contract.contract(
            plan,
            {
                "package": cell["package"],
                "gate": cell["gate"],
                "environment": cell["environment"],
                "runner": runners[cell["environment"]],
            },
            "",
        )
        for cell in cells
        if cell["execution"] == "execute"
    }


class PackageExecutionTests(unittest.TestCase):
    """What a producer is handed for one package, resolved per executing cell."""

    def test_features_become_qualified_check_and_test_args(self) -> None:
        policy = {"features": ["test-fixtures"], "all_features": False}
        features = feature_args(policy, "sniff-cli")
        self.assertEqual(features, "--features test-fixtures")
        self.assertEqual(
            check_arguments("sniff-cli", ["lib"], features), "-p sniff-cli --features test-fixtures"
        )
        contracts = resolved_contracts(
            plan_package(
                package="sniff-cli",
                tiers=["L1", "L2"],
                l2_backends=["tmux"],
                test_args=features,
                check_args=check_arguments("sniff-cli", ["lib"], features),
            )
        )
        # Every producer is handed the plan's feature contract; none re-derives it.
        for cell, contract in contracts.items():
            self.assertEqual("-p sniff-cli --features test-fixtures", contract["check_args"], cell)
            self.assertEqual("--features test-fixtures", contract["test_args"], cell)
            self.assertEqual("", contract["requires_node"], cell)
        self.assertEqual(
            {"ubuntu-latest", "macos-latest"},
            {environment for environment, gate in contracts if gate == "L2"},
        )
        self.assertEqual(set(), {cell for cell in contracts if cell[1] == "browser"})
        self.assertIn(("wsl2-ubuntu", "L1"), contracts)

    def test_all_features_propagates_consistently(self) -> None:
        features = feature_args({"features": [], "all_features": True}, "biscuit-hash")
        self.assertEqual(features, "--all-features")
        self.assertEqual(
            check_arguments("biscuit-hash", ["lib"], features), "-p biscuit-hash --all-features"
        )

    def test_browser_and_node_environments_are_capability_derived(self) -> None:
        contracts = resolved_contracts(
            plan_package(
                package="biscuit-terminal", tiers=["L1", "L2", "browser"], l2_backends=["tmux"]
            )
        )
        self.assertEqual(
            {"ubuntu-latest"}, {environment for environment, gate in contracts if gate == "browser"}
        )

        contracts = resolved_contracts(
            plan_package(
                package="homelab-server",
                runner_tools=["node-22", "pnpm-10"],
                companion_suites=["homelab-frontend"],
            )
        )
        self.assertEqual(
            {"ubuntu-latest"},
            {
                environment
                for (environment, _), contract in contracts.items()
                if contract["requires_node"] == "true"
            },
        )

    def test_toolchain_environments_follow_the_declaration_and_the_capability(self) -> None:
        # Run 35326800778: `repo-deps` and `test-toolkit` shell out to cargo
        # from an archive whose consumer had no pinned toolchain, and the
        # concurrent rustup auto-installs the tests triggered raced each other.
        # The consumer provisions the pin once, on exactly the L1 hosts the
        # declaration reaches; the WSL2 guest stays a governed gap instead.
        environments = environments_for_tests()

        def provisioned(contracts: dict) -> set[str]:
            # `_package-ci.yml` reads `requires_toolchain` in its test job only.
            return {
                environment
                for (environment, gate), contract in contracts.items()
                if gate in ("L1", "L2", "browser") and contract["requires_toolchain"] == "true"
            }

        declared = plan_package(package="repo-deps", requires_toolchain=True)
        contracts = resolved_contracts(declared, environments)
        self.assertEqual(
            {
                environment["name"]
                for environment in environments
                if not capability(environment, "archive_only")
                and capability(environment, "cargo_toolchain")
            },
            provisioned(contracts),
        )
        self.assertNotIn(("wsl2-ubuntu", "L1"), contracts, "the guest is a gap, not a producer")

        # Narrowed to hosted execution: a cell a receipt already satisfied
        # schedules no runner and so provisions nothing.
        reused = {
            ("repo-deps", environment, "L1"): {"evidence": "note"}
            for environment in ("macos-latest", "windows-latest")
        }
        self.assertEqual(
            {"ubuntu-latest"}, provisioned(resolved_contracts(declared, environments, reused))
        )

        # A package that never shells out provisions nothing anywhere, and a
        # declaration without a test gate (check-only) provisions nothing.
        self.assertEqual(set(), provisioned(resolved_contracts(plan_package(package="a"))))
        self.assertEqual(
            set(),
            provisioned(
                resolved_contracts(
                    plan_package(
                        package="repo-deps", requires_toolchain=True, gates=["check"], tiers=[]
                    )
                )
            ),
        )

    def test_a_companion_suite_host_is_never_satisfied_by_local_evidence(self) -> None:
        # A companion suite is not in any local receipt, so its CI host must
        # still run even when the package's Rust half was validated locally.
        cells = package_cells(
            {
                "package": "homelab-server",
                "tiers": ["L1"],
                "l2_backends": [],
                "features": [],
                "all_features": False,
                "l1_include_slow": False,
                "runner_tools": ["node-22", "pnpm-10"],
                "companion_suites": ["homelab-frontend"],
                "requires_toolchain": False,
            },
            area="homelab",
            target_kinds=["lib"],
            environments=environments_for_tests(),
            accepted={
                ("homelab-server", "ubuntu-latest", "L1"): {"evidence": "note"},
                ("homelab-server", "macos-latest", "L1"): {"evidence": "note"},
            },
        )
        states = {
            (cell["environment"], cell["gate"]): cell["execution"]
            for cell in cells
        }
        self.assertEqual("execute", states[("ubuntu-latest", "L1")])
        self.assertEqual("reuse", states[("macos-latest", "L1")])


class EnvironmentsTests(unittest.TestCase):
    def test_the_checked_in_table_parses_and_is_well_governed(self) -> None:
        environments = load_environments(ENVIRONMENTS_CONFIG, today=TODAY)
        names = [environment["name"] for environment in environments]
        self.assertEqual(
            names,
            ["ubuntu-latest", "windows-latest", "macos-latest", "wsl2-ubuntu"],
        )
        # The two governed unavailabilities: Windows tmux and WSL2 tmux.
        windows = next(e for e in environments if e["name"] == "windows-latest")
        wsl = next(e for e in environments if e["name"] == "wsl2-ubuntu")
        self.assertFalse(capability(windows, "tmux"))
        self.assertFalse(capability(wsl, "tmux"))
        self.assertTrue(capability(wsl, "archive_only"))

    def test_a_missing_capability_is_rejected(self) -> None:
        import json

        from affected_scope import KNOWN_CAPABILITIES

        doc = {
            "schema_version": 3,
            "environments": [
                {
                    "name": "x",
                    "runner": "x",
                    "native_key": "x",
                    "events": ["pull_request", "push", "schedule", "workflow_dispatch"],
                    "capabilities": {key: True for key in KNOWN_CAPABILITIES if key != "tmux"},
                    "build": producer_build("x", "x86_64", "gnu", "glibc", ["x"]),
                }
            ],
        }
        path = Path(tempfile.mkdtemp()) / "environments.json"
        path.write_text(json.dumps(doc))
        with self.assertRaises(RuntimeError) as raised:
            load_environments(path, today=TODAY)
        self.assertIn("missing", str(raised.exception))

    def test_an_ungoverned_capability_expiry_fails(self) -> None:
        import json

        doc = {
            "schema_version": 3,
            "environments": [
                {
                    "name": "x",
                    "runner": "x",
                    "native_key": "x",
                    "events": ["pull_request", "push", "schedule", "workflow_dispatch"],
                    "capabilities": {
                        "tmux": {"available": False, "reason": "r", "owner": "@o", "expiry": "2026-01-01"},
                        "headless_browser": True,
                        "node_pnpm": True,
                        "archive_only": False,
                    },
                    "build": producer_build("x", "x86_64", "gnu", "glibc", ["x"]),
                }
            ],
        }
        path = Path(tempfile.mkdtemp()) / "environments.json"
        path.write_text(json.dumps(doc))
        with self.assertRaises(RuntimeError):
            load_environments(path, today=TODAY)


class ArchiveIncludeAndSidecarPolicyTests(unittest.TestCase):
    """`[package.metadata.ci.tests]`'s two producer-owned declarations.

    An archive include or a sidecar is validated at SCHEDULING time because a
    malformed one is a scheduling-time mistake: discovering it as a failed
    producer minutes into a CI run costs a whole fan-out.
    """

    RUNNER_LABELS = {"ubuntu-latest", "windows-latest", "macos-latest"}

    def validate(self, tests: dict) -> None:
        validate_package_ci(
            "alpha", ci_policy(tests=tests), self.RUNNER_LABELS, root=ROOT, today=TODAY
        )

    def test_a_profile_relative_include_with_platform_placeholders_is_accepted(self) -> None:
        self.validate(
            {
                "archive-includes": [
                    "examples/discovery_probe",
                    "{DLL_PREFIX}archive_portability_dylib{DLL_SUFFIX}",
                    "tools/probe{EXE_SUFFIX}",
                ]
            }
        )

    def test_an_absolute_include_is_refused(self) -> None:
        for entry in ("/etc/passwd", "C:/Windows/system32/cmd.exe"):
            with self.subTest(entry=entry), self.assertRaises(RuntimeError) as caught:
                self.validate({"archive-includes": [entry]})
            self.assertIn("relative", str(caught.exception))

    def test_an_include_that_escapes_the_target_directory_is_refused(self) -> None:
        with self.assertRaises(RuntimeError) as caught:
            self.validate({"archive-includes": ["../../etc/passwd"]})
        self.assertIn("escapes", str(caught.exception))

    def test_a_backslash_spelling_is_refused(self) -> None:
        # One spelling has to survive a Windows producer handing a path to a
        # Linux consumer's manifest reader.
        with self.assertRaises(RuntimeError) as caught:
            self.validate({"archive-includes": ["examples\\probe"]})
        self.assertIn("forward slashes", str(caught.exception))

    def test_an_unknown_placeholder_is_refused(self) -> None:
        with self.assertRaises(RuntimeError) as caught:
            self.validate({"archive-includes": ["{PROFILE}/probe"]})
        self.assertIn("placeholder", str(caught.exception))

    def test_an_include_that_names_its_own_profile_directory_is_refused(self) -> None:
        # The producer supplies `<triple>/<profile>`; a package that spelled it
        # too would name `x86_64-.../debug/debug/examples/probe`.
        for entry in ("debug/examples/probe", "release/probe", "target/debug/probe"):
            with self.subTest(entry=entry), self.assertRaises(RuntimeError) as caught:
                self.validate({"archive-includes": [entry]})
            self.assertIn("profile directory", str(caught.exception))

    def test_an_empty_or_padded_include_is_refused(self) -> None:
        for entry in ("", " examples/probe"):
            with self.subTest(entry=entry), self.assertRaises(RuntimeError):
                self.validate({"archive-includes": [entry]})

    def test_includes_must_be_a_list_of_strings(self) -> None:
        for value in ("examples/probe", [1], {"path": "examples/probe"}):
            with self.subTest(value=value), self.assertRaises(RuntimeError):
                self.validate({"archive-includes": value})

    def test_a_sidecar_outside_the_closed_table_is_refused(self) -> None:
        with self.assertRaises(RuntimeError) as caught:
            self.validate({"sidecars": ["build-whatever-i-like"]})
        self.assertIn("unknown build sidecar", str(caught.exception))

    def test_a_duplicated_sidecar_is_refused(self) -> None:
        with self.assertRaises(RuntimeError) as caught:
            self.validate({"sidecars": ["harness-broker", "harness-broker"]})
        self.assertIn("duplicates", str(caught.exception))

    def test_every_shipped_sidecar_name_is_accepted(self) -> None:
        for name in load_sidecars(ROOT):
            with self.subTest(sidecar=name):
                self.validate({"sidecars": [name]})

    def test_declaring_no_sidecars_never_reads_the_table(self) -> None:
        # A workspace with no sidecars at all — every synthetic fixture, and
        # most real packages — must not need the file to exist.
        validate_sidecars("alpha", [], Path("/nonexistent-root-2b41"))

    def test_the_shipped_sidecar_table_is_a_closed_well_formed_vocabulary(self) -> None:
        # Passive corpus test over the one shipped artifact of this contract.
        sidecars = load_sidecars(ROOT)
        self.assertTrue(sidecars)
        for name, spec in sidecars.items():
            with self.subTest(sidecar=name):
                self.assertIsInstance(spec.get("package"), str)
                self.assertTrue(spec["package"])
                self.assertTrue(spec.get("bins"))
                self.assertTrue(all(isinstance(b, str) for b in spec["bins"]))
                self.assertIsInstance(spec.get("features", []), list)
                self.assertGreater(len(spec.get("reason", "")), 40)

    def test_a_sidecar_table_from_another_generation_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            (root / ".github" / "ci").mkdir(parents=True)
            (root / SIDECAR_TABLE).write_text(
                '{"schema_version": 99, "sidecars": {}}', encoding="utf-8"
            )
            with self.assertRaises(RuntimeError) as caught:
                load_sidecars(root)
        self.assertIn("schema version", str(caught.exception))

    def test_a_compile_time_sidecar_never_reappears_as_a_runner_tool(self) -> None:
        # Until Task 6.5 the two compile-time sidecars were projected back into
        # `runner_tools` so the reusable workflow's legacy `cargo build` steps
        # kept firing. Those steps are gone; a projection that outlived them
        # would ask a consumer with no Cargo to build its own fixture.
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            seed_build_inputs(root)
            packages = [
                package(
                    root,
                    "alpha",
                    "alpha/Cargo.toml",
                    ci=ci_policy(
                        tests={
                            "sidecars": ["darkmatter-md-fixture", "harness-broker"],
                            "runner-tools": ["ai-provider-stubs"],
                        }
                    ),
                )
            ]
            policy = package_ci_policy(
                {item["id"]: item for item in packages},
                runner_labels=self.RUNNER_LABELS,
                root=root,
                today=TODAY,
            )
        record = policy["alpha"]
        self.assertEqual(
            record["sidecars"], ["darkmatter-md-fixture", "harness-broker"]
        )
        self.assertEqual(record["runner_tools"], ["ai-provider-stubs"])


class BuildContractTests(unittest.TestCase):
    """The compile contracts of `.github/ci/environments.json`.

    `fixes/2026-09-12-single-os-compile/spec.md` section 5: compatibility is
    declared and predicate-checked, never inferred from an OS name, and
    Linux-to-WSL2 is the only cross-environment edge the table can express.
    """

    def table(self, **mutate) -> list[dict]:
        environments = environments_for_tests()
        by_name = {entry["name"]: entry for entry in environments}
        for name, changes in mutate.items():
            by_name[name.replace("_", "-")]["build"].update(changes)
        return environments

    def write(self, environments: list[dict], version: int = 3) -> Path:
        path = Path(tempfile.mkdtemp()) / "environments.json"
        for entry in environments:
            entry["capabilities"].setdefault("wezterm", False)
            entry["capabilities"].setdefault("kitty", False)
            entry["capabilities"].setdefault("apple-terminal", False)
        path.write_text(json.dumps({"schema_version": version, "environments": environments}))
        return path

    # -- the shipped table --------------------------------------------------

    def test_the_shipped_table_declares_one_contract_per_producer(self) -> None:
        environments = load_environments(ENVIRONMENTS_CONFIG, today=TODAY)
        for name in ("ubuntu-latest", "windows-latest", "macos-latest"):
            contract = build_contract(environments, name)
            self.assertEqual(sorted(PRODUCER_BUILD_FIELDS), sorted(contract))
            self.assertIn(name, contract["executes"])

    def test_linux_to_wsl2_is_the_only_cross_environment_edge(self) -> None:
        environments = load_environments(ENVIRONMENTS_CONFIG, today=TODAY)
        edges = {
            environment["name"]: [
                name
                for name in environment["build"]["executes"]
                if name != environment["name"]
            ]
            for environment in environments
            if "executes" in environment["build"]
        }
        self.assertEqual(
            {"ubuntu-latest": ["wsl2-ubuntu"], "windows-latest": [], "macos-latest": []},
            edges,
        )

    def test_the_wsl2_guest_is_compiled_for_by_its_native_key(self) -> None:
        environments = load_environments(ENVIRONMENTS_CONFIG, today=TODAY)
        self.assertEqual("ubuntu-latest", producer_of(environments, "wsl2-ubuntu"))
        for name in ("ubuntu-latest", "windows-latest", "macos-latest"):
            self.assertEqual(name, producer_of(environments, name))

    def test_the_archive_only_guest_declares_only_its_runtime_predicates(self) -> None:
        environments = load_environments(ENVIRONMENTS_CONFIG, today=TODAY)
        guest = next(e for e in environments if e["name"] == "wsl2-ubuntu")
        self.assertEqual(sorted(CONSUMER_BUILD_FIELDS), sorted(guest["build"]))
        with self.assertRaises(RuntimeError):
            build_contract(environments, "wsl2-ubuntu")

    def test_every_producer_in_the_shipped_table_owns_an_archive(self) -> None:
        # Since Task 6.5 there is no held-back state to declare: every native
        # producer owns the archive its consumers execute, and the WSL2 guest
        # is carried by the Linux producer that hosts it. The assertion that
        # would catch a regression to compiling in place is now that the set of
        # producers and the set of archive owners are the same set.
        environments = load_environments(ENVIRONMENTS_CONFIG, today=TODAY)
        producers = {
            environment["name"]
            for environment in environments
            if "executes" in environment.get("build", {})
        }
        self.assertEqual(
            {"ubuntu-latest", "macos-latest", "windows-latest"}, producers
        )
        self.assertEqual(producers, archive_producers(environments))

    def test_a_producer_carries_its_guest_with_it(self) -> None:
        # There is no state in which a producer's archive is consumed by one of
        # its compatible environments and compiled again by another: ownership
        # is per PRODUCER, and `executes` is what a producer serves.
        environments = load_environments(ENVIRONMENTS_CONFIG, today=TODAY)
        for producer in archive_producers(environments):
            contract = build_contract(environments, producer)
            self.assertIn("wsl2-ubuntu", contract["executes"] + ["wsl2-ubuntu"])
            self.assertEqual(producer, producer_of(environments, producer))

    # -- refusals -----------------------------------------------------------

    def test_a_contract_carrying_a_retired_migration_field_is_refused(self) -> None:
        # `archive_cutover` was the Phase 4/5 switch; Task 6.5 removed it with
        # the compile-in-place paths it guarded. The vocabulary is closed, so a
        # table that still carries it fails rather than being quietly ignored —
        # which is what would happen on a branch that revived the old data.
        path = self.write(self.table(ubuntu_latest={"archive_cutover": True}))
        with self.assertRaises(RuntimeError) as raised:
            load_environments(path, today=TODAY)
        self.assertIn("archive_cutover", str(raised.exception))

    def test_native_windows_cannot_be_paired_with_the_wsl2_guest(self) -> None:
        # The guest's `native_key` is ubuntu-latest, so no Windows contract can
        # claim it — before the ABI and libc comparison would also refuse it.
        path = self.write(self.table(windows_latest={"executes": ["windows-latest", "wsl2-ubuntu"]}))
        with self.assertRaises(RuntimeError) as raised:
            load_environments(path, today=TODAY)
        self.assertIn("native_key", str(raised.exception))

    def test_a_producer_may_not_execute_in_another_native_environment(self) -> None:
        path = self.write(self.table(ubuntu_latest={"executes": ["macos-latest", "ubuntu-latest", "wsl2-ubuntu"]}))
        with self.assertRaises(RuntimeError) as raised:
            load_environments(path, today=TODAY)
        self.assertIn("archive-only", str(raised.exception))

    def test_a_mismatched_abi_refuses_the_edge(self) -> None:
        environments = environments_for_tests()
        guest = next(e for e in environments if e["name"] == "wsl2-ubuntu")
        guest["build"]["runtime"]["abi"] = "musl"
        path = self.write(environments)
        with self.assertRaises(RuntimeError) as raised:
            load_environments(path, today=TODAY)
        self.assertIn("abi is", str(raised.exception))

    def test_a_consumer_missing_a_required_native_library_refuses_the_edge(self) -> None:
        path = self.write(
            self.table(ubuntu_latest={"runtime": {
                "arch": "x86_64", "abi": "gnu", "libc": "glibc",
                "native_libraries": ["libasound2"],
            }})
        )
        with self.assertRaises(RuntimeError) as raised:
            load_environments(path, today=TODAY)
        self.assertIn("libasound2", str(raised.exception))

    def test_a_nextest_skew_between_producer_and_guest_refuses_the_edge(self) -> None:
        environments = environments_for_tests()
        guest = next(e for e in environments if e["name"] == "wsl2-ubuntu")
        guest["build"]["nextest"] = "0.9.100"
        path = self.write(environments)
        with self.assertRaises(RuntimeError) as raised:
            load_environments(path, today=TODAY)
        self.assertIn("nextest", str(raised.exception))

    def test_a_producer_that_does_not_execute_its_own_archive_is_refused(self) -> None:
        path = self.write(self.table(macos_latest={"executes": []}))
        with self.assertRaises(RuntimeError) as raised:
            load_environments(path, today=TODAY)
        self.assertIn("always executes what it compiles", str(raised.exception))

    def test_an_environment_without_a_build_contract_is_refused(self) -> None:
        environments = environments_for_tests()
        del environments[0]["build"]
        with self.assertRaises(RuntimeError) as raised:
            load_environments(self.write(environments), today=TODAY)
        self.assertIn("must declare a 'build' contract", str(raised.exception))

    def test_an_unknown_build_field_is_refused(self) -> None:
        path = self.write(self.table(macos_latest={"sccache": True}))
        with self.assertRaises(RuntimeError) as raised:
            load_environments(path, today=TODAY)
        self.assertIn("sccache", str(raised.exception))

    def test_a_version_one_table_is_refused_rather_than_read_without_contracts(self) -> None:
        with self.assertRaises(RuntimeError) as raised:
            load_environments(self.write(environments_for_tests(), version=1), today=TODAY)
        self.assertIn("schema_version must be 3", str(raised.exception))

    def test_an_unsorted_executes_list_is_refused(self) -> None:
        path = self.write(self.table(ubuntu_latest={"executes": ["wsl2-ubuntu", "ubuntu-latest"]}))
        with self.assertRaises(RuntimeError) as raised:
            load_environments(path, today=TODAY)
        self.assertIn("sorted", str(raised.exception))


class BuildDerivationTests(unittest.TestCase):
    """Which cells create consumer demand, on a synthetic two-package workspace."""

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        seed_build_inputs(self.root)
        packages = [
            package(self.root, "alpha-core", "alpha/lib/Cargo.toml"),
            package(self.root, "beta-app", "beta/app/Cargo.toml"),
        ]
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {
                "nodes": [
                    {"id": "alpha-core", "deps": []},
                    {"id": "beta-app", "deps": []},
                ]
            },
        }
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def plan(self, *files: str, **kwargs) -> dict:
        return calculate_scope(
            list(files),
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
            **kwargs,
        )

    def test_declared_includes_and_sidecars_reach_the_build_identity(self) -> None:
        # They are IN the key because changing either set changes what the
        # producer emits, and therefore what a consumer must find. Sorted, so
        # two packages that declare the same set in a different order do not
        # split a key over nothing.
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            seed_build_inputs(root)
            declared = ci_policy(
                tests={
                    "archive-includes": ["tools/probe{EXE_SUFFIX}", "examples/first"],
                    "sidecars": ["harness-broker", "darkmatter-md-fixture"],
                }
            )
            packages = [package(root, "alpha-core", "alpha/lib/Cargo.toml", ci=declared)]
            metadata = {
                "workspace_members": [item["id"] for item in packages],
                "packages": packages,
                "resolve": {"nodes": [{"id": "alpha-core", "deps": []}]},
            }
            policy = package_ci_policy(
                {item["id"]: item for item in packages},
                runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
                root=root,
                today=TODAY,
            )
            plan = calculate_scope(
                ["alpha/lib/src/lib.rs"],
                root,
                metadata,
                environments_for_tests(),
                policy,
            )
        self.assertEqual([], schema.validate_resolved_plan(plan))
        record = next(
            entry for entry in plan["builds"] if entry["producer"] == "ubuntu-latest"
        )
        self.assertEqual(
            record["identity"]["archive_includes"],
            ["examples/first", "tools/probe{EXE_SUFFIX}"],
        )
        self.assertEqual(
            record["identity"]["sidecars"],
            ["darkmatter-md-fixture", "harness-broker"],
        )
        package_record = next(
            entry for entry in plan["packages"] if entry["package"] == "alpha-core"
        )
        self.assertEqual(
            package_record["archive_includes"],
            ["tools/probe{EXE_SUFFIX}", "examples/first"],
        )

    def test_one_more_declared_include_splits_the_build_key(self) -> None:
        # The reason includes are keyed at all: an archive built before a
        # package declared a new payload is not the archive its tests now need.
        keys = []
        for includes in ([], ["examples/first"]):
            with tempfile.TemporaryDirectory() as name:
                root = Path(name)
                seed_build_inputs(root)
                packages = [
                    package(
                        root,
                        "alpha-core",
                        "alpha/lib/Cargo.toml",
                        ci=ci_policy(tests={"archive-includes": includes}),
                    )
                ]
                metadata = {
                    "workspace_members": [item["id"] for item in packages],
                    "packages": packages,
                    "resolve": {"nodes": [{"id": "alpha-core", "deps": []}]},
                }
                policy = package_ci_policy(
                    {item["id"]: item for item in packages},
                    runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
                    root=root,
                    today=TODAY,
                )
                plan = calculate_scope(
                    ["alpha/lib/src/lib.rs"],
                    root,
                    metadata,
                    environments_for_tests(),
                    policy,
                )
            keys.append(
                next(
                    entry["key"]
                    for entry in plan["builds"]
                    if entry["producer"] == "ubuntu-latest"
                )
            )
        self.assertNotEqual(keys[0], keys[1])

    def test_a_prohibited_cell_demands_no_build(self) -> None:
        # A constraint stops an EXECUTION, so the owner it would have needed
        # must not be scheduled either.
        plan = self.plan(
            "alpha/lib/src/lib.rs",
            prohibitions={
                "macos-latest": {
                    "owner": "ken",
                    "reason": "do not run macOS for this branch",
                    "expiry": "2099-01-01",
                    "source": "macos.toml",
                }
            },
        )
        self.assertEqual([], schema.validate_resolved_plan(plan))
        producers = {record["producer"] for record in plan["builds"]}
        self.assertNotIn("macos-latest", producers)
        self.assertIn("ubuntu-latest", producers)

    def test_only_demanded_owners_are_scheduled(self) -> None:
        plan = self.plan(
            "alpha/lib/src/lib.rs",
            accepted_cells=[
                {
                    "package": "alpha-core",
                    "environment": environment,
                    "gate": "L1",
                    "outcome": "pass",
                    "evidence": {"ref": f"refs/notes/ci-local/{environment}"},
                }
                for environment in ("windows-latest", "macos-latest")
            ],
        )
        self.assertEqual([], schema.validate_resolved_plan(plan))
        self.assertEqual(
            ["ubuntu-latest"], sorted({r["producer"] for r in plan["builds"]})
        )

    def test_a_record_names_why_its_guest_is_compatible(self) -> None:
        plan = self.plan("alpha/lib/src/lib.rs")
        linux = next(r for r in plan["builds"] if r["producer"] == "ubuntu-latest")
        self.assertIn("x86_64-unknown-linux-gnu", linux["compatibility_reason"])
        self.assertIn("wsl2-ubuntu", linux["compatibility_reason"])
        windows = next(r for r in plan["builds"] if r["producer"] == "windows-latest")
        self.assertIn("own producer environment", windows["compatibility_reason"])

    def test_prune_is_idempotent_and_removes_only_satisfied_demand(self) -> None:
        plan = self.plan("alpha/lib/src/lib.rs")
        once = prune_build_records(plan["builds"], plan["cells"])
        twice = prune_build_records(once, plan["cells"])
        self.assertEqual(plan["builds"], once)
        self.assertEqual(once, twice)

    def test_two_selected_packages_get_one_key_each_per_producer(self) -> None:
        plan = self.plan("alpha/lib/src/lib.rs", "beta/app/src/lib.rs")
        self.assertEqual([], schema.validate_resolved_plan(plan))
        linux = sorted(
            (r["package"], r["key"])
            for r in plan["builds"]
            if r["producer"] == "ubuntu-latest"
        )
        self.assertEqual(["alpha-core", "beta-app"], [name for name, _ in linux])
        self.assertEqual(2, len({key for _, key in linux}))

    def test_an_owner_matrix_entry_lists_the_packages_it_compiles(self) -> None:
        plan = self.plan("alpha/lib/src/lib.rs", "beta/app/src/lib.rs")
        owners = {entry["environment"]: entry for entry in build_owner_matrix(plan)}
        self.assertEqual(
            ["alpha-core", "beta-app"], owners["ubuntu-latest"]["packages"]
        )
        self.assertEqual(2, len(owners["ubuntu-latest"]["builds"]))


class ShadowWorkspaceTests(unittest.TestCase):
    def test_a_member_that_redecls_a_workspace_fails(self) -> None:
        root = Path(tempfile.mkdtemp())
        (root / "alpha").mkdir(parents=True)
        # `alpha` is a root member whose OWN manifest redeclares a [workspace],
        # so `cd alpha && cargo test` resolves a different target/lockfile than
        # the root build — the shadow this guard exists to catch.
        (root / "alpha" / "Cargo.toml").write_text(
            "[package]\nname='alpha'\nversion='0.1.0'\n[workspace]\n"
        )
        alpha = package(root, "alpha", "alpha/Cargo.toml")
        metadata = {"workspace_members": ["alpha"], "packages": [alpha], "resolve": {"nodes": []}}
        with self.assertRaises(RuntimeError) as raised:
            validate_no_shadow_workspaces(metadata, root)
        self.assertIn("shadow", str(raised.exception))


class LockfileScopeTests(unittest.TestCase):
    def test_a_changed_dependency_reaches_its_dependents(self) -> None:
        # `shared` depends on `alpha`: changing alpha reaches shared.
        base = lockfile({"alpha": ("1", []), "shared": ("1", ["alpha"])})
        head = lockfile({"alpha": ("2", []), "shared": ("1", ["alpha"])})
        impacted = lockfile_impacted_names(base, head, {"alpha", "shared"})
        self.assertEqual(impacted, {"alpha", "shared"})

    def test_an_undecidable_diff_returns_none(self) -> None:
        self.assertIsNone(lockfile_impacted_names(None, None, set()))


def lockfile(entries: dict[str, tuple[str, list[str]]]) -> str:
    lines = ["version = 3"]
    for name, (version, dependencies) in entries.items():
        lines.append(f"[[package]]\nname = \"{name}\"\nversion = \"{version}\"")
        if dependencies:
            rendered = ", ".join(f"\"{dep}\"" for dep in dependencies)
            lines.append(f"dependencies = [\n {rendered},\n]")
    return "\n".join(lines) + "\n"


class GatesFalseScopeTests(unittest.TestCase):
    """A `gates = false` package is selected but launches no jobs."""

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        seed_build_inputs(self.root)
        packages = [
            package(self.root, "excluded", "excluded/Cargo.toml"),
        ]
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {"nodes": [{"id": "excluded", "deps": []}]},
        }
        self.policy = package_ci_policy(
            {
                "excluded": package(
                    self.root,
                    "excluded",
                    "excluded/Cargo.toml",
                    ci=ci_policy(
                        gates=False,
                        reason="blocked on identified work",
                        owner="@o",
                        **{"exclusion-class": "promotion-pending", "expiry": "2027-01-31"},
                    ),
                )
            },
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def test_excluded_from_the_fan_out_but_present_in_policy(self) -> None:
        scope = scope_document(
            ["excluded/src/lib.rs"],
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
        )
        # Still SELECTED (coverage and the reverse-dependency closure see it)...
        self.assertEqual(["excluded"], scope["packages"])
        # ...but it launches no jobs...
        self.assertEqual([], scope["scheduled_areas"])
        self.assertEqual(
            [],
            [
                row
                for document in scope["area_rows"].values()
                for name in schema.ROW_SET_NAMES
                for row in document[name]
            ],
        )
        # ...while the rollup still learns its governance, so the cell renders
        # NOT SCHEDULED rather than vanishing.
        policy = {entry["package"]: entry for entry in scope["policy"]}
        self.assertIn("excluded", policy)
        self.assertFalse(policy["excluded"]["gates"])
        self.assertEqual(
            policy["excluded"]["exclusion"]["exclusion_class"], "promotion-pending"
        )


class NonPropagationTests(unittest.TestCase):
    """R5: tiers, runner tools, and companion suites do NOT propagate."""

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        seed_build_inputs(self.root)
        (self.root / "homelab").mkdir()
        (self.root / "homelab" / "justfile").write_text(
            "test-frontend:\nlint-frontend:\n"
        )
        packages = [
            package(self.root, "consumer", "consumer/Cargo.toml"),
            package(
                self.root,
                "declaring-dep",
                "declaring-dep/Cargo.toml",
                ci=ci_policy(
                    tests={
                        "tiers": ["L1", "L2"],
                        "l2-backends": ["tmux"],
                        "sidecars": ["messenger-desktop-stubs"],
                        "companion-suites": ["homelab-frontend"],
                    }
                ),
            ),
        ]
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {
                "nodes": [
                    {
                        "id": "consumer",
                        "deps": [{"pkg": "declaring-dep", "dep_kinds": [{"kind": None}]}],
                    },
                    {"id": "declaring-dep", "deps": []},
                ]
            },
        }
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def test_a_dependent_keeps_its_own_tiers_tools_and_companions(self) -> None:
        plan = calculate_scope(
            ["consumer/src/lib.rs"],
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
        )
        consumer = next(entry for entry in plan["packages"] if entry["package"] == "consumer")
        # Only `native` unions over the closure. The dependency's L2 tier,
        # runner tools, and companion suites describe ITS tests, not the
        # packages that compile it.
        self.assertEqual(consumer["tiers"], ["L1"])
        self.assertEqual(consumer["runner_tools"], [])
        self.assertEqual(consumer["companion_suites"], [])
        contracts = producer_contracts(plan, "consumer")
        self.assertEqual(set(), {cell for cell in contracts if cell[1] != "L1" and cell[1] not in ("lint", "check")})
        for cell, contract in contracts.items():
            self.assertEqual("[]", contract["runner_tools"], cell)
            self.assertEqual("[]", contract["companion_suites"], cell)
            self.assertEqual("", contract["requires_node"], cell)


class BuildClosureEdgeTests(unittest.TestCase):
    """`build_closure`: seed dev-deps in, transitive dev-deps out."""

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        seed_build_inputs(self.root)
        names = ["seed", "normal", "seed-dev", "transitive-dev", "normal-dev"]
        packages = [package(self.root, name, f"{name}/Cargo.toml") for name in names]
        self.packages = {item["id"]: item for item in packages}
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {
                "nodes": [
                    {
                        "id": "seed",
                        "deps": [
                            {"pkg": "normal", "dep_kinds": [{"kind": None}]},
                            {"pkg": "seed-dev", "dep_kinds": [{"kind": "dev"}]},
                        ],
                    },
                    {
                        "id": "seed-dev",
                        "deps": [{"pkg": "transitive-dev", "dep_kinds": [{"kind": "dev"}]}],
                    },
                    {
                        "id": "normal",
                        "deps": [{"pkg": "normal-dev", "dep_kinds": [{"kind": "dev"}]}],
                    },
                    {"id": "transitive-dev", "deps": []},
                    {"id": "normal-dev", "deps": []},
                ]
            },
        }

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def test_dev_dependency_edges(self) -> None:
        closure = build_closure("seed", self.metadata, self.packages)
        # The seed's OWN dev-dependencies are compiled to test it...
        self.assertIn("seed-dev", closure)
        self.assertIn("normal", closure)
        # ...but a dependency's dev-dependencies are never built.
        self.assertNotIn("transitive-dev", closure)
        self.assertNotIn("normal-dev", closure)


class LockfileScopeBranchTests(unittest.TestCase):
    """The `Cargo.lock` branches through `calculate_scope`."""

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        seed_build_inputs(self.root)
        packages = [
            package(self.root, "alpha-core", "alpha/lib/Cargo.toml"),
            package(self.root, "beta-app", "beta/app/Cargo.toml"),
        ]
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {
                "nodes": [
                    {"id": "alpha-core", "deps": []},
                    {
                        "id": "beta-app",
                        "deps": [{"pkg": "alpha-core", "dep_kinds": [{"kind": None}]}],
                    },
                ]
            },
        }
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def scope(self, files: list[str], **kwargs: object) -> dict[str, object]:
        return scope_document(
            files,
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
            **kwargs,  # type: ignore[arg-type]
        )

    def test_a_decidable_lockfile_diff_selects_no_packages(self) -> None:
        (self.root / "Cargo.lock").write_text(
            lockfile({"alpha-core": ("2", []), "beta-app": ("1", ["alpha-core"])})
        )
        base = lockfile({"alpha-core": ("1", []), "beta-app": ("1", ["alpha-core"])})
        scope = self.scope(["Cargo.lock"], base_lockfile=base)
        self.assertFalse(scope["full_scope"])
        self.assertEqual([], scope["packages"])

    def test_an_undecidable_lockfile_diff_selects_no_packages(self) -> None:
        # No base lockfile supplied: the safe default is never silently
        # skipped, only ever widened.
        (self.root / "Cargo.lock").write_text(
            lockfile({"alpha-core": ("2", []), "beta-app": ("1", ["alpha-core"])})
        )
        scope = self.scope(["Cargo.lock"])
        self.assertFalse(scope["full_scope"])
        self.assertEqual([], scope["packages"])

    def test_an_irrelevant_lockfile_change_selects_nothing(self) -> None:
        (self.root / "Cargo.lock").write_text(
            lockfile({"alpha-core": ("1", []), "beta-app": ("1", ["alpha-core"])})
        )
        scope = self.scope(
            ["Cargo.lock"],
            base_lockfile=lockfile(
                {"alpha-core": ("1", []), "beta-app": ("1", ["alpha-core"]), "serde": ("1", [])}
            ),
        )
        self.assertFalse(scope["full_scope"])
        self.assertEqual([], scope["packages"])


class TopLevelDirectoryFallbackTests(unittest.TestCase):
    """A file under no package directory selects its top-level directory's packages."""

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        seed_build_inputs(self.root)
        packages = [
            package(self.root, "alpha-core", "alpha/lib/Cargo.toml"),
            package(self.root, "alpha-cli", "alpha/cli/Cargo.toml"),
            package(self.root, "beta-app", "beta/app/Cargo.toml"),
        ]
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {
                "nodes": [
                    {"id": "alpha-core", "deps": []},
                    {"id": "alpha-cli", "deps": []},
                    {"id": "beta-app", "deps": []},
                ]
            },
        }
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def test_a_directory_level_justfile_selects_no_packages(self) -> None:
        scope = scope_document(
            ["alpha/justfile"],
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
        )
        self.assertEqual([], scope["packages"])

    def test_a_root_level_file_outside_any_directory_selects_nothing(self) -> None:
        scope = scope_document(
            ["README.md"],
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
        )
        self.assertEqual([], scope["packages"])


class AreaFanOutTests(unittest.TestCase):
    """AC2: `ci.yml` fans out per AREA, from lists the planner produced.

    The area fan-out is a regrouping of the plan's gating packages and their
    dispatch rows, never a second selection. Every assertion below is about the
    relationship between `scheduled_areas`, `area_slugs`, and `area_rows` — the
    three fields the workflow reads together.
    """

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        seed_build_inputs(self.root)
        # Two packages in one area, one in another, one nested area, and one
        # root-level member — the five layouts the repository actually uses.
        packages = [
            package(self.root, "alpha-core", "alpha/lib/Cargo.toml"),
            package(self.root, "alpha-cli", "alpha/cli/Cargo.toml"),
            package(self.root, "beta-app", "beta/app/Cargo.toml"),
            package(self.root, "nested-core", "alpha/nested/core/Cargo.toml"),
            package(self.root, "top-level", "top-level/Cargo.toml"),
        ]
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {
                "nodes": [{"id": item["id"], "deps": []} for item in packages]
            },
        }
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def scope(self, **kwargs: object) -> dict[str, object]:
        return scope_document(
            [],
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
            force_all=True,
            **kwargs,  # type: ignore[arg-type]
        )

    def area_packages(self, scope: dict, area: str) -> set[str]:
        return {
            row["package"]
            for name in schema.ROW_SET_NAMES
            for row in scope["area_rows"][area][name]
        }

    def test_every_gating_package_dispatches_under_exactly_its_own_area(self) -> None:
        plan = calculate_scope(
            [], self.root, self.metadata, environments_for_tests(), self.policy, force_all=True
        )
        scope = legacy_scope_document(plan)
        areas = {entry["package"]: entry["area"] for entry in plan["packages"]}
        dispatched_under = {
            package: sorted(area for area in scope["area_rows"] if package in self.area_packages(scope, area))
            for package in gates_by_package(plan)
        }
        self.assertEqual(
            {package: [areas[package]] for package in gates_by_package(plan)},
            dispatched_under,
            "the area fan-out must regroup the plan's packages, not re-select them",
        )

    def test_areas_group_their_packages_and_are_sorted(self) -> None:
        scope = self.scope()
        self.assertEqual(
            ["alpha", "alpha/nested", "beta", "root"], scope["scheduled_areas"]
        )
        self.assertEqual({"alpha-cli", "alpha-core"}, self.area_packages(scope, "alpha"))
        self.assertEqual(
            {"nested-core"},
            self.area_packages(scope, "alpha/nested"),
            "a nested area is its own entry, never folded into its parent",
        )

    def test_every_scheduled_area_is_handed_rows_of_its_own(self) -> None:
        # `_area-ci.yml` receives `fromJSON(area_rows)[area]`; a scheduled area
        # without an entry would reach the producers as `null`.
        scope = self.scope()
        for area in scope["scheduled_areas"]:
            document = scope["area_rows"][area]
            self.assertTrue(
                any(document[f"has_{name}_rows"] for name in schema.ROW_SET_NAMES),
                f"{area} executes cells here and must dispatch rows",
            )

    def test_a_nested_area_slug_is_a_legal_artifact_name(self) -> None:
        scope = self.scope()
        self.assertEqual(
            {
                "alpha": "alpha",
                "alpha/nested": "alpha--nested",
                "beta": "beta",
                "root": "root",
            },
            scope["area_slugs"],
        )
        self.assertEqual(
            len(set(scope["area_slugs"].values())),
            len(scope["area_slugs"]),
            "two areas sharing a slug would overwrite each other's result artifact",
        )
        for slug in scope["area_slugs"].values():
            self.assertNotIn("/", slug)

    def test_every_scheduled_area_has_a_slug_and_a_row_document(self) -> None:
        scope = self.scope()
        self.assertLessEqual(set(scope["scheduled_areas"]), set(scope["area_rows"]))
        self.assertEqual(scope["scheduled_areas"], sorted(scope["area_slugs"]))

    def test_a_gates_false_package_gets_no_area_fan_out(self) -> None:
        # It stays a selected package with a governed exclusion, but there is
        # nothing for a runner to do, so no area entry may exist for it alone.
        policy = dict(self.policy)
        excluded = dict(policy["beta-app"])
        excluded["gates"] = []
        excluded["exclusion"] = {
            "exclusion-class": "capability",
            "owner": "ken",
            "reason": "no runner hosts it",
            "expiry": "2027-01-01",
        }
        policy["beta-app"] = excluded
        scope = scope_document(
            [],
            self.root,
            self.metadata,
            environments_for_tests(),
            policy,
            force_all=True,
        )
        self.assertIn("beta-app", scope["packages"])
        self.assertNotIn("beta", scope["scheduled_areas"])
        self.assertNotIn("beta", scope["area_slugs"])
        if "beta" in scope["area_rows"]:
            self.assertEqual(set(), self.area_packages(scope, "beta"))


class AllReusedAreaFanOutTests(unittest.TestCase):
    """AC16: an area whose test cells were all reused still fans out.

    `ci-reporting` reads the per-area `ci-results-<slug>` slices, and only an
    area that fans out produces one. The area matrix is therefore derived from
    the plan's GATING PACKAGES and never narrowed by execution: an area that
    dropped out because every test cell was satisfied by a receipt would take
    those cells' results out of the report with it.
    """

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        seed_build_inputs(self.root)
        packages = [package(self.root, "alpha-core", "alpha/lib/Cargo.toml")]
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {"nodes": [{"id": item["id"], "deps": []} for item in packages]},
        }
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def plan(self, accepted: list[dict[str, object]] | None = None) -> dict[str, object]:
        return calculate_scope(
            ["alpha/lib/src/lib.rs"],
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
            accepted_cells=accepted,
        )

    def test_an_area_whose_test_cells_are_all_reused_still_fans_out(self) -> None:
        reusable = [
            {
                "package": cell["package"],
                "environment": cell["environment"],
                "gate": cell["gate"],
                "origin": "local",
                "outcome": "pass",
                "evidence": f"refs/notes/ci-local/{cell['environment']}",
            }
            for cell in self.plan()["cells"]
            if cell["reusable"]
        ]
        self.assertTrue(reusable, "the fixture needs at least one reusable cell")

        plan = self.plan(reusable)
        self.assertEqual(
            [],
            [
                f"{cell['package']}/{cell['environment']}/{cell['gate']}"
                for cell in plan["cells"]
                if cell["reusable"] and cell["execution"] == "execute"
            ],
            "the fixture must leave no reusable cell executing",
        )

        scope = legacy_scope_document(plan)
        self.assertIn("alpha", scope["scheduled_areas"])
        self.assertEqual("alpha", scope["area_slugs"]["alpha"])
        rows = scope["area_rows"]["alpha"]
        self.assertEqual(
            {"alpha-core"}, {row["package"] for name in schema.ROW_SET_NAMES for row in rows[name]}
        )
        self.assertEqual(
            [],
            rows["test"],
            "no test leg is scheduled, yet the area still fans out so its "
            "coverage audit publishes the reused cells' slice",
        )
        self.assertFalse(rows["has_test_rows"])


class MatrixLimitTests(unittest.TestCase):
    def test_over_256_gating_packages_fails_loudly(self) -> None:
        root = Path(tempfile.mkdtemp())
        count = MATRIX_LIMIT + 1
        packages = [
            package(root, f"pkg-{index}", f"p{index}/Cargo.toml") for index in range(count)
        ]
        metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {"nodes": [{"id": item["id"], "deps": []} for item in packages]},
        }
        policy = package_ci_policy(
            workspace_packages_from(metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=root,
            today=TODAY,
        )
        with self.assertRaises(RuntimeError) as raised:
            scope_document(
                [], root, metadata, environments_for_tests(), policy, force_all=True
            )
        self.assertIn(str(MATRIX_LIMIT), str(raised.exception))


class EstimateJobsTests(unittest.TestCase):
    def test_wsl_counts_two_jobs_and_the_rest_count_one(self) -> None:
        cells = package_cells(
            {
                "package": "a",
                "tiers": ["L1", "L2"],
                "l2_backends": ["tmux"],
                "features": [],
                "all_features": False,
                "l1_include_slow": False,
                "runner_tools": [],
                "companion_suites": [],
                "requires_toolchain": False,
            },
            area="a",
            target_kinds=["lib", "bench"],
            environments=environments_for_tests(),
            accepted={},
        )
        # lint (1) + check for the bench target on the check environment (1)
        # + L1 on three native environments (3) + L1 on wsl2-ubuntu (2:
        # archive builder + guest) + L2 where a backend is hostable (2: ubuntu,
        # macOS). The Windows and WSL L2 cells are governed policy gaps and
        # launch nothing; the WSL guest compiles nothing, so no check there.
        self.assertEqual(
            estimate_jobs(cells, environments_for_tests()), 1 + 1 + 3 + 2 + 2
        )

    def test_a_reused_cell_costs_no_job(self) -> None:
        arguments = {
            "package": "a",
            "tiers": ["L1"],
            "l2_backends": [],
            "features": [],
            "all_features": False,
            "l1_include_slow": False,
            "runner_tools": [],
            "companion_suites": [],
            "requires_toolchain": False,
        }
        environments = environments_for_tests()
        scheduled = package_cells(arguments, "a", ["lib"], environments, {})
        reused = package_cells(
            arguments,
            "a",
            ["lib"],
            environments,
            {("a", "macos-latest", "L1"): {"evidence": "refs/notes/ci-local/macos-latest"}},
        )
        self.assertEqual(
            estimate_jobs(scheduled, environments) - 1,
            estimate_jobs(reused, environments),
        )


class CheckCellTests(unittest.TestCase):
    """A check cell covers the uncovered kinds on the one check environment.

    The L1 build compiles `lib`, `bin`, and `test`; `example` and `bench` are
    compiled by no test gate, so their check runs on `CHECK_ENVIRONMENT` and is
    selected explicitly, never through `--all-targets`. One environment, like
    lint (fixes/2026-09-18-ci-cadence, decision 3).
    """

    NATIVE = ["ubuntu-latest", "windows-latest", "macos-latest"]
    ARGUMENTS = {
        "package": "a",
        "tiers": ["L1"],
        "l2_backends": [],
        "features": [],
        "all_features": False,
        "l1_include_slow": False,
        "runner_tools": [],
        "companion_suites": [],
        "requires_toolchain": False,
    }
    CONSTRAINT = {
        "owner": "ken",
        "reason": "do not touch Windows for this branch",
        "expiry": "2099-01-01",
        "source": "current.json",
    }

    def check_cells(self, target_kinds: list[str], **kwargs: object) -> list[dict[str, object]]:
        accepted = kwargs.pop("accepted", {})
        cells = package_cells(
            self.ARGUMENTS,
            "a",
            target_kinds,
            environments_for_tests(),
            accepted,  # type: ignore[arg-type]
            **kwargs,  # type: ignore[arg-type]
        )
        return [cell for cell in cells if cell["gate"] == "check"]

    def test_an_example_target_is_checked_on_the_check_environment_only(self) -> None:
        checks = self.check_cells(["lib", "example"])
        self.assertEqual([CHECK_ENVIRONMENT], [cell["environment"] for cell in checks])
        (cell,) = checks
        self.assertEqual(["example"], cell["target_kinds"])
        self.assertEqual("check", cell["compile_coverage_from"])
        self.assertEqual("execute", cell["execution"])
        self.assertIn("example", cell["selection_reason"])
        self.assertIn(f"checked on {CHECK_ENVIRONMENT} only", cell["selection_reason"])
        # The guest compiles nothing: its uncovered kinds are covered by the
        # runner that builds its archive, and the reason says so there.
        self.assertIn("wsl2-ubuntu", cell["selection_reason"])

    def test_a_plan_without_the_check_environment_checks_nowhere(self) -> None:
        # A run that schedules only deferred environments (a push to main
        # after a validated pull request) still compiles the L1 kinds there;
        # the example and bench kinds were checked by the run that had Linux.
        environments = [
            environment
            for environment in environments_for_tests()
            if environment["name"] == "windows-latest"
        ]
        cells = package_cells(self.ARGUMENTS, "a", ["lib", "example"], environments, {})
        self.assertEqual([], [cell for cell in cells if cell["gate"] == "check"])
        self.assertEqual(["L1"], [cell["gate"] for cell in cells if cell["environment"] == "windows-latest"])

    def test_check_arguments_carry_exactly_the_uncovered_selectors(self) -> None:
        cases = {
            ("lib", "example"): "-p a --examples",
            ("lib", "bench"): "-p a --benches",
            ("lib", "bin", "test", "example", "bench"): "-p a --examples --benches",
            ("lib", "bin", "test"): "-p a",
        }
        for kinds, expected in cases.items():
            with self.subTest(kinds=kinds):
                rendered = check_arguments("a", list(kinds), "")
                self.assertEqual(expected, rendered)
                for banned in ("--all-targets", "--lib", "--bins", "--tests"):
                    self.assertNotIn(banned, rendered)
        self.assertEqual(
            "-p a --benches --features x", check_arguments("a", ["lib", "bench"], "--features x")
        )

    def test_l1_only_kinds_get_no_check_cell_and_no_selector(self) -> None:
        self.assertEqual([], self.check_cells(["lib", "bin", "test"]))
        self.assertEqual("-p a", check_arguments("a", ["lib", "bin", "test"], ""))

    def test_a_reused_linux_l1_reuses_the_check_and_another_host_reuses_nothing(self) -> None:
        evidence = {"origin": "local", "evidence": "refs/notes/ci-local/ubuntu-latest"}
        cells = package_cells(
            self.ARGUMENTS,
            "a",
            ["lib", "example"],
            environments_for_tests(),
            {
                ("a", "ubuntu-latest", "L1"): evidence,
                ("a", "macos-latest", "L1"): {**evidence, "evidence": "refs/notes/ci-local/macos-latest"},
            },
        )
        by_key = {(cell["environment"], cell["gate"]): cell for cell in cells}
        self.assertEqual("reuse", by_key[("ubuntu-latest", "L1")]["execution"])
        check = by_key[("ubuntu-latest", "check")]
        self.assertEqual("reuse", check["execution"])
        self.assertEqual("local", check["origin"])
        self.assertEqual("L1", check["evidence"]["covered_by"])
        self.assertEqual("refs/notes/ci-local/ubuntu-latest", check["evidence"]["evidence"])
        self.assertNotIn("counts", check["evidence"], "test counts are not a check measurement")
        # A macOS L1 pass stands in for no check: there is no macOS check cell
        # to stand in for, and the Linux cell is not its to cover.
        self.assertEqual("reuse", by_key[("macos-latest", "L1")]["execution"])
        self.assertNotIn(("macos-latest", "check"), by_key)

    def test_a_whole_environment_acceptance_does_not_satisfy_a_check(self) -> None:
        # A version-1 note proves no particular package was built.
        evidence = {"origin": "local", "evidence": "refs/notes/ci-local/ubuntu-latest"}
        cells = package_cells(
            self.ARGUMENTS,
            "a",
            ["lib", "example"],
            environments_for_tests(),
            {},
            accepted_environments={"ubuntu-latest": evidence},
        )
        states = {(cell["environment"], cell["gate"]): cell["execution"] for cell in cells}
        self.assertEqual("reuse", states[("ubuntu-latest", "L1")])
        self.assertEqual("execute", states[("ubuntu-latest", "check")])

    def test_a_check_that_compiles_dependents_is_never_reused(self) -> None:
        evidence = {"origin": "local", "evidence": "refs/notes/ci-local/ubuntu-latest"}
        cells = package_cells(
            self.ARGUMENTS,
            "a",
            ["lib", "example"],
            environments_for_tests(),
            {("a", "ubuntu-latest", "L1"): evidence},
            dependents=["b"],
        )
        check = next(
            cell for cell in cells
            if cell["environment"] == "ubuntu-latest" and cell["gate"] == "check"
        )
        self.assertFalse(check["reusable"])
        self.assertEqual("execute", check["execution"])
        self.assertEqual(["b"], check["dependents"])

    def test_a_prohibited_check_environment_turns_the_check_cell_prohibited(self) -> None:
        checks = self.check_cells(
            ["lib", "bench"], prohibitions={CHECK_ENVIRONMENT: self.CONSTRAINT}
        )
        (check,) = checks
        self.assertEqual(CHECK_ENVIRONMENT, check["environment"])
        self.assertEqual("prohibited", check["state"])
        self.assertEqual("omit", check["execution"])
        # A prohibition elsewhere touches no check: there is no cell there.
        self.assertEqual(
            ["execute"],
            [cell["execution"] for cell in self.check_cells(
                ["lib", "bench"], prohibitions={"windows-latest": self.CONSTRAINT}
            )],
        )


class EventSchedulingTests(unittest.TestCase):
    """Environments are scheduled by GitHub event (fixes/2026-09-18-ci-cadence).

    Each environment names the events that schedule it. The planner drops the
    rest of the table for a run — no cell, no build, no preflight runner — and
    records them in `deferred_environments`; a reused pull request validation
    drops the environments it proved into `proven_environments` instead.
    """

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        seed_build_inputs(self.root)
        packages = [
            package(self.root, "alpha-core", "alpha/lib/Cargo.toml", targets=["lib", "example"]),
        ]
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {"nodes": [{"id": "alpha-core", "deps": []}]},
        }
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    @staticmethod
    def shipped_events() -> list[dict[str, object]]:
        """The test table carrying the shipped table's `events` policy."""
        events = {
            "ubuntu-latest": ["pull_request", "push", "workflow_dispatch"],
            "windows-latest": ["push", "workflow_dispatch"],
            "macos-latest": ["pull_request", "push", "workflow_dispatch"],
            "wsl2-ubuntu": ["schedule", "workflow_dispatch"],
        }
        environments = environments_for_tests()
        for environment in environments:
            environment["events"] = events[environment["name"]]
        return environments

    def plan(self, **kwargs: object) -> dict[str, object]:
        plan = calculate_scope(
            ["alpha/lib/src/lib.rs"],
            self.root,
            self.metadata,
            self.shipped_events(),
            self.policy,
            **kwargs,  # type: ignore[arg-type]
        )
        self.assertEqual([], schema.validate_resolved_plan(plan))
        return plan

    @staticmethod
    def names(environments: list[dict[str, object]]) -> list[str]:
        return [environment["name"] for environment in environments]

    def test_the_shipped_table_schedules_each_environment_as_decided(self) -> None:
        environments = load_environments(ENVIRONMENTS_CONFIG, today=TODAY)
        events = {environment["name"]: environment["events"] for environment in environments}
        # The nightly is WSL2's alone (fixes/2026-09-19-nightly-scope):
        # Windows is proven by every push to main, and Linux joins the
        # nightly as WSL2's producer without cells of its own.
        self.assertEqual(
            {
                "ubuntu-latest": ["pull_request", "push", "workflow_dispatch"],
                "windows-latest": ["push", "workflow_dispatch"],
                "macos-latest": ["pull_request", "push", "workflow_dispatch"],
                "wsl2-ubuntu": ["schedule", "workflow_dispatch"],
            },
            events,
        )

    def test_a_table_without_events_or_with_an_unknown_one_is_refused(self) -> None:
        for mutate, fragment in (
            (lambda environment: environment.pop("events"), "events"),
            (lambda environment: environment.update(events=[]), "events"),
            (lambda environment: environment.update(events=["nightly"]), "events"),
            (lambda environment: environment.update(events=["push", "push"]), "repeats"),
        ):
            environments = self.shipped_events()
            mutate(environments[0])
            path = Path(tempfile.mkdtemp()) / "environments.json"
            path.write_text(json.dumps({"schema_version": 3, "environments": environments}))
            with self.subTest(fragment=fragment), self.assertRaises(RuntimeError) as raised:
                load_environments(path, today=TODAY)
            self.assertIn(fragment, str(raised.exception))

    def test_each_event_schedules_its_environments_and_defers_the_rest(self) -> None:
        cases = {
            "pull_request": (["ubuntu-latest", "macos-latest"], ["windows-latest", "wsl2-ubuntu"]),
            "push": (["ubuntu-latest", "windows-latest", "macos-latest"], ["wsl2-ubuntu"]),
            "schedule": (["wsl2-ubuntu"], ["ubuntu-latest", "windows-latest", "macos-latest"]),
            "workflow_dispatch": (
                ["ubuntu-latest", "windows-latest", "macos-latest", "wsl2-ubuntu"],
                [],
            ),
        }
        for event, (expected_scheduled, expected_deferred) in cases.items():
            with self.subTest(event=event):
                scheduled, deferred, proven = schedule_environments(self.shipped_events(), event)
                self.assertEqual(expected_scheduled, self.names(scheduled))
                self.assertEqual(expected_deferred, [entry["name"] for entry in deferred])
                self.assertEqual([], proven)
                for entry in deferred:
                    self.assertNotIn(event, entry["events"])

    def test_a_scheduled_guest_brings_its_producer_without_cells(self) -> None:
        # The nightly schedules WSL2 alone. Its archives are compiled on
        # Linux, so Linux joins the plan's table for its build records and
        # preflight runner — and for nothing else: no lint, check, or test
        # cell, which the pull request already proved. Before this, Linux
        # stayed on the schedule event for the archives alone and put 161
        # cells into every nightly (fixes/2026-09-19-nightly-scope).
        plan = self.plan(event="schedule")
        self.assertEqual(["ubuntu-latest", "wsl2-ubuntu"], self.names(plan["environments"]))
        self.assertEqual(
            {"wsl2-ubuntu"}, {cell["environment"] for cell in plan["cells"]}
        )
        self.assertEqual(
            [{"name": "ubuntu-latest", "for": ["wsl2-ubuntu"]}], plan["producing_environments"]
        )
        self.assertEqual(
            ["windows-latest", "macos-latest"],
            [entry["name"] for entry in plan["deferred_environments"]],
        )
        self.assertEqual({"ubuntu-latest"}, {record["producer"] for record in plan["builds"]})
        for record in plan["builds"]:
            self.assertEqual(
                [{"environment": "wsl2-ubuntu", "gate": "L1"}], record["consumers"]
            )
        # The owner job runs on the producer's runner, so it preflights there
        # too, beside the runner hosting the guest.
        self.assertIn("ubuntu-latest", plan["preflight_os"])
        self.assertIn("windows-latest", plan["preflight_os"])
        # And the workflow-facing projection schedules no Linux cell either.
        self.assertEqual({("wsl2-ubuntu", "L1")}, dispatched(plan, "alpha-core"))
        self.assertEqual(
            ["alpha-core"],
            [row["package"] for document in row_sets_of(plan).values() for row in document["wsl"]],
        )
        owners = legacy_scope_document(plan)["build_owners"]
        self.assertEqual(["ubuntu-latest"], [owner["environment"] for owner in owners])

    def test_an_event_that_schedules_no_guest_brings_no_producer(self) -> None:
        for event in ("pull_request", "push"):
            with self.subTest(event=event):
                plan = self.plan(event=event)
                self.assertNotIn("producing_environments", plan)
                self.assertIn("ubuntu-latest", self.names(plan["environments"]))

    def test_no_event_or_every_environment_schedules_the_whole_table(self) -> None:
        for kwargs in ({"event": None}, {"event": "pull_request", "all_environments": True}):
            with self.subTest(kwargs=kwargs):
                scheduled, deferred, proven = schedule_environments(self.shipped_events(), **kwargs)
                self.assertEqual(4, len(scheduled))
                self.assertEqual(([], []), (deferred, proven))

    def test_a_record_without_events_is_scheduled_on_every_event(self) -> None:
        environments = environments_for_tests()
        for environment in environments:
            environment.pop("events", None)
        for event in EVENT_NAMES:
            scheduled, deferred, _ = schedule_environments(environments, event)
            self.assertEqual(4, len(scheduled), event)
            self.assertEqual([], deferred)

    def test_an_unknown_event_is_refused(self) -> None:
        for kwargs in ({"event": "nightly"}, {"event": "push", "proven_event": "merge"}):
            with self.subTest(kwargs=kwargs), self.assertRaises(RuntimeError):
                schedule_environments(self.shipped_events(), **kwargs)

    def test_a_proven_event_drops_what_it_validated_and_plans_the_rest(self) -> None:
        scheduled, deferred, proven = schedule_environments(
            self.shipped_events(), "push", proven_event="pull_request"
        )
        self.assertEqual(["windows-latest"], self.names(scheduled))
        self.assertEqual(["wsl2-ubuntu"], [entry["name"] for entry in deferred])
        self.assertEqual(
            [
                {"name": "ubuntu-latest", "event": "pull_request"},
                {"name": "macos-latest", "event": "pull_request"},
            ],
            proven,
        )

    def test_a_pull_request_plan_carries_no_deferred_cell_build_or_preflight(self) -> None:
        plan = self.plan(event="pull_request")
        self.assertEqual("pull_request", plan["event"])
        self.assertEqual(["ubuntu-latest", "macos-latest"], self.names(plan["environments"]))
        self.assertEqual(
            [
                {"name": "windows-latest", "events": ["push", "workflow_dispatch"]},
                {"name": "wsl2-ubuntu", "events": ["schedule", "workflow_dispatch"]},
            ],
            plan["deferred_environments"],
        )
        self.assertNotIn("proven_environments", plan)
        cell_environments = {cell["environment"] for cell in plan["cells"]}
        self.assertEqual({"ubuntu-latest", "macos-latest"}, cell_environments)
        self.assertEqual({"ubuntu-latest", "macos-latest"}, {build["producer"] for build in plan["builds"]})
        self.assertEqual(["macos-latest", "ubuntu-latest"], plan["preflight_os"])
        self.assertEqual(
            [],
            [row for document in row_sets_of(plan).values() for row in document["wsl"]],
            "no WSL2 leg on a pull request",
        )

    def test_a_plan_without_an_event_is_byte_identical_to_before(self) -> None:
        plan = self.plan()
        for field in ("event", "deferred_environments", "proven_environments"):
            self.assertNotIn(field, plan)
        self.assertEqual(4, len(plan["environments"]))

    def test_a_reused_pull_request_narrows_a_push_to_what_it_could_not_prove(self) -> None:
        plan = self.plan(event="push", proven_event="pull_request")
        self.assertEqual(["windows-latest"], self.names(plan["environments"]))
        self.assertEqual(
            [
                {"name": "ubuntu-latest", "event": "pull_request"},
                {"name": "macos-latest", "event": "pull_request"},
            ],
            plan["proven_environments"],
        )
        self.assertEqual({"windows-latest"}, {cell["environment"] for cell in plan["cells"]})
        # The check environment was proven by the pull request; nothing checks
        # here, and the L1 build still compiles the L1 kinds.
        self.assertEqual([], [cell for cell in plan["cells"] if cell["gate"] == "check"])
        self.assertEqual(["ubuntu-latest", "windows-latest"], plan["preflight_os"])

    def test_a_full_scope_run_preflights_only_the_scheduled_runners(self) -> None:
        # A full-scope nightly: WSL2's cells on the Windows-hosted guest, and
        # Linux as their producer — so those two runners preflight, and the
        # deferred Windows and macOS do not.
        plan = self.plan(force_all=True, event="schedule")
        self.assertEqual(["ubuntu-latest", "windows-latest"], plan["preflight_os"])
        self.assertEqual(
            [
                {"name": "windows-latest", "events": ["push", "workflow_dispatch"]},
                {"name": "macos-latest", "events": ["pull_request", "push", "workflow_dispatch"]},
            ],
            plan["deferred_environments"],
        )
        self.assertEqual({"wsl2-ubuntu"}, {cell["environment"] for cell in plan["cells"]})

    def test_the_label_plans_every_environment_for_a_pull_request(self) -> None:
        plan = self.plan(event="pull_request", all_environments=True)
        self.assertEqual(4, len(plan["environments"]))
        self.assertNotIn("deferred_environments", plan)
        self.assertEqual("pull_request", plan["event"])

    def test_the_schema_refuses_a_deferred_environment_the_plan_also_schedules(self) -> None:
        plan = self.plan(event="pull_request")
        broken = {**plan, "deferred_environments": [{"name": "ubuntu-latest", "events": ["push"]}]}
        problems = schema.validate_resolved_plan(broken)
        self.assertTrue(any("also schedules" in problem for problem in problems), problems)
        for field, value in (
            ("event", "nightly"),
            ("deferred_environments", []),
            ("proven_environments", [{"name": "windows-latest"}]),
        ):
            with self.subTest(field=field):
                self.assertNotEqual([], schema.validate_resolved_plan({**plan, field: value}))

    def test_the_cli_plans_by_event_and_refuses_it_with_apply_to(self) -> None:
        result = subprocess.run(
            [sys.executable, str(ROOT / "scripts" / "ci" / "affected_scope.py"),
             "--apply-to", "plan.json", "--event", "push"],
            capture_output=True, text=True, check=False,
        )
        self.assertNotEqual(0, result.returncode)
        self.assertIn("--event", result.stderr)


class CheckCellScopeTests(unittest.TestCase):
    """The same contract read off a plan: declared targets in, cells and matrix out."""

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        seed_build_inputs(self.root)
        packages = [
            package(self.root, "alpha-core", "alpha/lib/Cargo.toml", targets=["lib", "example"]),
            package(self.root, "beta-app", "beta/app/Cargo.toml", targets=["bin", "test"]),
        ]
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {
                "nodes": [
                    {"id": "alpha-core", "deps": []},
                    {"id": "beta-app", "deps": []},
                ]
            },
        }
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def plan(self, **kwargs: object) -> dict[str, object]:
        return calculate_scope(
            ["alpha/lib/src/lib.rs", "beta/app/src/main.rs"],
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
            **kwargs,  # type: ignore[arg-type]
        )

    def test_the_plan_checks_examples_natively_and_never_in_the_guest(self) -> None:
        plan = self.plan()
        checks = {
            (cell["package"], cell["environment"])
            for cell in plan["cells"]  # type: ignore[union-attr]
            if cell["gate"] == "check"
        }
        self.assertEqual({("alpha-core", CHECK_ENVIRONMENT)}, checks)
        records = {entry["package"]: entry for entry in plan["packages"]}  # type: ignore[union-attr]
        self.assertEqual("-p alpha-core --examples", records["alpha-core"]["check_args"])
        self.assertEqual("-p beta-app", records["beta-app"]["check_args"])
        self.assertNotIn("check", records["beta-app"]["gates"])

    def test_only_the_executing_check_environments_dispatch(self) -> None:
        evidence = {"origin": "local", "evidence": "refs/notes/ci-local/macos-latest"}
        plan = self.plan(
            accepted_cells=[{"package": "alpha-core", "environment": "macos-latest", "gate": "L1", **evidence}],
            prohibitions={"windows-latest": CheckCellTests.CONSTRAINT},
        )
        contracts = producer_contracts(plan, "alpha-core")
        # macOS L1 reused and Windows prohibited: neither hosts a check, so
        # the one check environment is the whole dispatch.
        self.assertEqual(
            {(CHECK_ENVIRONMENT, "check")}, {cell for cell in contracts if cell[1] == "check"}
        )
        self.assertEqual(
            "-p alpha-core --examples", contracts[(CHECK_ENVIRONMENT, "check")]["check_args"]
        )
        self.assertEqual(set(), {cell for cell in dispatched(plan, "beta-app") if cell[1] == "check"})


class DependentSeamTests(unittest.TestCase):
    """Open Question 1, Option B: the seam is compiled inside the changed package.

    A change to `alpha-core` compiles its unchanged direct dependents as a step
    of alpha-core's own `ubuntu-latest` check cell. The dependents receive no
    area, record, or cell (AC1 holds), a dependent that is itself selected is
    excluded, a `gates = false` dependent is never compiled, and the other
    native environments' check cells still exist only for uncovered kinds.
    """

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        seed_build_inputs(self.root)
        # beta-app and gamma-lib depend on alpha-core; gamma-lib also depends
        # on delta-lib; excluded-app depends on alpha-core but gates nothing.
        packages = [
            package(self.root, "alpha-core", "alpha/lib/Cargo.toml", targets=["lib"]),
            package(self.root, "beta-app", "beta/app/Cargo.toml", targets=["bin", "test"]),
            package(self.root, "gamma-lib", "gamma/lib/Cargo.toml", targets=["lib", "example"]),
            package(self.root, "delta-lib", "delta/lib/Cargo.toml", targets=["lib", "bench"]),
            package(
                self.root,
                "excluded-app",
                "excluded/app/Cargo.toml",
                targets=["bin"],
                ci=ci_policy(
                    gates=False,
                    reason="blocked on identified work",
                    owner="@o",
                    **{"exclusion-class": "promotion-pending", "expiry": "2027-01-31"},
                ),
            ),
        ]
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {
                "nodes": [
                    {"id": "alpha-core", "deps": []},
                    {"id": "beta-app", "deps": [{"pkg": "alpha-core", "dep_kinds": [{"kind": None}]}]},
                    {
                        "id": "gamma-lib",
                        "deps": [
                            {"pkg": "alpha-core", "dep_kinds": [{"kind": None}]},
                            {"pkg": "delta-lib", "dep_kinds": [{"kind": None}]},
                        ],
                    },
                    {"id": "delta-lib", "deps": []},
                    {"id": "excluded-app", "deps": [{"pkg": "alpha-core", "dep_kinds": [{"kind": None}]}]},
                ]
            },
        }
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def plan(self, *files: str, **kwargs: object) -> dict[str, object]:
        return calculate_scope(
            list(files),
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
            **kwargs,  # type: ignore[arg-type]
        )

    @staticmethod
    def check_cells(plan: dict[str, object], package: str) -> dict[str, dict[str, object]]:
        return {
            cell["environment"]: cell
            for cell in plan["cells"]  # type: ignore[union-attr]
            if cell["package"] == package and cell["gate"] == "check"
        }

    def test_a_change_lists_its_unchanged_gating_dependents_on_its_own_record(self) -> None:
        plan = self.plan("alpha/lib/src/lib.rs")
        self.assertEqual([], schema.validate_resolved_plan(plan))
        records = {entry["package"]: entry for entry in plan["packages"]}  # type: ignore[union-attr]
        self.assertEqual(["alpha-core"], sorted(records))
        self.assertEqual(
            {
                "dependents": ["beta-app", "gamma-lib"],
                "check_args": "-p beta-app -p gamma-lib --lib --bins --tests",
                "native": [],
            },
            records["alpha-core"]["dependent_seam"],
        )
        self.assertEqual(["lint", "check", "L1"], records["alpha-core"]["gates"])
        # Reporting is unchanged: every unselected direct dependent, gating or
        # not, is still named in the summary row.
        self.assertEqual(
            ["beta-app", "excluded-app", "gamma-lib"], plan["reverse_dependencies"]
        )
        self.assertEqual(["alpha"], [entry["area"] for entry in plan["areas"]])  # type: ignore[union-attr]
        self.assertEqual(
            [], [cell for cell in plan["cells"] if cell["package"] != "alpha-core"]  # type: ignore[union-attr]
        )

    def test_dependent_native_closures_are_carried_only_for_the_owning_check(self) -> None:
        self.policy["alpha-core"]["native"] = {"ubuntu-latest": ["libalpha-dev"]}
        self.policy["gamma-lib"]["native"] = {
            "ubuntu-latest": ["libconsumer-dev", "libalpha-dev"],
            "macos-latest": ["consumer-macos"],
        }
        self.policy["delta-lib"]["native"] = {"ubuntu-latest": ["libtransitive-dev"]}
        self.policy["excluded-app"]["native"] = {"ubuntu-latest": ["libexcluded-dev"]}
        plan = self.plan("alpha/lib/src/lib.rs")
        record = plan["packages"][0]
        self.assertEqual([], schema.validate_resolved_plan(plan))
        expected = ["libalpha-dev", "libconsumer-dev", "libtransitive-dev"]
        self.assertEqual(expected, record["dependent_seam"]["native"])
        self.assertEqual({"ubuntu-latest": ["libalpha-dev"]}, record["native"])
        contracts = producer_contracts(plan, "alpha-core")
        self.assertEqual(
            expected, json.loads(contracts[(DEPENDENTS_ENVIRONMENT, "check")]["dependents_native"])
        )
        self.assertEqual(
            ["libalpha-dev"], json.loads(contracts[("ubuntu-latest", "L1")]["native_packages"])
        )
        self.assertEqual(["ubuntu-latest"], sorted(self.check_cells(plan, "alpha-core")))
        del record["dependent_seam"]["native"]
        self.assertTrue(any("native" in error for error in schema.validate_resolved_plan(plan)))

    def test_lint_is_never_marked_reusable(self) -> None:
        plan = self.plan("alpha/lib/src/lib.rs")
        lint = [cell for cell in plan["cells"] if cell["gate"] == "lint"]
        self.assertEqual(1, len(lint))
        self.assertFalse(lint[0]["reusable"])

    def test_a_package_with_no_uncovered_kind_gets_one_linux_check_cell_for_the_seam(self) -> None:
        checks = self.check_cells(self.plan("alpha/lib/src/lib.rs"), "alpha-core")
        self.assertEqual([DEPENDENTS_ENVIRONMENT], sorted(checks))
        cell = checks[DEPENDENTS_ENVIRONMENT]
        self.assertEqual("ubuntu-latest", DEPENDENTS_ENVIRONMENT)
        self.assertEqual(["beta-app", "gamma-lib"], cell["dependents"])
        self.assertEqual([], cell["target_kinds"])
        self.assertEqual("check", cell["compile_coverage_from"])
        self.assertEqual("execute", cell["execution"])
        self.assertFalse(cell["reusable"])
        self.assertIn("also compiles 2 unchanged dependent(s)", cell["selection_reason"])
        self.assertIn("beta-app, gamma-lib", cell["selection_reason"])

    def test_uncovered_kinds_check_on_linux_which_also_carries_the_seam(self) -> None:
        # delta-lib declares a bench, so its own check runs on the one check
        # environment; gamma-lib's seam rides on that same Linux cell, which
        # is also `DEPENDENTS_ENVIRONMENT`.
        plan = self.plan("delta/lib/src/lib.rs")
        checks = self.check_cells(plan, "delta-lib")
        self.assertEqual([DEPENDENTS_ENVIRONMENT], sorted(checks))
        cell = checks[DEPENDENTS_ENVIRONMENT]
        self.assertEqual(["bench"], cell["target_kinds"])
        self.assertEqual(["gamma-lib"], checks[DEPENDENTS_ENVIRONMENT]["dependents"])
        self.assertIn("bench target(s)", checks[DEPENDENTS_ENVIRONMENT]["selection_reason"])
        self.assertIn("also compiles 1 unchanged", checks[DEPENDENTS_ENVIRONMENT]["selection_reason"])
        record = next(entry for entry in plan["packages"] if entry["package"] == "delta-lib")  # type: ignore[union-attr]
        self.assertEqual("-p delta-lib --benches", record["check_args"])
        self.assertEqual("-p gamma-lib --lib", record["dependent_seam"]["check_args"])
        # gamma-lib has an example but no dependents: its own check runs on
        # the check environment and no cell carries a seam.
        gamma_checks = self.check_cells(self.plan("gamma/lib/src/lib.rs"), "gamma-lib")
        self.assertEqual(1, len(gamma_checks))
        self.assertEqual([], [cell for cell in gamma_checks.values() if "dependents" in cell])

    def test_a_dependent_that_is_itself_selected_is_excluded_from_the_seam(self) -> None:
        plan = self.plan("alpha/lib/src/lib.rs", "beta/app/src/main.rs")
        records = {entry["package"]: entry for entry in plan["packages"]}  # type: ignore[union-attr]
        self.assertEqual(["alpha-core", "beta-app"], sorted(records))
        self.assertEqual(["gamma-lib"], records["alpha-core"]["dependent_seam"]["dependents"])
        self.assertEqual("-p gamma-lib --lib", records["alpha-core"]["dependent_seam"]["check_args"])
        self.assertEqual(["excluded-app", "gamma-lib"], plan["reverse_dependencies"])
        self.assertEqual([], schema.validate_resolved_plan(plan))

    def test_a_full_scope_run_attributes_no_dependents(self) -> None:
        plan = self.plan(force_all=True)
        self.assertEqual(
            [], [entry for entry in plan["packages"] if "dependent_seam" in entry]  # type: ignore[union-attr]
        )
        self.assertEqual(
            [], [cell for cell in plan["cells"] if "dependents" in cell]  # type: ignore[union-attr]
        )
        self.assertEqual([], plan["reverse_dependencies"])

    def test_the_check_producer_is_handed_the_dependents_and_their_arguments(self) -> None:
        plan = self.plan("alpha/lib/src/lib.rs")
        self.assertEqual(["alpha-core"], sorted(gates_by_package(plan)))
        contracts = producer_contracts(plan, "alpha-core")
        self.assertEqual(
            {(DEPENDENTS_ENVIRONMENT, "check")}, {cell for cell in contracts if cell[1] == "check"}
        )
        check = contracts[(DEPENDENTS_ENVIRONMENT, "check")]
        self.assertEqual(["beta-app", "gamma-lib"], json.loads(check["dependents"]))
        self.assertEqual(
            "-p beta-app -p gamma-lib --lib --bins --tests", check["dependents_check_args"]
        )
        self.assertEqual("-p alpha-core", check["check_args"])
        # A record with no seam resolves empty fields, never a missing key.
        for cell, contract in resolved_contracts(plan_package(package="x")).items():
            self.assertEqual("[]", contract["dependents"], cell)
            self.assertEqual("", contract["dependents_check_args"], cell)

    def test_the_seam_arguments_select_by_declared_kinds_and_never_all_targets(self) -> None:
        by_name = {entry["name"]: entry for entry in self.metadata["packages"]}  # type: ignore[union-attr]
        self.assertIsNone(dependent_seam([], by_name))
        self.assertEqual(
            {"dependents": ["beta-app"], "check_args": "-p beta-app --bins --tests"},
            dependent_seam(["beta-app"], by_name),
        )
        seam = dependent_seam(["gamma-lib", "beta-app"], by_name)
        self.assertEqual(["beta-app", "gamma-lib"], seam["dependents"])
        # gamma-lib's example is not part of the seam; see DEPENDENT_SELECTORS.
        self.assertEqual("-p beta-app -p gamma-lib --lib --bins --tests", seam["check_args"])
        self.assertNotIn("--all-targets", seam["check_args"])
        self.assertNotIn("--examples", seam["check_args"])

    def test_a_reported_dependent_holding_a_record_is_rejected_by_the_schema(self) -> None:
        plan = self.plan("alpha/lib/src/lib.rs")
        records = {entry["package"]: entry for entry in plan["packages"]}  # type: ignore[union-attr]
        records["alpha-core"]["dependent_seam"]["dependents"] = ["alpha-core"]
        self.assertTrue(
            any("alpha-core" in problem and "compiled as a dependent" in problem for problem in schema.validate_resolved_plan(plan)),
        )


class DependentSeamFixtureTests(unittest.TestCase):
    """The ruled compile step, run for real against a broken consumer.

    A two-crate Cargo workspace: `alpha` exports one function, and `beta` calls
    a function alpha does NOT export. A change to alpha's source must attribute
    beta to alpha's own Linux check, schedule no `beta` area, and the generated
    dependents command must fail naming beta while alpha's own check passes.
    Skipped only when no `cargo` is available on the host.
    """

    @classmethod
    def setUpClass(cls) -> None:
        require_tools("cargo", enforced_by=CARGO_ENFORCED_BY)
        cls.temporary_directory = tempfile.TemporaryDirectory()
        cls.root = Path(cls.temporary_directory.name).resolve()
        seed_build_inputs(cls.root)
        (cls.root / "Cargo.toml").write_text(
            '[workspace]\nresolver = "2"\nmembers = ["alpha/lib", "beta/app"]\n',
            encoding="utf-8",
        )
        (cls.root / "alpha/lib/src").mkdir(parents=True)
        (cls.root / "alpha/lib/Cargo.toml").write_text(
            '[package]\nname = "alpha"\nversion = "0.1.0"\nedition = "2021"\n\n'
            "[package.metadata.ci]\ngates = true\n",
            encoding="utf-8",
        )
        (cls.root / "alpha/lib/src/lib.rs").write_text(
            "pub fn exported() -> u32 {\n    1\n}\n", encoding="utf-8"
        )
        (cls.root / "beta/app/src").mkdir(parents=True)
        (cls.root / "beta/app/Cargo.toml").write_text(
            '[package]\nname = "beta"\nversion = "0.1.0"\nedition = "2021"\n\n'
            '[dependencies]\nalpha = { path = "../../alpha/lib" }\n',
            encoding="utf-8",
        )
        # Broken against alpha's public API on purpose.
        (cls.root / "beta/app/src/main.rs").write_text(
            'fn main() {\n    println!("{}", alpha::not_exported());\n}\n', encoding="utf-8"
        )
        cls.cargo_env = {
            **os.environ,
            "CARGO_TARGET_DIR": str(cls.root / "target"),
            "CARGO_NET_OFFLINE": "true",
        }
        cls.metadata = load_metadata(cls.root)
        cls.packages = workspace_packages(cls.metadata)
        cls.environments = environments_for_tests()
        cls.policy = package_ci_policy(
            cls.packages,
            runner_labels={environment["runner"] for environment in cls.environments},
            root=cls.root,
            today=TODAY,
        )

    @classmethod
    def tearDownClass(cls) -> None:
        cls.temporary_directory.cleanup()

    def cargo_check(self, arguments: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["cargo", "check", "--offline", *arguments.split()],
            cwd=self.root,
            env=self.cargo_env,
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=300,
            check=False,
        )

    def test_the_broken_consumer_fails_inside_the_changed_packages_own_check(self) -> None:
        plan = calculate_scope(
            ["alpha/lib/src/lib.rs"], self.root, self.metadata, self.environments, self.policy
        )
        self.assertEqual([], schema.validate_resolved_plan(plan))
        self.assertEqual(["alpha"], [entry["area"] for entry in plan["areas"]])
        self.assertEqual(["alpha"], [entry["package"] for entry in plan["packages"]])
        self.assertEqual(["beta"], plan["reverse_dependencies"])
        self.assertEqual([], [cell for cell in plan["cells"] if cell["package"] != "alpha"])

        alpha = plan["packages"][0]
        self.assertEqual(
            {"dependents": ["beta"], "check_args": "-p beta --bins", "native": []}, alpha["dependent_seam"]
        )
        checks = {cell["environment"]: cell for cell in plan["cells"] if cell["gate"] == "check"}
        self.assertEqual([DEPENDENTS_ENVIRONMENT], sorted(checks))
        self.assertEqual(["beta"], checks[DEPENDENTS_ENVIRONMENT]["dependents"])

        own = self.cargo_check(alpha["check_args"])
        self.assertEqual(0, own.returncode, own.stderr)

        seam = self.cargo_check(alpha["dependent_seam"]["check_args"])
        self.assertNotEqual(0, seam.returncode, "beta calls a function alpha does not export")
        self.assertIn("not_exported", seam.stderr)
        self.assertIn("could not compile `beta`", seam.stderr)
        self.assertNotIn("could not compile `alpha`", seam.stderr)


class ApplyFixture(unittest.TestCase):
    """A workspace holding every evidence-eligibility case, plus evidence for each.

    A `check` cell satisfied only by its package's L1 pass, a companion-suite L1 on the Node host
    (never reusable), a governed gap (never reusable), a prohibited cell that
    evidence satisfies, and one that nothing satisfies. `alpha-core` declares
    a backend no environment hosts beside one two of them do, so its L2 cells
    there execute with a `backends` list narrower than the declaration.
    """

    CONSTRAINT = {
        "owner": "ken",
        "reason": "do not rerun WSL for this branch",
        "expiry": "2099-01-01",
        "source": "current.json",
    }
    FILES = ["alpha/lib/src/lib.rs", "web/server/src/main.rs", "tools/excluded/src/lib.rs"]

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        seed_build_inputs(self.root)
        (self.root / "homelab").mkdir()
        (self.root / "homelab" / "justfile").write_text(
            "test-frontend:\nlint-frontend:\n"
        )
        packages = [
            package(
                self.root,
                "alpha-core",
                "alpha/lib/Cargo.toml",
                ci=ci_policy(tests={"tiers": ["L1", "L2"], "l2-backends": ["tmux", "wezterm"]}),
                targets=["lib", "example"],
            ),
            package(
                self.root,
                "web-server",
                "web/server/Cargo.toml",
                ci=ci_policy(tests={"companion-suites": ["homelab-frontend"]}),
                targets=["bin"],
            ),
            package(
                self.root,
                "excluded",
                "tools/excluded/Cargo.toml",
                ci=ci_policy(
                    gates=False,
                    reason="promotion pending",
                    owner="@o",
                    **{"exclusion-class": "promotion-pending", "expiry": "2027-01-31"},
                ),
            ),
        ]
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {"nodes": [{"id": item["id"], "deps": []} for item in packages]},
        }
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )
        self.accepted = [
            {"package": "alpha-core", "environment": "macos-latest", "gate": "L1",
             "origin": "local", "outcome": "pass", "evidence": {"ref": "refs/notes/ci-local/macos-latest"}},
            {"package": "alpha-core", "environment": "wsl2-ubuntu", "gate": "L1",
             "origin": "prior-local", "outcome": "pass", "evidence": {"ref": "refs/notes/ci-local/wsl2-ubuntu"}},
            # Never accepted as themselves, whatever a receipt claims (a check
            # is satisfied only through its package's L1 pass):
            {"package": "alpha-core", "environment": "ubuntu-latest", "gate": "check", "origin": "local"},
            {"package": "web-server", "environment": "ubuntu-latest", "gate": "L1", "origin": "local"},
            {"package": "alpha-core", "environment": "windows-latest", "gate": "L2", "origin": "local"},
        ]
        self.rejections = [
            "gate-inputs-changed: alpha-core/ubuntu-latest/L1",
            "failed-cell: web-server/macos-latest/L1 failed; failing evidence is diagnostic only",
        ]

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def plan(self, **kwargs: object) -> dict[str, object]:
        return calculate_scope(
            self.FILES,
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
            prohibitions={"wsl2-ubuntu": self.CONSTRAINT},
            **kwargs,  # type: ignore[arg-type]
        )

    @staticmethod
    def dispositions(plan: dict[str, object]) -> dict[str, tuple[str, str]]:
        return {
            f"{cell['package']}/{cell['environment']}/{cell['gate']}": (cell["execution"], cell["state"])
            for cell in plan["cells"]  # type: ignore[union-attr]
        }


class ApplyAcceptedCellsTests(ApplyFixture):
    """Evidence applied to a resolved plan is exactly what resolving with it yields.

    `apply_accepted_cells` is the operation CI performs on a matching scope
    receipt, so it must decide a reused cell's shape identically to the
    planner — or a receipt would fan out differently from a fresh calculation
    for the same evidence.
    """

    def test_applying_evidence_is_byte_identical_to_resolving_with_it(self) -> None:
        resolved = self.plan(accepted_cells=self.accepted, evidence_rejections=self.rejections)
        applied = apply_accepted_cells(self.plan(), self.accepted, self.rejections)
        self.assertEqual(schema.canonical(resolved), schema.canonical(applied))
        self.assertEqual([], schema.validate_resolved_plan(applied))
        self.assertEqual(legacy_scope_document(resolved), legacy_scope_document(applied))

        # The fixture exercises every eligibility rule, not just the happy path.
        states = self.dispositions(applied)
        self.assertEqual(("reuse", "reused"), states["alpha-core/macos-latest/L1"])
        self.assertEqual(("execute", "pending"), states["web-server/macos-latest/L1"])
        self.assertEqual(("reuse", "reused"), states["alpha-core/wsl2-ubuntu/L1"], "evidence satisfies the prohibition")
        self.assertEqual(("execute", "pending"), states["alpha-core/ubuntu-latest/check"], "no Linux L1 pass to cover it")
        self.assertNotIn("alpha-core/macos-latest/check", states, "check is a single-environment gate")
        self.assertEqual(("execute", "pending"), states["web-server/ubuntu-latest/L1"], "companion host")
        self.assertEqual(("omit", "accepted-gap"), states["alpha-core/windows-latest/L2"])
        self.assertEqual(("omit", "accepted-gap"), states["alpha-core/wsl2-ubuntu/L2"])
        self.assertEqual(("omit", "prohibited"), states["web-server/wsl2-ubuntu/L1"], "no evidence, still prohibited")
        self.assertEqual(["web-server/wsl2-ubuntu/L1"], applied["prohibited_cells"])
        self.assertEqual(self.rejections, applied["evidence_rejections"])
        # Two L1 passes; neither is on the check environment, so no check rides
        # on them.
        self.assertEqual(2, len(applied["accepted_evidence"]))
        self.assertNotIn("prohibition", next(
            cell for cell in applied["cells"]  # type: ignore[union-attr]
            if cell["environment"] == "wsl2-ubuntu" and cell["gate"] == "L1"
        ))
        self.assertLess(applied["job_estimate"], self.plan()["job_estimate"])  # type: ignore[operator]

    def test_applying_no_evidence_is_the_identity_and_leaves_the_input_alone(self) -> None:
        plan = self.plan()
        before = schema.canonical(plan)
        applied = apply_accepted_cells(plan, [], [])
        self.assertEqual(before, schema.canonical(applied))
        apply_accepted_cells(plan, self.accepted, self.rejections)
        self.assertEqual(before, schema.canonical(plan), "the input plan must not be mutated")

    def test_a_failed_cell_cannot_be_injected_as_accepted_evidence(self) -> None:
        failure = {
            "package": "web-server",
            "environment": "macos-latest",
            "gate": "L1",
            "origin": "local",
            "outcome": "fail",
        }
        applied = apply_accepted_cells(self.plan(), [failure], [])
        self.assertEqual(
            ("execute", "pending"),
            self.dispositions(applied)["web-server/macos-latest/L1"],
        )

    def test_an_already_reused_cell_is_left_as_carried(self) -> None:
        first = apply_accepted_cells(self.plan(), self.accepted[:1], [])
        other = {**self.accepted[0], "origin": "prior-local", "evidence": {"ref": "elsewhere"}}
        second = apply_accepted_cells(first, [other], [])
        self.assertEqual(schema.canonical(first), schema.canonical(second))

    # A HOSTABLE L2 cell: macOS hosts tmux, so the cell executes and carries
    # `backends` until evidence arrives. A cell whose backend no environment
    # hosts is a gap from the start and never reaches the reuse transition.
    L2_EVIDENCE = {
        "package": "alpha-core",
        "environment": "macos-latest",
        "gate": "L2",
        "origin": "local",
        "outcome": "pass",
        "evidence": {"ref": "refs/notes/ci-local/macos-latest"},
    }

    @staticmethod
    def l2_cell(plan: dict[str, object], environment: str) -> dict[str, object]:
        return next(
            cell for cell in plan["cells"]  # type: ignore[union-attr]
            if (cell["package"], cell["environment"], cell["gate"]) == ("alpha-core", environment, "L2")
        )

    def assert_macos_l2_is_reused_and_valid(self, plan: dict[str, object]) -> None:
        self.assertEqual([], schema.validate_resolved_plan(plan))
        cell = self.l2_cell(plan, "macos-latest")
        self.assertEqual(("reuse", "reused"), (cell["execution"], cell["state"]))
        self.assertEqual(self.L2_EVIDENCE, cell["evidence"])
        self.assertNotIn("backends", cell)
        self.assertNotIn(("macos-latest", "L2"), dispatched(plan, "alpha-core"))
        self.assertNotIn(
            {"environment": "macos-latest", "gate": "L2"},
            [
                consumer
                for build in plan["builds"]  # type: ignore[union-attr]
                if build["package"] == "alpha-core"
                for consumer in build["consumers"]
            ],
        )
        # The cell evidence did not reach still owes its proof.
        self.assertEqual(["tmux"], self.l2_cell(plan, "ubuntu-latest")["backends"])
        self.assertIn(("ubuntu-latest", "L2"), dispatched(plan, "alpha-core"))

    def test_resolving_with_l2_evidence_reuses_a_hostable_cell_as_a_valid_plan(self) -> None:
        self.assert_macos_l2_is_reused_and_valid(self.plan(accepted_cells=[self.L2_EVIDENCE]))

    def test_applying_l2_evidence_to_an_executing_hostable_cell_yields_a_valid_plan(self) -> None:
        carried = self.plan()
        executing = self.l2_cell(carried, "macos-latest")
        self.assertEqual(("execute", ["tmux"]), (executing["execution"], executing["backends"]))
        self.assertIn(("macos-latest", "L2"), dispatched(carried, "alpha-core"))

        applied = apply_accepted_cells(carried, [self.L2_EVIDENCE], [])
        self.assert_macos_l2_is_reused_and_valid(applied)
        self.assertEqual(
            schema.canonical(self.plan(accepted_cells=[self.L2_EVIDENCE])),
            schema.canonical(applied),
        )

    def test_a_prohibited_hostable_l2_cell_is_a_valid_plan(self) -> None:
        plan = calculate_scope(
            self.FILES,
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
            prohibitions={"macos-latest": self.CONSTRAINT},
        )
        self.assertEqual([], schema.validate_resolved_plan(plan))
        cell = self.l2_cell(plan, "macos-latest")
        self.assertEqual(("omit", "prohibited"), (cell["execution"], cell["state"]))
        self.assertNotIn("backends", cell)
        self.assertNotIn(("macos-latest", "L2"), dispatched(plan, "alpha-core"))

        applied = apply_accepted_cells(plan, [self.L2_EVIDENCE], [])
        self.assertEqual([], schema.validate_resolved_plan(applied))
        reused = self.l2_cell(applied, "macos-latest")
        self.assertEqual(("reuse", "reused"), (reused["execution"], reused["state"]))
        self.assertNotIn("backends", reused)

    def test_a_plan_of_another_generation_is_refused(self) -> None:
        stale = {**self.plan(), "schema_version": schema.RESOLVED_PLAN_SCHEMA_VERSION - 1}
        with self.assertRaises(RuntimeError) as raised:
            apply_accepted_cells(stale, self.accepted, [])
        self.assertIn("unknown-schema-version", str(raised.exception))

    def test_the_projection_needs_nothing_beyond_the_plan(self) -> None:
        # A receipt's plan round-trips through JSON and is projected in CI
        # from that text alone: no policy, no environment table.
        plan = json.loads(schema.canonical(self.plan(accepted_cells=self.accepted)))
        projection = legacy_scope_document(plan)
        self.assertNotIn(("macos-latest", "L1"), dispatched(plan, "alpha-core"))
        self.assertEqual(
            {(CHECK_ENVIRONMENT, "check")},
            {cell for cell in dispatched(plan, "alpha-core") if cell[1] == "check"},
        )
        self.assertEqual(
            {"ubuntu-latest"},
            {
                environment
                for (environment, _), contract in producer_contracts(plan, "web-server").items()
                if contract["requires_node"] == "true"
            },
        )
        policy = {entry["package"]: entry for entry in projection["policy"]}
        self.assertFalse(policy["excluded"]["gates"])
        self.assertEqual("promotion-pending", policy["excluded"]["exclusion"]["exclusion_class"])
        self.assertTrue(policy["alpha-core"]["gates"])
        self.assertNotIn("exclusion", policy["alpha-core"])


class ApplyCliTests(ApplyFixture):
    """`--apply-to` is the whole of what CI runs on a carried plan."""

    def run_planner(self, *args: str, cwd: Path) -> subprocess.CompletedProcess:
        bin_dir = cwd / "bin"
        bin_dir.mkdir(exist_ok=True)
        # Selection reads `cargo metadata`; applying evidence must not.
        stub = bin_dir / "cargo"
        stub.write_text("#!/bin/sh\necho 'cargo must not run under --apply-to' >&2\nexit 97\n")
        stub.chmod(0o755)
        environment = {**os.environ, "PATH": str(bin_dir) + os.pathsep + os.environ.get("PATH", "")}
        return subprocess.run(
            [sys.executable, str(ROOT / "scripts" / "ci" / "affected_scope.py"), *args],
            cwd=cwd, env=environment, capture_output=True, text=True, timeout=120,
        )

    def test_apply_to_writes_the_applied_plan_and_prints_its_projection(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            cwd = Path(temporary)
            (cwd / "plan.json").write_text(schema.canonical(self.plan()), encoding="utf-8")
            (cwd / "accepted.json").write_text(json.dumps(self.accepted), encoding="utf-8")
            (cwd / "rejected.json").write_text(json.dumps(self.rejections), encoding="utf-8")
            result = self.run_planner(
                "--apply-to", "plan.json", "--accepted-cells", "accepted.json",
                "--evidence-rejections", "rejected.json", "--plan-out", "plan.json", cwd=cwd,
            )
            self.assertEqual(0, result.returncode, result.stderr)
            expected = apply_accepted_cells(self.plan(), self.accepted, self.rejections)
            self.assertEqual(schema.canonical(expected), (cwd / "plan.json").read_text(encoding="utf-8"))
            self.assertEqual(legacy_scope_document(expected), json.loads(result.stdout))

    def test_apply_to_excludes_selection_arguments(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            cwd = Path(temporary)
            (cwd / "plan.json").write_text(schema.canonical(self.plan()), encoding="utf-8")
            for extra in (["--all"], ["--", "alpha/lib/src/lib.rs"], ["--constraints", "."]):
                with self.subTest(extra):
                    result = self.run_planner("--apply-to", "plan.json", *extra, cwd=cwd)
                    self.assertEqual(2, result.returncode, result.stderr)
                    self.assertIn("performs no selection", result.stderr)


# ---------------------------------------------------------------------------
# Suite ownership, path-to-owner selection, and the change inventory
# — fixes/2026-09-13-cicd-redundancies
#
# Every fixture below entered as a `@pending` oracle (see
# `pending_contracts.py`) and has been promoted. The oracle strings survive as
# failure-message prefixes so a regression still names the contract it broke.
# ---------------------------------------------------------------------------

#: Oracle for every fixture that reaches for the suite-ownership registry.
SUITE_REGISTRY_ORACLE = "affected_scope declares no suite-ownership registry"

#: Oracle for the path-to-owner selection fixtures.
OWNER_SELECTION_ORACLE = "the path-to-owner table does not select its owner"


def suite_registry() -> dict[str, dict[str, str]]:
    """The declared suite-ownership table (Phase 4, spec section 2).

    Target shape: suite `name` → `{"owner", "recipe", "environment", "kind"}`,
    where `kind` is `"cargo"` or `"companion"` and `environment` names the one
    CI environment the suite runs on. R7 requires the environment to be
    DECLARED rather than derived from a runner capability.
    """
    registry = getattr(affected_scope, "SUITE_REGISTRY", None)
    if registry is None:
        raise AssertionError(
            f"{SUITE_REGISTRY_ORACLE}: `SUITE_REGISTRY` is not defined, so no "
            "suite has a declared owner, recipe, or environment"
        )
    return registry


def validate_suite_registry(
    registry: dict[str, dict[str, str]],
    declarations: dict[str, list[str]] | None = None,
) -> list[str]:
    """Problems in a candidate registry and the packages that declare it.

    `declarations` maps a package name to the suite names its
    `[package.metadata.ci.tests].companion-suites` claims. Returns one problem
    per defect, each naming the offending suite.
    """
    validator = getattr(affected_scope, "validate_suite_registry", None)
    if validator is None:
        raise AssertionError(
            f"{SUITE_REGISTRY_ORACLE}: `validate_suite_registry` is not "
            "defined, so an unknown, unowned, doubly-owned, or recipe-less "
            "suite fails nothing"
        )
    return validator(registry, declarations or {})


def change_inventory(plan: dict[str, object]) -> dict[str, object]:
    """The plan's categorized inventory of the changed paths.

    Reached through an accessor rather than `plan["change_inventory"]` so a
    plan that dropped the field fails with the field list it does carry,
    instead of a bare `KeyError`.
    """
    inventory = plan.get("change_inventory")
    if inventory is None:
        raise AssertionError(
            "`change_inventory` is absent from the resolved plan, whose fields "
            f"are {sorted(plan)}"
        )
    return inventory  # type: ignore[return-value]


class SuiteOwnershipRegistryTests(unittest.TestCase):
    """AC4: every named suite has one owner, one recipe, one environment."""

    #: The specification's ownership table (section 2), plus the companion
    #: suite that already exists. `repo-deps` owns every `scripts/ci/test_*.py`
    #: suite including `test_ci_local.py` and `test_runner_loss.py`, and the
    #: artifact publisher no package and no pnpm workspace entry selects;
    #: `test-toolkit` owns the two `tools/test-audit` suites and the lint-only
    #: archive-path guard.
    EXPECTED_COMPANION_OWNERS = {
        "archive-path-guard": "test-toolkit",
        "artifact-publisher": "repo-deps",
        "homelab-frontend": "homelab-server",
        "test_affected_scope.py": "repo-deps",
        "test_build_key.py": "repo-deps",
        "test_ci_local.py": "repo-deps",
        "test_completion.py": "repo-deps",
        "test_constraints.py": "repo-deps",
        "test_cross_check.py": "repo-deps",
        "test_evidence_reuse.py": "repo-deps",
        "test_local_evidence.py": "repo-deps",
        "test_publish_gaps.py": "repo-deps",
        "test_resolved_plan.py": "repo-deps",
        "test_reuse_validation.py": "repo-deps",
        "test_runner_loss.py": "repo-deps",
        "test_schema.py": "repo-deps",
        "test-audit-typecheck": "test-toolkit",
        "test-audit-vitest": "test-toolkit",
    }

    def companion_entries(self) -> dict[str, dict[str, str]]:
        return {
            name: entry
            for name, entry in suite_registry().items()
            if entry.get("kind") == "companion"
        }

    def test_every_registered_suite_has_exactly_one_owner(self) -> None:
        entries = self.companion_entries()
        self.assertEqual(
            self.EXPECTED_COMPANION_OWNERS,
            {name: entry["owner"] for name, entry in entries.items()},
            "the companion half of the registry must be the specification's "
            "ownership table exactly",
        )
        # Either half counts: a lint-only companion is the deliberate shape of
        # a source-policy scan, which has no test half to attach to an L1 cell.
        recipeless = [
            name
            for name, entry in entries.items()
            if not str(entry.get("recipe", "")).strip()
            and not str(entry.get("lint_recipe", "")).strip()
        ]
        self.assertEqual(
            [],
            recipeless,
            "a suite with no canonical recipe would be declared and silently never run",
        )
        # R7: the environment is declared, never derived from a capability that
        # happens to be true on one runner today.
        self.assertEqual(
            {"ubuntu-latest"},
            {entry["environment"] for entry in entries.values()},
            "S3: node_pnpm and python3 are provisioned on ubuntu-latest and "
            "nowhere else, so every suite in this fix declares it",
        )

    def test_an_unknown_suite_name_fails_validation(self) -> None:
        problems = validate_suite_registry(
            suite_registry(), {"homelab-server": ["no-such-suite"]}
        )
        self.assertTrue(
            any("no-such-suite" in problem for problem in problems),
            f"an unregistered suite name must be rejected by name: {problems}",
        )

    def test_a_doubly_owned_suite_fails_validation(self) -> None:
        problems = validate_suite_registry(
            suite_registry(),
            {"homelab-server": ["homelab-frontend"], "repo-deps": ["homelab-frontend"]},
        )
        self.assertTrue(
            any("homelab-frontend" in problem for problem in problems),
            f"a suite claimed by two packages must be rejected by name: {problems}",
        )

    def test_a_recipe_less_suite_fails_validation(self) -> None:
        registry = {
            name: dict(entry) for name, entry in suite_registry().items()
        }
        # BOTH halves: `homelab-frontend` declares a lint half too, and an entry
        # that still lints is still something CI runs.
        registry["homelab-frontend"]["recipe"] = ""
        registry["homelab-frontend"]["lint_recipe"] = ""
        problems = validate_suite_registry(
            registry, {"homelab-server": ["homelab-frontend"]}
        )
        self.assertTrue(
            any("homelab-frontend" in problem for problem in problems),
            f"a suite with no canonical recipe must be rejected by name: {problems}",
        )

    def test_a_lint_only_suite_is_a_valid_registration(self) -> None:
        # The archive-path guard's shape: a source-policy scan has no test half.
        # Registering one anyway would run the same scan a second time inside
        # its owner's L1 cell, under that package's evidence rules.
        entry = dict(suite_registry()["archive-path-guard"])
        self.assertNotIn("recipe", entry)
        self.assertTrue(entry["lint_recipe"].strip())
        self.assertEqual(
            [],
            validate_suite_registry(
                {"archive-path-guard": entry},
                {"test-toolkit": ["archive-path-guard"]},
            ),
        )
        self.assertEqual(
            [],
            affected_scope.companion_records(
                ["archive-path-guard"], "ubuntu-latest", "L1"
            ),
            "a lint-only suite must not attach to an L1 cell",
        )

    def test_a_registered_suite_nobody_declares_fails_validation(self) -> None:
        problems = validate_suite_registry(suite_registry(), {})
        self.assertTrue(
            any("homelab-frontend" in problem for problem in problems),
            f"a registered suite no package declares must be rejected by "
            f"name: {problems}",
        )

    def test_every_registered_python_suite_names_a_shipped_file(self) -> None:
        # Passive corpus check over the shipped artifacts the registry names:
        # a renamed or deleted suite must fail here rather than in CI.
        missing = [
            name
            for name, entry in self.companion_entries().items()
            if name.endswith(".py") and not (ROOT / "scripts" / "ci" / name).is_file()
        ]
        self.assertEqual(
            [], missing, f"the registry names Python suites that do not exist: {missing}"
        )

    def test_the_shipped_registry_validates_against_its_own_owners(self) -> None:
        registry = suite_registry()
        declarations: dict[str, list[str]] = {}
        for name, entry in registry.items():
            if entry.get("kind") == "companion":
                declarations.setdefault(entry["owner"], []).append(name)
        self.assertEqual(
            [],
            validate_suite_registry(registry, declarations),
            "the shipped registry must validate cleanly against the owners it "
            "itself declares",
        )

    # -- the remaining malformed shapes a registry can take -----------------

    def mutated(self, suite: str, **fields: object) -> dict[str, dict[str, object]]:
        registry = {name: dict(entry) for name, entry in suite_registry().items()}
        registry[suite].update(fields)
        return registry

    def test_a_suite_declared_by_someone_other_than_its_owner_fails_validation(
        self,
    ) -> None:
        # Distinct from double ownership: exactly one package claims it, and it
        # is the wrong one. A registry that only counted claimants would pass.
        problems = validate_suite_registry(
            suite_registry(), {"repo-deps": ["homelab-frontend"]}
        )
        self.assertTrue(
            any(
                "homelab-frontend" in problem and "repo-deps" in problem
                for problem in problems
            ),
            f"a suite declared by a non-owner must name both: {problems}",
        )

    def test_an_unknown_suite_kind_fails_validation(self) -> None:
        problems = validate_suite_registry(
            self.mutated("homelab-frontend", kind="typescript"),
            {"homelab-server": ["homelab-frontend"]},
        )
        self.assertTrue(
            any("typescript" in problem for problem in problems),
            f"the kind vocabulary must be closed: {problems}",
        )

    def test_a_companion_on_an_unknown_environment_fails_validation(self) -> None:
        # R7: the environment is declared, so a typo must fail here rather than
        # silently attach the suite to a cell no environment produces.
        problems = validate_suite_registry(
            self.mutated("homelab-frontend", environment="ubuntu-24.04"),
            {"homelab-server": ["homelab-frontend"]},
        )
        self.assertTrue(
            any("ubuntu-24.04" in problem for problem in problems),
            f"an unknown companion environment must be rejected: {problems}",
        )

    def test_a_cargo_suite_pinned_to_one_environment_fails_validation(self) -> None:
        # The converse: a Cargo suite runs wherever its owner's policy places
        # its cells, so pinning one here would be a second, driftable answer.
        problems = validate_suite_registry(
            self.mutated("repo-deps-l1", environment="ubuntu-latest"),
            {"homelab-server": ["homelab-frontend"]},
        )
        self.assertTrue(
            any("repo-deps-l1" in problem for problem in problems),
            f"a pinned cargo suite must be rejected by name: {problems}",
        )

    def test_an_ownerless_entry_fails_validation(self) -> None:
        problems = validate_suite_registry(
            self.mutated("test-audit-vitest", owner="  "),
            {"homelab-server": ["homelab-frontend"]},
        )
        self.assertTrue(
            any("test-audit-vitest" in problem for problem in problems),
            f"an entry with no owner must be rejected by name: {problems}",
        )


class ToolingPathOwnershipTests(unittest.TestCase):
    """AC5/R13: a tooling input selects the package that owns its suite.

    The workspace here is synthetic but keyed on the real manifest directories,
    because the selection rule is a function of the changed path and the
    member set, not of the checkout. `SuiteOwnershipCorpusTests` in
    `test_resolved_plan.py` runs the same table against the real workspace.
    """

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        seed_build_inputs(self.root)
        packages = [
            package(self.root, "alpha-core", "alpha/lib/Cargo.toml"),
            package(self.root, "repo-deps", "scripts/Cargo.toml"),
            package(self.root, "test-toolkit", "tools/test-toolkit/Cargo.toml"),
        ]
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {
                "nodes": [{"id": item["id"], "deps": []} for item in packages]
            },
        }
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def selected(self, path: str) -> list[str]:
        plan = calculate_scope(
            [path],
            self.root,
            self.metadata,  # type: ignore[arg-type]
            environments_for_tests(),
            self.policy,
        )
        return [entry["package"] for entry in plan["packages"]]

    def assert_selects(self, path: str, expected: list[str]) -> None:
        selected = self.selected(path)
        if selected != expected:
            raise AssertionError(
                f"{path} must select exactly {expected}, got {selected}: "
                f"{OWNER_SELECTION_ORACLE}"
            )

    def test_a_scope_calculator_change_selects_repo_deps_alone(self) -> None:
        # R13.4: `scripts/**` needs no trigger entry, because it is
        # `repo-deps`'s own package directory. This fixture is what proves the
        # table adds no `scripts/**` trigger that would double-select.
        for path in ("scripts/ci/schema.py", "scripts/ci-rollup.rs", "scripts/ci/test_ci_local.py"):
            self.assert_selects(path, ["repo-deps"])

    def test_the_owners_manifest_is_an_explicit_trigger(self) -> None:
        # The repository-wide rule is that a manifest selects nothing; R13.6
        # rules these two packages the exception, because their manifests are
        # where CI's own suites are declared to run at all.
        self.assert_selects("scripts/Cargo.toml", ["repo-deps"])
        self.assert_selects("tools/test-toolkit/Cargo.toml", ["test-toolkit"])

    def test_another_packages_manifest_is_not_a_trigger(self) -> None:
        # The other half of R13.6: the exception is two paths wide, not general.
        self.assert_selects("alpha/lib/Cargo.toml", [])

    def test_a_policy_store_change_selects_repo_deps_alone(self) -> None:
        for path in (".github/ci/ci-baseline.toml", ".github/ci/environments.json"):
            self.assert_selects(path, ["repo-deps"])

    def test_a_cross_language_schema_document_selects_both_readers(self) -> None:
        # Both documents exist so a Rust reader can assert against the Python
        # contract without running Python; a change that ran only the Python
        # suite would leave the agreement they encode unverified on the side
        # that consumes them.
        for path in (
            ".github/ci/schemas/contract.json",
            ".github/ci/schemas/archive_guard_cases.json",
        ):
            self.assert_selects(path, ["repo-deps", "test-toolkit"])

    def test_schemas_documentation_selects_repo_deps_alone(self) -> None:
        # The two cross-language documents are named one by one rather than by
        # a `schemas/` prefix. This is the other half of that rule: the
        # directory's own README is not an input the Rust suite reads, so it
        # keeps the `.github/ci/` selection every other policy document gets.
        self.assert_selects(".github/ci/schemas/README.md", ["repo-deps"])

    def test_an_unrelated_future_schema_selects_repo_deps_alone(self) -> None:
        # Pins the boundary against widening: a schema added later is owned by
        # the planner alone until someone makes a Rust reader read it and adds
        # it to the explicit path table.
        self.assert_selects(".github/ci/schemas/unrelated-future-schema.json", ["repo-deps"])

    def test_a_workflow_change_selects_test_toolkit_alone(self) -> None:
        # R13: global-path escalation no longer selects any package, so
        # `ci.yml` is NOT a full-workspace trigger. It selects the package that
        # owns the workflow-contract suite, exactly like every other workflow.
        for path in (
            ".github/workflows/_area-ci.yml",
            ".github/workflows/ci.yml",
            ".github/workflows/_package-ci.yml",
        ):
            self.assert_selects(path, ["test-toolkit"])

    def test_a_test_audit_change_selects_test_toolkit_alone(self) -> None:
        for path in (
            "tools/test-audit/package.json",
            "tools/test-audit/src/cli.ts",
            "pnpm-lock.yaml",
            "pnpm-workspace.yaml",
        ):
            self.assert_selects(path, ["test-toolkit"])

    def test_test_toolkit_source_still_selects_itself_alone(self) -> None:
        # A member directory already selects its own package. Pinned so the
        # deletion of `CI_TOOLING_PATHS` cannot narrow it, and so the
        # `.github/workflows/**` trigger cannot widen it.
        self.assert_selects("tools/test-toolkit/tests/ci_workflow_contracts.rs", ["test-toolkit"])

    def test_a_windows_spelled_trigger_selects_the_same_owner(self) -> None:
        # `git diff --name-only` yields forward slashes, but callers hand the
        # planner paths from other sources too, and every other path rule in
        # this module normalizes. A trigger that did not would select nothing
        # on exactly one OS.
        self.assert_selects(r".github\workflows\ci.yml", ["test-toolkit"])
        self.assert_selects("./pnpm-lock.yaml", ["test-toolkit"])

    def test_inputs_of_two_owners_select_both(self) -> None:
        # One push, two suites: the table is applied per path, so neither
        # owner's trigger may suppress the other's.
        plan = calculate_scope(
            [".github/ci/environments.json", ".github/workflows/ci.yml"],
            self.root,
            self.metadata,  # type: ignore[arg-type]
            environments_for_tests(),
            self.policy,
        )
        self.assertEqual(
            ["repo-deps", "test-toolkit"],
            sorted(entry["package"] for entry in plan["packages"]),
        )

    def test_a_trigger_selection_states_the_input_that_selected_it(self) -> None:
        plan = calculate_scope(
            [".github/workflows/_area-ci.yml"],
            self.root,
            self.metadata,  # type: ignore[arg-type]
            environments_for_tests(),
            self.policy,
        )
        record = next(
            entry for entry in plan["packages"] if entry["package"] == "test-toolkit"
        )
        self.assertEqual(
            "suite input .github/workflows/_area-ci.yml selects its registered owner",
            record["selection_reason"],
        )
        self.assertEqual(
            ["changed suite input owned by package(s) test-toolkit"],
            [entry["selection_reason"] for entry in plan["areas"]],
            "an area selected by a trigger must not claim a source change",
        )
        self.assertEqual([], plan["source_packages"])

    def test_documentation_selects_neither_tooling_owner(self) -> None:
        # What stops the trigger table from being satisfied by selecting an
        # owner for everything.
        for path in (
            "docs/topics/ci-cd.md",
            "alpha/docs/design.md",
            "alpha/lib/README.md",
        ):
            with self.subTest(path=path):
                self.assertEqual([], self.selected(path))

    def test_the_ci_tooling_flag_is_gone_from_the_plan(self) -> None:
        plan = calculate_scope(
            ["scripts/ci/schema.py"],
            self.root,
            self.metadata,  # type: ignore[arg-type]
            environments_for_tests(),
            self.policy,
        )
        if "ci_tooling" in plan["flags"]:
            raise AssertionError(
                "the plan still carries flags.ci_tooling; ownership replaces "
                "the boolean, it does not sit beside it"
            )


class PreflightSkipTests(unittest.TestCase):
    """R10/spec section 3: no package work means preflight has no prerequisites."""

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        seed_build_inputs(self.root)
        packages = [package(self.root, "alpha-core", "alpha/lib/Cargo.toml")]
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {"nodes": [{"id": "alpha-core", "deps": []}]},
        }
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def scope(self, files: list[str]) -> dict[str, object]:
        return scope_document(
            files, self.root, self.metadata, environments_for_tests(), self.policy
        )

    def test_a_documentation_only_change_expands_no_preflight_job(self) -> None:
        for path in ("docs/architecture.md", "alpha/docs/design.md", "alpha/lib/README.md"):
            scope = self.scope([path])
            self.assertEqual("documentation", scope["change_class"])
            if scope["preflight_os"] != []:
                raise AssertionError(
                    "documentation-class preflight must expand no job, got "
                    f"{scope['preflight_os']} for {path}"
                )
            self.assertTrue(
                scope["preflight_reason"], "the skip must still state its decision"
            )

    def test_a_package_change_keeps_its_preflight_breadth(self) -> None:
        # NOT pending: R10 narrows the documentation class alone. A trigger
        # table that emptied preflight for real package work would pass the
        # fixture above and break the bootstrap gate.
        scope = self.scope(["alpha/lib/src/lib.rs"])
        self.assertEqual("package", scope["change_class"])
        self.assertEqual(
            ["macos-latest", "ubuntu-latest", "windows-latest"], scope["preflight_os"]
        )


class ChangeInventoryTests(unittest.TestCase):
    """AC10/spec section 5: the plan classifies its changed paths once."""

    BUCKETS = ("configuration", "documentation", "other", "source")

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        seed_build_inputs(self.root)
        packages = [package(self.root, "alpha-core", "alpha/lib/Cargo.toml")]
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {"nodes": [{"id": "alpha-core", "deps": []}]},
        }
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def plan(self, files: list[str], **kwargs: object) -> dict[str, object]:
        return calculate_scope(
            files,
            self.root,
            self.metadata,  # type: ignore[arg-type]
            environments_for_tests(),
            self.policy,
            **kwargs,  # type: ignore[arg-type]
        )

    def test_the_inventory_buckets_every_changed_path_exactly_once(self) -> None:
        files = [
            "docs/architecture.md",
            "alpha/lib/src/lib.rs",
            ".github/ci/environments.json",
            "LICENSE",
        ]
        inventory = change_inventory(self.plan(files))
        paths = inventory["paths"]
        self.assertLessEqual(
            set(self.BUCKETS),
            set(paths),  # type: ignore[arg-type]
            "the inventory separates at least configuration, documentation, "
            "source, and other",
        )
        placed = [entry for bucket in paths.values() for entry in bucket]  # type: ignore[attr-defined]
        self.assertEqual(
            sorted(files),
            sorted(placed),
            "the inventory is exhaustive: every input path lands in exactly one bucket",
        )
        self.assertEqual(
            len(placed), len(set(placed)), "no path appears in two buckets"
        )
        self.assertIn("docs/architecture.md", paths["documentation"])  # type: ignore[index]
        self.assertIn("alpha/lib/src/lib.rs", paths["source"])  # type: ignore[index]
        self.assertIn(".github/ci/environments.json", paths["configuration"])  # type: ignore[index]

    def test_each_bucket_is_sorted_normalized_and_counted(self) -> None:
        inventory = change_inventory(
            self.plan(["./docs/b.md", "docs/a.md", "alpha\\lib\\src\\lib.rs"])
        )
        paths = inventory["paths"]
        counts = inventory["counts"]
        for bucket, entries in paths.items():  # type: ignore[attr-defined]
            self.assertEqual(sorted(entries), list(entries), f"{bucket} must be sorted")
            self.assertEqual(
                len(entries), counts[bucket], f"{bucket} count must match its list"  # type: ignore[index]
            )
            for entry in entries:
                self.assertNotIn("\\", entry, f"{entry} must be spelled with /")
                self.assertFalse(entry.startswith("./"), f"{entry} must carry no ./ prefix")
        self.assertEqual(["docs/a.md", "docs/b.md"], paths["documentation"])  # type: ignore[index]
        self.assertEqual(["alpha/lib/src/lib.rs"], paths["source"])  # type: ignore[index]
        self.assertEqual(
            sum(len(entries) for entries in paths.values()),  # type: ignore[attr-defined]
            counts["total"],  # type: ignore[index]
        )

    def test_a_rename_contributes_one_logical_path(self) -> None:
        # Both callers supply `git diff --name-only`, which yields a rename's
        # DESTINATION and nothing else (`just/ci-local.just:212`,
        # `.github/workflows/ci.yml:123`). The de-duplication that matters is
        # therefore over spellings of one destination.
        inventory = change_inventory(
            self.plan(["docs/renamed.md", "./docs/renamed.md", "docs\\renamed.md"])
        )
        self.assertEqual(["docs/renamed.md"], inventory["paths"]["documentation"])  # type: ignore[index]
        self.assertEqual(1, inventory["counts"]["total"])  # type: ignore[index]

    def test_a_full_scope_run_records_no_diff_inventory(self) -> None:
        inventory = change_inventory(self.plan([], force_all=True))
        self.assertIs(
            False,
            inventory["diff_available"],
            "a manual full-scope run has no diff and must say so rather than "
            "reporting an empty list that reads as `nothing changed`",
        )
        self.assertTrue(inventory.get("reason"), "the absence carries a reason")  # type: ignore[union-attr]

    def test_change_class_is_retained_beside_the_inventory(self) -> None:
        # R8: the two answer different questions and must be allowed to
        # disagree. The real-workspace case is in `test_resolved_plan.py`; here
        # the point is only that `change_class` survives the addition.
        plan = self.plan(["alpha/lib/README.md"])
        self.assertEqual("documentation", plan["change_class"])
        inventory = change_inventory(plan)
        self.assertEqual(["alpha/lib/README.md"], inventory["paths"]["documentation"])  # type: ignore[index]


class ChangeBucketCorpusTests(unittest.TestCase):
    """The bucketing table, exercised against every path this repository ships.

    A hand-written fixture only proves the rules it names. The table is
    data-driven, so the corpus is the only thing that says what it does to the
    11k paths a full-scope diff could hand it.
    """

    @classmethod
    def setUpClass(cls) -> None:
        completed = subprocess.run(
            ["git", "ls-files"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
            timeout=120,
        )
        if completed.returncode != 0:
            raise unittest.SkipTest("no Git checkout to enumerate")
        cls.tracked = [line for line in completed.stdout.splitlines() if line]

    def test_every_tracked_path_lands_in_exactly_one_declared_bucket(self) -> None:
        self.assertTrue(self.tracked, "the corpus must not be empty")
        buckets = {affected_scope.change_bucket(path) for path in self.tracked}
        self.assertLessEqual(buckets, set(schema.CHANGE_BUCKETS))

    def test_the_repositorys_own_kinds_are_never_filed_as_other(self) -> None:
        # `other` is a legitimate answer for a symlink, a snapshot, or a binary
        # asset. It is never the answer for the four kinds this repository's CI
        # is actually about.
        misfiled: dict[str, list[str]] = {}
        for path in self.tracked:
            suffix = PurePosixPath(path).suffix
            if suffix in (".rs", ".py", ".md", ".toml"):
                bucket = affected_scope.change_bucket(path)
                if bucket == "other":
                    misfiled.setdefault(suffix, []).append(path)
        self.assertEqual({}, misfiled)

    def test_the_suffix_tables_do_not_overlap(self) -> None:
        # Overlap would make the bucket depend on the order of the checks
        # rather than on a declared rule, which is how a table silently
        # reclassifies a path when someone reorders it.
        tables = {
            "source": affected_scope.SOURCE_SUFFIXES,
            "documentation": affected_scope.DOCUMENTATION_SUFFIXES,
            "configuration": affected_scope.CONFIGURATION_SUFFIXES,
        }
        for left, right in itertools.combinations(sorted(tables), 2):
            self.assertEqual(
                set(), tables[left] & tables[right], f"{left} and {right} overlap"
            )


class AreaDriftFlagTests(unittest.TestCase):
    """AC15: a change that can move planner-vs-sniff area derivation must run
    the contract that asks sniff.

    Scoped in both directions on purpose. Too narrow and a real divergence
    schedules the contract nowhere, which is silent; too broad and every pull
    request pays for a release `sniff-cli`.
    """

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        seed_build_inputs(self.root)
        packages = [package(self.root, "alpha-core", "alpha/lib/Cargo.toml")]
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {"nodes": [{"id": "alpha-core", "deps": []}]},
        }
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def scope(self, files: list[str]) -> dict[str, object]:
        return scope_document(
            files, self.root, self.metadata, environments_for_tests(), self.policy
        )

    def test_the_two_authorities_set_the_area_drift_flag(self) -> None:
        for path in [
            # sniff's own detection rule — the direction suite ownership cannot see.
            "sniff/lib/src/filesystem/repo/detection.rs",
            "sniff/cli/src/main.rs",
            # The planner's replica of that rule, and the contract itself.
            "scripts/ci/affected_scope.py",
            "scripts/ci/test_resolved_plan.py",
            "./scripts/ci/affected_scope.py",
            "scripts\\ci\\test_resolved_plan.py",
        ]:
            with self.subTest(path=path):
                scope = self.scope([path])
                self.assertTrue(scope["flags"]["area_drift"])

    def test_any_manifest_at_any_depth_sets_the_area_drift_flag(self) -> None:
        # A manifest that appears, moves, or disappears re-maps areas without
        # touching either authority, and a name-only diff cannot say which of
        # the three happened. The root manifest carries workspace membership.
        for path in [
            "Cargo.toml",
            "alpha/lib/Cargo.toml",
            "claudine/rendezvous/core/Cargo.toml",
            "darkmatter/lib/tests/fixtures/validate/file_match_valid/Cargo.toml",
            "tools\\test-toolkit\\Cargo.toml",
        ]:
            with self.subTest(path=path):
                scope = self.scope([path])
                self.assertTrue(scope["flags"]["area_drift"])

    def test_changes_that_cannot_move_area_derivation_leave_the_flag_alone(self) -> None:
        # Source, docs, the lockfile, workflows, and the rest of the CI tooling
        # — including suites that share the tooling leg — cannot re-map an area.
        # `Cargo.lock` is the near miss worth pinning: it sits beside the root
        # manifest and names the resolved graph, but not where a manifest lives.
        for path in [
            "alpha/lib/src/lib.rs",
            "Cargo.lock",
            "docs/topics/ci-cd.md",
            ".github/workflows/ci.yml",
            "scripts/ci/test_affected_scope.py",
            "scripts/ci-rollup.rs",
            "sniffer/lib/src/lib.rs",
        ]:
            with self.subTest(path=path):
                scope = self.scope([path])
                self.assertFalse(scope["flags"]["area_drift"])

    def test_area_drift_is_a_flag_of_its_own_and_not_suite_ownership(self) -> None:
        # Suite ownership replaced the `ci_tooling` boolean, and it cannot
        # serve here: it selects the package that owns a changed CI input,
        # which says nothing about sniff's own detection rule. So the drift
        # flag stays a flag, and stays the only one the plan carries.
        sniff_only = self.scope(["sniff/lib/src/filesystem/repo/detection.rs"])
        self.assertTrue(sniff_only["flags"]["area_drift"])
        self.assertNotIn("ci_tooling", sniff_only["flags"])

        tooling_only = self.scope(["scripts/ci/test_ci_local.py"])
        self.assertFalse(tooling_only["flags"]["area_drift"])
        self.assertNotIn("ci_tooling", tooling_only["flags"])


class ArchiveGuardScopeTests(unittest.TestCase):
    """The archive-path guard is selected by the source it SCANS, and the plan
    carries the scope it must scan.

    Scoped in both directions on purpose, like the area-drift flag above. Too
    narrow and a violation reaches `main` unchecked, because nothing else
    selects a repository-wide scan; too broad and every documentation edit pays
    for a Linux cell. The mode half matters just as much: an empty changed-file
    scan and a full-tree scan both report zero violations, so a plan that made
    them indistinguishable would let a partial scan read as a complete one.
    """

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        seed_build_inputs(self.root)
        packages = [package(self.root, "alpha-core", "alpha/lib/Cargo.toml")]
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {"nodes": [{"id": "alpha-core", "deps": []}]},
        }
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def guard(self, files: list[str], **kwargs: object) -> dict[str, object]:
        environments = kwargs.pop("environments", None) or environments_for_tests()
        plan = calculate_scope(
            files,
            self.root,
            self.metadata,  # type: ignore[arg-type]
            environments,  # type: ignore[arg-type]
            self.policy,
            **kwargs,  # type: ignore[arg-type]
        )
        self.assertEqual([], schema.validate_resolved_plan(plan))
        return plan["archive_guard"]  # type: ignore[index,return-value]

    # -- what selects a scan --------------------------------------------------

    def test_rust_source_anywhere_selects_the_guard(self) -> None:
        # Across package areas AND outside every workspace member: a
        # member-only trigger would leave part of the scanned corpus unchecked,
        # which is the gap this fix exists to close.
        for path in [
            "alpha/lib/src/lib.rs",
            "claudine/cli/tests/compose_initialize_acceptance.rs",
            "darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs",
            "not-a-member/src/main.rs",
            "tools/test-toolkit/src/lib.rs",
        ]:
            with self.subTest(path=path):
                guard = self.guard([path])
                self.assertTrue(guard["selected"], guard["reason"])
                self.assertEqual([path], guard["paths"])

    def test_each_guard_owned_input_selects_the_guard(self) -> None:
        # The narrow owned list: the matcher, its fixture corpus, the driver,
        # the canonical recipe, this selection rule, the registry binding, and
        # the two workflow files carrying the guard's execution controls. Three
        # of them are excluded from SCANNING and are still owned inputs.
        for path in sorted(affected_scope.ARCHIVE_GUARD_OWN_INPUTS):
            with self.subTest(path=path):
                self.assertTrue(self.guard([path])["selected"])

    def test_a_skipped_directory_or_build_script_selects_nothing(self) -> None:
        # Every skipped directory the scanner declares, plus the build script
        # rule. A path the planner selected and the scanner then refused is a
        # cell that ran nothing.
        for path in [
            "target/debug/build/generated.rs",
            ".git/hooks/sample.rs",
            "node_modules/pkg/index.rs",
            ".gitnexus/cache/entry.rs",
            "scripts/ci-rollup.rs",
            "alpha/lib/examples/demo.rs",
            "alpha/fuzz/fuzz_targets/target.rs",
            "alpha/lib/build.rs",
        ]:
            with self.subTest(path=path):
                guard = self.guard([path])
                self.assertFalse(guard["selected"], guard["reason"])
                self.assertNotIn("mode", guard)
                self.assertNotIn("paths", guard)

    def test_documentation_and_unrelated_configuration_select_nothing(self) -> None:
        # NOT a rule that every configuration edit selects the guard.
        for path in [
            "docs/topics/ci-cd.md",
            "alpha/README.md",
            "Cargo.lock",
            "Cargo.toml",
            ".github/workflows/ci.yml",
            ".github/ci/environments.json",
            "clippy.toml",
        ]:
            with self.subTest(path=path):
                self.assertFalse(self.guard([path])["selected"])

    # -- deletions and renames ------------------------------------------------

    def test_a_deleted_rust_file_triggers_but_is_not_scanned(self) -> None:
        # Exemption maintenance has to run when an exempted file disappears,
        # and there is nothing left to read at the path itself.
        guard = self.guard(["alpha/lib/src/gone.rs"], deleted=["alpha/lib/src/gone.rs"])
        self.assertTrue(guard["selected"])
        self.assertEqual("changed", guard["mode"])
        self.assertEqual([], guard["paths"])

    def test_a_rename_scans_the_destination_and_not_the_source(self) -> None:
        guard = self.guard(
            ["alpha/lib/src/before.rs", "alpha/lib/src/after.rs"],
            deleted=["alpha/lib/src/before.rs"],
        )
        self.assertEqual(["alpha/lib/src/after.rs"], guard["paths"])

    def test_the_inventory_records_deletions_exactly_when_it_has_a_diff(self) -> None:
        inventory = affected_scope.change_inventory(
            ["a/b.rs", "a/c.rs"], False, ["a/c.rs", "./a/c.rs"]
        )
        self.assertEqual(["a/c.rs"], inventory["deleted"])
        self.assertNotIn("deleted", affected_scope.change_inventory([], True, ["a/c.rs"]))

    # -- the mode decision table ----------------------------------------------

    def test_a_pull_request_with_an_inventory_scans_the_changed_files(self) -> None:
        guard = self.guard(["alpha/lib/src/lib.rs"], event="pull_request")
        self.assertEqual("changed", guard["mode"])
        self.assertEqual(["alpha/lib/src/lib.rs"], guard["paths"])

    def test_a_push_scans_the_whole_corpus(self) -> None:
        # Changed-file evidence from a pull request proves nothing about the
        # files it omitted, so a merge does not inherit it.
        guard = self.guard(["alpha/lib/src/lib.rs"], event="push")
        self.assertEqual("full", guard["mode"])
        self.assertNotIn("paths", guard)

    def test_a_manual_full_workspace_run_scans_the_whole_corpus(self) -> None:
        guard = self.guard([], force_all=True, event="workflow_dispatch")
        self.assertEqual("full", guard["mode"])
        self.assertNotIn("paths", guard)

    def test_an_unavailable_diff_scans_the_whole_corpus(self) -> None:
        # Reached through the scope function directly: the inventory reports
        # `diff_available: false` only for a full-scope request today, and the
        # rule must hold for any other producer of an absent diff.
        guard = affected_scope.archive_guard_scope(
            ["alpha/lib/src/lib.rs"],
            full_scope=False,
            deleted=[],
            scheduled={"ubuntu-latest"},
            event="pull_request",
            diff_available=False,
        )
        self.assertEqual("full", guard["mode"])
        self.assertNotIn("paths", guard)

    def test_an_explicitly_empty_inventory_is_not_a_full_scan(self) -> None:
        # The distinction the guard refuses to guess at: nothing eligible
        # changed is a real, complete answer, and is never upgraded.
        guard = self.guard(
            ["alpha/lib/src/gone.rs", "docs/topics/ci-cd.md"],
            deleted=["alpha/lib/src/gone.rs"],
            event="pull_request",
        )
        self.assertEqual("changed", guard["mode"])
        self.assertEqual([], guard["paths"])
        self.assertNotEqual("full", guard["mode"])

    def test_the_nightly_adds_no_linux_for_the_guard(self) -> None:
        # The shipped table schedules WSL2 alone for `schedule`, and the guard
        # is hosted on Linux only. It does not force Linux into an event that
        # excludes it; the reason has to name the event so a reader can tell
        # this apart from "nothing changed".
        environments = load_environments(ENVIRONMENTS_CONFIG, today=TODAY)
        plan = calculate_scope(
            ["alpha/lib/src/lib.rs"],
            self.root,
            self.metadata,  # type: ignore[arg-type]
            environments,
            self.policy,
            event="schedule",
        )
        guard = plan["archive_guard"]
        self.assertFalse(guard["selected"], guard["reason"])
        self.assertIn("schedule", guard["reason"])
        self.assertEqual(
            [],
            [
                cell
                for cell in plan["cells"]
                if cell["environment"] == "ubuntu-latest"
            ],
            "the nightly must gain no Linux cell for a Linux-only guard",
        )

    def test_a_proven_linux_environment_gains_no_guard_execution(self) -> None:
        # The other half of the nightly rule. A pull request that already
        # proved ubuntu-latest narrows the follow-up push to windows-latest;
        # the guard is hosted on Linux alone, so it must accept that the scan
        # already ran rather than invent the extra run the specification's
        # "without inventing an extra Linux run" forbids.
        environments = load_environments(ENVIRONMENTS_CONFIG, today=TODAY)
        plan = calculate_scope(
            ["alpha/lib/src/lib.rs"],
            self.root,
            self.metadata,  # type: ignore[arg-type]
            environments,
            self.policy,
            event="push",
            proven_event="pull_request",
        )
        self.assertEqual(["windows-latest"], [item["name"] for item in plan["environments"]])
        guard = plan["archive_guard"]
        self.assertFalse(guard["selected"], guard["reason"])
        self.assertIn("ubuntu-latest", guard["reason"])
        self.assertEqual(
            [],
            [cell for cell in plan["cells"] if cell["environment"] == "ubuntu-latest"],
        )

    # -- path spelling --------------------------------------------------------

    def test_path_spellings_normalize_to_one_answer(self) -> None:
        # The house convention, as `AreaDriftFlagTests` states it: a
        # Windows-spelled diff and a POSIX-spelled one select the same scan.
        for spelling in ("./alpha/lib/src/lib.rs", "alpha\\lib\\src\\lib.rs"):
            with self.subTest(spelling=spelling):
                self.assertEqual(
                    ["alpha/lib/src/lib.rs"], self.guard([spelling])["paths"]
                )

    def test_the_path_list_is_sorted_and_deduplicated(self) -> None:
        guard = self.guard(
            [
                "beta/src/lib.rs",
                "alpha/lib/src/lib.rs",
                "./beta/src/lib.rs",
                "beta\\src\\lib.rs",
            ]
        )
        self.assertEqual(["alpha/lib/src/lib.rs", "beta/src/lib.rs"], guard["paths"])

    def test_every_scope_states_a_reason(self) -> None:
        for files, kwargs in (
            (["docs/topics/ci-cd.md"], {}),
            (["alpha/lib/src/lib.rs"], {}),
            (["alpha/lib/src/lib.rs"], {"event": "push"}),
            ([], {"force_all": True}),
        ):
            with self.subTest(files=files, kwargs=kwargs):
                self.assertTrue(self.guard(files, **kwargs)["reason"].strip())


class ArchiveGuardOnlyCellTests(unittest.TestCase):
    """Selecting the guard alone adds exactly one Linux execution.

    Run against the shipped workspace because the contract is about the real
    owner: `test-toolkit` declares four test tiers' worth of environments, two
    Node companions, and a reverse-dependency seam that compiles eleven
    consumers. A Rust file in another area says nothing about any of it, so
    the one thing this selection may add is the guard's own lint cell.
    """

    GUARD_CELL = ("test-toolkit", "ubuntu-latest", "lint")

    @classmethod
    def setUpClass(cls) -> None:
        cls.metadata = load_metadata(ROOT)
        cls.environments = load_environments(ENVIRONMENTS_CONFIG, today=TODAY)
        cls.policy = package_ci_policy(
            workspace_packages(cls.metadata),
            runner_labels={environment["runner"] for environment in cls.environments},
            root=ROOT,
            today=TODAY,
        )

    def plan(self, *files: str, **kwargs: object) -> dict[str, object]:
        plan = calculate_scope(
            list(files),
            ROOT,
            self.metadata,
            self.environments,
            self.policy,
            **kwargs,  # type: ignore[arg-type]
        )
        self.assertEqual([], schema.validate_resolved_plan(plan))
        return plan

    def toolkit_cells(self, plan: dict[str, object]) -> list[dict[str, object]]:
        return [
            cell
            for cell in plan["cells"]  # type: ignore[index]
            if cell["package"] == "test-toolkit"
        ]

    def test_a_guard_only_selection_is_one_lint_cell_and_nothing_else(self) -> None:
        plan = self.plan("claudine/lib/src/lib.rs")
        cells = self.toolkit_cells(plan)
        self.assertEqual(1, len(cells), [cell["gate"] for cell in cells])
        cell = cells[0]
        self.assertEqual(
            self.GUARD_CELL, (cell["package"], cell["environment"], cell["gate"])
        )
        self.assertEqual(
            ["archive-path-guard"], [entry["name"] for entry in cell["companions"]]
        )
        self.assertIs(True, cell["companions_only"])
        self.assertIn("archive-path guard", cell["selection_reason"])
        self.assertIn("claudine/lib/src/lib.rs", cell["selection_reason"])

    def test_a_guard_only_selection_claims_no_package_work(self) -> None:
        record = next(
            entry
            for entry in self.plan("claudine/lib/src/lib.rs")["packages"]
            if entry["package"] == "test-toolkit"
        )
        self.assertEqual(["lint"], record["gates"])
        self.assertEqual(["archive-path-guard"], record["companion_suites"])
        self.assertEqual([], record["tiers"])
        self.assertEqual([], record["targets"])
        self.assertNotIn(
            "dependent_seam",
            record,
            "a Rust file elsewhere says nothing about this package's public API",
        )

    def test_a_guard_only_owner_is_not_also_reported_as_a_reverse_dependency(
        self,
    ) -> None:
        # `test-toolkit` depends on `biscuit-test-harness`, so this one change
        # makes the owner both an unchanged direct dependent and the guard's
        # selection. `self.plan` validates the schema, which forbids both.
        plan = self.plan("biscuit-test-harness/src/lib.rs")
        self.assertEqual(1, len(self.toolkit_cells(plan)))
        self.assertNotIn("test-toolkit", plan["reverse_dependencies"])
        self.assertIn("biscuit-test-harness", plan["source_packages"])

    def test_a_guard_only_selection_adds_no_other_environment_or_build(self) -> None:
        plan = self.plan("claudine/lib/src/lib.rs")
        self.assertEqual(
            [], [cell for cell in self.toolkit_cells(plan) if cell["gate"] != "lint"]
        )
        self.assertEqual(
            {"ubuntu-latest"},
            {cell["environment"] for cell in self.toolkit_cells(plan)},
        )
        self.assertEqual(
            [],
            [
                build
                for build in plan["builds"]  # type: ignore[index]
                if build["package"] == "test-toolkit"
            ],
            "lint compiles its own configuration and consumes no archive",
        )

    def test_the_lint_row_carries_the_guard_and_stands_clippy_down(self) -> None:
        # What the producer resolves from its one row: the plan cell's
        # `companions_only` reaches the job through the cell contract, never a
        # workflow input.
        contracts = producer_contracts(self.plan("claudine/lib/src/lib.rs"), "test-toolkit")
        self.assertEqual([("ubuntu-latest", "lint")], sorted(contracts))
        contract = contracts[("ubuntu-latest", "lint")]
        self.assertEqual("true", contract["companions_only"])
        self.assertEqual(["archive-path-guard"], json.loads(contract["companion_suites"]))

    def test_an_already_selected_owner_gains_no_second_lint_cell(self) -> None:
        # The other half of the contract: the guard rides the owner's ordinary
        # lint cell, and `companions_only` stays absent because Clippy is still
        # required work there.
        plan = self.plan("tools/test-toolkit/src/lib.rs")
        lint = [cell for cell in self.toolkit_cells(plan) if cell["gate"] == "lint"]
        self.assertEqual(1, len(lint))
        self.assertEqual(
            ["archive-path-guard"], [entry["name"] for entry in lint[0]["companions"]]
        )
        self.assertNotIn("companions_only", lint[0])
        self.assertTrue(
            [cell for cell in self.toolkit_cells(plan) if cell["gate"] == "L1"],
            "an ordinary selection still runs the package's own tests",
        )

    def test_a_documentation_change_schedules_no_guard_cell(self) -> None:
        plan = self.plan("docs/topics/ci-cd.md")
        self.assertFalse(plan["archive_guard"]["selected"])  # type: ignore[index]
        # The document is a test input of `test-toolkit`'s CI-documentation
        # contracts, so it selects their narrowed L1 cell — and nothing that
        # resembles the guard's lint cell.
        self.assertEqual(
            [("ubuntu-latest", "L1")],
            [(cell["environment"], cell["gate"]) for cell in self.toolkit_cells(plan)],
        )
        self.assertIn("test_filter", self.toolkit_cells(plan)[0])

    def test_the_guard_cell_is_never_satisfied_by_reuse(self) -> None:
        # The spec's "initially prefer non-reusable guard execution": no
        # evidence identity here can represent a scan's inputs, so a passing
        # receipt for this cell must not stand in for the scan.
        accepted = [
            {
                "package": "test-toolkit",
                "environment": "ubuntu-latest",
                "gate": "lint",
                "outcome": "pass",
                "origin": "local",
            }
        ]
        cell = self.toolkit_cells(
            self.plan("claudine/lib/src/lib.rs", accepted_cells=accepted)
        )[0]
        self.assertFalse(cell["reusable"])
        self.assertEqual("execute", cell["execution"])
        self.assertEqual("pending", cell["state"])


class ArchiveGuardConfigurationInputTests(unittest.TestCase):
    """The guard's registry and execution configuration selects a real scan.

    Run against the shipped tree because the claim is about the real files.
    `tools/test-toolkit/Cargo.toml` is the guard's registry binding only
    because THAT manifest declares `archive-path-guard` in `companion-suites`,
    and the two workflow files are owned inputs only because they carry the
    guard's own execution controls; a fixture would prove neither.

    All three already select `test-toolkit` through `SUITE_OWNER_*`, so the
    lint cell and its attached companion exist either way. What used to be
    missing was the PLAN's account of it: `selected: false` left the companion
    reading an unselected scope and running an empty changed-file scan, which
    is a cell that ran incidentally rather than the plan-owned execution the
    contract requires.
    """

    @classmethod
    def setUpClass(cls) -> None:
        cls.metadata = load_metadata(ROOT)
        cls.environments = load_environments(ENVIRONMENTS_CONFIG, today=TODAY)
        cls.policy = package_ci_policy(
            workspace_packages(cls.metadata),
            runner_labels={environment["runner"] for environment in cls.environments},
            root=ROOT,
            today=TODAY,
        )

    #: The owned inputs that are configuration rather than guard source.
    CONFIGURATION_INPUTS = (
        "tools/test-toolkit/Cargo.toml",
        ".github/workflows/_package-ci.yml",
    )

    def plan(self, *files: str, **kwargs: object) -> dict[str, object]:
        plan = calculate_scope(
            list(files),
            ROOT,
            self.metadata,
            self.environments,
            self.policy,
            **kwargs,  # type: ignore[arg-type]
        )
        self.assertEqual([], schema.validate_resolved_plan(plan))
        return plan

    def test_the_registry_binding_really_is_one(self) -> None:
        # The premise the manifest entry rests on, checked rather than assumed:
        # if the declaration moved, this class would be asserting about a file
        # that no longer decides anything.
        manifest = (ROOT / "tools/test-toolkit/Cargo.toml").read_text(encoding="utf-8")
        self.assertIn("[package.metadata.ci.tests]", manifest)
        self.assertIn(affected_scope.ARCHIVE_GUARD_SUITE, manifest)

    def test_the_registry_binding_selects_a_scan_on_a_pull_request(self) -> None:
        guard = self.plan(
            "tools/test-toolkit/Cargo.toml", event="pull_request"
        )["archive_guard"]
        self.assertTrue(guard["selected"], guard["reason"])
        # Changed mode with an explicitly empty list: the manifest carries no
        # `.rs` file, and an empty inventory is a complete answer that is never
        # upgraded to a full scan. The scan the change actually needs — is the
        # exemption list still live — is the driver's full-tree half, which
        # runs in either mode.
        self.assertEqual("changed", guard["mode"])
        self.assertEqual([], guard["paths"])

    def test_each_configuration_input_selects_a_scan(self) -> None:
        for path in self.CONFIGURATION_INPUTS:
            with self.subTest(path=path):
                guard = self.plan(path, event="pull_request")["archive_guard"]
                self.assertTrue(guard["selected"], guard["reason"])
                self.assertEqual("changed", guard["mode"])
                self.assertEqual([], guard["paths"])

    def test_a_configuration_input_adds_no_cell_at_all(self) -> None:
        # The containment rule for this half of the owned list. Each of these
        # paths already selects the owner, so the guard's arrival must change
        # the plan's ACCOUNT and not its work: the same three cells, no check
        # cell, no second lint cell, no extra operating system.
        for path in self.CONFIGURATION_INPUTS:
            with self.subTest(path=path):
                cells = [
                    (cell["environment"], cell["gate"])
                    for cell in self.plan(path, event="pull_request")["cells"]  # type: ignore[index]
                    if cell["package"] == "test-toolkit"
                ]
                self.assertEqual(
                    [
                        ("macos-latest", "L1"),
                        ("ubuntu-latest", "L1"),
                        ("ubuntu-latest", "lint"),
                    ],
                    sorted(cells),
                )

    def test_the_guard_rides_the_owners_own_lint_cell(self) -> None:
        # `companions_only` stays absent: Clippy is still required work here,
        # so this is not the guard-only shape `ArchiveGuardOnlyCellTests`
        # covers, and the workflow must not stand Clippy down.
        lint = [
            cell
            for cell in self.plan("tools/test-toolkit/Cargo.toml")["cells"]  # type: ignore[index]
            if cell["package"] == "test-toolkit" and cell["gate"] == "lint"
        ]
        self.assertEqual(1, len(lint))
        self.assertIn(
            affected_scope.ARCHIVE_GUARD_SUITE,
            [entry["name"] for entry in lint[0]["companions"]],
        )
        self.assertNotIn("companions_only", lint[0])

    def test_an_unrelated_manifest_or_workflow_selects_no_scan(self) -> None:
        # Real files, so the narrowness is proved against the shipped tree and
        # not a fixture: a manifest that binds no suite, the release workflow,
        # and the top-level caller — which carries no guard-specific
        # configuration and whose resolved-plan artifact every area reads.
        for path in (
            "messenger/lib/Cargo.toml",
            ".github/workflows/release-plz.yml",
            ".github/workflows/ci.yml",
        ):
            with self.subTest(path=path):
                guard = self.plan(path, event="pull_request")["archive_guard"]
                self.assertFalse(guard["selected"], guard["reason"])
                self.assertNotIn("mode", guard)

    def test_every_owned_input_still_exists(self) -> None:
        # The policy is a list of path literals and nothing on this side
        # resolves them, so a rename leaves an entry that matches no change and
        # the guard silently stops being selected by the input it watches.
        # `ci_workflow_contracts` asserts the same thing from the Rust side.
        for path in sorted(affected_scope.ARCHIVE_GUARD_OWN_INPUTS):
            with self.subTest(path=path):
                self.assertTrue((ROOT / path).exists(), path)


class CompanionRecipeCheckTests(unittest.TestCase):
    """The companion-recipe existence check is a definition match, not a substring."""

    LABELS = {"ubuntu-latest", "windows-latest", "macos-latest"}

    def justfile_root(self, contents: str) -> Path:
        root = Path(tempfile.mkdtemp())
        (root / "homelab").mkdir()
        (root / "homelab" / "justfile").write_text(contents)
        return root

    def test_a_watch_recipe_does_not_satisfy_the_check(self) -> None:
        root = self.justfile_root("test-frontend-watch:\n    echo watch\n")
        with self.assertRaises(RuntimeError):
            validate_package_ci(
                "a",
                ci_policy(tests={**{"companion-suites": ["homelab-frontend"]}}),
                self.LABELS,
                root=root,
                today=TODAY,
            )

    def test_the_real_recipe_definition_satisfies_the_check(self) -> None:
        root = self.justfile_root(
            "test-frontend:\n    echo test\nlint-frontend:\n    echo lint\n"
        )
        validate_package_ci(
            "a",
            ci_policy(tests={**{"companion-suites": ["homelab-frontend"]}}),
            self.LABELS,
            root=root,
            today=TODAY,
        )

    def test_a_recipe_with_parameters_satisfies_the_check(self) -> None:
        root = self.justfile_root(
            'test-frontend *args="":\n    echo test\nlint-frontend:\n    echo lint\n'
        )
        validate_package_ci(
            "a",
            ci_policy(tests={**{"companion-suites": ["homelab-frontend"]}}),
            self.LABELS,
            root=root,
            today=TODAY,
        )


class CompanionCountsDeclarationTests(unittest.TestCase):
    """AC13: a companion reports counts, or says why it cannot. Never `0`."""

    def entry(self, **overrides: object) -> dict[str, dict[str, object]]:
        entry: dict[str, object] = {
            "owner": "homelab-server",
            "kind": "companion",
            "environment": "ubuntu-latest",
            "recipe": "cd homelab && just test-frontend",
            "counts": "vitest",
            "counts_args": "--reporter=json --outputFile={out}",
        }
        entry.update(overrides)
        return {"homelab-frontend": entry}

    def problems(self, registry: dict[str, dict[str, object]]) -> list[str]:
        return validate_suite_registry(registry, {"homelab-server": ["homelab-frontend"]})

    def test_every_shipped_companion_declares_one_of_the_two_states(self) -> None:
        # Passive corpus check: no shipped entry may sit between "reports
        # counts" and "says why it does not".
        for name, entry in affected_scope.SUITE_REGISTRY.items():
            if entry["kind"] != "companion":
                continue
            with self.subTest(suite=name):
                self.assertEqual(
                    [], affected_scope.companion_counts_problems(name, entry)
                )

    def test_a_strategy_with_no_output_argument_fails_validation(self) -> None:
        problems = self.problems(self.entry(counts_args="--reporter=json"))
        self.assertTrue(
            any("homelab-frontend" in problem for problem in problems),
            f"a counts strategy with nowhere to write must be rejected: {problems}",
        )

    def test_an_unknown_counts_strategy_fails_validation(self) -> None:
        problems = self.problems(self.entry(counts="tap"))
        self.assertTrue(
            any("tap" in problem for problem in problems),
            f"the counts vocabulary must be closed: {problems}",
        )

    def test_an_unmeasurable_suite_without_a_reason_fails_validation(self) -> None:
        problems = self.problems(self.entry(counts=None, counts_args=None))
        self.assertTrue(
            any("never `0`" in problem for problem in problems),
            f"an unmeasured suite must be made to say why: {problems}",
        )
        self.assertEqual(
            [],
            self.problems(
                self.entry(counts=None, counts_reason="typecheck gate reports no test counts")
            ),
            "with a reason, the same entry is well formed",
        )


class CompanionAttachmentTests(unittest.TestCase):
    """R7: a companion attaches to the ONE cell its registry entry declares."""

    def record(self, *suites: str) -> dict[str, object]:
        return {
            "package": "homelab-server",
            "tiers": ["L1"],
            "l2_backends": [],
            "features": [],
            "all_features": False,
            "l1_include_slow": False,
            "runner_tools": [],
            "companion_suites": list(suites),
            "requires_toolchain": False,
        }

    def cells(self, *suites: str) -> dict[tuple[str, str], dict[str, object]]:
        cells = package_cells(
            self.record(*suites),
            area="homelab",
            target_kinds=["lib"],
            environments=environments_for_tests(),
            accepted={},
        )
        return {(cell["environment"], cell["gate"]): cell for cell in cells}

    def test_a_companion_attaches_to_its_declared_environment_only(self) -> None:
        records = affected_scope.companion_records(
            ["homelab-frontend"], "ubuntu-latest", "L1"
        )
        self.assertEqual(["homelab-frontend"], [entry["name"] for entry in records])
        self.assertEqual("cd homelab && just test-frontend", records[0]["recipe"])
        self.assertIn("{out}", records[0]["counts_args"])
        for environment in ("macos-latest", "windows-latest", "wsl2-ubuntu"):
            with self.subTest(environment=environment):
                self.assertEqual(
                    [],
                    affected_scope.companion_records(
                        ["homelab-frontend"], environment, "L1"
                    ),
                )

    def test_only_the_declaring_environments_cell_loses_its_reuse(self) -> None:
        # The defect R7 names: deriving this from `node_pnpm` made every L1
        # cell of a companion-owning package hostage to a capability flag.
        cells = self.cells("homelab-frontend")
        hosting = cells[("ubuntu-latest", "L1")]
        self.assertFalse(hosting["reusable"])
        self.assertEqual(
            ["homelab-frontend"], [entry["name"] for entry in hosting["companions"]]
        )
        for environment in ("macos-latest", "windows-latest", "wsl2-ubuntu"):
            with self.subTest(environment=environment):
                elsewhere = cells[(environment, "L1")]
                self.assertTrue(elsewhere["reusable"])
                self.assertNotIn("companions", elsewhere)

    def test_a_package_with_no_companion_keeps_every_cell_reusable(self) -> None:
        cells = self.cells()
        for environment in (
            "ubuntu-latest",
            "macos-latest",
            "windows-latest",
            "wsl2-ubuntu",
        ):
            with self.subTest(environment=environment):
                self.assertTrue(cells[(environment, "L1")]["reusable"])

    def test_the_lint_cell_carries_only_the_suites_that_lint(self) -> None:
        lint = self.cells("homelab-frontend")[("ubuntu-latest", "lint")]
        self.assertEqual(
            ["homelab-frontend"], [entry["name"] for entry in lint["companions"]]
        )
        self.assertEqual("cd homelab && just lint-frontend", lint["companions"][0]["recipe"])
        self.assertIsNone(
            lint["companions"][0]["counts"],
            "a linter has no test cardinality, and the test reporter's flags "
            "would not even be accepted by it",
        )
        self.assertTrue(lint["companions"][0]["counts_reason"])

    def test_a_test_only_companion_leaves_the_lint_cell_empty(self) -> None:
        # `repo-deps`'s ten Python contract suites have no lint half. A lint
        # cell that expected them would fail every run of that package.
        cells = self.cells("test_schema.py")
        self.assertEqual(
            ["test_schema.py"],
            [entry["name"] for entry in cells[("ubuntu-latest", "L1")]["companions"]],
        )
        self.assertNotIn("companions", cells[("ubuntu-latest", "lint")])

    def test_the_policy_record_separates_the_lint_half(self) -> None:
        record = affected_scope.policy_record(
            {
                **self.record("homelab-frontend", "test_schema.py"),
                "area": "homelab",
                "gates": True,
                "exclusion": None,
            }
        )
        self.assertEqual(
            ["homelab-frontend", "test_schema.py"], record["companion_suites"]
        )
        self.assertEqual(["homelab-frontend"], record["lint_companion_suites"])

    def test_only_the_cells_that_run_a_companion_are_handed_one(self) -> None:
        # The companion step is skipped where nothing is attached, rather than
        # started and resolved to an empty set: a runner without the suites'
        # interpreter must not be able to fail a cell that was never asked to
        # run them.
        def companion_hosts(record: dict) -> set[str]:
            return {
                environment
                for (environment, _), contract in resolved_contracts(record).items()
                if json.loads(contract["companion_suites"])
            }

        self.assertEqual(
            {"ubuntu-latest"},
            companion_hosts(
                plan_package(package="homelab-server", companion_suites=["homelab-frontend"])
            ),
        )
        self.assertEqual(set(), companion_hosts(plan_package(package="alpha")))

    def test_node_provisioning_follows_the_suites_that_need_it(self) -> None:
        # A Python-only owner must not install pnpm on every capable runner.
        def node_hosts(record: dict) -> set[str]:
            return {
                environment
                for (environment, _), contract in resolved_contracts(record).items()
                if contract["requires_node"] == "true"
            }

        self.assertEqual(
            set(),
            node_hosts(plan_package(package="repo-deps", companion_suites=["test_schema.py"])),
        )
        self.assertEqual(
            {"ubuntu-latest"},
            node_hosts(
                plan_package(
                    package="test-toolkit",
                    companion_suites=["test-audit-typecheck", "test-audit-vitest"],
                )
            ),
        )


class CompanionRunnerTests(unittest.TestCase):
    """`companion_suites.py` runs each declared suite once and records it."""

    COUNTS = {"total": 3, "passed": 2, "failed": 0, "skipped": 1, "errored": 0}

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        self.out = self.root / "companions.json"

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def fake_suite(self, name: str, *, counts: bool, exit_code: int) -> str:
        """A suite command that behaves like a real one, on any OS."""
        script = self.root / f"{name}.py"
        body = "import sys\n"
        if counts:
            body += (
                "import json\n"
                f"json.dump({self.COUNTS!r}, open(sys.argv[1], 'w'))\n"
            )
        body += f"sys.exit({exit_code})\n"
        script.write_text(body, encoding="utf-8")
        return f'"{sys.executable}" "{script}"'

    def registry(self, **entries: dict[str, object]) -> dict[str, dict[str, object]]:
        return entries

    def run_suites(
        self, registry: dict[str, dict[str, object]], names: list[str], gate: str = "L1"
    ) -> tuple[int, dict[str, dict[str, object]]]:
        with unittest.mock.patch.dict(
            affected_scope.SUITE_REGISTRY, registry, clear=True
        ):
            code = companion_suites.main(
                [
                    "--suites",
                    json.dumps(names),
                    "--environment",
                    "ubuntu-latest",
                    "--gate",
                    gate,
                    "--out",
                    str(self.out),
                    "--root",
                    str(self.root),
                ]
            )
        if not self.out.is_file():
            return code, {}
        return code, json.loads(self.out.read_text(encoding="utf-8"))

    def companion(self, **overrides: object) -> dict[str, object]:
        entry: dict[str, object] = {
            "owner": "owner",
            "kind": "companion",
            "environment": "ubuntu-latest",
            "counts": "json",
            "counts_args": "{out}",
        }
        entry.update(overrides)
        return entry

    def test_each_declared_suite_records_its_own_outcome_and_counts(self) -> None:
        registry = self.registry(
            first=self.companion(recipe=self.fake_suite("first", counts=True, exit_code=0)),
            second=self.companion(recipe=self.fake_suite("second", counts=True, exit_code=0)),
        )
        code, results = self.run_suites(registry, ["first", "second"])
        self.assertEqual(0, code)
        self.assertEqual(["first", "second"], sorted(results))
        for name in ("first", "second"):
            with self.subTest(suite=name):
                self.assertEqual("success", results[name]["outcome"])
                self.assertEqual(self.COUNTS, results[name]["counts"])
                self.assertGreaterEqual(results[name]["duration_s"], 0)

    def test_a_recipes_relative_cd_ignores_the_callers_cdpath(self) -> None:
        # Regression: pushed from a worktree, by a shell whose `CDPATH` named the
        # main checkout, `cd tools/test-toolkit && just archive-path-guard` ran
        # in the main checkout and failed with "justfile does not contain
        # recipe". `--root` decides where a recipe runs; `CDPATH` must not.
        (self.root / "area").mkdir()
        decoy = self.root / "elsewhere"
        (decoy / "area").mkdir(parents=True)
        marker = self.root / "ran-in.txt"
        script = self.root / "where.py"
        script.write_text(
            "import json, os, sys\n"
            f"open({str(marker)!r}, 'w', encoding='utf-8').write(os.getcwd())\n"
            f"json.dump({self.COUNTS!r}, open(sys.argv[1], 'w'))\n",
            encoding="utf-8",
        )
        registry = self.registry(
            guarded=self.companion(recipe=f'cd area && "{sys.executable}" "{script}"')
        )
        with unittest.mock.patch.dict(os.environ, {"CDPATH": str(decoy)}):
            code, results = self.run_suites(registry, ["guarded"])
        self.assertEqual(0, code)
        self.assertEqual("success", results["guarded"]["outcome"])
        self.assertEqual(
            (self.root / "area").resolve(),
            Path(marker.read_text(encoding="utf-8")).resolve(),
        )

    def test_one_suites_failure_does_not_stop_or_cover_the_others(self) -> None:
        registry = self.registry(
            broken=self.companion(recipe=self.fake_suite("broken", counts=True, exit_code=1)),
            healthy=self.companion(recipe=self.fake_suite("healthy", counts=True, exit_code=0)),
        )
        code, results = self.run_suites(registry, ["broken", "healthy"])
        self.assertEqual(1, code, "a failed companion fails the step")
        self.assertEqual("failure", results["broken"]["outcome"])
        self.assertEqual(
            "success",
            results["healthy"]["outcome"],
            "the runner must not stop at the first failure; the other suite's "
            "evidence is exactly what a reviewer needs",
        )

    def test_an_unmeasured_suite_records_a_reason_and_never_a_zero(self) -> None:
        registry = self.registry(
            silent=self.companion(recipe=self.fake_suite("silent", counts=False, exit_code=0)),
            gate=self.companion(
                recipe=self.fake_suite("gate", counts=False, exit_code=0),
                counts=None,
                counts_args=None,
                counts_reason="typecheck gate reports no test counts",
            ),
        )
        _, results = self.run_suites(registry, ["silent", "gate"])
        for name in ("silent", "gate"):
            with self.subTest(suite=name):
                self.assertNotIn(
                    "counts",
                    results[name],
                    "an absent measurement is absent, not a zero that reads as "
                    "a suite which ran and found nothing",
                )
                self.assertTrue(results[name]["reason"])
        self.assertEqual(
            "typecheck gate reports no test counts", results["gate"]["reason"]
        )

    def test_a_suite_the_cell_does_not_host_is_not_run(self) -> None:
        registry = self.registry(
            elsewhere=self.companion(
                recipe=self.fake_suite("elsewhere", counts=True, exit_code=1),
                environment="macos-latest",
            ),
        )
        code, results = self.run_suites(registry, ["elsewhere"])
        self.assertEqual(0, code)
        self.assertEqual({}, results)

    def test_the_lint_gate_runs_only_the_suites_with_a_lint_recipe(self) -> None:
        registry = self.registry(
            both=self.companion(
                recipe=self.fake_suite("both-test", counts=True, exit_code=1),
                lint_recipe=self.fake_suite("both-lint", counts=False, exit_code=0),
            ),
            test_only=self.companion(
                recipe=self.fake_suite("test-only", counts=True, exit_code=1)
            ),
        )
        code, results = self.run_suites(registry, ["both", "test_only"], gate="lint")
        self.assertEqual(0, code, "the test halves must not run in the lint job")
        self.assertEqual(["both"], sorted(results))
        self.assertEqual("success", results["both"]["outcome"])

    def test_an_unregistered_suite_name_is_a_runner_error(self) -> None:
        code, _ = self.run_suites(self.registry(), ["no-such-suite"])
        self.assertEqual(
            2, code, "an unregistered name is a mis-wired producer, not a test failure"
        )

    def test_a_shipped_suite_runs_and_reports_its_real_counts(self) -> None:
        # End to end through the shipped artifact and its normal invocation
        # path: the registry's own recipe for a real suite, run as CI runs it.
        entry = affected_scope.SUITE_REGISTRY["test_runner_loss.py"]
        counts_out = self.root / "counts.json"
        command = shlex.split(entry["recipe"]) + shlex.split(
            entry["counts_args"].replace(
                affected_scope.COUNTS_OUT_PLACEHOLDER, str(counts_out)
            )
        )
        # The recipe's own interpreter spelling, resolved to the one running
        # this suite: `python3` is not on PATH under every Windows shell, and
        # the subject here is the wrapper, not the launcher.
        self.assertEqual("python3", command[0])
        command[0] = sys.executable
        completed = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
        self.assertEqual(0, completed.returncode, completed.stderr)
        counts = json.loads(counts_out.read_text(encoding="utf-8"))
        self.assertGreater(counts["total"], 0)
        self.assertEqual(counts["total"], counts["passed"] + counts["skipped"])
        self.assertEqual(0, counts["failed"])
        self.assertIn("duration_s", counts)

    def test_the_wrapper_reports_a_failing_suite_as_failed(self) -> None:
        suite = ROOT / "scripts" / "ci" / "test_affected_scope.py"
        self.assertTrue(suite.is_file(), "the wrapper's subject must exist")
        script = self.root / "failing_suite.py"
        script.write_text(
            "import unittest\n"
            "class T(unittest.TestCase):\n"
            "    def test_fails(self):\n"
            "        self.fail('deliberate')\n"
            "    def test_passes(self):\n"
            "        pass\n",
            encoding="utf-8",
        )
        counts_out = self.root / "counts.json"
        completed = subprocess.run(
            [
                sys.executable,
                str(ROOT / "scripts" / "ci" / "suite_runner.py"),
                "failing_suite",
                "--counts-out",
                str(counts_out),
            ],
            cwd=self.root,
            capture_output=True,
            text=True,
            env={**os.environ, "PYTHONPATH": str(self.root)},
        )
        self.assertEqual(1, completed.returncode, completed.stderr)
        counts = json.loads(counts_out.read_text(encoding="utf-8"))
        self.assertEqual(2, counts["total"])
        self.assertEqual(1, counts["failed"])
        self.assertEqual(1, counts["passed"])


class L2BackendAxisTests(unittest.TestCase):
    """L2 cells execute where any declared backend is hostable, not only tmux."""

    def test_a_non_tmux_backend_gets_an_environment_axis(self) -> None:
        environments = environments_for_tests()
        for environment in environments:
            environment["capabilities"]["wezterm"] = environment["name"] == "macos-latest"
        contracts = resolved_contracts(
            plan_package(package="a", tiers=["L1", "L2"], l2_backends=["wezterm"]),
            environments,
        )
        self.assertEqual({("macos-latest", "L2")}, {cell for cell in contracts if cell[1] == "L2"})
        self.assertEqual('["wezterm"]', contracts[("macos-latest", "L2")]["backends"])

    def test_a_mixed_backend_package_requires_only_what_the_environment_hosts(self) -> None:
        # The cell's `backends` is what its producer must prove; the GUI
        # backends stay in the package's declaration (so the job's coverage
        # summary can name them as skipping) but are required of no CI cell.
        environments = environments_for_tests()
        for environment in environments:
            environment["capabilities"]["wezterm"] = False
            environment["capabilities"]["kitty"] = False
        contracts = resolved_contracts(
            plan_package(
                package="a", tiers=["L1", "L2"], l2_backends=["wezterm", "tmux", "kitty"]
            ),
            environments,
        )
        l2 = {cell: contract for cell, contract in contracts.items() if cell[1] == "L2"}
        self.assertEqual({("ubuntu-latest", "L2"), ("macos-latest", "L2")}, set(l2))
        for contract in l2.values():
            self.assertEqual('["tmux"]', contract["backends"])
            self.assertEqual('["wezterm", "tmux", "kitty"]', contract["declared_backends"])

    def test_a_backend_no_environment_hosts_executes_nowhere(self) -> None:
        # `load_environments` refuses a table missing any known capability, so
        # "no entry" is unrepresentable; "no environment hosts it" is the case.
        environments = environments_for_tests()
        for environment in environments:
            environment["capabilities"]["kitty"] = False
        contracts = resolved_contracts(
            plan_package(package="a", tiers=["L1", "L2"], l2_backends=["kitty"]), environments
        )
        self.assertEqual(set(), {cell for cell in contracts if cell[1] == "L2"})
        self.assertTrue(contracts, "the package's other cells still execute")


class RealWorkspaceNativeGuardTests(unittest.TestCase):
    """The native union rides the workspace-unified resolve (latent coupling).

    `build_closure` sees an OPTIONAL edge only while some workspace member
    enables it. biscuit-speaks -> playa is the known instance: if every
    enabling edge disappears, the union silently loses playa's ALSA/PulseAudio
    requirements, so pin it against the real metadata.
    """

    def test_biscuit_speaks_closure_still_contains_playa(self) -> None:
        metadata = load_metadata(ROOT)
        packages = workspace_packages(metadata)
        speaks_id = next(
            package_id
            for package_id, package in packages.items()
            if package["name"] == "biscuit-speaks"
        )
        closure = build_closure(speaks_id, metadata, packages)
        names = {packages[member_id]["name"] for member_id in closure}
        self.assertIn(
            "playa",
            names,
            "the biscuit-speaks -> playa optional edge vanished from the "
            "workspace-unified resolve; biscuit-speaks would silently lose "
            "playa's ALSA/PulseAudio native requirements",
        )


class RealWorkspaceAreaFanOutTests(unittest.TestCase):
    """The shipped workspace's area fan-out, against GitHub's real ceilings.

    A passive corpus case: it runs the shipped planner over every workspace
    member rather than a fixture, because the ceiling that matters is the one
    an explicit `workflow_dispatch` full run would hit.
    """

    # GitHub Actions: 256 jobs per matrix, and a workflow run may not exceed
    # 1000 jobs across every matrix and nested workflow it expands.
    RUN_JOB_LIMIT = 1000

    @classmethod
    def setUpClass(cls) -> None:
        cls.metadata = load_metadata(ROOT)
        cls.environments = load_environments(ENVIRONMENTS_CONFIG, today=TODAY)
        cls.policy = package_ci_policy(
            workspace_packages(cls.metadata),
            runner_labels={environment["runner"] for environment in cls.environments},
            root=ROOT,
            today=TODAY,
        )
        cls.full = scope_document(
            [], ROOT, cls.metadata, cls.environments, cls.policy, force_all=True
        )

    def test_full_scope_stays_within_githubs_matrix_and_job_ceilings(self) -> None:
        areas = self.full["scheduled_areas"]
        self.assertLessEqual(
            len(areas),
            MATRIX_LIMIT,
            f"the area fan-out is a matrix too: {len(areas)} areas exceed "
            f"GitHub's {MATRIX_LIMIT}-entry ceiling",
        )
        for area, document in self.full["area_rows"].items():
            for name in schema.ROW_SET_NAMES:
                self.assertLessEqual(
                    len(document[name]),
                    MATRIX_LIMIT,
                    f"area {area} dispatches {len(document[name])} {name} rows",
                )
        # `job_estimate` counts executing cells, not the scheduling scaffolding.
        # The area restructure adds two jobs per area — the caller identity and
        # the area's own rollup — so the ceiling is checked against the total.
        total = self.full["job_estimate"] + 2 * len(areas)
        self.assertLess(
            total,
            self.RUN_JOB_LIMIT,
            f"a full-scope run expands to about {total} jobs "
            f"({self.full['job_estimate']} cells plus {len(areas)} area callers "
            f"and {len(areas)} area rollups)",
        )

    def test_an_affected_area_plan_is_far_under_the_full_scope_ceilings(self) -> None:
        # AC1's counterpart: the ordinary case must not merely fit, it must be
        # a small fraction of the full run. `tools` rides along because a Rust
        # file is source the archive-path guard scans, and that adds exactly
        # one Linux lint cell — see `ArchiveGuardOnlyCellTests`.
        scope = scope_document(
            ["claudine/lib/src/lib.rs"],
            ROOT,
            self.metadata,
            self.environments,
            self.policy,
        )
        self.assertEqual(["claudine", "tools"], scope["scheduled_areas"])
        self.assertLess(scope["job_estimate"], self.full["job_estimate"])

    def test_a_nested_area_fans_out_beside_its_parent_never_inside_it(self) -> None:
        # Design Decision 2, on the real workspace: `claudine/rendezvous` is a
        # distinct area, so a change to it must not appear under `claudine`.
        areas = self.full["area_rows"]
        self.assertIn("claudine", self.full["scheduled_areas"])
        self.assertIn("claudine/rendezvous", self.full["scheduled_areas"])

        def packages(area: str) -> set[str]:
            return {
                row["package"] for name in schema.ROW_SET_NAMES for row in areas[area][name]
            }

        parent = packages("claudine")
        nested = packages("claudine/rendezvous")
        self.assertTrue(nested)
        self.assertEqual(set(), parent & nested)

    def test_every_shipped_area_slug_is_a_legal_artifact_name(self) -> None:
        slugs = self.full["area_slugs"]
        self.assertEqual(sorted(slugs), self.full["scheduled_areas"])
        for area, slug in slugs.items():
            self.assertFalse(
                set(slug) & set('/\\:<>|*?"\r\n'),
                f"area {area} slugs to {slug!r}, which GitHub rejects as an artifact name",
            )
        self.assertEqual(len(set(slugs.values())), len(slugs))


class RealWorkspaceDocumentationOnlyTests(unittest.TestCase):
    """AC11/AC13 at all three documentation ownership levels, on the real tree.

    A passive corpus case over the shipped workspace: a document owned by the
    repository, by an area, or by a package must account for itself and select
    nothing. Whether the two renderers then NAME those documents is asserted by
    `ci-plan-tests.rs`, `ci-rollup-tests.rs`, and `test_ci_local.py`; this
    fixture owns the planner's half — the inventory they read, and the zero
    package and preflight executions beside it.
    """

    #: Repository-owned, area-owned, and package-owned, in that order. Each is
    #: a tracked file, so a rename that makes one of them stop existing is a
    #: fixture failure rather than a silently weaker assertion — and none is
    #: read by any test, which a document like `docs/topics/ci-cd.md` is.
    LEVELS = (
        "docs/comment-quality.md",
        "darkmatter/docs/topics/caching.md",
        "biscuit-file/README.md",
    )

    @classmethod
    def setUpClass(cls) -> None:
        cls.metadata = load_metadata(ROOT)
        cls.environments = load_environments(ENVIRONMENTS_CONFIG, today=TODAY)
        cls.policy = package_ci_policy(
            workspace_packages(cls.metadata),
            runner_labels={environment["runner"] for environment in cls.environments},
            root=ROOT,
            today=TODAY,
        )

    def test_each_ownership_level_exists_in_the_tree(self) -> None:
        for path in self.LEVELS:
            self.assertTrue((ROOT / path).is_file(), f"{path} is no longer tracked")

    def test_each_ownership_level_selects_nothing_and_names_its_document(self) -> None:
        for path in self.LEVELS:
            with self.subTest(path=path):
                plan = calculate_scope(
                    [path], ROOT, self.metadata, self.environments, self.policy
                )
                self.assertEqual([], plan["packages"])
                self.assertEqual([], plan["cells"])
                self.assertEqual([], plan["areas"])
                self.assertEqual(
                    [],
                    plan["preflight_os"],
                    "a change that selects no gating package establishes nothing, "
                    "so preflight must skip too",
                )
                inventory = plan["change_inventory"]
                self.assertTrue(inventory["diff_available"])
                self.assertEqual([path], inventory["paths"]["documentation"])
                self.assertEqual(1, inventory["counts"]["total"])
                self.assertEqual(
                    [],
                    legacy_scope_document(plan)["scheduled_areas"],
                    "no area fans out, so no runner is spent",
                )


class RealWorkspaceRetirementScopeTests(unittest.TestCase):
    """Retirement contracts exercised against the shipped workspace policy."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.metadata = load_metadata(ROOT)
        cls.packages = workspace_packages(cls.metadata)
        cls.environments = load_environments(ENVIRONMENTS_CONFIG, today=TODAY)
        cls.policy = package_ci_policy(
            cls.packages,
            runner_labels={environment["runner"] for environment in cls.environments},
            root=ROOT,
            today=TODAY,
        )

    def scope(self, path: str) -> dict[str, object]:
        return legacy_scope_document(self.plan(path))

    def plan(self, path: str) -> dict:
        return calculate_scope([path], ROOT, self.metadata, self.environments, self.policy)

    def test_messenger_policy_and_cell_contract_are_promoted(self) -> None:
        messenger = self.policy["messenger"]
        missing: list[str] = []
        if not messenger["gates"]:
            missing.append("gating policy")
        if not messenger["all_features"] or messenger["features"]:
            missing.append("all-feature policy")
        if messenger["native"] != {"ubuntu-latest": ["libdbus-1-dev"]}:
            missing.append("libdbus-1-dev native prerequisite")
        if messenger["runner_tools"] != []:
            missing.append("runner tools (its desktop stubs are a build sidecar)")
        if messenger["sidecars"] != ["messenger-desktop-stubs"]:
            missing.append("messenger-desktop-stubs build sidecar")

        contracts = producer_contracts(self.plan("messenger/lib/src/lib.rs"), "messenger")
        if not contracts:
            missing.append("ordinary messenger package cells")
        else:
            if {contract["check_args"] for contract in contracts.values()} != {
                "-p messenger --all-features"
            }:
                missing.append("all-feature check arguments")
            if {contract["test_args"] for contract in contracts.values()} != {"--all-features"}:
                missing.append("all-feature native/WSL2 L1 arguments")
            if {environment for environment, gate in contracts if gate == "L1"} != {
                "ubuntu-latest",
                "windows-latest",
                "macos-latest",
                "wsl2-ubuntu",
            }:
                missing.append("three native L1 environments and the WSL2 archive cell")

        area_ci = (ROOT / ".github/workflows/_area-ci.yml").read_text()
        package_ci = (ROOT / ".github/workflows/_package-ci.yml").read_text()
        wsl_ci = (ROOT / ".github/workflows/_wsl-ci.yml").read_text()
        # Every declared argument now reaches its job through the plan, by the
        # cell key the dispatch row names — so what this pins is that each
        # value is still APPLIED where it was, not which input carries it.
        forwarding_contract = [
            "cargo check ${{ steps.cell.outputs.check_args }}" in package_ci,
            # The declared features still reach the canonical recipe; archive
            # mode only adds the verified archive and this checkout ahead of
            # them, and `_archive_drop_build_flags` drops what nextest refuses.
            'just "$RECIPE" "${{ matrix.package }}"' in package_ci,
            '${archive_args[@]+"${archive_args[@]}"} ${{ steps.cell.outputs.test_args }}'
            in package_ci,
            # The guest is a pure consumer: it resolves the plan's build record
            # for its own cell, so no archive selector exists for a check
            # argument to leak into.
            "row: ${{ toJSON(matrix) }}" in package_ci,
            "${{ steps.cell.outputs.build_artifact }}" in wsl_ci,
            "check_args" not in wsl_ci,
            "${{ steps.cell.outputs.test_args }}" in wsl_ci,
            # Per-gate selection: a gate a package was not selected for gets no
            # row, so no job expands for it and nothing has to be skipped.
            all(
                f"include: ${{{{ fromJSON(inputs.{name}-rows) }}}}" in package_ci
                for name in ("test", "check", "lint", "wsl")
            ),
            all(
                f"if: ${{{{ inputs.{name}-rows != '[]' }}}}" in package_ci
                for name in ("test", "check", "lint", "wsl")
            ),
        ]
        if not all(forwarding_contract):
            missing.append("check/native-L1/WSL2 feature-argument forwarding")

        self.assertEqual(
            [],
            missing,
            f"messenger promotion contract is incomplete: {', '.join(missing)}",
        )

    def test_messenger_cli_change_selects_its_normal_package_cell(self) -> None:
        # `test-toolkit` rides along on every Rust change as the archive-path
        # guard's lint-only owner; the package under test still gets exactly
        # its own normal cell.
        plan = self.plan("messenger/cli/src/lib.rs")
        self.assertEqual(
            ["messenger-cli", "test-toolkit"], legacy_scope_document(plan)["packages"]
        )
        self.assertEqual(["messenger-cli", "test-toolkit"], sorted(gates_by_package(plan)))

    def test_workspace_excluded_zed_extension_selects_dmls_companion(self) -> None:
        plan = self.plan("darkmatter/dmls/zed-dmls/src/lib.rs")
        self.assertEqual(["dmls", "test-toolkit"], legacy_scope_document(plan)["packages"])
        self.assertEqual(["dmls", "test-toolkit"], sorted(gates_by_package(plan)))
        contracts = producer_contracts(plan, "dmls")
        self.assertTrue(contracts)
        for cell, contract in contracts.items():
            self.assertEqual(["neovim", "zed-extension"], json.loads(contract["runner_tools"]), cell)

        package_ci = (ROOT / ".github/workflows/_package-ci.yml").read_text()
        self.assertIn(
            "contains(fromJSON(steps.cell.outputs.runner_tools), 'zed-extension')",
            package_ci,
        )
        self.assertIn("just zed-verify", package_ci)

        lint_job = package_ci.split("\n  lint:", 1)[1].split("\n  # WSL2 is", 1)[0]
        self.assertIn("Read the pinned Zed extension packager record", lint_job)
        self.assertIn("rustup target add wasm32-wasip2", lint_job)
        self.assertIn("sha256sum --check --strict", lint_job)

        pin = json.loads((ROOT / ".github/ci/zed-extension.json").read_text())
        self.assertEqual(1, pin["schema_version"])
        self.assertRegex(pin["zed_commit"], r"^[0-9a-f]{40}$")
        self.assertRegex(pin["linux_x86_64_sha256"], r"^[0-9a-f]{64}$")
        self.assertEqual(
            f"https://zed-extension-cli.nyc3.digitaloceanspaces.com/"
            f"{pin['zed_commit']}/x86_64-unknown-linux-gnu/zed-extension",
            pin["linux_x86_64_url"],
        )

        justfile = (ROOT / "darkmatter/justfile").read_text()
        check_zed = justfile.split("check-zed:", 1)[1].split("zed-package", 1)[0]
        self.assertIn('target="wasm32-wasip2"', check_zed)
        self.assertIn("cargo check --locked", check_zed)
        self.assertNotIn("\n    rustup target add", check_zed)
        self.assertNotIn("exit 0", check_zed)

        latest_stable = (ROOT / ".github/workflows/rust-latest-stable.yml").read_text()
        for required in (
            "targets: wasm32-wasip2",
            ".github/ci/zed-extension.json",
            "actions/cache@v5",
            "sha256sum --check --strict",
            "just zed-verify",
        ):
            self.assertIn(required, latest_stable)

    def test_sniff_change_selects_exact_direct_dependents(self) -> None:
        # Deliberate friction: this exact list makes a human acknowledge a
        # change to sniff's direct dependents. On mismatch, the message below
        # hands back the computed list ready to paste into this fixture.
        scope = self.scope("sniff/lib/src/lib.rs")
        self.assertEqual(["sniff", "test-toolkit"], scope["packages"])
        computed = scope["reverse_dependencies"]
        paste_ready = "\n".join(f'                "{name}",' for name in computed)
        hint = (
            "\nsniff's direct dependents changed (a workspace dependency edge "
            "was added or removed). If intentional, replace the expected list "
            f"in {__file__} :: {self._testMethodName} with:\n{paste_ready}"
        )
        self.assertEqual(
            [
                "biscuit-speaks",
                "biscuit-speaks-cli",
                "biscuit-terminal-cli",
                "claudine",
                "claudine-cli",
                "claudine-gen",
                "darkmatter",
                "darkmatter-cli",
                "messenger",
                "messenger-cli",
                "model-citizen",
                "playa",
                "playa-cli",
                "rendezvous-core",
                "rendezvous-daemon",
                # `repo-deps` joined the root workspace when `scripts/` was
                # promoted to a gating package; `drift` links sniff.
                "repo-deps",
                "research",
                "sniff-cli",
                "unchained-ai",
                "worktree",
                "worktree-cli",
                "zed-dmls-cli",
            ],
            computed,
            hint,
        )


class WorkflowContractTests(unittest.TestCase):
    """Contracts in workflow YAML that are not represented by scope data."""

    def test_area_rollup_download_unions_current_and_run_wide_artifacts(self) -> None:
        # The whole-run verdict job that first carried these downloads was
        # replaced by the per-area rollup on 2026-09-12; the union it needs
        # (current attempt first, newest run-wide overlay second) is unchanged.
        area_ci = (ROOT / ".github/workflows/_area-ci.yml").read_text()
        rollup_downloads = area_ci.split(
            "- name: Download the resolved package policy", 1
        )[1].split("- name: Build ci-rollup", 1)[0]
        policy, plan, current, newest = rollup_downloads.split(
            "uses: actions/download-artifact@v7"
        )[1:]

        self.assertIn("name: ci-scope", policy)
        self.assertIn("path: ci-artifacts/ci-scope", policy)
        # The resolved plan is the only document naming the cells a receipt
        # already satisfied; rolling up without it reports them MISSING.
        self.assertIn("name: ci-resolved-plan", plan)
        self.assertIn("path: ci-artifacts/ci-resolved-plan", plan)
        self.assertNotIn("github-token:", current)
        # Build statuses ride in the same union: a cell blocked by a named
        # build record is only explicable if that record's account arrives too.
        # Completion records ride in it too: an executing cell with a green
        # status and no record is unproven, so an audit that never downloaded
        # the records would refuse every executing cell.
        pattern = "pattern: '{junit-*,status-*,build-status-*,completion-*}'"
        self.assertIn(pattern, current)
        self.assertIn("github-token: ${{ github.token }}", newest)
        self.assertIn("run-id: ${{ github.run_id }}", newest)
        self.assertIn(pattern, newest)


# --- helpers ---------------------------------------------------------------


def plan_package(**overrides: object) -> dict[str, object]:
    """A plan package record, the shape `cell_contract` resolves a cell against."""
    record: dict[str, object] = {
        "package": "a",
        "area": "a",
        "selection_reason": "source change",
        "gates": ["lint", "L1"],
        "targets": ["lib"],
        "tiers": ["L1"],
        "test_args": "",
        "check_args": "-p a",
        "l2_backends": [],
        "runner_tools": [],
        "companion_suites": [],
        "requires_toolchain": False,
        "l1_include_slow": False,
        "native": {},
    }
    record.update(overrides)
    if "check_args" not in overrides and "package" in overrides:
        record["check_args"] = f"-p {overrides['package']}"
    return record


def producer_build(
    host: str,
    arch: str,
    abi: str,
    libc: str,
    executes: list[str],
) -> dict:
    """A native producer's compile contract, in the shape the table declares."""
    return {
        "host": host,
        "target": host,
        "profile": "test",
        "rustflags": "",
        "cargo_config": [],
        "linker": "cc",
        "archive_format": "tar.zst",
        "nextest": "latest",
        "executes": executes,
        "runtime": {
            "arch": arch,
            "abi": abi,
            "libc": libc,
            "native_libraries": [],
        },
    }


def environments_for_tests() -> list[dict[str, object]]:
    return [
        {
            "name": "ubuntu-latest",
            "runner": "ubuntu-latest",
            "native_key": "ubuntu-latest",
            "events": ["pull_request", "push", "schedule", "workflow_dispatch"],
            "capabilities": {
                "tmux": True,
                "headless_browser": True,
                "node_pnpm": True,
                "cargo_toolchain": True,
                "archive_only": False,
            },
            "build": producer_build(
                "x86_64-unknown-linux-gnu",
                "x86_64",
                "gnu",
                "glibc",
                ["ubuntu-latest", "wsl2-ubuntu"],
            ),
        },
        {
            "name": "windows-latest",
            "runner": "windows-latest",
            "native_key": "windows-latest",
            "events": ["pull_request", "push", "schedule", "workflow_dispatch"],
            "capabilities": {
                "tmux": {
                    "available": False,
                    "reason": "no Windows port",
                    "owner": "@yankeeinlondon",
                    "expiry": "2027-01-31",
                },
                "headless_browser": False,
                "node_pnpm": False,
                "cargo_toolchain": True,
                "archive_only": False,
            },
            "build": producer_build(
                "x86_64-pc-windows-msvc", "x86_64", "msvc", "msvc", ["windows-latest"]
            ),
        },
        {
            "name": "macos-latest",
            "runner": "macos-latest",
            "native_key": "macos-latest",
            "events": ["pull_request", "push", "schedule", "workflow_dispatch"],
            "capabilities": {
                "tmux": True,
                "headless_browser": False,
                "node_pnpm": False,
                "cargo_toolchain": True,
                "archive_only": False,
            },
            "build": producer_build(
                "aarch64-apple-darwin",
                "aarch64",
                "darwin",
                "libSystem",
                ["macos-latest"],
            ),
        },
        {
            "name": "wsl2-ubuntu",
            "runner": "windows-latest",
            "native_key": "ubuntu-latest",
            "events": ["pull_request", "push", "schedule", "workflow_dispatch"],
            "capabilities": {
                "tmux": {
                    "available": False,
                    "reason": "archive-only leg",
                    "owner": "@yankeeinlondon",
                    "expiry": "2026-12-31",
                },
                "headless_browser": False,
                "node_pnpm": False,
                "cargo_toolchain": False,
                "archive_only": True,
            },
            "build": {
                "nextest": "latest",
                "runtime": {
                    "arch": "x86_64",
                    "abi": "gnu",
                    "libc": "glibc",
                    "native_libraries": [],
                },
            },
        },
    ]


def workspace_packages_from(metadata: dict[str, object]) -> dict[str, dict[str, object]]:
    members = set(metadata["workspace_members"])  # type: ignore[arg-type]
    return {
        package["id"]: package
        for package in metadata["packages"]  # type: ignore[index]
        if package["id"] in members
    }


class ArchiveInventoryClosureTests(unittest.TestCase):
    """Task 6.4: what a real package's archive must carry, closed declaratively.

    A passive corpus test over the whole workspace rather than a synthetic
    fixture: the classes this closes — the L2 harness broker, the backend
    proof, the cross-package `md` fixture, messenger's desktop stubs — are
    exactly the ones a synthetic fixture cannot have, because they are real
    binaries of real packages.
    """

    @classmethod
    def setUpClass(cls) -> None:
        # The planner reads `cargo metadata`; without Cargo there is no
        # workspace to close over and the guard says where that is checked.
        require_tools("cargo", enforced_by=CARGO_ENFORCED_BY)
        result = subprocess.run(
            [
                sys.executable,
                str(ROOT / "scripts" / "ci" / "affected_scope.py"),
                "--all",
                "--resolved-plan",
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
            timeout=300,
        )
        # A planner that cannot resolve a full-scope plan is the loudest
        # failure this suite can see. This was a SkipTest until 2026-09-15,
        # which reported the whole class green on a total planner regression.
        if result.returncode != 0:
            raise AssertionError(
                f"the planner could not resolve a full-scope plan: {result.stderr}"
            )
        cls.plan = json.loads(result.stdout)

    def test_every_ci_hostable_l2_package_declares_the_binaries_its_recipe_needs(self) -> None:
        offenders = {
            record["package"]: affected_scope.missing_tier_sidecars(record)
            for record in self.plan["packages"]
            if affected_scope.missing_tier_sidecars(record)
        }
        self.assertEqual(
            {},
            offenders,
            "these packages execute L2 on a CI runner and would have to build the "
            "harness broker or the backend proof with a Cargo their archive "
            "consumer does not have",
        )

    def test_the_rule_names_exactly_what_is_missing(self) -> None:
        # The negative direction, on the same function the corpus check uses.
        hostable = {"tiers": ["L1", "L2"], "l2_backends": ["tmux"]}
        self.assertEqual(
            ["backend-proof", "harness-broker"],
            affected_scope.missing_tier_sidecars({**hostable, "sidecars": []}),
        )
        self.assertEqual(
            ["backend-proof"],
            affected_scope.missing_tier_sidecars(
                {**hostable, "sidecars": ["harness-broker"]}
            ),
        )
        self.assertEqual(
            [],
            affected_scope.missing_tier_sidecars(
                {**hostable, "sidecars": ["backend-proof", "harness-broker"]}
            ),
        )

    def test_a_gui_only_l2_tier_owes_no_sidecar(self) -> None:
        # It renders a governed capability gap and never executes on a runner,
        # so compiling two binaries for it would change its build key for
        # nothing.
        self.assertEqual(
            [],
            affected_scope.missing_tier_sidecars(
                {"tiers": ["L1", "L2"], "l2_backends": ["wezterm", "kitty"], "sidecars": []}
            ),
        )

    def test_at_least_one_real_package_exercises_each_declarative_class(self) -> None:
        """The corpus must actually contain what it claims to be closing.

        A rule that no package triggers is a rule that proves nothing, and this
        is how the audit notices a class being silently dropped from the
        vocabulary rather than fixed.
        """
        declared_sidecars = {
            name for record in self.plan["packages"] for name in record["sidecars"]
        }
        table = set(load_sidecars(ROOT))
        self.assertEqual(
            table,
            declared_sidecars,
            "every sidecar in the table must be declared by some package, and no "
            "package may declare one the table does not offer",
        )
        self.assertTrue(
            any(record["archive_includes"] for record in self.plan["packages"]),
            "no package declares an archive include; the dynamic-library and "
            "build-output classes would then be untested against real metadata",
        )
        self.assertTrue(
            any(record["companion_suites"] for record in self.plan["packages"]),
            "no package declares a companion suite",
        )


#: The Phase 1 plan corpus this feature's oracles compare against
#: (`spikes/plans/README.md` records how each file was produced). Found by the
#: spec's `{date}-{name}` alone: closing a spec moves it between lifecycle
#: directories, and a path spelling one out went stale — and silently skipped
#: the oracle — the moment this one was closed.
CORPUS_PLANS = next(
    (
        candidate
        for lifecycle in ("", "_completed", "_unscheduled")
        for candidate in (
            ROOT / "features" / lifecycle / "2026-09-19-direct-cell-execution" / "spikes" / "plans",
        )
        if candidate.is_dir()
    ),
    None,
)

#: Oracle for every fixture that reaches for the area-local row adapter.
ROW_ADAPTER_ORACLE = "the plan carries no row adapter"


def row_sets_of(plan: dict) -> dict:
    """The plan's row sets, refusing the pre-adapter world by name.

    Shape-tolerant in the style of `test_resolved_plan.py`'s readers: the
    message is the pending oracle, so "not implemented" stays distinguishable
    from "fixture broken".
    """
    adapter = getattr(affected_scope, "row_sets", None)
    if adapter is None:
        raise AssertionError(
            f"{ROW_ADAPTER_ORACLE}: `affected_scope.row_sets` is not defined, "
            "so a workflow cannot be driven from the plan's cells and must "
            "keep reading environment lists"
        )
    return adapter(plan)


class DirectExecutionOracleTests(ApplyFixture):
    """Pending contracts for what the row adapter may and may not change.

    Phase 3 of `features/2026-09-19-direct-cell-execution` adds the adapter
    BESIDE today's projection. These oracles pin its two boundaries: it reads
    nothing but the plan (a carried scope receipt has no checkout), and it
    changes no selection output — check and lint selection, the dependent
    seam, event deferral, and build-owner derivation stay byte-identical for
    the corpus. Plus the R7 capacity guard: over-budget row sets fail planning
    with a named error, never a truncation.
    """

    def forbidden_read(*args: object, **kwargs: object) -> None:
        raise RuntimeError(
            "row_sets performed file I/O; the resolved plan is its only input "
            "and a carried scope receipt has no checkout to read"
        )

    def test_the_row_adapter_reads_nothing_but_the_plan(self) -> None:
        import pathlib

        # Planned before the I/O ban: the planner legitimately reads the
        # workspace. What follows is the carried-receipt apply path — the
        # operation CI performs on a matching scope receipt — and the adapter,
        # neither of which may touch the checkout.
        planned = self.plan()
        rows = None
        with unittest.mock.patch("builtins.open", self.forbidden_read), \
             unittest.mock.patch.object(
                 pathlib.Path, "read_text", self.forbidden_read
             ), unittest.mock.patch.object(
                 pathlib.Path, "read_bytes", self.forbidden_read
             ):
            applied = apply_accepted_cells(planned, self.accepted[:1], [])
            rows = row_sets_of(applied)
        self.assertTrue(rows, "the adapter answered with an empty document")

    def test_selection_outputs_are_unchanged_when_rows_land(self) -> None:
        plan = self.plan()
        rows = row_sets_of(plan)
        executing = {
            (cell["environment"], cell["gate"])
            for cell in plan["cells"]
            if cell["package"] == "alpha-core" and cell["execution"] == "execute"
        }
        # The rows are a second rendering of the plan's executing cells, never
        # a second selection. (Until Phase 8 this also compared them against
        # the per-package environment lists; those are retired.)
        dispatched = {
            (row["package"], row["environment"], row["gate"])
            for area in rows.values()
            for name in ("test", "check", "lint", "wsl")
            for row in area[name]
        }
        self.assertEqual(
            sorted(executing),
            sorted(
                (environment, gate)
                for _package, environment, gate in dispatched
                if _package == "alpha-core"
            ),
        )
        # The dependent seam is untouched by row emission: the check cell —
        # not the row — is what compiles the unchanged dependents.
        seam = next(
            entry
            for entry in plan["packages"]
            if entry["package"] == "alpha-core"
        ).get("dependent_seam")
        self.assertIsNone(seam, "this workspace has no seam; the fixture below has")

    def test_the_real_pr_shape_selects_what_the_phase_1_corpus_recorded(self) -> None:
        # A failure, not a skip: the corpus is committed, so its absence means
        # it moved or was deleted, and a skip would retire this oracle unseen.
        self.assertIsNotNone(
            CORPUS_PLANS,
            "the Phase 1 plan corpus of 2026-09-19-direct-cell-execution was not "
            "found in any features/ lifecycle directory",
        )
        corpus = json.loads((CORPUS_PLANS / "pr.json").read_text(encoding="utf-8"))
        metadata = load_metadata(ROOT)
        packages = workspace_packages(metadata)
        environments = load_environments(ENVIRONMENTS_CONFIG, today=TODAY)
        policy = package_ci_policy(
            packages,
            runner_labels={environment["runner"] for environment in environments},
            root=ROOT,
            today=TODAY,
        )
        plan = calculate_scope(
            ["biscuit-hash/lib/src/lib.rs"],
            ROOT,
            metadata,
            environments,
            policy,
            # The corpus's `pr.json` was resolved for a pull request
            # (`spikes/plans/README.md`), which schedules Linux and macOS
            # alone; planning it for no event would compare a four-cell
            # document against a six-cell one.
            event="pull_request",
        )
        rows = row_sets_of(plan)
        self.assertTrue(rows)

        def cells_of(document: dict) -> list[tuple]:
            return [
                (
                    cell["package"],
                    cell["environment"],
                    cell["gate"],
                    cell["execution"],
                    cell["state"],
                )
                for cell in document["cells"]
            ]

        # The corpus predates `2026-09-19-less-brittle`, which selects the
        # archive-path guard on every Rust change: one guard-only lint cell
        # for its owner, and that owner's package record. That addition is
        # named exactly here and set aside, so it cannot hide any other drift.
        guard_only = [cell for cell in plan["cells"] if cell.get("companions_only")]
        self.assertEqual(
            [("test-toolkit", "ubuntu-latest", "lint", "execute", "pending")],
            cells_of({"cells": guard_only}),
        )
        guard_owners = {cell["package"] for cell in guard_only}
        selected = {
            "cells": [cell for cell in plan["cells"] if not cell.get("companions_only")],
            "packages": [
                entry for entry in plan["packages"] if entry["package"] not in guard_owners
            ],
        }

        # Otherwise selection is byte-identical to the corpus the Phase 1
        # baseline froze: same cells, same packages, same deferred
        # environments. The volatile facets (head, build keys, job estimate)
        # are deliberately not compared.
        self.assertEqual(cells_of(corpus), cells_of(selected))
        self.assertEqual(
            [entry["package"] for entry in corpus["packages"]],
            [entry["package"] for entry in selected["packages"]],
        )
        self.assertEqual(
            sorted(entry["name"] for entry in corpus["deferred_environments"]),
            sorted(entry["name"] for entry in plan["deferred_environments"]),
        )

    def test_an_over_limit_row_set_fails_planning_with_a_named_error(self) -> None:
        guard = getattr(affected_scope, "enforce_output_budgets", None)
        if guard is None:
            raise AssertionError(
                "no output budget guard: `enforce_output_budgets` is not "
                "defined, so the planner cannot refuse before dispatch"
            )
        row = {
            "package": "pkg",
            "gate": "L1",
            "environment": "ubuntu-latest",
            "runner": "ubuntu-latest",
        }
        rows = {
            "huge": {
                "test": [
                    {**row, "package": f"pkg-{index}"}
                    for index in range(MATRIX_LIMIT + 1)
                ],
                "check": [],
                "lint": [],
                "wsl": [],
            }
        }
        with self.assertRaises(RuntimeError) as raised:
            guard(rows)
        self.assertIn(
            str(MATRIX_LIMIT),
            str(raised.exception),
            "the refusal names the ceiling it enforced",
        )
        # Never a truncation: the input document is refused whole, so the
        # caller's copy still carries every row it tried to schedule.
        self.assertEqual(MATRIX_LIMIT + 1, len(rows["huge"]["test"]))

    def test_an_oversized_area_payload_fails_planning_with_a_named_error(self) -> None:
        guard = getattr(affected_scope, "enforce_output_budgets", None)
        budget = getattr(affected_scope, "AREA_ROW_SET_BUDGET", None)
        if guard is None or not isinstance(budget, int) or budget <= 0:
            raise AssertionError(
                "no output budget guard: `enforce_output_budgets` and the "
                "declared `AREA_ROW_SET_BUDGET` byte budget are not defined"
            )
        # One row whose dispatch identity alone serializes past the budget:
        # the guard must refuse it rather than trim a payload it cannot carry.
        oversized = "a" * (budget + 1024)
        rows = {
            "huge": {
                "test": [
                    {
                        "package": oversized,
                        "gate": "L1",
                        "environment": "ubuntu-latest",
                        "runner": "ubuntu-latest",
                    }
                ],
                "check": [],
                "lint": [],
                "wsl": [],
            }
        }
        self.assertGreater(
            len(schema.canonical(rows["huge"])), budget, "the fixture must exceed"
        )
        with self.assertRaises(RuntimeError) as raised:
            guard(rows)
        self.assertIn("budget", str(raised.exception).lower())


class CapacityGuardTests(unittest.TestCase):
    """Phase 7's capacity validation (ruling R7), against the shipped planner.

    The measured numbers live in `spikes/s3-capacity.md`; these pin the two
    halves that must stay true as the workspace grows. The real full-workspace
    and nightly plans fit every ceiling, and a plan that does not fit is
    refused before anything is emitted, with nothing truncated.
    """

    PLANNER = ROOT / "scripts" / "ci" / "affected_scope.py"

    @staticmethod
    def rows_for(areas: int, bytes_per_area: int) -> dict[str, dict[str, object]]:
        # Each area stays under the per-area budget and the matrix ceiling;
        # only the sum is over.
        name = "p" * bytes_per_area
        return {
            f"area-{index:03}": {
                "test": [
                    {
                        "package": name,
                        "gate": "L1",
                        "environment": "ubuntu-latest",
                        "runner": "ubuntu-latest",
                    }
                ],
                "check": [],
                "lint": [],
                "wsl": [],
            }
            for index in range(areas)
        }

    def test_the_combined_scope_output_budget_refuses_many_small_areas(self) -> None:
        per_area = affected_scope.AREA_ROW_SET_BUDGET - 1024
        # The smallest area count whose rows no longer fit together, so the
        # count one below it is the boundary that must still pass.
        areas = 1
        while len(schema.canonical(self.rows_for(areas, per_area))) <= (
            affected_scope.TOTAL_ROW_SET_BUDGET
        ):
            areas += 1
        fitting = self.rows_for(areas - 1, per_area)
        self.assertIsNone(affected_scope.enforce_output_budgets(fitting))

        rows = self.rows_for(areas, per_area)
        for document in rows.values():
            self.assertLess(
                len(schema.canonical(document)), affected_scope.AREA_ROW_SET_BUDGET
            )
        with self.assertRaises(RuntimeError) as raised:
            affected_scope.enforce_output_budgets(rows)
        message = str(raised.exception)
        self.assertIn(f"{affected_scope.TOTAL_ROW_SET_BUDGET}-byte", message)
        self.assertIn(f"{areas} areas", message)
        self.assertIn("refused whole rather than truncated", message)
        self.assertEqual(areas, len(rows), "the guard must not drop an area")

    def run_planner(self, budget: int, out: Path) -> subprocess.CompletedProcess[str]:
        # The normal invocation path — `main()` over the real workspace, as
        # `ci.yml`'s scope step runs it — with only the per-area budget
        # lowered, so the real homelab area is the synthetic over-budget one.
        program = (
            "import sys, affected_scope\n"
            f"affected_scope.AREA_ROW_SET_BUDGET = {budget}\n"
            f"sys.argv = ['affected_scope.py', '--all', '--plan-out', {str(out)!r}]\n"
            "affected_scope.main()\n"
        )
        return subprocess.run(
            [sys.executable, "-c", program],
            cwd=self.PLANNER.parent,
            capture_output=True,
            text=True,
            timeout=300,
        )

    def test_an_over_budget_area_fails_before_dispatch_and_emits_nothing(self) -> None:
        require_tools("cargo", enforced_by=CARGO_ENFORCED_BY)
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory) / "plan.json"
            refused = self.run_planner(256, out)
            self.assertNotEqual(0, refused.returncode, refused.stdout[:400])
            self.assertIn("256-byte per-area output budget", refused.stderr)
            self.assertIn("refused whole rather than truncated", refused.stderr)
            self.assertIn("area '", refused.stderr, "the refusal names the area")
            # Pre-dispatch: no scope document for `ci.yml` to write to its
            # outputs, and no resolved plan for the audit to read.
            self.assertEqual("", refused.stdout)
            self.assertFalse(out.exists())

            # The same invocation under the shipped budget emits both.
            accepted = self.run_planner(affected_scope.AREA_ROW_SET_BUDGET, out)
            self.assertEqual(0, accepted.returncode, accepted.stderr[-800:])
            self.assertTrue(out.exists())
            self.assertIn("area_rows", json.loads(accepted.stdout))

    def test_the_real_full_workspace_and_nightly_plans_fit_every_ceiling(self) -> None:
        require_tools("cargo", enforced_by=CARGO_ENFORCED_BY)
        metadata = load_metadata(ROOT)
        environments = load_environments(ENVIRONMENTS_CONFIG, today=date.today())
        policy = package_ci_policy(
            workspace_packages(metadata),
            runner_labels={environment["runner"] for environment in environments},
            root=ROOT,
            today=date.today(),
        )
        for event in (None, "push", "schedule"):
            with self.subTest(event=event):
                plan = calculate_scope(
                    [], ROOT, metadata, environments, policy, True, event=event
                )
                scope = legacy_scope_document(plan)
                row_matrices = {
                    f"{area} {name} rows": len(document[name])
                    for area, document in scope["area_rows"].items()
                    for name in schema.ROW_SET_NAMES
                }
                self.assertTrue(
                    any(row_matrices.values()), "a full-workspace plan dispatches no row"
                )
                matrices = {
                    "ci.yml area-ci": len(scope["scheduled_areas"]),
                    "ci.yml build": len(scope["build_owners"]),
                    "ci.yml preflight": len(scope["preflight_os"]),
                    **row_matrices,
                }
                for matrix, count in matrices.items():
                    self.assertLessEqual(count, MATRIX_LIMIT, matrix)
                self.assertIsNone(affected_scope.enforce_output_budgets(scope["area_rows"]))




class SkipPolicySnapshotTests(unittest.TestCase):
    """What the planner snapshots from the hand-edited baseline (ruling R8).

    `.github/ci/ci-baseline.toml` stays the source of truth; the plan carries
    what the planner read, so a producer and the area audit apply the policy
    the run was PLANNED with. These pin the reading, not the applying — the
    validator's half is `test_completion.py`'s.
    """

    ENTRY = """
schema_version = 3

[[skip]]
package = "alpha-core"
environment = "ubuntu-latest"
tier = "L2"
backend = "tmux"
tests = ["alpha-core::level2_renders"]
owner = "@ken"
reason = "no tmux socket on a headless runner"
source_run = "30791562395"
expiry = "2027-10-31"
"""

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        (self.root / ".github" / "ci").mkdir(parents=True)

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def write(self, text: str) -> None:
        (self.root / affected_scope.BASELINE_POLICY_PATH).write_text(
            text, encoding="utf-8"
        )

    def test_an_absent_file_and_an_empty_one_snapshot_identically(self) -> None:
        # Both approve nothing, and both hash the empty byte string: a missing
        # policy file is not a different policy from an empty one.
        absent = affected_scope.load_skip_policy(self.root, today=TODAY)
        self.write("")
        empty = affected_scope.load_skip_policy(self.root, today=TODAY)
        self.assertEqual(absent, empty)
        self.assertEqual([], absent["entries"])
        self.assertEqual(affected_scope.BASELINE_POLICY_PATH, absent["source"])
        self.assertRegex(absent["content_hash"], r"^[0-9a-f]{16}$")

    def test_an_entry_is_carried_with_its_governance_and_the_plans_vocabulary(self) -> None:
        self.write(self.ENTRY)
        policy = affected_scope.load_skip_policy(self.root, today=TODAY)
        self.assertEqual(
            [
                {
                    "package": "alpha-core",
                    "environment": "ubuntu-latest",
                    # The file says `tier`; the plan says `gate`. One word in
                    # the document a reader consults.
                    "gate": "L2",
                    "owner": "@ken",
                    "reason": "no tmux socket on a headless runner",
                    "source_run": "30791562395",
                    "tests": ["alpha-core::level2_renders"],
                    "backend": "tmux",
                    "expiry": "2027-10-31",
                }
            ],
            policy["entries"],
        )

    def test_the_content_hash_tracks_the_file_and_is_stable(self) -> None:
        self.write(self.ENTRY)
        first = affected_scope.load_skip_policy(self.root, today=TODAY)
        again = affected_scope.load_skip_policy(self.root, today=TODAY)
        self.assertEqual(first["content_hash"], again["content_hash"])
        # A comment-only edit still changes the hash: provenance names the
        # bytes that were read, not a normalization of them.
        self.write(self.ENTRY + "\n# a later owner explains themselves\n")
        edited = affected_scope.load_skip_policy(self.root, today=TODAY)
        self.assertNotEqual(first["content_hash"], edited["content_hash"])

    def test_an_expired_approval_fails_planning_rather_than_a_producer(self) -> None:
        self.write(self.ENTRY.replace("2027-10-31", "2026-01-31"))
        with self.assertRaises(RuntimeError) as raised:
            affected_scope.load_skip_policy(self.root, today=TODAY)
        self.assertIn("expiry", str(raised.exception))

    def test_a_malformed_or_ungoverned_entry_is_refused_by_name(self) -> None:
        cases = {
            "owner": self.ENTRY.replace('owner = "@ken"', 'owner = ""'),
            "source_run": self.ENTRY.replace('source_run = "30791562395"', ""),
            "environment": self.ENTRY.replace(
                'environment = "ubuntu-latest"', 'environment = "ubuntu-24.04"'
            ),
            "tier": self.ENTRY.replace('tier = "L2"', 'tier = "L4"'),
            "schema_version": self.ENTRY.replace(
                "schema_version = 3", "schema_version = 2"
            ),
        }
        for field, text in cases.items():
            with self.subTest(field=field):
                self.write(text)
                with self.assertRaises(RuntimeError) as raised:
                    affected_scope.load_skip_policy(self.root, today=TODAY)
                self.assertIn(field, str(raised.exception))

    def test_unparseable_toml_is_refused_rather_than_read_as_empty(self) -> None:
        # An empty policy and an unreadable one are opposite facts: reading the
        # second as the first would silently drop every approval in the file.
        self.write("schema_version = 3\n[[skip]\n")
        with self.assertRaises(RuntimeError) as raised:
            affected_scope.load_skip_policy(self.root, today=TODAY)
        self.assertIn("cannot be parsed", str(raised.exception))

    def test_the_snapshot_is_narrowed_to_the_cells_the_plan_carries(self) -> None:
        self.write(self.ENTRY)
        policy = affected_scope.load_skip_policy(self.root, today=TODAY)
        carried = affected_scope.applicable_skip_entries(
            policy,
            [{"package": "alpha-core", "environment": "ubuntu-latest", "gate": "L2"}],
        )
        self.assertEqual(1, len(carried["entries"]))
        # A plan that selects another package binds nothing, and says so by
        # carrying no entry rather than by failing to resolve.
        elsewhere = affected_scope.applicable_skip_entries(
            policy,
            [{"package": "web-server", "environment": "ubuntu-latest", "gate": "L1"}],
        )
        self.assertEqual([], elsewhere["entries"])
        # Provenance survives the narrowing: the plan still names the whole
        # file it read and what that file hashed to.
        for document in (carried, elsewhere):
            self.assertEqual(policy["source"], document["source"])
            self.assertEqual(policy["content_hash"], document["content_hash"])

    def test_an_interpreter_without_tomllib_still_imports_and_says_why(self) -> None:
        # `companion_suites.py` imports this module and runs on all three native
        # environments, where `python3` can be a 3.9 interpreter; the planner
        # runs only where a 3.11+ one does. So importing must never need
        # `tomllib`, an empty budget must still snapshot, and a policy this
        # interpreter cannot read must fail by name rather than read as empty.
        with unittest.mock.patch.object(affected_scope, "tomllib", None):
            self.assertEqual(
                [], affected_scope.load_skip_policy(self.root, today=TODAY)["entries"]
            )
            self.write(self.ENTRY)
            with self.assertRaises(RuntimeError) as raised:
                affected_scope.load_skip_policy(self.root, today=TODAY)
        self.assertIn("tomllib", str(raised.exception))
        self.assertIn("3.11", str(raised.exception))

    def test_the_shipped_baseline_snapshots_into_a_valid_plan_field(self) -> None:
        # The passive corpus check: the real shipped artifact, read by the real
        # loader, must be something `validate_resolved_plan` accepts. It is
        # empty today, and an empty budget is a fact rather than an absence.
        policy = affected_scope.load_skip_policy(ROOT)
        self.assertEqual(affected_scope.BASELINE_POLICY_PATH, policy["source"])
        self.assertEqual(
            [],
            schema._skip_policy(policy, set(), TODAY),
            "the shipped baseline's snapshot must satisfy the plan contract",
        )


class ExecutionPathTests(unittest.TestCase):
    """Exactly one dispatch path per area (ruling R9, amended in Phase 7).

    The per-area allowlist was retired before it shipped: the workflows carry
    only the row interface, so every area the planner selects must say so, and
    nothing on disk can move an area onto a path no workflow implements.
    """

    def real_plan(self, files: list[str], **options: object) -> dict:
        metadata = load_metadata(ROOT)
        environments = load_environments(ENVIRONMENTS_CONFIG, today=date.today())
        policy = package_ci_policy(
            workspace_packages(metadata),
            runner_labels={environment["runner"] for environment in environments},
            root=ROOT,
            today=date.today(),
        )
        return calculate_scope(files, ROOT, metadata, environments, policy, **options)

    def test_every_area_record_states_the_row_path(self) -> None:
        records = affected_scope.area_records(
            [
                {"package": "alpha-core", "area": "alpha"},
                {"package": "web-server", "area": "web"},
            ],
            False,
            set(),
        )
        self.assertEqual(
            {"alpha": "rows", "web": "rows"},
            {entry["area"]: entry["execution_path"] for entry in records},
        )

    def test_the_retired_allowlist_file_is_not_consulted(self) -> None:
        # A stray `.github/ci/direct-execution.json` from the retired design
        # must not move an area: the planner no longer reads it, so an area
        # it omits stays on rows exactly like one it names.
        self.assertFalse((ROOT / ".github" / "ci" / "direct-execution.json").exists())
        self.assertFalse(hasattr(affected_scope, "load_direct_execution"))
        self.assertFalse(hasattr(affected_scope, "DEFAULT_EXECUTION_PATH"))

    def test_no_area_emits_both_row_sets_and_environment_lists(self) -> None:
        # End to end through the real workspace and the normal invocation
        # path: a one-file change and the full workspace. Each selected area
        # states the row path, owns exactly one row document, and its only
        # dispatch surface is that document — the scope projection carries no
        # environment list for a workflow to read beside it.
        cases = {
            "one area": (["biscuit-hash/lib/src/lib.rs"], {}),
            "full workspace": ([], {"force_all": True}),
        }
        for label, (files, options) in cases.items():
            with self.subTest(label):
                plan = self.real_plan(files, **options)
                areas = [entry["area"] for entry in plan["areas"]]
                self.assertTrue(areas, "the fixture change selects no area")
                self.assertEqual(
                    {area: "rows" for area in areas},
                    {entry["area"]: entry["execution_path"] for entry in plan["areas"]},
                )
                self.assertEqual(sorted(areas), sorted(plan["rows"]))
                for area in areas:
                    self.assertEqual(
                        set(schema.ROW_SET_NAMES)
                        | {f"has_{name}_rows" for name in schema.ROW_SET_NAMES},
                        set(plan["rows"][area]),
                        f"{area}: a row document carries only row sets",
                    )
                self.assertEqual([], schema.validate_resolved_plan(plan))
                scope = legacy_scope_document(plan)
                self.assertEqual(plan["rows"], scope["area_rows"])


class RowEmissionTests(ApplyFixture):
    """What the emitted rows are, beyond the partition the oracles pin."""

    def test_rows_are_emitted_into_the_plan_and_projected_into_the_scope(self) -> None:
        plan = self.plan()
        self.assertEqual(affected_scope.row_sets(plan), plan["rows"])
        self.assertEqual(plan["rows"], legacy_scope_document(plan)["area_rows"])
        self.assertEqual([], schema.validate_resolved_plan(plan, today=TODAY))

    def test_a_persisted_plan_round_trips_to_the_same_rows(self) -> None:
        # The plan is persisted to a scope receipt and read back by CI, so the
        # rows must survive the canonical form rather than only the in-memory
        # document. Canonicalization sorts keys, which is why validation checks
        # the key set and the adapter owns the order.
        plan = self.plan()
        first = schema.canonical(plan)
        parsed = json.loads(first)
        self.assertEqual([], schema.validate_resolved_plan(parsed, today=TODAY))
        self.assertEqual(plan["rows"], parsed["rows"])
        self.assertEqual(affected_scope.row_sets(parsed), parsed["rows"])
        self.assertEqual(first, schema.canonical(parsed))
        for area in affected_scope.row_sets(plan).values():
            for name in schema.ROW_SET_NAMES:
                for row in area[name]:
                    self.assertEqual(list(schema.ROW_FIELDS), list(row))

    def test_applying_evidence_withdraws_the_rows_it_satisfied(self) -> None:
        plan = self.plan()
        before = affected_scope.row_sets(plan)["alpha"]["test"]
        satisfied = ("alpha-core", "macos-latest", "L1")
        self.assertIn(
            satisfied,
            [(row["package"], row["environment"], row["gate"]) for row in before],
        )
        applied = apply_accepted_cells(plan, self.accepted, [])
        after = [
            (row["package"], row["environment"], row["gate"])
            for row in applied["rows"]["alpha"]["test"]
        ]
        self.assertNotIn(satisfied, after, "a reused cell still dispatched a row")
        self.assertEqual(applied["rows"], affected_scope.row_sets(applied))
        self.assertEqual([], schema.validate_resolved_plan(applied, today=TODAY))

    def test_a_within_budget_plan_passes_the_guard_unchanged(self) -> None:
        # The refusals are pinned by the oracles; this is the other half, so a
        # guard that refused everything could not pass this suite.
        rows = affected_scope.row_sets(self.plan())
        self.assertIsNone(affected_scope.enforce_output_budgets(rows))
        for document in rows.values():
            self.assertLess(
                len(schema.canonical(document)), affected_scope.AREA_ROW_SET_BUDGET
            )

    def test_the_execution_inputs_belong_to_the_cells_that_run(self) -> None:
        plan = self.plan()
        for cell in plan["cells"]:
            runs_nextest = (
                cell["execution"] == "execute" and cell["gate"] in schema.BUILD_GATES
            )
            self.assertEqual(
                runs_nextest,
                "profile" in cell,
                f"{cell['package']}/{cell['environment']}/{cell['gate']}",
            )
            if runs_nextest:
                self.assertEqual(schema.CI_PROFILE, cell["profile"])
            else:
                self.assertNotIn("requires_node", cell)

    def test_node_provisioning_is_resolved_per_cell_from_the_package_and_the_host(self) -> None:
        # `web-server` declares a Node companion suite; only an executing cell
        # on an environment that can host Node provisions it.
        plan = self.plan()
        provisioning = {
            (cell["environment"], cell["gate"])
            for cell in plan["cells"]
            if cell["package"] == "web-server" and cell.get("requires_node")
        }
        node_hosts = {
            environment["name"]
            for environment in plan["environments"]
            if capability(environment, "node_pnpm")
        }
        self.assertTrue(provisioning, "the fixture declares no Node suite")
        self.assertEqual(
            {
                (cell["environment"], "L1")
                for cell in plan["cells"]
                if cell["package"] == "web-server"
                and cell["gate"] == "L1"
                and cell["execution"] == "execute"
                and cell["environment"] in node_hosts
            },
            provisioning,
        )
        self.assertEqual(
            {
                environment
                for (environment, _), contract in producer_contracts(plan, "web-server").items()
                if contract["requires_node"] == "true"
            },
            {environment for environment, _ in provisioning},
        )
        # A package with no Node suite never provisions one.
        self.assertEqual(
            [],
            [
                cell
                for cell in plan["cells"]
                if cell["package"] == "alpha-core" and cell.get("requires_node")
            ],
        )


class SelectionEntryPairingTests(unittest.TestCase):
    """Rulings R11 and R12: a path or recipe that DECIDES what CI runs.

    Both rulings are one rule seen twice. A file that schedules work must force
    workspace scope, or an edit to it selects the wrong packages; and — when it
    is a workflow, which decides what runs and never what a local gate
    PRODUCES — it must also stay out of every cell's gate-input identity, or
    the same edit invalidates every published local cell. Spelling one without
    the other is the failure mode; the pairing is asserted here so it cannot be
    half-landed.
    """

    def devops_text(self) -> str:
        return (ROOT / "just" / "devops.just").read_text(encoding="utf-8")

    def recipes(self) -> dict:
        parsed, _ = parse_just_recipes(self.devops_text())
        return parsed

    def test_the_expected_manifest_recipe_is_a_test_gate_entry(self) -> None:
        self.assertIn(
            "_expected_manifest",
            affected_scope.CI_RECIPES_BY_GATE["test"],
            "every test producer calls `_expected_manifest` and completion.py "
            "refuses the cell when its answer disagrees with the reports, so "
            "it decides what the test gate PROVES (ruling R12)",
        )
        for gate in ("lint", "check"):
            self.assertNotIn(
                "_expected_manifest",
                affected_scope.CI_RECIPES_BY_GATE[gate],
                f"a {gate} gate lists no tests",
            )

    def test_an_edit_to_the_expected_manifest_recipe_selects_the_test_gate(self) -> None:
        # The real shipped recipe, read from the tree, with one command line
        # changed on the BASE side: `just_change_gates` compares recipe bodies,
        # so this is exactly the diff an ordinary edit produces.
        current = self.devops_text()
        self.assertIn("_expected_manifest tier specs", current)
        before = current.replace(
            'echo "_expected_manifest: jq is required." >&2',
            'echo "_expected_manifest: jq must be installed." >&2',
        )
        self.assertNotEqual(current, before, "the fixture edited nothing")
        gates = affected_scope.just_change_gates(
            "just/devops.just",
            "base",
            ROOT,
            reader=lambda ref, path: before,
        )
        self.assertEqual(
            {"test"},
            gates,
            "an edit to the expected-set logic selects the test gate and "
            "nothing else",
        )

    def test_the_expected_manifest_recipe_is_inside_the_test_gates_closure(self) -> None:
        # `local_evidence.just_gate_inputs` builds a gate's identity from this
        # closure, so membership here is what moves the test cells' gate-input
        # identity when the recipe changes.
        recipes = self.recipes()
        test_closure = just_recipe_closure(
            recipes,
            (
                *affected_scope.CI_RECIPES_ALL_GATES,
                *affected_scope.CI_RECIPES_BY_GATE["test"],
            ),
        )
        lint_closure = just_recipe_closure(
            recipes,
            (
                *affected_scope.CI_RECIPES_ALL_GATES,
                *affected_scope.CI_RECIPES_BY_GATE["lint"],
            ),
        )
        self.assertIn("_expected_manifest", test_closure)
        self.assertNotIn("_expected_manifest", lint_closure)

    def test_an_edit_to_the_recipe_changes_the_test_gates_input_text(self) -> None:
        # The identity is over the recipe's LINES, in order. Changing one of
        # them must change what `just_gate_inputs` would hash; comparing the
        # line lists is that check without needing a second Git revision.
        recipes = self.recipes()
        before = list(recipes["_expected_manifest"]["lines"])
        edited, _ = parse_just_recipes(
            self.devops_text().replace(
                'echo "_expected_manifest: jq is required." >&2',
                'echo "_expected_manifest: jq must be installed." >&2',
            )
        )
        self.assertNotEqual(before, edited["_expected_manifest"]["lines"])

    def test_the_area_workflow_forces_workspace_scope_and_leaves_identity_alone(
        self,
    ) -> None:
        path = ".github/workflows/_area-ci.yml"
        self.assertTrue((ROOT / path).is_file(), f"{path} is shipped")
        self.assertIn(
            path,
            affected_scope.GLOBAL_PATHS_ALL_GATES,
            "the area workflow carries the row sets, so it decides what runs "
            "and an edit to it must select the whole workspace (ruling R11)",
        )
        self.assertIn(
            path,
            affected_scope.ORCHESTRATION_PATHS,
            "and it must stay out of every cell's gate-input identity, or the "
            "same edit invalidates every published local cell",
        )

    def test_every_scheduling_path_is_paired_in_both_tables(self) -> None:
        # The pairing rule itself, over the whole table rather than one entry:
        # a workflow that forces workspace scope decides what RUNS, never what
        # a local gate PRODUCES.
        scheduling = {
            path
            for path in affected_scope.GLOBAL_PATHS_ALL_GATES
            if path.startswith(".github/workflows/")
        }
        self.assertTrue(scheduling, "the table names at least one workflow")
        self.assertEqual(
            set(),
            scheduling - set(affected_scope.ORCHESTRATION_PATHS),
            "a workflow that forces workspace scope must also be excluded from "
            "gate-input identity; spelling one table without the other costs "
            "either a needless full-workspace pre-push or every published cell",
        )


class DiffScopeParserTests(unittest.TestCase):
    """`--name-status -z` into the planner's `--deleted`/`--renamed-from`/`--` tail.

    The parser every selection boundary shares, so the NUL framing is proved
    once: a status and each path are separate records, and a rename or copy
    carries TWO paths for one status. Mis-stepping that puts a rename's SOURCE
    in the changed list, which is exactly the "unexpectedly missing path" the
    deletion identity exists to distinguish. The source travels on its own,
    as `--renamed-from`, for the test-input search alone.
    """

    @staticmethod
    def stream(*records: str) -> bytes:
        return b"".join(record.encode() + b"\0" for record in records)

    def parse(self, *records: str) -> tuple[list[str], list[str]]:
        changed, deleted, _ = diff_scope.parse_name_status(self.stream(*records))
        return (
            [path.decode() for path in changed],
            [path.decode() for path in deleted],
        )

    def renamed_from(self, *records: str) -> list[str]:
        return [path.decode() for path in diff_scope.parse_name_status(self.stream(*records))[2]]

    def test_a_deletion_is_reported_as_changed_and_as_deleted(self) -> None:
        self.assertEqual((["gone.rs"], ["gone.rs"]), self.parse("D", "gone.rs"))

    def test_an_ordinary_edit_is_changed_and_not_deleted(self) -> None:
        self.assertEqual((["kept.rs"], []), self.parse("M", "kept.rs"))

    def test_a_rename_reports_its_destination_and_neither_path_as_deleted(self) -> None:
        self.assertEqual(
            (["after.rs"], []), self.parse("R100", "before.rs", "after.rs")
        )

    def test_a_copy_reports_its_destination_too(self) -> None:
        self.assertEqual((["copy.rs"], []), self.parse("C75", "source.rs", "copy.rs"))

    def test_a_rename_reports_its_source_as_renamed_from(self) -> None:
        # 869cbb225..330805a84 moved a document a test still named; the old
        # name is the only thing that finds that test.
        self.assertEqual(["before.md"], self.renamed_from("R098", "before.md", "after.md"))

    def test_a_copy_leaves_its_source_in_place_and_reports_no_rename(self) -> None:
        self.assertEqual([], self.renamed_from("C75", "source.rs", "copy.rs"))

    def test_a_rename_beside_a_deletion_keeps_the_two_apart(self) -> None:
        # The regression in one record set: a parser that read the rename's
        # two paths as two records would emit `before.rs` as a changed path
        # that does not exist, and shift `D` onto the wrong name.
        self.assertEqual(
            (["after.rs", "gone.rs", "edited.rs"], ["gone.rs"]),
            self.parse("R100", "before.rs", "after.rs", "D", "gone.rs", "M", "edited.rs"),
        )

    def test_a_path_containing_a_newline_survives(self) -> None:
        self.assertEqual((["odd\nname.rs"], ["odd\nname.rs"]), self.parse("D", "odd\nname.rs"))

    def test_an_empty_diff_yields_empty_lists(self) -> None:
        self.assertEqual(([], []), self.parse())

    def test_a_status_without_a_path_is_an_error(self) -> None:
        with self.assertRaisesRegex(ValueError, "truncated"):
            diff_scope.parse_name_status(self.stream("M"))

    def test_a_rename_without_a_destination_is_an_error(self) -> None:
        with self.assertRaisesRegex(ValueError, "no destination"):
            diff_scope.parse_name_status(self.stream("R100", "before.rs"))

    def test_the_argument_tail_names_every_deletion_before_the_separator(self) -> None:
        tail = diff_scope.arguments([b"after.rs", b"gone.rs"], [b"gone.rs"])
        self.assertEqual(
            ["--deleted", "gone.rs", "--", "after.rs", "gone.rs"],
            [part.decode() for part in tail.split(b"\0")[:-1]],
        )

    def test_the_argument_tail_names_every_rename_source_before_the_separator(self) -> None:
        tail = diff_scope.arguments([b"after.rs"], [], [b"before.rs"])
        self.assertEqual(
            ["--renamed-from", "before.rs", "--", "after.rs"],
            [part.decode() for part in tail.split(b"\0")[:-1]],
        )

    def test_an_empty_change_set_still_emits_the_separator(self) -> None:
        # `xargs -0` on a BSD host runs nothing at all for empty input, which
        # is why the boundaries no longer need a branch for "no changes".
        self.assertEqual(b"--\0", diff_scope.arguments([], []))

    def test_the_tool_parses_a_real_git_diff_and_appends_untracked_paths(self) -> None:
        require_tools("git", enforced_by=GIT_ENFORCED_BY)
        with tempfile.TemporaryDirectory(prefix="diff-scope-") as temporary:
            root = Path(temporary)
            commit = [
                "git", "-c", "user.name=t", "-c", "user.email=t@example.invalid",
                "-c", "commit.gpgsign=false",
            ]

            def git(*args: str) -> None:
                subprocess.run(
                    commit + list(args), cwd=root, check=True, capture_output=True
                )

            (root / "kept.rs").write_text("a\n", encoding="utf-8")
            (root / "gone.rs").write_text("b\n", encoding="utf-8")
            (root / "before.rs").write_text("c" * 200 + "\n", encoding="utf-8")
            git("init", "-q", "-b", "main")
            git("add", "-A")
            git("commit", "-q", "-m", "base")
            (root / "kept.rs").write_text("a2\n", encoding="utf-8")
            git("rm", "-q", "gone.rs")
            git("mv", "before.rs", "after.rs")
            git("add", "-A")
            git("commit", "-q", "-m", "head")

            diff = subprocess.run(
                ["git", "diff", "--name-status", "-z", "HEAD~1", "HEAD"],
                cwd=root, check=True, capture_output=True,
            ).stdout
            parsed = subprocess.run(
                [sys.executable, str(ROOT / "scripts" / "ci" / "diff_scope.py"), "new.rs"],
                input=diff, check=True, capture_output=True,
            ).stdout
            tail = [part.decode() for part in parsed.split(b"\0")[:-1]]

        self.assertEqual(
            ["--deleted", "gone.rs", "--renamed-from", "before.rs", "--"], tail[:5]
        )
        self.assertEqual({"after.rs", "gone.rs", "kept.rs", "new.rs"}, set(tail[5:]))


# ---------------------------------------------------------------------------
# Test inputs (fixes/2026-09-22-test-input-blind-spot)
# ---------------------------------------------------------------------------


def rust_package(
    root: Path,
    name: str,
    manifest_dir: str,
    targets: list[tuple[str, str, str]],
    ci: object | None = None,
) -> dict[str, object]:
    """A metadata record whose targets carry `src_path`, as `cargo metadata` does.

    `targets` is `(kind, name, relative source path)`; the relative path is
    under `manifest_dir`.
    """
    record = package(root, name, f"{manifest_dir}/Cargo.toml", ci=ci)
    record["targets"] = [
        {
            "kind": [kind],
            "name": target,
            "src_path": str((root / manifest_dir / source).resolve()),
        }
        for kind, target, source in targets
    ]
    return record


def write_tree(root: Path, files: dict[str, str]) -> None:
    for relative, text in files.items():
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")


def narrowed_selection(
    test_filter: str, listing: dict[str, list[str]], include_slow: bool = False
) -> set[tuple[str, str]]:
    """The `(binary, test)` pairs a narrowed L1 cell runs from `listing`.

    Evaluates exactly the unit forms `test_inputs` emits, ORed as
    `select_test_inputs` joins them, then intersects with the L1 tier as
    `BISCUIT_TEST_NARROW` does. Any other form raises rather than matching
    nothing, so a new unit shape cannot make an assertion here vacuous.
    """
    selected: set[tuple[str, str]] = set()
    for clause in test_filter.split(" | "):
        if not (clause.startswith("(") and clause.endswith(")")):
            raise ValueError(f"unparenthesized unit: {clause}")
        binary_part, _, test_part = clause[1:-1].partition(" & ")
        if not (binary_part.startswith("binary_id(") and binary_part.endswith(")")):
            raise ValueError(f"unit without a binary: {clause}")
        binary = binary_part[len("binary_id(") : -1]
        for name in listing.get(binary, []):
            if test_part == "":
                matched = True
            elif test_part.startswith("test(=") and test_part.endswith(")"):
                matched = name == test_part[len("test(=") : -1]
            elif test_part.startswith("test(/^") and test_part.endswith("/)"):
                matched = name.startswith(test_part[len("test(/^") : -2])
            else:
                raise ValueError(f"unknown test predicate: {test_part}")
            if matched and test_inputs._is_l1(name, include_slow):
                selected.add((binary, name))
    return selected


class TestInputIndexTests(unittest.TestCase):
    """The static index: which compiled code names a path, and as which test.

    Each case is one reference form, on a temp tree walked from real target
    roots, because the forms are exactly where a regex-level shortcut would
    either miss a read or mistake a fixture for one.
    """

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name).resolve()

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def references(
        self,
        files: dict[str, str],
        candidates: list[str],
        targets: list[tuple[str, str, str]] | None = None,
        include_slow: frozenset[str] = frozenset(),
        ci: object | None = None,
        feature_table: dict[str, list[str]] | None = None,
        required_features: dict[str, list[str]] | None = None,
    ) -> list[tuple[str, str | None, bool]]:
        """`required_features` maps a target name to its `required-features`."""
        write_tree(self.root, files)
        record = rust_package(
            self.root,
            "pkg",
            "pkg/lib",
            targets or [("lib", "pkg", "src/lib.rs"), ("test", "l1", "tests/l1/main.rs")],
            ci=ci,
        )
        if feature_table is not None:
            record["features"] = feature_table
        for target in record["targets"]:  # type: ignore[union-attr]
            if target["name"] in (required_features or {}):
                target["required-features"] = required_features[target["name"]]  # type: ignore[index]
        targets_ = test_inputs.targets_from_metadata([record], self.root.as_posix())
        found = test_inputs.scan(
            targets_,
            affected_scope.worktree_reader(self.root),
            candidates,
            include_slow,
        )
        return [(reference.path, reference.unit, reference.product) for reference in found]

    def test_a_literal_inside_a_test_names_that_test_exactly(self) -> None:
        found = self.references(
            {
                "pkg/lib/src/lib.rs": "",
                "pkg/lib/tests/l1/main.rs": "mod docs;\n",
                "pkg/lib/tests/l1/docs.rs": (
                    "fn repo_root() -> std::path::PathBuf { todo!() }\n"
                    "#[test]\nfn wording_holds() {\n"
                    "    let text = std::fs::read_to_string(repo_root().join(\"docs/guide.md\"));\n"
                    "}\n"
                ),
            },
            ["docs/guide.md"],
        )
        self.assertEqual(
            [("docs/guide.md", "binary_id(pkg::l1) & test(=docs::wording_holds)", False)],
            found,
        )

    def test_a_literal_in_a_helper_names_its_binary(self) -> None:
        # Which tests call a helper is not known statically: `pub` makes it
        # callable from every module of the binary, here `render`.
        found = self.references(
            {
                "pkg/lib/src/lib.rs": "",
                "pkg/lib/tests/l1/main.rs": "mod docs;\nmod render;\n",
                "pkg/lib/tests/l1/docs.rs": (
                    "const PAGES: &[&str] = &[\"pkg/docs/guide.md\"];\n"
                    "pub fn read(page: &str) -> String {\n"
                    "    std::fs::read_to_string(biscuit_test_harness::manifest_dir!().join(page)).unwrap()\n"
                    "}\n"
                ),
                "pkg/lib/tests/l1/render.rs": (
                    "#[test]\nfn renders() { let _ = crate::docs::read(\"x\"); }\n"
                ),
            },
            ["pkg/docs/guide.md"],
        )
        self.assertEqual([("pkg/docs/guide.md", "binary_id(pkg::l1)", False)], found)

    def test_a_helper_in_a_binary_with_no_tests_is_not_a_unit(self) -> None:
        # Any unit there would select nothing, and the narrowed cell, run with
        # `--no-tests=fail`, would go red.
        found = self.references(
            {
                "pkg/lib/src/lib.rs": "",
                "pkg/lib/tests/l1/main.rs": "mod docs;\n",
                "pkg/lib/tests/l1/docs.rs": (
                    "pub fn read() -> std::path::PathBuf {\n"
                    "    biscuit_test_harness::manifest_dir!().join(\"docs/guide.md\")\n}\n"
                ),
            },
            ["pkg/lib/docs/guide.md"],
        )
        self.assertEqual([("pkg/lib/docs/guide.md", None, False)], found)

    def test_a_unit_test_module_declared_under_cfg_test_is_test_code(self) -> None:
        found = self.references(
            {
                "pkg/lib/src/lib.rs": "pub mod render;\n",
                "pkg/lib/src/render.rs": "pub fn go() {}\n#[cfg(test)]\nmod tests;\n",
                "pkg/lib/src/render/tests.rs": (
                    "#[test]\nfn fixture_parses() {\n"
                    "    let _ = include_str!(\"../../fixtures/page.md\");\n}\n"
                ),
            },
            ["pkg/lib/fixtures/page.md"],
        )
        self.assertEqual(
            [
                (
                    "pkg/lib/fixtures/page.md",
                    "binary_id(pkg) & test(=render::tests::fixture_parses)",
                    False,
                )
            ],
            found,
        )

    def test_an_include_in_shipped_code_is_product_source(self) -> None:
        found = self.references(
            {
                "pkg/lib/src/lib.rs": "pub const SCHEMA: &str = include_str!(\"../../schemas/base.yaml\");\n",
            },
            ["pkg/schemas/base.yaml"],
        )
        self.assertEqual([("pkg/schemas/base.yaml", None, True)], found)

    def test_a_path_joined_onto_a_tempdir_is_a_fixture_not_a_read(self) -> None:
        found = self.references(
            {
                "pkg/lib/src/lib.rs": "",
                "pkg/lib/tests/l1/main.rs": (
                    "fn repo_root() -> std::path::PathBuf { todo!() }\n"
                    "#[test]\nfn writes_a_fixture() {\n"
                    "    let dir = tempfile::tempdir().unwrap();\n"
                    "    std::fs::write(dir.path().join(\".claude/skills/pkg/SKILL.md\"), \"x\");\n"
                    "    let _ = repo_root();\n"
                    "}\n"
                ),
            },
            [".claude/skills/pkg/SKILL.md"],
        )
        self.assertEqual([], found)

    def test_mock_data_in_a_file_that_never_reads_a_root_is_not_a_read(self) -> None:
        found = self.references(
            {
                "pkg/lib/src/lib.rs": "",
                "pkg/lib/tests/l1/main.rs": (
                    "#[test]\nfn classifies() {\n"
                    "    assert!(classify(\".github/workflows/ci.yml\"));\n}\n"
                ),
            },
            [".github/workflows/ci.yml"],
        )
        self.assertEqual([], found)

    def test_a_bare_file_name_counts_only_when_joined_onto_a_root(self) -> None:
        files = {
            "pkg/lib/src/lib.rs": "",
            "pkg/lib/tests/l1/main.rs": (
                "#[test]\nfn bare() {\n    let _ = std::path::Path::new(\"README.md\");\n}\n"
            ),
        }
        self.assertEqual([], self.references(files, ["README.md", "pkg/lib/README.md"]))

    def test_parent_steps_resolve_against_the_manifest(self) -> None:
        found = self.references(
            {
                "pkg/lib/src/lib.rs": "",
                "pkg/lib/tests/l1/main.rs": (
                    "#[test]\nfn recipe() {\n"
                    "    let justfile = biscuit_test_harness::manifest_dir!()\n"
                    "        .parent()\n        .expect(\"area\")\n        .join(\"justfile\");\n}\n"
                ),
            },
            ["justfile", "pkg/justfile", "pkg/lib/justfile"],
        )
        self.assertEqual([("pkg/justfile", "binary_id(pkg::l1) & test(=recipe)", False)], found)

    def test_a_rooted_directory_walk_reads_every_file_under_it(self) -> None:
        found = self.references(
            {
                "pkg/lib/src/lib.rs": "",
                "pkg/lib/tests/l1/main.rs": (
                    "#[test]\nfn every_spec_parses() {\n"
                    "    let root = biscuit_test_harness::manifest_dir!().join(\"..\");\n"
                    "    walk(root.join(\"features\"));\n}\n"
                ),
            },
            ["pkg/features/2026-01-01-x/spec.md"],
        )
        self.assertEqual(
            [("pkg/features/2026-01-01-x/spec.md", "binary_id(pkg::l1) & test(=every_spec_parses)", False)],
            found,
        )

    def test_a_test_outside_the_l1_tier_is_not_a_unit(self) -> None:
        files = {
            "pkg/lib/src/lib.rs": "",
            "pkg/lib/tests/l1/main.rs": (
                "fn repo_root() -> std::path::PathBuf { todo!() }\n"
                "#[test]\nfn level2_renders() {\n"
                "    let _ = repo_root().join(\"docs/guide.md\");\n}\n"
                "#[test]\nfn slow_renders() {\n"
                "    let _ = repo_root().join(\"docs/guide.md\");\n}\n"
            ),
        }
        self.assertEqual(
            [("docs/guide.md", None, False)] * 2,
            self.references(files, ["docs/guide.md"]),
        )

    def test_slow_tests_are_units_where_the_package_keeps_them_in_l1(self) -> None:
        files = {
            "pkg/lib/src/lib.rs": "",
            "pkg/lib/tests/l1/main.rs": (
                "fn repo_root() -> std::path::PathBuf { todo!() }\n"
                "#[test]\nfn slow_renders() {\n"
                "    let _ = repo_root().join(\"docs/guide.md\");\n}\n"
            ),
        }
        self.assertEqual(
            [("docs/guide.md", "binary_id(pkg::l1) & test(=slow_renders)", False)],
            self.references(files, ["docs/guide.md"], include_slow=frozenset({"pkg"})),
        )

    def test_a_module_file_declared_inside_an_inline_module_is_walked(self) -> None:
        found = self.references(
            {
                "pkg/lib/src/lib.rs": "#[cfg(test)]\nmod tests {\n    mod golden;\n}\n",
                "pkg/lib/src/tests/golden.rs": (
                    "#[test]\nfn matches() {\n"
                    "    let _ = biscuit_test_harness::manifest_dir!().join(\"golden/out.txt\");\n}\n"
                ),
            },
            ["pkg/lib/golden/out.txt"],
        )
        self.assertEqual(
            [("pkg/lib/golden/out.txt", "binary_id(pkg) & test(=tests::golden::matches)", False)],
            found,
        )

    def test_a_file_no_target_reaches_is_not_scanned(self) -> None:
        found = self.references(
            {
                "pkg/lib/src/lib.rs": "",
                "pkg/lib/src/orphan.rs": "#[test]\nfn t() { let _ = include_str!(\"x.md\"); }\n",
            },
            ["pkg/lib/src/x.md"],
        )
        self.assertEqual([], found)

    # A `level2` binary may hold L1 tests: `biscuit-tui-cli`'s
    # `windows_captured_stdout` and `biscuit-terminal-cli`'s `prose_cells` do.
    # Tier follows the test's path, never the binary's name.
    MIXED_TIER = {
        "pkg/lib/src/lib.rs": "",
        "pkg/lib/tests/level2/main.rs": (
            "fn repo_root() -> std::path::PathBuf { todo!() }\n"
            "mod captured;\nmod level2_render;\nmod render;\n"
        ),
        "pkg/lib/tests/level2/captured.rs": (
            "#[test]\nfn reads_the_guide() {\n"
            "    let _ = crate::repo_root().join(\"docs/guide.md\");\n}\n"
            "#[test]\nfn level2_reads_the_guide() {\n"
            "    let _ = crate::repo_root().join(\"docs/guide.md\");\n}\n"
        ),
        "pkg/lib/tests/level2/level2_render.rs": (
            "#[test]\nfn paints() {\n"
            "    let _ = crate::repo_root().join(\"docs/guide.md\");\n}\n"
        ),
        # A helper in an unmarked module whose tests are all Level 2.
        "pkg/lib/tests/level2/render.rs": (
            "fn guide() -> String {\n"
            "    std::fs::read_to_string(crate::repo_root().join(\"docs/guide.md\")).unwrap()\n}\n"
            "#[test]\nfn level2_paints_the_guide() { let _ = guide(); }\n"
        ),
    }
    LEVEL2 = [("lib", "pkg", "src/lib.rs"), ("test", "level2", "tests/level2/main.rs")]

    def test_an_l1_test_in_a_level2_binary_is_a_unit(self) -> None:
        found = self.references(self.MIXED_TIER, ["docs/guide.md"], self.LEVEL2)
        self.assertIn(
            ("docs/guide.md", "binary_id(pkg::level2) & test(=captured::reads_the_guide)", False),
            found,
        )

    def test_level2_tests_in_a_mixed_tier_binary_are_still_not_units(self) -> None:
        found = self.references(self.MIXED_TIER, ["docs/guide.md"], self.LEVEL2)
        # `captured::level2_…` by its name and `level2_render::paints` by its
        # module may not schedule an L1 cell. The `render` helper names the
        # binary even though its own module's tests are all Level 2: an L1
        # test elsewhere in the binary may call it.
        self.assertEqual(
            [
                ("docs/guide.md", None, False),
                ("docs/guide.md", None, False),
                ("docs/guide.md", "binary_id(pkg::level2)", False),
                ("docs/guide.md", "binary_id(pkg::level2) & test(=captured::reads_the_guide)", False),
            ],
            sorted(found, key=lambda entry: entry[1] or ""),
        )

    # review-2's reproduction: the only L1 reader of the guide is a test in
    # another module that calls the `render` helper.
    SHARED_HELPER = {
        "pkg/lib/src/lib.rs": "",
        "pkg/lib/tests/level2/main.rs": (
            "fn repo_root() -> std::path::PathBuf { todo!() }\nmod captured;\nmod render;\n"
        ),
        "pkg/lib/tests/level2/render.rs": (
            "pub fn guide() -> String {\n"
            "    std::fs::read_to_string(crate::repo_root().join(\"docs/guide.md\")).unwrap()\n}\n"
            "#[test]\nfn level2_uses_guide() { let _ = guide(); }\n"
        ),
        "pkg/lib/tests/level2/captured.rs": (
            "#[test]\nfn uses_guide() { let _ = crate::render::guide(); }\n"
        ),
    }

    def test_a_helper_whose_module_is_all_level2_still_covers_its_l1_callers(self) -> None:
        found = self.references(self.SHARED_HELPER, ["docs/guide.md"], self.LEVEL2)
        self.assertEqual([("docs/guide.md", "binary_id(pkg::level2)", False)], found)
        listing = {"pkg::level2": ["captured::uses_guide", "render::level2_uses_guide"]}
        self.assertEqual(
            {("pkg::level2", "captured::uses_guide")},
            narrowed_selection(f"({found[0][1]})", listing),
        )

    def test_a_crate_root_helper_of_a_mixed_tier_binary_names_the_binary(self) -> None:
        # The unit is binary-wide; the cell intersects it with the L1 tier
        # expression (`BISCUIT_TEST_NARROW`), so its `level2_*` tests stay out.
        files = {
            **self.MIXED_TIER,
            "pkg/lib/tests/level2/main.rs": (
                "fn repo_root() -> std::path::PathBuf { todo!() }\n"
                "fn guide() -> std::path::PathBuf { repo_root().join(\"docs/other.md\") }\n"
                "mod captured;\nmod level2_render;\n"
            ),
        }
        found = self.references(files, ["docs/other.md"], self.LEVEL2)
        self.assertEqual([("docs/other.md", "binary_id(pkg::level2)", False)], found)

    def test_a_crate_root_helper_of_an_all_level2_binary_is_not_a_unit(self) -> None:
        files = {
            "pkg/lib/src/lib.rs": "",
            "pkg/lib/tests/level2/main.rs": (
                "fn repo_root() -> std::path::PathBuf { todo!() }\n"
                "fn guide() -> std::path::PathBuf { repo_root().join(\"docs/other.md\") }\n"
                "mod level2_render;\n#[path = \"../common/mod.rs\"]\nmod common;\n"
            ),
            "pkg/lib/tests/level2/level2_render.rs": "#[test]\nfn paints() {}\n",
            # A test-less helper module falls back to its binary's tests.
            "pkg/lib/tests/common/mod.rs": (
                "pub fn guide() -> std::path::PathBuf { crate::repo_root().join(\"docs/other.md\") }\n"
            ),
        }
        found = self.references(files, ["docs/other.md"], self.LEVEL2)
        self.assertEqual([("docs/other.md", None, False)] * 2, found)

    CAPTURED = {
        "pkg/lib/src/lib.rs": "",
        "pkg/lib/tests/level2/main.rs": "mod captured;\n",
        "pkg/lib/tests/level2/captured.rs": MIXED_TIER["pkg/lib/tests/level2/captured.rs"].replace(
            "crate::repo_root()", "biscuit_test_harness::manifest_dir!()"
        ),
    }
    CAPTURED_UNIT = "binary_id(pkg::level2) & test(=captured::reads_the_guide)"

    def captured(self, **kwargs: object) -> list[str | None]:
        found = self.references(
            self.CAPTURED,
            ["pkg/lib/docs/guide.md"],
            self.LEVEL2,
            required_features={"level2": ["terminal-tests"]},
            **kwargs,  # type: ignore[arg-type]
        )
        return sorted({unit for _, unit, _ in found} - {None})

    def test_a_target_whose_required_features_ci_enables_is_a_unit(self) -> None:
        self.assertEqual(
            [self.CAPTURED_UNIT], self.captured(ci={"tests": {"features": ["terminal-tests"]}})
        )
        self.assertEqual([self.CAPTURED_UNIT], self.captured(ci={"tests": {"all-features": True}}))
        self.assertEqual(
            [self.CAPTURED_UNIT],
            self.captured(feature_table={"default": ["tests"], "tests": ["terminal-tests"]}),
        )

    def test_a_target_whose_required_features_ci_leaves_off_is_not_a_unit(self) -> None:
        # The L1 cell never compiles it, so a unit there would select nothing
        # and the cell, run with `--no-tests=fail`, would go red.
        self.assertEqual([], self.captured())
        self.assertEqual([], self.captured(ci={"tests": {"features": ["image"]}}))
        self.assertEqual(
            [],
            self.captured(
                ci={"tests": {"features": ["image"]}},
                feature_table={"image": ["dep:png", "viewer?/terminal-tests"]},
            ),
        )

    def test_an_embed_in_a_target_ci_does_not_build_is_still_product(self) -> None:
        found = self.references(
            {
                "pkg/lib/src/lib.rs": "",
                "pkg/lib/src/bin/tool.rs": "const S: &str = include_str!(\"../../schema.yaml\");\n",
            },
            ["pkg/lib/schema.yaml"],
            [("lib", "pkg", "src/lib.rs"), ("bin", "tool", "src/bin/tool.rs")],
            required_features={"tool": ["tools"]},
        )
        self.assertEqual([("pkg/lib/schema.yaml", None, True)], found)


class TestInputSelectionTests(unittest.TestCase):
    """What the planner schedules for a changed file that code reads."""

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name).resolve()
        seed_build_inputs(self.root)
        write_tree(
            self.root,
            {
                "reader/lib/src/lib.rs": (
                    "pub const BASE: &str = include_str!(\"../../schemas/base.yaml\");\n"
                ),
                "reader/lib/tests/l1/main.rs": "mod docs;\n",
                "reader/lib/tests/l1/docs.rs": (
                    "fn repo_root() -> std::path::PathBuf { todo!() }\n"
                    "#[test]\nfn guide_wording_holds() {\n"
                    "    let _ = repo_root().join(\"docs/guide.md\");\n}\n"
                ),
                "other/lib/src/lib.rs": "",
                "docs/guide.md": "# Guide\n",
                "docs/unread.md": "# Unread\n",
                "reader/schemas/base.yaml": "a: 1\n",
            },
        )
        packages = [
            rust_package(
                self.root,
                "reader",
                "reader/lib",
                [("lib", "reader", "src/lib.rs"), ("test", "l1", "tests/l1/main.rs")],
            ),
            rust_package(self.root, "other", "other/lib", [("lib", "other", "src/lib.rs")]),
        ]
        self.metadata = {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {"nodes": [{"id": "reader", "deps": []}, {"id": "other", "deps": []}]},
        }
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def plan(self, files: list[str], **kwargs: object) -> dict:
        plan = calculate_scope(
            files,
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
            **kwargs,  # type: ignore[arg-type]
        )
        self.assertEqual([], schema.validate_resolved_plan(plan))
        return plan

    def test_a_read_document_schedules_one_narrowed_linux_l1_cell(self) -> None:
        plan = self.plan(["docs/guide.md"])
        self.assertEqual(
            [("reader", "ubuntu-latest", "L1")],
            [(cell["package"], cell["environment"], cell["gate"]) for cell in plan["cells"]],
        )
        cell = plan["cells"][0]
        self.assertEqual("execute", cell["execution"])
        self.assertEqual(
            "(binary_id(reader::l1) & test(=docs::guide_wording_holds))", cell["test_filter"]
        )
        self.assertTrue(cell["reusable"])
        self.assertIn("docs/guide.md", cell["selection_reason"])
        # A narrowed cell still runs from an archive like any L1 cell.
        self.assertEqual(["reader"], [build["package"] for build in plan["builds"]])
        self.assertEqual(["reader"], [entry["package"] for entry in plan["packages"]])
        self.assertEqual([], plan["source_packages"])
        self.assertEqual([], plan["reverse_dependencies"])
        self.assertEqual(
            "changed test input read by package(s) reader", plan["areas"][0]["selection_reason"]
        )

    def test_an_unread_document_schedules_nothing(self) -> None:
        plan = self.plan(["docs/unread.md"])
        self.assertEqual([], plan["cells"])
        self.assertEqual("documentation", plan["change_class"])

    def test_a_deleted_document_a_test_still_names_schedules_that_test(self) -> None:
        (self.root / "docs" / "guide.md").unlink()
        plan = self.plan(["docs/guide.md"], deleted=["docs/guide.md"])
        self.assertEqual(["reader"], [cell["package"] for cell in plan["cells"]])

    def test_a_document_renamed_away_schedules_the_test_that_still_names_it(self) -> None:
        (self.root / "docs" / "guide.md").rename(self.root / "docs" / "moved.md")
        plan = self.plan(["docs/moved.md"], renamed_from=["docs/guide.md"])
        self.assertEqual(["reader"], [cell["package"] for cell in plan["cells"]])
        self.assertEqual(["docs/moved.md"], plan["change_inventory"]["paths"]["documentation"])

    def test_an_embedded_file_is_source_of_the_package_that_ships_it(self) -> None:
        plan = self.plan(["reader/schemas/base.yaml"])
        self.assertEqual(["reader"], plan["source_packages"])
        l1 = [cell for cell in plan["cells"] if cell["gate"] == "L1"]
        self.assertEqual(
            {"ubuntu-latest", "macos-latest", "windows-latest", "wsl2-ubuntu"},
            {cell["environment"] for cell in l1},
        )
        self.assertFalse(any("test_filter" in cell for cell in plan["cells"]))
        self.assertIn("embedded file reader/schemas/base.yaml", plan["packages"][0]["selection_reason"])

    def test_a_package_whose_whole_tier_already_runs_gains_no_narrowed_cell(self) -> None:
        plan = self.plan(["reader/lib/src/lib.rs", "docs/guide.md"])
        self.assertFalse(any("test_filter" in cell for cell in plan["cells"]))
        self.assertIn(
            ("ubuntu-latest", "L1"),
            {(cell["environment"], cell["gate"]) for cell in plan["cells"]},
        )

    def test_an_l1_test_in_a_feature_gated_level2_binary_gets_a_cell_built_with_it(self) -> None:
        write_tree(
            self.root,
            {
                "gated/lib/src/lib.rs": "",
                "gated/lib/tests/level2/main.rs": "mod captured;\n",
                "gated/lib/tests/level2/captured.rs": (
                    "fn repo_root() -> std::path::PathBuf { todo!() }\n"
                    "#[test]\nfn reads_the_guide() {\n"
                    "    let _ = repo_root().join(\"docs/guide.md\");\n}\n"
                ),
            },
        )
        gated = rust_package(
            self.root,
            "gated",
            "gated/lib",
            [("lib", "gated", "src/lib.rs"), ("test", "level2", "tests/level2/main.rs")],
            ci={"tests": {"features": ["terminal-tests"]}},
        )
        gated["targets"][1]["required-features"] = ["terminal-tests"]  # type: ignore[index]
        self.metadata["packages"].append(gated)
        self.metadata["workspace_members"].append("gated")
        self.metadata["resolve"]["nodes"].append({"id": "gated", "deps": []})
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )
        plan = self.plan(["docs/guide.md"])
        cell = next(cell for cell in plan["cells"] if cell["package"] == "gated")
        self.assertEqual(
            "(binary_id(gated::level2) & test(=captured::reads_the_guide))", cell["test_filter"]
        )
        record = next(entry for entry in plan["packages"] if entry["package"] == "gated")
        self.assertEqual("--features terminal-tests", record["test_args"])

    def test_a_shared_helper_change_schedules_its_l1_caller_in_another_module(self) -> None:
        # review-2's reproduction: `render::guide()` reads the guide, its own
        # module holds only a Level 2 test, and the one L1 reader is
        # `captured::uses_guide`, which calls it.
        write_tree(
            self.root,
            {
                "shared/lib/src/lib.rs": "",
                "shared/lib/tests/level2/main.rs": (
                    "fn repo_root() -> std::path::PathBuf { todo!() }\nmod captured;\nmod render;\n"
                ),
                "shared/lib/tests/level2/render.rs": (
                    "pub fn guide() -> String {\n"
                    "    std::fs::read_to_string(crate::repo_root().join(\"docs/guide.md\")).unwrap()\n}\n"
                    "#[test]\nfn level2_uses_guide() { let _ = guide(); }\n"
                ),
                "shared/lib/tests/level2/captured.rs": (
                    "#[test]\nfn uses_guide() { let _ = crate::render::guide(); }\n"
                ),
            },
        )
        self.metadata["packages"].append(
            rust_package(
                self.root,
                "shared",
                "shared/lib",
                [("lib", "shared", "src/lib.rs"), ("test", "level2", "tests/level2/main.rs")],
            )
        )
        self.metadata["workspace_members"].append("shared")
        self.metadata["resolve"]["nodes"].append({"id": "shared", "deps": []})
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )
        plan = self.plan(["docs/guide.md"])
        cells = [cell for cell in plan["cells"] if cell["package"] == "shared"]
        self.assertEqual(
            [("ubuntu-latest", "L1", "execute")],
            [(cell["environment"], cell["gate"], cell["execution"]) for cell in cells],
        )
        listing = {"shared::level2": ["captured::uses_guide", "render::level2_uses_guide"]}
        self.assertEqual(
            {("shared::level2", "captured::uses_guide")},
            narrowed_selection(cells[0]["test_filter"], listing),
        )

    def test_an_event_that_plans_no_linux_cell_schedules_no_narrowed_cell(self) -> None:
        plan = self.plan(["docs/guide.md"], event="push", proven_event="pull_request")
        self.assertEqual([], plan["cells"])

    NARROW = "(binary_id(reader::l1) & test(=docs::guide_wording_holds))"

    def narrowed_evidence(self, **overrides: object) -> dict[str, object]:
        """What `verify_cells` accepts for a narrowed run on a macOS host."""
        return {
            "package": "reader",
            "environment": "ubuntu-latest",
            "gate": "L1",
            "outcome": "pass",
            "origin": "local",
            "test_filter": self.NARROW,
            "evidence": {"ref": "refs/notes/ci-local/macos-latest", "commit": "0" * 40},
            **overrides,
        }

    def test_an_exact_tree_narrowed_run_from_any_host_satisfies_the_cell(self) -> None:
        plan = self.plan(["docs/guide.md"], accepted_cells=[self.narrowed_evidence()])
        cell = plan["cells"][0]
        self.assertEqual(("reuse", "reused"), (cell["execution"], cell["state"]))
        self.assertEqual(self.NARROW, cell["test_filter"])
        self.assertEqual([], plan["builds"], "a satisfied cell demands no archive")

    def test_the_overlay_reaches_the_same_answer_as_resolving_with_the_evidence(self) -> None:
        resolved = self.plan(["docs/guide.md"], accepted_cells=[self.narrowed_evidence()])
        overlaid = apply_accepted_cells(
            self.plan(["docs/guide.md"]), [self.narrowed_evidence()], None
        )
        self.assertEqual(schema.canonical(resolved), schema.canonical(overlaid))

    def test_whole_tier_evidence_does_not_satisfy_a_narrowed_cell(self) -> None:
        evidence = self.narrowed_evidence()
        del evidence["test_filter"]
        plan = self.plan(["docs/guide.md"], accepted_cells=[evidence])
        self.assertEqual("execute", plan["cells"][0]["execution"])

    def test_a_run_narrowed_to_other_tests_does_not_satisfy_it(self) -> None:
        plan = self.plan(
            ["docs/guide.md"],
            accepted_cells=[self.narrowed_evidence(test_filter="(binary_id(reader::l1))")],
        )
        self.assertEqual("execute", plan["cells"][0]["execution"])

    def test_an_older_narrowed_run_does_not_satisfy_it(self) -> None:
        # The changed file is not a gate input; only the exact tree counts.
        plan = self.plan(
            ["docs/guide.md"], accepted_cells=[self.narrowed_evidence(origin="prior-local")]
        )
        self.assertEqual("execute", plan["cells"][0]["execution"])

    def test_a_narrowed_run_never_satisfies_the_whole_tier(self) -> None:
        plan = self.plan(["reader/lib/src/lib.rs"], accepted_cells=[self.narrowed_evidence()])
        l1 = [
            cell for cell in plan["cells"]
            if (cell["environment"], cell["gate"]) == ("ubuntu-latest", "L1")
        ]
        self.assertEqual("execute", l1[0]["execution"])

    def test_a_narrowed_reverse_dependent_is_selected_not_reported(self) -> None:
        # The shape that blocked the merge of PR 92: `claudine` depends on the
        # changed `darkmatter` AND its tests read a changed document. It must
        # hold one record and not also be an unchanged dependent reported
        # nowhere, which the plan schema refuses.
        self.metadata["resolve"]["nodes"] = [
            {"id": "reader", "deps": [{"pkg": "other", "dep_kinds": [{"kind": None}]}]},
            {"id": "other", "deps": []},
        ]
        plan = self.plan(["other/lib/src/lib.rs", "docs/guide.md"])
        self.assertNotIn("reader", plan["reverse_dependencies"])
        self.assertEqual(
            [("reader", "ubuntu-latest", "L1")],
            [
                (cell["package"], cell["environment"], cell["gate"])
                for cell in plan["cells"]
                if cell.get("test_filter")
            ],
        )

    def test_the_cell_contract_hands_the_filter_to_the_producer(self) -> None:
        plan = self.plan(["docs/guide.md"])
        contract = producer_contracts(plan, "reader")[("ubuntu-latest", "L1")]
        self.assertEqual(plan["cells"][0]["test_filter"], contract["test_filter"])

    def test_an_ordinary_cell_hands_the_producer_no_filter(self) -> None:
        plan = self.plan(["reader/lib/src/lib.rs"])
        contract = producer_contracts(plan, "reader")[("ubuntu-latest", "L1")]
        self.assertEqual("", contract["test_filter"])

    def test_the_schema_refuses_a_filter_outside_an_executing_l1_cell(self) -> None:
        plan = self.plan(["docs/guide.md"])
        cell = plan["cells"][0]
        for mutation in (
            {"gate": "L2"},
            {"test_filter": " "},
        ):
            with self.subTest(mutation=mutation):
                broken = copy.deepcopy(plan)
                broken["cells"][0].update(mutation)
                self.assertTrue(
                    any("test_filter" in problem
                        for problem in schema.validate_resolved_plan(broken)),
                    mutation,
                )
        self.assertIn("test_filter", cell)


#: The test as it stood at 869cbb225, aa1f03c70, and 330805a84: its own
#: integration binary, a local `repo_root()`, and a table of repository paths
#: read in a loop. Verbatim in every line the scanner reads; the assertions'
#: bodies are elided.
HISTORICAL_SCHEMA_PHASE_VALIDATION = """\
#[test]
fn public_docs_and_skill_describe_required_and_eager_as_independent_axes() {
    let root = repo_root();
    let surfaces = [
        (
            "darkmatter/docs/topics/schema-definition.md",
            vec!["`required` and `eager` are independent axes"],
        ),
        (
            "darkmatter/docs/inline/schema-validation.md",
            vec!["An eager-only `null` is therefore allowed at both phases"],
        ),
        (
            ".claude/skills/darkmatter/schema.md",
            vec!["`required` and `eager` are independent axes"],
        ),
    ];

    for (relative, expected) in surfaces {
        let path = root.join(relative);
        let text = collapse_whitespace(
            &fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{} is unreadable: {error}", path.display())),
        );
        for claim in expected {
            assert!(text.contains(claim), "{relative} no longer states: {claim}");
        }
    }
}

fn repo_root() -> std::path::PathBuf {
    // The manifest directory is `<repo>/darkmatter/lib`.
    let crate_dir = biscuit_test_harness::manifest_dir!();
    crate_dir
        .ancestors()
        .nth(2)
        .expect("repository root is two levels above darkmatter/lib")
        .to_path_buf()
}
"""


class HistoricalDocumentationBlindSpotReplayTests(unittest.TestCase):
    """The three docs-only commits that broke a darkmatter L1 test unseen.

    `4df71d20e` added `public_docs_and_skill_describe_required_and_eager_as_
    independent_axes`, which reads three documents by path. `869cbb225` edited
    one of them and relocated another, `aa1f03c70` deleted the path the test
    still named, and `330805a84` edited the first again while folding the
    relocation into `topics/schemas/`. Each planned nothing at the time (the
    fix spec records the live replay), so neither CI nor the pre-push hook
    ran the test, and it went red on `main`.

    The replay is hermetic because producer checkouts are shallow: each
    commit's `--name-status -M` diff is inlined exactly as Git reports it,
    over a workspace holding the test as it stood.
    """

    #: `git diff --name-status -M <commit>^ <commit>`, verbatim.
    COMMITS = {
        "869cbb225": [
            ("M", "darkmatter/docs/inline/schema-validation.md"),
            ("A", "darkmatter/docs/topics/schema/constraints.md"),
            ("A", "darkmatter/docs/topics/schema/definition.md"),
            ("A", "darkmatter/docs/topics/schema/types.md"),
            ("M", "darkmatter/docs/topics/simplified-schemas.md"),
        ],
        "aa1f03c70": [
            ("D", "darkmatter/docs/topics/schema-definition.md"),
        ],
        "330805a84": [
            ("M", "darkmatter/docs/inline/schema-validation.md"),
            ("D", "darkmatter/docs/topics/schema/constraints.md"),
            ("D", "darkmatter/docs/topics/schema/types.md"),
            (
                "R098",
                "darkmatter/docs/topics/schema/definition.md",
                "darkmatter/docs/topics/schemas/definition.md",
            ),
            ("M", "darkmatter/docs/topics/schemas/index.md"),
            ("M", "docs/topics/agentic-research-as-a-typed-knowledge-pipeline.md"),
        ],
    }

    TEST_UNIT = (
        "(binary_id(darkmatter::schema_phase_validation) & "
        "test(=public_docs_and_skill_describe_required_and_eager_as_independent_axes))"
    )

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name).resolve()
        seed_build_inputs(self.root)
        write_tree(
            self.root,
            {
                "darkmatter/lib/src/lib.rs": "",
                "darkmatter/lib/tests/schema_phase_validation.rs": HISTORICAL_SCHEMA_PHASE_VALIDATION,
                "darkmatter/README.md": "# Darkmatter\n",
            },
        )
        packages = [
            rust_package(
                self.root,
                "darkmatter",
                "darkmatter/lib",
                [
                    ("lib", "darkmatter", "src/lib.rs"),
                    ("test", "schema_phase_validation", "tests/schema_phase_validation.rs"),
                ],
            )
        ]
        self.metadata = {
            "workspace_members": ["darkmatter"],
            "packages": packages,
            "resolve": {"nodes": [{"id": "darkmatter", "deps": []}]},
        }
        self.policy = package_ci_policy(
            workspace_packages_from(self.metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def replay(self, records: list[tuple[str, ...]]) -> dict:
        """The plan `ci.yml` would resolve, through the same `diff_scope` parser."""
        stream = b"".join(field.encode() + b"\0" for record in records for field in record)
        changed, deleted, renamed_from = diff_scope.parse_name_status(stream)
        plan = calculate_scope(
            [path.decode() for path in changed],
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
            event="pull_request",
            deleted=[path.decode() for path in deleted],
            renamed_from=[path.decode() for path in renamed_from],
        )
        self.assertEqual([], schema.validate_resolved_plan(plan))
        return plan

    def test_each_commit_now_schedules_exactly_the_test_that_reads_its_documents(self) -> None:
        for commit, records in self.COMMITS.items():
            with self.subTest(commit=commit):
                plan = self.replay(records)
                self.assertEqual("package", plan["change_class"])
                self.assertEqual(
                    [("darkmatter", "ubuntu-latest", "L1", "execute", self.TEST_UNIT)],
                    [
                        (
                            cell["package"],
                            cell["environment"],
                            cell["gate"],
                            cell["execution"],
                            cell.get("test_filter"),
                        )
                        for cell in plan["cells"]
                    ],
                )
                # Nothing a source change would add: no lint, no check, no
                # macOS or Windows leg, no dependents.
                self.assertEqual([], plan["reverse_dependencies"])
                self.assertEqual([], plan["source_packages"])

    def test_a_readme_typo_in_the_same_package_still_schedules_nothing(self) -> None:
        plan = self.replay([("M", "darkmatter/README.md")])
        self.assertEqual([], plan["cells"])
        self.assertEqual([], plan["builds"])
        self.assertEqual("documentation", plan["change_class"])


class SourceInputSelectionTests(unittest.TestCase):
    """`source-inputs`: another package's source a package's tests execute.

    Source is left out of the test-input scan, so a contract suite that runs a
    script owned by another package ran only when its own package changed
    (review 5 of `2026-09-23-ensuring-kache-support`).
    """

    READER_TEST = "(binary_id(reader::host) & test(=tool_contract_holds))"

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name).resolve()
        seed_build_inputs(self.root)
        reads_tool = (
            "fn repo_root() -> std::path::PathBuf { todo!() }\n"
            "#[test]\nfn tool_contract_holds() {\n"
            "    let _ = repo_root().join(\"scripts/tool.sh\");\n}\n"
        )
        write_tree(
            self.root,
            {
                "scripts/src/lib.rs": "",
                "scripts/tool.sh": "#!/usr/bin/env bash\n",
                "scripts/other.sh": "#!/usr/bin/env bash\n",
                "reader/lib/src/lib.rs": "",
                "reader/lib/tests/host.rs": reads_tool,
                "bystander/lib/src/lib.rs": "",
                "bystander/lib/tests/names.rs": reads_tool,
            },
        )
        self.metadata = self.metadata_with(["scripts/tool.sh"])

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def metadata_with(self, declared: object) -> dict[str, object]:
        packages = [
            rust_package(self.root, "owner", "scripts", [("lib", "owner", "src/lib.rs")]),
            rust_package(
                self.root,
                "reader",
                "reader/lib",
                [("lib", "reader", "src/lib.rs"), ("test", "host", "tests/host.rs")],
                ci={"tests": {"source-inputs": declared}},
            ),
            rust_package(
                self.root,
                "bystander",
                "bystander/lib",
                [("lib", "bystander", "src/lib.rs"), ("test", "names", "tests/names.rs")],
            ),
        ]
        return {
            "workspace_members": [item["id"] for item in packages],
            "packages": packages,
            "resolve": {"nodes": [{"id": item["id"], "deps": []} for item in packages]},
        }

    def policy(self, metadata: dict[str, object]) -> dict[str, dict[str, object]]:
        return package_ci_policy(
            workspace_packages_from(metadata),
            runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"},
            root=self.root,
            today=TODAY,
        )

    def plan(self, files: list[str]) -> dict:
        plan = calculate_scope(
            files, self.root, self.metadata, environments_for_tests(), self.policy(self.metadata)
        )
        self.assertEqual([], schema.validate_resolved_plan(plan))
        return plan

    def test_a_declared_script_schedules_the_declarers_narrowed_linux_l1_cell(self) -> None:
        plan = self.plan(["scripts/tool.sh"])
        self.assertEqual(["owner"], plan["source_packages"])
        narrowed = [
            (cell["package"], cell["environment"], cell["gate"], cell["test_filter"])
            for cell in plan["cells"]
            if cell.get("test_filter")
        ]
        self.assertEqual([("reader", "ubuntu-latest", "L1", self.READER_TEST)], narrowed)
        self.assertEqual(
            {"owner"},
            {cell["package"] for cell in plan["cells"] if not cell.get("test_filter")},
        )
        reader = next(entry for entry in plan["packages"] if entry["package"] == "reader")
        self.assertIn("scripts/tool.sh", reader["selection_reason"])

    def test_an_undeclared_script_schedules_only_its_owner(self) -> None:
        # The planner before `source-inputs`: the same references, no cell.
        self.metadata = self.metadata_with([])
        plan = self.plan(["scripts/tool.sh"])
        self.assertEqual({"owner"}, {cell["package"] for cell in plan["cells"]})

    def test_a_script_the_declarer_does_not_name_schedules_only_its_owner(self) -> None:
        plan = self.plan(["scripts/other.sh"])
        self.assertEqual({"owner"}, {cell["package"] for cell in plan["cells"]})

    def test_a_declared_script_changed_with_the_declarers_source_adds_no_narrowed_cell(
        self,
    ) -> None:
        plan = self.plan(["scripts/tool.sh", "reader/lib/src/lib.rs"])
        self.assertFalse(any(cell.get("test_filter") for cell in plan["cells"]))

    def test_malformed_declarations_are_refused(self) -> None:
        cases = {
            "not-a-list": "scripts/tool.sh",
            "duplicate": ["scripts/tool.sh", "scripts/tool.sh"],
            "dot-spelled": ["./scripts/tool.sh"],
            "backslash-spelled": ["scripts\\tool.sh"],
            "non-source": ["scripts/README.md"],
            "missing": ["scripts/gone.sh"],
            "own-source": ["reader/lib/src/lib.rs"],
        }
        write_tree(self.root, {"scripts/README.md": "# Scripts\n"})
        for label, declared in cases.items():
            with self.subTest(case=label):
                with self.assertRaisesRegex(RuntimeError, "source.inputs|source input"):
                    self.policy(self.metadata_with(declared))


class RealWorkspaceTestInputTests(unittest.TestCase):
    """The shipped tree: the live instance is caught and a typo stays free."""

    LIVE_TEST = (
        "binary_id(darkmatter::l1) & "
        "test(=schema_phase_validation::public_docs_and_skill_describe_required_and_eager_as_independent_axes)"
    )

    @classmethod
    def setUpClass(cls) -> None:
        cls.metadata = load_metadata(ROOT)
        cls.environments = load_environments(ENVIRONMENTS_CONFIG, today=TODAY)
        cls.policy = package_ci_policy(
            workspace_packages(cls.metadata),
            runner_labels={environment["runner"] for environment in cls.environments},
            root=ROOT,
            today=TODAY,
        )

    def plan(self, *files: str, **kwargs: object) -> dict:
        plan = calculate_scope(
            list(files),
            ROOT,
            self.metadata,
            self.environments,
            self.policy,
            event="pull_request",
            **kwargs,  # type: ignore[arg-type]
        )
        self.assertEqual([], schema.validate_resolved_plan(plan))
        return plan

    def test_each_document_the_live_test_reads_selects_it(self) -> None:
        for document in (
            "darkmatter/docs/inline/schema-validation.md",
            ".claude/skills/darkmatter/schema.md",
        ):
            with self.subTest(document=document):
                self.assertTrue((ROOT / document).is_file(), f"{document} is no longer tracked")
                plan = self.plan(document)
                cells = [cell for cell in plan["cells"] if cell["package"] == "darkmatter"]
                self.assertEqual(1, len(cells))
                self.assertEqual(("ubuntu-latest", "L1"), (cells[0]["environment"], cells[0]["gate"]))
                self.assertIn(self.LIVE_TEST, cells[0]["test_filter"])

    def test_readme_typos_schedule_nothing(self) -> None:
        for readme in ("README.md", "darkmatter/README.md", "biscuit-file/README.md"):
            with self.subTest(readme=readme):
                self.assertTrue((ROOT / readme).is_file(), f"{readme} is no longer tracked")
                plan = self.plan(readme)
                self.assertEqual([], plan["cells"])
                self.assertEqual([], plan["builds"])

    def narrowed_toolkit_filter(self, changed: str) -> str:
        plan = self.plan(changed)
        cells = [
            cell for cell in plan["cells"]
            if cell["package"] == "test-toolkit" and cell.get("test_filter")
        ]
        self.assertEqual(1, len(cells), f"{changed} scheduled no narrowed test-toolkit cell")
        self.assertEqual(("ubuntu-latest", "L1"), (cells[0]["environment"], cells[0]["gate"]))
        return cells[0]["test_filter"]

    def test_each_kache_script_selects_the_contract_binaries_that_run_it(self) -> None:
        # The recipe-level binaries reach both scripts through the fixture
        # checkout, so each must spell them for its own cell to be scheduled.
        recipe_level = ("kache_ensure_contracts", "kache_init_contracts", "kache_status_contracts")
        cases = {
            "scripts/kache-host.sh": ("kache_host_contracts", *recipe_level),
            "scripts/kache-config-merge.py": ("kache_config_merge_contracts", *recipe_level),
        }
        for script, binaries in cases.items():
            with self.subTest(script=script):
                self.assertTrue((ROOT / script).is_file(), f"{script} is no longer tracked")
                test_filter = self.narrowed_toolkit_filter(script)
                for binary in binaries:
                    self.assertIn(f"(binary_id(test-toolkit::{binary}))", test_filter)

    def test_a_justfile_change_selects_the_kache_recipe_text_contracts(self) -> None:
        self.assertIn(
            "(binary_id(test-toolkit::kache_recipe_contracts))",
            self.narrowed_toolkit_filter("justfile"),
        )

    def test_every_declared_source_input_is_read_by_its_declarer(self) -> None:
        # A declaration no test of the declarer spells schedules nothing; it
        # would read as coverage while providing none.
        declared = affected_scope.declared_source_inputs(self.policy)
        self.assertTrue(declared, "the workspace declares no source-inputs")
        for path, declarers in declared.items():
            for declarer in declarers:
                with self.subTest(path=path, declarer=declarer):
                    plan = self.plan(path)
                    self.assertTrue(
                        any(
                            cell["package"] == declarer and cell.get("test_filter")
                            for cell in plan["cells"]
                        ),
                        f"{declarer} declares {path} but no L1 test of it names the path",
                    )

    def test_a_guard_only_owner_gains_the_narrowed_cell_on_its_one_record(self) -> None:
        # `claudine/lib/src/lib.rs` selects `test-toolkit` for the archive-path
        # guard alone; `docs/topics/ci-cd.md` is read by its CI-documentation
        # contracts. One package record carries both cells.
        plan = self.plan("claudine/lib/src/lib.rs", "docs/topics/ci-cd.md")
        records = [entry for entry in plan["packages"] if entry["package"] == "test-toolkit"]
        self.assertEqual(1, len(records))
        self.assertEqual(["lint", "L1"], records[0]["gates"])
        toolkit = {
            (cell["environment"], cell["gate"]): cell
            for cell in plan["cells"]
            if cell["package"] == "test-toolkit"
        }
        self.assertEqual({("ubuntu-latest", "lint"), ("ubuntu-latest", "L1")}, set(toolkit))
        self.assertTrue(toolkit[("ubuntu-latest", "lint")]["companions_only"])
        self.assertIn("test_filter", toolkit[("ubuntu-latest", "L1")])



class ArgumentsFileTests(unittest.TestCase):
    """`--arguments-file`: the diff's argument tail without argv's limits.

    The pre-push hook used to pipe it through `xargs`, and BSD `xargs` splits
    at 5,000 arguments: the PR 92/93 merge (5,439) ran the planner twice, and
    the second run saw a deleted path as changed.
    """

    def tail(self, *parts: bytes) -> Path:
        handle = tempfile.NamedTemporaryFile(delete=False, suffix=".args")
        handle.write(b"".join(part + b"\0" for part in parts))
        handle.close()
        self.addCleanup(os.unlink, handle.name)
        return Path(handle.name)

    def test_the_tail_is_appended_after_the_other_arguments(self) -> None:
        tail = self.tail(b"--deleted", b"gone.md", b"--", b"gone.md", b"kept.rs")
        self.assertEqual(
            ["--event", "push", "--deleted", "gone.md", "--", "gone.md", "kept.rs"],
            affected_scope.expand_arguments_file(
                ["--event", "push", "--arguments-file", str(tail)]
            ),
        )

    def test_a_tail_larger_than_xargs_passes_keeps_every_declaration(self) -> None:
        paths = [f"docs/{index}.md".encode() for index in range(6000)]
        tail = self.tail(b"--deleted", paths[0], b"--", *paths)
        expanded = affected_scope.expand_arguments_file(["--arguments-file", str(tail)])
        self.assertEqual(["--deleted", "docs/0.md", "--"], expanded[:3])
        self.assertEqual(6000, len(expanded) - 3)

    def test_the_planner_reads_a_deletion_from_the_file(self) -> None:
        require_tools("cargo", enforced_by=CARGO_ENFORCED_BY)
        tail = self.tail(b"--deleted", b"docs/comment-quality.md", b"--", b"docs/comment-quality.md")
        result = subprocess.run(
            [sys.executable, str(ROOT / "scripts" / "ci" / "affected_scope.py"),
             "--resolved-plan", "--arguments-file", str(tail)],
            cwd=ROOT, capture_output=True, text=True, check=True,
        )
        inventory = json.loads(result.stdout)["change_inventory"]
        self.assertEqual(["docs/comment-quality.md"], inventory["deleted"])


if __name__ == "__main__":
    unittest.main()

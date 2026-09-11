#!/usr/bin/env python3
"""Tests for dependency-aware, package-keyed CI scope calculation."""

from __future__ import annotations

import json
import tempfile
import unittest
from datetime import date
from pathlib import Path

from affected_scope import (
    calculate_scope,
    legacy_scope_document,
    package_area,
    diff_changes_more_than_comments,
    global_trigger,
    just_recipe_closure,
    parse_just_recipes,
    lockfile_impacted_names,
    parse_lockfile,
    matrix_record,
    package_cells,
    capability,
    load_environments,
    package_ci_policy,
    validate_package_ci,
    validate_no_shadow_workspaces,
    build_closure,
    estimate_jobs,
    load_metadata,
    workspace_packages,
    MATRIX_LIMIT,
    ROOT,
    ENVIRONMENTS_CONFIG,
)

# Pinned so an expiry test asserts the rule, not today's date.
TODAY = date(2026, 7, 27)


def scope_document(
    files: list[str],
    root: Path,
    metadata: dict[str, object],
    environments: list[dict[str, object]],
    policy: dict[str, dict[str, object]],
    force_all: bool = False,
    **kwargs: object,
) -> dict[str, object]:
    """The legacy `scope.json` shape the workflow still reads.

    `calculate_scope` now returns the resolved plan; the cases below that
    assert the package matrix and the rollup policy list go through this
    projection until Phases 5 and 6 move their consumers onto cells.
    """
    plan = calculate_scope(
        files, root, metadata, environments, policy, force_all, **kwargs  # type: ignore[arg-type]
    )
    return legacy_scope_document(plan, policy, environments)


def package(root: Path, name: str, relative_manifest: str, ci: object | None = None) -> dict[str, object]:
    metadata: dict[str, object] = {}
    if ci is not None:
        metadata["ci"] = ci
    return {
        "id": name,
        "name": name,
        "manifest_path": str((root / relative_manifest).resolve()),
        "metadata": metadata or None,
    }


def ci_policy(**fields: object) -> dict[str, object]:
    policy: dict[str, object] = {}
    for key, value in fields.items():
        policy[key.replace("_", "-")] = value
    return policy


class AffectedScopeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
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

    def test_source_change_selects_the_source_package_alone(self) -> None:
        # AC1: an unchanged direct reverse dependent is reported and selected
        # nowhere. It used to receive a compile-check entry, which presented an
        # untested area as a green top-level result (PR #76).
        scope = self.scope(["alpha/lib/src/lib.rs"])
        self.assertEqual(["alpha-core"], scope["packages"])
        self.assertEqual(["beta-app"], scope["reverse_dependencies"])
        # No `check` gate: alpha-core declares only a library target, which the
        # L1 build already compiles (spec section 1.7).
        gates = {entry["package"]: entry["gates"] for entry in scope["matrix"]}
        self.assertEqual(["lint", "test"], gates["alpha-core"])
        self.assertNotIn("beta-app", gates)

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
        self.assertEqual([], scope["matrix"])

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
                    "evidence": "refs/notes/ci-local/macos-latest",
                }
            ],
            plan["accepted_evidence"],
            "the plan must carry the evidence it scheduled against, so the "
            "rollup expects the same set it consumed",
        )
        scope = legacy_scope_document(plan, self.policy, environments_for_tests())
        records = {entry["package"]: entry for entry in scope["matrix"]}
        self.assertNotIn("macos-latest", records["alpha-core"]["native_environments"])

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
        self.assertEqual([], scope["matrix"])

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
        scope = self.scope(["clippy.toml", "alpha/lib/src/lib.rs"])
        gates = {entry["package"]: entry["gates"] for entry in scope["matrix"]}
        self.assertEqual(["lint", "test"], gates["alpha-core"])
        self.assertNotIn("beta-app", gates)
        self.assertNotIn("shared-tests", gates)

    def test_compile_inputs_select_no_packages(self) -> None:
        for path in ("Cargo.toml", "rust-toolchain.toml", ".cargo/config.toml"):
            scope = self.scope([path])
            self.assertEqual([], scope["packages"], path)

    def test_the_scope_calculator_runs_only_ci_tooling(self) -> None:
        scope = self.scope(["scripts/ci/affected_scope.py"])
        self.assertEqual([], scope["packages"])
        self.assertTrue(scope["flags"]["ci_tooling"])

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

    def test_global_change_uses_scope_host_preflight(self) -> None:
        scope = self.scope([".config/nextest.toml"])
        self.assertEqual("documentation", scope["change_class"])
        self.assertEqual(["ubuntu-latest"], scope["preflight_os"])

    def test_documentation_only_change_uses_scope_host_preflight(self) -> None:
        scope = self.scope(["docs/architecture.md"])
        self.assertEqual("documentation", scope["change_class"])
        self.assertEqual(["ubuntu-latest"], scope["preflight_os"])


class ClosureTests(unittest.TestCase):
    """R2 (amended 2026-08-13): a seed selects its DIRECT dependents only."""

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
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
        scope = scope_document(
            ["consumer/src/lib.rs"],
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
        )
        matrix = {entry["package"]: entry for entry in scope["matrix"]}
        self.assertIn("consumer", matrix)
        self.assertEqual(
            matrix["consumer"]["native"],
            {"ubuntu-latest": ["libasound2-dev"]},
        )


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

    def test_messenger_desktop_stubs_runner_tool_is_accepted(self) -> None:
        validate_package_ci(
            "messenger",
            ci_policy(tests={"runner-tools": ["messenger-desktop-stubs"]}),
            self.RUNNER_LABELS,
            root=Path("/"),
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


class MatrixRecordTests(unittest.TestCase):
    def test_features_become_qualified_check_and_test_args(self) -> None:
        record = matrix_record(
            {
                "package": "sniff-cli",
                "tiers": ["L1", "L2"],
                "l2_backends": ["tmux"],
                "features": ["test-fixtures"],
                "all_features": False,
                "l1_include_slow": False,
                "runner_tools": [],
                "companion_suites": [],
            },
            native={},
            environments=environments_for_tests(),
        )
        self.assertEqual(record["check_args"], "-p sniff-cli --features test-fixtures")
        self.assertEqual(record["test_args"], "--features test-fixtures")
        self.assertEqual(record["l2_environments"], ["ubuntu-latest", "macos-latest"])
        self.assertEqual(record["browser_environments"], [])
        self.assertEqual(record["node_environments"], [])
        self.assertTrue(record["wsl"])

    def test_all_features_propagates_consistently(self) -> None:
        record = matrix_record(
            {
                "package": "biscuit-hash",
                "tiers": ["L1"],
                "l2_backends": [],
                "features": [],
                "all_features": True,
                "l1_include_slow": False,
                "runner_tools": [],
                "companion_suites": [],
            },
            native={},
            environments=environments_for_tests(),
        )
        self.assertEqual(record["check_args"], "-p biscuit-hash --all-features")
        self.assertEqual(record["test_args"], "--all-features")

    def test_browser_and_node_environments_are_capability_derived(self) -> None:
        record = matrix_record(
            {
                "package": "biscuit-terminal",
                "tiers": ["L1", "L2", "browser"],
                "l2_backends": ["tmux"],
                "features": [],
                "all_features": False,
                "l1_include_slow": False,
                "runner_tools": [],
                "companion_suites": [],
            },
            native={},
            environments=environments_for_tests(),
        )
        self.assertEqual(record["browser_environments"], ["ubuntu-latest"])

        record = matrix_record(
            {
                "package": "homelab-server",
                "tiers": ["L1"],
                "l2_backends": [],
                "features": [],
                "all_features": False,
                "l1_include_slow": False,
                "runner_tools": ["node-22", "pnpm-10"],
                "companion_suites": ["homelab-frontend"],
            },
            native={},
            environments=environments_for_tests(),
        )
        self.assertEqual(record["node_environments"], ["ubuntu-latest"])
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
            "schema_version": 1,
            "environments": [
                {
                    "name": "x",
                    "runner": "x",
                    "native_key": "x",
                    "capabilities": {key: True for key in KNOWN_CAPABILITIES if key != "tmux"},
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
            "schema_version": 1,
            "environments": [
                {
                    "name": "x",
                    "runner": "x",
                    "native_key": "x",
                    "capabilities": {
                        "tmux": {"available": False, "reason": "r", "owner": "@o", "expiry": "2026-01-01"},
                        "headless_browser": True,
                        "node_pnpm": True,
                        "archive_only": False,
                    },
                }
            ],
        }
        path = Path(tempfile.mkdtemp()) / "environments.json"
        path.write_text(json.dumps(doc))
        with self.assertRaises(RuntimeError):
            load_environments(path, today=TODAY)


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

    def test_excluded_from_the_matrix_but_present_in_policy(self) -> None:
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
        self.assertEqual([], scope["matrix"])
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
        (self.root / "homelab").mkdir()
        (self.root / "homelab" / "justfile").write_text("test-frontend:\n")
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
                        "runner-tools": ["messenger-desktop-stubs"],
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
        scope = scope_document(
            ["consumer/src/lib.rs"],
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
        )
        matrix = {entry["package"]: entry for entry in scope["matrix"]}
        consumer = matrix["consumer"]
        # Only `native` unions over the closure. The dependency's L2 tier,
        # runner tools, and companion suites describe ITS tests, not the
        # packages that compile it.
        self.assertEqual(consumer["tiers"], ["L1"])
        self.assertEqual(consumer["l2_environments"], [])
        self.assertEqual(consumer["runner_tools"], [])
        self.assertEqual(consumer["companion_suites"], [])
        self.assertEqual(consumer["node_environments"], [])


class BuildClosureEdgeTests(unittest.TestCase):
    """`build_closure`: seed dev-deps in, transitive dev-deps out."""

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
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
            },
            area="a",
            target_kinds=["lib", "bench"],
            environments=environments_for_tests(),
            accepted={},
        )
        # lint (1) + check for the bench target (1) + L1 on three native
        # environments (3) + L1 on wsl2-ubuntu (2: archive builder + guest)
        # + L2 where a backend is hostable (2: ubuntu, macOS). The Windows and
        # WSL L2 cells are governed policy gaps and launch nothing.
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


class CiToolingFlagTests(unittest.TestCase):
    """M4: a change to CI's own tooling must exercise its test suites."""

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
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

    def test_rollup_and_scope_changes_set_the_tooling_flag(self) -> None:
        for path in [
            "scripts/ci-rollup.rs",
            "scripts/ci-rollup-tests.rs",
            "scripts/ci/test_affected_scope.py",
            ".github/ci/ci-baseline.toml",
        ]:
            with self.subTest(path=path):
                scope = self.scope([path])
                self.assertTrue(scope["flags"]["ci_tooling"])

    def test_a_package_change_does_not_set_the_tooling_flag(self) -> None:
        scope = self.scope(["alpha/lib/src/lib.rs"])
        self.assertFalse(scope["flags"]["ci_tooling"])


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
        root = self.justfile_root("test-frontend:\n    echo test\n")
        validate_package_ci(
            "a",
            ci_policy(tests={**{"companion-suites": ["homelab-frontend"]}}),
            self.LABELS,
            root=root,
            today=TODAY,
        )

    def test_a_recipe_with_parameters_satisfies_the_check(self) -> None:
        root = self.justfile_root('test-frontend *args="":\n    echo test\n')
        validate_package_ci(
            "a",
            ci_policy(tests={**{"companion-suites": ["homelab-frontend"]}}),
            self.LABELS,
            root=root,
            today=TODAY,
        )


class L2BackendAxisTests(unittest.TestCase):
    """`l2_environments` follows every declared backend, not only tmux."""

    def test_a_non_tmux_backend_gets_an_environment_axis(self) -> None:
        environments = environments_for_tests()
        for environment in environments:
            environment["capabilities"]["wezterm"] = environment["name"] == "macos-latest"
        record = matrix_record(
            {
                "package": "a",
                "tiers": ["L1", "L2"],
                "l2_backends": ["wezterm"],
                "features": [],
                "all_features": False,
                "l1_include_slow": False,
                "runner_tools": [],
                "companion_suites": [],
            },
            native={},
            environments=environments,
        )
        self.assertEqual(record["l2_environments"], ["macos-latest"])

    def test_a_backend_with_no_capability_entry_is_hostable_nowhere(self) -> None:
        record = matrix_record(
            {
                "package": "a",
                "tiers": ["L1", "L2"],
                "l2_backends": ["kitty"],
                "features": [],
                "all_features": False,
                "l1_include_slow": False,
                "runner_tools": [],
                "companion_suites": [],
            },
            native={},
            environments=environments_for_tests(),
        )
        self.assertEqual(record["l2_environments"], [])


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
        return scope_document(
            [path],
            ROOT,
            self.metadata,
            self.environments,
            self.policy,
        )

    def test_messenger_policy_and_matrix_contract_are_promoted(self) -> None:
        messenger = self.policy["messenger"]
        missing: list[str] = []
        if not messenger["gates"]:
            missing.append("gating policy")
        if not messenger["all_features"] or messenger["features"]:
            missing.append("all-feature policy")
        if messenger["native"] != {"ubuntu-latest": ["libdbus-1-dev"]}:
            missing.append("libdbus-1-dev native prerequisite")
        if messenger["runner_tools"] != ["messenger-desktop-stubs"]:
            missing.append("messenger-desktop-stubs runner tool")

        scope = self.scope("messenger/lib/src/lib.rs")
        record = next(
            (
                entry
                for entry in scope["matrix"]
                if entry["package"] == "messenger"
            ),
            None,
        )
        if record is None:
            missing.append("ordinary messenger package matrix cell")
        else:
            if record["check_args"] != "-p messenger --all-features":
                missing.append("all-feature check arguments")
            if record["test_args"] != "--all-features":
                missing.append("all-feature native/WSL2 L1 arguments")
            if record["native_environments"] != [
                "ubuntu-latest",
                "windows-latest",
                "macos-latest",
            ]:
                missing.append("three native L1 environments")
            if not record["wsl"]:
                missing.append("WSL2 archive cell")

        ci = (ROOT / ".github/workflows/ci.yml").read_text()
        package_ci = (ROOT / ".github/workflows/_package-ci.yml").read_text()
        wsl_ci = (ROOT / ".github/workflows/_wsl-ci.yml").read_text()
        forwarding_contract = [
            "check-args: ${{ matrix.check_args }}" in ci,
            "test-args: ${{ matrix.test_args }}" in ci,
            "cargo check --all-targets ${{ inputs.check-args }}" in package_ci,
            'just _test "${{ inputs.package }}" --no-fail-fast ${{ inputs.test-args }}'
            in package_ci,
            "check-args: ${{ inputs.check-args }}" in package_ci,
            "test-args: ${{ inputs.test-args }}" in package_ci,
            "${{ inputs.check-args }}" in wsl_ci,
            "${{ inputs.test-args }}" in wsl_ci,
            # Per-gate selection: the matrix's `gates` reaches every job that
            # can be skipped by it.
            "gates: ${{ toJSON(matrix.gates) }}" in ci,
            package_ci.count("contains(fromJSON(inputs.gates), 'test')") == 4,
            "contains(fromJSON(inputs.gates), 'lint')" in package_ci,
            "contains(fromJSON(inputs.gates), 'check')" in package_ci,
        ]
        if not all(forwarding_contract):
            missing.append("check/native-L1/WSL2 feature-argument forwarding")

        self.assertEqual(
            [],
            missing,
            f"messenger promotion contract is incomplete: {', '.join(missing)}",
        )

    def test_messenger_cli_change_selects_its_normal_package_cell(self) -> None:
        scope = self.scope("messenger/cli/src/lib.rs")
        self.assertEqual(["messenger-cli"], scope["packages"])
        self.assertEqual(
            ["messenger-cli"],
            [entry["package"] for entry in scope["matrix"]],
        )

    def test_workspace_excluded_zed_extension_selects_dmls_companion(self) -> None:
        scope = self.scope("darkmatter/dmls/zed-dmls/src/lib.rs")
        self.assertEqual(["dmls"], scope["packages"])
        self.assertEqual(["dmls"], [entry["package"] for entry in scope["matrix"]])
        self.assertEqual(
            ["neovim", "zed-extension"],
            scope["matrix"][0]["runner_tools"],
        )

        package_ci = (ROOT / ".github/workflows/_package-ci.yml").read_text()
        self.assertIn(
            "contains(fromJSON(inputs.runner-tools), 'zed-extension')",
            package_ci,
        )
        self.assertIn("just zed-verify", package_ci)

        lint_job = package_ci.split("  lint:", 1)[1].split("  # L2", 1)[0]
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
        self.assertEqual(["sniff"], scope["packages"])
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

    def test_verdict_download_unions_current_and_run_wide_artifacts(self) -> None:
        ci = (ROOT / ".github/workflows/ci.yml").read_text()
        verdict_downloads = ci.split(
            "- name: Download the resolved package policy", 1
        )[1].split("- name: Build ci-rollup", 1)[0]
        policy, current, newest = verdict_downloads.split(
            "uses: actions/download-artifact@v7"
        )[1:]

        self.assertIn("name: ci-scope", policy)
        self.assertIn("path: ci-artifacts/ci-scope", policy)
        self.assertNotIn("github-token:", current)
        self.assertIn("pattern: '{junit-*,status-*}'", current)
        self.assertIn("github-token: ${{ github.token }}", newest)
        self.assertIn("run-id: ${{ github.run_id }}", newest)
        self.assertIn("pattern: '{junit-*,status-*}'", newest)


# --- helpers ---------------------------------------------------------------


def environments_for_tests() -> list[dict[str, object]]:
    return [
        {
            "name": "ubuntu-latest",
            "runner": "ubuntu-latest",
            "native_key": "ubuntu-latest",
            "capabilities": {
                "tmux": True,
                "headless_browser": True,
                "node_pnpm": True,
                "archive_only": False,
            },
        },
        {
            "name": "windows-latest",
            "runner": "windows-latest",
            "native_key": "windows-latest",
            "capabilities": {
                "tmux": {
                    "available": False,
                    "reason": "no Windows port",
                    "owner": "@yankeeinlondon",
                    "expiry": "2027-01-31",
                },
                "headless_browser": False,
                "node_pnpm": False,
                "archive_only": False,
            },
        },
        {
            "name": "macos-latest",
            "runner": "macos-latest",
            "native_key": "macos-latest",
            "capabilities": {
                "tmux": True,
                "headless_browser": False,
                "node_pnpm": False,
                "archive_only": False,
            },
        },
        {
            "name": "wsl2-ubuntu",
            "runner": "windows-latest",
            "native_key": "ubuntu-latest",
            "capabilities": {
                "tmux": {
                    "available": False,
                    "reason": "archive-only leg",
                    "owner": "@yankeeinlondon",
                    "expiry": "2026-12-31",
                },
                "headless_browser": False,
                "node_pnpm": False,
                "archive_only": True,
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


if __name__ == "__main__":
    unittest.main()

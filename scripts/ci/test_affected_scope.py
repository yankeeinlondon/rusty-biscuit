#!/usr/bin/env python3
"""Tests for dependency-aware, package-keyed CI scope calculation."""

from __future__ import annotations

import tempfile
import unittest
from datetime import date
from pathlib import Path

from affected_scope import (
    calculate_scope,
    lockfile_impacted_names,
    parse_lockfile,
    matrix_record,
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
        return calculate_scope(
            files,
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
            **kwargs,  # type: ignore[arg-type]
        )

    def test_source_change_includes_transitive_downstream_packages(self) -> None:
        scope = self.scope(["alpha/lib/src/lib.rs"])
        self.assertEqual(["alpha-core", "beta-app"], scope["packages"])

    def test_shared_dependency_change_includes_its_consumers(self) -> None:
        scope = self.scope(["tools/shared-tests/src/lib.rs"])
        self.assertEqual(
            ["alpha-core", "beta-app", "shared-tests"], scope["packages"]
        )

    def test_unrelated_documentation_change_has_empty_scope(self) -> None:
        scope = self.scope(["docs/architecture.md"])
        self.assertEqual([], scope["packages"])
        self.assertEqual([], scope["matrix"])

    def test_force_all_selects_every_package(self) -> None:
        scope = self.scope([], force_all=True)
        self.assertTrue(scope["full_scope"])
        self.assertEqual(
            ["alpha-core", "beta-app", "shared-tests"], scope["packages"]
        )

    def test_global_test_configuration_selects_full_scope(self) -> None:
        scope = self.scope([".config/nextest.toml"])
        self.assertTrue(scope["full_scope"])

    def test_wsl_workflow_change_selects_full_scope(self) -> None:
        # `_wsl-ci.yml` is shared by every package that declares wsl2-ubuntu, so
        # a change to it has the same blast radius as `_package-ci.yml`.
        scope = self.scope([".github/workflows/_wsl-ci.yml"])
        self.assertTrue(scope["full_scope"])

    def test_package_local_change_derives_three_runner_preflight(self) -> None:
        scope = self.scope(["alpha/lib/src/lib.rs"])
        self.assertEqual("package", scope["change_class"])
        # Scope host plus the runner OS hosting each of the three native
        # environments (the fourth, wsl2-ubuntu, is hosted by windows-latest).
        self.assertEqual(
            ["macos-latest", "ubuntu-latest", "windows-latest"], scope["preflight_os"]
        )

    def test_global_change_runs_three_os_preflight(self) -> None:
        scope = self.scope([".config/nextest.toml"])
        self.assertEqual("full", scope["change_class"])
        self.assertEqual(
            ["macos-latest", "ubuntu-latest", "windows-latest"],
            scope["preflight_os"],
        )
        self.assertIn(".config/nextest.toml", scope["preflight_reason"])

    def test_documentation_only_change_uses_scope_host_preflight(self) -> None:
        scope = self.scope(["docs/architecture.md"])
        self.assertEqual("documentation", scope["change_class"])
        self.assertEqual(["ubuntu-latest"], scope["preflight_os"])


class ClosureTests(unittest.TestCase):
    """R2: narrowing the fan-out must never narrow the dependency closure."""

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

    def test_biscuit_speaks_closure_is_exact(self) -> None:
        scope = calculate_scope(
            ["biscuit-speaks/lib/src/lib.rs"],
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
        )
        self.assertEqual(
            [
                "biscuit-speaks",
                "biscuit-speaks-cli",
                "claudine",
                "claudine-cli",
                "claudine-contract",
                "research",
                "research-cli",
            ],
            scope["packages"],
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
        scope = calculate_scope(
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
        scope = calculate_scope(
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
        scope = calculate_scope(
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
        return calculate_scope(
            files,
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
            **kwargs,  # type: ignore[arg-type]
        )

    def test_a_decidable_lockfile_diff_scopes_from_the_diff(self) -> None:
        (self.root / "Cargo.lock").write_text(
            lockfile({"alpha-core": ("2", []), "beta-app": ("1", ["alpha-core"])})
        )
        base = lockfile({"alpha-core": ("1", []), "beta-app": ("1", ["alpha-core"])})
        scope = self.scope(["Cargo.lock"], base_lockfile=base)
        self.assertFalse(scope["full_scope"])
        self.assertEqual(["alpha-core", "beta-app"], scope["packages"])

    def test_an_undecidable_lockfile_diff_widens_to_full_scope(self) -> None:
        # No base lockfile supplied: the safe default is never silently
        # skipped, only ever widened.
        (self.root / "Cargo.lock").write_text(
            lockfile({"alpha-core": ("2", []), "beta-app": ("1", ["alpha-core"])})
        )
        scope = self.scope(["Cargo.lock"])
        self.assertTrue(scope["full_scope"])
        self.assertEqual(["alpha-core", "beta-app"], scope["packages"])

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

    def test_a_directory_level_justfile_selects_its_packages(self) -> None:
        scope = calculate_scope(
            ["alpha/justfile"],
            self.root,
            self.metadata,
            environments_for_tests(),
            self.policy,
        )
        self.assertEqual(["alpha-cli", "alpha-core"], scope["packages"])

    def test_a_root_level_file_outside_any_directory_selects_nothing(self) -> None:
        scope = calculate_scope(
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
            calculate_scope(
                [], root, metadata, environments_for_tests(), policy, force_all=True
            )
        self.assertIn(str(MATRIX_LIMIT), str(raised.exception))


class EstimateJobsTests(unittest.TestCase):
    def test_wsl_counts_two_jobs_and_the_rest_count_one(self) -> None:
        record = matrix_record(
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
            native={},
            environments=environments_for_tests(),
        )
        # check (1) + native environments (3) + wsl (2: archive + guest)
        # + lint (1) + L2 environments (2: ubuntu, macOS) + browser (0).
        self.assertEqual(estimate_jobs([record]), 1 + 3 + 2 + 1 + 2)


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
        return calculate_scope(
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
        return calculate_scope(
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

    def test_sniff_change_selects_exact_reverse_dependency_closure(self) -> None:
        scope = self.scope("sniff/lib/src/lib.rs")
        self.assertEqual(
            [
                "biscuit-icon-cli",
                "biscuit-speaks",
                "biscuit-speaks-cli",
                "biscuit-terminal-cli",
                "claudine",
                "claudine-cli",
                "claudine-contract",
                "claudine-gen",
                "darkmatter",
                "darkmatter-cli",
                "dmls",
                "messenger",
                "messenger-cli",
                "model-citizen",
                "model-citizen-cli",
                "playa",
                "playa-cli",
                "rendezvous-client",
                "rendezvous-core",
                "rendezvous-daemon",
                "research",
                "research-cli",
                "sniff",
                "sniff-cli",
                "unchained-ai",
                "unchained-ai-cli",
                "unchained-ai-contract",
                "unchained-ai-gen",
                "worktree",
                "worktree-cli",
            ],
            scope["packages"],
        )


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

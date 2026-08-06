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

#!/usr/bin/env python3
"""Tests for dependency-aware CI scope calculation."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from affected_scope import calculate_scope, validate_area_schema


def package(root: Path, name: str, relative_manifest: str) -> dict[str, object]:
    return {
        "id": name,
        "name": name,
        "manifest_path": str(root / relative_manifest),
    }


class AffectedScopeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        self.area_config = [
            {"area": "alpha", "check_args": "-p alpha-core"},
            {"area": "beta", "check_args": "-p beta-app"},
        ]
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
                    {"id": "alpha-core", "deps": [{"pkg": "shared-tests"}]},
                    {"id": "beta-app", "deps": [{"pkg": "alpha-core"}]},
                    {"id": "shared-tests", "deps": []},
                ]
            },
        }

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def test_source_change_includes_transitive_downstream_area(self) -> None:
        scope = calculate_scope(
            ["alpha/lib/src/lib.rs"],
            self.root,
            self.metadata,
            self.area_config,
        )
        self.assertEqual(["alpha", "beta"], [area["area"] for area in scope["areas"]])
        self.assertEqual(["alpha-core", "beta-app"], scope["packages"])

    def test_shared_test_change_includes_consuming_areas(self) -> None:
        scope = calculate_scope(
            ["tools/shared-tests/src/lib.rs"],
            self.root,
            self.metadata,
            self.area_config,
        )
        self.assertEqual(["alpha", "beta"], [area["area"] for area in scope["areas"]])
        self.assertEqual(
            ["alpha-core", "beta-app", "shared-tests"], scope["packages"]
        )

    def test_area_level_change_selects_packages_owned_by_area(self) -> None:
        scope = calculate_scope(
            ["alpha/justfile"],
            self.root,
            self.metadata,
            self.area_config,
        )
        self.assertEqual(["alpha", "beta"], [area["area"] for area in scope["areas"]])

    def test_unrelated_documentation_change_has_empty_scope(self) -> None:
        scope = calculate_scope(
            ["docs/architecture.md"],
            self.root,
            self.metadata,
            self.area_config,
        )
        self.assertEqual([], scope["areas"])
        self.assertEqual([], scope["packages"])

    def test_force_all_selects_every_package_and_area(self) -> None:
        scope = calculate_scope(
            [],
            self.root,
            self.metadata,
            self.area_config,
            force_all=True,
        )
        self.assertTrue(scope["full_scope"])
        self.assertEqual(["alpha", "beta"], [area["area"] for area in scope["areas"]])
        self.assertEqual(
            ["alpha-core", "beta-app", "shared-tests"], scope["packages"]
        )

    def test_global_test_configuration_selects_full_scope(self) -> None:
        scope = calculate_scope(
            [".config/nextest.toml"],
            self.root,
            self.metadata,
            self.area_config,
        )
        self.assertTrue(scope["full_scope"])
        self.assertEqual(["alpha", "beta"], [area["area"] for area in scope["areas"]])

    def test_windows_path_separators_select_owning_area(self) -> None:
        scope = calculate_scope(
            [r"alpha\lib\src\lib.rs"],
            self.root,
            self.metadata,
            self.area_config,
        )
        self.assertEqual(["alpha", "beta"], [area["area"] for area in scope["areas"]])
        self.assertEqual("package", scope["change_class"])

    def test_package_local_change_derives_area_preflight_os(self) -> None:
        scope = calculate_scope(
            ["alpha/lib/src/lib.rs"],
            self.root,
            self.metadata,
            self.area_config,
        )
        self.assertEqual("package", scope["change_class"])
        # Scope host plus the areas' default full_os/soft_os policy — no macOS
        # penalty for a healthy package-local change.
        self.assertEqual(["ubuntu-latest", "windows-latest"], scope["preflight_os"])

    def test_global_change_runs_three_os_preflight(self) -> None:
        scope = calculate_scope(
            [".config/nextest.toml"],
            self.root,
            self.metadata,
            self.area_config,
        )
        self.assertEqual("full", scope["change_class"])
        self.assertEqual(
            ["macos-latest", "ubuntu-latest", "windows-latest"],
            scope["preflight_os"],
        )
        self.assertIn(".config/nextest.toml", scope["preflight_reason"])

    def test_kache_version_change_selects_full_scope(self) -> None:
        scope = calculate_scope(
            [".github/kache-version"],
            self.root,
            self.metadata,
            self.area_config,
        )
        self.assertTrue(scope["full_scope"])
        self.assertEqual("full", scope["change_class"])

    def test_documentation_only_change_uses_scope_host_preflight(self) -> None:
        scope = calculate_scope(
            ["docs/architecture.md"],
            self.root,
            self.metadata,
            self.area_config,
        )
        self.assertEqual("documentation", scope["change_class"])
        self.assertEqual(["ubuntu-latest"], scope["preflight_os"])


class AreaSchemaTests(unittest.TestCase):
    def test_valid_capability_policy_passes(self) -> None:
        validate_area_schema(
            [
                {
                    "area": "alpha",
                    "check_args": "-p alpha",
                    "backends": ["tmux", "wezterm"],
                    "native": {"ubuntu-latest": ["libasound2-dev"]},
                    "canary": True,
                }
            ]
        )

    def test_missing_required_field_is_rejected(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "missing required"):
            validate_area_schema([{"area": "alpha"}])

    def test_unknown_field_is_rejected(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "unknown field"):
            validate_area_schema([{"area": "alpha", "check_args": "-p alpha", "l3": True}])

    def test_unsupported_runner_os_is_rejected(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "unsupported runner OS"):
            validate_area_schema(
                [{"area": "alpha", "check_args": "-p alpha", "full_os": ["freebsd-latest"]}]
            )

    def test_unknown_backend_is_rejected(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "unknown L2 backend"):
            validate_area_schema(
                [{"area": "alpha", "check_args": "-p alpha", "backends": ["ghostty"]}]
            )

    def test_native_must_be_os_to_packages_map(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "native"):
            validate_area_schema(
                [{"area": "alpha", "check_args": "-p alpha", "native": ["libasound2-dev"]}]
            )

    def test_non_boolean_flag_is_rejected(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "must be a boolean"):
            validate_area_schema([{"area": "alpha", "check_args": "-p alpha", "l2": "yes"}])


if __name__ == "__main__":
    unittest.main()

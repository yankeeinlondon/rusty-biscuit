#!/usr/bin/env python3
"""Tests for dependency-aware CI scope calculation."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from affected_scope import calculate_scope


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


if __name__ == "__main__":
    unittest.main()

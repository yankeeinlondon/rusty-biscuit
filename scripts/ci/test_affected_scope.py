#!/usr/bin/env python3
"""Tests for dependency-aware CI scope calculation."""

from __future__ import annotations

import re
import tempfile
import unittest
from datetime import date
from pathlib import Path

from affected_scope import (
    calculate_scope,
    lockfile_impacted_names,
    parse_lockfile,
    environment_policy,
    validate_area_schema,
    validate_no_shadow_workspaces,
    validate_ownership,
)

# Pinned so an expiry test asserts the rule, not today's date.
TODAY = date(2026, 7, 27)


def excluded(area: str, **overrides: object) -> dict[str, object]:
    """A minimal, valid `"ci": false` record; overrides drop or replace fields."""
    record: dict[str, object] = {
        "area": area,
        "ci": False,
        "reason": "a stub",
        "owner": "@yankeeinlondon",
        "exclusion_class": "time-bounded",
        "expiry": "2027-01-31",
    }
    record.update(overrides)
    return {key: value for key, value in record.items() if value is not None}


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
        # Scope host plus the runner OS hosting each of the areas' default
        # environments. macOS is now one of them: it runs the real suite, so a
        # package-local change must prove macOS can bootstrap.
        self.assertEqual(
            ["macos-latest", "ubuntu-latest", "windows-latest"], scope["preflight_os"]
        )

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

    def test_wsl_workflow_change_selects_full_scope(self) -> None:
        # `_wsl-ci.yml` is shared by every area that declares `wsl2-ubuntu`, so a
        # change to it has the same blast radius as `_area-ci.yml`.
        scope = calculate_scope(
            [".github/workflows/_wsl-ci.yml"],
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
        # `area` is the only unconditionally required field; `check_args` and
        # `reason` are required by the `ci` flag and covered in OwnershipTests.
        with self.assertRaisesRegex(RuntimeError, "missing required"):
            validate_area_schema([{"check_args": "-p alpha"}])

    def test_unknown_field_is_rejected(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "unknown field"):
            validate_area_schema([{"area": "alpha", "check_args": "-p alpha", "l3": True}])

    def test_retired_full_os_field_is_rejected_by_name(self) -> None:
        # `full_os` named a RUNNER OS list. WSL2 is an environment a Windows
        # runner hosts, not a runner label, so the field was renamed rather than
        # widened — and a stale record must say so instead of landing in a
        # generic unknown-field list.
        with self.assertRaisesRegex(RuntimeError, "retired field 'full_os'.*environments"):
            validate_area_schema(
                [{"area": "alpha", "check_args": "-p alpha", "full_os": ["ubuntu-latest"]}],
                today=TODAY,
            )

    def test_retired_soft_os_field_is_rejected_by_name(self) -> None:
        # `soft_os` drove `continue-on-error`, which removed a leg from the run's
        # verdict. A record carrying it must fail loudly rather than be silently
        # ignored, and the message must say what replaces it.
        with self.assertRaisesRegex(RuntimeError, "retired field 'soft_os'.*baseline"):
            validate_area_schema(
                [{"area": "alpha", "check_args": "-p alpha", "soft_os": []}]
            )

    def test_unsupported_environment_is_rejected(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "unsupported environment"):
            validate_area_schema(
                [
                    {
                        "area": "alpha",
                        "check_args": "-p alpha",
                        "environments": ["freebsd-latest"],
                    }
                ],
                today=TODAY,
            )

    def test_unsupported_runner_os_is_rejected(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "unsupported runner OS"):
            validate_area_schema(
                [{"area": "alpha", "check_args": "-p alpha", "check_os": ["freebsd-latest"]}],
                today=TODAY,
            )

    def test_wsl_is_an_environment_but_never_a_check_runner_or_native_key(self) -> None:
        # `check_os` and `native` are keyed by RUNNER OS: a compile-check emits
        # no results, and a WSL2 guest IS Linux, so `_ensure-native-libs` reads
        # the `ubuntu-latest` package list from inside the guest.
        validate_area_schema(
            [
                {
                    "area": "alpha",
                    "check_args": "-p alpha",
                    "environments": ["ubuntu-latest", "wsl2-ubuntu"],
                }
            ],
            today=TODAY,
        )
        with self.assertRaisesRegex(RuntimeError, "'check_os' names unsupported runner OS"):
            validate_area_schema(
                [{"area": "alpha", "check_args": "-p alpha", "check_os": ["wsl2-ubuntu"]}],
                today=TODAY,
            )
        with self.assertRaisesRegex(RuntimeError, "'native' names unsupported OS"):
            validate_area_schema(
                [
                    {
                        "area": "alpha",
                        "check_args": "-p alpha",
                        "native": {"wsl2-ubuntu": ["libdbus-1-dev"]},
                    }
                ],
                today=TODAY,
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

    def test_non_boolean_node_flag_is_rejected(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "'node' must be a boolean"):
            validate_area_schema([{"area": "alpha", "check_args": "-p alpha", "node": "yes"}])

    def test_declared_node_capability_passes(self) -> None:
        validate_area_schema(
            [{"area": "alpha", "check_args": "-p alpha", "node": True}], today=TODAY
        )

    def test_node_capability_no_environment_can_host_is_rejected(self) -> None:
        # The failure mode the flag exists to prevent: `node_environments`
        # resolves empty, the recipe self-skips on every leg, and the area
        # reports PASS having run none of its JavaScript tests.
        with self.assertRaisesRegex(RuntimeError, r'"node": true.*none of which provisions pnpm'):
            validate_area_schema(
                [
                    {
                        "area": "alpha",
                        "check_args": "-p alpha",
                        "node": True,
                        "environments": ["windows-latest", "macos-latest"],
                    }
                ],
                today=TODAY,
            )


class EnvironmentPolicyTests(unittest.TestCase):
    """`environment` is not `os`, and the split must survive into the workflow."""

    def test_native_environments_are_the_ones_that_are_runner_labels(self) -> None:
        policy = environment_policy(
            {
                "area": "alpha",
                "environments": [
                    "ubuntu-latest",
                    "windows-latest",
                    "macos-latest",
                    "wsl2-ubuntu",
                ],
            }
        )
        # `wsl2-ubuntu` must never reach a `runs-on` matrix: that job runs on
        # windows-latest and executes through wsl-bash, so it would take every
        # `runner.os == 'Windows'` branch while testing Linux code.
        self.assertEqual(
            ["ubuntu-latest", "windows-latest", "macos-latest"],
            policy["native_environments"],
        )
        self.assertTrue(policy["wsl"])

    def test_wsl_runs_by_default(self) -> None:
        # WSL2 is a supported environment, not an opt-in extra: a green Linux leg
        # says nothing about the 9p boundary, WSLg capability probes, or NAT'd
        # networking that real WSL users hit.
        self.assertTrue(environment_policy({"area": "alpha"})["wsl"])

    def test_wsl_can_be_declined_per_area(self) -> None:
        policy = environment_policy(
            {"area": "alpha", "environments": ["ubuntu-latest", "windows-latest"]}
        )
        self.assertFalse(policy["wsl"])

    def test_wsl_never_becomes_a_runner_label(self) -> None:
        # A WSL job runs ON windows-latest and executes through wsl-bash. If
        # `wsl2-ubuntu` reached `runs-on`, every Windows-branch step — native
        # packages, paths, shells, cache keys — would apply to a Linux guest.
        policy = environment_policy({"area": "alpha"})
        self.assertNotIn("wsl2-ubuntu", policy["native_environments"])

    def test_default_environments_include_macos(self) -> None:
        # The macOS = compile-check-only decision was justified by "runner
        # minutes bill ~10x". The repo is public, so that is void.
        self.assertIn("macos-latest", environment_policy({"area": "alpha"})["native_environments"])

    def test_l2_environments_are_only_those_with_a_provisionable_backend(self) -> None:
        policy = environment_policy({"area": "alpha", "l2": True})
        # tmux installs on Linux (apt) and macOS (brew) and is the only backend
        # headless CI can host. Windows has no tmux port.
        self.assertEqual(["ubuntu-latest", "macos-latest"], policy["l2_environments"])

    def test_an_area_without_l2_tests_schedules_no_l2_leg(self) -> None:
        self.assertEqual([], environment_policy({"area": "alpha"})["l2_environments"])

    def test_wsl_never_appears_as_an_l2_environment(self) -> None:
        policy = environment_policy(
            {"area": "alpha", "l2": True, "environments": ["ubuntu-latest", "wsl2-ubuntu"]}
        )
        # The WSL leg runs from a nextest archive, which carries no broker binary
        # and hosts no tmux server.
        self.assertEqual(["ubuntu-latest"], policy["l2_environments"])

    def test_an_area_without_a_javascript_suite_provisions_no_node(self) -> None:
        self.assertEqual([], environment_policy({"area": "alpha"})["node_environments"])

    def test_node_is_provisioned_on_linux_only(self) -> None:
        # Not a reduction of tests within an area: the recipe is identical on
        # every environment and self-gates on the capability. The suite is jsdom
        # with no native dependency, so the other three legs would exercise
        # identical code paths for three more toolchain installs — the same
        # reasoning that already makes `lint` Linux-only.
        policy = environment_policy({"area": "alpha", "node": True})
        self.assertEqual(["ubuntu-latest"], policy["node_environments"])

    def test_wsl_never_appears_as_a_node_environment(self) -> None:
        # The WSL leg runs `just test` from a prebuilt nextest archive inside a
        # guest with no Node toolchain; the recipe skips there loudly.
        policy = environment_policy(
            {"area": "alpha", "node": True, "environments": ["ubuntu-latest", "wsl2-ubuntu"]}
        )
        self.assertEqual(["ubuntu-latest"], policy["node_environments"])

    def test_wsl_preflights_on_the_runner_that_hosts_it(self) -> None:
        root = Path(tempfile.mkdtemp())
        packages = [
            {"id": "alpha-core", "name": "alpha-core", "manifest_path": str(root / "alpha/lib/Cargo.toml")}
        ]
        metadata = {
            "workspace_members": ["alpha-core"],
            "packages": packages,
            "resolve": {"nodes": [{"id": "alpha-core", "deps": []}]},
        }
        scope = calculate_scope(
            ["alpha/lib/src/lib.rs"],
            root,
            metadata,
            [
                {
                    "area": "alpha",
                    "check_args": "-p alpha-core",
                    "environments": ["ubuntu-latest", "wsl2-ubuntu"],
                }
            ],
        )
        # Preflight runs on runner labels; `wsl2-ubuntu` resolves to its host.
        self.assertEqual(["ubuntu-latest", "windows-latest"], scope["preflight_os"])
        self.assertTrue(scope["areas"][0]["wsl"])
        self.assertEqual(["ubuntu-latest"], scope["areas"][0]["native_environments"])


class ExclusionPolicyTests(unittest.TestCase):
    """A `"ci": false` record must be owned and time-bounded (plan 3.5)."""

    def test_a_fully_declared_exclusion_passes(self) -> None:
        validate_area_schema([excluded("tabby")], today=TODAY)

    def test_exclusion_without_an_owner_is_rejected(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "must give a non-empty 'owner'"):
            validate_area_schema([excluded("tabby", owner=None)], today=TODAY)

    def test_exclusion_without_a_class_is_rejected(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "must give an 'exclusion_class'"):
            validate_area_schema([excluded("tabby", exclusion_class=None)], today=TODAY)

    def test_unknown_exclusion_class_is_rejected(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "must give an 'exclusion_class'"):
            validate_area_schema([excluded("tabby", exclusion_class="someday")], today=TODAY)

    def test_backlog_exclusion_without_an_expiry_is_rejected(self) -> None:
        # An exclusion with no end date is a permanent one wearing a temporary
        # label, which is exactly how the current 10 records accumulated.
        with self.assertRaisesRegex(RuntimeError, "must give an 'expiry'"):
            validate_area_schema([excluded("tabby", expiry=None)], today=TODAY)

    def test_lapsed_expiry_fails_loudly(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "expired on 2026-07-26"):
            validate_area_schema([excluded("tabby", expiry="2026-07-26")], today=TODAY)

    def test_expiry_on_the_boundary_day_is_still_valid(self) -> None:
        validate_area_schema([excluded("tabby", expiry="2026-07-27")], today=TODAY)

    def test_malformed_expiry_is_rejected(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "must be an ISO date"):
            validate_area_schema([excluded("tabby", expiry="soon")], today=TODAY)

    def test_capability_exclusion_is_permanent_and_takes_no_expiry(self) -> None:
        # `homelab` is the reference case: physical IoT hardware CI cannot host.
        # Nothing is going to change that, so a date would be theatre.
        validate_area_schema(
            [excluded("homelab", exclusion_class="capability", expiry=None)], today=TODAY
        )
        with self.assertRaisesRegex(RuntimeError, "permanent; drop 'expiry'"):
            validate_area_schema(
                [excluded("homelab", exclusion_class="capability")], today=TODAY
            )


class PolicyGapTests(unittest.TestCase):
    """A tier with tests and no provisionable backend is a gap, never a green skip."""

    def gap(self, **overrides: object) -> dict[str, object]:
        record: dict[str, object] = {
            "tier": "L2",
            "environments": ["windows-latest"],
            "reason": "tmux has no Windows port",
            "owner": "@yankeeinlondon",
            "expiry": "2027-01-31",
        }
        record.update(overrides)
        return {key: value for key, value in record.items() if value is not None}

    def l2_area(self, **overrides: object) -> dict[str, object]:
        # Both unprovisioned environments must be declared: Windows has no tmux
        # port at all, and the WSL2 leg runs a prebuilt archive with no broker
        # binary and no terminal server. Different causes, same obligation.
        record: dict[str, object] = {
            "area": "alpha",
            "check_args": "-p alpha",
            "l2": True,
            "backends": ["tmux"],
            "policy_gaps": [
                self.gap(),
                self.gap(
                    environments=["wsl2-ubuntu"],
                    reason="archive run carries no broker and hosts no terminal server",
                ),
            ],
        }
        record.update(overrides)
        return record

    def test_declared_gap_passes(self) -> None:
        validate_area_schema([self.l2_area()], today=TODAY)

    def test_l2_area_without_a_wsl_gap_is_rejected(self) -> None:
        # WSL2 runs the L1 suite, so an L2 tier is scheduled there too. Declaring
        # only the Windows gap leaves WSL2 exiting 0 having executed nothing.
        with self.assertRaisesRegex(RuntimeError, r"wsl2-ubuntu.*no L2 backend"):
            validate_area_schema([self.l2_area(policy_gaps=[self.gap()])], today=TODAY)

    def test_l2_area_without_a_windows_gap_is_rejected(self) -> None:
        # Windows runs the L1 suite, so an L2 tier IS scheduled there and exits 0
        # having executed nothing. That must read as POLICY GAP, not as a pass.
        with self.assertRaisesRegex(RuntimeError, r"no L2 backend.*POLICY GAP"):
            validate_area_schema([self.l2_area(policy_gaps=[])], today=TODAY)

    def test_an_area_that_never_runs_on_windows_needs_no_gap(self) -> None:
        validate_area_schema(
            [
                self.l2_area(
                    environments=["ubuntu-latest", "macos-latest"], policy_gaps=[]
                )
            ],
            today=TODAY,
        )

    def test_gap_missing_an_owner_is_rejected(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "missing required field\\(s\\): \\['owner'\\]"):
            validate_area_schema(
                [self.l2_area(policy_gaps=[self.gap(owner=None)])], today=TODAY
            )

    def test_gap_with_a_blank_owner_is_rejected(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "must give a non-empty 'owner'"):
            validate_area_schema(
                [self.l2_area(policy_gaps=[self.gap(owner="   ")])], today=TODAY
            )

    def test_lapsed_gap_expiry_fails_loudly(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "expired on 2026-07-26"):
            validate_area_schema(
                [self.l2_area(policy_gaps=[self.gap(expiry="2026-07-26")])], today=TODAY
            )

    def test_gap_for_an_environment_the_area_never_runs_is_rejected(self) -> None:
        # A gap describes a scheduled cell that cannot execute. Declaring one for
        # an unscheduled environment manufactures a row nothing produces.
        with self.assertRaisesRegex(RuntimeError, "environment\\(s\\) the area does not run"):
            validate_area_schema(
                [
                    self.l2_area(
                        environments=["ubuntu-latest", "macos-latest"],
                        policy_gaps=[self.gap()],
                    )
                ],
                today=TODAY,
            )

    def test_unknown_tier_is_rejected(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "unknown tier"):
            validate_area_schema(
                [self.l2_area(policy_gaps=[self.gap(tier="L4")])], today=TODAY
            )

    def test_unsupported_gap_environment_is_rejected(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "unsupported environment"):
            validate_area_schema(
                [self.l2_area(policy_gaps=[self.gap(environments=["freebsd-latest"])])],
                today=TODAY,
            )

    def test_gap_with_an_unknown_field_is_rejected(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "unknown field"):
            validate_area_schema(
                [self.l2_area(policy_gaps=[{**self.gap(), "backend": "tmux"}])],
                today=TODAY,
            )


class LiveAreaConfigTests(unittest.TestCase):
    """The checked-in `areas.json` must satisfy every rule above."""

    def setUp(self) -> None:
        import json

        config = Path(__file__).resolve().parents[2] / ".github" / "ci" / "areas.json"
        with config.open(encoding="utf-8") as handle:
            self.areas = json.load(handle)

    def test_checked_in_policy_validates(self) -> None:
        validate_area_schema(self.areas, today=TODAY)

    def test_every_l2_area_declares_its_backends(self) -> None:
        for area in self.areas:
            if area.get("l2"):
                self.assertTrue(
                    area.get("backends"),
                    f"{area['area']} sets l2 but declares no backends",
                )

    def test_an_area_whose_justfile_drives_pnpm_declares_the_node_capability(self) -> None:
        # Drift guard for the defect this capability was added to close. An area
        # whose canonical recipes shell out to pnpm, with nothing declaring it,
        # gets pnpm on no runner: `homelab`'s `just test` died on `pnpm: command
        # not found` after every Rust package passed, so its 22 frontend tests
        # had never once executed in CI. Declaration and usage must agree in
        # BOTH directions — an undeclared user is a silent gap, and a declared
        # non-user pays for a toolchain install that gates nothing.
        root = Path(__file__).resolve().parents[2]
        for area in self.areas:
            justfile = root / area["area"] / "justfile"
            if not justfile.exists():
                continue
            recipes = "\n".join(
                line
                for line in justfile.read_text(encoding="utf-8").splitlines()
                if not line.lstrip().startswith("#")
            )
            drives_pnpm = re.search(r"\bpnpm\b", recipes) is not None
            self.assertEqual(
                drives_pnpm,
                bool(area.get("node", False)),
                f"{area['area']}: its justfile invokes pnpm ({drives_pnpm}) but its declared "
                f"\"node\" capability is {bool(area.get('node', False))}",
            )

    def test_every_exclusion_is_owned(self) -> None:
        excluded_areas = [area for area in self.areas if area.get("ci") is False]
        self.assertEqual(10, len(excluded_areas))
        for area in excluded_areas:
            self.assertTrue(area.get("owner"), f"{area['area']} exclusion has no owner")
            # The audit found no capability-based exclusion among the ten; every
            # one is backlog, so every one carries a date.
            self.assertNotEqual("capability", area.get("exclusion_class"))
            self.assertTrue(area.get("expiry"), f"{area['area']} exclusion has no expiry")


class OwnershipTests(unittest.TestCase):
    """Every package's directory must map to a declared area (gating or not)."""

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        self.area_config = [
            {"area": "alpha", "check_args": "-p alpha-core", "ci": True},
            {"area": "tools", "reason": "test infrastructure", "ci": False},
        ]

        def package(name: str, relative_manifest: str) -> dict[str, object]:
            return {"id": name, "name": name, "manifest_path": str(self.root / relative_manifest)}

        packages = [
            package("alpha-core", "alpha/lib/Cargo.toml"),
            package("shared-tests", "tools/shared-tests/Cargo.toml"),
        ]
        self.metadata = {
            "workspace_members": [p["id"] for p in packages],
            "packages": packages,
            "resolve": {"nodes": []},
        }

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def test_gating_and_non_gating_areas_both_confer_ownership(self) -> None:
        validate_ownership(self.metadata, self.root, self.area_config)

    def test_package_under_no_declared_area_fails_by_name(self) -> None:
        self.metadata["packages"].append(
            {"id": "orphan", "name": "orphan", "manifest_path": str(self.root / "beta/app/Cargo.toml")}
        )
        self.metadata["workspace_members"].append("orphan")
        with self.assertRaisesRegex(RuntimeError, "fall under no declared area.*orphan"):
            validate_ownership(self.metadata, self.root, self.area_config)

    def test_gating_area_must_define_check_args(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "must define 'check_args'"):
            validate_area_schema([{"area": "alpha"}])

    def test_non_gating_area_must_give_a_reason(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "must give a non-empty 'reason'"):
            validate_area_schema([{"area": "tabby", "ci": False}], today=TODAY)

    def test_non_gating_area_needs_no_check_args(self) -> None:
        validate_area_schema([excluded("tabby")], today=TODAY)


class ShadowWorkspaceTests(unittest.TestCase):
    """A nested `[workspace]` must not re-declare a root workspace member."""

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        (self.root / "alpha" / "lib").mkdir(parents=True)
        (self.root / "standalone").mkdir()
        packages = [
            {
                "id": "alpha-core",
                "name": "alpha-core",
                "manifest_path": str(self.root / "alpha" / "lib" / "Cargo.toml"),
            }
        ]
        self.metadata = {
            "workspace_members": ["alpha-core"],
            "packages": packages,
            "resolve": {"nodes": []},
        }

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def test_no_nested_workspace_passes(self) -> None:
        validate_no_shadow_workspaces(self.metadata, self.root)

    def test_nested_workspace_over_a_root_member_fails_by_path(self) -> None:
        (self.root / "alpha" / "Cargo.toml").write_text('[workspace]\nmembers = ["lib"]\n')
        with self.assertRaisesRegex(RuntimeError, r"shadow root workspace members.*alpha/Cargo.toml"):
            validate_no_shadow_workspaces(self.metadata, self.root)

    def test_genuinely_standalone_workspace_is_allowed(self) -> None:
        # `scripts/` declares its own workspace and owns no root member, so it
        # is a real separate workspace rather than a shadow.
        (self.root / "standalone" / "Cargo.toml").write_text('[workspace]\nmembers = ["x"]\n')
        validate_no_shadow_workspaces(self.metadata, self.root)


def lockfile(entries: dict[str, tuple[str, list[str]]]) -> str:
    """Render `{name: (version, [deps])}` as Cargo.lock text."""
    blocks = ['version = 4\n']
    for name, (version, dependencies) in entries.items():
        block = f'[[package]]\nname = "{name}"\nversion = "{version}"\n'
        if dependencies:
            rendered = "".join(f'    "{dep}",\n' for dep in dependencies)
            block += f"dependencies = [\n{rendered}]\n"
        else:
            block += "dependencies = []\n"
        blocks.append(block)
    return "\n".join(blocks)


class LockfileScopeTests(unittest.TestCase):
    """`Cargo.lock` is scoped from its diff rather than escalated (PR #39)."""

    WORKSPACE = {"alpha-core", "beta-app", "shared-tests"}

    def test_a_dropped_dependency_impacts_only_its_dependents(self) -> None:
        base = lockfile(
            {
                "alpha-core": ("0.1.0", ["serial_test", "shared-tests"]),
                "beta-app": ("0.1.0", ["alpha-core"]),
                "shared-tests": ("0.1.0", []),
                "serial_test": ("3.0.0", []),
            }
        )
        head = lockfile(
            {
                "alpha-core": ("0.1.0", ["shared-tests"]),
                "beta-app": ("0.1.0", ["alpha-core"]),
                "shared-tests": ("0.1.0", []),
            }
        )

        impacted = lockfile_impacted_names(base, head, self.WORKSPACE)

        self.assertEqual({"alpha-core", "beta-app"}, impacted)
        self.assertNotIn(
            "shared-tests", impacted, "a crate the change cannot reach stays out"
        )

    def test_an_unchanged_lockfile_impacts_nothing(self) -> None:
        text = lockfile({"alpha-core": ("0.1.0", ["shared-tests"]),
                         "shared-tests": ("0.1.0", [])})

        self.assertEqual(set(), lockfile_impacted_names(text, text, self.WORKSPACE))

    def test_a_transitive_bump_reaches_every_dependent(self) -> None:
        base = lockfile(
            {
                "alpha-core": ("0.1.0", ["serde"]),
                "beta-app": ("0.1.0", ["alpha-core"]),
                "shared-tests": ("0.1.0", []),
                "serde": ("1.0.0", []),
            }
        )
        head = base.replace('version = "1.0.0"', 'version = "1.0.1"')

        impacted = lockfile_impacted_names(base, head, self.WORKSPACE)

        self.assertEqual({"alpha-core", "beta-app"}, impacted)

    def test_an_unavailable_base_is_undecidable_not_empty(self) -> None:
        head = lockfile({"alpha-core": ("0.1.0", [])})

        self.assertIsNone(lockfile_impacted_names(None, head, self.WORKSPACE))

    def test_an_unparseable_side_is_undecidable_not_empty(self) -> None:
        head = lockfile({"alpha-core": ("0.1.0", [])})

        self.assertIsNone(lockfile_impacted_names("", head, self.WORKSPACE))
        self.assertIsNone(lockfile_impacted_names(head, "", self.WORKSPACE))

    def test_parse_reduces_a_versioned_dependency_to_its_crate_name(self) -> None:
        text = (
            '[[package]]\nname = "alpha-core"\nversion = "0.1.0"\n'
            'dependencies = [\n'
            '    "serde 1.0.0 (registry+https://github.com/rust-lang/crates.io-index)",\n'
            ']\n'
        )

        self.assertEqual(
            {("alpha-core", "0.1.0"): frozenset({"serde"})}, parse_lockfile(text)
        )


class LockfileFullScopeFallbackTests(unittest.TestCase):
    """Without a decidable diff the lockfile must still select everything."""

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
        ]
        # Deliberately unrelated: `beta` appearing in a selection is then
        # evidence of escalation rather than of legitimate reverse-dependency
        # closure, which is the distinction these tests exist to draw.
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

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def test_a_lockfile_change_without_a_base_selects_the_full_workspace(self) -> None:
        (self.root / "Cargo.lock").write_text(
            lockfile({"alpha-core": ("0.1.0", [])}), encoding="utf-8"
        )

        scope = calculate_scope(
            ["Cargo.lock"], self.root, self.metadata, self.area_config
        )

        self.assertTrue(scope["full_scope"])
        self.assertEqual(["alpha", "beta"], [area["area"] for area in scope["areas"]])

    def test_a_decidable_lockfile_change_does_not_select_the_full_workspace(
        self,
    ) -> None:
        base = lockfile(
            {"alpha-core": ("0.1.0", ["serde"]), "beta-app": ("0.1.0", []),
             "serde": ("1.0.0", [])}
        )
        head = lockfile(
            {"alpha-core": ("0.1.0", []), "beta-app": ("0.1.0", [])}
        )
        (self.root / "Cargo.lock").write_text(head, encoding="utf-8")

        scope = calculate_scope(
            ["Cargo.lock"],
            self.root,
            self.metadata,
            self.area_config,
            False,
            base,
        )

        self.assertFalse(scope["full_scope"])
        self.assertEqual(["alpha"], [area["area"] for area in scope["areas"]])


if __name__ == "__main__":
    unittest.main()

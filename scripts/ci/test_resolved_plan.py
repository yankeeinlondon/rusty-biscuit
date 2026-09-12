#!/usr/bin/env python3
"""Regression fixtures for the canonical area-aware execution plan.

Every fixture here runs the *real* planner against the *real* workspace, so a
fixture cannot drift from the shipped policy.

Baseline for these numbers: `fixes/2026-09-11-cicd-cleanup/baseline-2026-09-11.md`.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import unittest
from concurrent.futures import ThreadPoolExecutor
from datetime import date
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import schema  # noqa: E402
from affected_scope import (  # noqa: E402
    ENVIRONMENTS_CONFIG,
    ROOT,
    area_slug,
    calculate_scope,
    legacy_scope_document,
    load_environments,
    load_metadata,
    manifest_directory,
    package_ci_policy,
    workspace_packages,
)

TODAY = date(2026, 7, 27)

#: The one workspace member where `sniff repo package-area` disagrees with
#: sniff's own area universe, measured 2026-09-11 over all 73 members.
#:
#: `sniff repo package-area` run from `biscuit-test-harness/` answers
#: `biscuit-test-harness`, but `sniff repo package-areas` does not list that
#: name, and the three identically shaped root-level members (`renderable`,
#: `biscuit-browser-harness`, `tabby`) all answer `root`. `root` is therefore
#: the answer consistent with `make_package_area` in
#: `sniff/lib/src/filesystem/repo/detection.rs` and with the universe, and it
#: is what the planner emits. Named here rather than tolerated silently: when
#: sniff is fixed, this entry fails and gets deleted.
SNIFF_SELF_INCONSISTENT = {"biscuit-test-harness": ("root", "biscuit-test-harness")}


def sniff_package_area(directory: Path) -> str:
    """The area sniff reports for a workspace member's manifest directory.

    Deliberately not reimplemented here — the point of the drift contract is to
    ask the other tool, not to compare the planner against a second copy of its
    own rule.
    """
    completed = subprocess.run(
        ["sniff", "repo", "package-area", "--json"],
        check=True,
        capture_output=True,
        text=True,
        cwd=directory,
        timeout=180,
    )
    return json.loads(completed.stdout)["name"]


class PlannerFixture(unittest.TestCase):
    """Shared workspace load. `cargo metadata` runs once for the whole module."""

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

    def plan(self, *files: str, force_all: bool = False, **resolution) -> dict:
        return calculate_scope(
            list(files),
            ROOT,
            self.metadata,
            self.environments,
            self.policy,
            force_all,
            **resolution,
        )

    # -- shape-tolerant readers ---------------------------------------------
    #
    # Each reader accepts the plan the planner emits *today* where the answer
    # exists in it, and fails with a precise, stable message where it does not.
    # That message is the pending oracle: it distinguishes "not implemented"
    # from "fixture broken", which is the whole point of the Phase 2 checkpoint.

    def job_packages(self, plan: dict) -> list[str]:
        """Packages this plan gives at least one job to.

        A `gates = false` package has a record and an empty gate list, so the
        filter is what separates "selected" from "scheduled".
        """
        return sorted(entry["package"] for entry in plan["packages"] if entry["gates"])

    def selected_areas(self, plan: dict) -> list[str]:
        self.assertIn(
            "areas",
            plan,
            "the resolved plan has no 'areas' field: the planner still emits the "
            "unversioned scope document, which knows only packages",
        )
        return sorted(entry["area"] for entry in plan["areas"])

    def cells(self, plan: dict) -> list[dict]:
        self.assertIn(
            "cells",
            plan,
            "the resolved plan has no 'cells' field: per-cell execution, origin, "
            "and state do not exist yet",
        )
        return plan["cells"]

    def manifest_directory(self, package: str) -> Path:
        return Path(
            next(
                record["manifest_path"]
                for record in self.packages.values()
                if record["name"] == package
            )
        ).parent

    def package_record(self, plan: dict, package: str) -> dict:
        self.assertEqual(
            plan.get("schema_version"),
            schema.RESOLVED_PLAN_SCHEMA_VERSION,
            "the resolved plan has no 'schema_version' field: the planner still "
            "emits the unversioned scope document",
        )
        return next(entry for entry in plan["packages"] if entry["package"] == package)


class SelectionTests(PlannerFixture):
    """AC1: only impacted areas are selected, and nothing else appears."""

    def test_claudine_and_playa_source_selects_only_those_packages(self) -> None:
        plan = self.plan("claudine/lib/src/lib.rs", "playa/lib/src/lib.rs")
        self.assertEqual(["claudine", "playa"], self.job_packages(plan))

    def test_claudine_and_playa_source_selects_those_two_as_source(self) -> None:
        # Non-pending: the half of AC1 that already holds. Guards against a
        # Phase 3 change that removes the over-selection by under-selecting.
        plan = self.plan("claudine/lib/src/lib.rs", "playa/lib/src/lib.rs")
        self.assertEqual(["claudine", "playa"], plan["source_packages"])

    def test_an_unchanged_dependent_area_receives_no_job(self) -> None:
        plan = self.plan("darkmatter/lib/src/lib.rs")
        scheduled = set(self.job_packages(plan))
        self.assertEqual(
            [],
            sorted(scheduled & set(plan["reverse_dependencies"])),
            "unchanged reverse dependents must receive no job, area, or cell",
        )

    def test_a_gates_false_package_is_selected_but_launches_nothing(self) -> None:
        # `gates = false` is an owned, dated exclusion, so the package stays
        # visible in its area with no gates and no cells. Dropping it would
        # hide a governed absence; giving it cells would demand results
        # nothing produces.
        plan = self.plan("biscuit-visualized/src/src/lib.rs")
        record = self.package_record(plan, "biscuit-visualized")
        self.assertEqual([], record["gates"])
        self.assertEqual("biscuit-visualized", record["area"])
        self.assertNotIn("biscuit-visualized", self.job_packages(plan))
        self.assertEqual(
            [],
            [cell for cell in self.cells(plan) if cell["package"] == "biscuit-visualized"],
        )

    def test_documentation_only_change_selects_no_package_job(self) -> None:
        plan = self.plan("docs/topics/ci-cd.md")
        self.assertEqual([], self.job_packages(plan))
        self.assertEqual("documentation", plan["change_class"])

    def test_ci_tooling_change_selects_no_cargo_package_job(self) -> None:
        plan = self.plan("scripts/ci/affected_scope.py")
        self.assertEqual([], self.job_packages(plan))
        self.assertTrue(plan["flags"]["ci_tooling"])


class DependentSeamTests(PlannerFixture):
    """Open Question 1, Option B, against the real workspace graph.

    A darkmatter change compiles its unchanged direct dependents (`dmls` and
    `darkmatter-cli` among them) inside darkmatter's own `ubuntu-latest` check
    cell; those dependents still receive no job, area, or cell.
    """

    def test_a_hub_change_attributes_its_unchanged_dependents_to_its_own_check(self) -> None:
        plan = self.plan("darkmatter/lib/src/lib.rs")
        record = self.package_record(plan, "darkmatter")
        seam = record["dependent_seam"]
        self.assertLessEqual({"dmls", "darkmatter-cli"}, set(seam["dependents"]))
        self.assertEqual(sorted(seam["dependents"]), seam["dependents"])
        self.assertLessEqual(set(seam["dependents"]), set(plan["reverse_dependencies"]))
        self.assertEqual(
            [], sorted(set(seam["dependents"]) & set(self.job_packages(plan)))
        )
        self.assertEqual([], schema.validate_resolved_plan(plan))
        for name in seam["dependents"]:
            self.assertRegex(seam["check_args"], rf"(^| )-p {name}( |$)")
        self.assertNotIn("--all-targets", seam["check_args"])

    def test_only_the_linux_check_cell_carries_the_seam(self) -> None:
        plan = self.plan("darkmatter/lib/src/lib.rs")
        record = self.package_record(plan, "darkmatter")
        checks = {
            cell["environment"]: cell
            for cell in self.cells(plan)
            if cell["package"] == "darkmatter" and cell["gate"] == "check"
        }
        self.assertIn("ubuntu-latest", checks)
        self.assertEqual(record["dependent_seam"]["dependents"], checks["ubuntu-latest"]["dependents"])
        for environment, cell in checks.items():
            if environment != "ubuntu-latest":
                self.assertNotIn("dependents", cell)
        matrix = {entry["package"]: entry for entry in legacy_scope_document(plan)["matrix"]}
        self.assertEqual(
            record["dependent_seam"]["check_args"], matrix["darkmatter"]["dependents_check_args"]
        )

    def test_a_full_scope_run_attributes_no_dependents(self) -> None:
        plan = self.plan(force_all=True)
        self.assertEqual([], [entry["package"] for entry in plan["packages"] if "dependent_seam" in entry])
        self.assertEqual([], [cell for cell in self.cells(plan) if "dependents" in cell])


class AreaGroupingTests(PlannerFixture):
    """AC2 and AC15: area is derived from the manifest directory."""

    def test_claudine_and_playa_select_their_two_areas(self) -> None:
        plan = self.plan("claudine/lib/src/lib.rs", "playa/lib/src/lib.rs")
        self.assertEqual(["claudine", "playa"], self.selected_areas(plan))

    def test_a_nested_area_is_its_own_area_not_its_parent(self) -> None:
        plan = self.plan("claudine/rendezvous/core/src/lib.rs")
        self.assertEqual(["claudine/rendezvous"], self.selected_areas(plan))

    def test_the_dmls_crate_belongs_to_the_darkmatter_area(self) -> None:
        # Measured, not assumed: `sniff repo package-area` run from
        # `darkmatter/dmls` answers `darkmatter`, because the area is the
        # manifest directory's parent. `darkmatter/dmls` IS an area in sniff's
        # universe — it is the area of `darkmatter/dmls/zed-dmls-cli`, not of
        # the `dmls` crate itself. AC15 makes sniff the authority, so the
        # planner must reproduce this rather than the intuitive grouping.
        plan = self.plan("darkmatter/dmls/src/lib.rs")
        self.assertEqual(["darkmatter"], self.selected_areas(plan))
        zed = self.plan("darkmatter/dmls/zed-dmls-cli/src/main.rs")
        self.assertEqual(["darkmatter/dmls"], self.selected_areas(zed))

    @unittest.skipUnless(shutil.which("sniff"), "requires sniff")
    def test_the_planners_area_matches_sniff_for_every_layout_the_repo_uses(
        self,
    ) -> None:
        # Five layouts, because sniff's rule is not "the manifest's parent
        # directory" and a reimplementation that assumed so would pass on the
        # common case and be wrong for three of these. Phase 3 derives the rule;
        # this fixture is the oracle it has to satisfy.
        plan = self.plan(force_all=True)
        self.selected_areas(plan)
        derived = {entry["package"]: entry["area"] for entry in plan["packages"]}
        mismatched = []
        for name in (
            "claudine",  # area/lib
            "rendezvous-core",  # nested area/component
            "dmls",  # manifest directory IS the nested area
            "test-toolkit",  # grouped under a shared `tools` area
            "biscuit-visualized",  # area/src
        ):
            if name not in derived:
                mismatched.append((name, "absent from the plan", ""))
                continue
            actual = sniff_package_area(self.manifest_directory(name))
            if derived[name] != actual:
                mismatched.append((name, derived[name], actual))
        self.assertEqual([], mismatched)

    @unittest.skipUnless(shutil.which("sniff"), "requires sniff")
    def test_every_workspace_members_area_matches_sniff(self) -> None:
        # AC15 in full: every member, not a sample. The five-layout fixture
        # above stays because it names the layouts a reader has to think about;
        # this one is what actually catches drift when a package moves.
        plan = self.plan(force_all=True)
        derived = {entry["package"]: entry["area"] for entry in plan["packages"]}
        directories = {
            record["name"]: ROOT / str(manifest_directory(ROOT, record))
            for record in self.packages.values()
        }
        self.assertEqual(
            set(derived),
            set(directories),
            "a full-scope plan must hold a record for every workspace member",
        )

        names = sorted(directories)
        with ThreadPoolExecutor(max_workers=8) as pool:
            answers = dict(
                zip(names, pool.map(sniff_package_area, [directories[n] for n in names]))
            )

        mismatched = {
            name: (derived[name], answers[name])
            for name in names
            if derived[name] != answers[name]
        }
        self.assertEqual(
            SNIFF_SELF_INCONSISTENT,
            mismatched,
            "the planner's area mapping drifted from `sniff repo package-area`; "
            "the planner replicates sniff's rule and owns no mapping of its own",
        )

    @unittest.skipUnless(shutil.which("sniff"), "requires sniff")
    def test_the_area_universe_is_a_subset_of_sniffs(self) -> None:
        universe = set(
            json.loads(
                subprocess.run(
                    ["sniff", "repo", "package-areas", "--json"],
                    check=True,
                    capture_output=True,
                    text=True,
                    cwd=ROOT,
                    timeout=120,
                ).stdout
            )
        )
        plan = self.plan(force_all=True)
        # A subset, not equality: sniff reports areas that contain no Cargo
        # workspace member at all (`agent-sandbox`, `visualizer`, `root`), which
        # the planner can never select.
        self.assertEqual(set(), set(self.selected_areas(plan)) - universe)

    def test_full_scope_gives_every_scheduled_package_a_selected_area(self) -> None:
        plan = self.plan(force_all=True)
        areas = set(self.selected_areas(plan))
        self.assertEqual(
            set(),
            {entry["area"] for entry in plan["packages"]} - areas,
            "every package contributes to an area the plan selected",
        )


class TargetCoverageTests(PlannerFixture):
    """AC3: explicit target kinds replace the blanket `--all-targets` contract."""

    def test_a_selected_package_names_the_target_kinds_it_needs(self) -> None:
        plan = self.plan("claudine/lib/src/lib.rs")
        record = self.package_record(plan, "claudine")
        self.assertTrue(set(record["targets"]) <= set(schema.TARGET_KINDS))
        self.assertIn("lib", record["targets"])

    def test_every_cell_reports_where_its_compile_coverage_came_from(self) -> None:
        plan = self.plan("claudine/lib/src/lib.rs")
        uncredited = [
            cell
            for cell in self.cells(plan)
            if cell["gate"] in ("check", "L1") and not cell["compile_coverage_from"]
        ]
        self.assertEqual([], uncredited)

    def test_no_matrix_entry_asks_for_every_target_by_flag(self) -> None:
        # Non-pending: `--all-targets` must never be smuggled through the
        # planner's argument strings, whichever shape the plan takes.
        plan = self.plan(force_all=True)
        entries = plan["packages"] if plan.get("schema_version") else plan["matrix"]
        offenders = [
            entry["package"]
            for entry in entries
            if "--all-targets" in f"{entry['check_args']} {entry['test_args']}"
        ]
        self.assertEqual([], offenders)

    NATIVE = ["ubuntu-latest", "windows-latest", "macos-latest"]
    SELECTORS = {"example": "--examples", "bench": "--benches"}
    BLANKET = ("--all-targets", "--lib", "--bins", "--tests")

    def check_environments(self, plan: dict, package: str) -> list[str]:
        return [
            cell["environment"]
            for cell in self.cells(plan)
            if cell["package"] == package and cell["gate"] == "check"
        ]

    def test_example_targets_are_checked_on_each_native_environment_not_the_guest(self) -> None:
        plan = self.plan("biscuit-speaks/lib/src/lib.rs")
        record = self.package_record(plan, "biscuit-speaks")
        self.assertEqual(["lib", "test", "example"], record["targets"], "fixture: examples only")
        self.assertEqual(self.NATIVE, self.check_environments(plan, "biscuit-speaks"))
        for cell in self.cells(plan):
            if cell["package"] == "biscuit-speaks" and cell["gate"] == "check":
                self.assertEqual(["example"], cell["target_kinds"])
                self.assertIn(cell["environment"], cell["selection_reason"])
        wsl = next(
            cell
            for cell in self.cells(plan)
            if cell["package"] == "biscuit-speaks" and cell["environment"] == "wsl2-ubuntu"
        )
        self.assertEqual("ubuntu-latest archive build", wsl["compile_coverage_from"])

    def test_check_args_carry_exactly_the_declared_uncovered_selectors(self) -> None:
        plan = self.plan(force_all=True)
        shapes_seen: set[tuple[str, ...]] = set()
        for record in plan["packages"]:
            if not record["gates"]:
                continue
            with self.subTest(package=record["package"]):
                expected = [
                    self.SELECTORS[kind] for kind in ("example", "bench") if kind in record["targets"]
                ]
                tokens = record["check_args"].split()
                self.assertEqual(expected, [token for token in tokens if token in self.SELECTORS.values()])
                self.assertEqual(["-p", record["package"]], tokens[:2])
                self.assertEqual([], [token for token in tokens if token in self.BLANKET])
                self.assertEqual(
                    self.NATIVE if expected else [],
                    self.check_environments(plan, record["package"]),
                )
                shapes_seen.add(tuple(expected))
        # Non-vacuous only if the workspace still holds every shape.
        self.assertEqual(
            {(), ("--examples",), ("--benches",), ("--examples", "--benches")}, shapes_seen
        )

    def test_a_reused_macos_l1_keeps_the_macos_check_cell_executing(self) -> None:
        accepted = [
            {
                "package": "biscuit-speaks",
                "environment": "macos-latest",
                "gate": "L1",
                "origin": "local",
            }
        ]
        plan = self.plan("biscuit-speaks/lib/src/lib.rs", accepted_cells=accepted)
        states = {
            (cell["environment"], cell["gate"]): cell["execution"]
            for cell in self.cells(plan)
            if cell["package"] == "biscuit-speaks"
        }
        self.assertEqual("reuse", states[("macos-latest", "L1")], "fixture: macOS L1 reused")
        self.assertEqual("execute", states[("macos-latest", "check")])
        scheduled = legacy_scope_document(plan)
        matrix = scheduled["area_matrix"]["biscuit-speaks"]["include"]
        entry = next(item for item in matrix if item["package"] == "biscuit-speaks")
        self.assertEqual(self.NATIVE, entry["check_os"])
        self.assertNotIn("macos-latest", entry["native_environments"])

    def test_the_workflow_command_joined_with_check_args_selects_only_the_declared_kinds(self) -> None:
        # The review's "test the final command": the planner's string and the
        # workflow's template, joined, and the guest's archive build kept apart.
        package_ci = (ROOT / ".github/workflows/_package-ci.yml").read_text()
        template = next(
            line.strip()
            for line in package_ci.splitlines()
            if line.strip().startswith("run: cargo check ")
        )
        self.assertEqual("run: cargo check ${{ inputs.check-args }}", template)
        plan = self.plan("biscuit-speaks/lib/src/lib.rs")
        record = self.package_record(plan, "biscuit-speaks")
        command = template.removeprefix("run: ").replace("${{ inputs.check-args }}", record["check_args"])
        self.assertTrue(
            command.startswith("cargo check -p biscuit-speaks --examples"), command
        )
        self.assertNotIn("--benches", command)
        for flag in self.BLANKET:
            self.assertNotIn(flag, command)
        self.assertIn(
            "archive-args: -p ${{ inputs.package }} ${{ inputs.test-args }}",
            package_ci,
            "the WSL archive build must receive package and features, never the check selectors",
        )


class EnvironmentCoverageTests(PlannerFixture):
    """AC3: Linux and Windows coverage survives a macOS reuse."""

    def test_linux_and_windows_compile_coverage_survives_a_macos_reuse(self) -> None:
        plan = self.plan("claudine/lib/src/lib.rs")
        compiled = {
            cell["environment"]
            for cell in self.cells(plan)
            if cell["gate"] in ("check", "L1")
        }
        self.assertLessEqual({"ubuntu-latest", "windows-latest"}, compiled)

    def test_wsl_compile_coverage_is_the_ubuntu_archive_build(self) -> None:
        plan = self.plan("claudine/lib/src/lib.rs")
        wsl_compiles = [
            cell
            for cell in self.cells(plan)
            if cell["environment"] == "wsl2-ubuntu" and cell["gate"] == "check"
        ]
        self.assertEqual(
            [],
            wsl_compiles,
            "the toolchain-free WSL guest never compiles; its compile coverage "
            "is the ubuntu-latest archive build",
        )


class PlanDocumentTests(PlannerFixture):
    """The planner's output is the frozen resolved plan, or it is not a plan."""

    def test_the_emitted_plan_validates_against_the_frozen_schema(self) -> None:
        plan = self.plan("claudine/lib/src/lib.rs")
        self.assertEqual([], schema.validate_resolved_plan(plan))

    def test_a_full_scope_plan_validates_against_the_frozen_schema(self) -> None:
        plan = self.plan(force_all=True)
        self.assertEqual([], schema.validate_resolved_plan(plan))

    def test_a_documentation_plan_validates_against_the_frozen_schema(self) -> None:
        plan = self.plan("docs/topics/ci-cd.md")
        self.assertEqual([], schema.validate_resolved_plan(plan))

    def test_the_plan_carries_no_single_excluded_environment(self) -> None:
        plan = self.plan("claudine/lib/src/lib.rs")
        self.assertNotIn(
            "malformed-receipt: resolved plan has unknown field 'excluded_environment'",
            schema.validate_resolved_plan(plan),
            "one excluded environment cannot express evidence from two hosts; "
            "the resolved plan replaces it with per-cell origin",
        )

    def test_the_job_estimate_stays_within_githubs_limits(self) -> None:
        # Non-pending: the ceiling holds today and Phase 6 must not breach it.
        plan = self.plan(force_all=True)
        self.assertLess(plan["job_estimate"], 1000)


class ClosurePathTests(PlannerFixture):
    """AC7: the plan carries the paths a cell's gate-input identity covers."""

    def test_a_selected_package_declares_its_own_manifest_directory(self) -> None:
        plan = self.plan("claudine/lib/src/lib.rs")
        paths = self.package_record(plan, "claudine")["input_paths"]
        self.assertIn("claudine/lib", paths)

    def test_the_closure_reaches_a_real_workspace_dependency(self) -> None:
        # claudine depends on darkmatter, so a darkmatter edit must be able to
        # move claudine's identity. If the closure stopped at the package's own
        # directory, an older receipt would survive a dependency change.
        plan = self.plan("claudine/lib/src/lib.rs")
        paths = self.package_record(plan, "claudine")["input_paths"]
        self.assertIn("darkmatter/lib", paths)

    def test_every_declared_path_exists_in_the_checkout(self) -> None:
        plan = self.plan("claudine/lib/src/lib.rs", "playa/lib/src/lib.rs")
        for entry in plan["packages"]:
            for path in entry.get("input_paths", []):
                with self.subTest(package=entry["package"], path=path):
                    self.assertTrue((ROOT / path).is_dir(), path)

    def test_a_non_gating_package_declares_no_closure(self) -> None:
        plan = self.plan(force_all=True)
        non_gating = [entry for entry in plan["packages"] if not entry["gates"]]
        self.assertTrue(non_gating, "the workspace has at least one gates=false package")
        for entry in non_gating:
            self.assertEqual([], entry.get("input_paths", []))


class ProhibitionTests(PlannerFixture):
    """AC17: a recorded constraint resolves to prohibited cells, never to jobs."""

    CONSTRAINT = {
        "owner": "ken",
        "reason": "do not rerun WSL for this branch",
        "expiry": "2099-01-01",
        "source": "current.json",
    }

    def wsl_cells(self, plan: dict) -> list[dict]:
        return [
            cell for cell in self.cells(plan) if cell["environment"] == "wsl2-ubuntu"
        ]

    def test_a_prohibited_environment_schedules_nothing(self) -> None:
        plan = self.plan(
            "claudine/lib/src/lib.rs", prohibitions={"wsl2-ubuntu": self.CONSTRAINT}
        )
        executing = [cell for cell in self.wsl_cells(plan) if cell["execution"] == "execute"]
        self.assertEqual([], executing)

    def test_a_prohibited_cell_stays_a_cell_and_names_its_constraint(self) -> None:
        plan = self.plan(
            "claudine/lib/src/lib.rs", prohibitions={"wsl2-ubuntu": self.CONSTRAINT}
        )
        prohibited = [cell for cell in self.wsl_cells(plan) if cell["state"] == "prohibited"]
        self.assertTrue(prohibited, "the WSL cells vanished instead of being prohibited")
        for cell in prohibited:
            self.assertEqual(self.CONSTRAINT, cell["prohibition"])
        self.assertEqual(
            sorted(
                f"{cell['package']}/wsl2-ubuntu/{cell['gate']}" for cell in prohibited
            ),
            sorted(plan["prohibited_cells"]),
        )

    def test_a_prohibited_plan_still_validates(self) -> None:
        plan = self.plan(
            "claudine/lib/src/lib.rs", prohibitions={"wsl2-ubuntu": self.CONSTRAINT}
        )
        self.assertEqual([], schema.validate_resolved_plan(plan))

    def test_another_environment_is_untouched_by_the_constraint(self) -> None:
        plan = self.plan(
            "claudine/lib/src/lib.rs", prohibitions={"wsl2-ubuntu": self.CONSTRAINT}
        )
        elsewhere = [
            cell
            for cell in self.cells(plan)
            if cell["environment"] != "wsl2-ubuntu" and cell["state"] == "prohibited"
        ]
        self.assertEqual([], elsewhere)
        self.assertTrue(
            any(
                cell["execution"] == "execute"
                for cell in self.cells(plan)
                if cell["environment"] == "ubuntu-latest"
            )
        )

    def test_evidence_satisfies_a_constraint_instead_of_violating_it(self) -> None:
        # A cell the constraint forbids but evidence already covers is reused,
        # not prohibited: the constraint protects the host's time, and a reused
        # cell consumes none of it.
        plan = self.plan("claudine/lib/src/lib.rs")
        wsl_gates = {cell["gate"] for cell in self.wsl_cells(plan)}
        accepted = [
            {
                "package": "claudine",
                "environment": "wsl2-ubuntu",
                "gate": gate,
                "origin": "prior-local",
            }
            for gate in sorted(wsl_gates)
        ]
        resolved = self.plan(
            "claudine/lib/src/lib.rs",
            accepted_cells=accepted,
            prohibitions={"wsl2-ubuntu": self.CONSTRAINT},
        )
        claudine_wsl = [
            cell
            for cell in self.wsl_cells(resolved)
            if cell["package"] == "claudine"
        ]
        self.assertTrue(claudine_wsl)
        for cell in claudine_wsl:
            self.assertEqual("reused", cell["state"])
        self.assertNotIn(
            "claudine/wsl2-ubuntu/L1",
            resolved["prohibited_cells"],
        )

    def test_no_constraint_leaves_the_plan_exactly_as_before(self) -> None:
        self.assertEqual(
            schema.canonical(self.plan("claudine/lib/src/lib.rs")),
            schema.canonical(self.plan("claudine/lib/src/lib.rs", prohibitions={})),
        )


class ResultCompletenessTests(PlannerFixture):
    """Spec §5: the machine-readable results survive `ci-verdict`'s removal.

    Once the global verdict job is gone, the only `ci-results` documents a run
    produces are the per-area slices `_area-ci.yml` uploads, one per entry in
    `scheduled_areas`. Their union is complete only if every cell the plan
    resolved belongs to a scheduled area — including the cells no runner will
    execute, which is precisely where a completeness hole would hide, because
    nothing red would appear when one went missing.
    """

    def scheduled(self, plan: dict) -> dict:
        return legacy_scope_document(plan)

    def test_every_resolved_cell_belongs_to_a_scheduled_area(self) -> None:
        plans = {
            "two areas": self.plan("claudine/lib/src/lib.rs", "playa/lib/src/lib.rs"),
            "nested area": self.plan("claudine/rendezvous/core/src/lib.rs"),
            "full scope": self.plan(force_all=True),
        }
        for label, plan in plans.items():
            scheduled = set(self.scheduled(plan)["scheduled_areas"])
            orphaned = sorted(
                {
                    f"{cell['package']}/{cell['environment']}/{cell['gate']}"
                    f" in {cell['area']}"
                    for cell in self.cells(plan)
                    if cell["area"] not in scheduled
                }
            )
            self.assertEqual(
                [],
                orphaned,
                f"{label}: a cell whose area never fans out reaches no result slice",
            )

    def test_an_area_whose_every_cell_is_reused_still_owns_a_result_slice(self) -> None:
        # The completeness hole this class exists for. A receipt covering all
        # of an area's cells removes every hosted execution from it; if the
        # area then dropped out of the fan-out, its local-origin results would
        # be reported nowhere at all, which is the PR #76 failure mode wearing
        # a different hat.
        #
        # playa-cli, not playa: a receipt can never satisfy a check cell, and
        # the playa library owns one for its unchanged dependents (Open
        # Question 1, Option B), so only a package with no dependents and no
        # example/bench kinds can have EVERY cell reused.
        plan = self.plan("playa/cli/src/main.rs")
        playa_cells = [cell for cell in self.cells(plan) if cell["area"] == "playa"]
        self.assertTrue(playa_cells, "the fixture must select the playa area")
        accepted = [
            {
                "package": cell["package"],
                "environment": cell["environment"],
                "gate": cell["gate"],
                "origin": "local",
            }
            for cell in playa_cells
        ]
        reused = self.plan("playa/cli/src/main.rs", accepted_cells=accepted)
        states = {
            cell["state"] for cell in self.cells(reused) if cell["area"] == "playa"
        }
        self.assertEqual({"reused"}, states, "the fixture must reuse every cell")

        scheduled = self.scheduled(reused)
        self.assertIn(
            "playa",
            scheduled["scheduled_areas"],
            "an all-reused area must still fan out, or its results reach no slice",
        )
        matrix = scheduled["area_matrix"]["playa"]["include"]
        self.assertTrue(matrix, "the area must still carry its package matrix")
        for entry in matrix:
            for field in (
                "native_environments",
                "check_os",
                "l2_environments",
                "browser_environments",
            ):
                self.assertEqual(
                    [],
                    entry[field],
                    f"{entry['package']}: no runner may be scheduled for a reused cell",
                )

    def test_every_scheduled_area_has_an_artifact_safe_slice_name(self) -> None:
        # The slice artifact is `ci-results-<slug>`; a nested area name is not
        # a legal artifact name, and a collision would have two areas
        # overwriting one document.
        plan = self.plan(force_all=True)
        scheduled = self.scheduled(plan)
        slugs = scheduled["area_slugs"]
        self.assertEqual(sorted(scheduled["scheduled_areas"]), sorted(slugs))
        self.assertEqual(
            len(slugs), len(set(slugs.values())), "two areas share one slice name"
        )
        for area, slug in slugs.items():
            self.assertEqual(area_slug(area), slug)
            self.assertNotIn("/", f"ci-results-{slug}.json")


if __name__ == "__main__":
    unittest.main(verbosity=2)

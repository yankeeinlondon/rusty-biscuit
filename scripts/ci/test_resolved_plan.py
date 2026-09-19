#!/usr/bin/env python3
"""Regression fixtures for the canonical area-aware execution plan.

Every fixture here runs the *real* planner against the *real* workspace, so a
fixture cannot drift from the shipped policy.

Baseline for these numbers: `fixes/2026-09-11-cicd-cleanup/baseline-2026-09-11.md`.
"""

from __future__ import annotations

import copy
import json
import os
import subprocess
import sys
import unittest
from concurrent.futures import ThreadPoolExecutor
from datetime import date
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import affected_scope  # noqa: E402
import build_key  # noqa: E402
import schema  # noqa: E402
from tool_guard import require_tools  # noqa: E402
from affected_scope import (  # noqa: E402
    ENVIRONMENTS_CONFIG,
    ROOT,
    apply_accepted_cells,
    area_slug,
    build_owner_matrix,
    calculate_scope,
    archive_producers,
    legacy_scope_document,
    load_environments,
    load_metadata,
    manifest_directory,
    package_ci_policy,
    workspace_packages,
)

TODAY = date(2026, 7, 27)

#: The one workspace member where `sniff repo package-area` disagrees with
#: sniff's own area universe, measured 2026-09-11 over all 73 members and
#: re-measured 2026-09-15 against a release `sniff-cli` built from this
#: worktree, which answers identically.
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

#: Named because a skip that says only "requires sniff" claims nothing about
#: where the invariant IS checked, and AC15 had no written answer to that.
#: `ci-tooling` and `preflight` both run this suite and neither provisions the
#: binary: a release `sniff-cli` costs 4m25s cold and 1m39s with a warm
#: dependency cache, measured in
#: `reviews/2026-09-15-python-test-code/spike-4-results.md`.
SNIFF_ENFORCED_BY = (
    "`ci.yml`'s `area-drift` job, which builds sniff-cli, sets "
    "BISCUIT_REQUIRE_SNIFF, and whose result `ci-gate` folds"
)
SNIFF_DETAIL = (
    "The nightly `area-drift` workflow is the backstop, and `just ci-local` on "
    "a developer host that has sniff installed runs it too. `ci-tooling` and "
    "`preflight` do not provision it."
)


def require_sniff() -> None:
    """Skip where `sniff` is genuinely absent; fail where it was provisioned."""
    require_tools("sniff", enforced_by=SNIFF_ENFORCED_BY, detail=SNIFF_DETAIL)


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

    def test_ci_tooling_change_schedules_its_owner_and_nothing_else(self) -> None:
        # Until `scripts/` joined the root workspace these suites ran in a job
        # no plan selected. `repo-deps` owns them now, so the change reaches CI
        # as an ordinary package job — and still only that one.
        plan = self.plan("scripts/ci/affected_scope.py")
        self.assertEqual(["repo-deps"], self.job_packages(plan))
        record = self.package_record(plan, "repo-deps")
        self.assertEqual("root", record["area"])
        self.assertEqual("package", plan["change_class"])


class SuiteOwnershipCorpusTests(PlannerFixture):
    """AC4/AC5 against the shipped registry, manifests, and workspace graph.

    `test_affected_scope.py` validates the registry as a table. These fixtures
    are what stop the table from being internally consistent and wrong about
    the repository: an owner that is not a member, a manifest claiming a suite
    the registry gives to someone else, or a trigger that selects the wrong
    package once the real member set is in play.
    """

    def test_every_registered_owner_is_a_workspace_member(self) -> None:
        members = {record["name"] for record in self.packages.values()}
        orphans = sorted(
            {
                entry["owner"]
                for entry in affected_scope.SUITE_REGISTRY.values()
                if entry["owner"] not in members
            }
        )
        self.assertEqual(
            [],
            orphans,
            "a suite owned by a non-member is owned by nothing CI can schedule",
        )

    def test_every_declared_companion_is_registered_to_its_declarer(self) -> None:
        declarations = {
            name: record["companion_suites"]
            for name, record in self.policy.items()
            if record["companion_suites"]
        }
        for package, suites in sorted(declarations.items()):
            for suite in suites:
                entry = affected_scope.SUITE_REGISTRY.get(suite, {})
                self.assertEqual(
                    package,
                    entry.get("owner"),
                    f"{package} declares companion suite '{suite}', registered "
                    f"to {entry.get('owner')!r}",
                )

    def test_every_shipped_declaration_validates_against_the_registry(self) -> None:
        declarations = {
            name: list(record["companion_suites"])
            for name, record in self.policy.items()
            if record["companion_suites"]
        }
        # Only the declared subset: a registered suite no manifest claims yet is
        # the separate defect `test_affected_scope.py` pins on the whole table.
        declared = {
            name: entry
            for name, entry in affected_scope.SUITE_REGISTRY.items()
            if any(name in suites for suites in declarations.values())
        }
        self.assertEqual(
            [], affected_scope.validate_suite_registry(declared, declarations)
        )

    def test_each_tooling_trigger_selects_exactly_its_owner(self) -> None:
        for path, expected in (
            (".github/ci/ci-baseline.toml", ["repo-deps"]),
            (".github/ci/environments.json", ["repo-deps"]),
            ("scripts/Cargo.toml", ["repo-deps"]),
            (".github/workflows/ci.yml", ["test-toolkit"]),
            (".github/workflows/_area-ci.yml", ["test-toolkit"]),
            ("tools/test-audit/package.json", ["test-toolkit"]),
            ("pnpm-lock.yaml", ["test-toolkit"]),
            ("pnpm-workspace.yaml", ["test-toolkit"]),
            ("tools/test-toolkit/Cargo.toml", ["test-toolkit"]),
            ("docs/topics/ci-cd.md", []),
            ("darkmatter/README.md", []),
        ):
            with self.subTest(path=path):
                self.assertEqual(expected, self.job_packages(self.plan(path)))

    def test_a_trigger_selection_compiles_no_reverse_dependents(self) -> None:
        # A workflow edit says nothing about test-toolkit's public API, so the
        # dependent seam its own source change carries must not appear here —
        # that seam compiles eleven consumers on Linux.
        triggered = self.plan(".github/workflows/_area-ci.yml")
        self.assertEqual(["test-toolkit"], self.job_packages(triggered))
        self.assertEqual([], triggered["reverse_dependencies"])
        self.assertNotIn(
            "dependent_seam", self.package_record(triggered, "test-toolkit")
        )
        self.assertEqual(
            [],
            [
                cell
                for cell in self.cells(triggered)
                if cell["gate"] == "check"
            ],
            "no unchanged dependent means no check cell to compile it in",
        )

        sourced = self.plan("tools/test-toolkit/src/lib.rs")
        self.assertIn(
            "dependent_seam",
            self.package_record(sourced, "test-toolkit"),
            "a real source change must still carry the seam",
        )


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

    def test_the_planners_area_matches_sniff_for_every_layout_the_repo_uses(
        self,
    ) -> None:
        require_sniff()
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

    def test_every_workspace_members_area_matches_sniff(self) -> None:
        require_sniff()
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
        # 8 is where the wall time stops improving: 10.9s serial, 7.1s at four,
        # 6.2s at eight, 6.1s at sixteen. The ~70s of system time is inside
        # sniff's own per-invocation walk and is flat across every pool width,
        # so narrowing the pool buys nothing and costs 4.7s (measured
        # 2026-09-15, `reviews/2026-09-15-python-test-code/spike-4-results.md`).
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

    def test_the_area_universe_is_a_subset_of_sniffs(self) -> None:
        require_sniff()
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

    def test_example_targets_are_checked_on_the_check_environment_not_the_guest(self) -> None:
        plan = self.plan("biscuit-speaks/lib/src/lib.rs")
        record = self.package_record(plan, "biscuit-speaks")
        self.assertEqual(["lib", "test", "example"], record["targets"], "fixture: examples only")
        self.assertEqual(
            [affected_scope.CHECK_ENVIRONMENT], self.check_environments(plan, "biscuit-speaks")
        )
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
                # One check environment, like lint (fixes/2026-09-18-ci-cadence,
                # decision 3): the example and bench kinds compile once.
                self.assertEqual(
                    [affected_scope.CHECK_ENVIRONMENT] if expected else [],
                    self.check_environments(plan, record["package"]),
                )
                shapes_seen.add(tuple(expected))
        # Non-vacuous only if the workspace still holds every shape.
        self.assertEqual(
            {(), ("--examples",), ("--benches",), ("--examples", "--benches")}, shapes_seen
        )

    def test_a_reused_linux_l1_reuses_the_check_cell_and_macos_reuses_none(self) -> None:
        def accepted(environment: str) -> list[dict[str, str]]:
            return [
                {
                    "package": "biscuit-speaks",
                    "environment": environment,
                    "gate": "L1",
                    "outcome": "pass",
                    "origin": "local",
                }
            ]

        def states(plan: dict) -> dict[tuple[str, str], str]:
            return {
                (cell["environment"], cell["gate"]): cell["execution"]
                for cell in self.cells(plan)
                if cell["package"] == "biscuit-speaks"
            }

        check = affected_scope.CHECK_ENVIRONMENT
        # biscuit-speaks has unchanged direct dependents, so its one check
        # cell also compiles their seam — which no local run builds, so the
        # Linux L1 pass covers the L1 cell and never the check.
        plan = self.plan("biscuit-speaks/lib/src/lib.rs", accepted_cells=accepted(check))
        linux = states(plan)
        self.assertEqual("reuse", linux[(check, "L1")], "fixture: Linux L1 reused")
        self.assertEqual("execute", linux[(check, "check")])
        seam = next(
            cell for cell in self.cells(plan)
            if cell["package"] == "biscuit-speaks" and cell["gate"] == "check"
        )
        self.assertFalse(seam["reusable"])
        self.assertIn("claudine", seam["dependents"])
        matrix = legacy_scope_document(plan)["area_matrix"]["biscuit-speaks"]["include"]
        entry = next(item for item in matrix if item["package"] == "biscuit-speaks")
        self.assertEqual([check], entry["check_os"])
        self.assertNotIn(check, entry["native_environments"])
        # A macOS pass stands in for no check: there is no macOS check cell.
        plan = self.plan("biscuit-speaks/lib/src/lib.rs", accepted_cells=accepted("macos-latest"))
        macos = states(plan)
        self.assertEqual("reuse", macos[("macos-latest", "L1")], "fixture: macOS L1 reused")
        self.assertNotIn(("macos-latest", "check"), macos)
        self.assertEqual("execute", macos[(check, "check")])

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
        # The guest consumes a build record rather than an archive argument
        # list, so there is no selector for a check flag to leak into. Comments
        # explain what was removed; only executable YAML can violate it.
        self.assertIn("builds: ${{ inputs.builds }}", package_ci)
        executable = "\n".join(
            line for line in package_ci.splitlines() if not line.lstrip().startswith("#")
        )
        self.assertNotIn("archive-args", executable)


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
                "outcome": "pass",
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

    def test_an_area_with_all_test_cells_reused_still_owns_a_result_slice(self) -> None:
        # Lint always executes, even when every test cell has prior evidence.
        # The area's slice must include those local-origin results alongside it.
        plan = self.plan("playa/cli/src/main.rs")
        playa_cells = [cell for cell in self.cells(plan) if cell["area"] == "playa"]
        self.assertTrue(playa_cells, "the fixture must select the playa area")
        accepted = [
            {
                "package": cell["package"],
                "environment": cell["environment"],
                "gate": cell["gate"],
                "outcome": "pass",
                "origin": "local",
            }
            for cell in playa_cells
        ]
        reused = self.plan("playa/cli/src/main.rs", accepted_cells=accepted)
        states = {
            cell["state"] for cell in self.cells(reused)
            if cell["area"] == "playa" and cell["gate"] != "lint"
        }
        self.assertEqual({"reused"}, states, "the fixture must reuse every test cell")
        lint = [cell for cell in self.cells(reused) if cell["gate"] == "lint"]
        self.assertEqual(1, len(lint))
        self.assertEqual("pending", lint[0]["state"])
        self.assertEqual("ci", lint[0]["origin"])
        self.assertFalse(lint[0]["reusable"])

        scheduled = self.scheduled(reused)
        self.assertIn(
            "playa",
            scheduled["scheduled_areas"],
            "an area with reused tests must still fan out, or its results reach no slice",
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


class BuildOwnershipTests(PlannerFixture):
    """Validation checkpoint 2 of `fixes/2026-09-12-single-os-compile/plan.md`.

    Every fixture runs the real planner against the real workspace, so a
    build key here is the key CI would compute for the same change.
    """

    #: Declares L1, L2, and browser, so its one compatible Linux configuration
    #: is demanded by more than one tier.
    THREE_TIER_PACKAGE = "biscuit-terminal"
    THREE_TIER_SOURCE = "biscuit-terminal/lib/src/lib.rs"

    def builds_of(self, plan: dict, package: str) -> dict[str, dict]:
        return {
            record["producer"]: record
            for record in plan["builds"]
            if record["package"] == package
        }

    def test_the_plan_is_valid_and_every_executing_tier_names_one_build(self) -> None:
        plan = self.plan(self.THREE_TIER_SOURCE)
        self.assertEqual([], schema.validate_resolved_plan(plan))
        for cell in plan["cells"]:
            executing_tier = (
                cell["execution"] == "execute" and cell["gate"] in schema.BUILD_GATES
            )
            self.assertEqual(
                executing_tier,
                "build" in cell,
                f"{cell['package']}/{cell['environment']}/{cell['gate']}",
            )

    def test_l1_and_browser_consumers_share_one_compatible_linux_key(self) -> None:
        # The specification's central claim: tiers whose only compile-time
        # difference is a runtime nextest filter compile once.
        plan = self.plan(self.THREE_TIER_SOURCE)
        keys = {
            (cell["environment"], cell["gate"]): cell["build"]
            for cell in plan["cells"]
            if cell["package"] == self.THREE_TIER_PACKAGE and "build" in cell
        }
        self.assertEqual(
            keys[("ubuntu-latest", "L1")],
            keys[("ubuntu-latest", "browser")],
            "L1 and browser differ only in a runtime filter and must share one build",
        )
        self.assertEqual(
            keys[("ubuntu-latest", "L1")],
            keys[("wsl2-ubuntu", "L1")],
            "the WSL2 guest consumes the Linux producer's archive",
        )

    def test_one_linux_build_still_yields_two_distinct_result_cells(self) -> None:
        # Spec section 5: a Linux pass is never WSL2 evidence.
        plan = self.plan(self.THREE_TIER_SOURCE)
        record = self.builds_of(plan, self.THREE_TIER_PACKAGE)["ubuntu-latest"]
        self.assertIn({"environment": "ubuntu-latest", "gate": "L1"}, record["consumers"])
        self.assertIn({"environment": "wsl2-ubuntu", "gate": "L1"}, record["consumers"])
        results = [
            (cell["environment"], cell["gate"])
            for cell in plan["cells"]
            if cell["package"] == self.THREE_TIER_PACKAGE and cell["gate"] == "L1"
        ]
        self.assertIn(("ubuntu-latest", "L1"), results)
        self.assertIn(("wsl2-ubuntu", "L1"), results)

    def test_each_native_producer_owns_exactly_one_key_for_the_package(self) -> None:
        plan = self.plan(self.THREE_TIER_SOURCE)
        owners = [
            record["producer"]
            for record in plan["builds"]
            if record["package"] == self.THREE_TIER_PACKAGE
        ]
        self.assertEqual(
            ["macos-latest", "ubuntu-latest", "windows-latest"], sorted(owners)
        )
        self.assertEqual(len(owners), len(set(owners)), "a key may have one owner only")

    def test_incompatible_target_abis_split_the_key(self) -> None:
        plan = self.plan(self.THREE_TIER_SOURCE)
        records = self.builds_of(plan, self.THREE_TIER_PACKAGE)
        keys = {name: record["key"] for name, record in records.items()}
        self.assertEqual(3, len(set(keys.values())), keys)
        self.assertNotIn(
            "wsl2-ubuntu", records["windows-latest"]["compatible_environments"]
        )

    def test_a_governed_gap_creates_no_consumer_demand(self) -> None:
        # Every L2 cell of this package is an accepted gap (its only backend is
        # a GUI emulator no runner hosts), so no build lists an L2 consumer.
        plan = self.plan(self.THREE_TIER_SOURCE)
        gaps = [
            cell
            for cell in plan["cells"]
            if cell["package"] == self.THREE_TIER_PACKAGE and cell["gate"] == "L2"
        ]
        self.assertTrue(gaps)
        self.assertTrue(all(cell["execution"] == "omit" for cell in gaps), gaps)
        self.assertEqual(
            [],
            [
                consumer
                for record in plan["builds"]
                for consumer in record["consumers"]
                if consumer["gate"] == "L2"
            ],
        )

    def test_an_all_reused_plan_schedules_no_owner(self) -> None:
        plan = self.plan(self.THREE_TIER_SOURCE)
        accepted = [
            {
                "package": cell["package"],
                "environment": cell["environment"],
                "gate": cell["gate"],
                "outcome": "pass",
                "evidence": {"ref": f"refs/notes/ci-local/{cell['environment']}"},
            }
            for cell in plan["cells"]
            if cell["execution"] == "execute" and cell["gate"] in schema.BUILD_GATES
        ]
        applied = apply_accepted_cells(plan, accepted, [])
        self.assertEqual([], schema.validate_resolved_plan(applied))
        self.assertEqual([], applied["builds"], "a satisfied cell demands no compile")
        self.assertTrue(all("build" not in cell for cell in applied["cells"]))

    def test_evidence_for_one_environment_leaves_the_other_owners_standing(self) -> None:
        plan = self.plan(self.THREE_TIER_SOURCE)
        before = self.builds_of(plan, self.THREE_TIER_PACKAGE)
        applied = apply_accepted_cells(
            plan,
            [
                {
                    "package": self.THREE_TIER_PACKAGE,
                    "environment": "macos-latest",
                    "gate": "L1",
                    "outcome": "pass",
                    "evidence": {"ref": "refs/notes/ci-local/macos-latest"},
                }
            ],
            [],
        )
        self.assertEqual([], schema.validate_resolved_plan(applied))
        after = self.builds_of(applied, self.THREE_TIER_PACKAGE)
        self.assertNotIn("macos-latest", after)
        self.assertEqual(
            {name: record["key"] for name, record in before.items() if name != "macos-latest"},
            {name: record["key"] for name, record in after.items()},
            "the overlay removes demand; it never recomputes a key",
        )

    def test_partial_evidence_shrinks_a_shared_records_consumer_list(self) -> None:
        plan = self.plan(self.THREE_TIER_SOURCE)
        applied = apply_accepted_cells(
            plan,
            [
                {
                    "package": self.THREE_TIER_PACKAGE,
                    "environment": "wsl2-ubuntu",
                    "gate": "L1",
                    "outcome": "pass",
                    "evidence": {"ref": "refs/notes/ci-local/wsl2-ubuntu"},
                }
            ],
            [],
        )
        self.assertEqual([], schema.validate_resolved_plan(applied))
        record = self.builds_of(applied, self.THREE_TIER_PACKAGE)["ubuntu-latest"]
        self.assertNotIn({"environment": "wsl2-ubuntu", "gate": "L1"}, record["consumers"])
        self.assertIn({"environment": "ubuntu-latest", "gate": "L1"}, record["consumers"])

    def test_lint_and_check_cells_reference_no_build_and_keep_their_coverage(self) -> None:
        # Spec section 6: Clippy is another compiler driver and check-only
        # kinds may emit no executable, so neither consumes an archive.
        plan = self.plan(self.THREE_TIER_SOURCE)
        compile_gates = [
            cell for cell in plan["cells"] if cell["gate"] in ("lint", "check")
        ]
        self.assertTrue(compile_gates)
        for cell in compile_gates:
            self.assertNotIn("build", cell)
        checks = [cell for cell in compile_gates if cell["gate"] == "check"]
        self.assertTrue(checks, "the check cells must survive build derivation")
        self.assertTrue(all(cell["compile_coverage_from"] == "check" for cell in checks))

    def test_two_packages_get_two_keys_from_one_owner(self) -> None:
        plan = self.plan("claudine/lib/src/lib.rs", "playa/lib/src/lib.rs")
        linux = [
            record for record in plan["builds"] if record["producer"] == "ubuntu-latest"
        ]
        self.assertEqual(
            ["claudine", "playa"], sorted(record["package"] for record in linux)
        )
        self.assertEqual(
            2, len({record["key"] for record in linux}), "one key per package"
        )

    def test_a_different_feature_graph_splits_the_key(self) -> None:
        # The isolated feature graph is a keyed input, so the same package
        # compiled with other features is a different build rather than a
        # silent union.
        plan = self.plan(self.THREE_TIER_SOURCE)
        record = self.builds_of(plan, self.THREE_TIER_PACKAGE)["ubuntu-latest"]
        base = build_key.planned_key(record["identity"])
        self.assertEqual(record["key"], base)
        for field, value in (
            ("features", "--no-default-features"),
            ("target_kinds", ["lib"]),
            ("rustflags", "-C debuginfo=0"),
            ("sidecars", ["messenger-desktop-stubs"]),
            ("archive_includes", ["fixtures/"]),
            ("lockfile", "0000000000000000"),
        ):
            other = {**record["identity"], field: value}
            self.assertNotEqual(
                base,
                build_key.planned_key(other),
                f"changing {field} must split the build key",
            )

    def test_no_unchanged_reverse_dependent_becomes_an_owner_or_a_consumer(self) -> None:
        plan = self.plan("darkmatter/lib/src/lib.rs")
        reported = set(plan["reverse_dependencies"])
        self.assertTrue(reported)
        self.assertEqual(
            [],
            sorted({record["package"] for record in plan["builds"]} & reported),
            "a reported dependent must receive no build record",
        )
        areas = {record["area"] for record in plan["areas"]}
        owners = build_owner_matrix(plan)
        self.assertTrue(owners)
        for owner in owners:
            for package in owner["packages"]:
                entry = self.package_record(plan, package)
                self.assertIn(entry["area"], areas)

    def cutover(self, plan: dict) -> set:
        return archive_producers(plan["environments"])

    def test_the_owner_matrix_is_derived_only_from_the_final_plan(self) -> None:
        plan = self.plan(self.THREE_TIER_SOURCE)
        owners = build_owner_matrix(plan)
        cutover = self.cutover(plan)
        self.assertEqual(sorted(cutover), [owner["environment"] for owner in owners])
        runners = {
            environment["name"]: environment["runner"]
            for environment in plan["environments"]
        }
        for owner in owners:
            self.assertEqual(runners[owner["environment"]], owner["runner"])
        self.assertEqual(
            sorted(
                record["key"]
                for record in plan["builds"]
                if record["producer"] in cutover
            ),
            sorted(entry["key"] for owner in owners for entry in owner["builds"]),
            "every cut-over record reaches exactly one owner slice",
        )

    def test_every_native_producer_reaches_the_owner_matrix_after_the_cutover(
        self,
    ) -> None:
        # Phase 5's outcome, as a property of the two workflow-facing
        # projections rather than of a workflow condition: macOS and native
        # Windows own their archives exactly as Linux already did, each on its
        # own runner, and each package's matrix entry names all three.
        plan = self.plan(self.THREE_TIER_SOURCE)
        owners = {owner["environment"]: owner for owner in build_owner_matrix(plan)}
        self.assertEqual(
            ["macos-latest", "ubuntu-latest", "windows-latest"], sorted(owners)
        )
        self.assertEqual("macos-latest", owners["macos-latest"]["runner"])
        self.assertEqual("windows-latest", owners["windows-latest"]["runner"])

        records = self.builds_of(plan, self.THREE_TIER_PACKAGE)
        entry = next(
            item
            for item in legacy_scope_document(plan)["matrix"]
            if item["package"] == self.THREE_TIER_PACKAGE
        )
        self.assertEqual(
            sorted(record["key"] for record in records.values()),
            sorted(item["key"] for item in entry["builds"]),
            "every producer's record now reaches its package's consumers",
        )
        # The one pairing that must stay structurally impossible, restated where
        # the workflow reads it: the Windows consumer's artifact is the Windows
        # owner's, never the Linux one the WSL2 guest shares.
        windows = next(
            item for item in entry["builds"] if item["producer"] == "windows-latest"
        )
        self.assertEqual(records["windows-latest"]["artifact"], windows["artifact"])
        self.assertNotEqual(records["ubuntu-latest"]["artifact"], windows["artifact"])
        self.assertEqual(
            [{"environment": "windows-latest", "gate": "L1"}],
            records["windows-latest"]["consumers"],
        )

    def test_an_environment_that_owns_nothing_reaches_no_owner_job(self) -> None:
        # The archive-only guest is the one environment with no producer
        # contract. It consumes the Linux record and must never appear as an
        # owner, in the matrix or in a package's build references — which is
        # what would send a consumer looking for an artifact nobody uploads.
        plan = self.plan(self.THREE_TIER_SOURCE)
        owners = {owner["environment"] for owner in build_owner_matrix(plan)}
        self.assertNotIn("wsl2-ubuntu", owners)
        self.assertEqual(owners, self.cutover(plan))
        scoped = legacy_scope_document(plan)
        entry = next(
            item
            for item in scoped["matrix"]
            if item["package"] == self.THREE_TIER_PACKAGE
        )
        self.assertEqual(
            set(),
            {item["producer"] for item in entry["builds"]} - owners,
            "a package's matrix entry names only archives an owner will produce",
        )
        # And the guest's cells are still served: by the Linux record, as one of
        # its consumers.
        linux = next(
            record
            for record in plan["builds"]
            if record["package"] == self.THREE_TIER_PACKAGE
            and record["producer"] == "ubuntu-latest"
        )
        self.assertIn(
            "wsl2-ubuntu",
            {consumer["environment"] for consumer in linux["consumers"]},
        )

    def test_the_package_matrix_carries_its_own_build_references(self) -> None:
        plan = self.plan(self.THREE_TIER_SOURCE)
        cutover = self.cutover(plan)
        scheduled = [
            record for record in plan["builds"] if record["producer"] in cutover
        ]
        self.assertTrue(scheduled)
        scoped = legacy_scope_document(plan)
        entry = next(
            item
            for item in scoped["matrix"]
            if item["package"] == self.THREE_TIER_PACKAGE
        )
        self.assertEqual(
            sorted(record["key"] for record in scheduled),
            sorted(item["key"] for item in entry["builds"]),
        )
        self.assertEqual(
            sorted(record["artifact"] for record in scheduled),
            sorted(item["artifact"] for item in entry["builds"]),
        )
        self.assertEqual(scoped["build_owners"], build_owner_matrix(plan))

    def test_the_owner_slices_flatten_to_the_workflow_matrix(self) -> None:
        plan = self.plan(self.THREE_TIER_SOURCE)
        scoped = legacy_scope_document(plan)
        owners = scoped["build_owners"]
        slices = scoped["build_slices"]
        self.assertEqual(
            sorted(entry["artifact"] for owner in owners for entry in owner["builds"]),
            [entry["artifact"] for entry in slices],
            "one leg per record, in deterministic artifact order",
        )
        self.assertEqual(
            [entry["artifact"] for entry in slices], scoped["build_artifacts"]
        )
        self.assertEqual(
            {entry["artifact"]: entry["runner"] for entry in slices},
            scoped["build_runners"],
        )
        for entry in slices:
            self.assertEqual(
                f"build-{entry['package']}-{entry['producer']}-{entry['key']}",
                entry["artifact"],
            )
            self.assertTrue(entry["consumers"], "an owner leg exists only for demand")

    def test_an_owner_leg_carries_its_own_native_prerequisites(self) -> None:
        plan = self.plan(self.THREE_TIER_SOURCE)
        scoped = legacy_scope_document(plan)
        for entry in scoped["build_slices"]:
            declared = self.package_record(plan, entry["package"])["native"]
            self.assertEqual(
                sorted(declared.get(entry["producer"], [])),
                entry["native"],
                "the leg installs the prerequisites of the compile it performs",
            )
        for owner in scoped["build_owners"]:
            self.assertEqual(
                sorted({name for build in owner["builds"] for name in build["native"]}),
                owner["native"],
                "the producer's union is the union of its records",
            )

    def test_one_linux_build_feeds_native_linux_and_the_wsl2_guest(self) -> None:
        # Validation checkpoint 4, as a property of the documents: one key, one
        # artifact, and one owner leg serving the native Linux tiers AND the
        # WSL2 guest — which keeps its own, separate result cell.
        plan = self.plan(self.THREE_TIER_SOURCE)
        record = self.builds_of(plan, self.THREE_TIER_PACKAGE)["ubuntu-latest"]
        consumers = {
            (entry["environment"], entry["gate"]) for entry in record["consumers"]
        }
        self.assertIn(("ubuntu-latest", "L1"), consumers)
        self.assertIn(("wsl2-ubuntu", "L1"), consumers)
        self.assertTrue(
            {gate for environment, gate in consumers if environment == "ubuntu-latest"}
            - {"L1"},
            "this fixture must also carry a Linux L2 or browser consumer",
        )
        self.assertEqual(
            {"ubuntu-latest", "wsl2-ubuntu"},
            {environment for environment, _ in consumers},
            "a Linux archive is consumed by exactly its own environment and its guest",
        )

        # The two environments reach the SAME artifact through the workflow
        # projections, and remain two distinct result cells.
        scoped = legacy_scope_document(plan)
        entry = next(
            item
            for item in scoped["matrix"]
            if item["package"] == self.THREE_TIER_PACKAGE
        )
        linux = [item for item in entry["builds"] if item["producer"] == "ubuntu-latest"]
        self.assertEqual(1, len(linux))
        self.assertEqual(record["artifact"], linux[0]["artifact"])
        self.assertEqual(
            [record["artifact"]],
            [
                slice_["artifact"]
                for slice_ in scoped["build_slices"]
                if slice_["package"] == self.THREE_TIER_PACKAGE
                and slice_["producer"] == "ubuntu-latest"
            ],
            "one owner leg, not one per consuming environment",
        )
        cells = {
            (cell["environment"], cell["gate"])
            for cell in plan["cells"]
            if cell["package"] == self.THREE_TIER_PACKAGE
            and cell.get("build") == record["key"]
        }
        self.assertEqual(consumers, cells)
        self.assertIn(("wsl2-ubuntu", "L1"), cells)
        self.assertIn(("ubuntu-latest", "L1"), cells)

    def test_a_documentation_change_schedules_no_owner_at_all(self) -> None:
        plan = self.plan("docs/architecture.md")
        self.assertEqual([], plan["builds"])
        scoped = legacy_scope_document(plan)
        self.assertEqual([], scoped["build_owners"])
        self.assertEqual([], scoped["build_slices"])
        self.assertEqual([], scoped["build_artifacts"])
        self.assertEqual({}, scoped["build_runners"])


class ScopeReceiptMigrationTests(PlannerFixture):
    """R9/AC10: the inventory bump costs exactly one scope-receipt miss.

    `SCOPE_RECEIPT_SCHEMA_VERSION` does not move. The miss comes from the
    receipt's embedded `plan_schema_version`, which is exactly what makes it a
    single rejection followed by one fresh calculation rather than an in-place
    upgrade of a document written against the previous generation.
    """

    def receipt(self, plan: dict, plan_schema_version: int) -> dict:
        projection = legacy_scope_document(plan)
        return {
            "schema_version": schema.SCOPE_RECEIPT_SCHEMA_VERSION,
            "plan_schema_version": plan_schema_version,
            "base": plan["base"],
            "head": plan["head"],
            "tree": "d" * 40,
            "plan": plan,
            "scope": {
                name: projection[name] for name in schema.SCOPE_PROJECTION_FIELDS
            },
        }

    def test_a_version_2_scope_receipt_misses_once_with_scope_schema(self) -> None:
        plan = self.plan("docs/testing-strategy.md")
        problems = schema.validate_scope_receipt(self.receipt(plan, 2))
        if not problems:
            raise AssertionError(
                "a version-2 scope receipt must miss with scope-schema once "
                "the plan carries the change inventory; it validated cleanly"
            )
        self.assertTrue(
            problems[0].startswith("scope-schema:"),
            f"the miss must use the existing coded reason: {problems}",
        )
        # "One fresh calculation, never an in-place upgrade": the rejected
        # receipt's plan is left exactly as it was found, and the planner's own
        # answer is the current generation.
        self.assertEqual(2, self.receipt(plan, 2)["plan_schema_version"])
        self.assertEqual(
            schema.RESOLVED_PLAN_SCHEMA_VERSION, self.plan("docs/testing-strategy.md")["schema_version"]
        )

    def test_a_current_generation_scope_receipt_still_validates(self) -> None:
        # NOT pending, and the non-vacuity guard for the fixture above: the
        # rejection must come from the version comparison, not from a receipt
        # this fixture builds wrongly.
        plan = self.plan("docs/testing-strategy.md")
        receipt = self.receipt(plan, schema.RESOLVED_PLAN_SCHEMA_VERSION)
        self.assertEqual([], schema.validate_scope_receipt(receipt))


class ChangeInventoryEndToEndTests(PlannerFixture):
    """AC10 through the shipped planner and its normal invocation path.

    The unit fixtures in `test_affected_scope.py` build a synthetic workspace.
    These run the real script the way `ci.yml` and `just/ci-local.just` run it,
    against the real checkout, so a change that only works in a temporary tree
    is still caught.
    """

    #: One real path per bucket. `tools/test-toolkit` is deliberate: it gates
    #: nothing, so R8's "the two are allowed to disagree" is exercised by real
    #: policy rather than by a fixture arranged to produce it.
    FILES = {
        "configuration": ".github/ci/environments.json",
        "documentation": "docs/testing-strategy.md",
        "source": "tools/test-toolkit/src/lib.rs",
        "other": "LICENSE",
    }

    def run_planner(self, *args: str) -> dict:
        completed = subprocess.run(
            [
                sys.executable,
                str(ROOT / "scripts" / "ci" / "affected_scope.py"),
                "--resolved-plan",
                *args,
            ],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
            timeout=300,
        )
        return json.loads(completed.stdout)

    def test_the_cli_buckets_the_real_paths_it_was_handed(self) -> None:
        plan = self.run_planner(*self.FILES.values())
        self.assertEqual([], schema.validate_resolved_plan(plan))
        self.assertEqual(schema.RESOLVED_PLAN_SCHEMA_VERSION, plan["schema_version"])
        inventory = plan["change_inventory"]
        self.assertIs(True, inventory["diff_available"])
        for bucket, path in self.FILES.items():
            self.assertEqual([path], inventory["paths"][bucket])
            self.assertEqual(1, inventory["counts"][bucket])
        self.assertEqual(len(self.FILES), inventory["counts"]["total"])

    def test_the_cli_records_no_diff_inventory_for_a_full_scope_run(self) -> None:
        inventory = self.run_planner("--all")["change_inventory"]
        self.assertIs(False, inventory["diff_available"])
        self.assertTrue(inventory["reason"])
        self.assertNotIn("paths", inventory)
        self.assertNotIn("counts", inventory)

    def test_the_inventory_survives_a_persist_read_persist_cycle(self) -> None:
        # The plan is persisted twice on the receipt path: `--plan-out` writes
        # it, the hook stores it in a Git note, and CI reads it back and applies
        # evidence to it. The inventory describes what changed, which evidence
        # cannot alter, so it must reach the fan-out byte-identical.
        first = schema.canonical(self.plan(*self.FILES.values()))
        applied = apply_accepted_cells(json.loads(first), [], [])
        second = schema.canonical(applied)
        third = schema.canonical(apply_accepted_cells(json.loads(second), [], []))
        self.assertEqual(
            json.loads(first)["change_inventory"],
            json.loads(second)["change_inventory"],
        )
        self.assertEqual(second, third)

    def test_the_inventory_disagrees_with_change_class_where_the_truth_does(
        self,
    ) -> None:
        # R8: the two answer different questions. `Cargo.lock` selects no
        # gating package, so the class is `documentation`; the path is plainly
        # `configuration`, and a reader seeing both is seeing the truth.
        plan = self.plan("Cargo.lock")
        self.assertEqual("documentation", plan["change_class"])
        self.assertEqual(
            ["Cargo.lock"], plan["change_inventory"]["paths"]["configuration"]
        )
        self.assertEqual([], plan["change_inventory"]["paths"]["documentation"])


if __name__ == "__main__":
    unittest.main(verbosity=2)

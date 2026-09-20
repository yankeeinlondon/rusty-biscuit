#!/usr/bin/env python3
"""Contract tests for the resolved plan and the validation receipt."""

from __future__ import annotations

import json
import re
import sys
import unittest
from copy import deepcopy
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import pending_contracts  # noqa: E402
import schema  # noqa: E402
from pending_contracts import ContractLanded, pending  # noqa: E402


SHA_A = "a" * 40
SHA_B = "b" * 40
SHA_T = "c" * 40


def plan(**overrides: object) -> dict:
    document = {
        "schema_version": schema.RESOLVED_PLAN_SCHEMA_VERSION,
        "base": SHA_A,
        "head": SHA_B,
        "change_class": "package",
        "change_inventory": change_inventory(),
        "full_scope": False,
        "full_scope_gates": [],
        "areas": [
            {
                "area": "claudine",
                "selection_reason": "source change under claudine/",
                "packages": ["claudine"],
            }
        ],
        "packages": [package()],
        "source_packages": ["claudine"],
        "reverse_dependencies": ["claudine-cli"],
        "environments": [
            {
                "name": name,
                "runner": "windows-latest" if name == "wsl2-ubuntu" else name,
                "native_key": "ubuntu-latest" if name == "wsl2-ubuntu" else name,
                "capabilities": {
                    "tmux": name in ("ubuntu-latest", "macos-latest"),
                    "headless_browser": name == "ubuntu-latest",
                    "node_pnpm": name == "ubuntu-latest",
                    "archive_only": name == "wsl2-ubuntu",
                },
            }
            for name in schema.ENVIRONMENTS
        ],
        "cells": [cell()],
        "builds": None,
        "accepted_evidence": [],
        "policy_gaps": [],
        "prohibited_cells": [],
        "job_estimate": 3,
        "preflight_os": ["ubuntu-latest"],
        "preflight_reason": "package-local change",
        "flags": {},
    }
    document.update(overrides)
    if document.get("builds") is None:
        document["builds"] = builds_for(document["cells"])
    return document


def change_inventory(**overrides: object) -> dict:
    document = {
        "diff_available": True,
        "paths": {
            "configuration": [],
            "documentation": [],
            "source": ["claudine/lib/src/lib.rs"],
            "other": [],
        },
        "counts": {
            "configuration": 0,
            "documentation": 0,
            "source": 1,
            "other": 0,
            "total": 1,
        },
    }
    document.update(overrides)
    return document


def package(**overrides: object) -> dict:
    record = {
        "package": "claudine",
        "area": "claudine",
        "selection_reason": "source change in claudine/lib/src/lib.rs",
        "gates": ["lint", "check", "L1"],
        "targets": ["lib", "test"],
        "tiers": ["L1"],
        "test_args": "",
        "check_args": "-p claudine",
        "l2_backends": [],
        "runner_tools": [],
        "archive_includes": [],
        "sidecars": [],
        "companion_suites": [],
        "l1_include_slow": False,
        "native": {},
    }
    record.update(overrides)
    return record


def cell(**overrides: object) -> dict:
    record = {
        "package": "claudine",
        "area": "claudine",
        "environment": "ubuntu-latest",
        "gate": "L1",
        "execution": "execute",
        "origin": "ci",
        "state": "pending",
        "reusable": True,
        "target_kinds": ["lib", "test"],
        "compile_coverage_from": "L1",
        "selection_reason": "source package on a required environment",
    }
    record.update(overrides)
    # A build reference belongs to an executing test tier and nowhere else, so
    # the fixture adds one on exactly the shape that must carry it. A case that
    # wants the *absence* asserted passes `build=None` and pops it.
    if (
        "build" not in overrides
        and record["execution"] == "execute"
        and record["gate"] in schema.BUILD_GATES
    ):
        record["build"] = BUILD_KEY
    if record.get("build") is None:
        record.pop("build", None)
    return record


#: One key for the whole module. A case that needs two distinct builds spells
#: the second out rather than deriving it, so a fixture cannot accidentally
#: satisfy a uniqueness rule it meant to break.
BUILD_KEY = "0123456789abcdef"


def build_identity(**overrides: object) -> dict:
    record = {
        "source_commit": SHA_B,
        "lockfile": "fedcba9876543210",
        "rust": "1.97.1",
        "nextest": "latest",
        "host": "x86_64-unknown-linux-gnu",
        "target": "x86_64-unknown-linux-gnu",
        "profile": "test",
        "rustflags": "",
        "cargo_config": [],
        "linker": "cc",
        "archive_format": "tar.zst",
        "package": "claudine",
        "target_kinds": ["lib", "test"],
        "features": "",
        "native": [],
        "archive_includes": [],
        "sidecars": [],
    }
    record.update(overrides)
    return record


def build(**overrides: object) -> dict:
    key = str(overrides.get("key", BUILD_KEY))
    package = str(overrides.get("package", "claudine"))
    producer = str(overrides.get("producer", "ubuntu-latest"))
    record = {
        "key": key,
        "package": package,
        "producer": producer,
        "artifact": f"build-{package}-{producer}-{key}",
        "compatible_environments": ["ubuntu-latest", "wsl2-ubuntu"],
        "compatibility_reason": "x86_64-unknown-linux-gnu archive produced on ubuntu-latest",
        "consumers": [{"environment": "ubuntu-latest", "gate": "L1"}],
        "identity": build_identity(package=package),
    }
    record.update(overrides)
    return record


def builds_for(cells: list[dict]) -> list[dict]:
    """The build records the given cells demand, one per referenced key.

    Derived rather than written out so a fixture that changes a cell's
    execution cannot leave behind a record claiming a consumer that no longer
    exists — which is a different defect from the one most cases are testing.
    """
    demand: dict[str, list[dict]] = {}
    owner: dict[str, dict] = {}
    for entry in cells:
        key = entry.get("build")
        if key is None:
            continue
        demand.setdefault(key, []).append(
            {"environment": entry["environment"], "gate": entry["gate"]}
        )
        owner.setdefault(key, entry)
    records = []
    for key, consumers in sorted(demand.items()):
        consumers.sort(key=lambda item: (item["environment"], item["gate"]))
        records.append(
            build(
                key=key,
                package=owner[key]["package"],
                consumers=consumers,
                compatible_environments=sorted(
                    {entry["environment"] for entry in consumers} | {"ubuntu-latest"}
                ),
            )
        )
    return records


def receipt(**overrides: object) -> dict:
    document = {
        "schema_version": schema.RECEIPT_SCHEMA_VERSION,
        "environment": "macos-latest",
        "base": SHA_A,
        "head": SHA_B,
        "tree": SHA_T,
        "scope_identity": "0123456789abcdef",
        "completion": "complete",
        "host": {
            "os": "MacOS",
            "kernel": "Darwin 27.0.0",
            "report_dir": "~/.rusty-biscuit/ci-evidence/bbbbbbbbb",
        },
        "cells": [receipt_cell()],
    }
    document.update(overrides)
    return document


def receipt_cell(**overrides: object) -> dict:
    record = {
        "package": "claudine",
        "gate": "L1",
        "outcome": "pass",
        "exit_code": 0,
        "completion": "complete",
        "counts": {"total": 10, "passed": 9, "failed": 0, "skipped": 1, "errored": 0},
        "duration_s": 35,
        "gate_input_identity": "deadbeefcafe",
        "backends": [],
        "report": "claudine-L1.xml",
    }
    record.update(overrides)
    return record


class ContractArtifactTests(unittest.TestCase):
    """The shipped `.github/ci/schemas/contract.json` is the cross-language copy."""

    def test_shipped_contract_matches_this_module(self):
        shipped = json.loads(schema.CONTRACT_PATH.read_text(encoding="utf-8"))
        self.assertEqual(
            shipped,
            schema.contract(),
            "contract.json has drifted; regenerate with "
            "`python3 scripts/ci/schema.py`",
        )

    def test_shipped_contract_is_byte_stable(self):
        expected = json.dumps(schema.contract(), indent=2) + "\n"
        self.assertEqual(schema.CONTRACT_PATH.read_text(encoding="utf-8"), expected)

    # -----------------------------------------------------------------------
    # R9: `RESOLVED_PLAN_SCHEMA_VERSION` moves when the plan's required field
    # set does. It reached 4 because build records and the change inventory
    # each claimed 3 on separate branches, so the merged shape needed a version
    # of its own. `RECEIPT_SCHEMA_VERSION`, `LEGACY_RECEIPT_SCHEMA_VERSION`, and
    # `SCOPE_RECEIPT_SCHEMA_VERSION` do NOT move; the scope receipt's embedded
    # `plan_schema_version` check is what produces the one intended miss.
    # -----------------------------------------------------------------------

    def test_the_plan_schema_carries_the_change_inventory_and_builds_at_version_4(self):
        if schema.RESOLVED_PLAN_SCHEMA_VERSION != 4:
            raise AssertionError(
                "the resolved plan schema must be version 4 once it requires "
                "both the change inventory and build records, got "
                f"{schema.RESOLVED_PLAN_SCHEMA_VERSION}"
            )
        self.assertIn(
            "builds",
            schema.RESOLVED_PLAN_FIELDS,
            "version 4 is the union: build records are required too",
        )
        self.assertIn(
            "change_inventory",
            schema.RESOLVED_PLAN_FIELDS,
            "the inventory is a REQUIRED plan field, not an optional sibling",
        )
        self.assertIs(True, schema.RESOLVED_PLAN_FIELDS["change_inventory"])
        shipped = json.loads(schema.CONTRACT_PATH.read_text(encoding="utf-8"))
        self.assertEqual(4, shipped["resolved_plan"]["schema_version"])
        self.assertIn("change_inventory", shipped["resolved_plan"]["document"])

    def test_a_plan_without_the_inventory_is_rejected(self):
        document = plan()
        document.pop("change_inventory", None)
        problems = schema.validate_resolved_plan(document)
        if not problems:
            raise AssertionError(
                "a plan without a change inventory must be rejected; "
                "validate_resolved_plan reported nothing"
            )
        self.assertTrue(
            any("change_inventory" in problem for problem in problems),
            f"the rejection must name the missing field: {problems}",
        )

    def test_the_other_schema_counters_do_not_move(self):
        # NOT pending (R9): pinned so the inventory bump cannot drag a receipt
        # version with it and invalidate evidence this fix never touched.
        self.assertEqual(2, schema.RECEIPT_SCHEMA_VERSION)
        self.assertEqual(1, schema.LEGACY_RECEIPT_SCHEMA_VERSION)
        self.assertEqual(1, schema.SCOPE_RECEIPT_SCHEMA_VERSION)

    def test_vocabularies_are_disjoint_where_they_must_be(self):
        # A cell state and an execution are separate axes; overlapping spellings
        # would let a reader confuse "what will happen" with "what is true now".
        self.assertEqual(set(schema.CELL_STATES) & set(schema.EXECUTIONS), set())

    def test_scope_rejections_are_their_own_vocabulary(self):
        # A scope miss refuses a whole document; a cell rejection refuses one
        # outcome. Sharing a spelling would let a summary conflate the two.
        self.assertEqual(set(schema.SCOPE_REJECTIONS) & set(schema.REJECTIONS), set())
        self.assertTrue(all(code.startswith("scope-") for code in schema.SCOPE_REJECTIONS))
        self.assertEqual(
            schema.contract()["scope_receipt"]["rejections"], list(schema.SCOPE_REJECTIONS)
        )

    def test_accepted_gap_state_is_not_a_github_conclusion(self):
        # Design Decision 10: the machine-readable state is never inferred from
        # GitHub's conclusion, so it must not be spelled like one.
        self.assertNotIn(
            schema.ACCEPTED_GAP_STATE.lower(),
            {"cancelled", "neutral", "skipped", "success", "failure"},
        )


SCHEMA_README_PATH = schema.ROOT / ".github" / "ci" / "schemas" / "README.md"


def schema_readme_prose() -> str:
    """The schema README as one line, with blockquote markers and wrapping removed."""
    lines = [
        re.sub(r"^\s*>\s?", "", line)
        for line in SCHEMA_README_PATH.read_text(encoding="utf-8").splitlines()
    ]
    return re.sub(r"\s+", " ", " ".join(lines))


class SchemaReadmeVersionTests(unittest.TestCase):
    """The schema README states this module's version counters in prose.

    Prose does not check itself: the README went on claiming a baseline
    generation the rollup had already moved past. Every version number it
    states about a document defined here is asserted against the constant
    that defines it. The result and baseline counters belong to
    `scripts/ci-rollup.rs` and are covered by `scripts/ci-rollup-tests.rs`.
    """

    def version_in_table(self, document: str) -> int:
        match = re.search(
            rf"^\|\s*{re.escape(document)}\s*\|\s*(\d+)\s*\|",
            SCHEMA_README_PATH.read_text(encoding="utf-8"),
            re.MULTILINE,
        )
        if match is None:
            raise AssertionError(
                f"the schema README's table states no version for {document!r}; "
                "a reworded row has to be taught to this contract, not ignored"
            )
        return int(match.group(1))

    def test_the_table_states_this_modules_versions(self):
        self.assertEqual(
            schema.RESOLVED_PLAN_SCHEMA_VERSION, self.version_in_table("Resolved plan")
        )
        self.assertEqual(
            schema.RECEIPT_SCHEMA_VERSION, self.version_in_table("Validation receipt")
        )
        self.assertEqual(
            schema.SCOPE_RECEIPT_SCHEMA_VERSION, self.version_in_table("Scope receipt")
        )

    def test_the_prose_version_inventory_states_this_modules_versions(self):
        match = re.search(
            r"versioned independently of the plan's (\d+), the receipt's (\d+)",
            schema_readme_prose(),
        )
        if match is None:
            raise AssertionError(
                "the schema README no longer states the plan and receipt versions "
                "alongside the rollup's own; a reworded inventory has to be taught "
                "to this contract, not ignored"
            )
        self.assertEqual(schema.RESOLVED_PLAN_SCHEMA_VERSION, int(match.group(1)))
        self.assertEqual(schema.RECEIPT_SCHEMA_VERSION, int(match.group(2)))

    def test_the_prose_restates_the_receipt_version_correctly(self):
        prose = schema_readme_prose()
        for pattern in (
            r"Both receipt producers.*?write version (\d+)",
            r"validation receipts are untouched, so their version stays at (\d+)",
        ):
            match = re.search(pattern, prose)
            if match is None:
                raise AssertionError(
                    f"the schema README no longer states {pattern!r}; a reworded "
                    "claim has to be taught to this contract, not ignored"
                )
            self.assertEqual(schema.RECEIPT_SCHEMA_VERSION, int(match.group(1)), pattern)


class ChangeInventoryValidationTests(unittest.TestCase):
    """AC10: a malformed inventory is a rejected plan, not a rendered lie.

    Every reader of the plan — the local renderer, `ci-reporting`, a human
    reading the artifact — takes the inventory at face value, so each invariant
    it rests on is refused here rather than discovered downstream.
    """

    def problems(self, **overrides: object) -> list[str]:
        return schema.validate_resolved_plan(
            plan(change_inventory=change_inventory(**overrides))
        )

    def assert_named(self, problems: list[str], fragment: str) -> None:
        self.assertTrue(problems, "the defect was accepted")
        self.assertTrue(
            all(problem.startswith("malformed-receipt:") for problem in problems),
            f"every inventory problem carries a code: {problems}",
        )
        self.assertTrue(
            any(fragment in problem for problem in problems),
            f"no problem named {fragment!r}: {problems}",
        )

    def test_a_no_diff_inventory_validates_with_its_reason(self):
        document = plan(
            change_inventory={"diff_available": False, "reason": "manual full scope"}
        )
        self.assertEqual([], schema.validate_resolved_plan(document))

    def test_a_no_diff_inventory_must_state_why(self):
        document = plan(change_inventory={"diff_available": False})
        self.assert_named(schema.validate_resolved_plan(document), "must state why")

    def test_a_no_diff_inventory_may_not_also_carry_buckets(self):
        document = plan(
            change_inventory={
                "diff_available": False,
                "reason": "manual full scope",
                "paths": {bucket: [] for bucket in schema.CHANGE_BUCKETS},
            }
        )
        self.assert_named(schema.validate_resolved_plan(document), "'paths'")

    def test_a_diff_inventory_may_not_also_carry_an_absence_reason(self):
        self.assert_named(self.problems(reason="both at once"), "absence reason")

    def test_a_diff_inventory_must_carry_its_buckets(self):
        document = plan(change_inventory={"diff_available": True})
        self.assert_named(schema.validate_resolved_plan(document), "'paths'")

    def test_an_unknown_bucket_is_rejected(self):
        paths = {bucket: [] for bucket in schema.CHANGE_BUCKETS}
        paths["vendored"] = []
        self.assert_named(self.problems(paths=paths), "must name exactly")

    def test_an_unsorted_bucket_is_rejected(self):
        self.assert_named(
            self.problems(
                paths={
                    "configuration": [],
                    "documentation": ["docs/b.md", "docs/a.md"],
                    "source": ["claudine/lib/src/lib.rs"],
                    "other": [],
                },
                counts={
                    "configuration": 0,
                    "documentation": 2,
                    "source": 1,
                    "other": 0,
                    "total": 3,
                },
            ),
            "is not sorted",
        )

    def test_a_windows_spelled_path_is_rejected(self):
        self.assert_named(
            self.problems(
                paths={
                    "configuration": [],
                    "documentation": [],
                    "source": ["claudine\\lib\\src\\lib.rs"],
                    "other": [],
                }
            ),
            "not a normalized repository-relative path",
        )

    def test_a_path_in_two_buckets_is_rejected(self):
        self.assert_named(
            self.problems(
                paths={
                    "configuration": [],
                    "documentation": ["README.md"],
                    "source": ["README.md"],
                    "other": [],
                },
                counts={
                    "configuration": 0,
                    "documentation": 1,
                    "source": 1,
                    "other": 0,
                    "total": 2,
                },
            ),
            "more than one bucket",
        )

    def test_a_count_that_disagrees_with_its_bucket_is_rejected(self):
        self.assert_named(
            self.problems(
                counts={
                    "configuration": 0,
                    "documentation": 0,
                    "source": 7,
                    "other": 0,
                    "total": 7,
                }
            ),
            "but the bucket holds 1",
        )

    def test_a_total_that_disagrees_with_the_buckets_is_rejected(self):
        self.assert_named(
            self.problems(
                counts={
                    "configuration": 0,
                    "documentation": 0,
                    "source": 1,
                    "other": 0,
                    "total": 99,
                }
            ),
            "but the buckets hold 1",
        )

    def test_an_unknown_inventory_field_is_rejected(self):
        self.assert_named(self.problems(change_class="documentation"), "unknown field")

    def test_a_non_object_inventory_is_rejected(self):
        self.assert_named(
            schema.validate_resolved_plan(plan(change_inventory=[])), "must be an object"
        )


class ResolvedPlanValidationTests(unittest.TestCase):
    def test_a_well_formed_plan_validates(self):
        self.assertEqual(schema.validate_resolved_plan(plan()), [])

    def test_round_trip_is_byte_identical(self):
        once = schema.canonical(plan())
        twice = schema.canonical(json.loads(once))
        thrice = schema.canonical(json.loads(twice))
        self.assertEqual(once, twice)
        self.assertEqual(twice, thrice)
        self.assertEqual(schema.validate_resolved_plan(json.loads(thrice)), [])

    def test_a_future_schema_version_is_rejected_by_code(self):
        problems = schema.validate_resolved_plan(plan(schema_version=99))
        self.assertEqual(len(problems), 1)
        self.assertTrue(problems[0].startswith("unknown-schema-version:"))

    def test_a_stale_version_is_named_even_when_the_shape_also_differs(self):
        # Why the version check precedes the field check. A real version-3
        # document is not just mislabeled — it also lacks a field this reader
        # requires, and reporting the field sends a reader after a corrupt
        # document when the answer is that the tool moved on.
        document = plan(schema_version=3)
        del document["change_inventory"]
        problems = schema.validate_resolved_plan(document)
        self.assertEqual(len(problems), 1)
        self.assertEqual(
            "unknown-schema-version: resolved plan is version 3, this tool writes "
            f"{schema.RESOLVED_PLAN_SCHEMA_VERSION}",
            problems[0],
        )

    def test_a_missing_field_names_the_field(self):
        document = plan()
        del document["cells"]
        self.assertIn(
            "malformed-receipt: resolved plan is missing required field 'cells'",
            schema.validate_resolved_plan(document),
        )

    def test_an_unknown_field_is_rejected(self):
        problems = schema.validate_resolved_plan(plan(excluded_environment="macos-latest"))
        self.assertIn(
            "malformed-receipt: resolved plan has unknown field 'excluded_environment'",
            problems,
        )

    def test_an_area_record_that_is_not_an_object_is_refused(self):
        # A bare name where a record belongs is the shape a hand-edited plan
        # drifts into; every field check downstream would read it as a string.
        self.assertIn(
            "malformed-receipt: area record must be an object",
            schema.validate_resolved_plan(plan(areas=["claudine"])),
        )

    def test_a_base_or_head_that_is_not_a_full_object_id_is_refused(self):
        # Both ends are checked, because evidence reuse compares the pair and
        # an abbreviated revision on either side would compare unequal.
        for field in ("base", "head"):
            with self.subTest(field=field):
                self.assertIn(
                    f"malformed-receipt: resolved plan {field} is not a full Git "
                    "object ID: 'abc1234'",
                    schema.validate_resolved_plan(plan(**{field: "abc1234"})),
                )

    def test_source_packages_must_be_a_list_of_strings(self):
        self.assertIn(
            "malformed-receipt: resolved plan source_packages must be a list of strings",
            schema.validate_resolved_plan(plan(source_packages="claudine")),
        )

    def test_an_environment_table_without_named_records_is_refused(self):
        # The table is what the fan-out reads `runs-on` from; a record with no
        # name would schedule a job nothing could attribute to a cell.
        self.assertIn(
            "malformed-receipt: resolved plan environments must be a list of named records",
            schema.validate_resolved_plan(plan(environments=[{"runner": "ubuntu-latest"}])),
        )

    def test_a_negative_job_estimate_is_refused(self):
        self.assertIn(
            "malformed-receipt: resolved plan job_estimate must be a non-negative integer",
            schema.validate_resolved_plan(plan(job_estimate=-1)),
        )

    def test_a_reverse_dependency_that_also_holds_a_package_record_is_refused(self):
        # AC1: naming a dependent is reporting, never selecting.
        self.assertIn(
            "unknown-package: 'claudine' is reported as an unchanged reverse "
            "dependency and must not also hold a package record",
            schema.validate_resolved_plan(plan(reverse_dependencies=["claudine"])),
        )

    def test_a_seam_dependent_that_also_holds_a_package_record_is_refused(self):
        # The seam compiles a dependent inside the changed package's check cell
        # precisely because the plan does not select it on its own.
        document = plan(
            packages=[
                package(
                    dependent_seam={
                        "dependents": ["claudine-cli"],
                        "check_args": "-p claudine-cli",
                        "native": [],
                    }
                ),
                package(package="claudine-cli"),
            ],
            reverse_dependencies=[],
        )
        self.assertIn(
            "unknown-package: 'claudine-cli' is compiled as a dependent of 'claudine' "
            "and must not also hold a package record",
            schema.validate_resolved_plan(document),
        )

    def test_a_package_naming_an_unselected_area_is_refused(self):
        self.assertIn(
            "unknown-package: package 'claudine' names area 'playa', which the plan "
            "does not select",
            schema.validate_resolved_plan(plan(packages=[package(area="playa")])),
        )

    def test_a_non_boolean_l1_include_slow_is_refused(self):
        # It is projected into the legacy matrix verbatim, so a truthy string
        # would reach the test job as the enabled policy.
        self.assertIn(
            "malformed-receipt: package claudine l1_include_slow must be a boolean",
            schema.validate_resolved_plan(plan(packages=[package(l1_include_slow="yes")])),
        )

    def test_an_exclusion_on_a_package_that_gates_something_is_refused(self):
        document = plan(
            packages=[
                package(
                    exclusion={
                        "class": "unsupported",
                        "owner": "@o",
                        "reason": "no backend",
                        "expiry": "2027-01-31",
                    }
                )
            ]
        )
        self.assertIn(
            "malformed-receipt: package claudine carries an exclusion exactly when it "
            "gates nothing",
            schema.validate_resolved_plan(document),
        )

    def test_a_dependent_seam_that_is_not_an_object_is_refused(self):
        self.assertIn(
            "malformed-receipt: package claudine dependent_seam must be an object",
            schema.validate_resolved_plan(
                plan(packages=[package(dependent_seam="claudine-cli")])
            ),
        )

    def test_a_dependent_seam_with_no_dependent_is_refused(self):
        document = plan(
            packages=[
                package(
                    dependent_seam={
                        "dependents": [],
                        "check_args": "-p claudine-cli",
                        "native": [],
                    }
                )
            ]
        )
        self.assertIn(
            "malformed-receipt: package claudine dependent_seam names no dependent; a "
            "record with none to compile carries no seam at all",
            schema.validate_resolved_plan(document),
        )

    def test_a_dependent_seam_without_check_args_is_refused(self):
        # The seam exists so the check cell compiles the dependents explicitly;
        # with no arguments it would compile the changed package a second time.
        document = plan(
            packages=[
                package(
                    dependent_seam={
                        "dependents": ["claudine-cli"],
                        "check_args": "",
                        "native": [],
                    }
                )
            ]
        )
        self.assertIn(
            "malformed-receipt: package claudine dependent_seam check_args must be a "
            "non-empty string",
            schema.validate_resolved_plan(document),
        )

    def test_a_cell_carrying_dependents_outside_check_is_refused(self):
        self.assertIn(
            "malformed-receipt: cell claudine/ubuntu-latest/L1 carries dependents but "
            "only a check cell compiles them",
            schema.validate_resolved_plan(plan(cells=[cell(dependents=["claudine-cli"])])),
        )

    def test_area_must_be_derived_from_the_package_record(self):
        problems = schema.validate_resolved_plan(plan(cells=[cell(area="playa")]))
        self.assertTrue(
            any("area is derived from package" in problem for problem in problems),
            problems,
        )

    def test_every_cell_carries_a_package_identity(self):
        # The build the cell demands is unselected for the same reason, and it
        # answers to the same code, so only the exact text pins this rule.
        self.assertIn(
            "unknown-package: cell ghost/ubuntu-latest/L1 has no package record; "
            "package is the stored identity and every cell must carry one",
            schema.validate_resolved_plan(plan(cells=[cell(package="ghost")])),
        )

    def test_every_area_package_and_cell_states_a_selection_reason(self):
        # Three separate rules end in the same words, so matching the shared
        # tail would also pass on the wrong one of the three firing.
        for mutate, expected in (
            (
                lambda doc: doc["areas"][0].update({"selection_reason": ""}),
                "malformed-receipt: area 'claudine' has no selection reason",
            ),
            (
                lambda doc: doc["packages"][0].update({"selection_reason": ""}),
                "malformed-receipt: package 'claudine' has no selection reason",
            ),
            (
                lambda doc: doc["cells"][0].update({"selection_reason": ""}),
                "malformed-receipt: cell claudine/ubuntu-latest/L1 has no selection "
                "reason",
            ),
        ):
            document = plan()
            mutate(document)
            self.assertIn(expected, schema.validate_resolved_plan(document))

    def test_a_reused_cell_must_name_its_evidence(self):
        problems = schema.validate_resolved_plan(
            plan(cells=[cell(execution="reuse", origin="local", state="reused")])
        )
        self.assertTrue(
            any(problem.startswith("missing-receipt:") for problem in problems), problems
        )

    def test_a_reused_cell_may_not_claim_ci_origin(self):
        problems = schema.validate_resolved_plan(
            plan(
                cells=[
                    cell(
                        execution="reuse",
                        origin="ci",
                        state="reused",
                        evidence={"ref": "refs/notes/ci-local/macos-latest"},
                    )
                ]
            )
        )
        self.assertTrue(any("is reused but its origin" in p for p in problems), problems)

    def test_an_accepted_gap_without_a_policy_entry_is_rejected(self):
        problems = schema.validate_resolved_plan(
            plan(cells=[cell(execution="omit", origin="none", state="accepted-gap")])
        )
        self.assertTrue(
            any("ungoverned gap" in problem for problem in problems), problems
        )

    def test_a_prohibited_cell_may_not_be_scheduled(self):
        problems = schema.validate_resolved_plan(
            plan(
                cells=[
                    cell(
                        execution="execute",
                        origin="ci",
                        state="prohibited",
                        prohibition={"reason": "do not rerun WSL", "owner": "ken"},
                    )
                ]
            )
        )
        self.assertTrue(
            any("prohibited yet scheduled to execute" in p for p in problems), problems
        )

    def test_a_non_boolean_reusable_is_refused(self):
        # Reusability is decided once at resolution time and read later without
        # policy, so a string here would be read as permission to reuse.
        self.assertIn(
            "malformed-receipt: cell claudine/ubuntu-latest/L1 reusable must be a boolean",
            schema.validate_resolved_plan(plan(cells=[cell(reusable="yes")])),
        )

    def test_a_reused_cell_that_no_evidence_may_satisfy_is_refused(self):
        document = plan(
            cells=[
                cell(
                    execution="reuse",
                    origin="local",
                    state="reused",
                    reusable=False,
                    evidence={"ref": "refs/notes/ci-local/ubuntu-latest"},
                )
            ]
        )
        self.assertIn(
            "malformed-receipt: cell claudine/ubuntu-latest/L1 is reused although no "
            "evidence may satisfy it",
            schema.validate_resolved_plan(document),
        )

    def test_an_executing_cell_with_a_non_ci_origin_is_refused(self):
        # The mirror of the reuse rule: what CI runs, CI owns.
        self.assertIn(
            "malformed-receipt: cell claudine/ubuntu-latest/L1 will execute but its "
            "origin is 'local'",
            schema.validate_resolved_plan(plan(cells=[cell(origin="local")])),
        )

    def test_a_prohibited_cell_without_a_constraint_is_refused(self):
        document = plan(cells=[cell(execution="omit", origin="none", state="prohibited")])
        self.assertIn(
            "malformed-receipt: cell claudine/ubuntu-latest/L1 is prohibited but names "
            "no constraint",
            schema.validate_resolved_plan(document),
        )

    def test_a_reused_state_without_a_reuse_execution_is_refused(self):
        # State and execution are separate axes, so nothing but this rule keeps
        # a cell from reporting reuse while it is scheduled to run.
        self.assertIn(
            "malformed-receipt: cell claudine/ubuntu-latest/L1 is in state 'reused' but "
            "its execution is 'execute'",
            schema.validate_resolved_plan(plan(cells=[cell(state="reused")])),
        )

    def test_unknown_environment_and_gate_use_their_own_codes(self):
        problems = schema.validate_resolved_plan(
            plan(cells=[cell(environment="freebsd", gate="L9")])
        )
        self.assertTrue(any(p.startswith("unknown-environment:") for p in problems), problems)
        self.assertTrue(any(p.startswith("unknown-gate:") for p in problems), problems)

    def test_blanket_all_targets_is_not_a_target_kind(self):
        problems = schema.validate_resolved_plan(
            plan(cells=[cell(target_kinds=["all-targets"])])
        )
        self.assertTrue(
            any("unknown value 'all-targets'" in problem for problem in problems), problems
        )


class BuildRecordValidationTests(unittest.TestCase):
    """Build ownership, machine-checked before any workflow acts on the plan.

    `fixes/2026-09-12-single-os-compile/spec.md` Design Decision 1: "Plan
    validation rejects dangling references, unconsumed builds, duplicate owners
    for one planned build key, and a consumer whose environment is incompatible
    with the producer."
    """

    def linux_and_wsl_plan(self) -> dict:
        """One Linux build serving distinct Linux and WSL2 result cells.

        The shape spec section 5 is written around: two `{package, environment,
        gate}` results, one archive, one owner.
        """
        cells = [
            cell(environment="ubuntu-latest", gate="L1"),
            cell(environment="wsl2-ubuntu", gate="L1", compile_coverage_from="ubuntu-latest archive build"),
        ]
        return plan(cells=cells)

    # -- positive shapes ----------------------------------------------------

    def test_one_linux_build_serves_distinct_linux_and_wsl2_cells(self):
        document = self.linux_and_wsl_plan()
        self.assertEqual([], schema.validate_resolved_plan(document))
        self.assertEqual(1, len(document["builds"]))
        record = document["builds"][0]
        self.assertEqual(
            [
                {"environment": "ubuntu-latest", "gate": "L1"},
                {"environment": "wsl2-ubuntu", "gate": "L1"},
            ],
            record["consumers"],
        )
        self.assertEqual(
            {record["key"]}, {entry["build"] for entry in document["cells"]}
        )

    def test_a_mixed_reuse_execute_and_gap_plan_validates(self):
        document = plan(
            cells=[
                cell(environment="ubuntu-latest", gate="L1"),
                cell(
                    environment="macos-latest",
                    gate="L1",
                    execution="reuse",
                    origin="local",
                    state="reused",
                    evidence={"ref": "refs/notes/ci-local/macos-latest"},
                ),
                cell(
                    environment="windows-latest",
                    gate="L2",
                    execution="omit",
                    origin="none",
                    state="accepted-gap",
                    gap={"owner": "@o", "reason": "no backend", "expiry": "2027-01-31"},
                ),
            ]
        )
        self.assertEqual([], schema.validate_resolved_plan(document))
        # Only the executing cell demands a build.
        self.assertEqual(1, len(document["builds"]))
        self.assertEqual(
            [{"environment": "ubuntu-latest", "gate": "L1"}],
            document["builds"][0]["consumers"],
        )

    def test_an_all_reused_plan_carries_no_build_at_all(self):
        document = plan(
            cells=[
                cell(
                    execution="reuse",
                    origin="local",
                    state="reused",
                    evidence={"ref": "refs/notes/ci-local/ubuntu-latest"},
                )
            ]
        )
        self.assertEqual([], schema.validate_resolved_plan(document))
        self.assertEqual([], document["builds"])

    # -- refusals -----------------------------------------------------------

    def test_a_test_execution_without_a_build_is_refused(self):
        document = plan()
        del document["cells"][0]["build"]
        problems = schema.validate_resolved_plan(document)
        self.assertTrue(
            any("will execute but references no build" in p for p in problems), problems
        )

    def test_a_dangling_build_reference_is_refused(self):
        document = plan()
        document["cells"][0]["build"] = "ffffffffffffffff"
        problems = schema.validate_resolved_plan(document)
        self.assertTrue(
            any("which the plan does not carry" in p for p in problems), problems
        )

    def test_an_unconsumed_build_is_refused(self):
        document = plan(cells=[cell()], builds=[build(consumers=[])])
        problems = schema.validate_resolved_plan(document)
        self.assertTrue(any("has no consumer" in p for p in problems), problems)

    def test_two_owners_for_one_build_key_are_refused(self):
        document = plan(
            cells=[cell()],
            builds=[build(), build(producer="macos-latest")],
        )
        problems = schema.validate_resolved_plan(document)
        self.assertTrue(any("has two owners" in p for p in problems), problems)

    def test_an_artifact_that_copies_a_sibling_records_name_is_refused(self):
        # The two artifact cases deliberately reach one rule from opposite
        # sides: here the name is well formed but belongs to another key, and
        # in the test below it is not derived from `{package, producer, key}`
        # at all. A hand-spelled name is what makes both reachable.
        document = plan(
            cells=[cell()],
            builds=[build(), build(key="fedcba9876543210", artifact=build()["artifact"])],
        )
        problems = schema.validate_resolved_plan(document)
        self.assertIn(
            "malformed-receipt: build claudine/ubuntu-latest/fedcba9876543210 artifact "
            f"is 'build-claudine-ubuntu-latest-{BUILD_KEY}', expected "
            "'build-claudine-ubuntu-latest-fedcba9876543210'",
            problems,
        )

    def test_an_artifact_that_is_not_package_keyed_is_refused(self):
        document = plan(builds=[build(artifact="build-claudine-L1")])
        problems = schema.validate_resolved_plan(document)
        self.assertTrue(any("artifact is 'build-claudine-L1'" in p for p in problems), problems)

    def test_a_consumer_outside_the_producers_compatibility_is_refused(self):
        # Native Windows may never execute the Linux owner's archive.
        document = plan(
            cells=[cell(environment="windows-latest")],
            builds=[
                build(
                    consumers=[{"environment": "windows-latest", "gate": "L1"}],
                    compatible_environments=["ubuntu-latest", "wsl2-ubuntu"],
                )
            ],
        )
        problems = schema.validate_resolved_plan(document)
        self.assertTrue(
            any("does not declare compatible" in p for p in problems), problems
        )

    def test_a_producer_outside_its_own_compatibility_list_is_refused(self):
        document = plan(builds=[build(compatible_environments=["wsl2-ubuntu"])])
        problems = schema.validate_resolved_plan(document)
        self.assertTrue(
            any("not among its own compatible_environments" in p for p in problems),
            problems,
        )

    def test_unsorted_or_duplicated_consumers_are_refused(self):
        document = plan(
            cells=[
                cell(environment="ubuntu-latest"),
                cell(environment="wsl2-ubuntu"),
            ],
            builds=[
                build(
                    consumers=[
                        {"environment": "wsl2-ubuntu", "gate": "L1"},
                        {"environment": "ubuntu-latest", "gate": "L1"},
                    ]
                )
            ],
        )
        problems = schema.validate_resolved_plan(document)
        self.assertTrue(any("must be sorted by" in p for p in problems), problems)

    def test_a_consumer_the_cells_do_not_demand_is_refused(self):
        document = plan(
            cells=[cell(environment="ubuntu-latest")],
            builds=[
                build(
                    consumers=[
                        {"environment": "ubuntu-latest", "gate": "L1"},
                        {"environment": "wsl2-ubuntu", "gate": "L1"},
                    ]
                )
            ],
        )
        problems = schema.validate_resolved_plan(document)
        self.assertTrue(
            any("but the cells referencing it are" in p for p in problems), problems
        )

    def test_a_build_attached_to_lint_or_check_is_refused(self):
        for gate in ("lint", "check"):
            document = plan(
                cells=[cell(gate=gate, target_kinds=[], build=BUILD_KEY)],
                builds=[build()],
            )
            problems = schema.validate_resolved_plan(document)
            # One rule covers both halves of "only an executing test tier" —
            # the gate being wrong and the cell not executing — so only the
            # cell label in the exact text says which half reached it.
            self.assertIn(
                f"malformed-receipt: cell claudine/ubuntu-latest/{gate} references "
                f"build '{BUILD_KEY}', but only an executing L1, L2, or browser "
                "cell consumes one",
                problems,
            )

    def test_a_reused_cell_may_not_reference_a_build(self):
        document = plan(
            cells=[
                cell(
                    execution="reuse",
                    origin="local",
                    state="reused",
                    evidence={"ref": "refs/notes/ci-local/ubuntu-latest"},
                    build=BUILD_KEY,
                )
            ],
            builds=[build()],
        )
        problems = schema.validate_resolved_plan(document)
        # The label naming a build gate is what separates this from the
        # lint/check case above: on `L1` the rule is only reachable because the
        # cell does not execute.
        self.assertIn(
            f"malformed-receipt: cell claudine/ubuntu-latest/L1 references build "
            f"'{BUILD_KEY}', but only an executing L1, L2, or browser cell "
            "consumes one",
            problems,
        )

    def test_a_build_compiling_another_package_than_its_consumer_is_refused(self):
        document = plan(builds=[build(package="playa")])
        problems = schema.validate_resolved_plan(document)
        self.assertTrue(any("which compiles 'playa'" in p for p in problems), problems)

    def test_a_build_for_an_unselected_package_is_refused(self):
        document = plan(
            cells=[cell()],
            builds=[build(), build(key="fedcba9876543210", package="ghost", consumers=[])],
        )
        problems = schema.validate_resolved_plan(document)
        self.assertTrue(
            any(p.startswith("unknown-package:") for p in problems), problems
        )

    def test_unsorted_compatible_environments_are_refused(self):
        # The list is part of what a reader compares between plan and manifest,
        # so one order is one contract.
        document = plan(
            builds=[build(compatible_environments=["wsl2-ubuntu", "ubuntu-latest"])]
        )
        self.assertIn(
            f"malformed-receipt: build claudine/ubuntu-latest/{BUILD_KEY} "
            "compatible_environments must be sorted and free of duplicates",
            schema.validate_resolved_plan(document),
        )

    def test_a_build_without_a_compatibility_reason_is_refused(self):
        self.assertIn(
            f"malformed-receipt: build claudine/ubuntu-latest/{BUILD_KEY} has no "
            "compatibility reason",
            schema.validate_resolved_plan(plan(builds=[build(compatibility_reason="")])),
        )

    def test_a_producer_outside_the_plans_environment_table_is_refused(self):
        document = plan(
            builds=[
                build(
                    producer="freebsd",
                    artifact=f"build-claudine-freebsd-{BUILD_KEY}",
                    compatible_environments=["ubuntu-latest"],
                )
            ]
        )
        problems = schema.validate_resolved_plan(document)
        self.assertTrue(
            any(p.startswith("unknown-environment:") for p in problems), problems
        )

    # -- identity -----------------------------------------------------------

    def test_the_key_must_be_a_sixteen_digit_hex_digest(self):
        # A SHA-256 here would mean the key came from somewhere other than the
        # one `ci-build key` boundary.
        for forged in ("0" * 64, "not-a-digest", "ABCDEF0123456789"):
            document = plan(
                cells=[cell(build=forged)],
                builds=[build(key=forged, artifact=f"build-claudine-ubuntu-latest-{forged}")],
            )
            problems = schema.validate_resolved_plan(document)
            self.assertTrue(
                any("16-digit hex build key" in p for p in problems), (forged, problems)
            )

    def test_every_unhashed_identity_field_is_required(self):
        for field in schema.BUILD_IDENTITY_FIELDS:
            identity = build_identity()
            del identity[field]
            problems = schema.validate_resolved_plan(plan(builds=[build(identity=identity)]))
            self.assertTrue(
                any(f"missing required field '{field}'" in p for p in problems),
                (field, problems),
            )

    def test_an_identity_naming_another_package_is_refused(self):
        document = plan(builds=[build(identity=build_identity(package="playa"))])
        problems = schema.validate_resolved_plan(document)
        self.assertTrue(any("identity names package 'playa'" in p for p in problems), problems)

    def test_an_empty_identity_field_is_refused(self):
        # Present-but-empty is the drift `_keys` cannot catch: the field is
        # there, so the key is computed over a configuration nobody declared.
        document = plan(builds=[build(identity=build_identity(profile=""))])
        self.assertIn(
            f"malformed-receipt: build claudine/ubuntu-latest/{BUILD_KEY} identity "
            "profile must be a non-empty string",
            schema.validate_resolved_plan(document),
        )

    def test_identity_rustflags_and_features_must_be_strings(self):
        # These two are the fields an absent value is spelled `""` for, so
        # `None` would hash differently from the empty configuration it means.
        document = plan(builds=[build(identity=build_identity(rustflags=None))])
        self.assertIn(
            f"malformed-receipt: build claudine/ubuntu-latest/{BUILD_KEY} identity "
            "rustflags and features must be strings; an absent value is the empty string",
            schema.validate_resolved_plan(document),
        )

    def test_identity_lists_must_be_sorted_so_one_configuration_is_one_key(self):
        document = plan(
            builds=[build(identity=build_identity(sidecars=["messenger-desktop-stubs", "darkmatter-md-fixture"]))]
        )
        problems = schema.validate_resolved_plan(document)
        self.assertTrue(any("identity sidecars must be sorted" in p for p in problems), problems)

    def test_builds_must_be_a_list(self):
        problems = schema.validate_resolved_plan(plan(builds={"key": "x"}))
        self.assertTrue(any("builds must be a list" in p for p in problems), problems)


class ReceiptValidationTests(unittest.TestCase):
    def test_a_well_formed_receipt_validates(self):
        self.assertEqual(schema.validate_receipt(receipt()), [])

    def test_round_trip_is_byte_identical(self):
        once = schema.canonical(receipt())
        twice = schema.canonical(json.loads(once))
        self.assertEqual(once, twice)
        self.assertEqual(schema.validate_receipt(json.loads(twice)), [])

    def test_a_version_one_note_is_legacy_not_malformed(self):
        problems = schema.validate_receipt({"schema_version": 1, "environment": "macos-latest"})
        self.assertEqual(len(problems), 1)
        self.assertTrue(problems[0].startswith("v1-requires-exact-tree:"))

    def test_the_v1_measurement_text_is_exact(self):
        self.assertEqual(schema.UNRECORDED_MEASUREMENT, "not recorded (v1 receipt)")

    def test_counts_must_add_up(self):
        broken = receipt_cell(
            counts={"total": 10, "passed": 1, "failed": 0, "skipped": 0, "errored": 0}
        )
        problems = schema.validate_receipt(receipt(cells=[broken]))
        self.assertTrue(any("parts sum to 1" in problem for problem in problems), problems)

    def test_a_failure_must_name_a_failing_test_or_declare_truncation(self):
        problems = schema.validate_receipt(
            receipt(cells=[receipt_cell(outcome="fail", exit_code=100)])
        )
        self.assertTrue(
            any("names no failing test" in problem for problem in problems), problems
        )
        truncated = receipt_cell(
            outcome="fail", exit_code=100, failed_tests=[], failure_detail_truncated=True
        )
        self.assertEqual(schema.validate_receipt(receipt(cells=[truncated])), [])

    def test_failure_detail_is_bounded(self):
        overflowing = receipt_cell(
            outcome="fail",
            exit_code=100,
            failed_tests=[f"t{index}" for index in range(schema.FAILURE_DETAIL_LIMIT + 1)],
        )
        problems = schema.validate_receipt(receipt(cells=[overflowing]))
        self.assertTrue(
            any("failure_detail_truncated" in problem for problem in problems), problems
        )

    def test_every_cell_carries_a_gate_input_identity(self):
        problems = schema.validate_receipt(
            receipt(cells=[receipt_cell(gate_input_identity="")])
        )
        self.assertTrue(
            any("no gate-input identity" in problem for problem in problems), problems
        )

    def test_two_records_for_one_cell_that_disagree_are_conflicting(self):
        problems = schema.validate_receipt(
            receipt(cells=[receipt_cell(), receipt_cell(outcome="fail", failed_tests=["t"])])
        )
        self.assertTrue(
            any(problem.startswith("conflicting-evidence:") for problem in problems), problems
        )

    def test_two_identical_records_for_one_cell_agree(self):
        self.assertEqual(
            schema.validate_receipt(receipt(cells=[receipt_cell(), receipt_cell()])), []
        )

    def test_an_empty_receipt_is_rejected(self):
        problems = schema.validate_receipt(receipt(cells=[]))
        self.assertTrue(any("non-empty list" in problem for problem in problems), problems)


def scope_receipt(**overrides: object) -> dict:
    document = {
        "schema_version": schema.SCOPE_RECEIPT_SCHEMA_VERSION,
        "base": SHA_A,
        "head": SHA_B,
        "tree": SHA_T,
        "plan_schema_version": schema.RESOLVED_PLAN_SCHEMA_VERSION,
        "plan": plan(),
        "scope": {name: [] for name in schema.SCOPE_PROJECTION_FIELDS} | {"packages": ["claudine"]},
    }
    document.update(overrides)
    return document


class ScopeReceiptValidationTests(unittest.TestCase):
    def test_a_well_formed_scope_receipt_validates(self):
        self.assertEqual(schema.validate_scope_receipt(scope_receipt()), [])

    def test_round_trip_is_byte_identical(self):
        once = schema.canonical(scope_receipt())
        twice = schema.canonical(json.loads(once))
        self.assertEqual(once, twice)

    def test_every_problem_is_a_scope_code(self):
        broken = [
            scope_receipt(schema_version=99),
            scope_receipt(plan_schema_version=99),
            scope_receipt(tree="not-a-sha"),
            scope_receipt(plan=plan(cells=[cell(package="ghost")])),
            scope_receipt(plan=plan(head=SHA_T)),
            scope_receipt(scope={"packages": ["claudine"]}),
            scope_receipt(scope={name: [] for name in schema.SCOPE_PROJECTION_FIELDS} | {"packages": ["other"]}),
            {"schema_version": schema.SCOPE_RECEIPT_SCHEMA_VERSION},
            "not an object",
        ]
        for document in broken:
            problems = schema.validate_scope_receipt(document)
            self.assertTrue(problems, document)
            for problem in problems:
                self.assertIn(problem.split(":")[0], schema.SCOPE_REJECTIONS, problem)

    def test_a_schema_mismatch_is_reported_as_scope_schema(self):
        # A receipt one generation behind is the live migration case: the plan
        # it carries lacks the fields CI needs to apply evidence and project
        # it, so it must miss as `scope-schema` rather than hit.
        other = schema.SCOPE_RECEIPT_SCHEMA_VERSION + 1
        previous_plan = schema.RESOLVED_PLAN_SCHEMA_VERSION - 1
        self.assertTrue(schema.validate_scope_receipt(scope_receipt(schema_version=other))[0].startswith("scope-schema:"))
        self.assertTrue(schema.validate_scope_receipt(scope_receipt(plan_schema_version=previous_plan))[0].startswith("scope-schema:"))

    def test_a_version_two_plan_misses_whole_rather_than_being_upgraded(self):
        # The live migration case for `fixes/2026-09-12-single-os-compile`: a
        # receipt written before build records carries cells that name no
        # build. CI must decline the whole document and resolve the plan
        # itself — deriving the missing builds here would mean inventing
        # ownership for a selection this tool did not make.
        stale = plan()
        del stale["builds"]
        for entry in stale["cells"]:
            entry.pop("build", None)
        stale["schema_version"] = schema.RESOLVED_PLAN_SCHEMA_VERSION - 1
        receipt = scope_receipt(
            plan=stale, plan_schema_version=schema.RESOLVED_PLAN_SCHEMA_VERSION - 1
        )
        problems = schema.validate_scope_receipt(receipt)
        self.assertTrue(problems[0].startswith("scope-schema:"), problems)
        self.assertEqual(1, len(problems), problems)
        # Refused as a document, not repaired in place.
        self.assertNotIn("builds", receipt["plan"])
        self.assertTrue(all("build" not in cell for cell in receipt["plan"]["cells"]))

    def test_the_carried_plan_must_name_the_receipts_base_and_head(self):
        problems = schema.validate_scope_receipt(scope_receipt(plan=plan(base=SHA_T)))
        self.assertTrue(any("names base" in problem for problem in problems), problems)

    def test_the_carried_projection_must_agree_with_the_plan(self):
        projection = {name: [] for name in schema.SCOPE_PROJECTION_FIELDS} | {"packages": []}
        problems = schema.validate_scope_receipt(scope_receipt(scope=projection))
        self.assertTrue(any("other packages" in problem for problem in problems), problems)

    def test_the_projection_fields_are_what_the_workflow_reads(self):
        # `ci.yml` reads these keys of `scope.json`; a projection missing one
        # would fail the fan-out after the planner was already skipped.
        workflow = (schema.ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
        for name in ("matrix", "scheduled_areas", "area_matrix", "area_slugs", "packages",
                     "full_scope", "flags", "job_estimate", "preflight_os",
                     "preflight_reason", "change_class"):
            self.assertIn(name, schema.SCOPE_PROJECTION_FIELDS)
            self.assertIn(f".{name}", workflow)


class ReusableCellsTests(unittest.TestCase):
    def test_a_complete_pass_is_reusable(self):
        accepted, rejected = schema.reusable_cells(receipt())
        self.assertEqual([entry["package"] for entry in accepted], ["claudine"])
        self.assertEqual(rejected, [])

    def test_a_complete_failure_is_diagnostic_only(self):
        failing = receipt(
            cells=[receipt_cell(outcome="fail", exit_code=100, failed_tests=["boom"])]
        )
        accepted, rejected = schema.reusable_cells(failing)
        self.assertEqual(accepted, [])
        self.assertEqual(
            rejected,
            ["failed-cell: claudine/macos-latest/L1 failed; failing evidence is diagnostic only"],
        )

    def test_an_interrupted_run_contributes_nothing(self):
        accepted, rejected = schema.reusable_cells(receipt(completion="interrupted"))
        self.assertEqual(accepted, [])
        self.assertTrue(rejected[0].startswith("incomplete-run:"), rejected)

    def test_an_interrupted_cell_leaves_its_siblings_reusable(self):
        mixed = receipt(
            cells=[
                receipt_cell(),
                receipt_cell(package="claudine-cli", completion="interrupted"),
            ]
        )
        accepted, rejected = schema.reusable_cells(mixed)
        self.assertEqual([entry["package"] for entry in accepted], ["claudine"])
        self.assertEqual(len(rejected), 1)
        self.assertIn("claudine-cli/macos-latest/L1", rejected[0])

    def test_a_malformed_receipt_reuses_nothing(self):
        broken = deepcopy(receipt())
        del broken["tree"]
        accepted, rejected = schema.reusable_cells(broken)
        self.assertEqual(accepted, [])
        self.assertTrue(rejected)


# ---------------------------------------------------------------------------
# Direct cell execution (features/2026-09-19-direct-cell-execution), Phase 2.
#
# Every fixture below is a pending oracle: it asserts the version-5 plan
# contract Phase 3 of that feature implements, and today it fails for the
# recorded reason. The shapes are pinned here so Phase 3 implements exactly
# this vocabulary:
#
# - plan-level `skip_policy` (REQUIRED): the snapshot of
#   `.github/ci/ci-baseline.toml` with `{source, content_hash, entries}` and
#   per-entry `{package, environment, gate, owner, reason, source_run}` plus
#   optional `backend` and `expiry`;
# - area-level `execution_path` (REQUIRED): "rows" or "lists" (ruling R9);
# - cell-level `profile` (optional in the table, required by consistency on
#   exactly an executing L1/L2/browser cell) and `requires_node` (optional
#   boolean, absent reads as false);
# - new rejection codes `skip-policy-cell`, `skip-policy-expired`, and
#   `skip-policy-provenance`, joining REJECTIONS;
# - `validate_resolved_plan(document, today=None)`: the optional date is what
#   turns an expired approval into a planner-time coded error (ruling R8).
# ---------------------------------------------------------------------------


def skip_policy(**overrides: object) -> dict:
    """A well-formed snapshot of an empty baseline file.

    The shipped `ci-baseline.toml` is empty, so the honest snapshot carries no
    entries; the fixture adds one through `entries=` where a case needs it.
    """
    document: dict = {
        "source": ".github/ci/ci-baseline.toml",
        "content_hash": "0123456789abcdef",
        "entries": [],
    }
    entries = overrides.pop("entries", None)
    if entries is not None:
        document["entries"] = entries
    document.update(overrides)
    return document


def skip_entry(**overrides: object) -> dict:
    record: dict = {
        "package": "claudine",
        "environment": "ubuntu-latest",
        "gate": "L1",
        "owner": "@ken",
        "reason": "flaky under load; tracked in the baseline file",
        "source_run": "12345678901",
    }
    record.update(overrides)
    return record


class DirectExecutionSchemaOracleTests(unittest.TestCase):
    """Pending contracts for the version-5 resolved plan (Phase 3 implements)."""

    def v5_plan(self, **overrides: object) -> dict:
        """A complete version-5 document: today's shape plus the new fields."""
        document = plan(
            skip_policy=skip_policy(
                entries=[skip_entry(backend="tmux", expiry="2027-06-30")]
            ),
        )
        document["schema_version"] = 5
        for entry in document["areas"]:
            entry["execution_path"] = "rows"
        for entry in document["cells"]:
            if entry["execution"] == "execute" and entry["gate"] in schema.BUILD_GATES:
                entry["profile"] = "ci"
        document.update(overrides)
        return document

    @pending(
        "schema-v5",
        "the resolved plan schema is still version 4; Phase 3 bumps it to 5 "
        "with the skip-policy snapshot beside the cells",
        oracle="version-5 plan schema",
    )
    def test_version_5_is_the_plans_schema_and_the_receipts_do_not_move(self):
        self.assertEqual(
            5,
            schema.RESOLVED_PLAN_SCHEMA_VERSION,
            "the direct-cell-execution fields force a version-5 plan: a "
            "version-5 plan schema is required once rows and the skip policy "
            "travel in the document",
        )
        self.assertEqual([], schema.validate_resolved_plan(self.v5_plan(), today="2026-09-20"))
        # The version-5 plan schema moves alone: validation receipts stay
        # reusable under their existing cell and gate-input checks, and the
        # scope receipt keeps missing through its plan_schema_version check.
        self.assertEqual(2, schema.RECEIPT_SCHEMA_VERSION)
        self.assertEqual(1, schema.LEGACY_RECEIPT_SCHEMA_VERSION)
        self.assertEqual(1, schema.SCOPE_RECEIPT_SCHEMA_VERSION)
        shipped = json.loads(schema.CONTRACT_PATH.read_text(encoding="utf-8"))
        self.assertEqual(5, shipped["resolved_plan"]["schema_version"])

    @pending(
        "schema-v5",
        "a version-4 document is still this tool's current generation, so it "
        "validates; Phase 3 makes it miss as unknown-schema-version",
        oracle="version-5 plan schema",
    )
    def test_a_version_4_plan_is_refused_by_version_before_field_set(self):
        self.assertEqual(
            5,
            schema.RESOLVED_PLAN_SCHEMA_VERSION,
            "a version-4 refusal only exists once the tool writes a version-5 "
            "plan schema",
        )
        stale = self.v5_plan()
        stale["schema_version"] = 4
        # A stale document also differs in shape; the version complaint must
        # still be the only one, or a reader is sent after a corrupt document
        # when the answer is that this tool moved on.
        del stale["skip_policy"]
        problems = schema.validate_resolved_plan(stale)
        self.assertEqual(
            [
                "unknown-schema-version: resolved plan is version 4, this tool "
                "writes 5"
            ],
            problems,
        )

    @pending(
        "schema-v5",
        "the skip-policy snapshot and the per-cell execution fields are not "
        "part of the plan vocabulary yet",
        oracle="skip_policy",
    )
    def test_the_new_execution_fields_are_required_where_the_spec_requires_them(self):
        self.assertIn(
            "skip_policy",
            schema.RESOLVED_PLAN_FIELDS,
            "skip_policy is a REQUIRED plan field: an optional snapshot would "
            "let a plan silently lack the exact-skip policy (skip_policy is "
            "the version-5 field this contract waits for)",
        )
        self.assertIs(True, schema.RESOLVED_PLAN_FIELDS["skip_policy"])
        self.assertIn("execution_path", schema.AREA_FIELDS)
        self.assertIs(True, schema.AREA_FIELDS["execution_path"])
        # Optional in the table, required by consistency exactly where the
        # specification assigns them.
        self.assertIs(False, schema.CELL_FIELDS.get("profile"))
        self.assertIs(False, schema.CELL_FIELDS.get("requires_node"))

        missing_policy = self.v5_plan()
        del missing_policy["skip_policy"]
        self.assertIn(
            "malformed-receipt: resolved plan is missing required field 'skip_policy'",
            schema.validate_resolved_plan(missing_policy, today="2026-09-20"),
        )
        missing_path = self.v5_plan()
        for entry in missing_path["areas"]:
            entry.pop("execution_path")
        self.assertTrue(
            any(
                "execution_path" in problem
                for problem in schema.validate_resolved_plan(
                    missing_path, today="2026-09-20"
                )
            ),
            "an area record without its execution path must be refused by name",
        )

    @pending(
        "schema-v5",
        "no skip-policy validation exists: an entry naming an unknown cell "
        "passes unnoticed because the plan carries no skip_policy at all",
        oracle="skip-policy-cell",
    )
    def test_an_entry_naming_a_cell_the_plan_does_not_carry_is_rejected(self):
        self.assertIn(
            "skip_policy",
            schema.RESOLVED_PLAN_FIELDS,
            "the skip-policy-cell rejection this fixture waits for cannot exist "
            "while the plan carries no skip_policy",
        )
        document = self.v5_plan(
            skip_policy=skip_policy(
                entries=[
                    skip_entry(),
                    skip_entry(package="claudine", environment="macos-latest", gate="lint"),
                ]
            ),
        )
        problems = schema.validate_resolved_plan(document, today="2026-09-20")
        self.assertTrue(
            any(problem.startswith("skip-policy-cell:") for problem in problems),
            f"an approval for a cell the plan does not carry is a coded "
            f"rejection, not a silent pass: {problems}",
        )

    @pending(
        "schema-v5",
        "no skip-policy validation exists: an expired entry passes unnoticed",
        oracle="skip-policy-expired",
    )
    def test_an_expired_entry_is_rejected_with_a_coded_reason(self):
        self.assertIn(
            "skip_policy",
            schema.RESOLVED_PLAN_FIELDS,
            "the skip-policy-expired rejection this fixture waits for cannot "
            "exist while the plan carries no skip_policy",
        )
        document = self.v5_plan(
            skip_policy=skip_policy(entries=[skip_entry(expiry="2026-01-01")]),
        )
        problems = schema.validate_resolved_plan(document, today="2026-09-20")
        self.assertTrue(
            any(problem.startswith("skip-policy-expired:") for problem in problems),
            f"an expired approval is a planner-time error (ruling R8): {problems}",
        )
        # An unexpired entry on the same shape stays valid, so the rule is the
        # expiry and not the presence of the field.
        fresh = self.v5_plan(
            skip_policy=skip_policy(entries=[skip_entry(expiry="2027-06-30")]),
        )
        self.assertEqual([], schema.validate_resolved_plan(fresh, today="2026-09-20"))

    @pending(
        "schema-v5",
        "no skip-policy validation exists: malformed provenance passes "
        "unnoticed",
        oracle="skip-policy-provenance",
    )
    def test_malformed_provenance_is_rejected_with_a_coded_reason(self):
        self.assertIn(
            "skip_policy",
            schema.RESOLVED_PLAN_FIELDS,
            "the skip-policy-provenance rejection this fixture waits for cannot "
            "exist while the plan carries no skip_policy",
        )
        no_hash = skip_policy()
        del no_hash["content_hash"]
        for broken in (
            skip_policy(content_hash="not-hex!"),
            skip_policy(source=""),
            no_hash,
        ):
            with self.subTest(policy=broken):
                document = self.v5_plan(skip_policy=broken)
                problems = schema.validate_resolved_plan(document, today="2026-09-20")
                self.assertTrue(
                    any(
                        problem.startswith("skip-policy-provenance:")
                        for problem in problems
                    ),
                    f"provenance names the file and its content hash; a "
                    f"snapshot that cannot say what it read excuses nothing: "
                    f"{problems}",
                )

    @pending(
        "schema-v5",
        "the per-cell execution inputs are not validated: a test cell without "
        "its canonical profile passes unnoticed",
        oracle="profile",
    )
    def test_a_profile_belongs_to_exactly_an_executing_test_cell(self):
        self.assertIn("profile", schema.CELL_FIELDS)
        document = self.v5_plan()
        executing_l1 = next(
            entry
            for entry in document["cells"]
            if entry["gate"] == "L1" and entry["execution"] == "execute"
        )
        del executing_l1["profile"]
        problems = schema.validate_resolved_plan(document, today="2026-09-20")
        self.assertTrue(
            any("profile" in problem for problem in problems),
            f"an executing test cell without its canonical profile leaves the "
            f"consumer to guess the selection: {problems}",
        )
        # The negative half: a lint cell carries no nextest profile at all.
        linting = self.v5_plan(
            cells=[cell(gate="lint", execution="execute", build=None, profile="ci")],
        )
        linting["cells"][0].pop("build", None)
        problems = schema.validate_resolved_plan(linting, today="2026-09-20")
        self.assertTrue(
            any("profile" in problem for problem in problems),
            f"a lint gate runs no nextest selection; a profile on it is a "
            f"misbinding: {problems}",
        )

    @pending(
        "schema-v5",
        "the skip-policy codes are not in the rejection vocabulary yet",
        oracle="skip-policy-cell",
    )
    def test_the_skip_policy_codes_join_the_rejection_vocabulary(self):
        for code in ("skip-policy-cell", "skip-policy-expired", "skip-policy-provenance"):
            self.assertIn(code, schema.REJECTIONS)
        self.assertIn("skip-policy-cell", schema.contract()["vocabulary"]["rejections"])


class PendingHarnessTests(unittest.TestCase):
    """The harness itself, so a pending fixture cannot pass vacuously."""

    def setUp(self):
        # These assert the harness's own branching, so they must see it in its
        # normal mode even under BISCUIT_PROMOTE_PENDING.
        self.promote = pending_contracts.PROMOTE
        pending_contracts.PROMOTE = False

    def tearDown(self):
        pending_contracts.PROMOTE = self.promote

    def test_a_body_failing_for_the_recorded_reason_passes(self):
        @pending("X", "not built yet", oracle="expected marker")
        def body(_self):
            raise AssertionError("expected marker in the failure")

        body(self)

    def test_a_body_failing_for_another_reason_reports_broken_setup(self):
        @pending("X", "not built yet", oracle="expected marker")
        def body(_self):
            raise AssertionError("no such file or directory")

        with self.assertRaises(AssertionError) as caught:
            body(self)
        self.assertIn("not for the recorded reason", str(caught.exception))

    def test_a_body_that_passes_demands_promotion(self):
        @pending("X", "not built yet", oracle="expected marker")
        def body(_self):
            return None

        with self.assertRaises(ContractLanded) as caught:
            body(self)
        self.assertIn("Remove the @pending decorator", str(caught.exception))


if __name__ == "__main__":
    unittest.main(verbosity=2)

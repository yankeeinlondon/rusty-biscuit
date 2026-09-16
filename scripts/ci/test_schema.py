#!/usr/bin/env python3
"""Contract tests for the resolved plan and the validation receipt."""

from __future__ import annotations

import json
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
        "full_scope": False,
        "full_scope_gates": [],
        "areas": [
            {
                "area": "claudine",
                "selection_reason": "source change under claudine/",
                "packages": ["claudine"],
            }
        ],
        "packages": [
            {
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
        ],
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
        "flags": {"ci_tooling": False},
    }
    document.update(overrides)
    if document.get("builds") is None:
        document["builds"] = builds_for(document["cells"])
    return document


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

    def test_area_must_be_derived_from_the_package_record(self):
        problems = schema.validate_resolved_plan(plan(cells=[cell(area="playa")]))
        self.assertTrue(
            any("area is derived from package" in problem for problem in problems),
            problems,
        )

    def test_every_cell_carries_a_package_identity(self):
        problems = schema.validate_resolved_plan(plan(cells=[cell(package="ghost")]))
        self.assertTrue(
            any(problem.startswith("unknown-package:") for problem in problems), problems
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

    def test_an_artifact_name_collision_is_refused(self):
        # Two records that agree on `{package, producer, key}` except for the
        # key would still collide if either spelled its artifact by hand.
        document = plan(
            cells=[cell()],
            builds=[build(), build(key="fedcba9876543210", artifact=build()["artifact"])],
        )
        problems = schema.validate_resolved_plan(document)
        self.assertTrue(any("artifact is" in p for p in problems), problems)

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

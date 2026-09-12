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
                "companion_suites": [],
                "native": {},
            }
        ],
        "source_packages": ["claudine"],
        "reverse_dependencies": ["claudine-cli"],
        "cells": [cell()],
        "accepted_evidence": [],
        "policy_gaps": [],
        "prohibited_cells": [],
        "job_estimate": 3,
        "preflight_os": ["ubuntu-latest"],
        "preflight_reason": "package-local change",
        "flags": {"ci_tooling": False},
    }
    document.update(overrides)
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
        "target_kinds": ["lib", "test"],
        "compile_coverage_from": "L1",
        "selection_reason": "source package on a required environment",
    }
    record.update(overrides)
    return record


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
        for mutate in (
            lambda doc: doc["areas"][0].update({"selection_reason": ""}),
            lambda doc: doc["packages"][0].update({"selection_reason": ""}),
            lambda doc: doc["cells"][0].update({"selection_reason": ""}),
        ):
            document = plan()
            mutate(document)
            problems = schema.validate_resolved_plan(document)
            self.assertTrue(
                any("selection reason" in problem for problem in problems), problems
            )

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
        self.assertTrue(schema.validate_scope_receipt(scope_receipt(schema_version=2))[0].startswith("scope-schema:"))
        self.assertTrue(schema.validate_scope_receipt(scope_receipt(plan_schema_version=2))[0].startswith("scope-schema:"))

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

    def test_a_complete_failure_is_still_reusable_evidence(self):
        failing = receipt(
            cells=[receipt_cell(outcome="fail", exit_code=100, failed_tests=["boom"])]
        )
        accepted, rejected = schema.reusable_cells(failing)
        self.assertEqual(rejected, [])
        self.assertEqual(accepted[0]["outcome"], "fail")

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

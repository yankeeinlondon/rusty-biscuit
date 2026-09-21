#!/usr/bin/env python3
"""Producer-completeness contracts (AC5) for `scripts/ci/completion.py`.

Phase 2 of `features/2026-09-19-direct-cell-execution` landed every fixture
here as a pending oracle; Phase 4 implemented the validator and promoted them,
so each body now asserts behavior that holds and a regression fails this suite.

## The contract these fixtures pin

The validator is `scripts/ci/completion.py` (Python 3 stdlib only, ruling R2),
invoked once per executing cell by the producer job after the gate:

```sh
python3 scripts/ci/completion.py \
    --plan ci-artifacts/ci-resolved-plan/resolved-plan.json \
    --cell claudine/ubuntu-latest/L1 \
    --expected-manifest target/nextest/ci-reports/expected-L1.json \
    --artifacts ci-artifacts \
    --companions "$RUNNER_TEMP/companions.json" \
    --backend-proofs "$RUNNER_TEMP/backend-proofs.json" \
    --target x86_64-unknown-linux-gnu \
    --run "$GITHUB_RUN_ID" --attempt "$GITHUB_RUN_ATTEMPT" \
    --out "$RUNNER_TEMP/completion.json"
```

- Exit `0` and the `--out` record is written exactly when validation
  succeeds; a refused validation exits nonzero, names its reason on stderr,
  and writes **no** completion record — `complete: true` cannot exist without
  the evidence behind it (ruling R10).
- Expected and observed are compared as **identities** (`<testsuite
  name>::<testcase name>`), never as counts, scoped by cell, backend, and
  companion suite.
- The expected manifest is schema version 2: explicit `tests`, `ignored`, and
  `excluded` identity sets per package plus provenance — `environment`,
  `tier`, `target`, `nextest_version`, `from_archive`, and the `selection`
  (`{filter, profile, test_args}`) the listing applied. A v1 manifest, a
  missing provenance field, or a target other than the comparison target is
  refused.
- The companions input is `companion_suites.py`'s document; the backend-proof
  input is a `{backend: {"proven": true}}` map.
- The completion record is keyed `{package, environment, gate}` and binds
  `head`, `run`, `attempt`, `nextest_version`, the report `reports` inventory,
  and `build` where the cell consumed an archive.

The two workflow-level AC5 cases — an upload failure failing the job, and
cancellation keeping diagnostics best-effort — are frozen as pending contracts
in `tools/test-toolkit/tests/ci_workflow_contracts.rs`, where the upload steps
live.

## What is checked against the shipped artifacts

`ShippedSelectionTests` reads the real `_tier_filter` for every tier and
asserts the validator accepts it, so the canonical-selection rule cannot
tighten until a shipped expression stops passing it.
`ShippedRecipeEndToEndTests` drives the real `just _expected_manifest` over a
scratch crate carrying one plain, one `#[ignore]`d, and one tier-marked test,
then validates the manifest it produced with the real CLI — the whole path,
through the shipped recipe, with nothing hand-written in between.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import affected_scope  # noqa: E402
import completion  # noqa: E402
import schema  # noqa: E402
import tool_guard  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
COMPLETION = ROOT / "scripts" / "ci" / "completion.py"

VALIDATOR_ORACLE = "the completion validator is not implemented"

SHA_A = "a" * 40
SHA_B = "b" * 40

TARGET = "x86_64-unknown-linux-gnu"
NEXTEST_VERSION = "cargo-nextest 0.9.136 (1d5bf1ec9 2026-05-16)"


# ---------------------------------------------------------------------------
# Fixture builders
# ---------------------------------------------------------------------------


def junit_case(name: str, outcome: str = "pass") -> str:
    body = {
        "pass": "",
        "fail": '<failure type="test failure">boom</failure>',
        "skip": "<skipped/>",
    }[outcome]
    return (
        f'        <testcase name="{name}" classname="claudine" '
        f'time="0.1">{body}</testcase>'
    )


def junit_document(cases: list[str], suite: str = "claudine") -> str:
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n'
        '<testsuites name="nextest-run" tests="0" failures="0" errors="0">\n'
        f'    <testsuite name="{suite}" tests="0" disabled="0" errors="0" '
        f'failures="0">\n'
        + "\n".join(cases)
        + "\n    </testsuite>\n"
        "</testsuites>\n"
    )


def package_record(**overrides: object) -> dict:
    record: dict = {
        "package": "claudine",
        "area": "claudine",
        "selection_reason": "fixture",
        "gates": ["lint", "check", "L1"],
        "targets": ["lib", "test"],
        "tiers": ["L1"],
        "test_args": "",
        "check_args": "-p claudine",
        "l2_backends": [],
        "runner_tools": [],
        "companion_suites": [],
        "archive_includes": [],
        "sidecars": [],
        "l1_include_slow": False,
        "native": {},
    }
    record.update(overrides)
    return record


def cell_record(**overrides: object) -> dict:
    record: dict = {
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
        "selection_reason": "fixture",
    }
    record.update(overrides)
    return record


def plan_document(**overrides: object) -> dict:
    """A minimal valid plan for one executing cell, at whatever version the
    schema currently writes.

    The version-5 fields (`skip_policy`, `execution_path`, `profile`) are
    added exactly when the schema requires them, so these fixtures work both
    before Phase 3 (the validator ignores the plan's version) and after it.
    """
    document: dict = {
        "schema_version": schema.RESOLVED_PLAN_SCHEMA_VERSION,
        "base": SHA_A,
        "head": SHA_B,
        "change_class": "package",
        "change_inventory": {"diff_available": False, "reason": "fixture"},
        "full_scope": False,
        "full_scope_gates": [],
        "areas": [
            {
                "area": "claudine",
                "selection_reason": "fixture",
                "packages": ["claudine"],
            }
        ],
        "packages": [package_record()],
        "source_packages": ["claudine"],
        "reverse_dependencies": [],
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
        "cells": [cell_record()],
        "builds": [],
        "accepted_evidence": [],
        "policy_gaps": [],
        "prohibited_cells": [],
        "job_estimate": 1,
        "preflight_os": ["ubuntu-latest"],
        "preflight_reason": "fixture",
        "flags": {},
    }
    document.update(overrides)
    if schema.RESOLVED_PLAN_FIELDS.get("skip_policy") and "skip_policy" not in document:
        document["skip_policy"] = {
            "source": ".github/ci/ci-baseline.toml",
            "content_hash": "0123456789abcdef",
            "entries": [],
        }
    if "execution_path" in schema.AREA_FIELDS:
        for area in document["areas"]:
            area.setdefault("execution_path", "rows")
    if "profile" in schema.CELL_FIELDS:
        for cell in document["cells"]:
            if (
                cell.get("execution") == "execute"
                and cell.get("gate") in schema.BUILD_GATES
            ):
                cell.setdefault("profile", "ci")
    return document


def skip_policy_with(**entry: object) -> dict:
    document: dict = {
        "source": ".github/ci/ci-baseline.toml",
        "content_hash": "0123456789abcdef",
        "entries": [],
    }
    if entry:
        document["entries"] = [
            {
                "package": "claudine",
                "environment": "ubuntu-latest",
                "gate": "L1",
                "owner": "@ken",
                "reason": "flaky under load",
                "source_run": "12345678901",
                **entry,
            }
        ]
    return document


def expected_manifest(**overrides: object) -> dict:
    """An expected-test manifest at schema version 2.

    The default lists the identities the happy-path JUnit reports, so a
    fixture changes one thing at a time.
    """
    document: dict = {
        "schema_version": 2,
        "environment": "ubuntu-latest",
        "tier": "L1",
        "target": TARGET,
        "nextest_version": NEXTEST_VERSION,
        "from_archive": False,
        "selection": {"filter": "package(claudine)", "profile": "ci", "test_args": ""},
        "packages": {
            "claudine": {
                "tests": ["claudine::green_one", "claudine::green_two"],
                "ignored": [],
                "excluded": [],
            }
        },
    }
    document.update(overrides)
    return document


def companions_document(outcomes: dict[str, str]) -> dict:
    """`companion_suites.py`'s result document, spelled for the given suites."""
    return {
        name: {
            "outcome": outcome,
            "duration_s": 1.5,
            **({"counts": {"total": 3, "passed": 3, "failed": 0, "skipped": 0, "errored": 0}}
               if outcome == "success" else
               {"reason": "the producer recorded no counts"}),
        }
        for name, outcome in outcomes.items()
    }


def backend_proofs_document(proven: list[str]) -> dict:
    return {backend: {"proven": True} for backend in proven}


# ---------------------------------------------------------------------------
# The harness
# ---------------------------------------------------------------------------


class ValidatorHarness(unittest.TestCase):
    """Shared harness. Declares no test methods, so each subclass runs only
    its own fixtures rather than inheriting a second copy of the matrix."""

    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="completion-oracle-")
        self.root = Path(self.temporary.name)
        self.addCleanup(self.temporary.cleanup)

    def require_validator(self) -> None:
        if not COMPLETION.is_file():
            raise AssertionError(
                f"{VALIDATOR_ORACLE}: scripts/ci/completion.py does not exist, "
                "so a producer cannot prove it ran the tests it expected"
            )

    def staging_tree(
        self,
        files: dict[str, str],
        entries: list[dict],
        directory: str = "junit-claudine-L1-ubuntu-latest",
    ) -> Path:
        """Write a JUnit staging tree the way `_stage_junit` leaves it.

        `files` maps XML-relative paths to XML documents; `entries` are the
        `manifest.jsonl` lines (the `xml` field names the same paths).
        """
        artifacts = self.root / "ci-artifacts"
        base = artifacts / directory
        for relative, document in files.items():
            target = base / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(document, encoding="utf-8")
        base.mkdir(parents=True, exist_ok=True)
        with (base / "manifest.jsonl").open("w", encoding="utf-8") as manifest:
            for entry in entries:
                manifest.write(json.dumps(entry) + "\n")
        return artifacts

    def manifest_entry(self, xml: str, **overrides: object) -> dict:
        entry: dict = {
            "tier": "L1",
            "package": "claudine",
            "xml": xml,
            "exit_code": 0,
            "environment": "ubuntu-latest",
            "duration_s": 7,
            "report_present": True,
        }
        entry.update(overrides)
        return entry

    def green_tree(self) -> Path:
        """The staging tree of a fully green, fully reported L1 run."""
        return self.staging_tree(
            {"L1/claudine.xml": junit_document(
                [junit_case("green_one"), junit_case("green_two")]
            )},
            [self.manifest_entry("L1/claudine.xml")],
        )

    def validate(
        self,
        *,
        plan: dict | None = None,
        manifest: dict | None = None,
        artifacts: Path | None = None,
        companions: dict | None = None,
        backend_proofs: dict | None = None,
        cell: str = "claudine/ubuntu-latest/L1",
        target: str = TARGET,
        include_manifest: bool = True,
    ) -> tuple[int, str, Path]:
        """Run the real CLI the producer runs; return exit, stderr, --out path."""
        self.require_validator()
        plan_path = self.root / "resolved-plan.json"
        plan_path.write_text(
            json.dumps(plan if plan is not None else plan_document()), encoding="utf-8"
        )
        manifest_path = self.root / "expected-L1.json"
        manifest_path.write_text(
            json.dumps(manifest if manifest is not None else expected_manifest()),
            encoding="utf-8",
        )
        companions_path = self.root / "companions.json"
        companions_path.write_text(
            json.dumps(companions if companions is not None else {}), encoding="utf-8"
        )
        proofs_path = self.root / "backend-proofs.json"
        proofs_path.write_text(
            json.dumps(backend_proofs if backend_proofs is not None else {}),
            encoding="utf-8",
        )
        out_path = self.root / "completion.json"
        manifest_argument = (
            ["--expected-manifest", str(manifest_path)] if include_manifest else []
        )
        completed = subprocess.run(
            [
                sys.executable,
                str(COMPLETION),
                "--plan",
                str(plan_path),
                "--cell",
                cell,
                *manifest_argument,
                "--artifacts",
                str(artifacts if artifacts is not None else self.green_tree()),
                "--companions",
                str(companions_path),
                "--backend-proofs",
                str(proofs_path),
                "--target",
                target,
                "--run",
                "123456",
                "--attempt",
                "1",
                "--out",
                str(out_path),
            ],
            capture_output=True,
            text=True,
            timeout=120,
            cwd=ROOT,
        )
        return completed.returncode, completed.stderr, out_path

    def record(self, path: Path) -> dict:
        return json.loads(path.read_text(encoding="utf-8"))


class CompletionValidatorOracleTests(ValidatorHarness):
    """The AC5 fixture matrix, against the shipped validator."""

    # -- the happy path every failure case is measured against --------------

    def test_a_green_fully_reported_run_completes_and_binds_its_evidence(self):
        code, stderr, out = self.validate()
        self.assertEqual(
            0, code, f"a green, fully reported run must complete: {stderr}"
        )
        self.assertTrue(out.is_file(), "validation success writes the record")
        record = self.record(out)
        self.assertEqual("claudine", record["package"])
        self.assertEqual("ubuntu-latest", record["environment"])
        self.assertEqual("L1", record["gate"])
        self.assertIs(True, record["complete"])
        self.assertEqual(SHA_B, record["head"])
        self.assertEqual("123456", str(record["run"]))
        self.assertEqual(1, record["attempt"])
        self.assertEqual(NEXTEST_VERSION, record["nextest_version"])
        self.assertEqual(["L1/claudine.xml"], record["reports"])

    # -- reports ------------------------------------------------------------

    def test_a_missing_report_fails_validation(self):
        artifacts = self.staging_tree(
            {},
            [self.manifest_entry("L1/claudine.xml")],
        )
        code, stderr, out = self.validate(artifacts=artifacts)
        self.assertNotEqual(0, code, "a manifest entry with no XML behind it fails")
        self.assertIn("report", stderr)
        self.assertFalse(out.exists(), "a refused validation writes no record")

    def test_a_malformed_report_fails_validation(self):
        artifacts = self.staging_tree(
            {"L1/claudine.xml": "<not-xml[{"},
            [self.manifest_entry("L1/claudine.xml")],
        )
        code, stderr, out = self.validate(artifacts=artifacts)
        self.assertNotEqual(0, code, "an unparseable report is not evidence")
        self.assertIn("malformed", stderr.lower())
        self.assertFalse(out.exists())

    # -- expected versus observed identities ---------------------------------

    def test_a_missing_expected_test_fails_and_a_skip_approval_cannot_excuse_it(self):
        # The report covers one of two expected tests; the approval exists, is
        # unexpired, and names exactly this cell — the strongest approval a
        # fixture can hand the validator. The specification's one intentional
        # tightening: an observed, approved skip is a skip; an ABSENT test is
        # a lost test.
        missing = self.staging_tree(
            {"L1/claudine.xml": junit_document([junit_case("green_one")])},
            [self.manifest_entry("L1/claudine.xml")],
        )
        plan = plan_document()
        if schema.RESOLVED_PLAN_FIELDS.get("skip_policy"):
            plan["skip_policy"] = skip_policy_with(expiry="2027-06-30")
        code, stderr, out = self.validate(artifacts=missing, plan=plan)
        self.assertNotEqual(
            0, code, "a missing expected test fails however it is excused"
        )
        self.assertIn("claudine::green_two", stderr)
        self.assertFalse(out.exists())

    def test_an_unexpected_identity_fails_validation(self):
        extra = self.staging_tree(
            {
                "L1/claudine.xml": junit_document(
                    [
                        junit_case("green_one"),
                        junit_case("green_two"),
                        junit_case("stowaway"),
                    ]
                )
            },
            [self.manifest_entry("L1/claudine.xml")],
        )
        code, stderr, out = self.validate(artifacts=extra)
        self.assertNotEqual(0, code, "an identity nothing expected is a defect")
        self.assertIn("claudine::stowaway", stderr)
        self.assertFalse(out.exists())

    def test_an_extra_filter_that_narrows_the_selection_is_refused(self):
        # Both sides agree — one test expected, one test reported — and both
        # were narrowed by an ad hoc filter. The only thing that catches it is
        # the manifest's recorded selection versus the plan's cell contract:
        # the plan says profile `ci` with no narrowing, the manifest admits a
        # different one.
        narrowed = expected_manifest(
            packages={
                "claudine": {
                    "tests": ["claudine::green_one"],
                    "ignored": [],
                    "excluded": [],
                }
            },
            selection={
                "filter": "package(claudine) & test(green_one)",
                "profile": "ci",
                "test_args": "",
            },
        )
        tree = self.staging_tree(
            {"L1/claudine.xml": junit_document([junit_case("green_one")])},
            [self.manifest_entry("L1/claudine.xml")],
        )
        code, stderr, out = self.validate(manifest=narrowed, artifacts=tree)
        self.assertNotEqual(
            0,
            code,
            "expected and observed shrinking together must not pass: the "
            "manifest's recorded selection is what makes the shrink visible",
        )
        self.assertIn("selection", stderr)
        self.assertFalse(out.exists())

    def test_duplicate_identities_fail_while_distinct_binaries_stay_distinct(self):
        # Two binaries may share a test NAME; their identities differ by
        # binary. The same identity twice is a duplicate report.
        shared_names = self.staging_tree(
            {
                "L1/claudine.xml": junit_document(
                    [junit_case("green_one"), junit_case("green_two")]
                ),
                "L1/retry.xml": junit_document(
                    [junit_case("green_one"), junit_case("green_two")]
                ),
            },
            [
                self.manifest_entry("L1/claudine.xml"),
                self.manifest_entry("L1/retry.xml"),
            ],
        )
        code, stderr, out = self.validate(artifacts=shared_names)
        self.assertNotEqual(0, code, "the same identity reported twice fails")
        self.assertIn("duplicate", stderr.lower())
        self.assertFalse(out.exists())

        distinct = self.staging_tree(
            {
                "L1/claudine.xml": junit_document(
                    [junit_case("green_one"), junit_case("green_two")], suite="claudine"
                ),
                "L1/helper.xml": junit_document(
                    [junit_case("green_one")], suite="claudine-helper"
                ),
            },
            [
                self.manifest_entry("L1/claudine.xml"),
                self.manifest_entry("L1/helper.xml"),
            ],
        )
        manifest = expected_manifest(
            packages={
                "claudine": {
                    "tests": [
                        "claudine::green_one",
                        "claudine::green_two",
                        "claudine-helper::green_one",
                    ],
                    "ignored": [],
                    "excluded": [],
                }
            }
        )
        code, stderr, out = self.validate(manifest=manifest, artifacts=distinct)
        self.assertEqual(
            0, code, f"distinct binaries keep distinct identities: {stderr}"
        )
        self.assertTrue(out.is_file())

    def test_retries_normalize_to_one_final_outcome_without_hiding_a_failure(self):
        failed_then_passed = self.staging_tree(
            {
                "L1/attempt-1.xml": junit_document(
                    [junit_case("green_one"), junit_case("green_two", "fail")]
                ),
                "L1/attempt-2.xml": junit_document(
                    [junit_case("green_one"), junit_case("green_two")]
                ),
            },
            [
                self.manifest_entry("L1/attempt-1.xml"),
                self.manifest_entry("L1/attempt-2.xml"),
            ],
        )
        code, stderr, out = self.validate(artifacts=failed_then_passed)
        self.assertEqual(
            0, code, f"the final attempt decides, and it passed: {stderr}"
        )
        self.assertTrue(out.is_file())

        passed_then_failed = self.staging_tree(
            {
                "L1/attempt-1.xml": junit_document(
                    [junit_case("green_one"), junit_case("green_two")]
                ),
                "L1/attempt-2.xml": junit_document(
                    [junit_case("green_one"), junit_case("green_two", "fail")]
                ),
            },
            [
                self.manifest_entry("L1/attempt-1.xml"),
                self.manifest_entry("L1/attempt-2.xml"),
            ],
        )
        code, stderr, out = self.validate(artifacts=passed_then_failed)
        self.assertNotEqual(
            0, code, "an earlier pass cannot hide the final attempt's failure"
        )
        self.assertIn("green_two", stderr)
        self.assertFalse(out.exists())

    # -- ignores, skips, and exclusions --------------------------------------

    def test_an_explicitly_ignored_test_is_expected_but_not_required(self):
        ignored = expected_manifest(
            packages={
                "claudine": {
                    "tests": ["claudine::green_one", "claudine::green_two"],
                    "ignored": ["claudine::slow_boot"],
                    "excluded": [],
                }
            }
        )
        # The report covers exactly `tests`; `slow_boot` never ran and no
        # approval exists for it — the manifest's own `ignored` set is the
        # plan-recorded expectation.
        tree = self.green_tree()
        code, stderr, out = self.validate(manifest=ignored, artifacts=tree)
        self.assertEqual(
            0, code, f"an ignored test is recorded, not missing: {stderr}"
        )
        self.assertTrue(out.is_file())

    def test_an_observed_skip_with_an_unexpired_approval_completes(self):
        skipped = self.staging_tree(
            {
                "L1/claudine.xml": junit_document(
                    [
                        junit_case("green_one"),
                        junit_case("green_two"),
                        junit_case("flaky_boot", "skip"),
                    ]
                )
            },
            [self.manifest_entry("L1/claudine.xml")],
        )
        manifest = expected_manifest(
            packages={
                "claudine": {
                    "tests": [
                        "claudine::green_one",
                        "claudine::green_two",
                        "claudine::flaky_boot",
                    ],
                    "ignored": [],
                    "excluded": [],
                }
            }
        )
        plan = plan_document()
        if schema.RESOLVED_PLAN_FIELDS.get("skip_policy"):
            plan["skip_policy"] = skip_policy_with(expiry="2027-06-30")
        code, stderr, out = self.validate(
            plan=plan, manifest=manifest, artifacts=skipped
        )
        self.assertEqual(
            0, code, f"an observed `<skipped/>` with a live approval passes: {stderr}"
        )
        self.assertTrue(out.is_file())

    def test_an_observed_skip_with_an_expired_approval_fails(self):
        skipped = self.staging_tree(
            {
                "L1/claudine.xml": junit_document(
                    [
                        junit_case("green_one"),
                        junit_case("green_two"),
                        junit_case("flaky_boot", "skip"),
                    ]
                )
            },
            [self.manifest_entry("L1/claudine.xml")],
        )
        manifest = expected_manifest(
            packages={
                "claudine": {
                    "tests": [
                        "claudine::green_one",
                        "claudine::green_two",
                        "claudine::flaky_boot",
                    ],
                    "ignored": [],
                    "excluded": [],
                }
            }
        )
        plan = plan_document()
        if schema.RESOLVED_PLAN_FIELDS.get("skip_policy"):
            plan["skip_policy"] = skip_policy_with(expiry="2026-01-01")
        code, stderr, out = self.validate(
            plan=plan, manifest=manifest, artifacts=skipped
        )
        self.assertNotEqual(0, code, "an expired approval excuses nothing")
        self.assertIn("flaky_boot", stderr)
        self.assertFalse(out.exists())

    def test_a_canonical_tier_exclusion_is_not_a_missing_test(self):
        excluded = expected_manifest(
            packages={
                "claudine": {
                    "tests": ["claudine::green_one", "claudine::green_two"],
                    "ignored": [],
                    "excluded": ["claudine::windows_only"],
                }
            }
        )
        code, stderr, out = self.validate(manifest=excluded)
        self.assertEqual(
            0,
            code,
            f"a test compiled out by cfg on this target is absent from the "
            f"expected set, not a skip and not missing: {stderr}",
        )
        self.assertTrue(out.is_file())

        # Non-vacuity: without the recorded exclusion the same absence fails,
        # so the pass above is the exclusion doing the work.
        unexplained = expected_manifest(
            packages={
                "claudine": {
                    "tests": [
                        "claudine::green_one",
                        "claudine::green_two",
                        "claudine::windows_only",
                    ],
                    "ignored": [],
                    "excluded": [],
                }
            }
        )
        code, stderr, out = self.validate(manifest=unexplained)
        self.assertNotEqual(0, code)
        self.assertIn("claudine::windows_only", stderr)

    # -- empty expected sets and companion-only cells ------------------------

    def test_an_empty_expected_set_without_a_plan_recorded_reason_fails(self):
        empty = expected_manifest(
            packages={
                "claudine": {"tests": [], "ignored": [], "excluded": []}
            }
        )
        tree = self.staging_tree(
            {"L1/claudine.xml": junit_document([])},
            [self.manifest_entry("L1/claudine.xml")],
        )
        code, stderr, out = self.validate(manifest=empty, artifacts=tree)
        self.assertNotEqual(
            0,
            code,
            "an empty expected set with nothing else declared is a drifted "
            "filter wearing a green report",
        )
        self.assertTrue(
            any(word in stderr.lower() for word in ("empty", "expected")),
            stderr,
        )
        self.assertFalse(out.exists())

    def test_a_companion_only_cell_completes_when_its_declared_suite_does(self):
        empty = expected_manifest(
            packages={
                "claudine": {"tests": [], "ignored": [], "excluded": []}
            }
        )
        tree = self.staging_tree(
            {"L1/claudine.xml": junit_document([])},
            [self.manifest_entry("L1/claudine.xml")],
        )
        plan = plan_document(
            cells=[
                cell_record(
                    companions=[
                        {
                            "name": "test_schema.py",
                            "recipe": "python3 scripts/ci/suite_runner.py test_schema.py",
                            "environment": "ubuntu-latest",
                            "counts": "json",
                        }
                    ]
                )
            ]
        )
        code, stderr, out = self.validate(
            plan=plan,
            manifest=empty,
            artifacts=tree,
            companions=companions_document({"test_schema.py": "success"}),
        )
        self.assertEqual(
            0,
            code,
            f"an empty suite is valid when the plan records the reason — the "
            f"companions this cell owes: {stderr}",
        )
        self.assertTrue(out.is_file())

    def test_a_companion_only_cell_fails_when_its_declared_suite_did_not_complete(
        self,
    ):
        empty = expected_manifest(
            packages={
                "claudine": {"tests": [], "ignored": [], "excluded": []}
            }
        )
        tree = self.staging_tree(
            {"L1/claudine.xml": junit_document([])},
            [self.manifest_entry("L1/claudine.xml")],
        )
        plan = plan_document(
            cells=[
                cell_record(
                    companions=[
                        {
                            "name": "test_schema.py",
                            "recipe": "python3 scripts/ci/suite_runner.py test_schema.py",
                            "environment": "ubuntu-latest",
                            "counts": "json",
                        }
                    ]
                )
            ]
        )
        # The companions document exists and reports nothing: the suite never
        # ran to completion on this leg.
        code, stderr, out = self.validate(
            plan=plan,
            manifest=empty,
            artifacts=tree,
            companions=companions_document({}),
        )
        self.assertNotEqual(0, code, "a declared suite that did not complete fails")
        self.assertIn("test_schema.py", stderr)
        self.assertFalse(out.exists())

    def test_a_failed_companion_suite_fails_the_cell(self):
        manifest = expected_manifest(
            packages={
                "claudine": {
                    "tests": ["claudine::green_one"],
                    "ignored": [],
                    "excluded": [],
                }
            }
        )
        tree = self.staging_tree(
            {"L1/claudine.xml": junit_document([junit_case("green_one")])},
            [self.manifest_entry("L1/claudine.xml")],
        )
        plan = plan_document(
            cells=[
                cell_record(
                    companions=[
                        {
                            "name": "test_schema.py",
                            "recipe": "python3 scripts/ci/suite_runner.py test_schema.py",
                            "environment": "ubuntu-latest",
                            "counts": "json",
                        }
                    ]
                )
            ]
        )
        code, stderr, out = self.validate(
            plan=plan,
            manifest=manifest,
            artifacts=tree,
            companions=companions_document({"test_schema.py": "failure"}),
        )
        self.assertNotEqual(0, code, "a failed companion fails its cell")
        self.assertIn("test_schema.py", stderr)
        self.assertFalse(out.exists())

    # -- backend proofs -------------------------------------------------------

    def test_a_required_backend_proof_absent_fails_and_present_passes(self):
        manifest = expected_manifest(
            tier="L2",
            packages={
                "claudine": {
                    "tests": ["claudine::terminal_roundtrip"],
                    "ignored": [],
                    "excluded": [],
                }
            },
        )
        tree = self.staging_tree(
            {
                "L2/claudine.xml": junit_document(
                    [junit_case("terminal_roundtrip")]
                )
            },
            [self.manifest_entry("L2/claudine.xml", tier="L2")],
            directory="junit-claudine-L2-ubuntu-latest",
        )
        plan = plan_document(
            packages=[package_record(tiers=["L1", "L2"], l2_backends=["tmux"])],
            cells=[cell_record(gate="L2")],
        )

        code, stderr, out = self.validate(
            plan=plan,
            manifest=manifest,
            artifacts=tree,
            cell="claudine/ubuntu-latest/L2",
            backend_proofs=backend_proofs_document([]),
        )
        self.assertNotEqual(0, code, "a declared backend with no proof fails")
        self.assertIn("tmux", stderr)
        self.assertFalse(out.exists())

        code, stderr, out = self.validate(
            plan=plan,
            manifest=manifest,
            artifacts=tree,
            cell="claudine/ubuntu-latest/L2",
            backend_proofs=backend_proofs_document(["tmux"]),
        )
        self.assertEqual(0, code, f"a proven backend satisfies the proof: {stderr}")
        self.assertTrue(out.is_file())


class ExpectedManifestV2OracleTests(ValidatorHarness):
    """The version-2 manifest's own contract (Phase 4's `_expected_manifest`).

    The happy-path shapes above already exercise `ignored`, `excluded`, and
    the provenance fields in passing; these pin the refusals.
    """

    def validate_manifest(self, manifest: dict) -> tuple[int, str]:
        return self.validate(manifest=manifest)[:2]

    def test_a_version_1_manifest_is_refused(self):
        legacy = expected_manifest()
        legacy["schema_version"] = 1
        legacy["packages"] = {
            "claudine": list(legacy["packages"]["claudine"]["tests"])
        }
        code, stderr = self.validate_manifest(legacy)
        self.assertNotEqual(0, code, "a v1 manifest has no ignored/excluded sets")
        self.assertIn("schema", stderr.lower())

    def test_the_resolved_nextest_version_and_provenance_are_required(self):
        # Not `subTest`: its failures are recorded rather than raised, which the
        # pending wrapper could not see when this fixture was frozen.
        problems = []
        for missing in (
            "nextest_version",
            "environment",
            "tier",
            "target",
            "from_archive",
            "selection",
        ):
            manifest = expected_manifest()
            del manifest[missing]
            code, stderr = self.validate_manifest(manifest)
            if code == 0 or missing not in stderr:
                problems.append(
                    f"a manifest without {missing} named nothing about where "
                    f"its expectation came from (exit {code}: {stderr.strip()})"
                )
        if problems:
            raise AssertionError("\n".join(problems))

    def test_a_manifest_generated_on_another_target_is_refused(self):
        elsewhere = expected_manifest(target="aarch64-apple-darwin")
        code, stderr = self.validate_manifest(elsewhere)
        self.assertNotEqual(
            0,
            code,
            "a listing from another target proves nothing about this one; "
            "cfg-compiled tests differ",
        )
        self.assertIn("target", stderr)


class ManifestCrossCheckTests(ValidatorHarness):
    """The manifest's optional declarations, checked against the plan.

    The plan stays the source of truth; the manifest's copy proves the job was
    provisioned for the work the plan scheduled rather than for something else.
    """

    def l2_plan(self, backends: list[str]) -> dict:
        return plan_document(
            packages=[package_record(tiers=["L1", "L2"], l2_backends=backends)],
            cells=[cell_record(gate="L2")],
        )

    def l2_tree(self) -> Path:
        return self.staging_tree(
            {"L2/claudine.xml": junit_document([junit_case("terminal_roundtrip")])},
            [self.manifest_entry("L2/claudine.xml", tier="L2")],
            directory="junit-claudine-L2-ubuntu-latest",
        )

    def l2_manifest(self, **overrides: object) -> dict:
        return expected_manifest(
            tier="L2",
            packages={
                "claudine": {
                    "tests": ["claudine::terminal_roundtrip"],
                    "ignored": [],
                    "excluded": [],
                }
            },
            **overrides,
        )

    def test_declared_backends_matching_the_plan_complete(self):
        code, stderr, out = self.validate(
            plan=self.l2_plan(["tmux"]),
            manifest=self.l2_manifest(backends=["tmux"]),
            artifacts=self.l2_tree(),
            cell="claudine/ubuntu-latest/L2",
            backend_proofs=backend_proofs_document(["tmux"]),
        )
        self.assertEqual(0, code, f"agreement is the ordinary case: {stderr}")
        self.assertEqual(["tmux"], self.record(out)["backends"])

    def test_a_producer_provisioned_for_other_backends_is_refused(self):
        code, stderr, out = self.validate(
            plan=self.l2_plan(["tmux"]),
            manifest=self.l2_manifest(backends=["wezterm"]),
            artifacts=self.l2_tree(),
            cell="claudine/ubuntu-latest/L2",
            backend_proofs=backend_proofs_document(["tmux", "wezterm"]),
        )
        self.assertNotEqual(
            0,
            code,
            "a job that required wezterm while the plan declares tmux proved "
            "something other than the scheduled cell, even with both proven",
        )
        self.assertIn("wezterm", stderr)
        self.assertIn("tmux", stderr)
        self.assertFalse(out.exists())

    def test_a_producer_that_ran_other_companions_is_refused(self):
        plan = plan_document(
            cells=[
                cell_record(
                    companions=[
                        {
                            "name": "test_schema.py",
                            "recipe": "python3 scripts/ci/suite_runner.py test_schema.py",
                            "environment": "ubuntu-latest",
                            "counts": "json",
                        }
                    ]
                )
            ]
        )
        code, stderr, out = self.validate(
            plan=plan,
            manifest=expected_manifest(companion_suites=["test_ci_local.py"]),
            companions=companions_document(
                {"test_schema.py": "success", "test_ci_local.py": "success"}
            ),
        )
        self.assertNotEqual(0, code, "the suites run must be the suites attached")
        self.assertIn("test_ci_local.py", stderr)
        self.assertFalse(out.exists())

    def test_run_ignored_is_refused_because_it_moves_the_expected_set(self):
        # With `--run-ignored`, nextest reports ignored tests as `matches`, so
        # the manifest's three identity sets stop meaning what they say.
        code, stderr, out = self.validate(
            manifest=expected_manifest(
                selection={
                    "filter": "package(claudine)",
                    "profile": "ci",
                    "test_args": "--run-ignored all",
                }
            )
        )
        self.assertNotEqual(0, code)
        self.assertIn("--run-ignored", stderr)
        self.assertFalse(out.exists())

    def test_a_profile_other_than_the_cells_is_refused(self):
        code, stderr, out = self.validate(
            manifest=expected_manifest(
                selection={
                    "filter": "package(claudine)",
                    "profile": "default",
                    "test_args": "",
                }
            )
        )
        self.assertNotEqual(
            0, code, "a listing under another profile selected other tests"
        )
        self.assertIn("profile", stderr)
        self.assertFalse(out.exists())


class CellBindingTests(ValidatorHarness):
    """Which cell a producer may certify, and what it may not certify at all."""

    def test_a_cell_the_plan_does_not_carry_is_refused(self):
        code, stderr, out = self.validate(cell="claudine/windows-latest/L1")
        self.assertNotEqual(0, code)
        self.assertIn("claudine/windows-latest/L1", stderr)
        self.assertFalse(out.exists())

    def test_a_reused_cell_has_no_work_to_certify(self):
        reused = plan_document(
            cells=[cell_record(execution="reuse", state="reused", origin="local")]
        )
        code, stderr, out = self.validate(plan=reused)
        self.assertNotEqual(
            0, code, "evidence already satisfied this cell; nothing ran here"
        )
        self.assertIn("completion-cell-not-executing", stderr)
        self.assertFalse(out.exists())

    def test_a_plan_carrying_the_same_cell_twice_is_refused(self):
        doubled = plan_document(cells=[cell_record(), cell_record()])
        code, stderr, out = self.validate(plan=doubled)
        self.assertNotEqual(0, code, "a cell key names one unit of work")
        self.assertIn("completion-cell-unknown", stderr)
        self.assertFalse(out.exists())

    def test_an_unreadable_plan_is_refused_by_name(self):
        broken = self.root / "broken-plan.json"
        broken.write_text("{not json", encoding="utf-8")
        self.require_validator()
        completed = subprocess.run(
            [
                sys.executable,
                str(COMPLETION),
                "--plan",
                str(broken),
                "--cell",
                "claudine/ubuntu-latest/L1",
                "--artifacts",
                str(self.root),
                "--out",
                str(self.root / "completion.json"),
            ],
            capture_output=True,
            text=True,
            timeout=120,
            cwd=ROOT,
        )
        self.assertEqual(
            2,
            completed.returncode,
            "an unreadable input is an infrastructure failure, not a coverage "
            "verdict; exit 1 is reserved for a cell that did not prove itself",
        )
        self.assertIn("completion-plan-unreadable", completed.stderr)

    def test_a_cell_that_did_not_prove_itself_exits_one(self):
        # The other half of the split above, so neither code can drift into
        # meaning the other.
        missing = self.staging_tree(
            {"L1/claudine.xml": junit_document([junit_case("green_one")])},
            [self.manifest_entry("L1/claudine.xml")],
        )
        code, stderr, out = self.validate(artifacts=missing)
        self.assertEqual(1, code, stderr)
        self.assertFalse(out.exists())

    def test_a_test_gate_without_a_manifest_cannot_be_certified(self):
        code, stderr, out = self.validate(include_manifest=False)
        self.assertNotEqual(
            0,
            code,
            "a green report alone is the exact claim this tool exists to refuse",
        )
        self.assertIn("completion-manifest-missing", stderr)
        self.assertFalse(out.exists())

    def test_a_check_gate_certifies_without_a_listing(self):
        # `check` and `lint` compile; they run no tests, so there is no listing
        # to compare and no nextest version to record.
        plan = plan_document(cells=[cell_record(gate="check", reusable=False)])
        code, stderr, out = self.validate(
            plan=plan, cell="claudine/ubuntu-latest/check", include_manifest=False
        )
        self.assertEqual(0, code, f"a check cell has nothing to list: {stderr}")
        record = self.record(out)
        self.assertEqual("check", record["gate"])
        self.assertEqual([], record["reports"])
        self.assertNotIn("nextest_version", record)


class CompletionRecordContractTests(ValidatorHarness):
    """The record itself: its field contract, its bindings, and its persistence."""

    def test_the_record_matches_the_frozen_field_contract(self):
        code, stderr, out = self.validate()
        self.assertEqual(0, code, stderr)
        record = self.record(out)
        missing = [
            name
            for name, required in schema.COMPLETION_RECORD_FIELDS.items()
            if required and name not in record
        ]
        self.assertEqual([], missing, "the record omits a required field")
        unknown = [
            name for name in record if name not in schema.COMPLETION_RECORD_FIELDS
        ]
        self.assertEqual([], unknown, "the record carries a field nothing describes")
        self.assertEqual([], schema.validate_completion_record(record))

    def test_the_shipped_contract_json_describes_the_record(self):
        # A passive corpus check over the shipped artifact: the Rust audit reads
        # this file, and serde ignores what it does not recognize, so a rename
        # would otherwise land silently.
        contract = json.loads(
            (ROOT / ".github" / "ci" / "schemas" / "contract.json").read_text(
                encoding="utf-8"
            )
        )
        self.assertEqual(
            schema.COMPLETION_RECORD_SCHEMA_VERSION,
            contract["completion_record"]["schema_version"],
        )
        self.assertEqual(
            schema.COMPLETION_RECORD_FIELDS, contract["completion_record"]["document"]
        )
        self.assertEqual(
            list(schema.COMPLETION_REJECTIONS),
            contract["completion_record"]["rejections"],
        )
        self.assertEqual(
            schema.EXPECTED_MANIFEST_SCHEMA_VERSION,
            contract["expected_manifest"]["schema_version"],
        )
        self.assertEqual(
            schema.EXPECTED_MANIFEST_FIELDS, contract["expected_manifest"]["document"]
        )

    def test_the_record_binds_the_build_and_the_gate_inputs_it_consumed(self):
        plan = plan_document(
            packages=[package_record(input_paths=["claudine/lib", "claudine/cli"])],
            cells=[cell_record(build="0123456789abcdef")],
        )
        code, stderr, out = self.validate(plan=plan)
        self.assertEqual(0, code, stderr)
        record = self.record(out)
        self.assertEqual("0123456789abcdef", record["build"])
        self.assertEqual(["claudine/cli", "claudine/lib"], record["gate_inputs"])

    def test_the_record_round_trips_through_the_file_it_is_published_as(self):
        code, stderr, out = self.validate()
        self.assertEqual(0, code, stderr)
        first = out.read_text(encoding="utf-8")
        reread = json.loads(first)
        self.assertEqual([], schema.validate_completion_record(reread))

        # Re-validating the same inputs must produce the same bytes: the record
        # is an artifact two jobs compare, not a per-run rendering.
        code, stderr, again = self.validate()
        self.assertEqual(0, code, stderr)
        self.assertEqual(first, again.read_text(encoding="utf-8"))
        self.assertEqual(reread, json.loads(again.read_text(encoding="utf-8")))

    def test_a_refusal_removes_a_record_an_earlier_validation_left(self):
        code, _, out = self.validate()
        self.assertEqual(0, code)
        self.assertTrue(out.is_file())

        stowaway = self.staging_tree(
            {
                "L1/claudine.xml": junit_document(
                    [
                        junit_case("green_one"),
                        junit_case("green_two"),
                        junit_case("stowaway"),
                    ]
                )
            },
            [self.manifest_entry("L1/claudine.xml")],
        )
        code, _, out = self.validate(artifacts=stowaway)
        self.assertNotEqual(0, code)
        self.assertFalse(
            out.exists(),
            "the upload step names the path, not the outcome; a stale record "
            "would be published as this run's proof",
        )


@tool_guard.requires_tools(
    "just",
    enforced_by="the repo-deps L1 cell, which provisions `just` on every environment",
    detail="The canonical tier expressions come from `just _tier_filter`.",
)
class ShippedSelectionTests(unittest.TestCase):
    """The shipped tier expressions, against the canonical-selection rule.

    A passive corpus check in the strict sense: the validator refuses a filter
    built from anything but the tier markers, so every expression the
    repository actually runs with has to pass it. If a tier gains a predicate,
    this fails here rather than turning every producer red.
    """

    def tier_filter(self, tier: str, package: str = "") -> str:
        arguments = ["just", "_tier_filter", tier]
        if package:
            arguments.append(package)
        completed = subprocess.run(
            arguments, capture_output=True, text=True, timeout=120, cwd=ROOT, check=True
        )
        return completed.stdout.strip()

    def assert_canonical(self, expression: str, where: str) -> None:
        problems = completion.selection_problems(
            {"filter": expression, "profile": "ci", "test_args": ""},
            {"environment": "ubuntu-latest", "gate": "L1", "profile": "ci"},
        )
        self.assertEqual(
            [], problems, f"the shipped {where} expression is not canonical: {expression}"
        )

    def test_every_shipped_tier_expression_is_canonical(self):
        for tier in ("L1", "L2", "L3", "browser", "real", "sanity"):
            self.assert_canonical(self.tier_filter(tier), tier)

    def test_the_per_package_and_slow_variants_are_canonical(self):
        # The two documented refinements of L1: `worktree-cli` drops its `perf_`
        # SLA tests, and `l1-include-slow` keeps `slow_` in.
        self.assert_canonical(self.tier_filter("L1", "worktree-cli"), "worktree-cli L1")
        completed = subprocess.run(
            ["just", "_tier_filter", "L1"],
            capture_output=True,
            text=True,
            timeout=120,
            cwd=ROOT,
            check=True,
            env={**os.environ, "BISCUIT_L1_INCLUDE_SLOW": "1"},
        )
        self.assert_canonical(completed.stdout.strip(), "l1-include-slow")

    def test_archive_mode_package_scoping_stays_canonical(self):
        # `_expected_manifest` wraps the tier expression when listing from an
        # archive, because `-p` is unavailable there.
        expression = self.tier_filter("L1")
        self.assert_canonical(f"package(claudine) & ({expression})", "archive-scoped L1")

    def test_an_ad_hoc_narrowing_of_a_shipped_expression_is_caught(self):
        # Non-vacuity: the rule above accepts every shipped expression, so this
        # proves it still rejects the thing it exists to reject.
        expression = self.tier_filter("L1")
        problems = completion.selection_problems(
            {
                "filter": f"({expression}) & test(one_named_test)",
                "profile": "ci",
                "test_args": "",
            },
            {"environment": "ubuntu-latest", "gate": "L1", "profile": "ci"},
        )
        self.assertTrue(problems, "an ad hoc test predicate must still be refused")


SCRATCH_TESTS = """\
#[test]
fn plain() {}

#[test]
#[ignore = "needs a fixture"]
fn ignored_by_attribute() {}

#[test]
fn level2_marked() {}
"""


@tool_guard.requires_tools(
    "just",
    "cargo",
    enforced_by="the repo-deps L1 cell, which provisions the toolchain and `just`",
    detail="The end-to-end path is `just _expected_manifest` over a real crate.",
)
class ShippedRecipeEndToEndTests(unittest.TestCase):
    """`just _expected_manifest` → `completion.py`, with nothing hand-written.

    The scratch crate is deliberately tiny and dependency-free: the contract
    under test is the recipe's listing and the validator's comparison, not
    anyone's build. It carries one plain test, one `#[ignore]`d test, and one
    tier-marked test, so all three identity sets are non-empty and the two
    absences that are NOT failures are both exercised on the real path.
    """

    def setUp(self) -> None:
        # `mkdtemp` + `ignore_errors`, not `TemporaryDirectory`: the scratch
        # crate compiles a `target/` tree inside this directory, and Windows
        # refuses to remove a file a just-exited process still holds open.
        # `ignore_cleanup_errors` would say the same thing but needs 3.10, and
        # the repository's Python floor is 3.9.
        self.root = Path(tempfile.mkdtemp(prefix="completion-e2e-"))
        self.addCleanup(shutil.rmtree, self.root, True)
        (self.root / "src").mkdir(parents=True)
        (self.root / "tests").mkdir(parents=True)
        (self.root / "Cargo.toml").write_text(
            '[package]\nname = "completion-probe"\nversion = "0.0.0"\n'
            'edition = "2021"\npublish = false\n\n[workspace]\n',
            encoding="utf-8",
        )
        (self.root / "src" / "lib.rs").write_text("", encoding="utf-8")
        (self.root / "tests" / "probe.rs").write_text(SCRATCH_TESTS, encoding="utf-8")
        self.stage = self.root / "stage"

    def expected_manifest_document(self) -> dict:
        completed = subprocess.run(
            [
                "just",
                "_expected_manifest",
                "L1",
                "completion-probe",
                "--manifest-path",
                str(self.root / "Cargo.toml"),
            ],
            capture_output=True,
            text=True,
            timeout=900,
            cwd=ROOT,
            env={
                **os.environ,
                "BISCUIT_CI_ENVIRONMENT": "ubuntu-latest",
                "BISCUIT_JUNIT_WORKSPACE_ROOT": str(self.root),
                "BISCUIT_JUNIT_STAGE_DIR": str(self.stage),
                # Explicit, not inherited: this suite runs as a companion of the
                # repo-deps L1 cell, where `NEXTEST_PROFILE=ci` is exported — and
                # the `ci` profile lives in the repository's `.config/nextest.toml`,
                # which a scratch workspace does not have.
                "NEXTEST_PROFILE": "default",
            },
        )
        if completed.returncode != 0:
            raise AssertionError(
                "the shipped `_expected_manifest` recipe failed on a scratch "
                f"crate:\n{completed.stdout}\n{completed.stderr}"
            )
        return json.loads(
            (self.stage / "expected-L1.json").read_text(encoding="utf-8")
        )

    def test_the_shipped_recipe_emits_a_manifest_the_validator_accepts(self):
        document = self.expected_manifest_document()
        self.assertEqual(2, document["schema_version"])
        listing = document["packages"]["completion-probe"]
        self.assertEqual(["completion-probe::probe::plain"], listing["tests"])
        self.assertEqual(
            ["completion-probe::probe::ignored_by_attribute"], listing["ignored"]
        )
        self.assertEqual(
            ["completion-probe::probe::level2_marked"], listing["excluded"]
        )
        self.assertTrue(document["nextest_version"].startswith("cargo-nextest"))
        self.assertFalse(document["from_archive"])
        self.assertEqual("default", document["selection"]["profile"])

        # The whole path: the recipe's own manifest, a report of exactly what
        # it expected, and the real CLI.
        report = self.stage / "L1" / "completion-probe.xml"
        report.parent.mkdir(parents=True, exist_ok=True)
        report.write_text(
            junit_document(
                [junit_case("plain")], suite="completion-probe::probe"
            ),
            encoding="utf-8",
        )
        (self.stage / "manifest.jsonl").write_text(
            json.dumps(
                {
                    "tier": "L1",
                    "package": "completion-probe",
                    "xml": "L1/completion-probe.xml",
                    "exit_code": 0,
                    "environment": "ubuntu-latest",
                    "duration_s": 1,
                    "report_present": True,
                }
            )
            + "\n",
            encoding="utf-8",
        )
        plan = plan_document(
            packages=[package_record(package="completion-probe", area="probe")],
            areas=[
                {
                    "area": "probe",
                    "selection_reason": "fixture",
                    "packages": ["completion-probe"],
                    "execution_path": "rows",
                }
            ],
            cells=[
                cell_record(
                    package="completion-probe", area="probe", profile="default"
                )
            ],
        )
        plan_path = self.root / "resolved-plan.json"
        plan_path.write_text(json.dumps(plan), encoding="utf-8")
        out = self.root / "completion.json"
        completed = subprocess.run(
            [
                sys.executable,
                str(COMPLETION),
                "--plan",
                str(plan_path),
                "--cell",
                "completion-probe/ubuntu-latest/L1",
                "--expected-manifest",
                str(self.stage / "expected-L1.json"),
                "--artifacts",
                str(self.stage),
                "--target",
                document["target"],
                "--run",
                "77",
                "--attempt",
                "1",
                "--out",
                str(out),
            ],
            capture_output=True,
            text=True,
            timeout=300,
            cwd=ROOT,
        )
        self.assertEqual(
            0,
            completed.returncode,
            f"the shipped recipe's own manifest must validate: {completed.stderr}",
        )
        record = json.loads(out.read_text(encoding="utf-8"))
        self.assertIs(True, record["complete"])
        self.assertEqual(document["nextest_version"], record["nextest_version"])
        self.assertEqual(["L1/completion-probe.xml"], record["reports"])

        # Non-vacuity: the ignored and tier-excluded identities are absent from
        # the report, and neither is a missing test. Report one and it is.
        (self.stage / "L1" / "completion-probe.xml").write_text(
            junit_document(
                [junit_case("plain"), junit_case("level2_marked")],
                suite="completion-probe::probe",
            ),
            encoding="utf-8",
        )
        completed = subprocess.run(
            [
                sys.executable,
                str(COMPLETION),
                "--plan",
                str(plan_path),
                "--cell",
                "completion-probe/ubuntu-latest/L1",
                "--expected-manifest",
                str(self.stage / "expected-L1.json"),
                "--artifacts",
                str(self.stage),
                "--target",
                document["target"],
                "--run",
                "77",
                "--attempt",
                "1",
                "--out",
                str(out),
            ],
            capture_output=True,
            text=True,
            timeout=300,
            cwd=ROOT,
        )
        self.assertNotEqual(0, completed.returncode)
        self.assertIn("level2_marked", completed.stderr)


class AuditEndToEndTests(unittest.TestCase):
    """plan → rows → producer artifacts → `ci-rollup rollup --area` → `verdict --area`.

    Nothing between the plan and the verdict is hand-written: the rows come from
    `affected_scope.row_sets`, each row's artifact names from the shipped
    `cell_contract.py`, each record from the shipped `completion.py`, and the
    judgement from the real `ci-rollup` binary through its normal CLI. One
    fixture per area shape the audit must judge (plan Phase 6, AC3, AC6): all
    reused, gap only, mixed, and executing cells whose producer call was
    skipped.
    """

    AREA = "claudine"
    RUN = "4242"
    #: An expiry no run of this suite reaches, so the gap stays accepted.
    FAR_EXPIRY = "2099-01-01"

    @classmethod
    def setUpClass(cls) -> None:
        # Built once, and located through cargo's own report rather than a
        # guessed `target/` path: `CARGO_TARGET_DIR` may move it anywhere.
        completed = subprocess.run(
            [
                "cargo", "build", "--quiet", "--no-default-features",
                "--bin", "ci-rollup",
                "--manifest-path", str(ROOT / "scripts" / "Cargo.toml"),
                "--message-format", "json",
            ],
            capture_output=True,
            text=True,
            timeout=900,
            cwd=ROOT,
        )
        if completed.returncode != 0:
            raise AssertionError(f"ci-rollup did not build:\n{completed.stderr}")
        executables = [
            message["executable"]
            for message in map(json.loads, completed.stdout.splitlines())
            if message.get("reason") == "compiler-artifact"
            and message.get("target", {}).get("name") == "ci-rollup"
            and message.get("executable")
        ]
        if not executables:
            raise AssertionError("cargo reported no ci-rollup executable")
        cls.rollup_tool = executables[-1]

    def setUp(self) -> None:
        self.root = Path(tempfile.mkdtemp(prefix="audit-e2e-"))
        self.addCleanup(shutil.rmtree, self.root, True)
        self.artifacts = self.root / "ci-artifacts"
        self.artifacts.mkdir()
        (self.root / "environments.json").write_text(
            json.dumps({"schema_version": 3, "environments": []}), encoding="utf-8"
        )
        (self.root / "ci-baseline.toml").write_text("schema_version = 3\n", encoding="utf-8")

    # -- the plan ----------------------------------------------------------

    def reused_cell(self) -> dict:
        return cell_record(
            environment="macos-latest",
            execution="reuse",
            origin="local",
            state="reused",
            evidence={
                "origin": "local",
                "outcome": "pass",
                "counts": {"total": 2, "passed": 2, "failed": 0, "skipped": 0, "errored": 0},
                "duration_s": 3.0,
                "evidence": {"ref": "refs/notes/ci-local/macos-latest", "commit": SHA_A},
            },
        )

    def gap_cell(self, expiry: str = FAR_EXPIRY) -> dict:
        return cell_record(
            environment="windows-latest",
            gate="L2",
            execution="omit",
            origin="none",
            state="accepted-gap",
            target_kinds=[],
            compile_coverage_from="",
            gap={
                "capability": "tmux",
                "governed": True,
                "policy": ".github/ci/environments.json",
                "owner": "@ken",
                "reason": "tmux has no Windows port",
                "expiry": expiry,
                "closes": "features/_unscheduled/windows-l2-ci-leg",
            },
        )

    def check_cell(self) -> dict:
        return cell_record(gate="check", target_kinds=["lib", "test"], compile_coverage_from="check")

    def write_plan(self, cells: list[dict]) -> Path:
        plan = plan_document(
            cells=cells,
            packages=[package_record(gates=["lint", "check", "L1", "L2"], tiers=["L1", "L2"])],
        )
        path = self.root / "resolved-plan.json"
        path.write_text(json.dumps(plan), encoding="utf-8")
        self.plan = plan
        self.plan_path = path
        return path

    # -- the producers -----------------------------------------------------

    def rows(self) -> list[dict]:
        document = affected_scope.row_sets(self.plan)[self.AREA]
        return [row for name in schema.ROW_SET_NAMES for row in document[name]]

    def contract(self, row: dict) -> dict[str, str]:
        output = self.root / f"contract-{row['gate']}-{row['environment']}"
        completed = subprocess.run(
            [
                sys.executable, str(ROOT / "scripts" / "ci" / "cell_contract.py"),
                "--plan", str(self.plan_path),
                "--row", json.dumps(row),
                "--head", self.plan["head"],
                "--github-output", str(output),
            ],
            capture_output=True, text=True, timeout=120,
        )
        self.assertEqual(0, completed.returncode, completed.stderr)
        return dict(
            line.split("=", 1)
            for line in output.read_text(encoding="utf-8").splitlines()
            if "=" in line
        )

    def produce(self, row: dict, failing: bool = False) -> int:
        """Run one row's producer half: stage its evidence, certify it with the
        shipped validator, and publish the status the job would. Returns the
        validator's exit code."""
        contract = self.contract(row)
        gate, environment = row["gate"], row["environment"]
        command = [
            sys.executable, str(COMPLETION),
            "--plan", str(self.plan_path),
            "--cell", contract["cell"],
            "--artifacts", str(self.artifacts),
            "--target", TARGET,
            "--run", self.RUN,
            "--attempt", "1",
            "--out", str(self.artifacts / contract["completion_artifact"] / "completion.json"),
        ]
        if gate in schema.BUILD_GATES:
            stage = self.artifacts / contract["junit_artifact"]
            (stage / gate).mkdir(parents=True)
            (stage / gate / "claudine.xml").write_text(
                junit_document(
                    [junit_case("green_one"), junit_case("green_two", "fail" if failing else "pass")]
                ),
                encoding="utf-8",
            )
            (stage / "manifest.jsonl").write_text(
                json.dumps({
                    "tier": gate, "package": "claudine", "xml": f"{gate}/claudine.xml",
                    "exit_code": 100 if failing else 0, "environment": environment,
                    "duration_s": 7, "report_present": True,
                }) + "\n",
                encoding="utf-8",
            )
            listing = self.root / f"expected-{gate}-{environment}.json"
            listing.write_text(
                json.dumps(expected_manifest(environment=environment, tier=gate)),
                encoding="utf-8",
            )
            command += ["--expected-manifest", str(listing)]
        status = self.artifacts / contract["status_artifact"]
        status.mkdir(parents=True)
        (status / "status.json").write_text(
            json.dumps({
                "package": "claudine", "job": gate, "environment": environment,
                "result": "failure" if failing else "success",
            }),
            encoding="utf-8",
        )
        return subprocess.run(command, capture_output=True, text=True, timeout=120).returncode

    # -- the audit ---------------------------------------------------------

    def audit(self, producers: str) -> tuple[int, int, str]:
        """`(rollup exit, verdict exit, verdict output)`, as the area job runs them."""
        results = self.root / "results.json"
        summary = self.root / "summary.md"
        rollup = subprocess.run(
            [
                self.rollup_tool, "rollup",
                "--artifacts", str(self.artifacts),
                "--plan", str(self.plan_path),
                "--environments", str(self.root / "environments.json"),
                "--area", self.AREA,
                "--out", str(results),
                "--summary", str(summary),
                "--run-id", self.RUN,
            ],
            capture_output=True, text=True, timeout=120,
        )
        self.assertIn(rollup.returncode, (0, 2), rollup.stderr)
        verdict = subprocess.run(
            [
                self.rollup_tool, "verdict",
                "--results", str(results),
                "--baseline", str(self.root / "ci-baseline.toml"),
                "--area", self.AREA,
                "--summary", str(summary),
                "--producers", producers,
            ],
            capture_output=True, text=True, timeout=120,
        )
        return rollup.returncode, verdict.returncode, verdict.stdout + verdict.stderr

    # -- the four shapes -----------------------------------------------------

    def test_an_all_reused_area_dispatches_nothing_and_is_still_judged(self) -> None:
        self.write_plan([self.reused_cell()])
        self.assertEqual([], self.rows(), "a reused cell creates no execution row")
        rollup, verdict, output = self.audit("skipped")
        self.assertEqual((0, 0), (rollup, verdict), output)

        # Judged, not waved through: a reused FAILURE blocks with no producer.
        cell = self.reused_cell()
        cell["evidence"]["outcome"] = "fail"
        self.write_plan([cell])
        _, verdict, output = self.audit("skipped")
        self.assertEqual(2, verdict, output)
        self.assertIn("cell-failed", output)

    def test_a_gap_only_area_dispatches_nothing_and_is_still_judged(self) -> None:
        self.write_plan([self.gap_cell()])
        self.assertEqual([], self.rows(), "an accepted gap creates no execution row")
        rollup, verdict, output = self.audit("skipped")
        self.assertEqual((0, 0), (rollup, verdict), output)
        self.assertIn("policy-gap-accepted", output)
        self.assertIn("ACCEPTED GAP", (self.root / "summary.md").read_text(encoding="utf-8"))

        self.write_plan([self.gap_cell(expiry="2020-01-01")])
        _, verdict, output = self.audit("skipped")
        self.assertEqual(2, verdict, "an expired gap blocks an area nothing executed in")
        self.assertIn("policy-gap-expired", output)

    def test_a_mixed_area_passes_only_with_every_executing_cell_certified(self) -> None:
        self.write_plan([cell_record(), self.check_cell(), self.reused_cell(), self.gap_cell()])
        rows = self.rows()
        self.assertEqual(
            {("L1", "ubuntu-latest"), ("check", "ubuntu-latest")},
            {(row["gate"], row["environment"]) for row in rows},
            "exactly the executing cells are dispatched",
        )
        for row in rows:
            self.assertEqual(0, self.produce(row), f"{row} certified itself")
        rollup, verdict, output = self.audit("success")
        self.assertEqual((0, 0), (rollup, verdict), output)
        summary = (self.root / "summary.md").read_text(encoding="utf-8")
        self.assertIn("Reused results", summary)
        self.assertIn("ACCEPTED GAP", summary)

        # A green check status whose record never arrived is unproven.
        shutil.rmtree(self.artifacts / "completion-claudine-check-ubuntu-latest")
        rollup, verdict, output = self.audit("success")
        self.assertEqual(0, rollup, "the cell's state is still what its status said")
        self.assertEqual(2, verdict, output)
        self.assertIn("completion-unproven", output)
        self.assertIn("claudine/ubuntu-latest/check", output)

    def test_a_failing_executing_cell_leaves_no_record_and_blocks_once(self) -> None:
        self.write_plan([cell_record(), self.reused_cell()])
        (row,) = self.rows()
        self.assertEqual(1, self.produce(row, failing=True), "a failed test is not certified")
        self.assertFalse((self.artifacts / "completion-claudine-L1-ubuntu-latest").exists())
        rollup, verdict, output = self.audit("success")
        self.assertEqual((2, 2), (rollup, verdict), output)
        self.assertIn("cell-failed", output)
        self.assertNotIn(
            "completion-unproven", output,
            "a failure is one problem: its absent record is not a second row",
        )

    def test_a_skipped_producer_over_executing_cells_is_missing_coverage(self) -> None:
        self.write_plan([cell_record(), self.check_cell(), self.reused_cell()])
        self.assertTrue(self.rows(), "the plan executes work in this area")
        _, verdict, output = self.audit("skipped")
        self.assertEqual(2, verdict, output)
        self.assertIn("producers-skipped", output)
        self.assertIn("claudine/ubuntu-latest/L1", output)


class FixtureSelfCheckTests(unittest.TestCase):
    """NOT pending: the fixtures' own builders, so they cannot drift silently."""

    def test_the_green_tree_reports_exactly_the_expected_identities(self):
        import re

        document = junit_document([junit_case("green_one"), junit_case("green_two")])
        names = re.findall(r'<testcase name="([^"]+)"', document)
        suite = re.search(r'<testsuite name="([^"]+)"', document).group(1)
        identities = [f"{suite}::{name}" for name in names]
        self.assertEqual(
            ["claudine::green_one", "claudine::green_two"],
            identities,
            "the JUnit builders and the manifest builder must agree on the "
            "identity convention (<testsuite name>::<testcase name>)",
        )

    def test_the_plan_fixture_adapts_to_the_current_schema(self):
        document = plan_document()
        self.assertEqual(
            schema.RESOLVED_PLAN_SCHEMA_VERSION, document["schema_version"]
        )
        if schema.RESOLVED_PLAN_FIELDS.get("skip_policy"):
            self.assertIn("skip_policy", document)


if __name__ == "__main__":
    unittest.main(verbosity=2)

#!/usr/bin/env python3
"""Producer-completeness contracts (AC5) for `scripts/ci/completion.py`.

Phase 2 of `features/2026-09-19-direct-cell-execution` lands every fixture
here as a pending oracle; Phase 4 implements the validator and promotes them.
Until then each body fails for its recorded reason — the validator does not
exist — and the suite stays green because the pending wrapper inverts the
assertion.

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
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import schema  # noqa: E402
from pending_contracts import pending  # noqa: E402

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
        completed = subprocess.run(
            [
                sys.executable,
                str(COMPLETION),
                "--plan",
                str(plan_path),
                "--cell",
                cell,
                "--expected-manifest",
                str(manifest_path),
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
    """The AC5 fixture matrix, pending until Phase 4 implements the tool."""

    # -- the happy path every failure case is measured against --------------

    @pending(
        "AC5",
        "scripts/ci/completion.py does not exist, so no producer can write a "
        "completion record",
        oracle=VALIDATOR_ORACLE,
    )
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

    @pending(
        "AC5",
        "scripts/ci/completion.py does not exist, so a missing report cannot "
        "be refused by the validator",
        oracle=VALIDATOR_ORACLE,
    )
    def test_a_missing_report_fails_validation(self):
        artifacts = self.staging_tree(
            {},
            [self.manifest_entry("L1/claudine.xml")],
        )
        code, stderr, out = self.validate(artifacts=artifacts)
        self.assertNotEqual(0, code, "a manifest entry with no XML behind it fails")
        self.assertIn("report", stderr)
        self.assertFalse(out.exists(), "a refused validation writes no record")

    @pending(
        "AC5",
        "scripts/ci/completion.py does not exist, so a malformed report "
        "cannot be refused by the validator",
        oracle=VALIDATOR_ORACLE,
    )
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

    @pending(
        "AC5",
        "scripts/ci/completion.py does not exist, so a missing expected test "
        "cannot fail the producer",
        oracle=VALIDATOR_ORACLE,
    )
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

    @pending(
        "AC5",
        "scripts/ci/completion.py does not exist, so an unexpected identity "
        "cannot be refused by the validator",
        oracle=VALIDATOR_ORACLE,
    )
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

    @pending(
        "AC5",
        "scripts/ci/completion.py does not exist, so a manifest whose "
        "recorded selection disagrees with the plan cannot be refused",
        oracle=VALIDATOR_ORACLE,
    )
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

    @pending(
        "AC5",
        "scripts/ci/completion.py does not exist, so duplicate identities "
        "cannot be refused by the validator",
        oracle=VALIDATOR_ORACLE,
    )
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

    @pending(
        "AC5",
        "scripts/ci/completion.py does not exist, so retries cannot be "
        "normalized into one final outcome",
        oracle=VALIDATOR_ORACLE,
    )
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

    @pending(
        "AC5",
        "scripts/ci/completion.py does not exist, so an `#[ignore]`d test's "
        "absence cannot be told apart from a lost test",
        oracle=VALIDATOR_ORACLE,
    )
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

    @pending(
        "AC5",
        "scripts/ci/completion.py does not exist, so an approved skip cannot "
        "be accepted by the validator",
        oracle=VALIDATOR_ORACLE,
    )
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

    @pending(
        "AC5",
        "scripts/ci/completion.py does not exist, so an expired approval "
        "cannot be refused by the validator",
        oracle=VALIDATOR_ORACLE,
    )
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

    @pending(
        "AC5",
        "scripts/ci/completion.py does not exist, so a cfg-excluded test's "
        "absence cannot be told apart from a lost test",
        oracle=VALIDATOR_ORACLE,
    )
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

    @pending(
        "AC5",
        "scripts/ci/completion.py does not exist, so an empty expected set "
        "cannot be refused for lack of a recorded reason",
        oracle=VALIDATOR_ORACLE,
    )
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

    @pending(
        "AC5",
        "scripts/ci/completion.py does not exist, so a companion-only cell "
        "cannot be validated",
        oracle=VALIDATOR_ORACLE,
    )
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

    @pending(
        "AC5",
        "scripts/ci/completion.py does not exist, so a suite that did not "
        "run cannot be refused by the validator",
        oracle=VALIDATOR_ORACLE,
    )
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

    @pending(
        "AC5",
        "scripts/ci/completion.py does not exist, so a failed companion "
        "cannot be refused by the validator",
        oracle=VALIDATOR_ORACLE,
    )
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

    @pending(
        "AC5",
        "scripts/ci/completion.py does not exist, so an absent backend proof "
        "cannot be refused by the validator",
        oracle=VALIDATOR_ORACLE,
    )
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

    @pending(
        "manifest-v2",
        "scripts/ci/completion.py does not exist, so a v1 manifest cannot be "
        "refused for its schema",
        oracle=VALIDATOR_ORACLE,
    )
    def test_a_version_1_manifest_is_refused(self):
        legacy = expected_manifest()
        legacy["schema_version"] = 1
        legacy["packages"] = {
            "claudine": list(legacy["packages"]["claudine"]["tests"])
        }
        code, stderr = self.validate_manifest(legacy)
        self.assertNotEqual(0, code, "a v1 manifest has no ignored/excluded sets")
        self.assertIn("schema", stderr.lower())

    @pending(
        "manifest-v2",
        "scripts/ci/completion.py does not exist, so a manifest without its "
        "provenance cannot be refused",
        oracle=VALIDATOR_ORACLE,
    )
    def test_the_resolved_nextest_version_and_provenance_are_required(self):
        # Not `subTest`: its failures are recorded rather than raised, so the
        # pending wrapper could not see them.
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

    @pending(
        "manifest-v2",
        "scripts/ci/completion.py does not exist, so a manifest from another "
        "target cannot be refused for the comparison",
        oracle=VALIDATOR_ORACLE,
    )
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

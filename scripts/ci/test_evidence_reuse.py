#!/usr/bin/env python3
"""Regression fixtures for multi-environment, per-cell evidence reuse.

Each fixture builds a real Git repository, attaches real notes, and asks the
evidence module the question CI asks. Phase 4 implemented every contract these
fixtures were written to hold, so none of them is pending any more.

The two entry points they pin:

    verify_cells(plan_path, base, head) -> (accepted_cells, rejection_reasons)
    gate_input_identity(paths, ref) -> str

`verify_cells` supersedes `verified_environment()`, which returns the *first*
matching environment as a bare name and so can never combine two hosts. The
older function survives only for the version-1 notes of spec section 3.6, and
the `test_today_*` fixtures below pin that narrower job.
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

import local_evidence  # noqa: E402
import schema  # noqa: E402
from local_evidence import NOTES_PREFIX, verified_environment  # noqa: E402


def target(name: str):
    """The entry point `name`, or a failure that says it is gone.

    A rename turns every fixture below into one legible message rather than
    twenty `AttributeError`s.
    """
    function = getattr(local_evidence, name, None)
    if function is None:
        raise AssertionError(
            f"local_evidence has no {name}(): per-cell, multi-environment "
            "verification is the contract of spec section 3, and nothing else "
            "in this repository provides it."
        )
    return function


class EvidenceFixture(unittest.TestCase):
    """A two-package repository with a resolved plan and writable notes."""

    def setUp(self) -> None:
        self.previous_cwd = Path.cwd()
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        os.chdir(self.root)
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.name", "CI Test")
        self.git("config", "user.email", "ci@example.com")
        self.write("pkg/alpha/src/lib.rs", "pub fn alpha() {}\n")
        self.write("pkg/beta/src/lib.rs", "pub fn beta() {}\n")
        self.write("docs/unrelated.md", "prose\n")
        self.git("add", "-A")
        self.git("commit", "-q", "-m", "base")
        self.base = self.git("rev-parse", "HEAD")
        self.write("pkg/alpha/src/lib.rs", "pub fn alpha() { /* work */ }\n")
        self.git("commit", "-q", "-am", "head")
        self.head = self.git("rev-parse", "HEAD")
        self.plan_path = self.root / "plan.json"
        self.plan_path.write_text(schema.canonical(self.plan()), encoding="utf-8")

    def tearDown(self) -> None:
        os.chdir(self.previous_cwd)
        self.temporary_directory.cleanup()

    def git(self, *args: str) -> str:
        return subprocess.run(
            ["git", *args], check=True, capture_output=True, text=True
        ).stdout.strip()

    def write(self, relative: str, text: str) -> None:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def add_note(self, environment: str, document: dict | str, commit: str | None = None) -> None:
        contents = document if isinstance(document, str) else schema.canonical(document)
        self.git(
            "notes",
            "--ref",
            f"{NOTES_PREFIX}/{environment}",
            "add",
            "-f",
            "-m",
            contents,
            commit or self.head,
        )

    # -- staged reports, as `just _stage_junit` leaves them -----------------

    def stage(self, *records: dict) -> Path:
        directory = self.root / "stage"
        directory.mkdir(exist_ok=True)
        manifest = directory / "manifest.jsonl"
        manifest.write_text(
            "".join(json.dumps(record) + "\n" for record in records), encoding="utf-8"
        )
        return directory

    def staged(self, package: str, gate: str = "L1", exit_code: int = 0, **extra) -> dict:
        return {
            "tier": gate,
            "package": package,
            "xml": f"{gate}/{package}.xml",
            "exit_code": exit_code,
            "environment": "macos-latest",
            "duration_s": 12,
            "report_present": True,
            **extra,
        }

    def report(self, directory: Path, record: dict, *, failures: list[str] = ()) -> None:
        # nextest spells a case as `classname` (the test binary) plus `name`
        # (the test path inside it); the receipt joins them back into one
        # Rust-shaped identity.
        cases = "".join(
            f'<testcase classname="{record["package"]}" name="{name}">'
            '<failure message="boom"/></testcase>'
            for name in failures
        )
        cases += '<testcase classname="ok" name="fine"/>' * 3
        total = len(failures) + 3
        path = directory / record["xml"]
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            f'<?xml version="1.0"?><testsuites>'
            f'<testsuite name="{record["package"]}" tests="{total}" '
            f'failures="{len(failures)}" errors="0" skipped="0">{cases}</testsuite>'
            "</testsuites>",
            encoding="utf-8",
        )

    def publish(self, contents: str, environment: str = "macos-latest") -> None:
        self.add_note(environment, contents)

    # -- documents ----------------------------------------------------------

    def plan(self) -> dict:
        cells = [
            self.plan_cell(package, environment, gate)
            for package in ("alpha", "beta")
            for environment, gate in (
                ("macos-latest", "L1"),
                ("ubuntu-latest", "L1"),
                ("wsl2-ubuntu", "L1"),
            )
        ]
        return {
            "schema_version": schema.RESOLVED_PLAN_SCHEMA_VERSION,
            "base": self.base,
            "head": self.head,
            "change_class": "package",
            "full_scope": False,
            "full_scope_gates": [],
            "areas": [
                {"area": "pkg", "selection_reason": "source change", "packages": ["alpha", "beta"]}
            ],
            "packages": [
                {
                    "package": name,
                    "area": "pkg",
                    "selection_reason": "source change",
                    "gates": ["L1"],
                    "targets": ["lib", "test"],
                    "tiers": ["L1"],
                    "test_args": "",
                    "check_args": f"-p {name}",
                    "l2_backends": [],
                    "runner_tools": [],
                    "companion_suites": [],
                    "l1_include_slow": False,
                    "native": {},
                    # The build closure the gate-input identity covers. Real
                    # plans get this from `cargo metadata`; here the two
                    # packages are independent, so each closure is its own
                    # directory.
                    "input_paths": [f"pkg/{name}"],
                }
                for name in ("alpha", "beta")
            ],
            "source_packages": ["alpha"],
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
            "cells": cells,
            "accepted_evidence": [],
            "policy_gaps": [],
            "prohibited_cells": [],
            "job_estimate": len(cells),
            "preflight_os": ["ubuntu-latest"],
            "preflight_reason": "package-local change",
            "flags": {"ci_tooling": False},
        }

    def plan_cell(self, package: str, environment: str, gate: str) -> dict:
        return {
            "package": package,
            "area": "pkg",
            "environment": environment,
            "gate": gate,
            "execution": "execute",
            "origin": "ci",
            "state": "pending",
            "reusable": True,
            "target_kinds": ["lib", "test"],
            "compile_coverage_from": gate,
            "selection_reason": "source package on a required environment",
        }

    def receipt(self, environment: str, cells: list[dict], **overrides: object) -> dict:
        document = {
            "schema_version": schema.RECEIPT_SCHEMA_VERSION,
            "environment": environment,
            "base": self.base,
            "head": self.head,
            "tree": self.git("rev-parse", f"{self.head}^{{tree}}"),
            "scope_identity": "00112233445566778899aabb",
            "completion": "complete",
            "host": {
                "os": "MacOS",
                "kernel": "Darwin 27.0.0",
                "report_dir": "~/.rusty-biscuit/ci-evidence/head",
            },
            "cells": cells,
        }
        document.update(overrides)
        return document

    def receipt_cell(self, package: str, gate: str = "L1", **overrides: object) -> dict:
        cell = {
            "package": package,
            "gate": gate,
            "outcome": "pass",
            "exit_code": 0,
            "completion": "complete",
            "counts": {"total": 4, "passed": 4, "failed": 0, "skipped": 0, "errored": 0},
            "duration_s": 12,
            "gate_input_identity": "aabbccddeeff",
            "backends": [],
            "report": f"{package}-{gate}.xml",
        }
        cell.update(overrides)
        return cell

    def keys(self, cells: list[dict]) -> list[str]:
        return sorted(
            f"{cell['package']}/{cell['environment']}/{cell['gate']}" for cell in cells
        )


class MultiEnvironmentTests(EvidenceFixture):
    """AC5: macOS and WSL evidence reused in one run."""

    def test_two_environments_are_accepted_together(self) -> None:
        for environment in ("macos-latest", "wsl2-ubuntu"):
            self.add_note(
                environment,
                self.receipt(
                    environment, [self.receipt_cell("alpha"), self.receipt_cell("beta")]
                ),
            )
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual(
            [
                "alpha/macos-latest/L1",
                "alpha/wsl2-ubuntu/L1",
                "beta/macos-latest/L1",
                "beta/wsl2-ubuntu/L1",
            ],
            self.keys(accepted),
        )
        self.assertEqual([], rejections)

    def test_one_environment_covering_one_package_leaves_the_other_scheduled(self) -> None:
        self.add_note("macos-latest", self.receipt("macos-latest", [self.receipt_cell("alpha")]))
        accepted, _ = target("verify_cells")(str(self.plan_path), self.base, self.head)
        self.assertEqual(["alpha/macos-latest/L1"], self.keys(accepted))

    def test_today_one_environment_is_the_most_that_can_be_named(self) -> None:
        # Non-pending: pins the defect PR #76 exposed, so Phase 4's change is
        # visibly a replacement and not an addition alongside it.
        self.assertEqual(
            "", verified_environment(str(self.plan_path), self.base, self.head)
        )
        self.assertIsInstance(
            verified_environment(str(self.plan_path), self.base, self.head), str
        )


class GateInputIdentityTests(EvidenceFixture):
    """AC7: an older head is reusable only when the cell's inputs are identical."""

    def advance(self, relative: str, text: str) -> str:
        self.write(relative, text)
        self.git("commit", "-q", "-am", f"touch {relative}")
        return self.git("rev-parse", "HEAD")

    def test_an_unrelated_change_leaves_the_identity_equal(self) -> None:
        identity = target("gate_input_identity")
        before = identity(["pkg/alpha"], self.head)
        after_head = self.advance("docs/unrelated.md", "more prose\n")
        self.assertEqual(before, identity(["pkg/alpha"], after_head))

    def test_a_change_inside_the_closure_changes_the_identity(self) -> None:
        identity = target("gate_input_identity")
        before = identity(["pkg/alpha"], self.head)
        after_head = self.advance("pkg/alpha/src/lib.rs", "pub fn alpha() { /* v2 */ }\n")
        self.assertNotEqual(before, identity(["pkg/alpha"], after_head))

    def test_an_older_receipt_is_reused_when_its_inputs_did_not_change(self) -> None:
        old_head = self.head
        note = self.receipt("macos-latest", [self.receipt_cell("alpha")])
        self.add_note("macos-latest", note, commit=old_head)
        self.advance("docs/unrelated.md", "more prose\n")
        self.head = self.git("rev-parse", "HEAD")
        self.plan_path.write_text(schema.canonical(self.plan()), encoding="utf-8")
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual(["alpha/macos-latest/L1"], self.keys(accepted))
        self.assertEqual([], rejections)

    def test_an_older_receipt_is_rejected_when_its_inputs_changed(self) -> None:
        old_head = self.head
        self.add_note(
            "macos-latest",
            self.receipt("macos-latest", [self.receipt_cell("alpha")]),
            commit=old_head,
        )
        self.advance("pkg/alpha/src/lib.rs", "pub fn alpha() { /* v2 */ }\n")
        self.head = self.git("rev-parse", "HEAD")
        self.plan_path.write_text(schema.canonical(self.plan()), encoding="utf-8")
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual([], self.keys(accepted))
        self.assertTrue(
            any(reason.startswith("gate-inputs-changed:") for reason in rejections),
            rejections,
        )

    def test_the_rejection_code_is_in_the_shared_vocabulary(self) -> None:
        self.assertIn("gate-inputs-changed", schema.REJECTIONS)


class NoteAttachmentTests(EvidenceFixture):
    """AC5/AC7/AC12: a receipt is credited only to the ref and commit it was filed under."""

    def test_a_receipt_under_the_wrong_environment_ref_is_credited_to_neither(self) -> None:
        self.add_note(
            "wsl2-ubuntu", self.receipt("macos-latest", [self.receipt_cell("alpha")])
        )
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual([], self.keys(accepted))
        self.assertEqual(1, len(rejections), rejections)
        self.assertTrue(rejections[0].startswith("environment-mismatch:"), rejections)
        self.assertIn("wsl2-ubuntu", rejections[0])
        self.assertIn("macos-latest", rejections[0])

    def test_a_receipt_declaring_another_head_is_rejected(self) -> None:
        self.add_note(
            "macos-latest",
            self.receipt("macos-latest", [self.receipt_cell("alpha")], head=self.base),
        )
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual([], self.keys(accepted))
        self.assertTrue(
            any(reason.startswith("revision-mismatch:") for reason in rejections),
            rejections,
        )

    def test_a_receipt_declaring_another_tree_is_rejected(self) -> None:
        # The declared tree is the base's, whose `pkg/alpha` differs from the
        # head's; pre-fix this was accepted as `prior-local` because the note
        # sits on the head itself and equivalence compared head with head.
        self.add_note(
            "macos-latest",
            self.receipt(
                "macos-latest",
                [self.receipt_cell("alpha")],
                tree=self.git("rev-parse", f"{self.base}^{{tree}}"),
            ),
        )
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual([], self.keys(accepted))
        self.assertTrue(
            any(reason.startswith("revision-mismatch:") for reason in rejections),
            rejections,
        )

    def test_the_rejection_codes_are_in_the_shared_vocabulary(self) -> None:
        self.assertIn("environment-mismatch", schema.REJECTIONS)
        self.assertIn("revision-mismatch", schema.REJECTIONS)


class OutcomeTests(EvidenceFixture):
    """AC8: a complete failure is evidence and stays a failure."""

    def test_a_complete_failure_is_accepted_as_a_failed_cell(self) -> None:
        failing = self.receipt_cell(
            "alpha", outcome="fail", exit_code=100, failed_tests=["alpha::boom"]
        )
        self.add_note("macos-latest", self.receipt("macos-latest", [failing]))
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual(["alpha/macos-latest/L1"], self.keys(accepted))
        self.assertEqual("fail", accepted[0]["outcome"])
        self.assertEqual([], rejections)

    def test_an_interrupted_run_yields_no_reusable_cell(self) -> None:
        self.add_note(
            "macos-latest",
            self.receipt(
                "macos-latest", [self.receipt_cell("alpha")], completion="interrupted"
            ),
        )
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual([], accepted)
        self.assertTrue(
            any(reason.startswith("incomplete-run:") for reason in rejections), rejections
        )

    def test_an_interrupted_cell_leaves_its_siblings_reusable(self) -> None:
        self.add_note(
            "macos-latest",
            self.receipt(
                "macos-latest",
                [
                    self.receipt_cell("alpha"),
                    self.receipt_cell("beta", completion="interrupted"),
                ],
            ),
        )
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual(["alpha/macos-latest/L1"], self.keys(accepted))
        self.assertEqual(1, len(rejections))


class DuplicateEvidenceTests(EvidenceFixture):
    """Two receipts for one cell: agreement is fine, disagreement is not."""

    def test_agreeing_duplicates_are_accepted_once(self) -> None:
        self.add_note(
            "macos-latest",
            self.receipt(
                "macos-latest", [self.receipt_cell("alpha"), self.receipt_cell("alpha")]
            ),
        )
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual(["alpha/macos-latest/L1"], self.keys(accepted))
        self.assertEqual([], rejections)

    def test_conflicting_duplicates_fail_safe(self) -> None:
        self.add_note(
            "macos-latest",
            self.receipt(
                "macos-latest",
                [
                    self.receipt_cell("alpha"),
                    self.receipt_cell(
                        "alpha", outcome="fail", exit_code=100, failed_tests=["boom"]
                    ),
                ],
            ),
        )
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual([], accepted)
        self.assertTrue(
            any(reason.startswith("conflicting-evidence:") for reason in rejections),
            rejections,
        )


class RejectionTests(EvidenceFixture):
    """AC7: every refusal names its specific reason."""

    def test_a_malformed_note_is_rejected_with_a_reason(self) -> None:
        self.add_note("macos-latest", "{not json")
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual([], accepted)
        self.assertTrue(
            any(reason.startswith("malformed-receipt:") for reason in rejections),
            rejections,
        )

    def test_today_a_malformed_note_fails_safe_but_says_nothing(self) -> None:
        # Non-pending: the fail-safe half already holds and must survive Phase 4.
        self.add_note("macos-latest", "{not json")
        self.assertEqual(
            "", verified_environment(str(self.plan_path), self.base, self.head)
        )

    def test_today_a_missing_note_fails_safe(self) -> None:
        self.assertEqual(
            "", verified_environment(str(self.plan_path), self.base, self.head)
        )

    def test_a_future_schema_version_is_rejected_with_its_own_code(self) -> None:
        note = self.receipt("macos-latest", [self.receipt_cell("alpha")])
        note["schema_version"] = 99
        self.add_note("macos-latest", note)
        _, rejections = target("verify_cells")(str(self.plan_path), self.base, self.head)
        self.assertTrue(
            any(reason.startswith("unknown-schema-version:") for reason in rejections),
            rejections,
        )


class LegacyReceiptTests(EvidenceFixture):
    """AC16: version-1 notes stay exact-tree, pass-only, and unmeasured."""

    def legacy_note(self) -> dict:
        return {
            "schema_version": 1,
            "base": self.base,
            "head": self.head,
            "tree": self.git("rev-parse", f"{self.head}^{{tree}}"),
            "environment": "macos-latest",
            "source_packages": ["alpha"],
            "reverse_dependencies": [],
            "l1_packages": ["alpha"],
            "l2_packages": [],
        }

    def test_a_v1_note_is_accepted_on_exact_tree_identity(self) -> None:
        self.add_note("macos-latest", self.legacy_note())
        accepted, _ = target("verify_cells")(str(self.plan_path), self.base, self.head)
        self.assertIn("alpha/macos-latest/L1", self.keys(accepted))

    def test_a_v1_note_is_never_accepted_through_gate_input_equivalence(self) -> None:
        old_head = self.head
        self.add_note("macos-latest", self.legacy_note(), commit=old_head)
        self.write("docs/unrelated.md", "more prose\n")
        self.git("commit", "-q", "-am", "unrelated")
        self.head = self.git("rev-parse", "HEAD")
        self.plan_path.write_text(schema.canonical(self.plan()), encoding="utf-8")
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual([], accepted)
        self.assertTrue(
            any(
                reason.startswith("v1-not-equivalence-eligible:") for reason in rejections
            ),
            rejections,
        )

    def test_a_v1_note_under_the_wrong_environment_ref_is_credited_to_neither(self) -> None:
        self.add_note("wsl2-ubuntu", self.legacy_note())
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual([], self.keys(accepted))
        self.assertEqual(1, len(rejections), rejections)
        self.assertTrue(rejections[0].startswith("environment-mismatch:"), rejections)

    def test_a_v1_cell_renders_its_measurements_as_unrecorded(self) -> None:
        self.add_note("macos-latest", self.legacy_note())
        accepted, _ = target("verify_cells")(str(self.plan_path), self.base, self.head)
        legacy = next(cell for cell in accepted if cell["package"] == "alpha")
        self.assertEqual(schema.UNRECORDED_MEASUREMENT, legacy["measurements"])

    def test_the_v1_note_the_hook_writes_today_is_still_understood(self) -> None:
        # Non-pending: v1 notes in the wild must keep working across the
        # migration, so this pins the current acceptance path.
        contents = local_evidence.record(
            str(self.legacy_scope()), self.base, self.head, "macos-latest"
        )
        self.add_note("macos-latest", contents)
        self.assertEqual(
            "macos-latest",
            verified_environment(str(self.legacy_scope()), self.base, self.head),
        )

    def legacy_scope(self) -> Path:
        path = self.root / "legacy-scope.json"
        if not path.exists():
            path.write_text(
                json.dumps(
                    {
                        "source_packages": ["alpha"],
                        "reverse_dependencies": [],
                        "matrix": [
                            {
                                "package": "alpha",
                                "gates": ["lint", "check", "test"],
                                "tiers": ["L1"],
                                "native_environments": ["macos-latest"],
                                "l2_environments": [],
                                "wsl": False,
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
        return path


class SuccessiveReceiptTests(EvidenceFixture):
    """AC5/AC7: receipts on successive commits of one environment combine per cell.

    A later run that covered fewer packages must not hide an older, still
    valid receipt for the packages it did not cover; conflicts on one cell go
    to the newest qualifying candidate, whatever its outcome.
    """

    ENV = "wsl2-ubuntu"

    def advance(self, relative: str, text: str) -> str:
        self.write(relative, text)
        self.git("commit", "-q", "-am", f"touch {relative}")
        self.head = self.git("rev-parse", "HEAD")
        self.plan_path.write_text(schema.canonical(self.plan()), encoding="utf-8")
        return self.head

    def receipt_at(self, commit: str, cells: list[dict], environment: str = ENV) -> dict:
        return self.receipt(
            environment, cells, head=commit, tree=self.git("rev-parse", f"{commit}^{{tree}}")
        )

    def legacy_note_at(self, commit: str, environment: str = "macos-latest") -> dict:
        return {
            "schema_version": 1,
            "base": self.base,
            "head": commit,
            "tree": self.git("rev-parse", f"{commit}^{{tree}}"),
            "environment": environment,
            "source_packages": ["alpha"],
            "reverse_dependencies": [],
            "l1_packages": ["alpha"],
            "l2_packages": [],
        }

    def by_key(self, accepted: list[dict]) -> dict[str, dict]:
        return {
            f"{cell['package']}/{cell['environment']}/{cell['gate']}": cell for cell in accepted
        }

    def test_a_newer_partial_receipt_does_not_hide_an_older_one(self) -> None:
        # The review's reproduction: alpha on C1, an unrelated commit C2 that
        # carries beta only, both packages' inputs unchanged.
        first = self.head
        self.add_note(self.ENV, self.receipt_at(first, [self.receipt_cell("alpha")]), first)
        second = self.advance("docs/unrelated.md", "more prose\n")
        self.add_note(self.ENV, self.receipt_at(second, [self.receipt_cell("beta")]), second)
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        cells = self.by_key(accepted)
        self.assertEqual(["alpha/wsl2-ubuntu/L1", "beta/wsl2-ubuntu/L1"], sorted(cells))
        self.assertEqual("prior-local", cells["alpha/wsl2-ubuntu/L1"]["origin"])
        self.assertEqual(first, cells["alpha/wsl2-ubuntu/L1"]["evidence"]["commit"])
        self.assertEqual("local", cells["beta/wsl2-ubuntu/L1"]["origin"])
        self.assertEqual(second, cells["beta/wsl2-ubuntu/L1"]["evidence"]["commit"])
        self.assertEqual([], rejections)

    def test_an_older_cell_whose_inputs_changed_is_refused_and_its_sibling_kept(self) -> None:
        first = self.head
        self.add_note(self.ENV, self.receipt_at(first, [self.receipt_cell("alpha")]), first)
        second = self.advance("pkg/alpha/src/lib.rs", "pub fn alpha() { /* v2 */ }\n")
        self.add_note(self.ENV, self.receipt_at(second, [self.receipt_cell("beta")]), second)
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual(["beta/wsl2-ubuntu/L1"], self.keys(accepted))
        self.assertEqual(1, len(rejections), rejections)
        self.assertTrue(rejections[0].startswith("gate-inputs-changed:"), rejections)
        self.assertIn(first[:9], rejections[0])

    def test_a_malformed_newer_note_does_not_hide_an_older_one(self) -> None:
        first = self.head
        self.add_note(
            self.ENV,
            self.receipt_at(first, [self.receipt_cell("alpha"), self.receipt_cell("beta")]),
            first,
        )
        second = self.advance("docs/unrelated.md", "more prose\n")
        self.add_note(self.ENV, "{not json", second)
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual(["alpha/wsl2-ubuntu/L1", "beta/wsl2-ubuntu/L1"], self.keys(accepted))
        self.assertEqual({first}, {cell["evidence"]["commit"] for cell in accepted})
        self.assertEqual(1, len(rejections), rejections)
        self.assertTrue(rejections[0].startswith("malformed-receipt:"), rejections)
        self.assertIn(second[:9], rejections[0])

    def test_a_newer_complete_failure_shadows_an_older_pass(self) -> None:
        first = self.head
        self.add_note(self.ENV, self.receipt_at(first, [self.receipt_cell("alpha")]), first)
        second = self.advance("docs/unrelated.md", "more prose\n")
        failing = self.receipt_cell(
            "alpha", outcome="fail", exit_code=100, failed_tests=["alpha::boom"]
        )
        self.add_note(self.ENV, self.receipt_at(second, [failing]), second)
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual(["alpha/wsl2-ubuntu/L1"], self.keys(accepted))
        self.assertEqual("fail", accepted[0]["outcome"])
        self.assertEqual("local", accepted[0]["origin"])
        self.assertEqual(second, accepted[0]["evidence"]["commit"])
        self.assertEqual([], rejections)

    def test_a_newer_pass_shadows_an_older_failure_too(self) -> None:
        # Precedence is recency, not outcome: the newest qualifying candidate
        # wins in both directions.
        first = self.head
        failing = self.receipt_cell(
            "alpha", outcome="fail", exit_code=100, failed_tests=["alpha::boom"]
        )
        self.add_note(self.ENV, self.receipt_at(first, [failing]), first)
        second = self.advance("docs/unrelated.md", "more prose\n")
        self.add_note(self.ENV, self.receipt_at(second, [self.receipt_cell("alpha")]), second)
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual(["alpha/wsl2-ubuntu/L1"], self.keys(accepted))
        self.assertEqual("pass", accepted[0]["outcome"])
        self.assertEqual(second, accepted[0]["evidence"]["commit"])
        self.assertEqual([], rejections)

    def test_an_older_v1_note_does_not_claim_a_cell_a_newer_v2_note_resolved(self) -> None:
        # An empty commit keeps the tree, so the older v1 note is exact-tree
        # eligible for the head and would otherwise claim both cells.
        first = self.head
        self.add_note("macos-latest", self.legacy_note_at(first), first)
        self.git("commit", "-q", "--allow-empty", "-m", "empty")
        self.head = self.git("rev-parse", "HEAD")
        self.plan_path.write_text(schema.canonical(self.plan()), encoding="utf-8")
        self.add_note(
            "macos-latest",
            self.receipt_at(self.head, [self.receipt_cell("alpha")], "macos-latest"),
            self.head,
        )
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        cells = self.by_key(accepted)
        self.assertEqual(["alpha/macos-latest/L1", "beta/macos-latest/L1"], sorted(cells))
        alpha, beta = cells["alpha/macos-latest/L1"], cells["beta/macos-latest/L1"]
        self.assertEqual(self.head, alpha["evidence"]["commit"])
        self.assertNotEqual(schema.UNRECORDED_MEASUREMENT, alpha["measurements"])
        self.assertEqual(schema.UNRECORDED_MEASUREMENT, beta["measurements"])
        self.assertEqual([], rejections)

    def test_a_v1_note_on_the_head_is_not_overridden_by_an_older_v2_note(self) -> None:
        first = self.head
        self.add_note(
            "macos-latest",
            self.receipt_at(first, [self.receipt_cell("alpha")], "macos-latest"),
            first,
        )
        second = self.advance("docs/unrelated.md", "more prose\n")
        self.add_note("macos-latest", self.legacy_note_at(second), second)
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        cells = self.by_key(accepted)
        self.assertEqual(["alpha/macos-latest/L1", "beta/macos-latest/L1"], sorted(cells))
        for cell in cells.values():
            self.assertEqual(schema.UNRECORDED_MEASUREMENT, cell["measurements"])
            self.assertEqual(1, cell["evidence"]["schema_version"])
            self.assertNotIn("commit", cell["evidence"])
        self.assertEqual([], rejections)

    def test_a_head_note_and_two_older_notes_on_another_environment_combine(self) -> None:
        first = self.head
        self.add_note(self.ENV, self.receipt_at(first, [self.receipt_cell("alpha")]), first)
        second = self.advance("docs/unrelated.md", "more prose\n")
        self.add_note(self.ENV, self.receipt_at(second, [self.receipt_cell("beta")]), second)
        third = self.advance("docs/unrelated.md", "even more prose\n")
        self.add_note(
            "macos-latest",
            self.receipt_at(
                third, [self.receipt_cell("alpha"), self.receipt_cell("beta")], "macos-latest"
            ),
            third,
        )
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        cells = self.by_key(accepted)
        self.assertEqual(
            {
                "alpha/macos-latest/L1": ("local", third),
                "beta/macos-latest/L1": ("local", third),
                "alpha/wsl2-ubuntu/L1": ("prior-local", first),
                "beta/wsl2-ubuntu/L1": ("prior-local", second),
            },
            {key: (cell["origin"], cell["evidence"]["commit"]) for key, cell in cells.items()},
        )
        self.assertEqual([], rejections)


class NoVerifyTests(EvidenceFixture):
    """AC7 / spec 3.7: `--no-verify` adds nothing and invalidates nothing."""

    def test_a_prior_receipt_survives_a_push_that_published_none(self) -> None:
        self.add_note("macos-latest", self.receipt("macos-latest", [self.receipt_cell("alpha")]))
        # `--no-verify` means the hook never ran: no new note is written, and
        # nothing removes the one already published for this exact head.
        accepted, _ = target("verify_cells")(str(self.plan_path), self.base, self.head)
        self.assertEqual(["alpha/macos-latest/L1"], self.keys(accepted))

    def test_today_a_head_with_no_note_verifies_to_nothing(self) -> None:
        self.assertEqual(
            "", verified_environment(str(self.plan_path), self.base, self.head)
        )


class ReceiptRecordingTests(EvidenceFixture):
    """AC4/AC8: what a finished local run publishes, and what CI then reads.

    The producing half of the contract. Every fixture above hands `verify_cells`
    a hand-built receipt; these build one the way the hook does — from the JUnit
    staging manifest `just _stage_junit` appends to — and feed it back through
    verification, so the writer and the reader are pinned to each other rather
    than to a fixture's idea of the document.

    The stage directory doubles as the retained report directory here: unlike
    the hook's, it outlives the call, so naming it is truthful.
    """

    def retained(self, directory: Path) -> Path:
        """A copy of the staged reports where the hook keeps them: elsewhere."""
        destination = self.root / "evidence" / self.head / "macos-latest"
        shutil.copytree(directory, destination)
        return destination

    def test_the_recorded_report_dir_is_exactly_what_was_passed(self) -> None:
        record = self.staged("alpha")
        directory = self.stage(record)
        self.report(directory, record)
        retained = self.retained(directory)
        document = json.loads(
            local_evidence.record_cells(
                str(self.plan_path),
                str(directory),
                "macos-latest",
                self.base,
                self.head,
                report_dir=str(retained),
            )
        )
        self.assertEqual(str(retained), document["host"]["report_dir"])
        self.assertNotIn("~/.rusty-biscuit", document["host"]["report_dir"])
        for cell in document["cells"]:
            self.assertTrue((retained / cell["report"]).is_file(), cell["report"])

    def test_an_empty_report_dir_is_refused(self) -> None:
        # Spec section 2: the receipt records where the reports are retained so
        # a reviewer can ask for them. Nothing may invent that path.
        record = self.staged("alpha")
        directory = self.stage(record)
        self.report(directory, record)
        for empty in ("", "   "):
            with self.subTest(report_dir=repr(empty)):
                with self.assertRaisesRegex(ValueError, "--report-dir"):
                    local_evidence.record_cells(
                        str(self.plan_path),
                        str(directory),
                        "macos-latest",
                        self.base,
                        self.head,
                        report_dir=empty,
                    )

    def test_a_report_dir_that_does_not_hold_the_reports_is_refused(self) -> None:
        record = self.staged("alpha")
        directory = self.stage(record)
        self.report(directory, record)
        elsewhere = self.root / "elsewhere"
        elsewhere.mkdir()
        with self.assertRaisesRegex(ValueError, "does not retain L1/alpha.xml"):
            local_evidence.record_cells(
                str(self.plan_path),
                str(directory),
                "macos-latest",
                self.base,
                self.head,
                report_dir=str(elsewhere),
            )

    def test_a_passing_run_publishes_measured_cells_that_verify_back(self) -> None:
        record = self.staged("alpha")
        directory = self.stage(record)
        self.report(directory, record)
        contents = local_evidence.record_cells(
            str(self.plan_path),
            str(directory),
            "macos-latest",
            self.base,
            self.head,
            report_dir=str(directory),
        )
        document = json.loads(contents)
        self.assertEqual([], schema.validate_receipt(document))
        cell = document["cells"][0]
        self.assertEqual("pass", cell["outcome"])
        self.assertEqual(
            {"total": 3, "passed": 3, "failed": 0, "skipped": 0, "errored": 0},
            cell["counts"],
        )
        self.assertEqual(12, cell["duration_s"])

        # Read back through the consumer, not through the fixture's own idea of
        # the document: the receipt must satisfy the very cell it claims.
        self.publish(contents)
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual(["alpha/macos-latest/L1"], self.keys(accepted))
        self.assertEqual([], rejections)
        self.assertEqual("3 test(s), 0 failed, 12s", accepted[0]["measurements"])

    def test_recording_the_same_run_twice_produces_identical_bytes(self) -> None:
        record = self.staged("alpha")
        directory = self.stage(record)
        self.report(directory, record)
        first = local_evidence.record_cells(
            str(self.plan_path),
            str(directory),
            "macos-latest",
            self.base,
            self.head,
            report_dir=str(directory),
        )
        self.publish(first)
        second = local_evidence.record_cells(
            str(self.plan_path),
            str(directory),
            "macos-latest",
            self.base,
            self.head,
            report_dir=str(directory),
        )
        self.assertEqual(first, second)
        self.assertEqual(first, schema.canonical(json.loads(second)))

    def test_a_complete_failure_is_published_and_stays_a_failure(self) -> None:
        record = self.staged("alpha", exit_code=100)
        directory = self.stage(record)
        self.report(directory, record, failures=["boom"])
        contents = local_evidence.record_cells(
            str(self.plan_path),
            str(directory),
            "macos-latest",
            self.base,
            self.head,
            report_dir=str(directory),
        )
        self.publish(contents)
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual(["alpha/macos-latest/L1"], self.keys(accepted))
        self.assertEqual("fail", accepted[0]["outcome"])
        self.assertEqual(["alpha::boom"], accepted[0]["failed_tests"])
        self.assertEqual([], rejections)

    def test_a_gate_that_produced_no_report_is_not_reusable(self) -> None:
        # A compile failure: nextest emits no XML, so the run has an exit code
        # and no attributable outcome.
        record = self.staged("alpha", exit_code=101, report_present=False)
        directory = self.stage(record)
        contents = local_evidence.record_cells(
            str(self.plan_path),
            str(directory),
            "macos-latest",
            self.base,
            self.head,
            report_dir=str(directory),
        )
        self.assertEqual("partial", json.loads(contents)["cells"][0]["completion"])
        self.publish(contents)
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual([], accepted)
        self.assertTrue(
            any(reason.startswith("incomplete-run:") for reason in rejections), rejections
        )

    def test_a_run_that_staged_nothing_publishes_nothing(self) -> None:
        directory = self.stage()
        with self.assertRaisesRegex(ValueError, "staged no L1/L2/browser report"):
            local_evidence.record_cells(
                str(self.plan_path),
                str(directory),
                "macos-latest",
                self.base,
                self.head,
                report_dir=str(directory),
            )

    def test_lint_and_check_are_never_published_from_a_local_run(self) -> None:
        # They stage no report, so a local receipt can never claim them; the
        # plan keeps them as CI-origin cells.
        directory = self.stage(
            {**self.staged("alpha"), "tier": "lint"},
            {**self.staged("alpha"), "tier": "check"},
        )
        with self.assertRaisesRegex(ValueError, "staged no L1/L2/browser report"):
            local_evidence.record_cells(
                str(self.plan_path),
                str(directory),
                "macos-latest",
                self.base,
                self.head,
                report_dir=str(directory),
            )

    def test_a_non_ancestor_base_cannot_be_recorded(self) -> None:
        record = self.staged("alpha")
        directory = self.stage(record)
        self.report(directory, record)
        self.git("checkout", "-q", "-b", "advanced", self.base)
        self.git("commit", "-q", "--allow-empty", "-m", "advance")
        advanced = self.git("rev-parse", "HEAD")
        with self.assertRaisesRegex(ValueError, "must be an ancestor"):
            local_evidence.record_cells(
                str(self.plan_path), str(directory), "macos-latest", advanced, self.head
            )


class BackendProofTests(EvidenceFixture):
    """A Level 2 cell is evidence only if a backend actually drove a test.

    An installed backend plus zero backend tests is not evidence: a Level 2
    suite whose backend is absent *skips*, and nextest prints PASS in about
    0.02 s. These fixtures pin the receipt's refusal to call that a tested cell.
    """

    def l2_plan_cells(self) -> None:
        plan = json.loads(self.plan_path.read_text(encoding="utf-8"))
        plan["cells"].append(self.plan_cell("alpha", "macos-latest", "L2"))
        for entry in plan["packages"]:
            if entry["package"] == "alpha":
                entry["gates"] = ["L1", "L2"]
                entry["tiers"] = ["L1", "L2"]
                entry["l2_backends"] = ["tmux"]
        plan["job_estimate"] = len(plan["cells"])
        self.assertEqual([], schema.validate_resolved_plan(plan))
        self.plan_path.write_text(schema.canonical(plan), encoding="utf-8")

    def staged_l2(self, *, required: list[str], ran: list[str]) -> Path:
        record = self.staged("alpha", gate="L2")
        directory = self.stage(self.staged("alpha"), record)
        self.report(directory, self.staged("alpha"))
        self.report(directory, record)
        (directory / "gate-backends.jsonl").write_text(
            json.dumps({"package": "alpha", "gate": "L2", "backends": required}) + "\n",
            encoding="utf-8",
        )
        (directory / "backend-executions.jsonl").write_text(
            "".join(
                json.dumps({"backend": backend, "test": "level2::x", "decision": "run"})
                + "\n"
                for backend in ran
            ),
            encoding="utf-8",
        )
        return directory

    def cells_of(self, directory: Path) -> dict[str, dict]:
        contents = local_evidence.record_cells(
            str(self.plan_path),
            str(directory),
            "macos-latest",
            self.base,
            self.head,
            report_dir=str(directory),
        )
        return {cell["gate"]: cell for cell in json.loads(contents)["cells"]}

    def test_a_proven_backend_is_named_on_the_cell(self) -> None:
        self.l2_plan_cells()
        cells = self.cells_of(self.staged_l2(required=["tmux"], ran=["tmux"]))
        self.assertEqual(["tmux"], cells["L2"]["backends"])
        self.assertEqual("complete", cells["L2"]["completion"])

    def test_a_backend_that_never_ran_makes_the_cell_unreusable(self) -> None:
        self.l2_plan_cells()
        directory = self.staged_l2(required=["tmux"], ran=[])
        cells = self.cells_of(directory)
        self.assertEqual([], cells["L2"]["backends"])
        self.assertEqual("partial", cells["L2"]["completion"])

        self.publish(
            local_evidence.record_cells(
                str(self.plan_path),
                str(directory),
                "macos-latest",
                self.base,
                self.head,
                report_dir=str(directory),
            )
        )
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual(
            ["alpha/macos-latest/L1"],
            self.keys(accepted),
            "an unproven L2 cell must stay scheduled while its L1 sibling is reused",
        )
        self.assertTrue(
            any("incomplete-run" in reason for reason in rejections), rejections
        )

    def test_only_a_required_backend_counts(self) -> None:
        # A backend that ran but was not required by this invocation belongs to
        # some other package's suite; the execution records name a test, not a
        # package, so the required list is what attributes them.
        self.l2_plan_cells()
        cells = self.cells_of(self.staged_l2(required=["tmux"], ran=["tmux", "kitty"]))
        self.assertEqual(["tmux"], cells["L2"]["backends"])

    def test_an_l1_cell_needs_no_backend_proof(self) -> None:
        self.l2_plan_cells()
        cells = self.cells_of(self.staged_l2(required=["tmux"], ran=[]))
        self.assertEqual("complete", cells["L1"]["completion"])
        self.assertEqual([], cells["L1"]["backends"])


class CrossCheckPublicationTests(EvidenceFixture):
    """AC5 / spec 3.8: a cross-check run becomes a receipt, or says why not.

    `cross-check` ships the developer's local tree — uncommitted work included —
    to a standing clone, so most of its runs must publish nothing. These
    fixtures pin the boundary in both directions.
    """

    def setUp(self) -> None:
        super().setUp()
        self.head_tree = self.git("rev-parse", f"{self.head}^{{tree}}")
        # The plan's wsl2-ubuntu L1 cells for alpha/beta already exist.
        record = self.staged("alpha")
        self.directory = self.stage(record)
        self.report(self.directory, record)
        self.report_path = self.directory / record["xml"]

    def publish_cross_check(self, **overrides) -> str:
        arguments = {
            "plan_path": str(self.plan_path),
            "package": "alpha",
            "report": str(self.report_path),
            "exit_code": 0,
            "duration_s": 61,
            "tested_tree": self.head_tree,
            "remote_dirty": False,
            "run_filters": "",
            "base": self.base,
            "head": self.head,
        }
        arguments.update(overrides)
        return local_evidence.record_cross_check(**arguments)

    def test_an_exact_tree_clean_unfiltered_run_publishes_and_verifies(self) -> None:
        contents = self.publish_cross_check()
        document = json.loads(contents)
        self.assertEqual("wsl2-ubuntu", document["environment"])
        self.assertEqual(self.head_tree, document["tree"])
        self.publish(contents, environment="wsl2-ubuntu")
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual(["alpha/wsl2-ubuntu/L1"], self.keys(accepted))
        self.assertEqual([], rejections)

    def test_a_patched_tree_publishes_nothing(self) -> None:
        with self.assertRaisesRegex(ValueError, "is not this head's tree"):
            self.publish_cross_check(tested_tree="0" * 40)

    def test_a_dirty_remote_worktree_publishes_nothing(self) -> None:
        with self.assertRaisesRegex(ValueError, "dirty"):
            self.publish_cross_check(remote_dirty=True)

    def test_a_filtered_run_publishes_nothing(self) -> None:
        with self.assertRaisesRegex(ValueError, "narrowed by"):
            self.publish_cross_check(run_filters="level2_")

    def test_a_package_outside_the_plan_publishes_nothing(self) -> None:
        with self.assertRaisesRegex(ValueError, "has no gamma/wsl2-ubuntu/L1 cell"):
            self.publish_cross_check(package="gamma")

    def test_a_run_with_no_report_publishes_nothing(self) -> None:
        self.report_path.unlink()
        with self.assertRaisesRegex(ValueError, "no usable report"):
            self.publish_cross_check()

    def test_a_complete_remote_failure_is_still_evidence(self) -> None:
        record = self.staged("alpha", exit_code=100)
        self.report(self.directory, record, failures=["boom"])
        contents = self.publish_cross_check(exit_code=100)
        self.assertEqual("fail", json.loads(contents)["cells"][0]["outcome"])

    def test_a_hook_receipt_and_a_cross_check_receipt_combine(self) -> None:
        # AC5 end to end: macOS from the hook, WSL from cross-check, one answer.
        self.publish(
            local_evidence.record_cells(
                str(self.plan_path),
                str(self.directory),
                "macos-latest",
                self.base,
                self.head,
                report_dir=str(self.directory),
            )
        )
        self.publish(self.publish_cross_check(), environment="wsl2-ubuntu")
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual(
            ["alpha/macos-latest/L1", "alpha/wsl2-ubuntu/L1"], self.keys(accepted)
        )
        self.assertEqual([], rejections)


class GateGlobalInputTests(EvidenceFixture):
    """Design Decision 4: one per-gate input classification, two consumers."""

    def test_every_gate_carries_the_shared_global_inputs(self) -> None:
        for gate in schema.GATES:
            with self.subTest(gate=gate):
                inputs = local_evidence.gate_global_inputs(gate)
                for shared in ("Cargo.lock", "Cargo.toml", "rust-toolchain.toml", ".cargo"):
                    self.assertIn(shared, inputs)

    def test_clippy_configuration_moves_only_the_lint_identity(self) -> None:
        self.assertIn("clippy.toml", local_evidence.gate_global_inputs("lint"))
        for gate in ("check", "L1", "L2"):
            self.assertNotIn("clippy.toml", local_evidence.gate_global_inputs(gate))

    def test_the_test_tiers_share_one_classification(self) -> None:
        self.assertEqual(
            local_evidence.gate_global_inputs("L1"),
            local_evidence.gate_global_inputs("L2"),
        )
        self.assertIn(".config/nextest.toml", local_evidence.gate_global_inputs("L1"))

    def test_a_lockfile_change_invalidates_an_older_receipt(self) -> None:
        paths = local_evidence.cell_input_paths(
            json.loads(self.plan_path.read_text(encoding="utf-8")), "alpha", "L1"
        )
        before = local_evidence.gate_input_identity(paths, self.head)
        self.write("Cargo.lock", 'version = 4\n')
        self.git("add", "-A")
        self.git("commit", "-q", "-m", "lockfile")
        self.assertNotEqual(
            before,
            local_evidence.gate_input_identity(paths, self.git("rev-parse", "HEAD")),
        )

    def test_a_package_with_no_declared_closure_has_no_input_paths(self) -> None:
        plan = json.loads(self.plan_path.read_text(encoding="utf-8"))
        for entry in plan["packages"]:
            entry.pop("input_paths", None)
        self.assertIsNone(local_evidence.cell_input_paths(plan, "alpha", "L1"))


if __name__ == "__main__":
    unittest.main(verbosity=2)

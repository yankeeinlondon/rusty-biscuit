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

import hashlib
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
import local_evidence  # noqa: E402
import observation_notes  # noqa: E402
import plan_fixtures  # noqa: E402
import schema  # noqa: E402
import test_completion  # noqa: E402
from affected_scope import change_inventory  # noqa: E402
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
        document = {
            "schema_version": schema.RESOLVED_PLAN_SCHEMA_VERSION,
            "base": self.base,
            "head": self.head,
            "change_class": "package",
            # The real producer, so a fixture plan cannot describe a shape the
            # planner no longer emits.
            "change_inventory": change_inventory(
                ["alpha/src/lib.rs", "beta/src/lib.rs"], False
            ),
            "archive_guard": plan_fixtures.archive_guard(
                ["alpha/src/lib.rs", "beta/src/lib.rs"]
            ),
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
                    "archive_includes": [],
                    "sidecars": [],
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
            "builds": [],
            "accepted_evidence": [],
            "policy_gaps": [],
            "prohibited_cells": [],
            "job_estimate": len(cells),
            "preflight_os": ["ubuntu-latest"],
            "preflight_reason": "package-local change",
            "flags": {},
        }
        return plan_fixtures.finalize_plan(document)

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


class JustRecipeIdentityTests(EvidenceFixture):
    """Just files enter a cell's identity by recipe, exactly as they enter
    selection (fixes/2026-09-19-just-recipe-identity).

    The fixture's justfile has one recipe per CI gate, a helper the test gate
    reaches through a `just <name>` call inside a module, and two recipes CI
    never runs. Every case asserts the identity and the planner's
    `just_change_gates` answer for the same edit, which is the contract: the
    two consumers read one classification.
    """

    JUSTFILE = (
        "import 'just/devops.just'\n"
        "set shell := ['bash', '-cu']\n"
        "\n"
        "# CI's test gate\n"
        "_test pkg *args='':\n"
        "    cargo nextest run -p {{ pkg }} -E \"$(just _tier_filter sanity)\" {{ args }}\n"
        "\n"
        "_lint pkg:\n"
        "    cargo fmt --check -p {{ pkg }}\n"
        "    cargo clippy -p {{ pkg }} -- -D warnings\n"
        "\n"
        "pre-push:\n"
        "    just ci-local --l2\n"
    )
    DEVOPS = (
        "_tier_filter tier:\n"
        "    echo 'test(/(^|::)level2_/)'\n"
        "\n"
        "notify message:\n"
        "    echo {{ message }}\n"
    )

    def setUp(self) -> None:
        super().setUp()
        self.write("justfile", self.JUSTFILE)
        self.write("just/devops.just", self.DEVOPS)
        self.git("add", "-A")
        self.git("commit", "-q", "-m", "just files")
        self.head = self.git("rev-parse", "HEAD")
        self.plan_path.write_text(schema.canonical(self.plan()), encoding="utf-8")

    def edit(self, relative: str, text: str) -> str:
        self.write(relative, text)
        self.git("commit", "-q", "-am", f"edit {relative}")
        return self.git("rev-parse", "HEAD")

    def identities(self, ref: str) -> dict[str, str]:
        plan = json.loads(self.plan_path.read_text(encoding="utf-8"))
        return {
            gate: local_evidence.gate_input_identity(
                local_evidence.cell_input_paths(plan, "alpha", gate), ref, gate
            )
            for gate in schema.GATES
        }

    def moved(self, before: str, after: str) -> set[str]:
        """The planner gates whose identity differs between two revisions."""
        earlier, later = self.identities(before), self.identities(after)
        return {
            local_evidence.planner_gate_of(gate)
            for gate in schema.GATES
            if earlier[gate] != later[gate]
        }

    def selected(self, relative: str, before: str) -> set[str]:
        """What the planner schedules for the same edit, from the same tree."""
        gates = affected_scope.just_change_gates(
            relative,
            before,
            self.root,
            reader=lambda ref, path: self.git("show", f"{ref}:{path}"),
        )
        self.assertIsNotNone(gates)
        return gates

    def test_a_recipe_no_gate_reaches_moves_nothing(self) -> None:
        before = self.head
        after = self.edit("just/devops.just", self.DEVOPS.replace("echo {{ message }}", "say {{ message }}"))
        self.assertEqual(set(), self.moved(before, after))
        self.assertEqual(set(), self.selected("just/devops.just", before))

        after_root = self.edit("justfile", self.JUSTFILE.replace("just ci-local --l2", "just ci-local"))
        self.assertEqual(set(), self.moved(before, after_root))
        self.assertEqual(set(), self.selected("justfile", after))

    def test_a_helper_the_test_gate_calls_moves_only_the_test_tiers(self) -> None:
        # `_tier_filter` is reached from `_test` through a `just` call in a
        # module, never named in `CI_RECIPES_BY_GATE` itself.
        before = self.head
        after = self.edit("just/devops.just", self.DEVOPS.replace("level2_", "browser_"))
        self.assertEqual({"test"}, self.moved(before, after))
        self.assertEqual({"test"}, self.selected("just/devops.just", before))

    def test_the_lint_recipe_moves_only_lint(self) -> None:
        before = self.head
        after = self.edit("justfile", self.JUSTFILE.replace("-D warnings", "-W warnings"))
        self.assertEqual({"lint"}, self.moved(before, after))
        self.assertEqual({"lint"}, self.selected("justfile", before))

    def test_text_outside_every_recipe_moves_every_gate(self) -> None:
        before = self.head
        after = self.edit("justfile", self.JUSTFILE.replace("'bash', '-cu'", "'bash', '-eu'"))
        self.assertEqual(set(affected_scope.GATES), self.moved(before, after))
        self.assertEqual(set(affected_scope.GATES), self.selected("justfile", before))

    def test_a_comment_only_edit_moves_nothing(self) -> None:
        before = self.head
        after = self.edit("justfile", self.JUSTFILE.replace("# CI's test gate", "# the L1 gate"))
        self.assertEqual(set(), self.moved(before, after))
        self.assertEqual(set(), self.selected("justfile", before))

    def test_command_order_inside_a_recipe_is_part_of_the_identity(self) -> None:
        before = self.head
        swapped = self.JUSTFILE.replace(
            "    cargo fmt --check -p {{ pkg }}\n    cargo clippy -p {{ pkg }} -- -D warnings\n",
            "    cargo clippy -p {{ pkg }} -- -D warnings\n    cargo fmt --check -p {{ pkg }}\n",
        )
        self.assertNotEqual(self.JUSTFILE, swapped)
        after = self.edit("justfile", swapped)
        self.assertEqual({"lint"}, self.moved(before, after))

    def test_an_older_receipt_survives_an_edit_to_a_recipe_no_gate_reaches(self) -> None:
        old_head = self.head
        self.add_note(
            "macos-latest",
            self.receipt("macos-latest", [self.receipt_cell("alpha")]),
            commit=old_head,
        )
        self.head = self.edit("just/devops.just", self.DEVOPS.replace("echo {{ message }}", "say {{ message }}"))
        self.plan_path.write_text(schema.canonical(self.plan()), encoding="utf-8")
        accepted, rejections = target("verify_cells")(str(self.plan_path), self.base, self.head)
        self.assertEqual(["alpha/macos-latest/L1"], self.keys(accepted))
        self.assertEqual([], rejections)

    def test_an_older_receipt_is_rejected_when_the_test_recipe_changed(self) -> None:
        old_head = self.head
        self.add_note(
            "macos-latest",
            self.receipt("macos-latest", [self.receipt_cell("alpha")]),
            commit=old_head,
        )
        self.head = self.edit("just/devops.just", self.DEVOPS.replace("level2_", "browser_"))
        self.plan_path.write_text(schema.canonical(self.plan()), encoding="utf-8")
        accepted, rejections = target("verify_cells")(str(self.plan_path), self.base, self.head)
        self.assertEqual([], self.keys(accepted))
        self.assertTrue(
            any(reason.startswith("gate-inputs-changed:") for reason in rejections), rejections
        )

    def test_a_tree_with_no_just_files_still_has_an_identity(self) -> None:
        # The fixture's base commit predates the justfile. An older receipt
        # there must compare, not crash, and it must differ from the head's.
        self.assertNotEqual(self.identities(self.base)["L1"], self.identities(self.head)["L1"])


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
    """Only a complete passing cell may suppress execution."""

    def test_a_complete_failure_is_retained_for_diagnosis_only(self) -> None:
        failing = self.receipt_cell(
            "alpha", outcome="fail", exit_code=100, failed_tests=["alpha::boom"]
        )
        self.add_note("macos-latest", self.receipt("macos-latest", [failing]))
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual([], accepted)
        self.assertEqual(1, len(rejections), rejections)
        self.assertTrue(
            rejections[0].startswith(
                "failed-cell: alpha/macos-latest/L1 failed; failing evidence is diagnostic only"
            ),
            rejections,
        )

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
        self.assertEqual([], accepted)
        self.assertEqual(1, len(rejections), rejections)
        self.assertTrue(rejections[0].startswith("failed-cell: alpha/wsl2-ubuntu/L1"), rejections)
        self.assertIn(second[:9], rejections[0])

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
        self.assertEqual(1, len(rejections), rejections)
        self.assertTrue(rejections[0].startswith("failed-cell: alpha/wsl2-ubuntu/L1"), rejections)
        self.assertIn(first[:9], rejections[0])

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


class NarrowedEvidenceTests(EvidenceFixture):
    """A narrowed test-input run stands in for its planned cell from any host.

    The one cross-environment rule (docs/cicd/test-inputs.md): the receipt
    cell must carry the plan cell's exact `test_filter`, and the receipt must
    be the head's exact tree.
    """

    NARROW = "(binary_id(alpha::l1) & test(=docs::guide_wording_holds))"

    def setUp(self) -> None:
        super().setUp()
        self.write_narrowed_plan()

    def write_narrowed_plan(self) -> None:
        plan = self.plan()
        for cell in plan["cells"]:
            if (cell["package"], cell["environment"], cell["gate"]) == ("alpha", "ubuntu-latest", "L1"):
                cell["test_filter"] = self.NARROW
        self.plan_path.write_text(schema.canonical(plan), encoding="utf-8")

    def verify(self) -> tuple[dict[str, dict], list[str]]:
        accepted, rejections = target("verify_cells")(str(self.plan_path), self.base, self.head)
        return {
            f"{cell['package']}/{cell['environment']}/{cell['gate']}": cell for cell in accepted
        }, rejections

    def test_a_macos_narrowed_run_satisfies_the_planned_linux_cell(self) -> None:
        self.add_note(
            "macos-latest",
            self.receipt("macos-latest", [self.receipt_cell("alpha", test_filter=self.NARROW)]),
        )
        accepted, _ = self.verify()
        self.assertEqual(["alpha/ubuntu-latest/L1"], sorted(accepted))
        cell = accepted["alpha/ubuntu-latest/L1"]
        self.assertEqual(("local", self.NARROW), (cell["origin"], cell["test_filter"]))
        self.assertEqual(f"{NOTES_PREFIX}/macos-latest", cell["evidence"]["ref"])
        self.assertIn("on macos-latest", cell["measurements"])

    def test_it_never_stands_in_for_the_hosts_own_whole_tier(self) -> None:
        self.add_note(
            "macos-latest",
            self.receipt("macos-latest", [self.receipt_cell("alpha", test_filter=self.NARROW)]),
        )
        accepted, _ = self.verify()
        self.assertNotIn("alpha/macos-latest/L1", accepted)

    def test_a_run_narrowed_to_other_tests_satisfies_nothing(self) -> None:
        self.add_note(
            "macos-latest",
            self.receipt(
                "macos-latest",
                [self.receipt_cell("alpha", test_filter="(binary_id(alpha::l1))")],
            ),
        )
        accepted, _ = self.verify()
        self.assertEqual({}, accepted)

    def test_an_older_narrowed_run_is_refused_even_when_gate_inputs_match(self) -> None:
        older = self.head
        self.add_note(
            "macos-latest",
            self.receipt("macos-latest", [self.receipt_cell("alpha", test_filter=self.NARROW)]),
            commit=older,
        )
        # A documentation change leaves alpha's gate inputs identical, which is
        # exactly why only the exact tree may vouch for a narrowed run.
        self.write("docs/unrelated.md", "edited prose\n")
        self.git("commit", "-q", "-am", "edit the document the test reads")
        self.head = self.git("rev-parse", "HEAD")
        self.write_narrowed_plan()
        accepted, rejections = self.verify()
        self.assertNotIn("alpha/ubuntu-latest/L1", accepted)
        self.assertTrue(
            any("narrowed" in reason and "exact tree" in reason for reason in rejections),
            rejections,
        )

    def test_whole_tier_evidence_on_linux_does_not_satisfy_the_narrowed_cell(self) -> None:
        self.add_note(
            "ubuntu-latest",
            self.receipt(
                "ubuntu-latest",
                [self.receipt_cell("alpha")],
                host={"os": "Linux", "kernel": "6.8", "report_dir": "/r"},
            ),
        )
        accepted, _ = self.verify()
        # Accepted by verification like any exact-tree cell; the planner's
        # `cell_evidence` is what refuses it for the narrowed cell, because
        # the filters differ (`TestInputSelectionTests`).
        self.assertIsNone(accepted["alpha/ubuntu-latest/L1"].get("test_filter"))


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

    def test_a_narrowed_test_input_run_is_recorded_with_its_filter(self) -> None:
        # docs/cicd/test-inputs.md: the narrowed cell is planned on
        # ubuntu-latest, run here on macOS, and recorded WITH its filter so it
        # can never read as this host's whole tier.
        narrow = "(binary_id(alpha::l1) & test(=docs::t))"
        plan = json.loads(self.plan_path.read_text(encoding="utf-8"))
        for cell in plan["cells"]:
            if (cell["package"], cell["environment"], cell["gate"]) == ("alpha", "ubuntu-latest", "L1"):
                cell["test_filter"] = narrow
        self.plan_path.write_text(schema.canonical(plan), encoding="utf-8")
        alpha, beta = self.staged("alpha"), self.staged("beta")
        directory = self.stage(alpha, beta)
        self.report(directory, alpha)
        self.report(directory, beta)
        document = json.loads(
            local_evidence.record_cells(
                str(self.plan_path),
                str(directory),
                "macos-latest",
                self.base,
                self.head,
                report_dir=str(directory),
            )
        )
        self.assertEqual([], schema.validate_receipt(document))
        self.assertEqual(
            {"alpha": narrow, "beta": None},
            {cell["package"]: cell.get("test_filter") for cell in document["cells"]},
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

    def test_a_same_head_retry_replaces_its_cell_and_preserves_prior_cells(self) -> None:
        alpha = self.staged("alpha", exit_code=100)
        beta = self.staged("beta")
        directory = self.stage(alpha, beta)
        self.report(directory, alpha, failures=["boom"])
        self.report(directory, beta)
        prior = local_evidence.record_cells(
            str(self.plan_path),
            str(directory),
            "macos-latest",
            self.base,
            self.head,
            report_dir=str(directory),
        )
        prior_path = self.root / "prior-receipt.json"
        prior_path.write_text(prior, encoding="utf-8")

        rerun = self.staged("alpha", duration_s=7)
        self.stage(rerun)
        self.report(directory, rerun)
        merged = json.loads(
            local_evidence.record_cells(
                str(self.plan_path),
                str(directory),
                "macos-latest",
                self.base,
                self.head,
                report_dir=str(directory),
                prior_receipt_path=str(prior_path),
            )
        )

        cells = {cell["package"]: cell for cell in merged["cells"]}
        self.assertEqual(["alpha", "beta"], sorted(cells))
        self.assertEqual("pass", cells["alpha"]["outcome"])
        self.assertEqual(7, cells["alpha"]["duration_s"])
        self.assertEqual("pass", cells["beta"]["outcome"])

    def test_a_complete_failure_is_published_for_diagnosis(self) -> None:
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
        self.assertEqual([], accepted)
        self.assertEqual(1, len(rejections), rejections)
        self.assertTrue(rejections[0].startswith("failed-cell: alpha/macos-latest/L1"), rejections)

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

    def test_an_advanced_base_can_record_measured_cells(self) -> None:
        record = self.staged("alpha")
        directory = self.stage(record)
        self.report(directory, record)
        self.git("checkout", "-q", "-b", "advanced", self.base)
        self.git("commit", "-q", "--allow-empty", "-m", "advance")
        advanced = self.git("rev-parse", "HEAD")
        plan = json.loads(self.plan_path.read_text(encoding="utf-8"))
        plan["base"] = advanced
        self.plan_path.write_text(schema.canonical(plan), encoding="utf-8")
        receipt = json.loads(local_evidence.record_cells(
            str(self.plan_path), str(directory), "macos-latest", advanced, self.head,
            report_dir=str(directory),
        ))
        self.assertEqual(advanced, receipt["base"])
        self.assertEqual("pass", receipt["cells"][0]["outcome"])


class BackendProofTests(EvidenceFixture):
    """A Level 2 cell is evidence only if a backend actually drove a test.

    An installed backend plus zero backend tests is not evidence: a Level 2
    suite whose backend is absent *skips*, and nextest prints PASS in about
    0.02 s. These fixtures pin the receipt's refusal to call that a tested cell.
    """

    def l2_plan_cells(self) -> None:
        plan = json.loads(self.plan_path.read_text(encoding="utf-8"))
        # An executing L2 cell names the backends its producer must prove
        # (plan schema 6): here the one tmux the package declares.
        plan["cells"].append({**self.plan_cell("alpha", "macos-latest", "L2"), "backends": ["tmux"]})
        for entry in plan["packages"]:
            if entry["package"] == "alpha":
                entry["gates"] = ["L1", "L2"]
                entry["tiers"] = ["L1", "L2"]
                entry["l2_backends"] = ["tmux"]
        plan["job_estimate"] = len(plan["cells"])
        # The added L2 cell executes, so it owes a build: the macOS record it
        # shares with alpha's own L1 cell on that producer.
        plan_fixtures.finalize_plan(plan)
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

    def test_a_verified_archive_run_records_the_build_it_actually_ran(self) -> None:
        """Task 6.3: an exact-tree receipt names the build key and digest.

        The archive is the program the cell tested; without the key and the
        producer's realized digest the receipt says only that *something*
        passed on that tree.
        """
        contents = self.publish_cross_check(
            build_key="0f1e2d3c4b5a6978", build_digest="9a8b7c6d5e4f3021"
        )
        document = json.loads(contents)
        self.assertEqual(
            {"key": "0f1e2d3c4b5a6978", "digest": "9a8b7c6d5e4f3021"},
            document["cells"][0]["build"],
        )
        self.assertEqual([], schema.validate_receipt(document))
        # And it is still ordinary evidence: naming a build changes nothing
        # about which cell it satisfies.
        self.publish(contents, environment="wsl2-ubuntu")
        accepted, rejections = target("verify_cells")(
            str(self.plan_path), self.base, self.head
        )
        self.assertEqual(["alpha/wsl2-ubuntu/L1"], self.keys(accepted))
        self.assertEqual([], rejections)

    def test_a_run_that_named_no_archive_records_no_build(self) -> None:
        # The native path — and every receipt written before archives existed —
        # must stay valid rather than carry a half-named build.
        document = json.loads(self.publish_cross_check(build_key="0f1e2d3c4b5a6978"))
        self.assertNotIn("build", document["cells"][0])
        self.assertEqual([], schema.validate_receipt(document))

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

    def test_orchestration_files_move_no_gate_identity(self) -> None:
        # A workflow decides what CI runs, never what a local gate produces,
        # so editing one leaves every published cell reusable
        # (fixes/2026-09-18-ci-cadence, decision 5). Measured on the branch
        # that motivated it: one workflow edit invalidated 38 cells and cost
        # a 45-minute pre-push.
        orchestration = {
            ".github/ci/environments.json",
            ".github/workflows/_package-ci.yml",
            ".github/workflows/_wsl-ci.yml",
            ".github/workflows/ci.yml",
            "scripts/ci/affected_scope.py",
            ".github/actions",
        }
        for gate in schema.GATES:
            with self.subTest(gate=gate):
                inputs = set(local_evidence.gate_global_inputs(gate))
                self.assertEqual(set(), inputs & orchestration)
                # And the inputs that DO change a local gate's product stay.
                self.assertLessEqual({"Cargo.lock", "rust-toolchain.toml"}, inputs)
                # The Just files are read by recipe (`JustRecipeIdentityTests`),
                # never as whole paths: as paths they made every edit under
                # `just/` invalidate every published cell.
                self.assertEqual(set(), inputs & {"justfile", "just"})

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


class ObservationFixture(EvidenceFixture):
    """Version-3 observation collections, built the way a producer writes them."""

    def retained_dir(self, name: str = "retained") -> Path:
        directory = self.root / name
        directory.mkdir(exist_ok=True)
        return directory

    def proof_for(self, directory: Path, package: str, gate: str, outcome: str) -> dict:
        """The proof a rig writes, with the two documents it cites retained."""
        listing = f'{{"package": "{package}", "run": "{directory.name}", "tests": 4}}'
        record = json.dumps(
            {"package": package, "gate": gate, "complete": True, "run": directory.name}
        )
        names = (
            schema.PROOF_EXPECTED_FILE.format(package=package, gate=gate),
            schema.PROOF_COMPLETION_FILE.format(package=package, gate=gate),
        )
        (directory / names[0]).write_text(listing, encoding="utf-8")
        proof = {
            "nextest_version": "0.9.99",
            "target": "aarch64-apple-darwin",
            "expected_listing_digest": hashlib.sha256(listing.encode()).hexdigest(),
        }
        if outcome == "pass":
            (directory / names[1]).write_text(record, encoding="utf-8")
            proof["completion_record_digest"] = hashlib.sha256(record.encode()).hexdigest()
        return proof

    def observation(
        self,
        package: str,
        gate: str = "L1",
        *,
        outcome: str = "pass",
        kind: str = "hook",
        retry_of: str | None = None,
        report_dir: Path | None = None,
        store: str = "host-dir",
        write_report: bool = True,
        **cell_overrides: object,
    ) -> dict:
        directory = report_dir or self.retained_dir()
        cell = self.receipt_cell(package, gate, outcome=outcome, **cell_overrides)
        cell["report"] = cell_overrides.get(
            "report", f"{package}-{gate}-{outcome}-{cell['duration_s']}.xml"
        )
        if outcome == "fail":
            cell.setdefault("failed_tests", ["suite::boom"])
            cell["exit_code"] = 100
            cell["counts"] = {"total": 4, "passed": 3, "failed": 1, "skipped": 0, "errored": 0}
        report = directory / cell["report"]
        if write_report:
            report.write_text(f"<testsuites>{package}{outcome}{cell['duration_s']}</testsuites>")
        digest, size = (
            observation_notes.file_digest(report) if report.is_file() else ("0" * 64, 0)
        )
        document = {
            "producer": {
                "kind": kind,
                "host": {"os": "Darwin", "kernel": "Darwin 27", "report_dir": str(directory)},
            },
            "base": self.base,
            "scope_identity": "00112233445566778899aabb",
            "completion": "complete",
            "cell": cell,
            "reports": [
                {"path": cell["report"], "sha256": digest, "bytes": size, "store": store}
            ],
        }
        if kind != "hook":
            document["proof"] = self.proof_for(directory, package, gate, outcome)
        if retry_of:
            document["retry_of"] = retry_of
        return observation_notes.with_identity(document)

    def collection(
        self, observations: list[dict], environment: str = "macos-latest", commit: str | None = None
    ) -> dict:
        commit = commit or self.head
        return {
            "schema_version": schema.COLLECTION_SCHEMA_VERSION,
            "environment": environment,
            "head": commit,
            "tree": self.git("rev-parse", f"{commit}^{{tree}}"),
            "observations": sorted(observations, key=lambda o: o["observation_id"]),
        }

    def verify(self, base: str | None = None, head: str | None = None, **kwargs):
        return local_evidence.verify_cells(
            str(self.plan_path), base or self.base, head or self.head, **kwargs
        )

    def accepted_keys(self, **kwargs) -> list[str]:
        accepted, _ = self.verify(**kwargs)
        return self.keys(accepted)


class ObservationCollectionTests(ObservationFixture):
    def test_a_collection_credits_every_package_it_holds(self) -> None:
        self.add_note(
            "macos-latest",
            self.collection([self.observation("alpha"), self.observation("beta")]),
        )
        accepted, rejections = self.verify()
        self.assertEqual(
            ["alpha/macos-latest/L1", "beta/macos-latest/L1"], self.keys(accepted)
        )
        self.assertEqual([], rejections)
        self.assertTrue(all("observation" in cell["evidence"] for cell in accepted))
        self.assertTrue(all(cell["origin"] == "local" for cell in accepted))

    def test_the_coverage_boundary_commit_is_a_candidate(self) -> None:
        # Beta's inputs never changed between the base and the head, so a pass
        # recorded ON the base commit still describes the head.
        self.add_note(
            "macos-latest",
            self.collection([self.observation("beta")], commit=self.base),
            self.base,
        )
        accepted, _ = self.verify()
        self.assertEqual(["beta/macos-latest/L1"], self.keys(accepted))
        self.assertEqual("prior-local", accepted[0]["origin"])

    def test_a_changed_gate_input_on_the_boundary_is_refused(self) -> None:
        self.add_note(
            "macos-latest",
            self.collection([self.observation("alpha")], commit=self.base),
            self.base,
        )
        accepted, rejections = self.verify()
        self.assertEqual([], accepted)
        self.assertTrue(any(r.startswith("gate-inputs-changed") for r in rejections))

    def test_a_newer_failure_beats_an_older_pass_of_equivalent_inputs(self) -> None:
        self.add_note(
            "macos-latest",
            self.collection([self.observation("beta")], commit=self.base),
            self.base,
        )
        self.add_note(
            "macos-latest",
            self.collection([self.observation("beta", outcome="fail")]),
        )
        accepted, rejections = self.verify()
        self.assertEqual([], accepted)
        self.assertTrue(any(r.startswith("failed-cell") for r in rejections))

    def test_a_later_passing_commit_restores_reuse_over_an_older_failure(self) -> None:
        self.add_note(
            "macos-latest",
            self.collection([self.observation("beta", outcome="fail")], commit=self.base),
            self.base,
        )
        self.add_note("macos-latest", self.collection([self.observation("beta")]))
        self.assertEqual(["beta/macos-latest/L1"], self.accepted_keys())

    def test_a_retry_that_passes_supersedes_the_failure_it_names(self) -> None:
        failed = self.observation("alpha", outcome="fail")
        passed = self.observation("alpha", retry_of=failed["observation_id"])
        self.add_note("macos-latest", self.collection([failed, passed]))
        self.assertEqual(["alpha/macos-latest/L1"], self.accepted_keys())

    def test_a_retry_that_fails_supersedes_the_pass_it_names(self) -> None:
        passed = self.observation("alpha")
        failed = self.observation("alpha", outcome="fail", retry_of=passed["observation_id"])
        self.add_note("macos-latest", self.collection([passed, failed]))
        accepted, _ = self.verify()
        self.assertEqual([], accepted)

    def test_unlinked_disagreeing_observations_on_one_commit_conflict(self) -> None:
        self.add_note(
            "macos-latest",
            self.collection(
                [self.observation("alpha"), self.observation("alpha", outcome="fail")]
            ),
        )
        accepted, rejections = self.verify()
        self.assertEqual([], accepted)
        self.assertTrue(any(r.startswith("conflicting-evidence") for r in rejections))

    def test_unlinked_agreeing_passes_credit_the_cell_once(self) -> None:
        self.add_note(
            "macos-latest",
            self.collection(
                [self.observation("alpha"), self.observation("alpha", duration_s=99)]
            ),
        )
        self.assertEqual(["alpha/macos-latest/L1"], self.accepted_keys())

    def test_results_on_unordered_commits_conflict(self) -> None:
        # Two parents of a merge: neither descends from the other.
        self.git("checkout", "-q", "-b", "side", self.base)
        self.write("docs/side.md", "side\n")
        self.git("add", "-A")
        self.git("commit", "-q", "-m", "side")
        side = self.git("rev-parse", "HEAD")
        self.git("checkout", "-q", "main")
        self.git("merge", "-q", "--no-ff", "-m", "merge", "side")
        merged = self.git("rev-parse", "HEAD")
        self.add_note("macos-latest", self.collection([self.observation("beta")], commit=self.head), self.head)
        self.add_note(
            "macos-latest",
            self.collection([self.observation("beta", outcome="fail")], commit=side),
            side,
        )
        accepted, rejections = self.verify(head=merged)
        self.assertEqual([], accepted)
        self.assertTrue(any(r.startswith("conflicting-evidence") for r in rejections))

    def test_an_incomplete_observation_neither_credits_nor_supersedes(self) -> None:
        good = self.observation("alpha")
        partial = self.observation("alpha", completion="partial")
        interrupted = {**self.observation("beta"), "completion": "interrupted"}
        interrupted = observation_notes.with_identity(
            {k: v for k, v in interrupted.items() if k != "observation_id"}
        )
        self.add_note(
            "macos-latest", self.collection([good, partial, interrupted])
        )
        accepted, rejections = self.verify()
        self.assertEqual(["alpha/macos-latest/L1"], self.keys(accepted))
        self.assertTrue(any(r.startswith("incomplete-run") for r in rejections))

    def test_a_dangling_retry_link_rejects_the_whole_collection(self) -> None:
        orphan = self.observation("alpha", retry_of="ab" * 32)
        self.add_note("macos-latest", self.collection([orphan]))
        accepted, rejections = self.verify()
        self.assertEqual([], accepted)
        self.assertTrue(any(r.startswith("observation-link-invalid") for r in rejections))

    def test_a_retry_across_cells_is_rejected(self) -> None:
        first = self.observation("alpha")
        crossing = self.observation("beta", retry_of=first["observation_id"])
        self.add_note("macos-latest", self.collection([first, crossing]))
        accepted, rejections = self.verify()
        self.assertEqual([], accepted)
        self.assertTrue(any(r.startswith("observation-link-invalid") for r in rejections))

    def test_an_id_that_is_not_the_identity_of_its_content_is_rejected(self) -> None:
        forged = {**self.observation("alpha"), "observation_id": "ab" * 32}
        self.add_note("macos-latest", self.collection([forged]))
        accepted, rejections = self.verify()
        self.assertEqual([], accepted)
        self.assertTrue(any(r.startswith("observation-id-mismatch") for r in rejections))

    def test_a_note_filed_under_the_wrong_environment_is_credited_to_neither(self) -> None:
        self.add_note(
            "ubuntu-latest",
            self.collection([self.observation("alpha")], environment="macos-latest"),
        )
        accepted, rejections = self.verify()
        self.assertEqual([], accepted)
        self.assertTrue(any(r.startswith("environment-mismatch") for r in rejections))


class ObservationReportTests(ObservationFixture):
    def test_missing_report_bytes_reject_a_pass(self) -> None:
        directory = self.retained_dir()
        observation = self.observation("alpha", report_dir=directory)
        (directory / observation["cell"]["report"]).unlink()
        self.add_note("macos-latest", self.collection([observation]))
        accepted, rejections = self.verify()
        self.assertEqual([], accepted)
        self.assertTrue(any(r.startswith("observation-report-unavailable") for r in rejections))

    def test_altered_report_bytes_reject_a_pass(self) -> None:
        directory = self.retained_dir()
        observation = self.observation("alpha", report_dir=directory)
        (directory / observation["cell"]["report"]).write_text("tampered")
        self.add_note("macos-latest", self.collection([observation]))
        accepted, rejections = self.verify()
        self.assertEqual([], accepted)
        self.assertTrue(any("differ from the recorded digest" in r for r in rejections))

    def test_a_directory_on_another_host_is_accepted_with_provenance(self) -> None:
        observation = self.observation("alpha")
        observation["producer"]["host"]["report_dir"] = str(self.root / "other-host" / "evidence")
        observation = observation_notes.with_identity(
            {k: v for k, v in observation.items() if k != "observation_id"}
        )
        self.add_note("macos-latest", self.collection([observation]))
        accepted, _ = self.verify()
        self.assertEqual(["alpha/macos-latest/L1"], self.keys(accepted))
        self.assertEqual(str(self.root / "other-host" / "evidence"),
                         accepted[0]["evidence"]["host"]["report_dir"])

    def test_an_artifact_reference_needs_a_probe_that_finds_it(self) -> None:
        observation = self.observation("alpha", store="artifact")
        self.add_note("macos-latest", self.collection([observation]))
        with self.subTest("no probe"):
            accepted, rejections = self.verify()
            self.assertEqual([], accepted)
            self.assertTrue(any("cannot be retrieved" in r for r in rejections))
        with self.subTest("expired"):
            self.assertEqual([], self.accepted_keys(report_probe=lambda report, obs: False))
        with self.subTest("retained"):
            self.assertEqual(
                ["alpha/macos-latest/L1"],
                self.accepted_keys(report_probe=lambda report, obs: True),
            )

    def test_a_rig_pass_is_rechecked_against_the_documents_its_proof_cites(self) -> None:
        for what, name in (("listing", schema.PROOF_EXPECTED_FILE), ("record", schema.PROOF_COMPLETION_FILE)):
            for damage in ("missing", "altered"):
                with self.subTest(what=what, damage=damage):
                    directory = self.retained_dir(f"{what}-{damage}")
                    observation = self.observation("alpha", kind="rig", report_dir=directory)
                    cited = directory / name.format(package="alpha", gate="L1")
                    if damage == "missing":
                        cited.unlink()
                    else:
                        cited.write_text("tampered")
                    self.add_note("macos-latest", self.collection([observation]))
                    accepted, rejections = self.verify()
                    self.assertEqual([], accepted)
                    self.assertTrue(
                        any(r.startswith("observation-report-unavailable") for r in rejections),
                        rejections,
                    )

    def test_a_record_that_certifies_another_cell_is_not_proof(self) -> None:
        directory = self.retained_dir()
        observation = self.observation("alpha", kind="rig", report_dir=directory)
        record = directory / schema.PROOF_COMPLETION_FILE.format(package="alpha", gate="L1")
        record.write_text('{"package": "beta", "gate": "L1", "complete": true}')
        document = {k: v for k, v in observation.items() if k != "observation_id"}
        document["proof"] = {
            **document["proof"],
            "completion_record_digest": hashlib.sha256(record.read_bytes()).hexdigest(),
        }
        self.add_note("macos-latest", self.collection([observation_notes.with_identity(document)]))
        accepted, rejections = self.verify()
        self.assertEqual([], accepted)
        self.assertTrue(any("does not certify" in r for r in rejections), rejections)

    def assert_rig_failure_blocks(self, with_proof: bool) -> None:
        failed = self.observation("beta", kind="rig", outcome="fail")
        if not with_proof:
            failed = observation_notes.with_identity(
                {k: v for k, v in failed.items() if k not in ("observation_id", "proof")}
            )
        self.add_note(
            "macos-latest",
            self.collection([self.observation("beta")], commit=self.base),
            self.base,
        )
        self.add_note("macos-latest", self.collection([failed]))
        self.assertEqual([], schema.validate_collection(self.collection([failed])))
        self.assertEqual([], self.accepted_keys())

    def test_a_rig_failure_with_a_listing_but_no_record_blocks_an_older_pass(self) -> None:
        self.assert_rig_failure_blocks(with_proof=True)

    def test_a_rig_failure_with_no_proof_blocks_an_older_pass(self) -> None:
        self.assert_rig_failure_blocks(with_proof=False)

    def test_a_passing_rig_observation_without_a_record_or_proof_is_invalid(self) -> None:
        passed = self.observation("alpha", kind="rig")
        for what in ("record", "proof"):
            with self.subTest(what=what):
                body = {k: v for k, v in passed.items() if k != "observation_id"}
                if what == "proof":
                    del body["proof"]
                else:
                    body["proof"] = {
                        k: v for k, v in body["proof"].items() if k != "completion_record_digest"
                    }
                problems = schema.validate_collection(
                    self.collection([observation_notes.with_identity(body)])
                )
                self.assertTrue(
                    problems and problems[0].startswith("observation-proof-missing"), problems
                )

    def test_a_failure_whose_reports_are_gone_still_blocks_an_older_pass(self) -> None:
        directory = self.retained_dir()
        failed = self.observation("beta", outcome="fail", report_dir=directory)
        (directory / failed["cell"]["report"]).unlink()
        self.add_note(
            "macos-latest",
            self.collection([self.observation("beta")], commit=self.base),
            self.base,
        )
        self.add_note("macos-latest", self.collection([failed]))
        self.assertEqual([], self.accepted_keys())

    def test_a_complete_cell_must_cite_its_report(self) -> None:
        observation = self.observation("alpha")
        observation["reports"] = []
        observation = observation_notes.with_identity(
            {k: v for k, v in observation.items() if k != "observation_id"}
        )
        self.add_note("macos-latest", self.collection([observation]))
        accepted, rejections = self.verify()
        self.assertEqual([], accepted)
        self.assertTrue(any(r.startswith("observation-report-unavailable") for r in rejections))

    def test_features_other_than_the_plans_are_refused(self) -> None:
        plain = self.observation("alpha")
        featured = observation_notes.with_identity(
            {**{k: v for k, v in self.observation("beta").items() if k != "observation_id"},
             "features": "--features extra"}
        )
        self.add_note("macos-latest", self.collection([plain, featured]))
        accepted, rejections = self.verify()
        self.assertEqual(["alpha/macos-latest/L1"], self.keys(accepted))
        self.assertTrue(any("ran with features" in r for r in rejections))

    def test_an_observation_that_misses_a_planned_backend_is_refused(self) -> None:
        observation = self.observation("alpha")
        plan = json.loads(self.plan_path.read_text(encoding="utf-8"))
        planned = {**plan["cells"][0], "backends": ["tmux"]}
        self.assertIn(
            "did not prove backend(s) ['tmux']",
            local_evidence._observation_refusal(
                observation, ("alpha", "macos-latest", "L1"), {"cells": [planned], "builds": []},
                "alpha/macos-latest/L1",
            ),
        )
        observation = self.observation("alpha", backends=["tmux"])
        self.assertIsNone(
            local_evidence._observation_refusal(
                observation, ("alpha", "macos-latest", "L1"), {"cells": [planned], "builds": []},
                "alpha/macos-latest/L1",
            )
        )


class BootstrapDiscoveryTests(ObservationFixture):
    ZERO = "0" * 40

    def test_an_all_zero_base_reads_the_head_note(self) -> None:
        self.add_note("macos-latest", self.collection([self.observation("alpha")]))
        accepted, rejections = self.verify(base=self.ZERO)
        self.assertEqual(["alpha/macos-latest/L1"], self.keys(accepted))
        self.assertEqual([], rejections)

    def test_an_all_zero_base_finds_equivalent_proof_on_an_ancestor(self) -> None:
        self.add_note(
            "macos-latest",
            self.collection([self.observation("beta")], commit=self.base),
            self.base,
        )
        self.assertEqual(["beta/macos-latest/L1"], self.accepted_keys(base=self.ZERO))

    def test_an_all_zero_base_ignores_proof_that_is_not_an_ancestor(self) -> None:
        self.git("checkout", "-q", "-b", "elsewhere", self.base)
        self.write("docs/elsewhere.md", "x\n")
        self.git("add", "-A")
        self.git("commit", "-q", "-m", "elsewhere")
        stranger = self.git("rev-parse", "HEAD")
        self.git("checkout", "-q", "main")
        self.add_note(
            "macos-latest",
            self.collection([self.observation("beta")], commit=stranger),
            stranger,
        )
        accepted, _ = self.verify(base=self.ZERO)
        self.assertEqual([], accepted)

    def test_an_all_zero_base_needs_no_merge_base(self) -> None:
        calls: list[tuple[str, ...]] = []
        original = local_evidence.git

        def spy(*args: str) -> str:
            calls.append(args)
            return original(*args)

        local_evidence.git = spy
        try:
            self.verify(base=self.ZERO)
        finally:
            local_evidence.git = original
        self.assertFalse([call for call in calls if call[0] == "merge-base"])

    def test_the_search_is_bounded_by_the_notes_that_exist(self) -> None:
        for index in range(3):
            self.write("docs/unrelated.md", f"edit {index}\n")
            self.git("commit", "-q", "-am", f"edit {index}")
        later = self.git("rev-parse", "HEAD")
        self.assertEqual([later, self.head], local_evidence.candidate_commits(self.ZERO, later)[:1] + [self.head][:1])
        self.add_note(
            "macos-latest", self.collection([self.observation("beta")], commit=self.base), self.base
        )
        commits = local_evidence.candidate_commits(self.ZERO, later)
        self.assertEqual([later, self.base], commits)


class StrictDocumentTests(ObservationFixture):
    def assert_nothing_accepted(self, text: str) -> None:
        self.add_note("macos-latest", text)
        accepted, rejections = self.verify()
        self.assertEqual([], accepted)
        self.assertTrue(any(r.startswith("malformed-receipt") for r in rejections), rejections)

    def test_a_duplicate_key_is_refused(self) -> None:
        text = schema.canonical(self.collection([self.observation("alpha")]))
        self.assert_nothing_accepted(text.replace('"environment":', '"environment":"x","environment":', 1))

    def test_trailing_content_is_refused(self) -> None:
        self.assert_nothing_accepted(
            schema.canonical(self.collection([self.observation("alpha")])) + "\ngarbage"
        )

    def test_a_non_finite_number_is_refused(self) -> None:
        text = schema.canonical(self.collection([self.observation("alpha")]))
        self.assert_nothing_accepted(text.replace('"duration_s":12', '"duration_s":NaN', 1))

    def test_a_boolean_schema_version_is_not_version_one(self) -> None:
        document = self.receipt("macos-latest", [self.receipt_cell("alpha")], schema_version=True)
        self.add_note("macos-latest", document)
        accepted, rejections = self.verify()
        self.assertEqual([], accepted)
        self.assertTrue(any(r.startswith("unknown-schema-version") for r in rejections))


def _dumps_with_duplicate(value, path, key):
    """`value` as JSON text with `key` written twice in the object at `path`."""
    if isinstance(value, dict):
        pairs = []
        if not path:
            pairs.append(f"{json.dumps(key)}:{json.dumps(value[key])}")
        for name, item in value.items():
            pairs.append(f"{json.dumps(name)}:{_dumps_with_duplicate(item, path[1:] if path and path[0] == name else None, key) if path and path[0] == name else json.dumps(item)}")
        return "{" + ",".join(pairs) + "}"
    if isinstance(value, list):
        index = path[0] if path else None
        return "[" + ",".join(
            _dumps_with_duplicate(item, path[1:], key) if position == index else json.dumps(item)
            for position, item in enumerate(value)
        ) + "]"
    return json.dumps(value)


class ObservationMatrixTests(ObservationFixture):
    """The input robustness matrix, walked through `verify_cells`.

    One edit per cell of the matrix, against unedited real-shape controls. The
    public decision is whether the edited cell is credited. `R` rows must
    leave the cell scheduled AND say why; `D` and `E` rows must still credit
    (the field has a defined absent or empty meaning).
    """

    ABSENT, NULL, WHOLE, ONE, EVERY, EMPTY, DUPLICATE = (
        "absent", "null", "whole-type", "one-element", "every-element", "empty", "duplicate-key",
    )

    def setUp(self) -> None:
        super().setUp()
        self.directory = self.retained_dir()

    def control(self, kind: str = "hook") -> dict:
        return self.collection([self.observation("alpha", kind=kind, report_dir=self.directory)])

    def publish_text(self, text: str) -> None:
        self.git("notes", "--ref", f"{NOTES_PREFIX}/macos-latest", "remove", "--ignore-missing", self.head)
        self.add_note("macos-latest", text)

    def credited(self, text: str) -> tuple[bool, list[str]]:
        self.publish_text(text)
        accepted, rejections = self.verify()
        return bool(accepted), rejections

    def get(self, document, path):
        for step in path:
            document = document[step]
        return document

    def reseal(self, document: dict) -> dict:
        """Recompute every id after an edit, so the edit and not the id is tested."""
        for index, observation in enumerate(document["observations"]):
            if isinstance(observation, dict) and "observation_id" in observation:
                body = {k: v for k, v in observation.items() if k != "observation_id"}
                document["observations"][index] = observation_notes.with_identity(body)
        return document

    def edit(self, kind: str, path: tuple, shape: str):
        """The control with `path` edited to `shape`, or `None` if the shape does not apply."""
        document = self.control(kind)
        parent = self.get(document, path[:-1])
        key = path[-1]
        current = parent[key] if (isinstance(parent, dict) and key in parent) or isinstance(parent, list) else None
        if shape == self.ABSENT:
            if not isinstance(parent, dict) or key not in parent:
                return None
            del parent[key]
        elif shape == self.NULL:
            parent[key] = None
        elif shape == self.WHOLE:
            parent[key] = 123 if not isinstance(current, int) or isinstance(current, bool) else "wrong"
        elif shape == self.ONE:
            if not isinstance(current, list) or not current:
                return None
            parent[key] = [*current[:-1], 123]
        elif shape == self.EVERY:
            if not isinstance(current, list) or not current:
                return None
            parent[key] = [123 for _ in current]
        elif shape == self.EMPTY:
            parent[key] = [] if isinstance(current, list) else {} if isinstance(current, dict) else ""
        elif shape == self.DUPLICATE:
            return None
        return document

    def walk(self, kind: str, fields: dict[tuple, dict[str, str]]) -> None:
        control_credited, control_rejections = self.credited(schema.canonical(self.control(kind)))
        self.assertTrue(control_credited, control_rejections)
        for path, outcomes in fields.items():
            for shape in (self.ABSENT, self.NULL, self.WHOLE, self.ONE, self.EVERY, self.EMPTY):
                expected = outcomes.get(shape)
                if expected is None:
                    continue
                with self.subTest(field="/".join(map(str, path)), shape=shape):
                    document = self.edit(kind, path, shape)
                    if document is None:
                        # A field the control does not carry has no absent edit:
                        # the control already IS the absent case.
                        self.assertEqual("D", expected)
                        continue
                    if path[0] == "observations":
                        document = self.reseal(document)
                    credited, rejections = self.credited(schema.canonical(document))
                    if expected == "R":
                        self.assertFalse(credited, "an edited field must not earn credit")
                        self.assertTrue(rejections, "a refusal must say why")
                    else:
                        self.assertTrue(credited, rejections)
            with self.subTest(field="/".join(map(str, path)), shape=self.DUPLICATE):
                document = self.control(kind)
                parent_path = path[:-1]
                parent = self.get(document, parent_path)
                if path[-1] not in parent:
                    continue
                text = _dumps_with_duplicate(document, tuple(parent_path), path[-1])
                credited, rejections = self.credited(text)
                self.assertFalse(credited)
                self.assertTrue(rejections)

    R = "R"
    ALL_REJECT = {"absent": "R", "null": "R", "whole-type": "R"}

    def test_collection_envelope(self) -> None:
        envelope = {
            ("schema_version",): {**self.ALL_REJECT, "empty": "R"},
            ("environment",): {**self.ALL_REJECT, "empty": "R"},
            ("head",): {**self.ALL_REJECT, "empty": "R"},
            ("tree",): {**self.ALL_REJECT, "empty": "R"},
            ("observations",): {**self.ALL_REJECT, "one-element": None, "empty": "E"},
        }
        self.walk("hook", {k: v for k, v in envelope.items() if k != ("observations",)})
        # An empty collection is a real "zero proof": valid, and credits nothing.
        empty = self.control("hook")
        empty["observations"] = []
        self.publish_text(schema.canonical(empty))
        accepted, rejections = self.verify()
        self.assertEqual(([], []), (accepted, rejections))
        for shape in ("absent", "null", "whole-type"):
            document = self.edit("hook", ("observations",), shape)
            credited, rejections = self.credited(schema.canonical(document))
            self.assertFalse(credited, shape)
            self.assertTrue(rejections, shape)

    def test_observation_fields(self) -> None:
        base = ("observations", 0)
        fields = {
            (*base, "observation_id"): {**self.ALL_REJECT, "empty": "R"},
            (*base, "retry_of"): {"absent": "D", "null": "R", "whole-type": "R", "empty": "R"},
            (*base, "producer"): {**self.ALL_REJECT, "empty": "R"},
            (*base, "base"): {**self.ALL_REJECT, "empty": "R"},
            (*base, "scope_identity"): {**self.ALL_REJECT, "empty": "R"},
            (*base, "completion"): {**self.ALL_REJECT, "empty": "R"},
            (*base, "cell"): {**self.ALL_REJECT, "empty": "R"},
            (*base, "reports"): {**self.ALL_REJECT, "one-element": "R", "every-element": "R", "empty": "R"},
            (*base, "features"): {"absent": "D", "null": "R", "whole-type": "R"},
        }
        # retry_of / observation_id edits are tested on their own below: the
        # reseal step would otherwise repair a deliberately wrong id.
        controllable = {k: v for k, v in fields.items() if k[-1] not in ("observation_id", "retry_of")}
        self.walk("hook", controllable)
        for shape in ("null", "whole-type", "empty"):
            document = self.control("hook")
            observation = document["observations"][0]
            observation["retry_of"] = None if shape == "null" else 123 if shape == "whole-type" else ""
            document = self.reseal(document)
            credited, rejections = self.credited(schema.canonical(document))
            self.assertFalse(credited, shape)
            self.assertTrue(rejections, shape)
        for shape in ("absent", "null", "whole-type", "empty"):
            document = self.control("hook")
            observation = document["observations"][0]
            if shape == "absent":
                del observation["observation_id"]
            else:
                observation["observation_id"] = None if shape == "null" else 123 if shape == "whole-type" else ""
            credited, rejections = self.credited(schema.canonical(document))
            self.assertFalse(credited, shape)
            self.assertTrue(rejections, shape)

    def test_producer_and_proof_fields(self) -> None:
        base = ("observations", 0)
        fields = {
            (*base, "producer", "kind"): {**self.ALL_REJECT, "empty": "R"},
            (*base, "producer", "host"): {**self.ALL_REJECT, "empty": "R"},
            (*base, "producer", "host", "os"): {**self.ALL_REJECT, "empty": "R"},
            (*base, "producer", "host", "kernel"): {**self.ALL_REJECT, "empty": "R"},
            (*base, "producer", "host", "report_dir"): {"absent": "R", "null": "R", "whole-type": "R"},
            (*base, "proof"): {**self.ALL_REJECT, "empty": "R"},
            (*base, "proof", "nextest_version"): {**self.ALL_REJECT, "empty": "R"},
            (*base, "proof", "target"): {**self.ALL_REJECT, "empty": "R"},
            (*base, "proof", "expected_listing_digest"): {**self.ALL_REJECT, "empty": "R"},
            (*base, "proof", "completion_record_digest"): {**self.ALL_REJECT, "empty": "R"},
        }
        self.walk("rig", fields)

    def test_a_hook_observation_carries_no_proof_and_the_others_must(self) -> None:
        hook = self.control("hook")
        hook["observations"][0]["proof"] = self.control("rig")["observations"][0]["proof"]
        credited, rejections = self.credited(schema.canonical(self.reseal(hook)))
        self.assertFalse(credited)
        self.assertTrue(rejections)

    def test_report_reference_fields(self) -> None:
        base = ("observations", 0, "reports", 0)
        fields = {
            (*base, "path"): {**self.ALL_REJECT, "empty": "R"},
            (*base, "sha256"): {**self.ALL_REJECT, "empty": "R"},
            (*base, "bytes"): {**self.ALL_REJECT, "empty": "R"},
            (*base, "store"): {**self.ALL_REJECT, "empty": "R"},
        }
        self.walk("hook", fields)

    def test_cell_fields(self) -> None:
        base = ("observations", 0, "cell")
        reject = {**self.ALL_REJECT, "empty": "R"}
        fields = {
            (*base, "package"): reject,
            (*base, "gate"): reject,
            (*base, "outcome"): reject,
            (*base, "exit_code"): reject,
            (*base, "completion"): reject,
            (*base, "counts"): {**reject, "one-element": None},
            (*base, "duration_s"): {"absent": "R", "null": "R", "whole-type": "R"},
            (*base, "gate_input_identity"): reject,
            (*base, "backends"): {"absent": "R", "null": "R", "whole-type": "R", "empty": "E"},
            (*base, "report"): reject,
            (*base, "failed_tests"): {"absent": "D", "null": "R", "whole-type": "R", "empty": "E"},
            (*base, "build"): {"absent": "D", "null": "R", "whole-type": "R"},
            (*base, "test_filter"): {"absent": "D", "null": "R", "whole-type": "R"},
        }
        # Edits to a cell that do not exist in the control need a baseline.
        control = self.control("hook")
        cell = control["observations"][0]["cell"]
        cell["failed_tests"] = []
        cell["build"] = {"key": "ab" * 8, "digest": "cd" * 8}
        cell["backends"] = ["tmux"]
        self.reseal(control)
        original = self.control
        self.control = lambda kind="hook": json.loads(json.dumps(control))
        try:
            self.walk("hook", fields)
        finally:
            self.control = original


class PublicationTests(ObservationFixture):
    """`observation_notes.publish` against a real bare remote and second clone."""

    def setUp(self) -> None:
        super().setUp()
        self.remote = self.root / "remote.git"
        subprocess.run(["git", "init", "-q", "--bare", str(self.remote)], check=True)
        self.git("remote", "add", "origin", str(self.remote))
        self.git("push", "-q", "origin", "main")
        self.clone = self.root / "clone"
        subprocess.run(
            ["git", "clone", "-q", str(self.remote), str(self.clone)], check=True
        )
        subprocess.run(["git", "-C", str(self.clone), "config", "user.name", "B"], check=True)
        subprocess.run(["git", "-C", str(self.clone), "config", "user.email", "b@example.com"], check=True)
        self.ref = f"{NOTES_PREFIX}/macos-latest"

    def publish_from(self, observations, *, repo: Path | None = None, **kwargs):
        return observation_notes.publish(
            "macos-latest", self.head, observations, repo=repo,
            sleep=lambda seconds: None, **kwargs,
        )

    def remote_ids(self, commit: str | None = None) -> list[str]:
        shown = subprocess.run(
            ["git", "--git-dir", str(self.remote), "notes", "--ref", self.ref, "show", commit or self.head],
            capture_output=True, text=True, check=True,
        ).stdout
        return [o["observation_id"] for o in json.loads(shown)["observations"]]

    def test_publication_lands_locally_and_on_the_remote(self) -> None:
        observation = self.observation("alpha")
        result = self.publish_from([observation])
        self.assertTrue(result.local and result.published, result.error)
        self.assertEqual(1, result.attempts)
        self.assertEqual([observation["observation_id"]], self.remote_ids())
        self.assertEqual(["alpha/macos-latest/L1"], self.accepted_keys())

    def test_sequential_packages_at_one_head_do_not_overwrite_each_other(self) -> None:
        alpha, beta = self.observation("alpha"), self.observation("beta")
        self.publish_from([alpha])
        self.publish_from([beta])
        self.assertEqual(
            sorted([alpha["observation_id"], beta["observation_id"]]), self.remote_ids()
        )
        self.assertEqual(
            ["alpha/macos-latest/L1", "beta/macos-latest/L1"], self.accepted_keys()
        )

    def test_republishing_one_observation_is_idempotent(self) -> None:
        observation = self.observation("alpha")
        self.publish_from([observation])
        before = self.git("notes", "--ref", self.ref, "show", self.head)
        result = self.publish_from([observation])
        self.assertTrue(result.published)
        self.assertEqual(before, self.git("notes", "--ref", self.ref, "show", self.head))

    def test_republishing_does_not_move_the_notes_ref(self) -> None:
        observation = self.observation("alpha")
        self.publish_from([observation])
        tip = self.git("rev-parse", self.ref)
        self.publish_from([observation])
        self.assertEqual(tip, self.git("rev-parse", self.ref))

    def kind_observation(self, kind: str, **options) -> dict:
        observation = self.observation("alpha", kind=kind, **options)
        if kind == "seed":
            producer = {**observation["producer"], "seed_id": "seed-1"}
            observation = observation_notes.with_identity({**observation, "producer": producer})
        return observation

    def forget_notes(self) -> None:
        subprocess.run(["git", "update-ref", "-d", self.ref], check=True)
        subprocess.run(["git", "--git-dir", str(self.remote), "update-ref", "-d", self.ref], check=True)

    def test_a_delayed_pass_published_after_a_newer_failure_is_not_its_retry(self) -> None:
        for kind in ("hook", "rig", "seed"):
            with self.subTest(kind):
                self.forget_notes()
                # The pass ran first; its publication was delayed past the failure's.
                passed = self.kind_observation(kind)
                failed = self.kind_observation(kind, outcome="fail", duration_s=33)
                self.publish_from([failed])
                self.publish_from([passed])
                collection = json.loads(self.git("notes", "--ref", self.ref, "show", self.head))
                self.assertEqual(
                    sorted([passed["observation_id"], failed["observation_id"]]),
                    sorted(o["observation_id"] for o in collection["observations"]),
                    "both are stored exactly as their producers wrote them",
                )
                self.assertEqual([], self.accepted_keys())
                _, rejections = self.verify()
                self.assertTrue(
                    [r for r in rejections if r.startswith("conflicting-evidence")], rejections
                )

    def test_a_retry_naming_its_predecessor_supersedes_it(self) -> None:
        for kind in ("hook", "rig", "seed"):
            with self.subTest(kind):
                self.forget_notes()
                failed = self.kind_observation(kind, outcome="fail")
                self.publish_from([failed])
                self.assertEqual([], self.accepted_keys())
                self.publish_from([self.kind_observation(
                    kind, duration_s=33, retry_of=failed["observation_id"])])
                self.assertEqual(["alpha/macos-latest/L1"], self.accepted_keys())

    def test_the_predecessor_is_the_cells_one_unsuperseded_observation_at_retry_start(self) -> None:
        predecessor = lambda: observation_notes.retry_predecessor(  # noqa: E731
            "macos-latest", self.head, "alpha", "L1")
        self.assertIsNone(predecessor(), "nothing to retry")
        first = self.observation("alpha", outcome="fail")
        self.publish_from([first, self.observation("beta")])
        self.assertEqual(first["observation_id"], predecessor())
        second = self.observation("alpha", duration_s=33, retry_of=first["observation_id"])
        self.publish_from([second])
        self.assertEqual(second["observation_id"], predecessor(), "the chain's head")
        self.publish_from([self.observation("alpha", outcome="fail", duration_s=34)])
        self.assertIsNone(predecessor(), "two unordered observations: neither is named")

    def test_a_second_host_unions_with_what_the_remote_already_holds(self) -> None:
        alpha, beta = self.observation("alpha"), self.observation("beta")
        self.publish_from([alpha])
        subprocess.run(["git", "-C", str(self.clone), "fetch", "-q", "origin", "main"], check=True)
        # The clone has never seen the first host's note.
        result = observation_notes.publish(
            "macos-latest", self.head, [beta], repo=self.clone, sleep=lambda s: None
        )
        self.assertTrue(result.published, result.error)
        self.assertEqual(
            sorted([alpha["observation_id"], beta["observation_id"]]), self.remote_ids()
        )

    def test_a_lost_race_is_retried_without_losing_the_winner(self) -> None:
        alpha, beta = self.observation("alpha"), self.observation("beta")
        winner = {"done": False}

        def other_host_wins(attempt: int) -> None:
            if not winner["done"]:
                winner["done"] = True
                subprocess.run(["git", "-C", str(self.clone), "fetch", "-q", "origin", "main"], check=True)
                observation_notes.publish(
                    "macos-latest", self.head, [beta], repo=self.clone, sleep=lambda s: None
                )

        # Seed the remote ref so this publisher's first push is non-fast-forward
        # after the other host moves it.
        self.publish_from([self.observation("beta", duration_s=1)])
        result = self.publish_from([alpha], before_push=other_host_wins)
        self.assertTrue(result.published, result.error)
        self.assertEqual(2, result.attempts)
        self.assertIn(alpha["observation_id"], self.remote_ids())
        self.assertIn(beta["observation_id"], self.remote_ids())

    def test_unrelated_commits_notes_survive_a_rebase_onto_the_remote(self) -> None:
        other = self.observation("beta")
        other_collection = self.collection([other], commit=self.base)
        subprocess.run(["git", "-C", str(self.clone), "fetch", "-q", "origin", "main"], check=True)
        observation_notes.publish(
            "macos-latest", self.base, other_collection["observations"],
            repo=self.clone, sleep=lambda s: None,
        )
        # This host holds an unpublished note of its own on another commit.
        unpublished = self.observation("beta", duration_s=5)
        observation_notes.publish(
            "macos-latest", self.head, [unpublished], remote=None, repo=None
        )
        result = self.publish_from([self.observation("alpha")])
        self.assertTrue(result.published, result.error)
        self.assertEqual([other["observation_id"]], self.remote_ids(self.base))
        self.assertIn(unpublished["observation_id"], self.remote_ids())

    def test_an_unreachable_remote_keeps_the_proof_local(self) -> None:
        self.git("remote", "set-url", "origin", str(self.root / "missing.git"))
        result = self.publish_from([self.observation("alpha")], attempts=2)
        self.assertTrue(result.local)
        self.assertFalse(result.published)
        self.assertIn("local only", result.error)
        self.assertEqual(["alpha/macos-latest/L1"], self.accepted_keys())

    def test_a_remote_that_keeps_winning_stops_after_the_bound(self) -> None:
        counter = {"n": 0}

        def always_lose(attempt: int) -> None:
            counter["n"] += 1
            subprocess.run(["git", "-C", str(self.clone), "fetch", "-q", "origin", "main"], check=True)
            observation_notes.publish(
                "macos-latest", self.head, [self.observation("beta", duration_s=100 + counter["n"])],
                repo=self.clone, sleep=lambda s: None,
            )

        self.publish_from([self.observation("beta", duration_s=1)])
        result = self.publish_from(
            [self.observation("alpha")], attempts=2, before_push=always_lose
        )
        self.assertFalse(result.published)
        self.assertEqual(2, result.attempts)
        self.assertIn("rejecting the push after 2 attempt", result.error)
        # The proof stays local, along with whatever the fetches merged in.
        self.assertEqual(
            ["alpha/macos-latest/L1", "beta/macos-latest/L1"], self.accepted_keys()
        )

    def test_an_oversized_collection_writes_nothing(self) -> None:
        observation = self.observation("alpha")
        observation["cell"]["failed_tests"] = []
        padded = dict(observation)
        padded["reports"] = [
            {**observation["reports"][0], "path": f"{n}-" + "x" * 200}
            for n in range(schema.COLLECTION_SIZE_LIMIT // 150)
        ]
        with self.assertRaises(observation_notes.PublicationError):
            self.publish_from([observation_notes.with_identity(
                {k: v for k, v in padded.items() if k != "observation_id"})])
        self.assertEqual(
            "", subprocess.run(["git", "notes", "--ref", self.ref, "list"],
                               capture_output=True, text=True).stdout.strip()
        )

    def test_the_same_id_with_different_content_is_refused(self) -> None:
        observation = self.observation("alpha")
        forged = {**observation, "cell": {**observation["cell"], "duration_s": 99}}
        with self.assertRaises(observation_notes.PublicationError):
            observation_notes.merge_observations([observation], [forged])


class RecorderPublicationTests(ObservationFixture):
    """The recorder's receipt, carried into a collection and read back."""

    def recorded_receipt(self) -> tuple[dict, Path]:
        record = self.staged("alpha")
        directory = self.stage(record)
        self.report(directory, record)
        retained = self.root / "evidence" / self.head / "macos-latest"
        shutil.copytree(directory, retained)
        document = json.loads(
            local_evidence.record_cells(
                str(self.plan_path), str(directory), "macos-latest", self.base, self.head,
                report_dir=str(retained),
            )
        )
        return document, retained

    def test_a_recorded_receipt_round_trips_through_the_collection(self) -> None:
        receipt, _ = self.recorded_receipt()
        result = observation_notes.publish(
            "macos-latest", self.head, observation_notes.receipt_observations(receipt),
            remote=None,
        )
        self.assertTrue(result.local)
        accepted, rejections = self.verify()
        self.assertEqual(["alpha/macos-latest/L1"], self.keys(accepted))
        self.assertEqual([], rejections)
        # read / write / read: a second publication of the same receipt changes nothing.
        before = self.git("notes", "--ref", f"{NOTES_PREFIX}/macos-latest", "show", self.head)
        observation_notes.publish(
            "macos-latest", self.head, observation_notes.receipt_observations(receipt), remote=None
        )
        self.assertEqual(
            before, self.git("notes", "--ref", f"{NOTES_PREFIX}/macos-latest", "show", self.head)
        )
        self.assertEqual(["alpha/macos-latest/L1"], self.accepted_keys())

    def test_an_existing_version_two_note_is_carried_not_overwritten(self) -> None:
        receipt, _ = self.recorded_receipt()
        self.add_note("macos-latest", receipt)
        beta = self.observation("beta")
        result = observation_notes.publish("macos-latest", self.head, [beta], remote=None)
        self.assertTrue(result.local)
        self.assertEqual(
            ["alpha/macos-latest/L1", "beta/macos-latest/L1"], self.accepted_keys()
        )

    def test_a_receipt_whose_reports_are_gone_publishes_nothing(self) -> None:
        receipt, retained = self.recorded_receipt()
        shutil.rmtree(retained)
        with self.assertRaises(observation_notes.PublicationError):
            observation_notes.receipt_observations(receipt)

    def test_the_publish_receipt_command_records_locally_without_a_remote(self) -> None:
        receipt, _ = self.recorded_receipt()
        path = self.root / "receipt.json"
        path.write_text(json.dumps(receipt))
        completed = subprocess.run(
            [sys.executable, str(Path(local_evidence.__file__).with_name("observation_notes.py")),
             "publish-receipt", "--receipt", str(path), "--plan", str(self.plan_path),
             "--head", self.head, "--environment", "macos-latest", "--no-push"],
            capture_output=True, text=True,
        )
        self.assertEqual(4, completed.returncode, completed.stderr)
        self.assertIn("local only", completed.stderr)
        self.assertEqual(["alpha/macos-latest/L1"], self.accepted_keys())

    def test_a_hook_receipt_is_published_only_with_the_plan_it_ran_under(self) -> None:
        receipt, _ = self.recorded_receipt()
        path = self.root / "receipt.json"
        path.write_text(json.dumps(receipt))
        completed = subprocess.run(
            [sys.executable, str(OBSERVATION_NOTES), "publish-receipt", "--receipt", str(path),
             "--head", self.head, "--environment", "macos-latest", "--no-push"],
            capture_output=True, text=True,
        )
        self.assertEqual(2, completed.returncode, completed.stderr)
        self.assertIn("--plan", completed.stderr)
        self.assertEqual([], self.accepted_keys())

    def test_a_hook_observation_carries_the_feature_arguments_its_package_ran_with(self) -> None:
        receipt, _ = self.recorded_receipt()
        plan = json.loads(self.plan_path.read_text(encoding="utf-8"))
        for package in plan["packages"]:
            if package["package"] == "alpha":
                package["test_args"] = "--features remote"
        observation, = observation_notes.receipt_observations(
            receipt, package_features=observation_notes.hook_features(plan)
        )
        self.assertEqual("--features remote", observation["features"])
        # The plan keys alpha's build on the same string, so the verifier
        # accepts the cell; an observation without it is refused, which every
        # hook observation was before it recorded its features.
        planned = {**plan["cells"][0], "build": "alpha-build"}
        featured_plan = {
            "cells": [planned],
            "builds": [{"key": "alpha-build", "identity": {"features": "--features remote"}}],
        }
        key = ("alpha", "macos-latest", "L1")
        self.assertIsNone(
            local_evidence._observation_refusal(observation, key, featured_plan, "alpha/macos-latest/L1")
        )
        bare, = observation_notes.receipt_observations(receipt)
        self.assertIn(
            "ran with features ''",
            local_evidence._observation_refusal(bare, key, featured_plan, "alpha/macos-latest/L1"),
        )


# ---------------------------------------------------------------------------
# Rig legs of `scripts/cross-check.sh`: producer -> discovery -> reuse
# ---------------------------------------------------------------------------

RIG_TARGETS = {
    "wsl2-ubuntu": "x86_64-unknown-linux-gnu",
    "windows-latest": "x86_64-pc-windows-msvc",
}
RIG_OS = {"wsl2-ubuntu": "Linux", "windows-latest": "Windows"}
OBSERVATION_NOTES = Path(observation_notes.__file__)


def write_rig_reports(
    directory: Path,
    package: str = "alpha",
    environment: str = "wsl2-ubuntu",
    *,
    expected: tuple[str, ...] = ("t_one", "t_two", "t_three"),
    reported: dict[str, str] | None = None,
    manifest: bool = True,
    report: bool = True,
    skip_listing: bool = False,
) -> Path:
    """A rig leg's staging directory, written the way `_stage_junit` and
    `_expected_manifest` leave it (`manifest.jsonl`, `L1/<package>.xml`,
    `expected-L1.json`) and `scp -r` fetches it."""
    shutil.rmtree(directory, ignore_errors=True)
    directory.mkdir(parents=True)
    suite = f"{package}::l1"
    reported = reported if reported is not None else {name: "pass" for name in expected}
    if report:
        cases = ""
        for name, outcome in reported.items():
            body = {"pass": "", "fail": "<failure>boom</failure>", "skip": "<skipped/>"}[outcome]
            cases += f'<testcase classname="{suite}" name="{name}">{body}</testcase>'
        failed = sum(1 for o in reported.values() if o == "fail")
        skipped = sum(1 for o in reported.values() if o == "skip")
        (directory / "L1").mkdir()
        (directory / "L1" / f"{package}.xml").write_text(
            f'<?xml version="1.0"?><testsuites><testsuite name="{suite}" '
            f'tests="{len(reported)}" failures="{failed}" errors="0" '
            f'skipped="{skipped}">{cases}</testsuite></testsuites>',
            encoding="utf-8",
        )
        entry = {
            "tier": "L1", "package": package, "xml": f"L1/{package}.xml",
            "exit_code": 100 if failed else 0, "environment": environment,
            "duration_s": 9, "report_present": True,
        }
        (directory / "manifest.jsonl").write_text(json.dumps(entry) + "\n", encoding="utf-8")
    else:
        (directory / "manifest.jsonl").write_text("", encoding="utf-8")
    if manifest:
        (directory / "expected-L1.json").write_text(
            json.dumps(
                {
                    "schema_version": 2,
                    "environment": environment,
                    "tier": "L1",
                    "target": RIG_TARGETS[environment],
                    "nextest_version": "cargo-nextest 0.9.136 (fixture)",
                    "from_archive": True,
                    "selection": {
                        "filter": f"package({package}) & ({test_completion.L1_SELECTION})",
                        "profile": "ci",
                        "test_args": "--archive-file x --workspace-remap y",
                    },
                    "backends": [],
                    "companion_suites": [],
                    "packages": {
                        package: {
                            "tests": [] if skip_listing else [f"{suite}::{n}" for n in expected],
                            "ignored": [],
                            "excluded": [],
                        }
                    },
                }
            ),
            encoding="utf-8",
        )
    return directory


class BlockingEvidenceTests(ObservationFixture):
    """`blocking_evidence` is `verify_cells`' decision read the other way round:
    the cells an unsuperseded failure, or an unordered conflict, keeps
    scheduled."""

    def blocked(self) -> dict:
        plan = json.loads(self.plan_path.read_text(encoding="utf-8"))
        return local_evidence.blocking_evidence(plan, self.head)

    def test_failure_conflict_retry_and_pass_decide_as_the_verifier_does(self) -> None:
        failed = self.observation("alpha", outcome="fail")
        retried = self.observation("alpha", retry_of=failed["observation_id"])
        cases = {
            "unsuperseded failure": ([failed], "later qualifying failure"),
            "unordered pass and failure": ([self.observation("alpha"), failed], "nothing orders"),
            "passing retry": ([failed, retried], None),
            "pass": ([self.observation("alpha")], None),
        }
        for label, (observations, reason) in cases.items():
            with self.subTest(label):
                self.add_note("macos-latest", self.collection(observations))
                blocked = self.blocked()
                accepted = self.accepted_keys()
                if reason is None:
                    self.assertEqual({}, blocked)
                    self.assertEqual(["alpha/macos-latest/L1"], accepted)
                else:
                    self.assertEqual([("alpha", "macos-latest", "L1")], list(blocked))
                    self.assertIn(reason, blocked[("alpha", "macos-latest", "L1")])
                    self.assertEqual([], accepted)

    def test_a_head_that_does_not_resolve_stops_rather_than_credits(self) -> None:
        plan = json.loads(self.plan_path.read_text(encoding="utf-8"))
        with self.assertRaises(RuntimeError):
            local_evidence.blocking_evidence(plan, "f" * 40)


class RigCellFixture(ObservationFixture):
    """A fetched rig staging directory published through the real command.

    The staging directories are written the way `_stage_junit` and
    `_expected_manifest` leave them (`manifest.jsonl`, `L1/<package>.xml`,
    `expected-L1.json`), and the command under test is the one
    `scripts/cross-check.sh` runs on the orchestrating host. Notes are read back
    through `verify_cells`, the question CI and the hook ask.
    """

    def plan(self) -> dict:
        document = super().plan()
        document["cells"] += [
            self.plan_cell(package, "windows-latest", "L1") for package in ("alpha", "beta")
        ]
        document["job_estimate"] = len(document["cells"])
        return plan_fixtures.finalize_plan(document)

    def setUp(self) -> None:
        super().setUp()
        self.head_tree = self.git("rev-parse", f"{self.head}^{{tree}}")
        self.remote = self.root / "remote.git"
        subprocess.run(["git", "init", "-q", "--bare", str(self.remote)], check=True)
        self.git("remote", "add", "origin", str(self.remote))
        self.git("push", "-q", "origin", "main")
        self.evidence = self.root / "evidence"

    def fetched(self, package: str = "alpha", environment: str = "wsl2-ubuntu", **kwargs) -> Path:
        """The staging directory a rig leg leaves, as `scp -r` fetches it."""
        return write_rig_reports(
            self.root / f"fetched-{environment}-{package}", package, environment, **kwargs
        )

    def publish_rig(
        self,
        environment: str,
        directory: Path,
        *,
        package: str = "alpha",
        exit_code: int = 0,
        tested_tree: str | None = None,
        dirty: bool = False,
        filters: str = "",
        run_id: str = "run-1",
        push: bool = True,
        cwd: Path | None = None,
        retry_of: str | None = None,
    ) -> subprocess.CompletedProcess:
        command = [
            sys.executable, str(OBSERVATION_NOTES), "publish-receipt", "--kind", "rig",
            "--plan", str(self.plan_path), "--package", package,
            "--environment", environment, "--reports-dir", str(directory),
            "--evidence-dir", str(self.evidence), "--run-id", run_id,
            "--head", self.head, "--base", self.base,
            "--tested-tree", self.head_tree if tested_tree is None else tested_tree,
            "--exit-code", str(exit_code), "--duration", "61",
            "--build-key", "0f1e2d3c4b5a6978", "--build-digest", "9a8b7c6d5e4f3021",
        ]
        if dirty:
            command.append("--remote-dirty")
        if filters:
            command += ["--run-filters", filters]
        if retry_of:
            command += ["--retry-of", retry_of]
        command += ["--remote", "origin"] if push else ["--no-push"]
        return subprocess.run(
            command, cwd=cwd or self.root, capture_output=True, text=True, timeout=120
        )

    def note(self, environment: str) -> dict | None:
        shown = subprocess.run(
            ["git", "notes", "--ref", f"{NOTES_PREFIX}/{environment}", "show", self.head],
            capture_output=True, text=True,
        )
        return json.loads(shown.stdout) if shown.returncode == 0 else None

    def observations(self, environment: str) -> list[dict]:
        note = self.note(environment)
        return note["observations"] if note else []

    def assertPublishedNothing(self, completed: subprocess.CompletedProcess, *reasons: str) -> None:
        self.assertEqual(3, completed.returncode, completed.stdout + completed.stderr)
        self.assertRegex(completed.stderr, r"cross-check: publishing no \S+ observation - ")
        for reason in reasons:
            self.assertIn(reason, completed.stderr)
        for environment in RIG_TARGETS:
            self.assertIsNone(self.note(environment), "a declined leg must record nothing")


class RigProducerToReuseTests(RigCellFixture):
    """AC: a finished rig leg is proof for its own environment's cell, only."""

    def test_wsl2_round_trips_to_exactly_its_own_cell(self) -> None:
        self.assertRoundTrip("wsl2-ubuntu", "windows-latest")

    def test_native_windows_round_trips_to_exactly_its_own_cell(self) -> None:
        self.assertRoundTrip("windows-latest", "wsl2-ubuntu")

    def assertRoundTrip(self, environment: str, other: str) -> None:
        completed = self.publish_rig(environment, self.fetched("alpha", environment))
        self.assertEqual(0, completed.returncode, completed.stdout + completed.stderr)
        self.assertIn(f"published a passing {environment} observation", completed.stdout)
        self.assertEqual([f"alpha/{environment}/L1"], self.accepted_keys())
        self.assertIsNone(self.note(other), "the other rig must be untouched")
        (observation,) = self.observations(environment)
        self.assertEqual("rig", observation["producer"]["kind"])
        self.assertEqual(RIG_OS[environment], observation["producer"]["host"]["os"])
        self.assertEqual("run-1", observation["producer"]["run_id"])
        self.assertEqual(RIG_TARGETS[environment], observation["proof"]["target"])
        self.assertEqual("pass", observation["cell"]["outcome"])

    def test_the_cited_proof_is_retained_in_one_directory_per_run(self) -> None:
        self.publish_rig("windows-latest", self.fetched("alpha", "windows-latest"))
        (observation,) = self.observations("windows-latest")
        retained = Path(observation["producer"]["host"]["report_dir"])
        self.assertEqual(
            self.evidence / self.head / "windows-latest" / "alpha-L1-run-1", retained
        )
        for name, field in (
            (schema.PROOF_EXPECTED_FILE, "expected_listing_digest"),
            (schema.PROOF_COMPLETION_FILE, "completion_record_digest"),
        ):
            content = (retained / name.format(package="alpha", gate="L1")).read_bytes()
            self.assertEqual(hashlib.sha256(content).hexdigest(), observation["proof"][field])
        record = json.loads((retained / "alpha-L1.completion.json").read_text())
        self.assertTrue(record["complete"])
        self.assertEqual("windows-latest", record["environment"])
        self.assertTrue((retained / "alpha-L1.xml").is_file())

    def test_both_rigs_for_one_package_are_credited_independently(self) -> None:
        self.publish_rig("windows-latest", self.fetched("alpha", "windows-latest"), run_id="w")
        self.publish_rig("wsl2-ubuntu", self.fetched("alpha", "wsl2-ubuntu"), run_id="l")
        self.assertEqual(
            ["alpha/windows-latest/L1", "alpha/wsl2-ubuntu/L1"], self.accepted_keys()
        )

    def test_sequential_packages_from_one_rig_are_unioned_not_overwritten(self) -> None:
        self.publish_rig("wsl2-ubuntu", self.fetched("alpha"), run_id="a")
        self.publish_rig("wsl2-ubuntu", self.fetched("beta"), package="beta", run_id="b")
        self.assertEqual(
            ["alpha/wsl2-ubuntu/L1", "beta/wsl2-ubuntu/L1"], self.accepted_keys()
        )
        shown = subprocess.run(
            ["git", "--git-dir", str(self.remote), "notes", "--ref",
             f"{NOTES_PREFIX}/wsl2-ubuntu", "show", self.head],
            capture_output=True, text=True, check=True,
        ).stdout
        self.assertEqual(2, len(json.loads(shown)["observations"]), "the remote carries both")

    def test_a_declined_later_cell_leaves_the_earlier_proof_untouched(self) -> None:
        self.publish_rig("wsl2-ubuntu", self.fetched("alpha"), run_id="a")
        before = self.git("notes", "--ref", f"{NOTES_PREFIX}/wsl2-ubuntu", "show", self.head)
        incomplete = self.fetched("beta", reported={"t_one": "pass"})
        completed = self.publish_rig("wsl2-ubuntu", incomplete, package="beta", run_id="b")
        self.assertEqual(3, completed.returncode)
        self.assertEqual(
            before, self.git("notes", "--ref", f"{NOTES_PREFIX}/wsl2-ubuntu", "show", self.head)
        )
        self.assertEqual(["alpha/wsl2-ubuntu/L1"], self.accepted_keys())


class RigFailureAndIncompleteTests(RigCellFixture):
    """A completed failure is an observation; an incomplete run is nothing."""

    def test_a_completed_failure_is_published_and_blocks_an_older_pass(self) -> None:
        self.publish_rig("wsl2-ubuntu", self.fetched("alpha"), run_id="pass")
        self.assertEqual(["alpha/wsl2-ubuntu/L1"], self.accepted_keys())
        failing = self.fetched("alpha", reported={"t_one": "pass", "t_two": "fail", "t_three": "pass"})
        completed = self.publish_rig("wsl2-ubuntu", failing, exit_code=100, run_id="fail")
        self.assertEqual(0, completed.returncode, completed.stdout + completed.stderr)
        self.assertIn("published a FAILING wsl2-ubuntu observation", completed.stdout)
        accepted, rejections = self.verify()
        self.assertEqual([], self.keys(accepted), "the newer failure must hide the older pass")
        self.assertTrue([r for r in rejections if r.startswith("failed-cell")], rejections)
        failure = next(o for o in self.observations("wsl2-ubuntu") if o["cell"]["outcome"] == "fail")
        self.assertNotIn("completion_record_digest", failure["proof"])
        self.assertEqual(
            {"nextest_version", "target", "expected_listing_digest"}, set(failure["proof"])
        )
        retained = Path(failure["producer"]["host"]["report_dir"])
        self.assertFalse(
            (retained / "alpha-L1.completion.json").exists(), "a failure has no completion record"
        )

    def retry_predecessor(self, environment: str) -> str:
        return subprocess.run(
            [sys.executable, str(OBSERVATION_NOTES), "retry-predecessor", "--head", self.head,
             "--environment", environment, "--package", "alpha"],
            cwd=self.root, capture_output=True, text=True, check=True, timeout=60,
        ).stdout.strip()

    def test_a_later_passing_retry_restores_reuse(self) -> None:
        failing = self.fetched(
            "alpha", "windows-latest", reported={"t_one": "fail", "t_two": "pass", "t_three": "pass"}
        )
        completed = self.publish_rig("windows-latest", failing, exit_code=100, run_id="fail")
        self.assertEqual(0, completed.returncode, completed.stdout + completed.stderr)
        self.assertEqual([], self.accepted_keys())
        # The retry learns its predecessor when it starts, as `cross-check.sh` does.
        predecessor = self.retry_predecessor("windows-latest")
        self.assertTrue(predecessor)
        completed = self.publish_rig(
            "windows-latest", self.fetched("alpha", "windows-latest"), run_id="pass",
            retry_of=predecessor,
        )
        self.assertEqual(0, completed.returncode, completed.stdout + completed.stderr)
        self.assertEqual(["alpha/windows-latest/L1"], self.accepted_keys())

    def test_a_delayed_pass_does_not_retry_a_failure_published_while_it_ran(self) -> None:
        # The passing leg started when the cell had no observation to retry.
        predecessor = self.retry_predecessor("wsl2-ubuntu")
        self.assertEqual("", predecessor)
        failing = self.fetched("alpha", reported={"t_one": "fail", "t_two": "pass", "t_three": "pass"})
        completed = self.publish_rig("wsl2-ubuntu", failing, exit_code=100, run_id="fail")
        self.assertEqual(0, completed.returncode, completed.stdout + completed.stderr)
        completed = self.publish_rig("wsl2-ubuntu", self.fetched("alpha"), run_id="pass")
        self.assertEqual(0, completed.returncode, completed.stdout + completed.stderr)
        self.assertEqual([], [o for o in self.observations("wsl2-ubuntu") if "retry_of" in o])
        accepted, rejections = self.verify()
        self.assertEqual([], self.keys(accepted))
        self.assertTrue([r for r in rejections if r.startswith("conflicting-evidence")], rejections)

    def test_a_hook_receipt_cannot_name_a_predecessor(self) -> None:
        completed = subprocess.run(
            [sys.executable, str(OBSERVATION_NOTES), "publish-receipt", "--receipt", "r.json",
             "--plan", "plan.json", "--head", self.head, "--environment", "macos-latest",
             "--retry-of", "ab" * 32],
            cwd=self.root, capture_output=True, text=True, timeout=60,
        )
        self.assertEqual(2, completed.returncode)
        self.assertIn("a hook receipt retries nothing", completed.stderr)

    def test_a_failure_without_a_manifest_is_published_without_proof(self) -> None:
        failing = self.fetched(
            "alpha", reported={"t_one": "fail", "t_two": "pass", "t_three": "pass"}, manifest=False
        )
        completed = self.publish_rig("wsl2-ubuntu", failing, exit_code=100)
        self.assertEqual(0, completed.returncode, completed.stdout + completed.stderr)
        (observation,) = self.observations("wsl2-ubuntu")
        self.assertEqual("fail", observation["cell"]["outcome"])
        self.assertNotIn("proof", observation)
        self.assertEqual([], self.accepted_keys())

    def test_a_pass_without_a_manifest_publishes_nothing(self) -> None:
        completed = self.publish_rig("wsl2-ubuntu", self.fetched("alpha", manifest=False))
        self.assertPublishedNothing(completed, "completion-manifest-missing")

    def test_a_missing_test_is_an_incomplete_run(self) -> None:
        short = self.fetched(
            "alpha", "windows-latest", reported={"t_one": "pass", "t_two": "pass"}
        )
        completed = self.publish_rig("windows-latest", short)
        self.assertPublishedNothing(completed, "completion-test-missing", "t_three")
        self.assertEqual([], self.accepted_keys())

    def test_zero_reports_against_a_nonempty_expectation_publish_nothing(self) -> None:
        completed = self.publish_rig("windows-latest", self.fetched("alpha", "windows-latest", report=False))
        self.assertPublishedNothing(completed, "no JUnit report")
        # The same, when a stray empty report exists but staged no entry for it.
        directory = self.fetched("alpha", "windows-latest", reported={})
        completed = self.publish_rig("windows-latest", directory)
        self.assertPublishedNothing(completed, "completion-test-missing")

    def test_a_failure_alongside_a_missing_test_is_incomplete_not_a_failure(self) -> None:
        mixed = self.fetched("alpha", reported={"t_one": "fail", "t_two": "pass"})
        completed = self.publish_rig("wsl2-ubuntu", mixed, exit_code=100)
        self.assertPublishedNothing(completed, "completion-test-missing")

    def test_an_empty_expectation_keeps_completions_refusal(self) -> None:
        empty = self.fetched("alpha", expected=(), reported={}, skip_listing=True)
        completed = self.publish_rig("wsl2-ubuntu", empty)
        self.assertPublishedNothing(completed, "completion-expected-empty")

    def test_an_exit_code_that_contradicts_the_report_publishes_nothing(self) -> None:
        completed = self.publish_rig("wsl2-ubuntu", self.fetched("alpha"), exit_code=1)
        self.assertPublishedNothing(completed, "exited 1", "passing")
        failing = self.fetched("alpha", reported={"t_one": "fail", "t_two": "pass", "t_three": "pass"})
        completed = self.publish_rig("wsl2-ubuntu", failing, exit_code=0)
        self.assertPublishedNothing(completed, "exited 0", "failing test")

    def test_a_manifest_from_the_other_environment_is_never_filed(self) -> None:
        """A WSL2 listing cannot make a native Windows observation, or the reverse."""
        wrong = self.fetched("alpha", "wsl2-ubuntu")
        completed = self.publish_rig("windows-latest", wrong)
        self.assertPublishedNothing(completed, "completion-manifest-provenance", "names environment 'wsl2-ubuntu'")

    def test_only_rig_environments_can_be_recorded(self) -> None:
        completed = self.publish_rig("macos-latest", self.fetched("alpha"))
        self.assertPublishedNothing(completed, "not a rig environment")


class RigQualificationTests(RigCellFixture):
    """Runs that are smoke evidence only: the reason is printed, nothing recorded."""

    def test_a_tree_that_is_not_the_heads_publishes_nothing(self) -> None:
        completed = self.publish_rig("wsl2-ubuntu", self.fetched("alpha"), tested_tree="0" * 40)
        self.assertPublishedNothing(completed, "is not this head's tree")
        self.assertFalse(self.evidence.exists(), "nothing is retained for a declined leg")

    def test_a_dirty_remote_publishes_nothing(self) -> None:
        completed = self.publish_rig("windows-latest", self.fetched("alpha", "windows-latest"), dirty=True)
        self.assertPublishedNothing(completed, "dirty")

    def test_a_filtered_run_publishes_nothing(self) -> None:
        completed = self.publish_rig("wsl2-ubuntu", self.fetched("alpha"), filters="level2_")
        self.assertPublishedNothing(completed, "narrowed by")

    def test_a_package_the_plan_did_not_schedule_publishes_nothing(self) -> None:
        completed = self.publish_rig("wsl2-ubuntu", self.fetched("gamma"), package="gamma")
        self.assertPublishedNothing(completed, "has no gamma/wsl2-ubuntu/L1 cell")

    def test_a_missing_staging_directory_publishes_nothing(self) -> None:
        completed = self.publish_rig("wsl2-ubuntu", self.root / "never-fetched")
        self.assertPublishedNothing(completed, "no JUnit report")

    def test_an_unreachable_remote_keeps_the_proof_local_and_others_scheduled(self) -> None:
        self.git("remote", "set-url", "origin", str(self.root / "missing.git"))
        completed = self.publish_rig("wsl2-ubuntu", self.fetched("alpha"))
        self.assertEqual(4, completed.returncode, completed.stdout + completed.stderr)
        self.assertIn("locally; it could not be pushed", completed.stderr)
        self.assertEqual(["alpha/wsl2-ubuntu/L1"], self.accepted_keys(), "proof stays on this host")
        # A reader who only has the remote sees nothing, so the cell stays scheduled.
        clone = self.root / "clone"
        subprocess.run(["git", "clone", "-q", str(self.remote), str(clone)], check=True)
        previous = Path.cwd()
        os.chdir(clone)
        try:
            accepted, _ = local_evidence.verify_cells(str(self.plan_path), self.base, self.head)
        finally:
            os.chdir(previous)
        self.assertEqual([], accepted)

    def test_a_run_with_no_push_target_is_recorded_locally_only(self) -> None:
        completed = self.publish_rig("wsl2-ubuntu", self.fetched("alpha"), push=False)
        self.assertEqual(4, completed.returncode)
        self.assertEqual(["alpha/wsl2-ubuntu/L1"], self.accepted_keys())

    def test_two_runs_of_one_cell_keep_separate_proof_directories(self) -> None:
        self.publish_rig("wsl2-ubuntu", self.fetched("alpha"), run_id="first")
        self.publish_rig("wsl2-ubuntu", self.fetched("alpha"), run_id="second")
        first = self.evidence / self.head / "wsl2-ubuntu" / "alpha-L1-first"
        second = self.evidence / self.head / "wsl2-ubuntu" / "alpha-L1-second"
        self.assertTrue((first / "alpha-L1.completion.json").is_file())
        self.assertTrue((second / "alpha-L1.completion.json").is_file())
        self.assertEqual(["alpha/wsl2-ubuntu/L1"], self.accepted_keys())

    def test_retained_proof_that_was_altered_is_not_credited(self) -> None:
        self.publish_rig("windows-latest", self.fetched("alpha", "windows-latest"))
        retained = self.evidence / self.head / "windows-latest" / "alpha-L1-run-1"
        (retained / "alpha-L1.completion.json").write_text("{}", encoding="utf-8")
        accepted, rejections = self.verify()
        self.assertEqual([], accepted)
        self.assertTrue([r for r in rejections if "observation-report-unavailable" in r], rejections)


class PartialRecoveryTests(ObservationFixture):
    """Rig and hosted proof through one verifier and overlay, across partial runs.

    A nightly can pass in one environment, fail or be cancelled in another, and
    be retried; what survives must be exactly what completed, and the retry must
    execute only what is still unproved.
    """

    def hosted(self, package: str, *, outcome: str = "pass", retry_of: str | None = None) -> dict:
        observation = self.observation(
            package, kind="hosted", store="artifact", outcome=outcome, retry_of=retry_of
        )
        body = {k: v for k, v in observation.items() if k != "observation_id"}
        body["producer"] = {
            **body["producer"],
            "workflow": ".github/workflows/ci.yml",
            "run_id": "424242",
            "attempt": 1,
        }
        if outcome == "fail":
            body.pop("proof", None)
        return observation_notes.with_identity(body)

    def rig(self, package: str, **kwargs) -> dict:
        return self.observation(
            package, kind="rig", report_dir=self.retained_dir(f"rig-{package}-{len(kwargs)}"), **kwargs
        )

    def overlaid(self, **kwargs) -> dict[str, tuple[str, str]]:
        accepted, rejections = self.verify(report_probe=lambda report, observation: True, **kwargs)
        plan = json.loads(self.plan_path.read_text(encoding="utf-8"))
        applied = affected_scope.apply_accepted_cells(plan, accepted, rejections)
        self.assertEqual([], schema.validate_resolved_plan(applied))
        return {
            f"{c['package']}/{c['environment']}/{c['gate']}": (c["execution"], c["state"])
            for c in applied["cells"]
        }

    def advance(self, relative: str, text: str) -> str:
        self.write(relative, text)
        self.git("commit", "-q", "-am", f"touch {relative}")
        self.head = self.git("rev-parse", "HEAD")
        self.plan_path.write_text(schema.canonical(self.plan()), encoding="utf-8")
        return self.head

    def test_proof_survives_a_failure_and_a_cancellation_and_the_retry_runs_only_the_rest(self) -> None:
        self.add_note(
            "wsl2-ubuntu",
            self.collection([self.rig("alpha"), self.rig("beta")], environment="wsl2-ubuntu"),
        )
        failed_beta = self.hosted("beta", outcome="fail")
        self.add_note(
            "macos-latest", self.collection([self.hosted("alpha"), failed_beta])
        )
        # ubuntu-latest was cancelled before it recorded anything.
        first = self.overlaid()
        self.assertEqual(("reuse", "reused"), first["alpha/wsl2-ubuntu/L1"])
        self.assertEqual(("reuse", "reused"), first["beta/wsl2-ubuntu/L1"])
        self.assertEqual(("reuse", "reused"), first["alpha/macos-latest/L1"])
        executing = sorted(key for key, (execution, _) in first.items() if execution == "execute")
        self.assertEqual(
            ["alpha/ubuntu-latest/L1", "beta/macos-latest/L1", "beta/ubuntu-latest/L1"], executing
        )

        # The retry passes the failed cell and runs the cancelled environment; the
        # earlier proof is untouched.
        self.add_note(
            "macos-latest",
            self.collection(
                [
                    self.hosted("alpha"),
                    failed_beta,
                    self.hosted("beta", retry_of=failed_beta["observation_id"]),
                ]
            ),
        )
        self.add_note(
            "ubuntu-latest",
            self.collection([self.hosted("alpha"), self.hosted("beta")], environment="ubuntu-latest"),
        )
        second = self.overlaid()
        self.assertEqual({"reuse"}, {execution for execution, _ in second.values()})

    def test_a_later_main_commit_reuses_unchanged_inputs_and_reruns_changed_ones(self) -> None:
        for environment, kind in (("wsl2-ubuntu", "rig"), ("macos-latest", "hosted")):
            made = self.rig if kind == "rig" else self.hosted
            self.add_note(
                environment,
                self.collection([made("alpha"), made("beta")], environment=environment),
            )
        self.advance("docs/unrelated.md", "prose that no gate reads\n")
        unchanged = self.overlaid()
        for key in ("alpha/wsl2-ubuntu/L1", "beta/wsl2-ubuntu/L1", "alpha/macos-latest/L1"):
            self.assertEqual(("reuse", "reused"), unchanged[key], key)
        self.advance("pkg/alpha/src/lib.rs", "pub fn alpha() { /* changed again */ }\n")
        changed = self.overlaid()
        self.assertEqual(("execute", "pending"), changed["alpha/wsl2-ubuntu/L1"])
        self.assertEqual(("execute", "pending"), changed["alpha/macos-latest/L1"])
        self.assertEqual(("reuse", "reused"), changed["beta/wsl2-ubuntu/L1"])
        self.assertEqual(("reuse", "reused"), changed["beta/macos-latest/L1"])

    def test_a_warn_mode_failure_in_one_environment_does_not_hide_a_pass_in_another(self) -> None:
        self.add_note(
            "macos-latest", self.collection([self.observation("alpha", outcome="fail")])
        )
        self.add_note(
            "wsl2-ubuntu",
            self.collection([self.rig("alpha")], environment="wsl2-ubuntu"),
        )
        states = self.overlaid()
        self.assertEqual(("execute", "pending"), states["alpha/macos-latest/L1"])
        self.assertEqual(("reuse", "reused"), states["alpha/wsl2-ubuntu/L1"])

    def test_an_unordered_rig_and_hosted_disagreement_executes_until_a_later_commit_settles_it(self) -> None:
        self.add_note(
            "wsl2-ubuntu",
            self.collection(
                [self.rig("beta"), self.hosted("beta", outcome="fail")], environment="wsl2-ubuntu"
            ),
        )
        conflicted = self.overlaid()
        self.assertEqual(("execute", "pending"), conflicted["beta/wsl2-ubuntu/L1"])
        _, rejections = self.verify(report_probe=lambda report, observation: True)
        self.assertTrue(any(r.startswith("conflicting-evidence") for r in rejections), rejections)

        later = self.advance("docs/unrelated.md", "prose that no gate reads\n")
        self.add_note(
            "wsl2-ubuntu",
            self.collection([self.rig("beta", duration_s=77)], environment="wsl2-ubuntu", commit=later),
            later,
        )
        settled = self.overlaid()
        self.assertEqual(("reuse", "reused"), settled["beta/wsl2-ubuntu/L1"])

    def test_a_hosted_pass_whose_artifacts_expired_executes_while_the_rig_pass_stays_reusable(self) -> None:
        self.add_note(
            "wsl2-ubuntu", self.collection([self.rig("alpha")], environment="wsl2-ubuntu")
        )
        self.add_note("macos-latest", self.collection([self.hosted("alpha")]))
        accepted, rejections = self.verify(report_probe=lambda report, observation: False)
        self.assertEqual(["alpha/wsl2-ubuntu/L1"], self.keys(accepted))
        self.assertTrue(
            any(r.startswith("observation-report-unavailable") for r in rejections), rejections
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)

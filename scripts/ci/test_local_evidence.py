#!/usr/bin/env python3
"""Tests for exact-tree local CI evidence."""

from __future__ import annotations

import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

import schema  # noqa: E402
from affected_scope import legacy_scope_document
from local_evidence import (  # noqa: E402
    NOTES_PREFIX,
    SCOPE_NOTES_REF,
    record,
    record_scope,
    verified_environment,
    verify_cells,
    verify_scope,
)


class RepositoryFixture(unittest.TestCase):
    """Two commits, `base` then `head`, with a legacy scope document beside them."""

    def setUp(self) -> None:
        self.previous_cwd = Path.cwd()
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        os.chdir(self.root)
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.name", "CI Test")
        self.git("config", "user.email", "ci@example.com")
        (self.root / "file").write_text("base\n", encoding="utf-8")
        self.git("add", "file")
        self.git("commit", "-q", "-m", "base")
        self.base = self.git("rev-parse", "HEAD")
        (self.root / "file").write_text("head\n", encoding="utf-8")
        self.git("commit", "-q", "-am", "head")
        self.head = self.git("rev-parse", "HEAD")
        self.scope = self.root / "scope.json"
        self.scope.write_text(
            json.dumps(
                {
                    "source_packages": ["alpha"],
                    "reverse_dependencies": ["consumer"],
                    "matrix": [
                        {
                            "package": "alpha",
                            "gates": ["lint", "check", "test"],
                            "tiers": ["L1", "L2"],
                            "native_environments": [
                                "ubuntu-latest",
                                "windows-latest",
                                "macos-latest",
                            ],
                            "l2_environments": ["ubuntu-latest", "macos-latest"],
                            "wsl": True,
                        },
                        {"package": "consumer", "gates": ["check"]},
                    ],
                }
            ),
            encoding="utf-8",
        )

    def tearDown(self) -> None:
        os.chdir(self.previous_cwd)
        self.temporary_directory.cleanup()

    def git(self, *args: str) -> str:
        return subprocess.run(
            ["git", *args], check=True, capture_output=True, text=True
        ).stdout.strip()

    def add_note(self, environment: str, contents: str) -> None:
        self.git(
            "notes",
            "--ref",
            f"{NOTES_PREFIX}/{environment}",
            "add",
            "-m",
            contents,
            self.head,
        )



class LocalEvidenceTests(RepositoryFixture):
    def test_record_names_exact_source_reverse_and_tier_scope(self) -> None:
        document = json.loads(
            record(str(self.scope), self.base, self.head, "macos-latest")
        )
        self.assertEqual(["alpha"], document["source_packages"])
        self.assertEqual(["consumer"], document["reverse_dependencies"])
        self.assertEqual(["alpha"], document["l1_packages"])
        self.assertEqual(["alpha"], document["l2_packages"])

    def test_matching_note_excludes_its_environment(self) -> None:
        contents = record(str(self.scope), self.base, self.head, "macos-latest")
        self.add_note("macos-latest", contents)
        self.assertEqual(
            "macos-latest", verified_environment(str(self.scope), self.base, self.head)
        )

    def test_scope_drift_invalidates_the_note(self) -> None:
        contents = record(str(self.scope), self.base, self.head, "macos-latest")
        self.add_note("macos-latest", contents)
        document = json.loads(self.scope.read_text(encoding="utf-8"))
        document["source_packages"].append("beta")
        self.scope.write_text(json.dumps(document), encoding="utf-8")
        self.assertEqual("", verified_environment(str(self.scope), self.base, self.head))

    def test_advanced_pr_base_reuses_only_matching_scope(self) -> None:
        contents = record(str(self.scope), self.base, self.head, "macos-latest")
        self.add_note("macos-latest", contents)
        self.git("checkout", "-q", "-b", "advanced-base", self.base)
        (self.root / "file").write_text("upstream\n", encoding="utf-8")
        self.git("commit", "-q", "-am", "advance PR base")
        pr_base = self.git("rev-parse", "HEAD")
        self.assertNotEqual(pr_base, self.git("merge-base", pr_base, self.head))
        self.assertEqual(
            "macos-latest", verified_environment(str(self.scope), pr_base, self.head)
        )

        document = json.loads(self.scope.read_text(encoding="utf-8"))
        document["source_packages"].append("upstream-package")
        self.scope.write_text(json.dumps(document), encoding="utf-8")
        self.assertEqual("", verified_environment(str(self.scope), pr_base, self.head))
        with self.assertRaisesRegex(ValueError, "must be an ancestor"):
            record(str(self.scope), pr_base, self.head, "macos-latest")

    def test_invalid_or_unrelated_base_is_a_cache_miss(self) -> None:
        contents = record(str(self.scope), self.base, self.head, "macos-latest")
        self.add_note("macos-latest", contents)
        self.assertEqual("", verified_environment(str(self.scope), "missing-ref", self.head))
        self.git("checkout", "-q", "--orphan", "unrelated")
        self.git("commit", "-q", "-am", "unrelated history")
        unrelated = self.git("rev-parse", "HEAD")
        self.assertEqual("", verified_environment(str(self.scope), unrelated, self.head))


class ScopeReceiptTests(RepositoryFixture):
    """The scope receipt of fixes/2026-09-10-local-affected-scope, R1-R3."""

    def setUp(self) -> None:
        super().setUp()
        self.plan = {
            "schema_version": schema.RESOLVED_PLAN_SCHEMA_VERSION,
            "base": self.base,
            "head": self.head,
            "change_class": "package",
            "full_scope": False,
            "full_scope_gates": [],
            "areas": [{"area": "pkg", "selection_reason": "source change", "packages": ["alpha"]}],
            "packages": [
                {
                    "package": "alpha",
                    "area": "pkg",
                    "selection_reason": "source change",
                    "gates": ["L1"],
                    "targets": ["lib", "test"],
                    "tiers": ["L1"],
                    "test_args": "",
                    "check_args": "-p alpha",
                    "l2_backends": [],
                    "runner_tools": [],
                    "companion_suites": [],
                    "l1_include_slow": False,
                    "native": {},
                }
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
            "cells": [
                {
                    "package": "alpha",
                    "area": "pkg",
                    "environment": "macos-latest",
                    "gate": "L1",
                    "execution": "execute",
                    "origin": "ci",
                    "state": "pending",
                    "reusable": True,
                    "target_kinds": ["lib", "test"],
                    "compile_coverage_from": "L1",
                    "selection_reason": "no evidence",
                }
            ],
            "accepted_evidence": [],
            "policy_gaps": [],
            "prohibited_cells": [],
            "job_estimate": 1,
            "preflight_os": ["macos-latest"],
            "preflight_reason": "package-local change",
            "flags": {"ci_tooling": False},
        }
        self.projection = {
            name: [] for name in schema.SCOPE_PROJECTION_FIELDS
        } | {
            "packages": ["alpha"],
            "area_matrix": {},
            "area_slugs": {},
            "full_scope": False,
            "change_class": "package",
            "preflight_reason": "package-local change",
            "job_estimate": 1,
            "flags": {"ci_tooling": False},
        }
        self.plan_path = self.root / "plan.json"
        self.plan_path.write_text(schema.canonical(self.plan), encoding="utf-8")
        self.projection_path = self.root / "projection.json"
        self.projection_path.write_text(schema.canonical(self.projection), encoding="utf-8")

    def receipt(self) -> str:
        return record_scope(str(self.plan_path), str(self.projection_path), self.base, self.head)

    def add_scope_note(self, contents: str) -> None:
        self.git("notes", "--ref", SCOPE_NOTES_REF, "add", "-f", "-m", contents, self.head)

    def test_a_receipt_binds_the_exact_base_head_and_tree(self) -> None:
        document = json.loads(self.receipt())
        self.assertEqual([], schema.validate_scope_receipt(document))
        self.assertEqual(self.base, document["base"])
        self.assertEqual(self.head, document["head"])
        self.assertEqual(self.git("rev-parse", f"{self.head}^{{tree}}"), document["tree"])
        self.assertEqual(self.plan, document["plan"])
        self.assertEqual(self.projection, document["scope"])

    def test_recording_twice_is_byte_identical(self) -> None:
        self.assertEqual(self.receipt(), self.receipt())

    def test_an_advanced_base_is_exactly_bound(self) -> None:
        self.git("checkout", "-q", "-b", "elsewhere", self.base)
        (self.root / "file").write_text("elsewhere\n", encoding="utf-8")
        self.git("commit", "-q", "-am", "elsewhere")
        elsewhere = self.git("rev-parse", "HEAD")
        self.plan["base"] = elsewhere
        self.plan_path.write_text(schema.canonical(self.plan), encoding="utf-8")
        self.add_scope_note(record_scope(str(self.plan_path), str(self.projection_path), elsewhere, self.head))
        document, reason = verify_scope(elsewhere, self.head)
        self.assertEqual("", reason)
        self.assertEqual(elsewhere, document["base"])
        document, reason = verify_scope(self.base, self.head)
        self.assertIsNone(document)
        self.assertTrue(reason.startswith("scope-base-mismatch:"), reason)

    def test_the_exact_identity_is_a_hit_carrying_both_documents(self) -> None:
        self.add_scope_note(self.receipt())
        document, reason = verify_scope(self.base, self.head)
        self.assertEqual("", reason)
        self.assertEqual(self.plan, document["plan"])
        self.assertEqual(self.projection, document["scope"])

    def test_a_missing_note_is_scope_missing(self) -> None:
        document, reason = verify_scope(self.base, self.head)
        self.assertIsNone(document)
        self.assertTrue(reason.startswith("scope-missing:"), reason)

    def test_an_unresolvable_pair_is_scope_missing(self) -> None:
        self.add_scope_note(self.receipt())
        _, reason = verify_scope("no-such-ref", self.head)
        self.assertTrue(reason.startswith("scope-missing:"), reason)

    def test_another_base_is_scope_base_mismatch(self) -> None:
        self.add_scope_note(self.receipt())
        self.git("checkout", "-q", "-b", "advanced", self.base)
        (self.root / "file").write_text("upstream\n", encoding="utf-8")
        self.git("commit", "-q", "-am", "advance the base")
        advanced = self.git("rev-parse", "HEAD")
        document, reason = verify_scope(advanced, self.head)
        self.assertIsNone(document)
        self.assertTrue(reason.startswith("scope-base-mismatch:"), reason)

    def test_a_note_declaring_another_head_is_scope_head_mismatch(self) -> None:
        document = json.loads(self.receipt())
        document["head"] = self.base
        self.add_scope_note(schema.canonical(document))
        _, reason = verify_scope(self.base, self.head)
        self.assertTrue(reason.startswith("scope-head-mismatch:"), reason)

    def test_a_note_declaring_another_tree_is_scope_tree_mismatch(self) -> None:
        document = json.loads(self.receipt())
        document["tree"] = self.git("rev-parse", f"{self.base}^{{tree}}")
        self.add_scope_note(schema.canonical(document))
        _, reason = verify_scope(self.base, self.head)
        self.assertTrue(reason.startswith("scope-tree-mismatch:"), reason)

    def test_another_schema_version_is_scope_schema(self) -> None:
        document = json.loads(self.receipt())
        document["schema_version"] = 99
        self.add_scope_note(schema.canonical(document))
        _, reason = verify_scope(self.base, self.head)
        self.assertTrue(reason.startswith("scope-schema:"), reason)

    def test_a_malformed_note_is_scope_malformed(self) -> None:
        self.add_scope_note("{not json")
        _, reason = verify_scope(self.base, self.head)
        self.assertTrue(reason.startswith("scope-malformed:"), reason)
        document = json.loads(self.receipt())
        del document["plan"]
        self.add_scope_note(schema.canonical(document))
        _, reason = verify_scope(self.base, self.head)
        self.assertTrue(reason.startswith("scope-malformed:"), reason)

    def test_an_older_commits_note_is_never_consulted(self) -> None:
        # Scope binds to an exact head: a note on the parent, however valid
        # for the parent, says nothing about this head.
        older = json.loads(self.receipt())
        self.git("notes", "--ref", SCOPE_NOTES_REF, "add", "-f", "-m", schema.canonical(older), self.base)
        _, reason = verify_scope(self.base, self.head)
        self.assertTrue(reason.startswith("scope-missing:"), reason)

    def test_the_scope_ref_is_not_an_environment_ref(self) -> None:
        # `verify_cells` walks `refs/notes/ci-local/<environment>`; the scope
        # note lives beside them and must be invisible to that walk.
        self.assertNotIn("scope", schema.ENVIRONMENTS)
        self.assertNotIn(SCOPE_NOTES_REF, {f"{NOTES_PREFIX}/{env}" for env in schema.ENVIRONMENTS})
        self.add_scope_note(self.receipt())
        accepted, rejections = verify_cells(str(self.plan_path), self.base, self.head)
        self.assertEqual(([], []), (accepted, rejections))

    def test_every_miss_code_is_in_the_shared_vocabulary(self) -> None:
        for code in ("scope-missing", "scope-schema", "scope-head-mismatch",
                     "scope-tree-mismatch", "scope-base-mismatch", "scope-malformed"):
            self.assertIn(code, schema.SCOPE_REJECTIONS)

    def test_the_cli_projects_the_canonical_plan_on_a_hit_and_exits_3_on_a_miss(self) -> None:
        tool = Path(__file__).resolve().parent / "local_evidence.py"

        def run(*args: str) -> subprocess.CompletedProcess:
            return subprocess.run(
                ["python3", str(tool), *args], capture_output=True, text=True, check=False
            )

        recorded = run(
            "scope-record", "--plan", str(self.plan_path), "--scope",
            str(self.projection_path), "--base", self.base, "--head", self.head,
        )
        self.assertEqual(0, recorded.returncode, recorded.stderr)
        self.assertEqual(self.receipt(), recorded.stdout.strip())
        outputs = ["--plan-out", "plan-out.json", "--scope-out", "scope-out.json",
                   "--reason-out", "reason.txt"]
        missed = run("scope-verify", "--base", self.base, "--head", self.head, *outputs)
        self.assertEqual(3, missed.returncode, missed.stderr)
        self.assertTrue(Path("reason.txt").read_text(encoding="utf-8").startswith("scope-missing:"))
        self.assertFalse(Path("plan-out.json").exists())
        self.add_scope_note(recorded.stdout.strip())
        hit = run("scope-verify", "--base", self.base, "--head", self.head, *outputs)
        self.assertEqual(0, hit.returncode, hit.stderr)
        self.assertEqual(schema.canonical(self.plan), Path("plan-out.json").read_text(encoding="utf-8"))
        self.assertEqual(schema.canonical(legacy_scope_document(self.plan)), Path("scope-out.json").read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()

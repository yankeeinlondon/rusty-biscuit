#!/usr/bin/env python3
"""Tests for exact-tree local CI evidence."""

from __future__ import annotations

import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from local_evidence import NOTES_PREFIX, record, verified_environment


class LocalEvidenceTests(unittest.TestCase):
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


if __name__ == "__main__":
    unittest.main()

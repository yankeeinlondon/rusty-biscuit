#!/usr/bin/env python3
"""Fixtures for the readable evidence-store listing (`just ci-evidence`).

Each fixture builds a real Git repository and a real evidence directory laid
out as the pre-push hook writes it, then asks `evidence_index.collect` what a
developer would see.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import evidence_index  # noqa: E402


def git(repo: Path, *args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=repo, check=True, capture_output=True, text=True
    ).stdout.strip()


def write_manifest(root: Path, sha: str, environment: str, records: list[dict]) -> Path:
    directory = root / sha / environment
    directory.mkdir(parents=True)
    manifest = directory / "manifest.jsonl"
    manifest.write_text("".join(json.dumps(r) + "\n" for r in records), encoding="utf-8")
    return manifest


def cell(package: str, exit_code: int = 0, duration_s: int = 10, present: bool = True) -> dict:
    return {
        "tier": "L1", "package": package, "xml": f"L1/{package}.xml", "exit_code": exit_code,
        "environment": "", "duration_s": duration_s, "report_present": present,
    }


class EvidenceIndexTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        base = Path(self.tmp.name)
        self.repo = base / "repo"
        self.root = base / "evidence"
        self.repo.mkdir()
        self.root.mkdir()
        git(self.repo, "init", "-q", "-b", "main")
        git(self.repo, "config", "user.email", "fixture@example.invalid")
        git(self.repo, "config", "user.name", "Fixture")
        git(self.repo, "config", "commit.gpgsign", "false")
        git(self.repo, "commit", "-q", "--allow-empty", "-m", "first commit")
        self.first = git(self.repo, "rev-parse", "HEAD")
        git(self.repo, "checkout", "-q", "-b", "feat/listing")
        git(self.repo, "commit", "-q", "--allow-empty", "-m", "second commit")
        self.second = git(self.repo, "rev-parse", "HEAD")

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_runs_are_named_by_branch_and_subject_newest_first(self) -> None:
        older = write_manifest(self.root, self.first, "macos-latest", [cell("sniff")])
        write_manifest(self.root, self.second, "macos-latest", [cell("sniff"), cell("tree-hugger", duration_s=50)])
        os.utime(older, (1_000_000, 1_000_000))

        entries = evidence_index.collect(self.root, self.repo)

        self.assertEqual([e["sha"] for e in entries], [self.second, self.first])
        self.assertEqual(entries[0]["branch"], "feat/listing")
        self.assertEqual(entries[0]["subject"], "second commit")
        self.assertEqual(entries[0]["cells"], 2)
        self.assertEqual(entries[0]["duration_s"], 60)
        self.assertEqual(entries[0]["result"], "pass")
        self.assertEqual(entries[1]["branch"], "main")

    def test_a_run_behind_its_branch_tip_is_named_relative_to_it(self) -> None:
        write_manifest(self.root, self.second, "macos-latest", [cell("a")])
        git(self.repo, "commit", "-q", "--allow-empty", "-m", "third commit")

        (entry,) = evidence_index.collect(self.root, self.repo)

        self.assertEqual(entry["branch"], "feat/listing~1")

    def test_a_failed_cell_outranks_a_missing_report(self) -> None:
        write_manifest(self.root, self.first, "macos-latest", [cell("a", present=False), cell("b", exit_code=101)])
        write_manifest(self.root, self.second, "macos-latest", [cell("a", present=False)])

        results = {e["sha"]: e["result"] for e in evidence_index.collect(self.root, self.repo)}

        self.assertEqual(results[self.first], "fail")
        self.assertEqual(results[self.second], "partial")

    def test_environments_combine_under_one_commit(self) -> None:
        write_manifest(self.root, self.second, "macos-latest", [cell("a")])
        write_manifest(self.root, self.second, "wsl2-ubuntu", [cell("a"), cell("b")])

        (entry,) = evidence_index.collect(self.root, self.repo)

        self.assertEqual([e["environment"] for e in entry["environments"]], ["macos-latest", "wsl2-ubuntu"])
        self.assertEqual(entry["cells"], 3)

    def test_an_unknown_commit_and_a_torn_manifest_line_are_still_listed(self) -> None:
        unknown = "f" * 40
        manifest = write_manifest(self.root, unknown, "macos-latest", [cell("a")])
        with manifest.open("a", encoding="utf-8") as handle:
            handle.write('{"tier": "L1", "pack')

        (entry,) = evidence_index.collect(self.root, self.repo)

        self.assertIsNone(entry["branch"])
        self.assertIsNone(entry["subject"])
        self.assertEqual(entry["cells"], 1)
        self.assertIn("(commit not in this clone)", evidence_index.render([entry], self.root, 120))

    def test_a_directory_without_a_manifest_is_empty(self) -> None:
        (self.root / self.first / "macos-latest").mkdir(parents=True)

        (entry,) = evidence_index.collect(self.root, self.repo)

        self.assertEqual(entry["result"], "empty")
        self.assertEqual(entry["environments"], [])

    def test_a_missing_root_lists_nothing(self) -> None:
        self.assertEqual(evidence_index.collect(self.root / "absent", self.repo), [])

    def test_the_root_override_matches_the_hook(self) -> None:
        previous = os.environ.get("BISCUIT_CI_EVIDENCE_DIR")
        os.environ["BISCUIT_CI_EVIDENCE_DIR"] = str(self.root)
        try:
            self.assertEqual(evidence_index.evidence_root(), self.root)
        finally:
            if previous is None:
                del os.environ["BISCUIT_CI_EVIDENCE_DIR"]
            else:
                os.environ["BISCUIT_CI_EVIDENCE_DIR"] = previous

    def test_render_truncates_only_the_subject(self) -> None:
        write_manifest(self.root, self.second, "macos-latest", [cell("a")])
        entries = evidence_index.collect(self.root, self.repo)
        entries[0]["subject"] = "x" * 300

        rendered = evidence_index.render(entries, self.root, 100).splitlines()

        self.assertTrue(all(len(line) <= 100 for line in rendered[2:]))
        self.assertIn(self.second[:10], rendered[3])
        self.assertTrue(rendered[3].endswith("…"))

    def test_durations_read_at_a_glance(self) -> None:
        self.assertEqual(evidence_index.format_duration(42), "42s")
        self.assertEqual(evidence_index.format_duration(600), "10m")
        self.assertEqual(evidence_index.format_duration(3725), "1h02m")


if __name__ == "__main__":
    unittest.main()

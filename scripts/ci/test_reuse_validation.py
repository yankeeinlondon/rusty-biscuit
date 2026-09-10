#!/usr/bin/env python3
"""Regression tests for the boundary that decides whether expensive CI runs."""
from __future__ import annotations

import copy
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock, patch

from reuse_validation import check_validation, find_validation, main, record_receipt

REPO = "owner/repo"
BASE, HEAD, MERGE, TREE = (c * 40 for c in "abcd")
RECEIPT = f"ci-validation-v1-{TREE}-{BASE}-{HEAD}"
PR = {
    "number": 70, "merged_at": "2026-09-08T00:00:00Z",
    "merge_commit_sha": MERGE,
    "base": {"sha": BASE, "ref": "main", "repo": {"full_name": REPO}},
    "head": {"sha": HEAD},
}
RUN = {
    "id": 295, "event": "pull_request", "path": ".github/workflows/ci.yml",
    "head_sha": HEAD, "status": "completed", "conclusion": "success",
    "repository": {"full_name": REPO}, "pull_requests": [{"number": 70}],
}


class ValidationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.pr = copy.deepcopy(PR)
        self.run = copy.deepcopy(RUN)
        self.artifact = {"name": RECEIPT, "expired": False}
        self.api = Mock(side_effect=lambda endpoint: self.response(endpoint))

    def response(self, endpoint: str):
        if "/commits/" in endpoint:
            return [self.pr]
        if "/workflows/ci.yml/runs?" in endpoint:
            return {"workflow_runs": [self.run]}
        if "/artifacts?" in endpoint:
            return {"artifacts": [self.artifact]}
        self.fail(f"unexpected API request: {endpoint}")

    def find(self, **kwargs):
        return find_validation(REPO, MERGE, kwargs.get("before", BASE),
                               kwargs.get("tree", TREE), self.api)[0]

    def test_matching_merge_reuses_successful_pr_run(self) -> None:
        self.assertEqual(self.find(), "295")
        self.assertIn(f"head_sha={HEAD}", self.api.call_args_list[1].args[0])
        self.assertIn(f"name={RECEIPT}", self.api.call_args_list[2].args[0])

    def test_different_tree_or_integration_base_runs_ci(self) -> None:
        for changed in ({"tree": "e" * 40}, {"before": "f" * 40}):
            with self.subTest(changed=changed):
                self.assertEqual(self.find(**changed), "")

    def test_direct_push_and_wrong_pr_associations_run_ci(self) -> None:
        for change in (
            {"merged_at": None}, {"merge_commit_sha": HEAD},
            {"base": {"ref": "feature", "repo": {"full_name": REPO}}},
            {"base": {"ref": "main", "repo": {"full_name": "other/repo"}}},
        ):
            with self.subTest(change=change):
                self.pr = {**copy.deepcopy(PR), **change}
                self.assertEqual(self.find(), "")

    def test_untrusted_or_unsuccessful_runs_cannot_supply_receipts(self) -> None:
        for change in (
            {"event": "push"}, {"path": ".github/workflows/other.yml"},
            {"head_sha": MERGE}, {"repository": {"full_name": "other/repo"}},
            {"status": "in_progress"}, {"status": "queued"},
            {"conclusion": "failure"}, {"conclusion": "cancelled"},
            {"conclusion": "skipped"}, {"conclusion": None},
        ):
            with self.subTest(change=change):
                self.run = {**copy.deepcopy(RUN), **change}
                self.assertEqual(self.find(), "")

    def test_run_level_pr_associations_are_not_required(self) -> None:
        # GitHub can return an empty pull_requests array for a run even when
        # the commit endpoint authoritatively identifies the merged PR.
        self.run["pull_requests"] = []
        self.assertEqual(self.find(), "295")

    def test_newer_failed_run_cannot_resurrect_old_success(self) -> None:
        self.api.side_effect = [
            [PR], {"workflow_runs": [RUN, {**RUN, "id": 296, "conclusion": "failure"}]},
        ]
        self.assertEqual(self.find(), "")
        self.assertEqual(self.api.call_count, 2)

    def test_missing_expired_and_old_protocol_receipts_run_ci(self) -> None:
        for artifact in ({}, {"name": RECEIPT, "expired": True},
                         {"name": RECEIPT.replace("v1", "v0"), "expired": False}):
            with self.subTest(artifact=artifact):
                self.artifact = artifact
                self.assertEqual(self.find(), "")

    def test_empty_api_results_run_ci(self) -> None:
        for responses in ([[]], [[PR], {"workflow_runs": []}],
                          [[PR], {"workflow_runs": [RUN]}, {"artifacts": []}]):
            with self.subTest(responses=responses):
                self.api.side_effect = responses
                self.assertEqual(self.find(), "")

    def test_new_branch_does_not_query_github(self) -> None:
        self.assertEqual(self.find(before="0" * 40), "")
        self.api.assert_not_called()

    def test_api_and_git_errors_fall_back_to_normal_ci(self) -> None:
        for error in (PermissionError(), subprocess.TimeoutExpired("gh", 15),
                      json.JSONDecodeError("bad response", "", 0), KeyError("missing")):
            with self.subTest(error=error), patch(
                "reuse_validation.git_revision", side_effect=[MERGE, TREE]
            ):
                self.api.side_effect = error
                run, reason = check_validation(
                    "push", "refs/heads/main", {"before": BASE}, REPO, MERGE, self.api,
                )
                self.assertEqual(run, "")
                self.assertIn("could not be verified", reason)
        with patch("reuse_validation.git_revision", side_effect=OSError()):
            self.assertEqual(check_validation(
                "push", "refs/heads/main", {"before": BASE}, REPO, MERGE, self.api,
            )[0], "")

    def test_manual_pr_and_other_branch_runs_never_reuse(self) -> None:
        with patch("reuse_validation.git_revision") as git:
            for event, ref in (("workflow_dispatch", "refs/heads/main"),
                               ("pull_request", "refs/pull/70/merge"),
                               ("push", "refs/heads/feature")):
                self.assertEqual(check_validation(event, ref, {}, REPO, MERGE, self.api)[0], "")
            git.assert_not_called()
            self.api.assert_not_called()

    def test_wrong_checkout_runs_ci_without_querying_github(self) -> None:
        with patch("reuse_validation.git_revision", return_value=HEAD):
            self.assertEqual(check_validation(
                "push", "refs/heads/main", {"before": BASE}, REPO, MERGE, self.api,
            )[0], "")
        self.api.assert_not_called()


class ReceiptTests(unittest.TestCase):
    def test_receipt_records_checkout_tree_instead_of_pr_head(self) -> None:
        with patch("reuse_validation.git_revision", side_effect=[MERGE, BASE, HEAD, TREE]):
            self.assertEqual(record_receipt({"pull_request": PR}, MERGE), RECEIPT)

    def test_receipt_rejects_overridden_checkout_or_wrong_merge_parents(self) -> None:
        for revisions in ([HEAD], [MERGE, HEAD], [MERGE, BASE, BASE]):
            with self.subTest(revisions=revisions), patch(
                "reuse_validation.git_revision", side_effect=revisions
            ):
                with self.assertRaises(ValueError):
                    record_receipt({"pull_request": PR}, MERGE)

    def test_check_writes_reuse_output_and_original_run_link(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            event = root / "event.json"
            event.write_text(json.dumps({"before": BASE}), encoding="utf-8")
            output, summary = root / "output", root / "summary"
            env = {
                "GITHUB_EVENT_PATH": str(event), "GITHUB_SHA": MERGE,
                "GITHUB_EVENT_NAME": "push", "GITHUB_REF": "refs/heads/main",
                "GITHUB_REPOSITORY": REPO, "GITHUB_SERVER_URL": "https://github.com",
                "GITHUB_OUTPUT": str(output), "GITHUB_STEP_SUMMARY": str(summary),
            }
            with patch.dict(os.environ, env), patch("sys.argv", ["reuse_validation.py", "check"]), \
                    patch("reuse_validation.check_validation", return_value=("295", "Matched.")), \
                    patch("reuse_validation.git_revision", return_value=TREE):
                main()
            self.assertEqual(output.read_text(encoding="utf-8"), "reuse=true\nrun_id=295\n")
            self.assertIn("https://github.com/owner/repo/actions/runs/295",
                          summary.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()

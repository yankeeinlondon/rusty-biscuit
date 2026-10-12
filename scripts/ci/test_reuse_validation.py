#!/usr/bin/env python3
"""Regression tests for the boundary that decides whether expensive CI runs."""
from __future__ import annotations

import copy
import io
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock, patch

import reuse_validation
import schema
from reuse_validation import (
    certification_document, check_validation, count_audits, find_validation, main,
    nightly_anchors_summary, pending_waiter, record_receipt, resolve_anchors,
)

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

    def test_an_unfinished_pr_run_is_awaited_and_then_reused(self) -> None:
        pending = {**RUN, "status": "in_progress", "conclusion": None}
        self.api.side_effect = [
            [PR], {"workflow_runs": [{**pending, "status": "queued"}]},
            {"workflow_runs": [pending]}, {"workflow_runs": [RUN]},
            {"artifacts": [self.artifact]},
        ]
        waits = Mock(return_value=True)
        run, _ = find_validation(REPO, MERGE, BASE, TREE, self.api, waits)
        self.assertEqual(run, "295")
        self.assertEqual(waits.call_count, 2)

    def test_an_awaited_run_that_fails_runs_ci(self) -> None:
        self.api.side_effect = [
            [PR], {"workflow_runs": [{**RUN, "status": "in_progress", "conclusion": None}]},
            {"workflow_runs": [{**RUN, "conclusion": "failure"}]},
        ]
        self.assertEqual(find_validation(REPO, MERGE, BASE, TREE, self.api, Mock(return_value=True))[0], "")

    def test_an_unfinished_run_past_the_wait_budget_runs_ci(self) -> None:
        self.api.side_effect = [
            [PR], {"workflow_runs": [{**RUN, "status": "in_progress", "conclusion": None}]},
        ]
        waits = Mock(return_value=False)
        run, reason = find_validation(REPO, MERGE, BASE, TREE, self.api, waits)
        self.assertEqual(run, "")
        self.assertIn("not completed", reason)
        waits.assert_called_once()
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


class PendingWaiterTests(unittest.TestCase):
    def test_a_spent_budget_stops_waiting_without_sleeping(self) -> None:
        with patch("reuse_validation.time.sleep") as sleep:
            self.assertFalse(pending_waiter(0)())
            sleep.assert_not_called()

    def test_each_wait_sleeps_at_most_one_interval(self) -> None:
        with patch("reuse_validation.time.sleep") as sleep:
            self.assertTrue(pending_waiter(10, interval=60)())
            self.assertLessEqual(sleep.call_args.args[0], 60)


class ReceiptTests(unittest.TestCase):
    def test_receipt_records_the_tree_of_the_pr_head_checkout(self) -> None:
        with patch("reuse_validation.git_revision", side_effect=[HEAD, TREE]):
            self.assertEqual(record_receipt({"pull_request": PR}), RECEIPT)

    def test_receipt_rejects_a_checkout_that_is_not_the_pr_head(self) -> None:
        # GitHub's synthetic merge is what `pull_request` checks out by default;
        # `ci.yml` overrides that with the head, and a receipt from any other
        # revision would name a tree this run did not test.
        for checkout in (MERGE, BASE):
            with self.subTest(checkout=checkout), patch(
                "reuse_validation.git_revision", return_value=checkout
            ):
                with self.assertRaises(ValueError):
                    record_receipt({"pull_request": PR})

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


WINDOWS, WSL2, LINUX = "windows-latest", "wsl2-ubuntu", "ubuntu-latest"
SCHEDULED = ["schedule", "workflow_dispatch"]
TABLE_NOW = [
    {"name": LINUX, "events": ["pull_request", "push", "workflow_dispatch"]},
    {"name": WINDOWS, "events": SCHEDULED},
    {"name": WSL2, "events": SCHEDULED},
]
#: The table as committed before the move: only WSL2 was scheduled.
TABLE_BEFORE = [
    {"name": LINUX, "events": ["pull_request", "push", "schedule", "workflow_dispatch"]},
    {"name": WINDOWS, "events": ["pull_request", "push", "workflow_dispatch"]},
    {"name": WSL2, "events": SCHEDULED},
]


def tiny_plan(anchors: list[dict], covered: dict[str, list[tuple[str, str]]] | None = None) -> dict:
    """The few plan fields a certification reads, with a cell per `(package, gate)`."""
    cells = []
    for environment, keys in (covered or {}).items():
        for package, gate in keys:
            cells.append({
                "package": package, "environment": environment, "gate": gate,
                "execution": "execute", "state": "pending",
            })
    return {"head": "e" * 40, "cells": cells, "coverage_anchors": anchors,
            "packages": [{"area": "a", "gates": ["L1"]}]}


def anchor_entry(environment: str, **overrides) -> dict:
    entry = {
        "environment": environment, "mode": "full", "anchor": None,
        "reason": "bootstrap-no-run",
        "source": {"kind": "none", "run_id": None, "head": None},
        "policy_delta": [], "inherited_cells": [],
    }
    entry.update(overrides)
    return entry


class NightlyAnchorTests(unittest.TestCase):
    """Each scheduled environment is anchored to the newest successful nightly
    that proved THAT environment, never to a run-wide base."""

    OLDER, NEWER, FOREIGN = ("1" * 40, "2" * 40, "3" * 40)

    def run_record(self, run_id: int, sha: str, **overrides) -> dict:
        return {
            "id": run_id, "event": "schedule", "path": ".github/workflows/ci.yml",
            "head_sha": sha, "head_branch": "main", "status": "completed",
            "conclusion": "success", "repository": {"full_name": REPO}, **overrides,
        }

    def certification(self, environment: str, run: dict, **edits) -> dict:
        """A real recorder-built certification: the control every edit starts from."""
        record = next(e for e in TABLE_NOW if e["name"] == environment)
        plan = tiny_plan([anchor_entry(environment)], {environment: [("pkg", "L1")]})
        document = certification_document(
            plan, environment, run_id=run["id"], attempt=1, head=run["head_sha"],
            tree="9" * 40, table=TABLE_NOW, audits=(1, 1), gate_passed=True,
        )
        self.assertEqual([], schema.validate_certification(document))
        self.assertEqual(schema.policy_digest(record), document["environments_digest"])
        document.update(edits)
        return document

    def resolve(
        self, runs, *, certifications=None, ancestors=None, legacy_table=None,
        audits=("success",), table=None, api_error=None,
    ):
        """Drive `resolve_anchors` with a fake GitHub.

        `certifications` maps (run id, environment) to the document (or a
        callable that raises) downloaded for that artifact; an environment with
        no entry has no artifact for that run.
        """
        certifications = certifications or {}
        ancestors = {self.OLDER, self.NEWER} if ancestors is None else ancestors
        table = TABLE_NOW if table is None else table

        def api(endpoint: str):
            if api_error is not None:
                raise api_error
            if "/workflows/ci.yml/runs?" in endpoint:
                self.assertIn("event=schedule", endpoint)
                self.assertIn("branch=main", endpoint)
                self.assertIn("status=success", endpoint)
                page = int(endpoint.split("&page=")[1])
                size = reuse_validation.NIGHTLY_PAGE_SIZE
                return {"workflow_runs": runs[(page - 1) * size: page * size], "total_count": len(runs)}
            if "/artifacts?name=" in endpoint:
                run_id = int(endpoint.split("/runs/")[1].split("/")[0])
                name = endpoint.split("name=")[1].split("&")[0]
                for (rid, environment), _ in certifications.items():
                    if rid == run_id and name.startswith(f"ci-coverage-v1-{environment}-"):
                        index = [WINDOWS, WSL2, LINUX].index(environment) + 1
                        return {"artifacts": [{"name": name, "expired": False, "id": rid * 100 + index}]}
                return {"artifacts": []}
            if "/jobs?" in endpoint:
                return {"jobs": [
                    {"name": f"area-ci (a{n}) / coverage-audit", "conclusion": result}
                    for n, result in enumerate(audits)
                ]}
            self.fail(f"unexpected API request: {endpoint}")

        def download(repo: str, artifact_id: int):
            run_id, index = divmod(artifact_id, 100)
            environment = [WINDOWS, WSL2, LINUX][index - 1]
            document = certifications[(run_id, environment)]
            if isinstance(document, Exception):
                raise document
            return document

        document = resolve_anchors(
            REPO, table, api, lambda sha: sha in ancestors,
            lambda sha: legacy_table if legacy_table is not None else None, download,
        )
        self.assertEqual([], schema.validate_anchor_input(document))
        return {entry["environment"]: entry for entry in document["anchors"]}

    def test_no_successful_nightly_is_a_bootstrap_for_every_scheduled_environment(self) -> None:
        anchors = self.resolve([])
        self.assertEqual({WINDOWS, WSL2}, set(anchors))
        for entry in anchors.values():
            self.assertEqual(("full", None, "bootstrap-no-run"), (entry["mode"], entry["anchor"], entry["reason"]))

    def test_an_api_failure_is_a_visible_full_scope_fallback_not_a_bootstrap(self) -> None:
        anchors = self.resolve([], api_error=RuntimeError("rate limited"))
        self.assertEqual({"api-failure"}, {entry["reason"] for entry in anchors.values()})
        self.assertEqual({"full"}, {entry["mode"] for entry in anchors.values()})

    def test_a_certified_dual_environment_nightly_anchors_both_environments(self) -> None:
        run = self.run_record(11, self.NEWER)
        anchors = self.resolve(
            [run], certifications={
                (11, WINDOWS): self.certification(WINDOWS, run),
                (11, WSL2): self.certification(WSL2, run),
            },
        )
        for entry in anchors.values():
            self.assertEqual("incremental", entry["mode"])
            self.assertEqual(self.NEWER, entry["anchor"])
            self.assertEqual("certified-ancestor", entry["reason"])
            self.assertEqual([{"package": "pkg", "gate": "L1"}], entry["covered_cells"])
            self.assertEqual("nightly-certified", entry["source"]["kind"])

    def test_a_legacy_wsl2_only_success_anchors_wsl2_and_leaves_windows_in_full_scope(self) -> None:
        run = self.run_record(10, self.OLDER)
        anchors = self.resolve([run], legacy_table=TABLE_BEFORE)
        self.assertEqual(
            ("incremental", self.OLDER, "legacy-wsl2-ancestor", None),
            tuple(anchors[WSL2][key] for key in ("mode", "anchor", "reason", "covered_cells")),
        )
        self.assertEqual(("full", None, "policy-uncertified"), (
            anchors[WINDOWS]["mode"], anchors[WINDOWS]["anchor"], anchors[WINDOWS]["reason"],
        ))

    def test_a_legacy_run_with_a_skipped_audit_earns_neither_anchor(self) -> None:
        run = self.run_record(10, self.OLDER)
        anchors = self.resolve([run], legacy_table=TABLE_BEFORE, audits=("success", "skipped"))
        self.assertEqual({"policy-uncertified"}, {e["reason"] for e in anchors.values()})
        anchors = self.resolve([run], legacy_table=TABLE_BEFORE, audits=())
        self.assertEqual({"policy-uncertified"}, {e["reason"] for e in anchors.values()}, "no audit at all")

    def test_a_legacy_run_without_readable_policy_provenance_earns_nothing(self) -> None:
        run = self.run_record(10, self.OLDER)
        anchors = self.resolve([run], legacy_table=None)
        self.assertEqual({"policy-uncertified"}, {e["reason"] for e in anchors.values()})

    def test_the_newest_qualifying_ancestor_wins(self) -> None:
        older, newer = self.run_record(10, self.OLDER), self.run_record(11, self.NEWER)
        anchors = self.resolve([older, newer], legacy_table=TABLE_BEFORE)
        self.assertEqual(self.NEWER, anchors[WSL2]["anchor"])
        self.assertEqual(11, anchors[WSL2]["source"]["run_id"])

    def test_failed_incomplete_foreign_and_non_ancestor_runs_never_anchor(self) -> None:
        runs = [
            self.run_record(10, self.OLDER),
            self.run_record(11, self.NEWER, conclusion="failure"),
            self.run_record(12, self.NEWER, status="in_progress", conclusion=None),
            self.run_record(13, self.NEWER, repository={"full_name": "other/repo"}),
            self.run_record(14, self.NEWER, path=".github/workflows/fuzz-nightly.yml"),
            self.run_record(15, self.NEWER, event="workflow_dispatch"),
            self.run_record(16, self.NEWER, head_branch="topic"),
            self.run_record(17, self.FOREIGN),
        ]
        anchors = self.resolve(runs, legacy_table=TABLE_BEFORE)
        self.assertEqual((self.OLDER, 10), (anchors[WSL2]["anchor"], anchors[WSL2]["source"]["run_id"]))

    def test_only_non_ancestor_runs_report_no_qualifying_ancestor(self) -> None:
        anchors = self.resolve([self.run_record(17, self.FOREIGN)], legacy_table=TABLE_BEFORE)
        self.assertEqual({"no-qualifying-ancestor"}, {e["reason"] for e in anchors.values()})

    def test_an_exhausted_search_is_reported_as_such(self) -> None:
        runs = [self.run_record(100 + n, self.FOREIGN) for n in range(3)]
        with patch.object(reuse_validation, "NIGHTLY_PAGE_SIZE", 2), \
                patch.object(reuse_validation, "NIGHTLY_MAX_PAGES", 1):
            anchors = self.resolve(runs)
        self.assertEqual({"search-exhausted"}, {e["reason"] for e in anchors.values()})

    def test_a_qualifying_run_found_before_the_budget_ends_is_not_exhausted(self) -> None:
        runs = [self.run_record(100, self.NEWER), self.run_record(99, self.FOREIGN), self.run_record(98, self.FOREIGN)]
        with patch.object(reuse_validation, "NIGHTLY_PAGE_SIZE", 2), \
                patch.object(reuse_validation, "NIGHTLY_MAX_PAGES", 1):
            anchors = self.resolve(runs, legacy_table=TABLE_BEFORE)
        self.assertEqual("legacy-wsl2-ancestor", anchors[WSL2]["reason"])

    def test_a_run_certified_for_another_environment_is_not_a_legacy_credit(self) -> None:
        # Windows is certified; WSL2 has no certification of that run, so the
        # legacy path (which presumes the run predates certifications) is shut.
        run = self.run_record(11, self.NEWER)
        anchors = self.resolve(
            [run], certifications={(11, WINDOWS): self.certification(WINDOWS, run)},
            legacy_table=TABLE_BEFORE,
        )
        self.assertEqual("certified-ancestor", anchors[WINDOWS]["reason"])
        self.assertEqual(("full", "policy-uncertified"), (anchors[WSL2]["mode"], anchors[WSL2]["reason"]))

    def test_certification_input_matrix_every_edit_loses_the_anchor(self) -> None:
        """One edit per cell of the certification document; the control is credited."""
        run = self.run_record(11, self.NEWER)
        control = self.certification(WSL2, run)
        credited = self.resolve([run], certifications={(11, WSL2): control, (11, WINDOWS): self.certification(WINDOWS, run)})
        self.assertEqual("certified-ancestor", credited[WSL2]["reason"], "control row")
        edits: dict[str, object] = {}
        for field in schema.CERTIFICATION_FIELDS:
            absent = {k: v for k, v in control.items() if k != field}
            edits[f"{field} absent"] = absent
            edits[f"{field} null"] = {**control, field: None}
            edits[f"{field} wrong type"] = {**control, field: {"x": 1} if not isinstance(control[field], dict) else 3}
            edits[f"{field} empty"] = {**control, field: [] if not isinstance(control[field], list) else "x"}
        edits["unknown field"] = {**control, "extra": 1}
        edits["other run"] = {**control, "run_id": 999}
        edits["other head"] = {**control, "head": self.OLDER}
        edits["other environment"] = {**control, "environment": WINDOWS}
        edits["other digest"] = {**control, "environments_digest": "f" * 64}
        edits["not certified"] = {**control, "certified": False}
        edits["audit skipped"] = {**control, "audits": {"required": 2, "passed": 1}}
        edits["duplicate key"] = ValueError("duplicate key 'head'")
        edits["trailing content"] = ValueError("Extra data")
        edits["unreadable artifact"] = OSError("zip")
        for label, edited in edits.items():
            with self.subTest(edit=label):
                anchors = self.resolve([run], certifications={(11, WSL2): edited})
                self.assertEqual(
                    ("full", None), (anchors[WSL2]["mode"], anchors[WSL2]["anchor"]),
                    "a malformed or mismatched certification never grants an anchor",
                )
                self.assertEqual("policy-uncertified", anchors[WSL2]["reason"])

    def test_an_expired_or_duplicated_certification_artifact_earns_nothing(self) -> None:
        run = self.run_record(11, self.NEWER)

        def api(endpoint: str):
            if "/workflows/ci.yml/runs?" in endpoint:
                return {"workflow_runs": [run], "total_count": 1}
            name = endpoint.split("name=")[1].split("&")[0]
            return {"artifacts": [{"name": name, "expired": True, "id": 1}]}

        document = resolve_anchors(REPO, TABLE_NOW, api, lambda sha: True, lambda sha: None,
                                   Mock(side_effect=AssertionError("must not download")))
        self.assertEqual({"policy-uncertified"}, {e["reason"] for e in document["anchors"]})

    def test_a_table_that_schedules_nothing_has_no_anchors(self) -> None:
        document = resolve_anchors(REPO, [TABLE_NOW[0]], Mock(return_value={"workflow_runs": [], "total_count": 0}))
        self.assertEqual([], document["anchors"])

    def test_the_summary_names_each_environments_anchor_and_reason(self) -> None:
        run = self.run_record(10, self.OLDER)
        document = {"anchors": list(self.resolve([run], legacy_table=TABLE_BEFORE).values())}
        text = nightly_anchors_summary(document)
        self.assertIn(f"`{WSL2}`: since {self.OLDER[:9]} (run 10) — legacy-wsl2-ancestor", text)
        self.assertIn(f"`{WINDOWS}`: the full gated workspace — policy-uncertified", text)

    def test_the_command_prints_the_anchor_document(self) -> None:
        env = {"GITHUB_REPOSITORY": REPO}
        document = {"schema_version": 1, "anchors": [anchor_entry(WSL2) | {"covered_cells": None}]}
        with patch.dict(os.environ, env), patch(
            "reuse_validation.resolve_anchors", return_value=document
        ), patch("sys.argv", ["reuse_validation.py", "nightly-anchors"]), patch(
            "sys.stdout", new_callable=io.StringIO
        ) as out, patch("sys.stderr", new_callable=io.StringIO):
            main()
        self.assertEqual(document, json.loads(out.getvalue()))


class CertificationTests(unittest.TestCase):
    def plan(self) -> dict:
        anchors = [anchor_entry(
            WSL2, mode="incremental", anchor="1" * 40, reason="certified-ancestor",
            source={"kind": "nightly-certified", "run_id": 7, "head": "1" * 40},
            inherited_cells=[{"package": "old", "gate": "L1"}],
        )]
        plan = tiny_plan(anchors, {WSL2: [("changed", "L1")]})
        plan["cells"].append({
            "package": "reused", "environment": WSL2, "gate": "L1",
            "execution": "reuse", "state": "reused",
        })
        plan["cells"].append({
            "package": "gap", "environment": WSL2, "gate": "L2",
            "execution": "omit", "state": "accepted-gap",
        })
        return plan

    def build(self, **overrides) -> dict:
        arguments = dict(
            run_id=21, attempt=1, head="e" * 40, tree="9" * 40, table=TABLE_NOW,
            audits=(2, 2), gate_passed=True,
        )
        arguments.update(overrides)
        return certification_document(self.plan(), WSL2, **arguments)

    def test_coverage_is_transitive_and_gaps_cover_nothing(self) -> None:
        document = self.build()
        self.assertEqual([], schema.validate_certification(document))
        self.assertEqual(
            [("changed", "L1"), ("old", "L1"), ("reused", "L1")],
            [(c["package"], c["gate"]) for c in document["covered_cells"]],
        )
        self.assertEqual(
            {"executing": 1, "reused": 1, "accepted_gap": 1, "inherited": 1}, document["cells"],
        )
        self.assertTrue(document["certified"])

    def test_a_failed_gate_or_an_unfinished_audit_never_certifies(self) -> None:
        self.assertFalse(self.build(gate_passed=False)["certified"])
        document = self.build(audits=(2, 1))
        self.assertFalse(document["certified"])
        self.assertEqual([], schema.validate_certification(document), "honestly uncertified, still well-formed")

    def test_a_plan_without_the_environments_anchor_cannot_be_certified(self) -> None:
        with self.assertRaises(ValueError):
            certification_document(
                self.plan(), WINDOWS, run_id=21, attempt=1, head="e" * 40, tree="9" * 40,
                table=TABLE_NOW, audits=(1, 1), gate_passed=True,
            )

    def test_only_successful_audit_jobs_count(self) -> None:
        jobs = [
            {"name": "area-ci (a) / coverage-audit", "conclusion": "success"},
            {"name": "area-ci (b) / coverage-audit", "conclusion": "skipped"},
            {"name": "area-ci (b) / test", "conclusion": "success"},
        ]
        self.assertEqual((2, 1), count_audits(jobs, 2))

    def test_the_command_refuses_a_plan_with_a_duplicate_key_or_trailing_content(self) -> None:
        text = json.dumps(self.plan())
        self.assertIn('"head"', text)
        for label, edited in (("duplicate key", text.replace("{", '{"head": "x", ', 1)),
                              ("trailing content", text + "{}")):
            with self.subTest(label), tempfile.TemporaryDirectory() as temp:
                plan_path = Path(temp) / "plan.json"
                plan_path.write_text(edited, encoding="utf-8")
                with patch.dict(os.environ, {"GITHUB_REPOSITORY": REPO}), patch(
                    "sys.argv", ["reuse_validation.py", "certify", "--plan", str(plan_path),
                                 "--out", temp, "--gate-result", "success"],
                ), self.assertRaises(SystemExit) as raised:
                    main()
                self.assertIn("unreadable", str(raised.exception))

    def test_the_command_writes_one_artifact_per_anchored_environment(self) -> None:
        plan = self.plan()
        plan["packages"] = [{"area": "a", "gates": ["L1"]}, {"area": "b", "gates": ["L1"]}]
        jobs = [{"name": f"area-ci ({a}) / coverage-audit", "conclusion": "success"} for a in "ab"]
        with tempfile.TemporaryDirectory() as temp:
            plan_path = Path(temp) / "plan.json"
            plan_path.write_text(json.dumps(plan), encoding="utf-8")
            env = {"GITHUB_REPOSITORY": REPO, "GITHUB_RUN_ID": "21", "GITHUB_RUN_ATTEMPT": "2"}
            with patch.dict(os.environ, env), patch(
                "sys.argv", ["reuse_validation.py", "certify", "--plan", str(plan_path),
                             "--out", temp, "--gate-result", "success"],
            ), patch("reuse_validation.run_jobs", return_value=jobs), patch(
                "reuse_validation.git_revision", return_value="9" * 40
            ), patch("schema.validate_resolved_plan", return_value=[]), patch(
                "sys.stdout", new_callable=io.StringIO
            ) as out:
                main()
            written = json.loads(out.getvalue())
            self.assertEqual([WSL2], [entry["environment"] for entry in written])
            document = json.loads(
                (Path(temp) / written[0]["name"] / "certification.json").read_text(encoding="utf-8")
            )
        self.assertEqual([], schema.validate_certification(document))
        self.assertEqual((21, 2, True), (document["run_id"], document["attempt"], document["certified"]))
        self.assertTrue(written[0]["name"].startswith(f"ci-coverage-v1-{WSL2}-{'e' * 40}-"))


if __name__ == "__main__":
    unittest.main()

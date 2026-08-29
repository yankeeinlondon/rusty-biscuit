#!/usr/bin/env python3
"""Tests for runner-loss attribution and the one-shot retry decision."""
from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from runner_loss import (
    classify,
    interrupted_step,
    parse_job_name,
    should_rerun,
    status_directory,
    synthesize_status,
)

# Trimmed from run 33209311538 attempt 1 (2026-08-28): the playa-cli WSL2 leg
# whose runner died during guest provisioning, exactly as GitHub reported it.
LOST_WSL_JOB = {
    "id": 99003990899,
    "name": "playa-cli / wsl2 (playa-cli) / test (playa-cli on wsl2-ubuntu)",
    "status": "completed",
    "conclusion": "failure",
    "steps": [
        {"name": "Set up job", "status": "completed", "conclusion": "success"},
        {"name": "Run actions/checkout@v4", "status": "completed", "conclusion": "success"},
        {"name": "Download the nextest archive", "status": "completed", "conclusion": "success"},
        {"name": "Provision the WSL2 guest", "status": "in_progress", "conclusion": None},
        {"name": "Let the endpoint settle before a second attempt", "status": "pending", "conclusion": None},
        {"name": "L1 tests from the archive", "status": "pending", "conclusion": None},
    ],
}
LOST_ANNOTATION = [
    {
        "annotation_level": "failure",
        "message": (
            "The hosted runner lost communication with the server. Anything in "
            "your workflow that terminates the runner process, starves it for "
            "CPU/Memory, or blocks its network access can cause this error."
        ),
    }
]
REAL_FAILURE_JOB = {
    "id": 98559525155,
    "name": "claudine-cli / test-l2 (claudine-cli on macos-latest)",
    "status": "completed",
    "conclusion": "failure",
    "steps": [{"name": "L2 tests", "status": "completed", "conclusion": "failure"}],
}
REAL_FAILURE_ANNOTATION = [
    {"annotation_level": "failure", "message": "Process completed with exit code 100."}
]
VERDICT_JOB = {"id": 1, "name": "ci-verdict", "status": "completed", "conclusion": "failure"}
PASSING_JOB = {
    "id": 2,
    "name": "darkmatter / test (darkmatter on ubuntu-latest)",
    "status": "completed",
    "conclusion": "success",
}


class JobNameTests(unittest.TestCase):
    def test_wsl2_leg_maps_to_l1_on_wsl2_ubuntu(self) -> None:
        self.assertEqual(
            {"package": "playa-cli", "job": "L1", "environment": "wsl2-ubuntu"},
            parse_job_name(LOST_WSL_JOB["name"]),
        )

    def test_native_legs_map_to_their_status_names(self) -> None:
        cases = {
            "sniff / test (sniff on windows-latest)": ("sniff", "L1", "windows-latest"),
            "dmls / test-l2 (dmls on macos-latest)": ("dmls", "L2", "macos-latest"),
            "darkmatter / test-browser (darkmatter on ubuntu-latest)": ("darkmatter", "browser", "ubuntu-latest"),
            "biscuit-file / check (biscuit-file on windows-latest)": ("biscuit-file", "check", "windows-latest"),
            "claudine-cli / lint (claudine-cli)": ("claudine-cli", "lint", None),
        }
        for name, (package, job, environment) in cases.items():
            self.assertEqual(
                {"package": package, "job": job, "environment": environment},
                parse_job_name(name),
                name,
            )

    def test_non_producer_and_archive_jobs_map_to_nothing(self) -> None:
        for name in [
            "ci-verdict",
            "Determine affected scope",
            "preflight (windows-latest)",
            "playa-cli / wsl2 (playa-cli) / archive (playa-cli for wsl2)",
        ]:
            self.assertIsNone(parse_job_name(name), name)

    def test_status_directory_omits_a_missing_environment(self) -> None:
        self.assertEqual(
            "status-claudine-cli-lint",
            status_directory({"package": "claudine-cli", "job": "lint", "environment": None}),
        )
        self.assertEqual(
            "status-playa-cli-L1-wsl2-ubuntu",
            status_directory({"package": "playa-cli", "job": "L1", "environment": "wsl2-ubuntu"}),
        )


class ClassifyTests(unittest.TestCase):
    def test_lost_runner_is_attributed_to_the_interrupted_step(self) -> None:
        result = classify([LOST_WSL_JOB, PASSING_JOB], {LOST_WSL_JOB["id"]: LOST_ANNOTATION})
        self.assertEqual([], result["other_failures"])
        [record] = result["runner_lost"]
        self.assertEqual("Provision the WSL2 guest", record["step"])
        self.assertEqual("wsl2-ubuntu", record["cell"]["environment"])

    def test_a_real_failure_is_not_a_lost_runner(self) -> None:
        result = classify(
            [REAL_FAILURE_JOB], {REAL_FAILURE_JOB["id"]: REAL_FAILURE_ANNOTATION}
        )
        self.assertEqual([], result["runner_lost"])
        self.assertEqual([REAL_FAILURE_JOB["name"]], result["other_failures"])

    def test_the_verdict_job_never_counts_as_a_failure(self) -> None:
        result = classify([LOST_WSL_JOB, VERDICT_JOB], {LOST_WSL_JOB["id"]: LOST_ANNOTATION})
        self.assertEqual([], result["other_failures"])

    def test_no_running_step_reports_none(self) -> None:
        self.assertIsNone(interrupted_step({"steps": [{"status": "completed", "conclusion": "success"}]}))


class RerunDecisionTests(unittest.TestCase):
    def lost_only(self) -> dict:
        return classify([LOST_WSL_JOB, VERDICT_JOB], {LOST_WSL_JOB["id"]: LOST_ANNOTATION})

    def test_first_attempt_with_only_lost_runners_reruns(self) -> None:
        rerun, reason = should_rerun(self.lost_only(), attempt=1)
        self.assertTrue(rerun, reason)

    def test_second_attempt_never_reruns(self) -> None:
        rerun, reason = should_rerun(self.lost_only(), attempt=2)
        self.assertFalse(rerun)
        self.assertIn("human", reason)

    def test_a_real_failure_alongside_a_lost_runner_blocks_the_rerun(self) -> None:
        classification = classify(
            [LOST_WSL_JOB, REAL_FAILURE_JOB],
            {LOST_WSL_JOB["id"]: LOST_ANNOTATION, REAL_FAILURE_JOB["id"]: REAL_FAILURE_ANNOTATION},
        )
        rerun, reason = should_rerun(classification, attempt=1)
        self.assertFalse(rerun)
        self.assertIn("claudine-cli / test-l2", reason)

    def test_nothing_lost_means_nothing_to_rerun(self) -> None:
        rerun, _ = should_rerun(classify([REAL_FAILURE_JOB], {REAL_FAILURE_JOB["id"]: []}), 1)
        self.assertFalse(rerun)


class SynthesizeStatusTests(unittest.TestCase):
    def test_writes_the_status_the_dead_producer_could_not(self) -> None:
        records = classify([LOST_WSL_JOB], {LOST_WSL_JOB["id"]: LOST_ANNOTATION})["runner_lost"]
        with tempfile.TemporaryDirectory() as tmp:
            [path] = synthesize_status(records, Path(tmp))
            self.assertTrue(path.endswith("status-playa-cli-L1-wsl2-ubuntu/status.json"))
            status = json.loads(Path(path).read_text())
            self.assertEqual("failure", status["result"])
            self.assertEqual("wsl2-ubuntu", status["environment"])
            self.assertIn("Provision the WSL2 guest", status["detail"])
            self.assertIn("lost communication", status["detail"])

    def test_never_overwrites_a_live_producers_status(self) -> None:
        records = classify([LOST_WSL_JOB], {LOST_WSL_JOB["id"]: LOST_ANNOTATION})["runner_lost"]
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp) / "status-playa-cli-L1-wsl2-ubuntu"
            directory.mkdir()
            (directory / "status.json").write_text('{"result":"success"}')
            self.assertEqual([], synthesize_status(records, Path(tmp)))
            self.assertEqual('{"result":"success"}', (directory / "status.json").read_text())

    def test_unmappable_jobs_are_skipped(self) -> None:
        record = {"id": 5, "name": "preflight (windows-latest)", "step": None, "cell": None}
        with tempfile.TemporaryDirectory() as tmp:
            self.assertEqual([], synthesize_status([record], Path(tmp)))


if __name__ == "__main__":
    unittest.main()

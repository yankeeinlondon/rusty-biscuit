#!/usr/bin/env python3
"""Tests for runner-loss attribution and the one-shot retry decision."""
from __future__ import annotations

import json
import re
import tempfile
import unittest
from pathlib import Path

from runner_loss import (
    classify,
    failed_step,
    failure_stage,
    interrupted_step,
    is_non_producer,
    parse_job_name,
    selected_packages,
    should_rerun,
    status_directory,
    synthesize_status,
)

WORKFLOWS = Path(__file__).resolve().parents[2] / ".github" / "workflows"

# Trimmed from run 33209311538 attempt 1 (2026-08-28): the playa-cli WSL2 leg
# whose runner died during guest provisioning. The name is respelled for the
# area restructure, which put the area above the package and moved the
# environment out of the parenthetical; `JobNameCorpusTests` below derives the
# same spelling from the shipped workflows rather than trusting this string.
LOST_WSL_JOB = {
    "id": 99003990899,
    "name": "area-ci (playa) / playa-cli / wsl2 / test (wsl2-ubuntu)",
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
    "name": "area-ci (claudine) / claudine-cli / test-l2 (macos-latest)",
    "status": "completed",
    "conclusion": "failure",
    "steps": [{"name": "L2 tests", "status": "completed", "conclusion": "failure"}],
}
REAL_FAILURE_ANNOTATION = [
    {"annotation_level": "failure", "message": "Process completed with exit code 100."}
]
VERDICT_JOB = {"id": 1, "name": "ci-verdict", "status": "completed", "conclusion": "failure"}
# The area's own rollup, which fails BECAUSE the producer below lost its
# runner: its cell is MISSING. Counting it as an unrelated failure is what
# would disable the one-shot retry.
AREA_ROLLUP_JOB = {
    "id": 4,
    "name": "area-ci (playa) / rollup",
    "status": "completed",
    "conclusion": "failure",
    "steps": [{"name": "Judge this area", "status": "completed", "conclusion": "failure"}],
}
ADVISORY_SUMMARY_JOB = {
    "id": 6,
    "name": "infrastructure summary (advisory)",
    "status": "completed",
    "conclusion": "failure",
    "steps": [],
}
PASSING_JOB = {
    "id": 2,
    "name": "area-ci (darkmatter) / darkmatter / test (ubuntu-latest)",
    "status": "completed",
    "conclusion": "success",
}
# The shape the repository's own rule calls out: the suite passed and the
# publication step failed. It is not a test regression and rerunning the suite
# would prove nothing.
UPLOAD_FAILURE_JOB = {
    "id": 3,
    "name": "area-ci (sniff) / sniff / test (windows-latest)",
    "status": "completed",
    "conclusion": "failure",
    "steps": [
        {"name": "L1 tests", "status": "completed", "conclusion": "success"},
        {"name": "Stage the JUnit report", "status": "completed", "conclusion": "success"},
        {"name": "Upload the JUnit report", "status": "completed", "conclusion": "failure"},
    ],
}
UPLOAD_FAILURE_ANNOTATION = [
    {"annotation_level": "failure", "message": "Artifact storage quota has been hit."}
]


class JobNameTests(unittest.TestCase):
    def test_wsl2_leg_maps_to_l1_on_wsl2_ubuntu(self) -> None:
        self.assertEqual(
            {"package": "playa-cli", "job": "L1", "environment": "wsl2-ubuntu"},
            parse_job_name(LOST_WSL_JOB["name"]),
        )

    def test_native_legs_map_to_their_status_names(self) -> None:
        cases = {
            "area-ci (sniff) / sniff / test (windows-latest)": ("sniff", "L1", "windows-latest"),
            "area-ci (darkmatter) / dmls / test-l2 (macos-latest)": ("dmls", "L2", "macos-latest"),
            "area-ci (darkmatter) / darkmatter / test-browser (ubuntu-latest)": ("darkmatter", "browser", "ubuntu-latest"),
            "area-ci (biscuit-file) / biscuit-file / check (windows-latest)": ("biscuit-file", "check", "windows-latest"),
            # `lint`'s label names its environment; its artifact does not.
            "area-ci (claudine) / claudine-cli / lint (ubuntu-latest)": ("claudine-cli", "lint", None),
        }
        for name, (package, job, environment) in cases.items():
            self.assertEqual(
                {"package": package, "job": job, "environment": environment},
                parse_job_name(name),
                name,
            )

    def test_a_nested_area_name_still_yields_its_package(self) -> None:
        # `claudine/rendezvous` contains the segment separator's own `/`, but
        # not the `" / "` GitHub joins job labels with.
        self.assertEqual(
            {"package": "rendezvous-cli", "job": "L1", "environment": "ubuntu-latest"},
            parse_job_name(
                "area-ci (claudine/rendezvous) / rendezvous-cli / test (ubuntu-latest)"
            ),
        )

    def test_non_producer_and_archive_jobs_map_to_nothing(self) -> None:
        for name in [
            "ci-verdict",
            "Determine affected scope",
            "preflight (windows-latest)",
            "area-ci (playa) / rollup",
            "area-ci (playa) / playa-cli / wsl2 / archive (playa-cli for wsl2)",
            "biscuit-tui-captured-stdout / biscuit-tui / captured-stdout / windows",
        ]:
            self.assertIsNone(parse_job_name(name), name)

    def test_the_retired_pre_area_spelling_no_longer_parses(self) -> None:
        # The label the parser was written for before the area level existed.
        # Kept as a fixture so the break Phase 6 introduced cannot silently
        # come back as "both spellings work".
        self.assertIsNone(parse_job_name("sniff / test (sniff on windows-latest)"))
        self.assertIsNone(
            parse_job_name("playa-cli / wsl2 (playa-cli) / test (playa-cli on wsl2-ubuntu)")
        )

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

    def test_a_judging_job_never_counts_as_a_failure(self) -> None:
        # The area rollup and the advisory summary judge the run. Their
        # failures FOLLOW from the lost producer's, so treating either as an
        # unrelated failure would veto the one-shot retry — the whole point of
        # the classification.
        for judge in (AREA_ROLLUP_JOB, ADVISORY_SUMMARY_JOB):
            self.assertTrue(is_non_producer(judge["name"]), judge["name"])
        result = classify(
            [LOST_WSL_JOB, AREA_ROLLUP_JOB, ADVISORY_SUMMARY_JOB],
            {
                LOST_WSL_JOB["id"]: LOST_ANNOTATION,
                AREA_ROLLUP_JOB["id"]: [],
                ADVISORY_SUMMARY_JOB["id"]: [],
            },
        )
        self.assertEqual([], result["other_failures"])
        self.assertEqual([], result["failures"])
        rerun, reason = should_rerun(result, attempt=1)
        self.assertTrue(rerun, reason)

    def test_a_producer_is_never_mistaken_for_a_judge(self) -> None:
        self.assertFalse(is_non_producer(REAL_FAILURE_JOB["name"]))
        self.assertFalse(is_non_producer("area-ci (playa) / playa-cli / test (ubuntu-latest)"))

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


class FailureStageTests(unittest.TestCase):
    def test_an_upload_failure_after_passing_tests_is_not_a_test_failure(self) -> None:
        result = classify(
            [UPLOAD_FAILURE_JOB], {UPLOAD_FAILURE_JOB["id"]: UPLOAD_FAILURE_ANNOTATION}
        )

        self.assertEqual([], result["runner_lost"])
        [failure] = result["failures"]
        self.assertEqual("Upload the JUnit report", failure["step"])
        self.assertEqual("artifact-upload", failure["stage"])
        self.assertNotEqual("test", failure["stage"])
        self.assertEqual(
            {"package": "sniff", "job": "L1", "environment": "windows-latest"},
            failure["cell"],
        )

    def test_a_test_step_failure_is_a_test_failure(self) -> None:
        result = classify([REAL_FAILURE_JOB], {REAL_FAILURE_JOB["id"]: REAL_FAILURE_ANNOTATION})
        [failure] = result["failures"]
        self.assertEqual("L2 tests", failure["step"])
        self.assertEqual("test", failure["stage"])

    def test_the_stage_reaches_the_retry_reason(self) -> None:
        # A human reading the retry decision must be able to tell what failed
        # without opening the job.
        classification = classify(
            [LOST_WSL_JOB, UPLOAD_FAILURE_JOB],
            {
                LOST_WSL_JOB["id"]: LOST_ANNOTATION,
                UPLOAD_FAILURE_JOB["id"]: UPLOAD_FAILURE_ANNOTATION,
            },
        )
        rerun, reason = should_rerun(classification, 1)
        self.assertFalse(rerun)
        self.assertIn("artifact-upload", reason)

    def test_a_job_that_failed_before_any_step_reports_unknown(self) -> None:
        self.assertIsNone(failed_step({"steps": []}))
        self.assertEqual("unknown", failure_stage(None))
        self.assertEqual("unknown", failure_stage(""))

    def test_every_stage_marker_classifies_a_real_step_name(self) -> None:
        cases = {
            "Upload the machine-readable rollup": "artifact-upload",
            "Build the nextest archive": "archive",
            "Set up the pinned Rust toolchain": "setup",
            "L2 tests": "test",
            "Lint": "lint",
            "Compile check": "check",
            "Post Run actions/checkout@v5": "setup",
            "Something nobody named": "other",
        }
        for step, stage in cases.items():
            self.assertEqual(stage, failure_stage(step), step)


class SynthesizeStatusTests(unittest.TestCase):
    def test_an_area_synthesizes_only_its_own_packages(self) -> None:
        records = classify([LOST_WSL_JOB], {LOST_WSL_JOB["id"]: LOST_ANNOTATION})["runner_lost"]
        with tempfile.TemporaryDirectory() as tmp:
            self.assertEqual([], synthesize_status(records, Path(tmp), {"claudine"}))
            self.assertEqual([], list(Path(tmp).iterdir()))
            [path] = synthesize_status(records, Path(tmp), {"playa-cli"})
            self.assertTrue(Path(path).exists())

    def test_no_narrowing_attributes_every_package(self) -> None:
        records = classify([LOST_WSL_JOB], {LOST_WSL_JOB["id"]: LOST_ANNOTATION})["runner_lost"]
        with tempfile.TemporaryDirectory() as tmp:
            self.assertEqual(1, len(synthesize_status(records, Path(tmp), None)))

    def test_package_selection_flattens_repeats_and_commas(self) -> None:
        self.assertEqual({"a", "b", "c"}, selected_packages(["a,b", " c "]))
        self.assertIsNone(selected_packages([]))
        self.assertIsNone(selected_packages([" , "]))


    def test_writes_the_status_the_dead_producer_could_not(self) -> None:
        records = classify([LOST_WSL_JOB], {LOST_WSL_JOB["id"]: LOST_ANNOTATION})["runner_lost"]
        with tempfile.TemporaryDirectory() as tmp:
            [path] = synthesize_status(records, Path(tmp))
            # Compare path components, not a string: Windows joins with `\`.
            self.assertEqual(
                ("status-playa-cli-L1-wsl2-ubuntu", "status.json"), Path(path).parts[-2:]
            )
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


def jobs_in(workflow: str) -> dict[str, str]:
    """Every job id in a shipped workflow, mapped to its block as text."""
    source = (WORKFLOWS / workflow).read_text(encoding="utf-8")
    body = source.partition("\njobs:\n")[2]
    blocks: dict[str, str] = {}
    current = ""
    for line in body.splitlines(keepends=True):
        header = re.match(r"^  ([A-Za-z0-9_-]+):[ \t]*$", line)
        if header:
            current = header.group(1)
            blocks[current] = ""
        elif current:
            blocks[current] += line
    return blocks


def resolve(text: str, package: str, environment: str) -> str:
    return (
        text.replace("${{ inputs.package }}", package)
        .replace("${{ matrix.package }}", package)
        .replace("${{ matrix.os }}", environment)
        .replace("${{ matrix.environment }}", environment)
        .replace("${{ matrix.area }}", environment)
    )


def job_label(job_id: str, block: str, package: str, environment: str) -> str:
    """The label GitHub gives a job: its `name:`, else the id plus its matrix."""
    declared = re.search(r"^    name: (.+)$", block, re.M)
    if declared:
        return resolve(declared.group(1).strip(), package, environment)
    matrix = re.search(
        r"^      matrix:\n(?:[ \t]*#.*\n)*[ \t]+([A-Za-z0-9_-]+):", block, re.M
    )
    if matrix is None:
        return job_id
    return f"{job_id} ({environment})"


def status_artifact(block: str, package: str, environment: str) -> str | None:
    declared = re.search(r"^          name: (status-.+)$", block, re.M)
    if declared is None:
        return None
    return resolve(declared.group(1).strip(), package, environment)


class JobNameCorpusTests(unittest.TestCase):
    """Every producer job in the shipped workflows, at its real label.

    The unit fixtures above spell job names by hand, which is how they stayed
    green while the area restructure renamed every one of them. This walks
    `_package-ci.yml` and `_wsl-ci.yml` instead, builds each producer's
    composite label from the caller chain those files actually declare, and
    requires the parser to land on the `status-…` artifact that same job
    uploads — so a rename on either side fails here.
    """

    AREA = "playa"
    PACKAGE = "playa-cli"
    ENVIRONMENT = "windows-latest"

    def area_prefix(self) -> str:
        """`area-ci (<area>) / <package> / `, derived, not assumed."""
        area_ci = jobs_in("ci.yml")["area-ci"]
        self.assertIsNone(
            re.search(r"^    name:", area_ci, re.M),
            "ci.yml's area-ci must stay unnamed so GitHub labels it from the matrix",
        )
        caller = job_label("area-ci", area_ci, self.PACKAGE, self.AREA)
        package_ci = jobs_in("_area-ci.yml")["package-ci"]
        package = job_label("package-ci", package_ci, self.PACKAGE, self.ENVIRONMENT)
        self.assertEqual(
            self.PACKAGE, package, "the area must label each leg with its package"
        )
        return f"{caller} / {package} / "

    def producers(self) -> dict[str, tuple[str, str]]:
        """Composite label -> (job id, status artifact) for every producer."""
        prefix = self.area_prefix()
        wsl2 = jobs_in("_package-ci.yml")["wsl2"]
        self.assertIn(
            "uses: ./.github/workflows/_wsl-ci.yml",
            wsl2,
            "the WSL2 delegation the parser skips must still be the `wsl2` job",
        )
        found: dict[str, tuple[str, str]] = {}
        for workflow, chain in (
            ("_package-ci.yml", prefix),
            ("_wsl-ci.yml", prefix + job_label("wsl2", wsl2, self.PACKAGE, self.ENVIRONMENT) + " / "),
        ):
            for job_id, block in jobs_in(workflow).items():
                status = status_artifact(block, self.PACKAGE, self.ENVIRONMENT)
                if status is None:
                    continue
                label = job_label(job_id, block, self.PACKAGE, self.ENVIRONMENT)
                found[chain + label] = (job_id, status)
        return found

    def test_every_producer_job_parses_to_the_status_it_uploads(self) -> None:
        for label, (job_id, status) in self.producers().items():
            cell = parse_job_name(label)
            self.assertIsNotNone(cell, f"{job_id}: {label} parses to no cell")
            self.assertEqual(self.PACKAGE, cell["package"], label)
            self.assertEqual(status, status_directory(cell), label)

    def test_every_status_uploading_job_is_covered(self) -> None:
        # Non-vacuity: the walk must find the whole gate set, not an empty one.
        found = {job_id for job_id, _ in self.producers().values()}
        self.assertEqual(
            {"check", "test", "lint", "test-l2", "test-browser", "wsl"}, found
        )

    def test_the_areas_own_rollup_is_recognized_as_a_judge(self) -> None:
        rollup = jobs_in("_area-ci.yml")["rollup"]
        label = self.area_prefix().split(" / ")[0] + " / " + job_label(
            "rollup", rollup, self.PACKAGE, self.ENVIRONMENT
        )
        self.assertTrue(is_non_producer(label), label)
        self.assertIsNone(parse_job_name(label), label)

    def test_the_advisory_summary_is_recognized_as_a_judge(self) -> None:
        summary = jobs_in("ci.yml")["summary"]
        label = job_label("summary", summary, self.PACKAGE, self.ENVIRONMENT)
        self.assertIn("continue-on-error: true", summary)
        self.assertTrue(is_non_producer(label), label)


if __name__ == "__main__":
    unittest.main()

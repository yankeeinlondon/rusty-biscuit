#!/usr/bin/env python3
"""Hosted producer cells become durable observations.

Every case runs the public entry points against a real Git repository, a real
bare remote, real files, and a simulated GitHub artifact store (`FakeGitHub`)
that answers the same two API paths the production code uses. Nothing here
replaces a function of the code under test.

The questions pinned:

* `emit` records a finished cell, passing or honestly failing, and refuses to
  invent one for interrupted, report-less, or unproven work;
* `publish` validates, groups, and publishes without ever publishing a malformed
  observation, survives an unrelated failure, and keeps proof local when the
  remote refuses it;
* `gh_probe` makes discovery fetch and compare the artifact, so a pass earns
  credit only while its bytes are retrievable and unaltered;
* a new head reuses the verified proof without rewriting it.
"""

from __future__ import annotations

import contextlib
import hashlib
import io
import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import affected_scope  # noqa: E402
import cell_contract  # noqa: E402
import hosted_observations  # noqa: E402
import local_evidence  # noqa: E402
import observation_notes  # noqa: E402
import plan_fixtures  # noqa: E402
import schema  # noqa: E402

REPOSITORY = "owner/repo"
WORKFLOW_REF = "owner/repo/.github/workflows/ci.yml@refs/heads/main"
ABSENT = object()


class FakeGitHub:
    """The three Actions API paths `hosted_observations` reads, over local data.

    The workflow-run listing returns every recorded run whatever the query asks
    for, so the code's own eligibility checks are what the tests exercise.
    """

    def __init__(self) -> None:
        self.artifacts: dict[int, dict] = {}
        self.runs: list[dict] = []
        self.calls: list[str] = []
        self.broken = False

    def upload(self, run: str, name: str, files: dict[str, bytes], expired: bool = False) -> int:
        identifier = 1000 + len(self.artifacts)
        self.artifacts[identifier] = {
            "run": run, "name": name, "files": dict(files), "expired": expired,
        }
        return identifier

    def alter(self, run: str, name: str, member: str, content: bytes) -> None:
        for artifact in self.artifacts.values():
            if artifact["run"] == run and artifact["name"] == name:
                artifact["files"][member] = content

    def add_run(self, run: str, **overrides: object) -> None:
        entry = {
            "id": int(run), "event": "schedule", "path": ".github/workflows/ci.yml",
            "head_branch": "main", "status": "completed", "conclusion": "failure",
            "repository": {"full_name": REPOSITORY},
        }
        entry.update(overrides)
        self.runs.append(entry)

    def delete(self, run: str, name: str) -> None:
        for identifier in [
            i for i, a in self.artifacts.items() if a["run"] == run and a["name"] == name
        ]:
            del self.artifacts[identifier]

    def expire(self, run: str, name: str) -> None:
        for artifact in self.artifacts.values():
            if artifact["run"] == run and artifact["name"] == name:
                artifact["expired"] = True

    def fetch(self, path: str) -> bytes:
        self.calls.append(path)
        if self.broken:
            raise hosted_observations.HostedError("the API is unreachable")
        prefix = f"repos/{REPOSITORY}/actions/"
        assert path.startswith(prefix), path
        rest = path[len(prefix):]
        if rest.startswith("workflows/ci.yml/runs?"):
            return json.dumps({"total_count": len(self.runs), "workflow_runs": self.runs}).encode()
        if rest.startswith("runs/"):
            run = rest.split("/")[1]
            page = int(re.search(r"[?&]page=(\d+)", rest).group(1))
            listed = [
                {"id": identifier, "name": entry["name"], "expired": entry["expired"]}
                for identifier, entry in sorted(self.artifacts.items())
                if entry["run"] == run
            ]
            size = hosted_observations.PAGE_SIZE
            batch = listed[(page - 1) * size : page * size]
            return json.dumps({"total_count": len(listed), "artifacts": batch}).encode()
        identifier = int(rest.split("/")[1])
        entry = self.artifacts[identifier]
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w") as archive:
            for member, content in entry["files"].items():
                archive.writestr(member, content)
        return buffer.getvalue()


class HostedFixture(unittest.TestCase):
    """A repository with two commits, a bare remote, and a hosted plan."""

    def setUp(self) -> None:
        self.previous_cwd = Path.cwd()
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name).resolve()
        self.repo = self.root / "repo"
        self.repo.mkdir()
        os.chdir(self.repo)
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.name", "CI Test")
        self.git("config", "user.email", "ci@example.com")
        self.write("pkg/alpha/src/lib.rs", "pub fn alpha() {}\n")
        self.write("pkg/beta/src/lib.rs", "pub fn beta() {}\n")
        self.git("add", "-A")
        self.git("commit", "-q", "-m", "base")
        self.first = self.git("rev-parse", "HEAD")
        self.write("pkg/alpha/src/lib.rs", "pub fn alpha() { /* work */ }\n")
        self.git("commit", "-q", "-am", "head")
        self.second = self.git("rev-parse", "HEAD")
        self.remote = self.root / "remote.git"
        subprocess.run(["git", "init", "-q", "--bare", str(self.remote)], check=True)
        self.git("remote", "add", "origin", str(self.remote))
        self.git("push", "-q", "origin", "main")
        self.github = FakeGitHub()
        self.staged = 0
        self.scratch = self.root / "scratch"
        self.scratch.mkdir()

    def tearDown(self) -> None:
        os.chdir(self.previous_cwd)
        self.temporary_directory.cleanup()

    # -- git ---------------------------------------------------------------

    def git(self, *args: str, cwd: Path | None = None) -> str:
        return subprocess.run(
            ["git", *args], check=True, capture_output=True, text=True, cwd=cwd or self.repo
        ).stdout.strip()

    def write(self, relative: str, text: str) -> None:
        path = self.repo / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")

    def ref(self, environment: str) -> str:
        return f"refs/notes/ci-local/{environment}"

    def remote_collection(self, environment: str, commit: str) -> dict | None:
        result = subprocess.run(
            ["git", "--git-dir", str(self.remote), "notes", "--ref", self.ref(environment),
             "show", commit],
            capture_output=True, text=True,
        )
        return json.loads(result.stdout) if result.returncode == 0 else None

    def local_collection(self, environment: str, commit: str) -> dict | None:
        result = subprocess.run(
            ["git", "notes", "--ref", self.ref(environment), "show", commit],
            capture_output=True, text=True,
        )
        return json.loads(result.stdout) if result.returncode == 0 else None

    def reset_notes(self) -> None:
        for environment in schema.ENVIRONMENTS:
            subprocess.run(["git", "update-ref", "-d", self.ref(environment)],
                           capture_output=True)
            subprocess.run(["git", "--git-dir", str(self.remote), "update-ref", "-d",
                            self.ref(environment)], capture_output=True)

    # -- the plan ------------------------------------------------------------

    def plan(self, head: str | None = None) -> dict:
        head = head or self.second
        cells = [
            {
                "package": package,
                "area": "pkg",
                "environment": environment,
                "gate": "L1",
                "execution": "execute",
                "origin": "ci",
                "state": "pending",
                "reusable": True,
                "target_kinds": ["lib", "test"],
                "compile_coverage_from": "L1",
                "selection_reason": "source package on a required environment",
            }
            for package, environment in (
                ("alpha", "ubuntu-latest"),
                ("alpha", "wsl2-ubuntu"),
                ("beta", "ubuntu-latest"),
            )
        ]
        document = {
            "schema_version": schema.RESOLVED_PLAN_SCHEMA_VERSION,
            "base": self.first,
            "head": head,
            "change_class": "package",
            "change_inventory": affected_scope.change_inventory(
                ["alpha/src/lib.rs", "beta/src/lib.rs"], False
            ),
            "archive_guard": plan_fixtures.archive_guard(["alpha/src/lib.rs"]),
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

    def plan_file(self, head: str | None = None, name: str = "plan.json") -> Path:
        path = self.scratch / name
        path.write_text(schema.canonical(self.plan(head)), encoding="utf-8")
        return path

    # -- a finished producer cell, as the workflow leaves it ------------------

    def stage_cell(
        self,
        package: str,
        environment: str,
        *,
        run: str = "100",
        attempt: int = 1,
        head: str | None = None,
        failing: tuple[str, ...] = (),
        completion: bool = True,
        expected: bool = True,
        report: bool = True,
        report_present: bool = True,
        exit_code: int | None = None,
        tag: str = "",
    ) -> tuple[Path, Path]:
        head = head or self.second
        self.staged += 1
        base = self.scratch / f"{self.staged:03d}-{package}-{environment}-{run}-{attempt}{tag}"
        stage = base / "stage"
        (stage / "L1").mkdir(parents=True)
        xml = f"L1/{package}.xml"
        if report:
            cases = "".join(
                f'<testcase classname="{package}" name="{name}"><failure message="boom"/></testcase>'
                for name in failing
            ) + '<testcase classname="ok" name="fine"/>' * 3
            (stage / xml).write_text(
                '<?xml version="1.0"?><testsuites>'
                f'<testsuite name="{package}" tests="{len(failing) + 3}" '
                f'failures="{len(failing)}" errors="0" skipped="0">{cases}</testsuite>'
                "</testsuites>",
                encoding="utf-8",
            )
        record = {
            "tier": "L1", "package": package, "xml": xml,
            "exit_code": (1 if failing else 0) if exit_code is None else exit_code,
            "environment": environment, "duration_s": 7, "report_present": report_present,
        }
        (stage / "manifest.jsonl").write_text(json.dumps(record) + "\n", encoding="utf-8")
        if expected:
            (stage / "expected-L1.json").write_text(
                json.dumps({"target": "x86_64-unknown-linux-gnu", "nextest_version": "0.9.99",
                            "tests": [f"{package}::fine"]}),
                encoding="utf-8",
            )
        completion_path = base / "completion" / "completion.json"
        if completion:
            completion_path.parent.mkdir()
            completion_path.write_text(
                json.dumps({
                    "schema_version": schema.COMPLETION_RECORD_SCHEMA_VERSION,
                    "package": package, "environment": environment, "gate": "L1",
                    "complete": True, "head": head, "run": run, "attempt": attempt,
                    "nextest_version": "0.9.99", "reports": [xml],
                }),
                encoding="utf-8",
            )
        return stage, completion_path

    def run_emit(
        self,
        package: str,
        environment: str,
        stage: Path,
        completion_path: Path,
        *,
        gate_outcome: str = "success",
        run: str = "100",
        attempt: int = 1,
        plan: Path | None = None,
        out: Path | None = None,
        cell: str | None = None,
    ) -> tuple[int, Path, str]:
        out = out or (stage.parent / "observation" / hosted_observations.OBSERVATION_FILE)
        argv = [
            "emit", "--plan", str(plan or self.plan_file()),
            "--cell", cell or f"{package}/{environment}/L1",
            "--stage", str(stage), "--gate-outcome", gate_outcome,
            "--run-id", run, "--attempt", str(attempt), "--workflow", WORKFLOW_REF,
            "--out", str(out),
        ]
        if completion_path.exists():
            argv += ["--completion", str(completion_path)]
        errors = io.StringIO()
        with contextlib.redirect_stderr(errors), contextlib.redirect_stdout(io.StringIO()):
            code = hosted_observations.main(argv, fetch=self.github.fetch)
        return code, out, errors.getvalue()

    def finish_cell(self, package: str, environment: str, *, run: str = "100",
                    attempt: int = 1, head: str | None = None, plan: Path | None = None,
                    failing: tuple[str, ...] = (), **stage_options) -> dict:
        """Run a producer cell to its end: stage, upload, emit, upload."""
        stage, completion_path = self.stage_cell(
            package, environment, run=run, attempt=attempt, head=head, failing=failing,
            completion=not failing, **stage_options,
        )
        files = {
            str(path.relative_to(stage)).replace(os.sep, "/"): path.read_bytes()
            for path in stage.rglob("*") if path.is_file()
        }
        self.github.upload(run, hosted_observations.junit_artifact(package, "L1", environment), files)
        if completion_path.exists():
            self.github.upload(
                run, hosted_observations.completion_artifact(package, "L1", environment),
                {"completion.json": completion_path.read_bytes()},
            )
        # The producer's checkout is the plan's head; so is this one's.
        self.git("checkout", "-q", head or self.second)
        try:
            code, out, errors = self.run_emit(
                package, environment, stage, completion_path,
                gate_outcome="failure" if failing else "success", run=run, attempt=attempt,
                plan=plan,
            )
        finally:
            self.git("checkout", "-q", "main")
        self.assertEqual(0, code, errors)
        self.github.upload(
            run, hosted_observations.observation_artifact(package, "L1", environment),
            {hosted_observations.OBSERVATION_FILE: out.read_bytes()},
        )
        return json.loads(out.read_text(encoding="utf-8"))

    def github_envelope(self, run: str, package: str) -> dict:
        name = hosted_observations.observation_artifact(package, "L1", "ubuntu-latest")
        artifact = next(
            a for a in self.github.artifacts.values() if a["run"] == run and a["name"] == name
        )
        return json.loads(artifact["files"][hosted_observations.OBSERVATION_FILE])

    def publish_run(self, run: str = "100") -> tuple[int, str]:
        errors = io.StringIO()
        with contextlib.redirect_stderr(errors), contextlib.redirect_stdout(io.StringIO()):
            code = hosted_observations.main(
                ["publish", "--repository", REPOSITORY, "--run-id", run],
                fetch=self.github.fetch,
            )
        return code, errors.getvalue()

    def recover(self, *extra: str) -> tuple[int, str]:
        errors = io.StringIO()
        with contextlib.redirect_stderr(errors), contextlib.redirect_stdout(io.StringIO()):
            code = hosted_observations.main(
                ["publish", "--repository", REPOSITORY, "--recent", *extra],
                fetch=self.github.fetch,
            )
        return code, errors.getvalue()

    def verify(self, plan: Path, base: str, head: str, **kwargs):
        probe = hosted_observations.gh_probe(
            REPOSITORY, fetch=self.github.fetch, log=lambda message: None
        )
        return local_evidence.verify_cells(str(plan), base, head, report_probe=probe, **kwargs)

    def credited(self, plan: Path, base: str, head: str) -> list[str]:
        accepted, _ = self.verify(plan, base, head)
        return sorted(f"{c['package']}/{c['environment']}/{c['gate']}" for c in accepted)


def with_edit(observation: dict, path: tuple, value: object) -> dict:
    """A copy of `observation` with one field set (or removed, for `ABSENT`)."""
    body = json.loads(json.dumps({k: v for k, v in observation.items() if k != "observation_id"}))
    cursor = body
    for step in path[:-1]:
        cursor = cursor[step]
    if value is ABSENT:
        del cursor[path[-1]]
    else:
        cursor[path[-1]] = value
    return observation_notes.with_identity(body)


class EmitTests(HostedFixture):
    def test_a_passing_cell_becomes_a_proven_hosted_observation(self) -> None:
        stage, completion_path = self.stage_cell("alpha", "ubuntu-latest")
        code, out, errors = self.run_emit("alpha", "ubuntu-latest", stage, completion_path)
        self.assertEqual(0, code, errors)
        envelope = json.loads(out.read_text(encoding="utf-8"))
        observation = envelope["observation"]
        self.assertEqual(
            ("ubuntu-latest", self.second), (envelope["environment"], envelope["head"])
        )
        self.assertEqual([], schema.validate_observation(observation, "ubuntu-latest"))
        self.assertEqual(
            {"kind": "hosted",
             "host": observation["producer"]["host"],
             "workflow": ".github/workflows/ci.yml", "run_id": "100", "attempt": 1},
            observation["producer"],
        )
        self.assertEqual("ubuntu-latest", observation["producer"]["host"]["label"])
        self.assertEqual("pass", observation["cell"]["outcome"])
        self.assertEqual(self.first, observation["base"])
        proof = observation["proof"]
        self.assertEqual(
            hashlib.sha256((stage / "expected-L1.json").read_bytes()).hexdigest(),
            proof["expected_listing_digest"],
        )
        self.assertEqual(
            hashlib.sha256(completion_path.read_bytes()).hexdigest(),
            proof["completion_record_digest"],
        )
        self.assertEqual(("0.9.99", "x86_64-unknown-linux-gnu"),
                         (proof["nextest_version"], proof["target"]))
        report = (stage / "L1" / "alpha.xml").read_bytes()
        self.assertEqual(
            [{"path": "L1/alpha.xml", "sha256": hashlib.sha256(report).hexdigest(),
              "bytes": len(report), "store": "artifact"}],
            observation["reports"],
        )
        self.assertEqual("L1/alpha.xml", observation["cell"]["report"])

    def test_the_wsl2_guest_cell_is_recorded_under_its_own_environment(self) -> None:
        stage, completion_path = self.stage_cell("alpha", "wsl2-ubuntu")
        code, out, errors = self.run_emit("alpha", "wsl2-ubuntu", stage, completion_path)
        self.assertEqual(0, code, errors)
        envelope = json.loads(out.read_text(encoding="utf-8"))
        self.assertEqual("wsl2-ubuntu", envelope["environment"])
        self.assertEqual("wsl2-ubuntu", envelope["observation"]["producer"]["host"]["label"])

    def test_an_honest_failure_is_recorded_without_a_completion_record(self) -> None:
        stage, completion_path = self.stage_cell(
            "alpha", "ubuntu-latest", failing=("suite::boom",), completion=False
        )
        code, out, errors = self.run_emit(
            "alpha", "ubuntu-latest", stage, completion_path, gate_outcome="failure"
        )
        self.assertEqual(0, code, errors)
        observation = json.loads(out.read_text(encoding="utf-8"))["observation"]
        self.assertEqual("fail", observation["cell"]["outcome"])
        self.assertEqual(["alpha::suite::boom"], observation["cell"]["failed_tests"])
        self.assertNotIn("completion_record_digest", observation["proof"])

    def test_a_failure_with_no_listing_is_recorded_without_proof(self) -> None:
        stage, completion_path = self.stage_cell(
            "alpha", "ubuntu-latest", failing=("suite::boom",), completion=False, expected=False
        )
        code, out, errors = self.run_emit(
            "alpha", "ubuntu-latest", stage, completion_path, gate_outcome="failure"
        )
        self.assertEqual(0, code, errors)
        observation = json.loads(out.read_text(encoding="utf-8"))["observation"]
        self.assertNotIn("proof", observation)
        self.assertEqual([], schema.validate_observation(observation, "ubuntu-latest"))

    def test_work_that_proves_nothing_writes_nothing(self) -> None:
        """Each row breaks one thing; the control row shows the rest is sound."""
        other_head = {"head": self.first}
        rows = {
            "gate cancelled": {"gate_outcome": "cancelled"},
            "gate skipped": {"gate_outcome": "skipped"},
            "gate outcome unknown": {"gate_outcome": ""},
            "no completion record": {"completion": False},
            "report never staged": {"report": False},
            "manifest says no report": {"report_present": False},
            "no expected listing": {"expected": False},
            "completion of another run": {"run_in_record": "999"},
            "completion of another attempt": {"attempt_in_record": 2},
            "completion of another head": other_head,
            "success but the report records a failure": {"failing": ("suite::boom",), "exit_code": 0},
            "unplanned cell": {"cell": "alpha/macos-latest/L1"},
            "gate without a report": {"cell": "alpha/ubuntu-latest/lint"},
            "malformed cell id": {"cell": "alpha"},
        }
        for label, row in [("control", {})] + list(rows.items()):
            with self.subTest(label):
                options = dict(row)
                gate_outcome = options.pop("gate_outcome", "success")
                cell = options.pop("cell", None)
                record_run = options.pop("run_in_record", "100")
                record_attempt = options.pop("attempt_in_record", 1)
                stage, completion_path = self.stage_cell(
                    "alpha", "ubuntu-latest", run=record_run, attempt=record_attempt,
                    tag=label.replace(" ", "-"), **options,
                )
                code, out, errors = self.run_emit(
                    "alpha", "ubuntu-latest", stage, completion_path,
                    gate_outcome=gate_outcome, cell=cell,
                )
                if label == "control":
                    self.assertEqual(0, code, errors)
                    self.assertTrue(out.is_file())
                else:
                    self.assertEqual(hosted_observations.EXIT_REFUSED, code)
                    self.assertFalse(out.exists(), "a refusal must write nothing")
                    self.assertIn("no observation", errors)

    def test_a_failed_gate_whose_report_names_no_failing_test_is_not_an_observation(self) -> None:
        stage, completion_path = self.stage_cell(
            "alpha", "ubuntu-latest", completion=False, exit_code=1
        )
        code, out, errors = self.run_emit(
            "alpha", "ubuntu-latest", stage, completion_path, gate_outcome="failure"
        )
        self.assertEqual(hosted_observations.EXIT_REFUSED, code, errors)
        self.assertFalse(out.exists())

    def test_a_refusal_removes_a_file_an_earlier_attempt_left(self) -> None:
        stage, completion_path = self.stage_cell("alpha", "ubuntu-latest")
        code, out, _ = self.run_emit("alpha", "ubuntu-latest", stage, completion_path)
        self.assertEqual(0, code)
        code, out, _ = self.run_emit(
            "alpha", "ubuntu-latest", stage, completion_path, gate_outcome="cancelled", out=out
        )
        self.assertEqual(hosted_observations.EXIT_REFUSED, code)
        self.assertFalse(out.exists())

    def test_a_checkout_that_is_not_the_tested_revision_is_refused(self) -> None:
        self.git("checkout", "-q", self.first)
        stage, completion_path = self.stage_cell("alpha", "ubuntu-latest")
        code, out, errors = self.run_emit("alpha", "ubuntu-latest", stage, completion_path)
        self.assertEqual(hosted_observations.EXIT_REFUSED, code)
        self.assertIn("binds to the revision the plan names", errors)
        self.assertFalse(out.exists())

    def test_workflow_references_normalize_to_a_path(self) -> None:
        for reference, expected in (
            (WORKFLOW_REF, ".github/workflows/ci.yml"),
            (".github/workflows/ci.yml", ".github/workflows/ci.yml"),
            ("a/b/.github/workflows/x.yml@refs/tags/v1", ".github/workflows/x.yml"),
        ):
            with self.subTest(reference):
                self.assertEqual(expected, hosted_observations.workflow_path(reference))

    def test_artifact_names_match_the_names_the_workflows_upload(self) -> None:
        plan = self.plan()
        row = {"package": "alpha", "gate": "L1", "environment": "wsl2-ubuntu",
               "runner": "windows-latest"}
        contract = cell_contract.contract(plan, row, plan["head"])
        self.assertEqual(
            contract["junit_artifact"], hosted_observations.junit_artifact("alpha", "L1", "wsl2-ubuntu")
        )
        self.assertEqual(
            contract["completion_artifact"],
            hosted_observations.completion_artifact("alpha", "L1", "wsl2-ubuntu"),
        )
        self.assertEqual(
            "observation-alpha-L1-wsl2-ubuntu",
            hosted_observations.observation_artifact("alpha", "L1", "wsl2-ubuntu"),
        )


class PublishTests(HostedFixture):
    def test_emit_publish_verify_round_trip_credits_the_pass(self) -> None:
        alpha = self.finish_cell("alpha", "ubuntu-latest")
        code, errors = self.publish_run()
        self.assertEqual(0, code, errors)
        collection = self.remote_collection("ubuntu-latest", self.second)
        self.assertEqual(
            [alpha["observation"]["observation_id"]],
            [o["observation_id"] for o in collection["observations"]],
        )
        self.assertEqual([], schema.validate_collection(collection))
        accepted, rejections = self.verify(self.plan_file(), self.first, self.second)
        self.assertEqual(["alpha"], [c["package"] for c in accepted], rejections)
        self.assertEqual(
            alpha["observation"]["observation_id"], accepted[0]["evidence"]["observation"]
        )

    def test_the_guest_publishes_under_the_wsl2_ref(self) -> None:
        self.finish_cell("alpha", "wsl2-ubuntu")
        code, errors = self.publish_run()
        self.assertEqual(0, code, errors)
        self.assertIsNotNone(self.remote_collection("wsl2-ubuntu", self.second))
        self.assertIsNone(self.remote_collection("ubuntu-latest", self.second))
        self.assertEqual(["alpha/wsl2-ubuntu/L1"],
                         self.credited(self.plan_file(), self.first, self.second))

    def test_republishing_a_run_publishes_nothing_new(self) -> None:
        self.finish_cell("alpha", "ubuntu-latest")
        self.assertEqual(0, self.publish_run()[0])
        before = self.git("notes", "--ref", self.ref("ubuntu-latest"), "show", self.second)
        self.assertEqual(0, self.publish_run()[0])
        self.assertEqual(before, self.git("notes", "--ref", self.ref("ubuntu-latest"), "show", self.second))
        remote = self.remote_collection("ubuntu-latest", self.second)
        self.assertEqual(before, schema.canonical(remote))

    def test_an_unrelated_failure_does_not_lose_the_passing_cell(self) -> None:
        self.finish_cell("alpha", "ubuntu-latest")
        self.finish_cell("beta", "ubuntu-latest", failing=("suite::boom",))
        code, errors = self.publish_run()
        self.assertEqual(0, code, errors)
        outcomes = {
            o["cell"]["package"]: o["cell"]["outcome"]
            for o in self.remote_collection("ubuntu-latest", self.second)["observations"]
        }
        self.assertEqual({"alpha": "pass", "beta": "fail"}, outcomes)
        self.assertEqual(["alpha/ubuntu-latest/L1"],
                         self.credited(self.plan_file(), self.first, self.second))

    def test_a_cancelled_run_publishes_only_what_was_uploaded(self) -> None:
        # alpha finished and uploaded; beta was cancelled before its emit step.
        self.finish_cell("alpha", "ubuntu-latest")
        stage, completion_path = self.stage_cell("beta", "ubuntu-latest", tag="-cancelled")
        code, out, _ = self.run_emit(
            "beta", "ubuntu-latest", stage, completion_path, gate_outcome="cancelled"
        )
        self.assertEqual(hosted_observations.EXIT_REFUSED, code)
        self.assertFalse(out.exists())
        code, errors = self.publish_run()
        self.assertEqual(0, code, errors)
        self.assertEqual(["alpha/ubuntu-latest/L1"],
                         self.credited(self.plan_file(), self.first, self.second))

    def test_a_run_with_no_observations_is_not_an_error(self) -> None:
        self.github.upload("100", "junit-alpha-L1-ubuntu-latest", {"manifest.jsonl": b""})
        code, errors = self.publish_run()
        self.assertEqual(0, code, errors)
        self.assertIsNone(self.remote_collection("ubuntu-latest", self.second))

    def test_an_unreachable_artifact_listing_is_a_visible_failure(self) -> None:
        self.github.broken = True
        code, errors = self.publish_run()
        self.assertEqual(hosted_observations.EXIT_NO_ARTIFACTS, code)
        self.assertIn("cannot collect run 100", errors)

    def test_an_expired_observation_artifact_is_skipped_with_a_message(self) -> None:
        self.finish_cell("alpha", "ubuntu-latest")
        self.github.expire("100", "observation-alpha-L1-ubuntu-latest")
        code, errors = self.publish_run()
        self.assertEqual(0, code, errors)
        self.assertIn("has expired", errors)
        self.assertIsNone(self.remote_collection("ubuntu-latest", self.second))

    def test_an_invalid_observation_is_rejected_and_the_rest_are_published(self) -> None:
        good = self.finish_cell("alpha", "ubuntu-latest")
        bad = self.finish_cell("beta", "ubuntu-latest")
        tampered = json.loads(json.dumps(bad))
        tampered["observation"]["cell"]["counts"]["passed"] = 99  # id no longer matches
        self.github.alter(
            "100", "observation-beta-L1-ubuntu-latest", hosted_observations.OBSERVATION_FILE,
            json.dumps(tampered).encode(),
        )
        self.github.upload("100", "observation-gamma-L1-ubuntu-latest",
                           {hosted_observations.OBSERVATION_FILE: b"{not json"})
        code, errors = self.publish_run()
        self.assertEqual(hosted_observations.EXIT_REJECTED, code, errors)
        self.assertIn("rejected", errors)
        self.assertEqual(
            [good["observation"]["observation_id"]],
            [o["observation_id"] for o in
             self.remote_collection("ubuntu-latest", self.second)["observations"]],
        )

    def test_files_and_members_that_could_escape_the_download_are_ignored(self) -> None:
        good = self.finish_cell("alpha", "ubuntu-latest")
        self.github.upload("100", "observation-evil-L1-ubuntu-latest", {
            "../escape.json": json.dumps(good).encode(),
            "nested/observation.json": json.dumps(good).encode(),
        })
        self.assertEqual(0, self.publish_run()[0])
        self.assertFalse((self.scratch.parent / "escape.json").exists())
        self.assertEqual(
            1, len(self.remote_collection("ubuntu-latest", self.second)["observations"])
        )

    def test_an_observation_of_another_run_is_not_published_by_this_one(self) -> None:
        self.finish_cell("alpha", "ubuntu-latest", run="100")
        envelope = self.github_envelope("100", "alpha")
        self.github.upload("200", "observation-alpha-L1-ubuntu-latest",
                           {hosted_observations.OBSERVATION_FILE: json.dumps(envelope).encode()})
        code, errors = self.publish_run("200")
        self.assertEqual(hosted_observations.EXIT_REJECTED, code, errors)
        self.assertIn("belongs to run 100, not 200", errors)
        self.assertIsNone(self.remote_collection("ubuntu-latest", self.second))

    def test_a_remote_that_refuses_the_push_leaves_the_proof_local_and_says_so(self) -> None:
        hook = self.remote / "hooks" / "pre-receive"
        hook.write_text("#!/bin/sh\necho 'refs/notes are protected' >&2\nexit 1\n")
        hook.chmod(0o755)
        alpha = self.finish_cell("alpha", "ubuntu-latest")
        directory = self.scratch / "downloaded"
        directory.mkdir()
        (directory / "observation.json").write_text(json.dumps(alpha))
        errors = io.StringIO()
        with contextlib.redirect_stderr(errors):
            summary = hosted_observations.publish_directory(
                directory, sleep=lambda seconds: None, attempts=2
            )
        self.assertEqual(hosted_observations.EXIT_LOCAL_ONLY, summary.exit_status)
        self.assertIn(hosted_observations.LOCAL_ONLY_MESSAGE, errors.getvalue())
        self.assertIsNone(self.remote_collection("ubuntu-latest", self.second))
        local = self.local_collection("ubuntu-latest", self.second)
        self.assertEqual(
            [alpha["observation"]["observation_id"]],
            [o["observation_id"] for o in local["observations"]],
        )

    def test_an_unreachable_remote_keeps_the_proof_local_too(self) -> None:
        alpha = self.finish_cell("alpha", "ubuntu-latest")
        directory = self.scratch / "downloaded"
        directory.mkdir()
        (directory / "observation.json").write_text(json.dumps(alpha))
        self.git("remote", "set-url", "origin", str(self.root / "missing.git"))
        with contextlib.redirect_stderr(io.StringIO()):
            summary = hosted_observations.publish_directory(
                directory, sleep=lambda seconds: None, attempts=2
            )
        self.assertEqual(hosted_observations.EXIT_LOCAL_ONLY, summary.exit_status)
        self.assertIsNotNone(self.local_collection("ubuntu-latest", self.second))

    def test_publication_is_per_group_so_one_environments_failure_spares_another(self) -> None:
        self.finish_cell("alpha", "ubuntu-latest")
        self.finish_cell("alpha", "wsl2-ubuntu")
        directory = self.scratch / "downloaded"
        directory.mkdir()
        for name in ("observation-alpha-L1-ubuntu-latest", "observation-alpha-L1-wsl2-ubuntu"):
            artifact = next(a for a in self.github.artifacts.values() if a["name"] == name)
            (directory / f"{name}.json").write_bytes(artifact["files"]["observation.json"])
        calls = []

        def publisher(environment, head, observations, **kwargs):
            calls.append(environment)
            if environment == "ubuntu-latest":
                raise observation_notes.PublicationError("the local write failed")
            return observation_notes.publish(environment, head, observations, **kwargs)

        with contextlib.redirect_stderr(io.StringIO()), contextlib.redirect_stdout(io.StringIO()):
            summary = hosted_observations.publish_directory(directory, publisher=publisher)
        self.assertEqual(["ubuntu-latest", "wsl2-ubuntu"], calls)
        self.assertEqual(hosted_observations.EXIT_REJECTED, summary.exit_status)
        self.assertIsNotNone(self.remote_collection("wsl2-ubuntu", self.second))


class DiscoveryTests(HostedFixture):
    """`gh_probe` through the public verifier: credit only for bytes that match."""

    def published_pass(self, **options) -> dict:
        envelope = self.finish_cell("alpha", "ubuntu-latest", **options)
        self.assertEqual(0, self.publish_run(options.get("run", "100"))[0])
        return envelope

    def test_a_retrievable_unaltered_report_earns_credit_and_costs_one_download(self) -> None:
        self.published_pass()
        plan = self.plan_file()
        self.github.calls.clear()
        probe = hosted_observations.gh_probe(
            REPOSITORY, fetch=self.github.fetch, log=lambda message: None
        )
        accepted, _ = local_evidence.verify_cells(
            str(plan), self.first, self.second, report_probe=probe
        )
        self.assertEqual(["alpha"], [c["package"] for c in accepted])
        downloads = [c for c in self.github.calls if c.endswith("/zip")]
        self.assertEqual(2, len(downloads), "the junit and completion archives, once each")
        self.assertEqual(1, len([c for c in self.github.calls if "/runs/" in c]))

    def test_altered_expired_or_missing_artifacts_earn_no_credit(self) -> None:
        junit = "junit-alpha-L1-ubuntu-latest"
        completion_name = "completion-alpha-L1-ubuntu-latest"
        scenarios = {
            "control": lambda: None,
            "report altered": lambda: self.github.alter("100", junit, "L1/alpha.xml", b"<testsuites/>"),
            "listing altered": lambda: self.github.alter("100", junit, "expected-L1.json", b"{}"),
            "artifact deleted": lambda: self.github.delete("100", junit),
            "junit expired": lambda: self.github.expire("100", junit),
            "completion altered": lambda: self.github.alter(
                "100", completion_name, "completion.json", b"{}"),
            "completion expired": lambda: self.github.expire("100", completion_name),
            "api down": lambda: setattr(self.github, "broken", True),
        }
        for label, damage in scenarios.items():
            with self.subTest(label):
                self.reset_notes()
                self.github = FakeGitHub()
                self.published_pass()
                damage()
                accepted, rejections = self.verify(self.plan_file(), self.first, self.second)
                if label == "control":
                    self.assertEqual(["alpha"], [c["package"] for c in accepted])
                else:
                    self.assertEqual([], accepted)
                    self.assertTrue(
                        any(r.startswith("observation-report-unavailable") for r in rejections),
                        rejections,
                    )

    def test_the_probe_never_raises(self) -> None:
        self.published_pass()
        observation = json.loads(
            self.git("notes", "--ref", self.ref("ubuntu-latest"), "show", self.second)
        )["observations"][0]
        probe = hosted_observations.gh_probe(
            REPOSITORY, fetch=lambda path: b"not json", log=lambda message: None
        )
        self.assertIs(False, probe(observation["reports"][0], observation))
        broken = dict(observation, producer={"kind": "hosted"})
        self.assertIs(False, probe(observation["reports"][0], broken))
        self.assertIs(False, probe({}, {}))

    def test_a_failure_still_blocks_an_older_pass_though_its_artifacts_are_gone(self) -> None:
        # beta passed at the first commit and is unchanged at the second.
        first_plan = self.plan_file(head=self.first, name="first.json")
        self.finish_cell("beta", "ubuntu-latest", run="100", head=self.first, plan=first_plan)
        self.assertEqual(0, self.publish_run("100")[0])
        second_plan = self.plan_file()
        self.assertEqual(["beta/ubuntu-latest/L1"],
                         self.credited(second_plan, self.first, self.second))
        self.finish_cell("beta", "ubuntu-latest", run="200", failing=("suite::boom",))
        self.assertEqual(0, self.publish_run("200")[0])
        for artifact in self.github.artifacts.values():
            if artifact["run"] == "200":
                artifact["expired"] = True
        self.assertEqual([], self.credited(second_plan, self.first, self.second))

    def test_a_new_head_reuses_the_verified_proof_and_never_rewrites_it(self) -> None:
        first_plan = self.plan_file(head=self.first, name="first.json")
        original = self.finish_cell(
            "beta", "ubuntu-latest", run="100", head=self.first, plan=first_plan
        )
        self.assertEqual(0, self.publish_run("100")[0])
        stored = self.git("notes", "--ref", self.ref("ubuntu-latest"), "show", self.first)
        # alpha changed between the commits; beta's closure did not.
        accepted, rejections = self.verify(self.plan_file(), self.first, self.second)
        self.assertEqual(["beta"], [c["package"] for c in accepted], rejections)
        self.assertEqual("prior-local", accepted[0]["origin"])
        self.assertEqual(
            original["observation"]["observation_id"], accepted[0]["evidence"]["observation"]
        )
        kept = json.loads(stored)["observations"][0]
        self.assertEqual(
            (self.first, "100", 1, original["observation"]["base"]),
            (json.loads(stored)["head"], kept["producer"]["run_id"],
             kept["producer"]["attempt"], kept["base"]),
        )
        self.assertEqual(original["observation"], kept)
        self.assertEqual(stored, self.git("notes", "--ref", self.ref("ubuntu-latest"), "show", self.first))
        self.assertIsNone(self.local_collection("ubuntu-latest", self.second))

    def test_a_rerun_attempt_is_linked_to_the_failure_it_replaces(self) -> None:
        failed = self.finish_cell("alpha", "ubuntu-latest", failing=("suite::boom",))
        self.assertEqual(0, self.publish_run()[0])
        self.assertEqual([], self.credited(self.plan_file(), self.first, self.second))
        # The re-run of the failed job at attempt 2: new artifacts, new observation.
        self.github = FakeGitHub()
        passed = self.finish_cell("alpha", "ubuntu-latest", attempt=2, tag="-rerun")
        self.assertEqual(0, self.publish_run()[0])
        collection = self.remote_collection("ubuntu-latest", self.second)
        retries = [o for o in collection["observations"] if "retry_of" in o]
        self.assertEqual([failed["observation"]["observation_id"]], [o["retry_of"] for o in retries])
        # Linking re-identifies the retry; everything the producer wrote is kept.
        stored = {k: v for k, v in retries[0].items() if k not in ("observation_id", "retry_of")}
        emitted = {k: v for k, v in passed["observation"].items() if k != "observation_id"}
        self.assertEqual(emitted, stored)
        self.assertEqual(["alpha/ubuntu-latest/L1"],
                         self.credited(self.plan_file(), self.first, self.second))


    def test_a_rerun_attempt_is_linked_when_only_the_remote_holds_its_predecessor(self) -> None:
        failed = self.finish_cell("alpha", "ubuntu-latest", failing=("suite::boom",))
        self.assertEqual(0, self.publish_run()[0])
        # The publishing job is a fresh checkout: no local notes ref.
        self.git("update-ref", "-d", self.ref("ubuntu-latest"))
        self.github = FakeGitHub()
        self.finish_cell("alpha", "ubuntu-latest", attempt=2, tag="-rerun")
        self.assertEqual(0, self.publish_run()[0])
        collection = self.remote_collection("ubuntu-latest", self.second)
        self.assertEqual([failed["observation"]["observation_id"]],
                         [o["retry_of"] for o in collection["observations"] if "retry_of" in o])
        self.assertEqual(["alpha/ubuntu-latest/L1"],
                         self.credited(self.plan_file(), self.first, self.second))

    def test_a_delayed_pass_from_an_earlier_run_does_not_hide_a_newer_failure(self) -> None:
        # Run 100 passed first; its publication was delayed until after run 101 failed.
        self.finish_cell("alpha", "ubuntu-latest", run="100")
        self.finish_cell("alpha", "ubuntu-latest", run="101", failing=("suite::boom",))
        self.assertEqual(0, self.publish_run("101")[0])
        self.assertEqual(0, self.publish_run("100")[0])
        collection = self.remote_collection("ubuntu-latest", self.second)
        self.assertEqual(2, len(collection["observations"]))
        self.assertEqual([], [o for o in collection["observations"] if "retry_of" in o])
        accepted, rejections = self.verify(self.plan_file(), self.first, self.second)
        self.assertEqual([], accepted)
        self.assertTrue([r for r in rejections if r.startswith("conflicting-evidence")], rejections)

    def test_a_rerun_attempt_published_before_its_predecessor_stays_unordered(self) -> None:
        self.finish_cell("alpha", "ubuntu-latest", failing=("suite::boom",))
        first_attempt = self.github
        self.github = FakeGitHub()
        self.finish_cell("alpha", "ubuntu-latest", attempt=2, tag="-rerun")
        second_attempt = self.github
        self.assertEqual(0, self.publish_run()[0])
        self.github = first_attempt
        self.assertEqual(0, self.publish_run()[0])
        self.github = second_attempt
        collection = self.remote_collection("ubuntu-latest", self.second)
        self.assertEqual([], [o for o in collection["observations"] if "retry_of" in o],
                         "a stored observation is never re-identified")
        self.assertEqual([], self.credited(self.plan_file(), self.first, self.second))


class RecoveryTests(HostedFixture):
    """A run whose collector never ran still has its finished cells discovered.

    Each case leaves the producers' uploads in place and never calls
    `publish --run-id`: the collector of the run that produced them was lost.
    """

    def orphan(self, run: str = "100", **run_fields: object) -> None:
        self.github.add_run(run, **run_fields)
        self.finish_cell("alpha", "ubuntu-latest", run=run)
        self.finish_cell("alpha", "wsl2-ubuntu", run=run)

    def test_the_nightly_scope_recovers_both_producers_locally_without_a_push(self) -> None:
        self.orphan()
        plan = self.plan_file()
        self.assertEqual([], self.credited(plan, self.first, self.second))
        code, errors = self.recover("--no-push")
        self.assertEqual(0, code, errors)
        self.assertEqual(["alpha/ubuntu-latest/L1", "alpha/wsl2-ubuntu/L1"],
                         self.credited(plan, self.first, self.second))
        self.assertIsNone(self.remote_collection("ubuntu-latest", self.second))
        self.assertIsNone(self.remote_collection("wsl2-ubuntu", self.second))

    def test_the_trusted_publisher_pushes_the_recovery_so_it_outlives_the_run(self) -> None:
        self.orphan()
        code, errors = self.recover()
        self.assertEqual(0, code, errors)
        for environment in ("ubuntu-latest", "wsl2-ubuntu"):
            collection = self.remote_collection(environment, self.second)
            self.assertEqual(["100"],
                             [o["producer"]["run_id"] for o in collection["observations"]])
        self.assertEqual(0, self.recover()[0])
        self.assertEqual(1, len(self.remote_collection("ubuntu-latest", self.second)["observations"]))

    def test_a_recovered_failure_is_recorded_and_earns_no_credit(self) -> None:
        self.github.add_run("100")
        self.finish_cell("alpha", "ubuntu-latest", failing=("suite::boom",))
        self.finish_cell("alpha", "wsl2-ubuntu")
        code, errors = self.recover("--no-push")
        self.assertEqual(0, code, errors)
        outcomes = [o["cell"]["outcome"]
                    for o in self.local_collection("ubuntu-latest", self.second)["observations"]]
        self.assertEqual(["fail"], outcomes)
        self.assertEqual(["alpha/wsl2-ubuntu/L1"],
                         self.credited(self.plan_file(), self.first, self.second))

    def test_expired_or_missing_artifacts_leave_their_cell_to_execute(self) -> None:
        self.orphan()
        self.github.expire("100", "observation-alpha-L1-wsl2-ubuntu")
        self.github.delete("100", "junit-alpha-L1-ubuntu-latest")
        code, errors = self.recover("--no-push")
        self.assertEqual(0, code, errors)
        self.assertIn("has expired", errors)
        self.assertIsNone(self.local_collection("wsl2-ubuntu", self.second))
        # Recorded, but its cited report is gone, so discovery refuses it.
        self.assertIsNotNone(self.local_collection("ubuntu-latest", self.second))
        accepted, rejections = self.verify(self.plan_file(), self.first, self.second)
        self.assertEqual([], accepted)
        self.assertTrue(
            any(r.startswith("observation-report-unavailable") for r in rejections), rejections
        )

    def test_runs_that_are_not_completed_nightlies_of_this_workflow_are_never_collected(self) -> None:
        rows = {
            "pull request": {"event": "pull_request"},
            "push": {"event": "push"},
            "manual dispatch": {"event": "workflow_dispatch"},
            "another workflow": {"path": ".github/workflows/other.yml"},
            "another branch": {"head_branch": "feature"},
            "another repository": {"repository": {"full_name": "fork/repo"}},
            "still running": {"status": "in_progress"},
            "string id": {"id": "100"},
        }
        for label, fields in rows.items():
            with self.subTest(label):
                self.reset_notes()
                self.github = FakeGitHub()
                self.orphan(**fields)
                self.github.calls.clear()
                code, errors = self.recover("--no-push")
                self.assertEqual(0, code, errors)
                self.assertEqual(
                    [], [call for call in self.github.calls if "/actions/runs/" in call], label
                )
                self.assertEqual([], self.credited(self.plan_file(), self.first, self.second))

    def test_only_the_newest_bounded_window_of_runs_is_collected(self) -> None:
        limit = hosted_observations.RECOVERY_RUNS
        oldest = 100
        self.github.add_run(str(oldest))
        self.finish_cell("alpha", "wsl2-ubuntu", run=str(oldest))
        for run in range(oldest + 1, oldest + 1 + limit):
            self.github.add_run(str(run))
        self.github.calls.clear()
        code, errors = self.recover("--no-push")
        self.assertEqual(0, code, errors)
        listed = {call.split("/runs/")[1].split("/")[0]
                  for call in self.github.calls if "/actions/runs/" in call}
        self.assertEqual({str(run) for run in range(oldest + 1, oldest + 1 + limit)}, listed)
        self.assertEqual([], self.credited(self.plan_file(), self.first, self.second))

    def test_one_uncollectable_run_does_not_withhold_another_and_fails_visibly(self) -> None:
        self.orphan("100")
        self.github.add_run("101")
        self.github.upload("101", "observation-alpha-L1-macos-latest", {"x": b"y"})
        for artifact in self.github.artifacts.values():
            if artifact["run"] == "101":
                artifact["files"] = None  # an archive that cannot be built
        original = self.github.fetch

        def fetch(path: str) -> bytes:
            if path.endswith("/zip") and self.github.artifacts[int(path.split("/")[-2])]["files"] is None:
                return b"not a zip"
            return original(path)

        errors = io.StringIO()
        with contextlib.redirect_stderr(errors), contextlib.redirect_stdout(io.StringIO()):
            code = hosted_observations.main(
                ["publish", "--repository", REPOSITORY, "--recent", "--no-push"], fetch=fetch
            )
        self.assertEqual(hosted_observations.EXIT_REJECTED, code, errors.getvalue())
        self.assertIn("cannot collect run 101", errors.getvalue())
        self.assertEqual(["alpha/ubuntu-latest/L1", "alpha/wsl2-ubuntu/L1"],
                         self.credited(self.plan_file(), self.first, self.second))

    def test_an_unreadable_run_list_is_a_visible_failure_that_credits_nothing(self) -> None:
        self.orphan()
        self.github.broken = True
        code, errors = self.recover("--no-push")
        self.assertEqual(hosted_observations.EXIT_NO_ARTIFACTS, code)
        self.assertIn("cannot list earlier scheduled runs", errors)
        self.github.broken = False
        self.assertEqual([], self.credited(self.plan_file(), self.first, self.second))


class ObservationFieldMatrixTests(HostedFixture):
    """Every field `emit` writes, absent/null/wrong-typed, through `publish`."""

    def setUp(self) -> None:
        super().setUp()
        self.envelope = self.finish_cell("alpha", "ubuntu-latest")
        self.rows: list[tuple[str, tuple, object]] = []
        report = ("observation", "reports", 0)
        for field, bad in (
            ("path", [ABSENT, None, 7, ""]),
            ("sha256", [ABSENT, None, 7, "", "zz" * 32, "A" * 64]),
            ("bytes", [ABSENT, None, "5", -1, 1.5, True]),
            ("store", [ABSENT, None, 5, "git", "host-dir"]),
        ):
            self.rows += [(f"report {field}={value!r}", (*report, field), value) for value in bad]
        proof = ("observation", "proof")
        for field in ("nextest_version", "target", "expected_listing_digest",
                      "completion_record_digest"):
            self.rows += [
                (f"proof {field}={value!r}", (*proof, field), value)
                for value in (ABSENT, None, 7, "")
            ]
        self.rows += [("proof=None", proof, None), ("proof=7", proof, 7), ("proof absent", proof, ABSENT)]
        producer = ("observation", "producer")
        for field, bad in (
            ("run_id", [ABSENT, None, 7]),
            ("attempt", [ABSENT, None, "1", 0, True]),
            ("workflow", [ABSENT, None, 7]),
            ("kind", ["rig", "hook", None]),
        ):
            self.rows += [(f"producer {field}={value!r}", (*producer, field), value) for value in bad]
        self.rows += [
            ("host label absent", (*producer, "host", "label"), ABSENT),
            ("host label of another environment", (*producer, "host", "label"), "macos-latest"),
            ("cell report absent", ("observation", "cell", "report"), ABSENT),
            ("cell report null", ("observation", "cell", "report"), None),
            ("environment unknown", ("environment",), "plan9"),
            ("environment null", ("environment",), None),
            ("head short", ("head",), "abc"),
            ("head null", ("head",), None),
            ("head not in the checkout", ("head",), "0" * 40),
        ]

    def envelope_with(self, path: tuple, value: object) -> dict:
        document = json.loads(json.dumps(self.envelope))
        if path[0] != "observation":
            if value is ABSENT:
                del document[path[0]]
            else:
                document[path[0]] = value
            return document
        edited = with_edit(document["observation"], path[1:], value)
        document["observation"] = edited
        return document

    def publish_one(self, document: dict, label: str):
        directory = self.scratch / ("row-" + hashlib.sha1(label.encode()).hexdigest()[:10])
        directory.mkdir()
        (directory / "observation.json").write_text(json.dumps(document), encoding="utf-8")
        with contextlib.redirect_stderr(io.StringIO()), contextlib.redirect_stdout(io.StringIO()):
            return hosted_observations.publish_directory(directory)

    def test_every_edit_is_rejected_before_anything_is_published(self) -> None:
        for label, path, value in self.rows:
            with self.subTest(label):
                summary = self.publish_one(self.envelope_with(path, value), label)
                self.assertEqual(1, len(summary.rejected), label)
                self.assertEqual([], summary.published)
                self.assertIsNone(self.remote_collection("ubuntu-latest", self.second))
                self.assertIsNone(self.local_collection("ubuntu-latest", self.second))

    def test_the_unedited_control_is_published_and_credited(self) -> None:
        summary = self.publish_one(self.envelope, "control")
        self.assertEqual([], summary.rejected)
        self.assertEqual(1, len(summary.published))
        self.assertEqual(["alpha/ubuntu-latest/L1"],
                         self.credited(self.plan_file(), self.first, self.second))

    def test_digests_that_are_well_formed_but_wrong_publish_and_then_earn_no_credit(self) -> None:
        wrong = "0" * 64
        rows = {
            "report sha256": (("observation", "reports", 0, "sha256"), wrong),
            "report bytes": (("observation", "reports", 0, "bytes"),
                             self.envelope["observation"]["reports"][0]["bytes"] + 1),
            "expected listing digest": (("observation", "proof", "expected_listing_digest"), wrong),
            "completion record digest": (("observation", "proof", "completion_record_digest"), wrong),
            "report path": (("observation", "reports", 0, "path"), "L1/missing.xml"),
        }
        for label, (path, value) in rows.items():
            with self.subTest(label):
                self.reset_notes()
                document = self.envelope_with(path, value)
                if label == "report path":
                    document["observation"] = with_edit(
                        document["observation"], ("cell", "report"), "L1/missing.xml"
                    )
                summary = self.publish_one(document, label)
                self.assertEqual([], summary.rejected, label)
                self.assertEqual(1, len(summary.published))
                accepted, rejections = self.verify(self.plan_file(), self.first, self.second)
                self.assertEqual([], accepted)
                self.assertTrue(
                    any(r.startswith("observation-report-unavailable") for r in rejections),
                    rejections,
                )


if __name__ == "__main__":
    unittest.main(verbosity=2)

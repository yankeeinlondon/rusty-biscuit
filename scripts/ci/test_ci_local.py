#!/usr/bin/env python3
"""Exercise the real local CI recipe with isolated command stubs, without Rust builds."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import schema  # noqa: E402


ROOT = Path(__file__).resolve().parents[2]
RECIPE = ROOT / "just" / "ci-local.just"
DEVOPS = ROOT / "just" / "devops.just"
JUST = shutil.which("just")


def thread_policy_recipe() -> str:
    lines = DEVOPS.read_text(encoding="utf-8").splitlines(keepends=True)
    start = next(index for index, line in enumerate(lines) if line.startswith("_test_threads:"))
    end = start + 1
    while end < len(lines) and (not lines[end].strip() or lines[end][0].isspace()):
        end += 1
    return "".join(lines[start:end])


def clean_policy_environment() -> dict[str, str]:
    environment = os.environ.copy()
    for key in ("CI", "GITHUB_ACTIONS", "BISCUIT_CI_ENVIRONMENT", "NEXTEST_TEST_THREADS"):
        environment.pop(key, None)
    return environment


@unittest.skipUnless(JUST and shutil.which("jq"), "requires just and jq")
class CiLocalTests(unittest.TestCase):
    def run_recipe(
        self, threads: str | None = None, cores: int = 16, reports: dict | None = None
    ) -> list[dict]:
        with tempfile.TemporaryDirectory(prefix="ci-local-test-") as temporary:
            root = Path(temporary)
            bin_dir = root / "bin"
            scripts = root / "scripts" / "ci"
            bin_dir.mkdir()
            scripts.mkdir(parents=True)
            shutil.copyfile(RECIPE, root / "ci-local.just")
            (root / "policy.just").write_text(thread_policy_recipe(), encoding="utf-8")
            (root / "justfile").write_text(
                'red := ""\ngreen := ""\nreset := ""\nimport "ci-local.just"\n',
                encoding="utf-8",
            )
            scope = {
                "packages": ["parallel", "serial"],
                "full_scope": False,
                "change_class": "package",
                "full_scope_gates": [],
                "matrix": [
                    {
                        "package": name,
                        "gates": ["lint", "check", "test"],
                        "tiers": ["L1", "L2"],
                        "test_args": "--features terminal-tests,daemon-tests",
                        "runner_tools": ["l2-parallel-self-spawn"] if name == "parallel" else [],
                        "l2_environments": ["macos-latest"],
                        "l2_backends": ["tmux"],
                    }
                    for name in ("parallel", "serial")
                ],
            }
            (root / "scope.json").write_text(json.dumps(scope), encoding="utf-8")
            (scripts / "affected_scope.py").write_text(
                'from pathlib import Path\nprint(Path("scope.json").read_text())\n',
                encoding="utf-8",
            )
            # The recipe's self-test loop; each is a no-op stub here because
            # this fixture tests the recipe's scheduling, not those suites.
            for suite in (
                "test_schema.py",
                "test_affected_scope.py",
                "test_resolved_plan.py",
                "test_ci_local.py",
                "test_constraints.py",
                "test_runner_loss.py",
            ):
                (scripts / suite).write_text("", encoding="utf-8")
            stubs = {
                "sniff": (
                    "import json, os, sys\n"
                    "if sys.argv[1:] == ['os', '--json']:\n"
                    "    print(json.dumps({'os_type': 'MacOS'}))\n"
                    "elif sys.argv[1:] == ['cpu', '--json']:\n"
                    "    print(json.dumps({'logical_cores': int(os.environ['TEST_CORES'])}))\n"
                    "else:\n"
                    "    raise SystemExit('unexpected sniff arguments: ' + repr(sys.argv))\n"
                ),
                "just": (
                    "import json, os, subprocess, sys\n"
                    "if sys.argv[1:] == ['_test_threads']:\n"
                    "    raise SystemExit(subprocess.run([os.environ['TEST_REAL_JUST'], '--justfile', os.environ['TEST_POLICY_RECIPE'], '_test_threads'], check=False).returncode)\n"
                    "assert sys.argv[1] in ('_lint', '_test', '_test_l2'), sys.argv\n"
                    "with open(os.environ['TEST_CALL_LOG'], 'a', encoding='utf-8') as log:\n"
                    "    log.write(json.dumps({'args': sys.argv[1:], "
                    "'threads': os.environ.get('BISCUIT_L2_THREADS'), "
                    "'backends': os.environ.get('BISCUIT_TEST_REQUIRED_BACKENDS')}) + '\\n')\n"
                ),
                "tmux": "raise SystemExit('tmux must only be detected, never started')\n",
                "cargo": "raise SystemExit('Rust builds are forbidden in this test')\n",
            }
            for name, body in stubs.items():
                path = bin_dir / name
                path.write_text("#!/usr/bin/env python3\n" + body, encoding="utf-8")
                path.chmod(0o755)
            environment = clean_policy_environment()
            environment.update({
                "PATH": str(bin_dir) + os.pathsep + environment.get("PATH", ""),
                "TEST_CALL_LOG": str(root / "calls.jsonl"),
                "TEST_CORES": str(cores),
                "TEST_REAL_JUST": JUST,
                "TEST_POLICY_RECIPE": str(root / "policy.just"),
            })
            for key in (
                "BISCUIT_L2_THREADS", "BISCUIT_TEST_REQUIRED_BACKENDS",
                "BISCUIT_CI_SCOPE_OUT", "WEZTERM_UNIX_SOCKET", "KITTY_LISTEN_ON",
            ):
                environment.pop(key, None)
            if threads is not None:
                environment["BISCUIT_L2_THREADS"] = threads
            if reports is not None:
                environment["BISCUIT_CI_REPORTS_OUT"] = str(root / "reports")
            result = subprocess.run(
                [JUST, "--justfile", str(root / "justfile"), "ci-local", "--all", "--l2"],
                cwd=root,
                env=environment,
                capture_output=True,
                text=True,
                timeout=30,
            )
            self.assertEqual(0, result.returncode, result.stdout + result.stderr)
            if reports is not None:
                staged = root / "reports" / "gate-backends.jsonl"
                reports["gate_backends"] = (
                    [json.loads(line) for line in staged.read_text().splitlines()]
                    if staged.is_file()
                    else []
                )
            return [json.loads(line) for line in (root / "calls.jsonl").read_text().splitlines()]

    def test_parallel_policy_is_bounded_and_does_not_leak_to_other_gates(self) -> None:
        calls = self.run_recipe()
        l2 = [call for call in calls if call["args"][0] == "_test_l2"]
        self.assertEqual(["14", "1"], [call["threads"] for call in l2])
        for call in l2:
            self.assertEqual("tmux", call["backends"])
            self.assertEqual(["--features", "terminal-tests,daemon-tests"], call["args"][2:])
        other = [call for call in calls if call["args"][0] != "_test_l2"]
        self.assertEqual(4, len(other))
        self.assertTrue(all(call["threads"] is None and call["backends"] is None for call in other))

    def test_explicit_thread_override_is_preserved_for_every_package(self) -> None:
        for threads in ("1", "4"):
            with self.subTest(threads=threads):
                calls = self.run_recipe(threads=threads)
                l2 = [call for call in calls if call["args"][0] == "_test_l2"]
                self.assertEqual([threads, threads], [call["threads"] for call in l2])

    def test_each_l2_invocation_records_the_backends_it_required(self) -> None:
        # The receipt's backend proof is the intersection of these names with
        # the backends `test-toolkit` saw drive a test. Without this half a
        # Level 2 cell that merely SKIPPED — nextest prints PASS in about
        # 0.02 s — would be indistinguishable from evidence.
        reports: dict = {}
        self.run_recipe(reports=reports)
        self.assertEqual(
            [
                {"package": "parallel", "gate": "L2", "backends": ["tmux"]},
                {"package": "serial", "gate": "L2", "backends": ["tmux"]},
            ],
            reports["gate_backends"],
        )

    def test_parallel_policy_reserves_two_cores_on_small_hosts(self) -> None:
        calls = self.run_recipe(cores=3)
        l2 = [call for call in calls if call["args"][0] == "_test_l2"]
        self.assertEqual(["1", "1"], [call["threads"] for call in l2])


@unittest.skipUnless(JUST and shutil.which("jq"), "requires just and jq")
class ThreadPolicyTests(unittest.TestCase):
    def run_policy(self, cores: int, markers: dict[str, str] | None = None, sniff_fails: bool = False) -> str:
        with tempfile.TemporaryDirectory(prefix="test-thread-policy-") as temporary:
            root = Path(temporary)
            bin_dir = root / "bin"
            bin_dir.mkdir()
            recipe = root / "justfile"
            recipe.write_text(thread_policy_recipe(), encoding="utf-8")
            sniff = bin_dir / "sniff"
            sniff.write_text(
                "#!/usr/bin/env python3\nimport json, os, sys\n"
                "assert sys.argv[1:] == ['cpu', '--json'], sys.argv\n"
                "if os.environ.get('TEST_SNIFF_FAILS') == '1':\n    raise SystemExit(1)\n"
                "print(json.dumps({'logical_cores': int(os.environ['TEST_CORES'])}))\n",
                encoding="utf-8",
            )
            sniff.chmod(0o755)
            for name in ("getconf", "sysctl", "nproc"):
                path = bin_dir / name
                path.write_text(
                    "#!/usr/bin/env python3\nimport os\nprint(os.environ['TEST_CORES'])\n",
                    encoding="utf-8",
                )
                path.chmod(0o755)
            environment = clean_policy_environment()
            environment.update({
                "PATH": str(bin_dir) + os.pathsep + environment.get("PATH", ""),
                "TEST_CORES": str(cores),
                "TEST_SNIFF_FAILS": "1" if sniff_fails else "0",
            })
            environment.update(markers or {})
            result = subprocess.run(
                [JUST, "--justfile", str(recipe), "_test_threads"],
                cwd=root,
                env=environment,
                capture_output=True,
                text=True,
                timeout=10,
            )
            self.assertEqual(0, result.returncode, result.stdout + result.stderr)
            actual = result.stdout.strip()
            self.assertRegex(actual, r"^[1-9][0-9]*$")
            return actual

    def test_local_policy_leaves_two_cores_with_one_thread_floor(self) -> None:
        for cores, expected in ((1, 1), (2, 1), (3, 1), (4, 2), (5, 3), (8, 6), (16, 14)):
            with self.subTest(cores=cores):
                self.assertEqual(str(expected), self.run_policy(cores))

    def test_each_ci_marker_keeps_all_cores_on_small_runners(self) -> None:
        for marker in ({"CI": "true"}, {"GITHUB_ACTIONS": "true"}, {"BISCUIT_CI_ENVIRONMENT": "windows-latest"}):
            for cores, expected in ((1, 1), (2, 2), (3, 3), (4, 4), (5, 3), (8, 6), (16, 14)):
                with self.subTest(marker=marker, cores=cores):
                    self.assertEqual(str(expected), self.run_policy(cores, marker))

    def test_false_ci_markers_keep_local_policy(self) -> None:
        self.assertEqual("2", self.run_policy(4, {"CI": "false", "GITHUB_ACTIONS": "false", "BISCUIT_CI_ENVIRONMENT": ""}))

    def test_platform_fallback_retains_policy_when_sniff_fails(self) -> None:
        self.assertEqual("6", self.run_policy(8, sniff_fails=True))
        self.assertEqual("4", self.run_policy(4, {"CI": "true"}, sniff_fails=True))


@unittest.skipUnless(JUST and shutil.which("jq"), "requires just and jq")
class L1ThreadForwardingTests(unittest.TestCase):
    def test_canonical_l1_forwards_default_and_preserves_explicit_override(self) -> None:
        lines = DEVOPS.read_text(encoding="utf-8").splitlines(keepends=True)
        start = next(index for index, line in enumerate(lines) if line.startswith("_test pkg "))
        end = start + 1
        while end < len(lines) and (not lines[end].strip() or lines[end][0].isspace()):
            end += 1
        recipe = "".join(lines[start:end])
        for override, expected in ((None, "14"), ("1", "1"), ("7", "7")):
            with self.subTest(override=override), tempfile.TemporaryDirectory(prefix="l1-threads-") as temporary:
                root = Path(temporary)
                bin_dir = root / "bin"
                bin_dir.mkdir()
                policy = root / "policy.just"
                policy.write_text(thread_policy_recipe(), encoding="utf-8")
                justfile = root / "justfile"
                justfile.write_text(
                    "".join(f'{name} := ""\n' for name in ("bold", "reset", "red", "green", "yellow"))
                    + "_storage_preflight:\n    @true\n\n" + recipe,
                    encoding="utf-8",
                )
                stubs = {
                    "sniff": "import json\nprint(json.dumps({'logical_cores': 16}))\n",
                    "cargo": (
                        "import json, os, sys\n"
                        "assert sys.argv[1] == 'nextest', sys.argv\n"
                        "assert sys.argv[2] in ('--version', 'run'), sys.argv\n"
                        "if sys.argv[2] == 'run':\n"
                        "    with open(os.environ['TEST_CALL_LOG'], 'a') as log:\n"
                        "        log.write(json.dumps({'args': sys.argv[1:], 'threads': os.environ.get('NEXTEST_TEST_THREADS')}) + '\\n')\n"
                    ),
                    "just": (
                        "import os, subprocess, sys\n"
                        "if sys.argv[1] == '_test_threads':\n"
                        "    assert 'TEST_POLICY_FORBIDDEN' not in os.environ, 'explicit override must skip calculation'\n"
                        "    raise SystemExit(subprocess.run([os.environ['TEST_REAL_JUST'], '--justfile', os.environ['TEST_POLICY_RECIPE'], '_test_threads'], check=False).returncode)\n"
                        "if sys.argv[1] == '_tier_filter':\n    print('all()')\n"
                        "else:\n    assert sys.argv[1] in ('_stage_junit_reset', '_stage_junit', '_speak'), sys.argv\n"
                    ),
                }
                for name, body in stubs.items():
                    path = bin_dir / name
                    path.write_text("#!/usr/bin/env python3\n" + body, encoding="utf-8")
                    path.chmod(0o755)
                environment = clean_policy_environment()
                for key in ("BISCUIT_BUILD_SCOPE", "BISCUIT_BUILD_FEATURES", "BISCUIT_TEST_RECORD_TIMING"):
                    environment.pop(key, None)
                environment.update({
                    "PATH": str(bin_dir) + os.pathsep + environment.get("PATH", ""),
                    "BISCUIT_NEXTEST_BIN": "cargo nextest",
                    "TEST_REAL_JUST": JUST,
                    "TEST_POLICY_RECIPE": str(policy),
                    "TEST_CALL_LOG": str(root / "calls.jsonl"),
                })
                if override is not None:
                    environment["NEXTEST_TEST_THREADS"] = override
                    environment["TEST_POLICY_FORBIDDEN"] = "1"
                result = subprocess.run(
                    [JUST, "--justfile", str(justfile), "_test", "fixture-package"],
                    cwd=root, env=environment, capture_output=True, text=True, timeout=10,
                )
                self.assertEqual(0, result.returncode, result.stdout + result.stderr)
                calls = [json.loads(line) for line in (root / "calls.jsonl").read_text().splitlines()]
                self.assertEqual(1, len(calls))
                self.assertEqual(expected, calls[0]["threads"])
                self.assertEqual(["nextest", "run", "-p", "fixture-package"], calls[0]["args"][:4])


# ---------------------------------------------------------------------------
# Frozen contracts for `just ci-local --plan` (fixes/2026-09-11-cicd-cleanup)
#
# The worker-limit, explicit-override, and non-focusing-L2 contracts this fix
# must PRESERVE are already covered above by CiLocalTests, ThreadPolicyTests,
# and L1ThreadForwardingTests. Nothing is duplicated here; what follows is only
# the pre-trigger plan surface, which does not exist yet.
# ---------------------------------------------------------------------------


@unittest.skipUnless(JUST and shutil.which("jq"), "requires just and jq")
class PlanSurfaceTests(unittest.TestCase):
    """AC17: the reviewable plan shown before a push or other trigger."""

    def resolved_plan(self, prohibited_is_covered: bool) -> dict:
        cells = [
            {
                "package": "alpha",
                "area": "pkg",
                "environment": "macos-latest",
                "gate": "L1",
                "execution": "reuse",
                "origin": "local",
                "state": "reused",
                "target_kinds": ["lib", "test"],
                "compile_coverage_from": "L1",
                "selection_reason": "satisfied by this host's receipt",
                "evidence": {"ref": "refs/notes/ci-local/macos-latest"},
            },
            {
                "package": "alpha",
                "area": "pkg",
                "environment": "ubuntu-latest",
                "gate": "L1",
                "execution": "execute",
                "origin": "ci",
                "state": "pending",
                "target_kinds": ["lib", "test"],
                "compile_coverage_from": "L1",
                "selection_reason": "no evidence for this environment",
            },
            {
                "package": "alpha",
                "area": "pkg",
                "environment": "windows-latest",
                "gate": "L2",
                "execution": "omit",
                "origin": "none",
                "state": "accepted-gap",
                "target_kinds": ["test"],
                "compile_coverage_from": None,
                "selection_reason": "no L2 backend is provisionable on Windows",
                "gap": {
                    "owner": "@yankeeinlondon",
                    "reason": "no L2 terminal backend is provisionable on Windows",
                    "expiry": "2027-01-31",
                },
            },
            {
                "package": "alpha",
                "area": "pkg",
                "environment": "wsl2-ubuntu",
                "gate": "L1",
                "execution": "reuse" if prohibited_is_covered else "omit",
                "origin": "prior-local" if prohibited_is_covered else "none",
                "state": "reused" if prohibited_is_covered else "prohibited",
                "target_kinds": ["lib", "test"],
                "compile_coverage_from": "L1",
                "selection_reason": (
                    "satisfied by a prior cross-check receipt"
                    if prohibited_is_covered
                    else "a persisted constraint forbids running WSL for this branch"
                ),
                **(
                    {"evidence": {"ref": "refs/notes/ci-local/wsl2-ubuntu"}}
                    if prohibited_is_covered
                    else {
                        "prohibition": {
                            "owner": "ken",
                            "reason": "do not rerun WSL for this branch",
                            "expiry": "2099-01-01",
                        }
                    }
                ),
            },
        ]
        return {
            "schema_version": schema.RESOLVED_PLAN_SCHEMA_VERSION,
            "base": "a" * 40,
            "head": "b" * 40,
            "change_class": "package",
            "full_scope": False,
            "full_scope_gates": [],
            "areas": [
                {"area": "pkg", "selection_reason": "source change", "packages": ["alpha"]}
            ],
            "packages": [
                {
                    "package": "alpha",
                    "area": "pkg",
                    "selection_reason": "source change",
                    "gates": ["L1", "L2"],
                    "targets": ["lib", "test"],
                    "tiers": ["L1", "L2"],
                    "test_args": "",
                    "check_args": "-p alpha",
                    "l2_backends": ["tmux"],
                    "runner_tools": [],
                    "companion_suites": [],
                    "native": {},
                }
            ],
            "source_packages": ["alpha"],
            "reverse_dependencies": [],
            "cells": cells,
            "accepted_evidence": [],
            "policy_gaps": [],
            "prohibited_cells": [] if prohibited_is_covered else ["alpha/wsl2-ubuntu/L1"],
            "job_estimate": len(cells),
            "preflight_os": ["ubuntu-latest"],
            "preflight_reason": "package-local change",
            "flags": {"ci_tooling": False},
        }

    def run_plan_with_output(self) -> tuple[str, dict]:
        """The rendered plan and the canonical JSON `--plan-out` wrote."""
        captured: dict = {}
        result = self.run_plan(capture=captured)
        self.assertEqual(0, result.returncode, result.stdout + result.stderr)
        return result.stdout, captured["plan"]

    def run_plan(
        self, prohibited_is_covered: bool = True, capture: dict | None = None
    ) -> subprocess.CompletedProcess:
        document = self.resolved_plan(prohibited_is_covered)
        self.assertEqual(
            [],
            schema.validate_resolved_plan(document),
            "the fixture's own plan must be valid, or a failure here would be "
            "the fixture's fault rather than the recipe's",
        )
        with tempfile.TemporaryDirectory(prefix="ci-local-plan-") as temporary:
            root = Path(temporary)
            bin_dir = root / "bin"
            scripts = root / "scripts" / "ci"
            bin_dir.mkdir()
            scripts.mkdir(parents=True)
            shutil.copyfile(RECIPE, root / "ci-local.just")
            (root / "policy.just").write_text(thread_policy_recipe(), encoding="utf-8")
            (root / "justfile").write_text(
                'red := ""\ngreen := ""\nreset := ""\nimport "ci-local.just"\n',
                encoding="utf-8",
            )
            (root / "plan.json").write_text(schema.canonical(document), encoding="utf-8")
            (scripts / "affected_scope.py").write_text(
                'from pathlib import Path\nprint(Path("plan.json").read_text())\n',
                encoding="utf-8",
            )
            for name in ("cargo", "sniff"):
                path = bin_dir / name
                path.write_text(
                    "#!/usr/bin/env python3\n"
                    f"raise SystemExit('--plan must run nothing, but called {name}')\n",
                    encoding="utf-8",
                )
                path.chmod(0o755)
            environment = clean_policy_environment()
            environment["PATH"] = str(bin_dir) + os.pathsep + environment.get("PATH", "")
            environment.pop("BISCUIT_CI_PLAN_OUT", None)
            environment.pop("BISCUIT_CI_CONSTRAINTS_DIR", None)
            command = [JUST, "--justfile", str(root / "justfile"), "ci-local", "--all", "--plan"]
            if capture is not None:
                command += ["--plan-out", str(root / "written-plan.json")]
            result = subprocess.run(
                command,
                cwd=root,
                env=environment,
                capture_output=True,
                text=True,
                timeout=30,
            )
            if capture is not None:
                capture["plan"] = json.loads(
                    (root / "written-plan.json").read_text(encoding="utf-8")
                )
            return result

    def test_plan_is_an_accepted_flag(self) -> None:
        result = self.run_plan()
        self.assertEqual(0, result.returncode, result.stdout + result.stderr)

    def test_plan_shows_every_cell_disposition(self) -> None:
        result = self.run_plan()
        self.assertEqual(0, result.returncode, result.stdout + result.stderr)
        for marker in ("macos-latest", "ubuntu-latest", "windows-latest", "wsl2-ubuntu"):
            self.assertIn(marker, result.stdout, f"the plan omits {marker}")
        for marker in ("reuse", "execute", "accepted-gap"):
            self.assertIn(marker, result.stdout, f"the plan never says {marker!r}")

    def test_plan_exits_nonzero_when_a_prohibited_cell_is_unsatisfied(self) -> None:
        result = self.run_plan(prohibited_is_covered=False)
        self.assertNotEqual(
            0,
            result.returncode,
            "a prohibited environment with no qualifying evidence must stop the "
            "trigger, not be silently dropped",
        )
        self.assertIn("wsl2-ubuntu", result.stdout + result.stderr)

    def test_plan_exits_zero_when_the_prohibited_cell_is_covered_by_evidence(self) -> None:
        result = self.run_plan(prohibited_is_covered=True)
        self.assertEqual(0, result.returncode, result.stdout + result.stderr)

    def test_plan_runs_no_gate_and_builds_nothing(self) -> None:
        # `cargo` and `sniff` are stubbed to fail loudly in `run_plan`, so a
        # zero exit is itself the proof that neither was reached.
        result = self.run_plan()
        self.assertEqual(0, result.returncode, result.stdout + result.stderr)
        self.assertNotIn("must run nothing", result.stdout + result.stderr)

    def test_plan_extends_the_preview_rather_than_replacing_it(self) -> None:
        self.assertIn("--dry-run", RECIPE.read_text(encoding="utf-8"))

    def test_plan_names_the_constraint_that_blocks_the_trigger(self) -> None:
        result = self.run_plan(prohibited_is_covered=False)
        combined = result.stdout + result.stderr
        self.assertIn("do not rerun WSL for this branch", combined)
        self.assertIn("ken", combined)
        self.assertIn("2099-01-01", combined)

    def test_plan_writes_the_same_cells_it_rendered(self) -> None:
        # The rendered table is a projection; the canonical JSON is the machine
        # interface. AC17 needs them to describe one plan, not two.
        rendered, document = self.run_plan_with_output()
        self.assertEqual([], schema.validate_resolved_plan(document))
        for cell in document["cells"]:
            key = f"{cell['package']}/{cell['environment']}/{cell['gate']}"
            self.assertIn(key, rendered, f"the rendered plan omits {key}")


WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"
SCOPE_STEP = "Calculate package and area scope"
NULL_OID = "0" * 40


def workflow_step_script(workflow: Path, step_name: str) -> str:
    """The `run: |` body of one named step, dedented into a standalone script.

    The stdlib has no YAML parser, so this leans on the workflow's fixed
    layout: a step opens at six spaces, its keys sit at eight, and a block
    scalar's lines at ten. Reaching the next step before `run: |` fails
    loudly rather than borrowing a neighbor's script.
    """
    lines = workflow.read_text(encoding="utf-8").splitlines()
    start = lines.index(f"      - name: {step_name}")
    run_index = None
    for index in range(start + 1, len(lines)):
        if lines[index].startswith("      - "):
            break
        if lines[index] == "        run: |":
            run_index = index
            break
    if run_index is None:
        raise AssertionError(f"step {step_name!r} has no `run: |` block")
    indent = 10
    body: list[str] = []
    for line in lines[run_index + 1 :]:
        if line.strip() and len(line) - len(line.lstrip()) < indent:
            break
        body.append(line[indent:])
    return "\n".join(body) + "\n"


SCOPE_REF = "refs/notes/ci-local/scope"
SCOPE_MARKER = "carried by the local scope receipt"
#: A real workspace source path, so a fixture commit touching it selects a
#: real package and the plan owns cells a validation receipt can satisfy.
SOURCE_FILE = "biscuit-hash/lib/src/lib.rs"


def fixture_git(root: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-c", "user.name=t", "-c", "user.email=t@example.invalid",
         "-c", "commit.gpgsign=false", *args],
        cwd=root, check=True, capture_output=True, text=True,
        env={**os.environ, "GIT_TERMINAL_PROMPT": "0"},
    ).stdout.strip()


class StepRun:
    """One execution of the extracted scope step."""

    def __init__(self, root: Path, result: subprocess.CompletedProcess, output: Path, summary: Path, planner_log: Path) -> None:
        self.result = result
        self.summary = summary.read_text(encoding="utf-8")
        self.planner_calls = planner_log.read_text(encoding="utf-8").splitlines() if planner_log.is_file() else []
        self.outputs = dict(
            line.split("=", 1)
            for line in output.read_text(encoding="utf-8").splitlines()
            if "=" in line
        )
        plan_path = root / "resolved-plan.json"
        self.plan_bytes = plan_path.read_text(encoding="utf-8") if plan_path.is_file() else ""
        self.plan = json.loads(self.plan_bytes) if self.plan_bytes else {}

    def scope_source(self) -> str:
        for line in self.summary.splitlines():
            if line.startswith("| scope source |"):
                return line.split("|")[2].strip()
        return ""


@unittest.skipUnless(shutil.which("bash") and shutil.which("jq"), "requires bash and jq")
class WorkflowScopeStepTests(unittest.TestCase):
    """The scope step's shell, run for real against every event shape it handles.

    The REAL planner and evidence verifier run (the workspace's, reached
    through a trampoline in the temp cwd, since the step spells them as
    cwd-relative paths); only the Git repository the step diffs and verifies
    against is a fixture. Three commits — `root`, `base`, `head` — give the
    diff-based events a resolvable `base..head` and a second real base for a
    mismatched scope receipt; the full-run events never look at them.
    """

    @classmethod
    def setUpClass(cls) -> None:
        cls.script = workflow_step_script(WORKFLOW, SCOPE_STEP)

    def run_step(
        self,
        event: str,
        *,
        push_base: str | None = None,
        scope_receipt=None,
        validation_receipt: bool = False,
        expect_failure: bool = False,
    ) -> StepRun:
        """The step for one event, over a fresh fixture repository.

        `scope_receipt(root, base, head) -> str` is attached to `head` under
        `refs/notes/ci-local/scope` before the step runs; `validation_receipt`
        publishes a complete macOS L1 receipt for the source package so the
        accepted-cells path is exercised.
        """
        with tempfile.TemporaryDirectory(prefix="ci-scope-step-") as temporary:
            root = Path(temporary).resolve()
            first, base, head = self.seed_repository(root)
            scripts = root / "scripts" / "ci"
            scripts.mkdir(parents=True)
            planner_log = root / "planner-calls.log"
            for tool in ("affected_scope.py", "local_evidence.py"):
                real = ROOT / "scripts" / "ci" / tool
                # The planner trampoline records every invocation: on a scope
                # hit the proof is that it was never invoked for selection.
                record = (
                    "import os, sys\n"
                    "if os.environ.get('TEST_PLANNER_LOG'):\n"
                    "    with open(os.environ['TEST_PLANNER_LOG'], 'a', encoding='utf-8') as log:\n"
                    "        log.write(' '.join(sys.argv[1:]) + '\\n')\n"
                    if tool == "affected_scope.py"
                    else ""
                )
                (scripts / tool).write_text(
                    "import runpy\n" + record
                    + f"runpy.run_path({str(real)!r}, run_name='__main__')\n",
                    encoding="utf-8",
                )
            shutil.copyfile(ROOT / "rust-toolchain.toml", root / "rust-toolchain.toml")
            if scope_receipt is not None:
                fixture_git(root, "notes", "--ref", SCOPE_REF, "add", "-f", "-m",
                            scope_receipt(root, base, head), head)
            if validation_receipt:
                self.publish_validation_receipt(root, base, head)
            output = root / "github-output"
            summary = root / "github-step-summary"
            output.touch()
            summary.touch()

            environment = clean_policy_environment()
            for key in ("GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"):
                environment.pop(key, None)
            environment.update(
                {
                    "GITHUB_OUTPUT": str(output),
                    "GITHUB_STEP_SUMMARY": str(summary),
                    "TEST_PLANNER_LOG": str(planner_log),
                    "EVENT_NAME": event,
                    # `github.event.before` is empty outside a push; the PR
                    # fields are empty outside a pull request.
                    "PUSH_BASE": (push_base if push_base is not None else base)
                    if event == "push"
                    else "",
                    "PUSH_HEAD": head,
                    "PR_BASE": base if event == "pull_request" else "",
                    "PR_HEAD": head if event == "pull_request" else "",
                }
            )
            if push_base == "FIRST":
                environment["PUSH_BASE"] = first
            result = subprocess.run(
                ["bash", "-c", self.script],
                cwd=root,
                env=environment,
                capture_output=True,
                text=True,
                timeout=120,
            )
            run = StepRun(root, result, output, summary, planner_log)
            if expect_failure:
                self.assertNotEqual(0, result.returncode, result.stdout + result.stderr)
                return run
            self.assertEqual(0, result.returncode, result.stdout + result.stderr)
            self.assertEqual([], schema.validate_resolved_plan(run.plan))
            self.assertEqual(head, run.plan["head"])
            return run

    @staticmethod
    def seed_repository(root: Path) -> tuple[str, str, str]:
        fixture_git(root, "init", "-q", "-b", "main")
        (root / "README.md").write_text("zero\n", encoding="utf-8")
        fixture_git(root, "add", "README.md")
        fixture_git(root, "commit", "-q", "-m", "first")
        first = fixture_git(root, "rev-parse", "HEAD")
        (root / "README.md").write_text("one\n", encoding="utf-8")
        fixture_git(root, "commit", "-q", "-am", "base")
        base = fixture_git(root, "rev-parse", "HEAD")
        (root / "README.md").write_text("two\n", encoding="utf-8")
        source = root / SOURCE_FILE
        source.parent.mkdir(parents=True)
        source.write_text("pub fn fixture() {}\n", encoding="utf-8")
        fixture_git(root, "add", "-A")
        fixture_git(root, "commit", "-q", "-m", "head")
        return first, base, fixture_git(root, "rev-parse", "HEAD")

    # -- receipts ------------------------------------------------------------

    @staticmethod
    def tool(root: Path, name: str, *args: str) -> str:
        result = subprocess.run(
            ["python3", str(root / "scripts" / "ci" / name), *args],
            cwd=root, capture_output=True, text=True, check=False,
            env={**os.environ, "GIT_TERMINAL_PROMPT": "0"},
        )
        if result.returncode != 0:
            raise AssertionError(f"{name} {' '.join(args)} failed: {result.stderr}")
        return result.stdout

    def planned_documents(self, root: Path, base: str, head: str) -> tuple[dict, dict]:
        """The real planner's plan and projection for the fixture's `base..head`."""
        projection = json.loads(self.tool(
            root, "affected_scope.py", "--plan-out", "receipt-plan.json",
            "--base", base, "--head", head, "--", "README.md", SOURCE_FILE,
        ))
        plan = json.loads((root / "receipt-plan.json").read_text(encoding="utf-8"))
        (root / "planner-calls.log").unlink(missing_ok=True)
        return plan, projection

    def local_scope_receipt(self, root: Path, base: str, head: str, mutate=None) -> str:
        """A scope receipt for `base..head` whose free text names its origin.

        The marker is what proves the handoff: the planner never writes it, so
        its presence in the step's outputs shows the receipt was used.
        """
        plan, projection = self.planned_documents(root, base, head)
        plan["preflight_reason"] = SCOPE_MARKER
        projection["preflight_reason"] = SCOPE_MARKER
        if mutate is not None:
            mutate(plan, projection)
        (root / "receipt-plan.json").write_text(schema.canonical(plan), encoding="utf-8")
        (root / "receipt-scope.json").write_text(schema.canonical(projection), encoding="utf-8")
        return self.tool(
            root, "local_evidence.py", "scope-record", "--plan", "receipt-plan.json",
            "--scope", "receipt-scope.json", "--base", base, "--head", head,
        ).strip()

    def publish_validation_receipt(self, root: Path, base: str, head: str) -> None:
        plan, _ = self.planned_documents(root, base, head)
        (root / "validation-plan.json").write_text(schema.canonical(plan), encoding="utf-8")
        stage = root / "stage" / "L1"
        stage.mkdir(parents=True)
        (stage / "biscuit-hash.xml").write_text(
            '<?xml version="1.0"?><testsuites><testsuite name="biscuit-hash" tests="1" '
            'failures="0" errors="0" skipped="0"><testcase classname="biscuit-hash" '
            'name="one"/></testsuite></testsuites>',
            encoding="utf-8",
        )
        (root / "stage" / "manifest.jsonl").write_text(
            json.dumps({"tier": "L1", "package": "biscuit-hash", "xml": "L1/biscuit-hash.xml",
                        "exit_code": 0, "environment": "macos-latest", "duration_s": 1,
                        "report_present": True}) + "\n",
            encoding="utf-8",
        )
        receipt = self.tool(
            root, "local_evidence.py", "record-cells", "--plan", "validation-plan.json",
            "--stage", "stage", "--base", base, "--head", head,
            "--environment", "macos-latest", "--report-dir", str(root / "stage"),
        ).strip()
        fixture_git(root, "notes", "--ref", "refs/notes/ci-local/macos-latest", "add", "-f", "-m", receipt, head)

    # -- event shapes --------------------------------------------------------

    def test_a_manual_run_reaches_the_planner_with_full_scope(self) -> None:
        run = self.run_step("workflow_dispatch")
        self.assertEqual("true", run.outputs["full_scope"])
        self.assertEqual(NULL_OID, run.plan["base"])

    def test_a_pull_request_diffs_its_base_and_head(self) -> None:
        run = self.run_step("pull_request")
        self.assertEqual("false", run.outputs["full_scope"])
        self.assertNotEqual(NULL_OID, run.plan["base"])

    def test_a_push_with_a_real_base_diffs_it(self) -> None:
        run = self.run_step("push")
        self.assertEqual("false", run.outputs["full_scope"])
        self.assertNotEqual(NULL_OID, run.plan["base"])

    def test_a_branch_creating_push_runs_the_full_scope(self) -> None:
        run = self.run_step("push", push_base=NULL_OID)
        self.assertEqual("true", run.outputs["full_scope"])
        self.assertEqual(NULL_OID, run.plan["base"])
        self.assertEqual("CI (no comparison base)", run.scope_source())

    # -- local scope evidence (2026-09-10 spec R3) ---------------------------

    def test_a_matching_scope_receipt_is_the_plan_and_the_planner_never_runs(self) -> None:
        run = self.run_step("push", scope_receipt=self.local_scope_receipt)
        self.assertEqual([], run.planner_calls, "the planner ran although the receipt matched")
        self.assertEqual(f"local ({SCOPE_REF} @ {run.plan['head'][:9]})", run.scope_source())
        self.assertEqual(SCOPE_MARKER, run.outputs["preflight_reason"])
        self.assertEqual(SCOPE_MARKER, run.plan["preflight_reason"])
        self.assertEqual(["biscuit-hash"], [entry["package"] for entry in run.plan["packages"]])
        self.assertEqual("true", run.outputs["has_packages"])

    def test_the_written_plan_is_byte_identical_to_the_receipts(self) -> None:
        receipts: dict = {}

        def remember(root: Path, base: str, head: str) -> str:
            receipts["text"] = self.local_scope_receipt(root, base, head)
            return receipts["text"]

        run = self.run_step("pull_request", scope_receipt=remember)
        carried = json.loads(receipts["text"])["plan"]
        self.assertEqual(schema.canonical(carried), run.plan_bytes)

    def test_a_receipt_for_another_base_falls_back_with_its_code(self) -> None:
        run = self.run_step("push", push_base="FIRST", scope_receipt=self.local_scope_receipt)
        self.assertTrue(run.scope_source().startswith("CI fallback (scope-base-mismatch:"), run.scope_source())
        self.assertEqual(1, len(run.planner_calls), run.planner_calls)
        self.assertNotEqual(SCOPE_MARKER, run.plan["preflight_reason"])

    def test_a_missing_receipt_falls_back_with_its_code(self) -> None:
        run = self.run_step("pull_request")
        self.assertTrue(run.scope_source().startswith("CI fallback (scope-missing:"), run.scope_source())
        self.assertEqual(1, len(run.planner_calls), run.planner_calls)

    def test_a_receipt_declaring_another_head_or_tree_falls_back(self) -> None:
        def other_head(root: Path, base: str, head: str) -> str:
            document = json.loads(self.local_scope_receipt(root, base, head))
            document["head"] = base
            return schema.canonical(document)

        def other_tree(root: Path, base: str, head: str) -> str:
            document = json.loads(self.local_scope_receipt(root, base, head))
            document["tree"] = fixture_git(root, "rev-parse", f"{base}^{{tree}}")
            return schema.canonical(document)

        for label, receipt, code in (
            ("head", other_head, "scope-head-mismatch"),
            ("tree", other_tree, "scope-tree-mismatch"),
        ):
            with self.subTest(label):
                run = self.run_step("pull_request", scope_receipt=receipt)
                self.assertTrue(run.scope_source().startswith(f"CI fallback ({code}:"), run.scope_source())
                self.assertNotEqual(SCOPE_MARKER, run.plan["preflight_reason"])
                self.assertEqual(1, len(run.planner_calls), run.planner_calls)

    def test_a_receipt_of_another_schema_version_falls_back(self) -> None:
        def future(root: Path, base: str, head: str) -> str:
            document = json.loads(self.local_scope_receipt(root, base, head))
            document["schema_version"] = 99
            return schema.canonical(document)

        run = self.run_step("pull_request", scope_receipt=future)
        self.assertTrue(run.scope_source().startswith("CI fallback (scope-schema:"), run.scope_source())

    def test_a_manual_run_never_consults_the_receipt(self) -> None:
        run = self.run_step("workflow_dispatch", scope_receipt=self.local_scope_receipt)
        self.assertEqual("CI (workflow_dispatch ignores local scope)", run.scope_source())
        self.assertEqual("true", run.outputs["full_scope"])
        self.assertNotEqual(SCOPE_MARKER, run.plan["preflight_reason"])
        self.assertEqual(1, len(run.planner_calls), run.planner_calls)

    def test_accepted_cells_on_a_scope_hit_are_resolved_by_a_compared_planner_run(self) -> None:
        # Option B of the evidence path: the accepted set is resolved by one
        # full planner run, and its selection is compared with the receipt's
        # rather than handed off.
        run = self.run_step("pull_request", scope_receipt=self.local_scope_receipt, validation_receipt=True)
        self.assertEqual(1, len(run.planner_calls), run.planner_calls)
        self.assertIn("--accepted-cells", run.planner_calls[0])
        self.assertTrue(run.scope_source().startswith("local ("), run.scope_source())
        reused = [
            cell for cell in run.plan["cells"]
            if cell["package"] == "biscuit-hash" and cell["environment"] == "macos-latest" and cell["gate"] == "L1"
        ]
        self.assertEqual(["reuse"], [cell["execution"] for cell in reused])

    def test_a_receipt_that_disagrees_with_the_planner_fails_the_step_loudly(self) -> None:
        def with_a_ghost(plan: dict, projection: dict) -> None:
            plan["areas"].append({"area": "ghost", "selection_reason": "fixture", "packages": ["ghost"]})
            plan["packages"].append({
                "package": "ghost", "area": "ghost", "selection_reason": "fixture",
                "gates": [], "targets": [], "tiers": [], "test_args": "", "check_args": "",
                "l2_backends": [], "runner_tools": [], "companion_suites": [], "native": {},
            })
            projection["packages"].append("ghost")

        def drifted(root: Path, base: str, head: str) -> str:
            return self.local_scope_receipt(root, base, head, mutate=with_a_ghost)

        run = self.run_step("pull_request", scope_receipt=drifted, validation_receipt=True, expect_failure=True)
        self.assertIn("select different areas or packages", run.result.stdout + run.result.stderr)


if __name__ == "__main__":
    unittest.main()

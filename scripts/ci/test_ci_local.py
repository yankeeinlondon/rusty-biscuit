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
from collections.abc import Callable, Mapping, Sequence
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import affected_scope  # noqa: E402
import plan_fixtures  # noqa: E402
import schema  # noqa: E402
import tool_guard  # noqa: E402
from tool_guard import require_tools, requires_tools  # noqa: E402
from affected_scope import change_inventory, legacy_scope_document  # noqa: E402
from workflow_reading import (  # noqa: E402
    WorkflowLayoutError,
    job_names,
    job_run_steps,
    step_run_lines,
    step_script,
)


ROOT = Path(__file__).resolve().parents[2]
RECIPE = ROOT / "just" / "ci-local.just"
DEVOPS = ROOT / "just" / "devops.just"
CONSTRAINTS = ROOT / "scripts" / "ci" / "constraints.py"
PRE_PUSH = ROOT / ".githooks" / "pre-push"
JUST = shutil.which("just")

#: The only job that runs this suite. `preflight` does not, so the tools below
#: are declared required in exactly one place and a developer host without them
#: still skips. It installs `just`; `jq` and a Bash >= 4.4 are in the
#: ubuntu-latest runner image.
CI_TOOLING = (
    "`ci.yml`'s `ci-tooling` job, the only one that runs this suite, which "
    "installs just and sets BISCUIT_REQUIRE_JUST, BISCUIT_REQUIRE_JQ, and "
    "BISCUIT_REQUIRE_BASH"
)


def relocate_home(environment: dict[str, str], home: Path) -> None:
    """Point the recipe's default constraint store at a home the test owns.

    Both variables, because `constraints.py` reads `Path.home()`: HOME on
    Unix, USERPROFILE on native Windows. Without this the developer's real
    `~/.rusty-biscuit/ci-constraints/` would decide these tests.
    """
    home.mkdir(parents=True, exist_ok=True)
    environment["HOME"] = str(home)
    environment["USERPROFILE"] = str(home)
    environment.pop("BISCUIT_CI_CONSTRAINTS_DIR", None)


def thread_policy_recipe() -> str:
    lines = DEVOPS.read_text(encoding="utf-8").splitlines(keepends=True)
    start = next(index for index, line in enumerate(lines) if line.startswith("_test_threads:"))
    end = start + 1
    while end < len(lines) and (not lines[end].strip() or lines[end][0].isspace()):
        end += 1
    return "".join(lines[start:end])


def clean_policy_environment() -> dict[str, str]:
    environment = os.environ.copy()
    for key in (
        "CI",
        "GITHUB_ACTIONS",
        "BISCUIT_CI_ENVIRONMENT",
        "BISCUIT_CI_REPORTS_OUT",
        "BISCUIT_CI_TARGET_DIR",
        "BISCUIT_JUNIT_TARGET_DIR",
        "BISCUIT_JUNIT_WORKSPACE_ROOT",
        "NEXTEST_PROFILE",
        "NEXTEST_TEST_THREADS",
    ):
        environment.pop(key, None)
    return environment


class PrePushEvidenceContractTests(unittest.TestCase):
    def test_a_same_head_retry_preserves_reports_and_merges_the_prior_receipt(self) -> None:
        hook = PRE_PUSH.read_text(encoding="utf-8")
        self.assertNotIn('rm -rf "$REPORT_DIR"', hook)
        self.assertIn('--prior-receipt "$PRIOR_RECEIPT_FILE"', hook)


@requires_tools("just", "jq", enforced_by=CI_TOOLING)
class CiLocalTests(unittest.TestCase):
    def run_recipe(
        self,
        threads: str | None = None,
        cores: int = 16,
        reports: dict | None = None,
        extra_matrix: Sequence[dict] = (),
        plan: dict | None = None,
        capture: dict | None = None,
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
                        "archive_includes": [],
                        "sidecars": [],
                        "l2_environments": ["macos-latest"],
                        "l2_backends": ["tmux"],
                    }
                    for name in ("parallel", "serial")
                ],
            }
            scope["packages"].extend(entry["package"] for entry in extra_matrix)
            scope["matrix"].extend(extra_matrix)
            (root / "scope.json").write_text(json.dumps(scope), encoding="utf-8")
            if plan is not None:
                (root / "plan.json").write_text(json.dumps(plan), encoding="utf-8")
            # The recipe hands the planner `--plan-out`; the canonical plan it
            # writes there is what the archive-path guard's scan scope comes
            # from, so the stub has to produce it rather than only stdout.
            (scripts / "affected_scope.py").write_text(
                "import sys\n"
                "from pathlib import Path\n"
                "argv = sys.argv[1:]\n"
                'if "--plan-out" in argv and Path("plan.json").is_file():\n'
                '    Path(argv[argv.index("--plan-out") + 1]).write_text(\n'
                '        Path("plan.json").read_text(), encoding="utf-8"\n'
                "    )\n"
                'print(Path("scope.json").read_text())\n',
                encoding="utf-8",
            )
            # Records the invocation and the plan the recipe pointed it at.
            # The RECIPE each suite runs is `companion_suites.py`'s to resolve
            # from `SUITE_REGISTRY`, which
            # `test_the_guard_runs_through_the_registrys_canonical_recipe`
            # pins against the real module.
            (scripts / "companion_suites.py").write_text(
                "#!/usr/bin/env python3\n"
                "import json, os, sys\n"
                "argv = sys.argv[1:]\n"
                "from pathlib import Path\n"
                'named = os.environ.get("BISCUIT_ARCHIVE_GUARD_PLAN")\n'
                "# Read here, not in the test: the recipe deletes its scratch\n"
                "# plan on exit, and the contract is what the guard WOULD have\n"
                "# scanned at the moment it was invoked.\n"
                'plan = Path(named).read_text(encoding="utf-8") if named and Path(named).is_file() else None\n'
                'with open(os.environ["TEST_COMPANION_LOG"], "a", encoding="utf-8") as log:\n'
                '    log.write(json.dumps({"args": argv, "plan": named, "plan_text": plan}) + "\\n")\n'
                'out = argv[argv.index("--out") + 1]\n'
                'open(out, "w", encoding="utf-8").write("{}\\n")\n',
                encoding="utf-8",
            )
            shutil.copyfile(CONSTRAINTS, scripts / "constraints.py")
            # The recipe's self-test loop; each is a no-op stub here because
            # this fixture tests the recipe's scheduling, not those suites.
            for suite in (
                "test_schema.py",
                "test_affected_scope.py",
                "test_resolved_plan.py",
                "test_ci_local.py",
                "test_constraints.py",
                "test_publish_gaps.py",
                "test_runner_loss.py",
                "test_build_key.py",
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
                    "'backends': os.environ.get('BISCUIT_TEST_REQUIRED_BACKENDS'), "
                    "'target_dir': os.environ.get('CARGO_TARGET_DIR'), "
                    "'rustc_wrapper': os.environ.get('RUSTC_WRAPPER'), "
                    "'junit_target_dir': os.environ.get('BISCUIT_JUNIT_TARGET_DIR'), "
                    "'junit_workspace_root': os.environ.get('BISCUIT_JUNIT_WORKSPACE_ROOT'), "
                    "'nextest_profile': os.environ.get('NEXTEST_PROFILE')}) + '\\n')\n"
                ),
                "tmux": "raise SystemExit('tmux must only be detected, never started')\n",
                "cargo": "raise SystemExit('Rust builds are forbidden in this test')\n",
            }
            for name, body in stubs.items():
                path = bin_dir / name
                path.write_text("#!/usr/bin/env python3\n" + body, encoding="utf-8")
                path.chmod(0o755)
            environment = clean_policy_environment()
            relocate_home(environment, root / "home")
            environment.update({
                "PATH": str(bin_dir) + os.pathsep + environment.get("PATH", ""),
                "CARGO_TARGET_DIR": str(root / "polluted-target"),
                "RUSTC_WRAPPER": "kache",
                "TEST_CALL_LOG": str(root / "calls.jsonl"),
                "TEST_CORES": str(cores),
                "TEST_REAL_JUST": JUST,
                "TEST_POLICY_RECIPE": str(root / "policy.just"),
                "TEST_COMPANION_LOG": str(root / "companions.jsonl"),
            })
            # The hook exports these for the run it wraps, and this suite runs
            # inside that run's self-test loop; a fed plan would refuse `--all`.
            for key in (
                "BISCUIT_L2_THREADS", "BISCUIT_TEST_REQUIRED_BACKENDS",
                "BISCUIT_CI_SCOPE_OUT", "BISCUIT_CI_PLAN_OUT", "BISCUIT_CI_PLAN_IN",
                "WEZTERM_UNIX_SOCKET", "KITTY_LISTEN_ON",
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
            if capture is not None:
                companion_log = root / "companions.jsonl"
                capture["stdout"] = result.stdout
                capture["companions"] = (
                    [json.loads(line) for line in companion_log.read_text().splitlines()]
                    if companion_log.is_file()
                    else []
                )
            if reports is not None:
                staged = root / "reports" / "gate-backends.jsonl"
                reports["gate_backends"] = (
                    [json.loads(line) for line in staged.read_text().splitlines()]
                    if staged.is_file()
                    else []
                )
            return [json.loads(line) for line in (root / "calls.jsonl").read_text().splitlines()]

    def test_gates_use_local_evidence_profile_and_an_unwrapped_reserved_target(self) -> None:
        calls = self.run_recipe()
        self.assertTrue(calls)
        for call in calls:
            self.assertTrue(call["target_dir"].endswith("/target/ci-local"), call)
            self.assertEqual("", call["rustc_wrapper"])
            self.assertEqual("local-evidence", call["nextest_profile"])
            self.assertTrue(call["junit_target_dir"].endswith("/target"), call)
            self.assertEqual(
                call["junit_target_dir"].removesuffix("/target"),
                call["junit_workspace_root"],
            )

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


#: The guard-only lint cell exactly as `affected_scope.archive_guard_only_selection`
#: projects it: one Linux lint cell, no tier, and its companions as the whole
#: required work.
GUARD_MATRIX_ENTRY = {
    "package": "test-toolkit",
    "gates": ["lint"],
    "tiers": [],
    "test_args": "",
    "runner_tools": [],
    "archive_includes": [],
    "sidecars": [],
    "l2_environments": [],
    "l2_backends": [],
    "companion_suites": ["archive-path-guard"],
    "lint_companions_only": True,
}

GUARD_PLAN = {
    "schema_version": schema.RESOLVED_PLAN_SCHEMA_VERSION,
    "cells": [
        {
            "package": "test-toolkit",
            "area": "tools",
            "environment": "ubuntu-latest",
            "gate": "lint",
            "execution": "execute",
            "origin": "ci",
            "state": "pending",
            "reusable": False,
            "companions_only": True,
            "companions": [{"name": "archive-path-guard"}],
        }
    ],
    "archive_guard": {
        "selected": True,
        "mode": "changed",
        "paths": ["darkmatter/dmls/src/lib.rs"],
        "reason": "this run carries a change inventory",
    },
}


@requires_tools("just", "jq", enforced_by=CI_TOOLING)
class ArchiveGuardLocalExecutionTests(unittest.TestCase):
    """`2026-09-19-less-brittle`: local validation of a planned guard cell.

    The specification allows a local run to execute the planned guard through
    its canonical recipe OR to report that the Linux execution is outstanding.
    The recipe does both, because a macOS run cannot manufacture Linux evidence
    and a silent local pass would read as if it had.
    """

    def guard_run(self, capture: dict) -> list[dict]:
        """One `ci-local --all --l2` over the ordinary fixture plus the guard cell."""
        return CiLocalTests.run_recipe(
            self, extra_matrix=[GUARD_MATRIX_ENTRY], plan=GUARD_PLAN, capture=capture
        )

    def test_a_planned_guard_cell_runs_against_the_plan_this_run_resolved(self) -> None:
        capture: dict = {}
        self.guard_run(capture)

        self.assertEqual(
            1,
            len(capture["companions"]),
            f"the guard must run exactly once: {capture['companions']}",
        )
        invocation = capture["companions"][0]
        arguments = dict(zip(invocation["args"][0::2], invocation["args"][1::2]))
        self.assertEqual(["archive-path-guard"], json.loads(arguments["--suites"]))
        self.assertEqual("ubuntu-latest", arguments["--environment"])
        self.assertEqual("lint", arguments["--gate"])

        # The scan scope is the plan this run just resolved, not a diff the
        # guard re-derived: the file the variable names IS the plan document.
        self.assertTrue(invocation["plan"], "BISCUIT_ARCHIVE_GUARD_PLAN must be set")
        self.assertIsNotNone(
            invocation["plan_text"],
            f"the variable named {invocation['plan']}, which did not exist",
        )
        self.assertEqual(
            GUARD_PLAN["archive_guard"],
            json.loads(invocation["plan_text"])["archive_guard"],
        )

    def test_the_guard_runs_through_the_registrys_canonical_recipe(self) -> None:
        # The recipe string itself is `companion_suites.py`'s to resolve, from
        # the one registry CI reads. This is the other half of the proof above:
        # the arguments the recipe passes select exactly this record, and the
        # record names the canonical recipe.
        records = affected_scope.companion_records(
            ["archive-path-guard"], "ubuntu-latest", "lint"
        )
        self.assertEqual(
            ["cd tools/test-toolkit && just archive-path-guard"],
            [record["recipe"] for record in records],
        )

    def test_a_companions_only_cell_does_not_lint_its_package(self) -> None:
        capture: dict = {}
        calls = self.guard_run(capture)
        self.assertEqual(
            [],
            [call for call in calls if call["args"][:2] == ["_lint", "test-toolkit"]],
            "the planner selected the guard, not the package; clippy is CI's to "
            "skip and must be skipped here too",
        )

    def test_a_non_linux_host_reports_the_linux_execution_as_outstanding(self) -> None:
        # The stub host is macOS. A local pass on it proves the source policy
        # holds in this tree; it is not the ubuntu-latest execution the plan
        # scheduled, and the run must say so rather than let a green summary
        # line imply otherwise.
        capture: dict = {}
        self.guard_run(capture)
        self.assertIn("not ubuntu-latest", capture["stdout"])
        self.assertIn(
            "test-toolkit/ubuntu-latest/lint remains OUTSTANDING", capture["stdout"]
        )

    def test_a_guard_only_lint_cell_is_not_reusable(self) -> None:
        # The recipe needs no machinery to withhold the local result: the
        # planner already marks every lint cell non-reusable, which is the
        # specification's "initially prefer non-reusable guard execution".
        plan = json.loads(
            subprocess.run(
                [
                    sys.executable,
                    str(ROOT / "scripts" / "ci" / "affected_scope.py"),
                    "--resolved-plan",
                    "darkmatter/dmls/src/lib.rs",
                ],
                cwd=ROOT,
                capture_output=True,
                text=True,
                check=True,
            ).stdout
        )
        guard_cells = [
            cell
            for cell in plan["cells"]
            if cell["package"] == "test-toolkit" and cell["gate"] == "lint"
        ]
        self.assertEqual(1, len(guard_cells), guard_cells)
        self.assertTrue(guard_cells[0]["companions_only"])
        self.assertFalse(guard_cells[0]["reusable"])
        self.assertEqual(
            [affected_scope.ARCHIVE_GUARD_SUITE],
            [companion["name"] for companion in guard_cells[0]["companions"]],
        )


@requires_tools("just", "jq", enforced_by=CI_TOOLING)
class CiLocalDiffScopeTests(unittest.TestCase):
    """`just ci-local`'s selection boundary, run over a real repository.

    The third caller of the planner, and the only one whose changed set also
    unions untracked files. It has to declare deletions exactly as CI's scope
    step does, and an untracked file — which no diff reports and which can
    never be a deletion — must not be dragged into that declaration.
    """

    @classmethod
    def setUpClass(cls) -> None:
        require_tools("just", "jq", "git", enforced_by=CI_TOOLING)

    #: Enough of a legacy projection for the recipe to reach its `--dry-run`
    #: exit: one package, so the zero-package branch (which reads the plan
    #: through `jq`) is not taken.
    SCOPE_DOCUMENT = {
        "packages": ["alpha"],
        "full_scope": False,
        "change_class": "package",
        "full_scope_gates": [],
        "matrix": [{"package": "alpha", "gates": ["lint"], "tiers": ["L1"],
                    "test_args": "", "runner_tools": [], "archive_includes": [],
                    "sidecars": [], "l2_environments": [], "l2_backends": []}],
    }

    def planner_arguments(self) -> list[str]:
        """Every argument `ci-local` handed the planner, for a fixture whose
        `base..head` deletes one Rust file and renames another, and whose
        working tree carries one untracked file."""
        with tempfile.TemporaryDirectory(prefix="ci-local-scope-") as temporary:
            root = Path(temporary).resolve()
            scripts = root / "scripts" / "ci"
            scripts.mkdir(parents=True)
            shutil.copyfile(RECIPE, root / "ci-local.just")
            (root / "policy.just").write_text(thread_policy_recipe(), encoding="utf-8")
            (root / "justfile").write_text(
                'red := ""\ngreen := ""\nreset := ""\nimport "ci-local.just"\n',
                encoding="utf-8",
            )
            shutil.copyfile(CONSTRAINTS, scripts / "constraints.py")
            # The parser is the boundary under test, so the shipped one runs.
            shutil.copyfile(ROOT / "scripts" / "ci" / "diff_scope.py",
                            scripts / "diff_scope.py")
            (scripts / "affected_scope.py").write_text(
                "import json, os, sys\n"
                "with open(os.environ['TEST_PLANNER_LOG'], 'w', encoding='utf-8') as log:\n"
                "    log.write(json.dumps(sys.argv[1:]))\n"
                f"print(json.dumps({self.SCOPE_DOCUMENT!r}))\n",
                encoding="utf-8",
            )

            for name, body in (
                ("kept.rs", "pub fn kept() {}\n"),
                ("gone.rs", "pub fn gone() {}\n"),
                ("before.rs", "pub fn moved() {}\n"),
            ):
                (root / name).write_text(body, encoding="utf-8")
            fixture_git(root, "init", "-q", "-b", "main")
            fixture_git(root, "add", "-A")
            fixture_git(root, "commit", "-q", "-m", "base")
            base = fixture_git(root, "rev-parse", "HEAD")
            (root / "kept.rs").write_text("pub fn kept() { let _ = 1; }\n", encoding="utf-8")
            fixture_git(root, "rm", "-q", "gone.rs")
            fixture_git(root, "mv", "before.rs", "after.rs")
            fixture_git(root, "add", "-A")
            fixture_git(root, "commit", "-q", "-m", "head")
            (root / "untracked.rs").write_text("pub fn fresh() {}\n", encoding="utf-8")

            environment = clean_policy_environment()
            relocate_home(environment, root / "home")
            environment.update({
                "TEST_PLANNER_LOG": str(root / "planner.json"),
                "CI_LOCAL_BASE": base,
            })
            for key in ("BISCUIT_CI_SCOPE_OUT", "BISCUIT_CI_PLAN_OUT",
                        "BISCUIT_CI_PLAN_IN", "GIT_DIR", "GIT_WORK_TREE",
                        "GIT_INDEX_FILE"):
                environment.pop(key, None)
            result = subprocess.run(
                [JUST, "--justfile", str(root / "justfile"), "ci-local", "--dry-run"],
                cwd=root, env=environment, capture_output=True, text=True, timeout=60,
            )
            self.assertEqual(0, result.returncode, result.stdout + result.stderr)
            return json.loads((root / "planner.json").read_text(encoding="utf-8"))

    def test_the_recipe_declares_its_deletion_and_lists_the_rename_destination(self) -> None:
        arguments = self.planner_arguments()
        separator = arguments.index("--")
        options, changed = arguments[:separator], arguments[separator + 1:]
        deleted = [
            options[index + 1]
            for index, option in enumerate(options)
            if option == "--deleted"
        ]
        self.assertEqual(["gone.rs"], deleted)
        # The deletion changed; its destination-less rename partner did not
        # change under its old name, and the untracked file is neither.
        self.assertEqual({"after.rs", "gone.rs", "kept.rs", "untracked.rs"}, set(changed))
        self.assertNotIn("before.rs", changed)
        self.assertNotIn("before.rs", deleted)
        self.assertNotIn("untracked.rs", deleted)


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


@requires_tools("just", "jq", enforced_by=CI_TOOLING)
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


@requires_tools("just", "jq", enforced_by=CI_TOOLING)
class PlanSurfaceTests(unittest.TestCase):
    """AC17: the reviewable plan shown before a push or other trigger."""

    @staticmethod
    def environment_records() -> list[dict]:
        return [
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
        ]

    def documentation_plan(self) -> dict:
        """A documentation-only change's plan: an inventory and no cell at all.

        Spec section 7's local half. The recipe must name the documents and
        state that no package test is required, as an affirmative scheduling
        decision rather than a warning.
        """
        return {
            "schema_version": schema.RESOLVED_PLAN_SCHEMA_VERSION,
            "base": "a" * 40,
            "head": "b" * 40,
            "change_class": "documentation",
            "change_inventory": change_inventory(
                ["docs/topics/ci-cd.md", "alpha/README.md"], False
            ),
            "archive_guard": plan_fixtures.archive_guard(),
            "full_scope": False,
            "full_scope_gates": [],
            "areas": [],
            "packages": [],
            "source_packages": [],
            "reverse_dependencies": [],
            "environments": self.environment_records(),
            "cells": [],
            "accepted_evidence": [],
            "policy_gaps": [],
            # No cell means no consumer, and an unconsumed build record is
            # removed rather than left in the plan.
            "builds": [],
            "prohibited_cells": [],
            "job_estimate": 0,
            "preflight_os": [],
            "preflight_reason": "no gating package; preflight establishes nothing",
            "flags": {},
        }

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
                "reusable": True,
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
                "reusable": True,
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
                "reusable": True,
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
                "reusable": True,
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
        return plan_fixtures.attach_builds({
            "schema_version": schema.RESOLVED_PLAN_SCHEMA_VERSION,
            "base": "a" * 40,
            "head": "b" * 40,
            "change_class": "package",
            # The real producer, so a fixture plan cannot describe a shape the
            # planner no longer emits.
            "change_inventory": change_inventory(["alpha/src/lib.rs"], False),
            "archive_guard": plan_fixtures.archive_guard(["alpha/src/lib.rs"]),
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
                    "archive_includes": [],
                    "sidecars": [],
                    "companion_suites": [],
                    "l1_include_slow": False,
                    "native": {},
                }
            ],
            "source_packages": ["alpha"],
            "reverse_dependencies": [],
            "environments": self.environment_records(),
            "cells": cells,
            "accepted_evidence": [],
            "policy_gaps": [],
            "builds": [],
            "prohibited_cells": [] if prohibited_is_covered else ["alpha/wsl2-ubuntu/L1"],
            "job_estimate": len(cells),
            "preflight_os": ["ubuntu-latest"],
            "preflight_reason": "package-local change",
            "flags": {},
        })

    def run_plan_with_output(self) -> tuple[str, dict]:
        """The rendered plan and the canonical JSON `--plan-out` wrote."""
        captured: dict = {}
        result = self.run_plan(capture=captured)
        self.assertEqual(0, result.returncode, result.stdout + result.stderr)
        return result.stdout, captured["plan"]

    #: The stand-in planner. It prints the fixture plan, except that every cell
    #: in an environment the store handed through `--constraints` forbids is
    #: resolved to `prohibited` carrying that record — the one planner behavior
    #: the plan surface depends on, so the recipe's store resolution is proven
    #: by the reason a refusal names.
    STUB_PLANNER = (
        "import json, sys\n"
        "from pathlib import Path\n"
        "sys.path.insert(0, str(Path(__file__).resolve().parent))\n"
        "import constraints\n"
        "plan = json.loads(Path('plan.json').read_text())\n"
        "args = sys.argv[1:]\n"
        "store = args[args.index('--constraints') + 1] if '--constraints' in args else ''\n"
        "active, _expired, malformed = constraints.load(store)\n"
        "for entry in active + malformed:\n"
        "    for cell in plan['cells']:\n"
        "        if cell['environment'] != entry.environment:\n"
        "            continue\n"
        "        cell.pop('evidence', None)\n"
        "        record = {key: str(entry.document.get(key, '')) for key in ('owner', 'reason', 'expiry')}\n"
        "        cell.update(execution='omit', origin='none', state='prohibited', prohibition=record,\n"
        "                    selection_reason='a persisted constraint forbids this environment')\n"
        "        key = f\"{cell['package']}/{cell['environment']}/{cell['gate']}\"\n"
        "        if key not in plan['prohibited_cells']:\n"
        "            plan['prohibited_cells'].append(key)\n"
        "print(json.dumps(plan))\n"
    )

    def run_plan(
        self,
        prohibited_is_covered: bool = True,
        capture: dict | None = None,
        origin: str = "",
        records: Mapping[str, dict] | None = None,
        document: dict | None = None,
    ) -> subprocess.CompletedProcess:
        """Run `just ci-local --all --plan` in a temp root with a relocated home.

        `records` maps a store-relative path to a record written under the
        relocated home's default store; `origin` becomes the root's `origin`
        remote so the recipe derives the store directory from it. `document`
        replaces the default package-change plan.
        """
        document = document or self.resolved_plan(prohibited_is_covered)
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
            (scripts / "affected_scope.py").write_text(self.STUB_PLANNER, encoding="utf-8")
            shutil.copyfile(CONSTRAINTS, scripts / "constraints.py")
            home = root / "home"
            for relative, record in (records or {}).items():
                path = home / ".rusty-biscuit" / "ci-constraints" / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(json.dumps(record), encoding="utf-8")
            if origin:
                for command in (["git", "init", "-q"], ["git", "remote", "add", "origin", origin]):
                    subprocess.run(command, cwd=root, check=True, capture_output=True, timeout=30)
            for name in ("cargo", "sniff"):
                path = bin_dir / name
                path.write_text(
                    "#!/usr/bin/env python3\n"
                    f"raise SystemExit('--plan must run nothing, but called {name}')\n",
                    encoding="utf-8",
                )
                path.chmod(0o755)
            environment = clean_policy_environment()
            relocate_home(environment, home)
            environment["PATH"] = str(bin_dir) + os.pathsep + environment.get("PATH", "")
            environment.pop("BISCUIT_CI_PLAN_OUT", None)
            environment.pop("BISCUIT_CI_PLAN_IN", None)
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

    def test_plan_finds_a_record_in_the_default_store_without_the_variable(self) -> None:
        # Review-5: a record written in one session must bind a fresh one that
        # never set BISCUIT_CI_CONSTRAINTS_DIR. The store directory is derived
        # from `origin`, so a record under another repository's directory is
        # not this repository's constraint.
        def record(reason: str, repository: str) -> dict:
            return {
                "environment": "wsl2-ubuntu",
                "reason": reason,
                "owner": "ken",
                "expiry": "2099-01-01",
                "repository": repository,
            }

        records = {
            "github.com/acme/widgets/wsl.json": record(
                "recorded in an earlier session", "github.com/acme/widgets"
            ),
            "github.com/other/repo/wsl.json": record("another repository's", "github.com/other/repo"),
        }
        result = self.run_plan(origin="git@github.com:acme/widgets.git", records=records)
        combined = result.stdout + result.stderr
        self.assertNotEqual(0, result.returncode, combined)
        self.assertIn("recorded in an earlier session", combined)
        self.assertNotIn("another repository's", combined)
        self.assertIn("Constraint store:", result.stdout)
        self.assertIn(str(Path("github.com", "acme", "widgets")), result.stdout)

        result = self.run_plan(origin="git@github.com:acme/other.git", records=records)
        self.assertEqual(0, result.returncode, result.stdout + result.stderr)

    def test_plan_renders_the_change_inventory_it_was_given(self) -> None:
        # Spec section 7: the local surface reads the plan's own inventory
        # field, so it cannot describe a different change than `ci-reporting`.
        rendered, document = self.run_plan_with_output()
        self.assertIn("Change inventory: 1 changed path(s)", rendered)
        self.assertIn("source (1) — alpha/src/lib.rs", rendered)
        self.assertEqual(
            ["alpha/src/lib.rs"],
            document["change_inventory"]["paths"]["source"],
            "the rendered inventory must be the written plan's, not a second one",
        )

    def test_a_documentation_only_plan_names_its_documents_and_requires_no_test(
        self,
    ) -> None:
        result = self.run_plan(document=self.documentation_plan())
        self.assertEqual(0, result.returncode, result.stdout + result.stderr)
        self.assertIn("Change inventory: 2 changed path(s)", result.stdout)
        for named in ("alpha/README.md", "docs/topics/ci-cd.md"):
            self.assertIn(named, result.stdout, f"the plan never names {named}")
        self.assertIn(
            "No package test is required: the resolved plan schedules no package cell.",
            result.stdout,
        )

    def test_a_documentation_only_plan_is_an_affirmative_decision(self) -> None:
        # Never a warning, failure, accepted gap, or fabricated passing result.
        result = self.run_plan(document=self.documentation_plan())
        combined = result.stdout + result.stderr
        for forbidden in ("WARN", "warning", "accepted-gap", "prohibited", "✗", "⛔"):
            self.assertNotIn(forbidden, combined, f"{forbidden!r} in:\n{combined}")

    def test_a_full_scope_plan_reports_that_no_diff_was_consulted(self) -> None:
        document = self.documentation_plan()
        document["change_inventory"] = change_inventory([], True)
        result = self.run_plan(document=document)
        self.assertEqual(0, result.returncode, result.stdout + result.stderr)
        self.assertIn("Change inventory: no diff was consulted", result.stdout)

    def test_plan_writes_the_same_cells_it_rendered(self) -> None:
        # The rendered table is a projection; the canonical JSON is the machine
        # interface. AC17 needs them to describe one plan, not two.
        rendered, document = self.run_plan_with_output()
        self.assertEqual([], schema.validate_resolved_plan(document))
        for cell in document["cells"]:
            key = f"{cell['package']}/{cell['environment']}/{cell['gate']}"
            self.assertIn(key, rendered, f"the rendered plan omits {key}")


PLANNER = ROOT / "scripts" / "ci" / "affected_scope.py"


@requires_tools("just", "jq", enforced_by=CI_TOOLING)
class PlanFedRunTests(unittest.TestCase):
    """The hook's path (ruling D2, audit W14): gates run FROM a resolved plan.

    A fed plan carries the evidence its caller verified, so the recipe must
    run only the cells the plan leaves executing for this host — plus a cell
    reused from a FAILURE, which is rerun — and must never run the planner's
    selection again. The planner on PATH is a trampoline that logs every
    invocation, refuses selection outright, and answers `--apply-to` with the
    real module, so the count is of real invocations, not of a stub's opinion.
    """

    #: Refuses selection so a regression to replanning fails the recipe loudly;
    #: `--apply-to` (the projection, which reads nothing from the checkout) is
    #: answered by the real planner.
    TRAMPOLINE = (
        "import os, subprocess, sys\n"
        "with open(os.environ['TEST_PLANNER_LOG'], 'a', encoding='utf-8') as log:\n"
        "    log.write(' '.join(sys.argv[1:]) + '\\n')\n"
        "if '--apply-to' not in sys.argv:\n"
        "    raise SystemExit('the planner selected scope although a plan was fed in')\n"
        "raise SystemExit(subprocess.run([sys.executable, os.environ['TEST_REAL_PLANNER'], *sys.argv[1:]], check=False).returncode)\n"
    )

    def fed_plan(self, alpha_reused_outcome: str) -> dict:
        """alpha/L1 reused (with the given outcome) beside beta/L1 executing.

        Both packages own a lint cell (hosted on ubuntu-latest, never
        reusable), and alpha also owns an L1 cell on ubuntu-latest so a cell
        executing in ANOTHER environment is present to be left alone.
        """
        def package(name: str) -> dict:
            return {
                "package": name,
                "area": "pkg",
                "selection_reason": "source change",
                "gates": ["lint", "L1"],
                "targets": ["lib", "test"],
                "tiers": ["L1"],
                "test_args": f"--features {name}-tests",
                "check_args": f"-p {name}",
                "l2_backends": [],
                "runner_tools": [],
                "archive_includes": [],
                "sidecars": [],
                "companion_suites": [],
                "l1_include_slow": False,
                "native": {},
            }

        def cell(name: str, environment: str, gate: str, **overrides) -> dict:
            record = {
                "package": name,
                "area": "pkg",
                "environment": environment,
                "gate": gate,
                "execution": "execute",
                "origin": "ci",
                "state": "pending",
                "reusable": True,
                "target_kinds": ["lib", "test"],
                "compile_coverage_from": "L1" if gate == "L1" else "",
                "selection_reason": "no evidence for this environment",
            }
            record.update(overrides)
            return record

        cells = [
            cell("alpha", "ubuntu-latest", "lint"),
            cell(
                "alpha",
                "macos-latest",
                "L1",
                execution="reuse",
                origin="prior-local",
                state="reused",
                selection_reason="satisfied by a prior receipt",
                evidence={
                    "ref": "refs/notes/ci-local/macos-latest",
                    "commit": "c" * 40,
                    "outcome": alpha_reused_outcome,
                    "completion": "complete",
                },
            ),
            cell("alpha", "ubuntu-latest", "L1"),
            cell("beta", "ubuntu-latest", "lint"),
            cell("beta", "macos-latest", "L1"),
        ]
        return plan_fixtures.attach_builds({
            "schema_version": schema.RESOLVED_PLAN_SCHEMA_VERSION,
            "base": "a" * 40,
            "head": "b" * 40,
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
            "packages": [package("alpha"), package("beta")],
            "source_packages": ["alpha", "beta"],
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
            "accepted_evidence": [cells[1]["evidence"]],
            "builds": [],
            "policy_gaps": [],
            "prohibited_cells": [],
            "job_estimate": 4,
            "preflight_os": ["ubuntu-latest"],
            "preflight_reason": "package-local change",
            "flags": {},
        })

    def run_fed(self, alpha_reused_outcome: str = "pass", through_env: bool = False) -> dict:
        """Run `just ci-local --l2` from the fed plan; the gate and planner calls."""
        document = self.fed_plan(alpha_reused_outcome)
        self.assertEqual([], schema.validate_resolved_plan(document), "fixture plan must be valid")
        with tempfile.TemporaryDirectory(prefix="ci-local-fed-") as temporary:
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
            (scripts / "affected_scope.py").write_text(self.TRAMPOLINE, encoding="utf-8")
            shutil.copyfile(CONSTRAINTS, scripts / "constraints.py")
            for suite in (
                "test_schema.py",
                "test_affected_scope.py",
                "test_resolved_plan.py",
                "test_ci_local.py",
                "test_constraints.py",
                "test_publish_gaps.py",
                "test_runner_loss.py",
                "test_build_key.py",
            ):
                (scripts / suite).write_text("", encoding="utf-8")
            stubs = {
                "sniff": (
                    "import json, sys\n"
                    "assert sys.argv[1:] == ['os', '--json'], sys.argv\n"
                    "print(json.dumps({'os_type': 'MacOS', 'kernel': 'Darwin'}))\n"
                ),
                "just": (
                    "import json, os, sys\n"
                    "assert sys.argv[1] in ('_lint', '_test', '_test_l2'), sys.argv\n"
                    "with open(os.environ['TEST_CALL_LOG'], 'a', encoding='utf-8') as log:\n"
                    "    log.write(json.dumps({'args': sys.argv[1:]}) + '\\n')\n"
                ),
                "cargo": "raise SystemExit('Rust builds are forbidden in this test')\n",
                "tmux": "raise SystemExit('tmux must only be detected, never started')\n",
            }
            for name, body in stubs.items():
                path = bin_dir / name
                path.write_text("#!/usr/bin/env python3\n" + body, encoding="utf-8")
                path.chmod(0o755)
            environment = clean_policy_environment()
            relocate_home(environment, root / "home")
            environment.update({
                "PATH": str(bin_dir) + os.pathsep + environment.get("PATH", ""),
                "TEST_CALL_LOG": str(root / "calls.jsonl"),
                "TEST_PLANNER_LOG": str(root / "planner.log"),
                "TEST_REAL_PLANNER": str(PLANNER),
            })
            for key in (
                "BISCUIT_L2_THREADS", "BISCUIT_TEST_REQUIRED_BACKENDS", "BISCUIT_CI_SCOPE_OUT",
                "BISCUIT_CI_PLAN_OUT", "BISCUIT_CI_PLAN_IN", "BISCUIT_CI_REPORTS_OUT",
                "WEZTERM_UNIX_SOCKET", "KITTY_LISTEN_ON",
            ):
                environment.pop(key, None)
            command = [JUST, "--justfile", str(root / "justfile"), "ci-local", "--l2"]
            if through_env:
                environment["BISCUIT_CI_PLAN_IN"] = str(root / "plan.json")
            else:
                command += ["--plan-in", str(root / "plan.json")]
            command += ["--plan-out", str(root / "written-plan.json")]
            result = subprocess.run(
                command, cwd=root, env=environment, capture_output=True, text=True, timeout=60
            )
            self.assertEqual(0, result.returncode, result.stdout + result.stderr)
            calls_path = root / "calls.jsonl"
            calls = (
                [json.loads(line)["args"] for line in calls_path.read_text().splitlines()]
                if calls_path.is_file()
                else []
            )
            planner_log = root / "planner.log"
            return {
                "calls": calls,
                "planner": planner_log.read_text().splitlines() if planner_log.is_file() else [],
                "stdout": result.stdout,
                "written_plan": json.loads((root / "written-plan.json").read_text()),
                "plan": document,
            }

    def test_a_reused_passing_cell_is_skipped_and_the_planner_never_selects(self) -> None:
        run = self.run_fed("pass")
        self.assertEqual(
            [["_lint", "alpha"], ["_lint", "beta"], ["_test", "beta", "--no-fail-fast", "--features", "beta-tests"]],
            run["calls"],
            "only beta's L1 runs here: alpha's is reused from a pass, alpha's other L1 "
            "belongs to ubuntu-latest, and lint is never reusable",
        )
        # One projection (`--apply-to`) and no selection: the trampoline would
        # have failed the recipe on a selection call, and the log proves the
        # projection call is the only one.
        self.assertEqual(1, len(run["planner"]), run["planner"])
        self.assertIn("--apply-to", run["planner"][0])
        self.assertIn("skip   alpha/L1", run["stdout"])
        self.assertIn("run    beta/L1", run["stdout"])

    def test_a_reused_failing_cell_is_rerun(self) -> None:
        run = self.run_fed("fail")
        self.assertEqual(
            [
                ["_lint", "alpha"],
                ["_test", "alpha", "--no-fail-fast", "--features", "alpha-tests"],
                ["_lint", "beta"],
                ["_test", "beta", "--no-fail-fast", "--features", "beta-tests"],
            ],
            run["calls"],
            "a cell whose newest prior evidence is a failure is rerun locally (D2)",
        )
        self.assertIn("rerun  alpha/L1", run["stdout"])
        self.assertEqual(1, len(run["planner"]), run["planner"])

    def test_the_cells_that_run_here_report_the_build_key_they_were_planned_for(self) -> None:
        """Task 6.2: one local target tree, and the key that says what it holds.

        A same-process run serializes no archive, so the planned build key is
        the only thing by which this run and a CI run can be said to have
        compiled the same program. It is reported for the cells that run on
        THIS environment, and for no others.
        """
        run = self.run_fed("pass")
        beta_key = plan_fixtures._key("beta", "macos-latest")
        alpha_linux_key = plan_fixtures._key("alpha", "ubuntu-latest")
        self.assertIn(
            "Build keys for macos-latest (one local target tree, no archive serialized)",
            run["stdout"],
        )
        self.assertIn(f"  beta  {beta_key}  L1", run["stdout"])
        self.assertNotIn(
            alpha_linux_key,
            run["stdout"],
            "a cell planned for another environment is not this run's build",
        )
        # alpha's macOS L1 is reused, so the planner gave it no build record at
        # all; a reported key here would be an invented one.
        self.assertNotIn("  alpha  ", run["stdout"].split("Build keys for")[1])

    def test_the_environment_form_feeds_the_same_plan_and_writes_it_back_unchanged(self) -> None:
        # The hook hands the plan over as BISCUIT_CI_PLAN_IN; `--plan-out` must
        # then be that plan byte-for-byte, evidence rejections included, so
        # the receipt the hook records is against the plan the gates ran from.
        run = self.run_fed("pass", through_env=True)
        self.assertEqual(
            [["_lint", "alpha"], ["_lint", "beta"], ["_test", "beta", "--no-fail-fast", "--features", "beta-tests"]],
            run["calls"],
        )
        self.assertEqual(run["plan"], run["written_plan"])


#: `CI_WORKFLOW_UNDER_TEST=<path>` runs the step suites against another copy
#: of the workflow, so a new regression can be shown to fail on the pre-change
#: step (the same seam `PRE_PUSH_HOOK_UNDER_TEST` gives the hook suite).
WORKFLOW = Path(os.environ.get("CI_WORKFLOW_UNDER_TEST") or ROOT / ".github" / "workflows" / "ci.yml")
SCOPE_STEP = "Calculate package and area scope"
NULL_OID = "0" * 40


BASH_OVERRIDE = "BISCUIT_TEST_BASH"
# Prefixes that commonly hold a modern Bash *behind* the system one on PATH:
# Homebrew (Apple Silicon and Intel), distro `/usr/bin`, and Git for Windows.
WELL_KNOWN_BASH: tuple[str, ...] = (
    "/opt/homebrew/bin/bash",
    "/usr/local/bin/bash",
    "/usr/bin/bash",
    str(Path(os.environ.get("ProgramFiles", r"C:\Program Files")) / "Git" / "bin" / "bash.exe"),
)
STEP_BASH_REQUIREMENT = "Bash >= 4.4 (`mapfile -d`) and jq"


def bash_version(candidate: str) -> tuple[int, int] | None:
    probe = 'printf "%s %s" "${BASH_VERSINFO[0]}" "${BASH_VERSINFO[1]}"'
    try:
        result = subprocess.run(
            [candidate, "-c", probe], capture_output=True, text=True, timeout=10
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if result.returncode != 0:
        return None
    parts = result.stdout.split()
    if len(parts) != 2 or not all(part.isdigit() for part in parts):
        return None
    return int(parts[0]), int(parts[1])


def bash_runs_the_scope_step(version: tuple[int, int]) -> bool:
    # The step's `mapfile -d ''` needs 4.4; macOS ships 3.2 as `/bin/bash`.
    major, minor = version
    return major >= 5 or (major == 4 and minor >= 4)


def bash_candidates(
    environment: Mapping[str, str], well_known: Sequence[str] = WELL_KNOWN_BASH
) -> list[str]:
    """Every place a Bash may live, in the order they are tried.

    `shutil.which` stops at the first PATH hit, which on macOS is whichever of
    the system 3.2 and a Homebrew 5.x the developer happened to put first —
    the exact PATH-ordering accident this resolver exists to remove.

    An override is the *only* candidate: falling through past an incompatible
    one to some other Bash would hide the misconfiguration it was set to fix.
    """
    override = environment.get(BASH_OVERRIDE)
    if override:
        return [override]
    names = ["bash"]
    if sys.platform == "win32":
        names = ["bash" + ext.lower() for ext in environment.get("PATHEXT", ".EXE").split(";") if ext]
    candidates: list[str] = []
    for entry in environment.get("PATH", "").split(os.pathsep):
        if entry:
            candidates.extend(str(Path(entry) / name) for name in names)
    candidates.extend(well_known)
    return list(dict.fromkeys(candidates))


def resolve_step_bash(
    environment: Mapping[str, str] | None = None,
    well_known: Sequence[str] = WELL_KNOWN_BASH,
) -> str | None:
    """The first Bash able to run the extracted scope step, as an absolute path, or None."""
    for candidate in bash_candidates(os.environ if environment is None else environment, well_known):
        if not Path(candidate).is_file():
            continue
        version = bash_version(candidate)
        if version is not None and bash_runs_the_scope_step(version):
            return str(Path(candidate).resolve())
    return None


STEP_BASH = resolve_step_bash()
SCOPE_REF = "refs/notes/ci-local/scope"
SCOPE_MARKER = "carried by the local scope receipt"
#: A real workspace source path, so a fixture commit touching it selects a
#: real package and the plan owns cells a validation receipt can satisfy.
SOURCE_FILE = "biscuit-hash/lib/src/lib.rs"
#: Three more paths in that same package, for the deletion-identity fixtures.
DELETED_FILE = "biscuit-hash/lib/src/gone.rs"
RENAMED_FROM = "biscuit-hash/lib/src/before.rs"
RENAMED_TO = "biscuit-hash/lib/src/after.rs"


def fixture_git(root: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-c", "user.name=t", "-c", "user.email=t@example.invalid",
         "-c", "commit.gpgsign=false", *args],
        cwd=root, check=True, capture_output=True, text=True,
        env={**os.environ, "GIT_TERMINAL_PROMPT": "0"},
    ).stdout.strip()


PLANNER_CALL = "affected_scope.py "
RUSTUP_CALL = "rustup "


class StepRun:
    """One execution of the scope job's `run:` steps through the scope step.

    `result` is the scope step's own process. `calls` is the shared tool log
    in invocation order — every planner and `rustup` call any step made —
    with `planner_calls` and `rustup_calls` the two projections of it.
    """

    def __init__(self, root: Path, result: subprocess.CompletedProcess, output: Path, summary: Path, call_log: Path) -> None:
        self.result = result
        self.summary = summary.read_text(encoding="utf-8")
        self.calls = call_log.read_text(encoding="utf-8").splitlines() if call_log.is_file() else []
        self.planner_calls = [call[len(PLANNER_CALL):] for call in self.calls if call.startswith(PLANNER_CALL)]
        self.rustup_calls = [call for call in self.calls if call.startswith(RUSTUP_CALL)]
        self.outputs = dict(
            line.split("=", 1)
            for line in output.read_text(encoding="utf-8").splitlines()
            if "=" in line
        )
        plan_path = root / "resolved-plan.json"
        self.plan_bytes = plan_path.read_text(encoding="utf-8") if plan_path.is_file() else ""
        self.plan = json.loads(self.plan_bytes) if self.plan_bytes else {}
        scope_path = root / "scope.json"
        self.scope = json.loads(scope_path.read_text(encoding="utf-8")) if scope_path.is_file() else {}

    def summary_row(self, field: str) -> str:
        for line in self.summary.splitlines():
            if line.startswith(f"| {field} |"):
                return line.split("|")[2].strip()
        return ""

    def scope_source(self) -> str:
        return self.summary_row("scope source")


class WorkflowScopeStepTests(unittest.TestCase):
    """The scope job's shell, run for real against every event shape it handles.

    Every `run:` step of the job through the scope step executes, in order,
    one shell each as GitHub runs them, so a toolchain step placed ahead of
    the receipt decision is seen — the proof that a receipt hit sets up no
    Rust has to span that boundary. The REAL planner and evidence verifier
    run (the workspace's, reached through a trampoline in the temp cwd, since
    the step spells them as cwd-relative paths); `rustup` is a counting stub
    first on PATH, since the toolchain must not be materialized by a test.
    Only the Git repository the step diffs and verifies against is a fixture.
    Three commits — `root`, `base`, `head` — give the diff-based events a
    resolvable `base..head` and a second real base for a mismatched scope
    receipt; the full-run events never look at them.

    The step runs under the Bash that `resolve_step_bash` found, never a bare
    `bash` from PATH; `BISCUIT_TEST_BASH` names one explicitly and is then the
    only one considered.
    """

    @classmethod
    def setUpClass(cls) -> None:
        # A developer host may lack a modern Bash, and skipping there is
        # honest. The hosted `ci-tooling` job is the only place these contracts
        # are guaranteed to execute, so a skip there would be a green cell that
        # verified nothing — which is what BISCUIT_REQUIRE_BASH rules out.
        require_tools(
            "bash",
            "jq",
            enforced_by=CI_TOOLING,
            detail=(
                f"This class needs {STEP_BASH_REQUIREMENT}; set {BASH_OVERRIDE} to "
                f"point at one (tried: {', '.join(bash_candidates(os.environ))})."
            ),
            locate=lambda tool: STEP_BASH if tool == "bash" else shutil.which(tool),
        )
        steps = job_run_steps(WORKFLOW, "scope")
        names = [step.name for step in steps]
        if SCOPE_STEP not in names:
            raise AssertionError(f"the scope job has no {SCOPE_STEP!r} run step")
        cls.steps = steps[: names.index(SCOPE_STEP) + 1]

    def run_step(
        self,
        event: str,
        *,
        push_base: str | None = None,
        scope_receipt=None,
        validation_receipt: bool = False,
        evidence: Sequence[dict] | None = None,
        expect_failure: bool = False,
        verifier_failure: str | None = None,
        overlay_failure: bool = False,
        seed=None,
    ) -> StepRun:
        """The job's run steps through the scope step, for one event, over a
        fresh fixture repository.

        `seed(root) -> (first, base, head)` builds that repository; the default
        is `seed_repository`, and every receipt fixture depends on the paths it
        commits, so a test needing other history supplies its own.

        `scope_receipt(root, base, head) -> str` is attached to `head` under
        `refs/notes/ci-local/scope` before the steps run; `validation_receipt`
        publishes a complete passing macOS L1 receipt for the source package
        so the accepted-cells path is exercised, and `evidence` publishes one
        receipt per entry with `publish_validation_receipt`'s keyword
        arguments (environment, outcome, mutate).

        `verifier_failure` breaks only `local_evidence.py verify` (the
        trampoline's other subcommands stay real): a digit string is the exit
        code it dies with, `"garbage"` prints a non-JSON line and exits 0.
        `overlay_failure` breaks only the planner's `--apply-to` form: it
        clobbers the `--plan-out` target and exits 7, so a step that let the
        overlay write straight into `resolved-plan.json` would hand the
        fan-out a corrupt plan.
        """
        with tempfile.TemporaryDirectory(prefix="ci-scope-step-") as temporary:
            root = Path(temporary).resolve()
            first, base, head = (seed or self.seed_repository)(root)
            scripts = root / "scripts" / "ci"
            scripts.mkdir(parents=True)
            # The diff parser is a real file, not a trampoline: it is the
            # boundary under test here, so the step must run the shipped one.
            shutil.copyfile(
                ROOT / "scripts" / "ci" / "diff_scope.py", scripts / "diff_scope.py"
            )
            call_log = root / "tool-calls.log"
            for tool in ("affected_scope.py", "local_evidence.py"):
                real = ROOT / "scripts" / "ci" / tool
                # The planner trampoline records every invocation in the
                # shared call log: on a scope hit the proof is that it was
                # never invoked for selection, and on a miss that the
                # toolchain was set up before it. Both trampolines carry an
                # evidence-processing fault, armed by `run_step`, that fires
                # on exactly one subcommand.
                if tool == "affected_scope.py":
                    fault = (
                        "import os, sys\n"
                        "if os.environ.get('TEST_CALL_LOG'):\n"
                        "    with open(os.environ['TEST_CALL_LOG'], 'a', encoding='utf-8') as log:\n"
                        f"        log.write({PLANNER_CALL!r} + ' '.join(sys.argv[1:]) + '\\n')\n"
                        "if os.environ.get('TEST_OVERLAY_FAILURE') and '--apply-to' in sys.argv:\n"
                        "    out = sys.argv[sys.argv.index('--plan-out') + 1]\n"
                        "    open(out, 'w', encoding='utf-8').write('{\"corrupt\": true}')\n"
                        "    sys.exit(7)\n"
                    )
                else:
                    fault = (
                        "import os, sys\n"
                        "mode = os.environ.get('TEST_VERIFIER_FAILURE')\n"
                        "if mode and sys.argv[1:2] == ['verify']:\n"
                        "    if mode == 'garbage':\n"
                        "        print('not json {')\n"
                        "        sys.exit(0)\n"
                        "    sys.exit(int(mode))\n"
                    )
                (scripts / tool).write_text(
                    "import runpy\n" + fault
                    + f"runpy.run_path({str(real)!r}, run_name='__main__')\n",
                    encoding="utf-8",
                )
            shutil.copyfile(ROOT / "rust-toolchain.toml", root / "rust-toolchain.toml")
            # The toolchain invocation counter. A real `rustup show` would
            # install the pinned toolchain on a bare host, which is exactly
            # the cost the receipt path must never pay, so the stub records
            # the call and does nothing else.
            stubs = root / "bin"
            stubs.mkdir()
            rustup = stubs / "rustup"
            rustup.write_text(
                "#!/bin/sh\n"
                f"printf '%s%s\\n' {RUSTUP_CALL!r} \"$*\" >> \"$TEST_CALL_LOG\"\n",
                encoding="utf-8",
            )
            rustup.chmod(0o755)
            if scope_receipt is not None:
                self.receipt_event = event
                fixture_git(root, "notes", "--ref", SCOPE_REF, "add", "-f", "-m",
                            scope_receipt(root, base, head), head)
            for receipt in ([{}] if validation_receipt else []) + list(evidence or []):
                self.publish_validation_receipt(root, base, head, **receipt)
            output = root / "github-output"
            summary = root / "github-step-summary"
            output.touch()
            summary.touch()

            environment = clean_policy_environment()
            for key in ("GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"):
                environment.pop(key, None)
            environment.update(
                {
                    "PATH": os.pathsep.join([str(stubs), environment.get("PATH", "")]),
                    "GITHUB_OUTPUT": str(output),
                    "GITHUB_STEP_SUMMARY": str(summary),
                    "TEST_CALL_LOG": str(call_log),
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
            if verifier_failure is not None:
                environment["TEST_VERIFIER_FAILURE"] = verifier_failure
            if overlay_failure:
                environment["TEST_OVERLAY_FAILURE"] = "1"
            # One shell per step, as GitHub runs them; a failing step ends
            # the job unless it is `continue-on-error` (the notes fetch is,
            # and fails here because the fixture has no `origin`).
            for step in self.steps:
                result = subprocess.run(
                    [STEP_BASH, "-c", step.script],
                    cwd=root,
                    env=environment,
                    capture_output=True,
                    text=True,
                    timeout=120,
                )
                if result.returncode != 0 and not step.continue_on_error:
                    break
            run = StepRun(root, result, output, summary, call_log)
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

    @staticmethod
    def seed_deletion_and_rename(root: Path) -> tuple[str, str, str]:
        """`base..head` deletes one Rust file and renames another.

        The two shapes the boundary must keep apart. A deletion has to reach
        the planner declared as one; a rename's SOURCE has to reach it as
        nothing at all, because a changed path that does not exist and was
        never reported deleted is precisely the ambiguity `--deleted` removes.
        """
        fixture_git(root, "init", "-q", "-b", "main")
        (root / "README.md").write_text("zero\n", encoding="utf-8")
        fixture_git(root, "add", "README.md")
        fixture_git(root, "commit", "-q", "-m", "first")
        first = fixture_git(root, "rev-parse", "HEAD")
        source = root / SOURCE_FILE
        source.parent.mkdir(parents=True)
        source.write_text("pub fn fixture() {}\n", encoding="utf-8")
        (root / DELETED_FILE).write_text("pub fn gone() {}\n", encoding="utf-8")
        (root / RENAMED_FROM).write_text("pub fn moved() {}\n", encoding="utf-8")
        fixture_git(root, "add", "-A")
        fixture_git(root, "commit", "-q", "-m", "base")
        base = fixture_git(root, "rev-parse", "HEAD")
        source.write_text("pub fn fixture() { let _ = 1; }\n", encoding="utf-8")
        fixture_git(root, "rm", "-q", DELETED_FILE)
        fixture_git(root, "mv", RENAMED_FROM, RENAMED_TO)
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

    #: The event the receipt under construction is planned for: `run_step`
    #: sets it to the event it is about to run, as the hook plans for the
    #: event the push will trigger. A receipt for another event must miss.
    receipt_event: str | None = None

    def planned_documents(self, root: Path, base: str, head: str) -> tuple[dict, dict]:
        """The real planner's plan and projection for the fixture's `base..head`."""
        event_args = ["--event", self.receipt_event] if self.receipt_event else []
        projection = json.loads(self.tool(
            root, "affected_scope.py", "--plan-out", "receipt-plan.json",
            "--base", base, "--head", head, *event_args, "--", "README.md", SOURCE_FILE,
        ))
        plan = json.loads((root / "receipt-plan.json").read_text(encoding="utf-8"))
        (root / "tool-calls.log").unlink(missing_ok=True)
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

    def publish_validation_receipt(
        self,
        root: Path,
        base: str,
        head: str,
        environment: str = "macos-latest",
        outcome: str = "pass",
        mutate=None,
    ) -> None:
        """A complete L1 receipt for the source package on `head`, under
        `refs/notes/ci-local/<environment>`.

        A failing outcome names its failing test, since a failure the report
        cannot attribute is recorded `partial` and reused by nothing.
        `mutate(document)` edits the recorded receipt before it is attached,
        for notes the verifier must refuse.
        """
        plan, _ = self.planned_documents(root, base, head)
        (root / "validation-plan.json").write_text(schema.canonical(plan), encoding="utf-8")
        stage_root = root / "stage" / environment
        stage = stage_root / "L1"
        stage.mkdir(parents=True)
        failures = 0 if outcome == "pass" else 1
        case = (
            '<testcase classname="biscuit-hash" name="one"/>' if outcome == "pass"
            else '<testcase classname="biscuit-hash" name="one"><failure message="boom"/></testcase>'
        )
        (stage / "biscuit-hash.xml").write_text(
            f'<?xml version="1.0"?><testsuites><testsuite name="biscuit-hash" tests="1" '
            f'failures="{failures}" errors="0" skipped="0">{case}</testsuite></testsuites>',
            encoding="utf-8",
        )
        (stage_root / "manifest.jsonl").write_text(
            json.dumps({"tier": "L1", "package": "biscuit-hash", "xml": "L1/biscuit-hash.xml",
                        "exit_code": failures, "environment": environment, "duration_s": 1,
                        "report_present": True}) + "\n",
            encoding="utf-8",
        )
        receipt = self.tool(
            root, "local_evidence.py", "record-cells", "--plan", "validation-plan.json",
            "--stage", str(stage_root), "--base", base, "--head", head,
            "--environment", environment, "--report-dir", str(stage_root),
        ).strip()
        if mutate is not None:
            document = json.loads(receipt)
            mutate(document)
            receipt = schema.canonical(document)
        fixture_git(root, "notes", "--ref", f"refs/notes/ci-local/{environment}", "add", "-f", "-m", receipt, head)

    # -- event shapes --------------------------------------------------------

    def test_a_manual_run_reaches_the_planner_with_full_scope(self) -> None:
        run = self.run_step("workflow_dispatch")
        self.assertEqual("true", run.outputs["full_scope"])
        self.assertEqual(NULL_OID, run.plan["base"])
        # Selection reads `cargo metadata`, so the manual path still sets up
        # the toolchain, and says it ran no verifier rather than `none`.
        self.assertEqual(["rustup show"], run.rustup_calls)
        self.assertEqual("n/a (workflow_dispatch runs no verifier)", run.summary_row("validation environments"))

    def test_a_pull_request_diffs_its_base_and_head(self) -> None:
        run = self.run_step("pull_request")
        self.assertEqual("false", run.outputs["full_scope"])
        self.assertNotEqual(NULL_OID, run.plan["base"])

    def test_a_push_with_a_real_base_diffs_it(self) -> None:
        run = self.run_step("push")
        self.assertEqual("false", run.outputs["full_scope"])
        self.assertNotEqual(NULL_OID, run.plan["base"])

    def test_a_pull_request_declares_every_deletion_to_the_planner(self) -> None:
        run = self.run_step("pull_request", seed=self.seed_deletion_and_rename)
        self.assertTrue(
            any("--deleted" in call for call in run.planner_calls),
            f"the step never passed --deleted: {run.planner_calls}",
        )
        inventory = run.plan["change_inventory"]
        self.assertTrue(inventory["diff_available"])
        self.assertEqual([DELETED_FILE], inventory["deleted"])
        changed = {path for paths in inventory["paths"].values() for path in paths}
        # A deletion changed: it is in its bucket AND declared removed.
        self.assertIn(DELETED_FILE, changed)
        # The guard therefore scans the file that still exists and skips the
        # one that does not, rather than reading both absences the same way.
        guard = run.plan["archive_guard"]
        self.assertEqual("changed", guard["mode"])
        self.assertIn(RENAMED_TO, guard["paths"])
        self.assertNotIn(DELETED_FILE, guard["paths"])

    def test_a_renames_source_is_neither_changed_nor_deleted(self) -> None:
        # The unexpectedly-missing path, produced the only way a real diff
        # produces one: mis-stepping `--name-status -z`, whose rename record
        # carries two paths for one status. The source exists nowhere at
        # `head`, so listing it as changed would hand the guard an absence it
        # could not explain — and listing it as deleted would claim the diff
        # reported a removal it never reported.
        run = self.run_step("pull_request", seed=self.seed_deletion_and_rename)
        inventory = run.plan["change_inventory"]
        changed = {path for paths in inventory["paths"].values() for path in paths}
        self.assertIn(RENAMED_TO, changed)
        self.assertNotIn(RENAMED_FROM, changed)
        self.assertNotIn(RENAMED_FROM, inventory["deleted"])

    def test_a_branch_creating_push_runs_the_full_scope(self) -> None:
        run = self.run_step("push", push_base=NULL_OID)
        self.assertEqual("true", run.outputs["full_scope"])
        self.assertEqual(NULL_OID, run.plan["base"])
        self.assertEqual("CI (no comparison base)", run.scope_source())

    # -- local scope evidence (2026-09-10 spec R3) ---------------------------

    def test_a_matching_scope_receipt_is_the_plan_and_the_planner_never_runs(self) -> None:
        run = self.run_step("push", scope_receipt=self.local_scope_receipt)
        self.assertEqual([], run.planner_calls, "the planner ran although the receipt matched")
        self.assertEqual([], run.rustup_calls, "a receipt hit needs no Rust, so no step may set up the toolchain (R9)")
        self.assertEqual(f"local ({SCOPE_REF} @ {run.plan['head'][:9]})", run.scope_source())
        self.assertEqual(SCOPE_MARKER, run.outputs["preflight_reason"])
        self.assertEqual(SCOPE_MARKER, run.plan["preflight_reason"])
        # `test-toolkit` rides along as the archive-path guard's lint-only
        # owner: the changed file is Rust, which is source the guard scans.
        self.assertEqual(
            ["biscuit-hash", "test-toolkit"],
            [entry["package"] for entry in run.plan["packages"]],
        )
        self.assertEqual("true", run.outputs["has_packages"])

    def test_the_written_plan_is_byte_identical_to_the_receipts(self) -> None:
        receipts: dict = {}

        def remember(root: Path, base: str, head: str) -> str:
            receipts["text"] = self.local_scope_receipt(root, base, head)
            return receipts["text"]

        run = self.run_step("pull_request", scope_receipt=remember)
        receipt = json.loads(receipts["text"])
        self.assertEqual(schema.canonical(receipt["plan"]), run.plan_bytes)
        self.assertEqual(receipt["scope"], run.scope)
        # Either document could serve: the carried projection is exactly what
        # projecting the carried plan yields, so a hit with no evidence and a
        # hit with evidence read the same shape.
        self.assertEqual(receipt["scope"], legacy_scope_document(receipt["plan"]))

    def test_a_receipt_for_another_base_falls_back_with_its_code(self) -> None:
        run = self.run_step("push", push_base="FIRST", scope_receipt=self.local_scope_receipt)
        self.assertTrue(run.scope_source().startswith("CI fallback (scope-base-mismatch:"), run.scope_source())
        self.assertEqual(1, len(run.planner_calls), run.planner_calls)
        self.assertNotEqual(SCOPE_MARKER, run.plan["preflight_reason"])

    def test_a_missing_receipt_falls_back_with_its_code(self) -> None:
        run = self.run_step("pull_request")
        self.assertTrue(run.scope_source().startswith("CI fallback (scope-missing:"), run.scope_source())
        self.assertEqual(1, len(run.planner_calls), run.planner_calls)

    # -- the toolchain is set up only for selection (2026-09-10 spec R9) -----

    def test_a_receipt_miss_sets_up_the_toolchain_once_before_selection(self) -> None:
        run = self.run_step("pull_request")
        self.assertEqual(["rustup show"], run.rustup_calls)
        # Order, from the shared log: the toolchain first, then the one
        # selection run that needs it, and nothing in between.
        self.assertEqual(2, len(run.calls), run.calls)
        self.assertEqual("rustup show", run.calls[0])
        self.assertTrue(run.calls[1].startswith(PLANNER_CALL), run.calls)
        self.assertNotIn("--apply-to", run.calls[1])

    def test_a_hit_with_accepted_evidence_sets_up_no_toolchain_and_reports_the_reuse(self) -> None:
        run = self.run_step("pull_request", scope_receipt=self.local_scope_receipt, validation_receipt=True)
        self.assertEqual(["reuse"], self.reused_l1(run))
        self.assertEqual([], run.rustup_calls, "verify --cells and --apply-to are Python; no step may set up Rust")
        self.assertEqual(1, len(run.planner_calls), run.planner_calls)
        self.assertIn("--apply-to", run.planner_calls[0])
        self.assertEqual("consulted: macos-latest; matched: macos-latest", run.summary_row("validation environments"))
        # No check rides on the macOS pass: check is a single-environment gate
        # hosted on Linux (fixes/2026-09-18-ci-cadence, decision 3).
        self.assertEqual("1: biscuit-hash/macos-latest/L1", run.summary_row("reused passing cells"))
        self.assertEqual("none", run.summary_row("cells retained (evidence incomplete or rejected)"))

    # -- the summary's evidence provenance (2026-09-10 spec R9, AC15) --------

    def test_all_rejected_evidence_still_publishes_its_rejections(self) -> None:
        def interrupted(document: dict) -> None:
            document["completion"] = "interrupted"

        run = self.run_step("pull_request", scope_receipt=self.local_scope_receipt,
                            evidence=[{"mutate": interrupted}])
        rejections = run.plan["evidence_rejections"]
        self.assertEqual(1, len(rejections), rejections)
        self.assertTrue(rejections[0].startswith("incomplete-run: the macos-latest run is interrupted"), rejections)
        self.assertEqual([], run.plan["accepted_evidence"])
        self.assertEqual(["execute"], self.reused_l1(run))
        # The refusal reached the plan through the overlay, with nothing to
        # accept, and selection still never ran.
        self.assertEqual(1, len(run.planner_calls), run.planner_calls)
        self.assertIn("--apply-to", run.planner_calls[0])
        self.assertEqual([], run.rustup_calls)
        self.assertEqual("consulted: macos-latest; matched: none", run.summary_row("validation environments"))
        self.assertEqual("none", run.summary_row("reused passing cells"))
        self.assertEqual("1 rejection(s): incomplete-run (1)", run.summary_row("cells retained (evidence incomplete or rejected)"))

    def test_passing_cells_are_reused_and_failing_cells_are_retained(self) -> None:
        run = self.run_step(
            "pull_request",
            scope_receipt=self.local_scope_receipt,
            evidence=[{"environment": "macos-latest", "outcome": "pass"},
                      {"environment": "ubuntu-latest", "outcome": "fail"}],
        )
        reused = {
            (cell["environment"], cell["evidence"]["outcome"])
            for cell in run.plan["cells"] if cell["execution"] == "reuse"
        }
        self.assertEqual({("macos-latest", "pass")}, reused)
        self.assertEqual("consulted: macos-latest, ubuntu-latest; matched: macos-latest",
                         run.summary_row("validation environments"))
        # No check rides on the macOS pass: check is a single-environment gate
        # hosted on Linux (fixes/2026-09-18-ci-cadence, decision 3).
        self.assertEqual("1: biscuit-hash/macos-latest/L1", run.summary_row("reused passing cells"))
        self.assertEqual("1 rejection(s): failed-cell (1)", run.summary_row("cells retained (evidence incomplete or rejected)"))
        self.assertEqual([], run.rustup_calls)

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

    def test_a_receipt_planned_for_another_event_falls_back(self) -> None:
        # A plan schedules the environments of the event it was planned for,
        # so a push-event receipt (Windows planned, say) cannot stand in for a
        # pull request, and one planned with no event at all cannot either.
        def push_receipt(root: Path, base: str, head: str) -> str:
            self.receipt_event = "push"
            return self.local_scope_receipt(root, base, head)

        def eventless_receipt(root: Path, base: str, head: str) -> str:
            self.receipt_event = None
            return self.local_scope_receipt(root, base, head)

        for label, receipt in (("push", push_receipt), ("none", eventless_receipt)):
            with self.subTest(label):
                run = self.run_step("pull_request", scope_receipt=receipt)
                self.assertTrue(
                    run.scope_source().startswith("CI fallback (scope-event-mismatch:"),
                    run.scope_source(),
                )
                self.assertNotEqual(SCOPE_MARKER, run.plan["preflight_reason"])
                self.assertEqual("pull_request", run.plan["event"])
                self.assertEqual(1, len(run.planner_calls), run.planner_calls)
                self.assertIn("--event pull_request", run.planner_calls[0])

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

    def test_dispatch_ignores_published_validation_outcomes(self) -> None:
        run = self.run_step(
            "workflow_dispatch",
            scope_receipt=self.local_scope_receipt,
            evidence=[{"environment": "macos-latest", "outcome": "pass"},
                      {"environment": "ubuntu-latest", "outcome": "fail"}],
            verifier_failure="7",
        )
        self.assertEqual("true", run.outputs["full_scope"])
        self.assertTrue(run.plan["cells"])
        self.assertFalse(any(cell["execution"] == "reuse" for cell in run.plan["cells"]))
        self.assertEqual([], run.plan["accepted_evidence"])
        self.assertEqual("n/a (workflow_dispatch runs no verifier)",
                         run.summary_row("validation environments"))
        self.assertEqual(1, len(run.planner_calls), run.planner_calls)
        self.assertNotIn("--apply-to", run.planner_calls[0])

    def test_scope_hit_projects_the_plan_instead_of_stale_legacy_scheduling(self) -> None:
        def stale_projection(root: Path, base: str, head: str) -> str:
            document = json.loads(self.local_scope_receipt(root, base, head))
            document["scope"]["matrix"] = []
            document["scope"]["scheduled_areas"] = []
            document["scope"]["area_matrix"] = {}
            return schema.canonical(document)

        run = self.run_step("pull_request", scope_receipt=stale_projection)
        self.assertEqual(SCOPE_MARKER, run.plan["preflight_reason"])
        self.assertEqual([], run.planner_calls)
        self.assertEqual([], run.rustup_calls)
        self.assertEqual(legacy_scope_document(run.plan), run.scope)
        self.assertTrue(run.scope["matrix"])
        self.assertTrue(run.scope["scheduled_areas"])

    def test_malformed_scope_notes_recalculate_through_the_real_step(self) -> None:
        for malformed in ("{broken json", "[]"):
            with self.subTest(note=malformed):
                run = self.run_step(
                    "pull_request",
                    scope_receipt=lambda root, base, head: malformed,
                )
                self.assertTrue(run.scope_source().startswith("CI fallback (scope-malformed:"),
                                run.scope_source())
                self.assertEqual(1, len(run.planner_calls), run.planner_calls)
                self.assertEqual(["rustup show"], run.rustup_calls)
                self.assertNotEqual(SCOPE_MARKER, run.plan["preflight_reason"])
                self.assertTrue(run.plan["cells"])
                self.assertFalse(any(cell["execution"] == "reuse" for cell in run.plan["cells"]))

    # -- validation evidence on top of scope (review-2, "still recomputed") --

    #: What applying evidence may change. Everything else on the carried plan
    #: — selection, targets, features, environments, policy — must reach the
    #: fan-out byte-for-byte as the hook resolved it.
    EVIDENCE_PLAN_FIELDS = (
        "accepted_evidence", "evidence_rejections", "prohibited_cells", "job_estimate",
        "change_class", "preflight_os", "preflight_reason",
        # A cell satisfied by evidence stops demanding its build, and a record
        # whose last consumer is satisfied is removed. The overlay derives no
        # key — it only drops demand the carried plan already computed.
        "builds",
    )
    EVIDENCE_CELL_FIELDS = (
        "execution", "origin", "state", "evidence", "prohibition", "build",
    )

    @classmethod
    def without_evidence(cls, plan: dict) -> str:
        stripped = {key: value for key, value in plan.items() if key not in cls.EVIDENCE_PLAN_FIELDS}
        stripped["cells"] = [
            {key: value for key, value in cell.items() if key not in cls.EVIDENCE_CELL_FIELDS}
            for cell in plan["cells"]
        ]
        return schema.canonical(stripped)

    @staticmethod
    def reused_l1(run: StepRun) -> list[str]:
        return [
            cell["execution"] for cell in run.plan["cells"]
            if cell["package"] == "biscuit-hash" and cell["environment"] == "macos-latest" and cell["gate"] == "L1"
        ]

    def test_evidence_on_a_scope_hit_is_applied_to_the_carried_plan_without_selection(self) -> None:
        receipts: dict = {}

        def remember(root: Path, base: str, head: str) -> str:
            receipts["text"] = self.local_scope_receipt(root, base, head)
            return receipts["text"]

        run = self.run_step("pull_request", scope_receipt=remember, validation_receipt=True)
        self.assertTrue(run.scope_source().startswith("local ("), run.scope_source())
        self.assertEqual(1, len(run.planner_calls), run.planner_calls)
        for call in run.planner_calls:
            self.assertIn("--apply-to resolved-plan.json", call, "a planner call performed selection")
        self.assertEqual(["reuse"], self.reused_l1(run))
        carried = json.loads(receipts["text"])["plan"]
        self.assertEqual(self.without_evidence(carried), self.without_evidence(run.plan))
        self.assertEqual([carried["cells"][0]["package"]], [cell["package"] for cell in run.plan["cells"][:1]])

        # The reused macOS L1 cell was biscuit-hash's only consumer of the macOS
        # build, so the overlay removes that record while every other producer's
        # survives. No key is recomputed: the overlay reads no checkout.
        def macos_builds(plan: dict) -> list[str]:
            return [
                record["key"]
                for record in plan["builds"]
                if record["package"] == "biscuit-hash" and record["producer"] == "macos-latest"
            ]

        self.assertEqual(1, len(macos_builds(carried)), carried["builds"])
        self.assertEqual([], macos_builds(run.plan), run.plan["builds"])
        self.assertTrue(run.plan["builds"], "unrelated producers must keep their records")
        self.assertTrue(
            {record["key"] for record in run.plan["builds"]}
            <= {record["key"] for record in carried["builds"]},
            "the overlay may only remove records, never mint one",
        )

    def test_the_projection_on_an_evidence_hit_is_derived_from_the_written_plan(self) -> None:
        run = self.run_step("pull_request", scope_receipt=self.local_scope_receipt, validation_receipt=True)
        self.assertEqual(legacy_scope_document(run.plan), run.scope)
        entry = next(item for item in run.scope["matrix"] if item["package"] == "biscuit-hash")
        self.assertNotIn("macos-latest", entry["native_environments"], "the reused cell must leave the fan-out")
        self.assertEqual(run.scope["job_estimate"], run.plan["job_estimate"])
        self.assertEqual(str(run.plan["job_estimate"]), run.outputs["job_estimate"])

    def test_evidence_on_a_scope_miss_is_applied_after_one_selection_run(self) -> None:
        run = self.run_step("pull_request", validation_receipt=True)
        self.assertTrue(run.scope_source().startswith("CI fallback (scope-missing:"), run.scope_source())
        selection = [call for call in run.planner_calls if "--apply-to" not in call]
        applications = [call for call in run.planner_calls if "--apply-to" in call]
        self.assertEqual(1, len(selection), run.planner_calls)
        self.assertEqual(1, len(applications), run.planner_calls)
        self.assertNotIn("--accepted-cells", selection[0], "selection must not resolve evidence")
        self.assertEqual(["reuse"], self.reused_l1(run))
        self.assertEqual(legacy_scope_document(run.plan), run.scope)

    # -- evidence processing adds work, never removes it (R8, audit W4) ------

    #: Every matrix output the fan-out reads. A step that died before writing
    #: them would lose the run's package work, which is the defect.
    MATRIX_OUTPUTS = (
        "scheduled_areas", "area_matrix", "area_slugs", "gap_areas", "packages",
        "package_names", "has_packages", "full_scope", "sniff",
        "job_estimate", "preflight_os", "preflight_reason", "change_class",
    )

    def assert_unmodified_plan_and_nothing_reused(self, run: StepRun, carried_receipt: str) -> None:
        self.assertEqual(schema.canonical(json.loads(carried_receipt)["plan"]), run.plan_bytes,
                         "the written plan must be the pre-overlay plan, byte for byte")
        self.assertEqual([], run.plan.get("accepted_evidence", []))
        self.assertEqual([], [cell for cell in run.plan["cells"] if cell.get("execution") == "reuse"])
        self.assertEqual(legacy_scope_document(run.plan), run.scope)
        self.assertEqual("true", run.outputs["has_packages"])
        self.assertEqual(set(), set(self.MATRIX_OUTPUTS) - set(run.outputs), "matrix outputs missing")
        # The provenance rows describe the unmodified plan, honestly.
        self.assertEqual("consulted: macos-latest; matched: none", run.summary_row("validation environments"))
        self.assertEqual("none", run.summary_row("reused passing cells"))
        self.assertEqual("none", run.summary_row("cells retained (evidence incomplete or rejected)"))

    def test_a_crashing_verifier_retains_the_plan_and_reuses_nothing(self) -> None:
        receipts: dict = {}

        def remember(root: Path, base: str, head: str) -> str:
            receipts["text"] = self.local_scope_receipt(root, base, head)
            return receipts["text"]

        run = self.run_step("pull_request", scope_receipt=remember, validation_receipt=True,
                            verifier_failure="42")
        self.assert_unmodified_plan_and_nothing_reused(run, receipts["text"])
        self.assertEqual([], run.planner_calls, "no overlay may run on a verifier crash")
        self.assertIn("| evidence processing | verifier failed (exit 42); no cell reused |", run.summary)
        self.assertIn("::warning title=CI scope::local evidence verifier failed (exit 42)", run.result.stdout)

    def test_a_verifier_printing_garbage_is_a_failure_not_evidence(self) -> None:
        receipts: dict = {}

        def remember(root: Path, base: str, head: str) -> str:
            receipts["text"] = self.local_scope_receipt(root, base, head)
            return receipts["text"]

        run = self.run_step("pull_request", scope_receipt=remember, validation_receipt=True,
                            verifier_failure="garbage")
        self.assert_unmodified_plan_and_nothing_reused(run, receipts["text"])
        self.assertEqual([], run.planner_calls)
        self.assertIn("| evidence processing | verifier output is not a JSON list; no cell reused |", run.summary)
        self.assertIn("::warning title=CI scope::local evidence verifier output is not a JSON list", run.result.stdout)

    def test_a_crashing_overlay_retains_the_plan_and_reuses_nothing(self) -> None:
        receipts: dict = {}

        def remember(root: Path, base: str, head: str) -> str:
            receipts["text"] = self.local_scope_receipt(root, base, head)
            return receipts["text"]

        run = self.run_step("pull_request", scope_receipt=remember, validation_receipt=True,
                            overlay_failure=True)
        # The verifier accepted the macOS L1 cell, so the overlay was reached.
        self.assertEqual(1, len(run.planner_calls), run.planner_calls)
        self.assertIn("--apply-to resolved-plan.json", run.planner_calls[0])
        self.assert_unmodified_plan_and_nothing_reused(run, receipts["text"])
        self.assertIn("| evidence processing | overlay failed (exit 7); no cell reused |", run.summary)
        self.assertIn("::warning title=CI scope::local evidence overlay failed (exit 7)", run.result.stdout)


GATE_STEP = "Fold the blocking jobs' results"


class WorkflowGateStepTests(unittest.TestCase):
    """`ci-gate`'s fold, run for real over every `needs.*.result` shape.

    The semantics come from the scratch-repository fixture
    (`fixes/2026-09-11-cicd-cleanup/fixtures/scratch-2026-09-12.md`): an
    unselected area's job is `skipped` and must not block; `failure` and
    `cancelled` must. The step needs nothing beyond POSIX shell plus Bash's
    here-string, so any Bash runs it; the scope step's version floor is not
    required here.
    """

    @classmethod
    def setUpClass(cls) -> None:
        cls.script = step_script(WORKFLOW, GATE_STEP)
        require_tools(
            "bash",
            enforced_by=CI_TOOLING,
            locate=lambda tool: STEP_BASH or shutil.which(tool),
        )
        cls.bash = STEP_BASH or shutil.which("bash")

    def fold(self, results: dict[str, str], script: str | None = None) -> subprocess.CompletedProcess:
        env_block = "".join(f"{job}:{result}\n" for job, result in results.items())
        return subprocess.run(
            [self.bash, "-c", script or self.script],
            env={**clean_policy_environment(), "RESULTS": env_block},
            capture_output=True,
            text=True,
            timeout=30,
        )

    @staticmethod
    def all_success() -> dict[str, str]:
        return {
            job: "success"
            for job in ("validation", "scope", "preflight", "area-ci")
        }

    def test_every_job_succeeded_passes(self) -> None:
        run = self.fold(self.all_success())
        self.assertEqual(0, run.returncode, run.stdout + run.stderr)
        self.assertIn("every blocking job succeeded or was skipped", run.stdout)

    def test_a_skipped_job_is_accepted(self) -> None:
        # An unselected area, and the documentation-class preflight that now
        # expands no job at all (R10).
        results = {**self.all_success(), "area-ci": "skipped", "preflight": "skipped"}
        run = self.fold(results)
        self.assertEqual(0, run.returncode, run.stdout + run.stderr)

    def test_a_failed_job_blocks_and_is_named(self) -> None:
        run = self.fold({**self.all_success(), "area-ci": "failure"})
        self.assertEqual(1, run.returncode, run.stdout + run.stderr)
        self.assertIn("ci-gate: blocked by area-ci", run.stderr)
        self.assertIn("area-ci: failure (blocks)", run.stdout)

    def test_a_cancelled_job_blocks(self) -> None:
        run = self.fold({**self.all_success(), "area-ci": "cancelled"})
        self.assertEqual(1, run.returncode, run.stdout + run.stderr)
        self.assertIn("area-ci", run.stderr)

    def test_every_blocking_job_is_named_not_just_the_first(self) -> None:
        run = self.fold({**self.all_success(), "scope": "failure", "preflight": "cancelled"})
        self.assertEqual(1, run.returncode)
        self.assertIn("blocked by scope preflight", run.stderr)

    def test_the_assertions_reject_a_fold_that_accepts_failure(self) -> None:
        # Non-vacuity: the same inputs against a fold widened to accept
        # `failure` exit 0, so the blocking assertions above are load-bearing.
        widened = self.script.replace("success|skipped)", "success|skipped|failure)")
        self.assertNotEqual(widened, self.script, "the accept clause must be where the tests expect it")
        run = self.fold({**self.all_success(), "area-ci": "failure"}, script=widened)
        self.assertEqual(0, run.returncode, "a widened fold lets a failed job through")


class StepBashResolverTests(unittest.TestCase):
    """`resolve_step_bash` against fake Bash executables, with no well-known fallbacks."""

    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="ci-step-bash-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()

    def fake_bash(self, name: str, body: str) -> str:
        directory = self.root / name
        directory.mkdir()
        if sys.platform == "win32":
            path = directory / "bash.exe"
            path.write_text(body, encoding="utf-8")
        else:
            path = directory / "bash"
            path.write_text("#!/bin/sh\n" + body + "\n", encoding="utf-8")
            path.chmod(0o755)
        return str(path)

    def resolve(self, path_entries: list[str], override: str | None = None) -> str | None:
        environment = {"PATH": os.pathsep.join(path_entries)}
        if override is not None:
            environment[BASH_OVERRIDE] = override
        return resolve_step_bash(environment, well_known=())

    @unittest.skipIf(sys.platform == "win32", "the fake Bash executables are POSIX shell scripts")
    def test_version_threshold_is_4_4(self) -> None:
        old = self.fake_bash("old", "printf '3 2'")
        floor = self.fake_bash("floor", "printf '4 4'")
        below = self.fake_bash("below", "printf '4 3'")
        broken = self.fake_bash("broken", "exit 7")

        self.assertIsNone(self.resolve([str(Path(old).parent)]))
        self.assertIsNone(self.resolve([str(Path(below).parent)]))
        self.assertIsNone(self.resolve([str(Path(broken).parent)]))
        self.assertEqual(floor, self.resolve([str(Path(floor).parent)]))
        # First compatible candidate in PATH order, not first candidate.
        self.assertEqual(
            floor,
            self.resolve([str(Path(old).parent), str(Path(broken).parent), str(Path(floor).parent)]),
        )

    @unittest.skipIf(sys.platform == "win32", "the fake Bash executables are POSIX shell scripts")
    def test_override_wins_over_path(self) -> None:
        on_path = self.fake_bash("on-path", "printf '5 3'")
        override = self.fake_bash("override", "printf '5 0'")
        old_override = self.fake_bash("old-override", "printf '3 2'")

        self.assertEqual(override, self.resolve([str(Path(on_path).parent)], override=override))
        # An incompatible override is reported, never quietly replaced.
        self.assertIsNone(self.resolve([str(Path(on_path).parent)], override=old_override))

    @unittest.skipIf(sys.platform == "win32", "the fake Bash executables are POSIX shell scripts")
    def test_missing_candidates_are_skipped(self) -> None:
        modern = self.fake_bash("modern", "printf '5 3'")
        missing_dir = str(self.root / "no-such-dir")
        missing_override = str(self.root / "no-such-bash")

        self.assertEqual(modern, self.resolve([missing_dir, str(Path(modern).parent)]))
        self.assertIsNone(self.resolve([missing_dir]))
        self.assertIsNone(self.resolve([str(Path(modern).parent)], override=missing_override))

    def test_override_is_exclusive_and_well_known_comes_last(self) -> None:
        path = os.pathsep.join(["/p1", "/p2"])
        self.assertEqual(
            ["/override/bash"],
            bash_candidates({"PATH": path, BASH_OVERRIDE: "/override/bash"}, well_known=("/well/known/bash",)),
        )
        candidates = bash_candidates({"PATH": path}, well_known=("/well/known/bash",))
        self.assertEqual("/well/known/bash", candidates[-1])
        first_p1 = next(i for i, c in enumerate(candidates) if c.startswith(str(Path("/p1"))))
        first_p2 = next(i for i, c in enumerate(candidates) if c.startswith(str(Path("/p2"))))
        self.assertLess(first_p1, first_p2)


ABSENT: Callable[[str], object] = lambda tool: None
PRESENT: Callable[[str], object] = lambda tool: f"/fake/bin/{tool}"


class ToolGuardTests(unittest.TestCase):
    """`tool_guard`, in all three directions a host-tool guard can go.

    Presence and environment are injected rather than reached for: a test that
    mutated PATH or `os.environ` to prove a guard would leak that mutation into
    every other class in this file.
    """

    def test_a_present_tool_runs_the_contract(self) -> None:
        tool_guard.require_tools(
            "just", "jq", enforced_by=CI_TOOLING, locate=PRESENT, environment={}
        )

    def test_an_absent_undeclared_tool_skips_and_names_where_it_is_enforced(self) -> None:
        with self.assertRaises(unittest.SkipTest) as raised:
            tool_guard.require_tools(
                "jq", enforced_by=CI_TOOLING, locate=ABSENT, environment={}
            )
        message = str(raised.exception)
        self.assertIn("jq is absent", message)
        self.assertIn("ci-tooling", message)
        self.assertIn("BISCUIT_REQUIRE_JQ", message)

    def test_an_absent_tool_the_job_declared_fails_instead_of_skipping(self) -> None:
        with self.assertRaises(AssertionError) as raised:
            tool_guard.require_tools(
                "jq",
                enforced_by=CI_TOOLING,
                locate=ABSENT,
                environment={"BISCUIT_REQUIRE_JQ": "1"},
            )
        message = str(raised.exception)
        self.assertIn("BISCUIT_REQUIRE_JQ declared jq provisioned", message)
        self.assertIn("ci-tooling", message)

    def test_only_the_absent_tool_decides_and_only_its_own_variable(self) -> None:
        # `just` present, `jq` absent, and only `just` declared: the declaration
        # that matters is the missing tool's, so this still skips.
        with self.assertRaises(unittest.SkipTest):
            tool_guard.require_tools(
                "just",
                "jq",
                enforced_by=CI_TOOLING,
                locate=lambda tool: None if tool == "jq" else "/fake/bin/just",
                environment={"BISCUIT_REQUIRE_JUST": "1"},
            )

    def test_a_multi_tool_guard_names_every_missing_tool(self) -> None:
        with self.assertRaises(unittest.SkipTest) as raised:
            tool_guard.require_tools(
                "just", "jq", enforced_by=CI_TOOLING, locate=ABSENT, environment={}
            )
        self.assertIn("just, jq are absent", str(raised.exception))

    def test_the_detail_reaches_the_message(self) -> None:
        with self.assertRaises(unittest.SkipTest) as raised:
            tool_guard.require_tools(
                "jq",
                enforced_by=CI_TOOLING,
                detail="Set BISCUIT_TEST_BASH to point at one.",
                locate=ABSENT,
                environment={},
            )
        self.assertTrue(str(raised.exception).endswith("Set BISCUIT_TEST_BASH to point at one."))

    def test_a_guard_cannot_be_written_without_naming_its_enforcing_job(self) -> None:
        # The defect this module replaces, made unrepresentable: `enforced_by`
        # is keyword-only with no default.
        with self.assertRaises(TypeError):
            tool_guard.require_tools("jq")  # type: ignore[call-arg]

    def test_the_declaring_variable_is_derived_from_the_tool_name(self) -> None:
        self.assertEqual("BISCUIT_REQUIRE_SNIFF", tool_guard.declaring_variable("sniff"))
        self.assertEqual(
            "BISCUIT_REQUIRE_CARGO_NEXTEST", tool_guard.declaring_variable("cargo-nextest")
        )

    def test_the_class_decorator_skips_fails_and_runs_the_same_three_ways(self) -> None:
        for variables, expected in (
            ({}, unittest.SkipTest),
            ({"BISCUIT_REQUIRE_JQ": "1"}, AssertionError),
        ):
            with self.subTest(environment=variables):
                @tool_guard.requires_tools(
                    "jq", enforced_by=CI_TOOLING, locate=ABSENT, environment=variables
                )
                class Guarded(unittest.TestCase):
                    pass

                with self.assertRaises(expected):
                    Guarded.setUpClass()

        ran: list[str] = []

        @tool_guard.requires_tools("jq", enforced_by=CI_TOOLING, locate=PRESENT, environment={})
        class Present(unittest.TestCase):
            @classmethod
            def setUpClass(cls) -> None:
                ran.append(cls.__name__)

        Present.setUpClass()
        self.assertEqual(["Present"], ran)


@requires_tools("bash", "jq", enforced_by=CI_TOOLING)
class NativeProvisioningTests(unittest.TestCase):
    def provision(self, workflow: str, job: str, runner: str, native: dict, dependents: list) -> list[str]:
        step = next(
            step for step in job_run_steps(ROOT / ".github/workflows" / workflow, job)
            if step.name == "Install native prerequisites"
        )
        with tempfile.TemporaryDirectory(prefix="ci-native-") as temporary:
            output = Path(temporary) / "arguments"
            environment = os.environ.copy()
            environment.update(
                NATIVE=json.dumps(native), RUNNER_KEY=runner,
                DEPENDENTS_NATIVE=json.dumps(dependents), NATIVE_OUTPUT=str(output),
            )
            result = subprocess.run(
                [shutil.which("bash"), "-c", 'just() { printf "%s\\n" "$@" >> "$NATIVE_OUTPUT"; }\n' + step.script],
                cwd=temporary, env=environment, capture_output=True, text=True, timeout=10,
            )
            self.assertEqual(0, result.returncode, result.stderr)
            return output.read_text().splitlines() if output.exists() else []

    def test_only_the_ubuntu_check_adds_dependent_prerequisites(self) -> None:
        native = {"ubuntu-latest": ["own-dev"], "macos-latest": ["own-macos"]}
        dependents = ["own-dev", "consumer-dev", "transitive-dev"]
        self.assertEqual(
            ["_ensure-native-libs", "consumer-dev", "own-dev", "transitive-dev"],
            self.provision("_package-ci.yml", "check", "ubuntu-latest", native, dependents),
        )
        self.assertEqual(
            ["_ensure-native-libs", "own-macos"],
            self.provision("_package-ci.yml", "check", "macos-latest", native, dependents),
        )
        for job in ["test", "lint", "test-l2", "test-browser"]:
            with self.subTest(job=job):
                self.assertEqual(
                    ["_ensure-native-libs", "own-dev"],
                    self.provision("_package-ci.yml", job, "ubuntu-latest", native, dependents),
                )

    def test_empty_native_lists_never_invoke_the_whole_workspace_installer(self) -> None:
        for job in ["check", "test", "lint", "test-l2", "test-browser"]:
            with self.subTest(job=job):
                self.assertEqual([], self.provision("_package-ci.yml", job, "ubuntu-latest", {}, []))

    def provision_owner(self, native: list) -> list[str]:
        """The build owner's provisioning step, which takes a flat list.

        The owner leg is given the prerequisites of the ONE record it compiles,
        already resolved for its producer's runner label by the planner — not a
        runner-keyed map to index into.
        """
        step = next(
            step
            for step in job_run_steps(ROOT / ".github/workflows/ci.yml", "build")
            if step.name == "Install native prerequisites"
        )
        with tempfile.TemporaryDirectory(prefix="ci-native-owner-") as temporary:
            output = Path(temporary) / "arguments"
            environment = os.environ.copy()
            environment.update(NATIVE=json.dumps(native), NATIVE_OUTPUT=str(output))
            result = subprocess.run(
                [
                    shutil.which("bash"),
                    "-c",
                    'just() { printf "%s\\n" "$@" >> "$NATIVE_OUTPUT"; }\n' + step.script,
                ],
                cwd=temporary,
                env=environment,
                capture_output=True,
                text=True,
                timeout=10,
            )
            self.assertEqual(0, result.returncode, result.stderr)
            return output.read_text().splitlines() if output.exists() else []

    def test_the_build_owner_installs_only_its_own_records_closure(self) -> None:
        # The WSL2 archive producer that used to own this step is gone: the
        # guest consumes the run's Linux build, and that build's owner installs
        # exactly what the record it compiles needs.
        self.assertEqual(
            ["_ensure-native-libs", "own-dev"], self.provision_owner(["own-dev"])
        )
        self.assertEqual([], self.provision_owner([]))

    def test_native_names_are_passed_as_literal_arguments(self) -> None:
        self.assertEqual(
            ["_ensure-native-libs", "literal*name", "name with spaces"],
            self.provision("_package-ci.yml", "check", "ubuntu-latest",
                           {"ubuntu-latest": ["literal*name", "name with spaces"]}, []),
        )


MUTANTS = Path(__file__).resolve().parent / "fixtures" / "workflow_mutants"

#: Mutants that really are a different workflow, so a reader that reads them
#: differently is right rather than broken. Measured with `yaml.safe_load`
#: during Spike 1; `comment-in-block` and `trailing-blank` add a line to a
#: literal block scalar, `folded-scalar` rejoins one, and `alias-run` replaces
#: a step's script.
CHANGED_MUTANTS = frozenset({"folded-scalar", "comment-in-block", "trailing-blank", "alias-run"})

#: `{mutant: {probe: "raises" | "differs"}}`, every probe not named here
#: reading exactly what it reads on `base.yml`. A `differs` is only admissible
#: for a `CHANGED_MUTANTS` entry -- see the non-vacuity test below.
MUTANT_VERDICTS = {
    "reindent-job": {"job_run_steps(ci-gate)": "raises", "step_run_lines(gate)": "raises"},
    "folded-scalar": {"job_run_steps(preflight)": "raises", "step_script(toolchain)": "raises"},
    "anchor": {},
    "flow-mapping": {},
    "key-reorder": {"step_script(toolchain)": "raises"},
    "comment-in-block": {"job_run_steps(preflight)": "differs", "step_script(toolchain)": "differs"},
    "quoted-job-name": {
        "job_names": "raises",
        "job_run_steps(preflight)": "raises",
        "job_run_steps(ci-gate)": "raises",
    },
    "trailing-blank": {"job_run_steps(preflight)": "differs", "step_script(toolchain)": "differs"},
    "flow-step": {"job_run_steps(preflight)": "raises"},
    "alias-run": {"job_run_steps(preflight)": "raises"},
}

PROBES = {
    "job_names": lambda path: job_names(path),
    "job_run_steps(preflight)": lambda path: [
        (step.name, step.script, step.continue_on_error) for step in job_run_steps(path, "preflight")
    ],
    "job_run_steps(ci-gate)": lambda path: [
        (step.name, step.script, step.continue_on_error) for step in job_run_steps(path, "ci-gate")
    ],
    "step_script(toolchain)": lambda path: step_script(path, "Verify toolchain and required tooling"),
    "step_run_lines(gate)": lambda path: step_run_lines(path, "Fold the blocking jobs' results"),
}


class WorkflowReadingCorpusTests(unittest.TestCase):
    """`workflow_reading` against YAML it does not implement.

    The reader is an indentation heuristic, which is what keeps `scripts/ci`
    free of a PyYAML dependency the stock macOS and `build-linux` interpreters
    do not have. The trade is only sound while every layout it cannot handle
    makes it REFUSE: a reader that silently returns a shorter step list lets
    `WorkflowScopeStepTests` execute a shorter job and still pass, and that
    suite is the only thing that runs `ci.yml`'s scope step for real.

    Each fixture in `fixtures/workflow_mutants/` is one YAML edit to
    `base.yml`, itself cut verbatim from `ci.yml`. Spike 1 measured each
    against a `yaml.safe_load` implementation of the same API and found no
    case where the heuristic reads a mutant differently without refusing.
    That is the property these tests hold onto.
    """

    def read(self, mutant: str, probe: str):
        return PROBES[probe](MUTANTS / f"{mutant}.yml")

    def test_every_mutant_is_either_read_the_same_or_refused(self) -> None:
        for mutant, verdicts in MUTANT_VERDICTS.items():
            for probe in PROBES:
                with self.subTest(mutant=mutant, probe=probe):
                    expected = verdicts.get(probe, "same")
                    if expected == "raises":
                        with self.assertRaises(WorkflowLayoutError):
                            self.read(mutant, probe)
                        continue
                    read = self.read(mutant, probe)
                    base = PROBES[probe](MUTANTS / "base.yml")
                    if expected == "same":
                        self.assertEqual(base, read)
                    else:
                        self.assertNotEqual(base, read)

    def test_only_a_semantically_changed_mutant_may_be_read_differently(self) -> None:
        # The load-bearing half: a silent `differs` on a mutant that did not
        # change the workflow is the failure mode the heuristic is allowed to
        # have none of.
        for mutant, verdicts in MUTANT_VERDICTS.items():
            for probe, verdict in verdicts.items():
                if verdict == "differs":
                    self.assertIn(mutant, CHANGED_MUTANTS, f"{mutant}/{probe}")

    def test_the_corpus_on_disk_is_the_corpus_under_test(self) -> None:
        # Non-vacuity: a mutant added to the fixtures without a verdict, or a
        # verdict for a mutant nobody cut, would otherwise pass unnoticed.
        on_disk = {path.stem for path in MUTANTS.glob("*.yml")} - {"base"}
        self.assertEqual(set(MUTANT_VERDICTS), on_disk)

    def test_the_base_itself_reads_cleanly(self) -> None:
        base = MUTANTS / "base.yml"
        self.assertEqual(["preflight", "ci-gate"], job_names(base))
        for probe in PROBES:
            with self.subTest(probe=probe):
                self.assertTrue(PROBES[probe](base))


# ---------------------------------------------------------------------------
# Phase 2 pending contracts — fixes/2026-09-13-cicd-redundancies
# ---------------------------------------------------------------------------


def workflow_job_block(workflow: Path, job: str) -> list[str]:
    """One job's raw lines, from its `  <id>:` header to the next job's.

    Same fixed-layout reading as `job_run_steps`, but the whole block
    rather than only its `run:` steps: the guards these fixtures assert on are
    job keys (`if:`, `name:`, `strategy:`), not step scripts.
    """
    lines = workflow.read_text(encoding="utf-8").splitlines()
    start = lines.index(f"  {job}:")
    end = next(
        (
            index
            for index in range(start + 1, len(lines))
            if lines[index][:2] == "  " and lines[index][2:3] not in (" ", "")
        ),
        len(lines),
    )
    return lines[start:end]


def job_key(block: list[str], key: str) -> str | None:
    """A job-level key's value, with a `>-` folded scalar joined onto one line."""
    for index, line in enumerate(block):
        if not line.startswith(f"    {key}:"):
            continue
        value = line[len(f"    {key}:") :].strip()
        if value not in (">-", ">", "|"):
            return value
        folded = []
        for following in block[index + 1 :]:
            if following.strip() and len(following) - len(following.lstrip()) <= 4:
                break
            folded.append(following.strip())
        return " ".join(folded)
    return None


class PreflightPrerequisiteTests(unittest.TestCase):
    """AC1/spec section 3: preflight establishes prerequisites and runs no suite."""

    def test_preflight_runs_no_test_suite(self) -> None:
        offenders = [
            step.name
            for step in job_run_steps(WORKFLOW, "preflight")
            # `cargo nextest run`, not `cargo nextest`: the retained tooling
            # check runs `cargo nextest --version`, which is a prerequisite
            # probe rather than a suite.
            if "python3 scripts/ci/test_" in step.script
            or "cargo nextest run" in step.script
            or "pnpm " in step.script
        ]
        if offenders:
            raise AssertionError(
                "preflight must run no test suite; it still runs "
                f"{len(offenders)}: {offenders}"
            )

    def test_preflight_retains_its_prerequisite_steps(self) -> None:
        # NOT pending, and the non-vacuity guard for the fixture above: the
        # suites must be REMOVED, not the whole job emptied. Slimming preflight
        # by deleting its toolchain and canonical-recipe checks would satisfy
        # AC1 while deleting the bootstrap gate the fan-out depends on.
        names = {step.name for step in job_run_steps(WORKFLOW, "preflight")}
        for required in (
            "Verify toolchain and required tooling",
            "Cargo metadata without build acceleration",
            "Validate canonical area recipes",
        ):
            self.assertIn(required, names)


class EmptyMatrixGuardTests(unittest.TestCase):
    """AC14/R12: a matrix job is guarded by a scalar plan output, before expansion.

    The two halves are asserted separately on purpose. The static half reads
    `ci.yml`; the plan half computes the scalar outputs `ci.yml` reads for each
    of the three plan shapes the specification names, so a guard cannot be
    declared against an output the planner never produces.
    """

    MATRIX_JOBS = ("preflight", "area-ci")

    @classmethod
    def setUpClass(cls) -> None:
        cls.blocks = {job: workflow_job_block(WORKFLOW, job) for job in cls.MATRIX_JOBS}

    def test_neither_matrix_job_is_labelled_with_a_matrix_expression(self) -> None:
        # NOT pending: both jobs already omit `name:`, and the comments above
        # them explain why. Pinned so adding the scalar guard cannot come with
        # a display name that reaches the Checks tab as raw expression text.
        for job, block in self.blocks.items():
            with self.subTest(job=job):
                name = job_key(block, "name")
                self.assertIsNone(
                    name,
                    f"{job} is skippable as a whole, so a declared name would "
                    f"render unevaluated; got {name!r}",
                )

    def test_both_matrix_jobs_carry_a_scalar_guard(self) -> None:
        for job, block in self.blocks.items():
            guard = job_key(block, "if")
            if not guard or "needs.scope.outputs." not in guard:
                raise AssertionError(
                    f"{job} must be guarded by a scalar plan output before "
                    f"matrix expansion; its `if:` is {guard!r}"
                )
            self.assertNotIn(
                "matrix.", guard, f"{job}'s guard must not read the matrix it gates"
            )

    @classmethod
    def plans(cls) -> dict[str, dict]:
        """The three plan shapes spec section 8 names, from the real planner.

        Cached on the class: `load_metadata` shells out to `cargo metadata`.
        """
        if getattr(cls, "_plans", None) is None:
            from affected_scope import (  # noqa: PLC0415
                ENVIRONMENTS_CONFIG,
                apply_accepted_cells,
                calculate_scope,
                load_environments,
                load_metadata,
                package_ci_policy,
                workspace_packages,
            )

            metadata = load_metadata(ROOT)
            environments = load_environments(ENVIRONMENTS_CONFIG)
            policy = package_ci_policy(
                workspace_packages(metadata),
                runner_labels={entry["runner"] for entry in environments},
                root=ROOT,
            )

            def plan_for(files: list[str]) -> dict:
                return calculate_scope(files, ROOT, metadata, environments, policy)

            scheduled = plan_for([SOURCE_FILE])
            reused = apply_accepted_cells(
                scheduled,
                [
                    {
                        "package": cell["package"],
                        "environment": cell["environment"],
                        "gate": cell["gate"],
                        "outcome": "pass",
                        "evidence": {"origin": "prior-local"},
                    }
                    for cell in scheduled["cells"]
                ],
                None,
                None,
            )
            cls._plans = {
                "documentation-only": plan_for(["docs/topics/ci-cd.md"]),
                "scheduled": scheduled,
                "all-cells-reused": reused,
            }
        return cls._plans

    def test_a_documentation_only_plan_skips_both_matrix_jobs(self) -> None:
        plan = self.plans()["documentation-only"]
        # `has_packages` already resolves to false; the guard `preflight` is
        # gaining reads `preflight_os`, emitted through `jq -c` (ci.yml:267),
        # so `'[]'` is an exact comparison against an empty list.
        self.assertEqual([], [entry for entry in plan["packages"] if entry["gates"]])
        if plan["preflight_os"] != []:
            raise AssertionError(
                "a documentation-only plan must publish an empty preflight "
                f"matrix, got {plan['preflight_os']}"
            )

    def test_a_plan_with_no_executing_test_cell_still_fans_out_its_area(self) -> None:
        # NOT pending, and the counterweight to the fixture above (AC16): an
        # all-reused plan must NOT empty its matrices. `ci-reporting` reads the
        # area slices, so an area whose test cells are all reused still has to
        # fan out far enough to publish one.
        #
        # `lint` and `check` stay CI-origin by design — a local receipt carries
        # no JUnit evidence for them — so "every cell reused" is unreachable and
        # "every TEST cell reused" is the real shape.
        plan = self.plans()["all-cells-reused"]
        self.assertEqual(
            [],
            [
                cell
                for cell in plan["cells"]
                if cell["execution"] == "execute" and cell["gate"] not in ("lint", "check")
            ],
            "the fixture must present a plan with no test cell left to execute",
        )
        self.assertNotEqual(
            [],
            [entry for entry in plan["packages"] if entry["gates"]],
            "reuse resolves cells; it never unselects a package",
        )
        self.assertNotEqual(
            [], plan["preflight_os"], "reuse is not the documentation class"
        )

    def test_a_scheduled_plan_publishes_both_matrices(self) -> None:
        # NOT pending: the non-vacuity guard for the whole class. A planner
        # that emptied every matrix would satisfy the pending fixture above.
        plan = self.plans()["scheduled"]
        self.assertNotEqual([], plan["preflight_os"])
        self.assertNotEqual([], [entry for entry in plan["packages"] if entry["gates"]])


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Exercise the real `scripts/cross-check.sh` with stubbed SSH, without a host.

The script's whole product is the remote script it generates and ships, so that
is what these fixtures capture: `ssh` and `scp` are stubs that record their
arguments and stage the shipped files, and every assertion is about the bytes a
build host would actually have run.

One part is executed rather than read: `CrossCheckSourceIdentityTests` runs the
shipped prelude against a real clone, because "every host lands on `plan.head`
with a clean tree" is the precondition `ci-build produce` enforces, and a test
that only reads the generated text cannot tell whether it holds.

What cannot be asserted here — that the remote TIER works on a real machine —
is `just cross-check`'s own job. What can be asserted, and is, is that the
sequence it ships is the producer/consumer contract CI uses rather than a
second implementation of it.
"""

from __future__ import annotations

import base64
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from typing import Callable

sys.path.insert(0, str(Path(__file__).resolve().parent))

import plan_fixtures  # noqa: E402
import schema  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "cross-check.sh"
PACKAGE = "alpha"

#: What the stub planner writes wherever the real planner would write the head
#: it was given. Substituted at call time so `plan.head` is the revision the
#: script actually shipped.
HEAD_PLACEHOLDER = "b" * 40

#: Every SSH destination the script may be told about, and the OS each names.
HOSTS = {
    "linux": ("BUILD_LINUX", "build-linux"),
    "windows": ("BUILD_WIN", "build-win-native"),
    "wsl": ("BUILD_WSL", "build-win"),
    "macos": ("BUILD_MACOS", "build-mac"),
}

#: `ssh`/`scp` replacements: they append one JSON line per invocation and, for
#: `scp`, copy the shipped files where the fixture can read them. The one query
#: `ssh` answers is the host preparation: a Unix one (its script on stdin) is
#: EXECUTED under the fixture's remote home (and its `CODING_DIR`, when a test
#: sets one), so the layout and pruning policy is the shell's rather than the
#: stub's; a Windows one answers with the directory the fixture names.
SSH_STUB = """#!/usr/bin/env python3
import json, os, subprocess, sys
with open(os.environ['CROSS_CHECK_LOG'], 'a', encoding='utf-8') as log:
    log.write(json.dumps({'tool': 'ssh', 'argv': sys.argv[1:]}) + '\\n')
command = sys.argv[-1]
if '-EncodedCommand' in command:
    print('clone-dir: ' + os.environ['CROSS_CHECK_WIN_CLONE_DIR'])
elif command == 'bash -ls':
    env = {k: v for k, v in os.environ.items() if k != 'CODING_DIR'}
    env['HOME'] = os.environ['CROSS_CHECK_REMOTE_HOME']
    if os.environ.get('CROSS_CHECK_REMOTE_CODING_DIR'):
        env['CODING_DIR'] = os.environ['CROSS_CHECK_REMOTE_CODING_DIR']
    raise SystemExit(subprocess.run(['bash', '-c', command], env=env).returncode)
"""

#: What the Windows host answers for its clone directory, minus the offset.
WIN_CODING_DIR = "B:\\coding"


def origin_host() -> str:
    """This machine as the script names it: sanitized, lowercased, no `--`."""
    host = re.sub(r"[^A-Za-z0-9-]", "-", socket.gethostname().split(".")[0])
    return re.sub(r"-+", "-", host).lower()


def clone_offset(worktree: str = "main") -> str:
    """The `{host}--{worktree}` offset the script must derive for this machine."""
    return f"{origin_host()}--{re.sub(r'[^A-Za-z0-9._-]', '-', worktree).lower()}"

SCP_STUB = """#!/usr/bin/env python3
import json, os, shutil, sys
argv = sys.argv[1:]
with open(os.environ['CROSS_CHECK_LOG'], 'a', encoding='utf-8') as log:
    log.write(json.dumps({'tool': 'scp', 'argv': argv}) + '\\n')
files = []
skip = False
for arg in argv:
    if skip:
        skip = False
        continue
    if arg == '-o':
        skip = True
        continue
    if arg.startswith('-') or ':' in arg:
        continue
    files.append(arg)
shipped = os.environ['CROSS_CHECK_SHIPPED']
os.makedirs(shipped, exist_ok=True)
for path in files:
    if not os.path.isfile(path):
        # A fetch of something the remote never produced — the receipt path's
        # "no JUnit report" branch.
        raise SystemExit(1)
    shutil.copyfile(path, os.path.join(shipped, os.path.basename(path)))
"""


def planner_stub(package: str) -> str:
    """A planner that answers one `--all --resolved-plan` call with a fixed plan.

    The real planner needs `cargo metadata` over this repository; what the
    script consumes from it is `builds[]`, and `plan_fixtures` derives those
    exactly as `affected_scope` does.
    """
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
            "selection_reason": "selected by --all",
        }
        for environment in schema.ENVIRONMENTS
    ]
    plan = plan_fixtures.finalize_plan(
        {
            "schema_version": schema.RESOLVED_PLAN_SCHEMA_VERSION,
            "base": "a" * 40,
            "head": HEAD_PLACEHOLDER,
            "change_class": "full",
            "full_scope": True,
            "full_scope_gates": [],
            "areas": [
                {"area": "pkg", "selection_reason": "--all", "packages": [package]}
            ],
            "packages": [
                {
                    "package": package,
                    "area": "pkg",
                    "selection_reason": "--all",
                    "gates": ["lint", "L1"],
                    "targets": ["lib", "test"],
                    "tiers": ["L1"],
                    "test_args": "",
                    "check_args": f"-p {package}",
                    "l2_backends": [],
                    "runner_tools": [],
                    "archive_includes": [],
                    "sidecars": [],
                    "companion_suites": [],
                    "l1_include_slow": False,
                    "native": {},
                }
            ],
            "source_packages": [package],
            "reverse_dependencies": [],
            "environments": [
                {
                    "name": name,
                    "runner": "windows-latest" if name == "wsl2-ubuntu" else name,
                    "native_key": "ubuntu-latest" if name == "wsl2-ubuntu" else name,
                    "capabilities": {
                        "tmux": False,
                        "headless_browser": False,
                        "node_pnpm": False,
                        "archive_only": name == "wsl2-ubuntu",
                    },
                }
                for name in schema.ENVIRONMENTS
            ],
            "cells": cells,
            "accepted_evidence": [],
            "builds": [],
            "policy_gaps": [],
            "prohibited_cells": [],
            "job_estimate": 4,
            "preflight_os": ["ubuntu-latest"],
            "preflight_reason": "--all",
            "flags": {"ci_tooling": False},
        }
    )
    # The stub answers with the head it was ASKED for. `ci-build` binds every
    # archive to `plan.head`, so a fixture whose plan named a fixed revision
    # could not tell a correct orchestration from a broken one.
    return (
        "import sys\n"
        "argv = sys.argv[1:]\n"
        f"head = argv[argv.index('--head') + 1] if '--head' in argv else {HEAD_PLACEHOLDER!r}\n"
        f"print({schema.canonical(plan)!r}.replace({HEAD_PLACEHOLDER!r}, head))\n"
    )


class CrossCheckHarness(unittest.TestCase):
    """Runs the real script with stubbed `ssh`/`scp`; owns no assertions."""

    maxDiff = None

    def ship(
        self,
        os_name: str,
        *extra: str,
        prepare: Callable[[Path, Callable[..., str]], None] | None = None,
        replay: bool = False,
        coding_dir: str | None = None,
        worktree: str | None = None,
        remote_setup: Callable[[Path], None] | None = None,
    ) -> dict:
        """Run the script for one OS and return everything it shipped.

        `prepare` is handed the fixture checkout and its `git` helper after the
        base commit is pushed, so a test can leave the tree dirty, ahead of the
        base, or both. `replay` then EXECUTES the shipped remote prelude against
        a fresh clone of the fixture's origin — the identity boundary itself,
        not an assertion about the text that implements it — and reports the
        revision that host ended up on. `coding_dir` is the remote host's
        `CODING_DIR`; `worktree` runs the script from a linked worktree of that
        name instead of the main checkout. `remote_setup` is handed the remote
        host's coding dir before the run, to plant existing clones there.
        """
        with tempfile.TemporaryDirectory(prefix="cross-check-") as temporary:
            root = Path(temporary)
            repo = root / "repo"
            bin_dir = root / "bin"
            shipped = root / "shipped"
            bin_dir.mkdir()
            (repo / "scripts" / "ci").mkdir(parents=True)
            shutil.copyfile(SCRIPT, repo / "scripts" / "cross-check.sh")
            (repo / "scripts" / "ci" / "affected_scope.py").write_text(
                planner_stub(PACKAGE), encoding="utf-8"
            )
            (repo / "README.md").write_text("fixture\n", encoding="utf-8")

            def git(*args: str, cwd: Path = repo) -> str:
                return subprocess.run(
                    ["git", *args],
                    cwd=cwd,
                    check=True,
                    capture_output=True,
                    text=True,
                    env={
                        **os.environ,
                        "GIT_AUTHOR_NAME": "t",
                        "GIT_AUTHOR_EMAIL": "t@example.com",
                        "GIT_COMMITTER_NAME": "t",
                        "GIT_COMMITTER_EMAIL": "t@example.com",
                        "GIT_CONFIG_GLOBAL": "/dev/null",
                    },
                ).stdout.strip()

            git("init", "--quiet", "--initial-branch=main")
            git("add", "-A")
            git("commit", "--quiet", "-m", "fixture")
            origin = root / "origin.git"
            subprocess.run(
                ["git", "init", "--bare", "--quiet", str(origin)], check=True
            )
            git("remote", "add", "origin", str(origin))
            git("push", "--quiet", "origin", "main")
            git("fetch", "--quiet", "origin")
            if prepare is not None:
                prepare(repo, git)
            checkout = repo
            if worktree is not None:
                checkout = root / worktree
                git("worktree", "add", "--quiet", "-b", worktree, str(checkout))

            remote_coding = Path(coding_dir) if coding_dir else root / "remote-home" / "coding"
            if remote_setup is not None:
                remote_coding.mkdir(parents=True, exist_ok=True)
                remote_setup(remote_coding)

            for name, body in (("ssh", SSH_STUB), ("scp", SCP_STUB)):
                path = bin_dir / name
                path.write_text(body, encoding="utf-8")
                path.chmod(0o755)

            log = root / "calls.jsonl"
            environment = {
                **os.environ,
                "PATH": str(bin_dir) + os.pathsep + os.environ["PATH"],
                "CROSS_CHECK_LOG": str(log),
                "CROSS_CHECK_SHIPPED": str(shipped),
                "CROSS_CHECK_REMOTE_HOME": str(root / "remote-home"),
                "CROSS_CHECK_REMOTE_CODING_DIR": coding_dir or "",
                "CROSS_CHECK_WIN_CLONE_DIR": WIN_CODING_DIR
                + "\\"
                + clone_offset(worktree or "main"),
                HOSTS[os_name][0]: HOSTS[os_name][1],
            }
            # The developer's own layout is not the remote's.
            environment.pop("CODING_DIR", None)
            for other, (variable, _) in HOSTS.items():
                if other != os_name:
                    environment.pop(variable, None)
            result = subprocess.run(
                [
                    "/bin/bash" if sys.platform == "darwin" else "bash",
                    str(checkout / "scripts" / "cross-check.sh"),
                    "--os",
                    os_name,
                    PACKAGE,
                    *extra,
                ],
                cwd=checkout,
                env=environment,
                capture_output=True,
                text=True,
                timeout=120,
            )
            calls = [
                json.loads(line)
                for line in (log.read_text().splitlines() if log.is_file() else [])
            ]
            remote = ""
            for candidate in sorted(shipped.glob("cross-check-*")):
                if candidate.suffix in (".sh", ".ps1"):
                    remote = candidate.read_text()
            plan = ""
            for candidate in sorted(shipped.glob("*-plan.json")):
                plan = candidate.read_text()
            clone_dir = ""
            prefix = f"clone: {HOSTS[os_name][1]}:"
            for line in result.stdout.splitlines():
                if line.startswith(prefix):
                    clone_dir = line[len(prefix):]
            report = {
                "stdout": result.stdout,
                "clone_dir": clone_dir,
                "remote_entries": self.settled_entries(remote_coding),
                "stderr": result.stderr,
                "returncode": result.returncode,
                "calls": calls,
                "remote": remote,
                "plan": plan,
                "bundled": sorted(p.name for p in shipped.glob("*.bundle")),
                # The developer's own repository, after the run.
                "local_branch": git("rev-parse", "--abbrev-ref", "HEAD"),
                "local_status": git("status", "--porcelain"),
                "local_stash": git("stash", "list"),
                "local_refs": git("for-each-ref", "--format=%(refname)"),
            }
            if replay:
                report.update(self.replay_prelude(Path(clone_dir), shipped, remote))
            return report

    @staticmethod
    def settled_entries(coding: Path) -> list[str]:
        """The remote coding dir once this origin's background deletes finish."""
        if not coding.is_dir():
            return []
        mine = f".cross-check-trash--{origin_host()}--"
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            if not any(p.name.startswith(mine) for p in coding.iterdir()):
                break
            time.sleep(0.1)
        return sorted(p.name for p in coding.iterdir())

    def replay_prelude(self, clone_dir: Path, shipped: Path, remote: str) -> dict:
        """Run the shipped remote prelude on a fresh clone, as a host would.

        Everything up to and including the checkout, which is the whole of the
        source-identity contract: after this the host either IS `plan.head` with
        a clean tree, or `ci-build produce` refuses it.
        """
        marker = "\ngit checkout --quiet --detach '"
        self.assertIn(marker, remote, "the prelude must end by checking out one revision")
        prelude = remote[: remote.index("\n", remote.index(marker) + 1)] + "\n"

        # What `scp` delivers before the remote script runs, into the directory
        # the host resolved.
        for artifact in shipped.iterdir():
            shutil.copyfile(artifact, clone_dir / artifact.name)

        run = subprocess.run(
            ["/bin/bash" if sys.platform == "darwin" else "bash", "-c", prelude],
            cwd=clone_dir,
            env=os.environ,
            capture_output=True,
            text=True,
            timeout=120,
        )
        host_repo = clone_dir / "rusty-biscuit"
        if run.returncode != 0 or not host_repo.is_dir():
            return {
                "replay_returncode": run.returncode,
                "replay_stderr": run.stderr,
                "host_head": "",
                "host_dirty": "",
                "host_tracked": [],
                "host_readme": "",
            }

        def host_git(*args: str) -> str:
            return subprocess.run(
                ["git", *args],
                cwd=host_repo,
                check=True,
                capture_output=True,
                text=True,
                env={**os.environ, "GIT_CONFIG_GLOBAL": "/dev/null"},
            ).stdout.strip()

        return {
            "replay_returncode": run.returncode,
            "replay_stderr": run.stderr,
            "host_head": host_git("rev-parse", "HEAD"),
            "host_dirty": host_git("status", "--porcelain"),
            "host_tracked": host_git("ls-files").splitlines(),
            "host_readme": (host_repo / "README.md").read_text(encoding="utf-8"),
        }

    def expected_key(self, os_name: str) -> str:
        return plan_fixtures._key(PACKAGE, plan_fixtures.PRODUCER_OF[
            {"linux": "ubuntu-latest", "wsl": "wsl2-ubuntu",
             "macos": "macos-latest", "windows": "windows-latest"}[os_name]
        ])


class CrossCheckShipTests(CrossCheckHarness):
    """What a build host is actually handed, per OS and per mode."""

    # -- the Unix hosts -----------------------------------------------------

    def test_each_unix_host_is_shipped_the_producer_consumer_sequence(self) -> None:
        for os_name in ("linux", "wsl", "macos"):
            with self.subTest(os=os_name):
                run = self.ship(os_name)
                self.assertEqual(0, run["returncode"], run["stdout"] + run["stderr"])
                remote = run["remote"]
                head = json.loads(run["plan"])["head"]
                key = self.expected_key(os_name)
                producer = {
                    "linux": "ubuntu-latest",
                    "wsl": "ubuntu-latest",
                    "macos": "macos-latest",
                }[os_name]
                environment = {
                    "linux": "ubuntu-latest",
                    "wsl": "wsl2-ubuntu",
                    "macos": "macos-latest",
                }[os_name]

                # The producer, named by the plan rather than by this script.
                self.assertIn(
                    f"produce --plan \"$plan\" --producer '{producer}' --key '{key}'",
                    remote,
                )
                # The verifier travels with what it verifies.
                self.assertIn('cp "$tool" "$out/tools/"', remote)
                # The transfer, into a directory the consumer owns.
                self.assertIn('cp -R "$out" "$consume/build"', remote)
                # A different checkout at the SAME revision the host is on,
                # named explicitly: the consumer's tree is the tree the archive
                # was bound to.
                self.assertIn(
                    f'git worktree add --detach --quiet "$src" \'{head}\'', remote
                )
                self.assertIn("mv target target.hold", remote)
                # Verification BEFORE the tier, and the tier in archive mode.
                verify = remote.index("just _ci_build_verify")
                test = remote.index("just _test ")
                self.assertLess(verify, test, "nothing may be extracted before the verdict")
                self.assertIn(
                    f"just _ci_build_verify \"$consume/build\" "
                    f"'build-{PACKAGE}-{producer}-{key}' '{environment}' \"$plan\"",
                    remote,
                )
                self.assertIn(
                    "--archive-file \"$archive_file\" --workspace-remap \"$src\"", remote
                )
                self.assertIn("BISCUIT_NEXTEST_BIN='cargo-nextest nextest'", remote)
                # The markers the receipt is assembled from.
                for marker in (
                    "cross-check-tree:",
                    "cross-check-dirty:",
                    "cross-check-key:",
                    "cross-check-digest:",
                    "cross-check-exit:",
                    "cross-check-duration:",
                ):
                    self.assertIn(marker, remote)
                # And the plan it all reads from was shipped beside it.
                self.assertIn('"builds"', run["plan"])
                self.assertIn(key, run["plan"])

    def test_no_unix_consumer_can_reach_a_compiler_for_the_package(self) -> None:
        # The producer compiles; the consumer must not. `cargo` appears exactly
        # twice — the producer tool and the archive — and never as a test runner.
        for os_name in ("linux", "wsl", "macos"):
            with self.subTest(os=os_name):
                remote = self.ship(os_name)["remote"]
                self.assertNotIn("cargo nextest run", remote)
                self.assertNotIn("cargo test", remote)
                self.assertIn("cargo build --release --manifest-path scripts/Cargo.toml", remote)

    def test_the_run_script_is_not_handed_to_a_login_shell(self) -> None:
        """A login shell's status is its `~/.bash_logout`'s, not the script's.

        On the WSL guest that file ends with a `clear_console -q` that fails
        without a tty, so `bash -l <script>` returned 1 for a run whose every
        test passed (diagnosed 2026-09-22). `bash -lc` still sources the
        profile, so `cargo` is on PATH, and the inner shell inherits it.
        """
        for os_name in ("linux", "wsl", "macos"):
            with self.subTest(os=os_name):
                run = self.ship(os_name)
                commands = [c["argv"][-1] for c in run["calls"] if c["tool"] == "ssh"]
                launch = [c for c in commands if "cross-check-" in c and ".sh" in c]
                self.assertTrue(launch, commands)
                for command in launch:
                    self.assertIn("bash -lc 'bash \"$0\"'", command)
                    self.assertNotIn("bash -l /", command)

    def test_every_ssh_and_scp_invocation_is_non_interactive(self) -> None:
        run = self.ship("linux")
        self.assertTrue(run["calls"], "the script must reach a host")
        for call in run["calls"]:
            self.assertIn(
                "BatchMode=yes",
                call["argv"],
                f"{call['tool']} must never be able to prompt: {call['argv']}",
            )

    # -- native Windows -----------------------------------------------------

    def test_native_windows_is_shipped_the_same_sequence_in_powershell(self) -> None:
        run = self.ship("windows")
        self.assertEqual(0, run["returncode"], run["stdout"] + run["stderr"])
        remote = run["remote"]
        key = self.expected_key("windows")
        self.assertIn(
            f"produce --plan $plan --producer 'windows-latest' --key '{key}'", remote
        )
        head = json.loads(run["plan"])["head"]
        self.assertIn(f"git worktree add --detach --quiet $src '{head}'", remote)
        self.assertIn('Move-Item "$repo\\target" "$repo\\target.hold"', remote)
        verify = remote.index("just _ci_build_verify")
        test = remote.index("just _test ")
        self.assertLess(verify, test)
        # Every path handed to a `just` recipe goes through `_native_path`: a
        # backslash in a recipe's `*args` is eaten by bash before nextest sees it.
        self.assertIn("just _native_path $src", remote)
        # `scripts` is a root-workspace member: its binaries land in the
        # workspace target dir, which is also the one the run hides.
        self.assertIn('$tool = "$repo\\target\\release\\ci-build.exe"', remote)
        self.assertIn('Move-Item "$repo\\target" "$repo\\target.hold"', remote)
        # Git for Windows caps a path at 260 characters unless told otherwise,
        # and this repository is deep enough to exceed that from the clone.
        self.assertIn("git config core.longpaths true", remote)
        self.assertIn("--archive-file $archiveFile --workspace-remap $nativeSrc", remote)
        self.assertNotIn("cargo nextest run", remote)
        self.assertIn("cross-check-key:", remote)
        self.assertPowerShellExitIsNotTheFunctionsOutput(remote)

    def assertPowerShellExitIsNotTheFunctionsOutput(self, remote: str) -> None:
        """A PowerShell function's output stream is not its return value.

        Every native command inside `Invoke-CrossCheck` writes to stdout, and
        that stdout becomes part of the function's output — so
        `$code = Invoke-CrossCheck` binds an array whose first element is a git
        message, and a failing run exits 0 and is summarized as a pass.
        Measured on `build-win-native`, 2026-09-14: a tier that exited 1 was
        reported `windows  pass`.
        """
        self.assertNotIn(
            "$code = Invoke-CrossCheck",
            remote,
            "the exit code must not be bound to the function's output stream",
        )
        self.assertIn("Invoke-CrossCheck | Out-Host", remote)
        self.assertIn("exit $script:code", remote)
        self.assertNotIn("return $LASTEXITCODE", remote)

    # -- the native escape hatch -------------------------------------------

    def test_a_build_flag_selects_the_native_path_and_ships_no_plan(self) -> None:
        """Archive mode cannot honor an override without changing the key.

        The declared feature arguments ARE the archive's, so a caller who wants
        others gets the older path — announced, and carrying nothing a receipt
        could be built from.
        """
        run = self.ship("linux", "--features", "terminal-tests", "level2_")
        self.assertEqual(0, run["returncode"], run["stdout"] + run["stderr"])
        self.assertIn("mode: native", run["stdout"])
        self.assertIn(
            f"cargo nextest run -p '{PACKAGE}' --no-fail-fast", run["remote"]
        )
        self.assertNotIn("ci-build produce", run["remote"])
        self.assertEqual("", run["plan"], "a native run ships no plan")

    def test_the_native_windows_path_reports_its_exit_code_the_same_way(self) -> None:
        # The escape hatch shares the wrapper, so it shares the hazard.
        self.assertPowerShellExitIsNotTheFunctionsOutput(
            self.ship("windows", "--all-features")["remote"]
        )

    def test_archive_mode_is_announced_with_the_key_each_host_will_run(self) -> None:
        run = self.ship("wsl")
        self.assertIn("mode: archive", run["stdout"])
        self.assertIn(
            f"wsl      {self.expected_key('wsl')} produced on ubuntu-latest, "
            "consumed as wsl2-ubuntu",
            run["stdout"],
        )


class CrossCheckSourceIdentityTests(CrossCheckHarness):
    """The revision every host tests, executed rather than asserted about.

    `ci-build produce` refuses any checkout whose `HEAD` is not `plan.head` or
    whose tracked tree is dirty, so the orchestration has exactly one job here:
    put every host on that revision, clean. These fixtures run the shipped
    prelude on a real clone and read back where it landed, for the two trees
    that used to arrive as a base checkout plus an unapplied-or-dirty patch.
    """

    @staticmethod
    def dirty_tree(repo: Path, git: Callable[..., str]) -> None:
        """Uncommitted tracked edits and an untracked file, as a developer has."""
        (repo / "README.md").write_text("fixture\nlocal edit\n", encoding="utf-8")
        (repo / "new-file.txt").write_text("untracked\n", encoding="utf-8")

    @staticmethod
    def ahead_tree(repo: Path, git: Callable[..., str]) -> None:
        """Committed but unpushed, and dirty on top of that."""
        (repo / "README.md").write_text("fixture\ncommitted\n", encoding="utf-8")
        git("add", "-A")
        git("commit", "--quiet", "-m", "local work")
        (repo / "README.md").write_text("fixture\ncommitted\nand dirty\n", encoding="utf-8")
        (repo / "new-file.txt").write_text("untracked\n", encoding="utf-8")

    def assertHostIsThePlannedRevision(self, run: dict) -> None:
        self.assertEqual(0, run["returncode"], run["stdout"] + run["stderr"])
        self.assertEqual(
            0, run["replay_returncode"], run.get("replay_stderr", "")
        )
        head = json.loads(run["plan"])["head"]
        self.assertEqual(
            head,
            run["host_head"],
            "the host must check out the revision the plan names, or `ci-build "
            "produce` refuses it as build-source-mismatch",
        )
        self.assertEqual(
            "",
            run["host_dirty"],
            "a tracked-dirty host is refused as build-source-mismatch",
        )

    def test_a_dirty_tree_reaches_the_host_as_one_clean_revision(self) -> None:
        run = self.ship("linux", prepare=self.dirty_tree, replay=True)
        self.assertHostIsThePlannedRevision(run)
        # The local content is there, and as COMMITTED content: the untracked
        # file is tracked on the host, so nothing about the tree is dirty.
        self.assertEqual("fixture\nlocal edit\n", run["host_readme"])
        self.assertIn("new-file.txt", run["host_tracked"])
        self.assertEqual(1, len(run["bundled"]), run["bundled"])

    def test_an_ahead_and_dirty_tree_reaches_the_host_as_one_clean_revision(self) -> None:
        run = self.ship("linux", prepare=self.ahead_tree, replay=True)
        self.assertHostIsThePlannedRevision(run)
        self.assertEqual("fixture\ncommitted\nand dirty\n", run["host_readme"])
        self.assertIn("new-file.txt", run["host_tracked"])

    def test_a_clean_tree_at_the_base_names_the_base_and_ships_no_bundle(self) -> None:
        run = self.ship("linux", replay=True)
        self.assertHostIsThePlannedRevision(run)
        self.assertEqual([], run["bundled"], "there is nothing to ship")

    def test_the_native_escape_hatch_lands_on_the_same_revision(self) -> None:
        # Native mode compiles on the host instead of consuming an archive, but
        # it must still test the tree the caller has, not the base.
        run = self.ship("linux", "--all-features", prepare=self.dirty_tree, replay=True)
        self.assertEqual(0, run["returncode"], run["stdout"] + run["stderr"])
        self.assertEqual(0, run["replay_returncode"], run.get("replay_stderr", ""))
        self.assertEqual("", run["host_dirty"])
        self.assertEqual("fixture\nlocal edit\n", run["host_readme"])

    def test_no_host_is_ever_asked_to_apply_a_patch(self) -> None:
        """`git apply` over the base is what `build-source-mismatch` refuses."""
        for os_name in ("linux", "wsl", "macos", "windows"):
            with self.subTest(os=os_name):
                remote = self.ship(os_name, prepare=self.dirty_tree)["remote"]
                self.assertNotIn("git apply", remote)
                self.assertIn("git fetch --quiet", remote)

    def test_shipping_leaves_the_developers_repository_alone(self) -> None:
        """The commit is a throwaway object, not a change to the working repo."""
        run = self.ship("linux", prepare=self.dirty_tree)
        self.assertEqual("main", run["local_branch"])
        self.assertEqual("", run["local_stash"], "the shared stash stack is not ours")
        # Still UNSTAGED and still untracked: the throwaway commit was written
        # through a scratch index, so the real one never saw either file.
        self.assertIn("M README.md", run["local_status"])
        self.assertIn("?? new-file.txt", run["local_status"])
        self.assertNotIn(
            "refs/cross-check/",
            run["local_refs"],
            "the ref the bundle was cut from must not outlive the run",
        )


class CrossCheckLayoutTests(CrossCheckHarness):
    """Where a host keeps the clone: the HOST's coding dir, one per checkout.

    Nothing about a build host's disk may be assumed locally. The coding dir is
    the host's `CODING_DIR`, else `~/coding`; under it each originating
    checkout gets `{host}-{worktree | main}`, so two checkouts never share a
    clone, its lock, or its target dir.
    """

    def assertNoFixedLayout(self, run: dict) -> None:
        shipped_to = [a for c in run["calls"] for a in c["argv"]]
        for text in [run["remote"], *shipped_to]:
            self.assertNotIn("ci-verification", text)
            self.assertNotIn("W:", text)

    def test_a_unix_host_defaults_to_its_home_coding_dir(self) -> None:
        for os_name in ("linux", "wsl", "macos"):
            with self.subTest(os=os_name):
                run = self.ship(os_name)
                self.assertEqual(0, run["returncode"], run["stdout"] + run["stderr"])
                self.assertTrue(
                    run["clone_dir"].endswith(f"/remote-home/coding/{clone_offset()}"),
                    run["clone_dir"],
                )
                self.assertIn(f"base={run['clone_dir']}\n", run["remote"])
                self.assertIn('repo="$base/rusty-biscuit"', run["remote"])
                scp = [c for c in run["calls"] if c["tool"] == "scp"]
                self.assertTrue(scp[0]["argv"][-1].endswith(f":{run['clone_dir']}/"))
                self.assertNoFixedLayout(run)

    def test_a_unix_host_honors_its_coding_dir(self) -> None:
        with tempfile.TemporaryDirectory(prefix="coding-dir-") as coding:
            run = self.ship("linux", coding_dir=coding)
            self.assertEqual(0, run["returncode"], run["stdout"] + run["stderr"])
            self.assertEqual(
                os.path.realpath(Path(coding) / clone_offset()),
                os.path.realpath(run["clone_dir"]),
            )

    def test_a_linked_worktree_gets_its_own_clone(self) -> None:
        run = self.ship("linux", worktree="feat-thing", replay=True)
        self.assertEqual(0, run["returncode"], run["stdout"] + run["stderr"])
        self.assertTrue(
            run["clone_dir"].endswith(f"/coding/{clone_offset('feat-thing')}"),
            run["clone_dir"],
        )
        self.assertEqual(0, run["replay_returncode"], run.get("replay_stderr", ""))

    def test_windows_uses_the_directory_the_host_resolves(self) -> None:
        run = self.ship("windows")
        self.assertEqual(0, run["returncode"], run["stdout"] + run["stderr"])
        expected = f"{WIN_CODING_DIR}\\{clone_offset()}"
        self.assertEqual(expected, run["clone_dir"])
        # Resolved on the host from its own environment, never assumed here.
        lookup = next(c for c in run["calls"] if "-EncodedCommand" in c["argv"][-1])
        encoded = lookup["argv"][-1].split("-EncodedCommand ", 1)[1]
        command = base64.b64decode(encoded).decode("utf-16-le")
        self.assertIn("$env:CODING_DIR", command)
        self.assertIn("Join-Path $env:USERPROFILE 'coding'", command)
        self.assertIn(f"'{clone_offset()}'", command)
        self.assertIn(f"$base = '{expected}'", run["remote"])
        self.assertIn('$repo = "$base\\rusty-biscuit"', run["remote"])
        scp = [c for c in run["calls"] if c["tool"] == "scp"]
        self.assertTrue(scp[0]["argv"][-1].endswith(":" + expected.replace("\\", "/") + "/"))
        self.assertNoFixedLayout(run)


def plant_clone(coding: Path, name: str, locked: bool = False) -> None:
    """A clone directory as an earlier run (or something else) left it."""
    clone = coding / name
    (clone / "rusty-biscuit" / "target").mkdir(parents=True)
    if locked:
        (clone / ".cross-check.lock").mkdir()


class CrossCheckStaleCloneTests(CrossCheckHarness):
    """Clones whose origin worktree is gone are removed before any leg runs.

    A clone is `<host>--<worktree>`, and the host part never contains `--`,
    so the first `--` always separates the two and only this origin host's
    clones are candidates.
    """

    def test_a_clone_for_a_removed_worktree_is_deleted(self) -> None:
        stale = f"{origin_host()}--gone"
        run = self.ship("linux", remote_setup=lambda c: plant_clone(c, stale))
        self.assertEqual(0, run["returncode"], run["stdout"] + run["stderr"])
        self.assertNotIn(stale, run["remote_entries"])
        self.assertIn(
            f"removing stale clone build-linux:{run['clone_dir'].rsplit('/', 1)[0]}/{stale} in the background",
            run["stdout"],
        )
        # Renamed to trash, then deleted detached: nothing of it remains.
        self.assertEqual([clone_offset()], run["remote_entries"])

    def test_trash_a_previous_delete_left_behind_is_deleted_again(self) -> None:
        trash = f".cross-check-trash--{origin_host()}--gone-1-1"
        run = self.ship("linux", remote_setup=lambda c: plant_clone(c, trash))
        self.assertEqual(0, run["returncode"], run["stdout"] + run["stderr"])
        self.assertEqual([clone_offset()], run["remote_entries"])

    def test_live_worktrees_keep_their_clones(self) -> None:
        def extra_worktree(repo: Path, git: Callable[..., str]) -> None:
            git("worktree", "add", "--quiet", "-b", "other", str(repo.parent / "other--x"))

        live = clone_offset("other--x")
        run = self.ship("linux", prepare=extra_worktree, remote_setup=lambda c: plant_clone(c, live))
        self.assertEqual(0, run["returncode"], run["stdout"] + run["stderr"])
        self.assertIn(live, run["remote_entries"])
        self.assertNotIn("stale clone", run["stdout"])

    def test_a_worktree_whose_directory_is_gone_counts_as_removed(self) -> None:
        # `git worktree list` still names it, marked `prunable`.
        def vanished_worktree(repo: Path, git: Callable[..., str]) -> None:
            path = repo.parent / "vanished"
            git("worktree", "add", "--quiet", "-b", "vanished", str(path))
            shutil.rmtree(path)

        stale = clone_offset("vanished")
        run = self.ship("linux", prepare=vanished_worktree, remote_setup=lambda c: plant_clone(c, stale))
        self.assertNotIn(stale, run["remote_entries"])

    def test_only_this_hosts_clones_are_candidates(self) -> None:
        host = origin_host()
        others = (
            f"{host}-notes",  # a real project that merely shares the prefix
            f"{host}-mini--main",  # another origin whose name extends this one
            f"other--{host}",
            "rusty-biscuit",
            ".cross-check-trash--other--gone-1-1",  # another origin's trash
        )

        def plant(coding: Path) -> None:
            for name in others:
                plant_clone(coding, name)

        run = self.ship("linux", remote_setup=plant)
        self.assertEqual(0, run["returncode"], run["stdout"] + run["stderr"])
        for name in others:
            self.assertIn(name, run["remote_entries"])
        self.assertNotIn("stale clone", run["stdout"])

    def test_a_locked_stale_clone_is_kept_and_reported(self) -> None:
        stale = f"{origin_host()}--gone"
        run = self.ship("linux", remote_setup=lambda c: plant_clone(c, stale, locked=True))
        self.assertEqual(0, run["returncode"], run["stdout"] + run["stderr"])
        self.assertIn(stale, run["remote_entries"])
        self.assertIn("its lock is held", run["stdout"])

    def test_pruning_happens_before_anything_is_shipped(self) -> None:
        run = self.ship("linux")
        tools = [c["tool"] for c in run["calls"]]
        self.assertEqual("ssh", tools[0])
        self.assertEqual(["bash -ls"], run["calls"][0]["argv"][-1:])
        self.assertIn("scp", tools[1:])

    def test_windows_prunes_under_the_same_rules(self) -> None:
        run = self.ship("windows")
        lookup = next(c for c in run["calls"] if "-EncodedCommand" in c["argv"][-1])
        command = base64.b64decode(
            lookup["argv"][-1].split("-EncodedCommand ", 1)[1]
        ).decode("utf-16-le")
        self.assertIn("$live = @('main')", command)
        self.assertIn(f"$prefix = '{origin_host()}--'", command)
        self.assertIn("if ($live -contains $entry.Name.Substring($prefix.Length)) { continue }", command)
        self.assertIn(".cross-check.lock", command)
        # Renamed at once, deleted outside the SSH session's job, past MAX_PATH.
        self.assertIn("Rename-Item -LiteralPath $dir -NewName $trash", command)
        self.assertIn("Invoke-CimMethod -ClassName Win32_Process -MethodName Create", command)
        self.assertIn('''rd /s /q "\\\\?\\''', command)
        self.assertIn(f'".cross-check-trash--$prefix"', command)
        self.assertNotIn("Remove-Item", command)

if __name__ == "__main__":
    unittest.main()

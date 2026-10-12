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

import affected_scope  # noqa: E402
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
host = sys.argv[-2] if len(sys.argv) > 1 else ''
runs = json.loads(os.environ.get('CROSS_CHECK_REMOTE_RUNS', '{}'))
if host in runs and 'cross-check-' in command and ('.sh' in command or '.ps1' in command):
    # The remote run itself: replay what a real host printed and exited with.
    import signal, time
    run = runs[host]
    sys.stdout.write(run['output'].replace('{TREE}', os.environ.get('CROSS_CHECK_TREE', '')))
    sys.stdout.flush()
    if run.get('interrupt'):
        # Ctrl-C at the terminal reaches the whole foreground process group.
        signal.signal(signal.SIGINT, signal.SIG_DFL)
        os.killpg(os.getpgrp(), signal.SIGINT)
        time.sleep(30)
    raise SystemExit(run.get('exit', 0))
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
positional = []
skip = False
for arg in argv:
    if skip:
        skip = False
        continue
    if arg == '-o':
        skip = True
        continue
    if not arg.startswith('-'):
        positional.append(arg)
if len(positional) >= 2 and ':' in positional[0] and ':' not in positional[-1]:
    # A fetch (`scp -r host:dir local/`): serve the directory the fixture names
    # for that host, or fail the way a remote that left nothing behind does.
    host, remote = positional[0].split(':', 1)
    source = json.loads(os.environ.get('CROSS_CHECK_FETCH_DIRS', '{}')).get(host)
    if not source:
        raise SystemExit(1)
    shutil.copytree(source, os.path.join(positional[-1], os.path.basename(remote.rstrip('/'))))
    raise SystemExit(0)
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


def planner_stub(package: str, delegate: bool = False) -> str:
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
            # The real producers, so the plan stays valid for the publisher that
            # reads it back (`schema.validate_resolved_plan`).
            "change_inventory": affected_scope.change_inventory(
                [f"{package}/src/lib.rs"], False
            ),
            "archive_guard": plan_fixtures.archive_guard([f"{package}/src/lib.rs"]),
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
    body = (
        "import sys\n"
        "argv = sys.argv[1:]\n"
        f"head = argv[argv.index('--head') + 1] if '--head' in argv else {HEAD_PLACEHOLDER!r}\n"
        f"print({schema.canonical(plan)!r}.replace({HEAD_PLACEHOLDER!r}, head))\n"
    )
    if not delegate:
        return body
    # The publisher imports the real module by this name. Run as a script this
    # file is the planner stub; imported, it hands over to the real planner so
    # the stub and the real publisher can share one `scripts/ci` directory.
    return (
        "import sys\n"
        "if __name__ != '__main__':\n"
        "    import importlib.util\n"
        f"    spec = importlib.util.spec_from_file_location('affected_scope', {str(ROOT / 'scripts' / 'ci' / 'affected_scope.py')!r})\n"
        "    module = importlib.util.module_from_spec(spec)\n"
        "    sys.modules['affected_scope'] = module\n"
        "    spec.loader.exec_module(module)\n"
        "else:\n"
        + "".join(f"    {line}\n" for line in body.splitlines())
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
        publisher: bool = False,
        declared: tuple[str, ...] | None = None,
        os_arg: str | None = None,
        remote_runs: dict[str, dict] | None = None,
        fetch_dirs: dict[str, Path] | None = None,
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

        `publisher` installs the REAL evidence tooling beside the stub planner,
        so the leg-end publication runs for real against the fixture's repository
        and bare origin; `remote_runs` (SSH destination -> `{output, exit,
        interrupt}`) replays what a rig printed for its run script, and
        `fetch_dirs` (destination -> directory) is what `scp -r` fetches from it.
        `declared` names every OS the environment declares and `os_arg` is the
        `--os` value (default: `os_name`).
        """
        with tempfile.TemporaryDirectory(prefix="cross-check-") as temporary:
            root = Path(temporary)
            repo = root / "repo"
            bin_dir = root / "bin"
            shipped = root / "shipped"
            bin_dir.mkdir()
            (repo / "scripts" / "ci").mkdir(parents=True)
            shutil.copyfile(SCRIPT, repo / "scripts" / "cross-check.sh")
            if publisher:
                for module in (ROOT / "scripts" / "ci").glob("*.py"):
                    if module.name != "affected_scope.py" and not module.name.startswith("test_"):
                        shutil.copyfile(module, repo / "scripts" / "ci" / module.name)
            (repo / "scripts" / "ci" / "affected_scope.py").write_text(
                planner_stub(PACKAGE, delegate=publisher), encoding="utf-8"
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
                "CROSS_CHECK_TREE": git("rev-parse", "HEAD^{tree}"),
                "CROSS_CHECK_REMOTE_RUNS": json.dumps(remote_runs or {}),
                "CROSS_CHECK_FETCH_DIRS": json.dumps(
                    {host: str(path) for host, path in (fetch_dirs or {}).items()}
                ),
                "BISCUIT_CI_EVIDENCE_DIR": str(root / "evidence"),
                "GIT_AUTHOR_NAME": "t",
                "GIT_AUTHOR_EMAIL": "t@example.com",
                "GIT_COMMITTER_NAME": "t",
                "GIT_COMMITTER_EMAIL": "t@example.com",
            }
            # The developer's own layout is not the remote's.
            environment.pop("CODING_DIR", None)
            for other, (variable, name) in HOSTS.items():
                environment[variable] = name
                if other not in (declared or (os_name,)):
                    environment.pop(variable, None)
            result = subprocess.run(
                [
                    "/bin/bash" if sys.platform == "darwin" else "bash",
                    str(checkout / "scripts" / "cross-check.sh"),
                    "--os",
                    os_arg or os_name,
                    PACKAGE,
                    *extra,
                ],
                cwd=checkout,
                env=environment,
                capture_output=True,
                text=True,
                timeout=120,
                start_new_session=True,
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
                "notes": {
                    name: self.note_of(git, "refs/notes/ci-local/" + name)
                    for name in schema.ENVIRONMENTS
                },
                "remote_refs": subprocess.run(
                    ["git", "--git-dir", str(origin), "for-each-ref", "--format=%(refname)",
                     "refs/notes"],
                    capture_output=True, text=True,
                ).stdout.split(),
                "evidence": sorted(
                    str(p.relative_to(root / "evidence"))
                    for p in (root / "evidence").rglob("*") if p.is_file()
                ),
            }
            if replay:
                report.update(self.replay_prelude(Path(clone_dir), shipped, remote))
            return report

    @staticmethod
    def note_of(git: Callable[..., str], ref: str) -> dict | None:
        """The collection on the fixture's HEAD under `ref`, or `None`."""
        try:
            return json.loads(git("notes", "--ref", ref, "show", "HEAD"))
        except subprocess.CalledProcessError:
            return None

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
                # The planner's key helper is the verifier the archive carried,
                # bound after it was verified and before any test runs.
                binding = 'export BISCUIT_CI_BUILD_BIN="$consume/build/tools/ci-build"'
                self.assertEqual(1, remote.count(binding), remote)
                self.assertLess(verify, remote.index(binding))
                self.assertLess(remote.index(binding), test)
                self.assertNotIn("BISCUIT_CI_BUILD_BIN=\"\\$", remote)
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
        # The same binding, spelled the way native Windows spells the file.
        binding = '$env:BISCUIT_CI_BUILD_BIN = "$consume\\build\\tools\\ci-build.exe"'
        self.assertEqual(1, remote.count(binding), remote)
        self.assertLess(verify, remote.index(binding))
        self.assertLess(remote.index(binding), test)
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
        # No archive, so no verified verifier to bind.
        self.assertNotIn("BISCUIT_CI_BUILD_BIN", run["remote"])
        windows = self.ship("windows", "--all-features")
        self.assertNotIn("BISCUIT_CI_BUILD_BIN", windows["remote"])

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

class RigPublicationRunTests(CrossCheckHarness):
    """The leg-end publication, EXECUTED against a fixture repository and bare origin.

    `ssh` and `scp` are stubs, so what the rig printed and what `scp -r` fetches
    are the fixture's; everything after that is the real script, the real
    `observation_notes.py publish-receipt --kind rig`, the real `completion.py`,
    and real notes in the fixture's repository. That is a faithful offline model
    of the orchestrating host; what it cannot model is the rig itself, which is
    exercised by `just cross-check` on the real hosts.
    """

    ENVIRONMENT = {"wsl": "wsl2-ubuntu", "windows": "windows-latest"}

    def markers(self, *, exit_code: int = 0, manifest: int = 0, dirty: int = 0, tree: str = "{TREE}") -> str:
        return (
            f"cross-check-tree: {tree}\n"
            f"cross-check-dirty: {dirty}\n"
            "cross-check-key: 0f1e2d3c4b5a6978\n"
            "cross-check-digest: 9a8b7c6d5e4f3021\n"
            f"cross-check-manifest-exit: {manifest}\n"
            f"cross-check-exit: {exit_code}\n"
            "cross-check-duration: 12\n"
        )

    def reports(self, tmp: Path, os_name: str, **kwargs) -> Path:
        import test_evidence_reuse  # the shared builder of a rig's staging directory

        return test_evidence_reuse.write_rig_reports(
            tmp / f"rig-{os_name}", PACKAGE, self.ENVIRONMENT[os_name], **kwargs
        )

    def leg(self, os_name: str, *extra: str, output: str | None = None, exit_code: int = 0,
            reports: dict | None = None, fetch: bool = True, **options) -> dict:
        """One rig leg: what it printed, what it left behind, and the script's reaction."""
        host = HOSTS[os_name][1]
        with tempfile.TemporaryDirectory(prefix="rig-reports-") as scratch:
            fetch_dirs = {}
            if fetch:
                fetch_dirs[host] = self.reports(Path(scratch), os_name, **(reports or {}))
            return self.ship(
                os_name,
                *extra,
                publisher=True,
                remote_runs={
                    host: {"output": self.markers() if output is None else output, "exit": exit_code}
                },
                fetch_dirs=fetch_dirs,
                **options,
            )

    def observations(self, run: dict, os_name: str) -> list[dict]:
        note = run["notes"][self.ENVIRONMENT[os_name]]
        return note["observations"] if note else []

    # -- publication: one leg, its own environment -------------------------------

    def test_a_wsl_leg_publishes_a_wsl2_observation_and_nothing_else(self) -> None:
        run = self.leg("wsl")
        self.assertEqual(0, run["returncode"], run["stdout"] + run["stderr"])
        self.assertIn("published a passing wsl2-ubuntu observation", run["stdout"])
        (observation,) = self.observations(run, "wsl")
        self.assertEqual("rig", observation["producer"]["kind"])
        self.assertEqual("Linux", observation["producer"]["host"]["os"])
        self.assertEqual("pass", observation["cell"]["outcome"])
        self.assertEqual(
            {"refs/notes/ci-local/wsl2-ubuntu"}, {r for r in run["remote_refs"] if "ci-local" in r},
            "published to the remote, under the WSL2 environment only",
        )
        self.assertIsNone(run["notes"]["windows-latest"])
        # The proof the observation cites survives the work directory's removal.
        retained = Path(observation["producer"]["host"]["report_dir"])
        self.assertTrue(any(e.endswith("alpha-L1.completion.json") for e in run["evidence"]), run["evidence"])
        self.assertEqual("evidence", retained.parent.parent.parent.name)

    def test_a_native_windows_leg_publishes_a_windows_observation_and_nothing_else(self) -> None:
        run = self.leg("windows")
        self.assertEqual(0, run["returncode"], run["stdout"] + run["stderr"])
        self.assertIn("published a passing windows-latest observation", run["stdout"])
        (observation,) = self.observations(run, "windows")
        self.assertEqual("Windows", observation["producer"]["host"]["os"])
        self.assertEqual("x86_64-pc-windows-msvc", observation["proof"]["target"])
        self.assertEqual({"refs/notes/ci-local/windows-latest"}, {r for r in run["remote_refs"] if "ci-local" in r})
        self.assertIsNone(run["notes"]["wsl2-ubuntu"], "WSL2 is never filed under Windows or the reverse")
        # PowerShell and `scp` spell the fetched path with forward slashes.
        fetches = [c["argv"] for c in run["calls"] if c["tool"] == "scp" and "-r" in c["argv"]]
        self.assertEqual(1, len(fetches), run["calls"])
        self.assertRegex(fetches[0][-2], r"^build-win-native:B:/coding/.+-reports$")

    def test_the_remote_reports_are_discarded_after_each_leg(self) -> None:
        wsl = self.leg("wsl")
        self.assertTrue(
            [c for c in wsl["calls"] if c["tool"] == "ssh" and c["argv"][-1].startswith("rm -rf ")
             and "-reports" in c["argv"][-1]], wsl["calls"])
        windows = self.leg("windows")
        self.assertTrue(
            [c for c in windows["calls"] if c["tool"] == "ssh"
             and c["argv"][-1].startswith("Remove-Item -Recurse -Force ") and "-reports" in c["argv"][-1]],
            windows["calls"])

    def test_a_completed_failure_is_published_as_a_failing_observation(self) -> None:
        run = self.leg(
            "windows", output=self.markers(exit_code=100), exit_code=100,
            reports={"reported": {"t_one": "fail", "t_two": "pass", "t_three": "pass"}},
        )
        self.assertEqual(1, run["returncode"], "the failing leg still fails the script")
        self.assertIn("published a FAILING windows-latest observation", run["stdout"])
        (observation,) = self.observations(run, "windows")
        self.assertEqual("fail", observation["cell"]["outcome"])
        self.assertNotIn("completion_record_digest", observation["proof"])
        self.assertIn("windows  FAIL", run["stdout"])

    def test_a_leg_retries_the_observation_that_existed_when_it_started(self) -> None:
        failing = self.leg(
            "wsl", output=self.markers(exit_code=100), exit_code=100,
            reports={"reported": {"t_one": "fail", "t_two": "pass", "t_three": "pass"}},
        )
        (failed,) = self.observations(failing, "wsl")

        def plant_the_failure(repo: Path, git: Callable[..., str]) -> None:
            head = git("rev-parse", "HEAD")
            note = repo.parent / "failure-note.json"
            note.write_text(json.dumps({**failing["notes"]["wsl2-ubuntu"], "head": head}), encoding="utf-8")
            git("notes", "--ref", "refs/notes/ci-local/wsl2-ubuntu", "add", "-F", str(note), head)

        run = self.leg("wsl", prepare=plant_the_failure)
        self.assertEqual(0, run["returncode"], run["stdout"] + run["stderr"])
        passed = [o for o in self.observations(run, "wsl") if o["cell"]["outcome"] == "pass"]
        self.assertEqual([failed["observation_id"]], [o.get("retry_of") for o in passed])

    def test_a_leg_with_nothing_to_retry_names_no_predecessor(self) -> None:
        (observation,) = self.observations(self.leg("wsl"), "wsl")
        self.assertNotIn("retry_of", observation)

    # -- publication declined, with the reason printed ----------------------------

    def assertDeclined(self, run: dict, os_name: str, reason: str) -> None:
        self.assertIn(f"publishing no {self.ENVIRONMENT[os_name]} observation", run["stdout"] + run["stderr"])
        self.assertIn(reason, run["stdout"] + run["stderr"])
        for name, note in run["notes"].items():
            self.assertIsNone(note, f"a declined leg must leave no {name} note")
        self.assertEqual([], run["evidence"], "nothing is retained for a declined leg")

    def test_a_failed_manifest_step_declines_but_the_tests_ran(self) -> None:
        for os_name in ("wsl", "windows"):
            with self.subTest(os=os_name):
                run = self.leg(os_name, output=self.markers(manifest=1))
                self.assertEqual(0, run["returncode"], "the tests passed, so the leg passes")
                self.assertDeclined(run, os_name, "expected-test manifest step reported '1'")

    def test_a_run_that_never_reached_its_tests_declines(self) -> None:
        run = self.leg("wsl", output="cross-check-tree: {TREE}\n", exit_code=1)
        self.assertDeclined(run, "wsl", "no cross-check-exit marker")
        self.assertEqual(1, run["returncode"])

    def test_a_tree_that_is_not_the_heads_declines(self) -> None:
        run = self.leg("wsl", output=self.markers(tree="0" * 40))
        self.assertDeclined(run, "wsl", "is not this head's tree")

    def test_a_dirty_remote_declines(self) -> None:
        run = self.leg("windows", output=self.markers(dirty=3))
        self.assertDeclined(run, "windows", "worktree was dirty")

    def test_a_filtered_run_declines(self) -> None:
        run = self.leg("wsl", "level2_")
        self.assertDeclined(run, "wsl", "narrowed by")

    def test_a_leg_whose_reports_cannot_be_fetched_declines(self) -> None:
        run = self.leg("wsl", fetch=False)
        self.assertDeclined(run, "wsl", "left no staged reports to fetch")

    def test_an_incomplete_run_declines_with_completions_codes(self) -> None:
        run = self.leg("windows", reports={"reported": {"t_one": "pass"}})
        self.assertDeclined(run, "windows", "completion-test-missing")

    def test_a_native_mode_run_publishes_nothing_and_fetches_nothing(self) -> None:
        for os_name in ("wsl", "windows"):
            with self.subTest(os=os_name):
                run = self.leg(os_name, "--features", "extra")
                self.assertEqual(0, run["returncode"], run["stdout"] + run["stderr"])
                self.assertIn("mode: native", run["stdout"])
                self.assertNotIn("publishing no", run["stdout"] + run["stderr"])
                self.assertFalse(
                    [c for c in run["calls"] if c["tool"] == "scp" and "-r" in c["argv"]],
                    "a native run stages no reports",
                )
                for note in run["notes"].values():
                    self.assertIsNone(note)

    def test_an_unreachable_origin_keeps_the_observation_local(self) -> None:
        def break_origin(repo: Path, git: Callable[..., str]) -> None:
            git("remote", "set-url", "--push", "origin", str(repo.parent / "gone.git"))

        run = self.leg("wsl", prepare=break_origin)
        self.assertEqual(0, run["returncode"], "the leg passed; only the remote copy failed")
        self.assertIn("locally; it could not be pushed", run["stdout"] + run["stderr"])
        self.assertEqual(1, len(self.observations(run, "wsl")), "the proof is on this host")
        self.assertEqual([], [r for r in run["remote_refs"] if "ci-local" in r])

    # -- rig busy, unreachable, interrupted ---------------------------------------

    def test_a_busy_rig_is_reported_as_busy_not_as_a_test_failure(self) -> None:
        run = self.leg("wsl", output="cross-check: gave up after 1800s\n", exit_code=75)
        self.assertEqual(1, run["returncode"])
        self.assertIn("wsl      BUSY", run["stdout"])
        self.assertIn("rig busy", run["stdout"] + run["stderr"])
        self.assertNotIn("publishing no", run["stdout"] + run["stderr"])
        self.assertFalse([c for c in run["calls"] if c["tool"] == "scp" and "-r" in c["argv"]])

    def test_an_unreachable_rig_is_reported_as_unreachable(self) -> None:
        run = self.leg("windows", output="", exit_code=255)
        self.assertIn("windows  UNREACHABLE", run["stdout"])
        self.assertEqual(1, run["returncode"])

    def test_an_interrupted_leg_records_nothing_and_keeps_earlier_proof(self) -> None:
        if sys.platform.startswith("linux") and "microsoft" in Path("/proc/version").read_text().lower():
            self.skipTest("--os all skips the local WSL2 leg on a WSL2 host")
        with tempfile.TemporaryDirectory(prefix="rig-reports-") as scratch:
            fetch_dirs = {
                HOSTS["windows"][1]: self.reports(Path(scratch), "windows"),
                HOSTS["wsl"][1]: self.reports(Path(scratch), "wsl"),
            }
            run = self.ship(
                "windows",
                publisher=True,
                declared=("windows", "wsl"),
                os_arg="all",
                remote_runs={
                    HOSTS["windows"][1]: {"output": self.markers()},
                    HOSTS["wsl"][1]: {"output": "cross-check-tree: {TREE}\n", "interrupt": True},
                },
                fetch_dirs=fetch_dirs,
            )
        self.assertNotEqual(0, run["returncode"])
        self.assertIn("cross-check: wsl interrupted; no observation recorded", run["stderr"])
        self.assertIn("windows  pass", run["stdout"])
        self.assertIn("wsl      INTERRUPTED", run["stdout"])
        self.assertEqual(1, len(self.observations(run, "windows")), "earlier proof is intact")
        self.assertIsNone(run["notes"]["wsl2-ubuntu"])
        self.assertEqual({"refs/notes/ci-local/windows-latest"}, {r for r in run["remote_refs"] if "ci-local" in r})

    def test_each_leg_is_published_before_the_next_leg_is_contacted(self) -> None:
        if sys.platform.startswith("linux") and "microsoft" in Path("/proc/version").read_text().lower():
            self.skipTest("--os all skips the local WSL2 leg on a WSL2 host")
        with tempfile.TemporaryDirectory(prefix="rig-reports-") as scratch:
            run = self.ship(
                "windows",
                publisher=True,
                declared=("windows", "wsl"),
                os_arg="all",
                remote_runs={
                    HOSTS["windows"][1]: {"output": self.markers()},
                    HOSTS["wsl"][1]: {"output": self.markers()},
                },
                fetch_dirs={
                    HOSTS["windows"][1]: self.reports(Path(scratch), "windows"),
                    HOSTS["wsl"][1]: self.reports(Path(scratch), "wsl"),
                },
            )
        self.assertEqual(0, run["returncode"], run["stdout"] + run["stderr"])
        windows = run["stdout"].index("published a passing windows-latest observation")
        wsl_leg = run["stdout"].index(f"== wsl ({HOSTS['wsl'][1]}")
        wsl = run["stdout"].index("published a passing wsl2-ubuntu observation")
        self.assertLess(windows, wsl_leg, "windows is published before the WSL2 leg starts")
        self.assertLess(wsl_leg, wsl)
        self.assertEqual(
            {"refs/notes/ci-local/windows-latest", "refs/notes/ci-local/wsl2-ubuntu"},
            {r for r in run["remote_refs"] if "ci-local" in r},
        )

    # -- what the rig is told to do ------------------------------------------------

    def test_every_rig_lists_its_expected_tests_before_running_them(self) -> None:
        for os_name, listing in (
            ("wsl", "just _expected_manifest L1 'alpha' \\\n    --archive-file \"$archive_file\" --workspace-remap \"$src\""),
            ("windows", "just _expected_manifest L1 'alpha' --archive-file $archiveFile --workspace-remap $nativeSrc"),
        ):
            with self.subTest(os=os_name):
                remote = self.ship(os_name)["remote"]
                self.assertIn(listing, remote)
                self.assertLess(remote.index("_expected_manifest L1"), remote.index("just _test "))
                self.assertIn("cross-check-manifest-exit", remote)
                binding = "export BISCUIT_CI_ENVIRONMENT" if os_name == "wsl" else "$env:BISCUIT_CI_ENVIRONMENT"
                self.assertLess(
                    remote.index(binding), remote.index("_expected_manifest L1"),
                    "the manifest is keyed by the environment the leg exports",
                )

    def test_the_staging_directory_survives_for_the_fetch(self) -> None:
        unix = self.ship("wsl")["remote"]
        self.assertIn('cp -R "$reports" "$reports_out"', unix)
        self.assertNotIn("-report.xml", unix)
        windows = self.ship("windows")["remote"]
        self.assertIn("Copy-Item $reports $reportsOut -Recurse -Force", windows)
        self.assertNotIn("-report.xml", windows)
        # The cleanup removes the consumer directory but not the per-run copy.
        self.assertNotIn("reportsOut -ErrorAction", windows.split("finally")[1])

class RigPublicationContractTests(unittest.TestCase):
    """The shape of the leg-end publication, read from the script.

    The behavior itself is executed in `RigPublicationRunTests`; these pin what
    a run cannot show from the outside: that nothing but the publisher writes
    notes, and the ordering the "publish as each cell finishes" contract needs.
    """

    def text(self) -> str:
        return SCRIPT.read_text(encoding="utf-8")

    def function_body(self, name: str) -> str:
        text = self.text()
        start = text.index(f"{name}() {{")
        return text[start : text.index("\n}\n", start)]

    def test_publication_goes_through_the_observation_publisher_only(self) -> None:
        body = self.function_body("publish_rig_observation")
        self.assertIn("scripts/ci/observation_notes.py publish-receipt --kind rig", body)
        self.assertNotIn("git notes", body)
        self.assertNotIn("git push", body)
        self.assertNotIn("publish_wsl_receipt", self.text())

    def test_the_proof_is_retained_where_the_publisher_creates_the_per_run_directory(self) -> None:
        body = self.function_body("publish_rig_observation")
        self.assertIn("BISCUIT_CI_EVIDENCE_DIR", body)
        self.assertIn('--run-id "${run_id}"', body)

    def test_each_leg_is_recorded_before_the_next_one_starts(self) -> None:
        loop = self.text()[self.text().index('for os in "${ORDER[@]}"; do\n    [[ -n "${want[${os}]:-}" && -z'):]
        self.assertLess(loop.index("run_unix"), loop.index("publish_rig_observation"))
        self.assertLess(loop.index("publish_rig_observation"), loop.index("\ndone\n"))
        self.assertLess(loop.index("\ndone\n"), loop.index("print_summary"))

    def test_a_legs_predecessor_is_fixed_before_the_leg_runs(self) -> None:
        loop = self.text()[self.text().index('for os in "${ORDER[@]}"; do\n    [[ -n "${want[${os}]:-}" && -z'):]
        self.assertLess(loop.index("retry-predecessor"), loop.index("run_windows"))
        self.assertLess(loop.index("retry-predecessor"), loop.index("run_unix"))
        self.assertIn('--retry-of "${RETRY_OF[${os}]}"', self.function_body("publish_rig_observation"))

    def test_both_rig_oses_are_recorded_and_no_other(self) -> None:
        body = self.function_body("is_rig")
        self.assertIn('"$1" == "wsl"', body)
        self.assertIn('"$1" == "windows"', body)
        self.assertNotIn("linux", body)
        self.assertNotIn("macos", body)

    def test_interrupts_are_trapped_and_leave_a_nonzero_exit(self) -> None:
        text = self.text()
        self.assertIn("trap 'on_interrupt 130' INT", text)
        self.assertIn("trap 'on_interrupt 143' TERM", text)
        body = self.function_body("on_interrupt")
        self.assertIn("interrupted; no observation recorded", body)
        self.assertIn('exit "$1"', body)

    def test_the_script_still_parses(self) -> None:
        completed = subprocess.run(["bash", "-n", str(SCRIPT)], capture_output=True, text=True)
        self.assertEqual(0, completed.returncode, completed.stderr)

    def test_shellcheck_has_no_warnings_when_installed(self) -> None:
        if shutil.which("shellcheck") is None:
            self.skipTest("shellcheck is not installed")
        completed = subprocess.run(
            ["shellcheck", "-S", "warning", str(SCRIPT)], capture_output=True, text=True
        )
        self.assertEqual(0, completed.returncode, completed.stdout)


if __name__ == "__main__":
    unittest.main()

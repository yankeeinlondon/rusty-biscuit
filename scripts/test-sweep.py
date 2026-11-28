"""Regression checks for the sweep report and recipe dispatcher."""

from contextlib import redirect_stdout
import importlib.util
import io
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from types import SimpleNamespace
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parent
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("sweep_report", SCRIPTS / "sweep-report.py")
report = importlib.util.module_from_spec(spec)
spec.loader.exec_module(report)


class EstimateTests(unittest.TestCase):
    def test_cap_uses_projected_free_space_and_excludes_earlier_candidates(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / "target"
            target.mkdir()
            incremental, stale, excess = [target / name for name in ("incremental", "stale", "excess")]
            for path in (incremental, stale, excess):
                path.write_bytes(b"x" * 8192)

            class FixtureDisk(report.Allocation):
                def __init__(self, root):
                    super().__init__(root)
                    self.volumes[root.stat().st_dev] = ("Checkout", 4 * report.GIB, 3 * report.GIB, report.GIB - 1)

            def candidates(path, criterion, *values):
                if criterion == "--maxsize":
                    self.assertEqual(values, ("200GB",))
                    return {target: {stale, excess}}, {target: 20_480}, False
                return {target: {stale}} if criterion == "--time" else {}, {}, False

            with patch.object(report, "Allocation", FixtureDisk), patch.object(report, "cargo_candidates", candidates):
                disk, savings, missing = report.preview(root, "14", "1", "120GB", "200GB",
                                                       {target: {incremental}}, set())
            device = root.stat().st_dev
            self.assertFalse(missing)
            self.assertEqual(savings["Incremental caches"][device], incremental.stat().st_blocks * 512)
            self.assertEqual(savings["Artifacts older than 14 days"][device], stale.stat().st_blocks * 512)
            self.assertEqual(savings["Cargo size cap (200 GB)"][device], excess.stat().st_blocks * 256)
            out = io.StringIO()
            with redirect_stdout(out):
                report.draw(disk, savings, missing, "Not installed")
            value = out.getvalue()
            self.assertNotIn(directory, value)
            self.assertIn("Free after", value)
            self.assertIn("Expected to reclaim", value)
            self.assertIn("No savings expected:\n  Artifacts from removed toolchains\n  Stale external targets", value)
            self.assertTrue(all(path.exists() for path in (incremental, stale, excess)))
            self.assertNotIn("\x1b[", value)

            for mode, heading, value in (
                ("color", "\x1b[1;36mSchedule:\x1b[0m", "\x1b[33mNot installed\x1b[0m"),
                ("bold", "\x1b[1mSchedule:\x1b[0m", "Not installed"),
            ):
                output = io.StringIO()
                with redirect_stdout(output):
                    report.draw(disk, savings, missing, "Not installed", mode=mode)
                self.assertIn(f"{heading} {value}", output.getvalue())

    def test_sizes_align_at_the_decimal_with_mixed_units_and_estimates(self):
        cells = [report.size_cell(size, estimated=estimated) for size, estimated in (
            (284 * 1024, True), (0, False), (1000 * report.GIB, True),
        )]
        self.assertEqual({cell.index(".") for cell in cells}, {6})
        self.assertEqual({len(cell) for cell in cells}, {12})

    def test_apfs_volumes_with_the_same_capacity_share_a_row(self):
        root = SimpleNamespace(stat=lambda: SimpleNamespace(st_dev=10))
        coding = SimpleNamespace(stat=lambda: SimpleNamespace(st_dev=11))
        restricted = SimpleNamespace(stat=lambda: SimpleNamespace(st_dev=12))
        sizes = {10: 1000, 11: 1000, 12: 500}
        def usage(path):
            return SimpleNamespace(f_frsize=4096, f_blocks=sizes[path.stat().st_dev],
                                   f_bfree=100, f_bavail=100)

        disk = report.Allocation.__new__(report.Allocation)
        disk.volumes, disk.devices = {}, {}
        with (patch.object(report.sys, "platform", "darwin"),
              patch.object(report.os, "statvfs", usage),
              patch.object(report, "apfs_container", return_value="disk3")):
            self.assertEqual(disk.volume(root, "System"), disk.volume(coding, "Checkout"))
            self.assertNotEqual(disk.volume(root, "System"), disk.volume(restricted, "Home"))
        self.assertEqual(len(disk.volumes), 2)

    def test_blocks_are_counted_only_when_last_hardlink_is_selected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first, second = root / "first", root / "second"
            first.write_bytes(b"x" * 8192)
            os.link(first, second)
            disk = report.Allocation(root)
            self.assertFalse(disk.measure([first], "Test")[1])
            self.assertEqual(sum(disk.measure([first, second], "Test")[1].values()), first.stat().st_blocks * 512)

    def test_nested_profile_records_stay_grouped_under_the_cargo_target(self):
        target = Path("/repo/target")
        artifact = target / "debug/deps/old"
        process = type("DryRun", (), {
            "stdout": [
                f'[DEBUG] cleaning: "{target}" with remove_older_until_fits\n',
                '[DEBUG] size_to_remove: 1234\n',
                f'[DEBUG] cleaning: "{target}/debug" with remove_not_built_with_in_a_profile\n',
                f'[DEBUG] Would remove: "{artifact}"\n',
                f'[INFO] Would clean: 1.00 KiB from "{target}"\n',
                f'[DEBUG] cleaning: "{target}/release" with remove_not_built_with_in_a_profile\n',
            ],
            "__enter__": lambda self: self,
            "__exit__": lambda self, *_: None,
            "wait": lambda self: 0,
        })()
        with patch.object(report.subprocess, "Popen", return_value=process) as spawn:
            candidates, excess, failed = report.cargo_candidates(target.parent, "--maxsize", "200GB")
        self.assertIn("--dry-run", spawn.call_args.args[0])
        self.assertEqual(candidates, {target: {artifact}})
        self.assertEqual(excess, {target: 1234})
        self.assertFalse(failed)


class DispatcherTests(unittest.TestCase):
    def setUp(self):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)
        self.scripts = self.root / "scripts"
        self.tools = self.root / "bin"
        self.remote = self.root / "coding" / "rusty-biscuit"
        self.home = self.root / "home"
        for path in (self.scripts, self.tools, self.remote / "scripts", self.home):
            path.mkdir(parents=True)
        for script in ("sweep-host.sh", "sweep-report.py", "macos-cargo-sweep.sh"):
            shutil.copyfile(SCRIPTS / script, self.scripts / script)
        for scripts in (self.scripts, self.remote / "scripts"):
            (scripts / "cargo-path.sh").write_text(":\n")
            (scripts / "linux-cargo-sweep.sh").write_text('echo "linux:$1"\n')
            (scripts / "sweep.sh").write_text(
                'if [[ "${1:-}" == --preview-input ]]; then\n'
                '    printf "%s\\0" "$PWD" 14 100 120GB 200GB\n'
                'else\n    echo "ran:${SWEEP_FIXTURE_HOST:-local}"\nfi\n'
            )
        (self.scripts / "cross-check-hosts.sh").write_text(
            'SSH=(ssh)\ncalling_host_identity() { origin_host=local; worktree=main; }\n'
        )
        self.env = dict(os.environ, PATH=f"{self.tools}:{os.environ['PATH']}", HOME=str(self.home),
                        CODING_DIR=str(self.remote.parent), BUILD_WIN="offline",
                        BUILD_WSL="guest", BUILD_MACOS="mac", BUILD_LINUX="guest")
        self.fake("uname", 'echo "${FIXTURE_OS:-Linux}"\n')
        self.fake("just", 'exit 0\n')
        self.fake("cargo", '[[ "$1" == sweep && "$2" == --dry-run ]]\n')
        self.fake("crontab", 'exit 1\n')
        self.fake("systemctl", 'exit 1\n')
        self.fake("powershell.exe", 'printf "%s\\n" "$@"\n')
        self.fake("launchctl", 'echo "launchctl:$1"\n')
        self.fake("plutil", 'exit 0\n')
        self.fake("ssh", '''shift
host="$1"
command="$2"
[[ "$host" != offline ]] || exit 42
case "$command" in
    echo*) echo "unix" ;;
    bash*) command="${command%?}"; SWEEP_FIXTURE_HOST="$host" bash -s -- "${command##* }" ;;
esac
''')

    def fake(self, name, body):
        path = self.tools / name
        path.write_text("#!/bin/bash\n" + body)
        path.chmod(0o755)

    def dispatch(self, *args, **env):
        return subprocess.run(["bash", "scripts/sweep-host.sh", *args], cwd=self.root,
                              env=dict(self.env, **env), capture_output=True, text=True)

    def test_all_visits_local_and_unique_remotes_even_after_failure(self):
        run = self.dispatch("run", "all")
        self.assertEqual(run.returncode, 1)
        self.assertEqual([line for line in run.stdout.splitlines() if line.startswith("ran:")],
                         ["ran:local", "ran:guest", "ran:mac"])
        status = self.dispatch("status", "all")
        self.assertEqual(status.returncode, 1)
        self.assertEqual(status.stdout.count("Sweep preview:"), 3)
        self.assertNotIn(str(self.root), status.stdout)
        self.assertIn("failed for offline (BUILD_WIN)", status.stderr)

    def test_install_dispatches_for_linux_windows_and_macos(self):
        linux = self.dispatch("install")
        self.assertEqual((linux.returncode, linux.stdout), (0, "linux:install\n"))
        windows = self.dispatch("install", FIXTURE_OS="CYGWIN_NT")
        self.assertEqual(windows.returncode, 0, windows.stderr)
        self.assertIn("scripts/windows-cargo-sweep.ps1\n-Operation\ninstall\n", windows.stdout)
        mac = self.dispatch("install", FIXTURE_OS="Darwin")
        self.assertEqual(mac.returncode, 0, mac.stderr)
        plist = self.home / "Library/LaunchAgents/com.ken.rusty-biscuit-sweep.plist"
        schedule = plistlib.loads(plist.read_bytes())
        self.assertEqual([entry["Weekday"] for entry in schedule["StartCalendarInterval"]], [0, 3])
        self.assertEqual(schedule["ProgramArguments"][-1], "run")
        self.assertEqual(self.dispatch("uninstall", FIXTURE_OS="Darwin").returncode, 0)
        self.assertFalse(plist.exists())

    def test_linux_install_keeps_the_timer_when_linger_requires_admin(self):
        shutil.copyfile(SCRIPTS / "linux-cargo-sweep.sh", self.scripts / "linux-cargo-sweep.sh")
        self.fake("systemctl", "exit 0\n")
        self.fake("loginctl", """if [[ "$1" == show-user ]]; then
    echo no
elif [[ "$1" == --no-ask-password && "$2" == enable-linger ]]; then
    echo "Auth denied" >&2
    exit 1
else
    printf '%s\n' "$*" > "$HOME/wrong-loginctl-flags"
    exit 1
fi
""")
        result = self.dispatch("install", XDG_DATA_HOME=str(self.home / "data"))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.home / "data/systemd/user/timers.target.wants/rusty-biscuit-sweep.timer").is_symlink())
        self.assertFalse((self.home / "wrong-loginctl-flags").exists())
        self.assertIn("Installed systemd user timer", result.stdout)
        self.assertIn("Scheduled sweeps may stop after you sign out.", result.stderr)
        self.assertIn("sudo loginctl enable-linger", result.stderr)
        self.assertNotIn("Auth denied", result.stderr)

    def test_incremental_cleanup_retries_or_defers_when_a_directory_changes(self):
        shutil.copyfile(SCRIPTS / "sweep.sh", self.scripts / "sweep.sh")
        self.fake("pgrep", '[[ "$RACE_CASE" == active && -f "$RACE_STATE" ]]\n')
        self.fake("cargo", 'printf "Cargo pass: %s\\n" "$3"\n')
        self.fake("tmutil", 'exit 0\n')
        self.fake("rm", """if [[ "$RACE_CASE" == permission ]]; then
    printf 'rm: cannot remove cache: Permission denied\\n' >&2
    exit 1
fi
if [[ ! -f "$RACE_STATE" ]]; then
    touch "$RACE_STATE"
    printf 'rm: cannot remove cache: Directory not empty\\n' >&2
    exit 1
fi
exec "$REAL_RM" "$@"
""")
        for case in ("idle", "active", "permission"):
            with self.subTest(case=case):
                worktree = self.root / case
                incremental = worktree / "target" / "debug" / "incremental"
                incremental.mkdir(parents=True)
                (incremental / "build-cache").write_bytes(b"x" * 8192)
                result = subprocess.run(
                    ["bash", self.scripts / "sweep.sh", str(worktree)],
                    env=dict(self.env, RACE_CASE=case, RACE_STATE=str(worktree / "raced"),
                             REAL_RM=shutil.which("rm"), SWEEP_INCREMENTAL_MAX_GIB="0",
                             SWEEP_ORPHAN_DIRS=str(self.home / "unused")),
                    capture_output=True, text=True,
                )
                if case == "idle":
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertIn("Retrying once.", result.stderr)
                    self.assertIn("Cargo pass: --maxsize", result.stdout)
                    self.assertFalse(incremental.exists())
                elif case == "active":
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertIn("Sweep deferred", result.stdout)
                    self.assertNotIn("Cargo pass:", result.stdout)
                    self.assertNotIn("Retrying", result.stderr)
                    self.assertTrue(incremental.exists())
                else:
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn("Permission denied", result.stderr)
                    self.assertNotIn("Cargo pass:", result.stdout)

    def test_direct_dry_run_keeps_incremental_and_orphan_files(self):
        shutil.copyfile(SCRIPTS / "sweep.sh", self.scripts / "sweep.sh")
        worktree = self.root / "worktree"
        inc = worktree / "target" / "debug" / "incremental"
        orphan = self.home / ".cache" / "old-target"
        inc.mkdir(parents=True)
        orphan.mkdir(parents=True)
        artifact = inc / "artifact"
        marker = orphan / "CACHEDIR.TAG"
        artifact.write_bytes(b"x" * 8192)
        marker.write_text("Created by Cargo\n")
        past = time.time() - 40 * 86400
        for path in (orphan, marker):
            os.utime(path, (past, past))
        self.fake("rm", 'echo "unexpected removal" >&2; exit 41\n')
        self.fake("tmutil", 'echo "unexpected snapshot thinning" >&2; exit 42\n')
        result = subprocess.run(["bash", self.scripts / "sweep.sh", "--dry-run", "."], cwd=worktree,
                                env=dict(self.env, SWEEP_INCREMENTAL_MAX_GIB="0",
                                         SWEEP_ORPHAN_DIRS=str(orphan.parent)),
                                capture_output=True, text=True)
        self.assertEqual((result.returncode, result.stderr), (0, ""))
        expected_savings = result.stdout.split("No savings expected")[0]
        self.assertIn("Incremental caches", expected_savings)
        self.assertIn("Stale external targets", expected_savings)
        self.assertTrue(artifact.exists() and marker.exists())


if __name__ == "__main__":
    unittest.main()

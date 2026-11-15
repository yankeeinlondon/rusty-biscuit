#!/usr/bin/env python3
"""Tests for `installers.py`: the anchored preflight transformation of dist's
real 0.34.0 installer, the hardened installer's behavior under fake platform
commands (no network, no real HOME), and dist checksum verification."""

from __future__ import annotations

import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))

import collections
import json
import re
import shutil
import subprocess
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
from pathlib import Path

sys.path.append(_HERE)
import installers  # noqa: E402
from tool_guard import require_tools  # noqa: E402

ENFORCED_BY = 'the repo-deps "distribution" companion suite (just distribution-test)'

FIXTURES = Path(_HERE) / "fixtures"
INSTALLER = FIXTURES / "biscuit-hash-cli-installer.sh"
RELEASE = "https://github.com/yankeeinlondon/rusty-biscuit/releases/download/biscuit-hash-cli-v0.1.0"
# Real tools the installer requires before it reaches archive selection.
REAL_TOOLS = ("chmod", "mkdir", "rm", "tar", "grep", "cat", "head", "tail", "awk", "sed", "cut", "tr")
SYSTEM_PATH = "/usr/bin:/bin:/usr/sbin:/sbin"


def original() -> str:
    return INSTALLER.read_text(encoding="utf-8")


def hardened() -> str:
    return installers.harden(original(), source="fixture")[0]


class TransformationTest(unittest.TestCase):
    def test_real_installer_hardens_and_parses(self) -> None:
        text, report = installers.harden(original(), source="fixture")
        self.assertEqual([i["what"] for i in report["insertions"]], ["preflight call", "preflight functions"])
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "installer.sh"
            path.write_text(text, encoding="utf-8")
            subprocess.run(["sh", "-n", str(path)], check=True)

    def test_hardening_adds_no_new_shellcheck_findings(self) -> None:
        """The template uses `local` throughout (SC3043); the block may add
        more of that and nothing else."""
        require_tools("shellcheck", enforced_by=ENFORCED_BY)
        def findings(text: str) -> collections.Counter[str]:
            with tempfile.TemporaryDirectory() as tmp:
                path = Path(tmp) / "installer.sh"
                path.write_text(text, encoding="utf-8")
                out = subprocess.run(["shellcheck", "-s", "sh", "-f", "gcc", str(path)], capture_output=True, text=True).stdout
            return collections.Counter(re.sub(r"^[^:]*:\d+:\d+: ", "", line) for line in out.splitlines())
        added = findings(hardened()) - findings(original())
        self.assertEqual({key for key in added if "[SC3043]" not in key}, set())

    def test_bytes_outside_insertions_are_unchanged(self) -> None:
        text = original()
        out = hardened()
        call_at = text.index("    get_architecture || return 1\n")
        self.assertEqual(out[:call_at], text[:call_at])
        self.assertTrue(out[call_at:].startswith(installers.CALL))
        rest = out[call_at + len(installers.CALL):]
        entry = 'download_binary_and_run_installer "$@" || exit 1\n'
        self.assertTrue(rest.endswith(entry))
        block_at = text.rindex(entry)
        block = installers.preflight_block(installers.distribution.MIN_GLIBC, installers.MACOS_FLOOR)
        self.assertEqual(rest[: block_at - call_at], text[call_at:block_at])
        self.assertEqual(rest[block_at - call_at:], block + entry)

    def test_preflight_call_precedes_selection_and_download(self) -> None:
        out = hardened()
        call = out.index(installers.CALL)
        self.assertLess(call, out.index('_artifact_name="$(select_archive_for_arch'))
        self.assertLess(call, out.index('_dir="$(ensure mktemp -d)"'))
        self.assertLess(call, out.index('if ! downloader "$_url" "$_file"'))

    def test_missing_anchor_is_named(self) -> None:
        for anchor in installers.ANCHORS:
            with self.subTest(anchor=anchor.name):
                broken = original().replace(anchor.text, "\n# drifted\n")
                with self.assertRaisesRegex(installers.InstallerError, f"'{anchor.name}' must occur exactly once, found 0"):
                    installers.harden(broken, source="fixture")

    def test_duplicated_anchor_is_named(self) -> None:
        for anchor in installers.ANCHORS:
            with self.subTest(anchor=anchor.name):
                text = original()
                at = text.index(anchor.text)
                broken = text[:at] + anchor.text + "\n" + text[at:]
                with self.assertRaisesRegex(installers.InstallerError, f"'{anchor.name}' must occur exactly once, found 2"):
                    installers.harden(broken, source="fixture")

    def test_already_hardened_is_refused(self) -> None:
        with self.assertRaisesRegex(installers.InstallerError, "already hardened"):
            installers.harden(hardened(), source="fixture")

    def test_dist_glibc_floor_must_match(self) -> None:
        drifted = original().replace('check_glibc "2" "35"', 'check_glibc "2" "17"')
        with self.assertRaisesRegex(installers.InstallerError, "disagrees with the 2.35 floor"):
            installers.harden(drifted, source="fixture")

    def test_cli_writes_output_and_reports(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "hardened.sh"
            result = subprocess.run([sys.executable, str(Path(_HERE) / "installers.py"), "harden", str(INSTALLER), "--output", str(out)],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(out.read_text(encoding="utf-8"), hardened())
            again = subprocess.run([sys.executable, str(Path(_HERE) / "installers.py"), "harden", str(out), "--output", str(out)],
                                   capture_output=True, text=True)
            self.assertEqual(again.returncode, 1)
            self.assertIn("already hardened", again.stderr)


class BuildContractTest(unittest.TestCase):
    """`macos_floor` reads `build-images.json` through the shared JSON loader;
    there is no fallback floor when the contract is absent or malformed."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.dir = Path(self._tmp.name)

    def contract(self, text: str) -> Path:
        path = self.dir / "build-images.json"
        path.write_text(text, encoding="utf-8")
        return path

    def real_with(self, value: object) -> str:
        data = json.loads(installers.BUILD_IMAGES.read_text(encoding="utf-8"))
        data["macos"]["deployment_target"] = value
        return json.dumps(data)

    def test_control_is_the_real_contract(self) -> None:
        self.assertEqual(installers.macos_floor(), installers.MACOS_FLOOR)
        self.assertEqual(installers.macos_floor(self.contract(self.real_with("15.0"))), "15.0")

    def test_absent_contract_is_an_error_not_a_default(self) -> None:
        with self.assertRaisesRegex(installers.InstallerError, r"build-images.json: cannot read the build contract"):
            installers.macos_floor(self.dir / "build-images.json")

    def test_deployment_target_shapes(self) -> None:
        cases = {"wrong type": (15, r"macos.deployment_target: must be a string, found integer"),
                 "not major.minor": ("15", r"macos.deployment_target: floor '15' is not '<major>.<minor>'")}
        for name, (value, pattern) in cases.items():
            with self.subTest(name), self.assertRaisesRegex((installers.InstallerError, installers.distribution.DistributionError), pattern):
                installers.macos_floor(self.contract(self.real_with(value)))

    def main(self, *argv: str, build_images: Path = installers.BUILD_IMAGES) -> tuple[int, str]:
        err = StringIO()
        with redirect_stdout(StringIO()), redirect_stderr(err):
            code = installers.main(list(argv), build_images=build_images)
        return code, err.getvalue()

    def test_cli_malformed_contract_writes_nothing(self) -> None:
        out = self.dir / "hardened.sh"
        bad = self.contract('{"macos": {"deployment_target": "15.0"}, "macos": {}}')
        code, err = self.main("harden", str(INSTALLER), "--output", str(out), build_images=bad)
        self.assertEqual(code, 1)
        self.assertIn(f"{bad.as_posix()}: invalid JSON: duplicate object key 'macos'", err)
        self.assertFalse(out.exists())

    def test_cli_unreadable_installer_is_located(self) -> None:
        source = self.dir / "installer.sh"
        source.write_bytes(b"#!/bin/sh\n\xff\n")
        code, err = self.main("harden", str(source), "--output", str(self.dir / "out.sh"))
        self.assertEqual(code, 1)
        self.assertIn(f"{source.as_posix()}: cannot read", err)
        self.assertFalse((self.dir / "out.sh").exists())


def fake(path: Path, body: str) -> None:
    path.write_text(f'#!/bin/sh\nprintf \'%s\\n\' "{path.name} $*" >> "$FAKE_LOG"\n{body}', encoding="utf-8")
    path.chmod(0o755)


class Sandbox:
    """A hardened installer run with only fake platform commands and real
    coreutils on PATH, and every home/install/temp path inside one temp dir."""

    def __init__(self, tmp: Path) -> None:
        self.root = tmp
        self.bin = tmp / "bin"
        self.home = tmp / "home"
        self.install = tmp / "install"
        self.tmpdir = tmp / "tmp"
        self.log = tmp / "calls.log"
        for directory in (self.bin, self.home, self.install, self.tmpdir):
            directory.mkdir()
        self.log.touch()
        for tool in REAL_TOOLS:
            real = shutil.which(tool, path=SYSTEM_PATH)
            assert real, tool
            (self.bin / tool).symlink_to(real)
        # get_architecture reads /proc/self/exe, else $SHELL, for ELF bitness.
        self.elf = tmp / "fake-elf"
        self.elf.write_bytes(b"\x7fELF\x02\x01\x01" + b"\0" * 57)
        self.script = tmp / "installer.sh"
        self.script.write_text(hardened(), encoding="utf-8")
        # macOS `mktemp -d` ignores TMPDIR; keep the download directory inside.
        fake(self.bin / "mktemp", f'exec {shutil.which("mktemp", path=SYSTEM_PATH)} -d "$TMPDIR/installer.XXXXXX"\n')
        fake(self.bin / "curl", "exit 22\n")
        fake(self.bin / "wget", "exit 8\n")

    def uname(self, os_name: str, arch: str) -> None:
        fake(self.bin / "uname", f'case "$1" in\n  -m) echo {arch} ;;\n  -o) echo GNU/Linux ;;\n  *) echo {os_name} ;;\nesac\n')

    def getconf(self, output: str) -> None:
        fake(self.bin / "getconf", f'[ "$1" = GNU_LIBC_VERSION ] || exit 1\necho "{output}"\n')

    def ldd(self, banner: str, *, stderr: bool = False, status: int = 0) -> None:
        redirect = " >&2" if stderr else ""
        fake(self.bin / "ldd", f"cat{redirect} <<'EOF'\n{banner}\nEOF\nexit {status}\n")

    def sw_vers(self, version: str) -> None:
        fake(self.bin / "sw_vers", f'[ "$1" = -productVersion ] || exit 1\necho {version}\n')
        fake(self.bin / "sysctl", "exit 1\n")

    def snapshot(self, *roots: Path) -> list[str]:
        return sorted(str(p.relative_to(self.root)) for root in roots for p in root.rglob("*"))

    def run(self) -> subprocess.CompletedProcess[str]:
        env = {
            "PATH": str(self.bin),
            "HOME": str(self.home),
            "CARGO_HOME": str(self.home / ".cargo"),
            "XDG_CONFIG_HOME": str(self.home / ".config"),
            "TMPDIR": str(self.tmpdir),
            "SHELL": str(self.elf),
            "BISCUIT_HASH_CLI_INSTALL_DIR": str(self.install),
            "BISCUIT_HASH_CLI_UNMANAGED_INSTALL": str(self.install),
            "BISCUIT_HASH_CLI_NO_MODIFY_PATH": "1",
            "FAKE_LOG": str(self.log),
        }
        return subprocess.run(["/bin/sh", str(self.script)], env=env, cwd=self.root, capture_output=True, text=True, timeout=30)

    def downloads(self) -> list[str]:
        return [line for line in self.log.read_text().splitlines() if line.split(" ", 1)[0] in ("curl", "wget")]


GLIBC_BANNER = "ldd (Ubuntu GLIBC {v}-0ubuntu3.1) {v}\nCopyright (C) 2022 Free Software Foundation, Inc."
MUSL_BANNER = "musl libc (x86_64)\nVersion 1.2.4\nDynamic Program Loader"


class BehaviorTest(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.box = Sandbox(Path(self._tmp.name))
        self.before = self.box.snapshot(self.box.home, self.box.install, self.box.tmpdir)

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def assert_refused(self, *needles: str) -> None:
        result = self.box.run()
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("cannot be installed", result.stderr)
        for needle in needles:
            self.assertIn(needle, result.stderr)
        self.assertEqual(self.box.downloads(), [])
        self.assertEqual(self.box.snapshot(self.box.home, self.box.install, self.box.tmpdir), self.before)

    def assert_download_attempted(self, target: str) -> None:
        result = self.box.run()
        self.assertNotIn("cannot be installed", result.stderr)
        downloads = self.box.downloads()
        self.assertTrue(downloads, result.stdout + result.stderr)
        self.assertTrue(downloads[0].startswith(f"curl -sSfL {RELEASE}/biscuit-hash-cli-{target}.tar.xz -o "), downloads)
        self.assertNotEqual(result.returncode, 0, "the fake download fails, so the install must not succeed")
        self.assertEqual(self.box.snapshot(self.box.home, self.box.install), [p for p in self.before if not p.startswith("tmp")])
        self.assertTrue(any(p.startswith("tmp/installer.") for p in self.box.snapshot(self.box.tmpdir)), "download went to the sandbox")

    def linux(self, arch: str = "x86_64", glibc: str | None = None) -> None:
        self.box.uname("Linux", arch)
        if glibc is not None:
            self.box.getconf(f"glibc {glibc}")
            self.box.ldd(GLIBC_BANNER.format(v=glibc))

    def test_linux_glibc_2_34_is_refused_before_download(self) -> None:
        self.linux(glibc="2.34")
        self.assert_refused("glibc 2.35 or newer", "found glibc 2.34")

    def test_linux_glibc_1_99_is_refused(self) -> None:
        self.linux(glibc="1.99")
        self.assert_refused("found glibc 1.99")

    def test_musl_is_refused_before_download(self) -> None:
        self.linux()
        self.box.getconf("")
        self.box.ldd(MUSL_BANNER, stderr=True, status=1)
        self.assert_refused("glibc 2.35 or newer", "found musl libc")

    def test_missing_detection_is_refused(self) -> None:
        self.linux()
        self.assert_refused("unable to detect the glibc version", "glibc 2.35 or newer")

    def test_garbage_ldd_output_is_refused(self) -> None:
        self.linux()
        self.box.ldd("something entirely different 9.x")
        self.assert_refused("unable to detect the glibc version", "something entirely different")

    def test_unparseable_glibc_version_is_refused(self) -> None:
        self.linux()
        self.box.getconf("glibc two.thirtyfive")
        self.assert_refused("unable to detect the glibc version", "two.thirtyfive")

    def test_ldd_fallback_reads_first_line(self) -> None:
        self.linux()
        self.box.ldd(GLIBC_BANNER.format(v="2.34"))
        self.assert_refused("found glibc 2.34")

    def test_unsupported_linux_architectures_are_refused(self) -> None:
        for arch in ("riscv64", "i686", "armv7l"):
            with self.subTest(arch=arch):
                self.linux(arch=arch, glibc="2.39")
                self.assert_refused(f"unsupported architecture: Linux {arch}")

    def test_unsupported_os_is_refused(self) -> None:
        self.box.uname("FreeBSD", "amd64")
        self.assert_refused("unsupported operating system: FreeBSD amd64")

    def test_supported_glibc_reaches_versioned_download(self) -> None:
        for glibc in ("2.35", "2.39", "2.100"):
            for arch, target in (("x86_64", "x86_64-unknown-linux-gnu"), ("aarch64", "aarch64-unknown-linux-gnu")):
                with self.subTest(glibc=glibc, arch=arch):
                    self.box.log.write_text("")
                    self.linux(arch=arch, glibc=glibc)
                    self.assert_download_attempted(target)

    def test_glibc_detected_by_ldd_alone_reaches_download(self) -> None:
        self.linux()
        self.box.ldd(GLIBC_BANNER.format(v="2.35"))
        self.assert_download_attempted("x86_64-unknown-linux-gnu")

    def test_macos_14_is_refused(self) -> None:
        self.box.uname("Darwin", "arm64")
        self.box.sw_vers("14.6")
        self.assert_refused("macOS 15.0 or newer", "found macOS 14.6")

    def test_macos_without_sw_vers_is_refused(self) -> None:
        self.box.uname("Darwin", "arm64")
        self.assert_refused("unable to detect the macOS version")

    def test_unsupported_macos_architecture_is_refused(self) -> None:
        self.box.uname("Darwin", "powerpc")
        self.box.sw_vers("15.0")
        self.assert_refused("unsupported architecture: macOS powerpc")

    def test_supported_macos_reaches_versioned_download(self) -> None:
        for version in ("15.0", "15.10", "26.1"):
            for arch, target in (("arm64", "aarch64-apple-darwin"), ("x86_64", "x86_64-apple-darwin")):
                with self.subTest(version=version, arch=arch):
                    self.box.log.write_text("")
                    self.box.uname("Darwin", arch)
                    self.box.sw_vers(version)
                    self.assert_download_attempted(target)

    def test_help_still_works_on_unsupported_platform(self) -> None:
        self.box.uname("Linux", "riscv64")
        self.box.script.write_text(hardened().replace('download_binary_and_run_installer "$@" || exit 1\n',
                                                      'download_binary_and_run_installer --help || exit 1\n'), encoding="utf-8")
        result = self.box.run()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("USAGE:", result.stdout)

    def test_quiet_refusal_still_explains(self) -> None:
        self.linux(glibc="2.34")
        self.box.script.write_text(hardened().replace('download_binary_and_run_installer "$@" || exit 1\n',
                                                      'download_binary_and_run_installer --quiet || exit 1\n'), encoding="utf-8")
        self.assert_refused("found glibc 2.34")


def sha256_line(directory: Path, name: str) -> str:
    """Test fixture generation only: the checksum dist would have written."""
    digest = subprocess.run(["shasum", "-a", "256", name], cwd=directory, capture_output=True, text=True, check=True).stdout.split()[0]
    return f"{digest} *{name}\n"


class CheckSumsTest(unittest.TestCase):
    ARCHIVES = ("pkg-aarch64-apple-darwin.tar.xz", "pkg-x86_64-unknown-linux-gnu.tar.xz")

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self._tmp.name)
        lines = []
        for name in self.ARCHIVES:
            (self.dir / name).write_bytes(name.encode() * 3)
            line = sha256_line(self.dir, name)
            (self.dir / f"{name}.sha256").write_text(line)
            lines.append(line)
        (self.dir / "sha256.sum").write_text("".join(lines))
        (self.dir / "pkg-installer.sh").write_text("#!/bin/sh\n")

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_control_verifies_every_entry(self) -> None:
        report = installers.check_sums(self.dir)
        self.assertEqual([v["name"] for v in report["verified"]], list(self.ARCHIVES))
        self.assertEqual(report["uncovered"], ["pkg-installer.sh"])

    def test_cli_reports_json(self) -> None:
        result = subprocess.run([sys.executable, str(Path(_HERE) / "installers.py"), "check-sums", str(self.dir)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('"uncovered"', result.stdout)

    def test_cli_non_utf8_sum_file_is_located(self) -> None:
        (self.dir / "sha256.sum").write_bytes(b"\xff\xfe\n")
        err = StringIO()
        with redirect_stdout(StringIO()), redirect_stderr(err):
            code = installers.main(["check-sums", str(self.dir)])
        self.assertEqual(code, 1)
        self.assertIn(f"{(self.dir / 'sha256.sum').as_posix()}: cannot read", err.getvalue())

    def test_stale_bytes_fail(self) -> None:
        (self.dir / self.ARCHIVES[0]).write_bytes(b"rebuilt")
        with self.assertRaisesRegex(installers.InstallerError, f"stale.*{re.escape(self.ARCHIVES[0])}: FAILED"):
            installers.check_sums(self.dir)

    def test_stale_sidecar_fails(self) -> None:
        sidecar = self.dir / f"{self.ARCHIVES[0]}.sha256"
        sidecar.write_text("0" * 64 + f" *{self.ARCHIVES[0]}\n")
        with self.assertRaisesRegex(installers.InstallerError, "disagrees with sha256.sum"):
            installers.check_sums(self.dir)

    def test_stale_unified_line_fails(self) -> None:
        unified = self.dir / "sha256.sum"
        unified.write_text(unified.read_text().replace(unified.read_text()[:64], "f" * 64, 1))
        with self.assertRaisesRegex(installers.InstallerError, "disagrees with sha256.sum"):
            installers.check_sums(self.dir)

    def test_consistently_stale_entries_fail_against_bytes(self) -> None:
        name = self.ARCHIVES[1]
        stale = "e" * 64 + f" *{name}\n"
        (self.dir / f"{name}.sha256").write_text(stale)
        unified = self.dir / "sha256.sum"
        unified.write_text(re.sub(rf"^[0-9a-f]{{64}} \*{re.escape(name)}$", stale.rstrip("\n"), unified.read_text(), flags=re.M))
        with self.assertRaisesRegex(installers.InstallerError, f"{re.escape(name)}: FAILED"):
            installers.check_sums(self.dir)

    def test_missing_referenced_file_fails(self) -> None:
        (self.dir / self.ARCHIVES[0]).unlink()
        with self.assertRaisesRegex(installers.InstallerError, "not a regular file"):
            installers.check_sums(self.dir)

    def test_sidecar_naming_another_file_fails(self) -> None:
        first, second = self.ARCHIVES
        (self.dir / f"{first}.sha256").write_text((self.dir / f"{second}.sha256").read_text())
        with self.assertRaisesRegex(installers.InstallerError, f"exactly one entry for '{re.escape(first)}'"):
            installers.check_sums(self.dir)

    def test_archive_without_entry_fails(self) -> None:
        (self.dir / "pkg-x86_64-apple-darwin.tar.xz").write_bytes(b"unlisted")
        with self.assertRaisesRegex(installers.InstallerError, "no entry for archive"):
            installers.check_sums(self.dir)

    def test_entry_without_sidecar_fails(self) -> None:
        (self.dir / f"{self.ARCHIVES[0]}.sha256").unlink()
        with self.assertRaisesRegex(installers.InstallerError, "sidecar is missing"):
            installers.check_sums(self.dir)

    def test_dist_output_does_not_checksum_the_installer(self) -> None:
        """dist's real `sha256.sum` lists archives only, so hardening the
        installer cannot make a dist checksum stale."""
        names = [line.split()[-1].lstrip("*") for line in (FIXTURES / "dist-sha256.sum").read_text().splitlines() if line.strip()]
        self.assertEqual(names, ["biscuit-hash-cli-aarch64-apple-darwin.tar.xz"])

    def test_dist_trailing_blank_line_is_accepted(self) -> None:
        """dist ends `sha256.sum` and each sidecar with a blank line
        (`fixtures/dist-sha256.sum`)."""
        for path in [self.dir / "sha256.sum", *self.dir.glob("*.sha256")]:
            path.write_text(path.read_text() + "\n")
        installers.check_sums(self.dir)


if __name__ == "__main__":
    unittest.main()

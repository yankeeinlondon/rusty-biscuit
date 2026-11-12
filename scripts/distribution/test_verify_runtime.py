#!/usr/bin/env python3
"""Fast tests for `verify_runtime.py`: no Docker, no network.

Archives, sidecars, and installers are stubs built in temporary directories;
the native install routes run real `sh`, `tar`, and `shasum` on them.
"""

from __future__ import annotations

import io
import json
import subprocess
import tarfile
import tempfile
import unittest
from pathlib import Path

import verify_runtime as vr

CONFIG = json.loads(vr.BUILD_IMAGES.read_text(encoding="utf-8"))
PACKAGE, COMMAND, VERSION = "biscuit-hash-cli", "bh", "0.1.0"
TARGET = "aarch64-apple-darwin"
ROOT = f"{PACKAGE}-{TARGET}"
ARCHIVE = f"{ROOT}.tar.xz"
BUILD_HOST = {"os": "Darwin", "arch": "aarch64", "build_tools": ["developer tools (/Library/Developer)"],
              "wsl2": False, "uname": "Darwin test arm64"}

STUB_INSTALLER = """\
#!/bin/sh
APP_NAME="{package}"
APP_VERSION="{version}"
biscuit_distribution_preflight() {{ :; }}
case "$1" in
    "{archive}")
            _arch="{target}"
{checksum}            ;;
esac
biscuit_distribution_preflight
src="${{BISCUIT_HASH_CLI_DOWNLOAD_URL#file://}}/{archive}"
dir=$(mktemp -d) || exit 1
tar -xf "$src" -C "$dir" || exit 1
cp "$dir/{root}/{command}" "$BISCUIT_HASH_CLI_UNMANAGED_INSTALL/{command}" || exit 1
rm -rf "$dir"
{extra}
"""


def no_libraries(ctx, target, binary, env):
    return {}, []


class Fixture:
    """An artifacts directory as the Package job leaves it, plus scratch and out."""

    def __init__(self, base: Path) -> None:
        self.base = base
        self.artifacts = base / "artifacts"
        self.scratch = base / "scratch"
        self.out = base / "out"
        self.artifacts.mkdir()
        self.scratch.mkdir()

    @property
    def archive(self) -> Path:
        return self.artifacts / ARCHIVE

    def write_archive(self, executables: dict[str, str] | None = None, notices: bool = True) -> None:
        executables = executables if executables is not None else {
            COMMAND: f'#!/bin/sh\ncase "$1" in --version) echo "{COMMAND} {VERSION}";; *) echo usage;; esac\n'}
        with tarfile.open(self.archive, "w:xz") as archive:
            files = [(name, body, 0o755) for name, body in executables.items()]
            if notices:
                files += [(name, f"{name}\n", 0o644) for name in vr.inspect_archive.NOTICE_FILES]
            for name, body, mode in files:
                data = body.encode()
                info = tarfile.TarInfo(f"{ROOT}/{name}")
                info.size, info.mode = len(data), mode
                archive.addfile(info, io.BytesIO(data))

    def write_sums(self) -> str:
        line = subprocess.run(["shasum", "-a", "256", "-b", ARCHIVE], cwd=self.artifacts, check=True,
                              capture_output=True, text=True).stdout
        (self.artifacts / f"{ARCHIVE}.sha256").write_text(line, encoding="utf-8")
        (self.artifacts / "sha256.sum").write_text(line, encoding="utf-8")
        return line.split()[0]

    def write_installer(self, digest: str | None, extra: str = "", version: str = VERSION) -> None:
        checksum = f'            _checksum_value="{digest}"\n' if digest else ""
        (self.artifacts / f"{PACKAGE}-installer.sh").write_text(
            STUB_INSTALLER.format(package=PACKAGE, version=version, archive=ARCHIVE, target=TARGET, root=ROOT,
                                  command=COMMAND, checksum=checksum, extra=extra), encoding="utf-8")

    def complete(self, extra: str = "") -> str:
        self.write_archive()
        digest = self.write_sums()
        self.write_installer(digest, extra)
        return digest

    def verify(self, command: str = COMMAND, host=BUILD_HOST) -> dict:
        return vr.verify(artifacts=self.artifacts, package=PACKAGE, command=command, version=VERSION, out=self.out,
                         targets=[TARGET], config=CONFIG, host=host, docker=None, docker_arch=None, channels=[],
                         scratch=self.scratch, observe=no_libraries)

    def assert_nothing_installed(self, case: unittest.TestCase) -> None:
        case.assertEqual(list(self.scratch.iterdir()), [], "an install root was created")
        case.assertFalse(self.out.exists(), "a report directory was created")


class FixtureCase(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.fx = Fixture(Path(self._tmp.name))

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def checks(self, report: dict) -> dict[str, dict]:
        return {check["route"]: check for check in report["checks"] if check["target"] == TARGET}


class EvidenceClassification(unittest.TestCase):
    def test_macos_host_with_tools_is_build_host_never_clean(self) -> None:
        self.assertEqual(vr.classify_macos(BUILD_HOST, "aarch64")[0], "native-build-host")
        plan = vr.plan_targets(CONFIG, [TARGET], BUILD_HOST, None)[TARGET]
        self.assertIn(("clean-runtime", "pending-infrastructure"), [(r, c) for r, c, _ in plan])

    def test_macos_host_without_tools_is_clean_runtime(self) -> None:
        clean = {**BUILD_HOST, "build_tools": []}
        self.assertEqual(vr.classify_macos(clean, "aarch64")[0], "native-clean-runtime")
        routes = [route for route, _, _ in vr.plan_targets(CONFIG, [TARGET], clean, None)[TARGET]]
        self.assertNotIn("clean-runtime", routes)

    def test_other_cpu_or_os_is_pending_not_emulated_pass(self) -> None:
        self.assertEqual(vr.classify_macos(BUILD_HOST, "x86_64")[0], "pending-infrastructure")
        self.assertEqual(vr.classify_macos({**BUILD_HOST, "os": "Linux"}, "aarch64")[0], "pending-infrastructure")

    def test_linux_container_native_only_on_matching_engine(self) -> None:
        self.assertEqual(vr.classify_linux_container("aarch64", "aarch64")[0], "native-clean-runtime")
        cls, reason = vr.classify_linux_container("aarch64", "x86_64")
        self.assertEqual(cls, "pending-infrastructure")
        self.assertIn("emulated", reason)
        self.assertEqual(vr.classify_linux_container(None, "aarch64")[0], "pending-infrastructure")

    def test_wsl2_is_labeled_separately_from_native_windows(self) -> None:
        cls, reason = vr.classify_wsl2(BUILD_HOST, "aarch64")
        self.assertEqual(cls, "pending-infrastructure")
        self.assertIn("not native Windows", reason)
        wsl = {**BUILD_HOST, "os": "Linux", "arch": "x86_64", "wsl2": True}
        self.assertEqual(vr.classify_wsl2(wsl, "x86_64")[0], "wsl2")
        self.assertEqual(vr.classify_wsl2(wsl, "aarch64")[0], "pending-infrastructure")

    def test_linux_plan_has_refusal_runtime_and_wsl2_routes(self) -> None:
        plan = vr.plan_targets(CONFIG, ["x86_64-unknown-linux-gnu"], BUILD_HOST, "aarch64")
        routes = {route: cls for route, cls, _ in plan["x86_64-unknown-linux-gnu"]}
        self.assertEqual(set(routes), {"glibc-refusal", "archive", "installer", "wsl2-archive", "wsl2-installer"})
        self.assertEqual(set(routes.values()), {"pending-infrastructure"})


class PendingAndDisabledNeverPass(unittest.TestCase):
    def test_check_rejects_pass_for_pending_or_disabled(self) -> None:
        for cls in ("pending-infrastructure", "disabled"):
            with self.assertRaises(ValueError):
                vr.Check("t", "r", cls, "pass", "reason")
        with self.assertRaises(ValueError):
            vr.Check("t", "r", "native-build-host", "pending", "reason")
        with self.assertRaises(ValueError):
            vr.Check("t", "r", "pending-infrastructure", "pending", "")
        with self.assertRaises(ValueError):
            vr.observed("t", "r", "disabled", [], {})

    def test_summary_of_only_pending_and_disabled_is_incomplete(self) -> None:
        checks = [vr.pending("t", "archive", "no host"), *vr.channel_checks(PACKAGE, {}, "policy")]
        summary = vr.summarize(checks)
        self.assertEqual((summary["result"], summary["passed"], summary["pending"], summary["disabled"]),
                         ("incomplete", 0, 1, 2))
        self.assertEqual(vr.exit_status(summary), 0)

    def test_a_failure_fails_and_disabled_does_not_block_a_pass(self) -> None:
        passed = vr.observed("t", "archive", "native-clean-runtime", [], {})
        failed = vr.observed("t", "installer", "native-clean-runtime", ["boom"], {})
        disabled = vr.channel_checks(PACKAGE, {}, "policy")
        self.assertEqual(vr.summarize([passed, *disabled])["result"], "pass")
        summary = vr.summarize([passed, failed, *disabled])
        self.assertEqual((summary["result"], vr.exit_status(summary)), ("fail", 1))

    def test_channels_disabled_by_policy_and_enabled_ones_pending(self) -> None:
        checks = {c.route: c for c in vr.channel_checks(PACKAGE, {"brew": "bh"}, "docs/deployment.yaml")}
        self.assertEqual(checks["channel:binstall"].status, "disabled")
        self.assertIn("packages.biscuit-hash-cli.binstall", checks["channel:binstall"].reason)
        self.assertEqual(checks["channel:brew"].status, "pending")

    def test_combined_report_keeps_pending_and_fails_on_binstall(self) -> None:
        runtime = {"package": PACKAGE, "version": VERSION, "host": BUILD_HOST, "docker_arch": None, "downloads": {},
                   "summary": vr.summarize([vr.pending("t", "r", "why")]),
                   "checks": [vr.asdict(vr.pending("t", "r", "why"))]}
        report, readme = vr.combine(runtime, {"status": "passed", "binstall": {"version": "1.26.2"}})
        self.assertEqual(report["summary"]["result"], "incomplete")
        self.assertIn("| t | r | pending-infrastructure | pending | why |", readme)
        report, _ = vr.combine(runtime, {"status": "failed", "errors": ["x"]})
        self.assertEqual((report["summary"]["result"], vr.exit_status(report["summary"])), ("fail", 1))


class LibraryLeakage(unittest.TestCase):
    def test_unresolved_library_is_refused(self) -> None:
        text = "\tlinux-vdso.so.1 (0x1)\n\tlibc.so.6 => /lib/libc.so.6 (0x2)\n\tlibasound.so.2 => not found\n"
        _, problems = vr.ldd_problems(text, CONFIG, PACKAGE)
        self.assertTrue(any("libasound.so.2 was not found" in p for p in problems), problems)

    def test_library_outside_allowed_set_is_refused(self) -> None:
        text = "\tlibc.so.6 => /lib/libc.so.6 (0x2)\n\tlibssl.so.3 => /usr/lib/libssl.so.3 (0x3)\n"
        _, problems = vr.ldd_problems(text, CONFIG, PACKAGE)
        self.assertEqual(len(problems), 1)
        self.assertIn("libssl.so.3", problems[0])

    def test_allowed_libraries_pass(self) -> None:
        text = ("\tlinux-vdso.so.1 (0x1)\n\tlibc.so.6 => /lib/aarch64-linux-gnu/libc.so.6 (0x2)\n"
                "\t/lib/ld-linux-aarch64.so.1 (0x3)\n\tlibgcc_s.so.1 => /lib/aarch64-linux-gnu/libgcc_s.so.1 (0x4)\n")
        self.assertEqual(vr.ldd_problems(text, CONFIG, PACKAGE)[1], [])

    def test_macos_runtime_load_from_homebrew_is_refused(self) -> None:
        text = ("dyld[1]: <A> /tmp/x/bh\ndyld[1]: <B> /usr/lib/libSystem.B.dylib\n"
                "dyld[1]: <C> /opt/homebrew/lib/libpcre2-8.0.dylib\n")
        loaded, problems = vr.dyld_problems(text, "/tmp/x/bh", CONFIG)
        self.assertEqual(len(loaded), 3)
        self.assertEqual(problems, ["loaded /opt/homebrew/lib/libpcre2-8.0.dylib, outside "
                                    f"{list(vr.MACOS_RUNTIME_PREFIXES)}"])
        self.assertEqual(vr.dyld_problems("", "/tmp/x/bh", CONFIG)[1], ["dyld reported no loaded libraries"])


class InstalledTreeAudits(unittest.TestCase):
    def test_exactly_the_command(self) -> None:
        self.assertEqual(vr.audit_tree([("bh", "f", True)], {"bh": True}), [])

    def test_extra_file_directory_and_wrong_modes(self) -> None:
        problems = vr.audit_tree([("bh", "f", False), ("bh-helper", "f", True), ("share", "d", False)], {"bh": True})
        self.assertEqual(problems, ["bh is not executable", "extra file bh-helper", "extra directory share"])
        self.assertEqual(vr.audit_tree([], {"bh": True}), ["missing bh"])
        self.assertEqual(vr.audit_tree([("r", "d", False), ("r/LICENSE", "f", True)], {"r/LICENSE": False}),
                         ["r/LICENSE is unexpectedly executable"])

    def test_profile_writes_and_other_home_files(self) -> None:
        entries = vr.parse_find_listing(".zshrc\tf\t644\n.config\td\t755\n.config/fish\td\t755\n"
                                        ".config/fish/conf.d\td\t755\n.config/fish/conf.d/bh.fish\tf\t644\n"
                                        "notes\tf\t600\n")
        problems = vr.audit_home(entries)
        self.assertIn("wrote shell profile .zshrc; an isolated install never edits PATH", problems)
        self.assertIn("wrote shell profile .config/fish/conf.d/bh.fish; an isolated install never edits PATH",
                      problems)
        self.assertIn("wrote notes under HOME", problems)
        self.assertNotIn("wrote .config under HOME", problems)

    def test_version_output_must_name_command_and_version(self) -> None:
        self.assertEqual(vr.command_problems("bh", "0.1.0", "--version", 0, "bh 0.1.0\n"), [])
        self.assertEqual(vr.command_problems("bh", "0.1.0", "--version", 0, "md 0.1.0\n"),
                         ["bh --version printed 'md 0.1.0', expected 'bh 0.1.0'"])
        self.assertEqual(vr.command_problems("bh", "0.1.0", "--help", None, ""), ["bh --help exited timeout"])
        self.assertEqual(vr.command_problems("bh", "0.1.0", "--help", 0, " \n"), ["bh --help printed nothing"])


class GlibcRefusal(unittest.TestCase):
    def write(self, out: Path, status: int, output: str, root: str = "", home: str = "", tmp: str = "") -> None:
        (out / "install.status").write_text(f"{status}\n")
        (out / "install.out").write_text(output)
        for name, text in (("root", root), ("home", home), ("tmp", tmp)):
            (out / f"{name}.tsv").write_text(text)

    def test_refusal_before_mutation(self) -> None:
        refusal = "ERROR: x 0.1.0 cannot be installed: requires glibc 2.35 or newer; found glibc 2.31\n"
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            self.write(out, 1, refusal)
            self.assertEqual(vr.refusal_problems(out), [])
            self.write(out, 1, refusal, tmp="tmp.abc\td\t700\n")
            self.assertEqual(len(vr.refusal_problems(out)), 1)
            self.write(out, 0, "everything's installed!\n", root="bh\tf\t755\n")
            self.assertEqual(len(vr.refusal_problems(out)), 3)
            self.write(out, 1, "need 'curl' or 'wget'\n")
            self.assertEqual(len(vr.refusal_problems(out)), 1)


class MalformedInputs(FixtureCase):
    def assert_located(self, where: Path | str, fragment: str, command: str = COMMAND) -> None:
        with self.assertRaises(vr.VerifyError) as raised:
            self.fx.verify(command=command)
        self.assertEqual(raised.exception.where, str(where))
        self.assertIn(fragment, str(raised.exception))
        self.fx.assert_nothing_installed(self)

    def test_missing_archive(self) -> None:
        self.fx.complete()
        self.fx.archive.unlink()
        self.assert_located(self.fx.archive, "does not exist")

    def test_missing_installer(self) -> None:
        self.fx.complete()
        (self.fx.artifacts / f"{PACKAGE}-installer.sh").unlink()
        self.assert_located(self.fx.artifacts / f"{PACKAGE}-installer.sh", "does not exist")

    def test_checksum_mismatch_found_by_shasum(self) -> None:
        self.fx.complete()
        self.fx.write_archive(notices=False)
        self.assert_located(self.fx.artifacts / f"{ARCHIVE}.sha256", "shasum -a 256 -c failed")

    def test_sha256_sum_disagrees_with_sidecar(self) -> None:
        digest = self.fx.complete()
        (self.fx.artifacts / "sha256.sum").write_text(f"{'0' * 64} *{ARCHIVE}\n")
        self.assertNotEqual(digest, "0" * 64)
        self.assert_located(self.fx.artifacts / "sha256.sum", "lists 0000")

    def test_installer_embeds_another_checksum(self) -> None:
        self.fx.complete()
        self.fx.write_installer("f" * 64)
        self.assert_located(self.fx.artifacts / f"{PACKAGE}-installer.sh", f"embeds {'f' * 64}")

    def test_wrong_command_for_package(self) -> None:
        self.fx.complete()
        self.assert_located("--command", "ships 'bh', not 'md'", command="md")

    def test_archive_without_the_command(self) -> None:
        self.fx.write_archive(executables={"md": "#!/bin/sh\n"})
        self.fx.write_installer(self.fx.write_sums())
        self.assert_located(self.fx.archive, "has no executable")

    def test_installer_for_another_version_or_unhardened(self) -> None:
        self.fx.complete()
        self.fx.write_installer(None, version="0.2.0")
        self.assert_located(self.fx.artifacts / f"{PACKAGE}-installer.sh", "APP_VERSION is '0.2.0'")
        installer = self.fx.artifacts / f"{PACKAGE}-installer.sh"
        self.fx.write_installer(None)
        installer.write_text(installer.read_text().replace("biscuit_distribution_preflight", "noop"))
        self.assert_located(installer, "no platform preflight")


class NativeInstalls(FixtureCase):
    def test_archive_and_installer_pass_with_exact_identity(self) -> None:
        digest = self.fx.complete()
        report = self.fx.verify()
        checks = self.checks(report)
        self.assertEqual({r: c["status"] for r, c in checks.items()},
                         {"archive": "pass", "installer": "pass", "clean-runtime": "pending"})
        self.assertEqual(checks["installer"]["evidence_class"], "native-build-host")
        self.assertEqual(checks["installer"]["details"]["installed"], ["bh"])
        identity = report["downloads"][TARGET]
        self.assertEqual((identity["sha256"], identity["installer_embedded_sha256"]), (digest, digest))
        self.assertEqual(identity["size"], self.fx.archive.stat().st_size)
        self.assertEqual(report["summary"]["result"], "incomplete")
        self.assertTrue((self.fx.out / vr.REPORT_NAME).is_file())
        self.assertEqual(list(self.fx.scratch.iterdir()), [], "scratch roots are removed")

    def test_installer_writing_a_profile_fails(self) -> None:
        self.fx.complete(extra='echo "export PATH=x" >> "$HOME/.zshrc"')
        checks = self.checks(self.fx.verify())
        self.assertEqual(checks["installer"]["status"], "fail")
        self.assertIn("wrote shell profile .zshrc", checks["installer"]["reason"])
        self.assertEqual(checks["archive"]["status"], "pass")

    def test_installer_installing_an_extra_file_fails(self) -> None:
        self.fx.complete(extra='touch "$BISCUIT_HASH_CLI_UNMANAGED_INSTALL/bh-update"')
        checks = self.checks(self.fx.verify())
        self.assertEqual(checks["installer"]["status"], "fail")
        self.assertIn("extra file bh-update", checks["installer"]["reason"])

    def test_failing_installer_fails(self) -> None:
        self.fx.complete(extra="exit 7")
        checks = self.checks(self.fx.verify())
        self.assertIn("the installer exited 7", checks["installer"]["reason"])

    def test_archive_with_extra_executable_fails(self) -> None:
        self.fx.write_archive(executables={COMMAND: "#!/bin/sh\necho bh 0.1.0\n", "bh-helper": "#!/bin/sh\n"})
        self.fx.write_installer(self.fx.write_sums())
        checks = self.checks(self.fx.verify())
        self.assertEqual(checks["archive"]["status"], "fail")
        self.assertIn(f"extra file {ROOT}/bh-helper", checks["archive"]["reason"])

    def test_pending_host_installs_nothing(self) -> None:
        self.fx.complete()
        report = self.fx.verify(host={**BUILD_HOST, "arch": "x86_64"})
        self.assertEqual({c["status"] for c in report["checks"]}, {"pending"})
        self.assertEqual(report["downloads"], {})
        self.assertEqual(list(self.fx.scratch.iterdir()), [])


if __name__ == "__main__":
    unittest.main()

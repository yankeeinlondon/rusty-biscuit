#!/usr/bin/env python3
"""Tests for `inspect_archive.py` and the `build-images.json` contract.

Run: /tmp/dist-venv/bin/python scripts/distribution/test_inspect.py
(the contract test imports `scripts/ci/distribution.py`, which needs the pins
in `scripts/ci/requirements-distribution.txt`). No network, Docker, or cargo:
archives are built in temporary directories around synthetic ELF/Mach-O
headers, and linkage checks read trimmed real `readelf`/`otool` output from
`fixtures/`.
"""

from __future__ import annotations

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))

import copy  # noqa: E402
import io  # noqa: E402
import json  # noqa: E402
import re  # noqa: E402
import struct  # noqa: E402
import subprocess  # noqa: E402
import tarfile  # noqa: E402
import tempfile  # noqa: E402
import unittest  # noqa: E402
import uuid  # noqa: E402
from contextlib import redirect_stderr, redirect_stdout  # noqa: E402
from pathlib import Path  # noqa: E402

import inspect_archive as inspect_mod  # noqa: E402

REPO = Path(HERE).parents[1]
sys.path.insert(0, str(REPO / "scripts" / "ci"))
import distribution  # noqa: E402

FIXTURES = Path(HERE) / "fixtures"
CONFIG_PATH = REPO / ".github" / "distribution" / "build-images.json"
CONFIG = inspect_mod.load_config(CONFIG_PATH)
TAG = "biscuit-hash-cli-v0.1.0"
COMMIT = "0123456789abcdef0123456789abcdef01234567"
SOURCE = (f"biscuit-hash-cli 0.1.0 (bh)\nSource: {inspect_mod.CANONICAL_URL}/tree/{TAG}/biscuit-hash/cli\n"
          f"Commit: {inspect_mod.CANONICAL_URL}/commit/{COMMIT}\n")


def macho(cputype: int = 0x0100000C, magic: bytes = inspect_mod.MACHO_64_LE) -> bytes:
    return magic + struct.pack("<iIIIII", cputype, 0, 2, 0, 0, 0) + b"\0" * 64


def elf(machine: int = 183, elf_class: int = 2) -> bytes:
    ident = b"\x7fELF" + bytes([elf_class, 1, 1, 0]) + b"\0" * 8
    return ident + struct.pack("<HHI", 3, machine, 1) + b"\0" * 64


def fixture(name: str) -> str:
    return (FIXTURES / name).read_text(encoding="utf-8")


class Archive:
    """A tar.xz built member by member; `default` gives a valid archive."""

    def __init__(self, package: str = "biscuit-hash-cli", target: str = "aarch64-apple-darwin",
                 command: str = "bh", binary: bytes | None = None) -> None:
        self.root = f"{package}-{target}"
        self.members: list[tuple[tarfile.TarInfo, bytes | None]] = []
        self.add_dir(self.root)
        self.add(command, binary if binary is not None else macho(), 0o755)
        for name, text in (("LICENSE", "AGPL-3.0-only\n"), ("README.md", "# bh\n"), ("SOURCE.txt", SOURCE),
                           ("THIRD-PARTY-NOTICES.md", "# Notices\n")):
            self.add(name, text.encode())

    def add_dir(self, name: str) -> None:
        info = tarfile.TarInfo(name)
        info.type, info.mode = tarfile.DIRTYPE, 0o755
        self.members.append((info, None))

    def add(self, name: str, data: bytes, mode: int = 0o644, *, rooted: bool = True) -> None:
        info = tarfile.TarInfo(f"{self.root}/{name}" if rooted else name)
        info.size, info.mode = len(data), mode
        self.members.append((info, data))

    def link(self, name: str, target: str, kind: bytes = tarfile.SYMTYPE) -> None:
        info = tarfile.TarInfo(f"{self.root}/{name}")
        info.type, info.linkname = kind, target
        self.members.append((info, None))

    def drop(self, name: str) -> None:
        self.members = [(info, data) for info, data in self.members if info.name != f"{self.root}/{name}"]

    def write(self, directory: Path) -> Path:
        path = directory / f"{self.root}.tar.xz"
        with tarfile.open(path, "w:xz", format=tarfile.PAX_FORMAT) as tar:
            for info, data in self.members:
                tar.addfile(info, io.BytesIO(data) if data is not None else None)
        return path


class InspectCase(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.dir = Path(self.tmp.name)

    def structure(self, archive: Archive, *, target: str = "aarch64-apple-darwin", command: str = "bh",
                  package: str = "biscuit-hash-cli") -> inspect_mod.Report:
        report = inspect_mod.Report("archive")
        binary = inspect_mod.check_archive(archive.write(self.dir), package, command, target, TAG, report)
        if binary is not None:
            inspect_mod.check_binary_header(binary, inspect_mod.target_settings(CONFIG, target), report, command)
        return report

    def assertFails(self, report: inspect_mod.Report, fragment: str) -> None:
        reasons = [f"{f['where']}: {f['reason']}" for f in report.failures]
        self.assertTrue(any(fragment in reason for reason in reasons), f"no failure mentions {fragment!r}: {reasons}")


class ArchiveStructureTests(InspectCase):
    def test_control_archive_passes(self) -> None:
        report = self.structure(Archive())
        self.assertEqual(report.failures, [])
        self.assertEqual(report.data["executables"], ["biscuit-hash-cli-aarch64-apple-darwin/bh"])

    def test_a_package_changelog_is_accepted_but_no_other_document(self) -> None:
        archive = Archive()
        archive.add("CHANGELOG.md", b"# Changelog\n")
        self.assertEqual(self.structure(archive).failures, [])
        archive.add("CHANGES.md", b"# Changes\n")
        self.assertFails(self.structure(archive), "is not one of the expected entries")
        executable = Archive()
        executable.add("CHANGELOG.md", b"# Changelog\n", 0o755)
        self.assertFails(self.structure(executable), "only executable")

    def test_parent_traversal_is_rejected(self) -> None:
        archive = Archive()
        archive.add("../escape", b"x")
        self.assertFails(self.structure(archive), "'..'")

    def test_absolute_path_is_rejected(self) -> None:
        archive = Archive()
        archive.add("/etc/passwd", b"x", rooted=False)
        self.assertFails(self.structure(archive), "absolute")

    def test_symlink_and_hardlink_are_rejected(self) -> None:
        archive = Archive()
        archive.link("escape", "../../etc/passwd")
        archive.link("copy", "biscuit-hash-cli-aarch64-apple-darwin/bh", tarfile.LNKTYPE)
        report = self.structure(archive)
        self.assertFails(report, "is a symlink")
        self.assertFails(report, "is a hardlink")

    def test_entry_outside_the_root_is_rejected(self) -> None:
        archive = Archive()
        archive.add("other-root/README.md", b"x", rooted=False)
        self.assertFails(self.structure(archive), "outside the single archive root")

    def test_extra_executable_is_rejected(self) -> None:
        archive = Archive()
        archive.add("render_dirty_tree", b"#!/bin/sh\n", 0o755)
        self.assertFails(self.structure(archive), "only executable")

    def test_executable_content_without_mode_bit_is_rejected(self) -> None:
        archive = Archive()
        archive.add("helper", macho(), 0o644)
        self.assertFails(self.structure(archive), "only executable")

    def test_command_without_executable_bit_is_rejected(self) -> None:
        archive = Archive()
        archive.drop("bh")
        archive.add("bh", macho(), 0o644)
        self.assertFails(self.structure(archive), "not marked executable")

    def test_duplicate_entry_is_rejected(self) -> None:
        archive = Archive()
        archive.add("LICENSE", b"other\n")
        self.assertFails(self.structure(archive), "more than once")

    def test_missing_and_empty_notices_are_rejected(self) -> None:
        for name in ("LICENSE", "README.md", "SOURCE.txt", "THIRD-PARTY-NOTICES.md"):
            with self.subTest(missing=name):
                archive = Archive()
                archive.drop(name)
                self.assertFails(self.structure(archive), f"{name}': is missing")
            with self.subTest(empty=name):
                archive = Archive()
                archive.drop(name)
                archive.add(name, b" \n")
                self.assertFails(self.structure(archive), f"{name}': is empty")

    def test_unsafe_members_are_never_written_and_launch_no_tool(self) -> None:
        """Absolute, `..`, and escaping-symlink members fail without being
        extracted, and the inspection tools never run on such an archive."""
        marker = f"inspect-escape-{uuid.uuid4().hex}"
        absolute = Path(tempfile.gettempdir()) / marker
        cases = {
            "absolute": lambda a: a.add(str(absolute), b"x", rooted=False),
            "parent": lambda a: a.add(f"../{marker}", b"x"),
            "symlink": lambda a: a.link(marker, f"../../{marker}"),
            "duplicate": lambda a: a.add("README.md", b"# again\n"),
        }
        for name, mutate in cases.items():
            with self.subTest(name):
                archive = Archive()
                mutate(archive)
                calls: list = []
                report = inspect_mod.inspect_archive(archive.write(self.dir), "biscuit-hash-cli", "bh", "aarch64-apple-darwin",
                                                     TAG, CONFIG, runner=lambda argv: calls.append(argv) or "")
                self.assertFalse(report.as_json()["passed"])
                self.assertEqual(calls, [])
                self.assertFalse(absolute.exists())
                self.assertFalse((self.dir.parent / marker).exists())

    def test_non_utf8_source_is_rejected(self) -> None:
        archive = Archive()
        archive.drop("SOURCE.txt")
        archive.add("SOURCE.txt", SOURCE.encode() + b"\xff\n")
        self.assertFails(self.structure(archive), "SOURCE.txt': is not UTF-8")

    def test_source_without_tag_or_commit_url_is_rejected(self) -> None:
        cases = {
            "no tree URL": (SOURCE.replace(f"/tree/{TAG}", "/tree/main"), "exact tagged source URL"),
            "tag prefix only": (SOURCE.replace(f"/tree/{TAG}/", f"/tree/{TAG}0/"), "exact tagged source URL"),
            "no commit URL": (SOURCE.replace(f"/commit/{COMMIT}", "/commit/abc123"), "40-hex commit URL"),
        }
        for label, (text, fragment) in cases.items():
            with self.subTest(label):
                archive = Archive()
                archive.drop("SOURCE.txt")
                archive.add("SOURCE.txt", text.encode())
                self.assertFails(self.structure(archive), fragment)

    def test_wrong_elf_machine_is_rejected(self) -> None:
        archive = Archive(target="aarch64-unknown-linux-gnu", binary=elf(machine=62))
        self.assertFails(self.structure(archive, target="aarch64-unknown-linux-gnu"), "built for x86_64, expected aarch64")

    def test_32_bit_elf_is_rejected(self) -> None:
        archive = Archive(target="x86_64-unknown-linux-gnu", binary=elf(machine=62, elf_class=1))
        self.assertFails(self.structure(archive, target="x86_64-unknown-linux-gnu"), "not 64-bit")

    def test_wrong_macho_cputype_is_rejected(self) -> None:
        archive = Archive(binary=macho(cputype=0x01000007))
        self.assertFails(self.structure(archive), "built for x86_64, expected arm64")

    def test_universal_macho_is_rejected(self) -> None:
        archive = Archive(binary=b"\xca\xfe\xba\xbe" + b"\0" * 64)
        self.assertFails(self.structure(archive), "universal")

    def test_elf_in_a_macos_archive_is_rejected(self) -> None:
        self.assertFails(self.structure(Archive(binary=elf())), "is elf, expected macho")


class ElfLinkageTests(InspectCase):
    def linkage(self, readelf: str, package: str = "biscuit-hash-cli",
                fragments: tuple[str, ...] = ()) -> inspect_mod.Report:
        report = inspect_mod.Report("archive")
        inspect_mod.check_elf_linkage(readelf, CONFIG, package, report, "bh", fragments)
        return report

    def test_real_readelf_output_passes(self) -> None:
        report = self.linkage(fixture("readelf-bh-aarch64-unknown-linux-gnu.txt"))
        self.assertEqual(report.failures, [])
        linkage = report.data["linkage"]
        self.assertIn("libc.so.6", linkage["needed"])
        self.assertLessEqual(inspect_mod.version_tuple(linkage["max_glibc"]), (2, 35))

    def test_glibc_newer_than_floor_is_rejected(self) -> None:
        text = fixture("readelf-bh-aarch64-unknown-linux-gnu.txt").replace("Name: GLIBC_2.17", "Name: GLIBC_2.36", 1)
        self.assertFails(self.linkage(text), "GLIBC_2.36")

    def test_glibc_exactly_at_floor_is_accepted(self) -> None:
        text = fixture("readelf-bh-aarch64-unknown-linux-gnu.txt").replace("Name: GLIBC_2.17", "Name: GLIBC_2.35", 1)
        report = self.linkage(text)
        self.assertEqual(report.failures, [])
        self.assertEqual(report.data["linkage"]["max_glibc"], "2.35")

    def test_missing_version_needs_is_not_a_pass(self) -> None:
        text = fixture("readelf-bh-aarch64-unknown-linux-gnu.txt")
        self.assertIn("Version needs section", text)
        self.assertFails(self.linkage(text.replace("Version needs section", "Version nothing section")),
                         "lists no GLIBC_ version needs")

    def test_unnumbered_glibc_version_is_rejected(self) -> None:
        text = fixture("readelf-bh-aarch64-unknown-linux-gnu.txt").replace("Name: GLIBC_2.17", "Name: GLIBC_PRIVATE", 1)
        self.assertFails(self.linkage(text), "GLIBC_PRIVATE")

    def needing(self, library: str) -> str:
        text = fixture("readelf-bh-aarch64-unknown-linux-gnu.txt")
        line = " 0x0000000000000001 (NEEDED)             Shared library: [libc.so.6]"
        self.assertIn(line, text)
        return text.replace(line, f"{line}\n 0x0000000000000001 (NEEDED)             Shared library: [{library}]", 1)

    def test_pcre2_shared_object_is_rejected(self) -> None:
        self.assertFails(self.linkage(self.needing("libpcre2-8.so.0")), "pcre2 must be linked statically")

    def test_library_outside_allowlist_is_rejected(self) -> None:
        self.assertFails(self.linkage(self.needing("libssl.so.3")), "libssl.so.3, which is not in the allowed list")

    def test_libasound_is_allowed_for_claudine_only(self) -> None:
        text = self.needing("libasound.so.2")
        self.assertEqual(self.linkage(text, package="claudine-cli").failures, [])
        self.assertFails(self.linkage(text), "libasound.so.2")

    def test_runpath_is_rejected(self) -> None:
        text = self.needing("libc.so.6").replace(
            "Shared library: [libc.so.6]\n", "Shared library: [libc.so.6]\n 0x000000000000001d (RUNPATH)            Library runpath: [/home/runner/work/lib]\n", 1)
        report = self.linkage(text, fragments=tuple(CONFIG["forbidden_path_fragments"]))
        self.assertFails(report, "RPATH/RUNPATH")
        self.assertFails(report, "build-host path")

    def test_ldd_output(self) -> None:
        control = ("\tlinux-vdso.so.1 (0x0000ffff9b2e8000)\n"
                   "\tlibgcc_s.so.1 => /lib/aarch64-linux-gnu/libgcc_s.so.1 (0x0000ffff9b200000)\n"
                   "\tlibc.so.6 => /lib/aarch64-linux-gnu/libc.so.6 (0x0000ffff9b050000)\n"
                   "\t/lib/ld-linux-aarch64.so.1 (0x0000ffff9b2af000)\n")
        report = inspect_mod.Report("ldd")
        inspect_mod.check_ldd(control, CONFIG, "biscuit-hash-cli", report, "ldd")
        self.assertEqual(report.failures, [])
        report = inspect_mod.Report("ldd")
        inspect_mod.check_ldd(control + "\tlibasound.so.2 => not found\n", CONFIG, "claudine-cli", report, "ldd")
        self.assertFails(report, "was not found")


class MachoLinkageTests(InspectCase):
    def linkage(self, load_commands: str, libraries: str | None = None, staging: str = "/Users/x/stage") -> inspect_mod.Report:
        report = inspect_mod.Report("archive")
        fragments = [*CONFIG["forbidden_path_fragments"], staging]
        libraries = libraries if libraries is not None else fixture("otool-libraries-bh-aarch64-apple-darwin.txt")
        inspect_mod.check_macho_linkage(load_commands, libraries, CONFIG, "biscuit-hash-cli", report, "bh", fragments)
        return report

    def test_real_otool_output_passes(self) -> None:
        report = self.linkage(fixture("otool-load-commands-bh-aarch64-apple-darwin.txt"))
        self.assertEqual(report.failures, [])
        self.assertEqual(report.data["linkage"]["minos"], "15.0")
        self.assertEqual(report.data["linkage"]["dylibs"], ["/usr/lib/libiconv.2.dylib", "/usr/lib/libSystem.B.dylib"])

    def with_dylib(self, path: str) -> tuple[str, str]:
        commands = fixture("otool-load-commands-bh-aarch64-apple-darwin.txt") + (
            f"Load command 16\n          cmd LC_LOAD_DYLIB\n      cmdsize 64\n         name {path} (offset 24)\n")
        libraries = fixture("otool-libraries-bh-aarch64-apple-darwin.txt") + f"\t{path} (compatibility version 1.0.0, current version 1.0.0)\n"
        return commands, libraries

    def test_pcre2_dylib_is_rejected(self) -> None:
        self.assertFails(self.linkage(*self.with_dylib("/usr/lib/libpcre2-8.0.dylib")), "pcre2 must be linked statically")

    def test_homebrew_dependency_is_rejected(self) -> None:
        report = self.linkage(*self.with_dylib("/opt/homebrew/opt/xz/lib/liblzma.5.dylib"))
        self.assertFails(report, "outside the system locations")
        self.assertFails(report, "/opt/homebrew")

    def test_rpath_relative_and_private_framework_dependencies_are_rejected(self) -> None:
        for path in ("@rpath/libfoo.dylib", "/System/Library/PrivateFrameworks/Foo.framework/Foo"):
            with self.subTest(path):
                self.assertFails(self.linkage(*self.with_dylib(path)), "outside the system locations")

    def test_rpath_is_rejected(self) -> None:
        commands = fixture("otool-load-commands-bh-aarch64-apple-darwin.txt") + (
            "Load command 17\n          cmd LC_RPATH\n      cmdsize 48\n         path /usr/local/Cellar/pcre2/lib (offset 12)\n")
        report = self.linkage(commands)
        self.assertFails(report, "LC_RPATH")
        self.assertFails(report, "/usr/local/Cellar")

    def test_staging_root_in_load_command_is_rejected(self) -> None:
        commands, libraries = self.with_dylib("/Users/x/stage/target/dist/libbad.dylib")
        self.assertFails(self.linkage(commands, libraries), "build-host path")

    def test_minos_14_is_rejected(self) -> None:
        commands = fixture("otool-load-commands-bh-aarch64-apple-darwin.txt").replace("minos 15.0", "minos 14.0")
        self.assertFails(self.linkage(commands), "minimum macOS 14.0, expected 15.0")

    def test_missing_build_version_is_rejected(self) -> None:
        commands = fixture("otool-load-commands-bh-aarch64-apple-darwin.txt").replace("LC_BUILD_VERSION", "LC_NOTE")
        self.assertFails(self.linkage(commands), "expected one")


class DriverTests(InspectCase):
    def test_inspect_archive_with_recorded_tool_output(self) -> None:
        archive = Archive().write(self.dir)
        outputs = {"-l": fixture("otool-load-commands-bh-aarch64-apple-darwin.txt"),
                   "-L": fixture("otool-libraries-bh-aarch64-apple-darwin.txt")}
        report = inspect_mod.inspect_archive(archive, "biscuit-hash-cli", "bh", "aarch64-apple-darwin", TAG, CONFIG,
                                             evidence=self.dir / "evidence", runner=lambda argv: outputs[argv[1]])
        self.assertEqual(report.failures, [])
        self.assertEqual(sorted(p.name for p in (self.dir / "evidence").iterdir()),
                         ["otool-libraries-biscuit-hash-cli-aarch64-apple-darwin.txt",
                          "otool-load-commands-biscuit-hash-cli-aarch64-apple-darwin.txt"])

    def test_missing_tool_is_a_located_failure(self) -> None:
        archive = Archive(target="aarch64-unknown-linux-gnu", binary=elf()).write(self.dir)

        def missing(argv):
            raise FileNotFoundError(argv[0])

        report = inspect_mod.inspect_archive(archive, "biscuit-hash-cli", "bh", "aarch64-unknown-linux-gnu", TAG,
                                             CONFIG, runner=missing)
        self.assertFails(report, "readelf is required")

    def test_cli_exit_status_and_json_report(self) -> None:
        archive = Archive()
        archive.add("../escape", b"x")
        path = archive.write(self.dir)
        with open(os.devnull, "w") as sink:
            stdout, stderr = io.StringIO(), sink
            original = sys.stdout, sys.stderr
            sys.stdout, sys.stderr = stdout, stderr
            try:
                status = inspect_mod.main(["archive", str(path), "--package", "biscuit-hash-cli", "--command", "bh",
                                           "--target", "aarch64-apple-darwin", "--tag", TAG])
            finally:
                sys.stdout, sys.stderr = original
        self.assertEqual(status, 1)
        report = json.loads(stdout.getvalue())
        self.assertFalse(report["passed"])
        self.assertTrue(report["failures"])


class ConfigLoaderTests(InspectCase):
    """`load_config`'s own strict JSON reading and validators (the script
    cannot import the shared loader), then one invalid shape per field it reads.
    Each negative case is one edit of the real `build-images.json`."""

    def load(self, text: str) -> dict:
        path = self.dir / "build-images.json"
        path.write_text(text, encoding="utf-8")
        return inspect_mod.load_config(path)

    def edited(self, path: tuple, value: object = None, *, delete: bool = False) -> str:
        data = copy.deepcopy(json.loads(CONFIG_PATH.read_text(encoding="utf-8")))
        parent = data
        for key in path[:-1]:
            parent = parent[key]
        if delete:
            del parent[path[-1]]
        else:
            parent[path[-1]] = value
        return json.dumps(data)

    def assertRefused(self, text: str, pattern: str) -> None:
        with self.assertRaisesRegex(inspect_mod.UsageError, pattern):
            self.load(text)

    def test_control_is_the_real_file(self) -> None:
        self.assertEqual(self.load(CONFIG_PATH.read_text(encoding="utf-8")), CONFIG)

    def test_json_reading(self) -> None:
        real = CONFIG_PATH.read_text(encoding="utf-8")
        cases = {
            "duplicate key": ('{"schema_version": 1, "schema_version": 1}', "duplicate object key 'schema_version'"),
            "trailing content": (real + "\n{}", "invalid JSON: Extra data"),
            "non-finite": (real.replace('"schema_version": 1', '"schema_version": NaN'), "invalid JSON constant NaN"),
            "malformed": (real[:-3], "invalid JSON"),
            "null root": ("null", "the document root must be a mapping, found null"),
            "boolean schema": (real.replace('"schema_version": 1', '"schema_version": true'), "schema_version: must be 1"),
        }
        for name, (text, pattern) in cases.items():
            with self.subTest(name):
                self.assertRefused(text, pattern)
        path = self.dir / "build-images.json"
        path.write_bytes(b'{"x": "\xff"}')
        with self.assertRaisesRegex(inspect_mod.UsageError, "cannot read build-images configuration"):
            inspect_mod.load_config(path)

    def test_validators_once(self) -> None:
        string, strings, mapping = inspect_mod.config_string, inspect_mod.config_strings, inspect_mod.config_mapping
        cases = [  # (validator, document, error after "f.json: k")
            (string, {}, ": is required"), (string, {"k": None}, ": must not be null"),
            (string, {"k": 1}, ": must be a string, found integer"), (string, {"k": " "}, ": must not be empty"),
            (strings, {}, ": is required"), (strings, {"k": None}, ": must not be null"),
            (strings, {"k": "a"}, ": must be a list, found string"), (strings, {"k": []}, ": must not be empty"),
            (strings, {"k": ["a", 2]}, "[1]: must be a non-empty string, found integer 2"),
            (strings, {"k": ["", None]}, "[0]: must be a non-empty string, found string ''"),
            (mapping, {}, ": is required"), (mapping, {"k": None}, ": must not be null"),
            (mapping, {"k": []}, ": must be a mapping, found list"), (mapping, {"k": {}}, ": must not be empty"),
        ]
        for validator, config, error in cases:
            with self.subTest(validator=validator.__name__, config=config):
                kwargs = {} if validator is string else {"allow_empty": False}
                with self.assertRaisesRegex(inspect_mod.UsageError, "^" + re.escape(f"f.json: k{error}") + "$"):
                    validator(config, "f.json", ("k",), **kwargs)
        self.assertEqual(strings({"k": []}, "f.json", ("k",), allow_empty=True), [])
        self.assertEqual(mapping({"k": {}}, "f.json", ("k",), allow_empty=True), {})

    def test_each_read_field_rejects_an_invalid_shape(self) -> None:
        cases = {
            ("max_glibc",): ("2.x", "max_glibc: must be a dotted version"),
            ("macos", "deployment_target"): (15, "macos.deployment_target: must be a string"),
            ("linux", "allowed_libraries"): ([], "linux.allowed_libraries: must not be empty"),
            ("linux", "allowed_interpreters"): (None, "linux.allowed_interpreters: must not be null"),
            ("macos", "allowed_library_prefixes"): ("/usr/lib/", "macos.allowed_library_prefixes: must be a list"),
            ("forbidden_library_patterns",): ([""], r"forbidden_library_patterns\[0\]: must be a non-empty string"),
            ("forbidden_path_fragments",): (..., "forbidden_path_fragments: is required"),
            ("targets",): ({}, "targets: must not be empty"),
            ("targets", "aarch64-apple-darwin"): ([], "targets.aarch64-apple-darwin: must be a mapping"),
            ("targets", "aarch64-apple-darwin", "os"): ("darwin", "targets.aarch64-apple-darwin.os: must be linux or macos"),
            ("targets", "aarch64-apple-darwin", "arch"): (..., "targets.aarch64-apple-darwin.arch: is required"),
            ("package_extras",): (..., "package_extras: is required"),
            ("package_extras", "claudine-cli"): ([], "package_extras.claudine-cli: must be a mapping"),
            ("package_extras", "claudine-cli", "allowed_libraries"): ("libasound.so.2", "allowed_libraries: must be a list"),
        }
        for path, (value, pattern) in cases.items():
            with self.subTest(path):
                self.assertRefused(self.edited(path, value, delete=value is ...), pattern)


class CommandLineTests(InspectCase):
    """`main` wiring: a positive `ldd` run, and located malformed inputs that
    write no report."""

    LDD = "\tlibc.so.6 => /lib/aarch64-linux-gnu/libc.so.6 (0x0000ffff9b050000)\n\t/lib/ld-linux-aarch64.so.1 (0x0000ffff9b2af000)\n"

    def main(self, *argv: str) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            status = inspect_mod.main(list(argv))
        return status, out.getvalue(), err.getvalue()

    def ldd(self, data: bytes, config: Path = CONFIG_PATH) -> tuple[int, str, str]:
        (self.dir / "ldd.txt").write_bytes(data)
        return self.main("ldd", str(self.dir / "ldd.txt"), "--package", "biscuit-hash-cli", "--target",
                         "aarch64-unknown-linux-gnu", "--config", str(config), "--report", str(self.dir / "report.json"))

    def test_ldd_control_passes(self) -> None:
        status, out, _ = self.ldd(self.LDD.encode())
        self.assertEqual(status, 0)
        self.assertTrue(json.loads(out)["passed"])
        self.assertTrue((self.dir / "report.json").is_file())

    def test_malformed_config_writes_no_report(self) -> None:
        bad = self.dir / "bad.json"
        bad.write_text(CONFIG_PATH.read_text(encoding="utf-8").replace('"max_glibc": "2.35"', '"max_glibc": null'), encoding="utf-8")
        status, out, err = self.ldd(self.LDD.encode(), config=bad)
        self.assertEqual((status, out), (2, ""))
        self.assertIn(f"{bad}: max_glibc: must not be null", err)
        self.assertFalse((self.dir / "report.json").exists())

    def test_non_utf8_ldd_output_writes_no_report(self) -> None:
        status, out, err = self.ldd(b"\xff\n")
        self.assertEqual((status, out), (2, ""))
        self.assertIn("ldd.txt: cannot read ldd output", err)
        self.assertFalse((self.dir / "report.json").exists())


class BuildImagesContractTests(unittest.TestCase):
    """`build-images.json` agrees with `distribution.py` and plan rules 11-13."""

    def test_targets_and_glibc_floor_match_distribution(self) -> None:
        self.assertEqual(list(CONFIG["targets"]), list(distribution.TARGETS))
        self.assertEqual(CONFIG["max_glibc"], distribution.MIN_GLIBC)
        self.assertEqual(sorted(t for t, s in CONFIG["targets"].items() if s["os"] == "linux"),
                         sorted(distribution.GNU_TARGETS))
        self.assertEqual(CONFIG["dist"]["version"], distribution.DIST_VERSION)
        self.assertEqual(list(CONFIG["dist"]["sha256"]), list(distribution.TARGETS))
        self.assertEqual(inspect_mod.CANONICAL_URL, distribution.CANONICAL_URL)
        self.assertEqual(set(CONFIG["package_extras"]) - set(distribution.ROSTER), set())

    def test_pinned_linux_image(self) -> None:
        linux = CONFIG["linux"]
        self.assertEqual(f"{linux['image']}@{linux['index_digest']}",
                         "ubuntu:22.04@sha256:076e7bf82319619f28dd0aa9c02ab34b055ad37100395b4f49a2ffba04fb5e11")
        self.assertEqual(linux["platform_digests"], {
            "amd64": "sha256:8a2321a92b438238f93b4346fff61bc4259c99fdd4d9b52ee86f153fee0c344f",
            "arm64": "sha256:a8fb2ba88181746111cbbe2969826d6546b71226785b196c93d2c3445a25000d",
        })
        self.assertEqual(linux["build_packages"],
                         ["build-essential", "pkg-config", "curl", "ca-certificates", "xz-utils", "binutils"])
        self.assertEqual(linux["runtime_packages"], [])
        self.assertEqual(CONFIG["package_extras"]["claudine-cli"],
                         {"build_packages": ["libasound2-dev"], "runtime_packages": ["libasound2"],
                          "allowed_libraries": ["libasound.so.2"]})

    def test_runners_and_macos_floor(self) -> None:
        self.assertEqual({t: s["runner"] for t, s in CONFIG["targets"].items()}, {
            "aarch64-apple-darwin": "macos-15", "x86_64-apple-darwin": "macos-15-intel",
            "aarch64-unknown-linux-gnu": "ubuntu-22.04-arm", "x86_64-unknown-linux-gnu": "ubuntu-22.04",
        })
        self.assertEqual(CONFIG["macos"]["deployment_target"], "15.0")
        self.assertEqual(CONFIG["required_env"], {"PCRE2_SYS_STATIC": "1"})
        for name in ("RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_RUSTC_WRAPPER"):
            self.assertIn(name, CONFIG["cleared_env"])

    def test_build_script_parses(self) -> None:
        result = subprocess.run(["bash", "-n", os.path.join(HERE, "build.sh")], capture_output=True, text=True,
                                timeout=30, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()

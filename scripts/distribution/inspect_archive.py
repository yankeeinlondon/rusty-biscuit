#!/usr/bin/env python3
"""Inspect a finished distribution archive and the binary inside it.

The archive must hold exactly one `<package>-<target>/` root with the
shipping command, `LICENSE`, `README.md`, `SOURCE.txt`, and
`THIRD-PARTY-NOTICES.md`, plus the package's `CHANGELOG.md` when it has one;
the binary must match the target's format and CPU
and link only what `.github/distribution/build-images.json` allows (plan rules
11-13 of `2026-10-28-publish`). Linux linkage comes from `readelf`, macOS
linkage from `otool`; their raw output is retained as evidence.

Stdlib only and Python 3.9-compatible: it runs inside the pinned Ubuntu 22.04
build container as well as on macOS runners.

    inspect_archive.py archive <archive> --package P --command C --target T --tag TAG
    inspect_archive.py ldd <ldd-output> --package P --target T

Each prints a JSON report (and writes it with `--report`). Exit status is 0
when the report passes, 1 when it lists failures, 2 on unusable input.
"""

from __future__ import annotations

import os
import sys

import argparse
import json
import re
import shutil
import struct
import subprocess
import tarfile
import tempfile
from pathlib import Path, PurePosixPath
from typing import Any, Callable, Dict, List, Optional, Sequence, Tuple

# In the repository the configuration is `.github/distribution/`; `build.sh`
# container modes copy it beside this file.
_HERE = Path(__file__).resolve().parent
DEFAULT_CONFIG = (_HERE / "build-images.json" if (_HERE / "build-images.json").is_file()
                  else _HERE.parent.parent / ".github" / "distribution" / "build-images.json")
CANONICAL_URL = "https://github.com/yankeeinlondon/rusty-biscuit"
NOTICE_FILES = ("LICENSE", "README.md", "SOURCE.txt", "THIRD-PARTY-NOTICES.md")
# dist adds a package directory's release-plz changelog when one exists.
OPTIONAL_FILES = ("CHANGELOG.md",)
SCHEMA_VERSION = 1
TOOL_TIMEOUT = 120
MAX_MEMBER_BYTES = 512 * 1024 * 1024

ELF_MACHINES = {62: "x86_64", 183: "aarch64"}
MACHO_CPUTYPES = {0x01000007: "x86_64", 0x0100000C: "arm64"}
MACHO_64_LE = b"\xcf\xfa\xed\xfe"
# Universal (fat) and 32-bit Mach-O. dist builds one target per archive, so a
# fat binary means something other than dist produced the file.
MACHO_OTHER = (b"\xca\xfe\xba\xbe", b"\xbe\xba\xfe\xca", b"\xca\xfe\xba\xbf", b"\xbf\xba\xfe\xca",
               b"\xce\xfa\xed\xfe", b"\xfe\xed\xfa\xce", b"\xfe\xed\xfa\xcf")
MACHO_DYLIB_COMMANDS = ("LC_LOAD_DYLIB", "LC_LOAD_WEAK_DYLIB", "LC_REEXPORT_DYLIB", "LC_LAZY_LOAD_DYLIB",
                        "LC_LOAD_UPWARD_DYLIB")
LDD_KERNEL_OBJECTS = ("linux-vdso.so.1", "linux-gate.so.1")

Runner = Callable[[Sequence[str]], str]


class UsageError(Exception):
    """Input the inspection cannot evaluate at all (exit 2)."""


class Report:
    def __init__(self, mode: str, **fields: Any) -> None:
        self.data: Dict[str, Any] = {"schema_version": SCHEMA_VERSION, "mode": mode, **fields}
        self.failures: List[Dict[str, str]] = []

    def fail(self, where: str, reason: str) -> None:
        self.failures.append({"where": where, "reason": reason})

    def as_json(self) -> Dict[str, Any]:
        return {**self.data, "passed": not self.failures, "failures": self.failures}


# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------


# This script cannot import `scripts/ci/distribution.py` (it runs where PyYAML
# is absent), so the loader and validators below mirror its JSON loader and
# `Fields`: absent, null, wrong type, and empty are distinct located errors.
_ABSENT = object()
_TYPE_NAMES = {bool: "boolean", int: "integer", float: "number", str: "string", list: "list", dict: "mapping"}


def _no_duplicates(pairs: List[Tuple[str, Any]]) -> Dict[str, Any]:
    result: Dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate object key {key!r}")
        result[key] = value
    return result


def _no_constant(token: str) -> Any:
    raise ValueError(f"invalid JSON constant {token}")


def _type_name(value: Any) -> str:
    return "null" if value is None else _TYPE_NAMES.get(type(value), type(value).__name__)


def _present(config: Dict[str, Any], source: str, path: Sequence[str]) -> Any:
    node: Any = config
    for part in path:
        node = node.get(part, _ABSENT) if isinstance(node, dict) else _ABSENT
    if node is _ABSENT:
        raise UsageError(f"{source}: {'.'.join(path)}: is required")
    if node is None:
        raise UsageError(f"{source}: {'.'.join(path)}: must not be null")
    return node


def config_string(config: Dict[str, Any], source: str, path: Sequence[str]) -> str:
    value = _present(config, source, path)
    if not isinstance(value, str):
        raise UsageError(f"{source}: {'.'.join(path)}: must be a string, found {_type_name(value)}")
    if not value.strip():
        raise UsageError(f"{source}: {'.'.join(path)}: must not be empty")
    return value


def config_strings(config: Dict[str, Any], source: str, path: Sequence[str], *, allow_empty: bool) -> List[str]:
    value = _present(config, source, path)
    if not isinstance(value, list):
        raise UsageError(f"{source}: {'.'.join(path)}: must be a list, found {_type_name(value)}")
    if not value and not allow_empty:
        raise UsageError(f"{source}: {'.'.join(path)}: must not be empty")
    for index, item in enumerate(value):
        if not isinstance(item, str) or not item.strip():
            raise UsageError(f"{source}: {'.'.join(path)}[{index}]: must be a non-empty string, found {_type_name(item)} {item!r}")
    return value


def config_mapping(config: Dict[str, Any], source: str, path: Sequence[str], *, allow_empty: bool) -> Dict[str, Any]:
    value = _present(config, source, path)
    if not isinstance(value, dict):
        raise UsageError(f"{source}: {'.'.join(path)}: must be a mapping, found {_type_name(value)}")
    if not value and not allow_empty:
        raise UsageError(f"{source}: {'.'.join(path)}: must not be empty")
    return value


def load_config(path: Path) -> Dict[str, Any]:
    """`build-images.json`, with every field this script reads validated, so
    the checks below index it directly."""
    source = str(path)
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise UsageError(f"{source}: cannot read build-images configuration: {error}") from None
    try:
        config = json.loads(text, object_pairs_hook=_no_duplicates, parse_constant=_no_constant)
    except ValueError as error:
        raise UsageError(f"{source}: invalid JSON: {error}") from None
    if not isinstance(config, dict):
        raise UsageError(f"{source}: the document root must be a mapping, found {_type_name(config)}")
    schema = _present(config, source, ("schema_version",))
    if isinstance(schema, bool) or schema != SCHEMA_VERSION:
        raise UsageError(f"{source}: schema_version: must be {SCHEMA_VERSION}, found {schema!r}")
    glibc = config_string(config, source, ("max_glibc",))
    if version_tuple(glibc) is None:
        raise UsageError(f"{source}: max_glibc: must be a dotted version, found {glibc!r}")
    config_string(config, source, ("macos", "deployment_target"))
    for strings in (("linux", "allowed_libraries"), ("linux", "allowed_interpreters"),
                    ("macos", "allowed_library_prefixes"), ("forbidden_library_patterns",),
                    ("forbidden_path_fragments",)):
        config_strings(config, source, strings, allow_empty=False)
    for target in config_mapping(config, source, ("targets",), allow_empty=False):
        config_mapping(config, source, ("targets", target), allow_empty=False)
        if config_string(config, source, ("targets", target, "os")) not in ("linux", "macos"):
            raise UsageError(f"{source}: targets.{target}.os: must be linux or macos")
        config_string(config, source, ("targets", target, "arch"))
    extras = config_mapping(config, source, ("package_extras",), allow_empty=True)
    for package in extras:
        if "allowed_libraries" in config_mapping(config, source, ("package_extras", package), allow_empty=True):
            config_strings(config, source, ("package_extras", package, "allowed_libraries"), allow_empty=True)
    return config


def target_settings(config: Dict[str, Any], target: str) -> Dict[str, Any]:
    settings = config["targets"].get(target)
    if settings is None:
        raise UsageError(f"target {target!r} is not one of {sorted(config['targets'])}")
    return settings


def package_extra(config: Dict[str, Any], package: str, key: str) -> List[str]:
    """A package's extra entries; a package or key without extras has none."""
    return list(config["package_extras"].get(package, {}).get(key, []))


def allowed_linux_libraries(config: Dict[str, Any], package: str) -> List[str]:
    return list(config["linux"]["allowed_libraries"]) + package_extra(config, package, "allowed_libraries")


def version_tuple(text: str) -> Optional[Tuple[int, ...]]:
    if not re.fullmatch(r"[0-9]+(\.[0-9]+)*", text):
        return None
    return tuple(int(part) for part in text.split("."))


# ---------------------------------------------------------------------------
# Archive structure
# ---------------------------------------------------------------------------


def _executable_magic(data: bytes) -> bool:
    head = data[:4]
    return head == b"\x7fELF" or head == MACHO_64_LE or head in MACHO_OTHER or data[:2] == b"#!"


def check_source_text(text: str, tag: str, report: Report, where: str) -> None:
    tree = re.escape(f"{CANONICAL_URL}/tree/{tag}")
    if not re.search(rf"{tree}(?=/|\s|$)", text):
        report.fail(where, f"does not name the exact tagged source URL {CANONICAL_URL}/tree/{tag}")
    if not re.search(rf"{re.escape(CANONICAL_URL)}/commit/[0-9a-f]{{40}}(?![0-9a-f])", text):
        report.fail(where, f"does not name a 40-hex commit URL under {CANONICAL_URL}/commit/")


def check_archive(archive: Path, package: str, command: str, target: str, tag: str,
                  report: Report) -> Optional[bytes]:
    """Validate every archive member without extracting through `tarfile`
    (which would honor links and absolute names). Returns the command's
    bytes when it is present as a regular executable file."""
    root = f"{package}-{target}"
    expected = {f"{root}/{name}" for name in (command, *NOTICE_FILES, *OPTIONAL_FILES)}
    contents: Dict[str, bytes] = {}
    entries: List[Dict[str, Any]] = []
    seen: set = set()
    try:
        tar = tarfile.open(archive, mode="r:xz")
    except (OSError, tarfile.TarError, EOFError) as error:
        report.fail(archive.name, f"is not a readable tar.xz archive: {error}")
        return None
    with tar:
        for member in tar:
            raw = member.name
            where = f"archive member {raw!r}"
            kind = ("symlink" if member.issym() else "hardlink" if member.islnk() else
                    "dir" if member.isdir() else "file" if member.isreg() else "other")
            entries.append({"name": raw, "type": kind, "mode": oct(member.mode & 0o7777), "size": member.size})
            name = raw.rstrip("/")
            parts = name.split("/")
            if raw.startswith("/") or re.match(r"^[A-Za-z]:", raw) or "\\" in raw:
                report.fail(where, "is an absolute or non-POSIX path")
                continue
            if any(part in ("..", ".", "") for part in parts):
                report.fail(where, "contains a '..', '.', or empty path component")
                continue
            if member.issym() or member.islnk():
                report.fail(where, f"is a {kind} (to {member.linkname!r}); archives hold no links")
                continue
            if kind == "other":
                report.fail(where, "is neither a regular file nor a directory")
                continue
            if parts[0] != root:
                report.fail(where, f"is outside the single archive root {root}/")
                continue
            if name in seen:
                report.fail(where, "appears more than once")
                continue
            seen.add(name)
            if member.isdir():
                if name != root:
                    report.fail(where, "is an unexpected directory")
                continue
            if name not in expected:
                report.fail(where, f"is not one of the expected entries {sorted(expected)}")
            if member.size > MAX_MEMBER_BYTES:
                report.fail(where, f"is larger than {MAX_MEMBER_BYTES} bytes")
                continue
            handle = tar.extractfile(member)
            data = handle.read() if handle else b""
            contents[name] = data
            is_command = name == f"{root}/{command}"
            if not is_command and (member.mode & 0o111 or _executable_magic(data)):
                report.fail(where, f"is executable; {command} is the only executable an archive ships")
            if is_command and not member.mode & 0o100:
                report.fail(where, "is not marked executable")
    report.data["entries"] = entries
    report.data["executables"] = sorted(e["name"] for e in entries
                                        if e["type"] == "file" and int(e["mode"], 8) & 0o111)
    for notice in NOTICE_FILES:
        path = f"{root}/{notice}"
        if path not in contents:
            report.fail(f"archive member {path!r}", "is missing")
        elif not contents[path].strip():
            report.fail(f"archive member {path!r}", "is empty")
    source = contents.get(f"{root}/SOURCE.txt")
    if source and source.strip():
        where = f"archive member '{root}/SOURCE.txt'"
        try:
            check_source_text(source.decode("utf-8"), tag, report, where)
        except UnicodeDecodeError as error:
            report.fail(where, f"is not UTF-8: {error}")
    binary = contents.get(f"{root}/{command}")
    if binary is None:
        report.fail(f"archive member '{root}/{command}'", "is missing")
    return binary


# ---------------------------------------------------------------------------
# Binary headers
# ---------------------------------------------------------------------------


def identify_binary(data: bytes) -> Dict[str, Any]:
    """Format and CPU from the file header alone."""
    if data[:4] == b"\x7fELF":
        if len(data) < 20:
            return {"format": "elf", "error": "truncated ELF header"}
        if data[4] != 2 or data[5] != 1:
            return {"format": "elf", "error": f"ELF class {data[4]}/data {data[5]} is not 64-bit little-endian"}
        machine = struct.unpack_from("<H", data, 18)[0]
        return {"format": "elf", "arch": ELF_MACHINES.get(machine, f"e_machine {machine}")}
    if data[:4] == MACHO_64_LE:
        if len(data) < 16:
            return {"format": "macho", "error": "truncated Mach-O header"}
        cputype, _subtype, filetype = struct.unpack_from("<iII", data, 4)
        cputype &= 0xFFFFFFFF
        arch = MACHO_CPUTYPES.get(cputype, f"cputype {cputype:#x}")
        if filetype != 2:
            return {"format": "macho", "arch": arch, "error": f"Mach-O filetype {filetype} is not MH_EXECUTE"}
        return {"format": "macho", "arch": arch}
    if data[:4] in MACHO_OTHER:
        return {"format": "macho", "error": "universal (fat) or 32-bit Mach-O; each archive ships one thin 64-bit binary"}
    return {"format": "unknown", "error": f"unrecognized header {data[:4].hex()}"}


def check_binary_header(data: bytes, settings: Dict[str, Any], report: Report, where: str) -> Dict[str, Any]:
    found = identify_binary(data)
    report.data["binary"] = found
    expected_format = "elf" if settings["os"] == "linux" else "macho"
    if "error" in found:
        report.fail(where, found["error"])
    elif found["format"] != expected_format:
        report.fail(where, f"is {found['format']}, expected {expected_format}")
    elif found["arch"] != settings["arch"]:
        report.fail(where, f"is built for {found['arch']}, expected {settings['arch']}")
    return found


# ---------------------------------------------------------------------------
# ELF linkage (readelf / ldd output)
# ---------------------------------------------------------------------------


def parse_readelf_version_needs(text: str) -> Dict[str, List[str]]:
    """`readelf -V` version needs (`.gnu.version_r`): library -> version names."""
    needs: Dict[str, List[str]] = {}
    inside = False
    library = None
    for line in text.splitlines():
        if line.startswith("Version needs section"):
            inside = True
            continue
        if inside and line and not line[0].isspace():
            inside = False
        if not inside:
            continue
        file_match = re.search(r"File:\s*(\S+)", line)
        if file_match:
            library = file_match.group(1)
            needs.setdefault(library, [])
            continue
        name_match = re.search(r"Name:\s*(\S+)", line)
        if name_match and library is not None:
            needs[library].append(name_match.group(1))
    return needs


def parse_readelf_dynamic(text: str) -> Dict[str, Any]:
    """`readelf -d` NEEDED/RPATH/RUNPATH and `readelf -l` interpreter."""
    needed = re.findall(r"\(NEEDED\)\s+Shared library: \[([^\]]+)\]", text)
    rpaths = re.findall(r"\((?:RPATH|RUNPATH)\)\s+Library (?:rpath|runpath): \[([^\]]*)\]", text)
    interpreter = re.search(r"Requesting program interpreter: ([^\]]+)\]", text)
    return {"needed": needed, "rpaths": rpaths, "interpreter": interpreter.group(1) if interpreter else None}


def _forbidden_library(name: str, config: Dict[str, Any]) -> Optional[str]:
    lowered = name.lower()
    for pattern in config["forbidden_library_patterns"]:
        if pattern in lowered:
            return pattern
    return None


def _forbidden_fragment(value: str, fragments: Sequence[str]) -> Optional[str]:
    for fragment in fragments:
        if fragment in value:
            return fragment
    return None


def check_elf_linkage(readelf: str, config: Dict[str, Any], package: str, report: Report, where: str,
                      fragments: Sequence[str]) -> None:
    dynamic = parse_readelf_dynamic(readelf)
    needs = parse_readelf_version_needs(readelf)
    allowed = allowed_linux_libraries(config, package)
    ceiling = version_tuple(config["max_glibc"])
    glibc: List[str] = []
    for library, names in needs.items():
        for name in names:
            if not name.startswith("GLIBC_"):
                continue
            glibc.append(name)
            number = version_tuple(name[len("GLIBC_"):])
            if number is None:
                report.fail(where, f"requires {name} from {library}, which is not a numbered glibc version")
            elif number > ceiling:
                report.fail(where, f"requires {name} from {library}, newer than the glibc {config['max_glibc']} floor")
    # Without version needs the glibc floor would go unchecked, not pass.
    if "libc.so.6" in dynamic["needed"] and not glibc:
        report.fail(where, "links libc.so.6, but readelf lists no GLIBC_ version needs to check against the floor")
    numbered = sorted({name for name in glibc if version_tuple(name[6:])}, key=lambda n: version_tuple(n[6:]))
    report.data["linkage"] = {**dynamic, "glibc_versions": numbered, "max_glibc": numbered[-1][6:] if numbered else None,
                              "version_needs": needs}
    if not dynamic["needed"]:
        report.fail(where, "readelf lists no NEEDED libraries; a glibc-linked executable needs at least libc.so.6")
    for library in dynamic["needed"]:
        pattern = _forbidden_library(library, config)
        if pattern:
            report.fail(where, f"links {library}; {pattern} must be linked statically")
        elif library not in allowed:
            report.fail(where, f"links {library}, which is not in the allowed list {allowed}")
    for rpath in dynamic["rpaths"]:
        report.fail(where, f"carries RPATH/RUNPATH {rpath!r}; shipped binaries have none")
    interpreter = dynamic["interpreter"]
    if interpreter is None:
        report.fail(where, "has no program interpreter; expected a dynamically linked glibc executable")
    elif interpreter not in config["linux"]["allowed_interpreters"]:
        report.fail(where, f"requests interpreter {interpreter}, not one of {config['linux']['allowed_interpreters']}")
    for value in [*dynamic["needed"], *dynamic["rpaths"], interpreter or ""]:
        fragment = _forbidden_fragment(value, fragments)
        if fragment:
            report.fail(where, f"names build-host path {value!r} ({fragment})")


def parse_ldd(text: str) -> List[Dict[str, Optional[str]]]:
    rows = []
    for line in text.splitlines():
        line = line.strip()
        if not line or line.endswith(":"):
            continue
        if "=>" in line:
            name, _, target = line.partition("=>")
            target = target.strip()
            path = None if target.startswith("not found") else target.split(" (")[0].strip()
            rows.append({"name": name.strip(), "path": path, "found": str(path is not None).lower()})
        else:
            path = line.split(" (")[0].strip()
            rows.append({"name": PurePosixPath(path).name, "path": path, "found": "true"})
    return rows


def check_ldd(text: str, config: Dict[str, Any], package: str, report: Report, where: str) -> None:
    rows = parse_ldd(text)
    report.data["ldd"] = rows
    allowed = set(allowed_linux_libraries(config, package)) | set(LDD_KERNEL_OBJECTS)
    if not rows:
        report.fail(where, "ldd reported no libraries")
    for row in rows:
        if row["found"] != "true":
            report.fail(where, f"{row['name']} was not found in the runtime image")
        elif row["name"] not in allowed:
            report.fail(where, f"resolves {row['name']} ({row['path']}), which is not in the allowed list")


# ---------------------------------------------------------------------------
# Mach-O linkage (otool output)
# ---------------------------------------------------------------------------


def parse_otool_dylibs(text: str) -> List[str]:
    """`otool -L`: the first line names the file; each following line is a dependency."""
    lines = [line for line in text.splitlines() if line.strip()]
    return [line.strip().split(" (compatibility version")[0].strip() for line in lines[1:]]


def parse_otool_load_commands(text: str) -> List[Dict[str, str]]:
    """`otool -l`: one dict per load command, `key value` lines folded in."""
    commands: List[Dict[str, str]] = []
    current: Optional[Dict[str, str]] = None
    for line in text.splitlines():
        if line.startswith("Load command"):
            current = {}
            commands.append(current)
            continue
        if current is None:
            continue
        match = re.match(r"\s*(\S+) (.*)$", line)
        if match and match.group(1) not in current:
            value = match.group(2).strip()
            if match.group(1) in ("name", "path"):
                value = re.sub(r" \(offset \d+\)$", "", value)
            current[match.group(1)] = value
    return commands


def check_macho_linkage(otool_l: str, otool_big_l: str, config: Dict[str, Any], package: str, report: Report,
                        where: str, fragments: Sequence[str]) -> None:
    commands = parse_otool_load_commands(otool_l)
    dylibs = parse_otool_dylibs(otool_big_l)
    for command in commands:
        if command.get("cmd") in MACHO_DYLIB_COMMANDS and command.get("name") not in dylibs:
            dylibs.append(command["name"])
    rpaths = [c.get("path", "") for c in commands if c.get("cmd") == "LC_RPATH"]
    builds = [c for c in commands if c.get("cmd") == "LC_BUILD_VERSION"]
    legacy = [c for c in commands if c.get("cmd") == "LC_VERSION_MIN_MACOSX"]
    minos = builds[0].get("minos") if builds else (legacy[0].get("version") if legacy else None)
    platform = builds[0].get("platform") if builds else None
    report.data["linkage"] = {"dylibs": dylibs, "rpaths": rpaths, "minos": minos, "platform": platform}
    expected = config["macos"]["deployment_target"]
    if len(builds) + len(legacy) != 1:
        report.fail(where, f"has {len(builds)} LC_BUILD_VERSION and {len(legacy)} LC_VERSION_MIN_MACOSX commands; expected one")
    if minos != expected:
        report.fail(where, f"has minimum macOS {minos}, expected {expected}")
    if builds and platform not in ("1", "MACOS"):
        report.fail(where, f"LC_BUILD_VERSION platform {platform} is not macOS (1)")
    for rpath in rpaths:
        report.fail(where, f"carries LC_RPATH {rpath!r}; shipped binaries have none")
    prefixes = config["macos"]["allowed_library_prefixes"]
    for dylib in dylibs:
        pattern = _forbidden_library(dylib, config)
        if pattern:
            report.fail(where, f"links {dylib}; {pattern} must be linked statically")
        elif not any(dylib.startswith(prefix) for prefix in prefixes):
            report.fail(where, f"links {dylib}, outside the system locations {prefixes}")
    # The first line is the inspected file's own (scratch) path.
    for line in otool_l.split("Load command", 1)[-1].splitlines():
        fragment = _forbidden_fragment(line, fragments)
        if fragment:
            report.fail(where, f"load command names build-host path ({fragment}): {line.strip()}")


# ---------------------------------------------------------------------------
# Driver
# ---------------------------------------------------------------------------


def run_tool(argv: Sequence[str]) -> str:
    if shutil.which(argv[0]) is None:
        raise FileNotFoundError(argv[0])
    env = {**os.environ, "LC_ALL": "C"}
    result = subprocess.run(list(argv), capture_output=True, text=True, timeout=TOOL_TIMEOUT, env=env, check=False)
    if result.returncode != 0:
        raise RuntimeError(f"{' '.join(argv)} exited {result.returncode}: {result.stderr.strip()}")
    return result.stdout


def _retain(evidence: Optional[Path], key: str, name: str, text: str, report: Report) -> None:
    if evidence is None:
        return
    evidence.mkdir(parents=True, exist_ok=True)
    (evidence / name).write_text(text, encoding="utf-8")
    report.data.setdefault("evidence", {})[key] = name


def inspect_archive(archive: Path, package: str, command: str, target: str, tag: str, config: Dict[str, Any],
                    *, staging_root: Optional[str] = None, evidence: Optional[Path] = None,
                    runner: Runner = run_tool) -> Report:
    settings = target_settings(config, target)
    report = Report("archive", archive=archive.name, package=package, command=command, target=target, tag=tag)
    binary = check_archive(archive, package, command, target, tag, report)
    # A refused archive runs no inspection tool on its contents.
    if binary is None or report.failures:
        return report
    where = f"{package}-{target}/{command}"
    found = check_binary_header(binary, settings, report, where)
    if "error" in found or found.get("arch") != settings["arch"]:
        return report
    fragments = list(config["forbidden_path_fragments"])
    if staging_root is not None:
        if not staging_root.strip():
            raise UsageError("--staging-root must not be empty")
        fragments.append(staging_root)
    with tempfile.TemporaryDirectory(prefix="dist-inspect-") as scratch:
        path = Path(scratch) / command
        path.write_bytes(binary)
        try:
            if settings["os"] == "linux":
                readelf = runner(["readelf", "--wide", "--dynamic", "--program-headers", "--version-info", str(path)])
                _retain(evidence, "readelf", f"readelf-{package}-{target}.txt", readelf, report)
                check_elf_linkage(readelf, config, package, report, where, fragments)
            else:
                otool_l = runner(["otool", "-l", str(path)])
                otool_big_l = runner(["otool", "-L", str(path)])
                # Distinct names: `-l`/`-L` collide on case-insensitive filesystems.
                _retain(evidence, "otool_load_commands", f"otool-load-commands-{package}-{target}.txt", otool_l, report)
                _retain(evidence, "otool_libraries", f"otool-libraries-{package}-{target}.txt", otool_big_l, report)
                check_macho_linkage(otool_l, otool_big_l, config, package, report, where, fragments)
        except FileNotFoundError as error:
            report.fail(where, f"{error.args[0]} is required to inspect this binary and is not installed")
        except (RuntimeError, subprocess.TimeoutExpired) as error:
            report.fail(where, f"inspection tool failed: {error}")
    return report


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="mode", required=True)
    archive = sub.add_parser("archive", help="inspect a finished <package>-<target>.tar.xz")
    archive.add_argument("archive", type=Path)
    archive.add_argument("--command", required=True)
    archive.add_argument("--tag", required=True)
    archive.add_argument("--staging-root", help="build checkout path that must not appear in load commands")
    archive.add_argument("--evidence-dir", type=Path, help="where to retain raw readelf/otool output")
    ldd = sub.add_parser("ldd", help="check `ldd` output captured in the runtime image")
    ldd.add_argument("ldd_output", type=Path)
    for mode in (archive, ldd):
        mode.add_argument("--package", required=True)
        mode.add_argument("--target", required=True)
        mode.add_argument("--config", type=Path, default=DEFAULT_CONFIG)
        mode.add_argument("--report", type=Path, help="also write the JSON report here")
    args = parser.parse_args(argv)
    try:
        config = load_config(args.config)
        if args.mode == "archive":
            report = inspect_archive(args.archive, args.package, args.command, args.target, args.tag, config,
                                     staging_root=args.staging_root, evidence=args.evidence_dir)
        else:
            if target_settings(config, args.target)["os"] != "linux":
                raise UsageError("ldd output applies to Linux targets only")
            report = Report("ldd", package=args.package, target=args.target, source=args.ldd_output.name)
            try:
                text = args.ldd_output.read_text(encoding="utf-8")
            except (OSError, UnicodeDecodeError) as error:
                raise UsageError(f"{args.ldd_output}: cannot read ldd output: {error}") from None
            check_ldd(text, config, args.package, report, args.ldd_output.name)
    except (UsageError, OSError) as error:
        print(f"inspect_archive.py: {error}", file=sys.stderr)
        return 2
    text = json.dumps(report.as_json(), indent=2, sort_keys=True) + "\n"
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(text, encoding="utf-8")
    sys.stdout.write(text)
    for failure in report.failures:
        print(f"inspect_archive.py: {failure['where']}: {failure['reason']}", file=sys.stderr)
    return 1 if report.failures else 0


if __name__ == "__main__":
    sys.exit(main())

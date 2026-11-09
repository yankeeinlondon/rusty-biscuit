#!/usr/bin/env python3
"""Harden dist's generated shell installer and verify dist's checksums.

- `harden` inserts a platform preflight into a dist 0.34.0
  `<package>-installer.sh`. It runs after argument parsing (so `--help` still
  works everywhere) and before architecture detection, archive selection,
  `mktemp`, or any download, and refuses with the required and found versions:
  Linux other than x86_64/aarch64 with glibc at or above the floor (musl and
  undetectable glibc included), macOS other than arm64/x86_64 at or above the
  macOS floor, and every other operating system. The transformation is
  anchored: each anchor must occur exactly once and in template order, so a
  changed dist template is an error naming the anchor, never a silent miss.
- `check-sums` verifies every `*.sha256` sidecar and `sha256.sum` entry dist
  wrote against the referenced file with the platform's `shasum -a 256 -c`.
  No digest is computed in Python (plan rule 14).

Both print a JSON report on stdout; a failure is a located message on stderr
and exit status 1. The operational guide is `docs/distribution.md`.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "ci"))
import distribution  # noqa: E402

REPOSITORY = Path(__file__).resolve().parents[2]
BUILD_IMAGES = REPOSITORY / ".github" / "distribution" / "build-images.json"
MACOS_FLOOR = "15.0"

PREFLIGHT_FUNCTION = "biscuit_distribution_preflight"
BLOCK_BEGIN = "# >>> biscuit-distribution installer preflight (scripts/distribution/installers.py)\n"
BLOCK_END = "# <<< biscuit-distribution installer preflight\n"

# `uname -m` spellings of each shipped target's architecture.
_UNAME_ARCH = {"aarch64-apple-darwin": "arm64", "x86_64-apple-darwin": "x86_64",
               "aarch64-unknown-linux-gnu": "aarch64", "x86_64-unknown-linux-gnu": "x86_64"}

_FLOOR = re.compile(r"^([0-9]+)\.([0-9]+)$")
_DIST_GLIBC_CHECK = re.compile(r'check_glibc "([0-9]+)" "([0-9]+)"')
_SUM_LINE = re.compile(r"^([0-9a-f]{64}) [ *]([A-Za-z0-9._+-]+)$")


class InstallerError(Exception):
    def __init__(self, where: str, message: str) -> None:
        super().__init__(f"{where}: {message}")


@dataclass(frozen=True)
class Anchor:
    name: str
    text: str


# Template order. The call is inserted at `architecture-detection` (inside
# `download_binary_and_run_installer`, before `select_archive_for_arch` and its
# download loop); the function block is inserted before `entry-point`.
ANCHORS = (
    Anchor("app-name", '\nAPP_NAME="'),
    Anchor("installer-function", "\ndownload_binary_and_run_installer() {\n"),
    Anchor("architecture-detection", "\n    get_architecture || return 1\n"),
    Anchor("archive-selection", '\n    _artifact_name="$(select_archive_for_arch "$_true_arch")" || return 1\n'),
    Anchor("entry-point", '\ndownload_binary_and_run_installer "$@" || exit 1\n'),
)


def _line(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def _floor(value: str, where: str) -> tuple[int, int]:
    match = _FLOOR.match(value)
    if not match:
        raise InstallerError(where, f"floor {value!r} is not '<major>.<minor>'")
    return int(match.group(1)), int(match.group(2))


def _read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise InstallerError(path.as_posix(), f"cannot read: {error}") from None


def macos_floor(path: Path = BUILD_IMAGES) -> str:
    """The build contract's `macos.deployment_target`."""
    source = path.as_posix()
    try:
        data = path.read_bytes()
    except OSError as error:
        raise InstallerError(source, f"cannot read the build contract: {error}") from None
    fields = distribution.Fields(distribution.Document(source, distribution.load_json(data, source=source)))
    value = fields.string(["macos", "deployment_target"])
    _floor(value, f"{source}: macos.deployment_target")
    return value


def preflight_block(glibc: str, macos: str) -> str:
    glibc_major, glibc_minor = _floor(glibc, "glibc")
    macos_major, macos_minor = _floor(macos, "macOS")
    linux = sorted({_UNAME_ARCH[t] for t in distribution.TARGETS if t.endswith("-linux-gnu")})
    darwin = sorted({_UNAME_ARCH[t] for t in distribution.TARGETS if t.endswith("-apple-darwin")})
    supported = f"Linux {'/'.join(linux)} with glibc {glibc}+, macOS {'/'.join(darwin)} {macos}+"
    body = f"""\
# Refuses an unsupported platform before anything is downloaded or written.
# Messages go to stderr even with --quiet: a refusal must always explain itself.
biscuit_preflight_refuse() {{
    echo "ERROR: $APP_NAME $APP_VERSION cannot be installed: $1" >&2
    echo "Supported platforms: {supported}" >&2
    exit 1
}}

# Sets _bp_major/_bp_minor from a dotted version ("15" means 15.0); fails
# unless both are decimal numbers short enough for `test` arithmetic.
biscuit_preflight_split() {{
    _bp_major="${{1%%.*}}"
    case "$1" in
        *.*) _bp_minor="${{1#*.}}" ;;
        *) _bp_minor=0 ;;
    esac
    _bp_minor="${{_bp_minor%%.*}}"
    case "$_bp_major" in
        ''|*[!0-9]*|???????*) return 1 ;;
    esac
    case "$_bp_minor" in
        ''|*[!0-9]*|???????*) return 1 ;;
    esac
}}

# Numeric comparison: major first, then minor (2.100 is newer than 2.35).
biscuit_preflight_at_least() {{
    [ "$_bp_major" -gt "$1" ] || {{ [ "$_bp_major" -eq "$1" ] && [ "$_bp_minor" -ge "$2" ]; }}
}}

biscuit_preflight_glibc() {{
    local _found=""
    local _ldd=""
    if command -v getconf > /dev/null 2>&1; then
        _found="$(getconf GNU_LIBC_VERSION 2> /dev/null)" || _found=""
        case "$_found" in
            "glibc "*) _found="${{_found#glibc }}" ;;
            *) _found="" ;;
        esac
    fi
    if [ -z "$_found" ] && command -v ldd > /dev/null 2>&1; then
        # musl's ldd prints its banner to stderr and exits 1.
        _ldd="$(ldd --version 2>&1)" || true
        case "$_ldd" in
            *musl*) biscuit_preflight_refuse "requires glibc {glibc} or newer; found musl libc" ;;
        esac
        _ldd="$(printf '%s\\n' "$_ldd" | head -n 1)"
        case "$_ldd" in
            *GLIBC*|*glibc*|*"GNU libc"*) _found="${{_ldd##* }}" ;;
        esac
    fi
    if [ -z "$_found" ] || ! biscuit_preflight_split "$_found"; then
        biscuit_preflight_refuse "unable to detect the glibc version (required: glibc {glibc} or newer; found: '${{_found:-$_ldd}}' from getconf GNU_LIBC_VERSION and ldd --version)"
    fi
    if ! biscuit_preflight_at_least {glibc_major} {glibc_minor}; then
        biscuit_preflight_refuse "requires glibc {glibc} or newer; found glibc $_found"
    fi
}}

biscuit_preflight_macos() {{
    local _found=""
    if command -v sw_vers > /dev/null 2>&1; then
        _found="$(sw_vers -productVersion 2> /dev/null)" || _found=""
    fi
    if [ -z "$_found" ] || ! biscuit_preflight_split "$_found"; then
        biscuit_preflight_refuse "unable to detect the macOS version (required: macOS {macos} or newer; found: '$_found' from sw_vers -productVersion)"
    fi
    if ! biscuit_preflight_at_least {macos_major} {macos_minor}; then
        biscuit_preflight_refuse "requires macOS {macos} or newer; found macOS $_found"
    fi
}}

{PREFLIGHT_FUNCTION}() {{
    local _os
    local _cpu
    _os="$(uname -s 2> /dev/null)" || _os=""
    _cpu="$(uname -m 2> /dev/null)" || _cpu=""
    case "$_os" in
        Linux)
            case "$_cpu" in
                {'|'.join(linux)}) ;;
                *) biscuit_preflight_refuse "unsupported architecture: Linux ${{_cpu:-unknown}}" ;;
            esac
            biscuit_preflight_glibc
            ;;
        Darwin)
            case "$_cpu" in
                {'|'.join(darwin)}) ;;
                *) biscuit_preflight_refuse "unsupported architecture: macOS ${{_cpu:-unknown}}" ;;
            esac
            biscuit_preflight_macos
            ;;
        *)
            biscuit_preflight_refuse "unsupported operating system: ${{_os:-unknown}} ${{_cpu:-unknown}}"
            ;;
    esac
}}
"""
    return BLOCK_BEGIN + body + BLOCK_END + "\n"


CALL = f"    {PREFLIGHT_FUNCTION}\n"


def harden(text: str, *, source: str, glibc: str = distribution.MIN_GLIBC, macos: str = MACOS_FLOOR) -> tuple[str, dict[str, Any]]:
    """The hardened installer and a report of where each insertion went.
    Bytes outside the two insertions are unchanged."""
    if PREFLIGHT_FUNCTION in text or BLOCK_BEGIN.strip() in text:
        raise InstallerError(source, "is already hardened (preflight marker present)")
    offsets: dict[str, int] = {}
    for anchor in ANCHORS:
        found = [m.start() for m in re.finditer(re.escape(anchor.text), text)]
        if len(found) != 1:
            lines = ", ".join(str(_line(text, at + 1)) for at in found) or "none"
            raise InstallerError(source, f"template anchor {anchor.name!r} must occur exactly once, found {len(found)} (lines: {lines}): {anchor.text.strip()!r}")
        offsets[anchor.name] = found[0]
    order = [anchor.name for anchor in ANCHORS]
    for before, after in zip(order, order[1:]):
        if offsets[before] >= offsets[after]:
            raise InstallerError(f"{source}:{_line(text, offsets[after] + 1)}", f"template anchor {after!r} must follow {before!r}")
    entry = ANCHORS[-1]
    if not text.endswith(entry.text):
        raise InstallerError(f"{source}:{_line(text, offsets[entry.name] + 1)}", f"template anchor {entry.name!r} must end the installer")

    floor = _floor(glibc, "glibc")
    calls = _DIST_GLIBC_CHECK.findall(text)
    if not calls:
        raise InstallerError(source, "template has no dist `check_glibc` call; was [dist.min-glibc-version] dropped?")
    for major, minor in calls:
        if (int(major), int(minor)) != floor:
            raise InstallerError(source, f"dist's check_glibc \"{major}\" \"{minor}\" disagrees with the {glibc} floor")

    call_at = offsets["architecture-detection"] + 1
    block_at = offsets["entry-point"] + 1
    block = preflight_block(glibc, macos)
    hardened = text[:call_at] + CALL + text[call_at:block_at] + block + text[block_at:]
    report = {
        "source": source,
        "glibc_floor": glibc,
        "macos_floor": macos,
        "anchors": {name: _line(text, at + 1) for name, at in offsets.items()},
        "insertions": [
            {"at_line": _line(text, call_at), "lines": CALL.count("\n"), "what": "preflight call"},
            {"at_line": _line(text, block_at), "lines": block.count("\n"), "what": "preflight functions"},
        ],
    }
    return hardened, report


def _parse_sums(path: Path, text: str) -> list[tuple[int, str, str]]:
    """(line, name, digest) for each entry. dist ends `sha256.sum` with a blank
    line, so blank lines are skipped; any other unrecognized line is an error."""
    entries = []
    for number, line in enumerate(text.splitlines(), 1):
        if not line.strip():
            continue
        match = _SUM_LINE.match(line)
        if not match:
            raise InstallerError(f"{path.as_posix()}:{number}", f"is not '<sha256> *<name>': {line!r}")
        entries.append((number, match.group(2), match.group(1)))
    return entries


def check_sums(assets: Path, *, run: Callable[..., subprocess.CompletedProcess[str]] = subprocess.run) -> dict[str, Any]:
    """Every checksum dist wrote matches the bytes it names, every sidecar
    agrees with `sha256.sum`, and every archive is covered by both."""
    where = assets.as_posix()
    if not assets.is_dir():
        raise InstallerError(where, "is not a directory")
    unified = assets / "sha256.sum"
    if not unified.is_file():
        raise InstallerError(unified.as_posix(), "dist's unified checksum file is missing")
    sums: dict[str, str] = {}
    for number, name, digest in _parse_sums(unified, _read_text(unified)):
        if name in sums:
            raise InstallerError(f"{unified.as_posix()}:{number}", f"lists {name!r} twice")
        sums[name] = digest

    sidecars = sorted(assets.glob("*.sha256"))
    for sidecar in sidecars:
        name = sidecar.name[: -len(".sha256")]
        entries = _parse_sums(sidecar, _read_text(sidecar))
        if len(entries) != 1 or entries[0][1] != name:
            raise InstallerError(sidecar.as_posix(), f"must hold exactly one entry for {name!r}, found {[e[1] for e in entries]}")
        if name not in sums:
            raise InstallerError(unified.as_posix(), f"has no entry for {name!r} (sidecar {sidecar.name} exists)")
        if sums[name] != entries[0][2]:
            raise InstallerError(sidecar.as_posix(), f"disagrees with sha256.sum for {name!r}")

    sidecar_names = {sidecar.name[: -len(".sha256")] for sidecar in sidecars}
    for name in sorted(sums):
        target = assets / name
        if target.is_symlink() or not target.is_file():
            raise InstallerError(unified.as_posix(), f"references {name!r}, which is not a regular file in {where}")
        if name not in sidecar_names:
            raise InstallerError(where, f"{name}.sha256 sidecar is missing for an entry in sha256.sum")
    for archive in sorted(assets.glob("*.tar.xz")):
        if archive.name not in sums:
            raise InstallerError(unified.as_posix(), f"has no entry for archive {archive.name!r}")

    try:
        dist_view = distribution.read_checksums(assets)
    except distribution.DistributionError as error:
        raise InstallerError(where, f"distribution.read_checksums disagrees: {error}") from None
    if dist_view != sums:
        raise InstallerError(where, "distribution.read_checksums disagrees with this verification")

    tool = shutil.which("shasum")
    if tool is None:
        raise InstallerError(where, "the platform checksum tool `shasum` is not installed")
    files = ["./sha256.sum", *(f"./{sidecar.name}" for sidecar in sidecars)]
    result = run([tool, "-a", "256", "-c", *files], cwd=assets, capture_output=True, text=True, check=False)
    ok: dict[str, int] = {}
    failed: list[str] = []
    for line in result.stdout.splitlines():
        name, _, status = line.rpartition(": ")
        if status == "OK":
            ok[name] = ok.get(name, 0) + 1
        elif name or status:
            failed.append(line)
    want = {name: 1 + (name in sidecar_names) for name in sums}
    if result.returncode != 0 or failed or ok != want:
        detail = "; ".join(failed) or result.stderr.strip() or f"checked {ok}, expected {want}"
        raise InstallerError(where, f"stale or unverifiable checksum (shasum -a 256 -c exited {result.returncode}): {detail}")

    listed = set(sums) | {"sha256.sum"} | {sidecar.name for sidecar in sidecars}
    return {
        "assets": where,
        "verified": [{"name": name, "sha256": sums[name], "sources": ["sha256.sum", f"{name}.sha256"]} for name in sorted(sums)],
        "uncovered": sorted(p.name for p in assets.iterdir() if p.is_file() and p.name not in listed),
    }


def main(argv: list[str] | None = None, *, build_images: Path = BUILD_IMAGES) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    commands = parser.add_subparsers(dest="command", required=True)
    harden_cmd = commands.add_parser("harden", help="insert the platform preflight into a dist shell installer")
    harden_cmd.add_argument("installer", type=Path)
    harden_cmd.add_argument("--output", type=Path, required=True, help="where to write the hardened installer (may equal the input)")
    sums_cmd = commands.add_parser("check-sums", help="verify dist's sha256 sidecars and sha256.sum with shasum")
    sums_cmd.add_argument("assets", type=Path)
    args = parser.parse_args(argv)
    try:
        if args.command == "harden":
            text = _read_text(args.installer)
            hardened, report = harden(text, source=args.installer.as_posix(), macos=macos_floor(build_images))
            args.output.write_text(hardened, encoding="utf-8")
            if args.output.resolve() != args.installer.resolve():
                shutil.copymode(args.installer, args.output)
            report["output"] = args.output.as_posix()
        else:
            report = check_sums(args.assets)
    except (InstallerError, distribution.DistributionError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())

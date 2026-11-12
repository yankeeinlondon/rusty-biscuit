#!/usr/bin/env python3
"""Runtime verification of locally built distribution outputs.

`run` takes a directory holding what the release workflow's Package job
uploads (`<package>-<target>.tar.xz`, its `.sha256`, `sha256.sum`, and the
hardened `<package>-installer.sh`) and, per target, installs the actual
archive and the actual shell installer into isolated temporary roots, runs
only `<command> --version` and `--help`, and requires that exactly the one
command was installed and no shell profile was written. Nothing else is ever
executed: no playback, interactive provider flow, or language-server loop.

- macOS targets run natively. A host with developer tools or Homebrew yields
  `native-build-host` evidence and a separate `pending-infrastructure`
  clean-runtime entry; only a host with neither is `native-clean-runtime`.
- Linux targets run in the pinned Ubuntu 22.04 runtime image from
  `.github/distribution/build-images.json` with `--network none`: the archive
  in the image itself, the installer in that image plus a downloader and `xz`
  (never a compiler), reading the archive through dist's
  `<PACKAGE>_DOWNLOAD_URL` override as a `file://` URL. `ldd` is retained and
  checked with `inspect_archive.py`'s allowed-library rules. The installer
  must also refuse in Ubuntu 20.04 (glibc 2.31) and leave its install root,
  HOME, and TMPDIR empty. A Docker engine of another CPU would only emulate,
  so such a target is `pending-infrastructure`, as are WSL2 runs off WSL2.
- `brew` and `binstall` are `disabled` unless `docs/deployment.yaml` enables
  them for the package; an enabled channel needs a live Release and is
  `pending-infrastructure` here.

Pending and disabled entries are never passes. Exit status: 0 when no check
failed, 1 when one did, 2 on unusable input (a located message; nothing has
been installed). Digests come from dist's own `.sha256` records and are
checked with `shasum -a 256 -c`; no digest is computed in Python.

`combine` folds this report and `verify_binstall.py fixture-check`'s report
into the `report.json` and `README.md` that `verify.sh` writes.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Callable, Iterable, Sequence

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE.parent / "ci"))
sys.path.insert(0, str(_HERE))
import distribution  # noqa: E402
import inspect_archive  # noqa: E402

REPOSITORY = _HERE.parents[1]
BUILD_IMAGES = REPOSITORY / ".github" / "distribution" / "build-images.json"
POLICY = REPOSITORY / "docs" / "deployment.yaml"
REPORT_NAME = "runtime-report.json"
SCHEMA_VERSION = 1

EVIDENCE_CLASSES = ("native-clean-runtime", "native-build-host", "emulated", "wsl2",
                    "pending-infrastructure", "disabled")
RUN_CLASSES = frozenset(EVIDENCE_CLASSES[:4])
STATUSES = ("pass", "fail", "pending", "disabled")
CHANNELS = ("brew", "binstall")

SYSTEM_PATH = "/usr/bin:/bin:/usr/sbin:/sbin"
COMMAND_TIMEOUT = 60
INSTALL_TIMEOUT = 300
DOCKER_TIMEOUT = 900
CONTAINER_BUILD_TOOLS = ("cc", "gcc", "clang", "ld", "rustc", "cargo")
# A shell profile or PATH hook an installer could write; anything else under
# the scratch HOME is reported as a plain extra file.
PROFILE_PATHS = (".profile", ".bashrc", ".bash_profile", ".bash_login", ".bash_logout", ".zshrc", ".zshenv",
                 ".zprofile", ".zlogin", ".config/fish", ".cargo/env", ".cargo/env.fish")
# What a downloader-equipped runtime needs to run dist's installer offline.
INSTALLER_PACKAGES = ("curl", "xz-utils")
# Ubuntu 20.04 ships glibc 2.31, below the 2.35 floor. Not in
# build-images.json because nothing is built on it; pinned the same way.
OLD_GLIBC_IMAGE = {
    "image": "ubuntu:20.04",
    "index_digest": "sha256:8feb4d8ca5354def3d8fce243717141ce31e2c428701f6682bd2fafe15388214",
    "platform_digests": {
        "amd64": "sha256:c664f8f86ed5a386b0a340d981b8f81714e21a8b9c73f658c4bea56aa179d54a",
        "arm64": "sha256:722ea796ac2d57eeb3627c58a582fc1acc58be51faf815e1bce1682ae5c092f7",
    },
    "glibc": "2.31",
}
_ARCH = {"arm64": "aarch64", "aarch64": "aarch64", "x86_64": "x86_64", "amd64": "x86_64"}
_SIDECAR = re.compile(r"^([0-9a-f]{64}) [ *](\S+)$")
_DYLD_LINE = re.compile(r"^dyld\[\d+\]: (?:<[^>]+> )?(/.*)$")
MACOS_RUNTIME_PREFIXES = ("/usr/lib/", "/System/Library/")


class VerifyError(Exception):
    """Unusable input, located by `where` (a path or option)."""

    def __init__(self, where: str, message: str) -> None:
        super().__init__(f"{where}: {message}")
        self.where = where


# ---------------------------------------------------------------------------
# Checks and the report
# ---------------------------------------------------------------------------


@dataclass
class Check:
    """One verification outcome. A pass needs an evidence class that ran
    something; `pending-infrastructure` is always `pending` and `disabled`
    always `disabled`, each with a reason."""

    target: str
    route: str
    evidence_class: str
    status: str
    reason: str = ""
    details: dict[str, Any] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if self.evidence_class not in EVIDENCE_CLASSES:
            raise ValueError(f"unknown evidence class {self.evidence_class!r}")
        if self.status not in STATUSES:
            raise ValueError(f"unknown status {self.status!r}")
        expected = {"pending-infrastructure": "pending", "disabled": "disabled"}.get(self.evidence_class)
        if expected is not None and self.status != expected:
            raise ValueError(f"{self.evidence_class} evidence must be {expected!r}, not {self.status!r}")
        if expected is None and self.status not in ("pass", "fail"):
            raise ValueError(f"{self.evidence_class} evidence ran, so it is pass or fail, not {self.status!r}")
        if self.status != "pass" and not self.reason:
            raise ValueError(f"a {self.status} check needs a reason")


def observed(target: str, route: str, evidence_class: str, problems: Sequence[str],
             details: dict[str, Any]) -> Check:
    if evidence_class not in RUN_CLASSES:
        raise ValueError(f"{evidence_class} is not an evidence class that runs anything")
    return Check(target, route, evidence_class, "fail" if problems else "pass", "; ".join(problems), details)


def pending(target: str, route: str, reason: str) -> Check:
    return Check(target, route, "pending-infrastructure", "pending", reason)


def summarize(checks: Sequence[Check]) -> dict[str, Any]:
    counts = {status: sum(1 for check in checks if check.status == status) for status in STATUSES}
    if counts["fail"]:
        result = "fail"
    elif counts["pending"] or not counts["pass"]:
        result = "incomplete"
    else:
        result = "pass"
    return {"result": result, "passed": counts["pass"], "failed": counts["fail"],
            "pending": counts["pending"], "disabled": counts["disabled"]}


def exit_status(summary: dict[str, Any]) -> int:
    return 1 if summary["result"] == "fail" else 0


# ---------------------------------------------------------------------------
# Evidence classification
# ---------------------------------------------------------------------------


def target_arch(config: dict[str, Any], target: str) -> str:
    settings = config.get("targets", {}).get(target)
    if not isinstance(settings, dict) or settings.get("arch") not in _ARCH:
        raise VerifyError(f"{BUILD_IMAGES.name}: targets.{target}", "names no known arch")
    return _ARCH[settings["arch"]]


def classify_macos(host: dict[str, Any], arch: str) -> tuple[str, str]:
    """The evidence class a native macOS run on `host` would produce."""
    if host["os"] != "Darwin":
        return "pending-infrastructure", f"no macOS host (this host is {host['os']} {host['arch']})"
    if host["arch"] != arch:
        return ("pending-infrastructure",
                f"no native {arch} macOS host (this host is {host['arch']}; Rosetta would be emulation)")
    if host["build_tools"]:
        return "native-build-host", f"host has {', '.join(host['build_tools'])}"
    return "native-clean-runtime", "macOS host without developer tools or Homebrew"


def classify_linux_container(docker_arch: str | None, arch: str) -> tuple[str, str]:
    """A container run is native only when the Docker engine has the target's CPU."""
    if docker_arch is None:
        return "pending-infrastructure", f"no Docker engine to run the pinned {arch} runtime image"
    if docker_arch != arch:
        return ("pending-infrastructure",
                f"no native {arch} Linux host (this Docker engine is {docker_arch}; an emulated container "
                "is not runtime evidence)")
    return "native-clean-runtime", f"pinned runtime image on a native {arch} Docker engine"


def classify_wsl2(host: dict[str, Any], arch: str) -> tuple[str, str]:
    if not host["wsl2"]:
        return "pending-infrastructure", "no WSL2 host to run the Linux artifact (WSL2 is not native Windows)"
    if host["arch"] != arch:
        return "pending-infrastructure", f"this WSL2 host is {host['arch']}, not {arch}"
    return "wsl2", "Linux artifact on WSL2 (not native Windows evidence)"


# ---------------------------------------------------------------------------
# Inputs: download identity, before anything is installed
# ---------------------------------------------------------------------------


@dataclass
class Artifact:
    target: str
    archive: Path
    sidecar: Path
    sha256: str
    size: int
    sha256_sum: str | None
    installer_sha256: str | None
    shasum_check: str

    def identity(self) -> dict[str, Any]:
        return {"file": self.archive.name, "size": self.size, "sha256": self.sha256,
                "sha256_source": self.sidecar.name, "verified_with": "shasum -a 256 -c",
                "shasum_check": self.shasum_check, "sha256_sum_entry": self.sha256_sum,
                "installer_embedded_sha256": self.installer_sha256}


def run_tool(argv: Sequence[str | os.PathLike[str]], *, timeout: int, cwd: Path | None = None,
             env: dict[str, str] | None = None) -> tuple[int | None, str]:
    """Status (None on timeout) and combined output; stdin is closed."""
    try:
        result = subprocess.run([str(arg) for arg in argv], cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=timeout, check=False)
    except subprocess.TimeoutExpired as error:
        output = error.stdout or b""
        return None, (output.decode("utf-8", "replace") if isinstance(output, bytes) else output)
    except OSError as error:
        return 127, str(error)
    return result.returncode, result.stdout.decode("utf-8", "replace")


def _text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError as error:
        raise VerifyError(str(path), f"is not UTF-8 text: {error}") from None


def read_sidecar(path: Path, name: str) -> str:
    lines = [line for line in _text(path).splitlines() if line.strip()]
    match = _SIDECAR.match(lines[0]) if len(lines) == 1 else None
    if not match:
        raise VerifyError(str(path), "is not one '<sha256> *<file>' line")
    if match.group(2) != name:
        raise VerifyError(str(path), f"names {match.group(2)!r}, not {name!r}")
    return match.group(1)


def sum_entry(path: Path, name: str) -> str | None:
    found = None
    for number, line in enumerate(_text(path).splitlines(), 1):
        if not line.strip():
            continue
        match = _SIDECAR.match(line)
        if not match:
            raise VerifyError(f"{path}:{number}", "is not a '<sha256> *<file>' line")
        if match.group(2) == name:
            if found is not None:
                raise VerifyError(f"{path}:{number}", f"lists {name} twice")
            found = match.group(1)
    return found


def installer_entry(text: str, archive: str) -> tuple[bool, str | None]:
    """Whether dist's installer names `archive`, and the SHA-256 it embeds for it."""
    match = re.search(r'\n\s*"' + re.escape(archive) + r'"\)\n(.*?)\n\s*;;', text, re.S)
    if not match:
        return False, None
    value = re.search(r'_checksum_value="([0-9a-f]{64})"', match.group(1))
    return True, value.group(1) if value else None


def archive_executables(path: Path, root: str) -> list[str]:
    try:
        with tarfile.open(path, "r:xz") as archive:
            members = archive.getmembers()
    except (tarfile.TarError, OSError, EOFError) as error:
        raise VerifyError(str(path), f"is not a readable .tar.xz: {error}") from None
    for member in members:
        if member.name.rstrip("/") != root and not member.name.startswith(root + "/"):
            raise VerifyError(str(path), f"member {member.name!r} is outside the {root}/ root")
    return sorted(member.name[len(root) + 1:] for member in members if member.isfile() and member.mode & 0o111)


def load_artifact(artifacts: Path, package: str, target: str, command: str, installer_text: str,
                  installer: Path) -> Artifact:
    name = f"{package}-{target}.tar.xz"
    archive, sidecar = artifacts / name, artifacts / f"{name}.sha256"
    for path in (archive, sidecar):
        if not path.is_file():
            raise VerifyError(str(path), "does not exist; build the target and its global step first")
    digest = read_sidecar(sidecar, name)
    status, output = run_tool(["shasum", "-a", "256", "-c", sidecar.name], cwd=artifacts, timeout=COMMAND_TIMEOUT)
    if status != 0:
        raise VerifyError(str(sidecar), f"shasum -a 256 -c failed (exit {status}): {output.strip()}")
    sums = artifacts / "sha256.sum"
    listed = None
    if sums.is_file():
        listed = sum_entry(sums, name)
        if listed is None:
            raise VerifyError(str(sums), f"has no entry for {name}")
        if listed != digest:
            raise VerifyError(str(sums), f"lists {listed} for {name}; {sidecar.name} says {digest}")
    named, embedded = installer_entry(installer_text, name)
    if not named:
        raise VerifyError(str(installer), f"offers no {name}")
    if embedded is not None and embedded != digest:
        raise VerifyError(str(installer), f"embeds {embedded} for {name}; {sidecar.name} says {digest}")
    executables = archive_executables(archive, f"{package}-{target}")
    if command not in executables:
        raise VerifyError(str(archive), f"has no executable {package}-{target}/{command}; it ships {executables}")
    return Artifact(target, archive, sidecar, digest, archive.stat().st_size, listed, embedded, output.strip())


def load_inputs(artifacts: Path, package: str, command: str, version: str,
                targets: Iterable[str]) -> tuple[Path, dict[str, Artifact]]:
    """Every runnable target's inputs, checked before anything is installed."""
    if distribution.ROSTER.get(package) != command:
        raise VerifyError("--command", f"{package} ships {distribution.ROSTER.get(package)!r}, not {command!r}")
    installer = artifacts / f"{package}-installer.sh"
    if not installer.is_file():
        raise VerifyError(str(installer), "does not exist; run dist's global step and installers.py harden")
    text = _text(installer)
    for key, want in (("APP_NAME", package), ("APP_VERSION", version)):
        match = re.search(rf'^{key}="([^"]*)"$', text, re.M)
        if not match or match.group(1) != want:
            raise VerifyError(str(installer), f"{key} is {match.group(1) if match else None!r}, expected {want!r}")
    if "biscuit_distribution_preflight" not in text:
        raise VerifyError(str(installer), "has no platform preflight; run installers.py harden on it")
    return installer, {target: load_artifact(artifacts, package, target, command, text, installer)
                       for target in targets}


# ---------------------------------------------------------------------------
# Installed-tree audits
# ---------------------------------------------------------------------------

Entry = tuple[str, str, bool]  # relative path, kind (f, d, l, other), executable


def list_tree(root: Path) -> list[Entry]:
    entries: list[Entry] = []
    for path in sorted(root.rglob("*")):
        mode = path.lstat().st_mode
        kind = "l" if path.is_symlink() else "d" if path.is_dir() else "f" if path.is_file() else "other"
        entries.append((path.relative_to(root).as_posix(), kind, bool(mode & 0o111) and kind == "f"))
    return entries


def parse_find_listing(text: str) -> list[Entry]:
    """`find DIR -mindepth 1 -printf '%P\\t%y\\t%m\\n'` output."""
    entries = []
    for line in text.splitlines():
        if not line:
            continue
        path, kind, mode = line.split("\t")
        entries.append((path, kind, kind == "f" and bool(int(mode, 8) & 0o111)))
    return entries


def audit_tree(entries: Sequence[Entry], expected: dict[str, bool]) -> list[str]:
    """`expected` maps each required file to whether it must be executable;
    anything else, other than the directories holding those files, is extra."""
    problems = []
    parents = {"/".join(path.split("/")[:depth]) for path in expected for depth in range(1, path.count("/") + 1)}
    seen = set()
    for path, kind, executable in entries:
        if kind == "d":
            if path not in parents:
                problems.append(f"extra directory {path}")
            continue
        if path not in expected:
            problems.append(f"extra file {path}")
            continue
        seen.add(path)
        if kind != "f":
            problems.append(f"{path} is not a regular file")
        elif expected[path] and not executable:
            problems.append(f"{path} is not executable")
        elif not expected[path] and executable:
            problems.append(f"{path} is unexpectedly executable")
    problems.extend(f"missing {path}" for path in sorted(set(expected) - seen))
    return problems


def audit_home(entries: Sequence[Entry]) -> list[str]:
    problems = []
    for path, _, _ in entries:
        if any(path == profile or path.startswith(profile + "/") for profile in PROFILE_PATHS):
            problems.append(f"wrote shell profile {path}; an isolated install never edits PATH")
        elif not any(other != path and other.startswith(path + "/") for other, _, _ in entries):
            problems.append(f"wrote {path} under HOME")
    return problems


def command_problems(command: str, version: str, flag: str, status: int | None, output: str) -> list[str]:
    if status != 0:
        return [f"{command} {flag} exited {'timeout' if status is None else status}"]
    if not output.strip():
        return [f"{command} {flag} printed nothing"]
    if flag == "--version":
        first = output.strip().splitlines()[0]
        if first.split()[:2] != [command, version]:
            return [f"{command} --version printed {first!r}, expected '{command} {version}'"]
    return []


def ldd_problems(text: str, config: dict[str, Any], package: str) -> tuple[list[dict[str, Any]], list[str]]:
    report = inspect_archive.Report("ldd")
    inspect_archive.check_ldd(text, config, package, report, "ldd")
    return report.data["ldd"], [failure["reason"] for failure in report.failures]


def dyld_problems(text: str, binary: str, config: dict[str, Any]) -> tuple[list[str], list[str]]:
    """Libraries dyld loaded (`DYLD_PRINT_LIBRARIES`), all system-owned."""
    fragments = config.get("forbidden_library_patterns", [])
    loaded = [match.group(1) for match in map(_DYLD_LINE.match, text.splitlines()) if match]
    problems = []
    if not loaded:
        problems.append("dyld reported no loaded libraries")
    for path in loaded:
        if os.path.realpath(path) == os.path.realpath(binary):
            continue
        if not path.startswith(MACOS_RUNTIME_PREFIXES):
            problems.append(f"loaded {path}, outside {list(MACOS_RUNTIME_PREFIXES)}")
        elif any(fragment in path.lower() for fragment in fragments):
            problems.append(f"loaded forbidden library {path}")
    return loaded, problems


# ---------------------------------------------------------------------------
# Native runs (macOS, and the Linux artifact on WSL2)
# ---------------------------------------------------------------------------


@dataclass
class Context:
    package: str
    command: str
    version: str
    config: dict[str, Any]
    out: Path
    work: Path
    header: str = ""

    @property
    def prefix(self) -> str:
        return self.package.upper().replace("-", "_")

    def transcript(self, target: str, route: str, steps: Sequence[dict[str, Any]], header: str = "") -> Path:
        path = self.out / target / f"{route}.txt"
        path.parent.mkdir(parents=True, exist_ok=True)
        body = "".join(f"\n$ {step['argv']}\n{step['output'].rstrip()}\n[exit {step['status']}]\n" for step in steps)
        path.write_text((header or self.header) + body, encoding="utf-8")
        return path


def _step(argv: str, status: int | None, output: str) -> dict[str, Any]:
    return {"argv": argv, "status": status, "output": output}


def observe_native_libraries(ctx: Context, target: str, binary: Path, env: dict[str, str]) -> tuple[dict[str, Any], list[str]]:
    if target.endswith("-apple-darwin"):
        try:
            result = subprocess.run([str(binary), "--version"], env={**env, "DYLD_PRINT_LIBRARIES": "1"},
                                    stdin=subprocess.DEVNULL, capture_output=True, timeout=COMMAND_TIMEOUT, check=False)
            text = result.stderr.decode("utf-8", "replace")
        except (OSError, subprocess.TimeoutExpired) as error:
            return {"dyld": None}, [f"dyld observation failed: {error}"]
        loaded, problems = dyld_problems(text, str(binary), ctx.config)
        return {"dyld_loaded": loaded, "step": _step(f"DYLD_PRINT_LIBRARIES=1 {ctx.command} --version",
                                                     result.returncode, "\n".join(loaded))}, problems
    status, text = run_tool(["ldd", binary], timeout=COMMAND_TIMEOUT)
    step = _step(f"ldd {ctx.command}", status, text)
    if status != 0:
        return {"step": step}, [f"ldd exited {status}"]
    rows, problems = ldd_problems(text, ctx.config, ctx.package)
    return {"ldd": rows, "step": step}, problems


Observer = Callable[[Context, str, Path, dict[str, str]], tuple[dict[str, Any], list[str]]]


def _run_installed(ctx: Context, binary: Path, env: dict[str, str], steps: list[dict[str, Any]]) -> list[str]:
    problems = []
    for flag in ("--version", "--help"):
        status, output = run_tool([binary, flag], env=env, timeout=COMMAND_TIMEOUT)
        steps.append(_step(f"{ctx.command} {flag}", status, output))
        problems += command_problems(ctx.command, ctx.version, flag, status, output)
    return problems


def native_archive(ctx: Context, artifact: Artifact, evidence_class: str, route: str = "archive",
                   observe: Observer = observe_native_libraries) -> Check:
    work = ctx.work / artifact.target / route
    root, home = work / "root", work / "home"
    root.mkdir(parents=True)
    home.mkdir()
    steps: list[dict[str, Any]] = []
    status, output = run_tool(["tar", "-xf", artifact.archive, "-C", root], timeout=INSTALL_TIMEOUT)
    steps.append(_step(f"tar -xf {artifact.archive.name}", status, output))
    if status != 0:
        return observed(artifact.target, route, evidence_class, [f"tar exited {status}"], {"steps": steps})
    base = f"{ctx.package}-{artifact.target}"
    expected = {f"{base}/{ctx.command}": True, **{f"{base}/{name}": False for name in inspect_archive.NOTICE_FILES}}
    entries = list_tree(root)
    problems = audit_tree(entries, expected)
    binary = root / base / ctx.command
    env = {"PATH": SYSTEM_PATH, "HOME": str(home)}
    libraries: dict[str, Any] = {}
    if binary.is_file():
        problems += _run_installed(ctx, binary, env, steps)
        libraries, library_problems = observe(ctx, artifact.target, binary, env)
        problems += library_problems
        if "step" in libraries:
            steps.append(libraries.pop("step"))
    problems += audit_home(list_tree(home))
    transcript = ctx.transcript(artifact.target, route, steps)
    return observed(artifact.target, route, evidence_class, problems,
                    {"installed": [entry[0] for entry in entries], "steps": steps, "libraries": libraries,
                     "transcript": transcript.relative_to(ctx.out).as_posix()})


def native_installer(ctx: Context, artifact: Artifact, installer: Path, evidence_class: str,
                     route: str = "installer", observe: Observer = observe_native_libraries) -> Check:
    work = ctx.work / artifact.target / route
    root, home, tmp = work / "root", work / "home", work / "tmp"
    for path in (root, home, tmp):
        path.mkdir(parents=True)
    env = {"PATH": SYSTEM_PATH, "HOME": str(home), "TMPDIR": str(tmp),
           f"{ctx.prefix}_UNMANAGED_INSTALL": str(root),
           f"{ctx.prefix}_DOWNLOAD_URL": artifact.archive.parent.resolve().as_uri()}
    steps: list[dict[str, Any]] = []
    status, output = run_tool(["sh", installer], env=env, timeout=INSTALL_TIMEOUT)
    steps.append(_step(f"sh {installer.name}", status, output))
    problems = [] if status == 0 else [f"the installer exited {status}"]
    entries = list_tree(root)
    problems += audit_tree(entries, {ctx.command: True})
    problems += audit_home(list_tree(home))
    libraries: dict[str, Any] = {}
    run_env = {"PATH": SYSTEM_PATH, "HOME": str(home)}
    if status == 0 and (root / ctx.command).is_file():
        problems += _run_installed(ctx, root / ctx.command, run_env, steps)
        libraries, library_problems = observe(ctx, artifact.target, root / ctx.command, run_env)
        problems += library_problems
        if "step" in libraries:
            steps.append(libraries.pop("step"))
    transcript = ctx.transcript(artifact.target, route, steps)
    return observed(artifact.target, route, evidence_class, problems,
                    {"installed": [entry[0] for entry in entries], "tmp_left": [e[0] for e in list_tree(tmp)],
                     "dist_checksum_skipped": "skipping sha256 checksum verification" in output,
                     "steps": steps, "libraries": libraries,
                     "transcript": transcript.relative_to(ctx.out).as_posix()})


# ---------------------------------------------------------------------------
# Linux containers
# ---------------------------------------------------------------------------

_CONTAINER_PRELUDE = r"""
set -u
out=/out
mkdir -p /scratch/root /scratch/home /scratch/tmp
{ echo "\$ uname -a"; uname -a; echo; echo "\$ cat /etc/os-release"; cat /etc/os-release
  echo; echo "\$ ldd --version"; ldd --version 2>&1 | awk 'NR == 1'
  echo; echo "\$ getconf GNU_LIBC_VERSION"; getconf GNU_LIBC_VERSION
  echo; echo "\$ dpkg-query (downloader packages)"; dpkg-query -W -f '${Package} ${Version}\n' curl xz-utils 2>/dev/null || true
} >"$out/os.txt" 2>&1
for tool in $BUILD_TOOLS; do command -v "$tool" || true; done >"$out/build-tools.txt"
run() { name=$1; shift; "$@" </dev/null >"$out/$name.out" 2>&1; echo $? >"$out/$name.status"; }
listing() { find "$1" -mindepth 1 -printf '%P\t%y\t%m\n' | sort >"$out/$2.tsv"; }
"""

_ARCHIVE_SCRIPT = _CONTAINER_PRELUDE + r"""
binary="/dist/$ROOT/$COMMAND"
run version env -i PATH="$SYSTEM_PATH" HOME=/scratch/home "$binary" --version
run help env -i PATH="$SYSTEM_PATH" HOME=/scratch/home "$binary" --help
run ldd ldd "$binary"
listing /scratch/home home
"""

_INSTALLER_SCRIPT = _CONTAINER_PRELUDE + r"""
run install env -i PATH="$SYSTEM_PATH" HOME=/scratch/home TMPDIR=/scratch/tmp \
    "${PREFIX}_UNMANAGED_INSTALL=/scratch/root" "${PREFIX}_DOWNLOAD_URL=file:///artifacts" \
    sh "/artifacts/$INSTALLER"
listing /scratch/root root
listing /scratch/home home
listing /scratch/tmp tmp
if [ "$INSTALL_ONLY" = 0 ] && [ -f "/scratch/root/$COMMAND" ]; then
    run version env -i PATH="$SYSTEM_PATH" HOME=/scratch/home "/scratch/root/$COMMAND" --version
    run help env -i PATH="$SYSTEM_PATH" HOME=/scratch/home "/scratch/root/$COMMAND" --help
    run ldd ldd "/scratch/root/$COMMAND"
fi
"""


class Docker:
    def __init__(self, binary: str, log_dir: Path) -> None:
        self.binary = binary
        self.log_dir = log_dir
        self._count = 0

    def arch(self) -> str | None:
        status, output = run_tool([self.binary, "info", "--format", "{{.Architecture}}"], timeout=COMMAND_TIMEOUT)
        return _ARCH.get(output.strip()) if status == 0 else None

    def image(self, base: str, platform_name: str, packages: Sequence[str]) -> str:
        """`base` itself, or a local image of it plus `packages` (built with
        network access; everything that runs in it later has none)."""
        if not packages:
            return base
        tag = "biscuit-verify-runtime:" + re.sub(r"[^A-Za-z0-9_.-]", "-",
                                                 f"{base.split('sha256:')[-1][:12]}-{platform_name}-{'_'.join(packages)}")[:100]
        context = Path(tempfile.mkdtemp(prefix="verify-runtime-image-"))
        try:
            (context / "Dockerfile").write_text(
                f"FROM {base}\nRUN apt-get update -qq && DEBIAN_FRONTEND=noninteractive apt-get install -y -qq "
                f"--no-install-recommends {' '.join(packages)} >/dev/null && rm -rf /var/lib/apt/lists/*\n",
                encoding="utf-8")
            status, output = run_tool([self.binary, "build", "--platform", platform_name, "-t", tag, str(context)],
                                      timeout=DOCKER_TIMEOUT)
        finally:
            shutil.rmtree(context, ignore_errors=True)
        self.log_dir.mkdir(parents=True, exist_ok=True)
        (self.log_dir / f"{tag.split(':')[1]}.log").write_text(output, encoding="utf-8")
        if status != 0:
            raise VerifyError(f"docker build {tag}", f"exit {status}; see {self.log_dir}")
        return tag

    def run(self, image: str, platform_name: str, mounts: Sequence[tuple[Path, str]], env: dict[str, str],
            script: str) -> None:
        """Runs `script` with no network. The container is removed explicitly
        rather than with `--rm`: some engines report a finished `--rm`
        container as missing."""
        self._count += 1
        name = f"verify-runtime-{os.getpid()}-{self._count}"
        argv = [self.binary, "run", "--name", name, "--platform", platform_name, "--network", "none"]
        for host, container in mounts:
            argv += ["-v", f"{host}:{container}"]
        for key, value in env.items():
            argv += ["-e", f"{key}={value}"]
        try:
            run_status, run_output = run_tool([*argv, image, "bash", "-c", script], timeout=DOCKER_TIMEOUT)
            status, state = run_tool([self.binary, "inspect", "-f", "{{.State.Status}} {{.State.ExitCode}}", name],
                                     timeout=COMMAND_TIMEOUT)
        finally:
            run_tool([self.binary, "rm", "-f", name], timeout=COMMAND_TIMEOUT)
        if status != 0 or state.split() != ["exited", "0"]:
            raise VerifyError(f"docker container {name}",
                              f"did not finish cleanly (run exit {run_status}, state {state.strip()!r}): "
                              f"{run_output.strip()[-400:]}")


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace") if path.is_file() else ""


def _status(path: Path) -> int | None:
    text = _read(path).strip()
    return int(text) if text.isdigit() else None


def container_results(ctx: Context, out: Path, *, install_only: bool = False) -> tuple[list[dict[str, Any]], list[str], dict[str, Any]]:
    """Steps, problems, and library observations from a container's /out."""
    steps, problems = [], []
    tools = _read(out / "build-tools.txt").split()
    if tools:
        problems.append(f"runtime image has build tools {tools}")
    if (out / "install.status").is_file():
        steps.append(_step(f"sh {ctx.package}-installer.sh", _status(out / "install.status"), _read(out / "install.out")))
    libraries: dict[str, Any] = {}
    if not install_only:
        for flag, name in (("--version", "version"), ("--help", "help")):
            if not (out / f"{name}.status").is_file():
                problems.append(f"{ctx.command} {flag} did not run")
                continue
            status, output = _status(out / f"{name}.status"), _read(out / f"{name}.out")
            steps.append(_step(f"{ctx.command} {flag}", status, output))
            problems += command_problems(ctx.command, ctx.version, flag, status, output)
        ldd_text = _read(out / "ldd.out")
        steps.append(_step(f"ldd {ctx.command}", _status(out / "ldd.status"), ldd_text))
        rows, ldd_failures = ldd_problems(ldd_text, ctx.config, ctx.package)
        libraries = {"ldd": rows}
        problems += ldd_failures
    return steps, problems, libraries


def linux_routes(ctx: Context, artifact: Artifact, installer: Path, docker: Docker, evidence_class: str) -> list[Check]:
    target, config = artifact.target, ctx.config
    settings = config["targets"][target]
    platform_name = settings["platform"]
    base = f"{config['linux']['image']}@{config['linux']['index_digest']}"
    runtime = [*config["linux"].get("runtime_packages", []),
               *inspect_archive.package_extra(config, ctx.package, "runtime_packages")]
    old_base = f"{OLD_GLIBC_IMAGE['image']}@{OLD_GLIBC_IMAGE['index_digest']}"
    env = {"SYSTEM_PATH": SYSTEM_PATH, "BUILD_TOOLS": " ".join(CONTAINER_BUILD_TOOLS), "COMMAND": ctx.command,
           "PREFIX": ctx.prefix, "INSTALLER": installer.name, "ROOT": f"{ctx.package}-{target}"}
    work = ctx.work / target
    staged = work / "artifacts"
    staged.mkdir(parents=True)
    shutil.copy2(artifact.archive, staged / artifact.archive.name)
    shutil.copy2(installer, staged / installer.name)
    extracted = work / "extracted"
    extracted.mkdir()
    status, output = run_tool(["tar", "-xf", artifact.archive, "-C", extracted], timeout=INSTALL_TIMEOUT)
    if status != 0:
        raise VerifyError(str(artifact.archive), f"tar exited {status}: {output.strip()}")
    images = {
        "archive": docker.image(base, platform_name, runtime),
        "installer": docker.image(base, platform_name, [*runtime, *INSTALLER_PACKAGES]),
        "glibc-refusal": docker.image(old_base, platform_name, list(INSTALLER_PACKAGES)),
    }
    checks = []
    for route in ("glibc-refusal", "archive", "installer"):
        # The container writes to the scratch area, which Docker can mount;
        # the transcript carries everything retained.
        out = work / f"{route}-out"
        out.mkdir()
        mounts = [(out.resolve(), "/out")]
        if route == "archive":
            mounts.append((extracted.resolve(), "/dist:ro"))
            script = _ARCHIVE_SCRIPT
        else:
            mounts.append((staged.resolve(), "/artifacts:ro"))
            script = _INSTALLER_SCRIPT
        run_env = {**env, "INSTALL_ONLY": "1" if route == "glibc-refusal" else "0"}
        docker.run(images[route], platform_name, mounts, run_env, script)
        details: dict[str, Any] = {"image": images[route], "platform": platform_name,
                                   "base": old_base if route == "glibc-refusal" else base,
                                   "os": _read(out / "os.txt")}
        if route == "glibc-refusal":
            problems = refusal_problems(out)
            steps, _, _ = container_results(ctx, out, install_only=True)
        else:
            steps, problems, libraries = container_results(ctx, out)
            details["libraries"] = libraries
            if route == "installer":
                problems += [] if _status(out / "install.status") == 0 else [
                    f"the installer exited {_status(out / 'install.status')}"]
                problems += audit_tree(parse_find_listing(_read(out / "root.tsv")), {ctx.command: True})
                details["installed"] = [e[0] for e in parse_find_listing(_read(out / "root.tsv"))]
                details["tmp_left"] = [e[0] for e in parse_find_listing(_read(out / "tmp.tsv"))]
            else:
                problems += audit_tree(list_tree(extracted), {
                    f"{env['ROOT']}/{ctx.command}": True,
                    **{f"{env['ROOT']}/{name}": False for name in inspect_archive.NOTICE_FILES}})
                details["installed"] = [e[0] for e in list_tree(extracted)]
            problems += audit_home(parse_find_listing(_read(out / "home.tsv")))
        details["steps"] = steps
        header = _read(out / "os.txt") + "".join(
            f"\n{label} listing after the run:\n{_read(out / f'{name}.tsv') or '(empty)'}\n"
            for name, label in (("root", "install root"), ("home", "HOME"), ("tmp", "TMPDIR"))
            if (out / f"{name}.tsv").is_file())
        details["transcript"] = ctx.transcript(target, route, steps, header).relative_to(ctx.out).as_posix()
        checks.append(observed(target, route, evidence_class, problems, details))
    return checks


def refusal_problems(out: Path) -> list[str]:
    """The installer must refuse on glibc 2.31 before writing anything."""
    problems = []
    status, output = _status(out / "install.status"), _read(out / "install.out")
    if status in (0, None):
        problems.append(f"the installer exited {status} on glibc {OLD_GLIBC_IMAGE['glibc']}; it must refuse")
    wanted = f"requires glibc 2.35 or newer; found glibc {OLD_GLIBC_IMAGE['glibc']}"
    if wanted not in output:
        problems.append(f"the refusal does not say {wanted!r}")
    for listing, label in (("root", "install root"), ("home", "HOME"), ("tmp", "TMPDIR")):
        if not (out / f"{listing}.tsv").is_file():
            problems.append(f"no {label} listing was recorded")
        elif parse_find_listing(_read(out / f"{listing}.tsv")):
            problems.append(f"the refused install changed its {label}: {_read(out / f'{listing}.tsv').split()}")
    return problems


# ---------------------------------------------------------------------------
# Channels and the run
# ---------------------------------------------------------------------------


def channel_checks(package: str, enabled: dict[str, str], policy_source: str) -> list[Check]:
    checks = []
    for channel in CHANNELS:
        if channel not in enabled:
            checks.append(Check("*", f"channel:{channel}", "disabled", "disabled",
                                f"{policy_source}: packages.{package}.{channel} is not enabled"))
        else:
            checks.append(pending("*", f"channel:{channel}",
                                  f"{channel} is enabled as {enabled[channel]!r}; it is verified only against a "
                                  "live Release (verify_binstall.py git/manifest, an isolated brew prefix)"))
    return checks


def policy_channels(policy: Path, package: str) -> dict[str, str]:
    try:
        loaded = distribution.load_policy(policy, distribution.cargo_workspace(REPOSITORY))
    except distribution.DistributionError as error:
        raise VerifyError(error.where, error.message) from None
    entry = loaded.entries.get(package)
    return dict(entry.channels) if entry is not None else {}


def _repository_relative(path: Path) -> str:
    try:
        return path.resolve().relative_to(REPOSITORY).as_posix()
    except ValueError:
        return path.as_posix()


def host_facts() -> dict[str, Any]:
    system, arch = platform.system(), _ARCH.get(platform.machine(), platform.machine())
    tools = []
    if system == "Darwin":
        status, output = run_tool(["xcode-select", "-p"], timeout=COMMAND_TIMEOUT)
        if status == 0 and output.strip():
            tools.append(f"developer tools ({output.strip()})")
        tools += [f"Homebrew ({path})" for path in ("/opt/homebrew/bin/brew", "/usr/local/bin/brew")
                  if Path(path).exists()]
    wsl2 = system == "Linux" and "wsl2" in _read(Path("/proc/sys/kernel/osrelease")).lower()
    facts = {"os": system, "arch": arch, "build_tools": tools, "wsl2": wsl2,
             "uname": run_tool(["uname", "-a"], timeout=COMMAND_TIMEOUT)[1].strip()}
    if system == "Darwin":
        facts["macos_version"] = run_tool(["sw_vers", "-productVersion"], timeout=COMMAND_TIMEOUT)[1].strip()
        facts["cpu"] = run_tool(["sysctl", "-n", "machdep.cpu.brand_string"], timeout=COMMAND_TIMEOUT)[1].strip()
    return facts


def plan_targets(config: dict[str, Any], targets: Sequence[str], host: dict[str, Any],
                 docker_arch: str | None) -> dict[str, list[tuple[str, str, str]]]:
    """Per target, the (route, evidence class, reason) entries to produce."""
    plan: dict[str, list[tuple[str, str, str]]] = {}
    for target in targets:
        arch = target_arch(config, target)
        if config["targets"][target]["os"] == "macos":
            cls, reason = classify_macos(host, arch)
            entries = [("archive", cls, reason), ("installer", cls, reason)]
            if cls == "native-build-host":
                entries.append(("clean-runtime", "pending-infrastructure",
                                "no clean macOS 15 VM without developer tools or Homebrew is provisioned"))
        else:
            cls, reason = classify_linux_container(docker_arch, arch)
            entries = [(route, cls, reason) for route in ("glibc-refusal", "archive", "installer")]
            wsl_cls, wsl_reason = classify_wsl2(host, arch)
            entries += [("wsl2-archive", wsl_cls, wsl_reason), ("wsl2-installer", wsl_cls, wsl_reason)]
        plan[target] = entries
    return plan


def verify(*, artifacts: Path, package: str, command: str, version: str, out: Path, targets: Sequence[str],
           config: dict[str, Any], host: dict[str, Any], docker: Docker | None, docker_arch: str | None,
           channels: list[Check], scratch: Path | None = None,
           observe: Observer = observe_native_libraries) -> dict[str, Any]:
    plan = plan_targets(config, targets, host, docker_arch)
    runnable = [t for t, entries in plan.items() if any(cls in RUN_CLASSES for _, cls, _ in entries)]
    installer, inputs = load_inputs(artifacts, package, command, version, runnable)
    out.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="verify-runtime-", dir=scratch))
    header = "".join(f"{key}: {host[key]}\n" for key in ("uname", "macos_version", "cpu", "build_tools") if key in host)
    ctx = Context(package, command, version, config, out, work, header)
    checks: list[Check] = []
    try:
        for target, entries in plan.items():
            artifact = inputs.get(target)
            container_done = False
            for route, cls, reason in entries:
                if cls not in RUN_CLASSES:
                    checks.append(pending(target, route, reason))
                elif route in ("glibc-refusal", "archive", "installer") and config["targets"][target]["os"] == "linux":
                    if not container_done:
                        assert docker is not None and artifact is not None
                        checks += linux_routes(ctx, artifact, installer, docker, cls)
                        container_done = True
                elif route.endswith("installer"):
                    assert artifact is not None
                    checks.append(native_installer(ctx, artifact, installer, cls, route, observe))
                else:
                    assert artifact is not None
                    checks.append(native_archive(ctx, artifact, cls, route, observe))
    finally:
        shutil.rmtree(work, ignore_errors=True)
    checks += channels
    summary = summarize(checks)
    report = {"schema_version": SCHEMA_VERSION, "package": package, "command": command, "version": version,
              "host": host, "docker_arch": docker_arch, "artifacts": str(artifacts),
              "downloads": {target: artifact.identity() for target, artifact in inputs.items()},
              "installer": {"file": installer.name, "size": installer.stat().st_size},
              "summary": summary, "checks": [asdict(check) for check in checks]}
    (out / REPORT_NAME).write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return report


# ---------------------------------------------------------------------------
# Combined report
# ---------------------------------------------------------------------------


def combine(runtime: dict[str, Any], binstall: dict[str, Any], runtime_dir: str = "runtime") -> tuple[dict[str, Any], str]:
    """`runtime_dir` is the runtime report's directory relative to the README."""
    binstall_status = binstall.get("status")
    binstall_check = {"route": "binstall fixture-check", "status": "pass" if binstall_status == "passed" else "fail",
                      "reason": "; ".join(binstall.get("errors") or []) or f"binstall report status {binstall_status!r}",
                      "binstall_version": (binstall.get("binstall") or {}).get("version")}
    summary = dict(runtime["summary"])
    if binstall_check["status"] == "fail":
        summary["result"] = "fail"
        summary["failed"] += 1
    else:
        summary["passed"] += 1
    report = {"schema_version": SCHEMA_VERSION, "summary": summary, "binstall_fixture": binstall_check,
              "runtime": runtime}
    lines = [f"# Distribution verification: {runtime['package']} {runtime['version']}", "",
             f"Result: **{summary['result']}** ({summary['passed']} passed, {summary['failed']} failed, "
             f"{summary['pending']} pending, {summary['disabled']} disabled). Pending and disabled entries "
             "are not passes.", "",
             f"Host: `{runtime['host'].get('uname', '')}`; Docker engine: {runtime['docker_arch'] or 'none'}.", "",
             "| Target | Route | Evidence class | Status | Notes |", "| --- | --- | --- | --- | --- |"]
    for check in runtime["checks"]:
        transcript = check["details"].get("transcript")
        note = check["reason"] or (f"[{transcript}]({runtime_dir}/{transcript})" if transcript else "")
        lines.append(f"| {check['target']} | {check['route']} | {check['evidence_class']} | {check['status']} | "
                     f"{note.replace('|', '/')} |")
    binstall_class = "native-build-host" if runtime["host"].get("build_tools") else "native-clean-runtime"
    lines.append(f"| * | binstall fixture-check | {binstall_class} | {binstall_check['status']} | "
                 f"cargo-binstall {binstall_check['binstall_version']} against a 127.0.0.1 fixture release |")
    lines += ["", "## Downloads", "", "| File | Size (bytes) | SHA-256 (dist `.sha256`, `shasum -a 256 -c`) |",
              "| --- | --- | --- |"]
    for identity in runtime["downloads"].values():
        lines.append(f"| {identity['file']} | {identity['size']} | `{identity['sha256']}` |")
    return report, "\n".join(lines) + "\n"


# ---------------------------------------------------------------------------
# Command line
# ---------------------------------------------------------------------------


def _parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    commands = parser.add_subparsers(dest="mode", required=True)
    run = commands.add_parser("run", help="install and run the built artifacts")
    run.add_argument("--artifacts", type=Path, required=True)
    run.add_argument("--package", required=True)
    run.add_argument("--command", required=True)
    run.add_argument("--version", required=True)
    run.add_argument("--out", type=Path, required=True)
    run.add_argument("--target", action="append", choices=distribution.TARGETS,
                     help="repeatable; default: every distribution target")
    run.add_argument("--config", type=Path, default=BUILD_IMAGES)
    run.add_argument("--policy", type=Path, default=POLICY)
    run.add_argument("--docker", default="docker")
    run.add_argument("--scratch-dir", type=Path, help="parent of the temporary install roots (Docker must mount it)")
    merge = commands.add_parser("combine", help="write report.json and README.md")
    merge.add_argument("--runtime-report", type=Path, required=True)
    merge.add_argument("--binstall-report", type=Path, required=True)
    merge.add_argument("--out", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = _parse_args(sys.argv[1:] if argv is None else argv)
    try:
        if args.mode == "combine":
            report, readme = combine(json.loads(args.runtime_report.read_text(encoding="utf-8")),
                                     json.loads(args.binstall_report.read_text(encoding="utf-8")),
                                     Path(os.path.relpath(args.runtime_report.resolve().parent,
                                                          args.out.resolve())).as_posix())
            args.out.mkdir(parents=True, exist_ok=True)
            (args.out / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
            (args.out / "README.md").write_text(readme, encoding="utf-8")
            print(json.dumps(report["summary"]))
            return exit_status(report["summary"])
        config = json.loads(args.config.read_text(encoding="utf-8"))
        targets = args.target or list(distribution.TARGETS)
        docker = shutil.which(args.docker)
        engine = Docker(docker, args.out / "images") if docker else None
        docker_arch = engine.arch() if engine else None
        report = verify(artifacts=args.artifacts.resolve(), package=args.package, command=args.command,
                        version=args.version, out=args.out.resolve(), targets=targets, config=config,
                        host=host_facts(), docker=engine, docker_arch=docker_arch,
                        channels=channel_checks(args.package, policy_channels(args.policy, args.package),
                                                _repository_relative(args.policy)),
                        scratch=args.scratch_dir)
    except (VerifyError, OSError, ValueError) as error:
        print(f"verify_runtime.py: error: {error}", file=sys.stderr)
        return 2
    print(json.dumps(report["summary"]))
    return exit_status(report["summary"])


if __name__ == "__main__":
    sys.exit(main())

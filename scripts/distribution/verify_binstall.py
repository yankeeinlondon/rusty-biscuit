#!/usr/bin/env python3
"""Nonpublishing verification of the cargo-binstall channel, pinned to
cargo-binstall `BINSTALL_VERSION` (1.26.2).

- `git --package P --target T --root R` installs the current manifest version
  through `cargo-binstall --git <repo-url> P`. Binstall clones the Git default
  HEAD; there is no Git tag selection, and an exact `--version` would only
  override the manifest version, so none is passed.
- `manifest --tag TAG --checkout C --target T` installs a historical version
  from an exact-tag checkout through `--manifest-path <package manifest>`,
  after proving the checkout is that tag and its manifest version equals the
  tag's version.
- `fixture-check --binstall BIN` proves extraction and optional-helper
  handling with the real pinned binstall against a release archive served
  from 127.0.0.1, with no live publication. It is an explicit acceptance
  command, not part of the unit suite. The fixture "binary" is a shell
  script, so it runs on macOS and Linux only.

Every route refuses (exit 2) before any network access when the target is not
a distribution target, the channel is disabled in the policy, the manifest's
binstall metadata is not in its enabled form, or the binstall binary is not
the pinned version. Binstall itself runs with only the `crate-meta-data`
strategy (compile fallback excluded), an isolated `CARGO_HOME`/`HOME` and
install root, and a bounded timeout. Afterwards exactly the roster command
must be installed (feature-gated helpers may be absent; any extra file is an
error), its `--version` must print the manifest version, and the debug log
must show binstall chose the package-tag release URL. Exit 1 is a failed
verification; `DIR/binstall-report.json` records every outcome.
"""

from __future__ import annotations

import argparse
import http.server
import json
import os
import platform
import shutil
import subprocess
import sys
import tarfile
import tempfile
import threading
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Mapping, Sequence

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "ci"))
from distribution import (  # noqa: E402
    BINSTALL_VERSION,
    CANONICAL_URL,
    ROSTER,
    TARGETS,
    Binary,
    DistributionError,
    Package,
    Workspace,
    _check_binstall_metadata,
    cargo_workspace,
    excluded_package_names,
    load_policy,
    load_workspace,
    parse_version,
    recognize_tag,
)

REPORT_NAME = "binstall-report.json"
FIXTURE_MANIFEST = Path(__file__).resolve().parent / "fixtures" / "binstall" / "Cargo.toml.in"
FIXTURE_PACKAGE = "binstall-fixture"
FIXTURE_COMMAND = "fixture-tool"
FIXTURE_VERSION = "0.3.1"
# A second fixture release whose archive lacks the unconditional binary.
FIXTURE_BROKEN_VERSION = "0.3.2"

BINSTALL_TIMEOUT = 900
VERSION_TIMEOUT = 30
GIT_TIMEOUT = 60

# Binstall 1.26.2 names the archive it downloads for installation in one debug
# line, `resolve: Downloading package url=<url> dst=<dir> fmt=<fmt>`. The
# candidate probes ("Checking for package at", "Downloading from") list every
# URL tried, and "Winning URL found!" prints the URL as a Debug struct, so only
# this line proves the choice by its literal URL.
DOWNLOAD_MARKER = "Downloading package url="

Runner = Callable[..., subprocess.CompletedProcess]


class VerificationFailed(Exception):
    """Binstall ran, and its result does not meet the channel contract."""


@dataclass
class Report:
    route: str
    package: str | None = None
    version: str | None = None
    target: str | None = None
    source: str | None = None
    url: str | None = None
    command: str | None = None
    status: str = "refused"
    errors: list[str] = field(default_factory=list)
    binstall: dict[str, Any] = field(default_factory=dict)
    installed: list[str] = field(default_factory=list)
    tolerated_absent: list[str] = field(default_factory=list)
    version_check: dict[str, Any] = field(default_factory=dict)
    cases: list[dict[str, Any]] = field(default_factory=list)

    def write(self, out: Path) -> Path:
        out.mkdir(parents=True, exist_ok=True)
        path = out / REPORT_NAME
        path.write_text(json.dumps(self.__dict__, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        return path


# ---------------------------------------------------------------------------
# Invocation
# ---------------------------------------------------------------------------


def binstall_command(
    binstall: str | Path,
    *,
    package: str,
    target: str,
    root: Path,
    git: str | None = None,
    manifest: Path | None = None,
    allow_insecure_http: bool = False,
) -> list[str]:
    """The pinned binstall invocation for exactly one of the Git or manifest
    routes. `--install-path` takes precedence over `--root` in binstall; both
    point into the isolated root so neither falls back to `CARGO_HOME`."""
    if (git is None) == (manifest is None):
        raise ValueError("exactly one of git or manifest is required")
    argv = [str(binstall)]
    argv += ["--git", git] if git is not None else ["--manifest-path", str(manifest)]
    argv += [
        "--no-confirm",
        "--disable-telemetry",
        # Otherwise binstall reads `~/.git-credentials` and gh's hosts file.
        "--no-discover-github-token",
        "--strategies", "crate-meta-data",
        "--targets", target,
        "--root", str(root),
        "--install-path", str(root / "bin"),
        "--no-track",
        "--log-level", "debug",
    ]
    if allow_insecure_http:
        argv.append("--allow-insecure-http")
    argv.append(package)
    return argv


def isolated_env(base: Mapping[str, str], work: Path) -> dict[str, str]:
    """`base` without binstall/Cargo install settings or GitHub tokens, with a
    fresh `CARGO_HOME` and `HOME` (binstall reads `binstall.toml` from Cargo
    home; its Git client reads the user's Git configuration from `HOME`)."""
    dropped = {"CARGO_HOME", "CARGO_INSTALL_ROOT", "GITHUB_TOKEN", "GH_TOKEN"}
    env = {key: value for key, value in base.items() if key not in dropped and not key.startswith("BINSTALL_")}
    for name in ("cargo-home", "home"):
        (work / name).mkdir(parents=True, exist_ok=True)
    env["CARGO_HOME"] = str(work / "cargo-home")
    env["HOME"] = str(work / "home")
    env["GIT_TERMINAL_PROMPT"] = "0"
    env["GIT_CONFIG_NOSYSTEM"] = "1"
    return env


def release_url(repository: str, package: str, version: str, target: str) -> str:
    return f"{repository}/releases/download/{package}-v{version}/{package}-{target}.tar.xz"


def resolve_binstall(path: str | None) -> str:
    if path is not None:
        return path
    found = shutil.which("cargo-binstall")
    if found is None:
        raise DistributionError("--binstall", "cargo-binstall is not on PATH; pass the pinned binary")
    return found


def check_binstall_version(binstall: str, *, runner: Runner, env: Mapping[str, str]) -> None:
    try:
        result = runner([binstall, "-V"], env=dict(env), stdin=subprocess.DEVNULL, capture_output=True,
                        text=True, timeout=VERSION_TIMEOUT, check=False)
    except (OSError, subprocess.TimeoutExpired) as error:
        raise DistributionError(f"{binstall} -V", f"cannot run: {error}") from None
    found = result.stdout.strip()
    if result.returncode != 0 or found != BINSTALL_VERSION:
        raise DistributionError(
            f"{binstall} -V",
            f"must print exactly {BINSTALL_VERSION}, found {found!r} (exit {result.returncode})",
        )


# ---------------------------------------------------------------------------
# Prechecks (no network)
# ---------------------------------------------------------------------------


def check_target(target: str) -> None:
    if target not in TARGETS:
        raise DistributionError("--target", f"{target!r} is not a distribution target; expected one of {list(TARGETS)}")


def check_channel(policy_path: Path, workspace: Workspace, package: Package) -> None:
    """The package ships a roster command, its policy entry enables
    `binstall` under the crate's own name, and its manifest metadata is in
    the enabled form. A CLI `--strategies` would override disabled metadata,
    so this check is what keeps an opt-out from being installed."""
    if package.name not in ROSTER:
        raise DistributionError(package.manifest_path, f"{package.name!r} ships no distributed command")
    policy = load_policy(policy_path, workspace)
    entry = policy.entries.get(package.name)
    channel = entry.channels.get("binstall") if entry is not None else None
    if channel is None:
        raise DistributionError(
            f"{policy.source}: packages.{package.name}.binstall",
            "the binstall channel is disabled; refusing before any download (no channel claim is made)",
        )
    if channel != package.name:
        raise DistributionError(
            f"{policy.source}: packages.{package.name}.binstall",
            f"binstall installs by crate name {package.name!r}; the channel resolves to {channel!r}",
        )
    _check_binstall_metadata(package, True)
    if package.repository is None:
        raise DistributionError(f"{package.manifest_path}: package.repository", "is required for `{ repo }`")


def _package(workspace: Workspace, name: str) -> Package:
    package = workspace.packages.get(name)
    if package is None:
        raise DistributionError("--package", f"{name!r} is not a workspace package")
    return package


def _git_output(checkout: Path, args: Sequence[str], *, runner: Runner) -> str:
    try:
        result = runner(["git", *args], cwd=str(checkout), stdin=subprocess.DEVNULL, capture_output=True,
                        text=True, timeout=GIT_TIMEOUT, check=False)
    except (OSError, subprocess.TimeoutExpired) as error:
        raise DistributionError(f"git {' '.join(args)} ({checkout})", f"cannot run: {error}") from None
    if result.returncode != 0:
        raise DistributionError(f"git {' '.join(args)} ({checkout})", result.stderr.strip() or f"exit {result.returncode}")
    return result.stdout


def check_tag_checkout(checkout: Path, tag: str, *, runner: Runner) -> str:
    """HEAD is the tag's peeled commit and no tracked file differs from it."""
    tagged = _git_output(checkout, ["rev-parse", "--verify", f"{tag}^{{commit}}"], runner=runner).strip()
    head = _git_output(checkout, ["rev-parse", "--verify", "HEAD"], runner=runner).strip()
    if head != tagged:
        raise DistributionError(f"{checkout}: HEAD", f"is {head}, not {tag}'s commit {tagged}")
    dirty = _git_output(checkout, ["status", "--porcelain", "--untracked-files=no"], runner=runner).strip()
    if dirty:
        raise DistributionError(f"{checkout}", f"tracked files differ from {tag}:\n{dirty}")
    return head


# ---------------------------------------------------------------------------
# Running binstall and checking its result
# ---------------------------------------------------------------------------


def run_binstall(argv: list[str], *, out: Path, env: Mapping[str, str], runner: Runner, timeout: int,
                 report: dict[str, Any], log_prefix: str = "binstall") -> str:
    """Run binstall, retain its logs, and return the combined log text."""
    report["argv"] = argv
    try:
        result = runner(argv, env=dict(env), stdin=subprocess.DEVNULL, capture_output=True, text=True,
                        timeout=timeout, check=False)
        stdout, stderr, status = result.stdout, result.stderr, result.returncode
    except subprocess.TimeoutExpired as error:
        stdout, stderr, status = _text(error.stdout), _text(error.stderr), None
    out.mkdir(parents=True, exist_ok=True)
    (out / f"{log_prefix}.stdout.log").write_text(stdout, encoding="utf-8")
    (out / f"{log_prefix}.stderr.log").write_text(stderr, encoding="utf-8")
    report["exit"] = status
    report["logs"] = [f"{log_prefix}.stdout.log", f"{log_prefix}.stderr.log"]
    if status is None:
        raise VerificationFailed(f"binstall exceeded {timeout} s")
    if status != 0:
        raise VerificationFailed(f"binstall exited {status}; see {out / f'{log_prefix}.stderr.log'}")
    return stdout + "\n" + stderr


def _text(value: bytes | str | None) -> str:
    if value is None:
        return ""
    return value.decode("utf-8", "replace") if isinstance(value, bytes) else value


def installed_files(root: Path) -> list[str]:
    if not root.exists():
        return []
    return sorted(path.relative_to(root).as_posix() for path in root.rglob("*") if not path.is_dir())


def url_was_chosen(log: str, url: str) -> bool:
    needle = f"{DOWNLOAD_MARKER}{url}"
    return any(line.endswith(needle) or f"{needle} " in line for line in log.splitlines())


def verify_installation(
    *,
    root: Path,
    command: str,
    binaries: Sequence[Binary],
    version: str,
    url: str,
    log: str,
    out: Path,
    env: Mapping[str, str],
    runner: Runner,
    report: Report,
) -> None:
    """Check the installed tree, the command's version, and the chosen URL;
    every failure is collected into `report.errors`."""
    files = installed_files(root)
    report.installed = files
    expected = f"bin/{command}"
    optional = [binary.name for binary in binaries if binary.required_features and binary.name != command]
    report.tolerated_absent = [name for name in optional if f"bin/{name}" not in files]
    for extra in (name for name in files if name != expected):
        report.errors.append(f"unexpected installed file {extra} (only {expected} may be installed)")
    if expected not in files:
        report.errors.append(f"{expected} was not installed")
    else:
        _check_version(root / expected, version, out=out, env=env, runner=runner, report=report)
    if not url_was_chosen(log, url):
        report.errors.append(f"the binstall log does not show {url} as the downloaded archive ({DOWNLOAD_MARKER!r})")


def _check_version(executable: Path, version: str, *, out: Path, env: Mapping[str, str], runner: Runner,
                   report: Report) -> None:
    try:
        result = runner([str(executable), "--version"], env=dict(env), stdin=subprocess.DEVNULL,
                        capture_output=True, text=True, timeout=VERSION_TIMEOUT, check=False)
        stdout, stderr, status = result.stdout, result.stderr, result.returncode
    except subprocess.TimeoutExpired as error:
        stdout, stderr, status = _text(error.stdout), _text(error.stderr), None
    except OSError as error:
        report.errors.append(f"{executable} --version cannot run: {error}")
        return
    (out / "version.stdout.log").write_text(stdout, encoding="utf-8")
    (out / "version.stderr.log").write_text(stderr, encoding="utf-8")
    report.version_check = {"exit": status, "stdout": stdout.strip()}
    if status != 0:
        report.errors.append(f"{executable.name} --version exited {status}")
    elif version not in stdout.split() and f"v{version}" not in stdout.split():
        report.errors.append(f"{executable.name} --version printed {stdout.strip()!r}, expected version {version}")


# ---------------------------------------------------------------------------
# Routes
# ---------------------------------------------------------------------------


def _finish(report: Report, out: Path) -> int:
    report.status = "failed" if report.errors else "passed"
    path = report.write(out)
    for error in report.errors:
        print(f"error: {error}", file=sys.stderr)
    print(json.dumps({"status": report.status, "report": str(path)}))
    return 1 if report.errors else 0


def _refuse(report: Report, out: Path, error: DistributionError) -> int:
    report.status = "refused"
    report.errors = [str(error)]
    report.write(out)
    print(f"refused: {error}", file=sys.stderr)
    return 2


def _install_and_verify(report: Report, *, argv: list[str], package: Package, out: Path, env: Mapping[str, str],
                        runner: Runner, timeout: int, root: Path) -> int:
    try:
        log = run_binstall(argv, out=out, env=env, runner=runner, timeout=timeout, report=report.binstall)
    except VerificationFailed as error:
        report.errors.append(str(error))
        report.installed = installed_files(root)
        return _finish(report, out)
    verify_installation(root=root, command=ROSTER[package.name], binaries=package.binaries, version=package.version,
                        url=report.url or "", log=log, out=out, env=env, runner=runner, report=report)
    return _finish(report, out)


def verify_git(
    *,
    package: str,
    target: str,
    policy: Path,
    workspace: Workspace,
    out: Path,
    repo_url: str = CANONICAL_URL,
    binstall: str | None = None,
    runner: Runner = subprocess.run,
    base_env: Mapping[str, str] | None = None,
    timeout: int = BINSTALL_TIMEOUT,
) -> int:
    report = Report(route="git", package=package, target=target, source=repo_url, command=ROSTER.get(package))
    root = out / "install-root"
    try:
        check_target(target)
        selected = _package(workspace, package)
        check_channel(policy, workspace, selected)
        report.version = selected.version
        report.url = release_url(selected.repository or "", package, selected.version, target)
        env = isolated_env(os.environ if base_env is None else base_env, out / "isolated")
        binary = resolve_binstall(binstall)
        check_binstall_version(binary, runner=runner, env=env)
    except DistributionError as error:
        return _refuse(report, out, error)
    report.binstall = {"path": binary, "version": BINSTALL_VERSION}
    argv = binstall_command(binary, package=package, target=target, root=root, git=repo_url)
    return _install_and_verify(report, argv=argv, package=selected, out=out, env=env, runner=runner,
                               timeout=timeout, root=root)


def verify_manifest(
    *,
    tag: str,
    checkout: Path,
    target: str,
    workspace: Workspace,
    out: Path,
    binstall: str | None = None,
    runner: Runner = subprocess.run,
    base_env: Mapping[str, str] | None = None,
    timeout: int = BINSTALL_TIMEOUT,
) -> int:
    report = Report(route="manifest")
    root = out / "install-root"
    try:
        check_target(target)
        report.target = target
        recognized = recognize_tag(tag, workspace)
        if not isinstance(recognized, tuple):
            raise DistributionError(f"tag {tag}", recognized.reason)
        selected, tag_version = recognized
        report.package, report.command = selected.name, ROSTER[selected.name]
        manifest = checkout / selected.manifest_path
        report.source = str(manifest)
        if parse_version(selected.version, f"{selected.manifest_path}: package.version") != tag_version:
            raise DistributionError(
                f"{selected.manifest_path}: package.version",
                f"is {selected.version}, but tag {tag} names {tag_version}; refusing before invoking binstall",
            )
        report.version = selected.version
        check_tag_checkout(checkout, tag, runner=runner)
        check_channel(checkout / "docs" / "deployment.yaml", workspace, selected)
        report.url = release_url(selected.repository or "", selected.name, selected.version, target)
        env = isolated_env(os.environ if base_env is None else base_env, out / "isolated")
        binary = resolve_binstall(binstall)
        check_binstall_version(binary, runner=runner, env=env)
    except DistributionError as error:
        return _refuse(report, out, error)
    report.binstall = {"path": binary, "version": BINSTALL_VERSION}
    argv = binstall_command(binary, package=selected.name, target=target, root=root, manifest=manifest)
    return _install_and_verify(report, argv=argv, package=selected, out=out, env=env, runner=runner,
                               timeout=timeout, root=root)


# ---------------------------------------------------------------------------
# Fixture acceptance check (real binstall, local archive server)
# ---------------------------------------------------------------------------


def render_fixture_manifest(port: int, version: str) -> str:
    text = FIXTURE_MANIFEST.read_text(encoding="utf-8")
    for placeholder, value in (("@PORT@", str(port)), ("@VERSION@", version)):
        if text.count(placeholder) != 1:
            raise DistributionError(str(FIXTURE_MANIFEST), f"must contain {placeholder} exactly once")
        text = text.replace(placeholder, value)
    return text


def build_fixture_archive(serve: Path, *, version: str, target: str, members: Mapping[str, str]) -> Path:
    """A dist-shaped `<package>-<target>.tar.xz` under the package-tag
    release path, holding `<package>-<target>/<name>` executable scripts."""
    stem = f"{FIXTURE_PACKAGE}-{target}"
    release = serve / "releases" / "download" / f"{FIXTURE_PACKAGE}-v{version}"
    release.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix="binstall-fixture-stage-"))
    try:
        for name, body in members.items():
            file = staging / stem / name
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_text(body, encoding="utf-8")
            file.chmod(0o755)
        archive = release / f"{stem}.tar.xz"
        with tarfile.open(archive, "w:xz") as tar:
            tar.add(staging / stem, arcname=stem)
        return archive
    finally:
        shutil.rmtree(staging)


class _QuietHandler(http.server.SimpleHTTPRequestHandler):
    requests: list[str]

    def log_message(self, format: str, *args: Any) -> None:
        self.requests.append(format % args)


def serve_directory(directory: Path) -> tuple[http.server.ThreadingHTTPServer, threading.Thread, list[str]]:
    requests: list[str] = []
    handler = type("Handler", (_QuietHandler,), {"requests": requests})

    def factory(*args: Any, **kwargs: Any) -> http.server.SimpleHTTPRequestHandler:
        return handler(*args, directory=str(directory), **kwargs)

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), factory)
    thread = threading.Thread(target=server.serve_forever, name="binstall-fixture-server", daemon=True)
    thread.start()
    return server, thread, requests


def host_target() -> str:
    machine = {"arm64": "aarch64", "aarch64": "aarch64", "x86_64": "x86_64", "AMD64": "x86_64"}.get(platform.machine())
    system = {"Darwin": "apple-darwin", "Linux": "unknown-linux-gnu"}.get(platform.system())
    if machine is None or system is None:
        raise DistributionError("--target", f"no distribution target for {platform.system()} {platform.machine()}")
    return f"{machine}-{system}"


def fixture_check(
    *,
    binstall: str,
    out: Path,
    target: str,
    runner: Runner = subprocess.run,
    base_env: Mapping[str, str] | None = None,
    timeout: int = BINSTALL_TIMEOUT,
) -> int:
    """Two local releases: one whose archive holds only the unconditional
    command (the feature-gated helper is absent and must be tolerated), and
    one whose archive lacks that command (binstall must fail and install
    nothing)."""
    report = Report(route="fixture", package=FIXTURE_PACKAGE, version=FIXTURE_VERSION, target=target,
                    command=FIXTURE_COMMAND)
    try:
        check_target(target)
        env = isolated_env(os.environ if base_env is None else base_env, out / "isolated")
        check_binstall_version(binstall, runner=runner, env=env)
    except DistributionError as error:
        return _refuse(report, out, error)
    report.binstall = {"path": binstall, "version": BINSTALL_VERSION}
    script = f"#!/bin/sh\necho '{FIXTURE_COMMAND} {FIXTURE_VERSION}'\n"
    serve = out / "fixture" / "serve"
    build_fixture_archive(serve, version=FIXTURE_VERSION, target=target, members={FIXTURE_COMMAND: script})
    build_fixture_archive(serve, version=FIXTURE_BROKEN_VERSION, target=target,
                          members={"README.txt": "no binaries here\n"})
    server, thread, requests = serve_directory(serve)
    try:
        port = server.server_address[1]
        base = f"http://127.0.0.1:{port}"
        binaries = (Binary(FIXTURE_COMMAND, ()), Binary("fixture-helper", ("helper",)))

        manifest = _write_fixture_manifest(out / "fixture" / "crate", port, FIXTURE_VERSION)
        case_out, root = out / "optional-helper", out / "optional-helper" / "install-root"
        report.source = str(manifest)
        report.url = release_url(base, FIXTURE_PACKAGE, FIXTURE_VERSION, target)
        argv = binstall_command(binstall, package=FIXTURE_PACKAGE, target=target, root=root, manifest=manifest,
                                allow_insecure_http=True)
        try:
            log = run_binstall(argv, out=case_out, env=env, runner=runner, timeout=timeout, report=report.binstall)
        except VerificationFailed as error:
            report.errors.append(f"optional-helper: {error}")
            report.installed = installed_files(root)
        else:
            verify_installation(root=root, command=FIXTURE_COMMAND, binaries=binaries, version=FIXTURE_VERSION,
                                url=report.url, log=log, out=case_out, env=env, runner=runner, report=report)
        if "fixture-helper" not in report.tolerated_absent and not report.errors:
            report.errors.append("optional-helper: the feature-gated helper was not recorded as tolerated-absent")
        report.cases.append({"case": "optional-helper", "exit": report.binstall.get("exit"),
                             "installed": report.installed, "tolerated_absent": report.tolerated_absent})

        broken = _write_fixture_manifest(out / "fixture" / "crate-missing-command", port, FIXTURE_BROKEN_VERSION)
        case_out, root = out / "missing-command", out / "missing-command" / "install-root"
        argv = binstall_command(binstall, package=FIXTURE_PACKAGE, target=target, root=root, manifest=broken,
                                allow_insecure_http=True)
        case: dict[str, Any] = {"case": "missing-command"}
        try:
            run_binstall(argv, out=case_out, env=env, runner=runner, timeout=timeout, report=case,
                         log_prefix="binstall")
            report.errors.append("missing-command: binstall succeeded although the archive lacks the command")
        except VerificationFailed as error:
            case["failure"] = str(error)
        case["installed"] = installed_files(root)
        if case["installed"]:
            report.errors.append(f"missing-command: binstall left files behind: {case['installed']}")
        report.cases.append(case)
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=10)
        (out / "server-requests.log").write_text("\n".join(requests) + "\n", encoding="utf-8")
    return _finish(report, out)


def _write_fixture_manifest(directory: Path, port: int, version: str) -> Path:
    directory.mkdir(parents=True, exist_ok=True)
    manifest = directory / "Cargo.toml"
    manifest.write_text(render_fixture_manifest(port, version), encoding="utf-8")
    return manifest


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def workspace_from(root: Path, metadata: Path | None) -> Workspace:
    """`root`'s workspace from Cargo, or from a saved `cargo metadata
    --no-deps --format-version 1` document."""
    if metadata is None:
        return cargo_workspace(root)

    def read(path: str) -> str | None:
        file = root / path
        if not file.is_file():
            return None
        try:
            return file.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError) as error:
            raise DistributionError(file.as_posix(), f"cannot read: {error}") from None

    manifest = read("Cargo.toml")
    if manifest is None:
        raise DistributionError(str(root / "Cargo.toml"), "the workspace manifest is missing")
    excluded = excluded_package_names(manifest, read, source=str(root / "Cargo.toml"))
    try:
        data = metadata.read_bytes()
    except OSError as error:
        raise DistributionError(metadata.as_posix(), f"cannot read: {error}") from None
    return load_workspace(data, source=metadata.as_posix(), excluded=excluded)


def _parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    commands = parser.add_subparsers(dest="command", required=True)

    def common(sub: argparse.ArgumentParser, *, binstall_required: bool = False) -> None:
        sub.add_argument("--binstall", required=binstall_required,
                         help=f"the cargo-binstall {BINSTALL_VERSION} binary (default: cargo-binstall on PATH)")
        sub.add_argument("--out", type=Path, required=True, help="report and log directory")
        sub.add_argument("--timeout", type=int, default=BINSTALL_TIMEOUT, help="binstall timeout in seconds")

    git = commands.add_parser("git", help="install the current manifest version through --git")
    git.add_argument("--package", required=True)
    git.add_argument("--target", required=True)
    git.add_argument("--root", type=Path, required=True, help="checkout holding docs/deployment.yaml")
    git.add_argument("--metadata", type=Path, help="saved cargo metadata for --root")
    git.add_argument("--repo-url", default=CANONICAL_URL)
    common(git)

    manifest = commands.add_parser("manifest", help="install a tagged version from an exact-tag checkout")
    manifest.add_argument("--tag", required=True)
    manifest.add_argument("--checkout", type=Path, required=True)
    manifest.add_argument("--target", required=True)
    common(manifest)

    fixture = commands.add_parser("fixture-check", help="real binstall against a local fixture release")
    fixture.add_argument("--target", help="default: this host's distribution target")
    common(fixture, binstall_required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None, *, runner: Runner = subprocess.run,
         base_env: Mapping[str, str] | None = None) -> int:
    args = _parse_args(sys.argv[1:] if argv is None else argv)
    out = args.out.resolve()
    try:
        if args.command == "git":
            root = args.root.resolve()
            return verify_git(package=args.package, target=args.target, policy=root / "docs" / "deployment.yaml",
                              workspace=workspace_from(root, args.metadata), out=out, repo_url=args.repo_url,
                              binstall=args.binstall, runner=runner, base_env=base_env, timeout=args.timeout)
        if args.command == "manifest":
            checkout = args.checkout.resolve()
            return verify_manifest(tag=args.tag, checkout=checkout, target=args.target,
                                   workspace=cargo_workspace(checkout), out=out, binstall=args.binstall,
                                   runner=runner, base_env=base_env, timeout=args.timeout)
        return fixture_check(binstall=args.binstall, out=out, target=args.target or host_target(),
                             runner=runner, base_env=base_env, timeout=args.timeout)
    except DistributionError as error:
        return _refuse(Report(route=args.command), out, error)


if __name__ == "__main__":
    sys.exit(main())

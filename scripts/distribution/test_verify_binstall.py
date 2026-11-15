#!/usr/bin/env python3
"""Tests for `verify_binstall.py`: the pinned invocation, the refusals that
happen before any network access, and the post-install checks. A fake runner
stands in for binstall, Git, and the installed command; nothing here touches
the network or runs a real binstall (`fixture-check` does that, explicitly)."""

from __future__ import annotations

import dataclasses
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
from pathlib import Path

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.append(_HERE)
import verify_binstall as vb  # noqa: E402
from distribution import (  # noqa: E402
    BINSTALL_ENABLED_STRATEGIES,
    BINSTALL_URLS,
    BINSTALL_VERSION,
    Binary,
    DistributionError,
    Workspace,
    cargo_workspace,
    load_toml,
)

REPOSITORY = Path(_HERE).parents[1]
POLICY = REPOSITORY / "docs" / "deployment.yaml"
PACKAGE = "biscuit-hash-cli"
TARGET = "aarch64-apple-darwin"
BINSTALL = "/pinned/cargo-binstall"
TAG_COMMIT = "a" * 40
EXPECTED_FLAGS = [
    "--no-confirm",
    "--disable-telemetry",
    "--no-discover-github-token",
    "--strategies", "crate-meta-data",
    "--targets", TARGET,
]


class FakeRunner:
    """Records every invocation and answers binstall `-V`, a binstall install
    (writing the files and log it is given), Git queries, and the installed
    command's `--version`."""

    def __init__(self, *, binstall_version: str = BINSTALL_VERSION, head: str = TAG_COMMIT,
                 installs: dict[str, str] | None = None, log: str = "", command_output: str = "") -> None:
        self.calls: list[list[str]] = []
        self.binstall_version = binstall_version
        self.head = head
        self.installs = installs or {}
        self.log = log
        self.command_output = command_output

    def __call__(self, argv, **kwargs) -> subprocess.CompletedProcess:
        argv = [str(arg) for arg in argv]
        self.calls.append(argv)
        if argv[0] == "git":
            return self._git(argv)
        if argv[0] == BINSTALL and argv[1:] == ["-V"]:
            return subprocess.CompletedProcess(argv, 0, self.binstall_version + "\n", "")
        if argv[0] == BINSTALL:
            install_path = Path(argv[argv.index("--install-path") + 1])
            for name, body in self.installs.items():
                file = install_path / name
                file.parent.mkdir(parents=True, exist_ok=True)
                file.write_text(body, encoding="utf-8")
            return subprocess.CompletedProcess(argv, 0, self.log, "")
        if argv[1:] == ["--version"]:
            return subprocess.CompletedProcess(argv, 0, self.command_output, "")
        raise AssertionError(f"unexpected invocation {argv}")

    def _git(self, argv: list[str]) -> subprocess.CompletedProcess:
        if argv[1:3] == ["rev-parse", "--verify"]:
            sha = TAG_COMMIT if argv[3].endswith("^{commit}") else self.head
            return subprocess.CompletedProcess(argv, 0, sha + "\n", "")
        if argv[1] == "status":
            return subprocess.CompletedProcess(argv, 0, "", "")
        raise AssertionError(f"unexpected git invocation {argv}")

    def installed(self) -> bool:
        return any(call[0] == BINSTALL and call[1:] != ["-V"] for call in self.calls)


def download_line(url: str) -> str:
    return f"cargo-binstall: DEBUG resolve: Downloading package url={url} dst=/tmp/x fmt=Txz"


def enable_metadata(workspace: Workspace, package: str) -> Workspace:
    """`workspace` with `package`'s binstall metadata in the enabled form."""
    selected = workspace.packages[package]
    metadata = dict(selected.metadata or {})
    metadata["binstall"] = {**metadata["binstall"], "disabled-strategies": list(BINSTALL_ENABLED_STRATEGIES)}
    packages = {**workspace.packages, package: dataclasses.replace(selected, metadata=metadata)}
    return dataclasses.replace(workspace, packages=packages)


def enabled_policy(directory: Path, package: str) -> Path:
    """A copy of the real policy with `package` approved and binstall on."""
    text = POLICY.read_text(encoding="utf-8")
    entry = re.compile(rf"(\n    {re.escape(package)}:\n        \{{\n)(.*?)(\n        \}})", re.S)
    match = entry.search(text)
    assert match is not None, f"{package} entry not found in {POLICY}"
    body = match.group(2).replace("approved: false", "approved: true").replace("binstall: false", "binstall: true")
    path = directory / "docs" / "deployment.yaml"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text[:match.start(2)] + body + text[match.end(2):], encoding="utf-8")
    return path


_WORKSPACE: list[Workspace] = []
# The `cargo metadata --no-deps` document `_WORKSPACE` was loaded from.
_METADATA: list[str] = []


def _capture(*args, **kwargs) -> subprocess.CompletedProcess:
    result = subprocess.run(*args, **kwargs)
    _METADATA.append(result.stdout)
    return result


class WorkspaceCase(unittest.TestCase):
    workspace: Workspace

    @classmethod
    def setUpClass(cls) -> None:
        # `cargo metadata` takes about a second; every class shares one call.
        if not _WORKSPACE:
            _WORKSPACE.append(cargo_workspace(REPOSITORY, run=_capture))
        cls.workspace = _WORKSPACE[0]
        cls.version = cls.workspace.packages[PACKAGE].version

    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="verify-binstall-test-"))
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)
        self.out = self.tmp / "out"

    def url(self, package: str = PACKAGE, version: str | None = None) -> str:
        return vb.release_url("https://github.com/yankeeinlondon/rusty-biscuit", package, version or self.version, TARGET)


class BinstallCommandTest(unittest.TestCase):
    def test_git_route_argv(self) -> None:
        root = Path("/iso/root")
        argv = vb.binstall_command(BINSTALL, package=PACKAGE, target=TARGET, root=root, git=vb.CANONICAL_URL)
        self.assertEqual(argv, [
            BINSTALL, "--git", "https://github.com/yankeeinlondon/rusty-biscuit", *EXPECTED_FLAGS,
            "--root", "/iso/root", "--install-path", "/iso/root/bin", "--no-track", "--log-level", "debug", PACKAGE,
        ])

    def test_manifest_route_argv(self) -> None:
        root, manifest = Path("/iso/root"), Path("/tag/biscuit-hash/cli/Cargo.toml")
        argv = vb.binstall_command(BINSTALL, package=PACKAGE, target=TARGET, root=root, manifest=manifest)
        self.assertEqual(argv, [
            BINSTALL, "--manifest-path", "/tag/biscuit-hash/cli/Cargo.toml", *EXPECTED_FLAGS,
            "--root", "/iso/root", "--install-path", "/iso/root/bin", "--no-track", "--log-level", "debug", PACKAGE,
        ])

    def test_no_version_selection_tag_or_compile(self) -> None:
        for route in ({"git": vb.CANONICAL_URL}, {"manifest": Path("/m/Cargo.toml")}):
            argv = vb.binstall_command(BINSTALL, package=PACKAGE, target=TARGET, root=Path("/r"), **route)
            self.assertNotIn("--version", argv)
            self.assertNotIn("--tag", argv)
            self.assertNotIn("--allow-insecure-http", argv)
            self.assertEqual(argv[argv.index("--strategies") + 1], "crate-meta-data")
            self.assertFalse(any("compile" in arg for arg in argv))

    def test_exactly_one_route(self) -> None:
        with self.assertRaises(ValueError):
            vb.binstall_command(BINSTALL, package=PACKAGE, target=TARGET, root=Path("/r"))
        with self.assertRaises(ValueError):
            vb.binstall_command(BINSTALL, package=PACKAGE, target=TARGET, root=Path("/r"),
                                git=vb.CANONICAL_URL, manifest=Path("/m"))

    def test_isolated_environment(self) -> None:
        with tempfile.TemporaryDirectory() as work:
            env = vb.isolated_env({"PATH": "/bin", "BINSTALL_STRATEGIES": "compile", "CARGO_INSTALL_ROOT": "/x",
                                   "CARGO_HOME": "/home/u/.cargo", "GITHUB_TOKEN": "t"}, Path(work))
            self.assertEqual(env["PATH"], "/bin")
            self.assertNotIn("BINSTALL_STRATEGIES", env)
            self.assertNotIn("CARGO_INSTALL_ROOT", env)
            self.assertNotIn("GITHUB_TOKEN", env)
            self.assertEqual(env["CARGO_HOME"], str(Path(work) / "cargo-home"))
            self.assertEqual(env["GIT_TERMINAL_PROMPT"], "0")


class GitRouteTest(WorkspaceCase):
    def test_disabled_channel_in_real_policy_refuses_without_running_anything(self) -> None:
        runner = FakeRunner()
        status = vb.verify_git(package=PACKAGE, target=TARGET, policy=POLICY, workspace=self.workspace,
                               out=self.out, binstall=BINSTALL, runner=runner, base_env={})
        self.assertEqual(status, 2)
        self.assertEqual(runner.calls, [])
        report = vb.json.loads((self.out / vb.REPORT_NAME).read_text())
        self.assertEqual(report["status"], "refused")
        self.assertIn("binstall channel is disabled", report["errors"][0])

    def test_enabled_policy_with_disabled_metadata_refuses(self) -> None:
        runner = FakeRunner()
        status = vb.verify_git(package=PACKAGE, target=TARGET, policy=enabled_policy(self.tmp, PACKAGE),
                               workspace=self.workspace, out=self.out, binstall=BINSTALL, runner=runner, base_env={})
        self.assertEqual(status, 2)
        self.assertEqual(runner.calls, [])

    def test_enabled_fixture_passes_precheck_and_verifies(self) -> None:
        workspace = enable_metadata(self.workspace, PACKAGE)
        vb.check_channel(enabled_policy(self.tmp, PACKAGE), workspace, workspace.packages[PACKAGE])
        runner = FakeRunner(installs={"bh": "x"}, log=download_line(self.url()), command_output=f"bh {self.version}\n")
        status = vb.verify_git(package=PACKAGE, target=TARGET, policy=enabled_policy(self.tmp, PACKAGE),
                               workspace=workspace, out=self.out, binstall=BINSTALL, runner=runner, base_env={})
        self.assertEqual(status, 0, (self.out / vb.REPORT_NAME).read_text())
        self.assertIn("--git", runner.calls[1])
        self.assertTrue((self.out / "binstall.stdout.log").is_file())

    def test_wrong_binstall_version_refuses_before_install(self) -> None:
        workspace = enable_metadata(self.workspace, PACKAGE)
        runner = FakeRunner(binstall_version="1.25.3")
        status = vb.verify_git(package=PACKAGE, target=TARGET, policy=enabled_policy(self.tmp, PACKAGE),
                               workspace=workspace, out=self.out, binstall=BINSTALL, runner=runner, base_env={})
        self.assertEqual(status, 2)
        self.assertEqual(runner.calls, [[BINSTALL, "-V"]])

    def test_unknown_target_refuses(self) -> None:
        runner = FakeRunner()
        status = vb.verify_git(package=PACKAGE, target="x86_64-pc-windows-msvc", policy=POLICY,
                               workspace=self.workspace, out=self.out, binstall=BINSTALL, runner=runner, base_env={})
        self.assertEqual(status, 2)
        self.assertEqual(runner.calls, [])


class ManifestRouteTest(WorkspaceCase):
    def verify(self, tag: str, runner: FakeRunner, workspace: Workspace | None = None,
               checkout: Path = REPOSITORY) -> int:
        return vb.verify_manifest(tag=tag, checkout=checkout, target=TARGET, workspace=workspace or self.workspace,
                                  out=self.out, binstall=BINSTALL, runner=runner, base_env={})

    def test_version_mismatch_refuses_before_anything_runs(self) -> None:
        runner = FakeRunner()
        self.assertEqual(self.verify(f"{PACKAGE}-v99.0.0", runner), 2)
        self.assertEqual(runner.calls, [])
        self.assertIn("refusing before invoking binstall", (self.out / vb.REPORT_NAME).read_text())

    def test_head_not_tag_commit_refuses(self) -> None:
        runner = FakeRunner(head="b" * 40)
        self.assertEqual(self.verify(f"{PACKAGE}-v{self.version}", runner), 2)
        self.assertFalse(runner.installed())
        self.assertTrue(all(call[0] == "git" for call in runner.calls))

    def test_disabled_channel_in_checkout_policy_refuses(self) -> None:
        runner = FakeRunner()
        self.assertEqual(self.verify(f"{PACKAGE}-v{self.version}", runner), 2)
        self.assertFalse(runner.installed())
        self.assertIn("binstall channel is disabled", (self.out / vb.REPORT_NAME).read_text())

    def test_library_tag_refuses(self) -> None:
        runner = FakeRunner()
        self.assertEqual(self.verify("biscuit-hash-v0.1.0", runner), 2)
        self.assertEqual(runner.calls, [])

    def test_enabled_checkout_installs_from_resolved_manifest(self) -> None:
        enabled_policy(self.tmp, PACKAGE)
        workspace = enable_metadata(self.workspace, PACKAGE)
        runner = FakeRunner(installs={"bh": "x"}, log=download_line(self.url()), command_output=f"bh {self.version}\n")
        self.assertEqual(self.verify(f"{PACKAGE}-v{self.version}", runner, workspace, checkout=self.tmp), 0)
        install = next(call for call in runner.calls if call[0] == BINSTALL and call[1] != "-V")
        manifest = self.tmp / workspace.packages[PACKAGE].manifest_path
        self.assertEqual(install[1:3], ["--manifest-path", str(manifest)])


class PostRunTest(WorkspaceCase):
    def check(self, package: str, files: dict[str, str], *, output: str | None = None, log: str | None = None) -> vb.Report:
        selected = self.workspace.packages[package]
        command = vb.ROSTER[package]
        root = self.tmp / "root"
        for name, body in files.items():
            (root / name).parent.mkdir(parents=True, exist_ok=True)
            (root / name).write_text(body, encoding="utf-8")
        url = self.url(package, selected.version)
        runner = FakeRunner(command_output=f"{command} {selected.version}\n" if output is None else output)
        report = vb.Report(route="test")
        self.out.mkdir(parents=True, exist_ok=True)
        vb.verify_installation(root=root, command=command, binaries=selected.binaries, version=selected.version,
                               url=url, log=download_line(url) if log is None else log, out=self.out, env={},
                               runner=runner, report=report)
        return report

    def test_control_passes(self) -> None:
        report = self.check(PACKAGE, {"bin/bh": "x"})
        self.assertEqual(report.errors, [])
        self.assertEqual(report.installed, ["bin/bh"])

    def test_extra_executable_fails(self) -> None:
        report = self.check(PACKAGE, {"bin/bh": "x", "bin/other": "x"})
        self.assertEqual(len(report.errors), 1)
        self.assertIn("bin/other", report.errors[0])

    def test_missing_command_fails(self) -> None:
        report = self.check(PACKAGE, {})
        self.assertIn("bin/bh was not installed", report.errors)

    def test_worktree_feature_gated_helper_absence_is_tolerated(self) -> None:
        helper = [binary for binary in self.workspace.packages["git-wt-cli"].binaries if binary.name == "render_dirty_tree"]
        self.assertEqual(helper, [Binary("render_dirty_tree", ("terminal-tests",))])
        report = self.check("git-wt-cli", {"bin/wt": "x"})
        self.assertEqual(report.errors, [])
        self.assertIn("render_dirty_tree", report.tolerated_absent)

    def test_version_output_mismatch_fails(self) -> None:
        report = self.check(PACKAGE, {"bin/bh": "x"}, output="bh 0.0.0-other\n")
        self.assertEqual(len(report.errors), 1)
        self.assertIn("--version printed", report.errors[0])

    def test_log_without_release_url_fails(self) -> None:
        url = self.url()
        for log in ("", f"cargo-binstall: DEBUG Checking for package at: '{url}'", download_line(url + ".sig")):
            with self.subTest(log=log):
                report = self.check(PACKAGE, {"bin/bh": "x"}, log=log)
                self.assertEqual(len(report.errors), 1)
                self.assertIn("downloaded archive", report.errors[0])


class CommandLineTest(WorkspaceCase):
    """`main` wiring for the saved-metadata loader: a real-shaped positive
    run, and located malformed input that launches nothing."""

    def enabled_metadata(self) -> str:
        data = json.loads(_METADATA[0])
        (entry,) = [p for p in data["packages"] if p["name"] == PACKAGE and p["id"] in data["workspace_members"]]
        entry["metadata"]["binstall"]["disabled-strategies"] = list(BINSTALL_ENABLED_STRATEGIES)
        return json.dumps(data)

    def run_main(self, metadata: str, runner: FakeRunner, *, manifest: bool = True) -> tuple[int, dict]:
        root = self.tmp / "root"
        root.mkdir(exist_ok=True)
        if manifest:
            # The real root manifest and the excluded packages it names, which
            # `workspace_from` resolves to keep their policy entries known.
            shutil.copyfile(REPOSITORY / "Cargo.toml", root / "Cargo.toml")
            for directory in load_toml((REPOSITORY / "Cargo.toml").read_text(encoding="utf-8"), source="Cargo.toml")["workspace"]["exclude"]:
                (root / directory).mkdir(parents=True, exist_ok=True)
                shutil.copyfile(REPOSITORY / directory / "Cargo.toml", root / directory / "Cargo.toml")
        enabled_policy(root, PACKAGE)
        saved = self.tmp / "metadata.json"
        saved.write_text(metadata, encoding="utf-8")
        argv = ["git", "--package", PACKAGE, "--target", TARGET, "--root", str(root), "--metadata", str(saved),
                "--binstall", BINSTALL, "--out", str(self.out)]
        with redirect_stdout(StringIO()), redirect_stderr(StringIO()):
            status = vb.main(argv, runner=runner, base_env={})
        return status, json.loads((self.out / vb.REPORT_NAME).read_text(encoding="utf-8"))

    def test_saved_metadata_control_verifies(self) -> None:
        runner = FakeRunner(installs={"bh": "x"}, log=download_line(self.url()), command_output=f"bh {self.version}\n")
        status, report = self.run_main(self.enabled_metadata(), runner)
        self.assertEqual((status, report["status"]), (0, "passed"), report)
        self.assertTrue(runner.installed())

    def test_malformed_metadata_is_located_and_launches_nothing(self) -> None:
        runner = FakeRunner()
        status, report = self.run_main(self.enabled_metadata() + "\n{}", runner)
        self.assertEqual((status, report["status"]), (2, "refused"))
        self.assertRegex(report["errors"][0], rf"^{re.escape((self.tmp / 'metadata.json').as_posix())}: invalid JSON")
        self.assertEqual(runner.calls, [])

    def test_missing_workspace_manifest_is_refused(self) -> None:
        """An absent root manifest once read as an empty one, dropping its
        `[workspace] exclude` list."""
        runner = FakeRunner()
        status, report = self.run_main(self.enabled_metadata(), runner, manifest=False)
        self.assertEqual(status, 2)
        self.assertRegex(report["errors"][0], r"root/Cargo.toml: the workspace manifest is missing")
        self.assertEqual(runner.calls, [])


class FixtureManifestTest(unittest.TestCase):
    def test_rendered_fixture_matches_the_enabled_metadata(self) -> None:
        manifest = load_toml(vb.render_fixture_manifest(43210, vb.FIXTURE_VERSION), source="fixture")
        binstall = manifest["package"]["metadata"]["binstall"]
        self.assertEqual(binstall["pkg-url"], BINSTALL_URLS["pkg-url"].replace("{ repo }", "http://127.0.0.1:43210"))
        self.assertEqual(binstall["pkg-fmt"], BINSTALL_URLS["pkg-fmt"])
        self.assertEqual(binstall["bin-dir"], BINSTALL_URLS["bin-dir"])
        self.assertEqual(binstall["disabled-strategies"], BINSTALL_ENABLED_STRATEGIES)
        bins = {entry["name"]: entry.get("required-features") for entry in manifest["bin"]}
        self.assertEqual(bins, {vb.FIXTURE_COMMAND: None, "fixture-helper": ["helper"]})
        self.assertEqual(manifest["package"]["version"], vb.FIXTURE_VERSION)

    def test_render_rejects_a_missing_placeholder(self) -> None:
        original = vb.FIXTURE_MANIFEST
        with tempfile.TemporaryDirectory() as directory:
            broken = Path(directory) / "Cargo.toml.in"
            broken.write_text('version = "@VERSION@"\n', encoding="utf-8")
            vb.FIXTURE_MANIFEST = broken
            try:
                with self.assertRaises(DistributionError):
                    vb.render_fixture_manifest(1, "0.1.0")
            finally:
                vb.FIXTURE_MANIFEST = original


if __name__ == "__main__":
    unittest.main()

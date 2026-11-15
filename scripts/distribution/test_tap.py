#!/usr/bin/env python3
"""Tests for `tap.py`: validation and anchored transformation of dist's real
0.34.0 formula, and the publisher's ordering and refusals through a fake
`TapClient`, a fake downloader, and a fake signature verifier. Nothing here
touches the network or a real tap."""

from __future__ import annotations

import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from typing import Any, Iterator
from unittest import mock

_HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.append(_HERE)
import tap  # noqa: E402
from distribution import TARGETS, DistributionError, PendingSetup, RemoteAsset, Selection  # noqa: E402
from tool_guard import require_tools  # noqa: E402

ENFORCED_BY = 'the repo-deps "distribution" companion suite (just distribution-test)'

FIXTURE = Path(_HERE) / "fixtures" / "formula" / "darkmatter-cli.rb"
COMMIT = "a" * 40
HEAD = "b" * 40
NEW = "c" * 40
# The fixture's own digest for the one target built locally; dist writes the
# others identically, so the four-target variant uses stand-in digests.
REAL_SHA = "e09ccfa1a932293ba60d43fcceb2f3e5911fd5aa84869afd06dbc02e625a5ee8"
DIGESTS = {
    "aarch64-apple-darwin": REAL_SHA,
    "x86_64-apple-darwin": "1" * 64,
    "aarch64-unknown-linux-gnu": "2" * 64,
    "x86_64-unknown-linux-gnu": "3" * 64,
}
PACKAGES = {  # package -> (command, brew formula name, formula class)
    "darkmatter-cli": ("md", "darkmatter-cli", "DarkmatterCli"),
    "biscuit-hash-cli": ("bh", "bh", "Bh"),
    "claudine-cli": ("claudine", "claudine", "Claudine"),
}
ENV = {"HOMEBREW_TAP_TOKEN": "token", "HOMEBREW_TAP_AUTHOR": "ken"}


def dist_formula(package: str = "darkmatter-cli", version: str = "0.1.0") -> str:
    """The real fixture with the three missing `sha256` lines inserted in
    dist's format, renamed for `package` and `version`."""
    text = FIXTURE.read_text(encoding="utf-8")
    for target, digest in DIGESTS.items():
        url = f"darkmatter-cli-{target}.tar.xz\"\n"
        if target != "aarch64-apple-darwin":
            text = text.replace(url, f'{url}      sha256 "{digest}"\n')
    command, _, cls = PACKAGES[package]
    text = text.replace("darkmatter-cli-v0.1.0/darkmatter-cli-", f"{package}-v{version}/{package}-")
    text = text.replace('version "0.1.0"', f'version "{version}"')
    text = text.replace("class DarkmatterCli <", f"class {cls} <")
    return text.replace('bin.install "md"', f'bin.install "{command}"')


def record(package: str = "darkmatter-cli", version: str = "0.1.0", **overrides: Any) -> dict[str, Any]:
    command, brew, _ = PACKAGES[package]
    base = {
        "status": "selected", "package": package, "command": command, "manifest_path": f"{package}/Cargo.toml",
        "version": version, "tag": f"{package}-v{version}", "commit": COMMIT, "targets": list(TARGETS),
        "channels": {"brew": brew}, "formula_update": True, "prerelease": False, "mode": "release", "publishable": True,
    }
    base.update(overrides)
    return base


def selection(*args: Any, **overrides: Any) -> Selection:
    return Selection.from_json(record(*args, **overrides), source="selection.json")


def write_assets(root: Path, package: str = "darkmatter-cli", version: str = "0.1.0", formula: str | None = None) -> Path:
    sums = []
    for target, digest in DIGESTS.items():
        name = f"{package}-{target}.tar.xz"
        (root / name).write_bytes(f"archive {name}\n".encode() * 50)
        sums.append(f"{digest} *{name}\n")
    (root / "sha256.sum").write_text("".join(sums) + "\n", encoding="utf-8")
    (root / f"{PACKAGES[package][1]}.rb").write_text(formula or dist_formula(package, version), encoding="utf-8")
    return root


def checksums(package: str = "darkmatter-cli") -> dict[str, str]:
    return {f"{package}-{target}.tar.xz": digest for target, digest in DIGESTS.items()}


class FakeClient(tap.TapClient):
    def __init__(self, *, viewer: str = "ken", installed: bytes | None = None, conflict: bool = False,
                 signature_valid: bool = True, author: str | None = "ken") -> None:
        self.calls: list[tuple[Any, ...]] = []
        self.viewer, self.installed, self.conflict = viewer, installed, conflict
        self.signature_valid, self.author = signature_valid, author

    def viewer_login(self) -> str:
        self.calls.append(("viewer_login",))
        return self.viewer

    def branch_head(self, branch: str) -> str:
        self.calls.append(("branch_head", branch))
        return HEAD

    def read_file(self, oid: str, path: str) -> bytes | None:
        self.calls.append(("read_file", oid, path))
        return self.installed

    def create_commit(self, branch: str, expected_head_oid: str, path: str, contents: bytes, headline: str) -> tap.CommitResult:
        self.calls.append(("create_commit", branch, expected_head_oid, path, contents, headline))
        if self.conflict:
            raise tap.TapConflict("tap@main", "Expected branch to point to X; rerun the same tag")
        return tap.CommitResult(NEW, self.signature_valid, "web-flow", self.author)

    def fetch_commit_for_verification(self, oid: str, dest_dir: Path) -> None:
        self.calls.append(("fetch_commit_for_verification", oid))

    def names(self) -> list[str]:
        return [call[0] for call in self.calls]


class Harness:
    """One publish run with recorded effects."""

    def __init__(self, assets: Path, client: FakeClient | None = None, mangle: Any = None) -> None:
        self.assets = assets
        self.client = client or FakeClient()
        self.factory_tokens: list[str] = []
        self.fetched: list[str] = []
        self.verified: list[str] = []
        self.mangle = mangle

    def factory(self, token: str) -> FakeClient:
        self.factory_tokens.append(token)
        return self.client

    def fetch(self, asset: RemoteAsset) -> Iterator[bytes]:
        self.fetched.append(asset.url)
        data = (self.assets / asset.name).read_bytes()
        if self.mangle:
            data = self.mangle(asset.name, data)
        yield from (data[i:i + 100] for i in range(0, len(data), 100))

    def verify(self, repo: Path, oid: str) -> str:
        self.verified.append(oid)
        return tap.WEB_FLOW_SIGNING_KEY

    def run(self, sel: Selection, env: dict[str, str] = ENV, dry_run: bool = False) -> dict[str, Any]:
        return tap.publish(sel, self.assets, env=env, client_factory=self.factory, fetch=self.fetch,
                           verify=self.verify, dry_run=dry_run)


class TempCase(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)

    def tearDown(self) -> None:
        self._tmp.cleanup()


class FormulaTest(unittest.TestCase):
    def test_unedited_fixture_is_the_real_single_target_shape(self) -> None:
        with self.assertRaisesRegex(DistributionError, r"darkmatter-cli.rb:12: the url for x86_64-apple-darwin is not followed by a sha256"):
            tap.prepare_formula(FIXTURE.read_text(encoding="utf-8"), selection(), checksums(), source="darkmatter-cli.rb")

    def test_class_name_follows_dist(self) -> None:
        self.assertEqual(tap.class_name("darkmatter-cli"), "DarkmatterCli")
        self.assertEqual(tap.class_name("git_wt-cli"), "GitWtCli")
        self.assertEqual(tap.class_name("bh"), "Bh")

    def test_transformation_only_inserts_declarations(self) -> None:
        original = dist_formula()
        prepared = tap.prepare_formula(original, selection(), checksums())
        added = [line for line in prepared.splitlines() if line not in original.splitlines()]
        self.assertEqual(added, [
            "  depends_on macos: :sequoia",
            '    if OS.linux? && OS::Linux::Glibc.system_version < Version.new("2.35")',
            '      odie "md requires glibc 2.35 or newer; found #{OS::Linux::Glibc.system_version}"',
        ])
        # The guard's closing `end` duplicates an existing line, so remove it by position.
        restored = prepared.replace(added[1] + "\n" + added[2] + "\n    end\n", "").replace(added[0] + "\n", "")
        self.assertEqual(restored, original)
        self.assertIn('  license "AGPL-3.0-only"\n  depends_on macos: :sequoia\n', prepared)
        self.assertIn("  def install\n    if OS.linux? && OS::Linux::Glibc", prepared)
        self.assertLess(prepared.index("odie "), prepared.index('bin.install "md"'))

    def test_claudine_gets_alsa_and_bh_does_not(self) -> None:
        claudine = tap.prepare_formula(dist_formula("claudine-cli"), selection("claudine-cli"), checksums("claudine-cli"))
        self.assertIn('  license "AGPL-3.0-only"\n  depends_on "alsa-lib" if OS.linux?\n  depends_on macos: :sequoia\n', claudine)
        bh = tap.prepare_formula(dist_formula("biscuit-hash-cli"), selection("biscuit-hash-cli"), checksums("biscuit-hash-cli"))
        self.assertNotIn("alsa-lib", bh)
        self.assertIn('odie "bh requires glibc 2.35', bh)

    def drift(self, text: str, pattern: str) -> None:
        with self.assertRaisesRegex(DistributionError, pattern):
            tap.prepare_formula(text, selection(), checksums(), source="f.rb")

    def test_missing_anchor(self) -> None:
        self.drift(dist_formula().replace("  def install\n", "  def install_it\n"),
                   r"formula template drift: anchor 'def install' .* occurs 0 times")

    def test_duplicate_anchor(self) -> None:
        text = dist_formula().replace("  def target_triple\n", "  def install\n  end\n\n  def target_triple\n")
        self.drift(text, r"formula template drift: anchor 'def install' .* occurs 2 times")

    def test_duplicate_license(self) -> None:
        self.drift(dist_formula().replace('  license "AGPL-3.0-only"\n', '  license "AGPL-3.0-only"\n  license "AGPL-3.0-only"\n'),
                   r"license \"AGPL-3.0-only\" exactly once")

    def test_wrong_license(self) -> None:
        self.drift(dist_formula().replace('license "AGPL-3.0-only"', 'license "MIT"'), r"exactly once, found \['license \"MIT\"'\]")

    def test_wrong_url(self) -> None:
        self.drift(dist_formula().replace("darkmatter-cli-v0.1.0/darkmatter-cli-x86_64-apple", "darkmatter-cli-v0.0.9/darkmatter-cli-x86_64-apple"),
                   r"f.rb:11: url .*v0.0.9.* is not one of this release's archives")

    def test_url_under_wrong_platform_block(self) -> None:
        text = dist_formula().replace("    if Hardware::CPU.intel?\n", "    if Hardware::CPU.arm?\n", 1)
        self.drift(text, r"not under dist's OS.mac\?/OS.linux\?")

    def test_wrong_sha(self) -> None:
        self.drift(dist_formula().replace("1" * 64, "4" * 64), r"f.rb:12: sha256 for darkmatter-cli-x86_64-apple-darwin.tar.xz is 4{64}, but sha256.sum has 1{64}")

    def test_extra_bin_install(self) -> None:
        text = dist_formula().replace('      bin.install "md"\n', '      bin.install "md"\n      bin.install "md-helper"\n', 1)
        self.drift(text, r"f.rb:\d+: 'bin.install \"md-helper\"' installs something other than the one command 'md'")

    def test_binary_alias_refused(self) -> None:
        self.drift(dist_formula().replace('"x86_64-apple-darwin": {},', '"x86_64-apple-darwin": {"md": ["mdx"]},'), r"BINARY_ALIASES must be empty")

    def test_wrong_version(self) -> None:
        self.drift(dist_formula().replace('version "0.1.0"', 'version "0.1.1"'), r"f.rb:4: is 'version \"0.1.1\"', expected version \"0.1.0\"")

    def test_wrong_class_name(self) -> None:
        self.drift(dist_formula().replace("class DarkmatterCli <", "class Darkmatter <"), r"f.rb:1: class is 'Darkmatter', expected 'DarkmatterCli'")

    def test_missing_checksum_entry(self) -> None:
        sums = checksums()
        del sums["darkmatter-cli-x86_64-unknown-linux-gnu.tar.xz"]
        with self.assertRaisesRegex(DistributionError, r"sha256.sum: has no entry for darkmatter-cli-x86_64-unknown-linux-gnu.tar.xz"):
            tap.prepare_formula(dist_formula(), selection(), sums)

    def test_installed_version_parsing(self) -> None:
        self.assertEqual(tap.installed_version(dist_formula(version="0.9.0").encode(), source="tap"), "0.9.0")
        for bad in (b'class X < Formula\nend\n', dist_formula().replace('version "0.1.0"', 'version "0.1"').encode(),
                    (dist_formula() + '  version "0.1.0"\n').encode()):
            with self.assertRaises(DistributionError):
                tap.installed_version(bad, source="tap")

    def test_prepared_formula_is_valid_ruby(self) -> None:
        require_tools("ruby", enforced_by=ENFORCED_BY)
        for package in PACKAGES:
            with self.subTest(package=package), tempfile.TemporaryDirectory() as tmp:
                path = Path(tmp) / "formula.rb"
                path.write_text(tap.prepare_formula(dist_formula(package), selection(package), checksums(package)), encoding="utf-8")
                result = subprocess.run(["ruby", "-c", str(path)], capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)


class PublishTest(TempCase):
    def harness(self, installed: str | None = None, version: str = "0.1.0", **client: Any) -> Harness:
        write_assets(self.tmp, version=version)
        data = None if installed is None else self.installed_bytes(installed)
        return Harness(self.tmp, FakeClient(installed=data, **client))

    def installed_bytes(self, version: str) -> bytes:
        return tap.prepare_formula(dist_formula(version=version), selection(version=version), checksums()).encode()

    def assert_no_commit(self, h: Harness) -> None:
        self.assertNotIn("create_commit", h.client.names())
        self.assertEqual(h.verified, [])

    def test_create_when_absent(self) -> None:
        h = self.harness()
        report = h.run(selection())
        self.assertEqual(report, {"status": "created", "formula": "Formula/darkmatter-cli.rb", "version": "0.1.0",
                                  "reason": "the tap has no formula; creating 0.1.0", "commit": NEW, "formula_current": True})
        self.assertEqual(h.factory_tokens, ["token"])
        self.assertEqual(h.client.names(), ["viewer_login", "branch_head", "read_file", "create_commit", "fetch_commit_for_verification"])
        self.assertEqual(sorted(h.fetched), sorted(tap.release_url(selection(), t) for t in TARGETS))
        _, branch, expected, path, contents, headline = h.client.calls[3]
        self.assertEqual((branch, expected, path), ("main", HEAD, "Formula/darkmatter-cli.rb"))
        self.assertEqual(headline, "darkmatter-cli 0.1.0")
        self.assertNotIn("\n", headline)
        self.assertEqual(contents, self.installed_bytes("0.1.0"))
        self.assertEqual(h.client.calls[2], ("read_file", HEAD, "Formula/darkmatter-cli.rb"))
        self.assertEqual(h.verified, [NEW])

    def test_update_older_to_newer(self) -> None:
        h = self.harness(installed="0.0.9")
        report = h.run(selection())
        self.assertEqual((report["status"], report["reason"], report["commit"], report["formula_current"]),
                         ("updated", "0.0.9 -> 0.1.0", NEW, True))

    def test_numeric_not_lexical_ordering(self) -> None:
        h = self.harness(installed="0.9.0", version="0.10.0")
        report = h.run(selection(version="0.10.0"))
        self.assertEqual((report["status"], report["reason"]), ("updated", "0.9.0 -> 0.10.0"))

    def test_numeric_not_lexical_ordering_never_downgrades(self) -> None:
        h = self.harness(installed="0.10.0", version="0.9.0")
        report = h.run(selection(version="0.9.0"))
        self.assertEqual((report["status"], report["commit"], report["formula_current"]), ("noop", None, False))
        self.assert_no_commit(h)

    def test_newer_installed_is_noop(self) -> None:
        h = self.harness(installed="0.2.0")
        report = h.run(selection())
        self.assertEqual((report["status"], report["commit"], report["formula_current"]), ("noop", None, False))
        self.assert_no_commit(h)

    def test_identical_is_noop(self) -> None:
        h = self.harness(installed="0.1.0")
        report = h.run(selection())
        self.assertEqual((report["status"], report["formula_current"]), ("noop", True))
        self.assert_no_commit(h)

    def test_same_version_different_bytes_fails(self) -> None:
        h = self.harness()
        h.client.installed = self.installed_bytes("0.1.0") + b"# edited\n"
        with self.assertRaisesRegex(DistributionError, "already in the tap with different contents"):
            h.run(selection())
        self.assert_no_commit(h)

    def test_prerelease_skips_without_touching_the_tap(self) -> None:
        h = self.harness(version="0.2.0-rc.1")
        report = h.run(selection(version="0.2.0-rc.1", prerelease=True, formula_update=False))
        self.assertEqual((report["status"], report["formula_current"]), ("skip", False))
        self.assertEqual((h.factory_tokens, h.fetched), ([], []))

    def test_refused_selections_touch_nothing(self) -> None:
        cases = {
            "verify mode": dict(mode="verify", publishable=False),
            "verify forged publishable": dict(mode="verify", publishable=True),
            "unpublishable": dict(publishable=False),
            "publishable missing": dict(publishable=None),
            "formula_update false": dict(formula_update=False),
            "no brew channel": dict(channels={}),
        }
        for name, overrides in cases.items():
            with self.subTest(name):
                h = self.harness()
                with self.assertRaises(DistributionError):
                    h.run(selection(**overrides))
                self.assertEqual((h.factory_tokens, h.fetched, h.client.calls), ([], [], []))

    def test_prerelease_flag_shape_is_validated_before_any_effect(self) -> None:
        """`Selection.from_json` leaves `prerelease` unchecked; an absent or
        non-boolean flag must not read as "stable"."""
        for name, value in (("absent", None), ("string", "false")):
            with self.subTest(name):
                h = self.harness()
                payload = record()
                if value is None:
                    del payload["prerelease"]
                else:
                    payload["prerelease"] = value
                with self.assertRaisesRegex(DistributionError, r"^selection: prerelease: (is required|must be true or false)"):
                    h.run(Selection.from_json(payload, source="selection.json"))
                self.assertEqual((h.factory_tokens, h.fetched, h.client.calls), ([], [], []))

    def test_missing_credentials_exit_before_any_client_call(self) -> None:
        for missing in ("HOMEBREW_TAP_TOKEN", "HOMEBREW_TAP_AUTHOR"):
            with self.subTest(missing):
                h = self.harness()
                env = {k: v for k, v in ENV.items() if k != missing}
                with self.assertRaisesRegex(PendingSetup, rf"{missing}: .*never falls back to GITHUB_TOKEN"):
                    h.run(selection(), env=env | {"GITHUB_TOKEN": "fallback"})
                self.assertEqual((h.factory_tokens, h.fetched, h.client.calls), ([], [], []))

    def test_viewer_must_be_author(self) -> None:
        h = self.harness(viewer="someone-else")
        with self.assertRaisesRegex(DistributionError, "belongs to 'someone-else', but HOMEBREW_TAP_AUTHOR is 'ken'"):
            h.run(selection())
        self.assertEqual(h.client.names(), ["viewer_login"])

    def test_download_mismatch_or_truncation_blocks_commit(self) -> None:
        manglers = {
            "different bytes": (lambda name, data: data[:-1] + b"X", "downloaded bytes differ"),
            "truncated": (lambda name, data: data[:-10], "download returned"),
        }
        for name, (mangle, pattern) in manglers.items():
            with self.subTest(name):
                h = self.harness()
                h.mangle = mangle
                with self.assertRaisesRegex(DistributionError, pattern):
                    h.run(selection())
                self.assertEqual(h.client.names(), ["viewer_login"])

    def test_asset_missing_from_bundle(self) -> None:
        h = self.harness()
        (self.tmp / "darkmatter-cli-x86_64-unknown-linux-gnu.tar.xz").unlink()
        with self.assertRaisesRegex(DistributionError, "not in the build bundle"):
            h.run(selection())
        self.assert_no_commit(h)

    def test_head_conflict_fails_once(self) -> None:
        h = self.harness(conflict=True)
        with self.assertRaisesRegex(tap.TapConflict, "rerun the same tag"):
            h.run(selection())
        self.assertEqual(h.client.names().count("create_commit"), 1)
        self.assertEqual(h.verified, [])

    def test_invalid_returned_signature_fails(self) -> None:
        h = self.harness(signature_valid=False)
        with self.assertRaisesRegex(tap.TapFailure, "valid signature") as caught:
            h.run(selection())
        self.assertEqual(caught.exception.commit, NEW)
        self.assertEqual(h.verified, [])

    def test_wrong_commit_author_fails(self) -> None:
        h = self.harness(author="web-flow")
        with self.assertRaisesRegex(tap.TapFailure, "author is 'web-flow'"):
            h.run(selection())

    def test_local_verification_failure_fails_once(self) -> None:
        h = self.harness()

        def reject(repo: Path, oid: str) -> str:
            h.verified.append(oid)
            raise DistributionError(f"commit {oid}", "git verify-commit failed")
        h.verify = reject  # type: ignore[method-assign]
        with self.assertRaisesRegex(tap.TapFailure, "git verify-commit failed"):
            h.run(selection())
        self.assertEqual(h.verified, [NEW])
        self.assertEqual(h.client.names().count("create_commit"), 1)

    def test_dry_run_stops_before_commit(self) -> None:
        h = self.harness(installed="0.0.9")
        report = h.run(selection(), dry_run=True)
        self.assertEqual((report["status"], report["reason"], report["formula_current"]), ("dry-run", "would update: 0.0.9 -> 0.1.0", False))
        self.assert_no_commit(h)
        self.assertEqual(len(h.fetched), 4)

    def test_malformed_installed_formula_fails(self) -> None:
        h = self.harness()
        h.client.installed = b"class DarkmatterCli < Formula\nend\n"
        with self.assertRaisesRegex(DistributionError, "exactly one `version"):
            h.run(selection())
        self.assert_no_commit(h)


class CommandLineTest(TempCase):
    def main(self, *argv: str, env: dict[str, str] = ENV, harness: Harness | None = None) -> tuple[int, dict[str, Any]]:
        out, err = io.StringIO(), io.StringIO()
        kwargs = {} if harness is None else dict(client_factory=harness.factory, fetch=harness.fetch, verify=harness.verify)
        with redirect_stdout(out), redirect_stderr(err):
            code = tap.main(list(argv), env=env, **kwargs)
        return code, json.loads(out.getvalue())

    def write_selection(self, **overrides: Any) -> str:
        path = self.tmp / "selection.json"
        path.write_text(json.dumps(record(**overrides)), encoding="utf-8")
        return str(path)

    def test_prepare_writes_the_formula(self) -> None:
        assets = write_assets(Path(tempfile.mkdtemp(dir=self.tmp)))
        output = self.tmp / "out.rb"
        code, result = self.main("prepare", "--selection", self.write_selection(), "--assets-dir", str(assets), "--output", str(output))
        self.assertEqual((code, result["status"]), (0, "prepared"))
        self.assertEqual(output.read_text(encoding="utf-8"), tap.prepare_formula(dist_formula(), selection(), checksums()))

    def test_publish_dry_run_report(self) -> None:
        assets = write_assets(Path(tempfile.mkdtemp(dir=self.tmp)))
        h = Harness(assets)
        report = self.tmp / "report.json"
        code, result = self.main("publish", "--selection", self.write_selection(), "--assets-dir", str(assets),
                                 "--dry-run", "--report", str(report), harness=h)
        self.assertEqual(code, 0)
        self.assertEqual(result, {"status": "dry-run", "formula": "Formula/darkmatter-cli.rb", "version": "0.1.0",
                                  "reason": "would create: the tap has no formula; creating 0.1.0", "commit": None,
                                  "formula_current": False})
        self.assertEqual(json.loads(report.read_text(encoding="utf-8")), result)

    def test_malformed_selection_is_located_and_launches_nothing(self) -> None:
        assets = write_assets(Path(tempfile.mkdtemp(dir=self.tmp)))
        path = self.tmp / "selection.json"
        path.write_text(json.dumps(record())[:-1] + ', "version": "0.1.0"}', encoding="utf-8")
        h = Harness(assets)
        code, result = self.main("publish", "--selection", str(path), "--assets-dir", str(assets), harness=h)
        self.assertEqual((code, result["status"]), (2, "failed"))
        self.assertIn(f"{path.as_posix()}: invalid JSON: duplicate object key 'version'", result["reason"])
        self.assertEqual((h.factory_tokens, h.fetched, h.client.calls, h.verified), ([], [], [], []))

    def test_exit_codes(self) -> None:
        assets = write_assets(Path(tempfile.mkdtemp(dir=self.tmp)))
        code, result = self.main("publish", "--selection", self.write_selection(), "--assets-dir", str(assets), env={})
        self.assertEqual((code, result["status"]), (3, "pending-setup"))
        self.assertIn("HOMEBREW_TAP_TOKEN", result["reason"])
        code, result = self.main("publish", "--selection", self.write_selection(mode="verify"), "--assets-dir", str(assets))
        self.assertEqual((code, result["status"]), (2, "failed"))
        h = Harness(assets, FakeClient(signature_valid=False))
        code, result = self.main("publish", "--selection", self.write_selection(), "--assets-dir", str(assets), harness=h)
        self.assertEqual((code, result["status"], result["commit"]), (2, "failed", NEW))


class GitHubClientTest(unittest.TestCase):
    class Recording(tap.GitHubTapClient):
        def __init__(self, response: Any, status: int = 200) -> None:
            super().__init__("token")
            self.response = response
            self.status = status
            self.bodies: list[Any] = []

        def _request(self, url: str, *, body: Any = None, accept: str = "") -> tuple[int, bytes]:
            self.bodies.append(body)
            raw = self.response if isinstance(self.response, bytes) else json.dumps(self.response).encode()
            return self.status, raw

    COMMIT = {"oid": NEW, "signature": {"isValid": True, "signer": {"login": "web-flow"}}, "author": {"user": {"login": "ken"}}}

    def commit_response(self, *path: Any, value: Any = None, delete: bool = False) -> dict[str, Any]:
        """The real-shaped control response with one field replaced or deleted."""
        commit = json.loads(json.dumps(self.COMMIT))
        if path:
            parent = commit
            for key in path[:-1]:
                parent = parent[key]
            if delete:
                del parent[path[-1]]
            else:
                parent[path[-1]] = value
        return {"data": {"createCommitOnBranch": {"commit": commit}}}

    def commit(self, response: Any, status: int = 200) -> tap.CommitResult:
        return self.Recording(response, status).create_commit("main", HEAD, "Formula/bh.rb", b"x", "bh 0.1.0")

    def test_commit_changes_one_path_with_a_bare_headline(self) -> None:
        client = self.Recording({"data": {"createCommitOnBranch": {"commit": {
            "oid": NEW, "signature": {"isValid": True, "signer": {"login": "web-flow"}}, "author": {"user": {"login": "ken"}}}}}})
        result = client.create_commit("main", HEAD, "Formula/bh.rb", b"formula\n", "bh 0.1.0")
        self.assertEqual(result, tap.CommitResult(NEW, True, "web-flow", "ken"))
        payload = client.bodies[0]["variables"]["input"]
        self.assertEqual(payload["message"], {"headline": "bh 0.1.0"})
        self.assertEqual(payload["expectedHeadOid"], HEAD)
        self.assertEqual(payload["branch"], {"repositoryNameWithOwner": "yankeeinlondon/homebrew-tap", "branchName": "main"})
        self.assertEqual(list(payload["fileChanges"]), ["additions"])
        self.assertEqual([a["path"] for a in payload["fileChanges"]["additions"]], ["Formula/bh.rb"])

    def test_head_conflict_is_recognized(self) -> None:
        client = self.Recording({"errors": [{"message": "Expected branch to point to \"abc\" but it did not"}]})
        with self.assertRaisesRegex(tap.TapConflict, "rerun the same tag"):
            client.create_commit("main", HEAD, "Formula/bh.rb", b"x", "bh 0.1.0")

    def test_missing_signature_is_invalid(self) -> None:
        client = self.Recording({"data": {"createCommitOnBranch": {"commit": {"oid": NEW, "signature": None, "author": None}}}})
        self.assertEqual(client.create_commit("main", HEAD, "Formula/bh.rb", b"x", "bh 0.1.0"), tap.CommitResult(NEW, False, None, None))

    def test_nullable_object_validator(self) -> None:
        """`_nullable_mapping`, once: null is a value, absence and other
        types are located errors, and an empty object reaches its fields."""
        self.assertEqual(self.commit(self.commit_response("signature", value=None)).signature_valid, False)
        cases = {
            "absent": (dict(delete=True), r"commit.signature: is required \(GitHub returns null"),
            "wrong type": (dict(value=True), r"commit.signature: must be a mapping, found boolean"),
            "empty": (dict(value={}), r"commit.signature.isValid: is required"),
        }
        for name, (change, pattern) in cases.items():
            with self.subTest(name), self.assertRaisesRegex(DistributionError, pattern):
                self.commit(self.commit_response("signature", **change))

    def test_each_commit_field_rejects_an_invalid_shape(self) -> None:
        cases = {
            ("oid",): ("abc", r"commit.oid: must be a full 40-digit commit SHA"),
            ("signature", "isValid"): ("true", r"signature.isValid: must be true or false, found string"),
            ("signature", "signer"): ([], r"signature.signer: must be a mapping, found list"),
            ("signature", "signer", "login"): (7, r"signer.login: must be a string, found integer"),
            ("author",): ("ken", r"commit.author: must be a mapping, found string"),
            ("author", "user"): (1, r"author.user: must be a mapping, found integer"),
            ("author", "user", "login"): (None, r"user.login: must not be null"),
        }
        for path, (value, pattern) in cases.items():
            with self.subTest(path), self.assertRaisesRegex(DistributionError, pattern):
                self.commit(self.commit_response(*path, value=value))

    def test_graphql_envelope_shapes(self) -> None:
        cases = {
            "not JSON": (b"<html>502</html>", 502, r"^GitHub GraphQL \(HTTP 502\): invalid JSON"),
            "duplicate key": (b'{"data": {}, "data": {}}', 200, r"invalid JSON: duplicate object key 'data'"),
            "list root": ([], 200, r"<document>: must be a mapping, found list|must be a mapping, found list"),
            "no data": ({}, 200, r": data: is required"),
            "null data": ({"data": None}, 200, r": data: must not be null"),
            "empty errors": ({"errors": []}, 200, r"errors: must not be empty"),
            "error without message": ({"errors": [{"type": "X"}]}, 200, r"errors\[0\].message: is required"),
            "failed without errors": ({"data": {}}, 500, r"HTTP 500\): the request failed without a GraphQL error"),
        }
        for name, (response, status, pattern) in cases.items():
            with self.subTest(name), self.assertRaisesRegex(DistributionError, pattern):
                self.commit(response, status)

    def test_viewer_and_branch_head_shapes(self) -> None:
        self.assertEqual(self.Recording({"data": {"viewer": {"login": "ken"}}}).viewer_login(), "ken")
        with self.assertRaisesRegex(DistributionError, r"viewer.login: is required"):
            self.Recording({"data": {"viewer": {}}}).viewer_login()
        self.assertEqual(self.Recording({"object": {"sha": HEAD}}).branch_head("main"), HEAD)
        for response, pattern in (({"object": {"sha": None}}, r"object.sha: must not be null"),
                                  (b'{"object": {"sha": "%s"}} trailing' % HEAD.encode(), r"invalid JSON")):
            with self.subTest(response=response), self.assertRaisesRegex(DistributionError, rf"^yankeeinlondon/homebrew-tap@main.*{pattern}"):
                self.Recording(response).branch_head("main")


class SignatureTest(TempCase):
    def fake_runner(self, *, fingerprints: tuple[str, ...] = tuple(sorted(tap.WEB_FLOW_FINGERPRINTS)),
                    signer: str = tap.WEB_FLOW_SIGNING_KEY, verify_code: int = 0) -> tuple[list[tuple[list[str], dict[str, str]]], Any]:
        calls: list[tuple[list[str], dict[str, str]]] = []

        def run(args: list[str], *, env: dict[str, str], **_: Any) -> subprocess.CompletedProcess[str]:
            calls.append((args, dict(env)))
            self.assertTrue(os.path.isdir(env["GNUPGHOME"]))
            stdout = stderr = ""
            if "--list-keys" in args:
                stdout = "".join(f"pub:-:4096:1:X:0:::-:::sc:\nfpr:::::::::{f}:\nsub:-:4096:1:Y:0:::::e:\nfpr:::::::::{'0' * 40}:\n" for f in fingerprints)
            if "verify-commit" in args:
                stderr = f"[GNUPG:] GOODSIG B5690EEEBB952194 GitHub\n[GNUPG:] VALIDSIG {signer} 2026-11-09 0 0 0 4 0 1 8 00 {signer}\n"
            return subprocess.CompletedProcess(args, verify_code if "verify-commit" in args else 0, stdout, stderr)
        return calls, run

    def test_isolated_home_and_pinned_key(self) -> None:
        calls, run = self.fake_runner()
        self.assertEqual(tap.verify_signature(self.tmp, NEW, run=run), tap.WEB_FLOW_SIGNING_KEY)
        homes = {env["GNUPGHOME"] for _, env in calls}
        self.assertEqual(len(homes), 1)
        home = homes.pop()
        self.assertNotEqual(home, os.path.expanduser("~/.gnupg"))
        self.assertFalse(os.path.exists(home), "the GPG home is removed afterwards")
        self.assertEqual(calls[0][0], ["gpg", "--batch", "--import", str(tap.WEB_FLOW_KEY_FILE)])
        self.assertEqual(calls[2][0][-3:], ["verify-commit", "--raw", NEW])
        for _, env in calls:
            self.assertEqual((env["GIT_CONFIG_GLOBAL"], env["GIT_CONFIG_NOSYSTEM"], env["HOME"]), (os.devnull, "1", home))

    def test_unpinned_key_refused(self) -> None:
        _, run = self.fake_runner(fingerprints=(tap.WEB_FLOW_SIGNING_KEY, "F" * 40))
        with self.assertRaisesRegex(DistributionError, "not the pinned web-flow keys"):
            tap.verify_signature(self.tmp, NEW, run=run)

    def test_other_signer_refused(self) -> None:
        _, run = self.fake_runner(signer="5DE3E0509C47EA3CF04A42D34AEE18F83AFDEB23")
        with self.assertRaisesRegex(DistributionError, "not GitHub's web-flow key"):
            tap.verify_signature(self.tmp, NEW, run=run)

    def test_verify_commit_failure_refused(self) -> None:
        _, run = self.fake_runner(verify_code=1)
        with self.assertRaisesRegex(DistributionError, "git verify-commit failed"):
            tap.verify_signature(self.tmp, NEW, run=run)

    def test_pinned_file_holds_exactly_the_pinned_fingerprints(self) -> None:
        require_tools("gpg", enforced_by=ENFORCED_BY)
        home = self.tmp / "gnupg"
        home.mkdir(mode=0o700)
        result = subprocess.run(["gpg", "--batch", "--with-colons", "--show-keys", str(tap.WEB_FLOW_KEY_FILE)],
                                env={**os.environ, "GNUPGHOME": str(home)}, capture_output=True, text=True, check=True)
        lines = result.stdout.splitlines()
        primaries = {lines[i + 1].split(":")[9] for i, line in enumerate(lines) if line.startswith("pub:")}
        self.assertEqual(primaries, tap.WEB_FLOW_FINGERPRINTS)


def tap_report(**overrides: Any) -> dict[str, Any]:
    base = {"status": "created", "formula": "Formula/darkmatter-cli.rb", "version": "0.1.0", "reason": "r",
            "commit": NEW, "formula_current": True}
    base.update(overrides)
    return base


class FormulaCurrentTest(unittest.TestCase):
    """The tap report field the workflow turns into the `brew-install` job's
    condition: a control and one invalid shape per field read."""

    def read(self, report: Any) -> bool:
        return tap.formula_current(report, selection(), source="tap-report.json")

    def test_status_decides_what_the_field_may_say(self) -> None:
        for status, current in (("created", True), ("updated", True), ("noop", True), ("noop", False), ("skip", False)):
            with self.subTest(status=status, current=current):
                self.assertIs(self.read(tap_report(status=status, formula_current=current)), current)

    def test_every_invalid_shape_is_a_located_error(self) -> None:
        without = {k: v for k, v in tap_report().items() if k != "formula_current"}
        cases = [
            (without, r"^tap-report\.json: formula_current: is required"),
            (tap_report(formula_current=None), r"^tap-report\.json: formula_current: must not be null"),
            (tap_report(formula_current="true"), r"^tap-report\.json: formula_current: must be true or false"),
            (tap_report(status="created", formula_current=False), r"formula_current: cannot be false for status 'created'"),
            (tap_report(status="updated", formula_current=False), r"formula_current: cannot be false for status 'updated'"),
            (tap_report(status="skip", formula_current=True), r"formula_current: cannot be true for status 'skip'"),
            (tap_report(status="dry-run", formula_current=False), r"status: must be one of"),
            (tap_report(status="failed"), r"status: must be one of"),
            ({k: v for k, v in tap_report().items() if k != "status"}, r"status: is required"),
            (tap_report(version="0.0.9"), r"version: is '0\.0\.9', but the selection is '0\.1\.0'"),
            (tap_report(formula="Formula/bh.rb"), r"formula: is 'Formula/bh\.rb'"),
            ([], r"must be a mapping"),
        ]
        for report, pattern in cases:
            with self.subTest(pattern=pattern), self.assertRaisesRegex(DistributionError, pattern):
                self.read(report)

    def test_publish_reports_round_trip(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            write_assets(Path(tmp))
            for installed, current in ((None, True), ("0.0.9", True), ("0.1.0", True), ("0.2.0", False)):
                with self.subTest(installed=installed):
                    data = None if installed is None else tap.prepare_formula(
                        dist_formula(version=installed), selection(version=installed), checksums()).encode()
                    report = Harness(Path(tmp), FakeClient(installed=data)).run(selection())
                    self.assertIs(self.read(json.loads(json.dumps(report))), current)


class FakeBrew:
    """A recording Homebrew: `brew` subcommands answer like the real one for a
    tap holding `tapped_version`; a step named in `fail` exits 1."""

    def __init__(self, root: Path, *, tapped_version: str = "0.1.0", listed: str | None = None,
                 reported: str = "md 0.1.0", fail: str | None = None, missing: bool = False) -> None:
        self.root, self.calls, self.envs = root, [], []
        self.listed = "darkmatter-cli 0.1.0" if listed is None else listed
        self.reported, self.fail, self.missing = reported, fail, missing
        self.tap_dir = root / "Taps" / "yankeeinlondon" / "homebrew-tap"
        (self.tap_dir / "Formula").mkdir(parents=True)
        formula = tap.prepare_formula(dist_formula(version=tapped_version), selection(version=tapped_version), checksums())
        (self.tap_dir / "Formula" / "darkmatter-cli.rb").write_text(formula, encoding="utf-8")

    def __call__(self, args: list[str], *, env: dict[str, str], **_: Any) -> subprocess.CompletedProcess[str]:
        self.calls.append(args)
        self.envs.append(env)
        if self.missing:
            raise FileNotFoundError(args[0])
        step = args[1] if args[0] == "brew" else args[0].rsplit("/", 1)[-1]
        out = {"--version": "Homebrew 7.0.12\n", "--repository": f"{self.tap_dir}\n", "list": f"{self.listed}\n",
               "--prefix": f"{self.root}\n", "md": f"{self.reported}\n"}.get(step, "")
        return subprocess.CompletedProcess(args, 1 if step == self.fail else 0, out, "boom" if step == self.fail else "")


DARWIN_ARM = ("Darwin", "arm64")


class InstallCheckTest(TempCase):
    def check(self, brew: FakeBrew, sel: Selection | None = None, target: str = "aarch64-apple-darwin",
              host: tuple[str, str] = DARWIN_ARM, env: dict[str, str] | None = None) -> dict[str, Any]:
        return tap.install_check(sel or selection(), target, env=env or {"PATH": "/bin"}, run=brew, host=host)

    def test_installs_from_the_public_tap_and_runs_the_installed_command(self) -> None:
        brew = FakeBrew(self.tmp)
        env = {"PATH": "/bin", "GITHUB_TOKEN": "t", "GH_TOKEN": "t", "HOMEBREW_GITHUB_API_TOKEN": "t", "HOMEBREW_TAP_TOKEN": "t"}
        evidence = self.check(brew, env=env)
        self.assertEqual(brew.calls, [
            ["brew", "--version"],
            ["brew", "tap", "yankeeinlondon/tap"],
            ["brew", "--repository", "yankeeinlondon/tap"],
            ["brew", "install", "yankeeinlondon/tap/darkmatter-cli"],
            ["brew", "list", "--versions", "darkmatter-cli"],
            ["brew", "--prefix"],
            [str(self.tmp / "bin" / "md"), "--version"],
            ["brew", "uninstall", "darkmatter-cli"],
        ])
        for seen in brew.envs:
            self.assertFalse(set(seen) & {"GITHUB_TOKEN", "GH_TOKEN", "HOMEBREW_GITHUB_API_TOKEN", "HOMEBREW_TAP_TOKEN"})
            self.assertEqual((seen["HOMEBREW_NO_AUTO_UPDATE"], seen["HOMEBREW_NO_ANALYTICS"], seen["HOMEBREW_NO_INSTALL_CLEANUP"],
                              seen["NONINTERACTIVE"], seen["PATH"]), ("1", "1", "1", "1", "/bin"))
        self.assertEqual((evidence["status"], evidence["target"], evidence["version"], evidence["homebrew"], evidence["command_version"]),
                         ("installed", "aarch64-apple-darwin", "0.1.0", "Homebrew 7.0.12", "md 0.1.0"))

    def test_each_target_must_run_on_its_own_host(self) -> None:
        for host, target in tap.HOST_TARGETS.items():
            with self.subTest(target=target):
                self.assertEqual(self.check(FakeBrew(Path(tempfile.mkdtemp(dir=self.tmp))), target=target, host=host)["target"], target)
        brew = FakeBrew(self.tmp)
        with self.assertRaisesRegex(DistributionError, r"--target: is x86_64-apple-darwin, but this runner is Darwin arm64"):
            self.check(brew, target="x86_64-apple-darwin")
        with self.assertRaisesRegex(DistributionError, r"--target: must be one of"):
            self.check(brew, target="x86_64-pc-windows-msvc")
        self.assertEqual(brew.calls, [])

    def test_a_release_without_a_formula_update_installs_nothing(self) -> None:
        for name, sel in (("prerelease", selection(version="0.2.0-rc.1", prerelease=True, formula_update=False)),
                          ("no update", selection(formula_update=False)),
                          ("verify", selection(mode="verify", publishable=False)),
                          ("no brew", selection(channels={}))):
            with self.subTest(name):
                brew = FakeBrew(Path(tempfile.mkdtemp(dir=self.tmp)))
                with self.assertRaises(DistributionError):
                    self.check(brew, sel)
                self.assertEqual(brew.calls, [])

    def test_every_failed_proof_fails_before_uninstall(self) -> None:
        cases = [
            (dict(tapped_version="0.2.0"), r"darkmatter-cli\.rb: names 0\.2\.0, but this run proves 0\.1\.0", "--repository"),
            (dict(listed="darkmatter-cli 0.0.9"), r"brew list --versions darkmatter-cli: is 'darkmatter-cli 0\.0\.9'", "list"),
            (dict(reported="md 0.0.9"), r"md --version: printed 'md 0\.0\.9', which does not contain 0\.1\.0", "md"),
            (dict(fail="install"), r"brew install yankeeinlondon/tap/darkmatter-cli: exited 1:\nboom", "install"),
            (dict(fail="tap"), r"brew tap yankeeinlondon/tap: exited 1", "tap"),
            (dict(missing=True), r"^brew: is not on PATH", "--version"),
        ]
        for options, pattern, last in cases:
            with self.subTest(pattern=pattern):
                brew = FakeBrew(Path(tempfile.mkdtemp(dir=self.tmp)), **options)
                with self.assertRaisesRegex(DistributionError, pattern):
                    self.check(brew)
                self.assertNotIn(["brew", "uninstall", "darkmatter-cli"], brew.calls)
                called = brew.calls[-1][1] if brew.calls[-1][0] == "brew" else brew.calls[-1][0].rsplit("/", 1)[-1]
                self.assertEqual(called, last)

    def test_unusable_paths_are_refused(self) -> None:
        brew = FakeBrew(self.tmp)
        brew.tap_dir = Path("relative")
        with self.assertRaisesRegex(DistributionError, r"brew --repository yankeeinlondon/tap: must print an absolute path"):
            self.check(brew)
        brew = FakeBrew(Path(tempfile.mkdtemp(dir=self.tmp)))
        (brew.tap_dir / "Formula" / "darkmatter-cli.rb").unlink()
        with self.assertRaisesRegex(DistributionError, r"the tapped formula cannot be read"):
            self.check(brew)


class InstallCommandLineTest(TempCase):
    def main(self, *argv: str, **kwargs: Any) -> tuple[int, dict[str, Any]]:
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            code = tap.main(list(argv), env={"PATH": "/bin"}, **kwargs)
        return code, json.loads(out.getvalue())

    def write(self, name: str, value: Any) -> str:
        path = self.tmp / name
        path.write_text(json.dumps(value), encoding="utf-8")
        return str(path)

    def test_formula_current_command(self) -> None:
        sel = self.write("selection.json", record())
        for current in (True, False):
            with self.subTest(current=current):
                report = self.write("tap-report.json", tap_report(status="noop", formula_current=current))
                code, result = self.main("formula-current", "--selection", sel, "--tap-report", report)
                self.assertEqual((code, result["status"], result["formula_current"]), (0, "read", current))
        code, result = self.main("formula-current", "--selection", sel, "--tap-report", self.write("bad.json", tap_report(formula_current="yes")))
        self.assertEqual((code, result["status"]), (2, "failed"))
        self.assertIn("bad.json: formula_current: must be true or false", result["reason"])
        code, result = self.main("formula-current", "--selection", sel, "--tap-report", str(self.tmp / "absent.json"))
        self.assertEqual((code, result["status"]), (2, "failed"))
        self.assertIn("absent.json: cannot read", result["reason"])

    def test_install_check_command_writes_evidence(self) -> None:
        sel = self.write("selection.json", record())
        brew = FakeBrew(Path(tempfile.mkdtemp(dir=self.tmp)))
        evidence = self.tmp / "evidence.json"
        code, result = self.main("install-check", "--selection", sel, "--target", "aarch64-apple-darwin",
                                 "--report", str(evidence), run=brew, host=DARWIN_ARM)
        self.assertEqual((code, result["status"]), (0, "installed"))
        self.assertEqual(json.loads(evidence.read_text(encoding="utf-8")), result)
        failing = FakeBrew(Path(tempfile.mkdtemp(dir=self.tmp)), fail="install")
        code, result = self.main("install-check", "--selection", sel, "--target", "aarch64-apple-darwin",
                                 "--report", str(evidence), run=failing, host=DARWIN_ARM)
        self.assertEqual((code, result["status"]), (2, "failed"))
        self.assertEqual(json.loads(evidence.read_text(encoding="utf-8"))["status"], "failed")


# Real output of Homebrew 2026-11-09 on a prepared four-target claudine
# formula derived from the darkmatter fixture (so its `desc` is darkmatter's).
AUDIT_OUTPUT = """local/distribution-audit/claudine
  * Stable: `version 0.1.0` is redundant with version scanned from URL
  * line 1, col 1: A `test do` test block should be added
  * line 2, col 8: Description is too long. It should be less than 80 characters. The current length is 82.
  * line 29, col 20: Freeze mutable objects assigned to constants.
  * line 30, col 5: Align the keys and values of a hash literal if they span more than one line.
  * line 33, col 5: Put a comma after the last item of a multiline hash.
Error: 6 problems in 1 formula detected.
"""
STYLE_OUTPUT = """Formula/bh.rb:1:1: C: FormulaAudit/Test: A test do test block should be added
Formula/bh.rb:2:9: C: [Correctable] FormulaAudit/Desc: Description shouldn't start with an article.
Formula/bh.rb:28:20: C: [Correctable] Style/MutableConstant: Freeze mutable objects assigned to constants.
Formula/bh.rb:29:5: C: [Correctable] Layout/HashAlignment: Align the keys and values of a hash literal if they span more than one line.
Formula/bh.rb:32:5: C: [Correctable] Style/TrailingCommaInHashLiteral: Put a comma after the last item of a multiline hash.

1 file inspected, 5 offenses detected, 4 offenses autocorrectable
"""


class AuditTest(unittest.TestCase):
    def test_only_dist_template_findings_are_accepted(self) -> None:
        self.assertEqual(tap.unexpected_findings("audit", AUDIT_OUTPUT),
                         ["Description is too long. It should be less than 80 characters. The current length is 82."])
        self.assertEqual(tap.unexpected_findings("style", STYLE_OUTPUT), ["FormulaAudit/Desc: Description shouldn't start with an article."])
        template_only = "\n".join(line for line in AUDIT_OUTPUT.splitlines() if "Description" not in line)
        self.assertEqual(tap.unexpected_findings("audit", template_only), [])
        self.assertIsNone(tap.unexpected_findings("audit", "Error: Refusing to load formula from untrusted tap\n"))
        self.assertEqual(tap.unexpected_findings("audit", "  * line 51, col 133: Line is too long. [154/118]\n"),
                         ["Line is too long. [154/118]"])

    def test_unusable_tap_location_runs_nothing_else(self) -> None:
        for stdout, code in (("", 0), ("relative/tap", 0), ("/brew/Taps/local/x", 1)):
            with self.subTest(stdout=stdout, code=code):
                calls: list[list[str]] = []

                def run(args: list[str], **_: Any) -> subprocess.CompletedProcess[str]:
                    calls.append(args)
                    return subprocess.CompletedProcess(args, code, stdout + "\n", "")
                with mock.patch.object(tap.shutil, "which", return_value="/usr/bin/brew"), \
                        self.assertRaisesRegex(DistributionError, r"must print the tap's absolute path"):
                    tap.run_audit("bh", Path("/tmp/bh.rb"), run=run)
                self.assertEqual(calls, [["brew", "--repository", "local/distribution-audit"]])

    def test_audit_commands(self) -> None:
        self.assertEqual(tap.audit_commands("bh", Path("/tmp/bh.rb"), Path("/brew/Taps/local/homebrew-distribution-audit")), [
            ["brew", "tap-new", "--no-git", "local/distribution-audit"],
            ["cp", "/tmp/bh.rb", "/brew/Taps/local/homebrew-distribution-audit/Formula/bh.rb"],
            ["brew", "trust", "local/distribution-audit"],
            ["brew", "audit", "--strict", "--formula", "local/distribution-audit/bh"],
            ["brew", "style", "local/distribution-audit/bh"],
        ])


if __name__ == "__main__":
    unittest.main()

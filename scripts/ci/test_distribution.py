#!/usr/bin/env python3
"""Tests for `distribution.py`: the strict policy loader, release and
verification preflight, the generated dist opt-in, plan validation, build
runners, build bundles, asset reconciliation, immutable publication, and tap
version decisions.

Policy cases start from the real `docs/deployment.yaml` and real
`cargo metadata` for this checkout, changing one value per case, so the
unedited control proves each edit is what changed the result. Requires the
pins in `requirements-distribution.txt`; a missing dependency fails the import
rather than skipping.
"""

from __future__ import annotations

import io
import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
import urllib.parse
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))

import distribution as d  # noqa: E402

REPO = Path(__file__).resolve().parents[2]
POLICY_PATH = REPO / "docs" / "deployment.yaml"
TAG_SHA = "1" * 40
MERGE_SHA = "2" * 40


def _real_metadata() -> str:
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"],
        cwd=REPO, capture_output=True, text=True, timeout=300, check=True,
    )
    return result.stdout


def _read(path: str) -> str | None:
    file = REPO / path
    return file.read_text(encoding="utf-8") if file.is_file() else None


METADATA = _real_metadata()
EXCLUDED = d.excluded_package_names((REPO / "Cargo.toml").read_text(encoding="utf-8"), _read)
WORKSPACE = d.load_workspace(METADATA, source="cargo metadata", excluded=EXCLUDED)
POLICY_TEXT = POLICY_PATH.read_text(encoding="utf-8")


def edit_entry(text: str, package: str, field: str, value: str | None) -> str:
    """Set (or with None, remove) one field in one flow-mapping entry of the
    real policy text."""
    block = re.search(rf"\n    {re.escape(package)}:\n        \{{\n(.*?)\n        \}}", text, re.S)
    assert block, package
    body = block.group(1)
    line = re.compile(rf"^            {re.escape(field)}:.*,$", re.M)
    if value is None:
        new_body, count = line.subn("", body)
        new_body = "\n".join(part for part in new_body.split("\n") if part)
    else:
        new_body, count = line.subn(f"            {field}: {value},", body)
    assert count == 1, (package, field)
    return text[: block.start(1)] + new_body + text[block.end(1):]


def approve(text: str, package: str, **channels: str) -> str:
    text = edit_entry(text, package, "approved", "true")
    for channel, value in channels.items():
        text = edit_entry(text, package, channel, value)
    return text


_PARSED: dict[tuple[str, str, frozenset[str]], d.Policy] = {}


def policy(text: str, source: str = "deployment.yaml", workspace: d.Workspace = WORKSPACE) -> d.Policy:
    """`parse_policy`, memoized because the pure-Python YAML parse of the
    real policy dominates the suite's time. The workspace enters a policy
    only through `Workspace.knows`, so the key holds the names it knows.
    Only successful parses are kept: every invalid case still parses and
    raises."""
    key = (text, source, frozenset(workspace.packages) | workspace.excluded)
    if key not in _PARSED:
        _PARSED[key] = d.parse_policy(text.encode("utf-8"), workspace, source=source)
    return _PARSED[key]


def release(tag: str, *, draft: bool = False, prerelease: bool = False) -> d.ReleaseState:
    return d.ReleaseState(tag, 42, draft, prerelease, f"{d.CANONICAL_URL}/releases/tag/{tag}")


def facts(tag: str = "biscuit-hash-cli-v0.1.0", **overrides) -> d.ReleaseFacts:
    values = dict(
        mode="release",
        repository=d.CANONICAL_REPOSITORY,
        tag=tag,
        release=release(tag),
        tag_commit=TAG_SHA,
        checkout_commit=TAG_SHA,
        proposal=d.ProposalProof(17, TAG_SHA, MERGE_SHA),
    )
    values.update(overrides)
    return d.ReleaseFacts(**values)


def policies(tagged: str, current: str | None = None) -> d.Policies:
    return d.Policies(current=policy(current if current is not None else tagged, "current"), tagged=policy(tagged, "tagged"))


def with_package(name: str, **changes) -> d.Workspace:
    packages = dict(WORKSPACE.packages)
    packages[name] = d.Package(**{**packages[name].__dict__, **changes})
    return d.Workspace(packages, WORKSPACE.excluded)


class YamlLoaderTests(unittest.TestCase):
    """Document shapes, decided once by `load_yaml` for every YAML input."""

    def load(self, text: str) -> d.Document:
        return d.load_yaml(text.encode("utf-8"), source="doc.yaml")

    def assertLocated(self, text: str, message: str, line: int | None = None) -> None:
        with self.assertRaises(d.DistributionError) as caught:
            self.load(text)
        self.assertIn(message, str(caught.exception))
        if line is not None:
            self.assertIn(f"doc.yaml:{line}:", str(caught.exception))

    def test_comments_and_flow_mappings_load(self) -> None:
        document = self.load("# c\npackages:\n    a: { crate: a, approved: false }  # trailing\n")
        self.assertEqual(document.value, {"packages": {"a": {"crate": "a", "approved": False}}})
        self.assertIn("doc.yaml:3:", document.where(["packages", "a", "approved"]))

    def test_duplicate_keys_are_refused_at_every_depth(self) -> None:
        self.assertLocated("a: 1\na: 2\n", "duplicate key", line=2)
        self.assertLocated("packages:\n  x: {}\n  x: {}\n", "packages.x: duplicate key", line=3)
        self.assertLocated("packages:\n  x: { brew: false, brew: true }\n", "packages.x.brew: duplicate key", line=2)

    def test_empty_value_and_null_are_null_not_absent(self) -> None:
        self.assertEqual(self.load("k:\n").value, {"k": None})
        self.assertEqual(self.load("k: null\n").value, {"k": None})
        self.assertEqual(self.load("k: ~\n").value, {"k": None})

    def test_only_lowercase_true_and_false_are_booleans(self) -> None:
        document = self.load("a: true\nb: false\nc: on\nd: yes\ne: True\nf: 'false'\n")
        self.assertEqual(document.value, {"a": True, "b": False, "c": "on", "d": "yes", "e": "True", "f": "false"})

    def test_a_second_document_is_refused(self) -> None:
        self.assertLocated("a: 1\n---\nb: 2\n", "invalid YAML")

    def test_trailing_invalid_content_is_refused(self) -> None:
        self.assertLocated("a: 1\n]garbage\n", "invalid YAML")

    def test_unsupported_tags_are_refused(self) -> None:
        self.assertLocated("a: !!binary aGk=\n", "unsupported tag")
        self.assertLocated("a: !custom x\n", "unsupported tag")
        self.assertLocated("a: !!set {x}\n", "unsupported tag")

    def test_merge_keys_are_refused(self) -> None:
        self.assertLocated("base: &b {x: 1}\nother:\n  <<: *b\n", "merge keys", line=3)

    def test_non_string_keys_are_refused(self) -> None:
        self.assertLocated("1: a\n", "keys must be strings")
        self.assertLocated("true: a\n", "keys must be strings")
        self.assertLocated("? [a]\n: b\n", "keys must be strings")

    def test_root_must_be_one_mapping(self) -> None:
        self.assertLocated("", "empty document")
        self.assertLocated("- a\n", "root must be a mapping")
        self.assertLocated("just text\n", "root must be a mapping")

    def test_aliases_resolve_and_recursion_is_refused(self) -> None:
        self.assertEqual(self.load("a: &x [1]\nb: *x\n").value, {"a": [1], "b": [1]})
        self.assertLocated("a: &x [*x]\n", "recursive alias")


class JsonAndTomlLoaderTests(unittest.TestCase):
    def test_json_shapes(self) -> None:
        self.assertEqual(d.load_json('{"name": "ünï"}', source="j"), {"name": "ünï"})
        self.assertIsNone(d.load_json("null", source="j"))
        for bad, message in (('{"a": 1, "a": 2}', "duplicate"), ('{"a": 1} x', "Extra data"),
                             ('{"a": NaN}', "NaN"), ('{"a": Infinity}', "Infinity"), ("{", "invalid JSON")):
            with self.subTest(bad=bad), self.assertRaisesRegex(d.DistributionError, message):
                d.load_json(bad, source="j")

    def test_toml_shapes(self) -> None:
        self.assertEqual(d.load_toml("", source="t"), {})
        self.assertEqual(d.load_toml("[a]\nb = 1\n", source="t"), {"a": {"b": 1}})
        for bad in ("a = 1\na = 2\n", "[a]\n[a]\n", "a = 1 x\n", "a = \n"):
            with self.subTest(bad=bad), self.assertRaisesRegex(d.DistributionError, "invalid TOML"):
                d.load_toml(bad, source="t")


class PolicyTests(unittest.TestCase):
    """`docs/deployment.yaml` field shapes; every invalid shape is a located
    error. Exceptions: absent `approved` and channels are false, and a null
    `crate` marks an unapproved, all-disabled entry."""

    def assertInvalid(self, text: str, message: str) -> d.DistributionError:
        with self.assertRaises(d.DistributionError) as caught:
            policy(text)
        self.assertIn(message, str(caught.exception))
        self.assertRegex(str(caught.exception), r"^deployment\.yaml:\d+:\d+: ")
        return caught.exception

    def test_the_real_policy_is_a_valid_all_disabled_control(self) -> None:
        loaded = d.load_policy(POLICY_PATH, WORKSPACE)
        self.assertGreater(len(loaded.entries), 60)
        self.assertEqual([name for name, entry in loaded.entries.items() if entry.approved or entry.channels], [])
        for package in d.ROSTER:
            self.assertIn(package, loaded.entries)
        self.assertEqual(loaded.raw, POLICY_PATH.read_bytes())

    def test_excluded_repository_packages_are_known_but_not_members(self) -> None:
        self.assertEqual(EXCLUDED, {"schematic-schema", "zed-dmls"})
        self.assertNotIn("zed-dmls", WORKSPACE.packages)
        policy(POLICY_TEXT)  # both have entries in the real policy

    def test_packages_shapes(self) -> None:
        self.assertInvalid("$schema: {}\n", "packages: is required")
        self.assertInvalid("packages:\n", "packages: must not be null")
        self.assertInvalid("packages: [a]\n", "packages: must be a mapping")
        self.assertEqual(policy("packages: {}\n").entries, {})

    def test_unknown_top_level_key_is_refused(self) -> None:
        self.assertInvalid(POLICY_TEXT + "extra: 1\n", "extra: unknown top-level key")

    def test_schema_and_types_are_optional_descriptions(self) -> None:
        # Their content is never interpreted: a description that reads like a
        # schema, or none at all, changes nothing.
        self.assertEqual(policy("$schema: 1\ntypes: [x]\npackages: {}\n").entries, {})
        self.assertEqual(policy("packages: {}\n").entries, {})

    def test_unknown_package_is_refused(self) -> None:
        self.assertInvalid(POLICY_TEXT.replace("    tabby:\n", "    tabbby:\n"), "packages.tabbby: is not a package")

    def test_entry_must_be_a_mapping(self) -> None:
        self.assertInvalid("packages:\n    tabby: true\n", "packages.tabby: must be a mapping")
        self.assertInvalid("packages:\n    tabby:\n", "packages.tabby: must not be null")

    def test_unknown_entry_field_is_refused(self) -> None:
        self.assertInvalid(edit_entry(POLICY_TEXT, "tabby", "snap", "false,\n            flatpak: false"), "packages.tabby.flatpak: unknown field")

    def test_crate_shapes(self) -> None:
        self.assertInvalid(edit_entry(POLICY_TEXT, "biscuit-hash-cli", "crate", None), "crate: is required")
        self.assertInvalid(edit_entry(POLICY_TEXT, "biscuit-hash-cli", "crate", "bh"), "must equal the package name")
        self.assertInvalid(edit_entry(POLICY_TEXT, "biscuit-hash-cli", "crate", "1"), "must be a string, found integer")
        self.assertInvalid(edit_entry(POLICY_TEXT, "biscuit-hash-cli", "crate", '""'), "must not be empty")
        nulled = edit_entry(POLICY_TEXT, "biscuit-hash-cli", "crate", "null")
        self.assertIsNone(policy(nulled).entries["biscuit-hash-cli"].crate)
        self.assertInvalid(approve(nulled, "biscuit-hash-cli"), "crate: is null, which only an unapproved entry may be")

    def test_approved_shapes(self) -> None:
        absent = edit_entry(POLICY_TEXT, "biscuit-hash-cli", "approved", None)
        self.assertFalse(policy(absent).entries["biscuit-hash-cli"].approved)
        self.assertTrue(policy(approve(POLICY_TEXT, "biscuit-hash-cli")).entries["biscuit-hash-cli"].approved)
        self.assertInvalid(edit_entry(POLICY_TEXT, "biscuit-hash-cli", "approved", "null"), "approved: must not be null")
        self.assertInvalid(edit_entry(POLICY_TEXT, "biscuit-hash-cli", "approved", ""), "approved: must not be null")
        self.assertInvalid(edit_entry(POLICY_TEXT, "biscuit-hash-cli", "approved", "'true'"), "must be true or false, found string")
        self.assertInvalid(edit_entry(POLICY_TEXT, "biscuit-hash-cli", "approved", "on"), "must be true or false, found string 'on'")
        self.assertInvalid(edit_entry(POLICY_TEXT, "biscuit-hash-cli", "approved", "1"), "found integer")
        self.assertInvalid(edit_entry(POLICY_TEXT, "biscuit-hash-cli", "approved", "[]"), "found list")

    def test_channel_shapes(self) -> None:
        approved = approve(POLICY_TEXT, "biscuit-hash-cli")
        entry = lambda text: policy(text).entries["biscuit-hash-cli"]  # noqa: E731
        self.assertEqual(entry(edit_entry(approved, "biscuit-hash-cli", "brew", None)).channels, {})
        self.assertEqual(entry(edit_entry(approved, "biscuit-hash-cli", "brew", "true")).channels, {"brew": "biscuit-hash-cli"})
        self.assertEqual(entry(edit_entry(approved, "biscuit-hash-cli", "brew", "bh")).channels, {"brew": "bh"})
        # A quoted "false" is a name, never a disabled channel.
        self.assertEqual(entry(edit_entry(approved, "biscuit-hash-cli", "brew", "'false'")).channels, {"brew": "false"})
        for value, message in (("null", "found null"), ("", "found null"), ("3", "found integer"),
                               ("[bh]", "found list"), ("'Bad Name'", "must match"), ('""', "must match")):
            with self.subTest(value=value):
                self.assertInvalid(edit_entry(approved, "biscuit-hash-cli", "brew", value), message)

    def test_every_channel_is_wired_to_the_shared_validator(self) -> None:
        approved = approve(POLICY_TEXT, "biscuit-hash-cli")
        for channel in d.CHANNELS:
            with self.subTest(channel=channel):
                self.assertEqual(policy(edit_entry(approved, "biscuit-hash-cli", channel, "true")).entries["biscuit-hash-cli"].channels, {channel: "biscuit-hash-cli"})
                self.assertInvalid(edit_entry(approved, "biscuit-hash-cli", channel, "7"), f"{channel}: must be true, false, or a name")

    def test_channel_on_an_unapproved_entry_is_refused(self) -> None:
        self.assertInvalid(edit_entry(POLICY_TEXT, "biscuit-hash-cli", "brew", "bh"), "brew: is enabled on an entry that is not approved")
        self.assertInvalid(edit_entry(POLICY_TEXT, "biscuit-hash-cli", "brew", "'false'"), "not approved")

    def test_channel_true_on_a_null_crate_is_refused(self) -> None:
        text = edit_entry(edit_entry(POLICY_TEXT, "tabby", "approved", "true"), "tabby", "brew", "true")
        self.assertInvalid(text, "crate is null")

    def test_duplicate_resolved_channel_names_are_refused(self) -> None:
        text = approve(approve(POLICY_TEXT, "biscuit-hash-cli", brew="bh"), "biscuit-file-cli", brew="bh")
        error = self.assertInvalid(text, "resolves to 'bh', already used by 'biscuit-file-cli'")
        self.assertIn("packages.biscuit-hash-cli.brew", str(error))
        # The same name in different channels is not a collision.
        policy(approve(approve(POLICY_TEXT, "biscuit-hash-cli", brew="bh"), "biscuit-file-cli", aur="bh"))


class DistBaseTests(unittest.TestCase):
    def test_the_checked_in_dist_configuration_is_the_reviewed_one(self) -> None:
        d.check_dist_base((REPO / "dist-workspace.toml").read_text(encoding="utf-8"))

    def test_a_changed_or_extra_setting_is_refused(self) -> None:
        text = (REPO / "dist-workspace.toml").read_text(encoding="utf-8")
        for edited, message in (
            (text.replace("precise-builds = true", "precise-builds = false"), "dist.precise-builds"),
            (text.replace('cargo-dist-version = "0.34.0"', 'cargo-dist-version = "0.33.1"'), "cargo-dist-version"),
            (text.replace("dist = false", 'dist = "false"'), "dist.dist"),
            (text.replace("[dist]\n", '[dist]\npublish-jobs = ["homebrew"]\n'), "dist.publish-jobs: is not a reviewed setting"),
            (text.replace('x86_64-unknown-linux-gnu = "2.35"', 'x86_64-unknown-linux-gnu = "2.34"'), "dist.min-glibc-version"),
            (text.replace('aarch64-unknown-linux-gnu = "2.35"\n', ""), "dist.min-glibc-version"),
            (text.replace('    "x86_64-unknown-linux-gnu",\n', ""), "dist.targets"),
        ):
            with self.subTest(message=message), self.assertRaisesRegex(d.DistributionError, re.escape(message)):
                d.check_dist_base(edited)


class WorkspaceTests(unittest.TestCase):
    def test_every_roster_package_ships_exactly_its_command(self) -> None:
        for package, command in d.ROSTER.items():
            with self.subTest(package=package):
                found = WORKSPACE.packages[package]
                ungated = [binary.name for binary in found.binaries if not binary.required_features]
                self.assertEqual(ungated, [command])
                self.assertEqual(found.repository, d.CANONICAL_URL)
                self.assertTrue(found.description and found.homepage)
                self.assertNotIn("dist", found.metadata or {})

    def test_worktree_helper_is_feature_gated(self) -> None:
        binaries = {binary.name: binary.required_features for binary in WORKSPACE.packages["git-wt-cli"].binaries}
        self.assertEqual(binaries, {"wt": (), "render_dirty_tree": ("terminal-tests",)})

    def test_metadata_shapes(self) -> None:
        document = json.loads(METADATA)
        package = next(p for p in document["packages"] if p["name"] == "biscuit-hash-cli")
        for key, value, message in (("repository", 3, "must be a string"), ("version", "1.2", "complete SemVer"),
                                    ("targets", [], "must not be empty"), ("metadata", [], "must be a mapping")):
            with self.subTest(key=key):
                edited = json.loads(METADATA)
                next(p for p in edited["packages"] if p["name"] == "biscuit-hash-cli")[key] = value
                with self.assertRaisesRegex(d.DistributionError, message):
                    d.load_workspace(json.dumps(edited), source="m")
        edited = json.loads(METADATA)
        del next(p for p in edited["packages"] if p["name"] == "biscuit-hash-cli")["repository"]
        with self.assertRaisesRegex(d.DistributionError, "repository: is required"):
            d.load_workspace(json.dumps(edited), source="m")
        self.assertIn("bh", [b.name for b in d.load_workspace(json.dumps(document), source="m").packages[package["name"]].binaries])


class SelectReleaseTests(unittest.TestCase):
    APPROVED = approve(POLICY_TEXT, "biscuit-hash-cli")

    def select(self, event: d.ReleaseFacts | None = None, tagged: str | None = None, current: str | None = None,
               workspace: d.Workspace = WORKSPACE) -> d.Preflight:
        tagged = self.APPROVED if tagged is None else tagged
        return d.select_release(event or facts(), d.Policies(policy(current if current is not None else tagged, "current", workspace), policy(tagged, "tagged", workspace)), workspace)

    def assertRefused(self, message: str, **kwargs) -> None:
        with self.assertRaises(d.DistributionError) as caught:
            self.select(**kwargs)
        self.assertIn(message, str(caught.exception))

    def test_a_one_entry_approval_selects_exactly_one_command(self) -> None:
        result = self.select()
        self.assertEqual(result.status, d.SELECTED)
        record = result.as_json()
        self.assertEqual(
            {k: record[k] for k in ("package", "command", "manifest_path", "version", "tag", "commit", "channels", "targets", "prerelease", "formula_update")},
            {"package": "biscuit-hash-cli", "command": "bh", "manifest_path": "biscuit-hash/cli/Cargo.toml",
             "version": "0.1.0", "tag": "biscuit-hash-cli-v0.1.0", "commit": TAG_SHA, "channels": {},
             "targets": list(d.TARGETS), "prerelease": False, "formula_update": False},
        )
        self.assertEqual(record["build_matrix"], [{"target": target} for target in d.TARGETS])
        self.assertEqual(record["proposal"], {"number": 17, "head_sha": TAG_SHA, "merge_sha": MERGE_SHA})
        self.assertEqual(record["tools"], {"dist": "0.34.0", "binstall": "1.26.2"})

    def test_every_roster_package_passes_its_manifest_checks_when_approved(self) -> None:
        for package, command in d.ROSTER.items():
            with self.subTest(package=package):
                version = WORKSPACE.packages[package].version
                result = self.select(facts(f"{package}-v{version}"), tagged=approve(POLICY_TEXT, package))
                self.assertEqual((result.status, result.record["command"]), (d.SELECTED, command))

    def test_the_real_policy_ignores_every_release(self) -> None:
        result = self.select(tagged=POLICY_TEXT)
        self.assertEqual(result.status, d.IGNORED)
        self.assertIn("not approved", result.reason)
        self.assertEqual(result.as_json()["build_matrix"], [])

    def test_library_integration_and_foreign_tags_are_ignored(self) -> None:
        for tag, reason in (("biscuit-hash-v0.1.0", "does not ship a binary"),
                            ("arcam-amp-integration-v0.1.0", "does not ship a binary"),
                            ("nothing-here-v1.0.0", "names no workspace package"),
                            ("v1.0.0", "names no workspace package")):
            with self.subTest(tag=tag):
                result = self.select(facts(tag, release=release(tag)))
                self.assertEqual((result.status, result.as_json()["build_matrix"]), (d.IGNORED, []))
                self.assertIn(reason, result.reason)

    def test_a_malformed_roster_tag_is_an_error(self) -> None:
        for tag in ("biscuit-hash-cli-v0.1", "biscuit-hash-cli-v", "biscuit-hash-cli-v01.0.0", "biscuit-hash-cli-vlatest"):
            with self.subTest(tag=tag):
                self.assertRefused("complete SemVer", event=facts(tag, release=release(tag)))

    def test_a_foreign_repository_is_ignored(self) -> None:
        self.assertEqual(self.select(facts(repository="someone/fork")).status, d.IGNORED)

    def test_current_approval_cannot_authorize_an_unapproved_tag(self) -> None:
        result = self.select(tagged=POLICY_TEXT, current=self.APPROVED)
        self.assertEqual(result.status, d.IGNORED)

    def test_revoked_approval_is_an_error(self) -> None:
        self.assertRefused("approval was revoked", tagged=self.APPROVED, current=POLICY_TEXT)

    def test_channels_must_be_enabled_in_both_policies(self) -> None:
        tagged = approve(POLICY_TEXT, "biscuit-hash-cli", brew="bh")
        record = self.select(tagged=tagged, current=self.APPROVED).record
        self.assertEqual((record["channels"], record["revoked_channels"]), ({}, ["brew"]))
        record = self.select(tagged=self.APPROVED, current=tagged).record
        self.assertEqual((record["channels"], record["revoked_channels"]), ({}, []))
        self.assertRefused("tagged policy names 'bh'", tagged=tagged, current=approve(POLICY_TEXT, "biscuit-hash-cli", brew="bhx"))

    def test_brew_is_selected_and_skips_the_formula_for_a_prerelease(self) -> None:
        tagged = approve(POLICY_TEXT, "biscuit-hash-cli", brew="bh")
        self.assertEqual(self.select(tagged=tagged).record["formula_update"], True)
        flagged = facts(release=release("biscuit-hash-cli-v0.1.0", prerelease=True))
        record = self.select(flagged, tagged=tagged).record
        self.assertEqual((record["prerelease"], record["formula_update"], record["channels"]), (True, False, {"brew": "bh"}))

    def test_a_semver_prerelease_tag_is_distributed_under_its_exact_version(self) -> None:
        workspace = with_package("biscuit-hash-cli", version="0.2.0-rc.1")
        tag = "biscuit-hash-cli-v0.2.0-rc.1"
        record = self.select(facts(tag, release=release(tag)), tagged=approve(POLICY_TEXT, "biscuit-hash-cli", brew="bh"), workspace=workspace).record
        self.assertEqual((record["version"], record["prerelease"], record["formula_update"]), ("0.2.0-rc.1", True, False))

    def test_unsupported_channels_are_refused_not_reported(self) -> None:
        for channel in ("install", "apt", "npm"):
            with self.subTest(channel=channel):
                self.assertRefused(f"{channel}: is enabled, but this workflow publishes only", tagged=approve(POLICY_TEXT, "biscuit-hash-cli", **{channel: "true"}))

    def test_binstall_channel_requires_the_package_name(self) -> None:
        self.assertRefused("installs by package name", tagged=approve(POLICY_TEXT, "biscuit-hash-cli", binstall="bh"))

    def test_binstall_channel_and_manifest_must_agree(self) -> None:
        self.assertRefused("disabled-strategies: must be ['quick-install', 'compile'] while the binstall channel is enabled",
                           tagged=approve(POLICY_TEXT, "biscuit-hash-cli", binstall="true"))
        metadata = {"binstall": {**d.BINSTALL_URLS, "disabled-strategies": list(d.BINSTALL_ENABLED_STRATEGIES)}}
        enabled = with_package("biscuit-hash-cli", metadata=metadata)
        self.assertEqual(self.select(tagged=approve(POLICY_TEXT, "biscuit-hash-cli", binstall="true"), workspace=enabled).record["channels"], {"binstall": "biscuit-hash-cli"})
        self.assertRaisesRegex(d.DistributionError, "while the binstall channel is disabled", self.select, workspace=enabled)
        for key in d.BINSTALL_URLS:
            with self.subTest(key=key):
                broken = with_package("biscuit-hash-cli", metadata={"binstall": {**metadata["binstall"], key: "x"}})
                self.assertRaisesRegex(d.DistributionError, f"binstall.{key}: must be", self.select, workspace=broken)
        self.assertRaisesRegex(d.DistributionError, "binstall: is required", self.select, workspace=with_package("biscuit-hash-cli", metadata=None))

    def test_a_checked_in_dist_opt_in_anywhere_is_refused(self) -> None:
        workspace = with_package("tabby" if "tabby" in WORKSPACE.packages else "biscuit-file-cli", metadata={"dist": {"dist": True}})
        self.assertRaisesRegex(d.DistributionError, "package.metadata.dist: is checked in", self.select, workspace=workspace)

    def test_manifest_checks(self) -> None:
        cases = (
            (dict(repository="https://github.com/yankeeinlondon/dockhand"), "package.repository: must be"),
            (dict(repository=None), "package.repository: must be"),
            (dict(description=None), "package.description: is required"),
            (dict(homepage=None), "package.homepage: is required"),
            (dict(binaries=(d.Binary("other", ()),)), "declares no 'bh' binary"),
            (dict(binaries=(d.Binary("bh", ("extra",)),)), "must not require features"),
            (dict(binaries=(d.Binary("bh", ()), d.Binary("helper", ()))), "[[bin]] helper: is built by default"),
            (dict(version="0.1.1"), "package.version: is '0.1.1' at the tagged commit"),
        )
        for change, message in cases:
            with self.subTest(change=change):
                with self.assertRaises(d.DistributionError) as caught:
                    self.select(workspace=with_package("biscuit-hash-cli", **change))
                self.assertIn(message, str(caught.exception))
        gated = (d.Binary("bh", ()), d.Binary("helper", ("test-fixtures",)))
        self.assertEqual(self.select(workspace=with_package("biscuit-hash-cli", binaries=gated)).status, d.SELECTED)

    def test_release_and_revision_checks(self) -> None:
        cases = (
            (dict(release=None), "no GitHub Release exists"),
            (dict(release=release("biscuit-hash-cli-v0.1.0", draft=True)), "is a draft"),
            (dict(release=release("other-v1.0.0")), "the Release names tag"),
            (dict(tag_commit=None), "does not exist"),
            (dict(checkout_commit="3" * 40), "but the checkout is"),
            (dict(proposal=None), "no merged release proposal"),
            (dict(proposal=d.ProposalProof(17, "4" * 40, MERGE_SHA)), "release proposal #17 ended at"),
        )
        for change, message in cases:
            with self.subTest(change=change):
                self.assertRefused(message, event=facts(**change))

    def test_missing_release_app_is_pending_setup(self) -> None:
        result = self.select(facts(proposal=d.PendingSetup("RELEASE_APP_BOT_LOGIN", "configure it")))
        self.assertEqual((result.status, result.as_json()["build_matrix"]), (d.PENDING_SETUP, []))
        self.assertIn("RELEASE_APP_BOT_LOGIN", result.reason)

    def test_manual_recovery_runs_the_same_checks(self) -> None:
        self.assertEqual(self.select(facts(mode="recovery")).record["mode"], "recovery")
        self.assertRefused("is a draft", event=facts(mode="recovery", release=release("biscuit-hash-cli-v0.1.0", draft=True)))
        self.assertRefused("unknown mode", event=facts(mode="schedule"))


class FakeForge(d.Forge):
    def __init__(self, releases=None, tags=None, proposals=None) -> None:
        self.releases, self.tags, self.proposals = releases or {}, tags or {}, proposals or {}
        self.calls: list[str] = []

    def release(self, tag):
        self.calls.append(f"release {tag}")
        return self.releases.get(tag)

    def tag_commit(self, tag):
        self.calls.append(f"tag {tag}")
        return self.tags.get(tag)

    def proposal(self, commit):
        self.calls.append(f"proposal {commit}")
        return self.proposals.get(commit)


def release_event(tag: str, *, action: str = "published", repository: str = d.CANONICAL_REPOSITORY) -> dict:
    return {
        "action": action,
        "release": {"id": 42, "tag_name": tag, "draft": False, "prerelease": False,
                    "html_url": f"{d.CANONICAL_URL}/releases/tag/{tag}", "target_commitish": "main"},
        "repository": {"full_name": repository},
    }


class GatherFactsTests(unittest.TestCase):
    TAG = "biscuit-hash-cli-v0.1.0"

    def forge(self) -> FakeForge:
        return FakeForge({self.TAG: release(self.TAG)}, {self.TAG: TAG_SHA}, {TAG_SHA: d.ProposalProof(17, TAG_SHA, MERGE_SHA)})

    def gather(self, event_name: str, payload, forge: d.Forge) -> d.ReleaseFacts | d.Preflight:
        return d.gather_facts(event_name, payload, source="event.json", repository=d.CANONICAL_REPOSITORY,
                              forge=forge, checkout_commit=TAG_SHA, tagged=WORKSPACE)

    def test_a_published_release_event_gathers_every_fact(self) -> None:
        gathered = self.gather("release", release_event(self.TAG), self.forge())
        self.assertEqual(gathered, facts())

    def test_manual_recovery_reads_the_release_from_the_forge(self) -> None:
        gathered = self.gather("workflow_dispatch", {"inputs": {"tag": self.TAG}}, self.forge())
        self.assertEqual(gathered, facts(mode="recovery"))

    def test_ignored_events_never_reach_the_forge(self) -> None:
        for name, payload in (("release", release_event(self.TAG, action="created")),
                              ("pull_request", {}),
                              ("release", release_event("biscuit-hash-v0.1.0"))):
            with self.subTest(name=name):
                forge = FakeForge()
                self.assertEqual(self.gather(name, payload, forge).status, d.IGNORED)
                self.assertEqual(forge.calls, [])

    def test_event_shapes_are_located(self) -> None:
        bad = release_event(self.TAG)
        bad["release"]["draft"] = "false"
        with self.assertRaisesRegex(d.DistributionError, r"event.json: release.draft: must be true or false"):
            self.gather("release", bad, self.forge())
        with self.assertRaisesRegex(d.DistributionError, r"inputs.tag: is required"):
            self.gather("workflow_dispatch", {"inputs": {}}, self.forge())
        with self.assertRaisesRegex(d.DistributionError, "repository.full_name"):
            self.gather("release", release_event(self.TAG, repository="other/repo"), self.forge())

    def test_the_api_must_agree_with_the_event(self) -> None:
        forge = self.forge()
        forge.releases[self.TAG] = d.ReleaseState(self.TAG, 99, False, False, "u")
        with self.assertRaisesRegex(d.DistributionError, "not the repository's Release"):
            self.gather("release", release_event(self.TAG), forge)


class StubbedGitHubForge(d.GitHubForge):
    def __init__(self, responses: dict, bot_login: str | None = "release-app[bot]") -> None:
        super().__init__(repository=d.CANONICAL_REPOSITORY, token=None, bot_login=bot_login, repo=REPO)
        self.responses = responses

    def _get(self, path: str):
        return self.responses.get(path, (404, None))


def proposal_pull(**changes) -> dict:
    pull = {
        "number": 17, "merged": True, "merge_commit_sha": MERGE_SHA,
        "labels": [{"name": "release"}],
        "base": {"ref": "main", "repo": {"full_name": d.CANONICAL_REPOSITORY}},
        "head": {"ref": "release-plz-2026-11-09", "sha": TAG_SHA, "repo": {"full_name": d.CANONICAL_REPOSITORY}},
        "user": {"login": "release-app[bot]", "type": "Bot"},
    }
    pull.update(changes)
    return pull


class GitHubProposalProofTests(unittest.TestCase):
    """The proof reuses the release path's own proposal identity checks."""

    LIST = f"/repos/{d.CANONICAL_REPOSITORY}/commits/{TAG_SHA}/pulls"
    PULL = f"/repos/{d.CANONICAL_REPOSITORY}/pulls/17"

    def test_the_merged_app_proposal_ending_at_the_tag_is_the_proof(self) -> None:
        forge = StubbedGitHubForge({self.LIST: (200, [{"number": 17}]), self.PULL: (200, proposal_pull())})
        self.assertEqual(forge.proposal(TAG_SHA), d.ProposalProof(17, TAG_SHA, MERGE_SHA))

    def test_no_proposal_or_another_head_is_no_proof(self) -> None:
        self.assertIsNone(StubbedGitHubForge({self.LIST: (200, [])}).proposal(TAG_SHA))
        other_head = proposal_pull(head={"ref": "release-plz-x", "sha": "5" * 40, "repo": {"full_name": d.CANONICAL_REPOSITORY}})
        self.assertIsNone(StubbedGitHubForge({self.LIST: (200, [{"number": 17}]), self.PULL: (200, other_head)}).proposal(TAG_SHA))

    def test_a_proposal_from_another_author_is_refused(self) -> None:
        forged = proposal_pull(user={"login": "someone", "type": "Bot"})
        forge = StubbedGitHubForge({self.LIST: (200, [{"number": 17}]), self.PULL: (200, forged)})
        with self.assertRaisesRegex(d.DistributionError, "pull request #17: pull_request.user.login"):
            forge.proposal(TAG_SHA)

    def test_api_failures_are_errors_not_absence(self) -> None:
        with self.assertRaisesRegex(d.DistributionError, "unexpected HTTP 500"):
            StubbedGitHubForge({self.LIST: (500, None)}).proposal(TAG_SHA)

    def test_missing_release_app_identity_is_pending_setup(self) -> None:
        pending = StubbedGitHubForge({}, bot_login=None).proposal(TAG_SHA)
        self.assertIsInstance(pending, d.PendingSetup)
        self.assertEqual(pending.setting, "RELEASE_APP_BOT_LOGIN")

    def test_release_lookup_distinguishes_absence_from_failure(self) -> None:
        path = f"/repos/{d.CANONICAL_REPOSITORY}/releases/tags/biscuit-hash-cli-v0.1.0"
        self.assertIsNone(StubbedGitHubForge({}).release("biscuit-hash-cli-v0.1.0"))
        with self.assertRaisesRegex(d.DistributionError, "unexpected HTTP 502"):
            StubbedGitHubForge({path: (502, None)}).release("biscuit-hash-cli-v0.1.0")
        body = release_event("biscuit-hash-cli-v0.1.0")["release"]
        self.assertEqual(StubbedGitHubForge({path: (200, body)}).release("biscuit-hash-cli-v0.1.0"), release("biscuit-hash-cli-v0.1.0"))


class SelectionAndConfigTests(unittest.TestCase):
    def selected(self, **channels: str) -> dict:
        tagged = approve(POLICY_TEXT, "darkmatter-cli", **channels)
        result = d.select_release(facts("darkmatter-cli-v0.1.0"), policies(tagged), WORKSPACE)
        return result.as_json()

    def test_config_names_only_the_selected_package_and_command(self) -> None:
        selection = d.Selection.from_json(self.selected(brew="md"), source="s")
        config = d.render_dist_config(selection)
        self.assertEqual(config.manifest_path, "darkmatter/cli/Cargo.toml")
        overlay = d.load_toml(config.overlay, source="overlay")
        self.assertEqual(overlay, {"package": {"metadata": {"dist": {
            "dist": True, "formula": "md", "tap": d.HOMEBREW_TAP, "installers": ["shell", "homebrew"],
            "include": [".distribution/SOURCE.txt", ".distribution/THIRD-PARTY-NOTICES.md"],
            "binaries": {"*": ["md"]}}}}})
        # dist's installer reads the glibc floor from the workspace only, so a
        # package-level copy would be a silent no-op.
        self.assertNotIn("glibc", config.overlay)
        self.assertNotIn("darkmatter-argv-recorder", config.overlay)
        plain = d.load_toml(d.render_dist_config(d.Selection.from_json(self.selected(), source="s")).overlay, source="o")
        self.assertEqual(plain["package"]["metadata"]["dist"]["installers"], ["shell"])

    def test_only_a_selected_record_can_be_configured(self) -> None:
        record = self.selected()
        for change, message in ((dict(status="ignored"), "only a selected preflight result"),
                                (dict(command="darkmatter-argv-recorder"), "shipping command"),
                                (dict(package="biscuit-hash-cli"), "shipping command"),
                                (dict(targets=["x86_64-pc-windows-msvc"]), "targets: must be"),
                                (dict(channels={"apt": "md"}), "not a supported channel"),
                                (dict(commit="abc"), "40-digit")):
            with self.subTest(change=change), self.assertRaisesRegex(d.DistributionError, message):
                d.Selection.from_json({**record, **change}, source="s")
        with self.assertRaises(TypeError):
            d.render_dist_config(record)  # type: ignore[arg-type]


# Real `dist plan --output-format=json` output from pinned dist 0.34.0, run in a
# staged copy with the generated opt-in (bh plain; md with the brew channel).
PLAN_BH = REPO / "scripts/ci/fixtures/distribution/dist-plan-biscuit-hash-cli.json"
PLAN_MD_BREW = REPO / "scripts/ci/fixtures/distribution/dist-plan-darkmatter-cli-brew.json"
# A package whose directory holds a release-plz changelog, which dist adds.
PLAN_SNIFF_BREW = REPO / "scripts/ci/fixtures/distribution/dist-plan-sniff-detection-cli-brew.json"


def selection_for(package: str, **channels: str) -> d.Selection:
    tag = f"{package}-v{WORKSPACE.packages[package].version}"
    result = d.select_release(facts(tag), policies(approve(POLICY_TEXT, package, **channels)), WORKSPACE)
    return d.Selection.from_json(result.as_json(), source="s")


def edited(plan: dict, edit) -> dict:
    copy = json.loads(json.dumps(plan))
    edit(copy)
    return copy


class ValidatePlanTests(unittest.TestCase):
    """Each case is the real plan with one edit; the unedited plan is the control."""

    def setUp(self) -> None:
        self.bh = d.load_json(PLAN_BH.read_bytes(), source=PLAN_BH.name)
        self.md = d.load_json(PLAN_MD_BREW.read_bytes(), source=PLAN_MD_BREW.name)
        self.bh_selection = selection_for("biscuit-hash-cli")
        self.md_selection = selection_for("darkmatter-cli", brew="true")

    def test_the_real_plans_are_valid_controls(self) -> None:
        d.validate_plan(self.bh_selection, self.bh)
        d.validate_plan(self.md_selection, self.md)

    def test_the_real_plans_ship_one_command_with_notices(self) -> None:
        # Rule 6: `binaries` replaces the packageables, so darkmatter-cli's
        # gated helper binaries appear nowhere in the real plan.
        for plan, command in ((self.bh, "bh"), (self.md, "md")):
            for name, artifact in plan["artifacts"].items():
                if artifact["kind"] == "executable-zip":
                    self.assertEqual(sorted(a["name"] for a in artifact["assets"]),
                                     sorted(["LICENSE", "README.md", "SOURCE.txt", "THIRD-PARTY-NOTICES.md", command]), name)
        self.assertNotIn("recorder", json.dumps(self.md))

    def test_a_package_changelog_is_the_only_extra_document_accepted(self) -> None:
        sniff = d.load_json(PLAN_SNIFF_BREW.read_bytes(), source=PLAN_SNIFF_BREW.name)
        selection = selection_for("sniff-detection-cli", brew="sniff")
        d.validate_plan(selection, sniff)
        archive = "sniff-detection-cli-x86_64-unknown-linux-gnu.tar.xz"
        self.assertIn({"kind": "changelog", "name": "CHANGELOG.md"},
                      [{k: a[k] for k in ("kind", "name")} for a in sniff["artifacts"][archive]["assets"]])

        def rename(name: str, kind: str = "changelog"):
            def edit(plan: dict) -> None:
                for asset in plan["artifacts"][archive]["assets"]:
                    if asset["kind"] == "changelog":
                        asset.update(kind=kind, name=name, path=name)
            return edit

        for edit, message in ((rename("CHANGES.md"), "unexpected changelog asset 'CHANGES.md'"),
                              (rename("CHANGELOG.md", kind="unknown"), "unexpected unknown asset 'CHANGELOG.md'")):
            with self.subTest(message=message), self.assertRaisesRegex(d.DistributionError, re.escape(message)):
                d.validate_plan(selection, edited(sniff, edit))

    def test_channel_correspondence(self) -> None:
        with self.assertRaisesRegex(d.DistributionError, r"unexpected \['darkmatter-cli\.rb'\]"):
            d.validate_plan(selection_for("darkmatter-cli"), self.md)
        with self.assertRaisesRegex(d.DistributionError, r"missing \['biscuit-hash-cli\.rb'\]"):
            d.validate_plan(selection_for("biscuit-hash-cli", brew="true"), self.bh)

    def test_one_edit_each_is_refused(self) -> None:
        archive = "biscuit-hash-cli-x86_64-unknown-linux-gnu.tar.xz"
        installer = "biscuit-hash-cli-installer.sh"
        github = lambda p: p["releases"][0]["hosting"]["github"]  # noqa: E731
        assets = lambda p: p["artifacts"][archive]["assets"]  # noqa: E731
        cases = (
            (lambda p: p.update(dist_version="0.33.1"), "dist_version: must be '0.34.0'"),
            (lambda p: p.update(announcement_tag="v0.1.0"), "announcement_tag: must be"),
            (lambda p: p.update(announcement_is_prerelease=True), "announcement_is_prerelease: must be false"),
            (lambda p: p.pop("announcement_is_prerelease"), "announcement_is_prerelease: is required"),
            (lambda p: p.update(announcement_is_prerelease="false"), "announcement_is_prerelease: must be true or false"),
            (lambda p: p["releases"].append(dict(p["releases"][0], app_name="darkmatter-cli")), "exactly one package, found 2"),
            (lambda p: p.update(releases=[]), "releases: must not be empty"),
            (lambda p: p.update(releases={}), "releases: must be a list"),
            (lambda p: p["releases"][0].update(app_name="darkmatter-cli"), "app_name: must be"),
            (lambda p: p["releases"][0].update(app_version="0.1.1"), "app_version: must be"),
            (lambda p: p["releases"][0].update(app_version=None), "app_version: must not be null"),
            (lambda p: github(p).update(artifact_base_url="https://example.com"), "artifact_base_url: must be"),
            (lambda p: github(p).update(artifact_download_path="/yankeeinlondon/rusty-biscuit/releases/latest/download"), "artifact_download_path: must be"),
            (lambda p: github(p).update(owner="someone"), "owner: must be"),
            (lambda p: p["releases"][0]["hosting"].update(axodotdev={}), "must host on GitHub only"),
            (lambda p: p["releases"][0]["artifacts"].append("biscuit-hash-cli-installer.ps1"), "unexpected ['biscuit-hash-cli-installer.ps1']"),
            (lambda p: p["releases"][0]["artifacts"].remove(archive), f"missing ['{archive}']"),
            (lambda p: p["releases"][0]["artifacts"].append(archive), "listed more than once: ['biscuit-hash-cli-x86_64-unknown-linux-gnu.tar.xz']"),
            (lambda p: p["artifacts"].pop(archive), f"names '{archive}', which artifacts does not define"),
            (lambda p: p["artifacts"][archive].update(kind="executable-msi"), "kind: must be 'executable-zip'"),
            (lambda p: p["artifacts"][archive].update(target_triples=["x86_64-unknown-linux-musl"]), "target_triples: must be"),
            (lambda p: p["artifacts"][archive].update(target_triples=["x86_64-unknown-linux-gnu", 7]), "target_triples[1]: must be a string"),
            (lambda p: p["artifacts"][archive].pop("checksum"), "checksum: is required"),
            (lambda p: assets(p).append({"name": "darkmatter-argv-recorder", "path": "darkmatter-argv-recorder", "kind": "executable"}), "must ship exactly ['bh']"),
            (lambda p: [a for a in assets(p) if a["kind"] == "executable"][0].update(name="bf", path="bf"), "must ship exactly ['bh']"),
            (lambda p: p["artifacts"][archive].update(assets=[a for a in assets(p) if a["kind"] != "license"]), "must include the root LICENSE"),
            (lambda p: p["artifacts"][archive].update(assets=[a for a in assets(p) if a["name"] != "SOURCE.txt"]), "must include the generated"),
            (lambda p: assets(p).append({"name": "fixture.json", "path": "fixture.json", "kind": "unknown"}), "unexpected unknown asset 'fixture.json'"),
            (lambda p: assets(p).append("README.md"), "assets[5]: must be a mapping"),
            (lambda p: p["artifacts"][installer]["target_triples"].pop(), "target_triples: must cover exactly"),
            (lambda p: p["artifacts"][installer].update(kind="executable-zip"), "kind: must be 'installer'"),
        )
        for edit, message in cases:
            with self.subTest(message=message), self.assertRaisesRegex(d.DistributionError, re.escape(message)):
                d.validate_plan(self.bh_selection, edited(self.bh, edit))

    def test_a_prerelease_plan_must_say_so(self) -> None:
        record = self.bh_selection.record | {"version": "0.1.0-rc.1", "tag": "biscuit-hash-cli-v0.1.0-rc.1"}
        selection = d.Selection.from_json(record, source="s")
        plan = edited(self.bh, lambda p: p.update(announcement_tag=selection.tag, announcement_is_prerelease=True))
        plan["releases"][0]["app_version"] = "0.1.0-rc.1"
        plan["releases"][0]["hosting"]["github"]["artifact_download_path"] = f"/{d.CANONICAL_REPOSITORY}/releases/download/{selection.tag}"
        d.validate_plan(selection, plan)
        plan["announcement_is_prerelease"] = False
        with self.assertRaisesRegex(d.DistributionError, "announcement_is_prerelease: must be true"):
            d.validate_plan(selection, plan)

    def test_a_non_mapping_plan_is_refused(self) -> None:
        for plan in ([], None, "plan"):
            with self.subTest(plan=plan), self.assertRaises(d.DistributionError):
                d.validate_plan(self.bh_selection, plan)


def fixture_git(root: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-c", "user.name=t", "-c", "user.email=t@example.invalid",
         "-c", "commit.gpgsign=false", *args],
        cwd=root, check=True, capture_output=True, text=True,
        env={**os.environ, "GIT_TERMINAL_PROMPT": "0"},
    ).stdout.strip()


MANIFEST = "biscuit-hash/cli/Cargo.toml"
MANIFEST_TEXT = '[package]\nname = "biscuit-hash-cli"\nversion = "0.1.0"\n\n[[bin]]\nname = "bh"\npath = "src/main.rs"\n'


class StageTests(unittest.TestCase):
    """A small committed repository stands in for the tagged checkout."""

    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        base = Path(self.tmp.name)
        self.source = base / "checkout"
        self.dest = base / "staging"
        self.notices = base / "notices.md"
        self.notices.write_text("# Third-party notices\n\nserde: MIT OR Apache-2.0\n", encoding="utf-8")
        files = {MANIFEST: MANIFEST_TEXT, "biscuit-hash/cli/src/main.rs": "fn main() {}\n",
                 "Cargo.lock": "version = 4\n", "LICENSE": "AGPL\n", "scripts/run.sh": "#!/bin/sh\n",
                 "darkmatter/tests/fixtures/data.bin": "\x00\x01"}
        for relative, text in files.items():
            (self.source / relative).parent.mkdir(parents=True, exist_ok=True)
            (self.source / relative).write_text(text, encoding="utf-8")
        (self.source / "scripts/run.sh").chmod(0o755)
        (self.source / "AGENTS.md").symlink_to("LICENSE")
        fixture_git(self.source, "init", "-q")
        fixture_git(self.source, "add", "-A")
        fixture_git(self.source, "commit", "-q", "-m", "tagged")
        (self.source / "target").mkdir()
        (self.source / "target/untracked.o").write_bytes(b"build output")
        self.commit = fixture_git(self.source, "rev-parse", "HEAD")
        self.selection = d.Selection.from_json(selection_for("biscuit-hash-cli").record | {"commit": self.commit}, source="s")

    def snapshot(self) -> dict[str, bytes]:
        return {p.relative_to(self.source).as_posix(): p.read_bytes() for p in self.source.rglob("*")
                if p.is_file() and ".git" not in p.relative_to(self.source).parts}

    def test_stage_copies_the_checkout_and_changes_only_the_opt_in(self) -> None:
        before = self.snapshot()
        staged = d.stage_distribution(self.selection, source=self.source, dest=self.dest, notices=self.notices)
        self.assertEqual(self.snapshot(), before, "the checkout must stay byte-clean")
        self.assertEqual(fixture_git(self.source, "status", "--porcelain"), "?? target/")
        self.assertEqual(staged.tracked_files, 7)
        self.assertEqual(staged.generated, ["biscuit-hash/cli/.distribution/SOURCE.txt", "biscuit-hash/cli/.distribution/THIRD-PARTY-NOTICES.md"])
        self.assertFalse((self.dest / "target").exists(), "untracked build output is never staged")
        self.assertFalse((self.dest / ".git").exists())
        self.assertEqual((self.dest / "Cargo.lock").read_bytes(), b"version = 4\n")
        self.assertTrue(os.access(self.dest / "scripts/run.sh", os.X_OK))
        self.assertEqual(os.readlink(self.dest / "AGENTS.md"), "LICENSE")
        manifest = (self.dest / MANIFEST).read_text(encoding="utf-8")
        self.assertTrue(manifest.startswith(MANIFEST_TEXT))
        # Round trip: the staged manifest parses, and its dist table is the rendered overlay.
        parsed = d.load_toml(manifest, source="staged")
        overlay = d.load_toml(d.render_dist_config(self.selection).overlay, source="overlay")
        self.assertEqual(parsed["package"]["metadata"], overlay["package"]["metadata"])
        self.assertEqual(parsed["bin"], [{"name": "bh", "path": "src/main.rs"}])
        source_txt = (self.dest / "biscuit-hash/cli/.distribution/SOURCE.txt").read_text(encoding="utf-8")
        self.assertIn(f"{d.CANONICAL_URL}/tree/biscuit-hash-cli-v0.1.0/biscuit-hash/cli", source_txt)
        self.assertIn(f"{d.CANONICAL_URL}/commit/{self.commit}", source_txt)
        self.assertEqual((self.dest / "biscuit-hash/cli/.distribution/THIRD-PARTY-NOTICES.md").read_bytes(), self.notices.read_bytes())
        self.assertIn("+[package.metadata.dist.binaries]", staged.overlay_diff)
        self.assertNotIn("\n-", staged.overlay_diff.split("@@", 2)[-1], "the diff only adds lines")

    def test_a_tampered_staging_tree_is_refused(self) -> None:
        d.stage_distribution(self.selection, source=self.source, dest=self.dest, notices=self.notices)
        paths = d.tracked_paths(self.source, self.commit)
        overlay = d.render_dist_config(self.selection).overlay
        generated = {f"biscuit-hash/cli/.distribution/{n}": (self.dest / "biscuit-hash/cli/.distribution" / n).read_bytes() for n in d.GENERATED_FILES}
        verify = lambda: d.verify_staging(self.source, self.dest, paths, MANIFEST, overlay, generated)  # noqa: E731
        verify()
        lock = self.dest / "Cargo.lock"
        lock.write_bytes(b"version = 3\n")
        with self.assertRaisesRegex(d.DistributionError, "staging Cargo.lock: differs from the tagged checkout"):
            verify()
        lock.write_bytes(b"version = 4\n")
        (self.dest / "biscuit-hash/cli/build.rs").write_text("fn main() {}\n")
        with self.assertRaisesRegex(d.DistributionError, r"extra \['biscuit-hash/cli/build.rs'\]"):
            verify()
        (self.dest / "biscuit-hash/cli/build.rs").unlink()
        (self.dest / MANIFEST).write_text(MANIFEST_TEXT + overlay.replace('"bh"', '"bh", "bf"'))
        with self.assertRaisesRegex(d.DistributionError, f"staging {MANIFEST}: differs"):
            verify()

    def test_refusals_before_anything_is_staged(self) -> None:
        other = d.Selection.from_json(self.selection.record | {"commit": "f" * 40}, source="s")
        with self.assertRaisesRegex(d.DistributionError, "not the selected commit"):
            d.stage_distribution(other, source=self.source, dest=self.dest, notices=self.notices)
        self.assertFalse(self.dest.exists())
        empty = self.notices.with_name("empty.md")
        empty.write_text(" \n")
        with self.assertRaisesRegex(d.DistributionError, "is empty; the archive must carry third-party notices"):
            d.stage_distribution(self.selection, source=self.source, dest=self.dest, notices=empty)
        with self.assertRaisesRegex(d.DistributionError, "cannot read the third-party notices"):
            d.stage_distribution(self.selection, source=self.source, dest=self.dest, notices=empty.with_name("absent.md"))
        self.dest.mkdir()
        (self.dest / "leftover").write_text("x")
        with self.assertRaisesRegex(d.DistributionError, "must be absent or empty"):
            d.stage_distribution(self.selection, source=self.source, dest=self.dest, notices=self.notices)
        with self.assertRaises(TypeError):
            d.stage_distribution(self.selection.record, source=self.source, dest=self.dest, notices=self.notices)  # type: ignore[arg-type]

    def test_a_dirty_or_reserved_checkout_is_refused(self) -> None:
        (self.source / "Cargo.lock").write_text("version = 3\n")
        with self.assertRaisesRegex(d.DistributionError, r"has tracked modifications: \['Cargo.lock'\]"):
            d.stage_distribution(self.selection, source=self.source, dest=self.dest, notices=self.notices)
        fixture_git(self.source, "checkout", "--", "Cargo.lock")
        reserved = self.source / "biscuit-hash/cli/.distribution/SOURCE.txt"
        reserved.parent.mkdir()
        reserved.write_text("checked in\n")
        fixture_git(self.source, "add", "-A", "biscuit-hash")
        fixture_git(self.source, "commit", "-q", "-m", "reserved")
        moved = d.Selection.from_json(self.selection.record | {"commit": fixture_git(self.source, "rev-parse", "HEAD")}, source="s")
        with self.assertRaisesRegex(d.DistributionError, "is reserved for generated files"):
            d.stage_distribution(moved, source=self.source, dest=self.dest, notices=self.notices)

    def test_a_manifest_without_a_final_newline_is_refused(self) -> None:
        (self.source / MANIFEST).write_text(MANIFEST_TEXT.rstrip("\n"))
        fixture_git(self.source, "commit", "-q", "-am", "no newline")
        moved = d.Selection.from_json(self.selection.record | {"commit": fixture_git(self.source, "rev-parse", "HEAD")}, source="s")
        with self.assertRaisesRegex(d.DistributionError, "must end with a newline"):
            d.stage_distribution(moved, source=self.source, dest=self.dest, notices=self.notices)


def sum_line(name: str, digit: str) -> str:
    return f"{digit * 64} *{name}\n"


def build_env(target: str, rustc: str = "rustc 1.98.1 (abc 2026-08-05)") -> dict:
    image = {"container": "ubuntu:22.04@sha256:" + "0" * 64} if "linux" in target else {"runner": "macos-15", "xcode": "Xcode 16.4"}
    return {"target": target, "rustc": rustc, "image": image}


class ProvenanceTests(unittest.TestCase):
    """Assets named by the real bh plan; checksum values are dist's, never computed here."""

    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.assets = Path(self.tmp.name)
        self.plan = d.load_json(PLAN_BH.read_bytes(), source=PLAN_BH.name)
        self.selection = selection_for("biscuit-hash-cli")
        lines = []
        names = sorted(n for n in d.expected_release_assets(self.selection) if n != "sha256.sum" and not n.endswith(".sha256"))
        for index, name in enumerate(names):
            (self.assets / name).write_bytes(b"x" * (index + 1))
            if name.endswith(".tar.xz"):
                digit = "0123456789"[index % 10]
                lines.append(sum_line(name, digit))
                (self.assets / f"{name}.sha256").write_text(sum_line(name, digit))
        lines.append(sum_line("biscuit-hash-cli-installer.sh", "a"))
        (self.assets / "sha256.sum").write_text("".join(lines))
        self.environments = [build_env(target) for target in d.TARGETS]

    def test_provenance_is_deterministic_and_complete(self) -> None:
        first = d.build_provenance(self.selection, self.plan, self.assets, self.environments)
        second = d.build_provenance(self.selection, self.plan, self.assets, list(reversed(self.environments)))
        self.assertEqual(json.dumps(first, sort_keys=True), json.dumps(second, sort_keys=True))
        self.assertEqual((first["tag"], first["commit"], first["tools"]), ("biscuit-hash-cli-v0.1.0", TAG_SHA, {"dist": "0.34.0", "rustc": "rustc 1.98.1 (abc 2026-08-05)"}))
        self.assertEqual([t["target"] for t in first["targets"]], list(d.TARGETS))
        names = [a["name"] for a in first["assets"]]
        self.assertEqual(names, sorted(d.expected_release_assets(self.selection)))
        by_name = {a["name"]: a for a in first["assets"]}
        archive = "biscuit-hash-cli-x86_64-apple-darwin.tar.xz"
        self.assertEqual(by_name[archive]["size"], (self.assets / archive).stat().st_size)
        self.assertEqual(by_name[archive]["sha256"], (self.assets / f"{archive}.sha256").read_text().split()[0])
        self.assertEqual(by_name["biscuit-hash-cli-installer.sh"]["sha256"], "a" * 64)
        self.assertIsNone(by_name["sha256.sum"]["sha256"])
        # No run URL, run ID, or time: those stay in the workflow artifacts.
        self.assertEqual(set(first), {"schema_version", "package", "command", "version", "tag", "commit", "source", "tools", "targets", "assets"})

    def test_inconsistent_inputs_are_refused(self) -> None:
        archive = "biscuit-hash-cli-aarch64-apple-darwin.tar.xz"
        sums = (self.assets / "sha256.sum").read_text()
        cases = (
            (lambda: (self.assets / archive).unlink(), f"missing ['{archive}']"),
            (lambda: (self.assets / "fixture.bin").write_bytes(b"x"), "unexpected ['fixture.bin']"),
            (lambda: (self.assets / f"{archive}.sha256").write_text(sum_line(archive, "f")), "disagrees with sha256.sum"),
            (lambda: (self.assets / f"{archive}.sha256").write_text(sum_line("other.tar.xz", "0")), "is not '<sha256>  biscuit-hash-cli"),
            (lambda: (self.assets / "sha256.sum").write_text(sums + "not a checksum line\n"), "is not '<sha256>  <name>'"),
            (lambda: (self.assets / "sha256.sum").write_text(sums + sums.splitlines(keepends=True)[0]), "twice"),
            (lambda: (self.assets / "sha256.sum").write_text("".join(l for l in sums.splitlines(keepends=True) if archive not in l)), "disagrees with sha256.sum"),
        )
        for damage, message in cases:
            with self.subTest(message=message):
                self.setUp()
                damage()
                with self.assertRaisesRegex(d.DistributionError, re.escape(message)):
                    d.build_provenance(self.selection, self.plan, self.assets, self.environments)

    def test_dist_real_unified_checksum_file_is_read(self) -> None:
        # dist 0.34.0's own `sha256.sum` (one local build) ends with a blank line.
        real = (REPO / "scripts/distribution/fixtures/dist-sha256.sum").read_bytes()
        self.assertTrue(real.endswith(b"\n\n"))
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "sha256.sum").write_bytes(real)
            sums = d.read_checksums(Path(tmp))
        self.assertEqual(list(sums), ["biscuit-hash-cli-aarch64-apple-darwin.tar.xz"])

    def test_build_environments_must_cover_each_target_once_with_one_toolchain(self) -> None:
        envs = self.environments
        cases = (
            (envs[:3], "must cover"),
            (envs + [envs[0]], "once each"),
            (envs[:3] + [build_env(d.TARGETS[3], "rustc 1.99.0")], "one Rust toolchain"),
            (envs[:3] + [dict(envs[3], image={"container": 7})], "image.container: must be a string"),
            (envs[:3] + [dict(envs[3], image=None)], "image: must not be null"),
            (envs[:3] + [{k: v for k, v in envs[3].items() if k != "rustc"}], "rustc: is required"),
            (envs[:3] + [dict(envs[3], target="x86_64-pc-windows-msvc")], "target: must be one of"),
        )
        for environments, message in cases:
            with self.subTest(message=message), self.assertRaisesRegex(d.DistributionError, re.escape(message)):
                d.build_provenance(self.selection, self.plan, self.assets, environments)

    def test_a_plan_for_another_selection_is_refused(self) -> None:
        with self.assertRaisesRegex(d.DistributionError, "announcement_tag: must be 'darkmatter-cli-v0.1.0'"):
            d.build_provenance(selection_for("darkmatter-cli"), self.plan, self.assets, self.environments)


class ReconcileAssetsTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)
        self.addCleanup(self.tmp.cleanup)
        self.content = {"a.tar.xz": b"A" * (d._CHUNK * 2 + 5), "sha256.sum": b"abc  a.tar.xz\n"}
        for name, data in self.content.items():
            (self.dir / name).write_bytes(data)
        self.expected = {name: self.dir / name for name in self.content}

    def remote(self, name: str, data: bytes) -> d.RemoteAsset:
        return d.RemoteAsset(name, len(data), 1, f"https://example.invalid/{name}")

    def reader(self, bodies: dict[str, bytes], chunk: int = 7):
        def read(asset: d.RemoteAsset):
            body = bodies[asset.name]
            for start in range(0, len(body), chunk):
                yield body[start:start + chunk]
        return read

    def test_first_upload_identical_partial_and_unrelated(self) -> None:
        self.assertEqual(d.reconcile_assets(self.expected, [], self.reader({})).as_json(),
                         {"reuse": [], "upload": ["a.tar.xz", "sha256.sum"], "conflicts": [], "unrelated": []})
        bodies = {"a.tar.xz": self.content["a.tar.xz"], "notes.txt": b"x"}
        existing = [self.remote(name, body) for name, body in bodies.items()]
        for chunk in (1, 7, d._CHUNK, d._CHUNK * 3):
            with self.subTest(chunk=chunk):
                plan = d.reconcile_assets(self.expected, existing, self.reader(bodies, chunk))
                self.assertEqual(plan.as_json(), {"reuse": ["a.tar.xz"], "upload": ["sha256.sum"], "conflicts": [], "unrelated": ["notes.txt"]})

    def test_different_bytes_conflict(self) -> None:
        changed = bytearray(self.content["a.tar.xz"])
        changed[-1] ^= 1
        plan = d.reconcile_assets(self.expected, [self.remote("a.tar.xz", bytes(changed))], self.reader({"a.tar.xz": bytes(changed)}))
        self.assertEqual(plan.conflicts, ["a.tar.xz"])
        shorter = self.content["a.tar.xz"][:-1]
        plan = d.reconcile_assets(self.expected, [self.remote("a.tar.xz", shorter)], self.reader({"a.tar.xz": shorter}))
        self.assertEqual(plan.conflicts, ["a.tar.xz"])

    def test_a_truncated_download_is_an_error(self) -> None:
        asset = self.remote("a.tar.xz", self.content["a.tar.xz"])
        with self.assertRaisesRegex(d.DistributionError, "download returned"):
            d.reconcile_assets(self.expected, [asset], self.reader({"a.tar.xz": self.content["a.tar.xz"][:-3]}))

    def test_remote_listing_shapes(self) -> None:
        self.assertEqual(d.remote_assets([], source="assets"), [])
        good = {"name": "a", "size": 1, "id": 2, "browser_download_url": "u"}
        self.assertEqual(d.remote_assets({"assets": [good]}, source="r")[0].name, "a")
        for listing, message in (([good, good], "duplicate asset"), ([dict(good, size=None)], "size: must not be null"),
                                 ([dict(good, size="1")], "must be an integer"), ([good, 3], r"\[1\]: must be a mapping"),
                                 ({"assets": None}, "assets: must not be null")):
            with self.subTest(message=message), self.assertRaisesRegex(d.DistributionError, message):
                d.remote_assets(listing, source="r")
        with self.assertRaisesRegex(d.DistributionError, "must not be empty"):
            d.reconcile_assets({}, [], self.reader({}))


class TapUpdateTests(unittest.TestCase):
    def test_versions_compare_numerically_and_never_downgrade(self) -> None:
        formula = b"class Bh < Formula\nend\n"
        cases = (
            ("0.10.0", None, "create"),
            ("0.10.0", "0.9.0", "update"),
            ("0.9.0", "0.10.0", "noop"),
            ("1.0.0", "1.0.0", "noop"),
            ("1.0.0-rc.1", "0.9.0", "skip"),
            ("1.0.0", "1.0.0-rc.1", "update"),
        )
        for candidate, installed, action in cases:
            with self.subTest(candidate=candidate, installed=installed):
                current = None if installed is None else d.FormulaVersion(installed, formula)
                self.assertEqual(d.select_tap_update(d.FormulaVersion(candidate, formula), current).action, action)

    def test_same_version_with_different_bytes_is_refused(self) -> None:
        with self.assertRaisesRegex(d.DistributionError, "different contents"):
            d.select_tap_update(d.FormulaVersion("1.0.0", b"a"), d.FormulaVersion("1.0.0", b"b"))

    def test_invalid_versions_are_located(self) -> None:
        for candidate, installed, where in (("1.0", None, "candidate"), ("1.0.0", "", "installed"), ("1.0.0", "v1.0.0", "installed")):
            with self.subTest(candidate=candidate, installed=installed), self.assertRaisesRegex(d.DistributionError, where):
                d.select_tap_update(d.FormulaVersion(candidate, b""), None if installed is None else d.FormulaVersion(installed, b""))


class PreflightFixture:
    """A temporary directory holding a preflight's inputs for `main`: real
    metadata, a policy approving bh, its release event, and a forge fixture."""

    TAG = "biscuit-hash-cli-v0.1.0"

    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.dir = Path(self.tmp.name)
        (self.dir / "metadata.json").write_text(METADATA, encoding="utf-8")
        (self.dir / "approved.yaml").write_text(approve(POLICY_TEXT, "biscuit-hash-cli"), encoding="utf-8")
        (self.dir / "event.json").write_text(json.dumps(release_event(self.TAG)), encoding="utf-8")
        forge = {"releases": {self.TAG: release_event(self.TAG)["release"]}, "tags": {self.TAG: TAG_SHA},
                 "proposals": {TAG_SHA: {"number": 17, "head_sha": TAG_SHA, "merge_sha": MERGE_SHA}}}
        (self.dir / "forge.json").write_text(json.dumps(forge), encoding="utf-8")

    def run_main(self, *argv: str) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            code = d.main(list(argv), env={"GITHUB_REPOSITORY": d.CANONICAL_REPOSITORY})
        return code, out.getvalue(), err.getvalue()

    def preflight(self, policy_file: str = "approved.yaml", *extra: str) -> tuple[int, str, str]:
        return self.run_main(
            "preflight", "--event-name", "release", "--event-path", str(self.dir / "event.json"),
            "--tagged-root", str(REPO), "--current-root", str(REPO),
            "--tagged-metadata", str(self.dir / "metadata.json"), "--current-metadata", str(self.dir / "metadata.json"),
            "--tagged-policy", str(self.dir / policy_file), "--current-policy", str(self.dir / policy_file),
            "--forge-fixture", str(self.dir / "forge.json"), "--checkout-commit", TAG_SHA, *extra,
        )


class CliTests(PreflightFixture, unittest.TestCase):
    """Wiring through `main`: one positive and one located malformed case per
    loader; the full matrices run above."""

    def test_preflight_selects_and_snapshots_the_policies(self) -> None:
        code, out, _ = self.preflight("approved.yaml", "--snapshot-dir", str(self.dir / "snap"), "--output", str(self.dir / "selection.json"))
        self.assertEqual(code, 0)
        result = json.loads(out)
        self.assertEqual((result["status"], result["command"], len(result["build_matrix"])), ("selected", "bh", 4))
        self.assertEqual((self.dir / "snap" / "tagged-deployment.yaml").read_bytes(), (self.dir / "approved.yaml").read_bytes())
        code, out, _ = self.run_main("config", "--selection", str(self.dir / "selection.json"))
        self.assertEqual((code, json.loads(out)["manifest_path"]), (0, "biscuit-hash/cli/Cargo.toml"))

    def test_preflight_with_the_real_policy_is_ignored(self) -> None:
        (self.dir / "real.yaml").write_bytes(POLICY_PATH.read_bytes())
        code, out, _ = self.preflight("real.yaml")
        self.assertEqual((code, json.loads(out)["status"], json.loads(out)["build_matrix"]), (0, "ignored", []))

    def test_a_malformed_policy_is_a_located_error_and_selects_nothing(self) -> None:
        text = edit_entry(POLICY_TEXT, "biscuit-hash-cli", "approved", "null")
        (self.dir / "bad.yaml").write_text(text, encoding="utf-8")
        code, out, err = self.preflight("bad.yaml")
        self.assertEqual(code, 2)
        self.assertEqual(json.loads(out)["status"], "invalid")
        self.assertRegex(err, r"bad\.yaml:\d+:\d+: packages\.biscuit-hash-cli\.approved: must not be null")
        self.assertNotIn("build_matrix", json.loads(out))

    def test_a_malformed_event_and_forge_are_located(self) -> None:
        (self.dir / "event.json").write_text('{"action": "published", "action": "x"}', encoding="utf-8")
        code, _, err = self.preflight()
        self.assertEqual(code, 2)
        self.assertIn("event.json: invalid JSON: duplicate object key 'action'", err)
        (self.dir / "event.json").write_text(json.dumps(release_event(self.TAG)), encoding="utf-8")
        (self.dir / "forge.json").write_text('{"releases": {}, "tags": [], "proposals": {}}', encoding="utf-8")
        code, _, err = self.preflight()
        self.assertEqual(code, 2)
        self.assertIn("forge.json: tags: must be a mapping", err)

    def test_a_missing_repository_is_an_error_not_an_ignored_release(self) -> None:
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            code = d.main(["preflight", "--event-name", "release", "--event-path", str(self.dir / "event.json"),
                           "--tagged-root", str(REPO), "--current-root", str(REPO),
                           "--tagged-metadata", str(self.dir / "metadata.json"), "--current-metadata", str(self.dir / "metadata.json"),
                           "--forge-fixture", str(self.dir / "forge.json"), "--checkout-commit", TAG_SHA], env={})
        self.assertEqual((code, json.loads(out.getvalue())["status"]), (2, "invalid"))
        self.assertIn("GITHUB_REPOSITORY: is required", err.getvalue())

    def test_pending_release_app_exits_with_its_own_status(self) -> None:
        forge = json.loads((self.dir / "forge.json").read_text())
        forge["proposals"][TAG_SHA] = "pending"
        (self.dir / "forge.json").write_text(json.dumps(forge), encoding="utf-8")
        code, out, _ = self.preflight()
        self.assertEqual((code, json.loads(out)["status"]), (3, "pending-setup"))

    def test_a_malformed_metadata_document_is_located(self) -> None:
        (self.dir / "metadata.json").write_text(METADATA + "x", encoding="utf-8")
        code, _, err = self.preflight()
        self.assertEqual(code, 2)
        self.assertIn("metadata.json: invalid JSON", err)

    def test_stage_and_provenance_commands(self) -> None:
        stage = StageTests("test_stage_copies_the_checkout_and_changes_only_the_opt_in")
        stage.setUp()
        self.addCleanup(stage.doCleanups)
        (self.dir / "selection.json").write_text(json.dumps(stage.selection.record), encoding="utf-8")
        args = ("stage", "--selection", str(self.dir / "selection.json"), "--source", str(stage.source),
                "--notices", str(stage.notices), "--record-dir", str(self.dir / "record"))
        code, out, _ = self.run_main(*args, "--dest", str(stage.dest))
        self.assertEqual((code, json.loads(out)["status"]), (0, "staged"))
        self.assertIn("+[package.metadata.dist]", (self.dir / "record/overlay.diff").read_text())
        self.assertEqual((self.dir / "record/dist-config.toml").read_text(), d.render_dist_config(stage.selection).overlay)
        (stage.source / "Cargo.lock").write_text("edited\n")
        code, out, err = self.run_main(*args, "--dest", str(self.dir / "second"))
        self.assertEqual((code, json.loads(out)["status"]), (2, "invalid"))
        self.assertIn("has tracked modifications: ['Cargo.lock']", err)
        self.assertFalse((self.dir / "second").exists())

        prov = ProvenanceTests("test_provenance_is_deterministic_and_complete")
        prov.setUp()
        self.addCleanup(prov.doCleanups)
        (self.dir / "bh.json").write_text(json.dumps(prov.selection.record), encoding="utf-8")
        envs = []
        for target, env in zip(d.TARGETS, prov.environments):
            (self.dir / f"{target}.json").write_text(json.dumps(env), encoding="utf-8")
            envs += ["--build-env", str(self.dir / f"{target}.json")]
        code, out, _ = self.run_main("provenance", "--selection", str(self.dir / "bh.json"), "--plan", str(PLAN_BH),
                                     "--assets-dir", str(prov.assets), *envs, "--write")
        self.assertEqual(code, 0)
        written = (prov.assets / d.PROVENANCE_NAME).read_text()
        self.assertEqual(json.loads(written), json.loads(out))
        # Rerunning beside the written record gives the same bytes: it is not an input.
        self.run_main("provenance", "--selection", str(self.dir / "bh.json"), "--plan", str(PLAN_BH),
                      "--assets-dir", str(prov.assets), *envs, "--write")
        self.assertEqual((prov.assets / d.PROVENANCE_NAME).read_text(), written)
        code, _, err = self.run_main("provenance", "--selection", str(self.dir / "bh.json"), "--plan", str(PLAN_BH),
                                     "--assets-dir", str(prov.assets), *envs[:6])
        self.assertEqual(code, 2)
        self.assertIn("build environments: must cover", err)

    def test_reconcile_and_tap_update_commands(self) -> None:
        built = self.dir / "built"
        built.mkdir()
        (built / "a.tar.xz").write_bytes(b"new")
        existing = self.dir / "existing"
        existing.mkdir()
        (existing / "a.tar.xz").write_bytes(b"old")
        (self.dir / "assets.json").write_text(json.dumps([{"name": "a.tar.xz", "size": 3, "id": 1, "browser_download_url": "u"}]), encoding="utf-8")
        code, out, _ = self.run_main("reconcile-assets", "--expected-dir", str(built), "--existing", str(self.dir / "assets.json"), "--existing-dir", str(existing))
        self.assertEqual((code, json.loads(out)["conflicts"]), (1, ["a.tar.xz"]))
        (self.dir / "f.rb").write_bytes(b"formula")
        code, out, _ = self.run_main("tap-update", "--candidate-version", "0.9.0", "--candidate-formula", str(self.dir / "f.rb"),
                                     "--installed-version", "0.10.0", "--installed-formula", str(self.dir / "f.rb"))
        self.assertEqual((code, json.loads(out)["action"]), (0, "noop"))
        code, _, err = self.run_main("tap-update", "--candidate-version", "0.9", "--candidate-formula", str(self.dir / "f.rb"))
        self.assertEqual(code, 2)
        self.assertIn("candidate formula version", err)


BUILD_IMAGES = REPO / ".github/distribution/build-images.json"
ENABLED_BINSTALL = {"binstall": {**d.BINSTALL_URLS, "disabled-strategies": list(d.BINSTALL_ENABLED_STRATEGIES)}}


def verification(package: str = "biscuit-hash-cli", channels: dict | None = None, workspace: d.Workspace = WORKSPACE) -> d.Selection:
    return d.Selection.from_json(d.select_verification(package, workspace, TAG_SHA, channels or {}).as_json(), source="verify")


class VerificationSelectionTests(unittest.TestCase):
    """A verification record builds like a release but can never publish."""

    def test_the_current_checkout_is_selected_without_publishing(self) -> None:
        result = d.select_verification("biscuit-hash-cli", WORKSPACE, TAG_SHA, {})
        self.assertEqual(result.status, d.SELECTED)
        record = result.as_json()
        self.assertEqual(
            {k: record[k] for k in ("package", "command", "version", "tag", "commit", "mode", "publishable", "release",
                                    "proposal", "channels", "revoked_channels", "formula_update", "prerelease", "targets")},
            {"package": "biscuit-hash-cli", "command": "bh", "version": "0.1.0", "tag": "biscuit-hash-cli-v0.1.0",
             "commit": TAG_SHA, "mode": "verify", "publishable": False, "release": None, "proposal": None,
             "channels": {}, "revoked_channels": [], "formula_update": False, "prerelease": False, "targets": list(d.TARGETS)},
        )
        # The later build steps accept it unchanged: config, plan, provenance.
        selection = d.Selection.from_json(record, source="verify")
        self.assertEqual(d.render_dist_config(selection).overlay, d.render_dist_config(selection_for("biscuit-hash-cli")).overlay)
        d.validate_plan(selection, d.load_json(PLAN_BH.read_bytes(), source="plan"))
        self.assertIn("/tree/biscuit-hash-cli-v0.1.0/", d.source_text(selection))

    def test_a_brew_fixture_name_is_validated_and_never_updates_the_formula(self) -> None:
        record = verification(channels={"brew": "bh"}).record
        self.assertEqual((record["channels"], record["formula_update"]), ({"brew": "bh"}, False))
        for name in ("Bad Name", "", "-x"):
            with self.subTest(name=name), self.assertRaisesRegex(d.DistributionError, r"^--brew: name .* must match"):
                d.select_verification("biscuit-hash-cli", WORKSPACE, TAG_SHA, {"brew": name})

    def test_binstall_must_agree_with_the_manifest(self) -> None:
        with self.assertRaisesRegex(d.DistributionError, "disabled-strategies: must be .* while the binstall channel is enabled"):
            d.select_verification("biscuit-hash-cli", WORKSPACE, TAG_SHA, {"binstall": "biscuit-hash-cli"})
        enabled = with_package("biscuit-hash-cli", metadata=ENABLED_BINSTALL)
        record = verification(channels={"binstall": "biscuit-hash-cli"}, workspace=enabled).record
        self.assertEqual(record["channels"], {"binstall": "biscuit-hash-cli"})
        with self.assertRaisesRegex(d.DistributionError, "^--binstall: cargo-binstall installs by package name"):
            d.select_verification("biscuit-hash-cli", enabled, TAG_SHA, {"binstall": "bh"})

    def test_non_roster_unknown_and_malformed_inputs_are_located(self) -> None:
        cases = (
            (("biscuit-hash", TAG_SHA, {}), "--package: 'biscuit-hash' is not one of the packages that ship a binary"),
            (("no-such-package", TAG_SHA, {}), "--package: 'no-such-package' is not one of"),
            (("biscuit-hash-cli", "abc", {}), "--checkout-commit: must be a full 40-digit"),
            (("biscuit-hash-cli", TAG_SHA, {"apt": "bh"}), "--apt: is not a channel this workflow publishes"),
        )
        for (package, commit, channels), message in cases:
            with self.subTest(message=message), self.assertRaisesRegex(d.DistributionError, "^" + re.escape(message)):
                d.select_verification(package, WORKSPACE, commit, channels)

    def test_manifest_checks_still_apply(self) -> None:
        with self.assertRaisesRegex(d.DistributionError, "package.repository: must be"):
            d.select_verification("biscuit-hash-cli", with_package("biscuit-hash-cli", repository=None), TAG_SHA, {})

    def test_a_prerelease_manifest_is_a_prerelease(self) -> None:
        record = verification(workspace=with_package("biscuit-hash-cli", version="0.2.0-rc.1")).record
        self.assertEqual((record["tag"], record["prerelease"]), ("biscuit-hash-cli-v0.2.0-rc.1", True))

    def test_a_forged_publishable_flag_or_mode_is_refused(self) -> None:
        verify = verification().record
        released = selection_for("biscuit-hash-cli").record
        self.assertEqual((released["mode"], released["publishable"]), ("release", True))
        cases = (
            (verify | {"publishable": True}, "publishable: must be false for mode 'verify'"),
            (released | {"publishable": False}, "publishable: must be true for mode 'release'"),
            (released | {"publishable": "true"}, "publishable: must be true or false"),
            ({k: v for k, v in released.items() if k != "publishable"}, "publishable: is required"),
            (released | {"mode": "publish"}, "mode: must be one of"),
            ({k: v for k, v in released.items() if k != "mode"}, "mode: is required"),
        )
        for record, message in cases:
            with self.subTest(message=message), self.assertRaisesRegex(d.DistributionError, re.escape(message)):
                d.Selection.from_json(record, source="s")


class BuildMatrixTests(unittest.TestCase):
    def setUp(self) -> None:
        self.images = d.load_json(BUILD_IMAGES.read_bytes(), source="images")
        self.selection = selection_for("biscuit-hash-cli")

    def test_the_real_build_images_name_every_runner_in_target_order(self) -> None:
        self.assertEqual(d.build_matrix(self.selection, self.images, source="images"), [
            {"target": "aarch64-apple-darwin", "runner": "macos-15", "os": "macos"},
            {"target": "x86_64-apple-darwin", "runner": "macos-15-intel", "os": "macos"},
            {"target": "aarch64-unknown-linux-gnu", "runner": "ubuntu-22.04-arm", "os": "linux"},
            {"target": "x86_64-unknown-linux-gnu", "runner": "ubuntu-22.04", "os": "linux"},
        ])

    def test_one_edit_each_is_located(self) -> None:
        cases = (
            (lambda i: i["targets"].pop("x86_64-apple-darwin"), "images: targets.x86_64-apple-darwin: is required"),
            (lambda i: i["targets"]["aarch64-unknown-linux-gnu"].update(runner=""), "targets.aarch64-unknown-linux-gnu.runner: must not be empty"),
            (lambda i: i["targets"]["aarch64-unknown-linux-gnu"].pop("runner"), "targets.aarch64-unknown-linux-gnu.runner: is required"),
            (lambda i: i["targets"]["aarch64-apple-darwin"].update(os="linux"), "targets.aarch64-apple-darwin.os: must be 'macos'"),
            (lambda i: i.update(targets=None), "targets: must not be null"),
        )
        for edit, message in cases:
            with self.subTest(message=message), self.assertRaisesRegex(d.DistributionError, re.escape(message)):
                d.build_matrix(self.selection, edited(self.images, edit), source="images")


def built_assets(case: unittest.TestCase) -> ProvenanceTests:
    """`ProvenanceTests`' faked dist outputs, plus the provenance record
    `provenance --write` renders from them."""
    prov = ProvenanceTests("test_provenance_is_deterministic_and_complete")
    prov.setUp()
    case.addCleanup(prov.doCleanups)
    record = d.build_provenance(prov.selection, prov.plan, prov.assets, prov.environments)
    (prov.assets / d.PROVENANCE_NAME).write_bytes(d.render_json(record))
    return prov


RUN_ID = 1001


def run_document(**changes) -> dict:
    run = {"id": RUN_ID, "path": ".github/workflows/distribute-binaries.yml@refs/heads/main",
           "repository": {"full_name": d.CANONICAL_REPOSITORY, "private": False}, "conclusion": "failure",
           "head_sha": "9" * 40, "status": "completed"}
    run.update(changes)
    return run


def jobs_document(package_conclusion="success") -> dict:
    return {"total_count": 3, "jobs": [
        {"name": "Preflight", "conclusion": "success", "id": 1},
        {"name": "Package", "conclusion": package_conclusion, "id": 2},
        {"name": "Upload", "conclusion": "failure", "id": 3},
    ]}


class BundleTests(unittest.TestCase):
    def setUp(self) -> None:
        self.prov = built_assets(self)
        self.selection = self.prov.selection
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.bundle = Path(tmp.name) / "bundle"
        self.record = d.write_bundle(self.selection, self.prov.plan, self.prov.assets, self.prov.environments, RUN_ID, self.bundle)

    def check(self, fresh: d.Selection | None = None, **kwargs) -> d.Bundle:
        kwargs.setdefault("current_run_id", RUN_ID)
        return d.check_bundle(self.bundle, fresh or self.selection, **kwargs)

    def test_round_trip(self) -> None:
        names = d.bundle_assets(self.selection)
        self.assertEqual(sorted(p.name for p in self.bundle.iterdir()), sorted(d.BUNDLE_ENTRIES))
        self.assertEqual(self.record | {"assets": {}}, {
            "schema_version": 1, "package": "biscuit-hash-cli", "version": "0.1.0", "tag": "biscuit-hash-cli-v0.1.0",
            "commit": TAG_SHA, "run_id": RUN_ID, "assets": {}})
        self.assertEqual(set(self.record["assets"]), names)
        self.assertEqual(d.load_json((self.bundle / "bundle.json").read_bytes(), source="b"), self.record)
        found = self.check()
        self.assertEqual((found.run_id, set(found.assets), found.selection.record), (RUN_ID, names, self.selection.record))
        for name, path in found.assets.items():
            self.assertEqual(path.read_bytes(), (self.prov.assets / name).read_bytes(), name)
        self.assertEqual(sorted(p.name for p in (self.bundle / "build-env").iterdir()), sorted(f"{t}.json" for t in d.TARGETS))

    def test_a_bundle_is_never_written_over_another_or_from_an_inconsistent_build(self) -> None:
        with self.assertRaisesRegex(d.DistributionError, "must be absent or empty"):
            d.write_bundle(self.selection, self.prov.plan, self.prov.assets, self.prov.environments, RUN_ID, self.bundle)
        target = self.bundle.with_name("second")
        provenance = self.prov.assets / d.PROVENANCE_NAME
        provenance.write_bytes(provenance.read_bytes().replace(b'"schema_version": 1', b'"schema_version": 2'))
        with self.assertRaisesRegex(d.DistributionError, "distribution-provenance.json: differs from the record"):
            d.write_bundle(self.selection, self.prov.plan, self.prov.assets, self.prov.environments, RUN_ID, target)
        provenance.unlink()
        with self.assertRaisesRegex(d.DistributionError, r"missing \['distribution-provenance.json'\]"):
            d.write_bundle(self.selection, self.prov.plan, self.prov.assets, self.prov.environments, RUN_ID, target)
        for run_id in (0, -1, True, "1001"):
            with self.subTest(run_id=run_id), self.assertRaisesRegex(d.DistributionError, "--run-id: must be a positive integer"):
                d.write_bundle(self.selection, self.prov.plan, self.prov.assets, self.prov.environments, run_id, target)
        self.assertFalse(target.exists())

    def test_one_edit_each_is_refused(self) -> None:
        archive = "biscuit-hash-cli-aarch64-apple-darwin.tar.xz"
        assets = lambda: self.bundle / "assets"  # noqa: E731

        def rewrite(path: Path, edit) -> None:
            value = d.load_json(path.read_bytes(), source=path.name)
            edit(value)
            path.write_bytes(d.render_json(value))

        cases = (
            (lambda: None, dict(fresh=d.Selection.from_json(self.selection.record | {"commit": "3" * 40}, source="s")),
             f"selection.json: commit: is '{TAG_SHA}', but the fresh selection is '{'3' * 40}'"),
            (lambda: None, dict(fresh=selection_for("biscuit-hash-cli", brew="bh")), "selection.json: channels"),
            (lambda: None, dict(fresh=verification()), "selection.json: publishable: is True, but the fresh selection is False"),
            (lambda: (assets() / archive).unlink(), {}, f"missing ['{archive}']"),
            (lambda: (assets() / "extra.bin").write_bytes(b"x"), {}, "unexpected ['extra.bin']"),
            (lambda: (self.bundle / "notes.txt").write_bytes(b"x"), {}, "unexpected ['notes.txt']"),
            (lambda: (self.bundle / "build-env" / "x86_64-unknown-linux-gnu.json").unlink(), {}, "missing ['x86_64-unknown-linux-gnu.json']"),
            (lambda: rewrite(self.bundle / "build-env" / "x86_64-unknown-linux-gnu.json", lambda e: e.update(target="aarch64-unknown-linux-gnu")),
             {}, "x86_64-unknown-linux-gnu.json: target: must be 'x86_64-unknown-linux-gnu'"),
            (lambda: rewrite(self.bundle / "bundle.json", lambda b: b["assets"].update({archive: 99})), {}, f"assets.{archive}: records 99 bytes"),
            (lambda: (assets() / archive).write_bytes(b"truncated"), {}, f"assets.{archive}: records"),
            (lambda: rewrite(self.bundle / "bundle.json", lambda b: b["assets"].pop(archive)), {}, "must list exactly"),
            (lambda: rewrite(self.bundle / "bundle.json", lambda b: b.update(commit="3" * 40)), {}, "bundle.json: commit: is '333"),
            (lambda: rewrite(self.bundle / "bundle.json", lambda b: b.update(run_id="1001")), {}, "run_id: must be an integer"),
            (lambda: rewrite(self.bundle / "plan.json", lambda p: p.update(announcement_tag="v0.1.0")), {}, "announcement_tag: must be"),
            (lambda: (assets() / d.PROVENANCE_NAME).write_bytes((assets() / d.PROVENANCE_NAME).read_bytes().replace(b'"schema_version": 1', b'"schema_version": 2')),
             {}, "distribution-provenance.json: differs from the record"),
            (lambda: None, dict(current_run_id=2002), "names run 1001, not this run; a reused bundle needs that run's API documents"),
            (lambda: None, dict(current_run_id=None), "names run 1001, not this run"),
            (lambda: None, dict(current_run_id=2002, run=run_document()), "a reused bundle needs"),
            (lambda: None, dict(current_run_id=2002, run=run_document(), jobs=jobs_document("failure")), "jobs[1].conclusion: is 'failure'"),
            (lambda: None, dict(current_run_id=2002, run=run_document(), jobs=jobs_document(None)), "jobs[1].conclusion: is null"),
            (lambda: None, dict(current_run_id=2002, run=run_document(), jobs={"jobs": [{"name": "package", "conclusion": "success"}]}),
             "exactly one 'Package' job (the latest attempt), found 0"),
            (lambda: None, dict(current_run_id=2002, run=run_document(path=".github/workflows/ci.yml@refs/heads/main"), jobs=jobs_document()), "path: is '.github/workflows/ci.yml"),
            (lambda: None, dict(current_run_id=2002, run=run_document(path=".github/workflows/distribute-binaries.yml.bak"), jobs=jobs_document()), "path: is"),
            (lambda: None, dict(current_run_id=2002, run=run_document(repository={"full_name": "someone/fork"}), jobs=jobs_document()), "repository.full_name: is 'someone/fork'"),
            (lambda: None, dict(current_run_id=2002, run=run_document(id=1002), jobs=jobs_document()), "id: is 1002, but the bundle names run 1001"),
        )
        for damage, kwargs, message in cases:
            with self.subTest(message=message):
                self.setUp()
                damage()
                with self.assertRaisesRegex(d.DistributionError, re.escape(message)):
                    self.check(**kwargs)

    def test_a_reused_bundle_with_its_successful_package_job_is_accepted(self) -> None:
        # Unknown API fields are ignored; the run's own conclusion is not read.
        found = self.check(current_run_id=2002, run=run_document(path=".github/workflows/distribute-binaries.yml"), jobs=jobs_document())
        self.assertEqual(found.run_id, RUN_ID)
        d.check_bundle_run(run_document(), jobs_document(), RUN_ID)


class FakePublishForge(d.PublishForge):
    """Release state for the selected bh release, with assets held as bytes.
    Every call is recorded; `upload_asset` is the only write."""

    def __init__(self, selection: d.Selection, existing: dict[str, bytes] | None = None) -> None:
        self.tag = selection.tag
        self.releases = {self.tag: release(self.tag)}
        self.tags = {self.tag: TAG_SHA}
        self.proposals = {TAG_SHA: d.ProposalProof(17, TAG_SHA, MERGE_SHA)}
        self.assets = dict(existing or {})
        self.races: dict[str, bytes] = {}
        self.truncated: set[str] = set()
        self.calls: list[tuple[str, object]] = []

    def release(self, tag):
        self.calls.append(("release", tag))
        return self.releases.get(tag)

    def tag_commit(self, tag):
        self.calls.append(("tag", tag))
        return self.tags.get(tag)

    def proposal(self, commit):
        self.calls.append(("proposal", commit))
        return self.proposals.get(commit)

    def list_assets(self, release_id):
        self.calls.append(("list", release_id))
        return [d.RemoteAsset(name, len(body), index, f"https://example.invalid/{name}") for index, (name, body) in enumerate(sorted(self.assets.items()))]

    def read_asset(self, asset):
        self.calls.append(("read", asset.name))
        body = self.assets[asset.name]
        if asset.name in self.truncated:
            body = body[:-1]
        for start in range(0, len(body), 3):
            yield body[start:start + 3]

    def upload_asset(self, release_id, name, path):
        self.calls.append(("upload", name))
        if name in self.races:
            self.assets[name] = self.races.pop(name)
            raise d.AssetExists(name)
        self.assets[name] = path.read_bytes()
        return d.RemoteAsset(name, len(self.assets[name]), 99, f"https://example.invalid/{name}")

    def uploads(self) -> list[str]:
        return [name for kind, name in self.calls if kind == "upload"]


class PublishTests(unittest.TestCase):
    APPROVED = approve(POLICY_TEXT, "biscuit-hash-cli")

    def setUp(self) -> None:
        self.prov = built_assets(self)
        self.selection = self.prov.selection
        self.assets = {path.name: path for path in self.prov.assets.iterdir()}
        self.local = {name: path.read_bytes() for name, path in self.assets.items()}
        self.order = d.upload_order(self.selection)
        self.forge = FakePublishForge(self.selection)

    def publish(self, *, selection: d.Selection | None = None, assets=None, current: str | None = None, fresh_rebuild: bool = False) -> d.PublishResult:
        return d.publish_release_assets(selection or self.selection, self.assets if assets is None else assets, self.forge,
                                        policy(self.APPROVED if current is None else current, "current"), fresh_rebuild=fresh_rebuild)

    def assertNoEffect(self, message: str, error=d.DistributionError, **kwargs) -> None:
        before = dict(self.forge.assets)
        with self.assertRaises(error) as caught:
            self.publish(**kwargs)
        self.assertIn(message, str(caught.exception))
        self.assertEqual((self.forge.uploads(), self.forge.assets), ([], before))

    def test_the_upload_order_puts_provenance_last(self) -> None:
        self.assertEqual(self.order[:8], sorted(n for n in self.local if ".tar.xz" in n))
        self.assertEqual(self.order[8:], ["sha256.sum", "biscuit-hash-cli-installer.sh", d.PROVENANCE_NAME])
        brew = d.upload_order(selection_for("biscuit-hash-cli", brew="bh"))
        self.assertEqual(brew[-2:], ["bh.rb", d.PROVENANCE_NAME])

    def test_first_upload_attaches_everything_in_order(self) -> None:
        result = self.publish()
        self.assertEqual(result.as_json(), {"status": "published", "tag": "biscuit-hash-cli-v0.1.0", "release_id": 42,
                                            "reused": [], "uploaded": self.order, "unrelated": []})
        self.assertEqual(self.forge.uploads(), self.order)
        self.assertEqual(self.forge.assets, self.local)
        # Every identity is rechecked before the first listing or write.
        first_list = self.forge.calls.index(("list", 42))
        self.assertEqual(self.forge.calls[:first_list], [("release", self.forge.tag), ("tag", self.forge.tag), ("proposal", TAG_SHA)])

    def test_an_identical_retry_uploads_nothing(self) -> None:
        self.forge.assets = dict(self.local)
        result = self.publish()
        self.assertEqual((result.reused, result.uploaded, self.forge.uploads()), (sorted(self.local), [], []))

    def test_a_partial_release_receives_only_the_missing_assets(self) -> None:
        self.forge.assets = {name: self.local[name] for name in self.order[:3]}
        result = self.publish()
        self.assertEqual((result.reused, result.uploaded), (sorted(self.order[:3]), self.order[3:]))
        self.assertEqual(self.forge.uploads(), self.order[3:])
        self.assertEqual(self.forge.assets, self.local)

    def test_a_conflict_uploads_nothing(self) -> None:
        name = self.order[0]
        self.forge.assets = {name: bytes(len(self.local[name]))}
        before = dict(self.forge.assets)
        with self.assertRaises(d.AssetConflict) as caught:
            self.publish()
        self.assertEqual(caught.exception.conflicts, [name])
        self.assertEqual((self.forge.uploads(), self.forge.assets), ([], before))

    def test_a_truncated_download_is_an_error(self) -> None:
        self.forge.assets = {self.order[0]: self.local[self.order[0]]}
        self.forge.truncated.add(self.order[0])
        self.assertNoEffect("download returned")

    def test_a_verification_selection_never_reaches_the_forge(self) -> None:
        self.assertNoEffect("is 'verify'; only a publishable release or recovery selection uploads assets", selection=verification())
        self.assertEqual(self.forge.calls, [])

    def test_revoked_approval_or_channel_has_no_effect(self) -> None:
        self.assertNoEffect("packages.biscuit-hash-cli.approved: approval was revoked after preflight", current=POLICY_TEXT)
        brew = selection_for("biscuit-hash-cli", brew="bh")
        formula = self.prov.assets / "bh.rb"
        formula.write_bytes(b"class Bh < Formula\nend\n")
        assets = {**self.assets, "bh.rb": formula}
        self.assertNoEffect("packages.biscuit-hash-cli.brew: resolved to 'bh' at preflight but is now None", selection=brew, assets=assets)
        self.assertNoEffect("but is now 'bhx'", selection=brew, assets=assets, current=approve(POLICY_TEXT, "biscuit-hash-cli", brew="bhx"))
        self.assertEqual(self.forge.calls, [])
        self.forge.assets = {}
        self.assertEqual(self.publish(selection=brew, assets=assets, current=approve(POLICY_TEXT, "biscuit-hash-cli", brew="bh")).uploaded[-2:],
                         ["bh.rb", d.PROVENANCE_NAME])

    def test_identity_races_have_no_effect(self) -> None:
        tag = self.forge.tag
        cases = (
            (lambda f: f.releases.pop(tag), "no GitHub Release exists"),
            (lambda f: f.releases.update({tag: d.ReleaseState(tag, 99, False, False, "u")}), "is Release 99, but preflight selected Release 42"),
            (lambda f: f.releases.update({tag: release(tag, draft=True)}), "is a draft"),
            (lambda f: f.tags.update({tag: "3" * 40}), "a published tag is never moved"),
            (lambda f: f.tags.pop(tag), "does not exist"),
            (lambda f: f.proposals.update({TAG_SHA: d.ProposalProof(18, TAG_SHA, MERGE_SHA)}), "release proposal is #18"),
            (lambda f: f.proposals.pop(TAG_SHA), "release proposal is none"),
        )
        for damage, message in cases:
            with self.subTest(message=message):
                self.forge = FakePublishForge(self.selection)
                damage(self.forge)
                self.assertNoEffect(message)
                self.assertNotIn(("list", 42), self.forge.calls)
        self.forge = FakePublishForge(self.selection)
        self.forge.proposals[TAG_SHA] = d.PendingSetup("RELEASE_APP_BOT_LOGIN", "configure it")
        self.assertNoEffect("RELEASE_APP_BOT_LOGIN", error=d.PendingSetup)

    def test_a_selection_without_release_identity_is_refused(self) -> None:
        record = self.selection.record | {"release": None}
        self.assertNoEffect("selection: release: must be a mapping, found null", selection=d.Selection.from_json(record, source="s"))
        self.assertNoEffect("must be exactly the bundle's", assets={k: v for k, v in self.assets.items() if k != d.PROVENANCE_NAME})
        self.assertEqual(self.forge.calls, [])

    def test_a_fresh_rebuild_of_a_partial_release_is_refused(self) -> None:
        self.forge.assets = {self.order[0]: self.local[self.order[0]]}
        self.assertNoEffect("a partial release needs its original build bundle: rerun with the bundle run id", fresh_rebuild=True)
        self.assertFalse([call for call in self.forge.calls if call[0] == "read"])
        self.forge = FakePublishForge(self.selection)
        self.assertEqual(self.publish(fresh_rebuild=True).uploaded, self.order)

    def test_an_upload_race_with_identical_bytes_is_reused(self) -> None:
        name = self.order[2]
        self.forge.races[name] = self.local[name]
        result = self.publish()
        self.assertIn(name, result.reused)
        self.assertEqual(result.uploaded, [n for n in self.order if n != name])
        self.assertEqual(self.forge.assets, self.local)

    def test_an_upload_race_with_different_bytes_is_a_conflict(self) -> None:
        name = self.order[2]
        self.forge.races[name] = b"other writer"
        with self.assertRaises(d.AssetConflict) as caught:
            self.publish()
        self.assertEqual(caught.exception.conflicts, [name])
        self.assertEqual(self.forge.uploads(), self.order[:3])
        self.assertNotIn(d.PROVENANCE_NAME, self.forge.assets, "an incomplete set never gains its provenance")

    def test_unrelated_assets_are_untouched_and_reported(self) -> None:
        self.forge.assets = {"notes.txt": b"release notes attachment"}
        result = self.publish()
        self.assertEqual(result.unrelated, ["notes.txt"])
        self.assertEqual(self.forge.assets["notes.txt"], b"release notes attachment")
        self.assertNotIn(("read", "notes.txt"), self.forge.calls)
        self.assertNotIn("notes.txt", self.forge.uploads())


class StubbedPublishGitHub(d.GitHubPublishForge):
    """The real GitHub adapter with its transport stubbed: every request it
    would make is recorded with its method."""

    def __init__(self, remote: dict[str, bytes]) -> None:
        super().__init__(repository=d.CANONICAL_REPOSITORY, token="token", bot_login="release-app[bot]", repo=REPO)
        tag = "biscuit-hash-cli-v0.1.0"
        self.responses = {
            f"/repos/{d.CANONICAL_REPOSITORY}/releases/tags/{tag}": (200, release_event(tag)["release"]),
            f"/repos/{d.CANONICAL_REPOSITORY}/commits/{TAG_SHA}/pulls": (200, [{"number": 17}]),
            f"/repos/{d.CANONICAL_REPOSITORY}/pulls/17": (200, proposal_pull()),
        }
        self.remote = remote
        self.requests: list[tuple[str, str]] = []

    def _get(self, path):
        self.requests.append(("GET", path))
        if path.startswith(f"/repos/{d.CANONICAL_REPOSITORY}/releases/42/assets?"):
            return 200, [{"name": n, "size": len(b), "id": i, "browser_download_url": f"https://example.invalid/{n}", "state": "uploaded"}
                         for i, (n, b) in enumerate(sorted(self.remote.items()))]
        return self.responses.get(path, (404, None))

    def _upload(self, url, path):
        self.requests.append(("POST", url))
        name = urllib.parse.unquote(url.split("?name=", 1)[1])
        if name in self.remote:
            return 422, {"message": "Validation Failed", "errors": [{"resource": "ReleaseAsset", "code": "already_exists", "field": "name"}]}
        self.remote[name] = path.read_bytes()
        return 201, {"name": name, "size": len(self.remote[name]), "id": 7, "browser_download_url": f"https://example.invalid/{name}"}

    def tag_commit(self, tag):
        self.requests.append(("git ls-remote", tag))
        return TAG_SHA

    def read_asset(self, asset):
        yield self.remote[asset.name]


class GitHubPublishForgeTests(unittest.TestCase):
    def test_publication_only_reads_and_attaches_assets(self) -> None:
        prov = built_assets(self)
        assets = {path.name: path for path in prov.assets.iterdir()}
        forge = StubbedPublishGitHub({})
        result = d.publish_release_assets(prov.selection, assets, forge, policy(approve(POLICY_TEXT, "biscuit-hash-cli")), fresh_rebuild=False)
        self.assertEqual(result.uploaded, d.upload_order(prov.selection))
        self.assertEqual({method for method, _ in forge.requests}, {"GET", "POST", "git ls-remote"})
        posts = [url for method, url in forge.requests if method == "POST"]
        prefix = f"https://uploads.github.com/repos/{d.CANONICAL_REPOSITORY}/releases/42/assets?name="
        self.assertEqual(posts, [prefix + urllib.parse.quote(name, safe="") for name in result.uploaded])
        # A second run races nothing and writes nothing.
        forge.requests.clear()
        self.assertEqual(d.publish_release_assets(prov.selection, assets, forge, policy(approve(POLICY_TEXT, "biscuit-hash-cli")), fresh_rebuild=False).uploaded, [])
        self.assertNotIn("POST", {method for method, _ in forge.requests})

    def test_already_exists_is_the_only_tolerated_upload_failure(self) -> None:
        forge = StubbedPublishGitHub({"a.tar.xz": b"x"})
        path = REPO / "LICENSE"
        with self.assertRaises(d.AssetExists):
            forge.upload_asset(42, "a.tar.xz", path)
        forge._upload = lambda url, p: (422, {"errors": [{"code": "invalid"}]})  # type: ignore[method-assign]
        with self.assertRaisesRegex(d.DistributionError, "unexpected HTTP 422"):
            forge.upload_asset(42, "b.tar.xz", path)
        forge._upload = lambda url, p: (422, None)  # type: ignore[method-assign]
        with self.assertRaisesRegex(d.DistributionError, "unexpected HTTP 422"):
            forge.upload_asset(42, "b.tar.xz", path)

    def test_the_module_has_no_release_edit_or_delete_request(self) -> None:
        source = (REPO / "scripts/ci/distribution.py").read_text(encoding="utf-8")
        for method in ("PATCH", "DELETE", "PUT"):
            self.assertNotIn(f'method="{method}"', source)
        self.assertEqual(re.findall(r'method="(\w+)"', source), ["POST"])


class NewCommandTests(unittest.TestCase):
    """CLI wiring for verify-select, build-matrix, bundle, check-bundle, and
    publish's refusals; the matrices run in-process above."""

    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.dir = Path(self.tmp.name)
        (self.dir / "metadata.json").write_text(METADATA, encoding="utf-8")

    def run_main(self, *argv: str, env: dict | None = None) -> tuple[int, dict, str]:
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            code = d.main(list(argv), env={"GITHUB_REPOSITORY": d.CANONICAL_REPOSITORY} if env is None else env)
        return code, json.loads(out.getvalue()), err.getvalue()

    def verify_select(self, *extra: str) -> tuple[int, dict, str]:
        return self.run_main("verify-select", "--root", str(REPO), "--metadata", str(self.dir / "metadata.json"),
                             "--checkout-commit", TAG_SHA, *extra)

    def write(self, name: str, value) -> Path:
        (self.dir / name).write_bytes(d.render_json(value))
        return self.dir / name

    def test_verify_select_and_build_matrix(self) -> None:
        code, record, _ = self.verify_select("--package", "biscuit-hash-cli", "--brew", "bh", "--output", str(self.dir / "verify.json"))
        self.assertEqual((code, record["mode"], record["publishable"], record["channels"]), (0, "verify", False, {"brew": "bh"}))
        code, out, _ = self.run_main("build-matrix", "--selection", str(self.dir / "verify.json"), "--images", str(BUILD_IMAGES))
        self.assertEqual((code, [e["runner"] for e in out["build_matrix"]]), (0, ["macos-15", "macos-15-intel", "ubuntu-22.04-arm", "ubuntu-22.04"]))
        self.assertEqual(list(out), ["build_matrix"])

        code, out, err = self.verify_select("--package", "biscuit-hash")
        self.assertEqual((code, out["status"]), (2, "invalid"))
        self.assertIn("--package: 'biscuit-hash' is not one of the packages that ship a binary", err)
        code, _, err = self.verify_select("--package", "biscuit-hash-cli", "--binstall")
        self.assertEqual(code, 2)
        self.assertIn("while the binstall channel is enabled", err)
        images = d.load_json(BUILD_IMAGES.read_bytes(), source="i")
        images["targets"]["x86_64-unknown-linux-gnu"]["runner"] = " "
        code, _, err = self.run_main("build-matrix", "--selection", str(self.dir / "verify.json"), "--images", str(self.write("images.json", images)))
        self.assertEqual(code, 2)
        self.assertIn("images.json: targets.x86_64-unknown-linux-gnu.runner: must not be empty", err)

    def test_bundle_and_check_bundle(self) -> None:
        prov = built_assets(self)
        selection = self.write("selection.json", prov.selection.record)
        envs = []
        for env in prov.environments:
            envs += ["--build-env", str(self.write(f"{env['target']}.json", env))]
        bundle = self.dir / "bundle"
        base = ("bundle", "--selection", str(selection), "--plan", str(PLAN_BH), "--assets-dir", str(prov.assets), *envs)
        code, out, err = self.run_main(*base, "--run-id", "x1", "--output-dir", str(bundle))
        self.assertEqual(code, 2)
        self.assertIn("--run-id: must be a positive workflow run id, found 'x1'", err)
        self.assertFalse(bundle.exists())
        code, out, _ = self.run_main(*base, "--run-id", str(RUN_ID), "--output-dir", str(bundle))
        self.assertEqual((code, out["status"], out["run_id"]), (0, "bundled", RUN_ID))

        check = ("check-bundle", "--bundle", str(bundle), "--selection", str(selection))
        code, out, _ = self.run_main(*check, "--current-run-id", str(RUN_ID))
        self.assertEqual((code, out["status"], out["reused"], len(out["assets"])), (0, "valid", False, 11))
        code, out, err = self.run_main(*check, "--current-run-id", "2002")
        self.assertEqual(code, 2)
        self.assertIn("a reused bundle needs that run's API documents (--run and --jobs)", err)
        run_docs = ("--run", str(self.write("run.json", run_document())), "--jobs", str(self.write("jobs.json", jobs_document())))
        code, out, _ = self.run_main(*check, "--current-run-id", "2002", *run_docs)
        self.assertEqual((code, out["reused"]), (0, True))
        self.write("jobs.json", jobs_document("cancelled"))
        code, _, err = self.run_main(*check, "--current-run-id", "2002", *run_docs)
        self.assertEqual(code, 2)
        self.assertIn("jobs.json: jobs[1].conclusion: is 'cancelled'", err)

    def current_root(self) -> Path:
        """A default-branch stand-in whose policy approves bh."""
        root = self.dir / "current"
        if root.exists():
            return root
        for relative in ("Cargo.toml", "schematic/schema/Cargo.toml", "darkmatter/dmls/zed-dmls/Cargo.toml"):
            (root / relative).parent.mkdir(parents=True, exist_ok=True)
            (root / relative).write_bytes((REPO / relative).read_bytes())
        (root / "docs").mkdir()
        (root / "docs/deployment.yaml").write_text(approve(POLICY_TEXT, "biscuit-hash-cli"), encoding="utf-8")
        return root

    def publish(self, selection: Path, bundle: Path, env: dict) -> tuple[int, dict, str]:
        return self.run_main("publish", "--selection", str(selection), "--bundle", str(bundle), "--current-root", str(self.current_root()),
                             "--current-metadata", str(self.dir / "metadata.json"), env=env)

    def test_publish_refuses_verification_and_foreign_repositories_before_any_network(self) -> None:
        env = {"GITHUB_REPOSITORY": d.CANONICAL_REPOSITORY, "GITHUB_TOKEN": "t", "RELEASE_APP_BOT_LOGIN": "release-app[bot]"}
        absent = self.dir / "no-bundle"
        with mock.patch.object(d, "GitHubPublishForge", side_effect=AssertionError("no forge")), \
             mock.patch.object(d.urllib.request, "urlopen", side_effect=AssertionError("no network")):
            code, out, err = self.publish(self.write("verify.json", verification().record), absent, env)
            self.assertEqual((code, out["status"]), (2, "invalid"))
            self.assertIn("verify.json: mode: is 'verify'; verification builds are never published", err)
            # A forged flag fails earlier, at the record itself.
            code, _, err = self.publish(self.write("forged.json", verification().record | {"publishable": True}), absent, env)
            self.assertEqual(code, 2)
            self.assertIn("publishable: must be false for mode 'verify'", err)
            release_selection = self.write("release.json", selection_for("biscuit-hash-cli").record)
            for repository in ("someone/fork", ""):
                with self.subTest(repository=repository):
                    code, _, err = self.publish(release_selection, absent, env | {"GITHUB_REPOSITORY": repository})
                    self.assertEqual(code, 2)
                    self.assertIn(f"GITHUB_REPOSITORY: is {repository!r}; only {d.CANONICAL_REPOSITORY} publishes", err)
            code, _, err = self.publish(release_selection, absent, {k: v for k, v in env.items() if k != "GITHUB_TOKEN"})
            self.assertIn("GITHUB_TOKEN: is required", err)

    def test_publish_exit_codes_through_the_command(self) -> None:
        prov = built_assets(self)
        bundle = self.dir / "bundle"
        d.write_bundle(prov.selection, prov.plan, prov.assets, prov.environments, RUN_ID, bundle)
        selection = self.write("release.json", prov.selection.record)
        env = {"GITHUB_REPOSITORY": d.CANONICAL_REPOSITORY, "GITHUB_TOKEN": "t"}
        conflicted = FakePublishForge(prov.selection, {"sha256.sum": b"different"})
        with mock.patch.object(d, "GitHubPublishForge", return_value=conflicted):
            code, out, _ = self.publish(selection, bundle, env)
        self.assertEqual((code, out["status"], out["conflicts"], conflicted.uploads()), (1, "refused", ["sha256.sum"], []))
        pending = FakePublishForge(prov.selection)
        pending.proposals[TAG_SHA] = d.PendingSetup("RELEASE_APP_BOT_LOGIN", "configure it")
        with mock.patch.object(d, "GitHubPublishForge", return_value=pending):
            code, out, _ = self.publish(selection, bundle, env)
        self.assertEqual((code, out["status"], pending.uploads()), (3, "pending-setup", []))
        fresh = FakePublishForge(prov.selection)
        with mock.patch.object(d, "GitHubPublishForge", return_value=fresh) as factory:
            code, out, _ = self.publish(selection, bundle, env)
        self.assertEqual((code, out["status"], out["uploaded"][-1]), (0, "published", d.PROVENANCE_NAME))
        self.assertEqual(factory.call_args.kwargs["repository"], d.CANONICAL_REPOSITORY)



# ---------------------------------------------------------------------------
# Input robustness: one shape matrix per shared validator, then one valid
# control and one invalid shape per wired field, at public results
# ---------------------------------------------------------------------------


ABSENT = object()  # a field missing from its mapping



class FieldValidatorTests(unittest.TestCase):
    """Every shared validator against absent, null, wrong whole type, and
    empty; collections also against one invalid member in a mixed list and
    all-invalid members. The wired-field tests below prove each field reaches
    its validator."""

    def check(self, read, value, message: str | None, expected=None) -> None:
        fields = d.Fields(d.Document("doc", {"k": value} if value is not ABSENT else {}))
        if message is None:
            self.assertEqual(read(fields, ["k"]), value if expected is None else expected)
            return
        with self.assertRaises(d.DistributionError) as caught:
            read(fields, ["k"])
        self.assertRegex(str(caught.exception), r"^doc: k(\[\d+\])?: ")
        self.assertIn(message, str(caught.exception))

    def matrix(self, read, cases) -> None:
        for value, message in cases:
            with self.subTest(value=value):
                self.check(read, value, message)

    def test_required_nonempty_string(self) -> None:
        self.matrix(lambda f, p: f.string(p), (
            ("bh", None), (ABSENT, "k: is required"), (None, "k: must not be null"), (7, "must be a string, found integer"),
            (True, "found boolean"), (["bh"], "found list"), ({}, "found mapping"), ("", "must not be empty"), ("  ", "must not be empty")))

    def test_string_list(self) -> None:
        self.matrix(lambda f, p: f.strings(p, allow_empty=False), (
            (["a", "b"], None), (ABSENT, "k: is required"), (None, "must not be null"), ("a", "must be a list, found string"),
            ({}, "must be a list, found mapping"), ([], "must not be empty"),
            (["a", 7], "k[1]: must be a string, found integer"), (["a", ""], "k[1]: must not be empty"),
            ([7, None], "k[0]: must be a string, found integer"), ([None, None], "k[0]: must not be null")))
        self.check(lambda f, p: f.strings(p, allow_empty=True), [], None)

    def test_record_list(self) -> None:
        self.matrix(lambda f, p: f.records(p, allow_empty=False), (
            ([{"a": 1}], None), (ABSENT, "k: is required"), (None, "must not be null"), ({"a": 1}, "must be a list, found mapping"),
            ("x", "found string"), ([], "must not be empty"),
            ([{"a": 1}, 3], "k[1]: must be a mapping, found integer"), ([None, "x"], "k[0]: must be a mapping, found null")))
        self.check(lambda f, p: f.records(p, allow_empty=True), [], None)

    def test_mapping(self) -> None:
        self.matrix(lambda f, p: f.mapping(p), (
            ({"a": 1}, None), (ABSENT, "k: is required"), (None, "must not be null"), ([], "must be a mapping, found list"),
            ("x", "found string"), (1, "found integer")))
        # An empty mapping is a valid shape (`packages: {}`, no channels); a
        # caller that needs members says so, as build-env `image` does.
        self.check(lambda f, p: f.mapping(p), {}, None)

    def test_boolean(self) -> None:
        self.matrix(lambda f, p: f.boolean(p), (
            (True, None), (False, None), (ABSENT, "k: is required"), (None, "must not be null"),
            ("true", "must be true or false, found string 'true'"), (1, "found integer 1"), ([], "found list"), ("", "found string ''")))
        # Only an absent field takes a default; an explicit null never does.
        self.check(lambda f, p: f.boolean(p, default=False), ABSENT, None, expected=False)
        self.check(lambda f, p: f.boolean(p, default=False), None, "must not be null")

    def test_integer(self) -> None:
        self.matrix(lambda f, p: f.integer(p), (
            (42, None), (ABSENT, "k: is required"), (None, "must not be null"), ("42", "must be an integer, found string"),
            (True, "found boolean"), (4.0, "found number"), ("", "found string")))

    def test_commit_id(self) -> None:
        self.matrix(lambda f, p: f.sha(p), (
            (TAG_SHA, None), (ABSENT, "k: is required"), (None, "must not be null"), (1, "must be a string"),
            ("", "must not be empty"), ("abc123", "full 40-digit commit SHA"), ("A" * 40, "full 40-digit"), ("1" * 41, "full 40-digit")))

    def test_semver(self) -> None:
        read = lambda f, p: str(f.version(p))  # noqa: E731
        self.matrix(read, (
            ("0.1.0", None), ("1.0.0-rc.1", None), (ABSENT, "k: is required"), (None, "must not be null"), (1, "must be a string"),
            ("", "must not be empty"), ("1.2", "complete SemVer"), ("v1.0.0", "complete SemVer"), ("01.0.0", "complete SemVer")))

    def test_asset_name(self) -> None:
        self.matrix(lambda f, p: f.asset_name(p), (
            ("bh", None), ("biscuit-hash-cli-x86_64-apple-darwin.tar.xz", None), (ABSENT, "k: is required"), (None, "must not be null"),
            (7, "must be a string"), ("", "must not be empty"), ("bin/bh", "must be a single file name"),
            ("../bh", "single file name"), ("..", "single file name"), (".", "single file name"), ("a\\b", "single file name")))

    def test_a_wrong_shaped_ancestor_is_located_at_the_ancestor(self) -> None:
        for root, path, message in ((None, ["k"], "doc: must be a mapping, found null"),
                                    ([], ["k"], "doc: must be a mapping, found list"),
                                    ({"r": None}, ["r", "id"], "doc: r: must be a mapping, found null"),
                                    ({"r": {}}, ["r", "a", "b"], "doc: r.a: is required"),
                                    ({"r": {"a": "x"}}, ["r", "a", 0], "doc: r.a: must be a list, found string")):
            with self.subTest(root=root, path=path), self.assertRaises(d.DistributionError) as caught:
                d.Fields(d.Document("doc", root)).string(path)
            self.assertEqual(str(caught.exception), message)
        # A defaulted boolean under a null parent is not silently its default.
        with self.assertRaisesRegex(d.DistributionError, "^doc: r: must be a mapping, found null$"):
            d.Fields(d.Document("doc", {"r": None})).boolean(["r", "approved"], default=False)


class LoaderGapTests(unittest.TestCase):
    """Loader shapes not already in the YAML, JSON, and TOML matrices above."""

    def test_json(self) -> None:
        self.assertEqual(d.load_json('{"n": "\\u00fc\\ud83d\\ude00", "m": "ü"}'.encode("utf-8"), source="j"), {"n": "ü\U0001F600", "m": "ü"})
        for bad, message in (('{"a": -Infinity}', "-Infinity"), (b'{"a": "\xff"}', "can't decode byte 0xff"), ("", "Expecting value"),
                             ('{"a": {"b": 1, "b": 1}}', "duplicate object key 'b'"), ("[1] [2]", "Extra data")):
            with self.subTest(bad=bad), self.assertRaisesRegex(d.DistributionError, "^j: " + re.escape("invalid JSON") + ".*" + re.escape(message)):
                d.load_json(bad, source="j")

    def test_a_null_json_root_where_a_mapping_is_required_is_located(self) -> None:
        for read, source in ((lambda v: d.Selection.from_json(v, source="selection.json"), "selection.json"),
                             (lambda v: d.validate_plan(selection_for("biscuit-hash-cli"), v), "dist plan"),
                             (lambda v: d.remote_assets(v, source="assets.json"), "assets.json"),
                             (lambda v: d.FixtureForge(v, source="forge.json"), "forge.json"),
                             (lambda v: d.load_workspace("null", source="metadata.json"), "metadata.json")):
            with self.subTest(source=source), self.assertRaises(d.DistributionError) as caught:
                read(None)
            self.assertEqual(str(caught.exception), f"{source}: must be a mapping, found null")

    def test_toml(self) -> None:
        self.assertEqual(d.load_toml(b"[workspace]\nexclude = []\n", source="t"), {"workspace": {"exclude": []}})
        self.assertEqual(d.load_toml("# only a comment\n", source="t"), {})
        for bad, message in (("a.b = 1\n[a]\nb = 2\n", "invalid TOML"), ("[a]\nb = 1\n[a.b]\n", "invalid TOML"),
                             ('a = "x"\n[[a]]\n', "invalid TOML"), (b"a = '\xff'\n", "not UTF-8")):
            with self.subTest(bad=bad), self.assertRaisesRegex(d.DistributionError, "^t: " + re.escape(message)):
                d.load_toml(bad, source="t")

    def test_yaml_is_utf8(self) -> None:
        with self.assertRaisesRegex(d.DistributionError, "^doc.yaml: not UTF-8"):
            d.load_yaml(b"packages: {a: '\xff'}\n", source="doc.yaml")


def metadata_with(edit) -> str:
    """The real `cargo metadata` with one edit to biscuit-hash-cli's entry."""
    document = json.loads(METADATA)
    edit(document, next(p for p in document["packages"] if p["name"] == "biscuit-hash-cli"))
    return json.dumps(document)


class WiredFieldTests(unittest.TestCase):
    """Each load-bearing field: the unedited real document is the valid
    control, and one invalid shape proves the field reaches its validator."""

    def assertCases(self, read, cases) -> None:
        for edit, message in cases:
            with self.subTest(message=message), self.assertRaisesRegex(d.DistributionError, re.escape(message)):
                read(edit)

    def test_metadata_fields(self) -> None:
        bh_index = [p["name"] for p in json.loads(METADATA)["packages"]].index("biscuit-hash-cli")
        at = f"packages[{bh_index}]"
        load = lambda edit: d.load_workspace(metadata_with(edit), source="m", excluded=EXCLUDED)  # noqa: E731
        self.assertEqual(load(lambda doc, pkg: None).packages["biscuit-hash-cli"], WORKSPACE.packages["biscuit-hash-cli"])
        self.assertCases(load, (
            (lambda doc, pkg: doc.update(workspace_root=None), "m: workspace_root: must not be null"),
            (lambda doc, pkg: doc.update(workspace_members=[]), "m: workspace_members: must not be empty"),
            (lambda doc, pkg: doc.update(packages={}), "m: packages: must be a list, found mapping"),
            (lambda doc, pkg: pkg.update(id=7), f"m: {at}.id: must be a string"),
            (lambda doc, pkg: pkg.update(name=""), f"m: {at}.name: must not be empty"),
            (lambda doc, pkg: pkg.update(manifest_path=None), f"m: {at}.manifest_path: must not be null"),
            (lambda doc, pkg: pkg.update(manifest_path="/elsewhere/Cargo.toml"), f"m: {at}.manifest_path: is outside the workspace root"),
            (lambda doc, pkg: pkg.update(description=7), f"m: {at}.description: must be a string"),
            (lambda doc, pkg: pkg.update(homepage=""), f"m: {at}.homepage: must not be empty"),
            (lambda doc, pkg: pkg.pop("metadata"), f"m: {at}.metadata: is required"),
            (lambda doc, pkg: pkg["targets"][0].update(kind="bin"), f"m: {at}.targets[0].kind: must be a list, found string"),
            (lambda doc, pkg: pkg["targets"][0].update(name=None), f"m: {at}.targets[0].name: must not be null"),
            (lambda doc, pkg: pkg["targets"][0].update({"required-features": None}), f"m: {at}.targets[0].required-features: must not be null"),
            (lambda doc, pkg: pkg["targets"][0].update({"required-features": ["ok", 3]}), f"m: {at}.targets[0].required-features[1]: must be a string"),
            (lambda doc, pkg: doc["packages"].append(dict(pkg)), "duplicate workspace package 'biscuit-hash-cli'"),
        ))

    def test_binstall_settings(self) -> None:
        select = lambda metadata: d.select_verification("biscuit-hash-cli", with_package("biscuit-hash-cli", metadata=metadata), TAG_SHA, {})  # noqa: E731
        disabled = {"binstall": {**d.BINSTALL_URLS, "disabled-strategies": list(d.BINSTALL_DISABLED_STRATEGIES)}}
        self.assertEqual(select(disabled).status, d.SELECTED)
        self.assertCases(select, (
            ({"binstall": None}, "binstall: must not be null"),
            ({"binstall": {**disabled["binstall"], "disabled-strategies": "compile"}}, "disabled-strategies: must be a list, found string"),
            ({"binstall": {**disabled["binstall"], "pkg-fmt": 1}}, "binstall.pkg-fmt: must be a string"),
        ))

    def test_release_event_fields(self) -> None:
        tag = GatherFactsTests.TAG
        gather = lambda edit: d.gather_facts("release", edited(release_event(tag), edit), source="event.json",  # noqa: E731
                                             repository=d.CANONICAL_REPOSITORY, forge=GatherFactsTests().forge(),
                                             checkout_commit=TAG_SHA, tagged=WORKSPACE)
        self.assertEqual(gather(lambda e: None), facts())
        self.assertCases(gather, (
            (lambda e: e.update(action=7), "event.json: action: must be a string"),
            (lambda e: e.update(release=None), "event.json: release: must be a mapping, found null"),
            (lambda e: e["release"].pop("tag_name"), "event.json: release.tag_name: is required"),
            (lambda e: e["release"].update(tag_name=None), "release.tag_name: must not be null"),
            (lambda e: e["release"].update(tag_name=7), "release.tag_name: must be a string"),
            (lambda e: e["release"].update(tag_name=""), "release.tag_name: must not be empty"),
            (lambda e: e["release"].update(tag_name="biscuit-hash-cli-v0.1"), "tag biscuit-hash-cli-v0.1: must be a complete SemVer"),
            (lambda e: e["release"].update(id="42"), "release.id: must be an integer"),
            (lambda e: e["release"].update(prerelease=None), "release.prerelease: must not be null"),
            (lambda e: e["release"].update(html_url=""), "release.html_url: must not be empty"),
            (lambda e: e.update(repository=None), "event.json: repository: must be a mapping, found null"),
        ))
        with self.assertRaisesRegex(d.DistributionError, "^event.json: inputs.tag: must not be empty"):
            d.gather_facts("workflow_dispatch", {"inputs": {"tag": ""}}, source="event.json", repository=d.CANONICAL_REPOSITORY,
                           forge=FakeForge(), checkout_commit=TAG_SHA, tagged=WORKSPACE)

    def test_forge_fixture_fields(self) -> None:
        tag = GatherFactsTests.TAG
        good = {"releases": {tag: release_event(tag)["release"]}, "tags": {tag: TAG_SHA},
                "proposals": {TAG_SHA: {"number": 17, "head_sha": TAG_SHA, "merge_sha": MERGE_SHA}}}

        def read(edit):
            forge = d.FixtureForge(edited(good, edit), source="forge.json")
            return forge.release(tag), forge.tag_commit(tag), forge.proposal(TAG_SHA)

        self.assertEqual(read(lambda f: None), (release(tag), TAG_SHA, d.ProposalProof(17, TAG_SHA, MERGE_SHA)))
        self.assertCases(read, (
            (lambda f: f.pop("proposals"), "forge.json: proposals: is required"),
            (lambda f: f["releases"][tag].update(id=None), f"releases.{tag}.id: must not be null"),
            (lambda f: f["releases"][tag].update(draft="false"), f"releases.{tag}.draft: must be true or false"),
            (lambda f: f["tags"].update({tag: "abc"}), f"tags.{tag}: must be a full 40-digit commit SHA"),
            (lambda f: f["proposals"][TAG_SHA].update(number="17"), "number: must be an integer"),
            (lambda f: f["proposals"][TAG_SHA].update(head_sha=TAG_SHA[:7]), "head_sha: must be a full 40-digit"),
            (lambda f: f["proposals"][TAG_SHA].pop("merge_sha"), "merge_sha: is required"),
            (lambda f: f["proposals"].update({TAG_SHA: "pendng"}), f"proposals.{TAG_SHA}: must be a mapping, found string"),
        ))

    def test_selection_fields(self) -> None:
        record = selection_for("biscuit-hash-cli").record
        load = lambda edit: d.Selection.from_json(edited(record, edit), source="s")  # noqa: E731
        self.assertEqual(load(lambda r: None).record, record)
        self.assertCases(load, (
            # Target identifiers: the whole matrix, at the field every build step reads.
            (lambda r: r.pop("targets"), "s: targets: is required"),
            (lambda r: r.update(targets=None), "s: targets: must not be null"),
            (lambda r: r.update(targets="x86_64-unknown-linux-gnu"), "s: targets: must be a list, found string"),
            (lambda r: r.update(targets=[]), "s: targets: must not be empty"),
            (lambda r: r.update(targets=[d.TARGETS[0], 7]), "s: targets[1]: must be a string"),
            (lambda r: r.update(targets=[7, None]), "s: targets[0]: must be a string"),
            (lambda r: r.update(targets=["x86_64-pc-windows-msvc"]), "s: targets: must be"),
            (lambda r: r.update(targets=list(reversed(d.TARGETS))), "s: targets: must be"),
            (lambda r: r.update(manifest_path=7), "s: manifest_path: must be a string"),
            (lambda r: r.update(version="0.1"), "s: version: must be a complete SemVer"),
            (lambda r: r.update(tag=None), "s: tag: must not be null"),
            (lambda r: r.update(formula_update="false"), "s: formula_update: must be true or false"),
            (lambda r: r.update(channels=None), "s: channels: must not be null"),
            (lambda r: r.update(channels={"brew": ""}), "s: channels.brew: must not be empty"),
        ))

    def test_plan_target_and_asset_path_fields(self) -> None:
        plan = d.load_json(PLAN_BH.read_bytes(), source="plan")
        selection = selection_for("biscuit-hash-cli")
        archive = "biscuit-hash-cli-aarch64-apple-darwin.tar.xz"
        validate = lambda edit: d.validate_plan(selection, edited(plan, edit))  # noqa: E731
        validate(lambda p: None)
        bh = lambda p: next(a for a in p["artifacts"][archive]["assets"] if a["kind"] == "executable")  # noqa: E731
        self.assertCases(validate, (
            (lambda p: p["artifacts"][archive].update(target_triples=[]), f"artifacts.{archive}.target_triples: must not be empty"),
            (lambda p: p["artifacts"][archive].update(target_triples=None), "target_triples: must not be null"),
            (lambda p: bh(p).update(path="../bh"), "path: must be a single file name, found '../bh'"),
            (lambda p: bh(p).update(path="bin/bh"), "path: must be a single file name"),
            (lambda p: bh(p).update(path="bf"), "path: must be 'bh', at the archive's top level"),
            (lambda p: bh(p).pop("path"), "path: is required"),
            (lambda p: bh(p).update(name="/bh"), "name: must be a single file name"),
        ))

    def test_remote_asset_fields(self) -> None:
        good = {"name": "a.tar.xz", "size": 1, "id": 2, "browser_download_url": "https://example.invalid/a", "state": "uploaded"}
        load = lambda edit: d.remote_assets([edited(good, edit)], source="assets.json")  # noqa: E731
        # Unknown GitHub fields (`state`) are ignored.
        self.assertEqual(load(lambda a: None), [d.RemoteAsset("a.tar.xz", 1, 2, "https://example.invalid/a")])
        self.assertCases(load, (
            (lambda a: a.update(name="dir/a.tar.xz"), "assets.json: [0].name: must be a single file name"),
            (lambda a: a.update(name=""), "[0].name: must not be empty"),
            (lambda a: a.update(size=-1), "[0].size: must not be negative"),
            (lambda a: a.update(id="2"), "[0].id: must be an integer"),
            (lambda a: a.update(browser_download_url=None), "[0].browser_download_url: must not be null"),
        ))
        with self.assertRaisesRegex(d.DistributionError, r"^assets.json: \[1\]: must be a mapping, found string"):
            d.remote_assets([good, "b.tar.xz"], source="assets.json")
        with self.assertRaisesRegex(d.DistributionError, r"^assets.json: \[0\]: must be a mapping, found integer"):
            d.remote_assets([1, 2], source="assets.json")

    def test_build_environment_fields(self) -> None:
        prov = ProvenanceTests("test_provenance_is_deterministic_and_complete")
        prov.setUp()
        self.addCleanup(prov.doCleanups)
        envs = prov.environments
        build = lambda last: d.build_provenance(prov.selection, prov.plan, prov.assets, envs[:3] + [last])  # noqa: E731
        build(envs[3])
        self.assertCases(build, (
            ({k: v for k, v in envs[3].items() if k != "target"}, "build environment 3: target: is required"),
            (dict(envs[3], target=None), "build environment 3: target: must not be null"),
            (dict(envs[3], target=""), "build environment 3: target: must not be empty"),
            (dict(envs[3], image={}), "build environment 3: image: must not be empty"),
            (dict(envs[3], rustc=7), "build environment 3: rustc: must be a string"),
        ))
        with self.assertRaisesRegex(d.DistributionError, "^build environment 0: must be a mapping, found null"):
            d.build_provenance(prov.selection, prov.plan, prov.assets, [None])

    def test_root_manifest_exclude_field(self) -> None:
        self.assertEqual(d.excluded_package_names("[workspace]\nmembers = []\n", _read), set())
        for text, message in (("[workspace]\nexclude = \"zed\"\n", "workspace.exclude: must be a list"),
                              ("[workspace]\nexclude = [\"no/such/dir\"]\n", "workspace.exclude[0]: no/such/dir/Cargo.toml does not exist"),
                              ("[workspace]\nexclude = [1]\n", "workspace.exclude[0]: must be a string"),
                              ("workspace = 1\n", "workspace: must be a mapping, found integer"),
                              ("[package]\nname = \"x\"\n", "workspace: is required")):
            with self.subTest(text=text), self.assertRaisesRegex(d.DistributionError, "^Cargo.toml: " + re.escape(message)):
                d.excluded_package_names(text, _read)


class ExceptionContractTests(unittest.TestCase):
    """The only places where absence or emptiness is not an error, and the
    nearby cases that stay errors."""

    def test_an_empty_policy_is_valid_and_ignores_every_release(self) -> None:
        empty = policy("packages: {}\n")
        self.assertEqual(empty.entries, {})
        result = d.select_release(facts(), d.Policies(current=empty, tagged=empty), WORKSPACE)
        self.assertEqual((result.status, result.as_json()["build_matrix"]), (d.IGNORED, []))
        self.assertIn("not approved", result.reason)

    def test_empty_collections_on_a_selected_distribution_are_errors(self) -> None:
        prov = built_assets(self)
        with self.assertRaisesRegex(d.DistributionError, "build environments: must cover"):
            d.build_provenance(prov.selection, prov.plan, prov.assets, [])
        empty_dir = Path(tempfile.mkdtemp())
        self.addCleanup(empty_dir.rmdir)
        with self.assertRaisesRegex(d.DistributionError, "must hold exactly the planned assets; unexpected \\[\\], missing"):
            d.build_provenance(prov.selection, prov.plan, empty_dir, prov.environments)
        forge = FakePublishForge(prov.selection)
        with self.assertRaisesRegex(d.DistributionError, "^assets: must be exactly the bundle's"):
            d.publish_release_assets(prov.selection, {}, forge, policy(approve(POLICY_TEXT, "biscuit-hash-cli"), "current"), fresh_rebuild=False)
        self.assertEqual(forge.calls, [])

    def test_a_404_is_an_error_for_the_proof_and_the_release_assets(self) -> None:
        tag = GatherFactsTests.TAG
        listing = f"/repos/{d.CANONICAL_REPOSITORY}/commits/{TAG_SHA}/pulls"
        with self.assertRaisesRegex(d.DistributionError, "unexpected HTTP 404"):
            StubbedGitHubForge({}).proposal(TAG_SHA)
        with self.assertRaisesRegex(d.DistributionError, "pulls/17: unexpected HTTP 404"):
            StubbedGitHubForge({listing: (200, [{"number": 17}])}).proposal(TAG_SHA)
        forge = StubbedPublishGitHub({})
        forge._get = lambda path: (404, None)  # type: ignore[method-assign]
        with self.assertRaisesRegex(d.DistributionError, "releases/42/assets.*unexpected HTTP 404"):
            forge.list_assets(42)
        # A Release lookup's 404 reads as absence only to be refused: the
        # selected tag must have its Release.
        self.assertIsNone(StubbedGitHubForge({}).release(tag))
        with self.assertRaisesRegex(d.DistributionError, f"^release {tag}: no GitHub Release exists"):
            d.select_release(facts(release=None), policies(approve(POLICY_TEXT, "biscuit-hash-cli")), WORKSPACE)

    def test_uploading_without_a_token_is_refused_before_any_request(self) -> None:
        forge = d.GitHubPublishForge(repository=d.CANONICAL_REPOSITORY, token=None, bot_login="release-app[bot]", repo=REPO)
        with mock.patch.object(d.urllib.request, "urlopen", side_effect=AssertionError("no network")), \
             self.assertRaisesRegex(d.DistributionError, "^GITHUB_TOKEN: is required to upload release assets"):
            forge.upload_asset(42, "a.tar.xz", REPO / "LICENSE")


class MissingInputTests(unittest.TestCase):
    """Missing or irregular files are located invalid input (exit 2), never a
    crash and never silently skipped."""

    def setUp(self) -> None:
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.dir = Path(tmp.name)

    def run_main(self, *argv: str) -> tuple[int, dict, str]:
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            code = d.main(list(argv), env={"GITHUB_REPOSITORY": d.CANONICAL_REPOSITORY})
        return code, json.loads(out.getvalue()), err.getvalue()

    def test_a_missing_or_empty_build_bundle_is_refused(self) -> None:
        selection = self.dir / "selection.json"
        selection.write_bytes(d.render_json(selection_for("biscuit-hash-cli").record))
        empty = self.dir / "empty"
        empty.mkdir()
        for bundle, message in ((self.dir / "absent", "absent: cannot read the build bundle"),
                                (empty, "empty: must hold exactly ['assets', 'build-env', 'bundle.json', 'plan.json', 'selection.json']")):
            with self.subTest(bundle=bundle.name):
                code, out, err = self.run_main("check-bundle", "--bundle", str(bundle), "--selection", str(selection), "--current-run-id", "1")
                self.assertEqual((code, out["status"]), (2, "invalid"))
                self.assertIn(message, err)

    def test_a_subdirectory_among_assets_is_refused_not_skipped(self) -> None:
        prov = built_assets(self)
        (prov.assets / "nested").mkdir()
        with self.assertRaisesRegex(d.DistributionError, "nested: must be a regular file"):
            d.build_provenance(prov.selection, prov.plan, prov.assets, prov.environments)
        built = self.dir / "built"
        (built / "nested").mkdir(parents=True)
        (built / "a.tar.xz").write_bytes(b"a")
        (self.dir / "assets.json").write_text("[]", encoding="utf-8")
        code, out, err = self.run_main("reconcile-assets", "--expected-dir", str(built), "--existing", str(self.dir / "assets.json"))
        self.assertEqual((code, out["status"]), (2, "invalid"))
        self.assertIn("built/nested: must be a regular file", err)
        (built / "nested").rmdir()
        code, out, _ = self.run_main("reconcile-assets", "--expected-dir", str(built), "--existing", str(self.dir / "assets.json"))
        self.assertEqual((code, out["upload"]), (0, ["a.tar.xz"]))

    def test_unreadable_inputs_are_invalid_not_a_crash(self) -> None:
        missing = self.dir / "missing"
        cases = (
            (("tap-update", "--candidate-version", "1.0.0", "--candidate-formula", str(missing)), "missing: cannot read"),
            (("reconcile-assets", "--expected-dir", str(missing), "--existing", str(missing)), "missing: cannot read"),
            (("validate-plan", "--selection", str(missing), "--plan", str(PLAN_BH)), "missing: cannot read"),
        )
        for argv, message in cases:
            with self.subTest(command=argv[0]):
                code, out, err = self.run_main(*argv)
                self.assertEqual((code, out["status"]), (2, "invalid"))
                self.assertIn(message, err)


class LoaderWiringTests(PreflightFixture, unittest.TestCase):
    """Per loader through `main`: a positive fixture, and a located malformed
    case after which no adapter (forge, network, subprocess) was launched."""

    def setUp(self) -> None:
        super().setUp()
        self.fixture_forge = self.enterContext(mock.patch.object(d, "FixtureForge", wraps=d.FixtureForge))
        for target, name in ((d, "GitHubForge"), (d, "GitHubPublishForge"), (d.urllib.request, "urlopen"), (d.subprocess, "run")):
            self.enterContext(mock.patch.object(target, name, side_effect=AssertionError(f"{name} launched")))

    def assertRefusedBeforeAdapters(self, result: tuple[int, str, str], message: str) -> None:
        code, out, err = result
        self.assertEqual((code, json.loads(out)["status"]), (2, "invalid"))
        self.assertRegex(err, message)
        self.assertEqual(self.fixture_forge.call_count, 0)

    def test_yaml_policy(self) -> None:
        code, out, _ = self.preflight()
        self.assertEqual((code, json.loads(out)["status"], self.fixture_forge.call_count), (0, "selected", 1))
        self.fixture_forge.reset_mock()
        (self.dir / "bad.yaml").write_text(POLICY_TEXT + "packages: {}\n", encoding="utf-8")
        self.assertRefusedBeforeAdapters(self.preflight("bad.yaml"), r"bad\.yaml:\d+:\d+: packages: duplicate key")
        self.assertRefusedBeforeAdapters(self.preflight("absent.yaml"), r"absent\.yaml: cannot read")

    def test_json_event_and_metadata(self) -> None:
        (self.dir / "event.json").write_text(json.dumps(release_event(self.TAG)) + " trailing", encoding="utf-8")
        self.assertRefusedBeforeAdapters(self.preflight(), r"event\.json: invalid JSON: Extra data")
        (self.dir / "event.json").write_text(json.dumps(release_event(self.TAG)), encoding="utf-8")
        (self.dir / "metadata.json").write_text(metadata_with(lambda doc, pkg: pkg.update(version=None)), encoding="utf-8")
        self.assertRefusedBeforeAdapters(self.preflight(), r"metadata\.json: packages\[\d+\]\.version: must not be null")

    def test_json_forge_fixture(self) -> None:
        (self.dir / "forge.json").write_text('{"releases": {}, "tags": {}, "proposals": {}, "tags": {}}', encoding="utf-8")
        self.assertRefusedBeforeAdapters(self.preflight(), r"forge\.json: invalid JSON: duplicate object key 'tags'")

    def test_toml_root_manifest(self) -> None:
        current = self.dir / "current"
        current.mkdir()
        (current / "Cargo.toml").write_text("[workspace]\nexclude = [\n", encoding="utf-8")
        argv = ["preflight", "--event-name", "release", "--event-path", str(self.dir / "event.json"),
                "--tagged-root", str(REPO), "--current-root", str(current),
                "--tagged-metadata", str(self.dir / "metadata.json"), "--current-metadata", str(self.dir / "metadata.json"),
                "--tagged-policy", str(self.dir / "approved.yaml"), "--current-policy", str(self.dir / "approved.yaml"),
                "--forge-fixture", str(self.dir / "forge.json"), "--checkout-commit", TAG_SHA]
        self.assertRefusedBeforeAdapters(self.run_main(*argv), r"current/Cargo\.toml: invalid TOML")
        (current / "Cargo.toml").unlink()
        self.assertRefusedBeforeAdapters(self.run_main(*argv), r"current/Cargo\.toml: cannot read")
        (current / "Cargo.toml").write_bytes((REPO / "Cargo.toml").read_bytes())
        self.assertRefusedBeforeAdapters(self.run_main(*argv), r"workspace\.exclude\[\d+\]: .* does not exist")

    def test_json_selection_and_plan(self) -> None:
        selection = self.dir / "selection.json"
        selection.write_bytes(d.render_json(selection_for("biscuit-hash-cli").record))
        code, out, _ = self.run_main("validate-plan", "--selection", str(selection), "--plan", str(PLAN_BH))
        self.assertEqual((code, json.loads(out)), (0, {"status": "valid", "tag": "biscuit-hash-cli-v0.1.0"}))
        plan = self.dir / "plan.json"
        plan.write_text(PLAN_BH.read_text(encoding="utf-8").replace('"dist_version"', '"dist_version": NaN, "x"', 1), encoding="utf-8")
        self.assertRefusedBeforeAdapters(self.run_main("validate-plan", "--selection", str(selection), "--plan", str(plan)),
                                         r"plan\.json: invalid JSON: invalid JSON constant NaN")
        selection.write_text('{"status": "selected"', encoding="utf-8")
        self.assertRefusedBeforeAdapters(self.run_main("validate-plan", "--selection", str(selection), "--plan", str(PLAN_BH)),
                                         r"selection\.json: invalid JSON")

    def test_publish_inputs_launch_no_forge(self) -> None:
        prov = built_assets(self)
        bundle = self.dir / "bundle"
        d.write_bundle(prov.selection, prov.plan, prov.assets, prov.environments, RUN_ID, bundle)
        selection = self.dir / "release.json"
        selection.write_bytes(d.render_json(prov.selection.record))
        current = self.dir / "current"
        for relative in ("Cargo.toml", "schematic/schema/Cargo.toml", "darkmatter/dmls/zed-dmls/Cargo.toml"):
            (current / relative).parent.mkdir(parents=True, exist_ok=True)
            (current / relative).write_bytes((REPO / relative).read_bytes())
        (current / "docs").mkdir()
        (current / "docs/deployment.yaml").write_text(edit_entry(approve(POLICY_TEXT, "biscuit-hash-cli"), "biscuit-hash-cli", "brew", "null"), encoding="utf-8")

        def publish(selection_path: Path, bundle_path: Path) -> tuple[int, str, str]:
            out, err = io.StringIO(), io.StringIO()
            with redirect_stdout(out), redirect_stderr(err):
                code = d.main(["publish", "--selection", str(selection_path), "--bundle", str(bundle_path), "--current-root", str(current),
                               "--current-metadata", str(self.dir / "metadata.json")],
                              env={"GITHUB_REPOSITORY": d.CANONICAL_REPOSITORY, "GITHUB_TOKEN": "t"})
            return code, out.getvalue(), err.getvalue()

        self.assertRefusedBeforeAdapters(publish(selection, bundle),
                                         r"deployment\.yaml:\d+:\d+: packages\.biscuit-hash-cli\.brew: must be true, false, or a name; found null")
        (current / "docs/deployment.yaml").write_text(approve(POLICY_TEXT, "biscuit-hash-cli"), encoding="utf-8")
        self.assertRefusedBeforeAdapters(publish(selection, self.dir / "absent"), r"absent: cannot read the build bundle")
        broken = self.dir / "broken.json"
        broken.write_text('{"status": "selected", "status": "selected"}', encoding="utf-8")
        self.assertRefusedBeforeAdapters(publish(broken, bundle), r"broken\.json: invalid JSON: duplicate object key")


LEDGER_RUN_ID = 9001


def selected(package: str = "biscuit-hash-cli", *, prerelease: bool = False, **channels: str) -> d.Selection:
    tag = f"{package}-v{WORKSPACE.packages[package].version}"
    workspace = with_package(package, metadata=ENABLED_BINSTALL) if "binstall" in channels else WORKSPACE
    approved = approve(POLICY_TEXT, package, **channels)
    tagged = d.Policies(current=policy(approved, "current", workspace), tagged=policy(approved, "tagged", workspace))
    result = d.select_release(facts(tag, release=release(tag, prerelease=prerelease)), tagged, workspace)
    return d.Selection.from_json(result.as_json(), source="selection")


def bh_entry(**changes) -> dict:
    entry = d.rollout_candidate(selected(), run_id=LEDGER_RUN_ID, install="success", binstall="skipped", tap="skipped")
    entry.update(changes)
    return entry


def ledger_text(verified: list | None = None, pending: list | None = None, **top) -> str:
    doc = {"first_package": "biscuit-hash-cli", "verified": verified if verified is not None else [],
           "pending": pending if pending is not None else []}
    doc.update(top)
    return json.dumps(doc)


def ledger(text: str) -> d.RolloutLedger:
    return d.parse_rollout(text.encode("utf-8"), source="rollout.json")


def successful_run(**changes) -> dict:
    run = {"id": LEDGER_RUN_ID, "repository": {"full_name": d.CANONICAL_REPOSITORY}, "path": d.DISTRIBUTION_WORKFLOW,
           "status": "completed", "conclusion": "success", "event": "release"}
    run.update(changes)
    return run


class RolloutLedgerTests(unittest.TestCase):
    """`.github/distribution/rollout.json`: the shared validators' matrices
    run in `FieldValidatorTests`; here each field gets a control and one
    invalid shape, and the gate is exercised through its public results."""

    def test_the_real_ledger_records_pending_setup_and_permits_only_bh(self) -> None:
        real = d.parse_rollout((REPO / ".github" / "distribution" / "rollout.json").read_bytes(), source="rollout.json")
        self.assertEqual((real.first_package, real.verified), ("biscuit-hash-cli", ()))
        self.assertEqual({package for _, package in real.pending}, set(d.ROSTER))
        self.assertIn(("live-release", "biscuit-hash-cli"), real.pending)
        d.check_policy_rollout(policy(POLICY_PATH.read_text(encoding="utf-8")), real)
        self.assertIsNone(d.check_rollout("biscuit-hash-cli", real))
        with self.assertRaisesRegex(d.DistributionError, "biscuit-file-cli cannot be distributed before the first biscuit-hash-cli release is verified"):
            d.check_rollout("biscuit-file-cli", real)

    def test_a_candidate_round_trips_through_the_ledger_and_permits_widening(self) -> None:
        entry = bh_entry()
        self.assertEqual(entry["channels"], {"archive": "pass", "shell-installer": "pass", "brew": "disabled", "binstall": "disabled"})
        self.assertEqual(entry["run_url"], f"{d.CANONICAL_URL}/actions/runs/{LEDGER_RUN_ID}")
        text = ledger_text([entry])
        first = ledger(text)
        again = ledger(json.dumps(json.loads(text), indent=2))
        self.assertEqual(first.verified, again.verified)
        permitted = d.check_rollout("darkmatter-cli", first)
        self.assertEqual((permitted.tag, permitted.commit, permitted.run_id), (entry["tag"], TAG_SHA, LEDGER_RUN_ID))
        d.check_policy_rollout(policy(approve(POLICY_TEXT, "darkmatter-cli")), first)

    def test_the_newest_verified_first_package_entry_is_used(self) -> None:
        newer = bh_entry(version="0.2.0", tag="biscuit-hash-cli-v0.2.0", run_id=7,
                         run_url=f"{d.CANONICAL_URL}/actions/runs/7", release_url=f"{d.CANONICAL_URL}/releases/tag/biscuit-hash-cli-v0.2.0")
        self.assertEqual(d.check_rollout("git-wt-cli", ledger(ledger_text([bh_entry(), newer]))).run_id, 7)

    def test_empty_lists_are_valid_and_permit_nothing_but_bh(self) -> None:
        empty = ledger(ledger_text())
        self.assertEqual((empty.verified, empty.pending), ((), ()))
        with self.assertRaises(d.DistributionError):
            d.check_rollout("sniff-detection-cli", empty)

    def test_only_a_verified_first_package_entry_permits_widening(self) -> None:
        other = d.rollout_candidate(selected("biscuit-file-cli"), run_id=LEDGER_RUN_ID, install="success", binstall="skipped", tap="skipped")
        pending = [{"name": "live-release", "package": "biscuit-hash-cli", "reason": "not yet"}]
        with self.assertRaisesRegex(d.DistributionError, r"rollout\.json: .*no verified biscuit-hash-cli entry"):
            d.check_rollout("tree-hugger-cli", ledger(ledger_text([other], pending)))

    def test_an_approval_without_the_first_chain_is_located_at_the_approval(self) -> None:
        with self.assertRaisesRegex(d.DistributionError, r"current:\d+:\d+: packages\.claudine-cli\.approved: claudine-cli cannot be distributed"):
            d.check_policy_rollout(policy(approve(POLICY_TEXT, "claudine-cli"), "current"), ledger(ledger_text()))

    def test_every_invalid_field_shape_is_a_located_error(self) -> None:
        pending = {"name": "approval", "package": "biscuit-hash-cli", "reason": "r"}
        targets = {target: "pass" for target in d.TARGETS}
        cases = [
            (json.dumps({"verified": [], "pending": []}), r"rollout\.json: first_package: is required"),
            (ledger_text(first_package="biscuit-file-cli"), r"first_package: must be 'biscuit-hash-cli'"),
            (ledger_text(extra=1), r"rollout\.json: extra: unknown field"),
            (json.dumps({"first_package": "biscuit-hash-cli", "verified": None, "pending": []}), r"verified: must not be null"),
            (json.dumps({"first_package": "biscuit-hash-cli", "verified": {}, "pending": []}), r"verified: must be a list, found mapping"),
            (ledger_text([bh_entry(), 3]), r"verified\[1\]: must be a mapping, found integer"),
            (ledger_text([3]), r"verified\[0\]: must be a mapping"),
            (ledger_text([bh_entry(notes="x")]), r"verified\[0\]\.notes: unknown field"),
            (ledger_text([{k: v for k, v in bh_entry().items() if k != "commit"}]), r"verified\[0\]\.commit: is required"),
            (ledger_text([bh_entry(package="biscuit-hash")]), r"verified\[0\]\.package: 'biscuit-hash' does not ship a binary"),
            (ledger_text([bh_entry(version="0.1")]), r"verified\[0\]\.version"),
            (ledger_text([bh_entry(tag="biscuit-hash-cli-v9.9.9")]), r"verified\[0\]\.tag: must be biscuit-hash-cli-v0\.1\.0"),
            (ledger_text([bh_entry(commit="abc")]), r"verified\[0\]\.commit: must be a full 40-digit commit SHA"),
            (ledger_text([bh_entry(run_id="9001")]), r"verified\[0\]\.run_id: must be an integer, found string"),
            (ledger_text([bh_entry(run_id=0)]), r"verified\[0\]\.run_id: must be a positive run id"),
            (ledger_text([bh_entry(run_url="https://example.com/run")]), r"verified\[0\]\.run_url: must be https://github\.com/"),
            (ledger_text([bh_entry(release_url=f"{d.CANONICAL_URL}/releases/latest")]), r"verified\[0\]\.release_url: must be"),
            (ledger_text([bh_entry(targets={k: v for k, v in targets.items() if k != "x86_64-apple-darwin"})]), r"targets\.x86_64-apple-darwin: is required"),
            (ledger_text([bh_entry(targets={**targets, "x86_64-pc-windows-msvc": "pass"})]), r"targets\.x86_64-pc-windows-msvc: unknown field"),
            (ledger_text([bh_entry(targets={**targets, "x86_64-apple-darwin": "pending"})]), r"targets\.x86_64-apple-darwin: must be 'pass'"),
            (ledger_text([bh_entry(targets=None)]), r"verified\[0\]\.targets: must not be null"),
            (ledger_text([bh_entry(channels={**bh_entry()["channels"], "archive": "disabled"})]), r"channels\.archive: must be one of \['pass'\]"),
            (ledger_text([bh_entry(channels={**bh_entry()["channels"], "binstall": "unchanged"})]), r"channels\.binstall: must be one of"),
            (ledger_text([bh_entry(channels={**bh_entry()["channels"], "brew": False})]), r"channels\.brew: must be a string, found boolean"),
            (ledger_text([bh_entry(), bh_entry()]), r"verified\[1\]\.tag: biscuit-hash-cli-v0\.1\.0 is already recorded"),
            (json.dumps({"first_package": "biscuit-hash-cli", "verified": [], "pending": None}), r"pending: must not be null"),
            (ledger_text(pending=[{**pending, "name": "Not A Slug"}]), r"pending\[0\]\.name: must match"),
            (ledger_text(pending=[{**pending, "package": "playa"}]), r"pending\[0\]\.package: 'playa' does not ship a binary"),
            (ledger_text(pending=[{**pending, "reason": ""}]), r"pending\[0\]\.reason: must not be empty"),
            (ledger_text(pending=[pending, pending]), r"pending\[1\]\.name: 'approval' is already pending for biscuit-hash-cli"),
            ('{"first_package": "biscuit-hash-cli", "first_package": "biscuit-hash-cli", "verified": [], "pending": []}', r"duplicate object key"),
            (ledger_text() + "x", r"rollout\.json: invalid JSON"),
        ]
        ledger(ledger_text([bh_entry()], [pending]))  # the unedited control
        for text, message in cases:
            with self.subTest(message=message), self.assertRaisesRegex(d.DistributionError, message):
                ledger(text)

    def test_the_run_must_be_the_canonical_workflow_and_successful(self) -> None:
        entry = ledger(ledger_text([bh_entry()])).verified[0]
        d.check_rollout_run(successful_run(), entry, source="run")
        d.check_rollout_run(successful_run(path=f"{d.DISTRIBUTION_WORKFLOW}@refs/heads/main"), entry, source="run")
        for run, message in (
            (successful_run(id=1), r"run: id: is 1, but the ledger names run 9001"),
            (successful_run(repository={"full_name": "someone/fork"}), r"repository\.full_name: is 'someone/fork'"),
            (successful_run(path=".github/workflows/ci.yml"), r"path: is '\.github/workflows/ci\.yml'"),
            (successful_run(status="in_progress", conclusion=None), r"conclusion: is not set"),
            (successful_run(conclusion="failure"), r"conclusion: is 'failure'"),
            ({k: v for k, v in successful_run().items() if k != "conclusion"}, r"run: conclusion: is required"),
        ):
            with self.subTest(message=message), self.assertRaisesRegex(d.DistributionError, message):
                d.check_rollout_run(run, entry, source="run")


class RolloutCandidateTests(unittest.TestCase):
    """The entry a publishing run proposes, from its jobs' results."""

    def candidate(self, selection: d.Selection | None = None, **results: str) -> dict:
        values = {"install": "success", "binstall": "skipped", "tap": "skipped", **results}
        return d.rollout_candidate(selection or selected(), run_id=LEDGER_RUN_ID, **values)

    def test_channel_results_follow_the_selection(self) -> None:
        brew = selected(brew="bh")
        both = selected(brew="bh", binstall="true")
        for selection, results, expected in (
            (selected(), {}, ("disabled", "disabled")),
            (brew, {"tap": "success"}, ("pass", "disabled")),
            (both, {"tap": "success", "binstall": "success"}, ("pass", "pass")),
            (selected(prerelease=True, brew="bh"), {}, ("unchanged", "disabled")),
        ):
            with self.subTest(expected=expected):
                entry = self.candidate(selection, **results)
                self.assertEqual((entry["channels"]["brew"], entry["channels"]["binstall"]), expected)
                self.assertEqual(entry["targets"], {target: "pass" for target in d.TARGETS})

    def test_a_run_that_did_not_prove_every_enabled_route_proposes_nothing(self) -> None:
        brew = selected(brew="bh")
        for selection, results, message in (
            (None, {"install": "failure"}, "install: is 'failure'"),
            (None, {"install": "skipped"}, "install: is 'skipped'"),
            (brew, {"tap": "skipped"}, "homebrew-tap: is 'skipped'; an enabled channel's job must be 'success'"),
            (brew, {"tap": "failure"}, "homebrew-tap: is 'failure'"),
            (None, {"tap": "success"}, "homebrew-tap: is 'success'; a disabled channel's job must be 'skipped'"),
            (selected(binstall="true"), {"binstall": "cancelled"}, "binstall: is 'cancelled'"),
            (None, {"install": "passed"}, "--install: must be one of"),
            (verification(), {}, "a verification run proves no rollout"),
        ):
            with self.subTest(message=message), self.assertRaisesRegex(d.DistributionError, re.escape(message)):
                self.candidate(selection, **results)


class RolloutPreflightTests(PreflightFixture, unittest.TestCase):
    """The gate through `main`: a package other than bh is refused until the
    ledger names a verified bh run that GitHub reports successful."""

    def setUp(self) -> None:
        super().setUp()
        version = WORKSPACE.packages["darkmatter-cli"].version
        self.tag = f"darkmatter-cli-v{version}"
        (self.dir / "md.yaml").write_text(approve(POLICY_TEXT, "darkmatter-cli"), encoding="utf-8")
        (self.dir / "md-event.json").write_text(json.dumps(release_event(self.tag)), encoding="utf-8")
        self.forge = {"releases": {self.tag: release_event(self.tag)["release"]}, "tags": {self.tag: TAG_SHA},
                      "proposals": {TAG_SHA: {"number": 17, "head_sha": TAG_SHA, "merge_sha": MERGE_SHA}},
                      "runs": {str(LEDGER_RUN_ID): successful_run()}}
        self.write_ledger(ledger_text([bh_entry()]))

    def write_ledger(self, text: str) -> None:
        (self.dir / "rollout.json").write_text(text, encoding="utf-8")

    def md_preflight(self) -> tuple[int, dict, str]:
        (self.dir / "md-forge.json").write_text(json.dumps(self.forge), encoding="utf-8")
        code, out, err = self.run_main(
            "preflight", "--event-name", "release", "--event-path", str(self.dir / "md-event.json"),
            "--tagged-root", str(REPO), "--current-root", str(REPO),
            "--tagged-metadata", str(self.dir / "metadata.json"), "--current-metadata", str(self.dir / "metadata.json"),
            "--tagged-policy", str(self.dir / "md.yaml"), "--current-policy", str(self.dir / "md.yaml"),
            "--forge-fixture", str(self.dir / "md-forge.json"), "--checkout-commit", TAG_SHA,
            "--rollout", str(self.dir / "rollout.json"),
        )
        return code, json.loads(out), err

    def assertRefused(self, pattern: str) -> None:
        code, result, err = self.md_preflight()
        self.assertEqual((code, result["status"]), (2, "invalid"))
        self.assertNotIn("build_matrix", result)
        self.assertRegex(err, pattern)

    def test_a_verified_bh_run_permits_the_second_package(self) -> None:
        code, result, _ = self.md_preflight()
        self.assertEqual((code, result["status"], result["command"]), (0, "selected", "md"))

    def test_without_a_verified_bh_entry_the_second_package_is_refused(self) -> None:
        self.write_ledger(ledger_text(pending=[{"name": "live-release", "package": "biscuit-hash-cli", "reason": "r"}]))
        self.assertRefused(r"rollout\.json: darkmatter-cli cannot be distributed before the first biscuit-hash-cli release is verified")

    def test_the_bh_run_must_exist_and_have_succeeded(self) -> None:
        self.forge["runs"] = {}
        self.assertRefused(r"names run 9001, which does not exist")
        self.forge["runs"] = {str(LEDGER_RUN_ID): successful_run(conclusion="failure")}
        self.assertRefused(r"run 9001: conclusion: is 'failure'")
        del self.forge["runs"]
        self.assertRefused(r"md-forge\.json: runs: is required")

    def test_a_malformed_ledger_is_located_even_for_bh(self) -> None:
        self.write_ledger('{"first_package": "biscuit-hash-cli", "verified": [], "verified": [], "pending": []}')
        code, out, err = self.preflight("approved.yaml", "--rollout", str(self.dir / "rollout.json"))
        self.assertEqual((code, json.loads(out)["status"]), (2, "invalid"))
        self.assertIn("rollout.json: invalid JSON: duplicate object key 'verified'", err)

    def test_an_ignored_release_does_not_need_the_ledger(self) -> None:
        (self.dir / "real.yaml").write_bytes(POLICY_PATH.read_bytes())
        code, out, _ = self.preflight("real.yaml", "--rollout", str(self.dir / "absent.json"))
        self.assertEqual((code, json.loads(out)["status"]), (0, "ignored"))

    def test_the_candidate_command_writes_an_entry_the_ledger_accepts(self) -> None:
        (self.dir / "selection.json").write_text(json.dumps(selected().record), encoding="utf-8")
        code, out, _ = self.run_main("rollout-candidate", "--selection", str(self.dir / "selection.json"), "--run-id", str(LEDGER_RUN_ID),
                                     "--install", "success", "--binstall", "skipped", "--tap", "skipped",
                                     "--output", str(self.dir / "candidate.json"))
        self.assertEqual(code, 0)
        self.write_ledger(ledger_text([json.loads((self.dir / "candidate.json").read_text())]))
        self.assertEqual(self.md_preflight()[0], 0)
        code, out, err = self.run_main("rollout-candidate", "--selection", str(self.dir / "selection.json"), "--run-id", "x",
                                       "--install", "success", "--binstall", "skipped", "--tap", "skipped")
        self.assertEqual((code, json.loads(out)["status"]), (2, "invalid"))


if __name__ == "__main__":
    unittest.main()

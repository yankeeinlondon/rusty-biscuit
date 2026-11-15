#!/usr/bin/env python3
"""Binary distribution decisions for a published package release.

release-plz owns versions, tags, and GitHub Releases (`release-plz.yml`,
`release_path.py`). This module decides whether one published release is
distributed, and with what, before anything is built or uploaded:

- `preflight` selects the single approved CLI whose `{package}-v{version}` tag
  was released, after checking approval in both the current and the tagged
  `docs/deployment.yaml`, the tag's commit, the manifest version, the Release,
  and the reviewed release proposal that produced the tag. Its JSON record is
  the only input the later commands accept.
- `verify-select` selects the current checkout for a nonpublishing
  verification build, approved by command-line fixture channels instead of the
  policy. Its record says `publishable: false`, which every later command
  checks and `publish` refuses.
- `build-matrix` names the pinned runner for each selected target.
- `config` renders the `dist` opt-in for that one package; `stage` copies the
  tagged checkout into a temporary tree, appends that opt-in, adds
  `SOURCE.txt` and the third-party notices, and proves nothing else changed.
  The repository never carries an opt-in.
- `validate-plan` checks a `dist plan --output-format=json` document against
  the selection.
- `provenance` writes `distribution-provenance.json` from the plan, dist's
  checksums, and each target's build environment.
- `reconcile-assets` compares built assets with a Release's existing assets
  byte for byte: identical ones are reused, missing ones uploaded, and a
  same-name asset with different bytes refuses the whole upload.
- `bundle` keeps one complete build (selection, plan, build environments,
  assets, provenance) as a workflow artifact; `check-bundle` proves a bundle
  matches a fresh preflight, and that a bundle from an earlier run came from
  the distribution workflow's successful `Package` job.
- `publish` rechecks approval, the Release, the tag, and the proposal, then
  attaches only the missing assets of a checked bundle to the published
  Release, provenance last. It never edits or deletes anything.
- `tap-update` decides whether a Homebrew formula is created, updated, or left
  alone, by SemVer order; it never downgrades.
- `preflight` also reads the rollout ledger (`.github/distribution/rollout.json`):
  no package but `biscuit-hash-cli` is distributed until the ledger holds a
  verified `biscuit-hash-cli` entry with every channel `pass` whose run GitHub
  reports successful. `rollout-candidate` writes the entry a successful
  publishing run proposes; a channel is `pass` only when its installation job
  succeeded on every target.

Every input is parsed strictly: an invalid document or field is a located
error (`file:line:column: field path: message`) and nothing runs. Exit codes:
0 selected, ignored, decided, or published; 1 refused (an asset conflict); 2 invalid
input; 3 pending setup. The operational guide is `docs/distribution.md`.
"""

from __future__ import annotations

import abc
import argparse
import difflib
import json
import os
import re
import shutil
import subprocess
import sys
import tomllib
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass, field
from pathlib import Path, PurePath
from typing import Any, Callable, Iterable, Iterator, Mapping, Sequence

import semver
import yaml

sys.path.insert(0, str(Path(__file__).resolve().parent))
from release_path import ReleasePathError, peeled_tag_target, select_publication  # noqa: E402

CANONICAL_REPOSITORY = "yankeeinlondon/rusty-biscuit"
CANONICAL_URL = f"https://github.com/{CANONICAL_REPOSITORY}"
HOMEBREW_TAP = "yankeeinlondon/homebrew-tap"

DIST_VERSION = "0.34.0"
BINSTALL_VERSION = "1.26.2"

TARGETS = (
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "x86_64-unknown-linux-gnu",
)

# Packages able to ship a binary, and the one command each ships. This is a
# capability list, not an approval: `docs/deployment.yaml` approves.
ROSTER: Mapping[str, str] = {
    "biscuit-hash-cli": "bh",
    "biscuit-file-cli": "bf",
    "tree-hugger-cli": "hug",
    "git-wt-cli": "wt",
    "sniff-detection-cli": "sniff",
    "darkmatter-cli": "md",
    "darkmatter-lsp": "dmls",
    "claudine-cli": "claudine",
}

# The channel vocabulary of `docs/deployment.yaml`. Only `binstall` and `brew`
# are published by this workflow; any other enabled channel on a selected
# package is refused rather than reported as published.
CHANNELS = (
    "brew", "uv", "apt", "nixpkgs", "winget", "chocolatey", "npm",
    "dnf", "scoop", "aur", "binstall", "install", "snap",
)
SUPPORTED_CHANNELS = frozenset({"binstall", "brew"})
POLICY_TOP_LEVEL = frozenset({"$schema", "types", "packages"})
ENTRY_FIELDS = frozenset({"crate", "approved", *CHANNELS})

# The binstall block every capable manifest carries (`[package.metadata.binstall]`).
BINSTALL_URLS = {
    "pkg-url": "{ repo }/releases/download/{ name }-v{ version }/{ name }-{ target }{ archive-suffix }",
    "pkg-fmt": "txz",
    "bin-dir": "{ name }-{ target }/{ bin }{ binary-ext }",
}
BINSTALL_ENABLED_STRATEGIES = ["quick-install", "compile"]
BINSTALL_DISABLED_STRATEGIES = ["crate-meta-data", "quick-install", "compile"]

# The glibc floor: Ubuntu 22.04's, the pinned Linux build image
# (`.github/distribution/build-images.json`).
MIN_GLIBC = "2.35"
GNU_TARGETS = tuple(target for target in TARGETS if target.endswith("-linux-gnu"))

# Settings `dist-workspace.toml` must hold; see that file for why.
DIST_BASE = {
    "cargo-dist-version": DIST_VERSION,
    "dist": False,
    "precise-builds": True,
    "ci": "github",
    "allow-dirty": ["ci"],
    "pr-run-mode": "skip",
    "create-release": False,
    "install-updater": False,
    "install-path": "CARGO_HOME",
    "source-tarball": False,
    "checksum": "sha256",
    "unix-archive": ".tar.xz",
    "installers": ["shell"],
    "targets": list(TARGETS),
    "min-glibc-version": {target: MIN_GLIBC for target in GNU_TARGETS},
}

# Files `stage` writes beside the selected manifest for dist to `include` in
# every archive, and the provenance asset `provenance` writes.
GENERATED_DIR = ".distribution"
SOURCE_NAME = "SOURCE.txt"
NOTICES_NAME = "THIRD-PARTY-NOTICES.md"
GENERATED_FILES = (SOURCE_NAME, NOTICES_NAME)
PROVENANCE_NAME = "distribution-provenance.json"
CHANGELOG_NAME = "CHANGELOG.md"

SELECTION_MODES = ("release", "recovery", "verify")

# The distribution workflow, which alone may produce a reusable build bundle,
# and the job in it that builds and bundles.
DISTRIBUTION_WORKFLOW = ".github/workflows/distribute-binaries.yml"
PACKAGE_JOB = "Package"

SELECTED = "selected"
IGNORED = "ignored"
PENDING_SETUP = "pending-setup"
INVALID = "invalid"

_CHANNEL_NAME = re.compile(r"^[a-z0-9][a-z0-9+._-]*$")
_SHA = re.compile(r"^[0-9a-f]{40}$")
_CHUNK = 1 << 16


class DistributionError(Exception):
    """Invalid input or a refused state, located by `where` (a file position
    and field path when one exists)."""

    def __init__(self, where: str, message: str) -> None:
        super().__init__(f"{where}: {message}")
        self.where = where
        self.message = message


class PendingSetup(Exception):
    """A selected release cannot proceed until `setting` is configured."""

    def __init__(self, setting: str, instruction: str) -> None:
        super().__init__(f"{setting}: {instruction}")
        self.setting = setting
        self.instruction = instruction


# ---------------------------------------------------------------------------
# Loaders: one strict loader per format
# ---------------------------------------------------------------------------


class StrictYamlLoader(yaml.SafeLoader):
    """`SafeLoader` with YAML 1.2-style plain-scalar resolution: only `true`
    and `false` are booleans (`on`, `yes`, `True` stay strings), and there are
    no timestamps, octals, or merge keys. Used only to compose nodes;
    `_convert` turns them into values and rejects what the loader allows but
    a policy must not contain."""

    yaml_implicit_resolvers: dict[Any, Any] = {}


StrictYamlLoader.add_implicit_resolver("tag:yaml.org,2002:bool", re.compile(r"^(?:true|false)$"), list("tf"))
StrictYamlLoader.add_implicit_resolver("tag:yaml.org,2002:null", re.compile(r"^(?:~|null|)$"), ["~", "n", ""])
StrictYamlLoader.add_implicit_resolver("tag:yaml.org,2002:int", re.compile(r"^[-+]?(?:0|[1-9][0-9]*)$"), list("-+0123456789"))
StrictYamlLoader.add_implicit_resolver(
    "tag:yaml.org,2002:float",
    re.compile(r"^[-+]?(?:[0-9]+\.[0-9]*|\.[0-9]+|[0-9]+(?=[eE]))(?:[eE][-+]?[0-9]+)?$"),
    list("-+0123456789."),
)

_STR, _INT, _FLOAT, _BOOL, _NULL = (f"tag:yaml.org,2002:{name}" for name in ("str", "int", "float", "bool", "null"))
_SEQ, _MAP = "tag:yaml.org,2002:seq", "tag:yaml.org,2002:map"


@dataclass(frozen=True)
class Document:
    """A parsed document and the position of every value in it, keyed by its
    field path (a tuple of keys and indexes)."""

    source: str
    value: Any
    marks: dict[tuple[Any, ...], tuple[int, int]] = field(default_factory=dict)

    def where(self, path: Sequence[Any]) -> str:
        """`source:line:column: a.b[0]`, using the nearest located ancestor."""
        key = tuple(path)
        while key not in self.marks and key:
            key = key[:-1]
        position = self.marks.get(key)
        prefix = f"{self.source}:{position[0]}:{position[1]}" if position else self.source
        return f"{prefix}: {render_path(path)}" if path else prefix


def render_path(path: Sequence[Any]) -> str:
    text = ""
    for part in path:
        text += f"[{part}]" if isinstance(part, int) else (f".{part}" if text else str(part))
    return text or "<document>"


def _mark(node: yaml.Node) -> tuple[int, int]:
    return node.start_mark.line + 1, node.start_mark.column + 1


def load_yaml(data: bytes, *, source: str) -> Document:
    """Exactly one YAML document whose root is a mapping."""
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError as error:
        raise DistributionError(source, f"not UTF-8: {error}") from None
    loader = StrictYamlLoader(text)
    try:
        root = loader.get_single_node()
    except yaml.MarkedYAMLError as error:
        mark = error.problem_mark or error.context_mark
        where = f"{source}:{mark.line + 1}:{mark.column + 1}" if mark else source
        raise DistributionError(where, f"invalid YAML: {error.problem or error.context}") from None
    except yaml.YAMLError as error:
        raise DistributionError(source, f"invalid YAML: {error}") from None
    finally:
        loader.dispose()
    if root is None:
        raise DistributionError(source, "empty document; expected one mapping")
    document = Document(source, None)
    value = _convert(root, (), document, set())
    if not isinstance(value, dict):
        raise DistributionError(f"{source}:1:1", "the document root must be a mapping")
    return Document(source, value, document.marks)


def _convert(node: yaml.Node, path: tuple[Any, ...], document: Document, active: set[int]) -> Any:
    document.marks.setdefault(path, _mark(node))
    where = lambda: document.where(path)  # noqa: E731 - only formatted on error
    if id(node) in active:
        raise DistributionError(where(), "recursive alias")
    if isinstance(node, yaml.ScalarNode):
        if node.tag == _STR:
            return node.value
        if node.tag == _BOOL:
            return node.value == "true"
        if node.tag == _NULL:
            return None
        if node.tag == _INT:
            return int(node.value)
        if node.tag == _FLOAT:
            return float(node.value)
        raise DistributionError(where(), f"unsupported tag {node.tag!r}")
    active = active | {id(node)}
    if isinstance(node, yaml.SequenceNode):
        if node.tag != _SEQ:
            raise DistributionError(where(), f"unsupported tag {node.tag!r}")
        return [_convert(item, (*path, index), document, active) for index, item in enumerate(node.value)]
    if node.tag != _MAP:
        raise DistributionError(where(), f"unsupported tag {node.tag!r}")
    result: dict[str, Any] = {}
    for key_node, value_node in node.value:
        if not isinstance(key_node, yaml.ScalarNode) or key_node.tag != _STR:
            line, column = _mark(key_node)
            raise DistributionError(f"{document.source}:{line}:{column}: {render_path(path)}", "mapping keys must be strings")
        key = key_node.value
        child = (*path, key)
        if key == "<<" and key_node.style is None:
            raise DistributionError(f"{document.source}:{_mark(key_node)[0]}:{_mark(key_node)[1]}: {render_path(child)}", "YAML merge keys are not supported")
        if key in result:
            line, column = _mark(key_node)
            raise DistributionError(f"{document.source}:{line}:{column}: {render_path(child)}", "duplicate key")
        document.marks[child] = _mark(key_node)
        result[key] = _convert(value_node, child, document, active)
        document.marks[child] = _mark(key_node)
    return result


def _reject_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate object key {key!r}")
        result[key] = value
    return result


def _reject_constant(token: str) -> Any:
    raise ValueError(f"invalid JSON constant {token}")


def load_json(text: str | bytes, *, source: str) -> Any:
    """One JSON document; duplicate keys, `NaN`/`Infinity`, and trailing
    content are errors."""
    try:
        if isinstance(text, bytes):
            text = text.decode("utf-8")
        return json.loads(text, object_pairs_hook=_reject_duplicates, parse_constant=_reject_constant)
    except (ValueError, UnicodeDecodeError) as error:
        raise DistributionError(source, f"invalid JSON: {error}") from None


def load_toml(text: str | bytes, *, source: str) -> dict[str, Any]:
    try:
        return tomllib.loads(text.decode("utf-8") if isinstance(text, bytes) else text)
    except UnicodeDecodeError as error:
        raise DistributionError(source, f"not UTF-8: {error}") from None
    except tomllib.TOMLDecodeError as error:
        raise DistributionError(source, f"invalid TOML: {error}") from None


# ---------------------------------------------------------------------------
# Shared field validators (absent, null, and wrong type are distinct errors)
# ---------------------------------------------------------------------------

_ABSENT = object()


def _get(root: Any, path: Sequence[Any]) -> Any:
    node = root
    for part in path:
        if isinstance(part, int):
            if not isinstance(node, list) or part >= len(node):
                return _ABSENT
            node = node[part]
        else:
            if not isinstance(node, dict) or part not in node:
                return _ABSENT
            node = node[part]
    return node


def _type_name(value: Any) -> str:
    if value is None:
        return "null"
    return {bool: "boolean", int: "integer", float: "number", str: "string", list: "list", dict: "mapping"}.get(type(value), type(value).__name__)


class Fields:
    """Located, typed reads from one parsed document."""

    def __init__(self, document: Document) -> None:
        self.document = document

    def error(self, path: Sequence[Any], message: str) -> DistributionError:
        return DistributionError(self.document.where(path), message)

    def lookup(self, path: Sequence[Any]) -> Any:
        """The value at `path`, or `_ABSENT` when only its last step is
        missing. An ancestor of the wrong shape (a null `release`, a list
        root where a mapping is required) is an error located at that
        ancestor, never mistaken for an absent field."""
        node = self.document.value
        for depth, part in enumerate(path):
            container = list if isinstance(part, int) else dict
            if not isinstance(node, container):
                raise self.error(path[:depth], f"must be a {'list' if container is list else 'mapping'}, found {_type_name(node)}")
            if part not in node if container is dict else part >= len(node):
                if depth < len(path) - 1:
                    raise self.error(path[:depth + 1], "is required")
                return _ABSENT
            node = node[part]
        return node

    def present(self, path: Sequence[Any]) -> Any:
        value = self.lookup(path)
        if value is _ABSENT:
            raise self.error(path, "is required")
        if value is None:
            raise self.error(path, "must not be null")
        return value

    def mapping(self, path: Sequence[Any]) -> dict[str, Any]:
        value = self.present(path)
        if not isinstance(value, dict):
            raise self.error(path, f"must be a mapping, found {_type_name(value)}")
        return value

    def records(self, path: Sequence[Any], *, allow_empty: bool) -> list[dict[str, Any]]:
        value = self.present(path)
        if not isinstance(value, list):
            raise self.error(path, f"must be a list, found {_type_name(value)}")
        if not value and not allow_empty:
            raise self.error(path, "must not be empty")
        for index, item in enumerate(value):
            if not isinstance(item, dict):
                raise self.error((*path, index), f"must be a mapping, found {_type_name(item)}")
        return value

    def string(self, path: Sequence[Any]) -> str:
        value = self.present(path)
        if not isinstance(value, str):
            raise self.error(path, f"must be a string, found {_type_name(value)}")
        if not value.strip():
            raise self.error(path, "must not be empty")
        return value

    def strings(self, path: Sequence[Any], *, allow_empty: bool) -> list[str]:
        value = self.present(path)
        if not isinstance(value, list):
            raise self.error(path, f"must be a list, found {_type_name(value)}")
        if not value and not allow_empty:
            raise self.error(path, "must not be empty")
        return [self.string((*path, index)) for index in range(len(value))]

    def boolean(self, path: Sequence[Any], *, default: bool | None = None) -> bool:
        if default is not None and self.lookup(path) is _ABSENT:
            return default
        value = self.present(path)
        if not isinstance(value, bool):
            raise self.error(path, f"must be true or false, found {_type_name(value)} {value!r}")
        return value

    def integer(self, path: Sequence[Any]) -> int:
        value = self.present(path)
        if isinstance(value, bool) or not isinstance(value, int):
            raise self.error(path, f"must be an integer, found {_type_name(value)}")
        return value

    def sha(self, path: Sequence[Any]) -> str:
        value = self.string(path)
        if not _SHA.match(value):
            raise self.error(path, f"must be a full 40-digit commit SHA, found {value!r}")
        return value

    def version(self, path: Sequence[Any]) -> semver.Version:
        return parse_version(self.string(path), self.document.where(path))

    def asset_name(self, path: Sequence[Any]) -> str:
        """A release asset name or an in-archive asset path: one file name,
        never a separator or a `.`/`..` step that would leave its directory."""
        value = self.string(path)
        if value in (".", "..") or any(character in value for character in "/\\\0"):
            raise self.error(path, f"must be a single file name, found {value!r}")
        return value


def parse_version(text: str, where: str) -> semver.Version:
    try:
        return semver.Version.parse(text)
    except (ValueError, TypeError) as error:
        raise DistributionError(where, f"must be a complete SemVer version, found {text!r} ({error})") from None


def _json_fields(value: Any, source: str) -> Fields:
    return Fields(Document(source, value))


# ---------------------------------------------------------------------------
# Workspace packages, from `cargo metadata --no-deps`
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Binary:
    name: str
    required_features: tuple[str, ...]


@dataclass(frozen=True)
class Package:
    name: str
    version: str
    manifest_path: str  # repository-relative, `/`-separated
    repository: str | None
    description: str | None
    homepage: str | None
    binaries: tuple[Binary, ...]
    metadata: dict[str, Any] | None


@dataclass(frozen=True)
class Workspace:
    """Workspace members, plus the names of packages the root manifest
    excludes from the workspace (they may appear in the policy, but are never
    distributable)."""

    packages: dict[str, Package]
    excluded: frozenset[str] = frozenset()

    def knows(self, name: str) -> bool:
        return name in self.packages or name in self.excluded


def load_workspace(metadata: str | bytes, *, source: str, excluded: Iterable[str] = ()) -> Workspace:
    fields = _json_fields(load_json(metadata, source=source), source)
    root = PurePath(fields.string(["workspace_root"]))
    members = set(fields.strings(["workspace_members"], allow_empty=False))
    packages: dict[str, Package] = {}
    for index, _ in enumerate(fields.records(["packages"], allow_empty=False)):
        base = ["packages", index]
        if fields.string([*base, "id"]) not in members:
            continue
        name = fields.string([*base, "name"])
        version = fields.string([*base, "version"])
        fields.version([*base, "version"])
        manifest = PurePath(fields.string([*base, "manifest_path"]))
        try:
            relative = manifest.relative_to(root).as_posix()
        except ValueError:
            raise fields.error([*base, "manifest_path"], f"is outside the workspace root {root}") from None
        binaries = []
        for target_index, _ in enumerate(fields.records([*base, "targets"], allow_empty=False)):
            target = [*base, "targets", target_index]
            if "bin" not in fields.strings([*target, "kind"], allow_empty=False):
                continue
            # Cargo omits `required-features` when a target has none.
            features = () if _get(fields.document.value, [*target, "required-features"]) is _ABSENT else tuple(
                fields.strings([*target, "required-features"], allow_empty=False)
            )
            binaries.append(Binary(fields.string([*target, "name"]), features))
        if name in packages:
            raise fields.error([*base, "name"], f"duplicate workspace package {name!r}")
        packages[name] = Package(
            name=name,
            version=version,
            manifest_path=relative,
            repository=_optional_string(fields, [*base, "repository"]),
            description=_optional_string(fields, [*base, "description"]),
            homepage=_optional_string(fields, [*base, "homepage"]),
            binaries=tuple(binaries),
            metadata=_optional_mapping(fields, [*base, "metadata"]),
        )
    return Workspace(packages, frozenset(excluded))


def _optional_string(fields: Fields, path: list[Any]) -> str | None:
    """Cargo writes `null` for an unset string field; absence is malformed."""
    value = _get(fields.document.value, path)
    if value is _ABSENT:
        raise fields.error(path, "is required (cargo metadata writes null when unset)")
    return None if value is None else fields.string(path)


def _optional_mapping(fields: Fields, path: list[Any]) -> dict[str, Any] | None:
    value = _get(fields.document.value, path)
    if value is _ABSENT:
        raise fields.error(path, "is required (cargo metadata writes null when unset)")
    return None if value is None else fields.mapping(path)


def excluded_package_names(root_manifest: str | bytes, read: Callable[[str], str | None], *, source: str = "Cargo.toml") -> set[str]:
    """Names of the packages at the root manifest's `[workspace] exclude`
    paths (none when `exclude` is absent; a missing `[workspace]` table is an
    error). `read(path)` returns a repository file's text, or None."""
    manifest = load_toml(root_manifest, source=source)
    fields = _json_fields(manifest, source)
    if fields.lookup(["workspace", "exclude"]) is _ABSENT:
        return set()
    names = set()
    for index, directory in enumerate(fields.strings(["workspace", "exclude"], allow_empty=True)):
        path = f"{directory.rstrip('/')}/Cargo.toml"
        text = read(path)
        if text is None:
            raise fields.error(["workspace", "exclude", index], f"{path} does not exist")
        names.add(_json_fields(load_toml(text, source=path), path).string(["package", "name"]))
    return names


def cargo_workspace(root: Path, *, run: Callable[..., subprocess.CompletedProcess[str]] = subprocess.run) -> Workspace:
    """The workspace at `root`, resolved by Cargo rather than by directory names."""
    result = run(
        ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1", "--manifest-path", str(root / "Cargo.toml")],
        capture_output=True, text=True, timeout=300, check=False,
    )
    if result.returncode != 0:
        raise DistributionError(f"cargo metadata ({root})", result.stderr.strip() or f"exit {result.returncode}")

    def read(path: str) -> str | None:
        file = root / path
        return file.read_text(encoding="utf-8") if file.is_file() else None

    excluded = excluded_package_names((root / "Cargo.toml").read_text(encoding="utf-8"), read, source=str(root / "Cargo.toml"))
    return load_workspace(result.stdout, source=f"cargo metadata ({root})", excluded=excluded)


# ---------------------------------------------------------------------------
# Approval policy (`docs/deployment.yaml`)
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class PolicyEntry:
    package: str
    crate: str | None
    approved: bool
    channels: dict[str, str]  # enabled channels only, with their resolved names


@dataclass(frozen=True)
class Policy:
    source: str
    raw: bytes
    entries: dict[str, PolicyEntry]
    document: Document


def load_policy(path: Path, packages: Workspace, *, source: str | None = None) -> Policy:
    return parse_policy(_read_file(path), packages, source=source or path.as_posix())


def parse_policy(data: bytes, packages: Workspace, *, source: str) -> Policy:
    """Validate the approval policy. Absent `approved` and absent channels are
    false; an explicit null is invalid. A channel is `false`, `true` (the crate
    name), or a portable name. `$schema` and `types` are descriptions only."""
    document = load_yaml(data, source=source)
    fields = Fields(document)
    for key in document.value:
        if key not in POLICY_TOP_LEVEL:
            raise fields.error([key], f"unknown top-level key; expected one of {sorted(POLICY_TOP_LEVEL)}")
    entries: dict[str, PolicyEntry] = {}
    claimed: dict[tuple[str, str], str] = {}
    for package in fields.mapping(["packages"]):
        base = ["packages", package]
        if not packages.knows(package):
            raise fields.error(base, "is not a package in this repository")
        fields.mapping(base)
        for key in document.value["packages"][package]:
            if key not in ENTRY_FIELDS:
                raise fields.error([*base, key], "unknown field")
        crate = _crate(fields, base, package)
        approved = fields.boolean([*base, "approved"], default=False)
        channels: dict[str, str] = {}
        for channel in CHANNELS:
            name = _channel(fields, [*base, channel], crate)
            if name is None:
                continue
            if not approved:
                raise fields.error([*base, channel], "is enabled on an entry that is not approved")
            owner = claimed.get((channel, name))
            if owner is not None:
                raise fields.error([*base, channel], f"resolves to {name!r}, already used by {owner!r}")
            claimed[(channel, name)] = package
            channels[channel] = name
        if crate is None and approved:
            raise fields.error([*base, "crate"], "is null, which only an unapproved entry may be")
        entries[package] = PolicyEntry(package, crate, approved, channels)
    return Policy(source, data, entries, document)


def _crate(fields: Fields, path: list[Any], package: str) -> str | None:
    value = _get(fields.document.value, [*path, "crate"])
    if value is _ABSENT:
        raise fields.error([*path, "crate"], "is required (null marks a package that is never distributed)")
    if value is None:
        return None
    crate = fields.string([*path, "crate"])
    if crate != package:
        raise fields.error([*path, "crate"], f"must equal the package name {package!r}, found {crate!r}")
    return crate


def _channel(fields: Fields, path: list[Any], crate: str | None) -> str | None:
    value = _get(fields.document.value, path)
    if value is _ABSENT or value is False:
        return None
    if value is None:
        raise fields.error(path, "must be true, false, or a name; found null")
    if value is True:
        if crate is None:
            raise fields.error(path, "is true but the entry's crate is null")
        return crate
    if not isinstance(value, str):
        raise fields.error(path, f"must be true, false, or a name; found {_type_name(value)}")
    if not _CHANNEL_NAME.match(value):
        raise fields.error(path, f"name {value!r} must match {_CHANNEL_NAME.pattern}")
    return value


def check_dist_base(text: str, *, source: str = "dist-workspace.toml") -> None:
    """The checked-in `dist` configuration holds exactly the pinned settings
    and opts no package in."""
    document = load_toml(text, source=source)
    fields = _json_fields(document, source)
    if fields.strings(["workspace", "members"], allow_empty=False) != ["cargo:."]:
        raise fields.error(["workspace", "members"], 'must be ["cargo:."]')
    dist = fields.mapping(["dist"])
    for key, expected in DIST_BASE.items():
        found = dist.get(key, _ABSENT)
        if found != expected or type(found) is not type(expected):
            raise fields.error(["dist", key], f"must be {expected!r}, found {'absent' if found is _ABSENT else repr(found)}")
    extra = sorted(set(dist) - set(DIST_BASE))
    if extra:
        raise fields.error(["dist", extra[0]], "is not a reviewed setting; add it to DIST_BASE with its reason")


# ---------------------------------------------------------------------------
# Release selection
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class ReleaseState:
    """The GitHub Release for the tag, as read from the event or the API."""

    tag: str
    release_id: int
    draft: bool
    prerelease: bool
    url: str


@dataclass(frozen=True)
class ProposalProof:
    """The merged release proposal whose final commit the tag names."""

    number: int
    head_sha: str
    merge_sha: str


@dataclass(frozen=True)
class ReleaseFacts:
    """Everything preflight decides on, gathered before it runs. `mode` is
    `release` (a published-release event) or `recovery` (a manual run naming
    an existing tag)."""

    mode: str
    repository: str
    tag: str
    release: ReleaseState | None
    tag_commit: str | None
    checkout_commit: str | None
    proposal: ProposalProof | PendingSetup | None


@dataclass(frozen=True)
class Policies:
    current: Policy
    tagged: Policy


@dataclass(frozen=True)
class Preflight:
    status: str
    reason: str
    tag: str | None = None
    record: dict[str, Any] | None = None

    def as_json(self) -> dict[str, Any]:
        result: dict[str, Any] = {"status": self.status, "reason": self.reason, "tag": self.tag}
        if self.record is not None:
            result.update(self.record)
        result["build_matrix"] = [{"target": target} for target in self.record["targets"]] if self.status == SELECTED and self.record else []
        return result


def _ignored(reason: str, tag: str | None) -> Preflight:
    return Preflight(IGNORED, reason, tag)


def recognize_tag(tag: str, packages: Workspace) -> tuple[Package, semver.Version] | Preflight:
    """The roster package a `{package}-v{version}` tag names, or an ignored
    result for a library, integration, or foreign tag. A malformed version on
    a roster package's tag is an error."""
    matches = [name for name in packages.packages if tag.startswith(f"{name}-v")]
    if not matches:
        return _ignored(f"tag {tag!r} names no workspace package", tag)
    name = max(matches, key=len)
    if name not in ROSTER:
        return _ignored(f"package {name!r} does not ship a binary (libraries and integrations are not distributed)", tag)
    version = parse_version(tag[len(name) + 2:], f"tag {tag}")
    return packages.packages[name], version


def select_release(event: ReleaseFacts, policy: Policies, packages: Workspace) -> Preflight:
    """Decide one release. `packages` is the tagged commit's workspace.

    Approval and every enabled channel must hold in both the tagged and the
    current policy: current policy can revoke an old release, but cannot
    approve a release that was unapproved when tagged. An unapproved release
    is ignored; a revoked one, a wrong revision, or a mismatched manifest is
    an error."""
    tag = event.tag
    if event.mode not in ("release", "recovery"):
        raise DistributionError("event", f"unknown mode {event.mode!r}")
    if event.repository != CANONICAL_REPOSITORY:
        return _ignored(f"repository {event.repository!r} is not {CANONICAL_REPOSITORY}", tag)
    recognized = recognize_tag(tag, packages)
    if isinstance(recognized, Preflight):
        return recognized
    package, version = recognized
    command = ROSTER[package.name]

    tagged = policy.tagged.entries.get(package.name)
    current = policy.current.entries.get(package.name)
    if tagged is None or not tagged.approved:
        return _ignored(f"{package.name} is not approved for distribution in the tagged {policy.tagged.source}", tag)
    if current is None or not current.approved:
        raise DistributionError(f"{policy.current.source}: packages.{package.name}.approved", "approval was revoked after this release was tagged")
    channels: dict[str, str] = {}
    revoked: list[str] = []
    for channel, name in sorted(tagged.channels.items()):
        current_name = current.channels.get(channel)
        if current_name is None:
            revoked.append(channel)
            continue
        if current_name != name:
            raise DistributionError(
                f"{policy.current.source}: packages.{package.name}.{channel}",
                f"resolves to {current_name!r}, but the tagged policy names {name!r}",
            )
        channels[channel] = name
    unsupported = sorted(set(channels) - SUPPORTED_CHANNELS)
    if unsupported:
        raise DistributionError(
            f"{policy.tagged.source}: packages.{package.name}.{unsupported[0]}",
            "is enabled, but this workflow publishes only binstall and brew; disable it",
        )
    if "binstall" in channels and channels["binstall"] != package.name:
        raise DistributionError(
            f"{policy.tagged.source}: packages.{package.name}.binstall",
            f"cargo-binstall installs by package name; use true, not {channels['binstall']!r}",
        )

    _check_package(package, command, "binstall" in channels, packages)
    if package.version != str(version):
        raise DistributionError(
            f"{package.manifest_path}: package.version",
            f"is {package.version!r} at the tagged commit, but the tag is {tag}",
        )

    release = event.release
    if release is None:
        raise DistributionError(f"release {tag}", "no GitHub Release exists for this tag")
    if release.tag != tag:
        raise DistributionError(f"release {tag}", f"the Release names tag {release.tag!r}")
    if release.draft:
        raise DistributionError(f"release {tag}", "is a draft; distribution runs only for a published Release")
    if event.tag_commit is None:
        raise DistributionError(f"tag {tag}", "does not exist")
    if event.checkout_commit != event.tag_commit:
        raise DistributionError(f"tag {tag}", f"names {event.tag_commit}, but the checkout is {event.checkout_commit}")
    proof = event.proposal
    if isinstance(proof, PendingSetup):
        return Preflight(PENDING_SETUP, str(proof), tag)
    if proof is None:
        raise DistributionError(f"tag {tag}", f"no merged release proposal has final commit {event.tag_commit}")
    if proof.head_sha != event.tag_commit:
        raise DistributionError(
            f"tag {tag}",
            f"names {event.tag_commit}, but release proposal #{proof.number} ended at {proof.head_sha}; a published tag is never moved",
        )

    prerelease = version.prerelease is not None or release.prerelease
    record = {
        "package": package.name,
        "command": command,
        "manifest_path": package.manifest_path,
        "version": str(version),
        "commit": event.tag_commit,
        "mode": event.mode,
        "publishable": True,
        "prerelease": prerelease,
        "release": {"id": release.release_id, "url": release.url},
        "proposal": {"number": proof.number, "head_sha": proof.head_sha, "merge_sha": proof.merge_sha},
        "channels": channels,
        "revoked_channels": revoked,
        # A prerelease never replaces the stable formula.
        "formula_update": "brew" in channels and not prerelease,
        "targets": list(TARGETS),
        "tools": {"dist": DIST_VERSION, "binstall": BINSTALL_VERSION},
    }
    return Preflight(SELECTED, f"{tag} is approved for distribution", tag, record)


def _check_package(package: Package, command: str, binstall: bool, packages: Workspace) -> None:
    where = package.manifest_path
    for other in sorted(packages.packages.values(), key=lambda item: item.name):
        if other.metadata and "dist" in other.metadata:
            raise DistributionError(
                f"{other.manifest_path}: package.metadata.dist",
                "is checked in; dist opt-ins are generated from docs/deployment.yaml only",
            )
    if package.repository != CANONICAL_URL:
        raise DistributionError(f"{where}: package.repository", f"must be {CANONICAL_URL}, found {package.repository!r}")
    for name, value in (("description", package.description), ("homepage", package.homepage)):
        if not value:
            raise DistributionError(f"{where}: package.{name}", "is required for a distributed package")
    shipped = [binary for binary in package.binaries if binary.name == command]
    if not shipped:
        raise DistributionError(f"{where}: [[bin]]", f"declares no {command!r} binary")
    if shipped[0].required_features:
        raise DistributionError(f"{where}: [[bin]] {command}", "must not require features; releases use default features")
    for binary in package.binaries:
        if binary.name != command and not binary.required_features:
            raise DistributionError(
                f"{where}: [[bin]] {binary.name}",
                "is built by default; cargo-binstall would require it in the archive. Gate it with required-features",
            )
    _check_binstall_metadata(package, binstall)


def _check_binstall_metadata(package: Package, enabled: bool) -> None:
    fields = _json_fields(package.metadata or {}, f"{package.manifest_path}: package.metadata")
    base = ["binstall"]
    fields.mapping(base)
    for key, expected in BINSTALL_URLS.items():
        if fields.string([*base, key]) != expected:
            raise fields.error([*base, key], f"must be {expected!r}")
    strategies = fields.strings([*base, "disabled-strategies"], allow_empty=False)
    expected = BINSTALL_ENABLED_STRATEGIES if enabled else BINSTALL_DISABLED_STRATEGIES
    if strategies != expected:
        state = "enabled" if enabled else "disabled"
        raise fields.error(
            [*base, "disabled-strategies"],
            f"must be {expected} while the binstall channel is {state}; the policy and manifest change together",
        )


def select_verification(package_name: str, packages: Workspace, checkout_commit: str, channels: Mapping[str, str]) -> Preflight:
    """A nonpublishing selection of the current checkout, approved by the
    caller's fixture `channels` rather than `docs/deployment.yaml`. It builds
    exactly what a release of the manifest version would, but its
    `publishable: false` keeps it from every upload and tap step, and it never
    updates a formula."""
    if package_name not in ROSTER:
        raise DistributionError("--package", f"{package_name!r} is not one of the packages that ship a binary: {sorted(ROSTER)}")
    package = packages.packages.get(package_name)
    if package is None:
        raise DistributionError("--package", f"{package_name!r} is not a member of this workspace")
    if not _SHA.match(checkout_commit):
        raise DistributionError("--checkout-commit", f"must be a full 40-digit commit SHA, found {checkout_commit!r}")
    resolved: dict[str, str] = {}
    for channel, name in sorted(channels.items()):
        if channel not in SUPPORTED_CHANNELS:
            raise DistributionError(f"--{channel}", f"is not a channel this workflow publishes; expected one of {sorted(SUPPORTED_CHANNELS)}")
        if not isinstance(name, str) or not _CHANNEL_NAME.match(name):
            raise DistributionError(f"--{channel}", f"name {name!r} must match {_CHANNEL_NAME.pattern}")
        if channel == "binstall" and name != package.name:
            raise DistributionError("--binstall", f"cargo-binstall installs by package name; it resolves to {package.name!r}, not {name!r}")
        resolved[channel] = name
    command = ROSTER[package.name]
    _check_package(package, command, "binstall" in resolved, packages)
    version = parse_version(package.version, f"{package.manifest_path}: package.version")
    tag = f"{package.name}-v{version}"
    record = {
        "package": package.name,
        "command": command,
        "manifest_path": package.manifest_path,
        "version": str(version),
        "commit": checkout_commit,
        "mode": "verify",
        "publishable": False,
        "prerelease": version.prerelease is not None,
        "release": None,
        "proposal": None,
        "channels": resolved,
        "revoked_channels": [],
        "formula_update": False,
        "targets": list(TARGETS),
        "tools": {"dist": DIST_VERSION, "binstall": BINSTALL_VERSION},
    }
    return Preflight(SELECTED, f"verification build of {tag} at {checkout_commit}; never published", tag, record)


# ---------------------------------------------------------------------------
# Gathering facts from an event and the forge
# ---------------------------------------------------------------------------


class Forge:
    """Read-only release state. Tests substitute a fake."""

    def release(self, tag: str) -> ReleaseState | None:
        raise NotImplementedError

    def tag_commit(self, tag: str) -> str | None:
        raise NotImplementedError

    def proposal(self, commit: str) -> ProposalProof | PendingSetup | None:
        raise NotImplementedError

    def workflow_run(self, run_id: int) -> Any | None:
        """The run's API record, or None when no such run exists."""
        raise NotImplementedError


def event_tag(event_name: str, payload: Any, *, source: str) -> tuple[str, str] | Preflight:
    """`(mode, tag)` for a published-release or manual recovery event."""
    fields = _json_fields(payload, source)
    if event_name == "release":
        action = fields.string(["action"])
        if action != "published":
            return _ignored(f"release action {action!r} is not published", None)
        return "release", fields.string(["release", "tag_name"])
    if event_name == "workflow_dispatch":
        return "recovery", fields.string(["inputs", "tag"])
    return _ignored(f"event {event_name!r} does not distribute releases", None)


def release_from_payload(payload: Any, *, source: str) -> ReleaseState:
    fields = _json_fields(payload, source)
    return ReleaseState(
        tag=fields.string(["release", "tag_name"]),
        release_id=fields.integer(["release", "id"]),
        draft=fields.boolean(["release", "draft"]),
        prerelease=fields.boolean(["release", "prerelease"]),
        url=fields.string(["release", "html_url"]),
    )


def gather_facts(
    event_name: str, payload: Any, *, source: str, repository: str, forge: Forge, checkout_commit: str | None,
    tagged: Workspace,
) -> ReleaseFacts | Preflight:
    found = event_tag(event_name, payload, source=source)
    if isinstance(found, Preflight):
        return found
    mode, tag = found
    recognized = recognize_tag(tag, tagged)
    if isinstance(recognized, Preflight):
        return recognized
    if mode == "release":
        release = release_from_payload(payload, source=source)
        repo_name = _json_fields(payload, source).string(["repository", "full_name"])
        if repo_name != repository:
            raise DistributionError(f"{source}: repository.full_name", f"is {repo_name!r}, but the run is in {repository!r}")
        # The API, not only the event, must agree the Release is published.
        current = forge.release(tag)
        if current is None or current.release_id != release.release_id:
            raise DistributionError(f"release {tag}", "the event's Release is not the repository's Release for this tag")
        release = current
    else:
        release = forge.release(tag)
    tag_commit = forge.tag_commit(tag)
    proposal = forge.proposal(tag_commit) if tag_commit else None
    return ReleaseFacts(mode, repository, tag, release, tag_commit, checkout_commit, proposal)


class FixtureForge(Forge):
    """Forge state from a JSON document, for fixtures and nonpublishing runs:
    `{"releases": {tag: {...}}, "tags": {tag: sha}, "proposals": {sha: {...} | "pending"}}`,
    plus `"runs": {id: {...}}`, required only when a run is looked up."""

    def __init__(self, data: Any, *, source: str) -> None:
        self.fields = _json_fields(data, source)
        for key in ("releases", "tags", "proposals"):
            self.fields.mapping([key])

    def release(self, tag: str) -> ReleaseState | None:
        if tag not in self.fields.document.value["releases"]:
            return None
        base = ["releases", tag]
        return ReleaseState(
            tag=self.fields.string([*base, "tag_name"]),
            release_id=self.fields.integer([*base, "id"]),
            draft=self.fields.boolean([*base, "draft"]),
            prerelease=self.fields.boolean([*base, "prerelease"]),
            url=self.fields.string([*base, "html_url"]),
        )

    def tag_commit(self, tag: str) -> str | None:
        return self.fields.sha(["tags", tag]) if tag in self.fields.document.value["tags"] else None

    def proposal(self, commit: str) -> ProposalProof | PendingSetup | None:
        if commit not in self.fields.document.value["proposals"]:
            return None
        base = ["proposals", commit]
        if self.fields.document.value["proposals"][commit] == "pending":
            return PendingSetup("RELEASE_APP_BOT_LOGIN", "the release App is not configured (see docs/releases.md)")
        return ProposalProof(self.fields.integer([*base, "number"]), self.fields.sha([*base, "head_sha"]), self.fields.sha([*base, "merge_sha"]))

    def workflow_run(self, run_id: int) -> Any | None:
        return self.fields.mapping(["runs"]).get(str(run_id))


class GitHubForge(Forge):
    """The GitHub API and `git ls-remote`, read-only. The proposal proof is
    the merged pull request whose head is the tag's commit, accepted only when
    `release_path.select_publication` accepts it as a release proposal."""

    def __init__(self, *, repository: str, token: str | None, bot_login: str | None, repo: Path) -> None:
        self.repository = repository
        self.token = token
        self.bot_login = bot_login
        self.repo = repo
        self.api = os.environ.get("GITHUB_API_URL", "https://api.github.com").rstrip("/")

    def _get(self, path: str) -> tuple[int, Any]:
        request = urllib.request.Request(f"{self.api}{path}")
        request.add_header("Accept", "application/vnd.github+json")
        request.add_header("X-GitHub-Api-Version", "2022-11-28")
        if self.token:
            request.add_header("Authorization", f"Bearer {self.token}")
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                return response.status, load_json(response.read(), source=f"GET {path}")
        except urllib.error.HTTPError as error:
            return error.code, None
        except (urllib.error.URLError, TimeoutError, OSError) as error:
            raise DistributionError(f"GET {path}", f"request failed: {error}") from None

    def release(self, tag: str) -> ReleaseState | None:
        path = f"/repos/{self.repository}/releases/tags/{urllib.parse.quote(tag, safe='')}"
        status, body = self._get(path)
        if status == 404:
            return None
        if status != 200:
            raise DistributionError(f"GET {path}", f"unexpected HTTP {status}")
        return release_from_payload({"release": body}, source=f"GET {path}")

    def tag_commit(self, tag: str) -> str | None:
        ref = f"refs/tags/{tag}"
        result = subprocess.run(
            ["git", "ls-remote", "--tags", "origin", ref, f"{ref}^{{}}"],
            cwd=self.repo, capture_output=True, text=True, timeout=60, check=False,
            env={**os.environ, "GIT_TERMINAL_PROMPT": "0"},
        )
        if result.returncode != 0:
            raise DistributionError(f"git ls-remote {tag}", result.stderr.strip() or f"exit {result.returncode}")
        try:
            return peeled_tag_target(result.stdout, tag)
        except ReleasePathError as error:
            raise DistributionError(error.path, error.message) from None

    def proposal(self, commit: str) -> ProposalProof | PendingSetup | None:
        if not self.bot_login:
            return PendingSetup(
                "RELEASE_APP_BOT_LOGIN",
                "repository variable is not set; distribution needs the release App's identity to prove the tag came from a reviewed proposal (see docs/releases.md)",
            )
        path = f"/repos/{self.repository}/commits/{commit}/pulls"
        status, body = self._get(path)
        if status != 200 or not isinstance(body, list):
            raise DistributionError(f"GET {path}", f"unexpected HTTP {status} or non-list body")
        for index, _ in enumerate(body):
            number = _json_fields(body, f"GET {path}").integer([index, "number"])
            pull_path = f"/repos/{self.repository}/pulls/{number}"
            status, pull = self._get(pull_path)
            if status != 200:
                raise DistributionError(f"GET {pull_path}", f"unexpected HTTP {status}")
            event = {"action": "closed", "pull_request": pull}
            try:
                selection = select_publication("pull_request", self.repository, event, self.bot_login)
            except ReleasePathError as error:
                raise DistributionError(f"pull request #{number}: {error.path}", error.message) from None
            if selection.selected and selection.outputs["head_sha"] == commit:
                return ProposalProof(number, selection.outputs["head_sha"], selection.outputs["merge_sha"])
        return None

    def workflow_run(self, run_id: int) -> Any | None:
        path = f"/repos/{self.repository}/actions/runs/{run_id}"
        status, body = self._get(path)
        if status == 404:
            return None
        if status != 200:
            raise DistributionError(f"GET {path}", f"unexpected HTTP {status}")
        return body


# ---------------------------------------------------------------------------
# dist configuration and plan
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Selection:
    """A successful preflight record; the only input build steps accept."""

    record: dict[str, Any]

    @classmethod
    def from_json(cls, payload: Any, *, source: str) -> Selection:
        fields = _json_fields(payload, source)
        status = fields.string(["status"])
        if status != SELECTED:
            raise fields.error(["status"], f"is {status!r}; only a selected preflight result can be built")
        package = fields.string(["package"])
        if ROSTER.get(package) != fields.string(["command"]):
            raise fields.error(["command"], f"is not {package}'s shipping command")
        fields.string(["manifest_path"])
        fields.version(["version"])
        fields.string(["tag"])
        fields.sha(["commit"])
        if fields.strings(["targets"], allow_empty=False) != list(TARGETS):
            raise fields.error(["targets"], f"must be {list(TARGETS)}")
        channels = fields.mapping(["channels"])
        for channel in channels:
            if channel not in SUPPORTED_CHANNELS:
                raise fields.error(["channels", channel], "is not a supported channel")
            fields.string(["channels", channel])
        fields.boolean(["formula_update"])
        mode = fields.string(["mode"])
        if mode not in SELECTION_MODES:
            raise fields.error(["mode"], f"must be one of {list(SELECTION_MODES)}, found {mode!r}")
        # Only a release or recovery preflight may publish; a verification
        # record claiming otherwise is forged.
        if fields.boolean(["publishable"]) != (mode != "verify"):
            raise fields.error(["publishable"], f"must be {str(mode != 'verify').lower()} for mode {mode!r}")
        return cls(payload)

    def __getattr__(self, name: str) -> Any:
        try:
            return self.record[name]
        except KeyError:
            raise AttributeError(name) from None


@dataclass(frozen=True)
class DistConfig:
    manifest_path: str
    overlay: str


def source_text(selection: Selection) -> str:
    """The archive's `SOURCE.txt`: where the exact source of these binaries is."""
    package_dir = PurePath(selection.manifest_path).parent.as_posix()
    return (
        f"{selection.package} {selection.version} ({selection.command})\n"
        f"License: AGPL-3.0-only, see LICENSE; third-party notices are in {NOTICES_NAME}.\n"
        f"Source: {CANONICAL_URL}/tree/{selection.tag}/{package_dir}\n"
        f"Commit: {CANONICAL_URL}/commit/{selection.commit}\n"
    )


def render_dist_config(selection: Selection) -> DistConfig:
    """The `[package.metadata.dist]` opt-in appended to the selected
    package's manifest in the temporary distribution copy. It names only that
    package and its one command.

    dist 0.34.0 ignores unknown keys silently, so every key here is one its
    `config/v0.rs` defines; `validate_plan` and the archive inspection prove
    their effect rather than trusting acceptance. The glibc floor is not
    here: dist's installer reads `min-glibc-version` from the workspace
    configuration only, so `dist-workspace.toml` carries it."""
    if not isinstance(selection, Selection):
        raise TypeError("render_dist_config requires a Selection from a successful preflight")
    installers = ["shell"]
    lines = ["", "# Generated by scripts/ci/distribution.py config for " + selection.tag + "; never checked in.",
             "[package.metadata.dist]", "dist = true"]
    if "brew" in selection.channels:
        installers.append("homebrew")
        lines += [f"formula = {json.dumps(selection.channels['brew'])}", f"tap = {json.dumps(HOMEBREW_TAP)}"]
    lines.append(f"installers = {json.dumps(installers)}")
    # Relative to the package directory; `stage` writes both files there.
    lines.append(f"include = {json.dumps([f'{GENERATED_DIR}/{name}' for name in GENERATED_FILES])}")
    # `binaries` replaces the package's packageables (`DistGraphBuilder::add_variant`),
    # so test-helper binaries never reach an archive.
    lines += ["", "[package.metadata.dist.binaries]", f'"*" = [{json.dumps(selection.command)}]', ""]
    return DistConfig(selection.manifest_path, "\n".join(lines))


# ---------------------------------------------------------------------------
# Staging: the tagged checkout plus the generated opt-in, in a temporary copy
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Staged:
    root: Path
    manifest_path: str
    generated: list[str]
    tracked_files: int
    overlay_diff: str

    def as_json(self) -> dict[str, Any]:
        return {"status": "staged", "root": self.root.as_posix(), "manifest_path": self.manifest_path,
                "generated": self.generated, "tracked_files": self.tracked_files}


def _git(source: Path, *args: str, run: Callable[..., subprocess.CompletedProcess[bytes]] = subprocess.run) -> bytes:
    result = run(["git", *args], cwd=source, capture_output=True, timeout=300, check=False)
    if result.returncode != 0:
        raise DistributionError(f"git {' '.join(args)} ({source.as_posix()})", result.stderr.decode("utf-8", "replace").strip())
    return result.stdout


def tracked_paths(source: Path, commit: str) -> list[str]:
    """The checkout's tracked files, after proving it is exactly `commit`
    with no tracked modification. Untracked and ignored files (a `target/`
    directory, say) are never staged."""
    head = _git(source, "rev-parse", "HEAD").decode().strip()
    if head != commit:
        raise DistributionError(f"checkout {source.as_posix()}", f"is at {head}, not the selected commit {commit}")
    dirty = _git(source, "status", "--porcelain=v1", "-z", "--untracked-files=no")
    if dirty:
        changed = sorted(entry[3:].decode("utf-8", "replace") for entry in dirty.split(b"\0") if entry)
        raise DistributionError(f"checkout {source.as_posix()}", f"has tracked modifications: {changed[:5]}")
    paths = [entry.decode("utf-8") for entry in _git(source, "ls-files", "-z", "--cached").split(b"\0") if entry]
    if not paths:
        raise DistributionError(f"checkout {source.as_posix()}", "tracks no files")
    return paths


def _copy_tracked(source: Path, dest: Path, paths: Sequence[str]) -> None:
    for relative in paths:
        origin = source / relative
        target = dest / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        if origin.is_symlink():
            os.symlink(os.readlink(origin), target)
        elif origin.is_file():
            shutil.copy2(origin, target)
        else:
            raise DistributionError(relative, "is tracked but is not a regular file or symlink (a submodule?); it cannot be staged")


def stage_distribution(selection: Selection, *, source: Path, dest: Path, notices: Path) -> Staged:
    """Copy the selected tagged checkout into `dest`, append the generated
    opt-in to the selected manifest, and add `SOURCE.txt` and the third-party
    notices beside it. `source` is never written.

    Before returning, every staged file is compared byte for byte with the
    checkout: the only differences allowed are the appended opt-in and the
    generated files, so the production sources and `Cargo.lock` that dist
    builds are the tagged ones."""
    if not isinstance(selection, Selection):
        raise TypeError("stage_distribution requires a Selection from a successful preflight")
    if dest.exists() and any(dest.iterdir()):
        raise DistributionError(dest.as_posix(), "must be absent or empty; staging never reuses a tree")
    try:
        notice_bytes = notices.read_bytes()
    except OSError as error:
        raise DistributionError(notices.as_posix(), f"cannot read the third-party notices: {error}") from None
    if not notice_bytes.strip():
        raise DistributionError(notices.as_posix(), "is empty; the archive must carry third-party notices")
    paths = tracked_paths(source, selection.commit)
    package_dir = PurePath(selection.manifest_path).parent
    generated = [(package_dir / GENERATED_DIR / name).as_posix() for name in GENERATED_FILES]
    clash = sorted(set(generated) & set(paths))
    if clash or any(path.startswith(f"{(package_dir / GENERATED_DIR).as_posix()}/") for path in paths):
        raise DistributionError(selection.manifest_path, f"{(package_dir / GENERATED_DIR).as_posix()} is tracked; it is reserved for generated files")
    if selection.manifest_path not in paths:
        raise DistributionError(selection.manifest_path, "is not a tracked file of the checkout")
    dest.mkdir(parents=True, exist_ok=True)
    _copy_tracked(source, dest, paths)
    config = render_dist_config(selection)
    original = (source / selection.manifest_path).read_bytes()
    if not original.endswith(b"\n"):
        raise DistributionError(selection.manifest_path, "must end with a newline before the opt-in is appended")
    (dest / selection.manifest_path).write_bytes(original + config.overlay.encode("utf-8"))
    contents = {generated[0]: source_text(selection).encode("utf-8"), generated[1]: notice_bytes}
    for relative, data in contents.items():
        (dest / relative).parent.mkdir(parents=True, exist_ok=True)
        (dest / relative).write_bytes(data)
    verify_staging(source, dest, paths, selection.manifest_path, config.overlay, contents)
    diff = "".join(difflib.unified_diff(
        original.decode("utf-8").splitlines(keepends=True),
        (original + config.overlay.encode("utf-8")).decode("utf-8").splitlines(keepends=True),
        fromfile=f"a/{selection.manifest_path}", tofile=f"b/{selection.manifest_path}"))
    return Staged(dest, selection.manifest_path, generated, len(paths), diff)


def verify_staging(source: Path, dest: Path, paths: Sequence[str], manifest_path: str, overlay: str,
                   generated: Mapping[str, bytes]) -> None:
    """Refuse a staged tree that differs from the checkout anywhere but the
    appended opt-in and the generated files."""
    expected = set(paths) | set(generated)
    found: set[str] = set()
    for directory, dirnames, filenames in os.walk(dest):
        base = Path(directory)
        for name in [*filenames, *(d for d in dirnames if (base / d).is_symlink())]:
            found.add((base / name).relative_to(dest).as_posix())
    extra = sorted(found - expected)
    missing = sorted(expected - found)
    if extra or missing:
        raise DistributionError(f"staging {dest.as_posix()}", f"file set differs from the checkout: extra {extra[:5]}, missing {missing[:5]}")
    for relative in paths:
        origin, staged = source / relative, dest / relative
        if origin.is_symlink() or staged.is_symlink():
            if not (origin.is_symlink() and staged.is_symlink() and os.readlink(origin) == os.readlink(staged)):
                raise DistributionError(f"staging {relative}", "symlink differs from the checkout")
            continue
        want = origin.read_bytes()
        if relative == manifest_path:
            want += overlay.encode("utf-8")
        if staged.read_bytes() != want:
            raise DistributionError(f"staging {relative}", "differs from the tagged checkout; only the dist opt-in may change")
    for relative, data in generated.items():
        if (dest / relative).read_bytes() != data:
            raise DistributionError(f"staging {relative}", "differs from the generated content")


# ---------------------------------------------------------------------------
# dist plan validation
# ---------------------------------------------------------------------------


def expected_release_assets(selection: Selection) -> set[str]:
    """Every file a selected release publishes besides the provenance record."""
    names = {f"{selection.package}-installer.sh", "sha256.sum"}
    if "brew" in selection.channels:
        names.add(f"{selection.channels['brew']}.rb")
    for target in TARGETS:
        names |= {archive_name(selection.package, target), archive_name(selection.package, target) + ".sha256"}
    return names


def archive_name(package: str, target: str) -> str:
    return f"{package}-{target}.tar.xz"


def validate_plan(selection: Selection, plan: Any) -> None:
    """A `dist plan --output-format=json` manifest releases exactly the
    selected package and version, for exactly the four targets, shipping only
    the selected command plus the license, readme, and generated notices, from
    the tag's own download path, with no installer or publisher beyond the
    effective channels. Field names follow dist 0.34.0's real output
    (`fixtures/distribution/dist-plan-*.json`)."""
    fields = _json_fields(plan, "dist plan")
    if fields.string(["dist_version"]) != DIST_VERSION:
        raise fields.error(["dist_version"], f"must be {DIST_VERSION!r}")
    if fields.string(["announcement_tag"]) != selection.tag:
        raise fields.error(["announcement_tag"], f"must be {selection.tag!r}")
    prerelease = parse_version(selection.version, "selection.version").prerelease is not None
    if fields.boolean(["announcement_is_prerelease"]) != prerelease:
        raise fields.error(["announcement_is_prerelease"], f"must be {str(prerelease).lower()} for {selection.version}")
    releases = fields.records(["releases"], allow_empty=False)
    if len(releases) != 1:
        raise fields.error(["releases"], f"must release exactly one package, found {len(releases)}")
    if fields.string(["releases", 0, "app_name"]) != selection.package:
        raise fields.error(["releases", 0, "app_name"], f"must be {selection.package!r}")
    if fields.string(["releases", 0, "app_version"]) != selection.version:
        raise fields.error(["releases", 0, "app_version"], f"must be {selection.version!r}")
    owner, repo = CANONICAL_REPOSITORY.split("/")
    hosting = ["releases", 0, "hosting", "github"]
    for key, want in (("artifact_base_url", "https://github.com"),
                      ("artifact_download_path", f"/{CANONICAL_REPOSITORY}/releases/download/{selection.tag}"),
                      ("owner", owner), ("repo", repo)):
        if fields.string([*hosting, key]) != want:
            raise fields.error([*hosting, key], f"must be {want!r}")
    if set(fields.mapping(["releases", 0, "hosting"])) != {"github"}:
        raise fields.error(["releases", 0, "hosting"], "must host on GitHub only")
    names = fields.strings(["releases", 0, "artifacts"], allow_empty=False)
    expected = expected_release_assets(selection)
    repeated = sorted({name for name in names if names.count(name) > 1})
    if repeated:
        raise fields.error(["releases", 0, "artifacts"], f"listed more than once: {repeated}")
    if set(names) != expected:
        raise fields.error(["releases", 0, "artifacts"],
                           f"must be exactly {sorted(expected)}; unexpected {sorted(set(names) - expected)}, missing {sorted(expected - set(names))}")
    artifacts = fields.mapping(["artifacts"])
    for name in names:
        if name not in artifacts:
            raise fields.error(["releases", 0, "artifacts"], f"names {name!r}, which artifacts does not define")
    for target in TARGETS:
        name = archive_name(selection.package, target)
        base = ["artifacts", name]
        if fields.string([*base, "kind"]) != "executable-zip":
            raise fields.error([*base, "kind"], "must be 'executable-zip'")
        if fields.strings([*base, "target_triples"], allow_empty=False) != [target]:
            raise fields.error([*base, "target_triples"], f"must be [{target!r}]")
        if fields.string([*base, "checksum"]) != name + ".sha256":
            raise fields.error([*base, "checksum"], f"must be {name + '.sha256'!r}")
        contents = []
        for index in range(len(fields.records([*base, "assets"], allow_empty=False))):
            name_in_archive = fields.asset_name([*base, "assets", index, "name"])
            if fields.asset_name([*base, "assets", index, "path"]) != name_in_archive:
                raise fields.error([*base, "assets", index, "path"], f"must be {name_in_archive!r}, at the archive's top level")
            contents.append((fields.string([*base, "assets", index, "kind"]), name_in_archive))
        contents.sort()
        executables = [asset for kind, asset in contents if kind == "executable"]
        if executables != [selection.command]:
            raise fields.error([*base, "assets"], f"must ship exactly [{selection.command!r}], found {executables}")
        other = [(kind, asset) for kind, asset in contents if kind != "executable"]
        if not {("license", "LICENSE"), ("readme", "README.md")} <= set(other):
            raise fields.error([*base, "assets"], f"must include the root LICENSE and the package README, found {other}")
        for kind, asset in other:
            # dist includes a package's release-plz `CHANGELOG.md` when the
            # package directory has one; it is documentation, never code.
            if (kind, asset) == ("changelog", CHANGELOG_NAME):
                continue
            if kind not in ("license", "readme") and asset not in GENERATED_FILES:
                raise fields.error([*base, "assets"], f"carries an unexpected {kind} asset {asset!r}")
        if not set(GENERATED_FILES) <= {asset for _, asset in other}:
            raise fields.error([*base, "assets"], f"must include the generated {list(GENERATED_FILES)}")
    installers = ["artifacts", f"{selection.package}-installer.sh"]
    if fields.string([*installers, "kind"]) != "installer":
        raise fields.error([*installers, "kind"], "must be 'installer'")
    if sorted(fields.strings([*installers, "target_triples"], allow_empty=False)) != sorted(TARGETS):
        raise fields.error([*installers, "target_triples"], f"must cover exactly {sorted(TARGETS)}")


# ---------------------------------------------------------------------------
# Provenance: one deterministic record of what was built, from dist's outputs
# ---------------------------------------------------------------------------

_SUM_LINE = re.compile(r"^([0-9a-f]{64}) [ *](\S+)$")


def read_checksums(assets: Path) -> dict[str, str]:
    """dist's `sha256.sum`, cross-checked with every `.sha256` sidecar it
    wrote. No digest is computed here: these are dist's own values, and
    `scripts/distribution/installers.py check-sums` verifies them against the
    bytes with the platform's checksum tool."""
    source = (assets / "sha256.sum").as_posix()
    try:
        text = (assets / "sha256.sum").read_text(encoding="utf-8")
    except OSError as error:
        raise DistributionError(source, f"cannot read dist's unified checksums: {error}") from None
    sums: dict[str, str] = {}
    for number, line in enumerate(text.splitlines(), 1):
        if not line:
            continue  # dist ends the file with a blank line
        match = _SUM_LINE.match(line)
        if not match:
            raise DistributionError(f"{source}:{number}", f"is not '<sha256>  <name>': {line!r}")
        if match.group(2) in sums:
            raise DistributionError(f"{source}:{number}", f"lists {match.group(2)!r} twice")
        sums[match.group(2)] = match.group(1)
    for sidecar in sorted(assets.glob("*.sha256")):
        line = sidecar.read_text(encoding="utf-8").strip()
        match = _SUM_LINE.match(line)
        name = sidecar.name[: -len(".sha256")]
        if not match or match.group(2) != name:
            raise DistributionError(sidecar.as_posix(), f"is not '<sha256>  {name}': {line!r}")
        if sums.get(name) != match.group(1):
            raise DistributionError(sidecar.as_posix(), f"disagrees with sha256.sum for {name}")
    return sums


def build_provenance(selection: Selection, plan: Any, assets: Path, environments: Sequence[Any]) -> dict[str, Any]:
    """`distribution-provenance.json`: package, tag, commit, targets,
    toolchain, dist pin, build images, and every asset's size and dist
    checksum, in sorted order so the same inputs give the same bytes. Run
    URLs and times stay out; they belong to the workflow artifacts."""
    validate_plan(selection, plan)
    expected = expected_release_assets(selection)
    present = set(_regular_files(assets)) - {PROVENANCE_NAME}
    if present != expected:
        raise DistributionError(assets.as_posix(), f"must hold exactly the planned assets; unexpected {sorted(present - expected)}, missing {sorted(expected - present)}")
    sums = read_checksums(assets)
    for target in TARGETS:
        if archive_name(selection.package, target) not in sums:
            raise DistributionError((assets / "sha256.sum").as_posix(), f"has no checksum for {archive_name(selection.package, target)}")
    builds: dict[str, dict[str, Any]] = {}
    for index, environment in enumerate(environments):
        fields = _json_fields(environment, f"build environment {index}")
        target = fields.string(["target"])
        if target not in TARGETS or target in builds:
            raise fields.error(["target"], f"must be one of {list(TARGETS)}, once each")
        image = fields.mapping(["image"])
        if not image:
            raise fields.error(["image"], "must not be empty; provenance names the build image")
        for key in image:
            fields.string(["image", key])
        builds[target] = {"rustc": fields.string(["rustc"]), "image": dict(sorted(image.items()))}
    if set(builds) != set(TARGETS):
        raise DistributionError("build environments", f"must cover {sorted(TARGETS)}, found {sorted(builds)}")
    toolchains = {build["rustc"] for build in builds.values()}
    if len(toolchains) != 1:
        raise DistributionError("build environments", f"must share one Rust toolchain, found {sorted(toolchains)}")
    return {
        "schema_version": 1,
        "package": selection.package,
        "command": selection.command,
        "version": selection.version,
        "tag": selection.tag,
        "commit": selection.commit,
        "source": f"{CANONICAL_URL}/tree/{selection.tag}",
        "tools": {"dist": DIST_VERSION, "rustc": toolchains.pop()},
        "targets": [{"target": target, "archive": archive_name(selection.package, target), **builds[target]} for target in TARGETS],
        "assets": [{"name": name, "size": (assets / name).stat().st_size, "sha256": sums.get(name)} for name in sorted(expected)],
    }


# ---------------------------------------------------------------------------
# Immutable assets and the Homebrew formula
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class RemoteAsset:
    name: str
    size: int
    asset_id: int
    url: str


@dataclass(frozen=True)
class AssetPlan:
    reuse: list[str]
    upload: list[str]
    conflicts: list[str]
    unrelated: list[str]

    def as_json(self) -> dict[str, Any]:
        return {"reuse": self.reuse, "upload": self.upload, "conflicts": self.conflicts, "unrelated": self.unrelated}


def remote_assets(listing: Any, *, source: str) -> list[RemoteAsset]:
    """A Release's `assets` list from the API. An empty list is valid before
    the first upload; duplicate names are not."""
    fields = _json_fields(listing, source)
    assets = []
    seen: set[str] = set()
    for index, _ in enumerate(fields.records([], allow_empty=True) if isinstance(listing, list) else fields.records(["assets"], allow_empty=True)):
        base = [index] if isinstance(listing, list) else ["assets", index]
        name = fields.asset_name([*base, "name"])
        if name in seen:
            raise fields.error([*base, "name"], f"duplicate asset {name!r}")
        seen.add(name)
        size = fields.integer([*base, "size"])
        if size < 0:
            raise fields.error([*base, "size"], "must not be negative")
        assets.append(RemoteAsset(name, size, fields.integer([*base, "id"]), fields.string([*base, "browser_download_url"])))
    return assets


def reconcile_assets(
    expected: Mapping[str, Path], existing: Sequence[RemoteAsset], read_remote: Callable[[RemoteAsset], Iterator[bytes]]
) -> AssetPlan:
    """Compare every expected asset with the Release before anything is
    uploaded. An identical asset is reused, a missing one is uploaded, and a
    same-name asset with different bytes is a conflict the caller must refuse;
    nothing is ever replaced or deleted."""
    if not expected:
        raise DistributionError("expected assets", "must not be empty")
    by_name = {asset.name: asset for asset in existing}
    reuse, upload, conflicts = [], [], []
    for name in sorted(expected):
        local = expected[name]
        remote = by_name.get(name)
        if remote is None:
            upload.append(name)
        elif remote.size == local.stat().st_size and _same_bytes(local, read_remote(remote), remote):
            reuse.append(name)
        else:
            conflicts.append(name)
    unrelated = sorted(set(by_name) - set(expected))
    return AssetPlan(reuse, upload, conflicts, unrelated)


def _same_bytes(local: Path, remote: Iterator[bytes], asset: RemoteAsset) -> bool:
    """Byte equality in bounded chunks. A remote body shorter than its listed
    size is an error, not a difference."""
    received = 0
    pending = b""
    with local.open("rb") as handle:
        for chunk in remote:
            received += len(chunk)
            pending += chunk
            while len(pending) >= _CHUNK or (pending and received >= asset.size):
                take = pending[:_CHUNK]
                if handle.read(len(take)) != take:
                    return False
                pending = pending[len(take):]
        if received != asset.size:
            raise DistributionError(f"asset {asset.name}", f"download returned {received} of {asset.size} bytes")
        if pending and handle.read(len(pending)) != pending:
            return False
        return handle.read(1) == b""


def download(asset: RemoteAsset, *, token: str | None = None) -> Iterator[bytes]:
    request = urllib.request.Request(asset.url)
    request.add_header("Accept", "application/octet-stream")
    if token:
        request.add_header("Authorization", f"Bearer {token}")
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            while chunk := response.read(_CHUNK):
                yield chunk
    except (urllib.error.URLError, TimeoutError, OSError) as error:
        raise DistributionError(f"asset {asset.name}", f"download failed: {error}") from None


@dataclass(frozen=True)
class FormulaVersion:
    version: str
    formula: bytes


@dataclass(frozen=True)
class TapDecision:
    action: str  # create, update, noop, skip
    reason: str


def select_tap_update(candidate: FormulaVersion, installed: FormulaVersion | None) -> TapDecision:
    """Whether the tap's formula changes, by SemVer order, never lexically. An
    older candidate is a successful no-op (a retried old run never
    downgrades); the same version must have the same formula bytes."""
    version = parse_version(candidate.version, "candidate formula version")
    if version.prerelease is not None:
        return TapDecision("skip", f"{version} is a prerelease; the stable formula is unchanged")
    if installed is None:
        return TapDecision("create", f"the tap has no formula; creating {version}")
    current = parse_version(installed.version, "installed formula version")
    if version > current:
        return TapDecision("update", f"{current} -> {version}")
    if version < current:
        return TapDecision("noop", f"the tap already has newer {current}; {version} is not installed over it")
    if candidate.formula != installed.formula:
        raise DistributionError("formula", f"version {version} is already in the tap with different contents; publish a new version")
    return TapDecision("noop", f"the tap already has identical {version}")


# ---------------------------------------------------------------------------
# Build runners
# ---------------------------------------------------------------------------


def build_matrix(selection: Selection, images: Any, *, source: str) -> list[dict[str, str]]:
    """One build job per selected target, in `TARGETS` order, on the runner
    `.github/distribution/build-images.json` pins for that target."""
    fields = _json_fields(images, source)
    fields.mapping(["targets"])
    entries = []
    for target in selection.targets:
        base = ["targets", target]
        fields.mapping(base)
        runner = fields.string([*base, "runner"])
        family = fields.string([*base, "os"])
        expected = "macos" if target.endswith("-apple-darwin") else "linux"
        if family != expected:
            raise fields.error([*base, "os"], f"must be {expected!r} for {target}, found {family!r}")
        entries.append({"target": target, "runner": runner, "os": family})
    return entries


# ---------------------------------------------------------------------------
# Build bundles: one verified build, kept as a workflow artifact so a partial
# upload is completed from the same bytes rather than a nondeterministic rebuild
# ---------------------------------------------------------------------------

BUNDLE_SCHEMA = 1
BUNDLE_RECORD = "bundle.json"
BUNDLE_ENTRIES = frozenset({BUNDLE_RECORD, "selection.json", "plan.json", "build-env", "assets"})
# The selection fields a reused bundle must share with the fresh preflight.
BUNDLE_IDENTITY = ("package", "command", "manifest_path", "version", "tag", "commit", "targets", "channels", "publishable")


def render_json(value: Any) -> bytes:
    """The one rendering of every JSON record this module writes to disk,
    including `distribution-provenance.json`, whose bytes are compared."""
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def bundle_assets(selection: Selection) -> set[str]:
    """Every asset a bundle carries and a Release receives."""
    return expected_release_assets(selection) | {PROVENANCE_NAME}


def _regular_files(directory: Path) -> dict[str, Path]:
    """Every entry of an asset directory, which may hold only regular files:
    a subdirectory or symlink is an error, never skipped."""
    try:
        entries = sorted(directory.iterdir())
    except OSError as error:
        raise DistributionError(directory.as_posix(), f"cannot read: {error}") from None
    found: dict[str, Path] = {}
    for entry in entries:
        if entry.is_symlink() or not entry.is_file():
            raise DistributionError(entry.as_posix(), "must be a regular file")
        found[entry.name] = entry
    return found


def _exact_files(directory: Path, expected: set[str], what: str) -> dict[str, Path]:
    found = _regular_files(directory)
    if set(found) != expected:
        raise DistributionError(
            directory.as_posix(),
            f"must hold exactly {what}; unexpected {sorted(set(found) - expected)}, missing {sorted(expected - set(found))}",
        )
    return found


def _check_provenance(selection: Selection, plan: Any, assets: Path, environments: Sequence[Any]) -> None:
    stored = assets / PROVENANCE_NAME
    if stored.read_bytes() != render_json(build_provenance(selection, plan, assets, environments)):
        raise DistributionError(stored.as_posix(), "differs from the record its plan, assets, and build environments render")


@dataclass(frozen=True)
class Bundle:
    root: Path
    run_id: int
    selection: Selection
    assets: dict[str, Path]


def write_bundle(selection: Selection, plan: Any, assets: Path, environments: Sequence[Any], run_id: int, output: Path) -> dict[str, Any]:
    """Copy one complete, self-consistent build into `output` and return its
    `bundle.json` record. `assets` must hold exactly the release assets and
    the provenance record `provenance --write` rendered from them."""
    if not isinstance(selection, Selection):
        raise TypeError("write_bundle requires a Selection from a successful preflight")
    if isinstance(run_id, bool) or not isinstance(run_id, int) or run_id <= 0:
        raise DistributionError("--run-id", f"must be a positive integer, found {run_id!r}")
    if output.exists() and (not output.is_dir() or any(output.iterdir())):
        raise DistributionError(output.as_posix(), "must be absent or empty; a bundle is never written over another")
    files = _exact_files(assets, bundle_assets(selection), f"the release assets and {PROVENANCE_NAME}")
    _check_provenance(selection, plan, assets, environments)
    record = {
        "schema_version": BUNDLE_SCHEMA,
        "package": selection.package,
        "version": selection.version,
        "tag": selection.tag,
        "commit": selection.commit,
        "run_id": run_id,
        "assets": {name: files[name].stat().st_size for name in sorted(files)},
    }
    output.mkdir(parents=True, exist_ok=True)
    (output / "selection.json").write_bytes(render_json(selection.record))
    (output / "plan.json").write_bytes(render_json(plan))
    (output / "build-env").mkdir()
    for index, environment in enumerate(environments):
        target = _json_fields(environment, f"build environment {index}").string(["target"])
        (output / "build-env" / f"{target}.json").write_bytes(render_json(environment))
    (output / "assets").mkdir()
    for name, path in files.items():
        shutil.copyfile(path, output / "assets" / name)
    (output / BUNDLE_RECORD).write_bytes(render_json(record))
    # The written copy must itself pass the check a later run applies.
    _check_bundle_contents(output, selection)
    return record


def _check_bundle_contents(bundle: Path, fresh: Selection) -> Bundle:
    try:
        entries = {entry.name for entry in bundle.iterdir()}
    except OSError as error:
        raise DistributionError(bundle.as_posix(), f"cannot read the build bundle: {error}") from None
    if entries != BUNDLE_ENTRIES:
        raise DistributionError(bundle.as_posix(), f"must hold exactly {sorted(BUNDLE_ENTRIES)}; unexpected {sorted(entries - BUNDLE_ENTRIES)}, missing {sorted(BUNDLE_ENTRIES - entries)}")
    record_path = bundle / BUNDLE_RECORD
    fields = _json_fields(_read_json_file(record_path), record_path.as_posix())
    if fields.integer(["schema_version"]) != BUNDLE_SCHEMA:
        raise fields.error(["schema_version"], f"must be {BUNDLE_SCHEMA}")
    run_id = fields.integer(["run_id"])
    if run_id <= 0:
        raise fields.error(["run_id"], "must be a positive workflow run id")
    selection_path = bundle / "selection.json"
    stored = Selection.from_json(_read_json_file(selection_path), source=selection_path.as_posix())
    for key in ("package", "version", "tag", "commit"):
        if fields.string([key]) != stored.record[key]:
            raise fields.error([key], f"is {fields.string([key])!r}, but selection.json names {stored.record[key]!r}")
    for key in BUNDLE_IDENTITY:
        if stored.record[key] != fresh.record[key]:
            raise DistributionError(f"{selection_path.as_posix()}: {key}", f"is {stored.record[key]!r}, but the fresh selection is {fresh.record[key]!r}")
    plan = _read_json_file(bundle / "plan.json")
    validate_plan(fresh, plan)
    env_files = _exact_files(bundle / "build-env", {f"{target}.json" for target in fresh.targets}, "one build environment per target")
    environments = []
    for target in fresh.targets:
        path = env_files[f"{target}.json"]
        environment = _read_json_file(path)
        if _json_fields(environment, path.as_posix()).string(["target"]) != target:
            raise DistributionError(f"{path.as_posix()}: target", f"must be {target!r}")
        environments.append(environment)
    assets = _exact_files(bundle / "assets", bundle_assets(fresh), f"the release assets and {PROVENANCE_NAME}")
    sizes = fields.mapping(["assets"])
    if set(sizes) != set(assets):
        raise fields.error(["assets"], f"must list exactly {sorted(assets)}; unexpected {sorted(set(sizes) - set(assets))}, missing {sorted(set(assets) - set(sizes))}")
    for name in sorted(assets):
        size = fields.integer(["assets", name])
        if size != assets[name].stat().st_size:
            raise fields.error(["assets", name], f"records {size} bytes, but the bundled file has {assets[name].stat().st_size}")
    _check_provenance(fresh, plan, bundle / "assets", environments)
    return Bundle(bundle, run_id, stored, assets)


def check_bundle(bundle: Path, fresh: Selection, *, current_run_id: int | None = None, run: Any = None, jobs: Any = None,
                 run_source: str = "run", jobs_source: str = "jobs") -> Bundle:
    """Validate a build bundle against a fresh preflight before it is reused
    or uploaded. A bundle from another run (or when the current run is not
    named) is accepted only with that run's API documents proving the
    distribution workflow's `Package` job succeeded."""
    if not isinstance(fresh, Selection):
        raise TypeError("check_bundle requires a Selection from a fresh preflight")
    found = _check_bundle_contents(bundle, fresh)
    if current_run_id is None or found.run_id != current_run_id:
        if run is None or jobs is None:
            raise DistributionError(
                f"{(bundle / BUNDLE_RECORD).as_posix()}: run_id",
                f"names run {found.run_id}, not this run; a reused bundle needs that run's API documents (--run and --jobs)",
            )
        check_bundle_run(run, jobs, found.run_id, run_source=run_source, jobs_source=jobs_source)
    return found


def check_bundle_run(run: Any, jobs: Any, run_id: int, *, run_source: str = "run", jobs_source: str = "jobs") -> None:
    """`GET /repos/{repo}/actions/runs/{id}` and its `/jobs` show the
    canonical distribution workflow's run whose `Package` job succeeded. The
    run's overall conclusion is not read: the upload is what may have failed."""
    fields = _json_fields(run, run_source)
    if fields.integer(["id"]) != run_id:
        raise fields.error(["id"], f"is {fields.integer(['id'])}, but the bundle names run {run_id}")
    repository = fields.string(["repository", "full_name"])
    if repository != CANONICAL_REPOSITORY:
        raise fields.error(["repository", "full_name"], f"is {repository!r}, not {CANONICAL_REPOSITORY}")
    path = fields.string(["path"])
    if path != DISTRIBUTION_WORKFLOW and not path.startswith(f"{DISTRIBUTION_WORKFLOW}@"):
        raise fields.error(["path"], f"is {path!r}; only {DISTRIBUTION_WORKFLOW} produces a reusable bundle")
    job_fields = _json_fields(jobs, jobs_source)
    records = job_fields.records(["jobs"], allow_empty=False)
    matches = [index for index in range(len(records)) if job_fields.string(["jobs", index, "name"]) == PACKAGE_JOB]
    if len(matches) != 1:
        raise job_fields.error(["jobs"], f"must hold exactly one {PACKAGE_JOB!r} job (the latest attempt), found {len(matches)}")
    conclusion_path = ["jobs", matches[0], "conclusion"]
    if _get(jobs, conclusion_path) is None:
        raise job_fields.error(conclusion_path, f"is null: the {PACKAGE_JOB} job has not completed")
    conclusion = job_fields.string(conclusion_path)
    if conclusion != "success":
        raise job_fields.error(conclusion_path, f"is {conclusion!r}; only a successful {PACKAGE_JOB} job's bundle is reused")


# ---------------------------------------------------------------------------
# Rollout ledger: the first `bh` chain gates every other package
# ---------------------------------------------------------------------------

# `bh` proves the whole chain first. Until the ledger holds a verified entry
# for it that records every channel as `pass`, tied to a successful
# distribution run, preflight refuses every other package even when
# `docs/deployment.yaml` approves it. Entries are evidence carried in by a
# reviewed repository change, never approvals.
ROLLOUT_LEDGER = ".github/distribution/rollout.json"
FIRST_PACKAGE = "biscuit-hash-cli"
ROLLOUT_CHANNELS = ("archive", "shell-installer", "brew", "binstall")
ROLLOUT_TOP_LEVEL = frozenset({"first_package", "verified", "pending"})
ROLLOUT_ENTRY_FIELDS = frozenset({"package", "version", "tag", "commit", "run_id", "run_url", "release_url", "targets", "channels"})
ROLLOUT_PENDING_FIELDS = frozenset({"name", "package", "reason"})
# What a verified entry may record per channel. `pass` is only ever an
# installation the run itself performed: the archive smoke and the shell
# installer on every target, `brew install` from the tap on every target, and
# cargo-binstall on every target. `unchanged` is a formula this run did not
# install (a prerelease, or an older retry under a newer formula); `disabled`
# is a channel the package does not ship. Both are valid records of a release,
# but only an entry with every channel `pass` is the first complete chain.
ROLLOUT_CHANNEL_RESULTS: Mapping[str, frozenset[str]] = {
    "archive": frozenset({"pass"}),
    "shell-installer": frozenset({"pass"}),
    "brew": frozenset({"pass", "unchanged", "disabled"}),
    "binstall": frozenset({"pass", "disabled"}),
}


@dataclass(frozen=True)
class RolloutEntry:
    package: str
    version: semver.Version
    tag: str
    commit: str
    run_id: int
    channels: tuple[tuple[str, str], ...]  # (channel, result), in ROLLOUT_CHANNELS order

    @property
    def incomplete(self) -> list[str]:
        """The channels this entry did not prove by installation."""
        return [f"{channel} {result!r}" for channel, result in self.channels if result != "pass"]


@dataclass(frozen=True)
class RolloutLedger:
    source: str
    first_package: str
    verified: tuple[RolloutEntry, ...]
    pending: tuple[tuple[str, str], ...]  # (name, package)


def _exact_keys(fields: Fields, path: list[Any], allowed: frozenset[str]) -> dict[str, Any]:
    value = fields.mapping(path)
    for key in value:
        if key not in allowed:
            raise fields.error([*path, key], f"unknown field; expected one of {sorted(allowed)}")
    for key in sorted(allowed):
        fields.present([*path, key])
    return value


def _rollout_entry(fields: Fields, path: list[Any]) -> RolloutEntry:
    _exact_keys(fields, path, ROLLOUT_ENTRY_FIELDS)
    package = fields.string([*path, "package"])
    if package not in ROSTER:
        raise fields.error([*path, "package"], f"{package!r} does not ship a binary")
    version = fields.version([*path, "version"])
    tag = fields.string([*path, "tag"])
    if tag != f"{package}-v{version}":
        raise fields.error([*path, "tag"], f"must be {package}-v{version}, found {tag!r}")
    commit = fields.sha([*path, "commit"])
    run_id = fields.integer([*path, "run_id"])
    if run_id <= 0:
        raise fields.error([*path, "run_id"], f"must be a positive run id, found {run_id}")
    for key, expected in (("run_url", f"{CANONICAL_URL}/actions/runs/{run_id}"),
                          ("release_url", f"{CANONICAL_URL}/releases/tag/{tag}")):
        if fields.string([*path, key]) != expected:
            raise fields.error([*path, key], f"must be {expected}")
    _exact_keys(fields, [*path, "targets"], frozenset(TARGETS))
    for target in TARGETS:
        if fields.string([*path, "targets", target]) != "pass":
            raise fields.error([*path, "targets", target], "must be 'pass'; a verified entry records only a target whose install passed")
    _exact_keys(fields, [*path, "channels"], frozenset(ROLLOUT_CHANNELS))
    channels = []
    for channel in ROLLOUT_CHANNELS:
        result = fields.string([*path, "channels", channel])
        if result not in ROLLOUT_CHANNEL_RESULTS[channel]:
            raise fields.error([*path, "channels", channel], f"must be one of {sorted(ROLLOUT_CHANNEL_RESULTS[channel])}, found {result!r}")
        channels.append((channel, result))
    return RolloutEntry(package, version, tag, commit, run_id, tuple(channels))


def parse_rollout(data: bytes, *, source: str) -> RolloutLedger:
    """Validate the rollout ledger. Both lists may be empty (nothing verified,
    nothing pending); every other shape is a located error."""
    fields = _json_fields(load_json(data, source=source), source)
    _exact_keys(fields, [], ROLLOUT_TOP_LEVEL)
    first = fields.string(["first_package"])
    if first != FIRST_PACKAGE:
        raise fields.error(["first_package"], f"must be {FIRST_PACKAGE!r}; the first chain is not configurable")
    verified: list[RolloutEntry] = []
    for index in range(len(fields.records(["verified"], allow_empty=True))):
        entry = _rollout_entry(fields, ["verified", index])
        if any(seen.tag == entry.tag for seen in verified):
            raise fields.error(["verified", index, "tag"], f"{entry.tag} is already recorded")
        verified.append(entry)
    pending: list[tuple[str, str]] = []
    for index in range(len(fields.records(["pending"], allow_empty=True))):
        base = ["pending", index]
        _exact_keys(fields, base, ROLLOUT_PENDING_FIELDS)
        name = fields.string([*base, "name"])
        if not _CHANNEL_NAME.match(name):
            raise fields.error([*base, "name"], f"must match {_CHANNEL_NAME.pattern}")
        package = fields.string([*base, "package"])
        if package not in ROSTER:
            raise fields.error([*base, "package"], f"{package!r} does not ship a binary")
        fields.string([*base, "reason"])
        if (name, package) in pending:
            raise fields.error([*base, "name"], f"{name!r} is already pending for {package}")
        pending.append((name, package))
    return RolloutLedger(source, first, tuple(verified), tuple(pending))


def load_rollout(path: Path, *, source: str | None = None) -> RolloutLedger:
    return parse_rollout(_read_file(path), source=source or path.as_posix())


def check_rollout(package: str, ledger: RolloutLedger) -> RolloutEntry | None:
    """The newest verified first-package entry whose every channel is `pass`
    (the first complete chain), which permits distributing `package`; None for
    the first package itself, which needs none. An entry with a `disabled` or
    `unchanged` channel is a valid record but never permits widening."""
    if package == ledger.first_package:
        return None
    entries = [entry for entry in ledger.verified if entry.package == ledger.first_package]
    if not entries:
        raise DistributionError(
            ledger.source,
            f"{package} cannot be distributed before the first {ledger.first_package} release is verified: "
            f"the ledger has no verified {ledger.first_package} entry (see docs/distribution.md)",
        )
    complete = [entry for entry in entries if not entry.incomplete]
    if not complete:
        found = "; ".join(f"verified[{ledger.verified.index(entry)}] {entry.tag} records {', '.join(entry.incomplete)}"
                          for entry in entries)
        raise DistributionError(
            ledger.source,
            f"{package} cannot be distributed before the first {ledger.first_package} release proves the complete chain: "
            f"no verified {ledger.first_package} entry records every channel as 'pass' ({found}) (see docs/distribution.md)",
        )
    return max(complete, key=lambda entry: entry.version)


def check_policy_rollout(policy: Policy, ledger: RolloutLedger) -> None:
    """Every package the policy approves is permitted by the ledger."""
    for package, entry in sorted(policy.entries.items()):
        if entry.approved:
            try:
                check_rollout(package, ledger)
            except DistributionError as error:
                raise DistributionError(policy.document.where(["packages", package, "approved"]), error.message) from None


def check_rollout_run(run: Any, entry: RolloutEntry, *, source: str) -> None:
    """`GET /repos/{repo}/actions/runs/{id}` shows the ledger entry's run: the
    canonical distribution workflow, completed successfully."""
    fields = _json_fields(run, source)
    if fields.integer(["id"]) != entry.run_id:
        raise fields.error(["id"], f"is {fields.integer(['id'])}, but the ledger names run {entry.run_id}")
    repository = fields.string(["repository", "full_name"])
    if repository != CANONICAL_REPOSITORY:
        raise fields.error(["repository", "full_name"], f"is {repository!r}, not {CANONICAL_REPOSITORY}")
    path = fields.string(["path"])
    if path != DISTRIBUTION_WORKFLOW and not path.startswith(f"{DISTRIBUTION_WORKFLOW}@"):
        raise fields.error(["path"], f"is {path!r}; only a {DISTRIBUTION_WORKFLOW} run verifies a rollout")
    if fields.present(["status"]) != "completed" or _get(run, ["conclusion"]) is None:
        raise fields.error(["conclusion"], "is not set: the run has not completed")
    conclusion = fields.string(["conclusion"])
    if conclusion != "success":
        raise fields.error(["conclusion"], f"is {conclusion!r}; only a successful run verifies a rollout")


def gate_rollout(result: Preflight, ledger: RolloutLedger, forge: Forge) -> None:
    """Refuse a selected release that widens past the first package without a
    verified first-package entry whose run GitHub confirms succeeded."""
    if result.status != SELECTED or result.record is None:
        return
    entry = check_rollout(result.record["package"], ledger)
    if entry is None:
        return
    run = forge.workflow_run(entry.run_id)
    if run is None:
        raise DistributionError(ledger.source, f"verified {entry.tag} names run {entry.run_id}, which does not exist")
    check_rollout_run(run, entry, source=f"run {entry.run_id}")


# Each workflow job's `needs.<job>.result`, as the candidate job passes it.
JOB_RESULTS = frozenset({"success", "failure", "cancelled", "skipped"})
# `needs.homebrew-tap.outputs.formula_current`: whether the tap's formula names
# this version after the tap job ran. Empty when the tap job did not run.
FORMULA_CURRENT = {"true": True, "false": False, "": None}


def rollout_candidate(selection: Selection, *, run_id: int, install: str, binstall: str, tap: str,
                      brew_install: str, formula_current: bool | None) -> dict[str, Any]:
    """The ledger entry a successful publishing run proposes. The workflow
    keeps it as an artifact; a reviewed change copies it into the ledger.

    A channel is `pass` only when the job that installs through it succeeded on
    every target; a publication job (the tap commit) never proves installation.
    Any combination of job results the workflow cannot produce is refused."""
    if not selection.publishable:
        raise DistributionError("selection", "a verification run proves no rollout")
    for name, result in (("install", install), ("binstall", binstall), ("tap", tap), ("brew-install", brew_install)):
        if result not in JOB_RESULTS:
            raise DistributionError(f"--{name}", f"must be one of {sorted(JOB_RESULTS)}, found {result!r}")
    if install != "success":
        raise DistributionError("install", f"is {install!r}; a rollout needs every target's install to pass")

    def expect(name: str, result: str, expected: str, why: str) -> None:
        if result != expected:
            raise DistributionError(name, f"is {result!r}; {why} must be {expected!r}")

    def skipped(why: str) -> None:
        expect("homebrew-tap", tap, "skipped", why)
        expect("brew-install", brew_install, "skipped", why)
        if formula_current is not None:
            raise DistributionError("formula-current", f"is {formula_current!r}, but the tap job did not run")

    if "brew" not in selection.channels:
        skipped("a disabled channel's job")
        brew = "disabled"
    elif not selection.formula_update:
        skipped("a release that does not update the formula (a prerelease)")
        brew = "unchanged"
    else:
        expect("homebrew-tap", tap, "success", "an enabled channel's job")
        if formula_current is None:
            raise DistributionError("formula-current", "is not set, but the tap job succeeded and must report it")
        if formula_current:
            expect("brew-install", brew_install, "success", "the Homebrew installation of a formula the tap holds at this version")
            brew = "pass"
        else:
            expect("brew-install", brew_install, "skipped", "the Homebrew installation under a newer formula")
            brew = "unchanged"

    if "binstall" in selection.channels:
        expect("binstall", binstall, "success", "an enabled channel's job")
        binstall_result = "pass"
    else:
        expect("binstall", binstall, "skipped", "a disabled channel's job")
        binstall_result = "disabled"
    entry = {
        "package": selection.package,
        "version": selection.version,
        "tag": selection.tag,
        "commit": selection.commit,
        "run_id": run_id,
        "run_url": f"{CANONICAL_URL}/actions/runs/{run_id}",
        "release_url": f"{CANONICAL_URL}/releases/tag/{selection.tag}",
        "targets": {target: "pass" for target in selection.targets},
        "channels": {
            "archive": "pass",
            "shell-installer": "pass",
            "brew": brew,
            "binstall": binstall_result,
        },
    }
    # The candidate must be exactly what the ledger accepts.
    _rollout_entry(_json_fields({"verified": [entry]}, "candidate"), ["verified", 0])
    return entry


# ---------------------------------------------------------------------------
# Immutable publication: attach missing assets to an existing Release
# ---------------------------------------------------------------------------


class AssetConflict(DistributionError):
    """A Release asset has the expected name but different bytes. Nothing is
    replaced or deleted; the version cannot be published from this build."""

    def __init__(self, where: str, message: str, conflicts: Sequence[str]) -> None:
        super().__init__(where, message)
        self.conflicts = list(conflicts)


class AssetExists(Exception):
    """An upload found its name already taken (HTTP 422 `already_exists`):
    a concurrent writer attached it first."""

    def __init__(self, name: str) -> None:
        super().__init__(name)
        self.name = name


class PublishForge(Forge, abc.ABC):
    """Release state plus the only write publication makes: attaching an
    asset. There is deliberately no method that edits or deletes."""

    @abc.abstractmethod
    def release(self, tag: str) -> ReleaseState | None: ...

    @abc.abstractmethod
    def tag_commit(self, tag: str) -> str | None: ...

    @abc.abstractmethod
    def proposal(self, commit: str) -> ProposalProof | PendingSetup | None: ...

    @abc.abstractmethod
    def list_assets(self, release_id: int) -> list[RemoteAsset]: ...

    @abc.abstractmethod
    def read_asset(self, asset: RemoteAsset) -> Iterator[bytes]: ...

    @abc.abstractmethod
    def upload_asset(self, release_id: int, name: str, path: Path) -> RemoteAsset:
        """Attach a new asset; raises `AssetExists` when the name is taken."""


_ASSET_PAGE = 100
_ASSET_PAGES = 10


class GitHubPublishForge(GitHubForge, PublishForge):
    """`GitHubForge` plus asset listing, public download, and upload."""

    def __init__(self, *, repository: str, token: str | None, bot_login: str | None, repo: Path) -> None:
        super().__init__(repository=repository, token=token, bot_login=bot_login, repo=repo)
        self.uploads = os.environ.get("GITHUB_UPLOAD_URL", "https://uploads.github.com").rstrip("/")

    def list_assets(self, release_id: int) -> list[RemoteAsset]:
        base = f"/repos/{self.repository}/releases/{release_id}/assets"
        listing: list[Any] = []
        for page in range(1, _ASSET_PAGES + 1):
            path = f"{base}?per_page={_ASSET_PAGE}&page={page}"
            status, body = self._get(path)
            if status != 200 or not isinstance(body, list):
                raise DistributionError(f"GET {path}", f"unexpected HTTP {status} or non-list body")
            listing += body
            if len(body) < _ASSET_PAGE:
                return remote_assets(listing, source=f"GET {base}")
        raise DistributionError(f"GET {base}", f"lists more than {_ASSET_PAGE * _ASSET_PAGES} assets")

    def read_asset(self, asset: RemoteAsset) -> Iterator[bytes]:
        # A published Release's download URL is public; the token is not sent
        # to the storage host it redirects to.
        return download(asset)

    def upload_asset(self, release_id: int, name: str, path: Path) -> RemoteAsset:
        url = f"{self.uploads}/repos/{self.repository}/releases/{release_id}/assets?name={urllib.parse.quote(name, safe='')}"
        status, body = self._upload(url, path)
        if status == 201:
            return remote_assets([body], source=f"POST {name}")[0]
        if status == 422 and _already_exists(body):
            raise AssetExists(name)
        raise DistributionError(f"POST {name}", f"unexpected HTTP {status}")

    def _upload(self, url: str, path: Path) -> tuple[int, Any]:
        if not self.token:
            raise DistributionError("GITHUB_TOKEN", "is required to upload release assets")
        with path.open("rb") as handle:
            request = urllib.request.Request(url, data=handle, method="POST")
            request.add_header("Accept", "application/vnd.github+json")
            request.add_header("X-GitHub-Api-Version", "2022-11-28")
            request.add_header("Authorization", f"Bearer {self.token}")
            request.add_header("Content-Type", "application/octet-stream")
            request.add_header("Content-Length", str(path.stat().st_size))
            try:
                with urllib.request.urlopen(request, timeout=300) as response:
                    return response.status, load_json(response.read(), source=f"POST {url}")
            except urllib.error.HTTPError as error:
                body = error.read()
                try:
                    return error.code, load_json(body, source=f"POST {url}")
                except DistributionError:
                    # The status alone is reported; an unreadable body never
                    # counts as `already_exists`.
                    return error.code, None
            except (urllib.error.URLError, TimeoutError, OSError) as error:
                raise DistributionError(f"POST {url}", f"upload failed: {error}") from None


def _already_exists(body: Any) -> bool:
    errors = body.get("errors") if isinstance(body, dict) else None
    return isinstance(errors, list) and any(isinstance(item, dict) and item.get("code") == "already_exists" for item in errors)


@dataclass(frozen=True)
class PublishResult:
    tag: str
    release_id: int
    reused: list[str]
    uploaded: list[str]
    unrelated: list[str]

    def as_json(self) -> dict[str, Any]:
        return {"status": "published", "tag": self.tag, "release_id": self.release_id,
                "reused": self.reused, "uploaded": self.uploaded, "unrelated": self.unrelated}


def upload_order(selection: Selection) -> list[str]:
    """Archives and their sidecars, the unified checksums, the installer, the
    formula, then the provenance record: its presence marks a complete set."""
    names = bundle_assets(selection)
    order = sorted(name for name in names if name.endswith(".tar.xz") or name.endswith(".tar.xz.sha256"))
    order += ["sha256.sum", f"{selection.package}-installer.sh"]
    if "brew" in selection.channels:
        order.append(f"{selection.channels['brew']}.rb")
    order.append(PROVENANCE_NAME)
    if sorted(order) != sorted(names):
        raise DistributionError("upload order", f"does not cover the bundle exactly: {sorted(set(order) ^ names)}")
    return order


def publish_release_assets(selection: Selection, assets: Mapping[str, Path], forge: PublishForge, current_policy: Policy, *,
                           fresh_rebuild: bool) -> PublishResult:
    """Attach the bundle's missing assets to the selected, already published
    Release. Approval, the Release, the tag, and the proposal are rechecked
    first; the complete expected set is then compared byte for byte, and any
    conflict uploads nothing. Existing identical assets are reused, unrelated
    ones are left alone, and nothing is ever edited or deleted."""
    if not isinstance(selection, Selection):
        raise TypeError("publish_release_assets requires a Selection from a successful preflight")
    fields = _json_fields(selection.record, "selection")
    mode = fields.string(["mode"])
    if mode not in ("release", "recovery") or fields.boolean(["publishable"]) is not True:
        raise DistributionError("selection.mode", f"is {mode!r}; only a publishable release or recovery selection uploads assets")
    release_id = fields.integer(["release", "id"])
    proof_number = fields.integer(["proposal", "number"])
    proof_head = fields.sha(["proposal", "head_sha"])
    expected = bundle_assets(selection)
    if set(assets) != expected:
        raise DistributionError("assets", f"must be exactly the bundle's; unexpected {sorted(set(assets) - expected)}, missing {sorted(expected - set(assets))}")
    order = upload_order(selection)
    tag = selection.tag

    _recheck_policy(selection, current_policy)
    state = forge.release(tag)
    if state is None:
        raise DistributionError(f"release {tag}", "no GitHub Release exists for this tag")
    if state.release_id != release_id:
        raise DistributionError(f"release {tag}", f"is Release {state.release_id}, but preflight selected Release {release_id}")
    if state.tag != tag:
        raise DistributionError(f"release {tag}", f"the Release names tag {state.tag!r}")
    if state.draft:
        raise DistributionError(f"release {tag}", "is a draft; assets are attached only to a published Release")
    commit = forge.tag_commit(tag)
    if commit is None:
        raise DistributionError(f"tag {tag}", "does not exist")
    if commit != selection.commit:
        raise DistributionError(f"tag {tag}", f"names {commit}, but preflight selected {selection.commit}; a published tag is never moved")
    proof = forge.proposal(commit)
    if isinstance(proof, PendingSetup):
        raise proof
    if proof is None or (proof.number, proof.head_sha) != (proof_number, proof_head):
        found = "none" if proof is None else f"#{proof.number} at {proof.head_sha}"
        raise DistributionError(f"tag {tag}", f"release proposal is {found}, but preflight proved #{proof_number} at {proof_head}")

    existing = forge.list_assets(release_id)
    present = sorted(expected & {asset.name for asset in existing})
    if fresh_rebuild and present:
        raise DistributionError(
            f"release {tag}",
            f"already has {present}; a partial release needs its original build bundle: rerun with the bundle run id",
        )
    plan = reconcile_assets(assets, existing, forge.read_asset)
    if plan.conflicts:
        raise AssetConflict(f"release {tag}", f"has different bytes under {plan.conflicts}; nothing was uploaded, and a changed build needs a new version", plan.conflicts)
    reused = list(plan.reuse)
    uploaded: list[str] = []
    missing = set(plan.upload)
    for name in order:
        if name not in missing:
            continue
        try:
            forge.upload_asset(release_id, name, assets[name])
            uploaded.append(name)
        except AssetExists:
            raced = [asset for asset in forge.list_assets(release_id) if asset.name == name]
            if not raced:
                raise DistributionError(f"asset {name}", "the upload reported it already exists, but the Release does not list it") from None
            if reconcile_assets({name: assets[name]}, raced, forge.read_asset).conflicts:
                raise AssetConflict(f"release {tag}", f"a concurrent writer attached {name!r} with different bytes", [name]) from None
            reused.append(name)

    final = reconcile_assets(assets, forge.list_assets(release_id), forge.read_asset)
    if final.reuse != sorted(expected):
        raise DistributionError(
            f"release {tag}",
            f"after upload not every asset is byte-identical: missing {final.upload}, conflicting {final.conflicts}",
        )
    return PublishResult(tag, release_id, sorted(reused), uploaded, final.unrelated)


def _recheck_policy(selection: Selection, current: Policy) -> None:
    where = f"{current.source}: packages.{selection.package}"
    entry = current.entries.get(selection.package)
    if entry is None or not entry.approved:
        raise DistributionError(f"{where}.approved", "approval was revoked after preflight; nothing was uploaded")
    for channel, name in sorted(selection.channels.items()):
        now = entry.channels.get(channel)
        if now != name:
            raise DistributionError(f"{where}.{channel}", f"resolved to {name!r} at preflight but is now {now!r}; nothing was uploaded")


# ---------------------------------------------------------------------------
# Command line
# ---------------------------------------------------------------------------


def _read_file(path: Path) -> bytes:
    """An input file's bytes; an unreadable input is invalid (exit 2), not a
    crash whose exit status 1 would read as a refused upload."""
    try:
        return path.read_bytes()
    except OSError as error:
        raise DistributionError(path.as_posix(), f"cannot read: {error}") from None


def _read_json_file(path: Path) -> Any:
    return load_json(_read_file(path), source=path.as_posix())


def _workspace(root: Path, metadata: Path | None) -> Workspace:
    if metadata is None:
        return cargo_workspace(root)
    def read(path: str) -> str | None:
        file = root / path
        return file.read_text(encoding="utf-8") if file.is_file() else None
    manifest = root / "Cargo.toml"
    excluded = excluded_package_names(_read_file(manifest), read, source=manifest.as_posix())
    return load_workspace(_read_file(metadata), source=metadata.as_posix(), excluded=excluded)


def _checkout_commit(root: Path) -> str:
    result = subprocess.run(["git", "rev-parse", "HEAD"], cwd=root, capture_output=True, text=True, timeout=60, check=False)
    if result.returncode != 0:
        raise DistributionError(f"git rev-parse HEAD ({root})", result.stderr.strip())
    return result.stdout.strip()


def _command_preflight(args: argparse.Namespace, env: Mapping[str, str]) -> tuple[int, Any]:
    tagged_ws = _workspace(args.tagged_root, args.tagged_metadata)
    current_ws = _workspace(args.current_root, args.current_metadata)
    tagged_policy_path = args.tagged_policy or args.tagged_root / "docs/deployment.yaml"
    current_policy_path = args.current_policy or args.current_root / "docs/deployment.yaml"
    policies = Policies(
        current=load_policy(current_policy_path, current_ws, source=f"current {current_policy_path.as_posix()}"),
        tagged=load_policy(tagged_policy_path, tagged_ws, source=f"tagged {tagged_policy_path.as_posix()}"),
    )
    if args.snapshot_dir:
        args.snapshot_dir.mkdir(parents=True, exist_ok=True)
        (args.snapshot_dir / "current-deployment.yaml").write_bytes(policies.current.raw)
        (args.snapshot_dir / "tagged-deployment.yaml").write_bytes(policies.tagged.raw)
    payload = _read_json_file(args.event_path)
    repository = env.get("GITHUB_REPOSITORY", "").strip()
    if not repository:
        raise DistributionError("GITHUB_REPOSITORY", "is required; it names the repository the run belongs to")
    if args.forge_fixture:
        forge: Forge = FixtureForge(_read_json_file(args.forge_fixture), source=args.forge_fixture.as_posix())
        checkout = args.checkout_commit
    else:
        forge = GitHubForge(repository=repository, token=env.get("GITHUB_TOKEN"), bot_login=env.get("RELEASE_APP_BOT_LOGIN") or None, repo=args.tagged_root)
        checkout = args.checkout_commit or _checkout_commit(args.tagged_root)
    facts = gather_facts(args.event_name, payload, source=args.event_path.as_posix(), repository=repository,
                         forge=forge, checkout_commit=checkout, tagged=tagged_ws)
    result = facts if isinstance(facts, Preflight) else select_release(facts, policies, tagged_ws)
    if result.status == SELECTED:
        rollout = args.rollout or args.current_root / ROLLOUT_LEDGER
        gate_rollout(result, load_rollout(rollout, source=f"current {rollout.as_posix()}"), forge)
    return (3 if result.status == PENDING_SETUP else 0), result.as_json()


def _command_config(args: argparse.Namespace, env: Mapping[str, str]) -> tuple[int, Any]:
    selection = Selection.from_json(_read_json_file(args.selection), source=args.selection.as_posix())
    config = render_dist_config(selection)
    return 0, {"manifest_path": config.manifest_path, "overlay": config.overlay}


def _command_stage(args: argparse.Namespace, env: Mapping[str, str]) -> tuple[int, Any]:
    selection = Selection.from_json(_read_json_file(args.selection), source=args.selection.as_posix())
    staged = stage_distribution(selection, source=args.source, dest=args.dest, notices=args.notices)
    if args.record_dir:
        args.record_dir.mkdir(parents=True, exist_ok=True)
        (args.record_dir / "overlay.diff").write_text(staged.overlay_diff, encoding="utf-8")
        (args.record_dir / "dist-config.toml").write_text(render_dist_config(selection).overlay, encoding="utf-8")
    return 0, staged.as_json()


def _command_provenance(args: argparse.Namespace, env: Mapping[str, str]) -> tuple[int, Any]:
    selection = Selection.from_json(_read_json_file(args.selection), source=args.selection.as_posix())
    record = build_provenance(selection, _read_json_file(args.plan), args.assets_dir,
                              [_read_json_file(path) for path in args.build_env])
    if args.write:
        (args.assets_dir / PROVENANCE_NAME).write_bytes(render_json(record))
    return 0, record


def _command_validate_plan(args: argparse.Namespace, env: Mapping[str, str]) -> tuple[int, Any]:
    selection = Selection.from_json(_read_json_file(args.selection), source=args.selection.as_posix())
    validate_plan(selection, _read_json_file(args.plan))
    return 0, {"status": "valid", "tag": selection.tag}


def _command_reconcile(args: argparse.Namespace, env: Mapping[str, str]) -> tuple[int, Any]:
    expected = _regular_files(args.expected_dir)
    existing = remote_assets(_read_json_file(args.existing), source=args.existing.as_posix())
    if args.existing_dir:
        def read_remote(asset: RemoteAsset) -> Iterator[bytes]:
            yield _read_file(args.existing_dir / asset.name)
    else:
        token = env.get("GITHUB_TOKEN")
        read_remote = lambda asset: download(asset, token=token)  # noqa: E731
    plan = reconcile_assets(expected, existing, read_remote)
    return (1 if plan.conflicts else 0), plan.as_json()


def _command_tap_update(args: argparse.Namespace, env: Mapping[str, str]) -> tuple[int, Any]:
    candidate = FormulaVersion(args.candidate_version, _read_file(args.candidate_formula))
    installed = None
    if args.installed_version is not None or args.installed_formula is not None:
        if args.installed_version is None or args.installed_formula is None:
            raise DistributionError("tap-update", "--installed-version and --installed-formula go together")
        installed = FormulaVersion(args.installed_version, _read_file(args.installed_formula))
    decision = select_tap_update(candidate, installed)
    return 0, {"action": decision.action, "reason": decision.reason}


def _run_id(text: str, where: str) -> int:
    if not re.fullmatch(r"[1-9][0-9]*", text):
        raise DistributionError(where, f"must be a positive workflow run id, found {text!r}")
    return int(text)


def _command_verify_select(args: argparse.Namespace, env: Mapping[str, str]) -> tuple[int, Any]:
    workspace = _workspace(args.root, args.metadata)
    channels: dict[str, str] = {}
    if args.brew is not None:
        channels["brew"] = args.brew
    if args.binstall:
        channels["binstall"] = args.package
    checkout = args.checkout_commit or _checkout_commit(args.root)
    return 0, select_verification(args.package, workspace, checkout, channels).as_json()


def _command_rollout_candidate(args: argparse.Namespace, env: Mapping[str, str]) -> tuple[int, Any]:
    selection = Selection.from_json(_read_json_file(args.selection), source=args.selection.as_posix())
    if args.formula_current not in FORMULA_CURRENT:
        raise DistributionError("--formula-current", f"must be one of {sorted(FORMULA_CURRENT)}, found {args.formula_current!r}")
    entry = rollout_candidate(selection, run_id=_run_id(args.run_id, "--run-id"), install=args.install,
                              binstall=args.binstall, tap=args.tap, brew_install=args.brew_install,
                              formula_current=FORMULA_CURRENT[args.formula_current])
    return 0, entry


def _command_build_matrix(args: argparse.Namespace, env: Mapping[str, str]) -> tuple[int, Any]:
    selection = Selection.from_json(_read_json_file(args.selection), source=args.selection.as_posix())
    return 0, {"build_matrix": build_matrix(selection, _read_json_file(args.images), source=args.images.as_posix())}


def _command_bundle(args: argparse.Namespace, env: Mapping[str, str]) -> tuple[int, Any]:
    selection = Selection.from_json(_read_json_file(args.selection), source=args.selection.as_posix())
    run_id = _run_id(args.run_id, "--run-id")
    record = write_bundle(selection, _read_json_file(args.plan), args.assets_dir,
                          [_read_json_file(path) for path in args.build_env], run_id, args.output_dir)
    return 0, {"status": "bundled", "output_dir": args.output_dir.as_posix(), **record}


def _command_check_bundle(args: argparse.Namespace, env: Mapping[str, str]) -> tuple[int, Any]:
    selection = Selection.from_json(_read_json_file(args.selection), source=args.selection.as_posix())
    current = None if args.current_run_id is None else _run_id(args.current_run_id, "--current-run-id")
    run = None if args.run is None else _read_json_file(args.run)
    jobs = None if args.jobs is None else _read_json_file(args.jobs)
    found = check_bundle(args.bundle, selection, current_run_id=current, run=run, jobs=jobs,
                         run_source=args.run.as_posix() if args.run else "run", jobs_source=args.jobs.as_posix() if args.jobs else "jobs")
    return 0, {"status": "valid", "tag": selection.tag, "run_id": found.run_id, "reused": found.run_id != current,
               "assets": sorted(found.assets)}


def _command_publish(args: argparse.Namespace, env: Mapping[str, str]) -> tuple[int, Any]:
    # Refusals that need no file, workspace, or network come first. There is
    # no forge fixture: a run that gets past them talks to GitHub.
    selection = Selection.from_json(_read_json_file(args.selection), source=args.selection.as_posix())
    if selection.mode not in ("release", "recovery") or selection.publishable is not True:
        raise DistributionError(f"{args.selection.as_posix()}: mode", f"is {selection.mode!r}; verification builds are never published")
    repository = env.get("GITHUB_REPOSITORY", "").strip()
    if repository != CANONICAL_REPOSITORY:
        raise DistributionError("GITHUB_REPOSITORY", f"is {repository!r}; only {CANONICAL_REPOSITORY} publishes release assets")
    token = env.get("GITHUB_TOKEN", "").strip()
    if not token:
        raise DistributionError("GITHUB_TOKEN", "is required to upload release assets")
    bundle = _check_bundle_contents(args.bundle, selection)
    workspace = _workspace(args.current_root, args.current_metadata)
    policy_path = args.current_root / "docs/deployment.yaml"
    current = load_policy(policy_path, workspace, source=f"current {policy_path.as_posix()}")
    forge = GitHubPublishForge(repository=repository, token=token, bot_login=env.get("RELEASE_APP_BOT_LOGIN") or None, repo=args.current_root)
    return 0, publish_release_assets(selection, bundle.assets, forge, current, fresh_rebuild=args.fresh_rebuild).as_json()


def _parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)

    preflight = commands.add_parser("preflight", help="select the approved package a release event distributes")
    preflight.add_argument("--event-name", required=True)
    preflight.add_argument("--event-path", type=Path, required=True)
    preflight.add_argument("--tagged-root", type=Path, required=True, help="checkout of the tag")
    preflight.add_argument("--current-root", type=Path, required=True, help="checkout of the default branch")
    preflight.add_argument("--tagged-metadata", type=Path, help="cargo metadata JSON instead of running cargo")
    preflight.add_argument("--current-metadata", type=Path)
    preflight.add_argument("--tagged-policy", type=Path)
    preflight.add_argument("--current-policy", type=Path)
    preflight.add_argument("--forge-fixture", type=Path, help="forge state from JSON instead of GitHub")
    preflight.add_argument("--checkout-commit")
    preflight.add_argument("--snapshot-dir", type=Path, help="write byte-identical policy snapshots here")
    preflight.add_argument("--rollout", type=Path, help=f"the rollout ledger (default: --current-root's {ROLLOUT_LEDGER})")

    config = commands.add_parser("config", help="render the dist opt-in for a selected release")
    config.add_argument("--selection", type=Path, required=True)

    stage = commands.add_parser("stage", help="copy the tagged checkout and apply the generated opt-in")
    stage.add_argument("--selection", type=Path, required=True)
    stage.add_argument("--source", type=Path, required=True, help="the clean tagged checkout")
    stage.add_argument("--dest", type=Path, required=True, help="an absent or empty staging directory")
    stage.add_argument("--notices", type=Path, required=True, help="third-party notices from scripts/distribution/licenses.sh")
    stage.add_argument("--record-dir", type=Path, help="write overlay.diff and dist-config.toml here")

    provenance = commands.add_parser("provenance", help="record what was built, from dist's outputs")
    provenance.add_argument("--selection", type=Path, required=True)
    provenance.add_argument("--plan", type=Path, required=True)
    provenance.add_argument("--assets-dir", type=Path, required=True, help="every collected release asset")
    provenance.add_argument("--build-env", type=Path, action="append", required=True, help="one per target, from build.sh")
    provenance.add_argument("--write", action="store_true", help=f"also write {PROVENANCE_NAME} into --assets-dir")

    plan = commands.add_parser("validate-plan", help="check a dist JSON plan against a selected release")
    plan.add_argument("--selection", type=Path, required=True)
    plan.add_argument("--plan", type=Path, required=True)

    reconcile = commands.add_parser("reconcile-assets", help="compare built assets with a Release's assets")
    reconcile.add_argument("--expected-dir", type=Path, required=True)
    reconcile.add_argument("--existing", type=Path, required=True, help="the Release's assets JSON from the API")
    reconcile.add_argument("--existing-dir", type=Path, help="local copies of the existing assets instead of downloading")

    tap = commands.add_parser("tap-update", help="decide whether the Homebrew formula changes")
    tap.add_argument("--candidate-version", required=True)
    tap.add_argument("--candidate-formula", type=Path, required=True)
    tap.add_argument("--installed-version")
    tap.add_argument("--installed-formula", type=Path)

    verify = commands.add_parser("verify-select", help="select the current checkout for a nonpublishing verification build")
    verify.add_argument("--package", required=True)
    verify.add_argument("--root", type=Path, required=True, help="the checkout to verify")
    verify.add_argument("--metadata", type=Path, help="cargo metadata JSON instead of running cargo")
    verify.add_argument("--checkout-commit", help="instead of the checkout's HEAD")
    verify.add_argument("--brew", help="fixture approval of the brew channel under this formula name")
    verify.add_argument("--binstall", action="store_true", help="fixture approval of the binstall channel")

    runners = commands.add_parser("build-matrix", help="the runner for each selected target")
    runners.add_argument("--selection", type=Path, required=True)
    runners.add_argument("--images", type=Path, required=True, help=".github/distribution/build-images.json")

    bundle = commands.add_parser("bundle", help="keep one verified build as a reusable bundle")
    bundle.add_argument("--selection", type=Path, required=True)
    bundle.add_argument("--plan", type=Path, required=True)
    bundle.add_argument("--assets-dir", type=Path, required=True, help=f"the release assets and {PROVENANCE_NAME}")
    bundle.add_argument("--build-env", type=Path, action="append", required=True, help="one per target, from build.sh")
    bundle.add_argument("--run-id", required=True, help="the workflow run producing the bundle")
    bundle.add_argument("--output-dir", type=Path, required=True, help="an absent or empty directory")

    check = commands.add_parser("check-bundle", help="validate a bundle against a fresh preflight before reuse or upload")
    check.add_argument("--bundle", type=Path, required=True)
    check.add_argument("--selection", type=Path, required=True, help="the fresh preflight record")
    check.add_argument("--run", type=Path, help="GET /repos/{repo}/actions/runs/{bundle run id}")
    check.add_argument("--jobs", type=Path, help="GET /repos/{repo}/actions/runs/{bundle run id}/jobs")
    check.add_argument("--current-run-id", help="this workflow run's id")

    publish = commands.add_parser("publish", help="attach a bundle's missing assets to its published Release")
    publish.add_argument("--selection", type=Path, required=True)
    publish.add_argument("--bundle", type=Path, required=True, help="a bundle check-bundle accepted")
    publish.add_argument("--current-root", type=Path, required=True, help="checkout of the default branch")
    publish.add_argument("--current-metadata", type=Path)
    publish.add_argument("--fresh-rebuild", action="store_true", help="the bundle was rebuilt, not reused; refuse a partial Release")

    candidate = commands.add_parser("rollout-candidate", help="the rollout ledger entry a successful publishing run proposes")
    candidate.add_argument("--selection", type=Path, required=True)
    candidate.add_argument("--run-id", required=True, help="this workflow run's id")
    candidate.add_argument("--install", required=True, help="needs.install.result")
    candidate.add_argument("--binstall", required=True, help="needs.binstall.result")
    candidate.add_argument("--tap", required=True, help="needs.homebrew-tap.result")
    candidate.add_argument("--brew-install", required=True, help="needs.brew-install.result")
    candidate.add_argument("--formula-current", required=True,
                           help="needs.homebrew-tap.outputs.formula_current: 'true', 'false', or '' when the tap job did not run")

    for command in (preflight, config, stage, plan, provenance, reconcile, tap, verify, runners, bundle, check, publish, candidate):
        command.add_argument("--output", type=Path, help="also write the JSON result here")
    return parser.parse_args(argv)


_COMMANDS = {
    "preflight": _command_preflight,
    "config": _command_config,
    "stage": _command_stage,
    "validate-plan": _command_validate_plan,
    "provenance": _command_provenance,
    "reconcile-assets": _command_reconcile,
    "tap-update": _command_tap_update,
    "verify-select": _command_verify_select,
    "build-matrix": _command_build_matrix,
    "bundle": _command_bundle,
    "check-bundle": _command_check_bundle,
    "publish": _command_publish,
    "rollout-candidate": _command_rollout_candidate,
}


def main(argv: Sequence[str] | None = None, *, env: Mapping[str, str] | None = None) -> int:
    args = _parse_args(sys.argv[1:] if argv is None else argv)
    try:
        code, result = _COMMANDS[args.command](args, os.environ if env is None else env)
    except AssetConflict as error:
        print(f"distribution: {error}", file=sys.stderr)
        result = {"status": "refused", "reason": str(error), "conflicts": error.conflicts}
        code = 1
    except PendingSetup as error:
        print(f"distribution: {error}", file=sys.stderr)
        result = {"status": PENDING_SETUP, "reason": str(error)}
        code = 3
    except DistributionError as error:
        print(f"distribution: {error}", file=sys.stderr)
        result = {"status": INVALID, "reason": str(error)}
        code = 2
    text = json.dumps(result, indent=2, sort_keys=True) + "\n"
    sys.stdout.write(text)
    if args.output:
        args.output.write_text(text, encoding="utf-8")
    return code


if __name__ == "__main__":
    sys.exit(main())

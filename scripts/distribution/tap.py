#!/usr/bin/env python3
"""Publish one release's Homebrew formula to `yankeeinlondon/homebrew-tap`.

The formula is a projection of dist's own `<formula>.rb` (plan rule 16):

- `prepare` strictly validates dist's formula against the selected release
  and dist's `sha256.sum`, then applies anchored transformations (macOS 15
  floor, Linux glibc floor, claudine's ALSA runtime). An anchor that does not
  occur exactly once is a "formula template drift" error; nothing else in
  dist's output changes. No network.
- `publish` additionally proves every referenced archive is downloadable and
  byte-identical to the local asset, compares versions with the tap by SemVer
  (`distribution.select_tap_update`), and writes only `Formula/<name>.rb`
  with one GraphQL `createCommitOnBranch` guarded by `expectedHeadOid`.
  GitHub signs that commit; the signature is then checked twice: GitHub's
  returned `isValid`, and `git verify-commit` against the pinned web-flow key
  in an isolated GPG home. A head conflict or signature failure fails once;
  rerun the same tag. Never force, never retry internally, never fall back to
  `GITHUB_TOKEN`.
- `audit` runs `brew audit --strict` and `brew style` on a prepared formula
  in a throwaway local tap.

Output is a JSON report on stdout (and `--report`). Exit status: 0 success
or a no-op, 2 invalid input or a failed check, 3 missing credentials or
setup.
"""

from __future__ import annotations

import argparse
import base64
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.error
import urllib.request
from abc import ABC, abstractmethod
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Iterator, Mapping, Sequence

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "ci"))
import distribution  # noqa: E402
from distribution import (  # noqa: E402
    CANONICAL_URL,
    HOMEBREW_TAP,
    TARGETS,
    Document,
    DistributionError,
    Fields,
    FormulaVersion,
    PendingSetup,
    RemoteAsset,
    Selection,
    parse_version,
    read_checksums,
    select_tap_update,
)

TOKEN_ENV = "HOMEBREW_TAP_TOKEN"
AUTHOR_ENV = "HOMEBREW_TAP_AUTHOR"
BRANCH = "main"
MIN_GLIBC = distribution.MIN_GLIBC
LICENSE = "AGPL-3.0-only"
AUDIT_TAP = "local/distribution-audit"
TIMEOUT = 60

# `https://github.com/web-flow.gpg`, downloaded 2026-11-09 into
# `github-web-flow.gpg` and inspected with `gpg --show-keys`. The file holds
# GitHub's expired 2017 key and the current signing key; the import must
# yield exactly this set, and a commit must carry a valid signature by the
# current key. Refresh both together if GitHub rotates the key.
WEB_FLOW_KEY_FILE = Path(__file__).resolve().parent / "github-web-flow.gpg"
WEB_FLOW_SIGNING_KEY = "968479A1AFF927E37D1A566BB5690EEEBB952194"
WEB_FLOW_FINGERPRINTS = frozenset({"5DE3E0509C47EA3CF04A42D34AEE18F83AFDEB23", WEB_FLOW_SIGNING_KEY})

# Packages whose Linux binary links a native runtime library (plan rule 16).
LINUX_DEPENDENCIES: Mapping[str, tuple[str, ...]] = {"claudine-cli": ("alsa-lib",)}

_URL_LINE = re.compile(r'^[ \t]*url "([^"]*)"[ \t]*$', re.MULTILINE)
_SHA_LINE = re.compile(r'^\s*sha256 "([0-9a-f]{64})"$')
_VERSION_LINE = re.compile(r'^[ \t]*version\b.*$', re.MULTILINE)
_VERSION_VALUE = re.compile(r'^\s*version "([^"]+)"\s*$')
_LICENSE_LINE = re.compile(r'^[ \t]*license\b.*$', re.MULTILINE)
_CLASS_LINE = re.compile(r'^class (\S+) < Formula$', re.MULTILINE)
_BIN_INSTALL = re.compile(r'\bbin\.install\b(?!_)')
_CONFLICT = ("expectedHeadOid", "Expected branch to point to")


def _fields(value: Any, source: str) -> Fields:
    return Fields(Document(source, value))


def _nullable_mapping(fields: Fields, path: list[Any]) -> dict[str, Any] | None:
    """A requested GraphQL object field: GitHub writes null when it is unset,
    so null is a value and absence is a malformed response."""
    parent = fields.mapping(path[:-1])
    if path[-1] not in parent:
        raise fields.error(path, "is required (GitHub returns null when unset)")
    return None if parent[path[-1]] is None else fields.mapping(path)


class TapFailure(DistributionError):
    """A failure after the tap commit exists; the report names the commit."""

    def __init__(self, where: str, message: str, commit: str) -> None:
        super().__init__(where, message)
        self.commit = commit


# ---------------------------------------------------------------------------
# Formula validation and transformation
# ---------------------------------------------------------------------------


def class_name(formula: str) -> str:
    """dist's formula class: split on `-`/`_`, capitalize each part."""
    return "".join(part[:1].upper() + part[1:] for part in re.split(r"[-_]", formula) if part)


def _line(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def _target_guards(target: str) -> tuple[str, str]:
    os_guard = "OS.mac?" if target.endswith("-apple-darwin") else "OS.linux?"
    cpu_guard = "Hardware::CPU.arm?" if target.startswith("aarch64-") else "Hardware::CPU.intel?"
    return os_guard, cpu_guard


def release_url(selection: Selection, target: str) -> str:
    return f"{CANONICAL_URL}/releases/download/{selection.tag}/{distribution.archive_name(selection.package, target)}"


def _platform_block(selection: Selection, sums: Mapping[str, str]) -> str:
    """dist 0.34.0's url/sha256 block for the four targets, in its order."""
    lines: list[str] = []
    for os_guard in ("OS.mac?", "OS.linux?"):
        lines.append(f"  if {os_guard}")
        for target in TARGETS:
            if _target_guards(target)[0] != os_guard:
                continue
            lines += [
                f"    if {_target_guards(target)[1]}",
                f'      url "{release_url(selection, target)}"',
                f'      sha256 "{sums[distribution.archive_name(selection.package, target)]}"',
                "    end",
            ]
        lines.append("  end")
    return "\n".join(lines) + "\n"


_EMPTY_ALIASES = "  BINARY_ALIASES = {\n" + ",\n".join(f'    "{t}": {{}}' for t in sorted(TARGETS)) + "\n  }\n"


def validate_formula(text: str, selection: Selection, sums: Mapping[str, str], *, source: str) -> None:
    """Refuse dist output that does not describe exactly this release."""
    formula = selection.channels["brew"]
    classes = list(_CLASS_LINE.finditer(text))
    if len(classes) != 1:
        raise DistributionError(source, f"must declare exactly one Formula class, found {len(classes)}")
    if classes[0].group(1) != class_name(formula):
        raise DistributionError(f"{source}:{_line(text, classes[0].start())}",
                                f"class is {classes[0].group(1)!r}, expected {class_name(formula)!r} for formula {formula!r}")

    versions = list(_VERSION_LINE.finditer(text))
    if len(versions) != 1:
        raise DistributionError(source, f"must declare `version` exactly once, found {len(versions)}")
    value = _VERSION_VALUE.match(versions[0].group(0))
    if not value or value.group(1) != selection.version:
        raise DistributionError(f"{source}:{_line(text, versions[0].start())}",
                                f"is {versions[0].group(0).strip()!r}, expected version \"{selection.version}\"")

    expected = {release_url(selection, target): target for target in TARGETS}
    seen: set[str] = set()
    lines = text.split("\n")
    for match in _URL_LINE.finditer(text):
        number = _line(text, match.start())
        url = match.group(1)
        where = f"{source}:{number}"
        if url not in expected:
            raise DistributionError(where, f"url {url!r} is not one of this release's archives {sorted(expected)}")
        if url in seen:
            raise DistributionError(where, f"url {url!r} appears twice")
        seen.add(url)
        archive = distribution.archive_name(selection.package, expected[url])
        sha = _SHA_LINE.match(lines[number]) if number < len(lines) else None
        if not sha:
            raise DistributionError(f"{source}:{number + 1}", f"the url for {expected[url]} is not followed by a sha256 line")
        if archive not in sums:
            raise DistributionError("sha256.sum", f"has no entry for {archive}")
        if sha.group(1) != sums[archive]:
            raise DistributionError(f"{source}:{number + 1}", f"sha256 for {archive} is {sha.group(1)}, but sha256.sum has {sums[archive]}")
    missing = sorted(expected[url] for url in set(expected) - seen)
    if missing:
        raise DistributionError(source, f"has no url for {missing}")
    if text.count(_platform_block(selection, sums)) != 1:
        raise DistributionError(source, "the url/sha256 entries are not under dist's OS.mac?/OS.linux? and Hardware::CPU.arm?/intel? blocks")

    licenses = list(_LICENSE_LINE.finditer(text))
    if len(licenses) != 1 or licenses[0].group(0) != f'  license "{LICENSE}"':
        found = [m.group(0).strip() for m in licenses]
        raise DistributionError(source, f"must declare license \"{LICENSE}\" exactly once, found {found}")

    installs = list(_BIN_INSTALL.finditer(text))
    if not installs:
        raise DistributionError(source, f"never runs bin.install \"{selection.command}\"")
    for match in installs:
        line_start = text.rfind("\n", 0, match.start()) + 1
        line_end = text.find("\n", match.start())
        line = text[line_start:line_end].strip()
        if line != f'bin.install "{selection.command}"':
            raise DistributionError(f"{source}:{_line(text, match.start())}",
                                    f"{line!r} installs something other than the one command {selection.command!r}")
    if text.count(_EMPTY_ALIASES) != 1:
        raise DistributionError(source, "BINARY_ALIASES must be empty for every target (no extra installed names)")


def _insert_after(text: str, anchor: str, name: str, insertion: str, source: str) -> str:
    count = text.count(anchor)
    if count != 1:
        raise DistributionError(source, f"formula template drift: anchor {name!r} ({anchor.strip()!r}) occurs {count} times, expected exactly once")
    index = text.index(anchor) + len(anchor)
    return text[:index] + insertion + text[index:]


def prepare_formula(dist_formula_text: str, selection: Selection, checksums: Mapping[str, str], *, source: str = "formula") -> str:
    """dist's formula, validated, with the platform floors and native runtime
    dependencies declared. Every other byte is dist's."""
    if "brew" not in selection.channels:
        raise DistributionError("selection: channels", f"{selection.package} has no brew channel")
    validate_formula(dist_formula_text, selection, checksums, source=source)
    # `brew style` orders named dependencies before `macos:`.
    declarations = "".join(f'  depends_on "{dependency}" if OS.linux?\n' for dependency in LINUX_DEPENDENCIES.get(selection.package, ()))
    declarations += "  depends_on macos: :sequoia\n"
    text = _insert_after(dist_formula_text, f'\n  license "{LICENSE}"\n', "license", declarations, source)
    glibc = (
        f'    if OS.linux? && OS::Linux::Glibc.system_version < Version.new("{MIN_GLIBC}")\n'
        f'      odie "{selection.command} requires glibc {MIN_GLIBC} or newer; found #{{OS::Linux::Glibc.system_version}}"\n'
        "    end\n"
    )
    return _insert_after(text, "\n  def install\n", "def install", glibc, source)


def installed_version(formula: bytes, *, source: str) -> str:
    """The single `version "..."` of the tap's current formula."""
    try:
        text = formula.decode("utf-8")
    except UnicodeDecodeError:
        raise DistributionError(source, "is not UTF-8") from None
    versions = list(_VERSION_LINE.finditer(text))
    value = _VERSION_VALUE.match(versions[0].group(0)) if len(versions) == 1 else None
    if not value:
        raise DistributionError(source, f"must declare exactly one `version \"<semver>\"` line, found {[m.group(0).strip() for m in versions]}")
    parse_version(value.group(1), f"{source}:{_line(text, versions[0].start())}")
    return value.group(1)


# ---------------------------------------------------------------------------
# Tap access
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class CommitResult:
    oid: str
    signature_valid: bool
    signer_login: str | None
    author_login: str | None


class TapConflict(DistributionError):
    pass


class TapClient(ABC):
    @abstractmethod
    def viewer_login(self) -> str: ...

    @abstractmethod
    def branch_head(self, branch: str) -> str: ...

    @abstractmethod
    def read_file(self, oid: str, path: str) -> bytes | None:
        """The file at commit `oid`, or None when it does not exist."""

    @abstractmethod
    def create_commit(self, branch: str, expected_head_oid: str, path: str, contents: bytes, headline: str) -> CommitResult:
        """Raises `TapConflict` when the branch no longer points at `expected_head_oid`."""

    @abstractmethod
    def fetch_commit_for_verification(self, oid: str, dest_dir: Path) -> None: ...


class GitHubTapClient(TapClient):
    """GraphQL writes and REST reads against `HOMEBREW_TAP`, authenticated
    only with `HOMEBREW_TAP_TOKEN`. One request per call: no retry loop."""

    API = "https://api.github.com"

    def __init__(self, token: str, *, repository: str = HOMEBREW_TAP) -> None:
        self.token = token
        self.repository = repository

    def _request(self, url: str, *, body: Any = None, accept: str = "application/vnd.github+json") -> tuple[int, bytes]:
        data = None if body is None else json.dumps(body).encode("utf-8")
        request = urllib.request.Request(url, data=data, method="POST" if data is not None else "GET")
        request.add_header("Authorization", f"Bearer {self.token}")
        request.add_header("Accept", accept)
        request.add_header("X-GitHub-Api-Version", "2022-11-28")
        if data is not None:
            request.add_header("Content-Type", "application/json")
        try:
            with urllib.request.urlopen(request, timeout=TIMEOUT) as response:
                return response.status, response.read()
        except urllib.error.HTTPError as error:
            return error.code, error.read()
        except (urllib.error.URLError, TimeoutError, OSError) as error:
            raise DistributionError(url, f"request failed: {error}") from None

    def _graphql(self, query: str, variables: Mapping[str, Any]) -> Fields:
        """The response's `data`, as located fields; GraphQL `errors` and any
        non-200 status are failures."""
        status, raw = self._request(f"{self.API}/graphql", body={"query": query, "variables": variables})
        source = f"GitHub GraphQL (HTTP {status})"
        fields = _fields(distribution.load_json(raw, source=source), source)
        payload = fields.mapping([])
        if "errors" in payload:
            count = len(fields.records(["errors"], allow_empty=False))
            messages = "; ".join(fields.string(["errors", index, "message"]) for index in range(count))
            if any(marker in messages for marker in _CONFLICT):
                raise TapConflict(
                    f"{self.repository}@{BRANCH}",
                    f"the branch moved while publishing ({messages}); rerun the same tag to compare against the new head",
                )
            raise DistributionError(source, messages)
        if status != 200:
            raise DistributionError(source, "the request failed without a GraphQL error")
        fields.mapping(["data"])
        return fields

    def viewer_login(self) -> str:
        return self._graphql("query { viewer { login } }", {}).string(["data", "viewer", "login"])

    def branch_head(self, branch: str) -> str:
        where = f"{self.repository}@{branch}"
        status, raw = self._request(f"{self.API}/repos/{self.repository}/git/ref/heads/{branch}")
        if status != 200:
            raise DistributionError(where, f"cannot read the branch head: HTTP {status}")
        return _fields(distribution.load_json(raw, source=where), where).sha(["object", "sha"])

    def read_file(self, oid: str, path: str) -> bytes | None:
        status, raw = self._request(f"{self.API}/repos/{self.repository}/contents/{path}?ref={oid}",
                                    accept="application/vnd.github.raw+json")
        if status == 404:
            return None
        if status != 200:
            raise DistributionError(f"{self.repository}:{path}@{oid}", f"cannot read: HTTP {status}")
        return raw

    def create_commit(self, branch: str, expected_head_oid: str, path: str, contents: bytes, headline: str) -> CommitResult:
        mutation = """
mutation($input: CreateCommitOnBranchInput!) {
  createCommitOnBranch(input: $input) {
    commit { oid signature { isValid signer { login } } author { user { login } } }
  }
}"""
        variables = {"input": {
            "branch": {"repositoryNameWithOwner": self.repository, "branchName": branch},
            "message": {"headline": headline},
            "expectedHeadOid": expected_head_oid,
            "fileChanges": {"additions": [{"path": path, "contents": base64.b64encode(contents).decode("ascii")}]},
        }}
        fields = self._graphql(mutation, variables)
        commit = ["data", "createCommitOnBranch", "commit"]
        oid = fields.sha([*commit, "oid"])
        # A null signature is an unsigned commit, which `publish` refuses.
        valid, signer = False, None
        if _nullable_mapping(fields, [*commit, "signature"]) is not None:
            valid = fields.boolean([*commit, "signature", "isValid"])
            if _nullable_mapping(fields, [*commit, "signature", "signer"]) is not None:
                signer = fields.string([*commit, "signature", "signer", "login"])
        author = None
        if (_nullable_mapping(fields, [*commit, "author"]) is not None
                and _nullable_mapping(fields, [*commit, "author", "user"]) is not None):
            author = fields.string([*commit, "author", "user", "login"])
        return CommitResult(oid, valid, signer, author)

    def fetch_commit_for_verification(self, oid: str, dest_dir: Path) -> None:
        # The tap is public: fetch anonymously, never through a credential helper.
        env = {**os.environ, "GIT_TERMINAL_PROMPT": "0", "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": os.devnull}
        for args in (["git", "init", "--quiet", str(dest_dir)],
                     ["git", "-C", str(dest_dir), "-c", "credential.helper=", "fetch", "--quiet", "--depth=1", "--no-tags",
                      f"https://github.com/{self.repository}.git", oid]):
            result = subprocess.run(args, env=env, capture_output=True, text=True, timeout=120)
            if result.returncode != 0:
                raise DistributionError(f"commit {oid}", f"`{' '.join(args[:4])}` failed: {result.stderr.strip()}")


Runner = Callable[..., subprocess.CompletedProcess[str]]


def verify_signature(repo_dir: Path, oid: str, *, key_file: Path = WEB_FLOW_KEY_FILE, run: Runner = subprocess.run) -> str:
    """`git verify-commit` with a fresh GPG home holding only GitHub's pinned
    web-flow key, and git configuration isolated from the host's. Returns the
    signing key's primary fingerprint."""
    if not key_file.is_file():
        raise DistributionError(key_file.as_posix(), "the pinned web-flow key is missing")
    with tempfile.TemporaryDirectory(prefix="tap-gnupg-") as home:
        os.chmod(home, 0o700)
        env = {"GNUPGHOME": home, "HOME": home, "PATH": os.environ.get("PATH", ""), "LC_ALL": "C",
               "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": os.devnull}

        def call(args: list[str]) -> subprocess.CompletedProcess[str]:
            return run(args, env=env, capture_output=True, text=True, timeout=TIMEOUT)

        imported = call(["gpg", "--batch", "--import", str(key_file)])
        if imported.returncode != 0:
            raise DistributionError(key_file.as_posix(), f"gpg --import failed: {imported.stderr.strip()}")
        listing = call(["gpg", "--batch", "--with-colons", "--list-keys"])
        primaries, previous = set(), ""
        for line in listing.stdout.splitlines():
            fields = line.split(":")
            if fields[0] == "fpr" and previous == "pub":
                primaries.add(fields[9])
            previous = fields[0]
        if primaries != WEB_FLOW_FINGERPRINTS:
            raise DistributionError(key_file.as_posix(), f"imported keys {sorted(primaries)} are not the pinned web-flow keys {sorted(WEB_FLOW_FINGERPRINTS)}")
        result = call(["git", "-C", str(repo_dir), "-c", "gpg.format=openpgp", "-c", "gpg.program=gpg",
                       "verify-commit", "--raw", oid])
        status = result.stderr.splitlines()
        valid = [line.split() for line in status if line.startswith("[GNUPG:] VALIDSIG ")]
        if result.returncode != 0 or not any(line.startswith("[GNUPG:] GOODSIG ") for line in status) or not valid:
            raise DistributionError(f"commit {oid}", f"git verify-commit failed against GitHub's web-flow key: {result.stderr.strip()}")
        primary = valid[0][-1]
        if primary != WEB_FLOW_SIGNING_KEY:
            raise DistributionError(f"commit {oid}", f"is signed by {primary}, not GitHub's web-flow key {WEB_FLOW_SIGNING_KEY}")
        return primary


# ---------------------------------------------------------------------------
# Publication
# ---------------------------------------------------------------------------


def formula_path(selection: Selection) -> str:
    return f"Formula/{selection.channels['brew']}.rb"


def _report(status: str, selection: Selection, reason: str, commit: str | None = None) -> dict[str, Any]:
    return {"status": status, "formula": formula_path(selection), "version": selection.version, "reason": reason, "commit": commit}


def load_prepared(selection: Selection, assets: Path) -> bytes:
    if "brew" not in selection.channels:
        raise DistributionError("selection: channels", f"{selection.package} has no brew channel")
    name = f"{selection.channels['brew']}.rb"
    path = assets / name
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise DistributionError(path.as_posix(), f"cannot read dist's formula: {error}") from None
    return prepare_formula(text, selection, read_checksums(assets), source=name).encode("utf-8")


def verify_downloads(formula: str, assets: Path, fetch: Callable[[RemoteAsset], Iterator[bytes]]) -> list[str]:
    """Every url the formula names is live and byte-identical to the local asset."""
    urls = _URL_LINE.findall(formula)
    for url in urls:
        name = url.rsplit("/", 1)[-1]
        local = assets / name
        if not local.is_file():
            raise DistributionError(local.as_posix(), f"the formula references {url}, but the asset is not in the build bundle")
        asset = RemoteAsset(name, local.stat().st_size, 0, url)
        if not distribution._same_bytes(local, fetch(asset), asset):
            raise DistributionError(url, f"downloaded bytes differ from {local.as_posix()}")
    return urls


def check_selection(selection: Selection) -> bool:
    """Refuse a selection that cannot reach the tap; returns its `prerelease`
    flag, which `Selection.from_json` does not validate."""
    record = selection.record
    if record.get("mode") not in ("release", "recovery") or record.get("publishable") is not True:
        raise DistributionError("selection", f"mode {record.get('mode')!r} with publishable={record.get('publishable')!r} never reaches the tap")
    if "brew" not in selection.channels:
        raise DistributionError("selection: channels", f"{selection.package} has no brew channel")
    return _fields(record, "selection").boolean(["prerelease"])


def publish(
    selection: Selection,
    assets: Path,
    *,
    env: Mapping[str, str],
    client_factory: Callable[[str], TapClient] = GitHubTapClient,
    fetch: Callable[[RemoteAsset], Iterator[bytes]] = distribution.download,
    verify: Callable[[Path, str], str] = verify_signature,
    dry_run: bool = False,
) -> dict[str, Any]:
    if check_selection(selection):
        return _report("skip", selection, f"{selection.version} is a prerelease; the stable formula is unchanged")
    if selection.record.get("formula_update") is not True:
        raise DistributionError("selection: formula_update", "is not true; this release does not update the formula")
    token, author = env.get(TOKEN_ENV), env.get(AUTHOR_ENV)
    if not token:
        raise PendingSetup(TOKEN_ENV, "add the repository secret with Contents write access to "
                           f"{HOMEBREW_TAP}; the publisher never falls back to GITHUB_TOKEN")
    if not author:
        raise PendingSetup(AUTHOR_ENV, "set the repository variable to the login that owns "
                           f"{TOKEN_ENV}; the publisher never falls back to GITHUB_TOKEN")
    client = client_factory(token)
    viewer = client.viewer_login()
    if viewer != author:
        raise DistributionError(TOKEN_ENV, f"belongs to {viewer!r}, but {AUTHOR_ENV} is {author!r}")

    prepared = load_prepared(selection, assets)
    verify_downloads(prepared.decode("utf-8"), assets, fetch)

    path = formula_path(selection)
    head = client.branch_head(BRANCH)
    current = client.read_file(head, path)
    installed = None if current is None else FormulaVersion(installed_version(current, source=f"{HOMEBREW_TAP}:{path}@{head}"), current)
    decision = select_tap_update(FormulaVersion(selection.version, prepared), installed)
    if decision.action in ("noop", "skip"):
        return _report(decision.action, selection, decision.reason)
    if dry_run:
        return _report("dry-run", selection, f"would {decision.action}: {decision.reason}")

    result = client.create_commit(BRANCH, head, path, prepared, f"{selection.channels['brew']} {selection.version}")
    where = f"{HOMEBREW_TAP}@{result.oid}"
    if not result.signature_valid:
        raise TapFailure(where, "GitHub did not report a valid signature for the commit; investigate before rerunning, never bypass", result.oid)
    if result.author_login != author:
        raise TapFailure(where, f"the commit's author is {result.author_login!r}, not {author!r}", result.oid)
    with tempfile.TemporaryDirectory(prefix="tap-verify-") as scratch:
        try:
            client.fetch_commit_for_verification(result.oid, Path(scratch))
            verify(Path(scratch), result.oid)
        except DistributionError as error:
            raise TapFailure(where, str(error), result.oid) from None
    status = "created" if decision.action == "create" else "updated"
    return _report(status, selection, decision.reason, result.oid)


# ---------------------------------------------------------------------------
# brew audit
# ---------------------------------------------------------------------------


def audit_commands(name: str, formula: Path, tap_repository: Path) -> list[list[str]]:
    """The commands the workflow runs to audit a prepared formula; the tap
    directory is `brew --repository local/distribution-audit`."""
    qualified = f"{AUDIT_TAP}/{name}"
    return [
        ["brew", "tap-new", "--no-git", AUDIT_TAP],
        ["cp", str(formula), str(tap_repository / "Formula" / f"{name}.rb")],
        # Current Homebrew refuses to load a formula from an untrusted tap.
        ["brew", "trust", AUDIT_TAP],
        ["brew", "audit", "--strict", "--formula", qualified],
        ["brew", "style", qualified],
    ]


# Findings dist 0.34.0's formula template always produces under
# `brew audit --strict` / `brew style` (measured 2026-11-09 with the real
# fixture). They are accepted so the audit can gate everything else; the
# transformations above add none, and any other finding, including a
# package's own `desc`, fails.
DIST_TEMPLATE_AUDIT = tuple(re.compile(p) for p in (
    r"^Stable: `version \S+` is redundant with version scanned from URL$",
    r"^A `test do` test block should be added$",
    r"^Freeze mutable objects assigned to constants\.$",
    r"^Align the keys and values of a hash literal if they span more than one line\.$",
    r"^Put a comma after the last item of a multiline hash\.$",
))
DIST_TEMPLATE_COPS = frozenset({"FormulaAudit/Test", "Style/MutableConstant", "Layout/HashAlignment",
                                "Style/TrailingCommaInHashLiteral"})
_AUDIT_FINDING = re.compile(r"^  \* (?:line \d+, col \d+: )?(.*)$")
_STYLE_FINDING = re.compile(r"^\S+:\d+:\d+: [A-Z]: (?:\[Correctable\] )?(\S+): (.*)$")


def unexpected_findings(kind: str, output: str) -> list[str] | None:
    """Findings in `brew audit` (`kind` "audit") or `brew style` output that
    are not dist template findings; None when the output lists no finding."""
    findings, unexpected = 0, []
    for line in output.splitlines():
        if kind == "audit" and (match := _AUDIT_FINDING.match(line)):
            findings += 1
            if not any(p.match(match.group(1)) for p in DIST_TEMPLATE_AUDIT):
                unexpected.append(match.group(1))
        elif kind == "style" and (match := _STYLE_FINDING.match(line)):
            findings += 1
            if match.group(1) not in DIST_TEMPLATE_COPS:
                unexpected.append(f"{match.group(1)}: {match.group(2)}")
    return unexpected if findings else None


def run_audit(name: str, formula: Path, *, run: Runner = subprocess.run) -> dict[str, Any]:
    if shutil.which("brew") is None:
        raise PendingSetup("brew", "Homebrew is required to audit the formula and is not on PATH")
    env = {**os.environ, "HOMEBREW_NO_AUTO_UPDATE": "1", "HOMEBREW_NO_ANALYTICS": "1", "NONINTERACTIVE": "1"}
    located = run(["brew", "--repository", AUDIT_TAP], env=env, capture_output=True, text=True, timeout=300)
    tap = Path(located.stdout.strip())
    # An empty answer would be `.`, putting the throwaway tap in the caller's directory.
    if located.returncode != 0 or not tap.is_absolute():
        raise DistributionError(f"brew --repository {AUDIT_TAP}",
                                f"must print the tap's absolute path; exited {located.returncode} with {located.stdout.strip()!r} {located.stderr.strip()}")
    ran, accepted = [], []
    for args in audit_commands(name, formula, tap):
        if args[1] == "tap-new" and (tap / "Formula").is_dir():
            continue  # a rerun on the same host reuses the throwaway tap
        result = run(args, env=env, capture_output=True, text=True, timeout=600)
        ran.append(" ".join(args))
        if result.returncode == 0:
            continue
        output = result.stdout + result.stderr
        unexpected = unexpected_findings(args[1], output) if args[1] in ("audit", "style") else None
        if unexpected is None or unexpected:
            detail = "\n".join(unexpected) if unexpected else "\n".join(output.rstrip().splitlines()[-60:])
            raise DistributionError(" ".join(args), "failed:\n" + detail)
        accepted.append(" ".join(args))
    return {"status": "audited", "formula": name, "commands": ran, "accepted_dist_template_findings": accepted}


# ---------------------------------------------------------------------------
# Command line
# ---------------------------------------------------------------------------


def _selection(path: Path) -> Selection:
    try:
        data = path.read_bytes()
    except OSError as error:
        raise DistributionError(path.as_posix(), f"cannot read: {error}") from None
    return Selection.from_json(distribution.load_json(data, source=path.as_posix()), source=path.as_posix())


def _parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)
    publish_cmd = commands.add_parser("publish", help="validate, verify downloads, and commit the formula")
    publish_cmd.add_argument("--selection", type=Path, required=True)
    publish_cmd.add_argument("--assets-dir", type=Path, required=True)
    publish_cmd.add_argument("--dry-run", action="store_true")
    publish_cmd.add_argument("--report", type=Path)
    prepare_cmd = commands.add_parser("prepare", help="validate and transform dist's formula (no network)")
    prepare_cmd.add_argument("--selection", type=Path, required=True)
    prepare_cmd.add_argument("--assets-dir", type=Path, required=True)
    prepare_cmd.add_argument("--output", type=Path, required=True)
    audit_cmd = commands.add_parser("audit", help="brew audit --strict and brew style a prepared formula")
    audit_cmd.add_argument("--formula", type=Path, required=True)
    audit_cmd.add_argument("--name", required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None, *, env: Mapping[str, str] | None = None,
         client_factory: Callable[[str], TapClient] = GitHubTapClient,
         fetch: Callable[[RemoteAsset], Iterator[bytes]] = distribution.download,
         verify: Callable[[Path, str], str] = verify_signature) -> int:
    args = _parse_args(sys.argv[1:] if argv is None else argv)
    env = os.environ if env is None else env
    selection: Selection | None = None
    code = 0
    try:
        if args.command == "audit":
            result = run_audit(args.name, args.formula)
        else:
            selection = _selection(args.selection)
            if args.command == "prepare":
                prepared = load_prepared(selection, args.assets_dir)
                args.output.write_bytes(prepared)
                result = {"status": "prepared", "formula": formula_path(selection), "output": args.output.as_posix()}
            else:
                result = publish(selection, args.assets_dir, env=env, client_factory=client_factory,
                                 fetch=fetch, verify=verify, dry_run=args.dry_run)
    except PendingSetup as error:
        print(f"tap: {error}", file=sys.stderr)
        code, result = 3, {"status": "pending-setup", "reason": str(error), "commit": None}
    except DistributionError as error:
        print(f"tap: {error}", file=sys.stderr)
        code, result = 2, {"status": "failed", "reason": str(error), "commit": getattr(error, "commit", None)}
    if selection is not None and "formula" not in result and "brew" in selection.channels:
        result.update(formula=formula_path(selection), version=selection.version)
    text = json.dumps(result, indent=2, sort_keys=True) + "\n"
    sys.stdout.write(text)
    if getattr(args, "report", None):
        args.report.write_text(text, encoding="utf-8")
    return code


if __name__ == "__main__":
    sys.exit(main())

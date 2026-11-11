#!/usr/bin/env python3
"""Checks, diagnostics, and publication for the release workflow (`release-plz.yml`).

- `select prepare|publish` decides whether the triggering event may prepare or
  publish a release, before any credential exists: preparation only from a
  successful `ci` push run on current `main`; publication only from a merged
  release proposal authored by the release App, merged with a merge commit.
- `inventory` reports each candidate package's remote tag and Release.
- `publish` releases each candidate from a fresh clone detached at the
  proposal's final commit, one release-plz process per package, recovering a
  tag whose Release is missing without ever moving a tag.
- `release-pr` opens or updates the proposal with the App's identity.
- `update` is the dry run's calculation: `release-plz update` in the same
  calculation environment `release-pr` uses (`calculation_env`).
- `calculation-report` and `diagnose` are the dry run's read-only evidence.
  `diagnose` captures the Git state a calculation runs in; every Git call runs
  with `GIT_OPTIONAL_LOCKS=0`, so not even the index stat cache is refreshed,
  and a stale stat cache is itself one of the states worth capturing.

Event data is read from `GITHUB_EVENT_PATH`, never interpolated into shell.
The operational guide is `docs/releases.md`.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
import urllib.error
import urllib.parse
import urllib.request
from contextlib import contextmanager
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterator, Sequence

# The file a historical calculation reported as locally modified, and the
# revision it was checking out at the time.
DEFAULT_PROBE_PATH = "scripts/init.ps1"
DEFAULT_HISTORICAL_REVISION = "9ba25e4636e5725c224017b870b07352684e3c30"

# Only configuration that can change how Git reads or writes file content is
# reported. Anything else may hold a credential (an `http.*.extraheader`, a
# credential helper, a token-bearing remote URL) and is never printed.
_REPORTED_CONFIG = re.compile(
    r"^(core\.(autocrlf|eol|safecrlf|filemode|symlinks|ignorecase|precomposeunicode"
    r"|checkstat|trustctime|fsmonitor|untrackedcache|attributesfile)"
    r"|filter\.[^=]*\.(clean|smudge|process|required)"
    r"|checkout\.[a-z]+|index\.[a-z]+|feature\.[a-z]+)$",
    re.IGNORECASE,
)


def _git_env() -> dict[str, str]:
    env = dict(os.environ)
    env["GIT_OPTIONAL_LOCKS"] = "0"
    env["GIT_TERMINAL_PROMPT"] = "0"
    env.setdefault("LC_ALL", "C")
    return env


def run_probe(repo: Path, argv: Sequence[str]) -> dict[str, Any]:
    """Run one read-only command and record its outcome without raising."""
    try:
        completed = subprocess.run(
            list(argv),
            cwd=repo,
            env=_git_env(),
            capture_output=True,
            check=False,
        )
    except OSError as error:
        return {"argv": list(argv), "exit": None, "error": str(error)}
    return {
        "argv": list(argv),
        "exit": completed.returncode,
        "stdout": completed.stdout.decode("utf-8", "replace"),
        "stderr": completed.stderr.decode("utf-8", "replace"),
    }


def _stdout(probe: dict[str, Any]) -> str | None:
    """A successful probe's output without its final newline, or `None` when it
    failed. Leading whitespace is kept: it is the index column of a porcelain
    status line."""
    if probe.get("exit") != 0:
        return None
    return probe["stdout"].rstrip("\n")


def byte_profile(data: bytes) -> dict[str, int]:
    """Line-ending counts: `crlf` pairs, plus `lf` and `cr` bytes outside a pair."""
    crlf = data.count(b"\r\n")
    return {
        "size": len(data),
        "crlf": crlf,
        "lf": data.count(b"\n") - crlf,
        "cr": data.count(b"\r") - crlf,
    }


def _probe_file(repo: Path, path: str) -> dict[str, Any]:
    git = ["git"]
    staged = run_probe(repo, [*git, "ls-files", "-s", "--", path])
    staged_line = _stdout(staged)
    index: dict[str, Any] = {"ls_files": staged}
    if staged_line:
        mode, blob, _stage_and_path = staged_line.split(" ", 2)
        index.update(mode=mode, blob=blob)
        content = subprocess.run(
            [*git, "cat-file", "blob", blob],
            cwd=repo,
            env=_git_env(),
            capture_output=True,
            check=False,
        )
        if content.returncode == 0:
            index["bytes"] = byte_profile(content.stdout)

    worktree: dict[str, Any] = {"present": (repo / path).is_file()}
    if worktree["present"]:
        worktree["bytes"] = byte_profile((repo / path).read_bytes())
        worktree["hash_with_filters"] = _stdout(
            run_probe(repo, [*git, "hash-object", f"--path={path}", "--", path])
        )
        worktree["hash_without_filters"] = _stdout(
            run_probe(repo, [*git, "hash-object", "--no-filters", "--", path])
        )
        # Equal to the index blob exactly when Git, re-reading the file with the
        # current attributes, sees no content change.
        worktree["matches_index_after_filters"] = (
            index.get("blob") is not None
            and worktree["hash_with_filters"] == index.get("blob")
        )

    return {
        "path": path,
        "eol": _stdout(run_probe(repo, [*git, "ls-files", "--eol", "--", path])),
        "attributes": _stdout(run_probe(repo, [*git, "check-attr", "-a", "--", path])),
        "diff_quiet_exit": run_probe(repo, [*git, "diff", "--quiet", "--", path]).get("exit"),
        "index": index,
        "worktree": worktree,
    }


def _historical(repo: Path, revision: str, path: str) -> dict[str, Any]:
    present = run_probe(repo, ["git", "cat-file", "-e", f"{revision}^{{commit}}"])
    result: dict[str, Any] = {"revision": revision, "available": present.get("exit") == 0}
    if result["available"]:
        for label, candidate in (("probe", path), ("gitattributes", ".gitattributes")):
            exists = run_probe(repo, ["git", "cat-file", "-e", f"{revision}:{candidate}"])
            result[f"{label}_present"] = exists.get("exit") == 0
    return result


def _reported_config(repo: Path) -> list[dict[str, str]]:
    listing = _stdout(run_probe(repo, ["git", "config", "--list", "--show-scope"]))
    entries: list[dict[str, str]] = []
    for line in (listing or "").splitlines():
        scope, _, setting = line.partition("\t")
        key, _, value = setting.partition("=")
        if _REPORTED_CONFIG.match(key):
            entries.append({"scope": scope, "key": key, "value": value})
    return entries


def collect_diagnostics(
    repo: Path,
    *,
    probe_path: str = DEFAULT_PROBE_PATH,
    historical_revision: str = DEFAULT_HISTORICAL_REVISION,
    tool: Sequence[str] = ("release-plz", "--version"),
) -> dict[str, Any]:
    """The read-only release diagnostic report for the checkout at `repo`.

    Raises `NotAGitCheckout` when `repo` is not inside a Git working tree; every
    other failed probe is recorded in the report rather than raised, because a
    diagnosis must survive the broken state it is describing.
    """
    inside = run_probe(repo, ["git", "rev-parse", "--is-inside-work-tree"])
    if _stdout(inside) != "true":
        raise NotAGitCheckout(f"{repo}: not inside a Git working tree ({inside.get('stderr', '').strip()})")

    tracked = _stdout(
        run_probe(repo, ["git", "status", "--porcelain=v1", "--untracked-files=no"])
    )
    untracked = _stdout(run_probe(repo, ["git", "ls-files", "--others", "--exclude-standard"]))
    return {
        "source_sha": _stdout(run_probe(repo, ["git", "rev-parse", "HEAD"])),
        "branch": _stdout(run_probe(repo, ["git", "rev-parse", "--abbrev-ref", "HEAD"])),
        "upstream": _stdout(
            run_probe(
                repo,
                ["git", "rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{upstream}"],
            )
        ),
        "git_version": _stdout(run_probe(repo, ["git", "--version"])),
        "tool_version": _stdout(run_probe(repo, list(tool))),
        "config": _reported_config(repo),
        "tracked_changes": tracked.splitlines() if tracked is not None else None,
        "untracked_not_ignored": untracked.splitlines() if untracked is not None else None,
        "probe": _probe_file(repo, probe_path),
        "historical": _historical(repo, historical_revision, probe_path),
    }


class NotAGitCheckout(Exception):
    """The diagnostic target is not a Git working tree."""


def render_summary(report: dict[str, Any]) -> str:
    """A Markdown digest of a report, for a job summary."""
    probe = report["probe"]
    history = report["historical"]
    lines = [
        "### Release calculation diagnostics",
        "",
        f"- source: `{report['source_sha']}` on `{report['branch']}` (upstream `{report['upstream']}`)",
        f"- tools: `{report['tool_version']}`, `{report['git_version']}`",
        f"- tracked changes: {len(report['tracked_changes'] or [])}; "
        f"untracked, not ignored: {len(report['untracked_not_ignored'] or [])}",
        f"- `{probe['path']}`: `{probe['eol']}`; matches index after filters: "
        f"{probe['worktree'].get('matches_index_after_filters')}",
        f"- `{history['revision']}` available: {history['available']}; "
        f"probe present: {history.get('probe_present')}; "
        f"`.gitattributes` present: {history.get('gitattributes_present')}",
    ]
    for entry in (report["tracked_changes"] or [])[:50]:
        lines.append(f"  - tracked: `{entry}`")
    for entry in (report["untracked_not_ignored"] or [])[:50]:
        lines.append(f"  - untracked: `{entry}`")
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------------
# Strict loaders and field validators
#
# Every load-bearing value read from a GitHub event, an API response, or the
# release configuration goes through these. An invalid shape is a located
# `ReleasePathError`, raised before any credential is minted or any release
# command runs; nothing is defaulted, filtered, or coerced.
# ---------------------------------------------------------------------------

CANONICAL_REPOSITORY = "yankeeinlondon/rusty-biscuit"
RELEASE_BRANCH = "main"
RELEASE_LABEL = "release"
# The `name:` of `.github/workflows/ci.yml`, which is what a `workflow_run`
# event reports as `workflow_run.name`. It must equal the title
# `release-plz.yml` triggers on; `release_preparation_accepts_the_ci_workflow_title`
# in `ci_workflow_contracts.rs` fails when either drifts.
CI_WORKFLOW_TITLE = "Continuous integration"
PR_BRANCH_PREFIX = "release-plz-"
TAG_TEMPLATE = "{{ package }}-v{{ version }}"
# The temporary first-release allowlist `release-plz.toml` declares. A
# calculation or proposal that names any other package is an error, never a
# reason to release it. Restoring workspace-wide eligibility edits both.
APPROVED_RELEASE_SET = frozenset({"git-wt-cli", "sniff-detection-cli"})

_SHA = re.compile(r"^[0-9a-f]{40}$")
_REPOSITORY = re.compile(r"^[A-Za-z0-9-]+/[A-Za-z0-9._-]+$")
# SemVer 2.0.0, including prerelease and build identifiers.
SEMVER = (
    r"(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)"
    r"(?:-(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*)?"
    r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
)
_SEMVER = re.compile(rf"^{SEMVER}$")


class ReleasePathError(Exception):
    """An invalid input or a refused release state, located by `path`."""

    def __init__(self, path: str, message: str) -> None:
        super().__init__(f"{path}: {message}")
        self.path = path
        self.message = message


def _reject_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate object key {key!r}")
        result[key] = value
    return result


def _reject_constant(token: str) -> Any:
    raise ValueError(f"invalid JSON constant {token}")


def load_json(text: str, *, source: str) -> Any:
    """Parse one JSON document, refusing duplicate keys, `NaN`/`Infinity`,
    and trailing content (which `json.loads` already rejects)."""
    try:
        return json.loads(text, object_pairs_hook=_reject_duplicates, parse_constant=_reject_constant)
    except ValueError as error:
        raise ReleasePathError(source, f"invalid JSON: {error}") from None


def load_event(path: Path) -> dict[str, Any]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as error:
        raise ReleasePathError(str(path), f"cannot read event file: {error}") from None
    event = load_json(text, source=str(path))
    if not isinstance(event, dict):
        raise ReleasePathError(str(path), f"event root must be an object, found {type(event).__name__}")
    return event


_ABSENT = object()


def _lookup(root: Any, path: str) -> Any:
    """The value at dotted `path`, or `_ABSENT`. An intermediate that is null or
    not an object is located at that intermediate."""
    current = root
    walked: list[str] = []
    for part in path.split("."):
        if current is None:
            raise ReleasePathError(".".join(walked), "is null")
        if not isinstance(current, dict):
            raise ReleasePathError(".".join(walked), f"must be an object, found {type(current).__name__}")
        walked.append(part)
        if part not in current:
            return _ABSENT
        current = current[part]
    return current


def _present(root: Any, path: str) -> Any:
    value = _lookup(root, path)
    if value is _ABSENT:
        raise ReleasePathError(path, "is absent")
    if value is None:
        raise ReleasePathError(path, "is null")
    return value


def req_object(root: Any, path: str) -> dict[str, Any]:
    value = _present(root, path)
    if not isinstance(value, dict):
        raise ReleasePathError(path, f"must be an object, found {type(value).__name__}")
    return value


def req_str(root: Any, path: str) -> str:
    value = _present(root, path)
    if not isinstance(value, str):
        raise ReleasePathError(path, f"must be a string, found {type(value).__name__}")
    if not value:
        raise ReleasePathError(path, "must not be empty")
    return value


def req_bool(root: Any, path: str) -> bool:
    value = _present(root, path)
    if not isinstance(value, bool):
        raise ReleasePathError(path, f"must be a boolean, found {type(value).__name__}")
    return value


def req_int(root: Any, path: str) -> int:
    value = _present(root, path)
    if isinstance(value, bool) or not isinstance(value, int):
        raise ReleasePathError(path, f"must be an integer, found {type(value).__name__}")
    return value


def req_repository(root: Any, path: str) -> str:
    value = req_str(root, path)
    if not _REPOSITORY.match(value):
        raise ReleasePathError(path, f"must be an owner/name repository, found {value!r}")
    return value


def req_sha(root: Any, path: str) -> str:
    value = req_str(root, path)
    if not _SHA.match(value):
        raise ReleasePathError(path, f"must be a full 40-digit commit SHA, found {value!r}")
    return value


def req_label_names(root: Any, path: str) -> list[str]:
    """Label names from a list of label objects. `[]` is valid; a bad element
    fails the whole list rather than being skipped."""
    value = _present(root, path)
    if not isinstance(value, list):
        raise ReleasePathError(path, f"must be a list, found {type(value).__name__}")
    names = []
    for index, element in enumerate(value):
        if not isinstance(element, dict):
            raise ReleasePathError(f"{path}[{index}]", f"must be a label object, found {type(element).__name__}")
        try:
            names.append(req_str(element, "name"))
        except ReleasePathError as error:
            raise ReleasePathError(f"{path}[{index}].{error.path}", error.message) from None
    return names


def _within(prefix: str, validator: Any, root: Any, path: str) -> Any:
    """`validator(root, path)` with any error located under `prefix`, the
    document the field was read from. A root that is not an object is located
    at `prefix` itself."""
    try:
        return validator(root, path)
    except ReleasePathError as error:
        raise ReleasePathError(f"{prefix}.{error.path}" if error.path else prefix, error.message) from None


# ---------------------------------------------------------------------------
# Release configuration
# ---------------------------------------------------------------------------

# Workspace settings the per-package publication configuration carries over
# unchanged. Anything absent keeps release-plz's documented default.
_CARRIED_SETTINGS = (
    "git_release_enable",
    "git_release_draft",
    "git_release_type",
    "git_release_latest",
    "git_release_name",
    "git_release_body",
    "git_tag_enable",
    "semver_check",
)


@dataclass(frozen=True)
class ReleaseConfig:
    workspace: dict[str, Any]
    packages: dict[str, dict[str, Any]]

    @property
    def enabled(self) -> frozenset[str]:
        return frozenset(name for name, entry in self.packages.items() if entry.get("release") is True)


def load_release_config(text: str, *, source: str = "release-plz.toml") -> ReleaseConfig:
    """Parse and validate the release configuration orchestration depends on.

    `tomllib` already refuses duplicate keys and trailing content. The checks
    here are the invariants publication relies on: the tag template it derives
    tags from, Git-only operation without a registry, the branch prefix it
    authenticates proposals by, and the temporary allowlist.
    """
    try:
        document = tomllib.loads(text)
    except tomllib.TOMLDecodeError as error:
        raise ReleasePathError(source, f"invalid TOML: {error}") from None

    workspace = req_object(document, "workspace")
    expected = {
        "release": False,
        "git_only": True,
        "publish": False,
        "release_always": False,
        "pr_branch_prefix": PR_BRANCH_PREFIX,
        "git_tag_name": TAG_TEMPLATE,
    }
    for key, wanted in expected.items():
        path = f"{source}:workspace.{key}"
        value = _within(f"{source}:workspace", req_bool if isinstance(wanted, bool) else req_str, workspace, key)
        if value != wanted:
            raise ReleasePathError(path, f"must be {wanted!r}, found {value!r}")
    for key in _CARRIED_SETTINGS:
        value = workspace.get(key, _ABSENT)
        if value is not _ABSENT and not isinstance(value, (str, bool)):
            raise ReleasePathError(f"{source}:workspace.{key}", f"must be a string or boolean, found {type(value).__name__}")
    if workspace.get("git_release_draft", False) is not False:
        raise ReleasePathError(f"{source}:workspace.git_release_draft", "must be false; publication refuses draft Releases")

    raw_packages = _present(document, "package")
    if not isinstance(raw_packages, list):
        raise ReleasePathError(f"{source}:package", f"must be a list of tables, found {type(raw_packages).__name__}")
    packages: dict[str, dict[str, Any]] = {}
    for index, entry in enumerate(raw_packages):
        where = f"{source}:package[{index}]"
        if not isinstance(entry, dict):
            raise ReleasePathError(where, f"must be a table, found {type(entry).__name__}")
        try:
            name = req_str(entry, "name")
        except ReleasePathError as error:
            raise ReleasePathError(f"{where}.{error.path}", error.message) from None
        if name in packages:
            raise ReleasePathError(f"{where}.name", f"duplicate package {name!r}")
        if "release" in entry and not isinstance(entry["release"], bool):
            raise ReleasePathError(f"{where}.release", f"must be a boolean, found {type(entry['release']).__name__}")
        if "changelog_path" in entry:
            try:
                req_str(entry, "changelog_path")
            except ReleasePathError as error:
                raise ReleasePathError(f"{where}.{error.path}", error.message) from None
        packages[name] = entry

    config = ReleaseConfig(workspace=workspace, packages=packages)
    if config.enabled != APPROVED_RELEASE_SET:
        raise ReleasePathError(
            f"{source}:package",
            f"release-eligible packages must be exactly {sorted(APPROVED_RELEASE_SET)}, "
            f"found {sorted(config.enabled)}",
        )
    return config


def _toml_value(value: Any) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, str):
        # A JSON string literal is a valid TOML basic string.
        return json.dumps(value, ensure_ascii=False)
    raise ReleasePathError("generated-config", f"unsupported value type {type(value).__name__}")


def package_release_config(config: ReleaseConfig, package: str, *, recovery: bool) -> str:
    """The release-plz configuration that releases `package` alone.

    Workspace eligibility stays off and only `package` opts in, with the
    original tag template, Git-only and registry settings, carried release
    settings, and its canonical changelog path. Recovery (the tag exists, its
    Release does not) disables tag creation so release-plz creates only the
    Release. The text is re-parsed before it is returned.
    """
    if package not in config.enabled:
        raise ReleasePathError(f"package {package}", "is not release-eligible")
    workspace: dict[str, Any] = {
        "release": False,
        "git_only": True,
        "publish": False,
        "release_always": False,
        "pr_branch_prefix": PR_BRANCH_PREFIX,
        "git_tag_name": TAG_TEMPLATE,
    }
    for key in _CARRIED_SETTINGS:
        if key in config.workspace:
            workspace[key] = config.workspace[key]
    if recovery:
        workspace["git_tag_enable"] = False
    entry: dict[str, Any] = {"name": package, "release": True}
    if "changelog_path" in config.packages[package]:
        entry["changelog_path"] = config.packages[package]["changelog_path"]

    lines = ["# Generated per-package publication configuration; never committed.", "[workspace]"]
    lines += [f"{key} = {_toml_value(value)}" for key, value in workspace.items()]
    lines += ["", "[[package]]"]
    lines += [f"{key} = {_toml_value(value)}" for key, value in entry.items()]
    text = "\n".join(lines) + "\n"
    reparsed = tomllib.loads(text)
    if reparsed["workspace"] != workspace or reparsed["package"] != [entry]:
        raise ReleasePathError("generated-config", "did not round-trip")
    return text


# ---------------------------------------------------------------------------
# Selection: which events may prepare or publish a release
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Selection:
    """`selected` false is a deliberate, successful no-op with a `reason`."""

    selected: bool
    reason: str
    outputs: dict[str, str] = field(default_factory=dict)


def _skip(reason: str) -> Selection:
    return Selection(False, reason)


def select_preparation(event_name: str, repository: str, event: dict[str, Any]) -> Selection:
    """Whether this event is a completed, successful push run on `main` of the
    workflow titled `CI_WORKFLOW_TITLE` in the canonical repository. Every
    other supported shape is a no-op."""
    if event_name != "workflow_run":
        return _skip(f"event {event_name!r} does not prepare releases")
    if repository != CANONICAL_REPOSITORY:
        return _skip(f"repository {repository!r} is not {CANONICAL_REPOSITORY}")
    if req_str(event, "workflow_run.status") != "completed":
        return _skip("the ci run has not completed")
    conclusion = req_str(event, "workflow_run.conclusion")
    if conclusion != "success":
        return _skip(f"the ci run concluded {conclusion!r}")
    name = req_str(event, "workflow_run.name")
    if name != CI_WORKFLOW_TITLE:
        return _skip(f"workflow {name!r} is not {CI_WORKFLOW_TITLE!r}")
    trigger = req_str(event, "workflow_run.event")
    if trigger != "push":
        return _skip(f"the ci run was triggered by {trigger!r}, not push")
    branch = req_str(event, "workflow_run.head_branch")
    if branch != RELEASE_BRANCH:
        return _skip(f"the ci run built {branch!r}, not {RELEASE_BRANCH}")
    for path in ("workflow_run.repository.full_name", "workflow_run.head_repository.full_name"):
        found = req_repository(event, path)
        if found != CANONICAL_REPOSITORY:
            raise ReleasePathError(path, f"must be {CANONICAL_REPOSITORY}, found {found!r}")
    head_sha = req_sha(event, "workflow_run.head_sha")
    return Selection(True, "successful ci push run on main", {"validated_sha": head_sha})


def select_publication(
    event_name: str, repository: str, event: dict[str, Any], bot_login: str | None
) -> Selection:
    """Whether this event is the merge of a release proposal authored by the
    configured App. Unrelated pull requests are no-ops; a plausible proposal
    (merged into `main`, labeled `release`, on a `release-plz-` branch) that
    fails any identity check is an error."""
    if event_name != "pull_request":
        return _skip(f"event {event_name!r} does not publish releases")
    if repository != CANONICAL_REPOSITORY:
        return _skip(f"repository {repository!r} is not {CANONICAL_REPOSITORY}")
    action = req_str(event, "action")
    if action != "closed":
        return _skip(f"pull request action {action!r} is not closed")
    if not req_bool(event, "pull_request.merged"):
        return _skip("the pull request was closed without merging")
    if RELEASE_LABEL not in req_label_names(event, "pull_request.labels"):
        return _skip(f"the pull request has no {RELEASE_LABEL!r} label")
    base_ref = req_str(event, "pull_request.base.ref")
    if base_ref != RELEASE_BRANCH:
        return _skip(f"the pull request merged into {base_ref!r}, not {RELEASE_BRANCH}")
    head_ref = req_str(event, "pull_request.head.ref")
    if not head_ref.startswith(PR_BRANCH_PREFIX):
        return _skip(f"branch {head_ref!r} is not a {PR_BRANCH_PREFIX}* release proposal")

    for path in ("pull_request.base.repo.full_name", "pull_request.head.repo.full_name"):
        found = req_repository(event, path)
        if found != CANONICAL_REPOSITORY:
            raise ReleasePathError(path, f"must be {CANONICAL_REPOSITORY}, found {found!r}")
    author_type = req_str(event, "pull_request.user.type")
    if author_type != "Bot":
        raise ReleasePathError("pull_request.user.type", f"must be 'Bot', found {author_type!r}")
    if not bot_login:
        raise ReleasePathError(
            "RELEASE_APP_BOT_LOGIN",
            "repository variable is not set; set it to the release App's bot login, "
            "including the [bot] suffix (see docs/releases.md)",
        )
    author = req_str(event, "pull_request.user.login")
    if author != bot_login:
        raise ReleasePathError("pull_request.user.login", f"must be the release App {bot_login!r}, found {author!r}")
    return Selection(
        True,
        "merged release proposal authored by the release App",
        {
            "merge_sha": req_sha(event, "pull_request.merge_commit_sha"),
            "head_sha": req_sha(event, "pull_request.head.sha"),
            "number": str(req_int(event, "pull_request.number")),
        },
    )


def require_app_configuration(env: dict[str, str], *, need_bot_login: bool) -> None:
    """Refuse a selected release path whose App configuration is incomplete.
    There is never a fallback to the workflow's default token."""
    missing = []
    if not env.get("RELEASE_APP_ID", "").strip():
        missing.append("repository variable RELEASE_APP_ID")
    if env.get("HAS_RELEASE_APP_PRIVATE_KEY") != "true":
        missing.append("repository secret RELEASE_APP_PRIVATE_KEY")
    login = env.get("RELEASE_APP_BOT_LOGIN", "").strip()
    if need_bot_login and not login:
        missing.append("repository variable RELEASE_APP_BOT_LOGIN")
    if missing:
        raise ReleasePathError("release App", f"not configured: {', '.join(missing)} (see docs/releases.md)")
    if need_bot_login and not login.endswith("[bot]"):
        raise ReleasePathError("RELEASE_APP_BOT_LOGIN", f"must be the App's bot login ending in [bot], found {login!r}")


# ---------------------------------------------------------------------------
# Git and the forge
# ---------------------------------------------------------------------------


class Runner:
    """Runs commands. Tests substitute a recording fake."""

    def run(self, argv: Sequence[str], *, cwd: Path, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
        return subprocess.run(list(argv), cwd=cwd, env=env, capture_output=True, text=True, check=False)


def _git_stdout(runner: Runner, repo: Path, *args: str, env: dict[str, str] | None = None) -> str:
    completed = runner.run(["git", *args], cwd=repo, env=env if env is not None else _git_env())
    if completed.returncode != 0:
        raise ReleasePathError(f"git {' '.join(args)}", completed.stderr.strip() or f"exit {completed.returncode}")
    return completed.stdout


def _git(runner: Runner, repo: Path, *args: str, env: dict[str, str] | None = None) -> str:
    return _git_stdout(runner, repo, *args, env=env).strip()


def _git_lines(runner: Runner, repo: Path, *args: str) -> list[str]:
    """Non-empty output lines, unstripped: a porcelain status line's leading
    space is its index column."""
    return [line for line in _git_stdout(runner, repo, *args).splitlines() if line]


def verify_freshness(runner: Runner, repo: Path, validated_sha: str) -> Selection:
    head = _git(runner, repo, "rev-parse", "HEAD")
    if head != validated_sha:
        return _skip(f"main advanced from {validated_sha} to {head}; that commit's own ci run owns its release calculation")
    return Selection(True, "main is the commit ci validated", {"validated_sha": validated_sha})


def verify_publication_topology(runner: Runner, repo: Path, merge_sha: str, head_sha: str) -> str:
    """Prove the checkout is the merge commit, that it is a two-parent merge
    whose second parent is the proposal's final commit, and return the
    proposal's base: the merge base of the first parent and that commit.

    A squash or rebase merge has one parent or a foreign second parent and is
    refused; ancestry alone would accept a fast-forward rebase."""
    head = _git(runner, repo, "rev-parse", "HEAD")
    if head != merge_sha:
        raise ReleasePathError("checkout", f"HEAD is {head}, expected merge commit {merge_sha}")
    parents = _git(runner, repo, "rev-list", "--parents", "-n", "1", merge_sha).split()[1:]
    if len(parents) != 2:
        raise ReleasePathError(
            "pull_request.merge_commit_sha",
            f"{merge_sha} has {len(parents)} parent(s); release proposals must be merged with a merge commit",
        )
    if parents[1] != head_sha:
        raise ReleasePathError(
            "pull_request.merge_commit_sha",
            f"second parent {parents[1]} is not the proposal's final commit {head_sha}",
        )
    ancestry = runner.run(["git", "merge-base", "--is-ancestor", head_sha, merge_sha], cwd=repo, env=_git_env())
    if ancestry.returncode != 0:
        raise ReleasePathError("pull_request.head.sha", f"{head_sha} is not an ancestor of {merge_sha}")
    return _git(runner, repo, "merge-base", parents[0], head_sha)


@dataclass(frozen=True)
class ReleaseRecord:
    """The Release for one tag: `exists` false only on an explicit 404."""

    exists: bool
    draft: bool = False
    url: str | None = None


class Forge:
    """Remote tag and Release state. Tests substitute a fake."""

    def remote_tag_target(self, tag: str) -> str | None:
        raise NotImplementedError

    def release(self, tag: str) -> ReleaseRecord:
        raise NotImplementedError

    def user_id(self, login: str) -> int:
        raise NotImplementedError


class GitHubForge(Forge):
    def __init__(self, *, repository: str, token: str | None, repo: Path, runner: Runner, git_env: dict[str, str]):
        self.repository = repository
        self.token = token
        self.repo = repo
        self.runner = runner
        self.git_env = git_env
        self.api = os.environ.get("GITHUB_API_URL", "https://api.github.com").rstrip("/")

    def _get(self, path: str) -> tuple[int, Any]:
        request = urllib.request.Request(f"{self.api}{path}")
        request.add_header("Accept", "application/vnd.github+json")
        request.add_header("X-GitHub-Api-Version", "2022-11-28")
        if self.token:
            request.add_header("Authorization", f"Bearer {self.token}")
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                return response.status, load_json(response.read().decode("utf-8"), source=f"GET {path}")
        except urllib.error.HTTPError as error:
            return error.code, None
        except (urllib.error.URLError, TimeoutError, OSError) as error:
            raise ReleasePathError(f"GET {path}", f"request failed: {error}") from None

    def remote_tag_target(self, tag: str) -> str | None:
        ref = f"refs/tags/{tag}"
        listing = _git(self.runner, self.repo, "ls-remote", "--tags", "origin", ref, f"{ref}^{{}}", env=self.git_env)
        return peeled_tag_target(listing, tag)

    def release(self, tag: str) -> ReleaseRecord:
        quoted = urllib.parse.quote(tag, safe="")
        status, body = self._get(f"/repos/{self.repository}/releases/tags/{quoted}")
        if status == 200:
            return _release_record(body, tag, f"GET releases/tags/{tag}")
        if status != 404:
            raise ReleasePathError(f"GET releases/tags/{tag}", f"unexpected HTTP {status}")
        # The by-tag endpoint omits drafts; a draft is visible only in the list
        # to a token with write access.
        for page in range(1, 11):
            status, body = self._get(f"/repos/{self.repository}/releases?per_page=100&page={page}")
            if status != 200 or not isinstance(body, list):
                raise ReleasePathError("GET releases", f"unexpected HTTP {status} or non-list body")
            for index, item in enumerate(body):
                if _within(f"GET releases[{index}]", req_str, item, "tag_name") == tag:
                    return _release_record(item, tag, f"GET releases[{index}]")
            if len(body) < 100:
                return ReleaseRecord(exists=False)
        raise ReleasePathError("GET releases", "more than 1000 Releases; cannot prove a draft is absent")

    def user_id(self, login: str) -> int:
        status, body = self._get(f"/users/{urllib.parse.quote(login, safe='')}")
        if status != 200:
            raise ReleasePathError(f"GET users/{login}", f"unexpected HTTP {status}")
        return _within(f"GET users/{login}", req_int, body, "id")


def _release_record(body: Any, tag: str, source: str) -> ReleaseRecord:
    if not isinstance(body, dict):
        raise ReleasePathError(source, "must be a Release object")
    found = _within(source, req_str, body, "tag_name")
    if found != tag:
        raise ReleasePathError(f"{source}.tag_name", f"expected {tag!r}, found {found!r}")
    return ReleaseRecord(exists=True, draft=_within(source, req_bool, body, "draft"),
                         url=_within(source, req_str, body, "html_url"))


def peeled_tag_target(listing: str, tag: str) -> str | None:
    """The commit a remote tag names, from `git ls-remote` output for the tag
    and its peeled `^{}` form. An annotated tag's own object is never the
    answer; its peeled commit is."""
    direct = peeled = None
    for line in listing.splitlines():
        sha, _, ref = line.partition("\t")
        if ref == f"refs/tags/{tag}":
            direct = sha
        elif ref == f"refs/tags/{tag}^{{}}":
            peeled = sha
        else:
            raise ReleasePathError(f"ls-remote {tag}", f"unexpected ref {ref!r}")
    target = peeled or direct
    if target is not None and not _SHA.match(target):
        raise ReleasePathError(f"ls-remote {tag}", f"invalid object name {target!r}")
    return target


# ---------------------------------------------------------------------------
# Candidates and their remote state
# ---------------------------------------------------------------------------

COMPLETE = "complete"
NEW = "new"
RECOVERY = "recovery"


@dataclass
class Candidate:
    package: str
    version: str
    tag: str
    manifest: str
    state: str | None = None
    remote_tag_target: str | None = None
    release_url: str | None = None
    error: str | None = None

    def as_dict(self) -> dict[str, Any]:
        return dict(self.__dict__)


def _parse_manifest(text: str, source: str) -> dict[str, Any]:
    try:
        return tomllib.loads(text)
    except tomllib.TOMLDecodeError as error:
        raise ReleasePathError(source, f"invalid TOML: {error}") from None


def _read_revision(runner: Runner, repo: Path, revision: str | None, path: str) -> str | None:
    """`path`'s text at `revision` (the working tree when None), or None when
    the file does not exist there."""
    if revision is None:
        file = repo / path
        return file.read_text(encoding="utf-8") if file.is_file() else None
    show = runner.run(["git", "show", f"{revision}:{path}"], cwd=repo, env=_git_env())
    return show.stdout if show.returncode == 0 else None


def _package_version(runner: Runner, repo: Path, revision: str | None, path: str) -> tuple[str, str] | None:
    """`(name, version)` declared by the manifest at `path`, or None when the
    file is absent at `revision` or declares no `[package]`. A version
    inherited from `[workspace.package]` is resolved from the root manifest at
    the same revision."""
    where = f"{revision or 'worktree'}:{path}"
    text = _read_revision(runner, repo, revision, path)
    if text is None:
        return None
    manifest = _parse_manifest(text, where)
    if "package" not in manifest:
        return None
    name = req_str(manifest, "package.name")
    version = _lookup(manifest, "package.version")
    if version == {"workspace": True}:
        root_text = _read_revision(runner, repo, revision, "Cargo.toml")
        root = _parse_manifest(root_text, f"{revision or 'worktree'}:Cargo.toml") if root_text is not None else {}
        version = _lookup(root, "workspace.package.version")
    if not isinstance(version, str) or not _SEMVER.match(version):
        raise ReleasePathError(f"{where}:package.version", f"must be a SemVer string, found {version!r}")
    return name, version


@dataclass(frozen=True)
class VersionChange:
    previous: str | None
    version: str
    manifest: str


def version_changes(runner: Runner, repo: Path, base: str, head: str | None) -> dict[str, VersionChange]:
    """Packages whose manifest version differs between `base` and `head` (the
    working tree when `head` is None), by package name."""
    names = _git(runner, repo, "diff", "--name-only", "--no-renames", base, *([head] if head else []), "--", "*Cargo.toml")
    changes: dict[str, VersionChange] = {}
    for path in filter(None, names.splitlines()):
        new = _package_version(runner, repo, head, path)
        if new is None:
            continue
        old = _package_version(runner, repo, base, path)
        if old == new:
            continue
        name, version = new
        if name in changes:
            raise ReleasePathError(path, f"duplicate package {name!r}")
        changes[name] = VersionChange(previous=old[1] if old and old[0] == name else None, version=version, manifest=path)
    return changes


def publication_candidates(runner: Runner, repo: Path, base: str, head: str) -> list[Candidate]:
    """The packages the proposal releases, with tags derived from the versions
    at its final commit. They must be exactly the approved set; any other
    version bump is refused rather than released."""
    changes = version_changes(runner, repo, base, head)
    unexpected = sorted(set(changes) - APPROVED_RELEASE_SET)
    missing = sorted(APPROVED_RELEASE_SET - set(changes))
    if unexpected or missing:
        raise ReleasePathError(
            "proposal",
            f"must bump exactly {sorted(APPROVED_RELEASE_SET)}; unexpected {unexpected}, missing {missing}",
        )
    return [
        Candidate(package=name, version=change.version, tag=f"{name}-v{change.version}", manifest=change.manifest)
        for name, change in sorted(changes.items())
    ]


def classify(candidate: Candidate, forge: Forge, head_sha: str) -> Candidate:
    """Fill `state` from the remote tag and Release. A tag at another commit, a
    Release without its tag, and a draft Release are refused."""
    candidate.state = candidate.error = None
    target = forge.remote_tag_target(candidate.tag)
    record = forge.release(candidate.tag)
    candidate.remote_tag_target = target
    candidate.release_url = record.url
    if target is not None and target != head_sha:
        candidate.error = f"tag {candidate.tag} points at {target}, not the proposal's final commit {head_sha}; a published tag is never moved"
    elif record.exists and record.draft:
        candidate.error = f"an unexpected draft Release exists for {candidate.tag}"
    elif record.exists and target is None:
        candidate.error = f"a Release exists for {candidate.tag} but its tag does not"
    elif record.exists:
        candidate.state = COMPLETE
    elif target is not None:
        candidate.state = RECOVERY
    else:
        candidate.state = NEW
    return candidate


def inventory(candidates: list[Candidate], forge: Forge, head_sha: str) -> list[Candidate]:
    """Classify every candidate; API failures are recorded as errors, never
    read as absence."""
    for candidate in candidates:
        try:
            classify(candidate, forge, head_sha)
        except ReleasePathError as error:
            candidate.state = None
            candidate.error = str(error)
    return candidates


# ---------------------------------------------------------------------------
# The calculation environment shared by `update` and `release-pr`
# ---------------------------------------------------------------------------

# A version calculation checks out every package's last release commit in
# release-plz's own copy of the repository. Walking back from a commit that has
# `.gitattributes` to one that predates it leaves `scripts/init.ps1` (unchanged
# blob) in its CRLF form under no rule; the copy's stale stat cache then makes
# Git see it as modified, and the next checkout aborts. Supplying the
# repository's own PowerShell rule as a fallback attributes file keeps that rule
# in force at commits without `.gitattributes`; where `.gitattributes` exists it
# takes precedence, so current trees behave exactly as before. The rule must
# stay identical to the `.gitattributes` line (a contract test compares them).
CALCULATION_ATTRIBUTES = "*.ps1 text eol=crlf\n"


def append_git_config(base: dict[str, str], entries: Sequence[tuple[str, str]]) -> dict[str, str]:
    """`base` plus `entries` as process-scoped Git configuration
    (`GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n`), after any entries already there."""
    env = dict(base)
    count = int(env.get("GIT_CONFIG_COUNT", "0") or "0")
    for offset, (key, value) in enumerate(entries):
        env[f"GIT_CONFIG_KEY_{count + offset}"] = key
        env[f"GIT_CONFIG_VALUE_{count + offset}"] = value
    env["GIT_CONFIG_COUNT"] = str(count + len(entries))
    return env


def calculation_env(base: dict[str, str], *, attributes_dir: Path) -> dict[str, str]:
    """`base` with `CALCULATION_ATTRIBUTES` as `core.attributesFile`, written
    into `attributes_dir`, which must lie outside the checkout so the clean-tree
    checks still see every file release-plz writes. Only processes receiving
    this environment are affected: no checkout, `.gitattributes`, or Git
    configuration file changes."""
    path = attributes_dir / "release-calculation.gitattributes"
    path.write_text(CALCULATION_ATTRIBUTES, encoding="utf-8")
    return append_git_config(base, [("core.attributesFile", str(path))])


@contextmanager
def calculation_environment(base: dict[str, str]) -> Iterator[dict[str, str]]:
    """`calculation_env` in a temporary directory removed afterwards."""
    with tempfile.TemporaryDirectory(prefix="release-calculation-") as directory:
        yield calculation_env(base, attributes_dir=Path(directory))


# ---------------------------------------------------------------------------
# Automation identity and authentication for one release-plz process
# ---------------------------------------------------------------------------

# Reads the token from the release-plz process's own environment, so the token
# is never written to disk, a remote URL, or a command line.
_CREDENTIAL_HELPER = '!f() { test "$1" = get || exit 0; echo username=x-access-token; echo "password=${GIT_TOKEN}"; }; f'


def credential_env(base: dict[str, str], *, token: str) -> dict[str, str]:
    """`base` with Git HTTPS authentication by `token`, scoped to the processes
    that receive this environment. The workflow's default token is removed."""
    env = {key: value for key, value in base.items() if key not in {"GITHUB_TOKEN", "GH_TOKEN"}}
    # An empty value first resets any helper inherited from other config.
    env = append_git_config(env, [("credential.helper", ""), ("credential.helper", _CREDENTIAL_HELPER)])
    env.update(GIT_TOKEN=token, GIT_TERMINAL_PROMPT="0")
    return env


def automation_env(base: dict[str, str], *, token: str, bot_login: str, bot_id: int) -> dict[str, str]:
    """The environment for one release-plz process: the App token for its API
    calls (`GIT_TOKEN`) and Git pushes, and the App bot as author and
    committer. Nothing global is changed."""
    env = credential_env(base, token=token)
    email = f"{bot_id}+{bot_login}@users.noreply.github.com"
    env.update(
        GIT_AUTHOR_NAME=bot_login,
        GIT_AUTHOR_EMAIL=email,
        GIT_COMMITTER_NAME=bot_login,
        GIT_COMMITTER_EMAIL=email,
    )
    return env


# ---------------------------------------------------------------------------
# Publication
# ---------------------------------------------------------------------------


@dataclass
class PublicationResult:
    exit_status: int
    candidates: list[Candidate]
    failures: list[str] = field(default_factory=list)


def publish(
    *,
    runner: Runner,
    forge: Forge,
    repo: Path,
    workdir: Path,
    config: ReleaseConfig,
    candidates: list[Candidate],
    head_sha: str,
    env: dict[str, str],
    repository: str = CANONICAL_REPOSITORY,
) -> PublicationResult:
    """Publish or recover every candidate, one release-plz process each, from a
    fresh clone detached at the proposal's final commit.

    Every candidate is classified before any is touched, and re-checked just
    before its own invocation. Complete candidates are skipped, so a rerun of
    the original event writes nothing. The first failing exit status is
    preserved; a final inventory follows success or failure.
    """
    failures: list[str] = []
    inventory(candidates, forge, head_sha)
    refused = [candidate.error for candidate in candidates if candidate.error]
    if refused:
        return PublicationResult(1, candidates, [f"refused before any mutation: {reason}" for reason in refused])

    status = 0
    for candidate in candidates:
        try:
            classify(candidate, forge, head_sha)
            if candidate.error:
                failures.append(candidate.error)
                status = 1
                break
            if candidate.state == COMPLETE:
                continue
            code = _release_one(runner, repo, workdir, config, candidate, head_sha, env, repository)
        except ReleasePathError as error:
            failures.append(f"{candidate.package}: {error}")
            status = 1
            break
        if code != 0:
            failures.append(f"release-plz release for {candidate.package} exited {code}")
            status = code
            break

    inventory(candidates, forge, head_sha)
    for candidate in candidates:
        if candidate.error:
            failures.append(candidate.error)
        elif candidate.state != COMPLETE and status == 0:
            failures.append(f"{candidate.tag} is {candidate.state} after publication")
    if failures and status == 0:
        status = 1
    return PublicationResult(status, candidates, failures)


def _release_one(
    runner: Runner,
    repo: Path,
    workdir: Path,
    config: ReleaseConfig,
    candidate: Candidate,
    head_sha: str,
    env: dict[str, str],
    repository: str,
) -> int:
    recovery = candidate.state == RECOVERY
    target = workdir / candidate.package
    if target.exists():
        shutil.rmtree(target)
    target.mkdir(parents=True)
    clone = target / "clone"
    _git(runner, repo, "clone", "--quiet", "--no-checkout", str(repo), str(clone))
    _git(runner, clone, "remote", "set-url", "origin", f"https://github.com/{repository}.git")
    _git(runner, clone, "checkout", "--quiet", "--detach", head_sha)
    if _git(runner, clone, "rev-parse", "HEAD") != head_sha:
        raise ReleasePathError("publication clone", f"HEAD is not {head_sha}")
    if recovery:
        # Local only: release-plz skips a package whose tag it can see, before
        # it checks the Release. The remote tag is never deleted or moved.
        _git(runner, clone, "tag", "-d", candidate.tag)
    # Outside the clone, so the checkout release-plz inspects stays clean.
    config_path = target / "release-plz.toml"
    config_path.write_text(package_release_config(config, candidate.package, recovery=recovery), encoding="utf-8")
    completed = runner.run(
        [
            "release-plz", "release",
            "--config", str(config_path),
            "--manifest-path", str(clone / "Cargo.toml"),
            "--forge", "github",
            "--repo-url", f"https://github.com/{repository}",
            "--output", "json",
        ],
        cwd=clone,
        env=env,
    )
    sys.stdout.write(completed.stdout)
    sys.stderr.write(completed.stderr)
    return completed.returncode


def render_inventory(title: str, candidates: list[Candidate], *, context: dict[str, str]) -> str:
    lines = [f"### {title}", ""]
    lines += [f"- {key}: `{value}`" for key, value in context.items()]
    lines += ["", "| Package | Tag | State | Remote tag target | Release |", "| --- | --- | --- | --- | --- |"]
    for candidate in candidates:
        state = candidate.state or f"ERROR: {candidate.error}"
        lines.append(
            f"| {candidate.package} | `{candidate.tag}` | {state} | "
            f"`{candidate.remote_tag_target or '-'}` | {candidate.release_url or '-'} |"
        )
    return "\n".join(lines) + "\n"


def calculation_report(runner: Runner, repo: Path) -> dict[str, Any]:
    """What a local `release-plz update` changed: version bumps by package,
    plus tracked edits and new non-ignored files. Any bump outside the approved
    set is an error."""
    changes = version_changes(runner, repo, "HEAD", None)
    unexpected = sorted(set(changes) - APPROVED_RELEASE_SET)
    tracked = _git_lines(runner, repo, "status", "--porcelain=v1", "--untracked-files=no")
    untracked = _git_lines(runner, repo, "ls-files", "--others", "--exclude-standard")
    return {
        "version_changes": {
            name: {"from": change.previous, "to": change.version, "manifest": change.manifest}
            for name, change in sorted(changes.items())
        },
        "unexpected_packages": unexpected,
        "proves_initial_release": set(changes) == APPROVED_RELEASE_SET,
        "tracked_changes": tracked,
        "untracked_not_ignored": untracked,
    }


def validate_release_pr_output(text: str) -> list[str]:
    """Packages a `release-plz release-pr --output json` proposal names. `{"prs":
    []}` is a valid no-op; any package outside the approved set is an error."""
    document = load_json(text, source="release-pr output")
    prs = _present(document, "prs")
    if not isinstance(prs, list):
        raise ReleasePathError("prs", f"must be a list, found {type(prs).__name__}")
    named: list[str] = []
    for index, pr in enumerate(prs):
        releases = _present(pr, "releases") if isinstance(pr, dict) else None
        if not isinstance(releases, list):
            raise ReleasePathError(f"prs[{index}].releases", "must be a list")
        for position, release in enumerate(releases):
            if not isinstance(release, dict):
                raise ReleasePathError(f"prs[{index}].releases[{position}]", "must be an object")
            named.append(req_str(release, "package_name"))
    unexpected = sorted(set(named) - APPROVED_RELEASE_SET)
    if unexpected:
        raise ReleasePathError("prs", f"proposal names packages outside the approved set: {unexpected}")
    return named


# ---------------------------------------------------------------------------
# Command entry points
# ---------------------------------------------------------------------------


def _escape(text: str) -> str:
    """Workflow-command escaping, so event-derived text cannot start a second
    command or end an annotation early."""
    return text.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")


def annotate(kind: str, title: str, message: str) -> None:
    print(f"::{kind} title={_escape(title).replace(':', '%3A').replace(',', '%2C')}::{_escape(message)}")


def _write_outputs(path: str | None, outputs: dict[str, str]) -> None:
    for key, value in outputs.items():
        if "\n" in value or "\r" in value:
            raise ReleasePathError(f"output {key}", "must be a single line")
    if path:
        with open(path, "a", encoding="utf-8") as handle:
            for key, value in outputs.items():
                handle.write(f"{key}={value}\n")


def _append_summary(text: str) -> None:
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as handle:
            handle.write(text)
    sys.stdout.write(text)


def _command_select(args: argparse.Namespace, env: dict[str, str], runner: Runner) -> int:
    event = load_event(Path(env.get("GITHUB_EVENT_PATH", "")))
    event_name = env.get("GITHUB_EVENT_NAME", "")
    repository = env.get("GITHUB_REPOSITORY", "")
    repo = args.repo.resolve()
    if args.path == "prepare":
        selection = select_preparation(event_name, repository, event)
        if selection.selected:
            selection = verify_freshness(runner, repo, selection.outputs["validated_sha"])
        if selection.selected:
            text = _read_revision(runner, repo, None, "release-plz.toml")
            if text is None:
                raise ReleasePathError("release-plz.toml", "is absent from the checkout")
            load_release_config(text)
            require_app_configuration(env, need_bot_login=True)
    else:
        selection = select_publication(event_name, repository, event, env.get("RELEASE_APP_BOT_LOGIN", "").strip() or None)
        if selection.selected:
            outputs = dict(selection.outputs)
            outputs["base_sha"] = verify_publication_topology(runner, repo, outputs["merge_sha"], outputs["head_sha"])
            load_release_config(_git(runner, repo, "show", f"{outputs['head_sha']}:release-plz.toml"),
                                source=f"{outputs['head_sha']}:release-plz.toml")
            require_app_configuration(env, need_bot_login=True)
            selection = Selection(True, selection.reason, outputs)
    if not selection.selected:
        annotate("notice", "release skipped", selection.reason)
    _write_outputs(env.get("GITHUB_OUTPUT"), {"selected": str(selection.selected).lower(), **selection.outputs})
    print(json.dumps({"selected": selection.selected, "reason": selection.reason, **selection.outputs}, sort_keys=True))
    return 0


def _forge(args: argparse.Namespace, env: dict[str, str], runner: Runner, token_var: str) -> GitHubForge:
    token = env.get(token_var) or None
    git_env = credential_env(_git_env(), token=token) if token else _git_env()
    return GitHubForge(repository=env.get("GITHUB_REPOSITORY", CANONICAL_REPOSITORY), token=token,
                       repo=args.repo.resolve(), runner=runner, git_env=git_env)


def _publication_context(args: argparse.Namespace, env: dict[str, str]) -> dict[str, str]:
    server = env.get("GITHUB_SERVER_URL", "https://github.com")
    return {
        "workflow run": f"{server}/{env.get('GITHUB_REPOSITORY', '')}/actions/runs/{env.get('GITHUB_RUN_ID', '?')} "
                        f"(attempt {env.get('GITHUB_RUN_ATTEMPT', '?')})",
        "merge commit": args.merge_sha,
        "proposal final commit (every tag's target)": args.head_sha,
        "recovery": "re-run this workflow run; it is bound to this merge commit",
    }


def _write_report(path: Path | None, payload: dict[str, Any]) -> None:
    if path is not None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def _command_inventory(args: argparse.Namespace, env: dict[str, str], runner: Runner) -> int:
    repo = args.repo.resolve()
    candidates = publication_candidates(runner, repo, args.base_sha, args.head_sha)
    inventory(candidates, _forge(args, env, runner, args.token_env), args.head_sha)
    context = _publication_context(args, env)
    _write_report(args.output, {"context": context, "candidates": [c.as_dict() for c in candidates]})
    _append_summary(render_inventory(args.title, candidates, context=context))
    errors = [candidate.error for candidate in candidates if candidate.error]
    for error in errors:
        annotate("error", "release inventory", error)
    return 1 if errors else 0


def _bot_login(env: dict[str, str]) -> str:
    """The App bot login a release-plz process commits as. Refused before any
    API request, so an empty login never becomes a `GET /users/` lookup."""
    login = env.get("RELEASE_APP_BOT_LOGIN", "").strip()
    if not login:
        raise ReleasePathError(
            "RELEASE_APP_BOT_LOGIN",
            "repository variable is not set; set it to the release App's bot login, "
            "including the [bot] suffix (see docs/releases.md)",
        )
    return login


def _command_publish(args: argparse.Namespace, env: dict[str, str], runner: Runner) -> int:
    repo = args.repo.resolve()
    token = env.get("GIT_TOKEN", "")
    if not token:
        raise ReleasePathError("GIT_TOKEN", "the App installation token is missing")
    login = _bot_login(env)
    config = load_release_config(_git(runner, repo, "show", f"{args.head_sha}:release-plz.toml"),
                                 source=f"{args.head_sha}:release-plz.toml")
    candidates = publication_candidates(runner, repo, args.base_sha, args.head_sha)
    forge = _forge(args, env, runner, "GIT_TOKEN")
    release_env = automation_env(dict(env), token=token, bot_login=login, bot_id=forge.user_id(login))
    args.workdir.mkdir(parents=True, exist_ok=True)
    result = publish(runner=runner, forge=forge, repo=repo, workdir=args.workdir, config=config,
                     candidates=candidates, head_sha=args.head_sha, env=release_env,
                     repository=env.get("GITHUB_REPOSITORY", CANONICAL_REPOSITORY))
    context = _publication_context(args, env)
    _write_report(args.output, {"context": context, "exit_status": result.exit_status, "failures": result.failures,
                                "candidates": [c.as_dict() for c in result.candidates]})
    _append_summary(render_inventory("Release publication inventory", result.candidates, context=context))
    for failure in result.failures:
        annotate("error", "release publication", failure)
    return result.exit_status


def _command_release_pr(args: argparse.Namespace, env: dict[str, str], runner: Runner) -> int:
    repo = args.repo.resolve()
    token = env.get("GIT_TOKEN", "")
    if not token:
        raise ReleasePathError("GIT_TOKEN", "the App installation token is missing")
    login = _bot_login(env)
    repository = env.get("GITHUB_REPOSITORY", CANONICAL_REPOSITORY)
    forge = _forge(args, env, runner, "GIT_TOKEN")
    bot_id = forge.user_id(login)
    with calculation_environment(dict(env)) as calculation:
        release_env = automation_env(calculation, token=token, bot_login=login, bot_id=bot_id)
        completed = runner.run(
            ["release-plz", "release-pr", "--config", "release-plz.toml", "--forge", "github",
             "--repo-url", f"https://github.com/{repository}", "--output", "json"],
            cwd=repo, env=release_env,
        )
    sys.stderr.write(completed.stderr)
    if args.output is not None:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(completed.stdout, encoding="utf-8")
    if completed.returncode != 0:
        annotate("error", "release proposal", f"release-plz release-pr exited {completed.returncode}")
        return completed.returncode
    named = validate_release_pr_output(completed.stdout.strip() or '{"prs": []}')
    _append_summary(f"### Release proposal\n\n- packages: {', '.join(sorted(named)) or 'none (nothing to release)'}\n")
    return 0


def _command_update(args: argparse.Namespace, env: dict[str, str], runner: Runner) -> int:
    """The dry run's calculation: `release-plz update`, edits left in place.
    No credential is added; its exit status is the command's."""
    with calculation_environment(dict(env)) as calculation:
        completed = runner.run(["release-plz", "update", "--config", "release-plz.toml"],
                               cwd=args.repo.resolve(), env=calculation)
    sys.stdout.write(completed.stdout)
    sys.stderr.write(completed.stderr)
    if completed.returncode != 0:
        annotate("error", "release calculation", f"release-plz update exited {completed.returncode}")
    return completed.returncode


def _command_calculation_report(args: argparse.Namespace, env: dict[str, str], runner: Runner) -> int:
    report = calculation_report(runner, args.repo.resolve())
    _write_report(args.output, report)
    lines = ["### Release calculation result", ""]
    for name, change in report["version_changes"].items():
        lines.append(f"- `{name}`: {change['from']} → {change['to']}")
    if not report["version_changes"]:
        lines.append("- no package version changed; this run does not prove the initial two-package proposal")
    lines += [f"  - tracked: `{line}`" for line in report["tracked_changes"]]
    lines += [f"  - new: `{line}`" for line in report["untracked_not_ignored"]]
    _append_summary("\n".join(lines) + "\n")
    if report["unexpected_packages"]:
        annotate("error", "release calculation", f"bumped packages outside the approved set: {report['unexpected_packages']}")
        return 1
    return 0


def _command_diagnose(args: argparse.Namespace, env: dict[str, str], runner: Runner) -> int:
    try:
        report = collect_diagnostics(
            args.repo,
            probe_path=args.probe_path,
            historical_revision=args.historical_revision,
        )
    except NotAGitCheckout as error:
        print(f"release diagnostics: {error}", file=sys.stderr)
        return 2
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    summary = render_summary(report)
    if args.summary is not None:
        with args.summary.open("a", encoding="utf-8") as handle:
            handle.write(summary)
    sys.stdout.write(summary)
    return 0


def _parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(prog="release_path.py", description=__doc__.splitlines()[0])
    commands = parser.add_subparsers(dest="command", required=True)

    diagnose = commands.add_parser("diagnose", help="write a read-only release diagnostic report")
    diagnose.add_argument("--repo", type=Path, default=Path.cwd())
    diagnose.add_argument("--output", type=Path, required=True, help="JSON report path")
    diagnose.add_argument("--summary", type=Path, help="append a Markdown digest here")
    diagnose.add_argument("--probe-path", default=DEFAULT_PROBE_PATH)
    diagnose.add_argument("--historical-revision", default=DEFAULT_HISTORICAL_REVISION)

    select = commands.add_parser("select", help="decide whether this event prepares or publishes a release")
    select.add_argument("path", choices=["prepare", "publish"])
    select.add_argument("--repo", type=Path, default=Path.cwd())

    for name, help_text in (
        ("inventory", "report every candidate's remote tag and Release state"),
        ("publish", "publish or recover every candidate from the proposal's final commit"),
    ):
        command = commands.add_parser(name, help=help_text)
        command.add_argument("--repo", type=Path, default=Path.cwd())
        command.add_argument("--merge-sha", required=True)
        command.add_argument("--head-sha", required=True)
        command.add_argument("--base-sha", required=True)
        command.add_argument("--output", type=Path)
        if name == "inventory":
            command.add_argument("--title", default="Release inventory")
            command.add_argument("--token-env", default="GITHUB_TOKEN", help="environment variable holding the API token")
        else:
            command.add_argument("--workdir", type=Path, required=True, help="disposable clones and configuration")

    release_pr = commands.add_parser("release-pr", help="open or update the release proposal with the App token")
    release_pr.add_argument("--repo", type=Path, default=Path.cwd())
    release_pr.add_argument("--output", type=Path)

    update = commands.add_parser("update", help="run release-plz update in the release calculation environment")
    update.add_argument("--repo", type=Path, default=Path.cwd())

    report = commands.add_parser("calculation-report", help="summarize what a local release-plz update changed")
    report.add_argument("--repo", type=Path, default=Path.cwd())
    report.add_argument("--output", type=Path)
    return parser.parse_args(argv)


_COMMANDS = {
    "diagnose": _command_diagnose,
    "select": _command_select,
    "inventory": _command_inventory,
    "publish": _command_publish,
    "release-pr": _command_release_pr,
    "update": _command_update,
    "calculation-report": _command_calculation_report,
}


def main(argv: Sequence[str] | None = None, *, env: dict[str, str] | None = None, runner: Runner | None = None) -> int:
    args = _parse_args(sys.argv[1:] if argv is None else argv)
    for name in ("merge_sha", "head_sha", "base_sha"):
        value = getattr(args, name, None)
        if value is not None and not _SHA.match(value):
            print(f"release path: --{name.replace('_', '-')} must be a full 40-digit commit SHA", file=sys.stderr)
            return 2
    try:
        return _COMMANDS[args.command](args, dict(os.environ) if env is None else env, runner or Runner())
    except ReleasePathError as error:
        annotate("error", "release path", str(error))
        return 1


if __name__ == "__main__":
    sys.exit(main())

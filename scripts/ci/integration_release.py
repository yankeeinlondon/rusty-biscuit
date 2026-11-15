#!/usr/bin/env python3
"""Select the integration a published GitHub Release belongs to.

`build-integrations.yml` builds an integration archive only for a Release whose
tag is exactly `{package}-v{semver}` for one of the integration packages
below. Any other Release (a CLI, a library, a malformed or misleading tag)
selects nothing, and the build job never starts. The selection is a matrix of
zero or one entries, and the tag's resolved commit is what the build checks
out.

Event data is read from `GITHUB_EVENT_PATH`, never interpolated into shell.
The operational guide is `docs/releases.md`.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Sequence

sys.path.insert(0, str(Path(__file__).resolve().parent))

from release_path import (  # noqa: E402
    CANONICAL_REPOSITORY,
    SEMVER,
    ReleasePathError,
    annotate,
    load_event,
    peeled_tag_target,
    req_str,
)

# Package name → package directory. The archive name and build target stay
# those the workflow has always used.
INTEGRATIONS = {
    "arcam-amp-integration": "homelab/arcam-amp-integration",
    "eversolo-integration": "homelab/eversolo-integration",
    "sony-receiver-integration": "homelab/sony-receiver-integration",
}

_TAG = re.compile(
    rf"^(?P<package>{'|'.join(re.escape(name) for name in INTEGRATIONS)})-v(?P<version>{SEMVER})$"
)


@dataclass(frozen=True)
class IntegrationSelection:
    """`package` None is a deliberate, successful no-op with a `reason`."""

    reason: str
    package: str | None = None
    package_dir: str | None = None
    tag: str | None = None
    version: str | None = None

    def matrix(self) -> dict[str, list[dict[str, str]]]:
        if self.package is None:
            return {"include": []}
        return {"include": [{"package": self.package, "package_dir": self.package_dir, "tag": self.tag}]}


def select_integration(event_name: str, repository: str, event: dict[str, Any]) -> IntegrationSelection:
    """The integration a `release: published` event's exact tag names."""
    if event_name != "release":
        return IntegrationSelection(f"event {event_name!r} is not a published release")
    if repository != CANONICAL_REPOSITORY:
        return IntegrationSelection(f"repository {repository!r} is not {CANONICAL_REPOSITORY}")
    action = req_str(event, "action")
    if action != "published":
        return IntegrationSelection(f"release action {action!r} is not published")
    tag = req_str(event, "release.tag_name")
    match = _TAG.fullmatch(tag)
    if match is None:
        return IntegrationSelection(f"tag {tag!r} is not an integration release")
    package = match["package"]
    return IntegrationSelection(
        "integration release",
        package=package,
        package_dir=INTEGRATIONS[package],
        tag=tag,
        version=match["version"],
    )


def resolve_tag_commit(repo: Path, tag: str) -> str:
    """The commit the remote tag names, peeled through an annotated tag."""
    ref = f"refs/tags/{tag}"
    completed = subprocess.run(
        ["git", "ls-remote", "--tags", "origin", ref, f"{ref}^{{}}"],
        cwd=repo,
        env={**os.environ, "GIT_TERMINAL_PROMPT": "0"},
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        raise ReleasePathError(f"ls-remote {tag}", completed.stderr.strip() or f"exit {completed.returncode}")
    target = peeled_tag_target(completed.stdout, tag)
    if target is None:
        raise ReleasePathError(f"release.tag_name {tag}", "the tag does not exist on origin")
    return target


def _write_outputs(path: str | None, outputs: dict[str, str]) -> None:
    if path:
        with open(path, "a", encoding="utf-8") as handle:
            for key, value in outputs.items():
                handle.write(f"{key}={value}\n")


def main(argv: Sequence[str] | None = None, *, env: dict[str, str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="integration_release.py", description=__doc__.splitlines()[0])
    commands = parser.add_subparsers(dest="command", required=True)
    select = commands.add_parser("select", help="emit the zero-or-one integration build matrix")
    select.add_argument("--repo", type=Path, default=Path.cwd())
    select.add_argument("--no-resolve", action="store_true", help="skip resolving the tag's commit")
    args = parser.parse_args(sys.argv[1:] if argv is None else argv)
    env = dict(os.environ) if env is None else env
    try:
        event = load_event(Path(env.get("GITHUB_EVENT_PATH", "")))
        selection = select_integration(env.get("GITHUB_EVENT_NAME", ""), env.get("GITHUB_REPOSITORY", ""), event)
        outputs = {"selected": "false", "matrix": json.dumps(selection.matrix(), separators=(",", ":"))}
        if selection.package is None:
            annotate("notice", "integration build skipped", selection.reason)
        else:
            outputs["selected"] = "true"
            outputs["package"] = selection.package
            outputs["tag"] = selection.tag or ""
            if not args.no_resolve:
                outputs["commit"] = resolve_tag_commit(args.repo, selection.tag or "")
    except ReleasePathError as error:
        annotate("error", "integration release", str(error))
        return 1
    _write_outputs(env.get("GITHUB_OUTPUT"), outputs)
    print(json.dumps(outputs, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())

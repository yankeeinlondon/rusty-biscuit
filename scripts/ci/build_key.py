#!/usr/bin/env python3
"""The one hashing boundary the planner computes build keys through.

`fixes/2026-09-12-single-os-compile/spec.md` defines the planned build key as
"a canonical digest of every plan-known compile-affecting input". The producer's
realized manifest, the consumer's verification, and the counter's event files
all digest through `biscuit-hash`'s xxHash; a planner that reached for
`hashlib` or Python's `hash()` would put a second implementation inside one
contract, and two implementations of one digest is how "the consumer ran the
producer's binaries" silently stops being checkable.

So this module shells out to the `ci-build key` subcommand and has no fallback.
An unavailable helper raises rather than degrading to another digest.

## Notes

Canonicalization stays on this side: [`schema.canonical`] is the serializer
every CI document already round-trips through, and asking Python and Rust to
independently agree on key order, float spelling, and non-ASCII escaping is the
other way two "identical" digests drift. The helper hashes the bytes it is
given.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any, Sequence

sys.path.insert(0, str(Path(__file__).resolve().parent))

import schema  # noqa: E402


#: Matches `ci-build.rs::KEY_SCHEMA_VERSION`. A mismatch is reported by the
#: helper, not tolerated here.
KEY_SCHEMA_VERSION = 1

#: Points at an already-built `ci-build`. CI and the pre-push hook set it so
#: neither pays for a Cargo invocation to learn a digest.
HELPER_ENV = "BISCUIT_CI_BUILD_BIN"

_SCRIPTS_MANIFEST = ("scripts", "Cargo.toml")

#: This repository's root. The helper lives beside this module, never inside
#: the workspace being planned: a synthetic fixture root has no `scripts/`, and
#: resolving the tool there would make the digest depend on where the plan is.
ROOT = Path(__file__).resolve().parents[2]

#: Resolved once per process: the helper's location does not change mid-run and
#: `calculate_scope` digests a batch for every plan a test suite resolves.
_COMMAND: list[str] | None = None


def _candidates() -> list[Path]:
    suffix = ".exe" if os.name == "nt" else ""
    # `scripts` is a member of the root workspace, so its binaries land in the
    # root target directory. A build under `scripts/target` is pre-consolidation
    # residue: stale, gitignored, and never what the planner should digest with.
    target = ROOT / "target"
    return [target / profile / f"ci-build{suffix}" for profile in ("release", "debug")]


def helper_command() -> list[str]:
    """The argv prefix that runs `ci-build`, resolved once per process.

    ## Returns

    An explicit binary where one is already built, otherwise a `cargo run`
    invocation against `scripts/Cargo.toml`.

    ## Errors

    Raises ``RuntimeError`` when no binary exists and Cargo is absent. The
    planner fails loudly rather than emitting a plan whose build keys came from
    somewhere else.
    """
    global _COMMAND
    if _COMMAND is not None:
        return list(_COMMAND)

    override = os.environ.get(HELPER_ENV)
    if override:
        path = Path(override)
        if not path.is_file():
            raise RuntimeError(
                f"{HELPER_ENV} names '{override}', which is not a file; point it at a "
                "built ci-build binary or unset it"
            )
        _COMMAND = [str(path)]
        return list(_COMMAND)

    for candidate in _candidates():
        if candidate.is_file():
            _COMMAND = [str(candidate)]
            return list(_COMMAND)

    manifest = ROOT.joinpath(*_SCRIPTS_MANIFEST)
    if shutil.which("cargo") and manifest.is_file():
        # `--no-default-features --features build-tools` deliberately: the
        # default feature set pulls sniff's duckdb and gix closure, and
        # resolving a plan must not compile a database engine to answer a hash.
        _COMMAND = [
            "cargo",
            "run",
            "--quiet",
            "--manifest-path",
            str(manifest),
            "--no-default-features",
            "--features",
            "build-tools",
            "--bin",
            "ci-build",
            "--",
        ]
        return list(_COMMAND)

    raise RuntimeError(
        "the ci-build key helper is unavailable: no binary at "
        f"{', '.join(str(path) for path in _candidates())}, no {HELPER_ENV}, and no "
        "cargo to build one. Build it with `cargo build --manifest-path "
        "scripts/Cargo.toml --bin ci-build`. The planner has no second digest to "
        "fall back to."
    )


def reset_helper_cache() -> None:
    """Forget the resolved helper. For fixtures that move it mid-process."""
    global _COMMAND
    _COMMAND = None


def planned_keys(material: Sequence[str]) -> list[str]:
    """The planned build key of each canonical input string, in order.

    One helper invocation for the whole batch: a plan with twelve build records
    is one subprocess, not twelve.

    ## Errors

    Raises ``RuntimeError`` when the helper is unavailable, exits non-zero, or
    answers with another schema generation or a different number of keys.
    """
    if not material:
        return []
    command = helper_command()
    request = json.dumps(
        {"schema_version": KEY_SCHEMA_VERSION, "material": list(material)},
        separators=(",", ":"),
    )
    try:
        completed = subprocess.run(
            [*command, "key"],
            input=request,
            capture_output=True,
            text=True,
            encoding="utf-8",
            cwd=ROOT,
        )
    except OSError as error:
        raise RuntimeError(f"could not run the ci-build key helper: {error}") from error
    if completed.returncode != 0:
        raise RuntimeError(
            f"the ci-build key helper exited {completed.returncode}: "
            f"{(completed.stderr or '').strip()}"
        )
    # `cargo run` writes build progress to stderr, but a stale registry warning
    # can still reach stdout; the response is the last line.
    lines = [line for line in (completed.stdout or "").splitlines() if line.strip()]
    if not lines:
        raise RuntimeError("the ci-build key helper produced no output")
    try:
        response = json.loads(lines[-1])
    except json.JSONDecodeError as error:
        raise RuntimeError(
            f"the ci-build key helper answered with non-JSON: {lines[-1][:200]!r}"
        ) from error
    if response.get("schema_version") != KEY_SCHEMA_VERSION:
        raise RuntimeError(
            f"the ci-build key helper answered schema version "
            f"{response.get('schema_version')!r}, this planner reads {KEY_SCHEMA_VERSION}"
        )
    keys = response.get("keys")
    if not isinstance(keys, list) or len(keys) != len(material):
        raise RuntimeError(
            f"the ci-build key helper answered {len(keys) if isinstance(keys, list) else '?'} "
            f"key(s) for {len(material)} input(s)"
        )
    return keys


def planned_key(identity: dict[str, Any]) -> str:
    """The planned build key of one identity record."""
    return planned_keys([schema.canonical(identity)])[0]

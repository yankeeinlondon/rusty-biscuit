#!/usr/bin/env python3
"""Regenerate every shipped schema artifact, all or nothing.

Three crates check in the prepared form of their compiled-in schema sources,
each built over the one before it:

    simplified-schema -> darkmatter -> claudine

Each crate's `shipped_schemas` example, run without a mode flag, prepares
its whole dependency chain from current text without reading any checked-in
artifact, accepts its freshly generated artifact over the fresh lower
artifacts of that same generation (the full acceptance `--check` and
`--write` run: header, sources, dependencies, revision, and every payload
section), and only then prints it on stdout; a refusal exits 1 and prints
nothing. This script runs the three in dependency order and passes each one
the candidates already emitted below it as `--lower <owner>=<path>`, so a
generator also refuses when the lower artifact it validated against is not
the one that will be published beside it. Claudine's generator is also
passed `--lifecycle-output <path>`: it checks its lifecycle artifact as a
registration reader loads it and writes the candidate there, and the script
stages it with the shipped artifacts. Only when every output is accepted are
they written to sibling temporary files and renamed over the checked-in
files; any refusal leaves every file as it was and no temporary file
behind. Generation never reads a checked-in file, so it runs over stale or
malformed ones. Nothing is checked after publication: the crates' freshness
tests are the standing check that the published files match generation.

Usage (from anywhere in the repository; `just shipped-schemas` runs it):

    python3 scripts/shipped_schemas.py

`--root` and `--generator` exist for this script's contract tests: `--root`
names the tree the artifact paths are relative to, and `--generator` replaces
`cargo run --quiet -p <package> --example shipped_schemas --` with another
program (a `.py` file runs under this interpreter), called as
`<program> <package> [--lower <owner>=<path>]... [--lifecycle-output <path>]`.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

# (package, owner in the artifact header, repository-relative artifact path),
# in dependency order.
ARTIFACTS = (
    ("simplified-schema", "simplified-schema", "simplified-schema/lib/generated/shipped.json"),
    ("darkmatter", "darkmatter", "darkmatter/lib/generated/shipped.json"),
    ("claudine", "claudine", "claudine/lib/generated/shipped.json"),
)

# Lifecycle artifacts a package's generator emits beside its shipped
# artifact: package -> repository-relative path.
LIFECYCLE_ARTIFACTS = {
    "claudine": "claudine/lib/generated/globals.lifecycle.json",
}


class Refused(Exception):
    """A step failed; nothing has been replaced."""


def generator_command(generator: str | None, package: str, *extra: str) -> list[str]:
    if generator is None:
        return ["cargo", "run", "--quiet", "-p", package, "--example", "shipped_schemas", "--", *extra]
    if generator.endswith(".py"):
        return [sys.executable, generator, package, *extra]
    return [generator, package, *extra]


def generate(
    root: Path, generator: str | None, package: str, lowers: list[tuple[str, Path]], lifecycle: Path | None = None
) -> bytes:
    """Runs `package`'s generator over the lower candidates `lowers`, which it
    must accept its own candidate over; with `lifecycle`, the generator also
    writes its lifecycle candidate there."""
    extra = [arg for owner, path in lowers for arg in ("--lower", f"{owner}={path}")]
    if lifecycle is not None:
        extra += ["--lifecycle-output", str(lifecycle)]
    command = generator_command(generator, package, *extra)
    completed = subprocess.run(command, cwd=root, stdin=subprocess.DEVNULL, capture_output=True)
    if completed.returncode != 0:
        detail = completed.stderr.decode("utf-8", "replace").strip()
        raise Refused(f"generating `{package}`'s artifact failed (exit {completed.returncode}):\n{detail}")
    return completed.stdout


def check_output(owner: str, path: str, data: bytes, lower_owners: list[str]) -> None:
    """Refuses output that cannot be the named owner's artifact."""
    if not data:
        raise Refused(f"`{path}`: the generator printed nothing")
    if not data.endswith(b"\n") or data.endswith(b"\n\n"):
        raise Refused(f"`{path}`: an artifact ends with exactly one line feed")
    try:
        document = json.loads(data.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise Refused(f"`{path}`: the generator printed something other than JSON: {error}") from error
    header = document.get("header") if isinstance(document, dict) else None
    if not isinstance(header, dict) or not isinstance(document.get("payload"), dict):
        raise Refused(f"`{path}`: the output has no `header` and `payload` objects")
    if header.get("owner") != owner:
        raise Refused(f"`{path}`: the header names owner {header.get('owner')!r}, not {owner!r}")
    dependencies = header.get("dependencies")
    named = [dependency.get("owner") for dependency in dependencies or [] if isinstance(dependency, dict)]
    if not isinstance(dependencies, list) or sorted(named) != sorted(lower_owners):
        raise Refused(f"`{path}`: the header's dependencies are {named!r}, expected {lower_owners!r}")


def check_lifecycle(path: str, candidate: Path) -> bytes:
    """Refuses a lifecycle candidate that is missing or cannot be one."""
    if not candidate.exists():
        raise Refused(f"`{path}`: the generator wrote no lifecycle candidate")
    data = candidate.read_bytes()
    if not data:
        raise Refused(f"`{path}`: the lifecycle candidate is empty")
    try:
        document = json.loads(data.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise Refused(f"`{path}`: the lifecycle candidate is not JSON: {error}") from error
    if not isinstance(document, dict) or not isinstance(document.get("catalog_hash"), str):
        raise Refused(f"`{path}`: the lifecycle candidate has no `catalog_hash`")
    return data


def replace_all(root: Path, outputs: list[tuple[str, bytes]]) -> list[str]:
    """Stages every output beside its file, then renames each into place.

    Returns the paths whose content changed.
    """
    staged: list[tuple[Path, Path]] = []
    try:
        for path, data in outputs:
            target = root / path
            temporary = target.with_name(target.name + ".tmp")
            temporary.write_bytes(data)
            staged.append((temporary, target))
    except OSError as error:
        for temporary, _ in staged:
            temporary.unlink(missing_ok=True)
        raise Refused(f"cannot stage the new artifacts: {error}") from error
    changed = []
    for (temporary, target), (path, data) in zip(staged, outputs):
        if target.exists() and target.read_bytes() == data:
            temporary.unlink()
            continue
        os.replace(temporary, target)
        changed.append(path)
    return changed


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Regenerate every shipped schema artifact.")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--generator")
    arguments = parser.parse_args(argv)
    root: Path = arguments.root

    try:
        outputs = []
        with tempfile.TemporaryDirectory(prefix="shipped-schemas-") as scratch:
            candidates: list[tuple[str, Path]] = []
            for package, owner, path in ARTIFACTS:
                lifecycle_path = LIFECYCLE_ARTIFACTS.get(package)
                lifecycle = Path(scratch) / f"{owner}.lifecycle.json" if lifecycle_path else None
                data = generate(root, arguments.generator, package, candidates, lifecycle)
                check_output(owner, path, data, [lower for lower, _ in candidates])
                candidate = Path(scratch) / f"{owner}.json"
                candidate.write_bytes(data)
                candidates.append((owner, candidate))
                outputs.append((path, data))
                if lifecycle_path and lifecycle is not None:
                    outputs.append((lifecycle_path, check_lifecycle(lifecycle_path, lifecycle)))
        changed = replace_all(root, outputs)
    except Refused as refusal:
        print(f"error: {refusal}", file=sys.stderr)
        return 1
    for path in changed:
        print(f"regenerated {path}")
    if not changed:
        print("every shipped artifact is current")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

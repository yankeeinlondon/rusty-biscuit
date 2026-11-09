#!/usr/bin/env python3
"""Append one storage observation of the build owner to a JSONL file.

The Ubuntu build owner compiles every planned record into one shared Cargo
target directory and keeps each completed archive until publication. When the
runner's disk fills, the job log is lost with it, so this diagnostic writes a
small JSON line at each boundary the build job names (cache restore, the
schematic fresh-output guard, every record attempt, publication) and the job
uploads the file under `if: always()`. Earlier lines survive a later failure.

Each line keeps its categories apart and never sums them into a free-space
figure: the archives and the target directory may share one filesystem, and
the schematic guard's output sits inside the target directory (`overlaps`).
Allocated bytes are counted (`st_blocks`), not apparent sizes, because free
space is what the categories compete for. Every measurement that cannot be
taken is recorded as `{"unavailable": "<reason>"}` rather than as zero.

A sample is a boundary observation, never a claim about the instantaneous peak
reached while linking or compressing between two boundaries.

## Examples

```bash
python3 scripts/ci/storage_sample.py --out "$RUNNER_TEMP/build-diagnostics/storage.jsonl" \
    --label after-cache-restore --target target --output "$RUNNER_TEMP/build" \
    --cache-hit "$CACHE_HIT" --inventory --restored-since before-cache-restore \
    --plan ci-artifacts/ci-resolved-plan/resolved-plan.json --producer ubuntu-latest \
    --live-path release
```

## Exit status

`0` when the line was written, even if some measurements in it are
unavailable; `1` when no line could be written; `2` for a usage error. Callers
ignore the status: a diagnostic never replaces the build's own result.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import sys
import time
from pathlib import Path
from typing import Any, Callable

SCHEMA = "storage-sample/1"

#: Cargo profile directory for a profile name, as `ci-build`'s `profile_dir`.
PROFILE_DIRS = {"test": "debug", "dev": "debug", "bench": "release"}

#: Where the schematic fresh-output guard's child builds share their target.
#: `schematic/gen/tests/common/fresh_output.rs::child_target_dir` puts it beside
#: the outer build's profile directories, i.e. directly inside the target dir
#: for a host build. Its generated crates live in the temp location and are
#: removed by `TempDir` when each test ends.
SCHEMATIC_CHILD_TARGET = "schematic-fresh-output"


def nearest_existing(path: Path) -> Path:
    """`path`, or its closest existing ancestor, for filesystem statistics."""
    candidate = path
    while not candidate.exists():
        parent = candidate.parent
        if parent == candidate:
            return candidate
        candidate = parent
    return candidate


def filesystem(path: Path, statvfs: Callable[[str], Any] | None = None) -> dict[str, Any]:
    """Free bytes and inodes of the filesystem holding `path`.

    A missing `path` is measured at its nearest existing ancestor, which is
    reported as `probed`. `statvfs` is injectable so a failed probe can be
    tested without a broken filesystem.
    """
    probed = nearest_existing(path)
    result: dict[str, Any] = {"path": str(path), "probed": str(probed), "exists": path.exists()}
    try:
        result["device"] = os.stat(probed).st_dev
    except OSError as error:
        result["device"] = {"unavailable": f"stat failed: {error}"}
    probe = statvfs if statvfs is not None else getattr(os, "statvfs", None)
    if probe is None:
        # Native Windows has no statvfs; byte counts still come from the OS.
        try:
            usage = shutil.disk_usage(probed)
            result["free_bytes"] = usage.free
            result["total_bytes"] = usage.total
        except OSError as error:
            result["free_bytes"] = {"unavailable": f"disk_usage failed: {error}"}
        result["free_inodes"] = {"unavailable": "statvfs is not available on this platform"}
        return result
    try:
        stats = probe(str(probed))
    except OSError as error:
        reason = {"unavailable": f"statvfs failed: {error}"}
        result.update(free_bytes=reason, free_inodes=reason)
        return result
    result["free_bytes"] = stats.f_bavail * stats.f_frsize
    result["total_bytes"] = stats.f_blocks * stats.f_frsize
    result["free_inodes"] = stats.f_favail
    result["total_inodes"] = stats.f_files
    return result


def allocated(path: Path) -> dict[str, Any]:
    """Allocated bytes and file count under `path`, never following symlinks.

    A hard-linked file is counted once. Entries that vanish or cannot be read
    during the walk are counted in `errors` rather than aborting the sample;
    the figure is then a lower bound, which `errors > 0` makes visible.
    """
    if not path.exists() and not path.is_symlink():
        return {"path": str(path), "exists": False, "allocated_bytes": 0, "files": 0, "errors": 0}
    seen: set[tuple[int, int]] = set()
    total = files = errors = 0
    apparent = not hasattr(os.stat_result, "st_blocks")
    stack = [path]
    while stack:
        current = stack.pop()
        try:
            info = os.lstat(current)
        except OSError:
            errors += 1
            continue
        if info.st_nlink > 1:
            identity = (info.st_dev, info.st_ino)
            if identity in seen:
                continue
            seen.add(identity)
        total += info.st_size if apparent else info.st_blocks * 512
        if os.path.isdir(current) and not os.path.islink(current):
            try:
                with os.scandir(current) as entries:
                    stack.extend(Path(entry.path) for entry in entries)
            except OSError:
                errors += 1
        else:
            files += 1
    result: dict[str, Any] = {
        "path": str(path),
        "exists": True,
        "allocated_bytes": total,
        "files": files,
        "errors": errors,
    }
    if apparent:
        result["measure"] = "apparent"
    return result


def measured(path: Path) -> dict[str, Any]:
    try:
        return allocated(path)
    except Exception as error:  # a diagnostic records, it never aborts
        return {"path": str(path), "unavailable": f"walk failed: {error}"}


def output_categories(output: Path) -> dict[str, list[Path]]:
    """Split the producer's output directory into its owned categories.

    `{artifact}-inventory` directories are inventory scratch, `*-sidecars`
    directories and archive files are retained record payloads, and everything
    else (manifests, statuses, generated Nextest configs) is metadata.
    """
    split: dict[str, list[Path]] = {"archives": [], "inventory": [], "metadata": []}
    try:
        entries = sorted(output.iterdir())
    except OSError:
        return split
    for entry in entries:
        name = entry.name
        if entry.is_dir() and name.endswith("-inventory"):
            split["inventory"].append(entry)
        elif entry.is_dir() and name.endswith("-sidecars"):
            split["archives"].append(entry)
        elif name.endswith((".tar.zst", ".tar.gz", ".zip")):
            split["archives"].append(entry)
        else:
            split["metadata"].append(entry)
    return split


def category(paths: list[Path], root: Path, overlaps: list[str]) -> dict[str, Any]:
    total = files = errors = 0
    unavailable = []
    for path in paths:
        sample = measured(path)
        if "unavailable" in sample:
            unavailable.append(sample["unavailable"])
            continue
        total += sample["allocated_bytes"]
        files += sample["files"]
        errors += sample["errors"]
    result: dict[str, Any] = {
        "root": str(root),
        "entries": len(paths),
        "allocated_bytes": total,
        "files": files,
        "errors": errors,
        "overlaps": overlaps,
    }
    if unavailable:
        result["unavailable"] = unavailable
    return result


def inventory(target: Path) -> dict[str, int]:
    """Allocated bytes of each target entry two levels deep (`a` and `a/b`).

    Two levels name both a host profile (`debug`) and a cross build's
    `{triple}/{profile}`, which is the granularity restored-entry
    classification needs.
    """
    entries: dict[str, int] = {}
    try:
        top = sorted(target.iterdir())
    except OSError:
        return entries
    for first in top:
        sample = measured(first)
        entries[first.name] = sample.get("allocated_bytes", 0)
        if first.is_dir() and not first.is_symlink():
            try:
                children = sorted(first.iterdir())
            except OSError:
                continue
            for second in children:
                if second.is_dir() and not second.is_symlink():
                    entries[f"{first.name}/{second.name}"] = measured(second).get("allocated_bytes", 0)
    return entries


def live_paths(plan: dict[str, Any], producer: str, extra: list[str]) -> list[str]:
    """Target-relative directories this owner's planned identities build into.

    An identity with a target triple writes `{triple}/{profile}` and its host
    build scripts and proc-macros under `{profile}`; one without writes
    `{profile}` only.
    """
    live = set(extra)
    for record in plan.get("builds", []):
        if record.get("producer") != producer:
            continue
        identity = record.get("identity", {})
        profile = str(identity.get("profile", ""))
        profile = PROFILE_DIRS.get(profile, profile)
        if not profile:
            continue
        live.add(profile)
        triple = str(identity.get("target", ""))
        if triple:
            live.add(f"{triple}/{profile}")
    return sorted(live)


def is_profile_dir(path: Path) -> bool:
    return (path / ".fingerprint").is_dir()


def classify_restored(
    before: dict[str, int], after: dict[str, int], target: Path, live: list[str]
) -> dict[str, Any]:
    """Which restored target entries are provably obsolete for this owner.

    Restored means present in `after` and absent from `before`. An entry is
    obsolete only when it is a Cargo profile directory (it holds `.fingerprint`)
    at `{profile}` or `{triple}/{profile}` that no planned identity or
    `--live-path` names. A live entry, and anything inside one, is `live`;
    every other restored entry is `unknown`, never recoverable merely because
    it is large.
    """
    restored = [name for name in after if name not in before]
    live_set = set(live)
    obsolete: dict[str, int] = {}
    live_found: dict[str, int] = {}
    unknown: dict[str, int] = {}

    def judge(name: str) -> None:
        if name in live_set or name.split("/", 1)[0] in live_set:
            live_found[name] = after[name]
        elif is_profile_dir(target.joinpath(*name.split("/"))):
            obsolete[name] = after[name]
        else:
            unknown[name] = after[name]

    for name in restored:
        if "/" not in name:
            path = target / name
            profiles = [
                child for child in after
                if child.startswith(f"{name}/") and is_profile_dir(target.joinpath(*child.split("/")))
            ]
            if name in live_set or is_profile_dir(path) or not profiles:
                judge(name)
            else:
                # A triple directory: each of its profiles is judged alone, and
                # whatever else it holds stays unknown.
                for child in profiles:
                    judge(child)
                rest = after[name] - sum(after[child] for child in profiles)
                if rest > 0:
                    unknown[name] = rest
        elif name.split("/", 1)[0] not in restored:
            judge(name)
    return {
        "live_paths": live,
        "obsolete": obsolete,
        "obsolete_bytes": sum(obsolete.values()),
        "live": live_found,
        "unknown": unknown,
    }


def previous_line(out: Path, label: str) -> dict[str, Any] | None:
    """The most recent earlier sample in `out` carrying `label`."""
    found = None
    try:
        with out.open(encoding="utf-8") as handle:
            for line in handle:
                try:
                    document = json.loads(line)
                except ValueError:
                    continue
                if document.get("label") == label:
                    found = document
    except OSError:
        return None
    return found


def first_monotonic(out: Path) -> float | None:
    try:
        with out.open(encoding="utf-8") as handle:
            for line in handle:
                try:
                    value = json.loads(line).get("monotonic_s")
                except ValueError:
                    continue
                if isinstance(value, (int, float)):
                    return float(value)
    except OSError:
        return None
    return None


def sample(args: argparse.Namespace, statvfs: Callable[[str], Any] | None = None) -> dict[str, Any]:
    """Build one sample document from parsed arguments."""
    target = Path(args.target)
    output = Path(args.output)
    temp = Path(args.temp) if args.temp else Path(os.environ.get("TMPDIR") or "/tmp")
    now = time.monotonic()
    started = first_monotonic(Path(args.out))
    document: dict[str, Any] = {
        "schema": SCHEMA,
        "label": args.label,
        "record": args.record,
        "outcome": args.outcome,
        "cache_hit": args.cache_hit,
        "wall_epoch_s": round(time.time(), 3),
        "monotonic_s": round(now, 3),
        "elapsed_s": round(now - started, 3) if started is not None else 0.0,
        "platform": sys.platform,
    }
    locations = {
        name: filesystem(path, statvfs)
        for name, path in (("target", target), ("output", output), ("temp", temp))
    }
    document["locations"] = locations
    devices = {name: entry.get("device") for name, entry in locations.items()}
    document["shared_filesystem"] = {
        f"{left}+{right}": devices[left] == devices[right]
        if isinstance(devices[left], int) and isinstance(devices[right], int)
        else {"unavailable": "device identity was not measured"}
        for left, right in (("target", "output"), ("target", "temp"), ("output", "temp"))
    }
    split = output_categories(output)
    document["categories"] = {
        "target": category([target], target, []),
        "archives": category(split["archives"], output, []),
        "inventory": category(split["inventory"], output, []),
        "metadata": category(split["metadata"], output, []),
        "schematic_guard": category([target / SCHEMATIC_CHILD_TARGET], target, ["target"]),
    }
    if args.inventory:
        document["target_inventory"] = inventory(target)
    if args.restored_since:
        before = previous_line(Path(args.out), args.restored_since)
        after = document.get("target_inventory")
        if before is None or "target_inventory" not in before or after is None:
            document["restore"] = {
                "unavailable": f"no earlier '{args.restored_since}' sample with an inventory"
            }
        else:
            live: list[str] = list(args.live_path)
            if args.plan:
                try:
                    plan = json.loads(Path(args.plan).read_text(encoding="utf-8"))
                    live = live_paths(plan, args.producer or "", args.live_path)
                except (OSError, ValueError) as error:
                    document["restore"] = {"unavailable": f"plan unreadable: {error}"}
                    return document
            document["restore"] = classify_restored(before["target_inventory"], after, target, live)
    return document


def parser() -> argparse.ArgumentParser:
    parse = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    parse.add_argument("--out", required=True, help="JSONL file the sample is appended to")
    parse.add_argument("--label", required=True, help="the boundary this sample observes")
    parse.add_argument("--target", default="target", help="Cargo's target directory")
    parse.add_argument("--output", required=True, help="the producer's archive output directory")
    parse.add_argument("--temp", help="the temporary-file location (default $TMPDIR or /tmp)")
    parse.add_argument("--record", help="the record artifact this boundary follows")
    parse.add_argument("--outcome", help="the record's result and stage, e.g. failure:compile")
    parse.add_argument("--cache-hit", help="the cache step's cache-hit output, verbatim")
    parse.add_argument("--inventory", action="store_true", help="record a two-level target inventory")
    parse.add_argument("--restored-since", help="classify entries absent from this earlier sample")
    parse.add_argument("--plan", help="resolved plan, naming this owner's live target paths")
    parse.add_argument("--producer", help="this owner's environment in the plan")
    parse.add_argument("--live-path", action="append", default=[], help="another live target path")
    return parse


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    out = Path(args.out)
    try:
        document = sample(args)
    except Exception as error:  # a diagnostic records, it never aborts
        document = {"schema": SCHEMA, "label": args.label, "unavailable": f"sampling failed: {error}"}
    line = json.dumps(document, sort_keys=True)
    try:
        out.parent.mkdir(parents=True, exist_ok=True)
        with out.open("a", encoding="utf-8") as handle:
            handle.write(line + "\n")
            handle.flush()
    except OSError as error:
        print(f"storage-sample: could not write {out}: {error}", file=sys.stderr)
        return 1
    free = document.get("locations", {}).get("target", {}).get("free_bytes")
    target = document.get("categories", {}).get("target", {}).get("allocated_bytes")
    print(f"storage-sample {args.label} {args.record or ''} free={free} target={target}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())

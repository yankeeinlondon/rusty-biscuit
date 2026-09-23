#!/usr/bin/env python3
"""Collect dependency-planning evidence without changing manifests or the lockfile.

Python 3.11+ is required. Each invocation owns a fresh report directory; failed
commands retain raw output but never publish it as a successful JSON artifact.
"""

import argparse
from collections import defaultdict
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import platform
import subprocess
import tempfile
import tomllib


def write_json(path, value):
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, default=str) + "\n", encoding="utf-8")
    temporary.replace(path)


def run_command(root, output, name, argv, *, json_output=False, timeout=300):
    record = {"command": argv, "cwd": str(root), "timeout_seconds": timeout}
    stdout = output / f"{name}.stdout.txt"
    stderr = output / f"{name}.stderr.txt"
    record.update(stdout=stdout.name, stderr=stderr.name)
    try:
        with stdout.open("wb") as out, stderr.open("wb") as err:
            result = subprocess.run(argv, cwd=root, stdout=out, stderr=err, timeout=timeout)
        record.update(exit_status=result.returncode, status="ok" if result.returncode == 0 else "failed")
    except (OSError, subprocess.TimeoutExpired) as error:
        record.update(exit_status=None, status="failed", error=str(error))
    data = None
    if record["status"] == "ok" and json_output:
        try:
            data = json.loads(stdout.read_text(encoding="utf-8"))
            if not isinstance(data, dict):
                raise ValueError("expected a JSON object")
            if name.startswith("metadata-"):
                if not isinstance(data.get("packages"), list) or not isinstance(data.get("workspace_members"), list):
                    raise ValueError("missing Cargo metadata package/member arrays")
                if name != "metadata-declarations" and not isinstance(data.get("resolve"), dict):
                    raise ValueError("missing Cargo resolved graph")
            artifact = output / f"{name}.json"
            write_json(artifact, data)
            record["artifact"] = artifact.name
        except (ValueError, UnicodeError) as error:
            record.update(status="failed", error=f"Invalid JSON artifact: {error}")
            data = None
    return record, data


def declarations(metadata):
    members = set(metadata["workspace_members"])
    return [dict(dep, package_id=pkg["id"], package_name=pkg["name"],
                 manifest_path=pkg["manifest_path"], kind=dep.get("kind") or "normal")
            for pkg in metadata["packages"] if pkg["id"] in members
            for dep in pkg["dependencies"]]


def summarize(metadata):
    """Keep source identity and graph edges; name equality is not control evidence."""
    direct = declarations(metadata)
    grouped = defaultdict(list)
    for dep in direct:
        grouped[(dep["name"], dep.get("source"), dep.get("path"), dep.get("registry"))].append(dep)
    requirements = [dict(name=key[0], source=key[1], path=key[2], registry=key[3],
                         declarations=values, declaration_count=len(values),
                         package_count=len({d["package_id"] for d in values}),
                         requirements=sorted({d["req"] for d in values}))
                    for key, values in sorted(grouped.items(), key=lambda item: repr(item[0]))]
    members = set(metadata["workspace_members"])
    external = [p for p in metadata["packages"] if p["id"] not in members]
    identities = defaultdict(list)
    for package in external:
        identities[package["name"]].append(package)
    duplicates = [dict(name=name, packages=[dict(id=p["id"], version=p["version"], source=p.get("source"),
                                               manifest_path=p.get("manifest_path"))
                                           for p in packages],
                       version_count=len({p["version"] for p in packages}),
                       source_count=len({p.get("source") or p["manifest_path"] for p in packages}))
                  for name, packages in sorted(identities.items()) if len(packages) > 1]
    edges = [dict(from_id=node["id"], to_id=dep["pkg"], name=dep["name"],
                  dep_kinds=dep["dep_kinds"])
             for node in (metadata.get("resolve") or {}).get("nodes", [])
             for dep in node.get("deps", [])]
    reverse = defaultdict(list)
    for edge in edges:
        reverse[edge["to_id"]].append(edge)
    return {"workspace-direct-dependencies": direct,
            "workspace-requirements-by-dependency": requirements,
            "resolved-duplicate-identities": duplicates,
            "resolved-dependency-edges": edges,
            "reverse-dependency-edges": dict(reverse)}


def manifest_snapshot(root, metadata):
    paths = {root / "Cargo.toml"}
    if metadata:
        members = set(metadata["workspace_members"])
        paths.update(Path(p["manifest_path"]) for p in metadata["packages"] if p["id"] in members)
    records = []
    for path in sorted(paths):
        try:
            raw = path.read_bytes()
            records.append(dict(path=str(path), sha256=hashlib.sha256(raw).hexdigest(),
                                manifest=tomllib.loads(raw.decode("utf-8"))))
        except (OSError, ValueError) as error:
            records.append(dict(path=str(path), error=str(error)))
    return records


def lock_hash(root):
    path = root / "Cargo.lock"
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.exists() else None


def collect(root, *, timeout=300):
    base = root / "target" / "dependency-report"
    base.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="run-", dir=base))
    index = dict(schema_version=1, started_at=datetime.now(timezone.utc).isoformat(),
                 repo_root=str(root), host=platform.platform(), python=platform.python_version(),
                 lockfile_before=lock_hash(root), commands={}, derived={},
                 scope="Metadata includes all targets; tree reports use the host target. Feature views are separate.")
    write_json(output / "index.json", dict(index, status="collecting"))
    metadata_views = {}
    commands = [
        ("git-revision", ["git", "rev-parse", "HEAD"], False),
        ("git-status", ["git", "status", "--porcelain=v1"], False),
        ("rust-version", ["rustc", "-Vv"], False),
        ("cargo-version", ["cargo", "-V"], False),
        ("outdated-version", ["cargo", "outdated", "--version"], False),
        ("metadata-declarations", ["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"], True),
        ("metadata-default", ["cargo", "metadata", "--format-version", "1", "--locked"], True),
        ("metadata-all-features", ["cargo", "metadata", "--format-version", "1", "--all-features", "--locked"], True),
        ("tree-duplicates", ["cargo", "tree", "--workspace", "--duplicates", "--locked"], False),
        ("tree-features", ["cargo", "tree", "--workspace", "-e", "features", "--locked"], False),
        ("outdated", ["cargo", "outdated", "--workspace", "--format", "json"], True),
    ]
    for name, argv, is_json in commands:
        record, data = run_command(root, output, name, argv, json_output=is_json,
                                   timeout=min(timeout, 15) if name.endswith("version") or name.startswith("git-") else timeout)
        index["commands"][name] = record
        if name.startswith("metadata-") and data is not None:
            metadata_views[name] = data
        write_json(output / "index.json", dict(index, status="collecting"))
    for view in ("metadata-declarations", "metadata-default", "metadata-all-features"):
        if view not in metadata_views:
            index["derived"][view] = dict(status="skipped", reason=f"{view} failed; see command diagnostics")
            continue
        reports = summarize(metadata_views[view])
        if view == "metadata-declarations":
            reports = {key: value for key, value in reports.items() if key.startswith("workspace-")}
        for name, value in reports.items():
            artifact = f"{view}-{name}.json"
            write_json(output / artifact, value)
            index["derived"][artifact] = dict(status="ok", input=f"{view}.json")
    baseline = next(iter(metadata_views.values()), None)
    manifests = manifest_snapshot(root, baseline)
    write_json(output / "manifests.json", manifests)
    index["manifest_coverage"] = "workspace members" if baseline else "root only; enumerate members from root manifest"
    index["manifest_errors"] = [m for m in manifests if "error" in m]
    index["lockfile_after"] = lock_hash(root)
    index["lockfile_unchanged"] = index["lockfile_before"] == index["lockfile_after"]
    index["finished_at"] = datetime.now(timezone.utc).isoformat()
    index["status"] = "complete" if (all(c["status"] == "ok" for c in index["commands"].values())
                                       and not index["manifest_errors"] and index["lockfile_unchanged"]) else "partial"
    write_json(output / "index.json", index)
    return output, index


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--timeout", type=int, default=300, help="Per Cargo report timeout in seconds")
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error("--timeout must be positive")
    root = args.repo_root.resolve()
    if not (root / "Cargo.toml").is_file():
        parser.error("--repo-root must contain Cargo.toml")
    output, index = collect(root, timeout=args.timeout)
    print(f"Evidence collection: **{index['status']}**.")
    print(f"Read [this run's index](<{(output / 'index.json').as_uri()}>) first.")
    print(f"Read [manifest declarations](<{(output / 'manifests.json').as_uri()}>).")
    print("Artifact paths inside the index are relative to its directory. Use only this run.")
    failed = [name for name, command in index["commands"].items() if command["status"] != "ok"]
    if failed:
        print("Unavailable reports: " + ", ".join(failed) + ". Read their captured diagnostics; do not infer a cause.")
    if not index["lockfile_unchanged"]:
        print("**Blocker:** Cargo.lock changed during collection; establish why before relying on this snapshot.")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Check that each migrated package's Cargo test targets equal its manifest.

Acceptance 1 and the Phase 8 `cargo metadata` sweep. For each of the four
`*-migration.json` manifests, it compares the package's `test`-kind targets
from `cargo metadata` with the manifest's `targets` on name, source path,
and required features. It also checks that the manifest declares
`autotests = false` and maps every old target to exactly one module.

Usage: metadata-check.py [--metadata FILE]

`--metadata` reads a saved `cargo metadata --format-version 1 --no-deps`
document in place of running Cargo, so the mutation check can feed it a
tampered copy. Exits 1 on any mismatch.
"""

import argparse
import json
import pathlib
import subprocess
import sys
import tomllib

FEATURE_DIR = pathlib.Path(__file__).resolve().parent.parent
REPO_ROOT = FEATURE_DIR.parent.parent
MANIFESTS = [
    "claudine-cli-migration.json",
    "darkmatter-migration.json",
    "darkmatter-cli-migration.json",
    "biscuit-terminal-migration.json",
]


def load_metadata(path):
    if path:
        return json.loads(pathlib.Path(path).read_text())
    output = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"],
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return json.loads(output)


def relative(path):
    return pathlib.Path(path).resolve().relative_to(REPO_ROOT.resolve()).as_posix()


def check_package(manifest, packages):
    problems = []
    name = manifest["package"]
    package = packages.get(name)
    if package is None:
        return [f"{name}: not in cargo metadata"]

    cargo_toml = tomllib.loads((REPO_ROOT / manifest["manifest"]).read_text())
    if cargo_toml.get("package", {}).get("autotests") is not False:
        problems.append(f"{name}: `autotests = false` missing from {manifest['manifest']}")

    actual = {
        (target["name"], relative(target["src_path"]), tuple(sorted(target.get("required-features") or [])))
        for target in package["targets"]
        if "test" in target["kind"]
    }
    declared = {
        (target["name"], target["path"], tuple(sorted(target["required_features"])))
        for target in manifest["targets"]
    }
    for extra in sorted(actual - declared):
        problems.append(f"{name}: undeclared Cargo test target {extra}")
    for missing in sorted(declared - actual):
        problems.append(f"{name}: declared target absent from Cargo {missing}")

    target_names = {target["name"] for target in manifest["targets"]}
    old_targets = {}
    for module in manifest["modules"]:
        old_targets.setdefault(module["old_target"], []).append(module)
        if module["target"] not in target_names:
            problems.append(f"{name}: {module['old_target']} maps to unknown target {module['target']}")
    for old_target, modules in sorted(old_targets.items()):
        if len(modules) != 1:
            problems.append(f"{name}: old target {old_target} maps to {len(modules)} modules")

    print(f"{name}: {len(actual)} Cargo test targets, {len(declared)} declared, {len(old_targets)} old targets mapped")
    return problems


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--metadata")
    args = parser.parse_args()

    metadata = load_metadata(args.metadata)
    packages = {package["name"]: package for package in metadata["packages"]}
    problems = []
    for file_name in MANIFESTS:
        manifest = json.loads((FEATURE_DIR / file_name).read_text())
        problems.extend(check_package(manifest, packages))

    for problem in problems:
        print(f"FAIL {problem}")
    print("PASS" if not problems else f"{len(problems)} problem(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())

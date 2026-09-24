#!/usr/bin/env python3
"""List the tests each probe's narrowed test-input units select.

For every probe path, take the units `test-input-probe.py` resolved for it,
and run `cargo nextest list` under the package's CI feature union with
`<unit> & <L1 tier filter>`, which is what a narrowed L1 cell runs.
Writes `test-inputs-listings-<label>.json`: {probe: {unit: [binary_id test, …]}}.

Usage: test-input-listings.py <probe-json> <label>
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

REPO = Path(subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip())
HERE = Path(__file__).resolve().parent

PROBES = {
    "tree-hugger": "tree-hugger/lib/tests/fixtures/corpus_manifest.yaml",
    "claudine": "claudine/cli/Cargo.toml",
    "sniff": "sniff/lib/benches/ci-bench-ids.txt",
    "biscuit-file": "biscuit-file/lib/tests/corpus/yaml_corpus.json",
    "schematic-gen": "schematic/gen/tests/fixtures/complex_auth.json",
    "biscuit-terminal-cli": "biscuit-terminal/cli/README.md",
    "claudine-gen": "claudine/docs/research/agent-errors/_schema.yaml",
    "dmls": "darkmatter/dmls/tests/fixtures/mapping_only_corpus/_docs.md",
    "sniff-cli": "sniff/cli/tests/snapshots/snapshots__cargo_monorepo_structure_json.snap",
}


def ci_features(package: str) -> list[str]:
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1", "--offline"], cwd=REPO, text=True))
    record = next(p for p in metadata["packages"] if p["name"] == package)
    return (((record.get("metadata") or {}).get("ci") or {}).get("tests") or {}).get("features", [])


def main() -> int:
    rows = json.loads(Path(sys.argv[1]).read_text())
    label = sys.argv[2]
    result: dict[str, dict[str, list[str]]] = {}
    for package, probe in PROBES.items():
        units = sorted({r["unit"] for r in rows if r["package"] == package and r["path"] == probe and r["unit"]})
        tier = subprocess.check_output(["just", "_tier_filter", "L1", package], cwd=REPO, text=True).strip()
        features = ci_features(package)
        result[probe] = {}
        for unit in units:
            command = ["cargo", "nextest", "list", "-p", package, "--message-format", "json",
                       "-E", f"({unit}) & ({tier})"]
            if features:
                command += ["--features", ",".join(features)]
            listing = json.loads(subprocess.check_output(command, cwd=REPO, text=True))
            selected = sorted(
                f"{suite['binary-id']} {name}"
                for suite in listing["rust-suites"].values()
                for name, case in suite["testcases"].items()
                if case["filter-match"]["status"] == "matches"
            )
            result[probe][unit] = selected
            print(f"{package}\t{unit}\t{len(selected)}", file=sys.stderr)
    (HERE / f"test-inputs-listings-{label}.json").write_text(json.dumps(result, indent=2) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

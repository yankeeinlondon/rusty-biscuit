#!/usr/bin/env python3
"""After-move test-input probe for one package (spec hazard 6, rulings R15).

Re-runs `baseline/test-input-probe.py` on the moved tree, keeps the rows for
the package's probe path, lists what each after unit selects in a narrowed L1
cell (`<unit> & <L1 tier filter>` under the CI feature union), and compares
that set with the before set from `baseline/test-inputs-listings-before.json`
after mapping every before identity through the migration manifest.

Exit 0 when every before test is selected after and nothing else is; exit 1
otherwise. Writes the evidence to `<package>/test-inputs.md` and
`<package>/test-inputs.json`.

Usage: test-input-check.py <package>
"""
from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path

FEATURE = Path(__file__).resolve().parent.parent
REPO = Path(subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip())
sys.path.insert(0, str(FEATURE / "baseline"))

import importlib.util  # noqa: E402

_spec = importlib.util.spec_from_file_location("listings", FEATURE / "baseline" / "test-input-listings.py")
listings = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(listings)


def main() -> int:
    package = sys.argv[1]
    probe = listings.PROBES[package]
    manifest = json.loads((FEATURE / f"{package}-migration.json").read_text())
    rename = {
        f"{row['old_binary_id']} {test['old']}": f"{row['binary_id']} {test['new']}"
        for row in manifest["modules"] for test in row["tests"]
    }

    before_rows = [r for r in json.loads((FEATURE / "baseline" / "test-input-probe-before.json").read_text())
                   if r["package"] == package and r["path"] == probe]
    before_sets = json.loads((FEATURE / "baseline" / "test-inputs-listings-before.json").read_text())[probe]
    with tempfile.TemporaryDirectory() as scratch:
        probe_json = Path(scratch) / "probe.json"
        subprocess.run([sys.executable, str(FEATURE / "baseline" / "test-input-probe.py"), "--json", str(probe_json)],
                       cwd=REPO, check=True, capture_output=True, text=True)
        after_all = json.loads(probe_json.read_text())
    after_rows = [r for r in after_all if r["package"] == package and r["path"] == probe]

    tier = subprocess.check_output(["just", "_tier_filter", "L1", package], cwd=REPO, text=True).strip()
    features = listings.ci_features(package)
    after_sets: dict[str, list[str]] = {}
    for unit in sorted({r["unit"] for r in after_rows if r["unit"]}):
        command = ["cargo", "nextest", "list", "-p", package, "--message-format", "json", "-E", f"({unit}) & ({tier})"]
        if features:
            command += ["--features", ",".join(features)]
        listing = json.loads(subprocess.check_output(command, cwd=REPO, text=True))
        after_sets[unit] = sorted(
            f"{suite['binary-id']} {name}"
            for suite in listing["rust-suites"].values()
            for name, case in suite["testcases"].items()
            if case["filter-match"]["status"] == "matches"
        )

    before_mapped = sorted(rename.get(t, f"UNMAPPED {t}") for unit in before_sets.values() for t in unit)
    after_union = sorted({t for unit in after_sets.values() for t in unit})
    missing = sorted(set(before_mapped) - set(after_union))
    extra = sorted(set(after_union) - set(before_mapped))
    ok = not missing and not extra and len(before_rows) == len(after_rows) and bool(after_sets) == bool(before_sets)

    out_dir = FEATURE / package
    out_dir.mkdir(exist_ok=True)
    (out_dir / "test-inputs.json").write_text(json.dumps({
        "probe": probe, "before_rows": before_rows, "after_rows": after_rows,
        "before_units": before_sets, "after_units": after_sets,
        "before_mapped": before_mapped, "missing": missing, "extra": extra, "identical": ok,
    }, indent=2) + "\n")
    lines = [
        f"# Test-input probe after the move — `{package}`",
        "",
        "Spec hazard 6, rulings R15. Script: `measure/test-input-check.py`, which re-runs",
        "`baseline/test-input-probe.py` (the planner's own `test_inputs.scan`) on the moved tree and",
        "lists each unit with `cargo nextest list -E '(<unit>) & (<L1 tier filter>)'` under the CI",
        f"feature union (`{','.join(features) or 'none'}`).",
        "",
        f"Probe path: `{probe}`",
        "",
        "| Side | Reference (source:line) | Unit | Tests selected |",
        "|---|---|---|---:|",
    ]
    for side, rows, sets in (("before", before_rows, before_sets), ("after", after_rows, after_sets)):
        for row in rows:
            lines.append(f"| {side} | `{row['source']}:{row['line']}` | `{row['unit']}` | {len(sets.get(row['unit'], []))} |")
    lines += [
        "",
        f"Before set mapped through `{package}-migration.json`: {len(before_mapped)} tests. "
        f"After set: {len(after_union)} tests. Missing: {len(missing)}. Extra: {len(extra)}.",
        "",
        f"**Verdict: {'identical' if ok else 'DIFFERENT'}.** The full lists are in `test-inputs.json`.",
    ]
    if missing or extra:
        lines += ["", "Missing:", *[f"- `{t}`" for t in missing], "", "Extra:", *[f"- `{t}`" for t in extra]]
    (out_dir / "test-inputs.md").write_text("\n".join(lines) + "\n")
    print("\n".join(lines))
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
